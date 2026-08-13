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
    let (_, code) = run_ltp(dir, &["init", "--name", "F10Test"]);
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

#[allow(dead_code)]
fn connect_nodes(dir: &std::path::Path, tree_id: &str, from: &str, to: &str) -> String {
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

/// Helper: set up FRT with INJ-001 attached
fn setup_frt_with_injection(dir: &std::path::Path) -> (String, String) {
    setup_workspace(dir);
    let inj = add_node(dir, "Inyeccion A", "inj");
    let tree_id = create_tree(dir, "frt", "Mi FRT");
    attach_node(dir, &tree_id, &inj);
    (tree_id, inj)
}

// --- UAT 10.1: nbr add creates empty NBR ---
#[test]
fn uat_10_1_nbr_add_creates_empty_nbr() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree_id, inj) = setup_frt_with_injection(dir);

    let (json, code) = run_ltp(
        dir,
        &["nbr", "add", "--tree", &tree_id, "--source-node", &inj],
    );
    assert_eq!(code, 0);
    assert!(json["success"].as_bool().unwrap());
    assert_eq!(json["action"], "nbr_add");
    assert_eq!(json["data"]["source_node"].as_str().unwrap(), inj);
    assert!(json["data"]["nbr_id"].as_str().unwrap().starts_with("NBR-"));
    assert!(json["data"]["trim_injection"].is_null());
}

// --- UAT 10.2: link connect --nbr creates edge inside NBR ---
#[test]
fn uat_10_2_link_connect_nbr_creates_edge_inside_nbr() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree_id, inj) = setup_frt_with_injection(dir);

    // Create a second node for the NBR chain
    let nde = add_node(dir, "Negative effect", "ude");

    // Create NBR
    let (json, _) = run_ltp(
        dir,
        &["nbr", "add", "--tree", &tree_id, "--source-node", &inj],
    );
    let nbr_id = json["data"]["nbr_id"].as_str().unwrap().to_string();

    // Connect inside NBR
    let (json, code) = run_ltp(
        dir,
        &[
            "link", "connect", "--tree", &tree_id, "--nbr", &nbr_id, "--from", &inj, "--to", &nde,
        ],
    );
    assert_eq!(code, 0, "link connect --nbr failed: {:?}", json);
    assert!(json["success"].as_bool().unwrap());
    assert!(!json["data"]["created_links"].as_array().unwrap().is_empty());

    // Verify edge is in NBR, not in trunk
    let (inspect, _) = run_ltp(dir, &["nbr", "inspect", &nbr_id, "--tree", &tree_id]);
    assert_eq!(inspect["data"]["edge_count"].as_u64().unwrap(), 1);
}

// --- UAT 10.3: nbr list shows summary ---
#[test]
fn uat_10_3_nbr_list_shows_summary() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree_id, inj) = setup_frt_with_injection(dir);

    // Create 2 NBRs
    run_ltp(
        dir,
        &["nbr", "add", "--tree", &tree_id, "--source-node", &inj],
    );
    let inj2 = add_node(dir, "Trim injection", "inj");
    attach_node(dir, &tree_id, &inj2);
    run_ltp(
        dir,
        &[
            "nbr",
            "add",
            "--tree",
            &tree_id,
            "--source-node",
            &inj,
            "--trim",
            &inj2,
        ],
    );

    let (json, code) = run_ltp(dir, &["nbr", "list", "--tree", &tree_id]);
    assert_eq!(code, 0);
    assert_eq!(json["data"]["nbr_count"].as_u64().unwrap(), 2);

    let branches = json["data"]["branches"].as_array().unwrap();
    assert_eq!(branches.len(), 2);
    assert!(
        !branches[1]["has_trim"].as_bool().unwrap() || branches[1]["has_trim"].as_bool().unwrap()
    );
    // Second NBR should have trim
    let with_trim = branches.iter().find(|b| b["has_trim"].as_bool().unwrap());
    assert!(with_trim.is_some());
}

// --- UAT 10.4: nbr inspect shows full chain ---
#[test]
fn uat_10_4_nbr_inspect_full_chain() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree_id, inj) = setup_frt_with_injection(dir);
    let nde1 = add_node(dir, "Bad effect 1", "ude");
    let nde2 = add_node(dir, "Bad effect 2", "ude");

    let (json, _) = run_ltp(
        dir,
        &["nbr", "add", "--tree", &tree_id, "--source-node", &inj],
    );
    let nbr_id = json["data"]["nbr_id"].as_str().unwrap().to_string();

    // Connect chain: inj -> nde1 -> nde2
    run_ltp(
        dir,
        &[
            "link", "connect", "--tree", &tree_id, "--nbr", &nbr_id, "--from", &inj, "--to", &nde1,
        ],
    );
    run_ltp(
        dir,
        &[
            "link", "connect", "--tree", &tree_id, "--nbr", &nbr_id, "--from", &nde1, "--to", &nde2,
        ],
    );

    let (json, code) = run_ltp(dir, &["nbr", "inspect", &nbr_id, "--tree", &tree_id]);
    assert_eq!(code, 0);
    assert_eq!(json["data"]["edge_count"].as_u64().unwrap(), 2);
    assert_eq!(json["data"]["nodes_involved"].as_array().unwrap().len(), 3);
    assert_eq!(json["data"]["source_node"].as_str().unwrap(), inj);
}

// --- UAT 10.5: nbr add with trim_injection ---
#[test]
fn uat_10_5_nbr_add_with_trim() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree_id, inj) = setup_frt_with_injection(dir);
    let trim_node = add_node(dir, "Trim injection", "inj");
    attach_node(dir, &tree_id, &trim_node);

    let (json, code) = run_ltp(
        dir,
        &[
            "nbr",
            "add",
            "--tree",
            &tree_id,
            "--source-node",
            &inj,
            "--trim",
            &trim_node,
        ],
    );
    assert_eq!(code, 0);
    assert_eq!(json["data"]["trim_injection"].as_str().unwrap(), trim_node);
}

// --- UAT 10.6: validate with NBR validates DAG independently ---
#[test]
fn uat_10_6_validate_with_nbr() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree_id, inj) = setup_frt_with_injection(dir);
    let nde = add_node(dir, "Negative effect", "ude");

    let (json, _) = run_ltp(
        dir,
        &["nbr", "add", "--tree", &tree_id, "--source-node", &inj],
    );
    let nbr_id = json["data"]["nbr_id"].as_str().unwrap().to_string();

    // Add valid edge in NBR
    run_ltp(
        dir,
        &[
            "link", "connect", "--tree", &tree_id, "--nbr", &nbr_id, "--from", &inj, "--to", &nde,
        ],
    );

    let (json, code) = run_ltp(dir, &["validate"]);
    assert_eq!(code, 0);
    assert!(json["success"].as_bool().unwrap());
}

// --- UAT 10.7: nbr add with non-existent source_node ---
#[test]
fn uat_10_7_nbr_add_source_node_not_found() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);
    let tree_id = create_tree(dir, "frt", "Test FRT");

    let (json, code) = run_ltp(
        dir,
        &[
            "nbr",
            "add",
            "--tree",
            &tree_id,
            "--source-node",
            "NODO-999",
        ],
    );
    assert_eq!(code, 1);
    assert!(!json["success"].as_bool().unwrap());
    assert_eq!(
        json["errors"][0]["code"].as_str().unwrap(),
        "NODE_NOT_FOUND"
    );
}

// --- UAT 10.8: nbr add with source_node not attached to tree ---
#[test]
fn uat_10_8_nbr_add_source_not_in_tree() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);
    let node = add_node(dir, "Detached node", "ude");
    let tree_id = create_tree(dir, "frt", "Test FRT");
    // node exists in pool but NOT attached to tree

    let (json, code) = run_ltp(
        dir,
        &["nbr", "add", "--tree", &tree_id, "--source-node", &node],
    );
    assert_eq!(code, 1);
    assert!(!json["success"].as_bool().unwrap());
    assert_eq!(
        json["errors"][0]["code"].as_str().unwrap(),
        "NODE_NOT_IN_TREE"
    );
}

// --- UAT 10.9: link connect --nbr with non-existent node ---
#[test]
fn uat_10_9_link_connect_nbr_referential_integrity() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree_id, inj) = setup_frt_with_injection(dir);

    let (json, _) = run_ltp(
        dir,
        &["nbr", "add", "--tree", &tree_id, "--source-node", &inj],
    );
    let nbr_id = json["data"]["nbr_id"].as_str().unwrap().to_string();

    let (json, code) = run_ltp(
        dir,
        &[
            "link", "connect", "--tree", &tree_id, "--nbr", &nbr_id, "--from", &inj, "--to",
            "NODO-999",
        ],
    );
    assert_eq!(code, 1);
    assert!(!json["success"].as_bool().unwrap());
    assert_eq!(
        json["errors"][0]["code"].as_str().unwrap(),
        "REFERENTIAL_INTEGRITY_VIOLATION"
    );
}

// --- UAT 10.10: nbr rm removes NBR, nodes stay in pool ---
#[test]
fn uat_10_10_nbr_rm_nodes_stay_in_pool() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree_id, inj) = setup_frt_with_injection(dir);
    let nde = add_node(dir, "Negative effect", "ude");

    let (json, _) = run_ltp(
        dir,
        &["nbr", "add", "--tree", &tree_id, "--source-node", &inj],
    );
    let nbr_id = json["data"]["nbr_id"].as_str().unwrap().to_string();

    // Add edge to NBR
    run_ltp(
        dir,
        &[
            "link", "connect", "--tree", &tree_id, "--nbr", &nbr_id, "--from", &inj, "--to", &nde,
        ],
    );

    // Remove NBR
    let (json, code) = run_ltp(dir, &["nbr", "rm", "--tree", &tree_id, "--nbr", &nbr_id]);
    assert_eq!(code, 0);
    assert!(json["success"].as_bool().unwrap());
    assert_eq!(json["data"]["edges_removed"].as_u64().unwrap(), 1);

    // Verify node still exists in pool
    let (json, code) = run_ltp(dir, &["node", "inspect", &nde]);
    assert_eq!(code, 0);
    assert!(json["success"].as_bool().unwrap());
}

// --- UAT 10.11: nbr rm non-existent ---
#[test]
fn uat_10_11_nbr_rm_not_found() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree_id, _inj) = setup_frt_with_injection(dir);

    let (json, code) = run_ltp(dir, &["nbr", "rm", "--tree", &tree_id, "--nbr", "NBR-999"]);
    assert_eq!(code, 1);
    assert!(!json["success"].as_bool().unwrap());
    assert_eq!(json["errors"][0]["code"].as_str().unwrap(), "NBR_NOT_FOUND");
}

// --- UAT 10.12: recursive NBR via reference ---
#[test]
fn uat_10_12_recursive_nbr_via_reference() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree_id, inj1) = setup_frt_with_injection(dir);
    let inj3 = add_node(dir, "Trim injection for NBR-001", "inj");
    attach_node(dir, &tree_id, &inj3);

    // NBR-001 with trim = INJ-003
    let (json, _) = run_ltp(
        dir,
        &[
            "nbr",
            "add",
            "--tree",
            &tree_id,
            "--source-node",
            &inj1,
            "--trim",
            &inj3,
        ],
    );
    let nbr1_id = json["data"]["nbr_id"].as_str().unwrap().to_string();

    // Add edge in NBR-001
    let nde = add_node(dir, "Bad effect from INJ-001", "ude");
    run_ltp(
        dir,
        &[
            "link", "connect", "--tree", &tree_id, "--nbr", &nbr1_id, "--from", &inj1, "--to", &nde,
        ],
    );

    // NBR-002 with source = INJ-003 (the trim of NBR-001 — recursion by reference)
    let (json, code) = run_ltp(
        dir,
        &["nbr", "add", "--tree", &tree_id, "--source-node", &inj3],
    );
    assert_eq!(code, 0);
    let nbr2_id = json["data"]["nbr_id"].as_str().unwrap().to_string();

    // Add edge in NBR-002
    let nde2 = add_node(dir, "Bad effect from trim", "ude");
    run_ltp(
        dir,
        &[
            "link", "connect", "--tree", &tree_id, "--nbr", &nbr2_id, "--from", &inj3, "--to",
            &nde2,
        ],
    );

    // Validate: both NBRs are valid DAGs independently
    let (json, code) = run_ltp(dir, &["validate"]);
    assert_eq!(code, 0);
    assert!(json["success"].as_bool().unwrap());
}

// --- UAT 10.13: cycle detection within NBR ---
#[test]
fn uat_10_13_cycle_within_nbr_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree_id, inj) = setup_frt_with_injection(dir);
    let nde = add_node(dir, "Effect A", "ude");

    let (json, _) = run_ltp(
        dir,
        &["nbr", "add", "--tree", &tree_id, "--source-node", &inj],
    );
    let nbr_id = json["data"]["nbr_id"].as_str().unwrap().to_string();

    // Create valid edge: inj -> nde
    run_ltp(
        dir,
        &[
            "link", "connect", "--tree", &tree_id, "--nbr", &nbr_id, "--from", &inj, "--to", &nde,
        ],
    );

    // Try to create cycle: nde -> inj
    let (json, code) = run_ltp(
        dir,
        &[
            "link", "connect", "--tree", &tree_id, "--nbr", &nbr_id, "--from", &nde, "--to", &inj,
        ],
    );
    assert_eq!(code, 1);
    assert!(!json["success"].as_bool().unwrap());
    assert_eq!(
        json["errors"][0]["code"].as_str().unwrap(),
        "CIRCULAR_DEPENDENCY_DETECTED"
    );
}

// --- UAT 10.14: nbr inspect non-existent NBR ---
#[test]
fn uat_10_14_nbr_inspect_not_found() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree_id, _) = setup_frt_with_injection(dir);

    let (json, code) = run_ltp(dir, &["nbr", "inspect", "NBR-999", "--tree", &tree_id]);
    assert_eq!(code, 1);
    assert!(!json["success"].as_bool().unwrap());
    assert_eq!(json["errors"][0]["code"].as_str().unwrap(), "NBR_NOT_FOUND");
}

// --- UAT 10.15: nbr add with non-existent trim node ---
#[test]
fn uat_10_15_nbr_add_trim_not_found() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree_id, inj) = setup_frt_with_injection(dir);

    let (json, code) = run_ltp(
        dir,
        &[
            "nbr",
            "add",
            "--tree",
            &tree_id,
            "--source-node",
            &inj,
            "--trim",
            "NODO-999",
        ],
    );
    assert_eq!(code, 1);
    assert!(!json["success"].as_bool().unwrap());
    assert_eq!(
        json["errors"][0]["code"].as_str().unwrap(),
        "NODE_NOT_FOUND"
    );
}

// --- UAT 10.16: multiple NBRs with same source_node ---
#[test]
fn uat_10_16_multiple_nbrs_same_source() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree_id, inj) = setup_frt_with_injection(dir);

    // First NBR
    let (json1, code1) = run_ltp(
        dir,
        &["nbr", "add", "--tree", &tree_id, "--source-node", &inj],
    );
    assert_eq!(code1, 0);

    // Second NBR with same source
    let (json2, code2) = run_ltp(
        dir,
        &["nbr", "add", "--tree", &tree_id, "--source-node", &inj],
    );
    assert_eq!(code2, 0);

    // Both should have different IDs
    let id1 = json1["data"]["nbr_id"].as_str().unwrap();
    let id2 = json2["data"]["nbr_id"].as_str().unwrap();
    assert_ne!(id1, id2);

    // List should show 2
    let (json, _) = run_ltp(dir, &["nbr", "list", "--tree", &tree_id]);
    assert_eq!(json["data"]["nbr_count"].as_u64().unwrap(), 2);
}

// --- UAT 10.17: trace --nbr includes NBR edges ---
#[test]
fn uat_10_17_trace_includes_nbr_edges() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (tree_id, inj) = setup_frt_with_injection(dir);
    let nde = add_node(dir, "Negative effect", "ude");
    attach_node(dir, &tree_id, &nde);

    // Trunk edge: inj -> nde (so trace without --nbr would find it)
    // Actually let's make the NBR-only scenario clearer
    let nbr_effect = add_node(dir, "NBR-only effect", "ude");

    let (json, _) = run_ltp(
        dir,
        &["nbr", "add", "--tree", &tree_id, "--source-node", &inj],
    );
    let nbr_id = json["data"]["nbr_id"].as_str().unwrap().to_string();

    // Edge inside NBR: inj -> nbr_effect
    run_ltp(
        dir,
        &[
            "link",
            "connect",
            "--tree",
            &tree_id,
            "--nbr",
            &nbr_id,
            "--from",
            &inj,
            "--to",
            &nbr_effect,
        ],
    );

    // Trace with --nbr should include the NBR edge
    let (json, code) = run_ltp(
        dir,
        &[
            "trace",
            &inj,
            "--tree",
            &tree_id,
            "--direction",
            "downstream",
            "--nbr",
        ],
    );
    assert_eq!(code, 0);
    assert!(json["success"].as_bool().unwrap());

    // The chain should contain the NBR-only effect node
    let chain = json["data"]["chain"].as_array().unwrap();
    let has_nbr_node = chain
        .iter()
        .any(|entry| entry["node"].as_str().unwrap_or("") == nbr_effect);
    assert!(
        has_nbr_node,
        "trace --nbr should include NBR edges. Chain: {:?}",
        chain
    );
}
