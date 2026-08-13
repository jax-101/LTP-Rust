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
    let (_, code) = run_ltp(dir, &["init", "--name", "LinkTest"]);
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

/// UAT 4.1: link connect --tree T --from A --to B creates edge SINGLE with ID LINK-001.
#[test]
fn uat_4_1_link_connect_single() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let node_a = add_node(dir, "Cause A", "ude");
    let node_b = add_node(dir, "Effect B", "ude");
    let tree_id = create_tree(dir, "crt", "TestTree");
    attach_node(dir, &tree_id, &node_a);
    attach_node(dir, &tree_id, &node_b);

    let (json, code) = run_ltp(
        dir,
        &[
            "link", "connect", "--tree", &tree_id, "--from", &node_a, "--to", &node_b,
        ],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["action"], "link_connect");
    assert_eq!(json["data"]["created_links"][0], "LINK-001");
    assert_eq!(json["data"]["tree_id"], tree_id);
}

/// UAT 4.2: link connect --from A,B --to C --operator AND creates edge AND with from: ["A","B"].
#[test]
fn uat_4_2_link_connect_and_operator() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let node_a = add_node(dir, "Cause A", "ude");
    let node_b = add_node(dir, "Cause B", "ude");
    let node_c = add_node(dir, "Effect C", "ude");
    let tree_id = create_tree(dir, "crt", "TestTree");
    attach_node(dir, &tree_id, &node_a);
    attach_node(dir, &tree_id, &node_b);
    attach_node(dir, &tree_id, &node_c);

    let (json, code) = run_ltp(
        dir,
        &[
            "link",
            "connect",
            "--tree",
            &tree_id,
            "--from",
            &format!("{},{}", node_a, node_b),
            "--to",
            &node_c,
            "--operator",
            "AND",
        ],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["created_links"][0], "LINK-001");

    // Verify stored edge has AND operator and from: [A, B]
    let tree_file = dir.join("trees").join(format!("{}.json", tree_id));
    let tree_content: Value =
        serde_json::from_str(&std::fs::read_to_string(&tree_file).unwrap()).unwrap();
    let edge = &tree_content["edges"][0];
    assert_eq!(edge["operator"], "AND");
    assert_eq!(edge["from"][0].as_str().unwrap(), node_a);
    assert_eq!(edge["from"][1].as_str().unwrap(), node_b);
}

/// UAT 4.3: link connect --from A --to B,C creates 2 SINGLE edges (A→B, A→C).
#[test]
fn uat_4_3_link_connect_multiple_to() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let node_a = add_node(dir, "Cause A", "ude");
    let node_b = add_node(dir, "Effect B", "ude");
    let node_c = add_node(dir, "Effect C", "ude");
    let tree_id = create_tree(dir, "crt", "TestTree");
    attach_node(dir, &tree_id, &node_a);
    attach_node(dir, &tree_id, &node_b);
    attach_node(dir, &tree_id, &node_c);

    let (json, code) = run_ltp(
        dir,
        &[
            "link",
            "connect",
            "--tree",
            &tree_id,
            "--from",
            &node_a,
            "--to",
            &format!("{},{}", node_b, node_c),
        ],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    let links = json["data"]["created_links"].as_array().unwrap();
    assert_eq!(links.len(), 2);
    assert_eq!(links[0], "LINK-001");
    assert_eq!(links[1], "LINK-002");
}

/// UAT 4.4: link connect --operator MAG without --weight emits warning.
#[test]
fn uat_4_4_link_connect_mag_no_weight() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let node_a = add_node(dir, "Cause A", "ude");
    let node_b = add_node(dir, "Effect B", "ude");
    let tree_id = create_tree(dir, "crt", "TestTree");
    attach_node(dir, &tree_id, &node_a);
    attach_node(dir, &tree_id, &node_b);

    let (json, code) = run_ltp(
        dir,
        &[
            "link",
            "connect",
            "--tree",
            &tree_id,
            "--from",
            &node_a,
            "--to",
            &node_b,
            "--operator",
            "MAG",
        ],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    let warnings = json["warnings"].as_array().unwrap();
    assert!(warnings.iter().any(|w| w["code"] == "MAG_WEIGHT_MISSING"));
}

/// UAT 4.5: link connect --operator MAG --weight 0.6 creates edge with weight.
#[test]
fn uat_4_5_link_connect_mag_with_weight() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let node_a = add_node(dir, "Cause A", "ude");
    let node_b = add_node(dir, "Effect B", "ude");
    let tree_id = create_tree(dir, "crt", "TestTree");
    attach_node(dir, &tree_id, &node_a);
    attach_node(dir, &tree_id, &node_b);

    let (json, code) = run_ltp(
        dir,
        &[
            "link",
            "connect",
            "--tree",
            &tree_id,
            "--from",
            &node_a,
            "--to",
            &node_b,
            "--operator",
            "MAG",
            "--weight",
            "0.6",
        ],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert!(json["warnings"].as_array().unwrap().is_empty());

    // Verify stored edge has weight
    let tree_file = dir.join("trees").join(format!("{}.json", tree_id));
    let tree_content: Value =
        serde_json::from_str(&std::fs::read_to_string(&tree_file).unwrap()).unwrap();
    let edge = &tree_content["edges"][0];
    assert_eq!(edge["operator"], "MAG");
    assert!((edge["weight"].as_f64().unwrap() - 0.6).abs() < f64::EPSILON);
}

/// UAT 4.6: link connect with non-existent node returns REFERENTIAL_INTEGRITY_VIOLATION.
#[test]
fn uat_4_6_link_connect_node_not_in_pool() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let node_a = add_node(dir, "Cause A", "ude");
    let tree_id = create_tree(dir, "crt", "TestTree");
    attach_node(dir, &tree_id, &node_a);

    let (json, code) = run_ltp(
        dir,
        &[
            "link", "connect", "--tree", &tree_id, "--from", &node_a, "--to", "UDE-999",
        ],
    );

    assert_ne!(code, 0);
    assert_eq!(json["success"], false);
    assert!(json["errors"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["code"] == "REFERENTIAL_INTEGRITY_VIOLATION"));
}

/// UAT 4.7: link connect with node not attached to tree returns NODE_NOT_IN_TREE.
#[test]
fn uat_4_7_link_connect_node_not_in_tree() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let node_a = add_node(dir, "Cause A", "ude");
    let node_b = add_node(dir, "Effect B", "ude");
    let tree_id = create_tree(dir, "crt", "TestTree");
    attach_node(dir, &tree_id, &node_a);
    // node_b is NOT attached

    let (json, code) = run_ltp(
        dir,
        &[
            "link", "connect", "--tree", &tree_id, "--from", &node_a, "--to", &node_b,
        ],
    );

    assert_ne!(code, 0);
    assert_eq!(json["success"], false);
    assert!(json["errors"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["code"] == "NODE_NOT_IN_TREE"));
}

/// UAT 4.8: Creating a cycle A→B→C→A returns CIRCULAR_DEPENDENCY_DETECTED.
#[test]
fn uat_4_8_link_connect_cycle_detection() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let node_a = add_node(dir, "Node A", "ude");
    let node_b = add_node(dir, "Node B", "ude");
    let node_c = add_node(dir, "Node C", "ude");
    let tree_id = create_tree(dir, "crt", "TestTree");
    attach_node(dir, &tree_id, &node_a);
    attach_node(dir, &tree_id, &node_b);
    attach_node(dir, &tree_id, &node_c);

    // A → B
    let (_, code) = run_ltp(
        dir,
        &[
            "link", "connect", "--tree", &tree_id, "--from", &node_a, "--to", &node_b,
        ],
    );
    assert_eq!(code, 0);

    // B → C
    let (_, code) = run_ltp(
        dir,
        &[
            "link", "connect", "--tree", &tree_id, "--from", &node_b, "--to", &node_c,
        ],
    );
    assert_eq!(code, 0);

    // C → A (cycle!)
    let (json, code) = run_ltp(
        dir,
        &[
            "link", "connect", "--tree", &tree_id, "--from", &node_c, "--to", &node_a,
        ],
    );

    assert_ne!(code, 0);
    assert_eq!(json["success"], false);
    assert_eq!(json["graph_health"]["valid_dag"], false);
    assert!(json["errors"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["code"] == "CIRCULAR_DEPENDENCY_DETECTED"));
}

/// UAT 4.9: link disconnect --tree T --links L1,L2 removes both edges.
#[test]
fn uat_4_9_link_disconnect() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let node_a = add_node(dir, "Cause A", "ude");
    let node_b = add_node(dir, "Effect B", "ude");
    let node_c = add_node(dir, "Effect C", "ude");
    let tree_id = create_tree(dir, "crt", "TestTree");
    attach_node(dir, &tree_id, &node_a);
    attach_node(dir, &tree_id, &node_b);
    attach_node(dir, &tree_id, &node_c);

    // Create two links
    let (json1, _) = run_ltp(
        dir,
        &[
            "link", "connect", "--tree", &tree_id, "--from", &node_a, "--to", &node_b,
        ],
    );
    let link1 = json1["data"]["created_links"][0]
        .as_str()
        .unwrap()
        .to_string();

    let (json2, _) = run_ltp(
        dir,
        &[
            "link", "connect", "--tree", &tree_id, "--from", &node_a, "--to", &node_c,
        ],
    );
    let link2 = json2["data"]["created_links"][0]
        .as_str()
        .unwrap()
        .to_string();

    // Disconnect both
    let (json, code) = run_ltp(
        dir,
        &[
            "link",
            "disconnect",
            "--tree",
            &tree_id,
            "--links",
            &format!("{},{}", link1, link2),
        ],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["action"], "link_disconnect");
    let removed = json["data"]["removed_links"].as_array().unwrap();
    assert_eq!(removed.len(), 2);

    // Verify tree has no edges
    let tree_file = dir.join("trees").join(format!("{}.json", tree_id));
    let tree_content: Value =
        serde_json::from_str(&std::fs::read_to_string(&tree_file).unwrap()).unwrap();
    assert_eq!(tree_content["edges"].as_array().unwrap().len(), 0);
}

/// UAT 4.10: link feedback creates entry in feedback_edges[] without triggering cycle error.
#[test]
fn uat_4_10_link_feedback() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let node_a = add_node(dir, "Effect A", "ude");
    let node_b = add_node(dir, "Root Cause B", "rc");
    let tree_id = create_tree(dir, "crt", "TestTree");
    attach_node(dir, &tree_id, &node_a);
    attach_node(dir, &tree_id, &node_b);

    // Create causal link B → A
    let (_, code) = run_ltp(
        dir,
        &[
            "link", "connect", "--tree", &tree_id, "--from", &node_b, "--to", &node_a,
        ],
    );
    assert_eq!(code, 0);

    // Create feedback loop A → B (positive) - this would be a cycle in edges, but feedback is separate
    let (json, code) = run_ltp(
        dir,
        &[
            "link",
            "feedback",
            "--tree",
            &tree_id,
            "--from",
            &node_a,
            "--to",
            &node_b,
            "--type",
            "positive",
            "--label",
            "Reinforcing loop",
        ],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["action"], "link_feedback");
    assert_eq!(json["data"]["id"], "FB-001");
    assert_eq!(json["data"]["loop_type"], "positive");

    // Verify stored in feedback_edges
    let tree_file = dir.join("trees").join(format!("{}.json", tree_id));
    let tree_content: Value =
        serde_json::from_str(&std::fs::read_to_string(&tree_file).unwrap()).unwrap();
    let fb = &tree_content["feedback_edges"][0];
    assert_eq!(fb["id"], "FB-001");
    assert_eq!(fb["loop_type"], "positive");
    assert_eq!(fb["label"], "Reinforcing loop");
}

/// UAT 4.11: ltp status after feedback reports feedback loop count.
#[test]
fn uat_4_11_status_reports_feedback() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let node_a = add_node(dir, "Effect A", "ude");
    let node_b = add_node(dir, "Root Cause B", "rc");
    let tree_id = create_tree(dir, "crt", "TestTree");
    attach_node(dir, &tree_id, &node_a);
    attach_node(dir, &tree_id, &node_b);

    // Create causal link B → A
    let (_, code) = run_ltp(
        dir,
        &[
            "link", "connect", "--tree", &tree_id, "--from", &node_b, "--to", &node_a,
        ],
    );
    assert_eq!(code, 0);

    // Create feedback loop
    let (_, code) = run_ltp(
        dir,
        &[
            "link", "feedback", "--tree", &tree_id, "--from", &node_a, "--to", &node_b, "--type",
            "positive",
        ],
    );
    assert_eq!(code, 0);

    // Check status
    let (json, code) = run_ltp(dir, &["status"]);

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    let trees = json["data"]["trees"].as_array().unwrap();
    assert_eq!(trees.len(), 1);
    assert_eq!(trees[0]["feedback_edge_count"], 1);
}
