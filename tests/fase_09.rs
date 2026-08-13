use std::process::Command;

use serde_json::Value;

fn ltp_bin() -> String {
    env!("CARGO_BIN_EXE_ltp-engine").to_string()
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
    let (_, code) = run_ltp(dir, &["init", "--name", "F9Test"]);
    assert_eq!(code, 0);
}

fn add_node(dir: &std::path::Path, label: &str, node_type: &str) -> String {
    let (json, code) = run_ltp(dir, &["node", "add", label, "--type", node_type]);
    assert_eq!(code, 0, "node add failed: {:?}", json);
    json["data"]["id"].as_str().unwrap().to_string()
}

fn create_tree(dir: &std::path::Path, tree_type: &str, name: &str) -> String {
    let (json, code) = run_ltp(dir, &["tree", "new", tree_type, name]);
    assert_eq!(code, 0, "tree new failed: {:?}", json);
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
    assert_eq!(code, 0, "link connect failed: {:?}", json);
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
    assert_eq!(code, 0, "assume add failed: {:?}", json);
    json["data"]["id"].as_str().unwrap().to_string()
}

/// Build a linear chain: A → B → C → D → E
fn linear_chain_setup(dir: &std::path::Path) -> (String, Vec<String>, Vec<String>) {
    setup_workspace(dir);
    let a = add_node(dir, "Node A", "RC");
    let b = add_node(dir, "Node B", "INT");
    let c = add_node(dir, "Node C", "INT");
    let d = add_node(dir, "Node D", "INT");
    let e = add_node(dir, "Node E", "UDE");
    let tree = create_tree(dir, "crt", "TestCRT");
    for n in [&a, &b, &c, &d, &e] {
        attach_node(dir, &tree, n);
    }
    let l1 = connect(dir, &tree, &a, &b);
    let l2 = connect(dir, &tree, &b, &c);
    let l3 = connect(dir, &tree, &c, &d);
    let l4 = connect(dir, &tree, &d, &e);
    (tree, vec![a, b, c, d, e], vec![l1, l2, l3, l4])
}

/// Build a diamond: A→B→D→E, A→C→D→E
fn diamond_setup(dir: &std::path::Path) -> (String, Vec<String>, Vec<String>) {
    setup_workspace(dir);
    let a = add_node(dir, "Node A", "RC");
    let b = add_node(dir, "Node B", "INT");
    let c = add_node(dir, "Node C", "INT");
    let d = add_node(dir, "Node D", "INT");
    let e = add_node(dir, "Node E", "UDE");
    let tree = create_tree(dir, "crt", "DiamondCRT");
    for n in [&a, &b, &c, &d, &e] {
        attach_node(dir, &tree, n);
    }
    let l1 = connect(dir, &tree, &a, &b);
    let l2 = connect(dir, &tree, &a, &c);
    let l3 = connect(dir, &tree, &b, &d);
    let l4 = connect(dir, &tree, &c, &d);
    let l5 = connect(dir, &tree, &d, &e);
    (tree, vec![a, b, c, d, e], vec![l1, l2, l3, l4, l5])
}

// --- UAT 9.1: Collapse creates macro_edge with interior_nodes ---
#[test]
fn uat_9_1_collapse_creates_macro_edge() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, nodes, links) = linear_chain_setup(dir.path());

    let (json, code) = run_ltp(
        dir.path(),
        &[
            "path",
            "collapse",
            "--tree",
            &tree,
            "--from",
            &nodes[0],
            "--to",
            &nodes[4],
            "--label",
            "Cadena logística",
        ],
    );
    assert_eq!(code, 0, "collapse failed: {:?}", json);
    assert!(json["success"].as_bool().unwrap());
    assert_eq!(json["action"], "path_collapse");

    let data = &json["data"];
    assert!(!data["macro_edge_id"].as_str().unwrap().is_empty());
    assert_eq!(data["from"], nodes[0]);
    assert_eq!(data["to"], nodes[4]);

    // Interior nodes: B, C, D
    let interior: Vec<String> = data["interior_nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(interior.len(), 3);
    assert!(interior.contains(&nodes[1]));
    assert!(interior.contains(&nodes[2]));
    assert!(interior.contains(&nodes[3]));

    // Interior links: all 4 links
    let int_links: Vec<String> = data["interior_links"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(int_links.len(), 4);
    for l in &links {
        assert!(int_links.contains(l));
    }
}

// --- UAT 9.2: tree walk shows macro_edge after collapse ---
#[test]
fn uat_9_2_tree_walk_shows_macro_edge() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, nodes, _links) = linear_chain_setup(dir.path());

    run_ltp(
        dir.path(),
        &[
            "path",
            "collapse",
            "--tree",
            &tree,
            "--from",
            &nodes[0],
            "--to",
            &nodes[4],
            "--label",
            "Cadena logística",
        ],
    );

    let (json, code) = run_ltp(dir.path(), &["tree", "walk", &tree]);
    assert_eq!(code, 0);

    // The walk output should still include all nodes (tactical nodes intact)
    let walk_data = &json["data"];
    let walk_nodes = walk_data["nodes"].as_array().unwrap();
    assert_eq!(walk_nodes.len(), 5);

    // Verify macro_edge persisted by reading the tree file on disk
    let tree_path = dir.path().join("trees").join(format!("{}.json", tree));
    let raw = std::fs::read_to_string(&tree_path).unwrap();
    let tree_json: Value = serde_json::from_str(&raw).unwrap();
    let macros = tree_json["macro_edges"].as_array().unwrap();
    assert_eq!(macros.len(), 1);
    assert_eq!(macros[0]["label"], "Cadena logística");
}

// --- UAT 9.3: Explode splits edge and creates INT node ---
#[test]
fn uat_9_3_explode_creates_intermediate_node() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, _nodes, links) = linear_chain_setup(dir.path());

    // Add assumption to first link
    let asm = add_assumption(dir.path(), &tree, &links[0], "Transport capacity stable");

    let (json, code) = run_ltp(
        dir.path(),
        &[
            "path",
            "explode",
            "--tree",
            &tree,
            "--link",
            &links[0],
            "--asm",
            &asm,
            "--label",
            "Nodo intermedio",
        ],
    );
    assert_eq!(code, 0, "explode failed: {:?}", json);
    assert!(json["success"].as_bool().unwrap());
    assert_eq!(json["action"], "path_explode");

    let data = &json["data"];
    let int_id = data["created_node_id"].as_str().unwrap();
    assert!(int_id.starts_with("INT-"));

    let created_links: Vec<String> = data["created_links"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(created_links.len(), 2);
    assert_eq!(data["removed_assumption"], asm);
    assert_eq!(data["original_link_removed"], links[0]);

    // Verify the new node exists
    let (node_json, nc) = run_ltp(dir.path(), &["node", "inspect", int_id]);
    assert_eq!(nc, 0);
    assert_eq!(node_json["data"]["label"], "Nodo intermedio");
    assert_eq!(node_json["data"]["node_type"], "INT");
}

// --- UAT 9.4: Replace substitutes macro_edge sub-graph ---
#[test]
fn uat_9_4_replace_substitutes_subgraph() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, nodes, _links) = linear_chain_setup(dir.path());

    // Collapse first
    let (coll_json, _) = run_ltp(
        dir.path(),
        &[
            "path",
            "collapse",
            "--tree",
            &tree,
            "--from",
            &nodes[0],
            "--to",
            &nodes[4],
            "--label",
            "Full chain",
        ],
    );
    let macro_id = coll_json["data"]["macro_edge_id"].as_str().unwrap();

    // Create injection node
    let inj = add_node(dir.path(), "Injection fix", "INJ");

    let (json, code) = run_ltp(
        dir.path(),
        &[
            "path",
            "replace",
            "--tree",
            &tree,
            "--macro-link",
            macro_id,
            "--by-node",
            &inj,
        ],
    );
    assert_eq!(code, 0, "replace failed: {:?}", json);
    assert!(json["success"].as_bool().unwrap());

    let data = &json["data"];
    assert_eq!(data["macro_link"], macro_id);
    assert_eq!(data["by_node"], inj);

    let sup_links: Vec<String> = data["superseded_links"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(sup_links.len(), 4);

    let sup_nodes: Vec<String> = data["superseded_nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(sup_nodes.len(), 3);

    let new_links: Vec<String> = data["new_links"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(new_links.len(), 2);
}

// --- UAT 9.5: Collapse with no directed path → error ---
#[test]
fn uat_9_5_collapse_no_directed_path() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, nodes, _links) = linear_chain_setup(dir.path());

    // Try to collapse from E → A (reverse direction)
    let (json, code) = run_ltp(
        dir.path(),
        &[
            "path",
            "collapse",
            "--tree",
            &tree,
            "--from",
            &nodes[4],
            "--to",
            &nodes[0],
            "--label",
            "Should fail",
        ],
    );
    assert_ne!(code, 0);
    assert!(!json["success"].as_bool().unwrap());
    assert_eq!(json["errors"][0]["code"], "NO_DIRECTED_PATH");
}

// --- UAT 9.6: Collapse with diamond (full sub-graph) ---
#[test]
fn uat_9_6_collapse_diamond_subgraph() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, nodes, links) = diamond_setup(dir.path());

    let (json, code) = run_ltp(
        dir.path(),
        &[
            "path",
            "collapse",
            "--tree",
            &tree,
            "--from",
            &nodes[0],
            "--to",
            &nodes[4],
            "--label",
            "Diamond collapse",
        ],
    );
    assert_eq!(code, 0, "diamond collapse failed: {:?}", json);

    let data = &json["data"];
    let interior: Vec<String> = data["interior_nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    // B, C, D are interior
    assert_eq!(interior.len(), 3);
    assert!(interior.contains(&nodes[1]));
    assert!(interior.contains(&nodes[2]));
    assert!(interior.contains(&nodes[3]));

    let int_links: Vec<String> = data["interior_links"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    // All 5 links are interior (they connect within the subgraph)
    assert_eq!(int_links.len(), 5);
    for l in &links {
        assert!(int_links.contains(l));
    }
}

// --- UAT 9.7: Explode with non-existent assumption → error ---
#[test]
fn uat_9_7_explode_assumption_not_in_link() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, _nodes, links) = linear_chain_setup(dir.path());

    let (json, code) = run_ltp(
        dir.path(),
        &[
            "path",
            "explode",
            "--tree",
            &tree,
            "--link",
            &links[0],
            "--asm",
            "ASM-999",
            "--label",
            "Should fail",
        ],
    );
    assert_ne!(code, 0);
    assert!(!json["success"].as_bool().unwrap());
    assert_eq!(json["errors"][0]["code"], "ASSUMPTION_NOT_IN_LINK");
}

// --- UAT 9.8: Replace with non-existent macro_edge → error ---
#[test]
fn uat_9_8_replace_macro_edge_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, _nodes, _links) = linear_chain_setup(dir.path());

    let inj = add_node(dir.path(), "Injection", "INJ");

    let (json, code) = run_ltp(
        dir.path(),
        &[
            "path",
            "replace",
            "--tree",
            &tree,
            "--macro-link",
            "MACRO-999",
            "--by-node",
            &inj,
        ],
    );
    assert_ne!(code, 0);
    assert!(!json["success"].as_bool().unwrap());
    assert_eq!(json["errors"][0]["code"], "MACRO_EDGE_NOT_FOUND");
}

// --- UAT 9.9: Replace with non-existent node → error ---
#[test]
fn uat_9_9_replace_node_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, nodes, _links) = linear_chain_setup(dir.path());

    // Create a macro_edge first
    let (coll_json, _) = run_ltp(
        dir.path(),
        &[
            "path", "collapse", "--tree", &tree, "--from", &nodes[0], "--to", &nodes[4], "--label",
            "Test",
        ],
    );
    let macro_id = coll_json["data"]["macro_edge_id"].as_str().unwrap();

    let (json, code) = run_ltp(
        dir.path(),
        &[
            "path",
            "replace",
            "--tree",
            &tree,
            "--macro-link",
            macro_id,
            "--by-node",
            "NODO-999",
        ],
    );
    assert_ne!(code, 0);
    assert!(!json["success"].as_bool().unwrap());
    assert_eq!(json["errors"][0]["code"], "NODE_NOT_FOUND");
}

// --- UAT 9.10: Collapse single direct edge (degenerate case) ---
#[test]
fn uat_9_10_collapse_single_edge() {
    let dir = tempfile::tempdir().unwrap();
    setup_workspace(dir.path());

    let a = add_node(dir.path(), "Node A", "RC");
    let b = add_node(dir.path(), "Node B", "UDE");
    let tree = create_tree(dir.path(), "crt", "SimpleCRT");
    attach_node(dir.path(), &tree, &a);
    attach_node(dir.path(), &tree, &b);
    let l1 = connect(dir.path(), &tree, &a, &b);

    let (json, code) = run_ltp(
        dir.path(),
        &[
            "path",
            "collapse",
            "--tree",
            &tree,
            "--from",
            &a,
            "--to",
            &b,
            "--label",
            "Single edge",
        ],
    );
    assert_eq!(code, 0, "single edge collapse failed: {:?}", json);

    let data = &json["data"];
    // No interior nodes (from/to are excluded)
    let interior: Vec<Value> = data["interior_nodes"].as_array().unwrap().clone();
    assert_eq!(interior.len(), 0);

    // But the link itself is interior
    let int_links: Vec<String> = data["interior_links"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(int_links.len(), 1);
    assert_eq!(int_links[0], l1);
}

// --- UAT 9.11: Collapse over path containing existing macro_edge → error ---
#[test]
fn uat_9_11_nested_macro_not_allowed() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, nodes, _links) = linear_chain_setup(dir.path());

    // First collapse: A → C (subset)
    let (_, code) = run_ltp(
        dir.path(),
        &[
            "path",
            "collapse",
            "--tree",
            &tree,
            "--from",
            &nodes[0],
            "--to",
            &nodes[2],
            "--label",
            "First collapse",
        ],
    );
    assert_eq!(code, 0);

    // Second collapse: A → E (encompasses the first)
    let (json, code) = run_ltp(
        dir.path(),
        &[
            "path",
            "collapse",
            "--tree",
            &tree,
            "--from",
            &nodes[0],
            "--to",
            &nodes[4],
            "--label",
            "Nested should fail",
        ],
    );
    assert_ne!(code, 0);
    assert!(!json["success"].as_bool().unwrap());
    assert_eq!(json["errors"][0]["code"], "NESTED_MACRO_NOT_ALLOWED");
}

// --- UAT 9.12: Explode verifies new edges have correct logic and status ---
#[test]
fn uat_9_12_explode_new_edges_properties() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, _nodes, links) = linear_chain_setup(dir.path());

    // Add assumption to link
    let asm = add_assumption(dir.path(), &tree, &links[0], "Test assumption");

    let (json, code) = run_ltp(
        dir.path(),
        &[
            "path",
            "explode",
            "--tree",
            &tree,
            "--link",
            &links[0],
            "--asm",
            &asm,
            "--label",
            "Exploded node",
        ],
    );
    assert_eq!(code, 0);

    let created_links: Vec<String> = json["data"]["created_links"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();

    // Inspect each new link via link inspect (positional LINK_ID + --tree)
    for link_id in &created_links {
        let (insp, ic) = run_ltp(dir.path(), &["link", "inspect", link_id, "--tree", &tree]);
        assert_eq!(ic, 0, "link inspect failed for {}: {:?}", link_id, insp);
        assert_eq!(insp["data"]["logic"], "sufficiency");
        assert_eq!(insp["data"]["status"], "active");
    }

    // Original link should be removed (inspect should fail)
    let (orig_insp, oc) = run_ltp(dir.path(), &["link", "inspect", &links[0], "--tree", &tree]);
    assert_ne!(oc, 0);
    assert_eq!(orig_insp["errors"][0]["code"], "LINK_NOT_FOUND");
}
