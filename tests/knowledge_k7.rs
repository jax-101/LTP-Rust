use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::{json, Value};

fn mcp_bin() -> String {
    env!("CARGO_BIN_EXE_ltp-mcp").to_string()
}

fn ltp_bin() -> String {
    env!("CARGO_BIN_EXE_ltp").to_string()
}

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

fn mcp_request(dir: &std::path::Path, request: &Value) -> Value {
    let msg = serde_json::to_string(request).unwrap();
    let responses = mcp_session(dir, &[&msg]);
    assert!(
        !responses.is_empty(),
        "Expected at least one response from MCP server"
    );
    responses.into_iter().next().unwrap()
}

fn mcp_call(dir: &std::path::Path, tool: &str, arguments: Value) -> Value {
    mcp_request(
        dir,
        &json!({
            "jsonrpc": "2.0", "id": 1,
            "method": "tools/call",
            "params": { "name": tool, "arguments": arguments }
        }),
    )
}

fn get_tool_content(resp: &Value) -> Value {
    let content = &resp["result"]["content"][0]["text"];
    serde_json::from_str(content.as_str().unwrap()).unwrap()
}

fn init_workspace(dir: &std::path::Path) {
    Command::new(ltp_bin())
        .args(["init", "--name", "mcp-k7-test"])
        .current_dir(dir)
        .output()
        .expect("failed to init workspace");
}

fn add_node(dir: &std::path::Path, label: &str, node_type: &str) -> Value {
    let resp = mcp_call(
        dir,
        "ltp/node_add",
        json!({"label": label, "type": node_type}),
    );
    get_tool_content(&resp)
}

// --- K7.1: tools/list includes 7 new knowledge tools ---
#[test]
fn k7_01_tools_list_includes_knowledge_tools() {
    let dir = tempfile::tempdir().unwrap();
    let resp = mcp_request(
        dir.path(),
        &json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {}}),
    );

    let tools = resp["result"]["tools"].as_array().unwrap();
    let tool_names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();

    assert!(tool_names.contains(&"ltp/knowledge_add"));
    assert!(tool_names.contains(&"ltp/knowledge_edit"));
    assert!(tool_names.contains(&"ltp/knowledge_rm"));
    assert!(tool_names.contains(&"ltp/knowledge_inspect"));
    assert!(tool_names.contains(&"ltp/knowledge_list"));
    assert!(tool_names.contains(&"ltp/knowledge_link"));
    assert!(tool_names.contains(&"ltp/knowledge_unlink"));
}

// --- K7.2: each tool has inputSchema with documented params ---
#[test]
fn k7_02_tools_have_input_schema() {
    let dir = tempfile::tempdir().unwrap();
    let resp = mcp_request(
        dir.path(),
        &json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {}}),
    );

    let tools = resp["result"]["tools"].as_array().unwrap();

    let knowledge_add = tools
        .iter()
        .find(|t| t["name"] == "ltp/knowledge_add")
        .unwrap();
    let schema = &knowledge_add["inputSchema"];
    assert_eq!(schema["type"], "object");
    assert!(schema["properties"]["label"].is_object());
    assert!(schema["properties"]["type"].is_object());
    assert!(schema["properties"]["source_uri"].is_object());
    assert!(schema["properties"]["source_excerpt"].is_object());
    assert!(schema["properties"]["status"].is_object());
    assert!(schema["properties"]["confidence"].is_object());
    assert!(schema["properties"]["tags"].is_object());

    let knowledge_link = tools
        .iter()
        .find(|t| t["name"] == "ltp/knowledge_link")
        .unwrap();
    let schema = &knowledge_link["inputSchema"];
    let required = schema["required"].as_array().unwrap();
    let required_strs: Vec<&str> = required.iter().map(|r| r.as_str().unwrap()).collect();
    assert!(required_strs.contains(&"id"));
    assert!(required_strs.contains(&"target"));
    assert!(required_strs.contains(&"relation"));
}

// --- K7.3: knowledge_add with valid params ---
#[test]
fn k7_03_knowledge_add_valid() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    let resp = mcp_call(
        dir.path(),
        "ltp/knowledge_add",
        json!({
            "label": "Media 18.3 dias",
            "type": "measurement",
            "source_excerpt": "ERP Q2",
            "status": "verified",
            "confidence": "high"
        }),
    );

    let content = get_tool_content(&resp);
    assert_eq!(content["success"], true);
    assert_eq!(content["data"]["created_knowledge_id"], "KN-001");
    assert_eq!(content["action"], "knowledge_add");
}

// --- K7.4: knowledge_add without source ---
#[test]
fn k7_04_knowledge_add_no_source() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    let resp = mcp_call(
        dir.path(),
        "ltp/knowledge_add",
        json!({
            "label": "Sin fuente",
            "type": "observation"
        }),
    );

    let content = get_tool_content(&resp);
    assert_eq!(content["success"], false);
    assert_eq!(content["errors"][0]["code"], "SOURCE_REQUIRED");
    assert_eq!(resp["result"]["isError"], true);
}

// --- K7.5: knowledge_add with empty label ---
#[test]
fn k7_05_knowledge_add_empty_label() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    let resp = mcp_call(
        dir.path(),
        "ltp/knowledge_add",
        json!({
            "label": "",
            "type": "measurement",
            "source_excerpt": "x"
        }),
    );

    let content = get_tool_content(&resp);
    assert_eq!(content["success"], false);
    assert_eq!(content["errors"][0]["code"], "LABEL_REQUIRED");
}

// --- K7.6: knowledge_add with invalid type ---
#[test]
fn k7_06_knowledge_add_invalid_type() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    let resp = mcp_call(
        dir.path(),
        "ltp/knowledge_add",
        json!({
            "label": "Test",
            "type": "INVALID_TYPE",
            "source_excerpt": "x"
        }),
    );

    // Should be a JSON-RPC error (invalid params)
    assert!(resp["error"].is_object());
    assert_eq!(resp["error"]["code"], -32602);
}

// --- K7.7: knowledge_edit updates status ---
#[test]
fn k7_07_knowledge_edit_status() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    mcp_call(
        dir.path(),
        "ltp/knowledge_add",
        json!({
            "label": "Test item",
            "type": "observation",
            "source_excerpt": "data"
        }),
    );

    let resp = mcp_call(
        dir.path(),
        "ltp/knowledge_edit",
        json!({
            "id": "KN-001",
            "status": "verified"
        }),
    );

    let content = get_tool_content(&resp);
    assert_eq!(content["success"], true);
    assert_eq!(content["data"]["status"], "verified");
}

// --- K7.8: knowledge_edit nonexistent ---
#[test]
fn k7_08_knowledge_edit_not_found() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    let resp = mcp_call(
        dir.path(),
        "ltp/knowledge_edit",
        json!({
            "id": "KN-999",
            "label": "new"
        }),
    );

    let content = get_tool_content(&resp);
    assert_eq!(content["success"], false);
    assert_eq!(content["errors"][0]["code"], "KNOWLEDGE_NOT_FOUND");
    assert_eq!(resp["result"]["isError"], true);
}

// --- K7.9: knowledge_rm removes item ---
#[test]
fn k7_09_knowledge_rm() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    mcp_call(
        dir.path(),
        "ltp/knowledge_add",
        json!({"label": "To remove", "type": "observation", "source_excerpt": "x"}),
    );

    let resp = mcp_call(dir.path(), "ltp/knowledge_rm", json!({"ids": "KN-001"}));
    let content = get_tool_content(&resp);
    assert_eq!(content["success"], true);
    assert!(content["data"]["removed"]
        .as_array()
        .unwrap()
        .contains(&json!("KN-001")));
}

// --- K7.10: knowledge_rm nonexistent ---
#[test]
fn k7_10_knowledge_rm_not_found() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    let resp = mcp_call(dir.path(), "ltp/knowledge_rm", json!({"ids": "KN-999"}));
    let content = get_tool_content(&resp);
    assert_eq!(content["success"], false);
    assert_eq!(content["errors"][0]["code"], "KNOWLEDGE_NOT_FOUND");
}

// --- K7.11: knowledge_link valid target ---
#[test]
fn k7_11_knowledge_link_valid() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    add_node(dir.path(), "Test UDE", "UDE");
    mcp_call(
        dir.path(),
        "ltp/knowledge_add",
        json!({"label": "Evidence", "type": "measurement", "source_excerpt": "data"}),
    );

    let resp = mcp_call(
        dir.path(),
        "ltp/knowledge_link",
        json!({"id": "KN-001", "target": "UDE-001", "relation": "supports"}),
    );

    let content = get_tool_content(&resp);
    assert_eq!(content["success"], true);
    assert_eq!(content["data"]["target"], "UDE-001");
    assert_eq!(content["data"]["relation"], "supports");
    assert_eq!(content["data"]["link_count"], 1);
}

// --- K7.12: knowledge_link nonexistent target ---
#[test]
fn k7_12_knowledge_link_target_not_found() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    mcp_call(
        dir.path(),
        "ltp/knowledge_add",
        json!({"label": "Evidence", "type": "measurement", "source_excerpt": "data"}),
    );

    let resp = mcp_call(
        dir.path(),
        "ltp/knowledge_link",
        json!({"id": "KN-001", "target": "UDE-999", "relation": "supports"}),
    );

    let content = get_tool_content(&resp);
    assert_eq!(content["success"], false);
    assert_eq!(content["errors"][0]["code"], "TARGET_NOT_FOUND");
}

// --- K7.13: knowledge_link duplicate is idempotent ---
#[test]
fn k7_13_knowledge_link_duplicate() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    add_node(dir.path(), "Test UDE", "UDE");
    mcp_call(
        dir.path(),
        "ltp/knowledge_add",
        json!({"label": "Evidence", "type": "measurement", "source_excerpt": "data"}),
    );

    mcp_call(
        dir.path(),
        "ltp/knowledge_link",
        json!({"id": "KN-001", "target": "UDE-001", "relation": "supports"}),
    );

    let resp = mcp_call(
        dir.path(),
        "ltp/knowledge_link",
        json!({"id": "KN-001", "target": "UDE-001", "relation": "supports"}),
    );

    let content = get_tool_content(&resp);
    assert_eq!(content["success"], true);
    let has_dup_warning = content["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|w| w["code"] == "DUPLICATE_LINK");
    assert!(has_dup_warning);
}

// --- K7.14: knowledge_unlink removes link ---
#[test]
fn k7_14_knowledge_unlink() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    add_node(dir.path(), "Test UDE", "UDE");
    mcp_call(
        dir.path(),
        "ltp/knowledge_add",
        json!({"label": "Evidence", "type": "measurement", "source_excerpt": "data"}),
    );
    mcp_call(
        dir.path(),
        "ltp/knowledge_link",
        json!({"id": "KN-001", "target": "UDE-001", "relation": "supports"}),
    );

    let resp = mcp_call(
        dir.path(),
        "ltp/knowledge_unlink",
        json!({"id": "KN-001", "target": "UDE-001"}),
    );

    let content = get_tool_content(&resp);
    assert_eq!(content["success"], true);
    assert_eq!(content["data"]["removed_count"], 1);
    assert_eq!(content["data"]["link_count"], 0);
}

// --- K7.15: knowledge_unlink nonexistent ---
#[test]
fn k7_15_knowledge_unlink_not_found() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    mcp_call(
        dir.path(),
        "ltp/knowledge_add",
        json!({"label": "Evidence", "type": "measurement", "source_excerpt": "data"}),
    );

    let resp = mcp_call(
        dir.path(),
        "ltp/knowledge_unlink",
        json!({"id": "KN-001", "target": "UDE-001"}),
    );

    let content = get_tool_content(&resp);
    assert_eq!(content["success"], false);
    assert_eq!(content["errors"][0]["code"], "LINK_NOT_FOUND");
}

// --- K7.16: knowledge_list with unlinked filter ---
#[test]
fn k7_16_knowledge_list_unlinked() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    mcp_call(
        dir.path(),
        "ltp/knowledge_add",
        json!({"label": "Unlinked", "type": "observation", "source_excerpt": "x"}),
    );
    add_node(dir.path(), "Node", "UDE");
    mcp_call(
        dir.path(),
        "ltp/knowledge_add",
        json!({"label": "To Link", "type": "measurement", "source_excerpt": "y"}),
    );
    mcp_call(
        dir.path(),
        "ltp/knowledge_link",
        json!({"id": "KN-002", "target": "UDE-001", "relation": "supports"}),
    );

    let resp = mcp_call(dir.path(), "ltp/knowledge_list", json!({"unlinked": true}));

    let content = get_tool_content(&resp);
    assert_eq!(content["success"], true);
    assert_eq!(content["data"]["total"], 1);
    assert_eq!(content["data"]["items"][0]["id"], "KN-001");
}

// --- K7.17: knowledge_list with target filter ---
#[test]
fn k7_17_knowledge_list_by_target() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    add_node(dir.path(), "Target", "UDE");
    mcp_call(
        dir.path(),
        "ltp/knowledge_add",
        json!({"label": "Linked", "type": "measurement", "source_excerpt": "x"}),
    );
    mcp_call(
        dir.path(),
        "ltp/knowledge_link",
        json!({"id": "KN-001", "target": "UDE-001", "relation": "supports"}),
    );
    mcp_call(
        dir.path(),
        "ltp/knowledge_add",
        json!({"label": "Not linked", "type": "observation", "source_excerpt": "y"}),
    );

    let resp = mcp_call(
        dir.path(),
        "ltp/knowledge_list",
        json!({"target": "UDE-001"}),
    );

    let content = get_tool_content(&resp);
    assert_eq!(content["data"]["total"], 1);
    assert_eq!(content["data"]["items"][0]["id"], "KN-001");
}

// --- K7.18: knowledge_list with target + relation filter ---
#[test]
fn k7_18_knowledge_list_target_and_relation() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    add_node(dir.path(), "Target", "UDE");
    mcp_call(
        dir.path(),
        "ltp/knowledge_add",
        json!({"label": "Supports", "type": "measurement", "source_excerpt": "x"}),
    );
    mcp_call(
        dir.path(),
        "ltp/knowledge_link",
        json!({"id": "KN-001", "target": "UDE-001", "relation": "supports"}),
    );
    mcp_call(
        dir.path(),
        "ltp/knowledge_add",
        json!({"label": "Contradicts", "type": "testimony", "source_excerpt": "y"}),
    );
    mcp_call(
        dir.path(),
        "ltp/knowledge_link",
        json!({"id": "KN-002", "target": "UDE-001", "relation": "contradicts"}),
    );

    let resp = mcp_call(
        dir.path(),
        "ltp/knowledge_list",
        json!({"target": "UDE-001", "relation": "supports"}),
    );

    let content = get_tool_content(&resp);
    assert_eq!(content["data"]["total"], 1);
    assert_eq!(content["data"]["items"][0]["id"], "KN-001");
}

// --- K7.19: knowledge_list no filters ---
#[test]
fn k7_19_knowledge_list_all() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    mcp_call(
        dir.path(),
        "ltp/knowledge_add",
        json!({"label": "A", "type": "measurement", "source_excerpt": "x"}),
    );
    mcp_call(
        dir.path(),
        "ltp/knowledge_add",
        json!({"label": "B", "type": "observation", "source_excerpt": "y"}),
    );

    let resp = mcp_call(dir.path(), "ltp/knowledge_list", json!({}));

    let content = get_tool_content(&resp);
    assert_eq!(content["data"]["total"], 2);
}

// --- K7.20: knowledge_inspect valid ---
#[test]
fn k7_20_knowledge_inspect_valid() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    mcp_call(
        dir.path(),
        "ltp/knowledge_add",
        json!({
            "label": "Detailed item",
            "type": "document",
            "source_uri": "https://example.com",
            "source_excerpt": "page 5",
            "status": "verified",
            "confidence": "high",
            "tags": ["q2", "logistics"]
        }),
    );

    let resp = mcp_call(dir.path(), "ltp/knowledge_inspect", json!({"id": "KN-001"}));

    let content = get_tool_content(&resp);
    assert_eq!(content["success"], true);
    assert_eq!(content["data"]["label"], "Detailed item");
    assert_eq!(content["data"]["type"], "document");
    assert_eq!(content["data"]["status"], "verified");
    assert_eq!(content["data"]["confidence"], "high");
    assert_eq!(content["data"]["source"]["uri"], "https://example.com");
    assert_eq!(content["data"]["source"]["excerpt"], "page 5");
    assert!(content["data"]["tags"]
        .as_array()
        .unwrap()
        .contains(&json!("q2")));
}

// --- K7.21: knowledge_inspect nonexistent ---
#[test]
fn k7_21_knowledge_inspect_not_found() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    let resp = mcp_call(dir.path(), "ltp/knowledge_inspect", json!({"id": "KN-999"}));

    let content = get_tool_content(&resp);
    assert_eq!(content["success"], false);
    assert_eq!(resp["result"]["isError"], true);
}

// --- K7.22: node_add with epistemic via MCP ---
#[test]
fn k7_22_node_add_with_epistemic() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    let resp = mcp_call(
        dir.path(),
        "ltp/node_add",
        json!({"label": "Known fact", "type": "UDE", "epistemic": "fact"}),
    );

    let content = get_tool_content(&resp);
    assert_eq!(content["success"], true);

    // Verify via inspect
    let inspect = mcp_call(dir.path(), "ltp/node_inspect", json!({"id": "UDE-001"}));
    let inspect_data = get_tool_content(&inspect);
    assert_eq!(inspect_data["data"]["epistemic"], "fact");
}

// --- K7.23: node_add without epistemic defaults to hypothesis ---
#[test]
fn k7_23_node_add_default_epistemic() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    mcp_call(
        dir.path(),
        "ltp/node_add",
        json!({"label": "Default node", "type": "UDE"}),
    );

    let inspect = mcp_call(dir.path(), "ltp/node_inspect", json!({"id": "UDE-001"}));
    let data = get_tool_content(&inspect);
    assert_eq!(data["data"]["epistemic"], "hypothesis");
}

// --- K7.24: node_edit with epistemic ---
#[test]
fn k7_24_node_edit_epistemic() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    mcp_call(
        dir.path(),
        "ltp/node_add",
        json!({"label": "Test", "type": "UDE"}),
    );

    let resp = mcp_call(
        dir.path(),
        "ltp/node_edit",
        json!({"id": "UDE-001", "epistemic": "fact"}),
    );

    let content = get_tool_content(&resp);
    assert_eq!(content["success"], true);

    let inspect = mcp_call(dir.path(), "ltp/node_inspect", json!({"id": "UDE-001"}));
    let data = get_tool_content(&inspect);
    assert_eq!(data["data"]["epistemic"], "fact");
}

// --- K7.25: node_list with epistemic filter ---
#[test]
fn k7_25_node_list_epistemic_filter() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    mcp_call(
        dir.path(),
        "ltp/node_add",
        json!({"label": "Fact node", "type": "UDE", "epistemic": "fact"}),
    );
    mcp_call(
        dir.path(),
        "ltp/node_add",
        json!({"label": "Hypo node", "type": "RC"}),
    );

    let resp = mcp_call(dir.path(), "ltp/node_list", json!({"epistemic": "fact"}));

    let content = get_tool_content(&resp);
    assert_eq!(content["success"], true);
    let nodes = content["data"]["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0]["id"], "UDE-001");
}

// --- K7.26: trace with show_knowledge ---
#[test]
fn k7_26_trace_with_knowledge() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    add_node(dir.path(), "Root cause", "RC");
    add_node(dir.path(), "Effect", "UDE");

    mcp_call(
        dir.path(),
        "ltp/tree_new",
        json!({"type": "CRT", "name": "test-crt"}),
    );
    mcp_call(
        dir.path(),
        "ltp/tree_attach",
        json!({"tree": "tree-crt-test-crt", "node": "RC-001"}),
    );
    mcp_call(
        dir.path(),
        "ltp/tree_attach",
        json!({"tree": "tree-crt-test-crt", "node": "UDE-001"}),
    );
    mcp_call(
        dir.path(),
        "ltp/link_connect",
        json!({"tree": "tree-crt-test-crt", "from": ["RC-001"], "to": ["UDE-001"]}),
    );

    mcp_call(
        dir.path(),
        "ltp/knowledge_add",
        json!({"label": "Evidence for RC", "type": "measurement", "source_excerpt": "data"}),
    );
    mcp_call(
        dir.path(),
        "ltp/knowledge_link",
        json!({"id": "KN-001", "target": "RC-001", "relation": "supports"}),
    );

    let resp = mcp_call(
        dir.path(),
        "ltp/trace",
        json!({
            "tree": "tree-crt-test-crt",
            "node_id": "UDE-001",
            "direction": "upstream",
            "show_knowledge": true
        }),
    );

    let content = get_tool_content(&resp);
    assert_eq!(content["success"], true);
    let chain = content["data"]["chain"].as_array().unwrap();
    let rc_entry = chain.iter().find(|e| e["node"] == "RC-001").unwrap();
    let knowledge = rc_entry["knowledge"].as_array().unwrap();
    assert!(!knowledge.is_empty());
    assert_eq!(knowledge[0]["id"], "KN-001");
    assert_eq!(knowledge[0]["relation"], "supports");
}

// --- K7.27: trace without show_knowledge ---
#[test]
fn k7_27_trace_without_knowledge() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    add_node(dir.path(), "Root cause", "RC");
    add_node(dir.path(), "Effect", "UDE");

    mcp_call(
        dir.path(),
        "ltp/tree_new",
        json!({"type": "CRT", "name": "test-crt"}),
    );
    mcp_call(
        dir.path(),
        "ltp/tree_attach",
        json!({"tree": "tree-crt-test-crt", "node": "RC-001"}),
    );
    mcp_call(
        dir.path(),
        "ltp/tree_attach",
        json!({"tree": "tree-crt-test-crt", "node": "UDE-001"}),
    );
    mcp_call(
        dir.path(),
        "ltp/link_connect",
        json!({"tree": "tree-crt-test-crt", "from": ["RC-001"], "to": ["UDE-001"]}),
    );

    let resp = mcp_call(
        dir.path(),
        "ltp/trace",
        json!({
            "tree": "tree-crt-test-crt",
            "node_id": "UDE-001",
            "direction": "upstream"
        }),
    );

    let content = get_tool_content(&resp);
    assert_eq!(content["success"], true);
    let chain = content["data"]["chain"].as_array().unwrap();
    let rc_entry = chain.iter().find(|e| e["node"] == "RC-001").unwrap();
    assert!(rc_entry.get("knowledge").is_none() || rc_entry["knowledge"].is_null());
}

// --- K7.28: tree_walk with show_knowledge ---
#[test]
fn k7_28_tree_walk_with_knowledge() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    add_node(dir.path(), "Node A", "UDE");
    mcp_call(
        dir.path(),
        "ltp/tree_new",
        json!({"type": "CRT", "name": "walk-test"}),
    );
    mcp_call(
        dir.path(),
        "ltp/tree_attach",
        json!({"tree": "tree-crt-walk-test", "node": "UDE-001"}),
    );
    mcp_call(
        dir.path(),
        "ltp/knowledge_add",
        json!({"label": "Support A", "type": "measurement", "source_excerpt": "x"}),
    );
    mcp_call(
        dir.path(),
        "ltp/knowledge_add",
        json!({"label": "Context A", "type": "document", "source_excerpt": "y"}),
    );
    mcp_call(
        dir.path(),
        "ltp/knowledge_link",
        json!({"id": "KN-001", "target": "UDE-001", "relation": "supports"}),
    );
    mcp_call(
        dir.path(),
        "ltp/knowledge_link",
        json!({"id": "KN-002", "target": "UDE-001", "relation": "contextualizes"}),
    );

    let resp = mcp_call(
        dir.path(),
        "ltp/tree_walk",
        json!({"tree_id": "tree-crt-walk-test", "show_knowledge": true}),
    );

    let content = get_tool_content(&resp);
    assert_eq!(content["success"], true);
    let nodes = content["data"]["nodes"].as_array().unwrap();
    let node_a = nodes.iter().find(|n| n["id"] == "UDE-001").unwrap();
    let kn = &node_a["knowledge"];
    assert_eq!(kn["supports"], 1);
    assert_eq!(kn["contextualizes"], 1);
    assert_eq!(kn["contradicts"], 0);
}

// --- K7.29: status includes knowledge_health ---
#[test]
fn k7_29_status_knowledge_health() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    mcp_call(
        dir.path(),
        "ltp/knowledge_add",
        json!({"label": "A", "type": "measurement", "source_excerpt": "x", "status": "verified"}),
    );
    mcp_call(
        dir.path(),
        "ltp/knowledge_add",
        json!({"label": "B", "type": "observation", "source_excerpt": "y"}),
    );

    let resp = mcp_call(dir.path(), "ltp/status", json!({}));
    let content = get_tool_content(&resp);

    assert_eq!(content["success"], true);
    let kh = &content["data"]["knowledge_health"];
    assert_eq!(kh["total"], 2);
    assert_eq!(kh["unlinked_items"], 2);
    assert_eq!(kh["by_status"]["verified"], 1);
    assert_eq!(kh["by_status"]["unverified"], 1);
}

// --- K7.30: validate reports knowledge warnings ---
#[test]
fn k7_30_validate_knowledge_warnings() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    // Create fact node
    add_node(dir.path(), "Confirmed fact", "UDE");
    mcp_call(
        dir.path(),
        "ltp/node_edit",
        json!({"id": "UDE-001", "epistemic": "fact"}),
    );

    // Attach to a tree (validate operates per-tree)
    mcp_call(
        dir.path(),
        "ltp/tree_new",
        json!({"type": "CRT", "name": "val-test"}),
    );
    mcp_call(
        dir.path(),
        "ltp/tree_attach",
        json!({"tree": "tree-crt-val-test", "node": "UDE-001"}),
    );

    // No supports = EPISTEMIC_UNGROUNDED
    let resp = mcp_call(dir.path(), "ltp/validate", json!({}));
    let content = get_tool_content(&resp);

    let details = content["data"]["details"].as_array().unwrap();
    let has_ungrounded = details.iter().any(|d| {
        d["warnings"]
            .as_array()
            .map(|ws| ws.iter().any(|w| w["code"] == "EPISTEMIC_UNGROUNDED"))
            .unwrap_or(false)
    });
    assert!(has_ungrounded, "Expected EPISTEMIC_UNGROUNDED warning");
}

// --- K7.31: knowledge_add + undo via MCP ---
#[test]
fn k7_31_undo_knowledge_add() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    mcp_call(
        dir.path(),
        "ltp/knowledge_add",
        json!({"label": "Undo me", "type": "observation", "source_excerpt": "x"}),
    );

    // Verify it exists
    let list = mcp_call(dir.path(), "ltp/knowledge_list", json!({}));
    let list_data = get_tool_content(&list);
    assert_eq!(list_data["data"]["total"], 1);

    // Undo
    let undo_resp = mcp_call(dir.path(), "ltp/undo", json!({}));
    let undo_data = get_tool_content(&undo_resp);
    assert_eq!(undo_data["success"], true);

    // Verify it's gone
    let list2 = mcp_call(dir.path(), "ltp/knowledge_list", json!({}));
    let list2_data = get_tool_content(&list2);
    assert_eq!(list2_data["data"]["total"], 0);
}

// --- K7.32: knowledge_link + undo via MCP ---
#[test]
fn k7_32_undo_knowledge_link() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    add_node(dir.path(), "Target", "UDE");
    mcp_call(
        dir.path(),
        "ltp/knowledge_add",
        json!({"label": "Link me", "type": "measurement", "source_excerpt": "x"}),
    );
    mcp_call(
        dir.path(),
        "ltp/knowledge_link",
        json!({"id": "KN-001", "target": "UDE-001", "relation": "supports"}),
    );

    // Verify link exists
    let inspect = mcp_call(dir.path(), "ltp/knowledge_inspect", json!({"id": "KN-001"}));
    let data = get_tool_content(&inspect);
    assert_eq!(data["data"]["links"].as_array().unwrap().len(), 1);

    // Undo link
    mcp_call(dir.path(), "ltp/undo", json!({}));

    // Verify link removed
    let inspect2 = mcp_call(dir.path(), "ltp/knowledge_inspect", json!({"id": "KN-001"}));
    let data2 = get_tool_content(&inspect2);
    assert_eq!(data2["data"]["links"].as_array().unwrap().len(), 0);
}

// --- K7.33: sequential adds produce sequential IDs ---
#[test]
fn k7_33_sequential_ids() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    let r1 = mcp_call(
        dir.path(),
        "ltp/knowledge_add",
        json!({"label": "A", "type": "measurement", "source_excerpt": "x"}),
    );
    let r2 = mcp_call(
        dir.path(),
        "ltp/knowledge_add",
        json!({"label": "B", "type": "observation", "source_excerpt": "y"}),
    );
    let r3 = mcp_call(
        dir.path(),
        "ltp/knowledge_add",
        json!({"label": "C", "type": "testimony", "source_excerpt": "z"}),
    );

    assert_eq!(
        get_tool_content(&r1)["data"]["created_knowledge_id"],
        "KN-001"
    );
    assert_eq!(
        get_tool_content(&r2)["data"]["created_knowledge_id"],
        "KN-002"
    );
    assert_eq!(
        get_tool_content(&r3)["data"]["created_knowledge_id"],
        "KN-003"
    );
}

// --- K7.34: knowledge_link without relation param ---
#[test]
fn k7_34_knowledge_link_missing_relation() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    let resp = mcp_call(
        dir.path(),
        "ltp/knowledge_link",
        json!({"id": "KN-001", "target": "UDE-001"}),
    );

    assert!(resp["error"].is_object());
    assert_eq!(resp["error"]["code"], -32602);
}

// --- K7.35: knowledge_edit with add_tags ---
#[test]
fn k7_35_knowledge_edit_add_tags() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    mcp_call(
        dir.path(),
        "ltp/knowledge_add",
        json!({"label": "Tagged", "type": "observation", "source_excerpt": "x"}),
    );

    let resp = mcp_call(
        dir.path(),
        "ltp/knowledge_edit",
        json!({"id": "KN-001", "add_tags": ["alpha", "beta"]}),
    );

    let content = get_tool_content(&resp);
    assert_eq!(content["success"], true);
    let tags = content["data"]["tags"].as_array().unwrap();
    assert!(tags.contains(&json!("alpha")));
    assert!(tags.contains(&json!("beta")));
}

// --- K7.36: knowledge_link with invalid relation ---
#[test]
fn k7_36_knowledge_link_invalid_relation() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    let resp = mcp_call(
        dir.path(),
        "ltp/knowledge_link",
        json!({"id": "KN-001", "target": "UDE-001", "relation": "invalid_value"}),
    );

    assert!(resp["error"].is_object());
    assert_eq!(resp["error"]["code"], -32602);
}

// --- K7.37: knowledge_inspect with dangling ref ---
#[test]
fn k7_37_knowledge_inspect_dangling_ref() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    add_node(dir.path(), "Will be deleted", "UDE");
    mcp_call(
        dir.path(),
        "ltp/knowledge_add",
        json!({"label": "Evidence", "type": "measurement", "source_excerpt": "x"}),
    );
    mcp_call(
        dir.path(),
        "ltp/knowledge_link",
        json!({"id": "KN-001", "target": "UDE-001", "relation": "supports"}),
    );

    // Delete the target node
    mcp_call(
        dir.path(),
        "ltp/node_rm",
        json!({"ids": ["UDE-001"], "force": true}),
    );

    // Inspect should still work with null label
    let resp = mcp_call(dir.path(), "ltp/knowledge_inspect", json!({"id": "KN-001"}));
    let content = get_tool_content(&resp);
    assert_eq!(content["success"], true);
    let links = content["data"]["links"].as_array().unwrap();
    assert_eq!(links.len(), 1);
    assert!(links[0]["target_label"].is_null());
}

// --- K7.38: node_edit with invalid epistemic ---
#[test]
fn k7_38_node_edit_invalid_epistemic() {
    let dir = tempfile::tempdir().unwrap();
    init_workspace(dir.path());

    mcp_call(
        dir.path(),
        "ltp/node_add",
        json!({"label": "Test", "type": "UDE"}),
    );

    let resp = mcp_call(
        dir.path(),
        "ltp/node_edit",
        json!({"id": "UDE-001", "epistemic": "invalid_value"}),
    );

    let content = get_tool_content(&resp);
    assert_eq!(content["success"], false);
}
