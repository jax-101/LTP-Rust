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
