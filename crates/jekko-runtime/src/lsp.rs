//! LSP client shell.
//!
//! Ported from `packages/jekko/src/lsp/`. Full LSP wiring is intentionally
//! deferred; this module gives downstream callers a stable trait + a thin
//! JSON-RPC framing helper they can build the real client on top of when
//! the rest of the runtime is in place.

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::error::{RuntimeError, RuntimeResult};

/// Minimal LSP request envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspRequest {
    /// JSON-RPC version (always `"2.0"`).
    pub jsonrpc: String,
    /// Request id.
    pub id: u64,
    /// LSP method name.
    pub method: String,
    /// Free-form params.
    pub params: serde_json::Value,
}

/// Minimal LSP response envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspResponse {
    /// JSON-RPC version.
    pub jsonrpc: String,
    /// Request id.
    pub id: u64,
    /// Result, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<serde_json::Value>,
}

/// Frame a JSON-RPC message in LSP's `Content-Length`-prefixed framing.
pub fn frame_message(message: &serde_json::Value) -> RuntimeResult<Vec<u8>> {
    let body = serde_json::to_vec(message)?;
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    let mut out = Vec::with_capacity(header.len() + body.len());
    out.extend_from_slice(header.as_bytes());
    out.extend_from_slice(&body);
    Ok(out)
}

/// Read one LSP-framed message from an async reader.
pub async fn read_message<R: AsyncReadExt + Unpin>(
    reader: &mut R,
) -> RuntimeResult<serde_json::Value> {
    let mut header = Vec::new();
    let mut byte = [0u8; 1];
    while !header.ends_with(b"\r\n\r\n") {
        let n = reader.read(&mut byte).await?;
        if n == 0 {
            return Err(RuntimeError::other("eof reading lsp header"));
        }
        header.push(byte[0]);
    }
    let header_str = String::from_utf8_lossy(&header);
    let mut len: usize = 0;
    for line in header_str.split("\r\n") {
        if let Some(value) = line.strip_prefix("Content-Length: ") {
            len = value
                .trim()
                .parse()
                .map_err(|err: std::num::ParseIntError| RuntimeError::other(err.to_string()))?;
        }
    }
    let mut body = vec![0u8; len];
    reader.read_exact(&mut body).await?;
    let value: serde_json::Value = serde_json::from_slice(&body)?;
    Ok(value)
}

/// Write one LSP-framed message to an async writer.
pub async fn write_message<W: AsyncWriteExt + Unpin>(
    writer: &mut W,
    message: &serde_json::Value,
) -> RuntimeResult<()> {
    let bytes = frame_message(message)?;
    writer.write_all(&bytes).await?;
    writer.flush().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn round_trip_framed_message() {
        let msg = serde_json::json!({ "jsonrpc": "2.0", "id": 1, "method": "ping" });
        let bytes = frame_message(&msg).unwrap();
        let mut reader = &bytes[..];
        let back = read_message(&mut reader).await.unwrap();
        assert_eq!(back, msg);
    }
}
