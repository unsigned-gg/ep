//! Cortex mesh pool client & wire adapters (`pool:<capability>` and `service-escapement-pool`).
//!
//! Bridges `escapement-serve` to the Cortex mesh pool plane:
//! - Outbound `POST /v1/mesh/messages` to `pool:<capability>` with structured `refs`
//! - Outbound `GET /v1/mesh/message-status?peer_id=<uuid>&pool=<slug>&stream_seq=<n>`
//! - Inbound `LaneMessageKind::Msg` result frames to `role:service-escapement-pool`

use ep_domain::{
    CapabilitySlug, PoolDeliveryRefs, PoolMessageStatus, PoolResultOutcome, PoolResultRefs,
    MAX_SAFE_STRUCT_INT,
};

/// Request payload for dispatching a task to a Cortex capability pool (`pool:<slug>`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolSendRequest {
    /// Target capability pool slug.
    pub pool: CapabilitySlug,
    /// Short subject line for the mesh frame.
    pub subject: String,
    /// Markdown / JSON instruction body for the worker.
    pub body: String,
    /// Delivery correlation refs attached to the mesh envelope.
    pub refs: PoolDeliveryRefs,
}

/// Parsed response from `POST /v1/mesh/messages` when addressing `pool:<slug>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolSendResponse {
    /// Unique message UUID assigned by Cortex.
    pub message_id: String,
    /// Monotonic pool sequence number (`<= 2^53`).
    pub stream_seq: u64,
}

/// Parsed response from `GET /v1/mesh/message-status`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolStatusView {
    /// Current pool message status (`unread`, `pending`, `acked`, `dead_lettered`).
    pub status: PoolMessageStatus,
    /// Member role/UUID currently holding or having acked the message.
    pub member: Option<String>,
    /// Number of delivery attempts recorded by Cortex.
    pub deliveries: u32,
}

/// Serialize a [`PoolSendRequest`] into the JSON body expected by `POST /v1/mesh/messages`.
#[must_use]
pub fn serialize_pool_send_body(req: &PoolSendRequest) -> String {
    let recipient = req.pool.pool_address();
    let refs_json = match &req.refs.traceparent {
        Some(tp) => format!(
            r#"{{"task_id":"{}","attempt":{},"traceparent":"{}"}}"#,
            escape_json(&req.refs.task_id),
            req.refs.attempt,
            escape_json(tp)
        ),
        None => format!(
            r#"{{"task_id":"{}","attempt":{}}}"#,
            escape_json(&req.refs.task_id),
            req.refs.attempt
        ),
    };
    format!(
        r#"{{"recipient":"{}","kind":"msg","subject":"{}","body":"{}","refs":{}}}"#,
        escape_json(&recipient),
        escape_json(&req.subject),
        escape_json(&req.body),
        refs_json
    )
}

/// Parse the JSON response from `POST /v1/mesh/messages` for a pool send.
///
/// # Errors
/// Returns an error description if `id` or `stream_seq` is missing or exceeds `2^53`.
pub fn parse_pool_send_response(json: &str) -> Result<PoolSendResponse, String> {
    let message_id = parse_json_string(json, "id")
        .or_else(|| parse_json_string(json, "message_id"))
        .ok_or_else(|| "missing message id in pool send response".to_string())?;
    let stream_seq = parse_json_u64(json, "stream_seq")
        .ok_or_else(|| "missing stream_seq in pool send response".to_string())?;
    if stream_seq == 0 || stream_seq > MAX_SAFE_STRUCT_INT {
        return Err(format!("invalid stream_seq: {stream_seq}"));
    }
    Ok(PoolSendResponse {
        message_id,
        stream_seq,
    })
}

/// Parse the JSON response from `GET /v1/mesh/message-status`.
///
/// # Errors
/// Returns an error description if `status` is missing or unrecognized.
pub fn parse_pool_status_response(json: &str) -> Result<PoolStatusView, String> {
    let status_str = parse_json_string(json, "status")
        .ok_or_else(|| "missing status field in pool message-status response".to_string())?;
    let status = PoolMessageStatus::parse(&status_str)
        .ok_or_else(|| format!("unknown pool message status: {status_str}"))?;
    let member = parse_json_string(json, "member");
    let deliveries = parse_json_u64(json, "deliveries")
        .and_then(|d| u32::try_from(d).ok())
        .unwrap_or(0);
    Ok(PoolStatusView {
        status,
        member,
        deliveries,
    })
}

/// Parse inbound `PoolResultRefs` from a `service-escapement-pool` result JSON body.
///
/// # Errors
/// Returns an error description if any required field is missing or invalid.
pub fn parse_pool_result_refs(json: &str) -> Result<PoolResultRefs, String> {
    let stream_seq = parse_json_u64(json, "stream_seq")
        .ok_or_else(|| "missing stream_seq in pool result".to_string())?;
    let task_id = parse_json_string(json, "task_id")
        .ok_or_else(|| "missing task_id in pool result".to_string())?;
    let attempt = parse_json_u64(json, "attempt")
        .and_then(|a| u32::try_from(a).ok())
        .ok_or_else(|| "missing or invalid attempt in pool result".to_string())?;
    let outcome_str = parse_json_string(json, "outcome")
        .ok_or_else(|| "missing outcome in pool result".to_string())?;
    let outcome = PoolResultOutcome::parse(&outcome_str)
        .ok_or_else(|| format!("invalid outcome in pool result: {outcome_str}"))?;
    let error = parse_json_string(json, "error");
    let traceparent = parse_json_string(json, "traceparent");

    PoolResultRefs::new(stream_seq, task_id, attempt, outcome, error, traceparent)
        .map_err(|e| e.to_string())
}

fn escape_json(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

fn parse_json_string(json: &str, field: &str) -> Option<String> {
    let marker = format!("\"{field}\":\"");
    let start = json.find(&marker)?;
    let rest = &json[start + marker.len()..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn parse_json_u64(json: &str, field: &str) -> Option<u64> {
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

    #[test]
    fn serialize_pool_send_body_formats_recipient_and_refs() {
        let slug = CapabilitySlug::new("build").unwrap();
        let refs = PoolDeliveryRefs::new(
            slug.clone(),
            1,
            "t-42",
            1,
            Some("00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01".into()),
        )
        .unwrap();
        let req = PoolSendRequest {
            pool: slug,
            subject: "dispatch t-42".into(),
            body: "compile workspace".into(),
            refs,
        };
        let body = serialize_pool_send_body(&req);
        assert!(body.contains(r#""recipient":"pool:build""#));
        assert!(body.contains(r#""task_id":"t-42""#));
        assert!(body.contains(r#""attempt":1"#));
        assert!(body.contains(
            r#""traceparent":"00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01""#
        ));
    }

    #[test]
    fn parse_pool_send_response_enforces_ieee754_bound() {
        let ok_json = r#"{"id":"msg-1","stream_seq":42}"#;
        let parsed = parse_pool_send_response(ok_json).unwrap();
        assert_eq!(parsed.message_id, "msg-1");
        assert_eq!(parsed.stream_seq, 42);

        let overflow_json = r#"{"id":"msg-1","stream_seq":9007199254740993}"#;
        assert!(parse_pool_send_response(overflow_json).is_err());
    }

    #[test]
    fn parse_pool_status_response_parses_all_states() {
        let json = r#"{"pool":"build","stream_seq":7,"status":"dead_lettered","member":"w1","deliveries":3}"#;
        let view = parse_pool_status_response(json).unwrap();
        assert_eq!(view.status, PoolMessageStatus::DeadLettered);
        assert_eq!(view.member.as_deref(), Some("w1"));
        assert_eq!(view.deliveries, 3);
    }

    #[test]
    fn parse_pool_result_refs_validates_contract() {
        let json = r#"{"stream_seq":7,"task_id":"t-42","attempt":1,"outcome":"ok","traceparent":"00-11111111111111111111111111111111-2222222222222222-01"}"#;
        let res = parse_pool_result_refs(json).unwrap();
        assert_eq!(res.stream_seq, 7);
        assert_eq!(res.task_id, "t-42");
        assert_eq!(res.attempt, 1);
        assert_eq!(res.outcome, PoolResultOutcome::Ok);
    }
}
