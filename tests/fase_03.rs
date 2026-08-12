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
            "Failed to parse JSON output.\nstdout: {}\nstderr: {}",
            stdout,
            String::from_utf8_lossy(&output.stderr)
        )
    });

    (json, code)
}

fn setup_workspace(dir: &std::path::Path) {
    let (_, code) = run_ltp(dir, &["init", "--name", "TreeTest"]);
    assert_eq!(code, 0);
}

fn add_node(dir: &std::path::Path, label: &str, node_type: &str) -> String {
    let (json, _) = run_ltp(dir, &["node", "add", label, "--type", node_type]);
    json["data"]["id"].as_str().unwrap().to_string()
}

/// UAT 3.1: tree new crt creates tree with logic sufficiency.
#[test]
fn uat_3_1_tree_new_crt() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let (json, code) = run_ltp(dir, &["tree", "new", "crt", "CRT Logística"]);

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["action"], "tree_new");
    assert_eq!(json["data"]["id"], "tree-crt-crt-log-stica");
    assert_eq!(json["data"]["tree_type"], "crt");
    assert_eq!(json["data"]["logic"], "sufficiency");

    // Verify file exists
    assert!(dir.join("trees/tree-crt-crt-log-stica.json").is_file());
}

/// UAT 3.2: tree new ec creates tree with logic necessity.
#[test]
fn uat_3_2_tree_new_ec() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let (json, code) = run_ltp(dir, &["tree", "new", "ec", "Evaporating Cloud"]);

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["tree_type"], "ec");
    assert_eq!(json["data"]["logic"], "necessity");
}

/// UAT 3.3: tree list shows trees with summary info.
#[test]
fn uat_3_3_tree_list() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    run_ltp(dir, &["tree", "new", "crt", "First CRT"]);
    run_ltp(dir, &["tree", "new", "ec", "My EC"]);

    let (json, code) = run_ltp(dir, &["tree", "list"]);

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["count"], 2);

    let trees = json["data"]["trees"].as_array().unwrap();
    assert_eq!(trees.len(), 2);
}

/// UAT 3.4: tree attach adds node to tree's nodes[].
#[test]
fn uat_3_4_tree_attach() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let node_id = add_node(dir, "Test UDE", "UDE");
    run_ltp(dir, &["tree", "new", "crt", "Test CRT"]);

    let tree_id = "tree-crt-test-crt";
    let (json, code) = run_ltp(
        dir,
        &["tree", "attach", "--tree", tree_id, "--node", &node_id],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["tree_id"], tree_id);
    assert_eq!(json["data"]["node_id"], node_id);

    // Verify in tree file
    let tree_content: Value = serde_json::from_str(
        &std::fs::read_to_string(dir.join(format!("trees/{}.json", tree_id))).unwrap(),
    )
    .unwrap();
    let nodes = tree_content["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0]["ref"], node_id);
}

/// UAT 3.5: tree attach with role assigns role.
#[test]
fn uat_3_5_tree_attach_with_role() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let node_id = add_node(dir, "Problem node", "UDE");
    run_ltp(dir, &["tree", "new", "crt", "Role CRT"]);

    let tree_id = "tree-crt-role-crt";
    let (json, code) = run_ltp(
        dir,
        &[
            "tree",
            "attach",
            "--tree",
            tree_id,
            "--node",
            &node_id,
            "--role",
            "core_problem",
        ],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["role"], "core_problem");

    // Verify role in tree file
    let tree_content: Value = serde_json::from_str(
        &std::fs::read_to_string(dir.join(format!("trees/{}.json", tree_id))).unwrap(),
    )
    .unwrap();
    assert_eq!(tree_content["nodes"][0]["role"], "core_problem");
}

/// UAT 3.6: tree detach removes node but keeps it in pool.
#[test]
fn uat_3_6_tree_detach() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let node_id = add_node(dir, "Detach me", "UDE");
    run_ltp(dir, &["tree", "new", "crt", "Detach CRT"]);
    let tree_id = "tree-crt-detach-crt";
    run_ltp(
        dir,
        &["tree", "attach", "--tree", tree_id, "--node", &node_id],
    );

    let (json, code) = run_ltp(
        dir,
        &["tree", "detach", "--tree", tree_id, "--node", &node_id],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);

    // Node gone from tree
    let tree_content: Value = serde_json::from_str(
        &std::fs::read_to_string(dir.join(format!("trees/{}.json", tree_id))).unwrap(),
    )
    .unwrap();
    assert_eq!(tree_content["nodes"].as_array().unwrap().len(), 0);

    // Node still in pool
    assert!(dir.join(format!("nodes/{}.json", node_id)).is_file());
}

/// UAT 3.7: tree clone creates new tree with shared node refs.
#[test]
fn uat_3_7_tree_clone() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let node_id = add_node(dir, "Shared node", "UDE");
    run_ltp(dir, &["tree", "new", "crt", "Original"]);
    let tree_id = "tree-crt-original";
    run_ltp(
        dir,
        &["tree", "attach", "--tree", tree_id, "--node", &node_id],
    );

    let (json, code) = run_ltp(dir, &["tree", "clone", tree_id, "--name", "CRT What-If"]);

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["original_id"], tree_id);

    let new_id = json["data"]["new_id"].as_str().unwrap();
    assert!(dir.join(format!("trees/{}.json", new_id)).is_file());

    // Shared node ref
    let clone_content: Value = serde_json::from_str(
        &std::fs::read_to_string(dir.join(format!("trees/{}.json", new_id))).unwrap(),
    )
    .unwrap();
    assert_eq!(clone_content["nodes"][0]["ref"], node_id);
}

/// UAT 3.8: tree diff reports differences.
#[test]
fn uat_3_8_tree_diff() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let node_a = add_node(dir, "Node A", "UDE");
    let node_b = add_node(dir, "Node B", "RC");

    run_ltp(dir, &["tree", "new", "crt", "Diff A"]);
    let tree_a = "tree-crt-diff-a";
    run_ltp(
        dir,
        &["tree", "attach", "--tree", tree_a, "--node", &node_a],
    );
    run_ltp(
        dir,
        &["tree", "attach", "--tree", tree_a, "--node", &node_b],
    );

    // Clone then detach one node from clone
    run_ltp(dir, &["tree", "clone", tree_a, "--name", "Diff B"]);
    let tree_b = "tree-crt-diff-b";

    // Add a new node to B only
    let node_c = add_node(dir, "Node C", "INJ");
    run_ltp(
        dir,
        &["tree", "attach", "--tree", tree_b, "--node", &node_c],
    );

    // Detach node_b from B
    run_ltp(
        dir,
        &["tree", "detach", "--tree", tree_b, "--node", &node_b],
    );

    let (json, code) = run_ltp(dir, &["tree", "diff", tree_a, tree_b]);

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);

    let nodes_added = json["data"]["nodes_added"].as_array().unwrap();
    let nodes_removed = json["data"]["nodes_removed"].as_array().unwrap();

    assert!(nodes_added.iter().any(|n| n.as_str() == Some(&node_c)));
    assert!(nodes_removed.iter().any(|n| n.as_str() == Some(&node_b)));
}

/// UAT 3.9: tree walk topological order.
#[test]
fn uat_3_9_tree_walk_topological() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let node_a = add_node(dir, "Root cause", "RC");
    let node_b = add_node(dir, "Effect", "UDE");

    run_ltp(dir, &["tree", "new", "crt", "Walk CRT"]);
    let tree_id = "tree-crt-walk-crt";
    run_ltp(
        dir,
        &["tree", "attach", "--tree", tree_id, "--node", &node_a],
    );
    run_ltp(
        dir,
        &["tree", "attach", "--tree", tree_id, "--node", &node_b],
    );

    let (json, code) = run_ltp(dir, &["tree", "walk", tree_id, "--order", "topological"]);

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["order"], "topological");

    let nodes = json["data"]["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 2);
}

/// UAT 3.10: tree walk reverse order.
#[test]
fn uat_3_10_tree_walk_reverse() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let node_a = add_node(dir, "Cause", "RC");
    let node_b = add_node(dir, "Effect", "UDE");

    run_ltp(dir, &["tree", "new", "crt", "Reverse CRT"]);
    let tree_id = "tree-crt-reverse-crt";
    run_ltp(
        dir,
        &["tree", "attach", "--tree", tree_id, "--node", &node_a],
    );
    run_ltp(
        dir,
        &["tree", "attach", "--tree", tree_id, "--node", &node_b],
    );

    let (json, code) = run_ltp(dir, &["tree", "walk", tree_id, "--order", "reverse"]);

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["order"], "reverse");

    let nodes = json["data"]["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 2);
}

/// UAT 3.11: tree rm deletes tree, nodes stay in pool.
#[test]
fn uat_3_11_tree_rm() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let node_id = add_node(dir, "Survivor", "UDE");
    run_ltp(dir, &["tree", "new", "crt", "Doomed CRT"]);
    let tree_id = "tree-crt-doomed-crt";
    run_ltp(
        dir,
        &["tree", "attach", "--tree", tree_id, "--node", &node_id],
    );

    let (json, code) = run_ltp(dir, &["tree", "rm", tree_id]);

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);

    // Tree file gone
    assert!(!dir.join(format!("trees/{}.json", tree_id)).is_file());

    // Node still in pool
    assert!(dir.join(format!("nodes/{}.json", node_id)).is_file());
}
