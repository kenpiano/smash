//! DAP transport layer — Content-Length based message framing.

use crate::error::DapError;

/// Encode a JSON value into a DAP wire-format message with Content-Length header.
pub fn encode_message(value: &serde_json::Value) -> Vec<u8> {
    let body = serde_json::to_string(value).unwrap_or_default();
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    let mut buf = Vec::with_capacity(header.len() + body.len());
    buf.extend_from_slice(header.as_bytes());
    buf.extend_from_slice(body.as_bytes());
    buf
}

/// Decode a DAP wire-format message from a byte buffer.
///
/// Returns the parsed JSON value and the number of bytes consumed from the
/// buffer. If the buffer does not contain a complete message, returns a
/// `Transport` error.
pub fn decode_message(data: &[u8]) -> Result<(serde_json::Value, usize), DapError> {
    let data_str = std::str::from_utf8(data)
        .map_err(|e| DapError::Transport(format!("invalid UTF-8: {e}")))?;

    // Find the header/body separator.
    let separator = "\r\n\r\n";
    let sep_pos = data_str
        .find(separator)
        .ok_or_else(|| DapError::Transport("incomplete header: missing \\r\\n\\r\\n".into()))?;

    let header_part = &data_str[..sep_pos];
    let body_start = sep_pos + separator.len();

    // Parse Content-Length from header.
    let content_length = parse_content_length(header_part)?;

    // Check we have enough bytes for the body.
    let total_consumed = body_start + content_length;
    if data.len() < total_consumed {
        return Err(DapError::Transport(format!(
            "incomplete body: expected {content_length} bytes, have {}",
            data.len() - body_start
        )));
    }

    let body_bytes = &data[body_start..total_consumed];
    let value: serde_json::Value = serde_json::from_slice(body_bytes)
        .map_err(|e| DapError::InvalidResponse(format!("JSON parse error: {e}")))?;

    Ok((value, total_consumed))
}

/// Parse the Content-Length value from the header section.
fn parse_content_length(header: &str) -> Result<usize, DapError> {
    for line in header.split("\r\n") {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("Content-Length:") {
            let value = value.trim();
            return value.parse::<usize>().map_err(|e| {
                DapError::Transport(format!("invalid Content-Length value '{value}': {e}"))
            });
        }
    }
    Err(DapError::Transport("missing Content-Length header".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_serialize_request() {
        let req = serde_json::json!({
            "seq": 1,
            "type": "request",
            "command": "initialize",
            "arguments": {
                "adapterID": "lldb"
            }
        });
        let encoded = encode_message(&req);
        let s = String::from_utf8(encoded.clone()).unwrap();
        assert!(s.starts_with("Content-Length: "));
        assert!(s.contains("\r\n\r\n"));

        // Should be decodable.
        let (decoded, consumed) = decode_message(&encoded).unwrap();
        assert_eq!(decoded, req);
        assert_eq!(consumed, encoded.len());
    }

    #[test]
    fn transport_parse_response() {
        let resp = serde_json::json!({
            "seq": 2,
            "type": "response",
            "request_seq": 1,
            "success": true,
            "command": "initialize",
            "body": {}
        });
        let encoded = encode_message(&resp);
        let (decoded, consumed) = decode_message(&encoded).unwrap();
        assert_eq!(decoded, resp);
        assert_eq!(consumed, encoded.len());
    }

    #[test]
    fn transport_parse_event() {
        let evt = serde_json::json!({
            "seq": 3,
            "type": "event",
            "event": "stopped",
            "body": {
                "reason": "breakpoint",
                "threadId": 1
            }
        });
        let encoded = encode_message(&evt);
        let (decoded, consumed) = decode_message(&encoded).unwrap();
        assert_eq!(decoded, evt);
        assert_eq!(consumed, encoded.len());
    }

    #[test]
    fn transport_malformed_header() {
        let data = b"Bad-Header: 42\r\n\r\n{}";
        let result = decode_message(data);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("missing Content-Length"),
            "got: {err}"
        );
    }

    #[test]
    fn transport_incomplete_body() {
        // Header says 100 bytes, but body is short.
        let data = b"Content-Length: 100\r\n\r\n{\"short\":true}";
        let result = decode_message(data);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("incomplete body"), "got: {err}");
    }

    #[test]
    fn transport_multiple_messages() {
        let msg1 = serde_json::json!({"seq": 1, "type": "request", "command": "init"});
        let msg2 = serde_json::json!({"seq": 2, "type": "event", "event": "output"});

        let mut buf = encode_message(&msg1);
        let buf2 = encode_message(&msg2);
        buf.extend_from_slice(&buf2);

        // Decode first message.
        let (decoded1, consumed1) = decode_message(&buf).unwrap();
        assert_eq!(decoded1, msg1);

        // Decode second message from remaining bytes.
        let (decoded2, consumed2) = decode_message(&buf[consumed1..]).unwrap();
        assert_eq!(decoded2, msg2);
        assert_eq!(consumed1 + consumed2, buf.len());
    }

    #[test]
    fn transport_missing_separator() {
        let data = b"Content-Length: 2\r\n{}";
        let result = decode_message(data);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("incomplete header"), "got: {err}");
    }
}
