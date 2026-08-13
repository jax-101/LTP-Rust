use std::io::{self, BufRead, Write};

use serde_json::Value;

use crate::mcp::dispatch::dispatch_tool;
use crate::mcp::tools::all_tools;
use crate::mcp::types::{
    InitializeResult, JsonRpcError, JsonRpcRequest, JsonRpcResponse, ServerCapabilities,
    ServerInfo, ToolCallParams, ToolCallResult, ToolsCapability, ToolsListResult,
};
use crate::workspace::FsStorage;

/// Run the MCP server loop, reading JSON-RPC from stdin and writing responses to stdout.
///
/// Returns when stdin reaches EOF (graceful shutdown).
pub fn run(storage: FsStorage) {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let response = handle_message(trimmed, &storage);

        if let Some(resp) = response {
            let json = serde_json::to_string(&resp).unwrap_or_else(|_| {
                r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32603,"message":"Internal serialization error"}}"#.to_string()
            });
            let _ = writeln!(stdout_lock, "{json}");
            let _ = stdout_lock.flush();
        }
    }
}

/// Process a single JSON-RPC message. Returns None for notifications (no id).
fn handle_message(raw: &str, storage: &FsStorage) -> Option<JsonRpcResponse> {
    let request: JsonRpcRequest = match serde_json::from_str(raw) {
        Ok(req) => req,
        Err(_) => {
            return Some(JsonRpcResponse::error(None, JsonRpcError::parse_error()));
        }
    };

    // Notifications (no id) don't get responses
    request.id.as_ref()?;

    let id = request.id;
    let response = dispatch_method(&request.method, &request.params, storage);

    match response {
        Ok(result) => Some(JsonRpcResponse::success(id, result)),
        Err(error) => Some(JsonRpcResponse::error(id, error)),
    }
}

/// Route a method to its handler.
fn dispatch_method(
    method: &str,
    params: &Value,
    storage: &FsStorage,
) -> Result<Value, JsonRpcError> {
    match method {
        "initialize" => handle_initialize(),
        "tools/list" => handle_tools_list(),
        "tools/call" => handle_tools_call(params, storage),
        _ => Err(JsonRpcError::method_not_found(method)),
    }
}

/// Handle `initialize` — return server capabilities.
fn handle_initialize() -> Result<Value, JsonRpcError> {
    let result = InitializeResult {
        protocol_version: "2024-11-05".to_string(),
        capabilities: ServerCapabilities {
            tools: ToolsCapability {
                list_changed: false,
            },
        },
        server_info: ServerInfo {
            name: "ltp-engine".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
    };
    serde_json::to_value(&result)
        .map_err(|e| JsonRpcError::invalid_params(&format!("serialization error: {e}")))
}

/// Handle `tools/list` — return all tool definitions.
fn handle_tools_list() -> Result<Value, JsonRpcError> {
    let result = ToolsListResult { tools: all_tools() };
    serde_json::to_value(&result)
        .map_err(|e| JsonRpcError::invalid_params(&format!("serialization error: {e}")))
}

/// Handle `tools/call` — dispatch to the appropriate tool.
fn handle_tools_call(params: &Value, storage: &FsStorage) -> Result<Value, JsonRpcError> {
    let call_params: ToolCallParams = serde_json::from_value(params.clone())
        .map_err(|e| JsonRpcError::invalid_params(&format!("invalid tools/call params: {e}")))?;

    let result: ToolCallResult = dispatch_tool(&call_params.name, &call_params.arguments, storage)?;

    serde_json::to_value(&result)
        .map_err(|e| JsonRpcError::invalid_params(&format!("serialization error: {e}")))
}
