//! Integration tests for `macro-assume gather` (Slice 1, Phase M2).
//!
//! Exercises the read-only "gather" command end-to-end through the CLI binary:
//! the happy path over a collapsed long arrow (H1) plus the referential-integrity
//! error paths (C1: macro-edge not found, C2: tree not found).

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
            "Failed to parse JSON.\nstdout: {}\nstderr: {}",
            stdout,
            String::from_utf8_lossy(&output.stderr)
        )
    });
    (json, code)
}

fn setup_workspace(dir: &std::path::Path) {
    let (_, code) = run_ltp(dir, &["init", "--name", "MacroAssumeTest"]);
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
    assert_eq!(code, 0, "link connect failed: {json:?}");
    json["data"]["created_links"][0]
        .as_str()
        .unwrap()
        .to_string()
}

fn add_assumption(dir: &std::path::Path, tree_id: &str, link_id: &str, text: &str) -> String {
    let (json, code) = run_ltp(
        dir,
        &[
            "assume", "add", "--tree", tree_id, "--link", link_id, "--text", text,
        ],
    );
    assert_eq!(code, 0, "assume add failed: {json:?}");
    json["data"]["id"].as_str().unwrap().to_string()
}

fn collapse(dir: &std::path::Path, tree_id: &str, from: &str, to: &str, label: &str) -> String {
    let (json, code) = run_ltp(
        dir,
        &[
            "path", "collapse", "--tree", tree_id, "--from", from, "--to", to, "--label", label,
        ],
    );
    assert_eq!(code, 0, "path collapse failed: {json:?}");
    json["data"]["macro_edge_id"].as_str().unwrap().to_string()
}

/// Build a linear chain A → B → C → D → E.
///
/// Returns `(tree_id, nodes[A..E], links[L1..L4])`. Nothing is collapsed.
fn linear_chain(dir: &std::path::Path) -> (String, Vec<String>, Vec<String>) {
    setup_workspace(dir);
    let a = add_node(dir, "Node A", "RC");
    let b = add_node(dir, "Node B", "INT");
    let c = add_node(dir, "Node C", "INT");
    let d = add_node(dir, "Node D", "INT");
    let e = add_node(dir, "Node E", "UDE");
    let tree = create_tree(dir, "crt", "ChainCRT");
    for n in [&a, &b, &c, &d, &e] {
        attach_node(dir, &tree, n);
    }
    let l1 = connect(dir, &tree, &a, &b);
    let l2 = connect(dir, &tree, &b, &c);
    let l3 = connect(dir, &tree, &c, &d);
    let l4 = connect(dir, &tree, &d, &e);
    (tree, vec![a, b, c, d, e], vec![l1, l2, l3, l4])
}

/// Build a linear chain and collapse it whole (A → E) into a single long arrow.
///
/// Returns `(tree_id, links[L1..L4], macro_edge_id)`. Interior links = all 4 edges
/// (collapse is non-destructive: interior edges stay in `tree.edges`).
fn collapsed_chain(dir: &std::path::Path) -> (String, Vec<String>, String) {
    let (tree, nodes, links) = linear_chain(dir);
    let macro_id = collapse(dir, &tree, &nodes[0], &nodes[4], "Cadena lógica");
    (tree, links, macro_id)
}

fn string_array(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap().to_string())
        .collect()
}

// --- UAT H1: gather over a fresh (unsummarized) long arrow ---
#[test]
fn uat_h1_gather_fresh_macro_edge() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, links, macro_id) = collapsed_chain(dir.path());

    // Two interior links carry assumptions; the other two stay empty.
    let asm1 = add_assumption(dir.path(), &tree, &links[0], "El proveedor cumple SLA");
    let asm2 = add_assumption(dir.path(), &tree, &links[1], "La demanda es estable");

    let (json, code) = run_ltp(
        dir.path(),
        &[
            "macro-assume",
            "gather",
            "--tree",
            &tree,
            "--macro-link",
            &macro_id,
        ],
    );
    assert_eq!(code, 0, "gather failed: {json:?}");
    assert!(json["success"].as_bool().unwrap());
    assert_eq!(json["action"], "macro_assume_gather");
    assert_eq!(json["data"]["macro_link"], macro_id);

    // Interior: all 4 links present, keyed by link_id; empties included.
    let interior = &json["data"]["interior"];
    assert!(interior.is_object());
    let interior_obj = interior.as_object().unwrap();
    assert_eq!(
        interior_obj.len(),
        4,
        "all interior links present: {interior:?}"
    );
    assert_eq!(interior[&links[0]].as_array().unwrap().len(), 1);
    assert_eq!(interior[&links[0]][0]["id"], asm1);
    assert_eq!(interior[&links[0]][0]["status"], "valid");
    assert_eq!(interior[&links[1]].as_array().unwrap().len(), 1);
    assert_eq!(interior[&links[1]][0]["id"], asm2);
    assert!(interior[&links[2]].as_array().unwrap().is_empty());
    assert!(interior[&links[3]].as_array().unwrap().is_empty());

    // Diff: no summary yet, so both interior ASMs are unmapped, nothing dangling.
    let diff = &json["data"]["diff"];
    assert_eq!(diff["summary_count"], 0);
    let unmapped = string_array(&diff["unmapped"]);
    assert_eq!(unmapped, vec![asm1, asm2]);
    assert!(diff["dangling"].as_array().unwrap().is_empty());

    // Non-blocking: a fresh long arrow is a valid DAG.
    assert!(json["graph_health"]["valid_dag"].as_bool().unwrap());
    assert!(json["errors"].as_array().unwrap().is_empty());
}

// --- UAT C1: gather with a non-existent macro-edge => MACRO_EDGE_NOT_FOUND ---
#[test]
fn uat_c1_gather_macro_edge_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, _links, _macro_id) = collapsed_chain(dir.path());

    let (json, code) = run_ltp(
        dir.path(),
        &[
            "macro-assume",
            "gather",
            "--tree",
            &tree,
            "--macro-link",
            "MACRO-999",
        ],
    );
    assert_eq!(code, 1, "expected failure exit code: {json:?}");
    assert!(!json["success"].as_bool().unwrap());
    assert_eq!(json["errors"][0]["code"], "MACRO_EDGE_NOT_FOUND");
    // Empty data envelope preserves the queried macro_link for context.
    assert_eq!(json["data"]["macro_link"], "MACRO-999");
    assert!(json["data"]["interior"].as_object().unwrap().is_empty());
}

// --- UAT C2: gather against a non-existent tree => TREE_NOT_FOUND ---
#[test]
fn uat_c2_gather_tree_not_found() {
    let dir = tempfile::tempdir().unwrap();
    setup_workspace(dir.path());

    let (json, code) = run_ltp(
        dir.path(),
        &[
            "macro-assume",
            "gather",
            "--tree",
            "TREE-999",
            "--macro-link",
            "MACRO-001",
        ],
    );
    assert_eq!(code, 1, "expected failure exit code: {json:?}");
    assert!(!json["success"].as_bool().unwrap());
    assert_eq!(json["errors"][0]["code"], "TREE_NOT_FOUND");
}

// ============================================================================
// Phase M3: add / rm / list
// ============================================================================

fn macro_add(
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

fn macro_rm(dir: &std::path::Path, tree: &str, macro_link: &str, asm: &str) -> (Value, i32) {
    run_ltp(
        dir,
        &[
            "macro-assume",
            "rm",
            "--tree",
            tree,
            "--macro-link",
            macro_link,
            "--asm",
            asm,
        ],
    )
}

fn macro_list(dir: &std::path::Path, tree: &str, macro_link: &str) -> (Value, i32) {
    run_ltp(
        dir,
        &[
            "macro-assume",
            "list",
            "--tree",
            tree,
            "--macro-link",
            macro_link,
        ],
    )
}

/// Read the single tree file in `trees/` as JSON (for on-disk serialization assertions).
fn read_only_tree(dir: &std::path::Path) -> Value {
    let trees_dir = dir.join("trees");
    let entry = std::fs::read_dir(&trees_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.path().extension().and_then(|x| x.to_str()) == Some("json"))
        .expect("exactly one tree file expected");
    let content = std::fs::read_to_string(entry.path()).unwrap();
    serde_json::from_str(&content).unwrap()
}

fn warning_codes(json: &Value) -> Vec<String> {
    json["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["code"].as_str().unwrap().to_string())
        .collect()
}

// --- UAT H2: add with 2 valid projection refs => MASM-001, canonical order ---
#[test]
fn uat_h2_add_with_projections() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, links, macro_id) = collapsed_chain(dir.path());
    let asm1 = add_assumption(dir.path(), &tree, &links[0], "interior 1");
    let asm2 = add_assumption(dir.path(), &tree, &links[1], "interior 2");

    // Refs given out of order; engine must dedup+sort canonically.
    let projection = format!("{asm2},{asm1}");
    let (json, code) = macro_add(
        dir.path(),
        &tree,
        &macro_id,
        "Resumen causal",
        Some(&projection),
    );
    assert_eq!(code, 0, "add failed: {json:?}");
    assert!(json["success"].as_bool().unwrap());
    assert_eq!(json["action"], "macro_assume_add");
    assert_eq!(json["data"]["created_assumption_id"], "MASM-001");
    assert_eq!(json["data"]["macro_link"], macro_id);
    assert_eq!(
        string_array(&json["data"]["projection_refs"]),
        vec![asm1, asm2]
    );
    // Grounded => no ungrounded warning.
    assert!(!warning_codes(&json).contains(&"MACRO_ASSUMPTION_UNGROUNDED".to_string()));
}

// --- UAT H3: list after 2 adds => 2 items in insertion order ---
#[test]
fn uat_h3_list_after_two_adds() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, links, macro_id) = collapsed_chain(dir.path());
    let asm1 = add_assumption(dir.path(), &tree, &links[0], "interior 1");

    let (_, c1) = macro_add(dir.path(), &tree, &macro_id, "Resumen A", Some(&asm1));
    assert_eq!(c1, 0);
    // Second summary grounded on an interior LINK ref.
    let (_, c2) = macro_add(dir.path(), &tree, &macro_id, "Resumen B", Some(&links[1]));
    assert_eq!(c2, 0);

    let (json, code) = macro_list(dir.path(), &tree, &macro_id);
    assert_eq!(code, 0, "list failed: {json:?}");
    assert!(json["success"].as_bool().unwrap());
    assert_eq!(json["data"]["count"], 2);
    let asms = json["data"]["assumptions"].as_array().unwrap();
    assert_eq!(asms.len(), 2);
    assert_eq!(asms[0]["id"], "MASM-001");
    assert_eq!(asms[1]["id"], "MASM-002");
    assert_eq!(asms[0]["status"], "valid");
}

// --- UAT H4: add without projection on an empty interior => no ungrounded warning ---
#[test]
fn uat_h4_add_no_projection_empty_interior() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, links, macro_id) = collapsed_chain(dir.path());

    // Dismantle every interior link so the macro's live interior becomes empty.
    let (_, dc) = run_ltp(
        dir.path(),
        &[
            "link",
            "disconnect",
            "--tree",
            &tree,
            "--links",
            &links.join(","),
        ],
    );
    assert_eq!(dc, 0, "disconnect failed");

    let (json, code) = macro_add(dir.path(), &tree, &macro_id, "Resumen degenerado", None);
    assert_eq!(code, 0, "add failed: {json:?}");
    assert!(json["success"].as_bool().unwrap());
    assert!(json["data"]["projection_refs"]
        .as_array()
        .unwrap()
        .is_empty());
    // Empty interior => nothing to ground against => NO ungrounded warning.
    assert!(!warning_codes(&json).contains(&"MACRO_ASSUMPTION_UNGROUNDED".to_string()));
}

// --- UAT B1: add with empty text => TEXT_REQUIRED, MASM counter not consumed ---
#[test]
fn uat_b1_add_empty_text() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, links, macro_id) = collapsed_chain(dir.path());
    let asm1 = add_assumption(dir.path(), &tree, &links[0], "interior 1");

    let (json, code) = macro_add(dir.path(), &tree, &macro_id, "", Some(&asm1));
    assert_eq!(code, 1, "expected failure: {json:?}");
    assert!(!json["success"].as_bool().unwrap());
    assert_eq!(json["errors"][0]["code"], "TEXT_REQUIRED");

    // Counter must not have advanced: the next valid add is still MASM-001.
    let (json2, code2) = macro_add(dir.path(), &tree, &macro_id, "Resumen válido", Some(&asm1));
    assert_eq!(code2, 0, "valid add failed: {json2:?}");
    assert_eq!(json2["data"]["created_assumption_id"], "MASM-001");
}

// --- UAT B2: duplicate projection refs => deduped to one ---
#[test]
fn uat_b2_add_duplicate_projection() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, links, macro_id) = collapsed_chain(dir.path());
    let asm1 = add_assumption(dir.path(), &tree, &links[0], "interior 1");

    let projection = format!("{asm1},{asm1}");
    let (json, code) = macro_add(dir.path(), &tree, &macro_id, "Resumen", Some(&projection));
    assert_eq!(code, 0, "add failed: {json:?}");
    assert_eq!(string_array(&json["data"]["projection_refs"]), vec![asm1]);
}

// --- UAT B4: non-empty interior, no projection => ungrounded warning but success ---
#[test]
fn uat_b4_add_ungrounded_warns_but_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, _links, macro_id) = collapsed_chain(dir.path());

    let (json, code) = macro_add(dir.path(), &tree, &macro_id, "Resumen sin anclar", None);
    assert_eq!(code, 0, "add should still succeed: {json:?}");
    assert!(json["success"].as_bool().unwrap());
    let warnings = json["warnings"].as_array().unwrap();
    let ungrounded = warnings
        .iter()
        .find(|w| w["code"] == "MACRO_ASSUMPTION_UNGROUNDED")
        .expect("ungrounded warning expected");
    assert_eq!(ungrounded["macro_link"], macro_id);
    assert_eq!(ungrounded["assumption_id"], "MASM-001");
}

// --- UAT B5: rm the last MASM => assumptions omitted from stored JSON ---
#[test]
fn uat_b5_rm_last_masm_omits_field() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, _links, macro_id) = collapsed_chain(dir.path());

    // Add without projection (ungrounded warning is harmless) then remove it.
    let (add_json, _) = macro_add(dir.path(), &tree, &macro_id, "Resumen efímero", None);
    let masm = add_json["data"]["created_assumption_id"].as_str().unwrap();
    let (rm_json, rc) = macro_rm(dir.path(), &tree, &macro_id, masm);
    assert_eq!(rc, 0, "rm failed: {rm_json:?}");
    assert_eq!(rm_json["data"]["removed_assumption"], masm);

    // list is empty...
    let (list_json, _) = macro_list(dir.path(), &tree, &macro_id);
    assert_eq!(list_json["data"]["count"], 0);

    // ...and the stored macro_edge omits the `assumptions` key (skip_serializing_if).
    let tree_json = read_only_tree(dir.path());
    let macro_edge = tree_json["macro_edges"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["id"] == *macro_id)
        .expect("macro edge on disk");
    assert!(
        macro_edge.get("assumptions").is_none(),
        "empty assumptions must be omitted: {macro_edge:?}"
    );
}

// --- UAT C1 (add/rm variants): non-existent macro-link => MACRO_EDGE_NOT_FOUND ---
#[test]
fn uat_c1_add_rm_macro_edge_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, _links, _macro_id) = collapsed_chain(dir.path());

    let (add_json, ac) = macro_add(dir.path(), &tree, "MACRO-999", "Resumen", None);
    assert_eq!(ac, 1, "add: {add_json:?}");
    assert_eq!(add_json["errors"][0]["code"], "MACRO_EDGE_NOT_FOUND");

    let (rm_json, rc) = macro_rm(dir.path(), &tree, "MACRO-999", "MASM-001");
    assert_eq!(rc, 1, "rm: {rm_json:?}");
    assert_eq!(rm_json["errors"][0]["code"], "MACRO_EDGE_NOT_FOUND");
}

// --- UAT C3: projection ref not in interior => PROJECTION_REF_NOT_IN_INTERIOR, no mutation ---
#[test]
fn uat_c3_add_projection_not_in_interior() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, _links, macro_id) = collapsed_chain(dir.path());

    let (json, code) = macro_add(dir.path(), &tree, &macro_id, "Resumen", Some("LINK-999"));
    assert_eq!(code, 1, "expected failure: {json:?}");
    assert_eq!(json["errors"][0]["code"], "PROJECTION_REF_NOT_IN_INTERIOR");
    assert_eq!(json["errors"][0]["ref"], "LINK-999");

    // No mutation: nothing was authored.
    let (list_json, _) = macro_list(dir.path(), &tree, &macro_id);
    assert_eq!(list_json["data"]["count"], 0);
}

// --- UAT C4: projection ref on a PERIPHERAL (non-interior) link => PROJECTION_REF_NOT_IN_INTERIOR ---
#[test]
fn uat_c4_add_projection_peripheral_asm() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, nodes, links) = linear_chain(dir.path());
    // Collapse only A..D; the D->E link (links[3]) stays peripheral.
    let macro_id = collapse(dir.path(), &tree, &nodes[0], &nodes[3], "Cadena parcial");

    // Assumption on the peripheral link D->E.
    let peripheral = add_assumption(dir.path(), &tree, &links[3], "supuesto periférico");

    let (json, code) = macro_add(dir.path(), &tree, &macro_id, "Resumen", Some(&peripheral));
    assert_eq!(code, 1, "expected failure: {json:?}");
    assert_eq!(json["errors"][0]["code"], "PROJECTION_REF_NOT_IN_INTERIOR");
    assert_eq!(json["errors"][0]["ref"], peripheral);
}

// --- UAT C5: projection ref pointing at a MASM (self/lateral) => PROJECTION_REF_INVALID ---
#[test]
fn uat_c5_add_projection_masm_invalid() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, _links, macro_id) = collapsed_chain(dir.path());

    let (json, code) = macro_add(dir.path(), &tree, &macro_id, "Resumen", Some("MASM-001"));
    assert_eq!(code, 1, "expected failure: {json:?}");
    assert_eq!(json["errors"][0]["code"], "PROJECTION_REF_INVALID");
    assert_eq!(json["errors"][0]["ref"], "MASM-001");
}

// --- UAT C6: rm of a non-existent MASM => MACRO_ASSUMPTION_NOT_FOUND ---
#[test]
fn uat_c6_rm_nonexistent_masm() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, _links, macro_id) = collapsed_chain(dir.path());

    let (json, code) = macro_rm(dir.path(), &tree, &macro_id, "MASM-999");
    assert_eq!(code, 1, "expected failure: {json:?}");
    assert_eq!(json["errors"][0]["code"], "MACRO_ASSUMPTION_NOT_FOUND");
}

// ============================================================================
// Phase M4: validate integration (non-blocking warnings)
// ============================================================================

/// Run `validate --tree <id>` and return the warnings recorded for that tree.
fn validate_tree_warnings(dir: &std::path::Path, tree: &str) -> Vec<Value> {
    let (json, code) = run_ltp(dir, &["validate", "--tree", tree]);
    assert_eq!(
        code, 0,
        "validate should succeed (warnings are non-blocking): {json:?}"
    );
    json["data"]["details"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["tree_id"] == *tree)
        .map(|d| d["warnings"].as_array().unwrap().clone())
        .unwrap_or_default()
}

fn find_warning<'a>(warnings: &'a [Value], code: &str) -> &'a Value {
    warnings
        .iter()
        .find(|w| w["code"] == code)
        .unwrap_or_else(|| panic!("warning {code} not found in {warnings:?}"))
}

// --- UAT I1: projection to an interior ASM, then delete it => SUMMARY_STALE dangling=[ASM] ---
#[test]
fn uat_i1_asm_removed_makes_summary_stale() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, links, macro_id) = collapsed_chain(dir.path());
    let asm1 = add_assumption(dir.path(), &tree, &links[0], "supuesto interior");

    // Ground a summary to the interior ASM.
    let (add_json, ac) = macro_add(dir.path(), &tree, &macro_id, "Resumen", Some(&asm1));
    assert_eq!(ac, 0, "add failed: {add_json:?}");

    // Remove the interior ASM the summary depended on.
    let (rm_json, rc) = run_ltp(
        dir.path(),
        &["assume", "rm", "--tree", &tree, "--asm", &asm1],
    );
    assert_eq!(rc, 0, "assume rm failed: {rm_json:?}");

    let warnings = validate_tree_warnings(dir.path(), &tree);
    let stale = find_warning(&warnings, "LONG_ARROW_SUMMARY_STALE");
    assert_eq!(stale["macro_link"], macro_id);
    assert_eq!(stale["dangling"], serde_json::json!([asm1]));
    assert_eq!(stale["unmapped"], serde_json::json!([]));
}

// --- UAT I3: split an interior link (insert-between) + unmapped new ASM => SUMMARY_STALE ---
#[test]
fn uat_i3_split_interior_link_and_unmapped() {
    let dir = tempfile::tempdir().unwrap();
    let (tree, links, macro_id) = collapsed_chain(dir.path());

    // Summary grounded to the interior LINK L1.
    let (add_json, ac) = macro_add(dir.path(), &tree, &macro_id, "Resumen", Some(&links[0]));
    assert_eq!(ac, 0, "add failed: {add_json:?}");

    // A fresh assumption on a *surviving* interior link L2 — the summary maps nothing to it.
    let asm_new = add_assumption(dir.path(), &tree, &links[1], "supuesto nuevo");

    // Split L1 by inserting a new INT node: removes L1 (=> dangling ref).
    let x = add_node(dir.path(), "Node X", "INT");
    attach_node(dir.path(), &tree, &x);
    let (ins_json, ic) = run_ltp(
        dir.path(),
        &[
            "link",
            "insert-between",
            "--tree",
            &tree,
            "--link",
            &links[0],
            "--node",
            &x,
            "--insert-before-effect",
        ],
    );
    assert_eq!(ic, 0, "insert-between failed: {ins_json:?}");

    let warnings = validate_tree_warnings(dir.path(), &tree);
    let stale = find_warning(&warnings, "LONG_ARROW_SUMMARY_STALE");
    assert_eq!(stale["macro_link"], macro_id);
    // L1 no longer exists in the live interior => dangling.
    assert_eq!(
        string_array(&stale["dangling"]),
        vec![links[0].clone()],
        "dangling should contain the split interior link"
    );
    // The new interior ASM on L2 is not mapped by any summary => unmapped.
    assert!(
        string_array(&stale["unmapped"]).contains(&asm_new),
        "unmapped should contain the new interior ASM: {stale:?}"
    );
}
