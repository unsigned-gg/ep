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

/// Pluggable transport driver for Cortex mesh capability pools (`pool:<slug>`).
pub trait CortexPoolDriver {
    /// Dispatch a message to `pool:<slug>` (`POST /v1/mesh/messages`).
    ///
    /// # Errors
    /// Returns an error description if the transport fails or the response violates the contract.
    fn send_pool(&mut self, req: &PoolSendRequest) -> Result<PoolSendResponse, String>;

    /// Query delivery status (`GET /v1/mesh/message-status?peer_id=<uuid>&pool=<slug>&stream_seq=<n>`).
    ///
    /// # Errors
    /// Returns an error description if the transport fails or the status payload is malformed.
    fn message_status(
        &self,
        peer_id: &str,
        pool: &CapabilitySlug,
        stream_seq: u64,
    ) -> Result<PoolStatusView, String>;
}

/// Deterministic in-memory [`CortexPoolDriver`] for unit tests and dry-run simulations.
#[derive(Debug, Default, Clone)]
pub struct InMemoryCortexPoolDriver {
    next_seq: u64,
    sent: Vec<PoolSendRequest>,
    statuses: std::collections::BTreeMap<(String, u64), PoolStatusView>,
}

impl InMemoryCortexPoolDriver {
    /// Create a new in-memory driver starting at `stream_seq = 1`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            next_seq: 1,
            sent: Vec::new(),
            statuses: std::collections::BTreeMap::new(),
        }
    }

    /// Return all requests dispatched through this driver.
    #[must_use]
    pub fn sent_requests(&self) -> &[PoolSendRequest] {
        &self.sent
    }

    /// Override the status returned for `(pool_slug, stream_seq)`.
    pub fn set_status(&mut self, pool: &CapabilitySlug, stream_seq: u64, view: PoolStatusView) {
        self.statuses
            .insert((pool.as_str().to_string(), stream_seq), view);
    }
}

impl CortexPoolDriver for InMemoryCortexPoolDriver {
    fn send_pool(&mut self, req: &PoolSendRequest) -> Result<PoolSendResponse, String> {
        if self.next_seq == 0 {
            self.next_seq = 1;
        }
        let seq = self.next_seq;
        if seq > MAX_SAFE_STRUCT_INT {
            return Err(format!("stream_seq overflow: {seq}"));
        }
        self.next_seq += 1;
        self.sent.push(req.clone());
        self.statuses.insert(
            (req.pool.as_str().to_string(), seq),
            PoolStatusView {
                status: PoolMessageStatus::Unread,
                member: None,
                deliveries: 1,
            },
        );
        Ok(PoolSendResponse {
            message_id: format!("mem-msg-{seq}"),
            stream_seq: seq,
        })
    }

    fn message_status(
        &self,
        _peer_id: &str,
        pool: &CapabilitySlug,
        stream_seq: u64,
    ) -> Result<PoolStatusView, String> {
        self.statuses
            .get(&(pool.as_str().to_string(), stream_seq))
            .cloned()
            .ok_or_else(|| {
                format!(
                    "no pool message status for {}:{}",
                    pool.as_str(),
                    stream_seq
                )
            })
    }
}

/// Live HTTP/1.1 [`CortexPoolDriver`] backed by `std::net::TcpStream`.
#[derive(Debug, Clone)]
pub struct HttpCortexPoolDriver {
    endpoint_addr: String,
    bearer_token: Option<String>,
    timeout_ms: u64,
}

impl HttpCortexPoolDriver {
    /// Create a live HTTP driver targeting `endpoint_addr` (e.g. `"127.0.0.1:9400"` or `"http://127.0.0.1:9400"`).
    #[must_use]
    pub fn new(
        endpoint_addr: impl Into<String>,
        bearer_token: Option<String>,
        timeout_ms: u64,
    ) -> Self {
        let raw = endpoint_addr.into();
        let stripped = raw
            .trim_start_matches("http://")
            .trim_start_matches("https://")
            .trim_end_matches('/')
            .to_string();
        Self {
            endpoint_addr: stripped,
            bearer_token,
            timeout_ms: timeout_ms.clamp(50, 30_000),
        }
    }

    fn request(&self, method: &str, path: &str, body: Option<&str>) -> Result<String, String> {
        use std::io::{Read, Write};
        use std::net::{TcpStream, ToSocketAddrs};
        use std::time::Duration;

        let timeout = Duration::from_millis(self.timeout_ms);
        let addr = self
            .endpoint_addr
            .to_socket_addrs()
            .map_err(|e| format!("resolve {}: {e}", self.endpoint_addr))?
            .next()
            .ok_or_else(|| format!("no socket address for {}", self.endpoint_addr))?;

        let mut stream =
            TcpStream::connect_timeout(&addr, timeout).map_err(|e| format!("connect: {e}"))?;
        let _ = stream.set_read_timeout(Some(timeout));
        let _ = stream.set_write_timeout(Some(timeout));

        let auth_header = match &self.bearer_token {
            Some(tok) if !tok.is_empty() => format!("Authorization: Bearer {tok}\r\n"),
            _ => String::new(),
        };
        let payload = body.unwrap_or("");
        let http_req = format!(
            "{method} {path} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\n{auth_header}Content-Length: {}\r\nConnection: close\r\n\r\n{payload}",
            self.endpoint_addr,
            payload.len()
        );

        stream
            .write_all(http_req.as_bytes())
            .map_err(|e| format!("write: {e}"))?;
        stream.flush().map_err(|e| format!("flush: {e}"))?;

        let mut raw_resp = String::new();
        stream
            .read_to_string(&mut raw_resp)
            .map_err(|e| format!("read: {e}"))?;

        let (head, resp_body) = raw_resp
            .split_once("\r\n\r\n")
            .ok_or_else(|| "malformed HTTP response from Cortex".to_string())?;
        let status_line = head.lines().next().unwrap_or("");
        let status_code: u16 = status_line
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(500);

        if !(200..300).contains(&status_code) {
            return Err(format!("cortex HTTP {status_code}: {resp_body}"));
        }
        Ok(resp_body.to_string())
    }
}

impl CortexPoolDriver for HttpCortexPoolDriver {
    fn send_pool(&mut self, req: &PoolSendRequest) -> Result<PoolSendResponse, String> {
        let json_body = serialize_pool_send_body(req);
        let resp = self.request("POST", "/v1/mesh/messages", Some(&json_body))?;
        parse_pool_send_response(&resp)
    }

    fn message_status(
        &self,
        peer_id: &str,
        pool: &CapabilitySlug,
        stream_seq: u64,
    ) -> Result<PoolStatusView, String> {
        let path = format!(
            "/v1/mesh/message-status?peer_id={peer_id}&pool={}&stream_seq={stream_seq}",
            pool.as_str()
        );
        let resp = self.request("GET", &path, None)?;
        parse_pool_status_response(&resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::thread;

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

    #[test]
    fn http_cortex_pool_driver_round_trips_send_and_status_over_tcp() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let server = thread::spawn(move || {
            for idx in 0..2 {
                let (mut stream, _) = listener.accept().unwrap();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut req_line = String::new();
                reader.read_line(&mut req_line).unwrap();
                let mut content_len = 0usize;
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line.trim().is_empty() {
                        break;
                    }
                    if let Some(rest) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                        content_len = rest.trim().parse().unwrap_or(0);
                    }
                }
                if content_len > 0 {
                    let mut buf = vec![0u8; content_len];
                    reader.read_exact(&mut buf).unwrap();
                }

                let body = if idx == 0 {
                    assert!(req_line.starts_with("POST /v1/mesh/messages "));
                    r#"{"id":"cortex-uuid-99","stream_seq":99}"#
                } else {
                    assert!(req_line.starts_with(
                        "GET /v1/mesh/message-status?peer_id=peer-1&pool=build&stream_seq=99 "
                    ));
                    r#"{"pool":"build","stream_seq":99,"status":"pending","member":"rev-worker-1","deliveries":1}"#
                };
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(resp.as_bytes()).unwrap();
                stream.flush().unwrap();
            }
        });

        let mut driver = HttpCortexPoolDriver::new(addr.to_string(), Some("tok-test".into()), 2000);
        let slug = CapabilitySlug::new("build").unwrap();
        let refs = PoolDeliveryRefs::new(slug.clone(), 1, "t-99", 1, None).unwrap();
        let send_resp = driver
            .send_pool(&PoolSendRequest {
                pool: slug.clone(),
                subject: "dispatch t-99".into(),
                body: "cargo test -j 2".into(),
                refs,
            })
            .unwrap();
        assert_eq!(send_resp.message_id, "cortex-uuid-99");
        assert_eq!(send_resp.stream_seq, 99);

        let status_view = driver.message_status("peer-1", &slug, 99).unwrap();
        assert_eq!(status_view.status, PoolMessageStatus::Pending);
        assert_eq!(status_view.member.as_deref(), Some("rev-worker-1"));
        assert_eq!(status_view.deliveries, 1);

        server.join().unwrap();
    }
}
