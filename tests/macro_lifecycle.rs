//! Adversarial end-to-end suite for the long-arrow lifecycle (Slice 2, phases M2–M6).
//!
//! Exercises `macro add` / `macro expand` / `macro promote` through the CLI binary against
//! real temporary workspaces. Per the project's `feedback_deep-uats` rule this is **not**
//! happy-path only: it covers boundaries (B), corrupt/referential inputs (C), illegal
//! transitions and cross-command interactions (I) — including the DAG cycle blocks I7/I9 —
//! and ordering/idempotency under undo/redo/batch (O).
//!
//! Coverage map (see `PLAN_long-arrow-slice2.md` §UATs): H1–H5, B1–B6, C1–C6, I1–I9, O1–O5.

use std::process::Command;

use serde_json::Value;

// ============================================================================
// Harness
// ============================================================================

fn ltp_bin() -> String {
    env!("CARGO_BIN_EXE_ltp").to_string()
}

/// Run the CLI in `dir` and return `(parsed_json_stdout, exit_code)`.
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
            "Failed to parse JSON.\nargs: {args:?}\nstdout: {stdout}\nstderr: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    });
    (json, code)
}

fn setup(dir: &std::path::Path) {
    let (_, code) = run_ltp(dir, &["init", "--name", "MacroLifecycle"]);
    assert_eq!(code, 0);
}

fn add_node(dir: &std::path::Path, label: &str, node_type: &str) -> String {
    let (json, code) = run_ltp(dir, &["node", "add", label, "--type", node_type]);
    assert_eq!(code, 0, "node add failed: {json:?}");
    json["data"]["id"].as_str().unwrap().to_string()
}

fn create_tree(dir: &std::path::Path, tree_type: &str, name: &str) -> String {
    let (json, code) = run_ltp(dir, &["tree", "new", tree_type, name]);
    assert_eq!(code, 0, "tree new failed: {json:?}");
    json["data"]["id"].as_str().unwrap().to_string()
}

fn attach(dir: &std::path::Path, tree: &str, node: &str) {
    let (_, code) = run_ltp(dir, &["tree", "attach", "--tree", tree, "--node", node]);
    assert_eq!(code, 0);
}

fn connect(dir: &std::path::Path, tree: &str, from: &str, to: &str) -> String {
    let (json, code) = run_ltp(
        dir,
        &[
            "link", "connect", "--tree", tree, "--from", from, "--to", to,
        ],
    );
    assert_eq!(code, 0, "link connect failed: {json:?}");
    json["data"]["created_links"][0]
        .as_str()
        .unwrap()
        .to_string()
}

fn macro_add(dir: &std::path::Path, tree: &str, from: &str, to: &str, label: &str) -> (Value, i32) {
    run_ltp(
        dir,
        &[
            "macro", "add", "--tree", tree, "--from", from, "--to", to, "--label", label,
        ],
    )
}

fn macro_expand(dir: &std::path::Path, tree: &str, macro_link: &str, steps: &str) -> (Value, i32) {
    run_ltp(
        dir,
        &[
            "macro",
            "expand",
            "--tree",
            tree,
            "--macro-link",
            macro_link,
            "--steps",
            steps,
        ],
    )
}

fn macro_promote(dir: &std::path::Path, tree: &str, macro_link: &str) -> (Value, i32) {
    run_ltp(
        dir,
        &[
            "macro",
            "promote",
            "--tree",
            tree,
            "--macro-link",
            macro_link,
        ],
    )
}

fn macro_assume_add(
    dir: &std::path::Path,
    tree: &str,
    macro_link: &str,
    text: &str,
    projection: Option<&str>,
) -> (Value, i32) {
    let mut args = vec![
        "macro-assume",
        "add",
        "--tree",
        tree,
        "--macro-link",
        macro_link,
        "--text",
        text,
    ];
    if let Some(p) = projection {
        args.push("--projection");
        args.push(p);
    }
    run_ltp(dir, &args)
}

fn undo(dir: &std::path::Path) {
    let (json, code) = run_ltp(dir, &["undo"]);
    assert_eq!(code, 0, "undo failed: {json:?}");
    assert!(json["success"].as_bool().unwrap(), "undo not ok: {json:?}");
}

fn redo(dir: &std::path::Path) {
    let (json, code) = run_ltp(dir, &["redo"]);
    assert_eq!(code, 0, "redo failed: {json:?}");
    assert!(json["success"].as_bool().unwrap(), "redo not ok: {json:?}");
}

/// Read a specific tree file from disk as JSON.
fn read_tree(dir: &std::path::Path, tree: &str) -> Value {
    let path = dir.join("trees").join(format!("{tree}.json"));
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read tree file {}: {e}", path.display()));
    serde_json::from_str(&content).unwrap()
}

/// Read a sequential counter (e.g. `"MACRO"`, `"INT"`, `"ASM"`) from `.ltp/counters.json`.
fn counter(dir: &std::path::Path, key: &str) -> i64 {
    let path = dir.join(".ltp").join("counters.json");
    let content = std::fs::read_to_string(&path).expect("counters.json");
    let json: Value = serde_json::from_str(&content).unwrap();
    json[key].as_i64().unwrap_or(0)
}

fn node_refs(tree: &Value) -> Vec<String> {
    tree["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["ref"].as_str().unwrap().to_string())
        .collect()
}

fn edge_ids(tree: &Value) -> Vec<String> {
    tree["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["id"].as_str().unwrap().to_string())
        .collect()
}

fn str_vec(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap().to_string())
        .collect()
}

/// All warning codes recorded by `validate` for `tree` (validate is non-blocking → exit 0).
fn all_warning_codes(dir: &std::path::Path, tree: &str) -> Vec<String> {
    let (json, code) = run_ltp(dir, &["validate", "--tree", tree]);
    assert_eq!(code, 0, "validate should be non-blocking: {json:?}");
    json["data"]["details"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["tree_id"] == *tree)
        .map(|d| {
            d["warnings"]
                .as_array()
                .unwrap()
                .iter()
                .map(|w| w["code"].as_str().unwrap().to_string())
                .collect()
        })
        .unwrap_or_default()
}

/// A single warning object matching `code` for `tree` (panics if absent).
fn find_warning(dir: &std::path::Path, tree: &str, code: &str) -> Value {
    let (json, rc) = run_ltp(dir, &["validate", "--tree", tree]);
    assert_eq!(rc, 0, "validate should be non-blocking: {json:?}");
    json["data"]["details"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["tree_id"] == *tree)
        .and_then(|d| {
            d["warnings"]
                .as_array()
                .unwrap()
                .iter()
                .find(|w| w["code"] == code)
                .cloned()
        })
        .unwrap_or_else(|| panic!("warning {code} not found for tree {tree}"))
}

// --- Fixtures -------------------------------------------------------------

/// A CRT (sufficiency) tree with two attached nodes A (RC-001) and E (UDE-001) and a fresh
/// reservation `MACRO-001` from A → E. No real edges exist. Returns `(tree, a, e, macro_id)`.
fn reserve_fixture(dir: &std::path::Path) -> (String, String, String, String) {
    setup(dir);
    let a = add_node(dir, "Root cause", "RC");
    let e = add_node(dir, "Undesirable effect", "UDE");
    let tree = create_tree(dir, "crt", "Lifecycle");
    attach(dir, &tree, &a);
    attach(dir, &tree, &e);
    let (json, code) = macro_add(dir, &tree, &a, &e, "Salto lógico");
    assert_eq!(code, 0, "macro add failed: {json:?}");
    let macro_id = json["data"]["macro_edge_id"].as_str().unwrap().to_string();
    (tree, a, e, macro_id)
}

/// Like [`reserve_fixture`] but with a pre-existing real edge E → A (LINK-001), so materializing
/// the reservation A → E (via expand or promote) would close a cycle. Returns `(tree, a, e, macro)`.
fn cycle_reserve_fixture(dir: &std::path::Path) -> (String, String, String, String) {
    setup(dir);
    let a = add_node(dir, "Root cause", "RC");
    let e = add_node(dir, "Undesirable effect", "UDE");
    let tree = create_tree(dir, "crt", "CycleLifecycle");
    attach(dir, &tree, &a);
    attach(dir, &tree, &e);
    connect(dir, &tree, &e, &a); // real back-edge E -> A (LINK-001)
    let (json, code) = macro_add(dir, &tree, &a, &e, "Salto que cerraría ciclo");
    assert_eq!(code, 0, "macro add failed: {json:?}");
    let macro_id = json["data"]["macro_edge_id"].as_str().unwrap().to_string();
    (tree, a, e, macro_id)
}

/// A linear chain A → B → C collapsed whole into a single overlay long arrow. Returns
/// `(tree, from, to, macro_id)`; the macro is an `Overlay` (created by `path collapse`).
fn collapsed_overlay_fixture(dir: &std::path::Path) -> (String, String, String, String) {
    setup(dir);
    let a = add_node(dir, "A", "RC");
    let b = add_node(dir, "B", "INT");
    let c = add_node(dir, "C", "UDE");
    let tree = create_tree(dir, "crt", "Collapsed");
    for n in [&a, &b, &c] {
        attach(dir, &tree, n);
    }
    connect(dir, &tree, &a, &b);
    connect(dir, &tree, &b, &c);
    let (json, code) = run_ltp(
        dir,
        &[
            "path", "collapse", "--tree", &tree, "--from", &a, "--to", &c, "--label", "Cadena",
        ],
    );
    assert_eq!(code, 0, "path collapse failed: {json:?}");
    let macro_id = json["data"]["macro_edge_id"].as_str().unwrap().to_string();
    (tree, a, c, macro_id)
}

// ============================================================================
// Happy path (H)
// ============================================================================

// H1: macro add → MACRO-001, reservation, empty interior.
#[test]
fn h1_macro_add_creates_reservation() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, a, e, macro_id) = reserve_fixture(dir.path());

    assert_eq!(macro_id, "MACRO-001");

    let (json, _) = macro_add(dir.path(), &tree, &a, &e, "otra"); // second add → MACRO-002
    assert_eq!(json["data"]["macro_edge_id"], "MACRO-002");

    let t = read_tree(dir.path(), &tree);
    let me = &t["macro_edges"][0];
    assert_eq!(me["id"], "MACRO-001");
    assert_eq!(me["status"], "reservation");
    assert_eq!(me["from"], a);
    assert_eq!(me["to"], e);
    assert_eq!(me["label"], "Salto lógico");
    assert!(me["interior_nodes"].as_array().unwrap().is_empty());
    assert!(me["interior_links"].as_array().unwrap().is_empty());
    // A fresh reservation never breaks the DAG (ADR-010: outside the DAG).
    let (add_json, _) = macro_add(dir.path(), &tree, &a, &e, "z");
    assert!(add_json["graph_health"]["valid_dag"].as_bool().unwrap());
}

// H2: macro expand --steps "a,b" → 2 INT + 3 LINK, overlay, chain verified by trace.
#[test]
fn h2_macro_expand_two_steps() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, a, e, macro_id) = reserve_fixture(dir.path());

    let (json, code) = macro_expand(dir.path(), &tree, &macro_id, "a,b");
    assert_eq!(code, 0, "expand failed: {json:?}");
    assert_eq!(json["action"], "macro_expand");
    assert_eq!(json["data"]["status"], "overlay");
    assert_eq!(
        str_vec(&json["data"]["created_nodes"]),
        ["INT-001", "INT-002"]
    );
    assert_eq!(
        str_vec(&json["data"]["created_links"]),
        ["LINK-001", "LINK-002", "LINK-003"]
    );

    // On-disk wiring: from → INT-001 → INT-002 → to.
    let t = read_tree(dir.path(), &tree);
    let edges = t["edges"].as_array().unwrap();
    let edge = |id: &str| edges.iter().find(|e| e["id"] == id).unwrap().clone();
    assert_eq!(edge("LINK-001")["from"][0], a);
    assert_eq!(edge("LINK-001")["to"], "INT-001");
    assert_eq!(edge("LINK-002")["from"][0], "INT-001");
    assert_eq!(edge("LINK-002")["to"], "INT-002");
    assert_eq!(edge("LINK-003")["from"][0], "INT-002");
    assert_eq!(edge("LINK-003")["to"], e);

    // Trace upstream from the effect reaches the whole materialized chain.
    let (trace, tc) = run_ltp(
        dir.path(),
        &[
            "trace",
            &e,
            "--tree",
            &tree,
            "--direction",
            "upstream",
            "--depth",
            "5",
        ],
    );
    assert_eq!(tc, 0, "trace failed: {trace:?}");
    let chain: Vec<String> = trace["data"]["chain"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["node"].as_str().unwrap().to_string())
        .collect();
    for expected in [a.as_str(), "INT-001", "INT-002", e.as_str()] {
        assert!(
            chain.contains(&expected.to_string()),
            "trace chain {chain:?} missing {expected}"
        );
    }
}

// H3: macro promote of a reservation with no assumptions → 1 SINGLE atomic edge, macro consumed.
#[test]
fn h3_macro_promote_no_assumptions() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, a, e, macro_id) = reserve_fixture(dir.path());

    let (json, code) = macro_promote(dir.path(), &tree, &macro_id);
    assert_eq!(code, 0, "promote failed: {json:?}");
    assert_eq!(json["data"]["created_link"], "LINK-001");
    assert!(json["data"]["migrated_assumptions"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(json["data"]["from"], a);
    assert_eq!(json["data"]["to"], e);

    let t = read_tree(dir.path(), &tree);
    assert!(
        t["macro_edges"].as_array().unwrap().is_empty(),
        "macro consumed"
    );
    let edges = t["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0]["operator"], "SINGLE");
    assert_eq!(edges[0]["from"][0], a);
    assert_eq!(edges[0]["to"], e);
    assert!(edges[0]["assumptions"].as_array().unwrap().is_empty());
}

// H4: macro promote of a reservation carrying 2 MacroAssumptions → atomic edge with 2 Assumptions.
#[test]
fn h4_macro_promote_migrates_assumptions() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, _a, _e, macro_id) = reserve_fixture(dir.path());

    let (j1, c1) = macro_assume_add(
        dir.path(),
        &tree,
        &macro_id,
        "El proveedor cumple SLA",
        None,
    );
    assert_eq!(c1, 0, "macro-assume add failed: {j1:?}");
    let (j2, c2) = macro_assume_add(dir.path(), &tree, &macro_id, "La demanda es estable", None);
    assert_eq!(c2, 0, "macro-assume add failed: {j2:?}");

    let (json, code) = macro_promote(dir.path(), &tree, &macro_id);
    assert_eq!(code, 0, "promote failed: {json:?}");
    assert_eq!(
        str_vec(&json["data"]["migrated_assumptions"]),
        ["ASM-001", "ASM-002"]
    );

    let t = read_tree(dir.path(), &tree);
    assert!(t["macro_edges"].as_array().unwrap().is_empty());
    let asms = t["edges"][0]["assumptions"].as_array().unwrap();
    assert_eq!(asms.len(), 2);
    assert_eq!(asms[0]["id"], "ASM-001");
    assert_eq!(asms[0]["text"], "El proveedor cumple SLA");
    assert_eq!(asms[0]["status"], "valid");
    assert_eq!(asms[1]["id"], "ASM-002");
    assert_eq!(asms[1]["text"], "La demanda es estable");
}

// H5: macro expand --steps "único" (n=1) → 1 INT + 2 LINK.
#[test]
fn h5_macro_expand_single_step() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, _a, _e, macro_id) = reserve_fixture(dir.path());

    let (json, code) = macro_expand(dir.path(), &tree, &macro_id, "único");
    assert_eq!(code, 0, "expand failed: {json:?}");
    assert_eq!(str_vec(&json["data"]["created_nodes"]), ["INT-001"]);
    assert_eq!(
        str_vec(&json["data"]["created_links"]),
        ["LINK-001", "LINK-002"]
    );
    assert_eq!(json["data"]["status"], "overlay");
}

// ============================================================================
// Boundary (B)
// ============================================================================

// B1: macro add --label "" → LABEL_REQUIRED; MACRO counter not incremented.
#[test]
fn b1_macro_add_empty_label() {
    let dir = tempfile::tempdir().unwrap();
    setup(dir.path());
    let a = add_node(dir.path(), "A", "RC");
    let e = add_node(dir.path(), "E", "UDE");
    let tree = create_tree(dir.path(), "crt", "B1");
    attach(dir.path(), &tree, &a);
    attach(dir.path(), &tree, &e);

    let (json, code) = macro_add(dir.path(), &tree, &a, &e, "");
    assert_eq!(code, 1, "expected failure: {json:?}");
    assert_eq!(json["errors"][0]["code"], "LABEL_REQUIRED");
    assert_eq!(
        counter(dir.path(), "MACRO"),
        0,
        "counter must not burn on validation failure"
    );
    assert!(read_tree(dir.path(), &tree)["macro_edges"]
        .as_array()
        .unwrap()
        .is_empty());
}

// B2: macro add --from X --to X → RESERVATION_SELF_LOOP.
#[test]
fn b2_macro_add_self_loop() {
    let dir = tempfile::tempdir().unwrap();
    setup(dir.path());
    let a = add_node(dir.path(), "A", "RC");
    let tree = create_tree(dir.path(), "crt", "B2");
    attach(dir.path(), &tree, &a);

    let (json, code) = macro_add(dir.path(), &tree, &a, &a, "loop");
    assert_eq!(code, 1, "expected failure: {json:?}");
    assert_eq!(json["errors"][0]["code"], "RESERVATION_SELF_LOOP");
    assert_eq!(json["errors"][0]["node_id"], a);
    assert_eq!(counter(dir.path(), "MACRO"), 0);
}

// B3: macro expand --steps "" → STEPS_REQUIRED; no INT/LINK; status stays reservation.
#[test]
fn b3_macro_expand_empty_steps() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, _a, _e, macro_id) = reserve_fixture(dir.path());

    let (json, code) = macro_expand(dir.path(), &tree, &macro_id, "");
    assert_eq!(code, 1, "expected failure: {json:?}");
    assert_eq!(json["errors"][0]["code"], "STEPS_REQUIRED");
    assert_eq!(counter(dir.path(), "INT"), 0);
    assert_eq!(counter(dir.path(), "LINK"), 0);

    let t = read_tree(dir.path(), &tree);
    assert_eq!(t["macro_edges"][0]["status"], "reservation");
    assert!(t["edges"].as_array().unwrap().is_empty());
    // Also robust to whitespace-only / comma-only garbage.
    let (json2, code2) = macro_expand(dir.path(), &tree, &macro_id, " , , ");
    assert_eq!(code2, 1, "expected failure: {json2:?}");
    assert_eq!(json2["errors"][0]["code"], "STEPS_REQUIRED");
}

// B4: macro expand --steps "l1,l1" → 2 distinct INT with the same label; no error.
#[test]
fn b4_macro_expand_duplicate_labels() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, _a, _e, macro_id) = reserve_fixture(dir.path());

    let (json, code) = macro_expand(dir.path(), &tree, &macro_id, "l1,l1");
    assert_eq!(code, 0, "expand failed: {json:?}");
    assert_eq!(
        str_vec(&json["data"]["created_nodes"]),
        ["INT-001", "INT-002"]
    );

    let (i1, _) = run_ltp(dir.path(), &["node", "inspect", "INT-001"]);
    let (i2, _) = run_ltp(dir.path(), &["node", "inspect", "INT-002"]);
    assert_eq!(i1["data"]["label"], "l1");
    assert_eq!(i2["data"]["label"], "l1");
    assert_ne!(i1["data"]["id"], i2["data"]["id"], "distinct INT nodes");
}

// B5: macro expand on a NECESSITY tree (PRT) → edges carry logic NECESSITY (derived from tree).
#[test]
fn b5_macro_expand_necessity_logic() {
    let dir = tempfile::tempdir().unwrap();
    setup(dir.path());
    let a = add_node(dir.path(), "Prereq", "RC");
    let e = add_node(dir.path(), "Objective", "UDE");
    let tree = create_tree(dir.path(), "prt", "B5");
    attach(dir.path(), &tree, &a);
    attach(dir.path(), &tree, &e);
    let (add, ac) = macro_add(dir.path(), &tree, &a, &e, "salto necesidad");
    assert_eq!(ac, 0, "macro add failed: {add:?}");
    let macro_id = add["data"]["macro_edge_id"].as_str().unwrap().to_string();

    let (json, code) = macro_expand(dir.path(), &tree, &macro_id, "paso");
    assert_eq!(code, 0, "expand failed: {json:?}");

    let t = read_tree(dir.path(), &tree);
    for edge in t["edges"].as_array().unwrap() {
        assert_eq!(
            edge["logic"], "NECESSITY",
            "edge {edge:?} should be NECESSITY"
        );
    }
}

// B6: promote a reservation whose endpoints are the only nodes → validate reports no orphan.
#[test]
fn b6_macro_promote_endpoints_not_orphan() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, _a, _e, macro_id) = reserve_fixture(dir.path());

    let (json, code) = macro_promote(dir.path(), &tree, &macro_id);
    assert_eq!(code, 0, "promote failed: {json:?}");

    let codes = all_warning_codes(dir.path(), &tree);
    assert!(
        !codes.contains(&"ORPHAN_NODE_IN_TREE".to_string()),
        "endpoints joined by the atomic edge must not be orphans: {codes:?}"
    );
    assert!(
        !codes.contains(&"LONG_ARROW_RESERVATION_PENDING".to_string()),
        "no reservation remains after promote: {codes:?}"
    );
}

// ============================================================================
// Corrupt / referential (C)
// ============================================================================

// C1: add / expand / promote against a non-existent tree → TREE_NOT_FOUND.
#[test]
fn c1_tree_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let (_tree, a, e, macro_id) = reserve_fixture(dir.path());

    let (add, ac) = macro_add(dir.path(), "TREE-999", &a, &e, "x");
    assert_eq!(ac, 1);
    assert_eq!(add["errors"][0]["code"], "TREE_NOT_FOUND");

    let (exp, ec) = macro_expand(dir.path(), "TREE-999", &macro_id, "a");
    assert_eq!(ec, 1);
    assert_eq!(exp["errors"][0]["code"], "TREE_NOT_FOUND");

    let (pro, pc) = macro_promote(dir.path(), "TREE-999", &macro_id);
    assert_eq!(pc, 1);
    assert_eq!(pro["errors"][0]["code"], "TREE_NOT_FOUND");
}

// C2: expand / promote with a non-existent macro-link → MACRO_EDGE_NOT_FOUND.
#[test]
fn c2_macro_edge_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, _a, _e, _macro_id) = reserve_fixture(dir.path());

    let (exp, ec) = macro_expand(dir.path(), &tree, "MACRO-999", "a");
    assert_eq!(ec, 1);
    assert_eq!(exp["errors"][0]["code"], "MACRO_EDGE_NOT_FOUND");

    let (pro, pc) = macro_promote(dir.path(), &tree, "MACRO-999");
    assert_eq!(pc, 1);
    assert_eq!(pro["errors"][0]["code"], "MACRO_EDGE_NOT_FOUND");
}

// C3: macro add with a from-node not attached to the tree → NODE_NOT_IN_TREE; no mutation.
#[test]
fn c3_macro_add_node_not_in_tree() {
    let dir = tempfile::tempdir().unwrap();
    setup(dir.path());
    let e = add_node(dir.path(), "E", "UDE");
    let tree = create_tree(dir.path(), "crt", "C3");
    attach(dir.path(), &tree, &e); // only E is attached; RC-999 does not exist / is not attached

    let (json, code) = macro_add(dir.path(), &tree, "RC-999", &e, "x");
    assert_eq!(code, 1, "expected failure: {json:?}");
    assert_eq!(json["errors"][0]["code"], "NODE_NOT_IN_TREE");
    assert_eq!(json["errors"][0]["node_id"], "RC-999");
    assert_eq!(counter(dir.path(), "MACRO"), 0);
    assert!(read_tree(dir.path(), &tree)["macro_edges"]
        .as_array()
        .unwrap()
        .is_empty());
}

// C4: macro expand over an Overlay (from path collapse) → NOT_A_RESERVATION; no mutation.
#[test]
fn c4_expand_over_overlay() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, _a, _c, macro_id) = collapsed_overlay_fixture(dir.path());

    let before = read_tree(dir.path(), &tree);
    let (json, code) = macro_expand(dir.path(), &tree, &macro_id, "extra");
    assert_eq!(code, 1, "expected failure: {json:?}");
    assert_eq!(json["errors"][0]["code"], "NOT_A_RESERVATION");

    let after = read_tree(dir.path(), &tree);
    assert_eq!(
        edge_ids(&before),
        edge_ids(&after),
        "no mutation on NOT_A_RESERVATION"
    );
    assert_eq!(node_refs(&before), node_refs(&after));
}

// C5: macro promote over an Overlay → NOT_A_RESERVATION.
#[test]
fn c5_promote_over_overlay() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, _a, _c, macro_id) = collapsed_overlay_fixture(dir.path());

    let (json, code) = macro_promote(dir.path(), &tree, &macro_id);
    assert_eq!(code, 1, "expected failure: {json:?}");
    assert_eq!(json["errors"][0]["code"], "NOT_A_RESERVATION");
    // Still an overlay, unchanged.
    let t = read_tree(dir.path(), &tree);
    assert_eq!(t["macro_edges"][0]["status"], "overlay");
}

// C6: a legacy macro_edge (`"status":"active"`, no `assumptions`) deserializes as Overlay.
#[test]
fn c6_legacy_active_status_deserializes_as_overlay() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, _a, _c, _macro_id) = collapsed_overlay_fixture(dir.path());

    // Rewrite the on-disk macro_edge to the pre-Slice-2 shape: status "active", no assumptions.
    let path = dir.path().join("trees").join(format!("{tree}.json"));
    let mut t = read_tree(dir.path(), &tree);
    let me = &mut t["macro_edges"][0];
    me["status"] = Value::String("active".to_string());
    if let Some(obj) = me.as_object_mut() {
        obj.remove("assumptions");
    }
    std::fs::write(&path, serde_json::to_string_pretty(&t).unwrap()).unwrap();

    // validate must accept it (no deserialization error) and treat it as an overlay:
    // no RESERVATION_PENDING is emitted.
    let codes = all_warning_codes(dir.path(), &tree);
    assert!(
        !codes.contains(&"LONG_ARROW_RESERVATION_PENDING".to_string()),
        "legacy 'active' must be read as overlay, not reservation: {codes:?}"
    );

    // A fresh gather confirms the engine parsed it as a valid overlay macro (interior populated).
    let (gather, gc) = run_ltp(
        dir.path(),
        &[
            "macro-assume",
            "gather",
            "--tree",
            &tree,
            "--macro-link",
            "MACRO-001",
        ],
    );
    assert_eq!(gc, 0, "gather over legacy overlay failed: {gather:?}");
    assert!(gather["success"].as_bool().unwrap());
}

// ============================================================================
// Illegal transitions / interaction (I)
// ============================================================================

// I1: expand then expand again → second call is NOT_A_RESERVATION (already overlay).
#[test]
fn i1_double_expand() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, _a, _e, macro_id) = reserve_fixture(dir.path());

    let (j1, c1) = macro_expand(dir.path(), &tree, &macro_id, "a");
    assert_eq!(c1, 0, "first expand failed: {j1:?}");

    let (j2, c2) = macro_expand(dir.path(), &tree, &macro_id, "b");
    assert_eq!(c2, 1, "second expand should fail: {j2:?}");
    assert_eq!(j2["errors"][0]["code"], "NOT_A_RESERVATION");
}

// I2: promote then promote/expand again → MACRO_EDGE_NOT_FOUND (reservation consumed).
#[test]
fn i2_reuse_after_promote() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, _a, _e, macro_id) = reserve_fixture(dir.path());

    let (j0, c0) = macro_promote(dir.path(), &tree, &macro_id);
    assert_eq!(c0, 0, "promote failed: {j0:?}");

    let (j1, c1) = macro_promote(dir.path(), &tree, &macro_id);
    assert_eq!(c1, 1);
    assert_eq!(j1["errors"][0]["code"], "MACRO_EDGE_NOT_FOUND");

    let (j2, c2) = macro_expand(dir.path(), &tree, &macro_id, "a");
    assert_eq!(c2, 1);
    assert_eq!(j2["errors"][0]["code"], "MACRO_EDGE_NOT_FOUND");
}

// I3: a bare reservation → validate emits RESERVATION_PENDING and NOT orphan on the endpoints.
#[test]
fn i3_reservation_pending_not_orphan() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, a, e, macro_id) = reserve_fixture(dir.path());

    let codes = all_warning_codes(dir.path(), &tree);
    assert!(
        !codes.contains(&"ORPHAN_NODE_IN_TREE".to_string()),
        "reservation endpoints are logically connected: {codes:?}"
    );
    assert!(codes.contains(&"LONG_ARROW_RESERVATION_PENDING".to_string()));

    let pending = find_warning(dir.path(), &tree, "LONG_ARROW_RESERVATION_PENDING");
    assert_eq!(pending["macro_link"], macro_id);
    assert_eq!(pending["from"], a);
    assert_eq!(pending["to"], e);
}

// I4: path collapse yields an Overlay macro; validate emits no RESERVATION_PENDING.
#[test]
fn i4_collapse_is_overlay() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, _a, _c, _macro_id) = collapsed_overlay_fixture(dir.path());

    let t = read_tree(dir.path(), &tree);
    assert_eq!(t["macro_edges"][0]["status"], "overlay");

    let codes = all_warning_codes(dir.path(), &tree);
    assert!(
        !codes.contains(&"LONG_ARROW_RESERVATION_PENDING".to_string()),
        "an overlay is not a pending reservation: {codes:?}"
    );
}

// I5: macro-assume add --projection LINK-xxx over a reservation → PROJECTION_REF_NOT_IN_INTERIOR.
#[test]
fn i5_macro_assume_projection_into_empty_interior() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, _a, _e, macro_id) = reserve_fixture(dir.path());

    let (json, code) = macro_assume_add(dir.path(), &tree, &macro_id, "sup", Some("LINK-999"));
    assert_eq!(code, 1, "expected failure: {json:?}");
    assert_eq!(json["errors"][0]["code"], "PROJECTION_REF_NOT_IN_INTERIOR");
}

// I6: an ungrounded macro-assume "ages" — clean on the reservation, UNGROUNDED after expand.
#[test]
fn i6_assumption_ages_on_expand() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, _a, _e, macro_id) = reserve_fixture(dir.path());

    // Add a summary without projection on the (empty-interior) reservation: allowed, not ungrounded.
    let (add, ac) = macro_assume_add(dir.path(), &tree, &macro_id, "sup sin proyección", None);
    assert_eq!(ac, 0, "macro-assume add failed: {add:?}");

    let before = all_warning_codes(dir.path(), &tree);
    assert!(
        !before.contains(&"MACRO_ASSUMPTION_UNGROUNDED".to_string()),
        "empty interior → not ungrounded yet: {before:?}"
    );
    assert!(before.contains(&"LONG_ARROW_RESERVATION_PENDING".to_string()));

    // Expand: the interior becomes non-empty while refs stay empty → the summary is now ungrounded.
    let (exp, ec) = macro_expand(dir.path(), &tree, &macro_id, "paso");
    assert_eq!(ec, 0, "expand failed: {exp:?}");

    let after = all_warning_codes(dir.path(), &tree);
    assert!(
        after.contains(&"MACRO_ASSUMPTION_UNGROUNDED".to_string()),
        "non-empty interior + empty refs → ungrounded: {after:?}"
    );
    assert!(
        !after.contains(&"LONG_ARROW_RESERVATION_PENDING".to_string()),
        "overlay is no longer a pending reservation: {after:?}"
    );
}

// I7: promote that would close a cycle → CIRCULAR_DEPENDENCY_DETECTED, no mutation.
#[test]
fn i7_promote_blocks_cycle() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, _a, _e, macro_id) = cycle_reserve_fixture(dir.path());

    let (json, code) = macro_promote(dir.path(), &tree, &macro_id);
    assert_eq!(code, 1, "promote should be blocked: {json:?}");
    assert_eq!(json["errors"][0]["code"], "CIRCULAR_DEPENDENCY_DETECTED");
    assert!(
        json["errors"][0]["cycle_path"].is_array(),
        "cycle_path context expected: {json:?}"
    );
    assert!(!json["graph_health"]["valid_dag"].as_bool().unwrap());

    // No mutation: reservation intact, no atomic edge minted, no ASM burned.
    let t = read_tree(dir.path(), &tree);
    assert_eq!(t["macro_edges"][0]["id"], macro_id);
    assert_eq!(t["macro_edges"][0]["status"], "reservation");
    assert_eq!(
        edge_ids(&t),
        ["LINK-001"],
        "only the pre-existing back-edge remains"
    );
    assert_eq!(
        counter(dir.path(), "ASM"),
        0,
        "no ASM counter burned on the blocked path"
    );
}

// I8: expanding a reservation whose endpoints were orphans → no orphan, no pending afterwards.
#[test]
fn i8_expand_clears_orphan_and_pending() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, _a, _e, macro_id) = reserve_fixture(dir.path());

    let (exp, ec) = macro_expand(dir.path(), &tree, &macro_id, "paso");
    assert_eq!(ec, 0, "expand failed: {exp:?}");

    let codes = all_warning_codes(dir.path(), &tree);
    assert!(
        !codes.contains(&"ORPHAN_NODE_IN_TREE".to_string()),
        "endpoints joined by real edges: {codes:?}"
    );
    assert!(
        !codes.contains(&"LONG_ARROW_RESERVATION_PENDING".to_string()),
        "now an overlay: {codes:?}"
    );
}

// I9: expand a reservation A→E when a real path E→…→A exists → cycle block, nothing materialized.
#[test]
fn i9_expand_blocks_cycle() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, _a, _e, macro_id) = cycle_reserve_fixture(dir.path());

    let (json, code) = macro_expand(dir.path(), &tree, &macro_id, "x");
    assert_eq!(code, 1, "expand should be blocked: {json:?}");
    assert_eq!(json["errors"][0]["code"], "CIRCULAR_DEPENDENCY_DETECTED");
    assert!(json["errors"][0]["cycle_path"].is_array());
    assert!(!json["graph_health"]["valid_dag"].as_bool().unwrap());

    // Pre-check blocks before persisting: no INT node attached, status still reservation, no new edge.
    let t = read_tree(dir.path(), &tree);
    assert_eq!(t["macro_edges"][0]["status"], "reservation");
    assert!(t["macro_edges"][0]["interior_nodes"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(
        !node_refs(&t).contains(&"INT-001".to_string()),
        "no INT materialized on the blocked path"
    );
    assert_eq!(
        edge_ids(&t),
        ["LINK-001"],
        "only the pre-existing back-edge remains"
    );
    // The INT node file must not have been written (counter may burn, file must not).
    assert!(
        !dir.path().join("nodes").join("INT-001.json").exists(),
        "no orphan INT file left behind"
    );
}

// ============================================================================
// Ordering / idempotency (O)
// ============================================================================

// O1: undo of macro add removes it; redo restores the SAME MACRO id; counter does not rewind.
#[test]
fn o1_undo_redo_macro_add() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, _a, _e, macro_id) = reserve_fixture(dir.path());
    assert_eq!(counter(dir.path(), "MACRO"), 1);

    undo(dir.path());
    assert!(
        read_tree(dir.path(), &tree)["macro_edges"]
            .as_array()
            .unwrap()
            .is_empty(),
        "undo removes the reservation"
    );
    assert_eq!(
        counter(dir.path(), "MACRO"),
        1,
        "counter never rewinds (ADR-009 snapshot)"
    );

    redo(dir.path());
    let t = read_tree(dir.path(), &tree);
    assert_eq!(
        t["macro_edges"][0]["id"], macro_id,
        "redo restores the same id"
    );
    assert_eq!(t["macro_edges"][0]["status"], "reservation");
}

// O2: undo of macro expand removes INT/LINK and reverts to reservation; redo restores same ids.
#[test]
fn o2_undo_redo_macro_expand() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, _a, _e, macro_id) = reserve_fixture(dir.path());
    let (exp, ec) = macro_expand(dir.path(), &tree, &macro_id, "a,b");
    assert_eq!(ec, 0, "expand failed: {exp:?}");

    undo(dir.path());
    let after_undo = read_tree(dir.path(), &tree);
    assert_eq!(after_undo["macro_edges"][0]["status"], "reservation");
    assert!(after_undo["macro_edges"][0]["interior_nodes"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(after_undo["edges"].as_array().unwrap().is_empty());
    assert!(
        !node_refs(&after_undo).contains(&"INT-001".to_string()),
        "INT detached"
    );
    assert!(
        !dir.path().join("nodes").join("INT-001.json").exists(),
        "INT file removed"
    );

    redo(dir.path());
    let after_redo = read_tree(dir.path(), &tree);
    assert_eq!(after_redo["macro_edges"][0]["status"], "overlay");
    assert_eq!(
        str_vec(&after_redo["macro_edges"][0]["interior_nodes"]),
        ["INT-001", "INT-002"]
    );
    assert_eq!(edge_ids(&after_redo), ["LINK-001", "LINK-002", "LINK-003"]);
    assert!(dir.path().join("nodes").join("INT-001.json").exists());
}

// O3: undo of macro promote restores the reservation WITH its MacroAssumptions; redo re-applies.
#[test]
fn o3_undo_redo_macro_promote() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, _a, _e, macro_id) = reserve_fixture(dir.path());
    let (ma, mc) = macro_assume_add(dir.path(), &tree, &macro_id, "sup migrable", None);
    assert_eq!(mc, 0, "macro-assume add failed: {ma:?}");
    let (pro, pc) = macro_promote(dir.path(), &tree, &macro_id);
    assert_eq!(pc, 0, "promote failed: {pro:?}");

    undo(dir.path());
    let after_undo = read_tree(dir.path(), &tree);
    assert!(
        after_undo["edges"].as_array().unwrap().is_empty(),
        "atomic edge gone"
    );
    let me = &after_undo["macro_edges"][0];
    assert_eq!(me["id"], macro_id);
    assert_eq!(me["status"], "reservation");
    let asms = me["assumptions"].as_array().unwrap();
    assert_eq!(asms.len(), 1, "MacroAssumption restored intact");
    assert_eq!(asms[0]["id"], "MASM-001");
    assert_eq!(asms[0]["text"], "sup migrable");

    redo(dir.path());
    let after_redo = read_tree(dir.path(), &tree);
    assert!(
        after_redo["macro_edges"].as_array().unwrap().is_empty(),
        "consumed again"
    );
    assert_eq!(after_redo["edges"][0]["assumptions"][0]["id"], "ASM-001");
    assert_eq!(
        after_redo["edges"][0]["assumptions"][0]["text"],
        "sup migrable"
    );
}

// O4: a batch (begin-batch … add + expand + macro-assume add … end-batch) reverts in one undo.
#[test]
fn o4_batch_single_undo() {
    let dir = tempfile::tempdir().unwrap();
    setup(dir.path());
    let a = add_node(dir.path(), "A", "RC");
    let e = add_node(dir.path(), "E", "UDE");
    let tree = create_tree(dir.path(), "crt", "O4");
    attach(dir.path(), &tree, &a);
    attach(dir.path(), &tree, &e);

    let (bb, bc) = run_ltp(
        dir.path(),
        &["history", "begin-batch", "--label", "long-arrow"],
    );
    assert_eq!(bc, 0, "begin-batch failed: {bb:?}");
    let (add, ac) = macro_add(dir.path(), &tree, &a, &e, "salto");
    assert_eq!(ac, 0, "macro add failed: {add:?}");
    let macro_id = add["data"]["macro_edge_id"].as_str().unwrap().to_string();
    let (exp, ec) = macro_expand(dir.path(), &tree, &macro_id, "a,b");
    assert_eq!(ec, 0, "expand failed: {exp:?}");
    let (ma, mc) = macro_assume_add(dir.path(), &tree, &macro_id, "sup", None);
    assert_eq!(mc, 0, "macro-assume add failed: {ma:?}");
    let (eb, ebc) = run_ltp(dir.path(), &["history", "end-batch"]);
    assert_eq!(ebc, 0, "end-batch failed: {eb:?}");

    // Sanity: everything is present before undo.
    let before = read_tree(dir.path(), &tree);
    assert_eq!(before["macro_edges"].as_array().unwrap().len(), 1);
    assert_eq!(edge_ids(&before).len(), 3);

    // A single undo reverts the whole batch.
    undo(dir.path());
    let after = read_tree(dir.path(), &tree);
    assert!(
        after["macro_edges"].as_array().unwrap().is_empty(),
        "macro gone"
    );
    assert!(after["edges"].as_array().unwrap().is_empty(), "edges gone");
    assert_eq!(node_refs(&after), [a, e], "back to the two original nodes");
}

// O5: macro expand --steps "a,b,c" → stable INT-001..003 / LINK-001..004 ordering.
#[test]
fn o5_expand_stable_id_ordering() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, _a, _e, macro_id) = reserve_fixture(dir.path());

    let (json, code) = macro_expand(dir.path(), &tree, &macro_id, "a,b,c");
    assert_eq!(code, 0, "expand failed: {json:?}");
    assert_eq!(
        str_vec(&json["data"]["created_nodes"]),
        ["INT-001", "INT-002", "INT-003"]
    );
    assert_eq!(
        str_vec(&json["data"]["created_links"]),
        ["LINK-001", "LINK-002", "LINK-003", "LINK-004"]
    );
}
