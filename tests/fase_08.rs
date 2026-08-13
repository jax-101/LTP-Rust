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
    let (_, code) = run_ltp(dir, &["init", "--name", "F8Test"]);
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

/// Build a 3-node chain: RC-001 → INT-001 → UDE-001 in a CRT
fn chain_setup(dir: &std::path::Path) -> (String, Vec<String>, Vec<String>) {
    setup_workspace(dir);
    let rc = add_node(dir, "Root Cause", "RC");
    let int = add_node(dir, "Intermediate", "INT");
    let ude = add_node(dir, "Undesirable Effect", "UDE");
    let tree = create_tree(dir, "crt", "TraceCRT");
    attach_node(dir, &tree, &rc);
    attach_node(dir, &tree, &int);
    attach_node(dir, &tree, &ude);
    let l1 = connect(dir, &tree, &rc, &int);
    let l2 = connect(dir, &tree, &int, &ude);
    (tree, vec![rc, int, ude], vec![l1, l2])
}

// --- UAT 8.1: Trace downstream full chain ---
#[test]
fn uat_8_1_trace_downstream_full_chain() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree, nodes, _links) = chain_setup(dir);

    let (json, code) = run_ltp(
        dir,
        &[
            "trace",
            &nodes[0],
            "--tree",
            &tree,
            "--direction",
            "downstream",
        ],
    );
    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["action"], "trace");

    let chain = json["data"]["chain"].as_array().unwrap();
    assert!(chain.len() >= 3);

    // Verify all nodes appear in chain
    let chain_nodes: Vec<&str> = chain.iter().map(|e| e["node"].as_str().unwrap()).collect();
    assert!(chain_nodes.contains(&nodes[0].as_str()));
    assert!(chain_nodes.contains(&nodes[1].as_str()));
    assert!(chain_nodes.contains(&nodes[2].as_str()));

    // Each entry except terminal should have link_to_next with link_status
    for entry in chain {
        if entry["link_to_next"].is_object() {
            assert!(entry["link_to_next"]["status"].as_str().is_some());
        }
    }
}

// --- UAT 8.2: Trace upstream to roots ---
#[test]
fn uat_8_2_trace_upstream_to_roots() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree, nodes, _links) = chain_setup(dir);

    let (json, code) = run_ltp(
        dir,
        &[
            "trace",
            &nodes[2],
            "--tree",
            &tree,
            "--direction",
            "upstream",
        ],
    );
    assert_eq!(code, 0);
    assert_eq!(json["success"], true);

    let chain = json["data"]["chain"].as_array().unwrap();
    let chain_nodes: Vec<&str> = chain.iter().map(|e| e["node"].as_str().unwrap()).collect();
    assert!(chain_nodes.contains(&nodes[0].as_str()));
}

// --- UAT 8.3: Trace upstream with depth limit ---
#[test]
fn uat_8_3_trace_upstream_depth_limit() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree, nodes, _links) = chain_setup(dir);

    // UDE → upstream depth 1 should NOT reach RC (2 hops away)
    let (json, code) = run_ltp(
        dir,
        &[
            "trace",
            &nodes[2],
            "--tree",
            &tree,
            "--direction",
            "upstream",
            "--depth",
            "1",
        ],
    );
    assert_eq!(code, 0);
    assert_eq!(json["success"], true);

    let chain = json["data"]["chain"].as_array().unwrap();
    let chain_nodes: Vec<&str> = chain.iter().map(|e| e["node"].as_str().unwrap()).collect();
    // Should include UDE and INT but not RC
    assert!(chain_nodes.contains(&nodes[2].as_str()));
    assert!(chain_nodes.contains(&nodes[1].as_str()));
    assert!(!chain_nodes.contains(&nodes[0].as_str()));
}

// --- UAT 8.4: Trace includes feedback loops by default ---
#[test]
fn uat_8_4_trace_includes_feedback_by_default() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree, nodes, _links) = chain_setup(dir);

    // Add a feedback edge: UDE → RC (positive loop)
    let (_, code) = run_ltp(
        dir,
        &[
            "link", "feedback", "--tree", &tree, "--from", &nodes[2], "--to", &nodes[0], "--type",
            "positive",
        ],
    );
    assert_eq!(code, 0);

    let (json, code) = run_ltp(
        dir,
        &[
            "trace",
            &nodes[0],
            "--tree",
            &tree,
            "--direction",
            "downstream",
        ],
    );
    assert_eq!(code, 0);
    let feedback = json["data"]["feedback_loops"].as_array().unwrap();
    assert!(!feedback.is_empty());
    assert_eq!(feedback[0]["loop_type"], "positive");
}

// --- UAT 8.5: Trace excludes feedback with --no-feedback ---
#[test]
fn uat_8_5_trace_excludes_feedback() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree, nodes, _links) = chain_setup(dir);

    // Add feedback
    let (_, code) = run_ltp(
        dir,
        &[
            "link", "feedback", "--tree", &tree, "--from", &nodes[2], "--to", &nodes[0], "--type",
            "positive",
        ],
    );
    assert_eq!(code, 0);

    let (json, code) = run_ltp(
        dir,
        &[
            "trace",
            &nodes[0],
            "--tree",
            &tree,
            "--direction",
            "downstream",
            "--no-feedback",
        ],
    );
    assert_eq!(code, 0);
    let feedback = json["data"]["feedback_loops"].as_array().unwrap();
    assert!(feedback.is_empty());
}

// --- UAT 8.6: Trace includes NBR edges with --nbr ---
#[test]
fn uat_8_6_trace_includes_nbr_edges() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let inj = add_node(dir, "Injection", "INJ");
    let nde = add_node(dir, "Negative Effect", "DE");
    let tree = create_tree(dir, "frt", "NBRTree");
    attach_node(dir, &tree, &inj);
    attach_node(dir, &tree, &nde);

    // Manually inject an NBR branch into the tree JSON (nbr CLI not yet implemented)
    let tree_path = dir.join("trees").join(format!("{}.json", tree));
    let tree_json = std::fs::read_to_string(&tree_path).unwrap();
    let mut tree_val: serde_json::Value = serde_json::from_str(&tree_json).unwrap();
    tree_val["nbr_branches"] = serde_json::json!([{
        "id": "NBR-001",
        "source_node": inj,
        "edges": [{
            "id": "LINK-NBR-001",
            "from": [&inj],
            "to": &nde,
            "operator": "SINGLE",
            "weight": null,
            "status": "active",
            "logic": "SUFFICIENCY",
            "assumptions": []
        }],
        "trim_injection": null
    }]);
    std::fs::write(&tree_path, serde_json::to_string_pretty(&tree_val).unwrap()).unwrap();

    // Trace without --nbr: should NOT include NDE (only trunk edges)
    let (json, code) = run_ltp(
        dir,
        &["trace", &inj, "--tree", &tree, "--direction", "downstream"],
    );
    assert_eq!(code, 0);
    let chain = json["data"]["chain"].as_array().unwrap();
    let chain_nodes: Vec<&str> = chain.iter().map(|e| e["node"].as_str().unwrap()).collect();
    assert!(!chain_nodes.contains(&nde.as_str()));

    // Trace with --nbr: should include NDE
    let (json, code) = run_ltp(
        dir,
        &[
            "trace",
            &inj,
            "--tree",
            &tree,
            "--direction",
            "downstream",
            "--nbr",
        ],
    );
    assert_eq!(code, 0);
    let chain = json["data"]["chain"].as_array().unwrap();
    let chain_nodes: Vec<&str> = chain.iter().map(|e| e["node"].as_str().unwrap()).collect();
    assert!(chain_nodes.contains(&nde.as_str()));
}

// --- UAT 8.7: Link inspect full detail ---
#[test]
fn uat_8_7_link_inspect_full_detail() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree, nodes, links) = chain_setup(dir);

    let (json, code) = run_ltp(dir, &["link", "inspect", &links[0], "--tree", &tree]);
    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["action"], "link_inspect");
    assert_eq!(json["data"]["id"], links[0]);
    assert_eq!(json["data"]["from"][0], nodes[0]);
    assert_eq!(json["data"]["to"], nodes[1]);
    assert!(json["data"]["from_labels"][0]["label"].as_str().is_some());
    assert!(json["data"]["to_label"].as_str().is_some());
    assert_eq!(json["data"]["operator"], "SINGLE");
    assert_eq!(json["data"]["status"], "active");
    assert_eq!(json["data"]["logic"], "sufficiency");
}

// --- UAT 8.8: Link find between nodes ---
#[test]
fn uat_8_8_link_find_between_nodes() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree, nodes, links) = chain_setup(dir);

    let (json, code) = run_ltp(
        dir,
        &[
            "link", "find", "--tree", &tree, "--from", &nodes[0], "--to", &nodes[1],
        ],
    );
    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    let found = json["data"]["links"].as_array().unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0]["id"], links[0]);

    // Find between non-connected nodes: empty array
    let (json, code) = run_ltp(
        dir,
        &[
            "link", "find", "--tree", &tree, "--from", &nodes[0], "--to", &nodes[2],
        ],
    );
    assert_eq!(code, 0);
    let found = json["data"]["links"].as_array().unwrap();
    assert!(found.is_empty());
}

// --- UAT 8.9: Trace node not found ---
#[test]
fn uat_8_9_trace_node_not_found() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree, _nodes, _links) = chain_setup(dir);

    let (json, code) = run_ltp(
        dir,
        &[
            "trace",
            "NODO-999",
            "--tree",
            &tree,
            "--direction",
            "upstream",
        ],
    );
    assert_ne!(code, 0);
    assert_eq!(json["success"], false);
    assert_eq!(json["errors"][0]["code"], "NODE_NOT_FOUND");
}

// --- UAT 8.10: Trace tree not found ---
#[test]
fn uat_8_10_trace_tree_not_found() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (_tree, nodes, _links) = chain_setup(dir);

    let (json, code) = run_ltp(
        dir,
        &[
            "trace",
            &nodes[0],
            "--tree",
            "tree-inexistente",
            "--direction",
            "upstream",
        ],
    );
    assert_ne!(code, 0);
    assert_eq!(json["success"], false);
    assert_eq!(json["errors"][0]["code"], "TREE_NOT_FOUND");
}

// --- UAT 8.11: Trace node not in tree ---
#[test]
fn uat_8_11_trace_node_not_in_tree() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree, _nodes, _links) = chain_setup(dir);

    // Create a node but don't attach it
    let detached = add_node(dir, "Detached Node", "UDE");

    let (json, code) = run_ltp(
        dir,
        &[
            "trace",
            &detached,
            "--tree",
            &tree,
            "--direction",
            "upstream",
        ],
    );
    assert_ne!(code, 0);
    assert_eq!(json["success"], false);
    assert_eq!(json["errors"][0]["code"], "NODE_NOT_IN_TREE");
}

// --- UAT 8.12: Trace leaf node (no downstream) → empty chain ---
#[test]
fn uat_8_12_trace_leaf_node_empty_chain() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree, nodes, _links) = chain_setup(dir);

    // UDE is a leaf (no outgoing edges downstream)
    let (json, code) = run_ltp(
        dir,
        &[
            "trace",
            &nodes[2],
            "--tree",
            &tree,
            "--direction",
            "downstream",
        ],
    );
    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    let chain = json["data"]["chain"].as_array().unwrap();
    // Only the start node with no link_to_next
    assert_eq!(chain.len(), 1);
    assert_eq!(chain[0]["node"], nodes[2]);
    assert!(chain[0]["link_to_next"].is_null());
}

// --- UAT 8.13: Trace root node (no upstream) → empty chain ---
#[test]
fn uat_8_13_trace_root_node_empty_chain() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree, nodes, _links) = chain_setup(dir);

    // RC is a root (no incoming edges upstream)
    let (json, code) = run_ltp(
        dir,
        &[
            "trace",
            &nodes[0],
            "--tree",
            &tree,
            "--direction",
            "upstream",
        ],
    );
    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    let chain = json["data"]["chain"].as_array().unwrap();
    assert_eq!(chain.len(), 1);
    assert_eq!(chain[0]["node"], nodes[0]);
    assert!(chain[0]["link_to_next"].is_null());
}

// --- UAT 8.14: Trace with broken link → chain_health reports it ---
#[test]
fn uat_8_14_trace_broken_link_chain_health() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree, nodes, links) = chain_setup(dir);

    // Add an assumption and invalidate it to break the link
    let (json, code) = run_ltp(
        dir,
        &[
            "assume",
            "add",
            "--tree",
            &tree,
            "--link",
            &links[1],
            "--text",
            "Test assumption",
        ],
    );
    assert_eq!(code, 0);
    let asm_id = json["data"]["id"].as_str().unwrap().to_string();

    let (_, code) = run_ltp(
        dir,
        &[
            "invalidate",
            "--tree",
            &tree,
            "--link",
            &links[1],
            "--asm",
            &asm_id,
        ],
    );
    assert_eq!(code, 0);

    // Trace downstream from RC should include the broken link
    let (json, code) = run_ltp(
        dir,
        &[
            "trace",
            &nodes[0],
            "--tree",
            &tree,
            "--direction",
            "downstream",
        ],
    );
    assert_eq!(code, 0);
    assert_eq!(json["data"]["chain_health"]["fully_connected"], false);

    let broken = json["data"]["chain_health"]["broken_links"]
        .as_array()
        .unwrap();
    assert!(broken.iter().any(|v| v.as_str().unwrap() == links[1]));
}

// --- UAT 8.15: Link inspect with assumptions ---
#[test]
fn uat_8_15_link_inspect_with_assumptions() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree, _nodes, links) = chain_setup(dir);

    // Add 2 assumptions
    let (json, code) = run_ltp(
        dir,
        &[
            "assume",
            "add",
            "--tree",
            &tree,
            "--link",
            &links[0],
            "--text",
            "First assumption",
        ],
    );
    assert_eq!(code, 0);
    let asm1 = json["data"]["id"].as_str().unwrap().to_string();

    let (json, code) = run_ltp(
        dir,
        &[
            "assume",
            "add",
            "--tree",
            &tree,
            "--link",
            &links[0],
            "--text",
            "Second assumption",
        ],
    );
    assert_eq!(code, 0);
    let asm2 = json["data"]["id"].as_str().unwrap().to_string();

    // Invalidate the first one
    let (_, code) = run_ltp(
        dir,
        &[
            "invalidate",
            "--tree",
            &tree,
            "--link",
            &links[0],
            "--asm",
            &asm1,
        ],
    );
    assert_eq!(code, 0);

    // Inspect the link
    let (json, code) = run_ltp(dir, &["link", "inspect", &links[0], "--tree", &tree]);
    assert_eq!(code, 0);

    let assumptions = json["data"]["assumptions"].as_array().unwrap();
    assert_eq!(assumptions.len(), 2);

    // One should be invalid, one valid
    let statuses: Vec<&str> = assumptions
        .iter()
        .map(|a| a["status"].as_str().unwrap())
        .collect();
    assert!(statuses.contains(&"invalid"));
    assert!(statuses.contains(&"valid"));

    // Verify IDs match
    let ids: Vec<&str> = assumptions
        .iter()
        .map(|a| a["id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&asm1.as_str()));
    assert!(ids.contains(&asm2.as_str()));
}
