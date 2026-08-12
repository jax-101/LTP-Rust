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
    let (json, code) = run_ltp(dir, &["init", "--name", "TestNodes"]);
    assert_eq!(code, 0, "init failed: {:?}", json);
}

/// UAT 2a.1: node add creates UDE-001.json with correct data.
#[test]
fn uat_2a_1_node_add_ude() {
    let tmp = tempfile::tempdir().expect("failed to create tempdir");
    let dir = tmp.path();
    setup_workspace(dir);

    let (json, code) = run_ltp(
        dir,
        &[
            "node",
            "add",
            "El tiempo de entrega supera 15 días",
            "--type",
            "UDE",
            "--tags",
            "logistica,critico",
        ],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["action"], "node_add");
    assert_eq!(json["data"]["id"], "UDE-001");
    assert_eq!(json["data"]["node_type"], "UDE");
    assert_eq!(json["data"]["label"], "El tiempo de entrega supera 15 días");
    assert_eq!(
        json["data"]["tags"],
        serde_json::json!(["logistica", "critico"])
    );
    assert_eq!(json["data"]["observable"], true);

    // Verify file on disk
    assert!(dir.join("nodes/UDE-001.json").is_file());

    // Verify counter incremented
    let counters: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join(".ltp/counters.json")).unwrap())
            .unwrap();
    assert_eq!(counters["UDE"], 1);
}

/// UAT 2a.2: node add with causal conjunction generates CLR#2 warning.
#[test]
fn uat_2a_2_node_add_clr2_warning() {
    let tmp = tempfile::tempdir().expect("failed to create tempdir");
    let dir = tmp.path();
    setup_workspace(dir);

    let (json, code) = run_ltp(
        dir,
        &[
            "node",
            "add",
            "Vendemos poco porque no hay marketing",
            "--type",
            "UDE",
        ],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);

    // Node is still created
    assert_eq!(json["data"]["id"], "UDE-001");

    // Warning present
    let warnings = json["warnings"]
        .as_array()
        .expect("warnings should be array");
    assert!(!warnings.is_empty());
    assert_eq!(warnings[0]["code"], "CLR2_CONJUNCTION_DETECTED");
}

/// UAT 2a.3: node add RC creates RC-001.json.
#[test]
fn uat_2a_3_node_add_rc() {
    let tmp = tempfile::tempdir().expect("failed to create tempdir");
    let dir = tmp.path();
    setup_workspace(dir);

    let (json, code) = run_ltp(dir, &["node", "add", "Falta de capacidad", "--type", "RC"]);

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["id"], "RC-001");
    assert!(dir.join("nodes/RC-001.json").is_file());

    // Counter RC incremented
    let counters: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join(".ltp/counters.json")).unwrap())
            .unwrap();
    assert_eq!(counters["RC"], 1);
}

/// UAT 2a.4: node edit updates label on disk.
#[test]
fn uat_2a_4_node_edit_label() {
    let tmp = tempfile::tempdir().expect("failed to create tempdir");
    let dir = tmp.path();
    setup_workspace(dir);

    // Create node first
    run_ltp(
        dir,
        &[
            "node",
            "add",
            "Texto original",
            "--type",
            "UDE",
            "--tags",
            "critico",
        ],
    );

    let (json, code) = run_ltp(dir, &["node", "edit", "UDE-001", "--label", "Nuevo texto"]);

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["label"], "Nuevo texto");

    // Verify on disk
    let node: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("nodes/UDE-001.json")).unwrap())
            .unwrap();
    assert_eq!(node["label"], "Nuevo texto");
}

/// UAT 2a.5: node edit adds a tag.
#[test]
fn uat_2a_5_node_edit_add_tag() {
    let tmp = tempfile::tempdir().expect("failed to create tempdir");
    let dir = tmp.path();
    setup_workspace(dir);

    run_ltp(
        dir,
        &[
            "node",
            "add",
            "Test node",
            "--type",
            "UDE",
            "--tags",
            "critico",
        ],
    );

    let (json, code) = run_ltp(dir, &["node", "edit", "UDE-001", "--add-tag", "urgente"]);

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(
        json["data"]["tags"],
        serde_json::json!(["critico", "urgente"])
    );
}

/// UAT 2a.6: node edit removes a tag.
#[test]
fn uat_2a_6_node_edit_rm_tag() {
    let tmp = tempfile::tempdir().expect("failed to create tempdir");
    let dir = tmp.path();
    setup_workspace(dir);

    run_ltp(
        dir,
        &[
            "node",
            "add",
            "Test node",
            "--type",
            "UDE",
            "--tags",
            "critico,logistica",
        ],
    );

    let (json, code) = run_ltp(dir, &["node", "edit", "UDE-001", "--rm-tag", "critico"]);

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["tags"], serde_json::json!(["logistica"]));
}

/// UAT 2a.7: node edit sets observable to false.
#[test]
fn uat_2a_7_node_edit_observable() {
    let tmp = tempfile::tempdir().expect("failed to create tempdir");
    let dir = tmp.path();
    setup_workspace(dir);

    run_ltp(dir, &["node", "add", "Test node", "--type", "UDE"]);

    let (json, code) = run_ltp(dir, &["node", "edit", "UDE-001", "--observable", "false"]);

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["observable"], false);
}

/// UAT 2a.8: node list shows all pool nodes.
#[test]
fn uat_2a_8_node_list() {
    let tmp = tempfile::tempdir().expect("failed to create tempdir");
    let dir = tmp.path();
    setup_workspace(dir);

    run_ltp(dir, &["node", "add", "First UDE", "--type", "UDE"]);
    run_ltp(dir, &["node", "add", "A root cause", "--type", "RC"]);
    run_ltp(dir, &["node", "add", "Second UDE", "--type", "UDE"]);

    let (json, code) = run_ltp(dir, &["node", "list"]);

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["count"], 3);

    let nodes = json["data"]["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 3);

    // Verify sorted by ID
    assert_eq!(nodes[0]["id"], "RC-001");
    assert_eq!(nodes[1]["id"], "UDE-001");
    assert_eq!(nodes[2]["id"], "UDE-002");
}

/// UAT 2a.9: node search finds by substring.
#[test]
fn uat_2a_9_node_search() {
    let tmp = tempfile::tempdir().expect("failed to create tempdir");
    let dir = tmp.path();
    setup_workspace(dir);

    run_ltp(
        dir,
        &[
            "node",
            "add",
            "El tiempo de entrega supera 15 días",
            "--type",
            "UDE",
        ],
    );
    run_ltp(dir, &["node", "add", "Falta de capacidad", "--type", "RC"]);
    run_ltp(
        dir,
        &[
            "node",
            "add",
            "Entregas tardías al cliente",
            "--type",
            "UDE",
        ],
    );

    let (json, code) = run_ltp(dir, &["node", "search", "--query", "entrega"]);

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["count"], 2);

    let matches = json["data"]["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 2);
}
