use std::process::Command;

use serde_json::Value;

fn ltp_bin() -> String {
    env!("CARGO_BIN_EXE_ltp").to_string()
}

fn run_ltp(dir: &std::path::Path, args: &[&str]) -> (Value, i32) {
    let output = Command::new(ltp_bin())
        .args(args)
        .current_dir(dir)
        .output()
        .expect("failed to execute ltp binary");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let code = output.status.code().unwrap_or(-1);
    let json: Value = serde_json::from_str(&stdout).unwrap_or_else(|_| {
        panic!(
            "Failed to parse JSON.\nstdout: {}\nstderr: {}",
            stdout,
            String::from_utf8_lossy(&output.stderr)
        )
    });
    (json, code)
}

fn setup_workspace(dir: &std::path::Path) {
    let (_, code) = run_ltp(dir, &["init", "--name", "F7Test"]);
    assert_eq!(code, 0);
}

fn add_node(dir: &std::path::Path, label: &str, node_type: &str) -> String {
    let (json, _) = run_ltp(dir, &["node", "add", label, "--type", node_type]);
    json["data"]["id"].as_str().unwrap().to_string()
}

fn create_tree(dir: &std::path::Path, tree_type: &str, name: &str) -> String {
    let (json, _) = run_ltp(dir, &["tree", "new", tree_type, name]);
    json["data"]["id"].as_str().unwrap().to_string()
}

fn attach_node(dir: &std::path::Path, tree_id: &str, node_id: &str) {
    let (_, code) = run_ltp(
        dir,
        &["tree", "attach", "--tree", tree_id, "--node", node_id],
    );
    assert_eq!(code, 0);
}

fn connect(dir: &std::path::Path, tree_id: &str, from: &str, to: &str) -> String {
    let (json, code) = run_ltp(
        dir,
        &[
            "link", "connect", "--tree", tree_id, "--from", from, "--to", to,
        ],
    );
    assert_eq!(code, 0);
    json["data"]["created_links"][0]
        .as_str()
        .unwrap()
        .to_string()
}

/// Setup: workspace + 2 nodes + 1 tree + attach + connect → returns (tree_id, link_id)
fn basic_setup(dir: &std::path::Path) -> (String, String) {
    setup_workspace(dir);
    let n1 = add_node(dir, "Cause A", "UDE");
    let n2 = add_node(dir, "Effect B", "UDE");
    let tree = create_tree(dir, "crt", "TestTree");
    attach_node(dir, &tree, &n1);
    attach_node(dir, &tree, &n2);
    let link = connect(dir, &tree, &n1, &n2);
    (tree, link)
}

/// Setup with 3 nodes and 2 links for multi-edge tests
fn multi_edge_setup(dir: &std::path::Path) -> (String, String, String) {
    setup_workspace(dir);
    let n1 = add_node(dir, "Cause A", "UDE");
    let n2 = add_node(dir, "Effect B", "UDE");
    let n3 = add_node(dir, "Effect C", "UDE");
    let tree = create_tree(dir, "crt", "TestTree");
    attach_node(dir, &tree, &n1);
    attach_node(dir, &tree, &n2);
    attach_node(dir, &tree, &n3);
    let link1 = connect(dir, &tree, &n1, &n2);
    let link2 = connect(dir, &tree, &n1, &n3);
    (tree, link1, link2)
}

// --- UAT 7.1: assume add ---
#[test]
fn test_assume_add() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree, link) = basic_setup(dir);

    let (json, code) = run_ltp(
        dir,
        &[
            "assume",
            "add",
            "--tree",
            &tree,
            "--link",
            &link,
            "--text",
            "Market remains stable",
        ],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["action"], "assume_add");
    assert_eq!(json["data"]["id"], "ASM-001");
    assert_eq!(json["data"]["link_id"], link.as_str());
    assert_eq!(json["data"]["tree_id"], tree.as_str());
    assert_eq!(json["data"]["text"], "Market remains stable");
}

// --- UAT 7.2: assume edit ---
#[test]
fn test_assume_edit() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree, link) = basic_setup(dir);

    run_ltp(
        dir,
        &[
            "assume",
            "add",
            "--tree",
            &tree,
            "--link",
            &link,
            "--text",
            "Original text",
        ],
    );

    let (json, code) = run_ltp(
        dir,
        &[
            "assume",
            "edit",
            "--tree",
            &tree,
            "--asm",
            "ASM-001",
            "--text",
            "Updated text",
        ],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["action"], "assume_edit");
    assert_eq!(json["data"]["id"], "ASM-001");
    assert_eq!(json["data"]["text"], "Updated text");
}

// --- UAT 7.3: assume list ---
#[test]
fn test_assume_list() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree, link) = basic_setup(dir);

    run_ltp(
        dir,
        &[
            "assume",
            "add",
            "--tree",
            &tree,
            "--link",
            &link,
            "--text",
            "Assumption one",
        ],
    );
    run_ltp(
        dir,
        &[
            "assume",
            "add",
            "--tree",
            &tree,
            "--link",
            &link,
            "--text",
            "Assumption two",
        ],
    );

    let (json, code) = run_ltp(dir, &["assume", "list", "--tree", &tree]);

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["action"], "assume_list");

    let assumptions = json["data"]["assumptions"].as_array().unwrap();
    assert_eq!(assumptions.len(), 2);
    assert_eq!(assumptions[0]["id"], "ASM-001");
    assert_eq!(assumptions[0]["link_id"], link.as_str());
    assert_eq!(assumptions[0]["status"], "valid");
    assert_eq!(assumptions[1]["id"], "ASM-002");
}

// --- UAT 7.4: assume list filter by status ---
#[test]
fn test_assume_list_filter() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree, link) = basic_setup(dir);

    run_ltp(
        dir,
        &[
            "assume",
            "add",
            "--tree",
            &tree,
            "--link",
            &link,
            "--text",
            "Valid assumption",
        ],
    );
    // Invalidate it
    run_ltp(
        dir,
        &[
            "invalidate",
            "--tree",
            &tree,
            "--link",
            &link,
            "--asm",
            "ASM-001",
        ],
    );
    // Add another valid one
    run_ltp(
        dir,
        &[
            "assume",
            "add",
            "--tree",
            &tree,
            "--link",
            &link,
            "--text",
            "Still valid",
        ],
    );

    let (json, code) = run_ltp(
        dir,
        &["assume", "list", "--tree", &tree, "--status", "valid"],
    );

    assert_eq!(code, 0);
    let assumptions = json["data"]["assumptions"].as_array().unwrap();
    assert_eq!(assumptions.len(), 1);
    assert_eq!(assumptions[0]["id"], "ASM-002");
    assert_eq!(assumptions[0]["status"], "valid");
}

// --- UAT 7.5: assume move ---
#[test]
fn test_assume_move() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree, link1, link2) = multi_edge_setup(dir);

    run_ltp(
        dir,
        &[
            "assume",
            "add",
            "--tree",
            &tree,
            "--link",
            &link1,
            "--text",
            "Moveable assumption",
        ],
    );

    let (json, code) = run_ltp(
        dir,
        &[
            "assume",
            "move",
            "--tree",
            &tree,
            "--asm",
            "ASM-001",
            "--to-link",
            &link2,
        ],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["action"], "assume_move");
    assert_eq!(json["data"]["id"], "ASM-001");
    assert_eq!(json["data"]["from_link"], link1.as_str());
    assert_eq!(json["data"]["to_link"], link2.as_str());

    // Verify it's now in link2
    let (list_json, _) = run_ltp(dir, &["assume", "list", "--tree", &tree]);
    let assumptions = list_json["data"]["assumptions"].as_array().unwrap();
    assert_eq!(assumptions[0]["link_id"], link2.as_str());
}

// --- UAT 7.6: assume rm ---
#[test]
fn test_assume_rm() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree, link) = basic_setup(dir);

    run_ltp(
        dir,
        &[
            "assume",
            "add",
            "--tree",
            &tree,
            "--link",
            &link,
            "--text",
            "To be removed",
        ],
    );

    let (json, code) = run_ltp(dir, &["assume", "rm", "--tree", &tree, "--asm", "ASM-001"]);

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["action"], "assume_rm");
    assert_eq!(json["data"]["id"], "ASM-001");

    // Verify it's gone
    let (list_json, _) = run_ltp(dir, &["assume", "list", "--tree", &tree]);
    let assumptions = list_json["data"]["assumptions"].as_array().unwrap();
    assert_eq!(assumptions.len(), 0);
}

// --- UAT 7.7: invalidate basic ---
#[test]
fn test_invalidate_basic() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree, link) = basic_setup(dir);

    run_ltp(
        dir,
        &[
            "assume",
            "add",
            "--tree",
            &tree,
            "--link",
            &link,
            "--text",
            "Will be invalidated",
        ],
    );

    let (json, code) = run_ltp(
        dir,
        &[
            "invalidate",
            "--tree",
            &tree,
            "--link",
            &link,
            "--asm",
            "ASM-001",
        ],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["action"], "invalidate");
    assert_eq!(json["data"]["asm_id"], "ASM-001");
    assert_eq!(json["data"]["link_id"], link.as_str());
    assert_eq!(json["data"]["link_status"], "broken");
    assert_eq!(json["data"]["changed"], true);
    assert!(json["data"]["injection_id"].is_null());
}

// --- UAT 7.8: invalidate with injection ---
#[test]
fn test_invalidate_with_injection() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree, link) = basic_setup(dir);

    run_ltp(
        dir,
        &[
            "assume",
            "add",
            "--tree",
            &tree,
            "--link",
            &link,
            "--text",
            "Bad assumption",
        ],
    );

    let (json, code) = run_ltp(
        dir,
        &[
            "invalidate",
            "--tree",
            &tree,
            "--link",
            &link,
            "--asm",
            "ASM-001",
            "--injection",
            "Add redundancy to system",
        ],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["changed"], true);
    assert_eq!(json["data"]["injection_id"], "INJ-001");

    // Verify INJ node exists in pool
    let (inspect_json, inspect_code) = run_ltp(dir, &["node", "inspect", "INJ-001"]);
    assert_eq!(inspect_code, 0);
    assert_eq!(inspect_json["data"]["label"], "Add redundancy to system");
    assert_eq!(inspect_json["data"]["node_type"], "INJ");
}

// --- UAT 7.9: assume add link not found ---
#[test]
fn test_assume_add_link_not_found() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree, _link) = basic_setup(dir);

    let (json, code) = run_ltp(
        dir,
        &[
            "assume",
            "add",
            "--tree",
            &tree,
            "--link",
            "LINK-999",
            "--text",
            "Should fail",
        ],
    );

    assert_eq!(code, 1);
    assert_eq!(json["success"], false);
    assert_eq!(json["errors"][0]["code"], "LINK_NOT_FOUND");
}

// --- UAT 7.10: assume add tree not found ---
#[test]
fn test_assume_add_tree_not_found() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let (json, code) = run_ltp(
        dir,
        &[
            "assume",
            "add",
            "--tree",
            "TREE-999",
            "--link",
            "LINK-001",
            "--text",
            "Should fail",
        ],
    );

    assert_eq!(code, 1);
    assert_eq!(json["success"], false);
    assert_eq!(json["errors"][0]["code"], "TREE_NOT_FOUND");
}

// --- UAT 7.11: assume edit asm not found ---
#[test]
fn test_assume_edit_asm_not_found() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree, _link) = basic_setup(dir);

    let (json, code) = run_ltp(
        dir,
        &[
            "assume",
            "edit",
            "--tree",
            &tree,
            "--asm",
            "ASM-999",
            "--text",
            "Should fail",
        ],
    );

    assert_eq!(code, 1);
    assert_eq!(json["success"], false);
    assert_eq!(json["errors"][0]["code"], "ASSUMPTION_NOT_FOUND");
}

// --- UAT 7.12: assume move target not found ---
#[test]
fn test_assume_move_target_not_found() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree, link) = basic_setup(dir);

    run_ltp(
        dir,
        &[
            "assume", "add", "--tree", &tree, "--link", &link, "--text", "Stay put",
        ],
    );

    let (json, code) = run_ltp(
        dir,
        &[
            "assume",
            "move",
            "--tree",
            &tree,
            "--asm",
            "ASM-001",
            "--to-link",
            "LINK-999",
        ],
    );

    assert_eq!(code, 1);
    assert_eq!(json["success"], false);
    assert_eq!(json["errors"][0]["code"], "LINK_NOT_FOUND");

    // Verify assumption was NOT removed from original edge
    let (list_json, _) = run_ltp(dir, &["assume", "list", "--tree", &tree]);
    let assumptions = list_json["data"]["assumptions"].as_array().unwrap();
    assert_eq!(assumptions.len(), 1);
    assert_eq!(assumptions[0]["link_id"], link.as_str());
}

// --- UAT 7.13: invalidate asm not in link ---
#[test]
fn test_invalidate_asm_not_in_link() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree, link1, link2) = multi_edge_setup(dir);

    // Add assumption to link1
    run_ltp(
        dir,
        &[
            "assume", "add", "--tree", &tree, "--link", &link1, "--text", "On link1",
        ],
    );

    // Try to invalidate ASM-001 via link2 (wrong link)
    let (json, code) = run_ltp(
        dir,
        &[
            "invalidate",
            "--tree",
            &tree,
            "--link",
            &link2,
            "--asm",
            "ASM-001",
        ],
    );

    assert_eq!(code, 1);
    assert_eq!(json["success"], false);
    assert_eq!(json["errors"][0]["code"], "ASSUMPTION_NOT_IN_LINK");
}

// --- UAT 7.14: invalidate idempotent ---
#[test]
fn test_invalidate_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree, link) = basic_setup(dir);

    run_ltp(
        dir,
        &[
            "assume",
            "add",
            "--tree",
            &tree,
            "--link",
            &link,
            "--text",
            "Will be double-invalidated",
        ],
    );

    // First invalidation
    let (json1, code1) = run_ltp(
        dir,
        &[
            "invalidate",
            "--tree",
            &tree,
            "--link",
            &link,
            "--asm",
            "ASM-001",
        ],
    );
    assert_eq!(code1, 0);
    assert_eq!(json1["data"]["changed"], true);

    // Second invalidation (idempotent)
    let (json2, code2) = run_ltp(
        dir,
        &[
            "invalidate",
            "--tree",
            &tree,
            "--link",
            &link,
            "--asm",
            "ASM-001",
            "--injection",
            "Should NOT create INJ",
        ],
    );
    assert_eq!(code2, 0);
    assert_eq!(json2["success"], true);
    assert_eq!(json2["data"]["changed"], false);
    assert!(json2["data"]["injection_id"].is_null());

    // Verify warning
    let warnings = json2["warnings"].as_array().unwrap();
    assert!(warnings.iter().any(|w| w["code"] == "ALREADY_INVALIDATED"));

    // Verify no INJ node was created
    let (inspect_json, inspect_code) = run_ltp(dir, &["node", "inspect", "INJ-001"]);
    assert_eq!(inspect_code, 1);
    assert_eq!(inspect_json["success"], false);
}

// --- UAT 7.15: assume list multi-edge ---
#[test]
fn test_assume_list_multi_edge() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree, link1, link2) = multi_edge_setup(dir);

    // Add 2 assumptions on link1
    run_ltp(
        dir,
        &[
            "assume",
            "add",
            "--tree",
            &tree,
            "--link",
            &link1,
            "--text",
            "First on link1",
        ],
    );
    run_ltp(
        dir,
        &[
            "assume",
            "add",
            "--tree",
            &tree,
            "--link",
            &link1,
            "--text",
            "Second on link1",
        ],
    );
    // Add 1 assumption on link2
    run_ltp(
        dir,
        &[
            "assume",
            "add",
            "--tree",
            &tree,
            "--link",
            &link2,
            "--text",
            "First on link2",
        ],
    );

    let (json, code) = run_ltp(dir, &["assume", "list", "--tree", &tree]);

    assert_eq!(code, 0);
    let assumptions = json["data"]["assumptions"].as_array().unwrap();
    assert_eq!(assumptions.len(), 3);

    // Verify each entry shows its correct link_id
    assert_eq!(assumptions[0]["link_id"], link1.as_str());
    assert_eq!(assumptions[1]["link_id"], link1.as_str());
    assert_eq!(assumptions[2]["link_id"], link2.as_str());
}
