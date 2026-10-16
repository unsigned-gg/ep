//! `ep` CLI — Escapement scheduler, deterministic DAG planner, WAL inspector, and Cortex mesh status tool.

use std::env;
use std::fs::File;
use std::io::BufReader;
use std::process::ExitCode;

use ep_domain::{CapabilitySlug, MAX_SAFE_STRUCT_INT};
use escapement_core::dispatch::{Task, TaskId};
use escapement_core::lanes::{LaneScheduler, PriorityLane};
use escapement_core::persistence::{self, WalOp};
use escapement_core::plan::{Plan, PlanNode};
use escapement_core::registry::Registry;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct PlanNodeSpec {
    id: String,
    capability: String,
    #[serde(default)]
    priority: u8,
    #[serde(default)]
    needs: Vec<String>,
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        println!("ep (escapement) v1.1.0 — metered, deterministic dispatch engine");
        let reg = Registry::new();
        println!("Registry active with {} registered agents.", reg.len());
        return ExitCode::SUCCESS;
    }

    match run_cli(&args) {
        Ok(output) => {
            print!("{output}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run_cli(args: &[String]) -> Result<String, String> {
    match args[0].as_str() {
        "plan" => {
            if args.len() < 3 {
                return Err("usage: ep plan <validate|render> <path.json>".into());
            }
            let raw = std::fs::read_to_string(&args[2])
                .map_err(|e| format!("failed to read {}: {e}", args[2]))?;
            let plan = parse_plan_json(&raw)?;
            match args[1].as_str() {
                "validate" => Ok(format!("plan valid: {} nodes, 0 cycles\n", plan.len())),
                "render" => Ok(render_plan(&plan)),
                other => Err(format!("unknown plan subcommand: {other}")),
            }
        }
        "wal" => {
            if args.len() < 3 {
                return Err("usage: ep wal <inspect|replay> <path.jsonl>".into());
            }
            let file =
                File::open(&args[2]).map_err(|e| format!("failed to open {}: {e}", args[2]))?;
            let reader = BufReader::new(file);
            match args[1].as_str() {
                "inspect" => inspect_wal(reader),
                "replay" => replay_wal(reader),
                other => Err(format!("unknown wal subcommand: {other}")),
            }
        }
        "lanes" => {
            let sub = args.get(1).map_or("status", String::as_str);
            match sub {
                "status" => Ok(lanes_status_report()),
                other => Err(format!("unknown lanes subcommand: {other}")),
            }
        }
        "mesh" => {
            let sub = args.get(1).map_or("status", String::as_str);
            match sub {
                "status" => Ok(mesh_status_report()),
                other => Err(format!("unknown mesh subcommand: {other}")),
            }
        }
        "--help" | "-h" | "help" => Ok(usage_text()),
        other => Err(format!("unknown command: {other}\n{}", usage_text())),
    }
}

fn usage_text() -> String {
    concat!(
        "ep (escapement) v1.1.0\n",
        "Commands:\n",
        "  ep plan validate <path.json>   Validate DAG plan (duplicate IDs, missing needs, cycles)\n",
        "  ep plan render <path.json>     Render deterministic topological order\n",
        "  ep wal inspect <path.jsonl>    Count WAL entries by operation kind\n",
        "  ep wal replay <path.jsonl>     Reconstruct submitted, completed, and in-flight tasks\n",
        "  ep lanes status                Display priority lanes and deficit weights\n",
        "  ep mesh status                 Display Cortex mesh lane & pool contract status\n",
    )
    .to_string()
}

fn parse_plan_json(raw: &str) -> Result<Plan, String> {
    let specs: Vec<PlanNodeSpec> =
        serde_json::from_str(raw).map_err(|e| format!("invalid plan JSON: {e}"))?;
    let mut nodes = Vec::with_capacity(specs.len());
    for spec in specs {
        let _slug = CapabilitySlug::new(&spec.capability)
            .map_err(|e| format!("node '{}' capability error: {e}", spec.id))?;
        let id = TaskId::new(spec.id).map_err(|e| e.to_string())?;
        let mut needs = Vec::with_capacity(spec.needs.len());
        for need in spec.needs {
            needs.push(TaskId::new(need).map_err(|e| e.to_string())?);
        }
        let task = Task {
            id,
            required_capability: spec.capability,
            priority: spec.priority,
        };
        nodes.push(PlanNode::new(task).with_needs(needs));
    }
    Plan::from_nodes(nodes).map_err(|e| e.to_string())
}

fn render_plan(plan: &Plan) -> String {
    let mut out = String::new();
    for (idx, node) in plan.topological_order().iter().enumerate() {
        let needs: Vec<&str> = node.needs.iter().map(TaskId::as_str).collect();
        out.push_str(&format!(
            "{}. {} [capability={}, priority={}, needs=[{}]]\n",
            idx + 1,
            node.task.id,
            node.task.required_capability,
            node.task.priority,
            needs.join(",")
        ));
    }
    out
}

fn inspect_wal(reader: impl std::io::BufRead) -> Result<String, String> {
    let entries = persistence::replay(reader).map_err(|e| e.to_string())?;
    let mut submit = 0usize;
    let mut dispatch = 0usize;
    let mut pool_dispatch = 0usize;
    let mut complete = 0usize;
    let mut fail = 0usize;
    let mut timeout = 0usize;
    let mut cancel = 0usize;

    for entry in &entries {
        match entry.op {
            WalOp::Submit => submit += 1,
            WalOp::Dispatch => dispatch += 1,
            WalOp::PoolDispatch => pool_dispatch += 1,
            WalOp::Complete => complete += 1,
            WalOp::Fail => fail += 1,
            WalOp::Timeout => timeout += 1,
            WalOp::Cancel => cancel += 1,
        }
    }

    Ok(format!(
        "wal entries: {}\n  submit: {}\n  dispatch: {}\n  pool_dispatch: {}\n  complete: {}\n  fail: {}\n  timeout: {}\n  cancel: {}\n",
        entries.len(),
        submit,
        dispatch,
        pool_dispatch,
        complete,
        fail,
        timeout,
        cancel
    ))
}

fn replay_wal(reader: impl std::io::BufRead) -> Result<String, String> {
    let entries = persistence::replay(reader).map_err(|e| e.to_string())?;
    let state = persistence::reconstruct(&entries);
    let in_flight: Vec<&str> = state.in_flight.iter().map(TaskId::as_str).collect();
    let completed: Vec<&str> = state.completed.iter().map(TaskId::as_str).collect();
    Ok(format!(
        "recovered state:\n  submitted: {}\n  completed: {} [{}]\n  in_flight: {} [{}]\n",
        state.task_specs.len(),
        state.completed.len(),
        completed.join(","),
        state.in_flight.len(),
        in_flight.join(",")
    ))
}

fn lanes_status_report() -> String {
    let mut sched = LaneScheduler::new(1000);
    let defaults = [("critical", 8u32), ("high", 4), ("normal", 2), ("low", 1)];
    for (name, weight) in defaults {
        let _ = sched.add_lane(PriorityLane::new(name, weight));
    }
    let registered = defaults
        .iter()
        .filter(|(name, _)| sched.has_lane(name))
        .count();
    format!(
        "lanes registered: {registered}\n  critical: weight=8\n  high: weight=4\n  normal: weight=2\n  low: weight=1\n"
    )
}

fn mesh_status_report() -> String {
    format!(
        concat!(
            "lane: estate/w4C:p1\n",
            "role: session-agy-ceres-estate-w4c-p1\n",
            "daemon_role: service-escapement\n",
            "pool_dispatcher_role: service-escapement-pool\n",
            "pool_addressing: pool:<capability>\n",
            "status_endpoint: GET /v1/mesh/message-status?peer_id=<uuid>&pool=<slug>&stream_seq=<n>\n",
            "max_safe_struct_int: {}\n",
        ),
        MAX_SAFE_STRUCT_INT
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use escapement_core::persistence::{write_entry, WalEntry};

    #[test]
    fn parse_and_render_plan_topological_order() {
        let raw = r#"[
            {"id":"deploy","capability":"deploy","priority":9,"needs":["test"]},
            {"id":"build","capability":"build","priority":5,"needs":[]},
            {"id":"test","capability":"test","priority":7,"needs":["build"]}
        ]"#;
        let plan = parse_plan_json(raw).unwrap();
        assert_eq!(plan.len(), 3);
        let rendered = render_plan(&plan);
        let lines: Vec<&str> = rendered.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("1. build "));
        assert!(lines[1].starts_with("2. test "));
        assert!(lines[2].starts_with("3. deploy "));
    }

    #[test]
    fn wal_inspect_and_replay_reports_pool_dispatch() {
        let mut buf = Vec::new();
        write_entry(
            &mut buf,
            &WalEntry::submit(TaskId::new("t1").unwrap(), "build", 5),
        )
        .unwrap();
        write_entry(
            &mut buf,
            &WalEntry::pool_dispatch(TaskId::new("t1").unwrap(), "build", 77, 1),
        )
        .unwrap();

        let inspected = inspect_wal(std::io::Cursor::new(&buf)).unwrap();
        assert!(inspected.contains("wal entries: 2"));
        assert!(inspected.contains("pool_dispatch: 1"));

        let replayed = replay_wal(std::io::Cursor::new(&buf)).unwrap();
        assert!(replayed.contains("in_flight: 1 [t1]"));
    }

    #[test]
    fn lanes_and_mesh_status_reports_are_populated() {
        let lanes = lanes_status_report();
        assert!(lanes.contains("lanes registered: 4"));
        let mesh = mesh_status_report();
        assert!(mesh.contains("role: session-agy-ceres-estate-w4c-p1"));
        assert!(mesh.contains("pool_dispatcher_role: service-escapement-pool"));
        assert!(mesh.contains("9007199254740992"));
    }
}
