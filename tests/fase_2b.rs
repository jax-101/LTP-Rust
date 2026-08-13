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

fn setup_workspace(dir: &std::path::Path) {
    let (_, code) = run_ltp(dir, &["init", "--name", "F2bTest"]);
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

fn connect_nodes(dir: &std::path::Path, tree_id: &str, from: &str, to: &str) -> String {
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

/// UAT 2b.1: node rm elimina nodo del pool Y todos sus edges en todos los trees.
#[test]
fn uat_2b_1_node_rm_with_edges() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let node_a = add_node(dir, "Cause A", "ude");
    let node_b = add_node(dir, "Effect B", "ude");
    let node_c = add_node(dir, "Effect C", "rc");
    let tree_id = create_tree(dir, "crt", "TestTree");
    attach_node(dir, &tree_id, &node_a);
    attach_node(dir, &tree_id, &node_b);
    attach_node(dir, &tree_id, &node_c);
    connect_nodes(dir, &tree_id, &node_a, &node_b);
    connect_nodes(dir, &tree_id, &node_b, &node_c);

    let (json, code) = run_ltp(dir, &["node", "rm", &node_b]);

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["action"], "node_rm");
    assert_eq!(json["data"]["removed_nodes"][0], node_b.as_str());
    assert!(json["data"]["removed_edges_count"].as_u64().unwrap() >= 2);

    // Verify node is gone from pool
    let (json_list, _) = run_ltp(dir, &["node", "list"]);
    let nodes = json_list["data"]["nodes"].as_array().unwrap();
    assert!(!nodes.iter().any(|n| n["id"].as_str().unwrap() == node_b));

    // Verify edges referencing node_b are removed from tree
    let tree_file = dir.join("trees").join(format!("{}.json", tree_id));
    let tree_content: Value =
        serde_json::from_str(&std::fs::read_to_string(&tree_file).unwrap()).unwrap();
    let edges = tree_content["edges"].as_array().unwrap();
    for edge in edges {
        let from_arr = edge["from"].as_array().unwrap();
        assert!(!from_arr.iter().any(|f| f.as_str().unwrap() == node_b));
        assert_ne!(edge["to"].as_str().unwrap(), node_b.as_str());
    }

    // Verify node_b is removed from tree's nodes[]
    let tree_nodes = tree_content["nodes"].as_array().unwrap();
    assert!(!tree_nodes
        .iter()
        .any(|n| n["ref"].as_str().unwrap() == node_b));
}

/// UAT 2b.2: node rm de nodo inexistente retorna NODE_NOT_FOUND.
#[test]
fn uat_2b_2_node_rm_not_found() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let (json, code) = run_ltp(dir, &["node", "rm", "UDE-999"]);

    assert_eq!(code, 1);
    assert_eq!(json["success"], false);
    assert_eq!(json["errors"][0]["code"], "NODE_NOT_FOUND");
}

/// UAT 2b.3: node rm batch — elimina múltiples nodos con todos sus edges.
#[allow(clippy::too_many_lines)]
#[test]
fn uat_2b_3_node_rm_batch() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let node_a = add_node(dir, "Node A", "ude");
    let node_b = add_node(dir, "Node B", "rc");
    let node_c = add_node(dir, "Node C", "ude");
    let tree_id = create_tree(dir, "crt", "BatchTree");
    attach_node(dir, &tree_id, &node_a);
    attach_node(dir, &tree_id, &node_b);
    attach_node(dir, &tree_id, &node_c);
    connect_nodes(dir, &tree_id, &node_a, &node_c);
    connect_nodes(dir, &tree_id, &node_b, &node_c);

    // Remove A and B in batch
    let ids_arg = format!("{},{}", node_a, node_b);
    let (json, code) = run_ltp(dir, &["node", "rm", &ids_arg]);

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    let removed = json["data"]["removed_nodes"].as_array().unwrap();
    assert_eq!(removed.len(), 2);
    assert!(json["data"]["removed_edges_count"].as_u64().unwrap() >= 2);

    // Verify pool only has node_c
    let (json_list, _) = run_ltp(dir, &["node", "list"]);
    let nodes = json_list["data"]["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0]["id"].as_str().unwrap(), node_c);
}

/// UAT 2b.4: node inspect muestra trees, rol y conexiones.
#[test]
fn uat_2b_4_node_inspect() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let node_a = add_node(dir, "Cause A", "ude");
    let node_b = add_node(dir, "Effect B", "ude");
    let node_c = add_node(dir, "Effect C", "rc");
    let tree_id = create_tree(dir, "crt", "InspectTree");
    attach_node(dir, &tree_id, &node_a);
    attach_node(dir, &tree_id, &node_b);
    attach_node(dir, &tree_id, &node_c);

    // Attach node_a with a role in a second tree
    let tree_id_2 = create_tree(dir, "ec", "SecondTree");
    let (_, code) = run_ltp(
        dir,
        &[
            "tree",
            "attach",
            "--tree",
            &tree_id_2,
            "--node",
            &node_a,
            "--role",
            "objective",
        ],
    );
    assert_eq!(code, 0);

    // Connect A->B and B->C in first tree
    connect_nodes(dir, &tree_id, &node_a, &node_b);
    connect_nodes(dir, &tree_id, &node_b, &node_c);

    let (json, code) = run_ltp(dir, &["node", "inspect", &node_b]);

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["action"], "node_inspect");
    assert_eq!(json["data"]["id"], node_b.as_str());
    assert_eq!(json["data"]["label"], "Effect B");

    // Verify trees participation
    let trees = json["data"]["trees"].as_array().unwrap();
    assert_eq!(trees.len(), 1);
    assert_eq!(trees[0]["tree_id"].as_str().unwrap(), tree_id);

    // Verify connections (edges inbound and outbound)
    let connections = &trees[0]["connections"];
    let inbound = connections["inbound"].as_array().unwrap();
    let outbound = connections["outbound"].as_array().unwrap();
    assert_eq!(inbound.len(), 1);
    assert_eq!(outbound.len(), 1);
    assert!(inbound[0]["from"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f.as_str().unwrap() == node_a));
    assert_eq!(outbound[0]["to"].as_str().unwrap(), node_c.as_str());

    // Inspect node_a which is in 2 trees
    let (json_a, _) = run_ltp(dir, &["node", "inspect", &node_a]);
    let trees_a = json_a["data"]["trees"].as_array().unwrap();
    assert_eq!(trees_a.len(), 2);

    // Verify the second tree has role "objective"
    let second = trees_a
        .iter()
        .find(|t| t["tree_id"].as_str().unwrap() == tree_id_2)
        .unwrap();
    assert_eq!(second["role"].as_str().unwrap(), "objective");
}

/// UAT 2b.5: node split divide un nodo, hereda edges entrantes al primero y salientes al segundo.
#[test]
fn uat_2b_5_node_split() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let node_a = add_node(dir, "Root Cause", "rc");
    let node_int = add_node(dir, "Mixed idea A and B", "int");
    let node_c = add_node(dir, "Effect", "ude");
    let tree_id = create_tree(dir, "crt", "SplitTree");
    attach_node(dir, &tree_id, &node_a);
    attach_node(dir, &tree_id, &node_int);
    attach_node(dir, &tree_id, &node_c);
    connect_nodes(dir, &tree_id, &node_a, &node_int);
    connect_nodes(dir, &tree_id, &node_int, &node_c);

    let (json, code) = run_ltp(
        dir,
        &[
            "node", "split", &node_int, "--into", "Idea A", "Idea B", "--tree", &tree_id,
        ],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["action"], "node_split");

    let new_nodes = json["data"]["new_nodes"].as_array().unwrap();
    assert_eq!(new_nodes.len(), 2);
    let first_id = new_nodes[0]["id"].as_str().unwrap();
    let second_id = new_nodes[1]["id"].as_str().unwrap();

    // Verify original node removed from pool
    let (json_list, _) = run_ltp(dir, &["node", "list"]);
    let nodes = json_list["data"]["nodes"].as_array().unwrap();
    assert!(!nodes.iter().any(|n| n["id"].as_str().unwrap() == node_int));
    assert!(nodes.iter().any(|n| n["id"].as_str().unwrap() == first_id));
    assert!(nodes.iter().any(|n| n["id"].as_str().unwrap() == second_id));

    // Verify edges: A->first_id and second_id->C
    let tree_file = dir.join("trees").join(format!("{}.json", tree_id));
    let tree_content: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&tree_file).unwrap()).unwrap();
    let edges = tree_content["edges"].as_array().unwrap();

    let has_a_to_first = edges.iter().any(|e| {
        e["from"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f.as_str().unwrap() == node_a)
            && e["to"].as_str().unwrap() == first_id
    });
    let has_second_to_c = edges.iter().any(|e| {
        e["from"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f.as_str().unwrap() == second_id)
            && e["to"].as_str().unwrap() == node_c
    });

    assert!(has_a_to_first, "Expected edge from A to first new node");
    assert!(has_second_to_c, "Expected edge from second new node to C");

    // Verify no edges reference original node
    for edge in edges {
        let from_arr = edge["from"].as_array().unwrap();
        assert!(!from_arr.iter().any(|f| f.as_str().unwrap() == node_int));
        assert_ne!(edge["to"].as_str().unwrap(), node_int.as_str());
    }
}

/// UAT 2b.6: node list --tree --type filtra por tree y tipo.
#[test]
fn uat_2b_6_node_list_tree_filter() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let node_a = add_node(dir, "UDE in tree", "ude");
    let node_b = add_node(dir, "RC in tree", "rc");
    let _node_c = add_node(dir, "UDE not in tree", "ude");
    let tree_id = create_tree(dir, "crt", "FilterTree");
    attach_node(dir, &tree_id, &node_a);
    attach_node(dir, &tree_id, &node_b);

    // List with --tree filter
    let (json, code) = run_ltp(dir, &["node", "list", "--tree", &tree_id]);
    assert_eq!(code, 0);
    let nodes = json["data"]["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 2);
    assert!(nodes.iter().any(|n| n["id"].as_str().unwrap() == node_a));
    assert!(nodes.iter().any(|n| n["id"].as_str().unwrap() == node_b));

    // List with --tree AND --type UDE
    let (json2, code2) = run_ltp(dir, &["node", "list", "--tree", &tree_id, "--type", "UDE"]);
    assert_eq!(code2, 0);
    let nodes2 = json2["data"]["nodes"].as_array().unwrap();
    assert_eq!(nodes2.len(), 1);
    assert_eq!(nodes2[0]["id"].as_str().unwrap(), node_a);
}

/// UAT 2b.7: node list --tree --status filtra por status.
#[test]
fn uat_2b_7_node_list_status_filter() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let node_a = add_node(dir, "Active node", "ude");
    let node_b = add_node(dir, "Another node", "ude");
    let tree_id = create_tree(dir, "crt", "StatusTree");
    attach_node(dir, &tree_id, &node_a);
    attach_node(dir, &tree_id, &node_b);

    // Both are active by default
    let (json, code) = run_ltp(
        dir,
        &["node", "list", "--tree", &tree_id, "--status", "active"],
    );
    assert_eq!(code, 0);
    let nodes = json["data"]["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 2);

    // Filter by draft (should be empty)
    let (json2, code2) = run_ltp(
        dir,
        &["node", "list", "--tree", &tree_id, "--status", "draft"],
    );
    assert_eq!(code2, 0);
    let nodes2 = json2["data"]["nodes"].as_array().unwrap();
    assert_eq!(nodes2.len(), 0);
}
