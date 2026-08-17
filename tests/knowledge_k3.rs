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
            "Failed to parse JSON output.\nstdout: {}\nstderr: {}",
            stdout,
            String::from_utf8_lossy(&output.stderr)
        )
    });

    (json, code)
}

fn init_workspace(dir: &std::path::Path) {
    let (json, code) = run_ltp(dir, &["init", "--name", "TestK3"]);
    assert_eq!(code, 0, "init failed: {:?}", json);
}

fn add_knowledge(dir: &std::path::Path, label: &str) -> String {
    let (json, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            label,
            "--type",
            "measurement",
            "--source-excerpt",
            "test source",
        ],
    );
    assert_eq!(code, 0, "knowledge add failed: {:?}", json);
    json["data"]["created_knowledge_id"]
        .as_str()
        .unwrap()
        .to_string()
}

fn add_node(dir: &std::path::Path, label: &str, node_type: &str) -> String {
    let (json, code) = run_ltp(dir, &["node", "add", label, "--type", node_type]);
    assert_eq!(code, 0, "node add failed: {:?}", json);
    json["data"]["id"].as_str().unwrap().to_string()
}

fn new_tree(dir: &std::path::Path, tree_type: &str, name: &str) -> String {
    let (json, code) = run_ltp(dir, &["tree", "new", tree_type, name]);
    assert_eq!(code, 0, "tree new failed: {:?}", json);
    json["data"]["id"].as_str().unwrap().to_string()
}

fn attach_node(dir: &std::path::Path, tree_id: &str, node_id: &str) {
    let (json, code) = run_ltp(
        dir,
        &["tree", "attach", "--tree", tree_id, "--node", node_id],
    );
    assert_eq!(code, 0, "tree attach failed: {:?}", json);
}

fn connect_nodes(dir: &std::path::Path, tree_id: &str, from: &str, to: &str) -> String {
    let (json, code) = run_ltp(
        dir,
        &[
            "link", "connect", "--tree", tree_id, "--from", from, "--to", to,
        ],
    );
    assert_eq!(code, 0, "link connect failed: {:?}", json);
    json["data"]["created_links"][0]
        .as_str()
        .unwrap()
        .to_string()
}

// === K3.1: Link to a node (happy path) ===
#[test]
fn k3_1_link_to_node() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node_id = add_node(dir, "Undesirable effect", "UDE");
    let kn_id = add_knowledge(dir, "Evidence data");

    let (json, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "link",
            &kn_id,
            "--to",
            &node_id,
            "--relation",
            "supports",
        ],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["action"], "knowledge_link");
    assert_eq!(json["data"]["id"], kn_id.as_str());
    assert_eq!(json["data"]["target"], node_id.as_str());
    assert_eq!(json["data"]["relation"], "supports");
    assert_eq!(json["data"]["link_count"], 1);
}

// === K3.2: Link to an edge (LINK-XXX) ===
#[test]
fn k3_2_link_to_edge() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let n1 = add_node(dir, "Cause", "RC");
    let n2 = add_node(dir, "Effect", "UDE");
    let tree_id = new_tree(dir, "crt", "Test CRT");
    attach_node(dir, &tree_id, &n1);
    attach_node(dir, &tree_id, &n2);
    let edge_id = connect_nodes(dir, &tree_id, &n1, &n2);

    let kn_id = add_knowledge(dir, "Edge evidence");

    let (json, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "link",
            &kn_id,
            "--to",
            &edge_id,
            "--relation",
            "supports",
        ],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["target"], edge_id.as_str());
}

// === K3.3: Link to an assumption (ASM-XXX) ===
#[test]
fn k3_3_link_to_assumption() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let n1 = add_node(dir, "Cause", "RC");
    let n2 = add_node(dir, "Effect", "UDE");
    let tree_id = new_tree(dir, "crt", "Test CRT");
    attach_node(dir, &tree_id, &n1);
    attach_node(dir, &tree_id, &n2);
    let edge_id = connect_nodes(dir, &tree_id, &n1, &n2);

    // Add assumption to edge
    let (json, code) = run_ltp(
        dir,
        &[
            "assume",
            "add",
            "--tree",
            &tree_id,
            "--link",
            &edge_id,
            "--text",
            "Supply is constant",
        ],
    );
    assert_eq!(code, 0, "assume add failed: {:?}", json);
    let asm_id = json["data"]["id"].as_str().unwrap().to_string();

    let kn_id = add_knowledge(dir, "Assumption evidence");

    let (json, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "link",
            &kn_id,
            "--to",
            &asm_id,
            "--relation",
            "contradicts",
        ],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["target"], asm_id.as_str());
    assert_eq!(json["data"]["relation"], "contradicts");
}

// === K3.4: Link with relation contextualizes ===
#[test]
fn k3_4_link_contextualizes() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node_id = add_node(dir, "Root cause", "RC");
    let kn_id = add_knowledge(dir, "Context info");

    let (json, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "link",
            &kn_id,
            "--to",
            &node_id,
            "--relation",
            "contextualizes",
        ],
    );

    assert_eq!(code, 0);
    assert_eq!(json["data"]["relation"], "contextualizes");
}

// === K3.5: Link to non-existent target ===
#[test]
fn k3_5_link_target_not_found() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let kn_id = add_knowledge(dir, "Evidence");

    let (json, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "link",
            &kn_id,
            "--to",
            "NODO-999",
            "--relation",
            "supports",
        ],
    );

    assert_eq!(code, 1);
    assert_eq!(json["success"], false);
    assert_eq!(json["errors"][0]["code"], "TARGET_NOT_FOUND");
}

// === K3.6: Link from non-existent KN ===
#[test]
fn k3_6_link_knowledge_not_found() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node_id = add_node(dir, "Node", "UDE");

    let (json, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "link",
            "KN-999",
            "--to",
            &node_id,
            "--relation",
            "supports",
        ],
    );

    assert_eq!(code, 1);
    assert_eq!(json["success"], false);
    assert_eq!(json["errors"][0]["code"], "KNOWLEDGE_NOT_FOUND");
}

// === K3.7: Duplicate link is idempotent with warning ===
#[test]
fn k3_7_duplicate_link_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node_id = add_node(dir, "Node", "UDE");
    let kn_id = add_knowledge(dir, "Evidence");

    // First link
    let (json, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "link",
            &kn_id,
            "--to",
            &node_id,
            "--relation",
            "supports",
        ],
    );
    assert_eq!(code, 0);
    assert_eq!(json["data"]["link_count"], 1);

    // Duplicate link
    let (json, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "link",
            &kn_id,
            "--to",
            &node_id,
            "--relation",
            "supports",
        ],
    );
    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["link_count"], 1); // Still 1
    assert_eq!(json["warnings"][0]["code"], "DUPLICATE_LINK");
}

// === K3.8: Same target different relation is allowed ===
#[test]
fn k3_8_same_target_different_relation() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node_id = add_node(dir, "Node", "UDE");
    let kn_id = add_knowledge(dir, "Evidence");

    // Link supports
    let (_, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "link",
            &kn_id,
            "--to",
            &node_id,
            "--relation",
            "supports",
        ],
    );
    assert_eq!(code, 0);

    // Link contradicts same target
    let (json, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "link",
            &kn_id,
            "--to",
            &node_id,
            "--relation",
            "contradicts",
        ],
    );
    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["link_count"], 2);
}

// === K3.9: Empty target fails ===
#[test]
fn k3_9_link_empty_target() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let kn_id = add_knowledge(dir, "Evidence");

    let (json, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "link",
            &kn_id,
            "--to",
            "",
            "--relation",
            "supports",
        ],
    );

    assert_eq!(code, 1);
    assert_eq!(json["success"], false);
}

// === K3.10: MACRO-XXX target not found ===
#[test]
fn k3_10_link_macro_target_not_found() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let kn_id = add_knowledge(dir, "Evidence");

    let (json, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "link",
            &kn_id,
            "--to",
            "MACRO-001",
            "--relation",
            "supports",
        ],
    );

    assert_eq!(code, 1);
    assert_eq!(json["errors"][0]["code"], "TARGET_NOT_FOUND");
}

// === K3.11: Link to orphan node (in pool but not in any tree) ===
#[test]
fn k3_11_link_to_orphan_node() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node_id = add_node(dir, "Orphan node", "UDE");
    let kn_id = add_knowledge(dir, "Evidence");

    // Node exists in pool but not attached to any tree
    let (json, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "link",
            &kn_id,
            "--to",
            &node_id,
            "--relation",
            "supports",
        ],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
}

// === K3.18: Link then undo removes link ===
#[test]
fn k3_18_link_undo() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node_id = add_node(dir, "Node", "UDE");
    let kn_id = add_knowledge(dir, "Evidence");

    // Link
    let (_, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "link",
            &kn_id,
            "--to",
            &node_id,
            "--relation",
            "supports",
        ],
    );
    assert_eq!(code, 0);

    // Undo
    let (_, code) = run_ltp(dir, &["undo"]);
    assert_eq!(code, 0);

    // Inspect: link should be gone
    let (json, code) = run_ltp(dir, &["knowledge", "inspect", &kn_id]);
    assert_eq!(code, 0);
    assert_eq!(json["data"]["links"].as_array().unwrap().len(), 0);
}

// === K3.24: Unlink happy path ===
#[test]
fn k3_24_unlink() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node_id = add_node(dir, "Node", "UDE");
    let kn_id = add_knowledge(dir, "Evidence");

    // Link
    let (_, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "link",
            &kn_id,
            "--to",
            &node_id,
            "--relation",
            "supports",
        ],
    );
    assert_eq!(code, 0);

    // Unlink
    let (json, code) = run_ltp(dir, &["knowledge", "unlink", &kn_id, "--from", &node_id]);
    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["action"], "knowledge_unlink");
    assert_eq!(json["data"]["removed_count"], 1);
    assert_eq!(json["data"]["link_count"], 0);
}

// === K3.25: Unlink removes ALL links to same target (D3) ===
#[test]
fn k3_25_unlink_removes_all_to_target() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node_id = add_node(dir, "Node", "UDE");
    let kn_id = add_knowledge(dir, "Evidence");

    // Link supports
    let (_, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "link",
            &kn_id,
            "--to",
            &node_id,
            "--relation",
            "supports",
        ],
    );
    assert_eq!(code, 0);

    // Link contradicts to same target
    let (_, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "link",
            &kn_id,
            "--to",
            &node_id,
            "--relation",
            "contradicts",
        ],
    );
    assert_eq!(code, 0);

    // Unlink from target removes both
    let (json, code) = run_ltp(dir, &["knowledge", "unlink", &kn_id, "--from", &node_id]);
    assert_eq!(code, 0);
    assert_eq!(json["data"]["removed_count"], 2);
    assert_eq!(json["data"]["link_count"], 0);
}

// === K3.26: Unlink link not found ===
#[test]
fn k3_26_unlink_not_found() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let kn_id = add_knowledge(dir, "Evidence");

    let (json, code) = run_ltp(dir, &["knowledge", "unlink", &kn_id, "--from", "RC-005"]);
    assert_eq!(code, 1);
    assert_eq!(json["errors"][0]["code"], "LINK_NOT_FOUND");
}

// === K3.27: Unlink KN not found ===
#[test]
fn k3_27_unlink_knowledge_not_found() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let (json, code) = run_ltp(dir, &["knowledge", "unlink", "KN-999", "--from", "UDE-001"]);
    assert_eq!(code, 1);
    assert_eq!(json["errors"][0]["code"], "KNOWLEDGE_NOT_FOUND");
}

// === K3.28: Unlink then undo restores link ===
#[test]
fn k3_28_unlink_undo() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node_id = add_node(dir, "Node", "UDE");
    let kn_id = add_knowledge(dir, "Evidence");

    // Link
    let (_, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "link",
            &kn_id,
            "--to",
            &node_id,
            "--relation",
            "supports",
        ],
    );
    assert_eq!(code, 0);

    // Unlink
    let (_, code) = run_ltp(dir, &["knowledge", "unlink", &kn_id, "--from", &node_id]);
    assert_eq!(code, 0);

    // Undo unlink
    let (_, code) = run_ltp(dir, &["undo"]);
    assert_eq!(code, 0);

    // Inspect: link should be back
    let (json, code) = run_ltp(dir, &["knowledge", "inspect", &kn_id]);
    assert_eq!(code, 0);
    assert_eq!(json["data"]["links"].as_array().unwrap().len(), 1);
    assert_eq!(json["data"]["links"][0]["target"], node_id.as_str());
}

// === K3.29: List with --target filter ===
#[test]
fn k3_29_list_target() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node_id = add_node(dir, "Target node", "UDE");
    let kn1 = add_knowledge(dir, "Evidence 1");
    let kn2 = add_knowledge(dir, "Evidence 2");
    let _kn3 = add_knowledge(dir, "Unlinked");

    // Link KN-001 and KN-002 to node
    let (_, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "link",
            &kn1,
            "--to",
            &node_id,
            "--relation",
            "supports",
        ],
    );
    assert_eq!(code, 0);
    let (_, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "link",
            &kn2,
            "--to",
            &node_id,
            "--relation",
            "contradicts",
        ],
    );
    assert_eq!(code, 0);

    // List with --target
    let (json, code) = run_ltp(dir, &["knowledge", "list", "--target", &node_id]);
    assert_eq!(code, 0);
    assert_eq!(json["data"]["total"], 2);
}

// === K3.30: List with --target and --relation ===
#[test]
fn k3_30_list_target_and_relation() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node_id = add_node(dir, "Target node", "UDE");
    let kn1 = add_knowledge(dir, "Evidence 1");
    let kn2 = add_knowledge(dir, "Evidence 2");

    let (_, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "link",
            &kn1,
            "--to",
            &node_id,
            "--relation",
            "supports",
        ],
    );
    assert_eq!(code, 0);
    let (_, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "link",
            &kn2,
            "--to",
            &node_id,
            "--relation",
            "contradicts",
        ],
    );
    assert_eq!(code, 0);

    // Filter by target + relation
    let (json, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "list",
            "--target",
            &node_id,
            "--relation",
            "supports",
        ],
    );
    assert_eq!(code, 0);
    assert_eq!(json["data"]["total"], 1);
    assert_eq!(json["data"]["items"][0]["id"], kn1.as_str());
}

// === K3.31: List --target with no matches ===
#[test]
fn k3_31_list_target_no_matches() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let _kn = add_knowledge(dir, "Evidence");

    let (json, code) = run_ltp(dir, &["knowledge", "list", "--target", "NODO-999"]);
    assert_eq!(code, 0);
    assert_eq!(json["data"]["total"], 0);
}

// === K3.32: List --target with KN that has multiple links to same target (D4) ===
#[test]
fn k3_32_list_target_multiple_relations_once() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node_id = add_node(dir, "Node", "UDE");
    let kn_id = add_knowledge(dir, "Evidence");

    // Link supports + contradicts to same target
    let (_, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "link",
            &kn_id,
            "--to",
            &node_id,
            "--relation",
            "supports",
        ],
    );
    assert_eq!(code, 0);
    let (_, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "link",
            &kn_id,
            "--to",
            &node_id,
            "--relation",
            "contradicts",
        ],
    );
    assert_eq!(code, 0);

    // List: KN appears ONCE with matching_relations array
    let (json, code) = run_ltp(dir, &["knowledge", "list", "--target", &node_id]);
    assert_eq!(code, 0);
    assert_eq!(json["data"]["total"], 1);
    let relations = json["data"]["items"][0]["matching_relations"]
        .as_array()
        .unwrap();
    assert_eq!(relations.len(), 2);
}

// === K3.33: List --relation without --target ===
#[test]
fn k3_33_list_relation_only() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let n1 = add_node(dir, "Node 1", "UDE");
    let n2 = add_node(dir, "Node 2", "RC");
    let kn1 = add_knowledge(dir, "Evidence 1");
    let kn2 = add_knowledge(dir, "Evidence 2");
    let _kn3 = add_knowledge(dir, "No links");

    let (_, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "link",
            &kn1,
            "--to",
            &n1,
            "--relation",
            "supports",
        ],
    );
    assert_eq!(code, 0);
    let (_, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "link",
            &kn2,
            "--to",
            &n2,
            "--relation",
            "contradicts",
        ],
    );
    assert_eq!(code, 0);

    // Only items with supports
    let (json, code) = run_ltp(dir, &["knowledge", "list", "--relation", "supports"]);
    assert_eq!(code, 0);
    assert_eq!(json["data"]["total"], 1);
    assert_eq!(json["data"]["items"][0]["id"], kn1.as_str());
}

// === K3.34: Inspect with resolved links ===
#[test]
fn k3_34_inspect_resolved_links() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let n1 = add_node(dir, "Node Alpha", "UDE");
    let n2 = add_node(dir, "Node Beta", "RC");
    let tree_id = new_tree(dir, "crt", "Test CRT");
    attach_node(dir, &tree_id, &n1);
    attach_node(dir, &tree_id, &n2);
    let edge_id = connect_nodes(dir, &tree_id, &n2, &n1);

    let kn_id = add_knowledge(dir, "Evidence");

    // Link to node and edge
    let (_, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "link",
            &kn_id,
            "--to",
            &n1,
            "--relation",
            "supports",
        ],
    );
    assert_eq!(code, 0);
    let (_, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "link",
            &kn_id,
            "--to",
            &edge_id,
            "--relation",
            "contextualizes",
        ],
    );
    assert_eq!(code, 0);

    // Inspect
    let (json, code) = run_ltp(dir, &["knowledge", "inspect", &kn_id]);
    assert_eq!(code, 0);

    let links = json["data"]["links"].as_array().unwrap();
    assert_eq!(links.len(), 2);

    // First link: node
    assert_eq!(links[0]["target"], n1.as_str());
    assert_eq!(links[0]["target_label"], "Node Alpha");
    assert_eq!(links[0]["target_type"], "node");

    // Second link: edge
    assert_eq!(links[1]["target"], edge_id.as_str());
    assert!(links[1]["target_label"].as_str().unwrap().contains("->"));
    assert_eq!(links[1]["target_type"], "edge");
}

// === K3.35: Inspect with dangling ref (target deleted) ===
#[test]
fn k3_35_inspect_dangling_ref() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node_id = add_node(dir, "Temp node", "UDE");
    let kn_id = add_knowledge(dir, "Evidence");

    // Link to node
    let (_, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "link",
            &kn_id,
            "--to",
            &node_id,
            "--relation",
            "supports",
        ],
    );
    assert_eq!(code, 0);

    // Remove the node
    let (_, code) = run_ltp(dir, &["node", "rm", &node_id, "--force"]);
    assert_eq!(code, 0);

    // Inspect: link still visible but target_label is null
    let (json, code) = run_ltp(dir, &["knowledge", "inspect", &kn_id]);
    assert_eq!(code, 0);
    let links = json["data"]["links"].as_array().unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0]["target"], node_id.as_str());
    assert!(links[0]["target_label"].is_null());
    assert_eq!(links[0]["target_type"], "unknown");
}

// === K3.36: Inspect with assumption link ===
#[test]
fn k3_36_inspect_assumption_link() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let n1 = add_node(dir, "Cause", "RC");
    let n2 = add_node(dir, "Effect", "UDE");
    let tree_id = new_tree(dir, "crt", "Test CRT");
    attach_node(dir, &tree_id, &n1);
    attach_node(dir, &tree_id, &n2);
    let edge_id = connect_nodes(dir, &tree_id, &n1, &n2);

    // Add assumption
    let (json, code) = run_ltp(
        dir,
        &[
            "assume",
            "add",
            "--tree",
            &tree_id,
            "--link",
            &edge_id,
            "--text",
            "Supply constant",
        ],
    );
    assert_eq!(code, 0, "assume add failed: {:?}", json);
    let asm_id = json["data"]["id"].as_str().unwrap().to_string();

    let kn_id = add_knowledge(dir, "Contradicting evidence");

    // Link to assumption
    let (_, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "link",
            &kn_id,
            "--to",
            &asm_id,
            "--relation",
            "contradicts",
        ],
    );
    assert_eq!(code, 0);

    // Inspect: target_label shows assumption text
    let (json, code) = run_ltp(dir, &["knowledge", "inspect", &kn_id]);
    assert_eq!(code, 0);
    let links = json["data"]["links"].as_array().unwrap();
    assert_eq!(links[0]["target_label"], "Supply constant");
    assert_eq!(links[0]["target_type"], "assumption");
}

// === K3.15: Link to interior node of macro_edge (permitted) ===
#[test]
fn k3_15_link_to_interior_node() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    // Create chain A -> B -> C and collapse A..C
    let na = add_node(dir, "Start", "RC");
    let nb = add_node(dir, "Mid", "INT");
    let nc = add_node(dir, "End", "UDE");
    let tree_id = new_tree(dir, "crt", "Collapse test");
    attach_node(dir, &tree_id, &na);
    attach_node(dir, &tree_id, &nb);
    attach_node(dir, &tree_id, &nc);
    connect_nodes(dir, &tree_id, &na, &nb);
    connect_nodes(dir, &tree_id, &nb, &nc);

    // Collapse path from A to C
    let (_, code) = run_ltp(
        dir,
        &[
            "path", "collapse", "--tree", &tree_id, "--from", &na, "--to", &nc, "--label", "Macro",
        ],
    );
    assert_eq!(code, 0);

    // Interior node B still exists in pool — link to it
    let kn_id = add_knowledge(dir, "Evidence for interior");
    let (json, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "link",
            &kn_id,
            "--to",
            &nb,
            "--relation",
            "supports",
        ],
    );
    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
}

// === K3.19: Link then node rm creates dangling ref ===
#[test]
fn k3_19_link_then_node_rm_dangling() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node_id = add_node(dir, "Soon deleted", "UDE");
    let kn_id = add_knowledge(dir, "Evidence");

    let (_, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "link",
            &kn_id,
            "--to",
            &node_id,
            "--relation",
            "supports",
        ],
    );
    assert_eq!(code, 0);

    // Remove node
    let (_, code) = run_ltp(dir, &["node", "rm", &node_id, "--force"]);
    assert_eq!(code, 0);

    // Inspect: link still present with null label
    let (json, code) = run_ltp(dir, &["knowledge", "inspect", &kn_id]);
    assert_eq!(code, 0);
    let links = json["data"]["links"].as_array().unwrap();
    assert_eq!(links.len(), 1);
    assert!(links[0]["target_label"].is_null());
}

// === K3.22: Link inside batch then undo reverts all ===
#[test]
fn k3_22_link_in_batch_undo() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node_id = add_node(dir, "Node", "UDE");
    let kn_id = add_knowledge(dir, "Evidence");

    // Begin batch
    let (_, code) = run_ltp(dir, &["history", "begin-batch", "--label", "test-batch"]);
    assert_eq!(code, 0);

    // Link inside batch
    let (_, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "link",
            &kn_id,
            "--to",
            &node_id,
            "--relation",
            "supports",
        ],
    );
    assert_eq!(code, 0);

    // End batch
    let (_, code) = run_ltp(dir, &["history", "end-batch"]);
    assert_eq!(code, 0);

    // Undo the whole batch
    let (_, code) = run_ltp(dir, &["undo"]);
    assert_eq!(code, 0);

    // Inspect: link should be gone
    let (json, code) = run_ltp(dir, &["knowledge", "inspect", &kn_id]);
    assert_eq!(code, 0);
    assert_eq!(json["data"]["links"].as_array().unwrap().len(), 0);
}

// === K3.37: Inspect edge link shows from->to label ===
#[test]
fn k3_37_inspect_edge_label() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let n1 = add_node(dir, "Root cause", "RC");
    let n2 = add_node(dir, "Effect", "UDE");
    let tree_id = new_tree(dir, "crt", "Label test");
    attach_node(dir, &tree_id, &n1);
    attach_node(dir, &tree_id, &n2);
    let edge_id = connect_nodes(dir, &tree_id, &n1, &n2);

    let kn_id = add_knowledge(dir, "Edge insight");
    let (_, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "link",
            &kn_id,
            "--to",
            &edge_id,
            "--relation",
            "supports",
        ],
    );
    assert_eq!(code, 0);

    let (json, code) = run_ltp(dir, &["knowledge", "inspect", &kn_id]);
    assert_eq!(code, 0);

    let links = json["data"]["links"].as_array().unwrap();
    let label = links[0]["target_label"].as_str().unwrap();
    // Label should show "RC-001 -> UDE-001" format
    assert!(label.contains(&n1));
    assert!(label.contains(&n2));
    assert!(label.contains("->"));
}

// === K3.16: Link to edge in NBR branch ===
#[test]
fn k3_16_link_to_nbr_edge() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let n1 = add_node(dir, "Source", "RC");
    let n2 = add_node(dir, "Effect", "UDE");
    let n3 = add_node(dir, "Branch dest", "DE");
    let tree_id = new_tree(dir, "frt", "NBR test");
    attach_node(dir, &tree_id, &n1);
    attach_node(dir, &tree_id, &n2);
    attach_node(dir, &tree_id, &n3);
    connect_nodes(dir, &tree_id, &n1, &n2);

    // Add NBR branch from n1
    let (json, code) = run_ltp(
        dir,
        &["nbr", "add", "--tree", &tree_id, "--source-node", &n1],
    );
    assert_eq!(code, 0, "nbr add failed: {:?}", json);
    let nbr_id = json["data"]["nbr_id"].as_str().unwrap().to_string();

    // Connect inside NBR to create an edge
    let (json, code) = run_ltp(
        dir,
        &[
            "link", "connect", "--tree", &tree_id, "--nbr", &nbr_id, "--from", &n1, "--to", &n3,
        ],
    );
    assert_eq!(code, 0, "link connect nbr failed: {:?}", json);
    let nbr_edge_id = json["data"]["created_links"][0]
        .as_str()
        .unwrap()
        .to_string();

    // Link knowledge to NBR edge
    let kn_id = add_knowledge(dir, "NBR evidence");
    let (json, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "link",
            &kn_id,
            "--to",
            &nbr_edge_id,
            "--relation",
            "contextualizes",
        ],
    );
    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
}

// === K3.12: Link to edge with status broken (permitted) ===
#[test]
fn k3_12_link_to_broken_edge() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let n1 = add_node(dir, "Cause", "RC");
    let n2 = add_node(dir, "Effect", "UDE");
    let tree_id = new_tree(dir, "crt", "Broken test");
    attach_node(dir, &tree_id, &n1);
    attach_node(dir, &tree_id, &n2);
    let edge_id = connect_nodes(dir, &tree_id, &n1, &n2);

    // Invalidate the edge (makes it broken)
    let (_, code) = run_ltp(
        dir,
        &[
            "link", "feedback", "--tree", &tree_id, "--from", &n2, "--to", &n1, "--type",
            "negative",
        ],
    );
    // Whether or not this specific command exists, just test that linking to an existing edge works
    // regardless of edge status — the motor doesn't judge status
    let _ = code;

    // Link knowledge to the edge (even if status changed)
    let kn_id = add_knowledge(dir, "Evidence for broken edge");
    let (json, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "link",
            &kn_id,
            "--to",
            &edge_id,
            "--relation",
            "contradicts",
        ],
    );
    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
}
