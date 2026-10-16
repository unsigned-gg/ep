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
use escapement_serve::{Orchestrator, OrchestratorConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct PlanNodeSpec {
    id: String,
    capability: String,
    #[serde(default)]
    priority: u8,
    #[serde(default)]
    needs: Vec<String>,
}

#[derive(Debug, Serialize)]
struct PlanNodeJson<'a> {
    step: usize,
    id: &'a str,
    capability: &'a str,
    priority: u8,
    needs: Vec<&'a str>,
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

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|a| a == flag)
}

fn positional_args(args: &[String]) -> Vec<&str> {
    args.iter()
        .filter(|a| !a.starts_with("--"))
        .map(String::as_str)
        .collect()
}

fn run_cli(args: &[String]) -> Result<String, String> {
    let json_mode = has_flag(args, "--json");
    let mermaid_mode = has_flag(args, "--mermaid");
    let pos = positional_args(args);

    if pos.is_empty() {
        return Ok(usage_text());
    }

    match pos[0] {
        "plan" => {
            if pos.len() < 3 {
                return Err(
                    "usage: ep plan <validate|render> <path.json> [--json|--mermaid]".into(),
                );
            }
            let raw = std::fs::read_to_string(pos[2])
                .map_err(|e| format!("failed to read {}: {e}", pos[2]))?;
            let plan = parse_plan_json(&raw)?;
            match pos[1] {
                "validate" => {
                    if json_mode {
                        Ok(format!(
                            "{{\"valid\":true,\"nodes\":{},\"cycles\":0}}\n",
                            plan.len()
                        ))
                    } else {
                        Ok(format!("plan valid: {} nodes, 0 cycles\n", plan.len()))
                    }
                }
                "render" => {
                    if mermaid_mode {
                        Ok(render_plan_mermaid(&plan))
                    } else if json_mode {
                        render_plan_json(&plan)
                    } else {
                        Ok(render_plan(&plan))
                    }
                }
                other => Err(format!("unknown plan subcommand: {other}")),
            }
        }
        "wal" => {
            if pos.len() < 3 {
                return Err("usage: ep wal <inspect|replay> <path.jsonl> [--json]".into());
            }
            let file = File::open(pos[2]).map_err(|e| format!("failed to open {}: {e}", pos[2]))?;
            let reader = BufReader::new(file);
            match pos[1] {
                "inspect" => inspect_wal(reader, json_mode),
                "replay" => replay_wal(reader, json_mode),
                other => Err(format!("unknown wal subcommand: {other}")),
            }
        }
        "lanes" => {
            let sub = pos.get(1).copied().unwrap_or("status");
            match sub {
                "status" => Ok(lanes_status_report(json_mode)),
                other => Err(format!("unknown lanes subcommand: {other}")),
            }
        }
        "mesh" => {
            let sub = pos.get(1).copied().unwrap_or("status");
            match sub {
                "status" => Ok(mesh_status_report(json_mode)),
                other => Err(format!("unknown mesh subcommand: {other}")),
            }
        }
        "serve" => {
            let bind_addr = parse_bind_addr(args).unwrap_or_else(|| "127.0.0.1:8080".to_string());
            let orch = Orchestrator::new(OrchestratorConfig::default());
            escapement_serve::http::serve(&bind_addr, orch)
                .map_err(|e| format!("escapement-serve failed on {bind_addr}: {e}"))?;
            Ok(String::new())
        }
        "--help" | "-h" | "help" => Ok(usage_text()),
        other => Err(format!("unknown command: {other}\n{}", usage_text())),
    }
}

fn parse_bind_addr(args: &[String]) -> Option<String> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--bind" {
            return iter.next().cloned();
        }
        if let Some(rest) = arg.strip_prefix("--bind=") {
            return Some(rest.to_string());
        }
    }
    None
}

fn usage_text() -> String {
    concat!(
        "ep (escapement) v1.1.0\n",
        "Commands:\n",
        "  ep plan validate <path.json> [--json]            Validate DAG plan (duplicate IDs, missing needs, cycles)\n",
        "  ep plan render <path.json> [--json|--mermaid]    Render deterministic topological order or Mermaid DAG\n",
        "  ep wal inspect <path.jsonl> [--json]             Count WAL entries by operation kind\n",
        "  ep wal replay <path.jsonl> [--json]              Reconstruct submitted, completed, and in-flight tasks\n",
        "  ep lanes status [--json]                         Display priority lanes and deficit weights\n",
        "  ep mesh status [--json]                          Display Cortex mesh lane & pool contract status\n",
        "  ep serve [--bind <addr>]                         Run the escapement-serve HTTP daemon (default 127.0.0.1:8080)\n",
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

fn render_plan_json(plan: &Plan) -> Result<String, String> {
    let ordered = plan.topological_order();
    let items: Vec<PlanNodeJson<'_>> = ordered
        .iter()
        .enumerate()
        .map(|(idx, node)| PlanNodeJson {
            step: idx + 1,
            id: node.task.id.as_str(),
            capability: &node.task.required_capability,
            priority: node.task.priority,
            needs: node.needs.iter().map(TaskId::as_str).collect(),
        })
        .collect();
    let mut s = serde_json::to_string(&items).map_err(|e| e.to_string())?;
    s.push('\n');
    Ok(s)
}

fn render_plan_mermaid(plan: &Plan) -> String {
    let mut out = String::from("flowchart TD\n");
    for node in plan.topological_order() {
        out.push_str(&format!(
            "  {}[\"{} ({}, p={})\"]\n",
            node.task.id, node.task.id, node.task.required_capability, node.task.priority
        ));
        for dep in &node.needs {
            out.push_str(&format!("  {} --> {}\n", dep, node.task.id));
        }
    }
    out
}

fn inspect_wal(reader: impl std::io::BufRead, json_mode: bool) -> Result<String, String> {
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

    if json_mode {
        Ok(format!(
            "{{\"total\":{},\"submit\":{},\"dispatch\":{},\"pool_dispatch\":{},\"complete\":{},\"fail\":{},\"timeout\":{},\"cancel\":{}}}\n",
            entries.len(),
            submit,
            dispatch,
            pool_dispatch,
            complete,
            fail,
            timeout,
            cancel
        ))
    } else {
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
}

fn replay_wal(reader: impl std::io::BufRead, json_mode: bool) -> Result<String, String> {
    let entries = persistence::replay(reader).map_err(|e| e.to_string())?;
    let state = persistence::reconstruct(&entries);
    let in_flight: Vec<&str> = state.in_flight.iter().map(TaskId::as_str).collect();
    let completed: Vec<&str> = state.completed.iter().map(TaskId::as_str).collect();
    if json_mode {
        let completed_json = serde_json::to_string(&completed).map_err(|e| e.to_string())?;
        let in_flight_json = serde_json::to_string(&in_flight).map_err(|e| e.to_string())?;
        Ok(format!(
            "{{\"submitted\":{},\"completed\":{},\"in_flight\":{}}}\n",
            state.task_specs.len(),
            completed_json,
            in_flight_json
        ))
    } else {
        Ok(format!(
            "recovered state:\n  submitted: {}\n  completed: {} [{}]\n  in_flight: {} [{}]\n",
            state.task_specs.len(),
            state.completed.len(),
            completed.join(","),
            state.in_flight.len(),
            in_flight.join(",")
        ))
    }
}

fn lanes_status_report(json_mode: bool) -> String {
    let mut sched = LaneScheduler::new(1000);
    let defaults = [("critical", 8u32), ("high", 4), ("normal", 2), ("low", 1)];
    for (name, weight) in defaults {
        let _ = sched.add_lane(PriorityLane::new(name, weight));
    }
    let registered = defaults
        .iter()
        .filter(|(name, _)| sched.has_lane(name))
        .count();
    if json_mode {
        format!(
            "{{\"registered\":{registered},\"lanes\":[{{\"name\":\"critical\",\"weight\":8}},{{\"name\":\"high\",\"weight\":4}},{{\"name\":\"normal\",\"weight\":2}},{{\"name\":\"low\",\"weight\":1}}]}}\n"
        )
    } else {
        format!(
            "lanes registered: {registered}\n  critical: weight=8\n  high: weight=4\n  normal: weight=2\n  low: weight=1\n"
        )
    }
}

fn mesh_status_report(json_mode: bool) -> String {
    if json_mode {
        format!(
            "{{\"lane\":\"estate/w4C:p1\",\"role\":\"session-agy-ceres-estate-w4c-p1\",\"daemon_role\":\"service-escapement\",\"pool_dispatcher_role\":\"service-escapement-pool\",\"pool_addressing\":\"pool:<capability>\",\"status_endpoint\":\"GET /v1/mesh/message-status?peer_id=<uuid>&pool=<slug>&stream_seq=<n>\",\"max_safe_struct_int\":{}}}\n",
            MAX_SAFE_STRUCT_INT
        )
    } else {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use escapement_core::persistence::{write_entry, WalEntry};

    #[test]
    fn parse_and_render_plan_topological_order_json_and_mermaid() {
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

        let json_out = render_plan_json(&plan).unwrap();
        assert!(json_out.contains("\"id\":\"build\""));
        assert!(json_out.contains("\"step\":1"));

        let mermaid_out = render_plan_mermaid(&plan);
        assert!(mermaid_out.starts_with("flowchart TD\n"));
        assert!(mermaid_out.contains("build --> test"));
        assert!(mermaid_out.contains("test --> deploy"));
    }

    #[test]
    fn wal_inspect_and_replay_reports_pool_dispatch_text_and_json() {
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

        let inspected = inspect_wal(std::io::Cursor::new(&buf), false).unwrap();
        assert!(inspected.contains("wal entries: 2"));
        assert!(inspected.contains("pool_dispatch: 1"));

        let inspected_json = inspect_wal(std::io::Cursor::new(&buf), true).unwrap();
        assert!(inspected_json.contains("\"pool_dispatch\":1"));

        let replayed = replay_wal(std::io::Cursor::new(&buf), false).unwrap();
        assert!(replayed.contains("in_flight: 1 [t1]"));

        let replayed_json = replay_wal(std::io::Cursor::new(&buf), true).unwrap();
        assert!(replayed_json.contains("\"in_flight\":[\"t1\"]"));
    }

    #[test]
    fn lanes_and_mesh_status_reports_are_populated_in_text_and_json() {
        let lanes = lanes_status_report(false);
        assert!(lanes.contains("lanes registered: 4"));
        let lanes_json = lanes_status_report(true);
        assert!(lanes_json.contains("\"registered\":4"));

        let mesh = mesh_status_report(false);
        assert!(mesh.contains("role: session-agy-ceres-estate-w4c-p1"));
        assert!(mesh.contains("pool_dispatcher_role: service-escapement-pool"));
        assert!(mesh.contains("9007199254740992"));

        let mesh_json = mesh_status_report(true);
        assert!(mesh_json.contains("\"role\":\"session-agy-ceres-estate-w4c-p1\""));
        assert!(mesh_json.contains("\"max_safe_struct_int\":9007199254740992"));
    }
}
