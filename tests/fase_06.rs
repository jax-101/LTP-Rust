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
    let (_, code) = run_ltp(dir, &["init", "--name", "F6Test"]);
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

/// `assume add` is not implemented yet (src/assume/mod.rs is a stub), so
/// UATs that need an edge with an assumption inject it directly into the
/// tree JSON file on disk instead of going through the CLI.
fn inject_assumption(
    dir: &std::path::Path,
    tree_id: &str,
    link_id: &str,
    asm_id: &str,
    text: &str,
) {
    let tree_file = dir.join("trees").join(format!("{}.json", tree_id));
    let mut tree_content: Value =
        serde_json::from_str(&std::fs::read_to_string(&tree_file).unwrap()).unwrap();

    let edges = tree_content["edges"].as_array_mut().unwrap();
    let edge = edges
        .iter_mut()
        .find(|e| e["id"] == link_id)
        .expect("link_id not found in tree edges");

    let assumption = serde_json::json!({
        "id": asm_id,
        "status": "valid",
        "text": text,
    });

    match edge["assumptions"].as_array_mut() {
        Some(arr) => arr.push(assumption),
        None => edge["assumptions"] = serde_json::json!([assumption]),
    }

    std::fs::write(
        &tree_file,
        serde_json::to_string_pretty(&tree_content).unwrap(),
    )
    .unwrap();
}

/// UAT 6.1: reverse sin assumptions invierte from<->to.
#[test]
fn uat_6_1_link_reverse_no_assumptions() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let a = add_node(dir, "Cause", "rc");
    let b = add_node(dir, "Effect", "ude");
    let tree = create_tree(dir, "crt", "ReverseTest");
    attach_node(dir, &tree, &a);
    attach_node(dir, &tree, &b);
    let link = connect(dir, &tree, &a, &b);

    let (json, code) = run_ltp(dir, &["link", "reverse", "--tree", &tree, "--link", &link]);

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["action"], "link_reverse");
    assert_eq!(json["data"]["new_from"][0], b);
    assert_eq!(json["data"]["new_to"], a);

    // Verify persisted on disk.
    let tree_file = dir.join("trees").join(format!("{}.json", tree));
    let tree_content: Value =
        serde_json::from_str(&std::fs::read_to_string(&tree_file).unwrap()).unwrap();
    let edge = &tree_content["edges"][0];
    assert_eq!(edge["from"][0].as_str().unwrap(), b);
    assert_eq!(edge["to"].as_str().unwrap(), a);
}

/// UAT 6.2: reverse con assumptions sin --force -> error.
#[test]
fn uat_6_2_link_reverse_with_assumptions_no_force() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let a = add_node(dir, "Cause", "rc");
    let b = add_node(dir, "Effect", "ude");
    let tree = create_tree(dir, "crt", "ReverseTest2");
    attach_node(dir, &tree, &a);
    attach_node(dir, &tree, &b);
    let link = connect(dir, &tree, &a, &b);

    inject_assumption(dir, &tree, &link, "ASM-001", "Some assumption");

    let (json, code) = run_ltp(dir, &["link", "reverse", "--tree", &tree, "--link", &link]);

    assert_eq!(code, 1);
    assert_eq!(json["success"], false);
    assert_eq!(json["errors"][0]["code"], "REVERSE_REQUIRES_FORCE");

    // Edge must remain unchanged since the reversal was rejected.
    let tree_file = dir.join("trees").join(format!("{}.json", tree));
    let tree_content: Value =
        serde_json::from_str(&std::fs::read_to_string(&tree_file).unwrap()).unwrap();
    let edge = &tree_content["edges"][0];
    assert_eq!(edge["from"][0].as_str().unwrap(), a);
    assert_eq!(edge["to"].as_str().unwrap(), b);
}

/// UAT 6.3: reverse con assumptions + --force -> invierte + marks needs_review.
#[test]
fn uat_6_3_link_reverse_with_force() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let a = add_node(dir, "Cause", "rc");
    let b = add_node(dir, "Effect", "ude");
    let tree = create_tree(dir, "crt", "ReverseTest3");
    attach_node(dir, &tree, &a);
    attach_node(dir, &tree, &b);
    let link = connect(dir, &tree, &a, &b);

    inject_assumption(dir, &tree, &link, "ASM-001", "Test assumption");

    let (json, code) = run_ltp(
        dir,
        &[
            "link", "reverse", "--tree", &tree, "--link", &link, "--force",
        ],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["new_from"][0], b);
    assert_eq!(json["data"]["new_to"], a);

    // Assumption must be marked needs_review after a forced reversal.
    let tree_file = dir.join("trees").join(format!("{}.json", tree));
    let tree_content: Value =
        serde_json::from_str(&std::fs::read_to_string(&tree_file).unwrap()).unwrap();
    let edge = &tree_content["edges"][0];
    assert_eq!(edge["assumptions"][0]["status"], "needs_review");
}

/// UAT 6.4: link move redirige destino.
#[test]
fn uat_6_4_link_move_new_to() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let a = add_node(dir, "Cause", "rc");
    let b = add_node(dir, "Old Effect", "ude");
    let d = add_node(dir, "New Effect", "ude");
    let tree = create_tree(dir, "crt", "MoveTest");
    attach_node(dir, &tree, &a);
    attach_node(dir, &tree, &b);
    attach_node(dir, &tree, &d);
    let link = connect(dir, &tree, &a, &b);

    let (json, code) = run_ltp(
        dir,
        &[
            "link", "move", "--tree", &tree, "--link", &link, "--new-to", &d,
        ],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["action"], "link_move");
    assert_eq!(json["data"]["link_id"], link);

    // Verify persisted on disk.
    let tree_file = dir.join("trees").join(format!("{}.json", tree));
    let tree_content: Value =
        serde_json::from_str(&std::fs::read_to_string(&tree_file).unwrap()).unwrap();
    let edge = &tree_content["edges"][0];
    assert_eq!(edge["from"][0].as_str().unwrap(), a);
    assert_eq!(edge["to"].as_str().unwrap(), d);
}

/// UAT 6.5: insert-between SINGLE — A→B becomes A→X→B.
#[test]
fn uat_6_5_insert_between_single() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let a = add_node(dir, "Cause A", "rc");
    let b = add_node(dir, "Effect B", "ude");
    let x = add_node(dir, "Intermediate X", "int");
    let tree = create_tree(dir, "crt", "InsertTest");
    attach_node(dir, &tree, &a);
    attach_node(dir, &tree, &b);
    attach_node(dir, &tree, &x);
    let link = connect(dir, &tree, &a, &b);

    let (json, code) = run_ltp(
        dir,
        &[
            "link",
            "insert-between",
            "--tree",
            &tree,
            "--link",
            &link,
            "--node",
            &x,
        ],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["removed_link"], link);
    assert_eq!(json["data"]["created_links"].as_array().unwrap().len(), 2);

    // Verify persisted on disk: original edge gone, two new edges A->X, X->B.
    let tree_file = dir.join("trees").join(format!("{}.json", tree));
    let tree_content: Value =
        serde_json::from_str(&std::fs::read_to_string(&tree_file).unwrap()).unwrap();
    let edges = tree_content["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 2);
    assert!(!edges.iter().any(|e| e["id"] == link));
    let a_to_x = edges
        .iter()
        .find(|e| e["from"][0] == a)
        .expect("edge A->X missing");
    assert_eq!(a_to_x["to"], x);
    let x_to_b = edges
        .iter()
        .find(|e| e["from"][0] == x)
        .expect("edge X->B missing");
    assert_eq!(x_to_b["to"], b);
}

/// UAT 6.6: insert-between AND + --insert-after-cause A.
#[test]
fn uat_6_6_insert_between_and_after_cause() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let a = add_node(dir, "Cause A", "rc");
    let b = add_node(dir, "Cause B", "rc");
    let c = add_node(dir, "Effect C", "ude");
    let x = add_node(dir, "Intermediate X", "int");
    let tree = create_tree(dir, "crt", "InsertAND");
    attach_node(dir, &tree, &a);
    attach_node(dir, &tree, &b);
    attach_node(dir, &tree, &c);
    attach_node(dir, &tree, &x);

    // Create AND edge [A,B] → C
    let (json, code) = run_ltp(
        dir,
        &[
            "link",
            "connect",
            "--tree",
            &tree,
            "--from",
            &format!("{},{}", a, b),
            "--to",
            &c,
            "--operator",
            "AND",
        ],
    );
    assert_eq!(code, 0);
    let link = json["data"]["created_links"][0]
        .as_str()
        .unwrap()
        .to_string();

    let (json, code) = run_ltp(
        dir,
        &[
            "link",
            "insert-between",
            "--tree",
            &tree,
            "--link",
            &link,
            "--node",
            &x,
            "--insert-after-cause",
            &a,
        ],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    // A extracted from group, A→X created; X replaces A in the group.
    // The original grouped edge survives (modified), so nothing is removed
    // and only the one new A->X edge is created.
    assert_eq!(json["data"]["removed_link"], "");
    assert_eq!(json["data"]["created_links"].as_array().unwrap().len(), 1);

    // Verify persisted on disk: original edge still present with X in from[]
    // instead of A, plus a new SINGLE edge A->X.
    let tree_file = dir.join("trees").join(format!("{}.json", tree));
    let tree_content: Value =
        serde_json::from_str(&std::fs::read_to_string(&tree_file).unwrap()).unwrap();
    let edges = tree_content["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 2);
    let original = edges
        .iter()
        .find(|e| e["id"] == link)
        .expect("original grouped edge missing");
    let from: Vec<&str> = original["from"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(from.contains(&x.as_str()));
    assert!(!from.contains(&a.as_str()));
    assert_eq!(original["to"], c);
    let a_to_x = edges
        .iter()
        .find(|e| e["id"] != link)
        .expect("new A->X edge missing");
    assert_eq!(a_to_x["from"][0], a);
    assert_eq!(a_to_x["to"], x);
}

/// UAT 6.7: insert-between AND + --insert-before-effect.
#[test]
fn uat_6_7_insert_between_before_effect() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let a = add_node(dir, "Cause A", "rc");
    let b = add_node(dir, "Cause B", "rc");
    let c = add_node(dir, "Effect C", "ude");
    let x = add_node(dir, "Intermediate X", "int");
    let tree = create_tree(dir, "crt", "InsertBeforeEffect");
    attach_node(dir, &tree, &a);
    attach_node(dir, &tree, &b);
    attach_node(dir, &tree, &c);
    attach_node(dir, &tree, &x);

    let (json, code) = run_ltp(
        dir,
        &[
            "link",
            "connect",
            "--tree",
            &tree,
            "--from",
            &format!("{},{}", a, b),
            "--to",
            &c,
            "--operator",
            "AND",
        ],
    );
    assert_eq!(code, 0);
    let link = json["data"]["created_links"][0]
        .as_str()
        .unwrap()
        .to_string();

    let (json, code) = run_ltp(
        dir,
        &[
            "link",
            "insert-between",
            "--tree",
            &tree,
            "--link",
            &link,
            "--node",
            &x,
            "--insert-before-effect",
        ],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    // [A,B]→C becomes [A,B]→X + X→C: original removed, two new edges.
    assert_eq!(json["data"]["created_links"].as_array().unwrap().len(), 2);
    assert_eq!(json["data"]["removed_link"], link);

    // Verify persisted on disk.
    let tree_file = dir.join("trees").join(format!("{}.json", tree));
    let tree_content: Value =
        serde_json::from_str(&std::fs::read_to_string(&tree_file).unwrap()).unwrap();
    let edges = tree_content["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 2);
    assert!(!edges.iter().any(|e| e["id"] == link));
    let ab_to_x = edges
        .iter()
        .find(|e| e["to"] == x)
        .expect("edge [A,B]->X missing");
    assert_eq!(ab_to_x["operator"], "AND");
    let from: Vec<&str> = ab_to_x["from"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(from.contains(&a.as_str()));
    assert!(from.contains(&b.as_str()));
    let x_to_c = edges
        .iter()
        .find(|e| e["to"] == c)
        .expect("edge X->C missing");
    assert_eq!(x_to_c["from"][0], x);
    assert_eq!(x_to_c["operator"], "SINGLE");
}

/// UAT 6.8: group two SINGLE edges -> one AND edge.
#[test]
fn uat_6_8_link_group() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let a = add_node(dir, "Cause A", "rc");
    let b = add_node(dir, "Cause B", "rc");
    let c = add_node(dir, "Effect C", "ude");
    let tree = create_tree(dir, "crt", "GroupTest");
    attach_node(dir, &tree, &a);
    attach_node(dir, &tree, &b);
    attach_node(dir, &tree, &c);
    let l1 = connect(dir, &tree, &a, &c);
    let l2 = connect(dir, &tree, &b, &c);

    let (json, code) = run_ltp(
        dir,
        &[
            "link",
            "group",
            "--tree",
            &tree,
            "--links",
            &format!("{},{}", l1, l2),
            "--operator",
            "AND",
        ],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["action"], "link_group");
    assert!(json["data"]["created_link"]
        .as_str()
        .unwrap()
        .starts_with("LINK-"));
    assert_eq!(json["data"]["removed_links"].as_array().unwrap().len(), 2);

    // Verify persisted on disk: originals gone, one AND edge with both causes.
    let tree_file = dir.join("trees").join(format!("{}.json", tree));
    let tree_content: Value =
        serde_json::from_str(&std::fs::read_to_string(&tree_file).unwrap()).unwrap();
    let edges = tree_content["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 1);
    assert!(!edges.iter().any(|e| e["id"] == l1 || e["id"] == l2));
    let grouped = &edges[0];
    assert_eq!(grouped["operator"], "AND");
    assert_eq!(grouped["to"], c);
    let from: Vec<&str> = grouped["from"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(from.contains(&a.as_str()));
    assert!(from.contains(&b.as_str()));
}

/// UAT 6.9: group edges that don't share same `to` -> error.
#[test]
fn uat_6_9_link_group_different_to() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let a = add_node(dir, "Cause A", "rc");
    let b = add_node(dir, "Cause B", "rc");
    let c = add_node(dir, "Effect C", "ude");
    let d = add_node(dir, "Effect D", "ude");
    let tree = create_tree(dir, "crt", "GroupFail");
    attach_node(dir, &tree, &a);
    attach_node(dir, &tree, &b);
    attach_node(dir, &tree, &c);
    attach_node(dir, &tree, &d);
    let l1 = connect(dir, &tree, &a, &c);
    let l2 = connect(dir, &tree, &b, &d);

    let (json, code) = run_ltp(
        dir,
        &[
            "link",
            "group",
            "--tree",
            &tree,
            "--links",
            &format!("{},{}", l1, l2),
            "--operator",
            "AND",
        ],
    );

    assert_eq!(code, 1);
    assert_eq!(json["success"], false);
    assert_eq!(json["errors"][0]["code"], "GROUP_DESTINATION_MISMATCH");

    // Nothing should have been mutated on disk.
    let tree_file = dir.join("trees").join(format!("{}.json", tree));
    let tree_content: Value =
        serde_json::from_str(&std::fs::read_to_string(&tree_file).unwrap()).unwrap();
    let edges = tree_content["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 2);
}

/// UAT 6.10: dissolve a grouped edge -> each cause becomes SINGLE.
///
/// `assume add` is not implemented yet, so the assumption used to verify
/// inheritance is injected directly into the tree JSON (see
/// `inject_assumption` above), mirroring how UATs 6.2/6.3 exercise
/// assumption-bearing edges.
#[test]
fn uat_6_10_link_dissolve() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let a = add_node(dir, "Cause A", "rc");
    let b = add_node(dir, "Cause B", "rc");
    let c = add_node(dir, "Effect C", "ude");
    let tree = create_tree(dir, "crt", "DissolveTest");
    attach_node(dir, &tree, &a);
    attach_node(dir, &tree, &b);
    attach_node(dir, &tree, &c);

    // Create AND edge [A,B]→C
    let (json, code) = run_ltp(
        dir,
        &[
            "link",
            "connect",
            "--tree",
            &tree,
            "--from",
            &format!("{},{}", a, b),
            "--to",
            &c,
            "--operator",
            "AND",
        ],
    );
    assert_eq!(code, 0);
    let group_link = json["data"]["created_links"][0]
        .as_str()
        .unwrap()
        .to_string();

    // Add assumption to test inheritance (assume add not implemented yet).
    inject_assumption(dir, &tree, &group_link, "ASM-001", "Some assumption");

    let (json, code) = run_ltp(
        dir,
        &["link", "dissolve", "--tree", &tree, "--link", &group_link],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["removed_link"], group_link);
    assert_eq!(json["data"]["created_links"].as_array().unwrap().len(), 2);

    // Verify persisted on disk: original gone, two SINGLE edges to C, each
    // inheriting the assumption marked needs_review.
    let tree_file = dir.join("trees").join(format!("{}.json", tree));
    let tree_content: Value =
        serde_json::from_str(&std::fs::read_to_string(&tree_file).unwrap()).unwrap();
    let edges = tree_content["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 2);
    assert!(!edges.iter().any(|e| e["id"] == group_link));
    for edge in edges {
        assert_eq!(edge["operator"], "SINGLE");
        assert_eq!(edge["to"], c);
        assert_eq!(edge["from"].as_array().unwrap().len(), 1);
        assert_eq!(edge["assumptions"][0]["status"], "needs_review");
    }
    let causes: Vec<&str> = edges
        .iter()
        .map(|e| e["from"][0].as_str().unwrap())
        .collect();
    assert!(causes.contains(&a.as_str()));
    assert!(causes.contains(&b.as_str()));
}
