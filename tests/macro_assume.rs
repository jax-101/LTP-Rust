//! Integration tests for `macro-assume gather` (Slice 1, Phase M2).
//!
//! Exercises the read-only "gather" command end-to-end through the CLI binary:
//! the happy path over a collapsed long arrow (H1) plus the referential-integrity
//! error paths (C1: macro-edge not found, C2: tree not found).

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
    let (_, code) = run_ltp(dir, &["init", "--name", "MacroAssumeTest"]);
    assert_eq!(code, 0);
}

fn add_node(dir: &std::path::Path, label: &str, node_type: &str) -> String {
    let (json, code) = run_ltp(dir, &["node", "add", label, "--type", node_type]);
    assert_eq!(code, 0, "node add failed: {json:?}");
    json["data"]["id"].as_str().unwrap().to_string()
}

fn create_tree(dir: &std::path::Path, tree_type: &str, name: &str) -> String {
    let (json, code) = run_ltp(dir, &["tree", "new", tree_type, name]);
    assert_eq!(code, 0, "tree new failed: {json:?}");
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
    assert_eq!(code, 0, "link connect failed: {json:?}");
    json["data"]["created_links"][0]
        .as_str()
        .unwrap()
        .to_string()
}

fn add_assumption(dir: &std::path::Path, tree_id: &str, link_id: &str, text: &str) -> String {
    let (json, code) = run_ltp(
        dir,
        &[
            "assume", "add", "--tree", tree_id, "--link", link_id, "--text", text,
        ],
    );
    assert_eq!(code, 0, "assume add failed: {json:?}");
    json["data"]["id"].as_str().unwrap().to_string()
}

fn collapse(dir: &std::path::Path, tree_id: &str, from: &str, to: &str, label: &str) -> String {
    let (json, code) = run_ltp(
        dir,
        &[
            "path", "collapse", "--tree", tree_id, "--from", from, "--to", to, "--label", label,
        ],
    );
    assert_eq!(code, 0, "path collapse failed: {json:?}");
    json["data"]["macro_edge_id"].as_str().unwrap().to_string()
}

/// Build a linear chain A → B → C → D → E and collapse it into a single long arrow.
///
/// Returns `(tree_id, links[L1..L4], macro_edge_id)`. Interior links = all 4 edges
/// (collapse is non-destructive: interior edges stay in `tree.edges`).
fn collapsed_chain(dir: &std::path::Path) -> (String, Vec<String>, String) {
    setup_workspace(dir);
    let a = add_node(dir, "Node A", "RC");
    let b = add_node(dir, "Node B", "INT");
    let c = add_node(dir, "Node C", "INT");
    let d = add_node(dir, "Node D", "INT");
    let e = add_node(dir, "Node E", "UDE");
    let tree = create_tree(dir, "crt", "ChainCRT");
    for n in [&a, &b, &c, &d, &e] {
        attach_node(dir, &tree, n);
    }
    let l1 = connect(dir, &tree, &a, &b);
    let l2 = connect(dir, &tree, &b, &c);
    let l3 = connect(dir, &tree, &c, &d);
    let l4 = connect(dir, &tree, &d, &e);
    let macro_id = collapse(dir, &tree, &a, &e, "Cadena lógica");
    (tree, vec![l1, l2, l3, l4], macro_id)
}

fn string_array(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap().to_string())
        .collect()
}

// --- UAT H1: gather over a fresh (unsummarized) long arrow ---
#[test]
fn uat_h1_gather_fresh_macro_edge() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, links, macro_id) = collapsed_chain(dir.path());

    // Two interior links carry assumptions; the other two stay empty.
    let asm1 = add_assumption(dir.path(), &tree, &links[0], "El proveedor cumple SLA");
    let asm2 = add_assumption(dir.path(), &tree, &links[1], "La demanda es estable");

    let (json, code) = run_ltp(
        dir.path(),
        &[
            "macro-assume",
            "gather",
            "--tree",
            &tree,
            "--macro-link",
            &macro_id,
        ],
    );
    assert_eq!(code, 0, "gather failed: {json:?}");
    assert!(json["success"].as_bool().unwrap());
    assert_eq!(json["action"], "macro_assume_gather");
    assert_eq!(json["data"]["macro_link"], macro_id);

    // Interior: all 4 links present, keyed by link_id; empties included.
    let interior = &json["data"]["interior"];
    assert!(interior.is_object());
    let interior_obj = interior.as_object().unwrap();
    assert_eq!(
        interior_obj.len(),
        4,
        "all interior links present: {interior:?}"
    );
    assert_eq!(interior[&links[0]].as_array().unwrap().len(), 1);
    assert_eq!(interior[&links[0]][0]["id"], asm1);
    assert_eq!(interior[&links[0]][0]["status"], "valid");
    assert_eq!(interior[&links[1]].as_array().unwrap().len(), 1);
    assert_eq!(interior[&links[1]][0]["id"], asm2);
    assert!(interior[&links[2]].as_array().unwrap().is_empty());
    assert!(interior[&links[3]].as_array().unwrap().is_empty());

    // Diff: no summary yet, so both interior ASMs are unmapped, nothing dangling.
    let diff = &json["data"]["diff"];
    assert_eq!(diff["summary_count"], 0);
    let unmapped = string_array(&diff["unmapped"]);
    assert_eq!(unmapped, vec![asm1, asm2]);
    assert!(diff["dangling"].as_array().unwrap().is_empty());

    // Non-blocking: a fresh long arrow is a valid DAG.
    assert!(json["graph_health"]["valid_dag"].as_bool().unwrap());
    assert!(json["errors"].as_array().unwrap().is_empty());
}

// --- UAT C1: gather with a non-existent macro-edge => MACRO_EDGE_NOT_FOUND ---
#[test]
fn uat_c1_gather_macro_edge_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, _links, _macro_id) = collapsed_chain(dir.path());

    let (json, code) = run_ltp(
        dir.path(),
        &[
            "macro-assume",
            "gather",
            "--tree",
            &tree,
            "--macro-link",
            "MACRO-999",
        ],
    );
    assert_eq!(code, 1, "expected failure exit code: {json:?}");
    assert!(!json["success"].as_bool().unwrap());
    assert_eq!(json["errors"][0]["code"], "MACRO_EDGE_NOT_FOUND");
    // Empty data envelope preserves the queried macro_link for context.
    assert_eq!(json["data"]["macro_link"], "MACRO-999");
    assert!(json["data"]["interior"].as_object().unwrap().is_empty());
}

// --- UAT C2: gather against a non-existent tree => TREE_NOT_FOUND ---
#[test]
fn uat_c2_gather_tree_not_found() {
    let dir = tempfile::tempdir().unwrap();
    setup_workspace(dir.path());

    let (json, code) = run_ltp(
        dir.path(),
        &[
            "macro-assume",
            "gather",
            "--tree",
            "TREE-999",
            "--macro-link",
            "MACRO-001",
        ],
    );
    assert_eq!(code, 1, "expected failure exit code: {json:?}");
    assert!(!json["success"].as_bool().unwrap());
    assert_eq!(json["errors"][0]["code"], "TREE_NOT_FOUND");
}
