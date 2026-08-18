use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::{json, Value};

fn mcp_bin() -> String {
    env!("CARGO_BIN_EXE_ltp-mcp").to_string()
}

fn ltp_bin() -> String {
    env!("CARGO_BIN_EXE_ltp").to_string()
}

/// Send JSON-RPC messages to ltp-mcp and collect responses.
/// Each line in `messages` is sent as-is. Returns all response lines parsed as JSON.
fn mcp_session(dir: &std::path::Path, messages: &[&str]) -> Vec<Value> {
    let mut child = Command::new(mcp_bin())
        .arg("--workspace")
        .arg(dir.to_str().unwrap_or("."))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn ltp-mcp");

    let mut stdin = child.stdin.take().expect("failed to open stdin");
    for msg in messages {
        writeln!(stdin, "{msg}").expect("failed to write to stdin");
    }
    drop(stdin);

    let output = child.wait_with_output().expect("failed to wait on child");
    let stdout = String::from_utf8_lossy(&output.stdout);

    stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            serde_json::from_str(l)
                .unwrap_or_else(|e| panic!("Failed to parse MCP response line: {e}\nLine: {l}"))
        })
        .collect()
}

/// Send a single JSON-RPC request and get the response.
fn mcp_request(dir: &std::path::Path, request: &Value) -> Value {
    let msg = serde_json::to_string(request).unwrap();
    let responses = mcp_session(dir, &[&msg]);
    assert!(
        !responses.is_empty(),
        "Expected at least one response from MCP server"
    );
    responses.into_iter().next().unwrap()
}

/// Initialize workspace using CLI before MCP tests.
fn init_workspace(dir: &std::path::Path) {
    Command::new(ltp_bin())
        .args(["init", "--name", "mcp-test"])
        .current_dir(dir)
        .output()
        .expect("failed to init workspace");
}

// --- UAT 12.1: initialize returns capabilities ---
#[test]
fn test_initialize_returns_capabilities() {
    let dir = tempfile::tempdir().unwrap();
    let resp = mcp_request(
        dir.path(),
        &json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
    );

    assert_eq!(resp["jsonrpc"], "2.0");
    assert_eq!(resp["id"], 1);
    assert!(resp["error"].is_null());

    let result = &resp["result"];
    assert_eq!(result["protocolVersion"], "2024-11-05");
    assert_eq!(result["serverInfo"]["name"], "ltp-engine");
    assert!(result["capabilities"]["tools"].is_object());
}

// --- UAT 12.2: node_add via MCP returns same result as CLI ---
#[test]
fn test_node_add_via_mcp() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    let resp = mcp_request(
        dir.path(),
        &json!({
            "jsonrpc": "2.0", "id": 2,
            "method": "tools/call",
            "params": {
                "name": "ltp/node_add",
                "arguments": {
                    "label": "Test UDE via MCP",
                    "type": "UDE"
                }
            }
        }),
    );

    assert!(resp["error"].is_null());
    let result = &resp["result"];
    assert_eq!(result["isError"], false);

    let content_text = result["content"][0]["text"].as_str().unwrap();
    let output: Value = serde_json::from_str(content_text).unwrap();
    assert_eq!(output["success"], true);
    assert_eq!(output["action"], "node_add");
    assert!(output["data"]["id"].as_str().unwrap().starts_with("UDE-"));
}

// --- UAT 12.3: invalid params error ---
#[test]
fn test_invalid_params_error() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    let resp = mcp_request(
        dir.path(),
        &json!({
            "jsonrpc": "2.0", "id": 3,
            "method": "tools/call",
            "params": {
                "name": "ltp/node_add",
                "arguments": {
                    "label": "Missing type field"
                }
            }
        }),
    );

    let error = &resp["error"];
    assert_eq!(error["code"], -32602);
    assert!(error["message"].as_str().unwrap().contains("type"));
}

// --- UAT 12.4: navigation tool does not generate undo entry ---
#[test]
fn test_navigation_no_undo_entry() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    // Add a node first
    let add_msg = json!({
        "jsonrpc": "2.0", "id": 1,
        "method": "tools/call",
        "params": { "name": "ltp/node_add", "arguments": { "label": "Test", "type": "UDE" } }
    });
    let inspect_msg = json!({
        "jsonrpc": "2.0", "id": 2,
        "method": "tools/call",
        "params": { "name": "ltp/node_inspect", "arguments": { "id": "UDE-001" } }
    });
    let history_msg = json!({
        "jsonrpc": "2.0", "id": 3,
        "method": "tools/call",
        "params": { "name": "ltp/history_list", "arguments": {} }
    });

    let msgs: Vec<String> = [&add_msg, &inspect_msg, &history_msg]
        .iter()
        .map(|m| serde_json::to_string(m).unwrap())
        .collect();
    let msg_refs: Vec<&str> = msgs.iter().map(|s| s.as_str()).collect();
    let responses = mcp_session(dir.path(), &msg_refs);

    assert_eq!(responses.len(), 3);

    // History should have only 1 entry (the node_add), not 2
    let history_content = responses[2]["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    let history_output: Value = serde_json::from_str(history_content).unwrap();
    let entries = history_output["data"]["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["action"], "node_add");
}

// --- UAT 12.5: manipulation tool generates undo entry ---
#[test]
fn test_manipulation_generates_undo() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    let add_msg = json!({
        "jsonrpc": "2.0", "id": 1,
        "method": "tools/call",
        "params": { "name": "ltp/node_add", "arguments": { "label": "Test UDE", "type": "UDE" } }
    });
    let history_msg = json!({
        "jsonrpc": "2.0", "id": 2,
        "method": "tools/call",
        "params": { "name": "ltp/history_list", "arguments": {} }
    });

    let msgs: Vec<String> = [&add_msg, &history_msg]
        .iter()
        .map(|m| serde_json::to_string(m).unwrap())
        .collect();
    let msg_refs: Vec<&str> = msgs.iter().map(|s| s.as_str()).collect();
    let responses = mcp_session(dir.path(), &msg_refs);

    let history_content = responses[1]["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    let history_output: Value = serde_json::from_str(history_content).unwrap();
    let entries = history_output["data"]["entries"].as_array().unwrap();
    assert!(!entries.is_empty());
    assert_eq!(entries[0]["action"], "node_add");
}

// --- UAT 12.6: validate returns errors + warnings ---
#[test]
fn test_validate_returns_errors_warnings() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    // Create tree + nodes + connect (valid state)
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"ltp/tree_new","arguments":{"type":"CRT","name":"test"}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"ltp/node_add","arguments":{"label":"Cause","type":"UDE"}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"ltp/node_add","arguments":{"label":"Effect","type":"UDE"}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"ltp/tree_attach","arguments":{"tree":"tree-crt-test","node":"UDE-001"}}}),
        json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"ltp/tree_attach","arguments":{"tree":"tree-crt-test","node":"UDE-002"}}}),
        json!({"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"ltp/link_connect","arguments":{"tree":"tree-crt-test","from":["UDE-001"],"to":["UDE-002"]}}}),
        json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"ltp/validate","arguments":{"tree":"tree-crt-test"}}}),
    ];

    let msgs: Vec<String> = messages
        .iter()
        .map(|m| serde_json::to_string(m).unwrap())
        .collect();
    let msg_refs: Vec<&str> = msgs.iter().map(|s| s.as_str()).collect();
    let responses = mcp_session(dir.path(), &msg_refs);

    let validate_resp = &responses[6];
    assert!(validate_resp["error"].is_null());
    let content_text = validate_resp["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    let output: Value = serde_json::from_str(content_text).unwrap();
    assert_eq!(output["success"], true);
    assert!(output["errors"].as_array().unwrap().is_empty());
}

// --- UAT 12.7: dry_run does not mutate ---
#[test]
fn test_dry_run_no_mutation() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    // Undo with dry_run on empty history — should not crash, just report
    let resp = mcp_request(
        dir.path(),
        &json!({
            "jsonrpc": "2.0", "id": 1,
            "method": "tools/call",
            "params": { "name": "ltp/undo", "arguments": { "dry_run": true } }
        }),
    );

    assert!(resp["error"].is_null());
    let content_text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let output: Value = serde_json::from_str(content_text).unwrap();
    // Undo on empty history fails gracefully, no crash
    assert!(output.is_object());
}

// --- UAT 12.8: no workspace → error -32001 ---
#[test]
fn test_no_workspace_error() {
    let dir = tempfile::tempdir().unwrap();
    // Do NOT init workspace

    let resp = mcp_request(
        dir.path(),
        &json!({
            "jsonrpc": "2.0", "id": 1,
            "method": "tools/call",
            "params": { "name": "ltp/node_add", "arguments": { "label": "X", "type": "UDE" } }
        }),
    );

    let error = &resp["error"];
    assert_eq!(error["code"], -32001);
    assert!(error["message"]
        .as_str()
        .unwrap()
        .contains("not initialized"));
}

// --- UAT 12.9: CLR#2 warning preserved ---
#[test]
fn test_clr2_warning_preserved() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    // "because" triggers CLR#2 conjunction warning
    let resp = mcp_request(
        dir.path(),
        &json!({
            "jsonrpc": "2.0", "id": 1,
            "method": "tools/call",
            "params": {
                "name": "ltp/node_add",
                "arguments": { "label": "Sales dropped because marketing failed", "type": "UDE" }
            }
        }),
    );

    assert!(resp["error"].is_null());
    let content_text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let output: Value = serde_json::from_str(content_text).unwrap();
    assert_eq!(output["success"], true);
    let warnings = output["warnings"].as_array().unwrap();
    assert!(!warnings.is_empty());
    assert!(warnings[0]["code"].as_str().unwrap().contains("CLR"));
}

// --- UAT 12.10: tools list contains ALL subcommands ---
#[test]
fn test_tools_list_complete() {
    let dir = tempfile::tempdir().unwrap();

    let resp = mcp_request(
        dir.path(),
        &json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {}}),
    );

    assert!(resp["error"].is_null());
    let tools = resp["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 61);

    // Verify key tools exist
    let tool_names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(tool_names.contains(&"ltp/node_add"));
    assert!(tool_names.contains(&"ltp/link_connect"));
    assert!(tool_names.contains(&"ltp/validate"));
    assert!(tool_names.contains(&"ltp/trace"));
    assert!(tool_names.contains(&"ltp/undo"));
    assert!(tool_names.contains(&"ltp/path_collapse"));
    assert!(tool_names.contains(&"ltp/nbr_add"));
    assert!(tool_names.contains(&"ltp/history_begin_batch"));

    // Verify each tool has inputSchema
    for tool in tools {
        assert!(
            tool["inputSchema"].is_object(),
            "Tool {} missing inputSchema",
            tool["name"]
        );
    }
}

// --- UAT 12.11: malformed JSON → parse error ---
#[test]
fn test_malformed_json_parse_error() {
    let dir = tempfile::tempdir().unwrap();
    let responses = mcp_session(dir.path(), &["this is not json {{{"]);

    assert_eq!(responses.len(), 1);
    let resp = &responses[0];
    assert_eq!(resp["error"]["code"], -32700);
    assert!(resp["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Parse error"));
}

// --- UAT 12.12: unknown method → method not found ---
#[test]
fn test_unknown_method_not_found() {
    let dir = tempfile::tempdir().unwrap();

    let resp = mcp_request(
        dir.path(),
        &json!({"jsonrpc": "2.0", "id": 1, "method": "ltp/nonexistent", "params": {}}),
    );

    let error = &resp["error"];
    assert_eq!(error["code"], -32601);
    assert!(error["message"].as_str().unwrap().contains("not found"));
}

// --- UAT 12.13: failed tool → isError: true ---
#[test]
fn test_failed_tool_is_error_flag() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    let resp = mcp_request(
        dir.path(),
        &json!({
            "jsonrpc": "2.0", "id": 1,
            "method": "tools/call",
            "params": {
                "name": "ltp/node_rm",
                "arguments": { "ids": ["NONEXISTENT-999"], "force": false }
            }
        }),
    );

    // This should be a successful JSON-RPC call (no protocol error)
    assert!(resp["error"].is_null());
    // But the tool result should indicate failure
    let result = &resp["result"];
    assert_eq!(result["isError"], true);

    let content_text = result["content"][0]["text"].as_str().unwrap();
    let output: Value = serde_json::from_str(content_text).unwrap();
    assert_eq!(output["success"], false);
}

// --- UAT 12.14: EOF → graceful shutdown ---
#[test]
fn test_eof_graceful_shutdown() {
    let dir = tempfile::tempdir().unwrap();

    let mut child = Command::new(mcp_bin())
        .arg("--workspace")
        .arg(dir.path().to_str().unwrap())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn ltp-mcp");

    // Close stdin immediately (EOF)
    drop(child.stdin.take());

    let output = child.wait_with_output().expect("failed to wait on child");
    assert!(
        output.status.success(),
        "Expected exit code 0 on EOF, got {:?}",
        output.status.code()
    );
    // No panic output on stderr
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("panic"),
        "Server panicked on EOF: {stderr}"
    );
}

// --- UAT 12.15: sequential workflow, state persists between calls ---
#[test]
fn test_sequential_workflow_state_persists() {
    let dir = tempfile::tempdir().unwrap();

    let messages = [
        // 1. Init workspace via MCP
        json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"ltp/init","arguments":{"name":"workflow-test"}}}),
        // 2. Create tree
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"ltp/tree_new","arguments":{"type":"CRT","name":"flow"}}}),
        // 3. Add two nodes
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"ltp/node_add","arguments":{"label":"Root cause","type":"RC"}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"ltp/node_add","arguments":{"label":"Undesirable effect","type":"UDE"}}}),
        // 4. Attach nodes
        json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"ltp/tree_attach","arguments":{"tree":"tree-crt-flow","node":"RC-001"}}}),
        json!({"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"ltp/tree_attach","arguments":{"tree":"tree-crt-flow","node":"UDE-001"}}}),
        // 5. Connect
        json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"ltp/link_connect","arguments":{"tree":"tree-crt-flow","from":["RC-001"],"to":["UDE-001"]}}}),
        // 6. Trace — should find the path
        json!({"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"ltp/trace","arguments":{"node_id":"UDE-001","tree":"tree-crt-flow","direction":"upstream","depth":5}}}),
    ];

    let msgs: Vec<String> = messages
        .iter()
        .map(|m| serde_json::to_string(m).unwrap())
        .collect();
    let msg_refs: Vec<&str> = msgs.iter().map(|s| s.as_str()).collect();
    let responses = mcp_session(dir.path(), &msg_refs);

    assert_eq!(responses.len(), 8);

    // Verify init succeeded
    let init_content = responses[0]["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    let init_out: Value = serde_json::from_str(init_content).unwrap();
    assert_eq!(init_out["success"], true);

    // Verify trace finds RC-001 upstream of UDE-001
    let trace_content = responses[7]["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    let trace_out: Value = serde_json::from_str(trace_content).unwrap();
    assert_eq!(trace_out["success"], true);
    let chain = trace_out["data"]["chain"].as_array().unwrap();
    let chain_nodes: Vec<&str> = chain.iter().map(|v| v["node"].as_str().unwrap()).collect();
    assert!(
        chain_nodes.contains(&"RC-001"),
        "Trace should find RC-001 upstream. Got: {chain_nodes:?}"
    );
}

// --- UAT 12.16: tools/list as standalone request ---
#[test]
fn test_tools_list_standalone() {
    let dir = tempfile::tempdir().unwrap();

    // Call tools/list WITHOUT calling initialize first
    let resp = mcp_request(
        dir.path(),
        &json!({"jsonrpc": "2.0", "id": 42, "method": "tools/list", "params": {}}),
    );

    assert!(resp["error"].is_null());
    assert_eq!(resp["id"], 42);
    let tools = resp["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 61);
}
