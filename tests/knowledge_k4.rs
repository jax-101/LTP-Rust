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
    let (json, code) = run_ltp(dir, &["init", "--name", "TestK4"]);
    assert_eq!(code, 0, "init failed: {:?}", json);
}

// === K4.1: node add --epistemic fact ===
#[test]
fn k4_1_node_add_epistemic_fact() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let (json, code) = run_ltp(
        dir,
        &[
            "node",
            "add",
            "Test",
            "--type",
            "UDE",
            "--epistemic",
            "fact",
        ],
    );
    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["epistemic"], "fact");

    // Verify on disk
    let node_file = dir.join("nodes").join("UDE-001.json");
    let content: Value =
        serde_json::from_str(&std::fs::read_to_string(node_file).unwrap()).unwrap();
    assert_eq!(content["epistemic"], "fact");
}

// === K4.2: node add without --epistemic (default hypothesis, omitted from JSON) ===
#[test]
fn k4_2_node_add_default_hypothesis_omitted() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let (json, code) = run_ltp(dir, &["node", "add", "Test", "--type", "UDE"]);
    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["epistemic"], "hypothesis");

    // On disk: field OMITTED (skip_serializing_if = is_hypothesis)
    let node_file = dir.join("nodes").join("UDE-001.json");
    let content: Value =
        serde_json::from_str(&std::fs::read_to_string(node_file).unwrap()).unwrap();
    assert!(
        content.get("epistemic").is_none(),
        "hypothesis should be omitted from JSON on disk"
    );
}

// === K4.3: node add --epistemic invalid_value ===
#[test]
fn k4_3_node_add_invalid_epistemic() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let (json, code) = run_ltp(
        dir,
        &[
            "node",
            "add",
            "Test",
            "--type",
            "UDE",
            "--epistemic",
            "invalid_value",
        ],
    );
    assert_ne!(code, 0);
    assert_eq!(json["success"], false);
    assert_eq!(json["errors"][0]["code"], "INVALID_EPISTEMIC");
}

// === K4.4: node edit --epistemic fact (adds field) ===
#[test]
fn k4_4_node_edit_add_epistemic() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    run_ltp(dir, &["node", "add", "Test", "--type", "UDE"]);
    let (json, code) = run_ltp(dir, &["node", "edit", "UDE-001", "--epistemic", "fact"]);
    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["epistemic"], "fact");

    // Verify on disk
    let node_file = dir.join("nodes").join("UDE-001.json");
    let content: Value =
        serde_json::from_str(&std::fs::read_to_string(node_file).unwrap()).unwrap();
    assert_eq!(content["epistemic"], "fact");
}

// === K4.5: node edit --epistemic hypothesis (back to default → omitted) ===
#[test]
fn k4_5_node_edit_back_to_hypothesis_omitted() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    run_ltp(
        dir,
        &[
            "node",
            "add",
            "Test",
            "--type",
            "UDE",
            "--epistemic",
            "fact",
        ],
    );
    let (json, code) = run_ltp(
        dir,
        &["node", "edit", "UDE-001", "--epistemic", "hypothesis"],
    );
    assert_eq!(code, 0);
    assert_eq!(json["data"]["epistemic"], "hypothesis");

    // On disk: field OMITTED
    let node_file = dir.join("nodes").join("UDE-001.json");
    let content: Value =
        serde_json::from_str(&std::fs::read_to_string(node_file).unwrap()).unwrap();
    assert!(
        content.get("epistemic").is_none(),
        "hypothesis should be omitted from JSON on disk"
    );
}

// === K4.6: node edit --epistemic fact when already fact (idempotent) ===
#[test]
fn k4_6_node_edit_epistemic_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    run_ltp(
        dir,
        &[
            "node",
            "add",
            "Test",
            "--type",
            "UDE",
            "--epistemic",
            "fact",
        ],
    );
    let (json, code) = run_ltp(dir, &["node", "edit", "UDE-001", "--epistemic", "fact"]);
    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["epistemic"], "fact");
}

// === K4.7: node edit --epistemic fact → undo ===
#[test]
fn k4_7_node_edit_epistemic_undo() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    run_ltp(dir, &["node", "add", "Test", "--type", "UDE"]);
    run_ltp(dir, &["node", "edit", "UDE-001", "--epistemic", "fact"]);
    run_ltp(dir, &["undo"]);

    // After undo, should be back to default (no epistemic field on disk)
    let node_file = dir.join("nodes").join("UDE-001.json");
    let content: Value =
        serde_json::from_str(&std::fs::read_to_string(node_file).unwrap()).unwrap();
    assert!(
        content.get("epistemic").is_none(),
        "undo should restore original state without epistemic"
    );
}

// === K4.8: node list --epistemic hypothesis (includes nodes without explicit field) ===
#[test]
fn k4_8_node_list_epistemic_hypothesis_includes_default() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    // Create tree and nodes
    run_ltp(dir, &["tree", "new", "CRT", "TestTree"]);
    run_ltp(dir, &["node", "add", "Implicit hypo", "--type", "UDE"]);
    run_ltp(
        dir,
        &[
            "node",
            "add",
            "Explicit hypo",
            "--type",
            "UDE",
            "--epistemic",
            "hypothesis",
        ],
    );
    run_ltp(
        dir,
        &[
            "node",
            "add",
            "A fact",
            "--type",
            "UDE",
            "--epistemic",
            "fact",
        ],
    );

    // Attach to tree
    run_ltp(
        dir,
        &[
            "tree",
            "attach",
            "--tree",
            "tree-crt-testtree",
            "--node",
            "UDE-001",
        ],
    );
    run_ltp(
        dir,
        &[
            "tree",
            "attach",
            "--tree",
            "tree-crt-testtree",
            "--node",
            "UDE-002",
        ],
    );
    run_ltp(
        dir,
        &[
            "tree",
            "attach",
            "--tree",
            "tree-crt-testtree",
            "--node",
            "UDE-003",
        ],
    );

    let (json, code) = run_ltp(
        dir,
        &[
            "node",
            "list",
            "--tree",
            "tree-crt-testtree",
            "--epistemic",
            "hypothesis",
        ],
    );
    assert_eq!(code, 0);
    assert_eq!(json["data"]["count"], 2); // implicit + explicit hypothesis
}

// === K4.9: node list --epistemic fact ===
#[test]
fn k4_9_node_list_epistemic_fact() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    run_ltp(
        dir,
        &[
            "node",
            "add",
            "A fact",
            "--type",
            "UDE",
            "--epistemic",
            "fact",
        ],
    );
    run_ltp(dir, &["node", "add", "Hypothesis", "--type", "UDE"]);
    run_ltp(
        dir,
        &[
            "node",
            "add",
            "Another fact",
            "--type",
            "RC",
            "--epistemic",
            "fact",
        ],
    );

    let (json, code) = run_ltp(dir, &["node", "list", "--epistemic", "fact"]);
    assert_eq!(code, 0);
    assert_eq!(json["data"]["count"], 2);
}

// === K4.10: node list --epistemic fact with 0 matches ===
#[test]
fn k4_10_node_list_epistemic_no_matches() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    run_ltp(dir, &["node", "add", "Hypo1", "--type", "UDE"]);
    run_ltp(dir, &["node", "add", "Hypo2", "--type", "RC"]);

    let (json, code) = run_ltp(dir, &["node", "list", "--epistemic", "fact"]);
    assert_eq!(code, 0);
    assert_eq!(json["data"]["count"], 0);
    assert_eq!(json["data"]["nodes"].as_array().unwrap().len(), 0);
}

// === K4.11: node list --epistemic hypothesis --type UDE (combined filters) ===
#[test]
fn k4_11_node_list_epistemic_combined_type() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    run_ltp(dir, &["node", "add", "UDE hypo", "--type", "UDE"]);
    run_ltp(dir, &["node", "add", "RC hypo", "--type", "RC"]);
    run_ltp(
        dir,
        &[
            "node",
            "add",
            "UDE fact",
            "--type",
            "UDE",
            "--epistemic",
            "fact",
        ],
    );

    let (json, code) = run_ltp(
        dir,
        &["node", "list", "--epistemic", "hypothesis", "--type", "UDE"],
    );
    assert_eq!(code, 0);
    assert_eq!(json["data"]["count"], 1);
    assert_eq!(json["data"]["nodes"][0]["id"], "UDE-001");
}

// === K4.12: Load existing node without epistemic field (backwards-compatible) ===
#[test]
fn k4_12_backwards_compatible_no_field() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    // Manually write a node without epistemic field
    let node_json = r#"{
  "id": "UDE-001",
  "type": "UDE",
  "label": "Legacy node",
  "tags": [],
  "observable": true,
  "metadata": {
    "status": "active"
  }
}"#;
    std::fs::write(dir.join("nodes").join("UDE-001.json"), node_json).unwrap();

    // Should load fine with default hypothesis
    let (json, code) = run_ltp(dir, &["node", "inspect", "UDE-001"]);
    assert_eq!(code, 0);
    assert_eq!(json["data"]["epistemic"], "hypothesis");
}

// === K4.13: Node with unknown epistemic value in JSON ===
#[test]
fn k4_13_unknown_epistemic_value_error() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    // Write a node with unknown epistemic value
    let node_json = r#"{
  "id": "UDE-001",
  "type": "UDE",
  "label": "Bad node",
  "tags": [],
  "observable": true,
  "epistemic": "unknown_value",
  "metadata": {
    "status": "active"
  }
}"#;
    std::fs::write(dir.join("nodes").join("UDE-001.json"), node_json).unwrap();

    // Should fail to load
    let (json, code) = run_ltp(dir, &["node", "inspect", "UDE-001"]);
    assert_ne!(code, 0);
    assert_eq!(json["success"], false);
}

// === K4.14: node inspect shows epistemic fact ===
#[test]
fn k4_14_node_inspect_shows_epistemic() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    run_ltp(
        dir,
        &[
            "node",
            "add",
            "Grounded fact",
            "--type",
            "UDE",
            "--epistemic",
            "fact",
        ],
    );
    let (json, code) = run_ltp(dir, &["node", "inspect", "UDE-001"]);
    assert_eq!(code, 0);
    assert_eq!(json["data"]["epistemic"], "fact");
}

// === K4.15: node inspect shows epistemic hypothesis for default ===
#[test]
fn k4_15_node_inspect_default_shows_hypothesis() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    run_ltp(dir, &["node", "add", "Default node", "--type", "UDE"]);
    let (json, code) = run_ltp(dir, &["node", "inspect", "UDE-001"]);
    assert_eq!(code, 0);
    assert_eq!(json["data"]["epistemic"], "hypothesis");
}

// === K4.16: tree clone — epistemic is a node property, shared via pool ===
#[test]
fn k4_16_tree_clone_epistemic_shared() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    run_ltp(dir, &["tree", "new", "CRT", "Source"]);
    run_ltp(
        dir,
        &[
            "node",
            "add",
            "Fact node",
            "--type",
            "UDE",
            "--epistemic",
            "fact",
        ],
    );
    run_ltp(
        dir,
        &[
            "tree",
            "attach",
            "--tree",
            "tree-crt-source",
            "--node",
            "UDE-001",
        ],
    );
    run_ltp(
        dir,
        &["tree", "clone", "tree-crt-source", "--name", "Cloned"],
    );

    // Node in cloned tree still has epistemic=fact (same node pool)
    let (json, code) = run_ltp(dir, &["node", "inspect", "UDE-001"]);
    assert_eq!(code, 0);
    assert_eq!(json["data"]["epistemic"], "fact");
}

// === K4.17: node edit non-existent node ===
#[test]
fn k4_17_node_edit_not_found() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let (json, code) = run_ltp(dir, &["node", "edit", "UDE-999", "--epistemic", "fact"]);
    assert_ne!(code, 0);
    assert_eq!(json["success"], false);
    assert_eq!(json["errors"][0]["code"], "NODE_NOT_FOUND");
}

// === K4.18: Node with "epistemic": null in JSON ===
#[test]
fn k4_18_epistemic_null_deserializes_default() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node_json = r#"{
  "id": "UDE-001",
  "type": "UDE",
  "label": "Null epistemic",
  "tags": [],
  "observable": true,
  "epistemic": null,
  "metadata": {
    "status": "active"
  }
}"#;
    std::fs::write(dir.join("nodes").join("UDE-001.json"), node_json).unwrap();

    // serde with default should handle null gracefully
    let (json, code) = run_ltp(dir, &["node", "inspect", "UDE-001"]);
    assert_eq!(code, 0);
    assert_eq!(json["data"]["epistemic"], "hypothesis");
}

// === K4.19: node split — new nodes get default hypothesis ===
#[test]
fn k4_19_node_split_epistemic_not_inherited() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    run_ltp(dir, &["tree", "new", "CRT", "SplitTest"]);
    run_ltp(
        dir,
        &[
            "node",
            "add",
            "Original fact",
            "--type",
            "UDE",
            "--epistemic",
            "fact",
        ],
    );
    run_ltp(
        dir,
        &[
            "tree",
            "attach",
            "--tree",
            "tree-crt-splittest",
            "--node",
            "UDE-001",
        ],
    );

    let (json, code) = run_ltp(
        dir,
        &[
            "node",
            "split",
            "UDE-001",
            "--into",
            "Part A",
            "Part B",
            "--tree",
            "tree-crt-splittest",
        ],
    );
    assert_eq!(code, 0);

    // New nodes should have default hypothesis
    let new_ids: Vec<&str> = json["data"]["new_nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["id"].as_str().unwrap())
        .collect();

    for new_id in &new_ids {
        let (inspect, _) = run_ltp(dir, &["node", "inspect", new_id]);
        assert_eq!(
            inspect["data"]["epistemic"], "hypothesis",
            "Split node {} should have default hypothesis, not inherited fact",
            new_id
        );
    }
}
