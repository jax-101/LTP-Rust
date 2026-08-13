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
    let (_, code) = run_ltp(dir, &["init", "--name", "ValidateTest"]);
    assert_eq!(code, 0);
}

fn add_node(dir: &std::path::Path, label: &str, node_type: &str) -> String {
    let (json, _) = run_ltp(dir, &["node", "add", label, "--type", node_type]);
    json["data"]["id"].as_str().unwrap().to_string()
}

fn add_node_with_observable(
    dir: &std::path::Path,
    label: &str,
    node_type: &str,
    observable: bool,
) -> String {
    let obs_str = if observable { "true" } else { "false" };
    let (json, _) = run_ltp(
        dir,
        &[
            "node",
            "add",
            label,
            "--type",
            node_type,
            "--observable",
            obs_str,
        ],
    );
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

fn attach_node_with_role(dir: &std::path::Path, tree_id: &str, node_id: &str, role: &str) {
    let (_, code) = run_ltp(
        dir,
        &[
            "tree", "attach", "--tree", tree_id, "--node", node_id, "--role", role,
        ],
    );
    assert_eq!(code, 0);
}

fn connect(dir: &std::path::Path, tree_id: &str, from: &str, to: &str) {
    let (_, code) = run_ltp(
        dir,
        &[
            "link", "connect", "--tree", tree_id, "--from", from, "--to", to,
        ],
    );
    assert_eq!(code, 0);
}

fn connect_and(dir: &std::path::Path, tree_id: &str, from: &str, to: &str) {
    let (_, code) = run_ltp(
        dir,
        &[
            "link",
            "connect",
            "--tree",
            tree_id,
            "--from",
            from,
            "--to",
            to,
            "--operator",
            "AND",
        ],
    );
    assert_eq!(code, 0);
}

/// UAT 5.1: validate on healthy workspace returns no errors, no warnings.
#[test]
fn uat_5_1_validate_healthy_workspace() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let node_a = add_node(dir, "Cause A", "rc");
    let node_b = add_node(dir, "Effect B", "ude");

    let tree = create_tree(dir, "crt", "CRT Test");
    attach_node(dir, &tree, &node_a);
    attach_node(dir, &tree, &node_b);
    connect(dir, &tree, &node_a, &node_b);

    let (json, code) = run_ltp(dir, &["validate"]);
    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["total_errors"], 0);
    // CLR#4 warning expected (single input to B), but no errors
    assert_eq!(
        json["data"]["details"][0]["errors"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
}

/// UAT 5.2: Tree with manually injected cycle -> CIRCULAR_DEPENDENCY_DETECTED.
#[test]
fn uat_5_2_cycle_detected() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let node_a = add_node(dir, "Node A", "rc");
    let node_b = add_node(dir, "Node B", "ude");
    let node_c = add_node(dir, "Node C", "int");

    let tree = create_tree(dir, "crt", "CRT Cycle");
    attach_node(dir, &tree, &node_a);
    attach_node(dir, &tree, &node_b);
    attach_node(dir, &tree, &node_c);
    connect(dir, &tree, &node_a, &node_b);
    connect(dir, &tree, &node_b, &node_c);

    // Manually inject a cycle by editing the tree JSON
    let tree_path = dir.join("trees").join(format!("{}.json", tree));
    let content = std::fs::read_to_string(&tree_path).unwrap();
    let mut tree_json: Value = serde_json::from_str(&content).unwrap();

    let cycle_edge = serde_json::json!({
        "id": "LINK-999",
        "from": [node_c],
        "to": node_a,
        "operator": "SINGLE",
        "weight": null,
        "status": "active",
        "logic": "SUFFICIENCY",
        "assumptions": []
    });
    tree_json["edges"].as_array_mut().unwrap().push(cycle_edge);
    std::fs::write(
        &tree_path,
        serde_json::to_string_pretty(&tree_json).unwrap(),
    )
    .unwrap();

    let (json, code) = run_ltp(dir, &["validate"]);
    assert_eq!(code, 1);
    assert_eq!(json["success"], false);

    let errors = &json["data"]["details"][0]["errors"];
    assert!(errors
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["code"] == "CIRCULAR_DEPENDENCY_DETECTED"));
}

/// UAT 5.3: Edge references non-existent node -> REFERENTIAL_INTEGRITY_VIOLATION.
#[test]
fn uat_5_3_referential_integrity_violation() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let node_a = add_node(dir, "Node A", "rc");
    let tree = create_tree(dir, "crt", "CRT Integrity");
    attach_node(dir, &tree, &node_a);

    // Manually inject an edge referencing a non-existent node
    let tree_path = dir.join("trees").join(format!("{}.json", tree));
    let content = std::fs::read_to_string(&tree_path).unwrap();
    let mut tree_json: Value = serde_json::from_str(&content).unwrap();

    let bad_edge = serde_json::json!({
        "id": "LINK-999",
        "from": [node_a],
        "to": "GHOST-001",
        "operator": "SINGLE",
        "weight": null,
        "status": "active",
        "logic": "SUFFICIENCY",
        "assumptions": []
    });
    tree_json["edges"].as_array_mut().unwrap().push(bad_edge);
    std::fs::write(
        &tree_path,
        serde_json::to_string_pretty(&tree_json).unwrap(),
    )
    .unwrap();

    let (json, code) = run_ltp(dir, &["validate"]);
    assert_eq!(code, 1);

    let errors = &json["data"]["details"][0]["errors"];
    assert!(errors
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["code"] == "REFERENTIAL_INTEGRITY_VIOLATION"));
}

/// UAT 5.4: EC without node with role 'objective' -> EC_VALIDATION.
#[test]
fn uat_5_4_ec_missing_objective() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let req1 = add_node(dir, "Requirement 1", "req");
    let req2 = add_node(dir, "Requirement 2", "req");

    let tree = create_tree(dir, "ec", "EC Missing Obj");
    attach_node_with_role(dir, &tree, &req1, "requirement");
    attach_node_with_role(dir, &tree, &req2, "requirement");

    let (json, code) = run_ltp(dir, &["validate"]);
    assert_eq!(code, 1);

    let errors = &json["data"]["details"][0]["errors"];
    assert!(errors.as_array().unwrap().iter().any(
        |e| e["code"] == "EC_VALIDATION" && e["detail"].as_str().unwrap().contains("objective")
    ));
}

/// UAT 5.5: EC with only 1 requirement -> EC_VALIDATION.
#[test]
fn uat_5_5_ec_one_requirement() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let obj = add_node(dir, "Objective", "obj");
    let req1 = add_node(dir, "Requirement 1", "req");

    let tree = create_tree(dir, "ec", "EC One Req");
    attach_node_with_role(dir, &tree, &obj, "objective");
    attach_node_with_role(dir, &tree, &req1, "requirement");

    let (json, code) = run_ltp(dir, &["validate"]);
    assert_eq!(code, 1);

    let errors = &json["data"]["details"][0]["errors"];
    assert!(errors
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["code"] == "EC_VALIDATION"
            && e["detail"].as_str().unwrap().contains("at least 2")));
}

/// UAT 5.6: EC requirement without prerequisite -> EC_VALIDATION.
#[test]
fn uat_5_6_ec_requirement_without_prerequisite() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let obj = add_node(dir, "Objective", "obj");
    let req1 = add_node(dir, "Requirement 1", "req");
    let req2 = add_node(dir, "Requirement 2", "req");
    let pre1 = add_node(dir, "Prerequisite 1", "pre");

    let tree = create_tree(dir, "ec", "EC No Pre");
    attach_node_with_role(dir, &tree, &obj, "objective");
    attach_node_with_role(dir, &tree, &req1, "requirement");
    attach_node_with_role(dir, &tree, &req2, "requirement");
    attach_node_with_role(dir, &tree, &pre1, "prerequisite");
    connect(dir, &tree, &pre1, &req1);

    let (json, code) = run_ltp(dir, &["validate"]);
    assert_eq!(code, 1);

    let errors = &json["data"]["details"][0]["errors"];
    assert!(errors.as_array().unwrap().iter().any(|e| {
        e["code"] == "EC_VALIDATION" && e["detail"].as_str().unwrap().contains("no prerequisite")
    }));
}

/// UAT 5.7: Node with "porque" in label -> CLR2 warning.
#[test]
fn uat_5_7_clr2_conjunction_warning() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let node_a = add_node(dir, "Causa raiz", "rc");
    let node_b = add_node(dir, "Vendemos poco porque no hay marketing", "ude");

    let tree = create_tree(dir, "crt", "CRT CLR2");
    attach_node(dir, &tree, &node_a);
    attach_node(dir, &tree, &node_b);
    connect(dir, &tree, &node_a, &node_b);

    let (json, code) = run_ltp(dir, &["validate"]);
    assert_eq!(code, 0); // warnings don't cause failure

    let warnings = &json["data"]["details"][0]["warnings"];
    assert!(warnings
        .as_array()
        .unwrap()
        .iter()
        .any(|w| w["code"] == "CLR2_CONJUNCTION_DETECTED"));
}

/// UAT 5.8: Node with only 1 incoming SINGLE edge -> CLR4 warning.
#[test]
fn uat_5_8_clr4_insufficient_cause() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let node_a = add_node(dir, "Single cause", "rc");
    let node_b = add_node(dir, "Effect", "ude");

    let tree = create_tree(dir, "crt", "CRT CLR4");
    attach_node(dir, &tree, &node_a);
    attach_node(dir, &tree, &node_b);
    connect(dir, &tree, &node_a, &node_b);

    let (json, code) = run_ltp(dir, &["validate"]);
    assert_eq!(code, 0);

    let warnings = &json["data"]["details"][0]["warnings"];
    assert!(warnings
        .as_array()
        .unwrap()
        .iter()
        .any(|w| w["code"] == "CLR4_INSUFFICIENT_CAUSE"));
}

/// UAT 5.9: AND edge with >4 inputs -> CLR4/5 warning.
#[test]
fn uat_5_9_clr4_5_excessive_and() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let n1 = add_node(dir, "Cause 1", "rc");
    let n2 = add_node(dir, "Cause 2", "rc");
    let n3 = add_node(dir, "Cause 3", "rc");
    let n4 = add_node(dir, "Cause 4", "rc");
    let n5 = add_node(dir, "Cause 5", "rc");
    let effect = add_node(dir, "Big effect", "ude");

    let tree = create_tree(dir, "crt", "CRT AND");
    attach_node(dir, &tree, &n1);
    attach_node(dir, &tree, &n2);
    attach_node(dir, &tree, &n3);
    attach_node(dir, &tree, &n4);
    attach_node(dir, &tree, &n5);
    attach_node(dir, &tree, &effect);

    let from_list = format!("{},{},{},{},{}", n1, n2, n3, n4, n5);
    connect_and(dir, &tree, &from_list, &effect);

    let (json, code) = run_ltp(dir, &["validate"]);
    assert_eq!(code, 0);

    let warnings = &json["data"]["details"][0]["warnings"];
    assert!(warnings
        .as_array()
        .unwrap()
        .iter()
        .any(|w| w["code"] == "CLR4_5_EXCESSIVE_AND_INPUTS"));
}

/// UAT 5.10: Intangible node (observable: false) with <2 outgoing edges -> CLR7 warning.
#[test]
fn uat_5_10_clr7_intangible_no_predicted() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let intangible = add_node_with_observable(dir, "Hidden cause", "rc", false);
    let effect = add_node(dir, "Effect", "ude");

    let tree = create_tree(dir, "crt", "CRT CLR7");
    attach_node(dir, &tree, &intangible);
    attach_node(dir, &tree, &effect);
    connect(dir, &tree, &intangible, &effect);

    let (json, code) = run_ltp(dir, &["validate"]);
    assert_eq!(code, 0);

    let warnings = &json["data"]["details"][0]["warnings"];
    assert!(warnings
        .as_array()
        .unwrap()
        .iter()
        .any(|w| w["code"] == "CLR7_INTANGIBLE_NO_PREDICTED"));
}

/// UAT 5.11: UDE in from pointing to RC -> CLR6 type inversion warning.
#[test]
fn uat_5_11_clr6_type_inversion() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let ude = add_node(dir, "Undesirable effect", "ude");
    let rc = add_node(dir, "Root cause", "rc");

    let tree = create_tree(dir, "crt", "CRT CLR6");
    attach_node(dir, &tree, &ude);
    attach_node(dir, &tree, &rc);
    connect(dir, &tree, &ude, &rc);

    let (json, code) = run_ltp(dir, &["validate"]);
    assert_eq!(code, 0);

    let warnings = &json["data"]["details"][0]["warnings"];
    assert!(warnings
        .as_array()
        .unwrap()
        .iter()
        .any(|w| w["code"] == "CLR6_TYPE_INVERSION"));
}

/// UAT 5.12: Node attached to tree without edges -> ORPHAN_NODE_IN_TREE warning.
#[test]
fn uat_5_12_orphan_node_in_tree() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let node_a = add_node(dir, "Connected A", "rc");
    let node_b = add_node(dir, "Connected B", "ude");
    let orphan = add_node(dir, "Orphan node", "int");

    let tree = create_tree(dir, "crt", "CRT Orphan");
    attach_node(dir, &tree, &node_a);
    attach_node(dir, &tree, &node_b);
    attach_node(dir, &tree, &orphan);
    connect(dir, &tree, &node_a, &node_b);

    let (json, code) = run_ltp(dir, &["validate"]);
    assert_eq!(code, 0);

    let warnings = &json["data"]["details"][0]["warnings"];
    assert!(warnings
        .as_array()
        .unwrap()
        .iter()
        .any(|w| w["code"] == "ORPHAN_NODE_IN_TREE"));
}

/// UAT 5.13: validate --tree filters to a single tree.
#[test]
fn uat_5_13_validate_single_tree() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let node_a = add_node(dir, "Node A", "rc");
    let node_b = add_node(dir, "Node B", "ude");

    let tree1 = create_tree(dir, "crt", "CRT One");
    let tree2 = create_tree(dir, "crt", "CRT Two");
    attach_node(dir, &tree1, &node_a);
    attach_node(dir, &tree1, &node_b);
    attach_node(dir, &tree2, &node_a);
    attach_node(dir, &tree2, &node_b);
    connect(dir, &tree1, &node_a, &node_b);
    connect(dir, &tree2, &node_a, &node_b);

    let (json, code) = run_ltp(dir, &["validate", "--tree", &tree1]);
    assert_eq!(code, 0);
    assert_eq!(json["data"]["trees_validated"], 1);
    assert_eq!(json["data"]["details"][0]["tree_id"], tree1);
}

/// UAT 5.14: NBR with internal cycle -> CIRCULAR_DEPENDENCY_DETECTED.
#[test]
fn uat_5_14_nbr_cycle() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let node_a = add_node(dir, "Node A", "rc");
    let node_b = add_node(dir, "Node B", "ude");

    let tree = create_tree(dir, "frt", "FRT NBR");
    attach_node(dir, &tree, &node_a);
    attach_node(dir, &tree, &node_b);
    connect(dir, &tree, &node_a, &node_b);

    // Manually inject NBR with cycle
    let tree_path = dir.join("trees").join(format!("{}.json", tree));
    let content = std::fs::read_to_string(&tree_path).unwrap();
    let mut tree_json: Value = serde_json::from_str(&content).unwrap();

    let nbr = serde_json::json!({
        "id": "NBR-001",
        "source_node": node_a,
        "edges": [
            {
                "id": "LINK-N1",
                "from": ["X"],
                "to": "Y",
                "operator": "SINGLE",
                "weight": null,
                "status": "active",
                "logic": "SUFFICIENCY",
                "assumptions": []
            },
            {
                "id": "LINK-N2",
                "from": ["Y"],
                "to": "X",
                "operator": "SINGLE",
                "weight": null,
                "status": "active",
                "logic": "SUFFICIENCY",
                "assumptions": []
            }
        ],
        "trim_injection": null
    });
    tree_json["nbr_branches"].as_array_mut().unwrap().push(nbr);
    std::fs::write(
        &tree_path,
        serde_json::to_string_pretty(&tree_json).unwrap(),
    )
    .unwrap();

    let (json, code) = run_ltp(dir, &["validate"]);
    assert_eq!(code, 1);

    let errors = &json["data"]["details"][0]["errors"];
    let nbr_error = errors
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["code"] == "CIRCULAR_DEPENDENCY_DETECTED" && e.get("nbr_id").is_some());
    assert!(nbr_error.is_some());
}
