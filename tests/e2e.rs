#![allow(dead_code)]

use std::fs;
use std::process::Command;

use serde_json::Value;

// ─── Helpers ───────────────────────────────────────────────────────────────

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
    let (_, code) = run_ltp(dir, &["init", "--name", "E2ETest"]);
    assert_eq!(code, 0);
}

fn add_node(dir: &std::path::Path, label: &str, node_type: &str) -> String {
    let (json, code) = run_ltp(dir, &["node", "add", label, "--type", node_type]);
    assert_eq!(code, 0, "node add failed: {:?}", json);
    json["data"]["id"].as_str().unwrap().to_string()
}

fn add_node_with_observable(
    dir: &std::path::Path,
    label: &str,
    node_type: &str,
    observable: bool,
) -> String {
    let obs_str = if observable { "true" } else { "false" };
    let (json, code) = run_ltp(
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
    assert_eq!(code, 0, "node add failed: {:?}", json);
    json["data"]["id"].as_str().unwrap().to_string()
}

fn node_edit(dir: &std::path::Path, node_id: &str, args: &[&str]) {
    let mut cmd_args = vec!["node", "edit", node_id];
    cmd_args.extend_from_slice(args);
    let (json, code) = run_ltp(dir, &cmd_args);
    assert_eq!(code, 0, "node edit failed: {:?}", json);
}

fn node_rm(dir: &std::path::Path, node_ids: &str) {
    let (json, code) = run_ltp(dir, &["node", "rm", node_ids]);
    assert_eq!(code, 0, "node rm failed: {:?}", json);
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

fn attach_node_with_role(dir: &std::path::Path, tree_id: &str, node_id: &str, role: &str) {
    let (_, code) = run_ltp(
        dir,
        &[
            "tree", "attach", "--tree", tree_id, "--node", node_id, "--role", role,
        ],
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

fn connect_and(dir: &std::path::Path, tree_id: &str, from: &str, to: &str) -> String {
    let (json, code) = run_ltp(
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
    assert_eq!(code, 0, "link connect AND failed: {:?}", json);
    json["data"]["created_links"][0]
        .as_str()
        .unwrap()
        .to_string()
}

fn connect_xor(dir: &std::path::Path, tree_id: &str, from: &str, to: &str) -> String {
    let (json, code) = run_ltp(
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
            "XOR",
        ],
    );
    assert_eq!(code, 0, "link connect XOR failed: {:?}", json);
    json["data"]["created_links"][0]
        .as_str()
        .unwrap()
        .to_string()
}

fn link_disconnect(dir: &std::path::Path, tree_id: &str, link_id: &str) {
    let (json, code) = run_ltp(
        dir,
        &["link", "disconnect", "--tree", tree_id, "--links", link_id],
    );
    assert_eq!(code, 0, "link disconnect failed: {:?}", json);
}

fn link_add_cause(
    dir: &std::path::Path,
    tree_id: &str,
    link_id: &str,
    node_id: &str,
    promote_to: Option<&str>,
) {
    let mut args = vec![
        "link",
        "add-cause",
        "--tree",
        tree_id,
        "--link",
        link_id,
        "--node",
        node_id,
    ];
    if let Some(op) = promote_to {
        args.push("--promote-to");
        args.push(op);
    }
    let (json, code) = run_ltp(dir, &args);
    assert_eq!(code, 0, "link add-cause failed: {:?}", json);
}

fn link_reoperator(dir: &std::path::Path, tree_id: &str, link_id: &str, operator: &str) {
    let (json, code) = run_ltp(
        dir,
        &[
            "link",
            "reoperator",
            "--tree",
            tree_id,
            "--link",
            link_id,
            "--operator",
            operator,
        ],
    );
    assert_eq!(code, 0, "link reoperator failed: {:?}", json);
}

fn link_inspect(dir: &std::path::Path, tree_id: &str, link_id: &str) -> Value {
    let (json, code) = run_ltp(dir, &["link", "inspect", link_id, "--tree", tree_id]);
    assert_eq!(code, 0, "link inspect failed: {:?}", json);
    json
}

fn assume_add(dir: &std::path::Path, tree_id: &str, link_id: &str, text: &str) -> String {
    let (json, code) = run_ltp(
        dir,
        &[
            "assume", "add", "--tree", tree_id, "--link", link_id, "--text", text,
        ],
    );
    assert_eq!(code, 0, "assume add failed: {:?}", json);
    json["data"]["id"].as_str().unwrap().to_string()
}

fn invalidate(
    dir: &std::path::Path,
    tree_id: &str,
    link_id: &str,
    asm_id: &str,
    injection: Option<&str>,
) -> Value {
    let mut args = vec![
        "invalidate",
        "--tree",
        tree_id,
        "--link",
        link_id,
        "--asm",
        asm_id,
    ];
    if let Some(inj) = injection {
        args.push("--injection");
        args.push(inj);
    }
    let (json, code) = run_ltp(dir, &args);
    assert_eq!(code, 0, "invalidate failed: {:?}", json);
    json
}

fn undo(dir: &std::path::Path) -> Value {
    let (json, code) = run_ltp(dir, &["undo"]);
    assert_eq!(code, 0, "undo failed: {:?}", json);
    json
}

fn redo(dir: &std::path::Path) -> Value {
    let (json, code) = run_ltp(dir, &["redo"]);
    assert_eq!(code, 0, "redo failed: {:?}", json);
    json
}

fn validate(dir: &std::path::Path) -> Value {
    let (json, code) = run_ltp(dir, &["validate"]);
    assert_eq!(code, 0, "validate failed: {:?}", json);
    json
}

fn validate_tree(dir: &std::path::Path, tree_id: &str) -> Value {
    let (json, code) = run_ltp(dir, &["validate", "--tree", tree_id]);
    assert_eq!(code, 0, "validate --tree failed: {:?}", json);
    json
}

fn trace(dir: &std::path::Path, node_id: &str, tree_id: &str, direction: &str) -> Value {
    let (json, code) = run_ltp(
        dir,
        &[
            "trace",
            node_id,
            "--tree",
            tree_id,
            "--direction",
            direction,
        ],
    );
    assert_eq!(code, 0, "trace failed: {:?}", json);
    json
}

fn trace_with_depth(
    dir: &std::path::Path,
    node_id: &str,
    tree_id: &str,
    direction: &str,
    depth: &str,
) -> Value {
    let (json, code) = run_ltp(
        dir,
        &[
            "trace",
            node_id,
            "--tree",
            tree_id,
            "--direction",
            direction,
            "--depth",
            depth,
        ],
    );
    assert_eq!(code, 0, "trace with depth failed: {:?}", json);
    json
}

fn nbr_add(dir: &std::path::Path, tree_id: &str, source_node: &str) -> String {
    let (json, code) = run_ltp(
        dir,
        &[
            "nbr",
            "add",
            "--tree",
            tree_id,
            "--source-node",
            source_node,
        ],
    );
    assert_eq!(code, 0, "nbr add failed: {:?}", json);
    json["data"]["nbr_id"].as_str().unwrap().to_string()
}

fn nbr_add_with_trim(
    dir: &std::path::Path,
    tree_id: &str,
    source_node: &str,
    trim: &str,
) -> String {
    let (json, code) = run_ltp(
        dir,
        &[
            "nbr",
            "add",
            "--tree",
            tree_id,
            "--source-node",
            source_node,
            "--trim",
            trim,
        ],
    );
    assert_eq!(code, 0, "nbr add with trim failed: {:?}", json);
    json["data"]["nbr_id"].as_str().unwrap().to_string()
}

fn nbr_connect(dir: &std::path::Path, tree_id: &str, nbr_id: &str, from: &str, to: &str) {
    let (json, code) = run_ltp(
        dir,
        &[
            "link", "connect", "--tree", tree_id, "--nbr", nbr_id, "--from", from, "--to", to,
        ],
    );
    assert_eq!(code, 0, "nbr connect failed: {:?}", json);
}

fn path_collapse(
    dir: &std::path::Path,
    tree_id: &str,
    from: &str,
    to: &str,
    label: &str,
) -> String {
    let (json, code) = run_ltp(
        dir,
        &[
            "path", "collapse", "--tree", tree_id, "--from", from, "--to", to, "--label", label,
        ],
    );
    assert_eq!(code, 0, "path collapse failed: {:?}", json);
    json["data"]["macro_edge_id"].as_str().unwrap().to_string()
}

fn path_replace(dir: &std::path::Path, tree_id: &str, macro_link: &str, by_node: &str) {
    let (json, code) = run_ltp(
        dir,
        &[
            "path",
            "replace",
            "--tree",
            tree_id,
            "--macro-link",
            macro_link,
            "--by-node",
            by_node,
        ],
    );
    assert_eq!(code, 0, "path replace failed: {:?}", json);
}

fn tree_clone(dir: &std::path::Path, tree_id: &str, name: &str) -> String {
    let (json, code) = run_ltp(dir, &["tree", "clone", tree_id, "--name", name]);
    assert_eq!(code, 0, "tree clone failed: {:?}", json);
    json["data"]["new_id"].as_str().unwrap().to_string()
}

fn tree_diff(dir: &std::path::Path, tree_a: &str, tree_b: &str) -> Value {
    let (json, code) = run_ltp(dir, &["tree", "diff", tree_a, tree_b]);
    assert_eq!(code, 0, "tree diff failed: {:?}", json);
    json
}

fn begin_batch(dir: &std::path::Path, label: &str) {
    let (json, code) = run_ltp(dir, &["history", "begin-batch", "--label", label]);
    assert_eq!(code, 0, "begin-batch failed: {:?}", json);
}

fn end_batch(dir: &std::path::Path) {
    let (json, code) = run_ltp(dir, &["history", "end-batch"]);
    assert_eq!(code, 0, "end-batch failed: {:?}", json);
}

fn history_check(dir: &std::path::Path) -> Value {
    let (json, code) = run_ltp(dir, &["history", "check"]);
    assert_eq!(code, 0, "history check failed: {:?}", json);
    json
}

fn history_invalidate_from(dir: &std::path::Path, seq: &str) {
    let (json, code) = run_ltp(dir, &["history", "invalidate", "--from", seq]);
    assert_eq!(code, 0, "history invalidate failed: {:?}", json);
}

fn status(dir: &std::path::Path) -> Value {
    let (json, code) = run_ltp(dir, &["status"]);
    assert_eq!(code, 0, "status failed: {:?}", json);
    json
}

fn has_warning_code(json: &Value, code: &str) -> bool {
    // Check top-level warnings (non-validate commands)
    if json["warnings"]
        .as_array()
        .map(|w| w.iter().any(|entry| entry["code"].as_str() == Some(code)))
        .unwrap_or(false)
    {
        return true;
    }
    // Check validate-style nested warnings in data.details[].warnings[]
    if let Some(details) = json["data"]["details"].as_array() {
        for detail in details {
            if let Some(warnings) = detail["warnings"].as_array() {
                if warnings.iter().any(|w| w["code"].as_str() == Some(code)) {
                    return true;
                }
            }
        }
    }
    false
}

fn has_error_code(json: &Value, code: &str) -> bool {
    // Check top-level errors
    if json["errors"]
        .as_array()
        .map(|e| e.iter().any(|entry| entry["code"].as_str() == Some(code)))
        .unwrap_or(false)
    {
        return true;
    }
    // Check validate-style nested errors in data.details[].errors[]
    if let Some(details) = json["data"]["details"].as_array() {
        for detail in details {
            if let Some(errors) = detail["errors"].as_array() {
                if errors.iter().any(|e| e["code"].as_str() == Some(code)) {
                    return true;
                }
            }
        }
    }
    false
}

// ─── E2E.1: CRT completo ──────────────────────────────────────────────────

#[test]
fn e2e_01_crt_complete_workflow() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    // Add 5 UDEs + 3 RCs + 2 INTs
    let ude1 = add_node(dir, "Tiempo de entrega supera 15 dias", "UDE");
    let ude2 = add_node(dir, "Clientes migran a competencia", "UDE");
    let ude3 = add_node(dir, "Ventas caen 20%", "UDE");
    let ude4 = add_node(dir, "Costos logisticos altos", "UDE");
    let ude5 = add_node(dir, "Rentabilidad negativa", "UDE");
    let rc1 = add_node(dir, "Falta de capacidad transporte", "RC");
    let rc2 = add_node(dir, "Rutas no optimizadas", "RC");
    let rc3 = add_node(dir, "Sin sistema de tracking", "RC");
    let int1 = add_node(dir, "Entregas se retrasan", "INT");
    let int2 = add_node(dir, "Clientes no tienen visibilidad", "INT");

    // Create CRT and attach all
    let tree = create_tree(dir, "crt", "CRT Logistica");
    for n in [
        &ude1, &ude2, &ude3, &ude4, &ude5, &rc1, &rc2, &rc3, &int1, &int2,
    ] {
        attach_node(dir, &tree, n);
    }

    // Connect causal chain
    connect(dir, &tree, &rc1, &int1);
    connect(dir, &tree, &rc2, &int1);
    connect(dir, &tree, &rc3, &int2);
    connect(dir, &tree, &int1, &ude1);
    connect(dir, &tree, &int2, &ude2);
    connect(dir, &tree, &ude1, &ude3);
    connect(dir, &tree, &ude2, &ude3);
    connect(dir, &tree, &ude3, &ude5);
    connect(dir, &tree, &ude4, &ude5);
    connect(dir, &tree, &rc1, &ude4);

    // Validate
    let v = validate(dir);
    assert!(v["success"].as_bool().unwrap());
    assert_eq!(v["errors"].as_array().unwrap().len(), 0);
    assert!(v["graph_health"]["valid_dag"].as_bool().unwrap());

    // Status
    let s = status(dir);
    assert!(s["success"].as_bool().unwrap());
    assert_eq!(s["graph_health"]["orphan_nodes_count"].as_u64().unwrap(), 0);
}

// ─── E2E.2: Insuficiencia → corrección ────────────────────────────────────

#[test]
fn e2e_02_insufficiency_correction() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let rc1 = add_node(dir, "Causa unica", "RC");
    let ude1 = add_node(dir, "Efecto", "UDE");
    let tree = create_tree(dir, "crt", "TestCRT");
    attach_node(dir, &tree, &rc1);
    attach_node(dir, &tree, &ude1);
    let link = connect(dir, &tree, &rc1, &ude1);

    // Validate should warn CLR#4 (single cause)
    let v = validate_tree(dir, &tree);
    assert!(has_warning_code(&v, "CLR4_INSUFFICIENT_CAUSE"));

    // Fix: add second cause
    let rc2 = add_node(dir, "Segunda causa", "RC");
    attach_node(dir, &tree, &rc2);
    link_add_cause(dir, &tree, &link, &rc2, Some("AND"));

    // Re-validate: warning should be gone
    let v2 = validate_tree(dir, &tree);
    assert!(!has_warning_code(&v2, "CLR4_INSUFFICIENT_CAUSE"));
}

// ─── E2E.3: Invalidación completa ─────────────────────────────────────────

#[test]
fn e2e_03_invalidation_complete_lifecycle() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let rc1 = add_node(dir, "Root Cause", "RC");
    let ude1 = add_node(dir, "Effect", "UDE");
    let tree = create_tree(dir, "crt", "TestCRT");
    attach_node(dir, &tree, &rc1);
    attach_node(dir, &tree, &ude1);
    let link = connect(dir, &tree, &rc1, &ude1);
    let asm = assume_add(dir, &tree, &link, "Capacity stays constant");

    // Invalidate with injection
    let inv = invalidate(dir, &tree, &link, &asm, Some("Increase capacity"));
    assert!(inv["success"].as_bool().unwrap());
    assert!(inv["data"]["changed"].as_bool().unwrap());
    let inj_id = inv["data"]["injection_id"].as_str().unwrap();

    // Verify: INJ exists, link is broken
    assert!(dir.join(format!("nodes/{}.json", inj_id)).exists());
    let inspect = link_inspect(dir, &tree, &link);
    assert_eq!(inspect["data"]["status"].as_str().unwrap(), "broken");

    // Undo
    undo(dir);

    // Verify: INJ gone, link active, ASM valid
    assert!(!dir.join(format!("nodes/{}.json", inj_id)).exists());
    let inspect2 = link_inspect(dir, &tree, &link);
    assert_eq!(inspect2["data"]["status"].as_str().unwrap(), "active");
    let asms = inspect2["data"]["assumptions"].as_array().unwrap();
    assert_eq!(asms[0]["status"].as_str().unwrap(), "valid");
}

// ─── E2E.4: EC validation ─────────────────────────────────────────────────

#[test]
fn e2e_04_ec_validation() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    // Create nodes for EC
    let obj = add_node(dir, "Maximize profit", "OBJ");
    let req1 = add_node(dir, "Cut costs", "REQ");
    let req2 = add_node(dir, "Increase revenue", "REQ");
    let pre1 = add_node(dir, "Reduce staff", "PRE");
    let pre2 = add_node(dir, "Invest in marketing", "PRE");

    // Create EC tree and attach with roles
    let tree = create_tree(dir, "ec", "Profit EC");
    attach_node_with_role(dir, &tree, &obj, "objective");
    attach_node_with_role(dir, &tree, &req1, "requirement");
    attach_node_with_role(dir, &tree, &req2, "requirement");
    attach_node_with_role(dir, &tree, &pre1, "prerequisite");
    attach_node_with_role(dir, &tree, &pre2, "prerequisite");

    // Connect: requirements → objective, prerequisites → requirements
    connect(dir, &tree, &req1, &obj);
    connect(dir, &tree, &req2, &obj);
    connect(dir, &tree, &pre1, &req1);
    connect(dir, &tree, &pre2, &req2);

    // XOR between prerequisites
    connect_xor(dir, &tree, &pre1, &pre2);

    // Validate: should be clean
    let v = validate_tree(dir, &tree);
    assert!(v["success"].as_bool().unwrap());
    assert_eq!(v["errors"].as_array().unwrap().len(), 0);
}

// ─── E2E.5: CRT→EC→FRT cycle ──────────────────────────────────────────────

#[test]
fn e2e_05_crt_ec_frt_cycle() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    // Build CRT
    let rc1 = add_node(dir, "Root cause 1", "RC");
    let int1 = add_node(dir, "Intermediate effect", "INT");
    let ude1 = add_node(dir, "Core problem", "UDE");
    let crt = create_tree(dir, "crt", "Main CRT");
    for n in [&rc1, &int1, &ude1] {
        attach_node(dir, &crt, n);
    }
    let l1 = connect(dir, &crt, &rc1, &int1);
    connect(dir, &crt, &int1, &ude1);

    // Build EC using the same core problem node (shared)
    let obj = add_node(dir, "Resolve core problem", "OBJ");
    let req1 = add_node(dir, "Requirement A", "REQ");
    let req2 = add_node(dir, "Requirement B", "REQ");
    let pre1 = add_node(dir, "Prereq A", "PRE");
    let pre2 = add_node(dir, "Prereq B", "PRE");
    let ec = create_tree(dir, "ec", "Resolution EC");
    attach_node_with_role(dir, &ec, &obj, "objective");
    attach_node_with_role(dir, &ec, &req1, "requirement");
    attach_node_with_role(dir, &ec, &req2, "requirement");
    attach_node_with_role(dir, &ec, &pre1, "prerequisite");
    attach_node_with_role(dir, &ec, &pre2, "prerequisite");
    // Attach core problem to EC as context
    attach_node(dir, &ec, &ude1);
    connect(dir, &ec, &req1, &obj);
    connect(dir, &ec, &req2, &obj);
    connect(dir, &ec, &pre1, &req1);
    connect(dir, &ec, &pre2, &req2);
    connect_xor(dir, &ec, &pre1, &pre2);

    // Invalidate an assumption in CRT → leads to injection
    let asm = assume_add(dir, &crt, &l1, "No capacity increase possible");
    let inv = invalidate(dir, &crt, &l1, &asm, Some("Increase fleet"));
    let inj_id = inv["data"]["injection_id"].as_str().unwrap().to_string();

    // Build FRT with injection
    let frt = create_tree(dir, "frt", "Future RT");
    attach_node(dir, &frt, &inj_id);
    let de1 = add_node(dir, "Desired effect 1", "DE");
    attach_node(dir, &frt, &de1);
    connect(dir, &frt, &inj_id, &de1);

    // Add NBR to FRT
    let nde1 = add_node(dir, "Negative effect", "UDE");
    let trim = add_node(dir, "Trim injection", "INJ");
    let nbr = nbr_add_with_trim(dir, &frt, &inj_id, &trim);
    nbr_connect(dir, &frt, &nbr, &inj_id, &nde1);

    // Validate all trees
    let v_crt = validate_tree(dir, &crt);
    assert_eq!(v_crt["errors"].as_array().unwrap().len(), 0);
    let v_ec = validate_tree(dir, &ec);
    assert_eq!(v_ec["errors"].as_array().unwrap().len(), 0);
    let v_frt = validate_tree(dir, &frt);
    assert_eq!(v_frt["errors"].as_array().unwrap().len(), 0);

    // Verify shared node is visible in both CRT and EC
    let (inspect_json, _) = run_ltp(dir, &["node", "inspect", &ude1]);
    let trees_arr = inspect_json["data"]["trees"].as_array().unwrap();
    assert!(trees_arr.len() >= 2);
}

// ─── E2E.6: Batch undo ────────────────────────────────────────────────────

#[test]
fn e2e_06_batch_undo() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let tree = create_tree(dir, "crt", "BatchTest");

    begin_batch(dir, "Build CRT");

    // 10 operations inside batch
    let mut nodes = Vec::new();
    for i in 0..10 {
        let n = add_node(dir, &format!("Node {}", i), "UDE");
        attach_node(dir, &tree, &n);
        nodes.push(n);
    }

    end_batch(dir);

    // Verify: 10 nodes exist
    let (list_json, _) = run_ltp(dir, &["node", "list"]);
    let listed = list_json["data"]["nodes"].as_array().unwrap();
    assert_eq!(listed.len(), 10);

    // Single undo reverts entire batch
    undo(dir);

    // Verify: 0 nodes
    let (list_json2, _) = run_ltp(dir, &["node", "list"]);
    let listed2 = list_json2["data"]["nodes"].as_array().unwrap();
    assert_eq!(listed2.len(), 0);

    // Tree should still exist (created before batch)
    let (tree_list, _) = run_ltp(dir, &["tree", "list"]);
    assert!(!tree_list["data"]["trees"].as_array().unwrap().is_empty());
}

// ─── E2E.7: Clone + diff ──────────────────────────────────────────────────

#[test]
fn e2e_07_clone_diff() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let a = add_node(dir, "Node A", "RC");
    let b = add_node(dir, "Node B", "INT");
    let c = add_node(dir, "Node C", "UDE");
    let tree = create_tree(dir, "crt", "Original");
    for n in [&a, &b, &c] {
        attach_node(dir, &tree, n);
    }
    connect(dir, &tree, &a, &b);
    connect(dir, &tree, &b, &c);

    // Clone
    let clone = tree_clone(dir, &tree, "What-If");

    // Find edges in the clone (clone creates new edge IDs)
    let (find_json, _) = run_ltp(
        dir,
        &["link", "find", "--tree", &clone, "--from", &a, "--to", &b],
    );
    let clone_l1 = find_json["data"]["links"][0]["id"].as_str().unwrap();

    // Modify clone: add new edge, remove one
    let d = add_node(dir, "Node D", "RC");
    attach_node(dir, &clone, &d);
    connect(dir, &clone, &d, &c);
    link_disconnect(dir, &clone, clone_l1);

    // Diff
    let diff = tree_diff(dir, &tree, &clone);
    assert!(diff["success"].as_bool().unwrap());
    let data = &diff["data"];

    // Should report edges added and nodes added
    let edges_added = data["edges_added"].as_array().unwrap();
    let nodes_added = data["nodes_added"].as_array().unwrap();
    assert!(!edges_added.is_empty(), "Should detect new edge in clone");
    assert!(!nodes_added.is_empty(), "Should detect new node in clone");
}

// ─── E2E.8: Trace depth ───────────────────────────────────────────────────

#[test]
fn e2e_08_trace_depth() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    // Build linear chain of 8 levels: n0 → n1 → ... → n7
    let tree = create_tree(dir, "crt", "DeepChain");
    let mut nodes = Vec::new();
    for i in 0..8 {
        let n = add_node(dir, &format!("Level {}", i), "INT");
        attach_node(dir, &tree, &n);
        nodes.push(n);
    }
    for i in 0..7 {
        connect(dir, &tree, &nodes[i], &nodes[i + 1]);
    }

    // Trace upstream from last node with depth 3 (returns depth+1 entries including start)
    let t3 = trace_with_depth(dir, &nodes[7], &tree, "upstream", "3");
    let chain3 = t3["data"]["chain"].as_array().unwrap();
    assert!(
        chain3.len() <= 4,
        "depth 3 should return at most 4 entries, got {}",
        chain3.len()
    );
    assert!(
        chain3.len() >= 3,
        "depth 3 should return at least 3 entries, got {}",
        chain3.len()
    );

    // Trace without depth limit — should return full chain (7 hops from n7 to n0)
    let t_full = trace(dir, &nodes[7], &tree, "upstream");
    let chain_full = t_full["data"]["chain"].as_array().unwrap();
    assert!(
        chain_full.len() >= 7,
        "full trace should return at least 7 entries, got {}",
        chain_full.len()
    );
    // Depth-limited trace should be strictly shorter than full
    assert!(chain3.len() < chain_full.len());
}

// ─── E2E.9: Nodo compartido multi-tree ────────────────────────────────────

#[test]
fn e2e_09_shared_node_multi_tree() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let shared = add_node(dir, "Original label", "UDE");
    let n2 = add_node(dir, "Another node", "RC");

    let tree_a = create_tree(dir, "crt", "TreeA");
    let tree_b = create_tree(dir, "crt", "TreeB");

    attach_node(dir, &tree_a, &shared);
    attach_node(dir, &tree_a, &n2);
    attach_node(dir, &tree_b, &shared);
    attach_node(dir, &tree_b, &n2);
    connect(dir, &tree_a, &n2, &shared);
    connect(dir, &tree_b, &n2, &shared);

    // Edit shared node label
    node_edit(dir, &shared, &["--label", "Updated label"]);

    // Verify both trees see the updated label via node inspect
    let node_file = fs::read_to_string(dir.join(format!("nodes/{}.json", shared))).unwrap();
    let node_json: Value = serde_json::from_str(&node_file).unwrap();
    assert_eq!(node_json["label"].as_str().unwrap(), "Updated label");

    // Node inspect shows participation in both trees
    let (inspect, _) = run_ltp(dir, &["node", "inspect", &shared]);
    let trees_arr = inspect["data"]["trees"].as_array().unwrap();
    assert_eq!(trees_arr.len(), 2);
}

// ─── E2E.10: Counters recovery ────────────────────────────────────────────

#[test]
fn e2e_10_counters_recovery() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    // Add 5 UDE nodes
    for i in 1..=5 {
        add_node(dir, &format!("UDE number {}", i), "UDE");
    }

    // Delete counters.json
    let counters_path = dir.join(".ltp/counters.json");
    assert!(counters_path.exists());
    fs::remove_file(&counters_path).unwrap();

    // Add another UDE — should auto-rebuild counters
    let (json, code) = run_ltp(dir, &["node", "add", "UDE number 6", "--type", "UDE"]);
    assert_eq!(code, 0);
    assert!(json["success"].as_bool().unwrap());

    // Should be UDE-006 (rebuilt from scanning nodes/)
    let new_id = json["data"]["id"].as_str().unwrap();
    assert_eq!(new_id, "UDE-006");

    // Counters file should be recreated
    assert!(counters_path.exists());
}

// ─── E2E.11: Invalidate + trace lifecycle ─────────────────────────────────

#[test]
fn e2e_11_invalidate_trace_lifecycle() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let rc = add_node(dir, "Root cause", "RC");
    let int = add_node(dir, "Intermediate", "INT");
    let ude = add_node(dir, "Effect", "UDE");
    let tree = create_tree(dir, "crt", "TraceLifecycle");
    for n in [&rc, &int, &ude] {
        attach_node(dir, &tree, n);
    }
    let l1 = connect(dir, &tree, &rc, &int);
    connect(dir, &tree, &int, &ude);

    // Add assumption and invalidate
    let asm = assume_add(dir, &tree, &l1, "Assumption A");
    invalidate(dir, &tree, &l1, &asm, None);

    // Trace upstream from UDE → should show broken link
    let t = trace(dir, &ude, &tree, "upstream");
    let chain = t["data"]["chain"].as_array().unwrap();
    let broken_entry = chain.iter().find(|e| {
        e["link_to_next"]
            .as_object()
            .map(|l| l["status"].as_str() == Some("broken"))
            .unwrap_or(false)
    });
    assert!(broken_entry.is_some(), "Should find a broken link in chain");
    assert!(!t["data"]["chain_health"]["fully_connected"]
        .as_bool()
        .unwrap());

    // Undo invalidation
    undo(dir);

    // Trace again → link should be active
    let t2 = trace(dir, &ude, &tree, "upstream");
    assert!(t2["data"]["chain_health"]["fully_connected"]
        .as_bool()
        .unwrap());
}

// ─── E2E.12: Path collapse + validate ─────────────────────────────────────

#[test]
fn e2e_12_path_collapse_validate() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let a = add_node(dir, "Node A", "RC");
    let b = add_node(dir, "Node B", "INT");
    let c = add_node(dir, "Node C", "INT");
    let d = add_node(dir, "Node D", "UDE");
    let tree = create_tree(dir, "crt", "CollapseTest");
    for n in [&a, &b, &c, &d] {
        attach_node(dir, &tree, n);
    }
    connect(dir, &tree, &a, &b);
    connect(dir, &tree, &b, &c);
    connect(dir, &tree, &c, &d);

    // Collapse A→D
    path_collapse(dir, &tree, &a, &d, "Full chain");

    // Validate should NOT report interior nodes as orphans
    let v = validate_tree(dir, &tree);
    assert!(v["success"].as_bool().unwrap());
    assert_eq!(v["errors"].as_array().unwrap().len(), 0);
}

// ─── E2E.13: NBR + invalidate + undo ──────────────────────────────────────

#[test]
fn e2e_13_nbr_invalidate_undo() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    // FRT trunk
    let inj1 = add_node(dir, "Injection 1", "INJ");
    let de1 = add_node(dir, "Desired effect", "DE");
    let frt = create_tree(dir, "frt", "FRT Test");
    attach_node(dir, &frt, &inj1);
    attach_node(dir, &frt, &de1);
    let trunk_link = connect(dir, &frt, &inj1, &de1);
    let asm = assume_add(dir, &frt, &trunk_link, "No side effects");

    // NBR
    let nde = add_node(dir, "Negative effect", "UDE");
    let nbr = nbr_add(dir, &frt, &inj1);
    nbr_connect(dir, &frt, &nbr, &inj1, &nde);

    // Invalidate trunk assumption
    invalidate(dir, &frt, &trunk_link, &asm, None);

    // Verify trunk link is broken
    let inspect = link_inspect(dir, &frt, &trunk_link);
    assert_eq!(inspect["data"]["status"].as_str().unwrap(), "broken");

    // Undo
    undo(dir);

    // Verify trunk restored
    let inspect2 = link_inspect(dir, &frt, &trunk_link);
    assert_eq!(inspect2["data"]["status"].as_str().unwrap(), "active");

    // Verify NBR still intact
    let (nbr_list, _) = run_ltp(dir, &["nbr", "list", "--tree", &frt]);
    let nbrs = nbr_list["data"]["branches"].as_array().unwrap();
    assert_eq!(nbrs.len(), 1);
}

// ─── E2E.14: Agent simulation workflow ─────────────────────────────────────

#[test]
fn e2e_14_agent_simulation_workflow() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let rc = add_node(dir, "Root cause", "RC");
    let ude = add_node(dir, "Effect", "UDE");
    let tree = create_tree(dir, "crt", "AgentSim");
    attach_node(dir, &tree, &rc);
    attach_node(dir, &tree, &ude);
    let link = connect(dir, &tree, &rc, &ude);

    // Agent pattern: status → trace → inspect → mutate → validate → undo → trace → redo

    // 1. Status (navigation — no undo entry)
    let s = status(dir);
    assert!(s["success"].as_bool().unwrap());

    // 2. Trace (navigation)
    let t = trace(dir, &ude, &tree, "upstream");
    assert!(t["success"].as_bool().unwrap());

    // 3. Link inspect (navigation)
    let insp = link_inspect(dir, &tree, &link);
    assert!(insp["success"].as_bool().unwrap());

    // 4. Mutate: add-cause (mutation → generates undo entry)
    let rc2 = add_node(dir, "Second cause", "RC");
    attach_node(dir, &tree, &rc2);
    link_add_cause(dir, &tree, &link, &rc2, Some("AND"));

    // 5. Validate (navigation)
    let v = validate_tree(dir, &tree);
    assert!(v["success"].as_bool().unwrap());
    assert_eq!(v["errors"].as_array().unwrap().len(), 0);

    // 6. Undo (reverts add-cause)
    undo(dir);

    // 7. Trace again (should work fine with single cause restored)
    let t2 = trace(dir, &ude, &tree, "upstream");
    assert!(t2["success"].as_bool().unwrap());
    let chain = t2["data"]["chain"].as_array().unwrap();
    assert!(!chain.is_empty());

    // 8. Redo (re-applies add-cause)
    redo(dir);

    // 9. Final validate
    let v2 = validate_tree(dir, &tree);
    assert_eq!(v2["errors"].as_array().unwrap().len(), 0);
}

// ─── E2E.15: Undo cross-tree cascade ──────────────────────────────────────

#[test]
fn e2e_15_undo_cross_tree_cascade() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let shared = add_node(dir, "Shared node", "UDE");
    let n_a = add_node(dir, "Node in A", "RC");
    let n_b = add_node(dir, "Node in B", "RC");

    let tree_a = create_tree(dir, "crt", "TreeA");
    let tree_b = create_tree(dir, "crt", "TreeB");

    attach_node(dir, &tree_a, &shared);
    attach_node(dir, &tree_a, &n_a);
    attach_node(dir, &tree_b, &shared);
    attach_node(dir, &tree_b, &n_b);

    connect(dir, &tree_a, &n_a, &shared);
    connect(dir, &tree_b, &n_b, &shared);

    // Remove shared node (cascades edges in both trees)
    node_rm(dir, &shared);

    // Verify node gone
    assert!(!dir.join(format!("nodes/{}.json", shared)).exists());

    // Undo should restore node + edges in BOTH trees
    undo(dir);

    // Node file restored
    assert!(dir.join(format!("nodes/{}.json", shared)).exists());

    // Edges restored in both trees
    let (inspect, _) = run_ltp(dir, &["node", "inspect", &shared]);
    let trees_arr = inspect["data"]["trees"].as_array().unwrap();
    assert_eq!(trees_arr.len(), 2, "Should be in both trees after undo");
}

// ─── E2E.16: EC incremental construction ──────────────────────────────────

#[test]
fn e2e_16_ec_incremental_construction() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let obj = add_node(dir, "Maximize throughput", "OBJ");
    let ec = create_tree(dir, "ec", "Incremental EC");

    // Step 1: Only objective → validate should error (missing requirements)
    attach_node_with_role(dir, &ec, &obj, "objective");
    let (v1, _) = run_ltp(dir, &["validate", "--tree", &ec]);
    assert!(has_error_code(&v1, "EC_VALIDATION"));

    // Step 2: Add requirements → validate should error (missing prerequisites)
    let req1 = add_node(dir, "Reduce batch size", "REQ");
    let req2 = add_node(dir, "Increase speed", "REQ");
    attach_node_with_role(dir, &ec, &req1, "requirement");
    attach_node_with_role(dir, &ec, &req2, "requirement");
    connect(dir, &ec, &req1, &obj);
    connect(dir, &ec, &req2, &obj);
    let (v2, _) = run_ltp(dir, &["validate", "--tree", &ec]);
    assert!(has_error_code(&v2, "EC_VALIDATION"));

    // Step 3: Add prerequisites → validate should now pass (structure complete)
    let pre1 = add_node(dir, "Prereq A", "PRE");
    let pre2 = add_node(dir, "Prereq B", "PRE");
    attach_node_with_role(dir, &ec, &pre1, "prerequisite");
    attach_node_with_role(dir, &ec, &pre2, "prerequisite");
    connect(dir, &ec, &pre1, &req1);
    connect(dir, &ec, &pre2, &req2);
    let v3 = validate_tree(dir, &ec);
    assert!(!has_error_code(&v3, "EC_VALIDATION"));

    // Step 4: Add XOR for conflict resolution → still validates clean
    connect_xor(dir, &ec, &pre1, &pre2);
    let v4 = validate_tree(dir, &ec);
    assert!(!has_error_code(&v4, "EC_VALIDATION"));
}

// ─── E2E.17: Path replace + undo roundtrip ────────────────────────────────

#[test]
fn e2e_17_path_replace_undo_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let a = add_node(dir, "Node A", "RC");
    let b = add_node(dir, "Node B", "INT");
    let c = add_node(dir, "Node C", "INT");
    let d = add_node(dir, "Node D", "UDE");
    let tree = create_tree(dir, "crt", "ReplaceTest");
    for n in [&a, &b, &c, &d] {
        attach_node(dir, &tree, n);
    }
    connect(dir, &tree, &a, &b);
    connect(dir, &tree, &b, &c);
    connect(dir, &tree, &c, &d);

    // Collapse A→D
    let macro_id = path_collapse(dir, &tree, &a, &d, "Full chain");

    // Replace with injection
    let inj = add_node(dir, "Injection X", "INJ");
    attach_node(dir, &tree, &inj);
    path_replace(dir, &tree, &macro_id, &inj);

    // Verify links are superseded (read tree file)
    let tree_file = fs::read_to_string(dir.join(format!("trees/{}.json", tree))).unwrap();
    let tree_json: Value = serde_json::from_str(&tree_file).unwrap();
    let edges = tree_json["edges"].as_array().unwrap();
    let superseded_count = edges
        .iter()
        .filter(|e| e["status"].as_str() == Some("superseded"))
        .count();
    assert!(superseded_count > 0, "Some edges should be superseded");

    // Undo replace
    undo(dir);

    // Verify links restored to active
    let tree_file2 = fs::read_to_string(dir.join(format!("trees/{}.json", tree))).unwrap();
    let tree_json2: Value = serde_json::from_str(&tree_file2).unwrap();
    let edges2 = tree_json2["edges"].as_array().unwrap();
    let active_count = edges2
        .iter()
        .filter(|e| e["status"].as_str() == Some("active"))
        .count();
    assert_eq!(
        active_count,
        edges2.len(),
        "All edges should be active after undo"
    );
}

// ─── E2E.18: History divergence recovery ───────────────────────────────────

#[test]
fn e2e_18_history_divergence_recovery() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    // Add nodes to generate history entries
    let _n1 = add_node(dir, "Node 1", "UDE");
    let n2 = add_node(dir, "Node 2", "RC");
    let _n3 = add_node(dir, "Node 3", "INT");

    // Manually edit n2's JSON (simulates external modification)
    let n2_path = dir.join(format!("nodes/{}.json", n2));
    let content = fs::read_to_string(&n2_path).unwrap();
    let mut node_json: Value = serde_json::from_str(&content).unwrap();
    node_json["label"] = Value::String("Externally modified".to_string());
    fs::write(&n2_path, serde_json::to_string_pretty(&node_json).unwrap()).unwrap();

    // history check should detect divergence (uses valid: bool field)
    let check = history_check(dir);
    assert!(check["success"].as_bool().unwrap());
    let entries = check["data"]["entries"].as_array().unwrap();
    let has_diverged = entries.iter().any(|e| e["valid"].as_bool() == Some(false));
    assert!(has_diverged, "Should detect diverged entry");

    // Find the first diverged seq
    let diverged_seq = entries
        .iter()
        .find(|e| e["valid"].as_bool() == Some(false))
        .unwrap()["seq"]
        .as_u64()
        .unwrap();

    // Invalidate from diverged point
    history_invalidate_from(dir, &diverged_seq.to_string());

    // history check again: remaining entries should be valid
    let check2 = history_check(dir);
    let entries2 = check2["data"]["entries"].as_array().unwrap();
    let all_valid = entries2.iter().all(|e| e["valid"].as_bool() == Some(true));
    assert!(
        all_valid || entries2.is_empty(),
        "All remaining entries should be valid"
    );
}

// ─── E2E.19: Multi-warning iterative fix ───────────────────────────────────

#[test]
fn e2e_19_multi_warning_iterative_fix() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let tree = create_tree(dir, "crt", "WarningFix");

    // Setup: create a graph with 3 distinct warnings (separate target nodes)

    // CLR#4: single cause (insufficient) — rc1 → ude1 (ude1 only has 1 incoming)
    let rc1 = add_node(dir, "Single cause", "RC");
    let ude1 = add_node(dir, "Effect with single cause", "UDE");
    attach_node(dir, &tree, &rc1);
    attach_node(dir, &tree, &ude1);
    let l1 = connect(dir, &tree, &rc1, &ude1);

    // CLR#6: type inversion (UDE in from position → RC)
    // Give rc_to a second incoming edge to avoid spurious CLR4
    let ude_from = add_node(dir, "UDE as cause", "UDE");
    let rc_to = add_node(dir, "RC as effect", "RC");
    let rc_extra1 = add_node(dir, "Extra cause for rc_to", "RC");
    attach_node(dir, &tree, &ude_from);
    attach_node(dir, &tree, &rc_to);
    attach_node(dir, &tree, &rc_extra1);
    connect(dir, &tree, &ude_from, &rc_to);
    connect(dir, &tree, &rc_extra1, &rc_to);

    // CLR#7: intangible node with <2 outgoing (targets separate ude2)
    // Give ude2 a second incoming edge to avoid spurious CLR4
    let intangible = add_node_with_observable(dir, "Intangible cause", "RC", false);
    let ude2 = add_node(dir, "Effect from intangible", "UDE");
    let rc_extra2 = add_node(dir, "Extra cause for ude2", "RC");
    attach_node(dir, &tree, &intangible);
    attach_node(dir, &tree, &ude2);
    attach_node(dir, &tree, &rc_extra2);
    connect(dir, &tree, &intangible, &ude2);
    connect(dir, &tree, &rc_extra2, &ude2);

    // Validate: should have all 3 warnings
    let v1 = validate_tree(dir, &tree);
    assert!(has_warning_code(&v1, "CLR4_INSUFFICIENT_CAUSE"));
    assert!(has_warning_code(&v1, "CLR6_TYPE_INVERSION"));
    assert!(has_warning_code(&v1, "CLR7_INTANGIBLE_NO_PREDICTED"));

    // Fix 1: CLR#4 — add second cause to make it sufficient
    let rc2 = add_node(dir, "Second cause", "RC");
    attach_node(dir, &tree, &rc2);
    link_add_cause(dir, &tree, &l1, &rc2, Some("AND"));

    let v2 = validate_tree(dir, &tree);
    assert!(!has_warning_code(&v2, "CLR4_INSUFFICIENT_CAUSE"));
    assert!(has_warning_code(&v2, "CLR6_TYPE_INVERSION"));
    assert!(has_warning_code(&v2, "CLR7_INTANGIBLE_NO_PREDICTED"));

    // Fix 2: CLR#6 — remove the inverted edge (disconnect) and reconnect properly
    // We'll remove the inverted relationship by removing both nodes from tree
    // and adding them back in correct orientation
    let (find_json, _) = run_ltp(
        dir,
        &[
            "link", "find", "--tree", &tree, "--from", &ude_from, "--to", &rc_to,
        ],
    );
    let inverted_link = find_json["data"]["links"][0]["id"].as_str().unwrap();
    link_disconnect(dir, &tree, inverted_link);
    // Connect in correct direction
    connect(dir, &tree, &rc_to, &ude_from);

    let v3 = validate_tree(dir, &tree);
    assert!(!has_warning_code(&v3, "CLR6_TYPE_INVERSION"));
    assert!(has_warning_code(&v3, "CLR7_INTANGIBLE_NO_PREDICTED"));

    // Fix 3: CLR#7 — add second outgoing edge from intangible node
    let predicted = add_node(dir, "Predicted effect", "UDE");
    attach_node(dir, &tree, &predicted);
    connect(dir, &tree, &intangible, &predicted);

    let v4 = validate_tree(dir, &tree);
    assert!(!has_warning_code(&v4, "CLR7_INTANGIBLE_NO_PREDICTED"));

    // CLR6 and CLR7 should be fully resolved
    assert!(!has_warning_code(&v4, "CLR6_TYPE_INVERSION"));
    // CLR4 may still fire for nodes introduced during fixes (expected —
    // each fix can introduce new insufficient-cause patterns). The key assertion
    // is that our ORIGINAL ude1 no longer triggers it (verified in v2 above).
}
