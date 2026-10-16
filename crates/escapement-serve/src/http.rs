//! HTTP server for escapement-serve — a zero-dependency TCP listener that
//! serves the dispatch API.
//!
//! Routes:
//! - GET /healthz — liveness
//! - GET /readyz — readiness (orchestrator healthy)
//! - GET /version — service identity
//! - POST /dispatch — submit a task (JSON body)
//! - GET /jobs — list job states
//! - GET /jobs/:id — query job state

use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::TcpListener;

use escapement_core::dispatch::{Task, TaskId};
use escapement_core::protocol::PROTOCOL_VERSION;

use crate::{Orchestrator, OrchestratorError};

const SERVICE: &str = "escapement-serve";
const VERSION: &str = "1.0.0";

/// Start the HTTP server on the given address.
///
/// # Errors
/// Returns [`io::Error`] if the listener can't bind.
pub fn serve(addr: &str, orch: Orchestrator) -> io::Result<()> {
    let listener = TcpListener::bind(addr)?;
    eprintln!("escapement-serve v{VERSION} listening on {addr}");

    let mut orch = orch;
    for stream in listener.incoming() {
        let stream = match stream {
            Ok(s) => s,
            Err(e) => {
                eprintln!("accept error: {e}");
                continue;
            }
        };
        if let Err(e) = handle_request(&mut orch, stream) {
            eprintln!("request error: {e}");
        }
    }
    Ok(())
}

fn handle_request(orch: &mut Orchestrator, stream: std::net::TcpStream) -> io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut writer = stream;

    // Read the request line and headers.
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        write_response(&mut writer, 400, &json_error("malformed request"))?;
        return Ok(());
    }

    let method = parts[0];
    let path = parts[1];

    // Read headers — extract traceparent and content-length.
    let mut content_length = 0;
    let mut traceparent: Option<String> = None;
    loop {
        let mut header = String::new();
        let n = reader.read_line(&mut header)?;
        if n == 0 || header.trim().is_empty() {
            break;
        }
        let lower = header.to_lowercase();
        if lower.starts_with("content-length:") {
            content_length = lower
                .trim_start_matches("content-length:")
                .trim()
                .parse()
                .unwrap_or(0);
        } else if lower.starts_with("traceparent:") {
            traceparent = Some(lower.trim_start_matches("traceparent:").trim().to_string());
        }
    }

    // Read body if present.
    let body = if content_length > 0 {
        let mut buf = vec![0u8; content_length];
        reader.read_exact(&mut buf)?;
        String::from_utf8_lossy(&buf).to_string()
    } else {
        String::new()
    };

    // Parse inbound trace context (W3C Trace Context).
    let trace_ctx = traceparent
        .as_deref()
        .and_then(escapement_core::TraceContext::from_header);

    // Route — pass the trace context so spans can be created.
    let response = route(orch, method, path, &body, trace_ctx.as_ref());

    // If we created a span, export it.
    if let Some(ctx) = &trace_ctx {
        if let Some(span) = &response.2 {
            let _ = orch.export_span(span, ctx);
        }
    }

    write_response(&mut writer, response.0, &response.1)?;
    Ok(())
}

fn route(
    orch: &mut Orchestrator,
    method: &str,
    path: &str,
    body: &str,
    trace_ctx: Option<&escapement_core::TraceContext>,
) -> (u16, String, Option<escapement_core::Span>) {
    match (method, path) {
        ("GET", "/healthz") => {
            let healthy = orch.is_healthy();
            let json = format!(
                r#"{{"ok":true,"healthy":{healthy},"queue_depth":{}}}"#,
                orch.queue_depth()
            );
            (200, json, None)
        }
        ("GET", "/readyz") => (200, r#"{"status":"ok"}"#.into(), None),
        ("GET", "/version") => (
            200,
            format!(
                r#"{{"service":"{SERVICE}","version":"{VERSION}","protocol":"{PROTOCOL_VERSION}"}}"#
            ),
            None,
        ),
        ("POST", "/dispatch") => handle_dispatch(orch, body, trace_ctx),
        ("POST", "/tasks/claim") => {
            let (status, resp) = handle_claim(orch, body);
            (status, resp, None)
        }
        ("POST", "/tasks/complete") => {
            let (status, resp) = handle_complete(orch, body);
            (status, resp, None)
        }
        ("POST", "/tasks/fail") => {
            let (status, resp) = handle_fail(orch, body);
            (status, resp, None)
        }
        ("POST", "/pool/results") => {
            let (status, resp) = handle_pool_results(orch, body);
            (status, resp, None)
        }
        ("GET", "/jobs") => {
            let (status, resp) = handle_list_jobs(orch);
            (status, resp, None)
        }
        ("GET", p) if p.starts_with("/jobs/") => {
            let (status, resp) = handle_get_job(orch, &p["/jobs/".len()..]);
            (status, resp, None)
        }
        _ => (404, json_error("not found"), None),
    }
}

fn handle_dispatch(
    orch: &mut Orchestrator,
    body: &str,
    trace_ctx: Option<&escapement_core::TraceContext>,
) -> (u16, String, Option<escapement_core::Span>) {
    let task = match parse_dispatch_body(body) {
        Ok(t) => t,
        Err(msg) => return (400, json_error(&msg), None),
    };

    // Create a child span if we have a trace context.
    let span = trace_ctx.map(|ctx| {
        let mut s = ctx.child_span("escapement.dispatch", &task.id);
        s.set_attribute("capability", &task.required_capability);
        s.set_attribute("priority", task.priority.to_string());
        s.end();
        s
    });

    match orch.submit_task(task) {
        Ok(()) => (
            202,
            format!(
                r#"{{"protocol":"{PROTOCOL_VERSION}","decision":"admitted","queue_depth":{}}}"#,
                orch.queue_depth()
            ),
            span,
        ),
        Err(OrchestratorError::DuplicateTask(_)) => (409, json_error("duplicate task"), span),
        Err(OrchestratorError::ProviderAtCapacity(p)) => (
            429,
            format!(r#"{{"error":"provider at capacity","provider":"{p}"}}"#),
            span,
        ),
        Err(OrchestratorError::BudgetExceeded(p)) => (
            429,
            format!(r#"{{"error":"budget exceeded","provider":"{p}"}}"#),
            span,
        ),
        Err(OrchestratorError::RateLimited) => (429, json_error("rate limited"), span),
        Err(OrchestratorError::UnknownTask(_)) => (404, json_error("unknown task"), span),
        Err(OrchestratorError::InvalidStateTransition(_)) => {
            (409, json_error("invalid state transition"), span)
        }
        Err(OrchestratorError::InvalidPoolRefs(msg)) => (400, json_error(&msg), span),
        Err(OrchestratorError::WakerUnhealthy(msg)) => (503, json_error(&msg), span),
        Err(OrchestratorError::PoolTransport(msg)) => (502, json_error(&msg), span),
    }
}

fn handle_claim(orch: &mut Orchestrator, body: &str) -> (u16, String) {
    let Some(agent_str) = parse_json_string(body, "agent_id") else {
        return (400, json_error("missing agent_id"));
    };
    let Ok(agent_id) = escapement_core::registry::AgentId::new(agent_str) else {
        return (400, json_error("invalid agent_id"));
    };
    match orch.claim_task(&agent_id) {
        Some((task_id, capability)) => (
            200,
            format!(
                r#"{{"task_id":"{}","capability":"{}"}}"#,
                task_id.as_str(),
                capability
            ),
        ),
        None => (204, String::new()),
    }
}

fn handle_complete(orch: &mut Orchestrator, body: &str) -> (u16, String) {
    let Some(id_str) = parse_json_string(body, "id").or_else(|| parse_json_string(body, "task_id"))
    else {
        return (400, json_error("missing task id"));
    };
    let Ok(task_id) = TaskId::new(id_str) else {
        return (400, json_error("invalid task id"));
    };
    match orch.complete_task(&task_id) {
        Ok(()) => (
            200,
            format!(r#"{{"id":"{}","state":"completed"}}"#, task_id.as_str()),
        ),
        Err(_) => (404, json_error("unknown or non-assigned task")),
    }
}

fn handle_fail(orch: &mut Orchestrator, body: &str) -> (u16, String) {
    let Some(id_str) = parse_json_string(body, "id").or_else(|| parse_json_string(body, "task_id"))
    else {
        return (400, json_error("missing task id"));
    };
    let Ok(task_id) = TaskId::new(id_str) else {
        return (400, json_error("invalid task id"));
    };
    let reason = parse_json_string(body, "reason").unwrap_or_else(|| "unspecified failure".into());
    match orch.fail_task(&task_id, reason) {
        Ok(()) => (
            200,
            format!(r#"{{"id":"{}","state":"failed"}}"#, task_id.as_str()),
        ),
        Err(_) => (404, json_error("unknown or non-assigned task")),
    }
}

fn handle_pool_results(orch: &mut Orchestrator, body: &str) -> (u16, String) {
    let result_refs = match crate::pool::parse_pool_result_refs(body) {
        Ok(r) => r,
        Err(msg) => return (400, json_error(&msg)),
    };
    let Ok(task_id) = TaskId::new(&result_refs.task_id) else {
        return (400, json_error("invalid task_id"));
    };
    let pool_status = parse_json_string(body, "pool_status")
        .as_deref()
        .and_then(ep_domain::PoolMessageStatus::parse)
        .unwrap_or(ep_domain::PoolMessageStatus::Acked);

    match orch.reconcile_pool_job(&task_id, pool_status, Some(&result_refs)) {
        Ok(resolution) => {
            let res_str = match resolution {
                ep_domain::JobResolution::InFlight => "in_flight",
                ep_domain::JobResolution::Completed => "completed",
                ep_domain::JobResolution::Requeue { .. } => "requeued",
                ep_domain::JobResolution::DeadLetter { .. } => "dead_lettered",
            };
            (
                200,
                format!(
                    r#"{{"task_id":"{}","resolution":"{res_str}"}}"#,
                    task_id.as_str()
                ),
            )
        }
        Err(_) => (404, json_error("unknown or non-pool-assigned task")),
    }
}

fn handle_list_jobs(orch: &Orchestrator) -> (u16, String) {
    let depth = orch.queue_depth();
    let entries: Vec<String> = orch
        .list_jobs()
        .into_iter()
        .map(|(id, cap, pri, attempt, state)| {
            format!(
                r#"{{"job_id":"{}","capability":"{}","priority":{},"attempt":{},"state":"{}"}}"#,
                id.as_str(),
                cap,
                pri,
                attempt,
                state
            )
        })
        .collect();
    (
        200,
        format!(
            r#"{{"jobs":[{}],"queue_depth":{depth}}}"#,
            entries.join(",")
        ),
    )
}

fn handle_get_job(orch: &Orchestrator, job_id: &str) -> (u16, String) {
    let Ok(task_id) = TaskId::new(job_id) else {
        return (400, json_error("invalid job id"));
    };
    match orch.get_job(&task_id) {
        Some((cap, pri, attempt, state)) => (
            200,
            format!(
                r#"{{"job_id":"{}","capability":"{}","priority":{},"attempt":{},"state":"{}"}}"#,
                task_id.as_str(),
                cap,
                pri,
                attempt,
                state
            ),
        ),
        None => (404, json_error("job not found")),
    }
}

fn parse_dispatch_body(body: &str) -> Result<Task, String> {
    let task_id = parse_json_string(body, "id").ok_or_else(|| "missing task id".to_string())?;
    let capability =
        parse_json_string(body, "capability").ok_or_else(|| "missing capability".to_string())?;
    let priority = parse_json_numeric(body, "priority").unwrap_or(0);

    let id = TaskId::new(task_id).map_err(|_| "invalid task id".to_string())?;
    Ok(Task {
        id,
        required_capability: capability,
        priority,
    })
}

fn write_response(writer: &mut impl Write, status: u16, body: &str) -> io::Result<()> {
    let status_text = match status {
        200 => "OK",
        202 => "Accepted",
        204 => "No Content",
        400 => "Bad Request",
        404 => "Not Found",
        409 => "Conflict",
        429 => "Too Many Requests",
        _ => "Internal Server Error",
    };
    write!(
        writer,
        "HTTP/1.1 {status} {status_text}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )?;
    writer.flush()
}

fn json_error(msg: &str) -> String {
    format!(r#"{{"error":"{msg}"}}"#)
}

fn parse_json_string(json: &str, field: &str) -> Option<String> {
    let marker = format!("\"{field}\":\"");
    let start = json.find(&marker)?;
    let rest = &json[start + marker.len()..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn parse_json_numeric(json: &str, field: &str) -> Option<u8> {
    let marker = format!("\"{field}\":");
    let start = json.find(&marker)?;
    let rest = json[start + marker.len()..].trim_start();
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    if end == 0 {
        return None;
    }
    rest[..end].parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OrchestratorConfig;
    use escapement_core::registry::AgentId;

    #[test]
    fn parse_dispatch_body_valid() {
        let body = r#"{"id":"t1","capability":"build","priority":5}"#;
        let task = parse_dispatch_body(body).unwrap();
        assert_eq!(task.id.as_str(), "t1");
        assert_eq!(task.required_capability, "build");
        assert_eq!(task.priority, 5);
    }

    #[test]
    fn parse_dispatch_body_missing_id() {
        let body = r#"{"capability":"build"}"#;
        assert!(parse_dispatch_body(body).is_err());
    }

    #[test]
    fn parse_dispatch_body_no_priority_defaults_zero() {
        let body = r#"{"id":"t1","capability":"build"}"#;
        let task = parse_dispatch_body(body).unwrap();
        assert_eq!(task.priority, 0);
    }

    #[test]
    fn json_error_format() {
        let s = json_error("test error");
        assert!(s.contains("test error"));
        assert!(s.contains("error"));
    }

    #[test]
    fn http_jobs_and_task_lifecycle_routes() {
        let mut orch = Orchestrator::new(OrchestratorConfig::default());
        orch.register_provider("gateway", 5);
        orch.registry_mut()
            .register(AgentId::new("a1").unwrap(), ["build".into()], 0)
            .unwrap();

        // Unknown job returns 404 (not stub 200 unknown)
        let (status, _) = handle_get_job(&orch, "t1");
        assert_eq!(status, 404);

        // Submit via /dispatch
        let (status, _, _) = route(
            &mut orch,
            "POST",
            "/dispatch",
            r#"{"id":"t1","capability":"build","priority":7}"#,
            None,
        );
        assert_eq!(status, 202);

        // Query /jobs/t1 -> queued
        let (status, body, _) = route(&mut orch, "GET", "/jobs/t1", "", None);
        assert_eq!(status, 200);
        assert!(body.contains(r#""state":"queued""#));

        // Claim via /tasks/claim -> assigned
        let (status, body, _) = route(
            &mut orch,
            "POST",
            "/tasks/claim",
            r#"{"agent_id":"a1"}"#,
            None,
        );
        assert_eq!(status, 200);
        assert!(body.contains(r#""task_id":"t1""#));

        let (status, body, _) = route(&mut orch, "GET", "/jobs/t1", "", None);
        assert_eq!(status, 200);
        assert!(body.contains(r#""state":"assigned""#));

        // Complete via /tasks/complete -> completed
        let (status, body, _) = route(&mut orch, "POST", "/tasks/complete", r#"{"id":"t1"}"#, None);
        assert_eq!(status, 200);
        assert!(body.contains(r#""state":"completed""#));

        let (status, body, _) = route(&mut orch, "GET", "/jobs/t1", "", None);
        assert_eq!(status, 200);
        assert!(body.contains(r#""state":"completed""#));
    }

    #[test]
    fn http_pool_results_route_reconciles_state() {
        let mut orch = Orchestrator::new(OrchestratorConfig::default());
        orch.register_provider("gateway", 5);
        route(
            &mut orch,
            "POST",
            "/dispatch",
            r#"{"id":"t-pool","capability":"review","priority":9}"#,
            None,
        );
        orch.dispatch_pool(55, None).unwrap();

        let (status, body, _) = route(&mut orch, "GET", "/jobs/t-pool", "", None);
        assert_eq!(status, 200);
        assert!(body.contains(r#""state":"pool_assigned""#));

        let (status, body, _) = route(
            &mut orch,
            "POST",
            "/pool/results",
            r#"{"stream_seq":55,"task_id":"t-pool","attempt":1,"outcome":"ok"}"#,
            None,
        );
        assert_eq!(status, 200);
        assert!(body.contains(r#""resolution":"completed""#));

        let (status, body, _) = route(&mut orch, "GET", "/jobs/t-pool", "", None);
        assert_eq!(status, 200);
        assert!(body.contains(r#""state":"completed""#));
    }
}
