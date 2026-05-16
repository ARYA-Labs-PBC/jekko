//! MCP (Model Context Protocol) client shell.
//!
//! Ported from `packages/jekko/src/mcp/index.ts`. Like LSP this is a thin
//! JSON-RPC framing layer rather than a full client — the surface area is
//! still in flux upstream and the runtime only needs to know how to send
//! framed messages and decode the response envelope.

use serde::{Deserialize, Serialize};

use crate::error::RuntimeResult;

/// One MCP JSON-RPC request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest {
    /// JSON-RPC version.
    pub jsonrpc: String,
    /// Request id (string in MCP).
    pub id: String,
    /// Method name (e.g. `"tools/list"`).
    pub method: String,
    /// Free-form params.
    #[serde(default)]
    pub params: serde_json::Value,
}

/// One MCP JSON-RPC response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResponse {
    /// JSON-RPC version.
    pub jsonrpc: String,
    /// Request id.
    pub id: String,
    /// Result, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<serde_json::Value>,
}

/// Construct a new request envelope.
pub fn request(
    id: impl Into<String>,
    method: impl Into<String>,
    params: serde_json::Value,
) -> McpRequest {
    McpRequest {
        jsonrpc: "2.0".to_string(),
        id: id.into(),
        method: method.into(),
        params,
    }
}

/// Encode a request as newline-delimited JSON (the MCP stdio framing).
pub fn encode_request(req: &McpRequest) -> RuntimeResult<Vec<u8>> {
    let mut bytes = serde_json::to_vec(req)?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// Decode a response from one newline-delimited JSON record.
pub fn decode_response(line: &str) -> RuntimeResult<McpResponse> {
    Ok(serde_json::from_str(line)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_round_trip() {
        let req = request("1", "tools/list", serde_json::json!({}));
        let bytes = encode_request(&req).unwrap();
        let text = std::str::from_utf8(&bytes).unwrap().trim_end();
        let decoded: McpRequest = serde_json::from_str(text).unwrap();
        assert_eq!(decoded.method, "tools/list");
    }

    #[test]
    fn response_round_trip() {
        let line = r#"{"jsonrpc":"2.0","id":"1","result":{"tools":[]}}"#;
        let resp = decode_response(line).unwrap();
        assert!(resp.result.is_some());
    }
}
