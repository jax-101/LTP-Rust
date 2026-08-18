use std::process::Command;

use serde_json::Value;

// ─── Helpers ───────────────────────────────────────────────────────────────

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
    let (json, code) = run_ltp(dir, &["init", "--name", "E2E_Knowledge"]);
    assert_eq!(code, 0, "init failed: {:?}", json);
}

fn add_node(dir: &std::path::Path, label: &str, ntype: &str) -> String {
    let (json, code) = run_ltp(dir, &["node", "add", label, "--type", ntype]);
    assert_eq!(code, 0, "node add failed: {:?}", json);
    json["data"]["id"].as_str().unwrap().to_string()
}

fn add_tree(dir: &std::path::Path, ttype: &str, name: &str) -> String {
    let (json, code) = run_ltp(dir, &["tree", "new", ttype, name]);
    assert_eq!(code, 0, "tree new failed: {:?}", json);
    json["data"]["id"].as_str().unwrap().to_string()
}

fn attach_node(dir: &std::path::Path, tree_id: &str, node_id: &str) {
    let (json, code) = run_ltp(
        dir,
        &["tree", "attach", "--tree", tree_id, "--node", node_id],
    );
    assert_eq!(code, 0, "tree attach failed: {:?}", json);
}

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

fn add_knowledge(dir: &std::path::Path, label: &str, ktype: &str, source: &str) -> String {
    let (json, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            label,
            "--type",
            ktype,
            "--source-excerpt",
            source,
        ],
    );
    assert_eq!(code, 0, "knowledge add failed: {:?}", json);
    json["data"]["created_knowledge_id"]
        .as_str()
        .unwrap()
        .to_string()
}

fn link_knowledge(dir: &std::path::Path, kn_id: &str, target: &str, relation: &str) {
    let (json, code) = run_ltp(
        dir,
        &[
            "knowledge",
            "link",
            kn_id,
            "--to",
            target,
            "--relation",
            relation,
        ],
    );
    assert_eq!(code, 0, "knowledge link failed: {:?}", json);
}

#[allow(dead_code)]
fn unlink_knowledge(dir: &std::path::Path, kn_id: &str, target: &str) {
    let (json, code) = run_ltp(dir, &["knowledge", "unlink", kn_id, "--from", target]);
    assert_eq!(code, 0, "knowledge unlink failed: {:?}", json);
}

fn edit_knowledge_status(dir: &std::path::Path, kn_id: &str, status: &str) {
    let (json, code) = run_ltp(dir, &["knowledge", "edit", kn_id, "--status", status]);
    assert_eq!(code, 0, "knowledge edit status failed: {:?}", json);
}

fn edit_node_epistemic(dir: &std::path::Path, node_id: &str, epistemic: &str) {
    let (json, code) = run_ltp(dir, &["node", "edit", node_id, "--epistemic", epistemic]);
    assert_eq!(code, 0, "node edit epistemic failed: {:?}", json);
}

fn undo(dir: &std::path::Path) {
    let (json, code) = run_ltp(dir, &["undo"]);
    assert_eq!(code, 0, "undo failed: {:?}", json);
}

fn redo(dir: &std::path::Path) {
    let (json, code) = run_ltp(dir, &["redo"]);
    assert_eq!(code, 0, "redo failed: {:?}", json);
}

fn begin_batch(dir: &std::path::Path, label: &str) {
    let (json, code) = run_ltp(dir, &["history", "begin-batch", "--label", label]);
    assert_eq!(code, 0, "begin-batch failed: {:?}", json);
}

fn end_batch(dir: &std::path::Path) {
    let (json, code) = run_ltp(dir, &["history", "end-batch"]);
    assert_eq!(code, 0, "end-batch failed: {:?}", json);
}

fn validate(dir: &std::path::Path) -> Value {
    let (json, code) = run_ltp(dir, &["validate"]);
    assert_eq!(code, 0, "validate failed: {:?}", json);
    json
}

fn get_knowledge_warnings(json: &Value) -> Vec<&Value> {
    let details = json["data"]["details"].as_array().unwrap();
    if let Some(kp) = details.iter().find(|d| d["tree_id"] == "_knowledge_pool") {
        kp["warnings"].as_array().unwrap().iter().collect()
    } else {
        vec![]
    }
}

fn has_warning(warnings: &[&Value], code: &str) -> bool {
    warnings.iter().any(|w| w["code"] == code)
}

fn has_warning_for_node(warnings: &[&Value], code: &str, node_id: &str) -> bool {
    warnings
        .iter()
        .any(|w| w["code"] == code && w["node_id"].as_str().map(|s| s == node_id).unwrap_or(false))
}

// ============================================================================
// K6.1: Hypothesis-driven cycle — full promotion
// ============================================================================

#[test]
fn k6_1_hypothesis_driven_cycle_full() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    // 1. Add knowledge as hypothesis
    let kn = add_knowledge(dir, "Lead time 18.3 days", "measurement", "ERP Q2");

    // 2. Create RC node (default epistemic = hypothesis)
    let rc = add_node(dir, "Long lead times", "RC");

    // 3. Link knowledge to node
    link_knowledge(dir, &kn, &rc, "supports");

    // 4. Verify knowledge and promote node to fact
    edit_knowledge_status(dir, &kn, "verified");
    edit_node_epistemic(dir, &rc, "fact");

    // 5. Validate should be clean (fact with verified support)
    let json = validate(dir);
    let warnings = get_knowledge_warnings(&json);
    assert!(
        !has_warning_for_node(&warnings, "EPISTEMIC_UNGROUNDED", &rc),
        "Fact with verified support should not be ungrounded"
    );
    assert!(
        !has_warning_for_node(&warnings, "EPISTEMIC_CONTRADICTED", &rc),
        "No contradiction present"
    );
}

// ============================================================================
// K6.2: Hypothesis-driven cycle — validate BEFORE promotion
// ============================================================================

#[test]
fn k6_2_hypothesis_cycle_validate_before_promotion() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let kn = add_knowledge(dir, "Evidence", "measurement", "source");
    let rc = add_node(dir, "Root cause", "RC");
    link_knowledge(dir, &kn, &rc, "supports");

    // Promote node to fact BEFORE verifying knowledge
    edit_node_epistemic(dir, &rc, "fact");

    // Validate: fact with unverified support is NOT ungrounded (unverified counts as active)
    // But if we had refuted it, it would be ungrounded.
    // Here knowledge is unverified (default) — still counts as grounding per D5.
    let json = validate(dir);
    let warnings = get_knowledge_warnings(&json);
    assert!(
        !has_warning_for_node(&warnings, "EPISTEMIC_UNGROUNDED", &rc),
        "Unverified support still counts as active grounding"
    );
}

// ============================================================================
// K6.3: Refutation cascade — support refuted leaves node ungrounded
// ============================================================================

#[test]
fn k6_3_refutation_cascade() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let kn = add_knowledge(dir, "Evidence", "measurement", "ERP");
    let node = add_node(dir, "Fact node", "UDE");
    link_knowledge(dir, &kn, &node, "supports");
    edit_node_epistemic(dir, &node, "fact");

    // Refute the evidence
    edit_knowledge_status(dir, &kn, "refuted");

    // Validate: fact with only refuted support = UNGROUNDED
    let json = validate(dir);
    let warnings = get_knowledge_warnings(&json);
    assert!(
        has_warning_for_node(&warnings, "EPISTEMIC_UNGROUNDED", &node),
        "Refuted support leaves fact ungrounded"
    );
}

// ============================================================================
// K6.4: Refutation does NOT cascade to indirectly linked nodes
// ============================================================================

#[test]
fn k6_4_refutation_no_cascade_to_chain() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    // Chain: n1 → n2 → n3 (all facts)
    let n1 = add_node(dir, "Root cause", "RC");
    let n2 = add_node(dir, "Intermediate", "INT");
    let n3 = add_node(dir, "Effect", "UDE");
    edit_node_epistemic(dir, &n1, "fact");
    edit_node_epistemic(dir, &n2, "fact");
    edit_node_epistemic(dir, &n3, "fact");

    // Knowledge supports only n1
    let kn1 = add_knowledge(dir, "Evidence for n1", "measurement", "src");
    link_knowledge(dir, &kn1, &n1, "supports");
    edit_knowledge_status(dir, &kn1, "verified");

    // n2 and n3 have their own supports
    let kn2 = add_knowledge(dir, "Evidence for n2", "measurement", "src");
    link_knowledge(dir, &kn2, &n2, "supports");
    edit_knowledge_status(dir, &kn2, "verified");

    let kn3 = add_knowledge(dir, "Evidence for n3", "measurement", "src");
    link_knowledge(dir, &kn3, &n3, "supports");
    edit_knowledge_status(dir, &kn3, "verified");

    // Refute kn1 — only n1 should be affected
    edit_knowledge_status(dir, &kn1, "refuted");

    let json = validate(dir);
    let warnings = get_knowledge_warnings(&json);
    assert!(
        has_warning_for_node(&warnings, "EPISTEMIC_UNGROUNDED", &n1),
        "n1 should be ungrounded after refutation"
    );
    assert!(
        !has_warning_for_node(&warnings, "EPISTEMIC_UNGROUNDED", &n2),
        "n2 should not be affected by n1's refutation"
    );
    assert!(
        !has_warning_for_node(&warnings, "EPISTEMIC_UNGROUNDED", &n3),
        "n3 should not be affected by n1's refutation"
    );
}

// ============================================================================
// K6.5: Inbox workflow — unlinked items drain as they get linked
// ============================================================================

#[test]
fn k6_5_inbox_workflow() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    // Add 5 unlinked items
    let kn1 = add_knowledge(dir, "Item 1", "observation", "src");
    let kn2 = add_knowledge(dir, "Item 2", "observation", "src");
    let kn3 = add_knowledge(dir, "Item 3", "observation", "src");
    let kn4 = add_knowledge(dir, "Item 4", "observation", "src");
    let kn5 = add_knowledge(dir, "Item 5", "observation", "src");

    // Verify inbox has 5
    let (json, _) = run_ltp(dir, &["knowledge", "list", "--unlinked"]);
    assert_eq!(json["data"]["total"], 5);

    // Create target nodes and link 3
    let n1 = add_node(dir, "Target 1", "UDE");
    let n2 = add_node(dir, "Target 2", "RC");
    link_knowledge(dir, &kn1, &n1, "supports");
    link_knowledge(dir, &kn2, &n1, "contradicts");
    link_knowledge(dir, &kn3, &n2, "contextualizes");

    // Verify inbox has 2
    let (json, _) = run_ltp(dir, &["knowledge", "list", "--unlinked"]);
    assert_eq!(json["data"]["total"], 2);

    // Link remaining 2
    link_knowledge(dir, &kn4, &n2, "supports");
    link_knowledge(dir, &kn5, &n2, "supports");

    // Verify inbox is empty
    let (json, _) = run_ltp(dir, &["knowledge", "list", "--unlinked"]);
    assert_eq!(json["data"]["total"], 0);
}

// ============================================================================
// K6.6: Contradiction detection — supports AND contradicts on same fact
// ============================================================================

#[test]
fn k6_6_contradiction_detection() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node = add_node(dir, "Disputed fact", "UDE");
    edit_node_epistemic(dir, &node, "fact");

    let kn_support = add_knowledge(dir, "Support evidence", "measurement", "ERP");
    link_knowledge(dir, &kn_support, &node, "supports");
    edit_knowledge_status(dir, &kn_support, "verified");

    let kn_contra = add_knowledge(dir, "Contradicting data", "observation", "field report");
    link_knowledge(dir, &kn_contra, &node, "contradicts");
    edit_knowledge_status(dir, &kn_contra, "verified");

    let json = validate(dir);
    let warnings = get_knowledge_warnings(&json);
    assert!(
        has_warning_for_node(&warnings, "EPISTEMIC_CONTRADICTED", &node),
        "Fact with verified contradiction should trigger CONTRADICTED"
    );
}

// ============================================================================
// K6.7: Contradiction on hypothesis — no warning
// ============================================================================

#[test]
fn k6_7_contradiction_on_hypothesis_no_warning() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    // Default epistemic = hypothesis
    let node = add_node(dir, "Hypothesis node", "UDE");

    let kn = add_knowledge(dir, "Contradicting data", "observation", "src");
    link_knowledge(dir, &kn, &node, "contradicts");
    edit_knowledge_status(dir, &kn, "verified");

    let json = validate(dir);
    let warnings = get_knowledge_warnings(&json);
    assert!(
        !has_warning_for_node(&warnings, "EPISTEMIC_CONTRADICTED", &node),
        "CONTRADICTED does not apply to hypothesis nodes"
    );
}

// ============================================================================
// K6.8: Multi-target link — one KN supports multiple nodes
// ============================================================================

#[test]
fn k6_8_multi_target_link() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let n1 = add_node(dir, "Effect A", "UDE");
    let n2 = add_node(dir, "Root cause B", "RC");
    let kn = add_knowledge(dir, "Cross-cutting evidence", "measurement", "src");

    link_knowledge(dir, &kn, &n1, "supports");
    link_knowledge(dir, &kn, &n2, "supports");

    let (json, code) = run_ltp(dir, &["knowledge", "inspect", &kn]);
    assert_eq!(code, 0);

    let links = json["data"]["links"].as_array().unwrap();
    assert_eq!(links.len(), 2);

    let targets: Vec<&str> = links
        .iter()
        .map(|l| l["target"].as_str().unwrap())
        .collect();
    assert!(targets.contains(&n1.as_str()));
    assert!(targets.contains(&n2.as_str()));
}

// ============================================================================
// K6.9: Same target, different relations — both visible
// ============================================================================

#[test]
fn k6_9_same_target_different_relations() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node = add_node(dir, "Complex node", "UDE");
    let kn = add_knowledge(dir, "Mixed evidence", "measurement", "src");

    link_knowledge(dir, &kn, &node, "supports");
    link_knowledge(dir, &kn, &node, "contradicts");

    let (json, code) = run_ltp(dir, &["knowledge", "inspect", &kn]);
    assert_eq!(code, 0);

    let links = json["data"]["links"].as_array().unwrap();
    assert_eq!(links.len(), 2);

    let relations: Vec<&str> = links
        .iter()
        .filter(|l| l["target"].as_str().unwrap() == node)
        .map(|l| l["relation"].as_str().unwrap())
        .collect();
    assert!(relations.contains(&"supports"));
    assert!(relations.contains(&"contradicts"));
}

// ============================================================================
// K6.10: Undo roundtrip — granular undo of link then add
// ============================================================================

#[test]
fn k6_10_undo_roundtrip_granular() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node = add_node(dir, "Target", "UDE");
    let kn = add_knowledge(dir, "Evidence", "measurement", "src");
    link_knowledge(dir, &kn, &node, "supports");

    // Undo link
    undo(dir);
    let (json, _) = run_ltp(dir, &["knowledge", "inspect", &kn]);
    let links = json["data"]["links"].as_array().unwrap();
    assert_eq!(links.len(), 0, "Link should be undone");

    // KN still exists
    assert!(dir.join(format!("knowledge/{}.json", kn)).exists());

    // Undo add
    undo(dir);
    assert!(
        !dir.join(format!("knowledge/{}.json", kn)).exists(),
        "KN should be removed after undoing add"
    );
}

// ============================================================================
// K6.11: Undo + redo restores link
// ============================================================================

#[test]
fn k6_11_undo_redo_restores_link() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node = add_node(dir, "Target", "UDE");
    let kn = add_knowledge(dir, "Evidence", "measurement", "src");
    link_knowledge(dir, &kn, &node, "supports");

    // Undo link
    undo(dir);
    let (json, _) = run_ltp(dir, &["knowledge", "inspect", &kn]);
    assert_eq!(json["data"]["links"].as_array().unwrap().len(), 0);

    // Redo link
    redo(dir);
    let (json, _) = run_ltp(dir, &["knowledge", "inspect", &kn]);
    let links = json["data"]["links"].as_array().unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0]["target"].as_str().unwrap(), node);
    assert_eq!(links[0]["relation"].as_str().unwrap(), "supports");
}

// ============================================================================
// K6.12: Batch + undo — entire batch reverted atomically
// ============================================================================

#[test]
fn k6_12_batch_undo_atomic() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node = add_node(dir, "Pre-existing node", "UDE");

    begin_batch(dir, "Knowledge setup");

    let kn = add_knowledge(dir, "Batch evidence", "measurement", "src");
    link_knowledge(dir, &kn, &node, "supports");
    edit_node_epistemic(dir, &node, "fact");

    end_batch(dir);

    // Verify state
    assert!(dir.join(format!("knowledge/{}.json", kn)).exists());
    let (json, _) = run_ltp(dir, &["node", "inspect", &node]);
    assert_eq!(json["data"]["epistemic"], "fact");

    // Single undo reverts entire batch
    undo(dir);

    assert!(
        !dir.join(format!("knowledge/{}.json", kn)).exists(),
        "KN should be gone after batch undo"
    );
    let (json, _) = run_ltp(dir, &["node", "inspect", &node]);
    // Epistemic should be back to default (hypothesis, not in JSON or shown as hypothesis)
    let epistemic = json["data"]
        .get("epistemic")
        .and_then(|v| v.as_str())
        .unwrap_or("hypothesis");
    assert_eq!(
        epistemic, "hypothesis",
        "Epistemic should revert to default"
    );
}

// ============================================================================
// K6.13: Node rm creates dangling ref → validate warns
// ============================================================================

#[test]
fn k6_13_node_rm_dangling_ref() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node = add_node(dir, "Will be removed", "UDE");
    let kn = add_knowledge(dir, "Evidence", "measurement", "src");
    link_knowledge(dir, &kn, &node, "supports");

    // Remove the node
    run_ltp(dir, &["node", "rm", &node, "--force"]);

    let json = validate(dir);
    let warnings = get_knowledge_warnings(&json);
    assert!(
        warnings.iter().any(|w| w["code"] == "DANGLING_KNOWLEDGE_REF"
            && w["target"].as_str().unwrap() == node),
        "Should have DANGLING_KNOWLEDGE_REF after node removal"
    );
}

// ============================================================================
// K6.14: Node rm → undo restores ref → validate clean
// ============================================================================

#[test]
fn k6_14_node_rm_undo_restores_ref() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node = add_node(dir, "Will be removed then restored", "UDE");
    let kn = add_knowledge(dir, "Evidence", "measurement", "src");
    link_knowledge(dir, &kn, &node, "supports");

    run_ltp(dir, &["node", "rm", &node, "--force"]);

    // Undo restores the node
    undo(dir);

    let json = validate(dir);
    let warnings = get_knowledge_warnings(&json);
    assert!(
        !warnings.iter().any(|w| w["code"] == "DANGLING_KNOWLEDGE_REF"
            && w["target"].as_str().unwrap() == node),
        "After undo, ref is no longer dangling"
    );
}

// ============================================================================
// K6.15: Backwards compat — nodes without epistemic treated as hypothesis
// ============================================================================

#[test]
fn k6_15_backwards_compat_default_hypothesis() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    // Create nodes without explicit epistemic (default = hypothesis)
    let n1 = add_node(dir, "Legacy node 1", "UDE");
    let n2 = add_node(dir, "Legacy node 2", "RC");

    // List with --epistemic hypothesis should include them
    let tree_id = add_tree(dir, "crt", "test");
    attach_node(dir, &tree_id, &n1);
    attach_node(dir, &tree_id, &n2);

    let (json, code) = run_ltp(
        dir,
        &[
            "node",
            "list",
            "--tree",
            &tree_id,
            "--epistemic",
            "hypothesis",
        ],
    );
    assert_eq!(code, 0);

    let nodes = json["data"]["nodes"].as_array().unwrap();
    let ids: Vec<&str> = nodes.iter().map(|n| n["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&n1.as_str()));
    assert!(ids.contains(&n2.as_str()));
}

// ============================================================================
// K6.16: Backwards compat — no epistemic warnings for hypothesis nodes
// ============================================================================

#[test]
fn k6_16_backwards_compat_no_warnings() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    // Nodes without epistemic (default hypothesis)
    add_node(dir, "Node without epistemic 1", "UDE");
    add_node(dir, "Node without epistemic 2", "RC");

    let json = validate(dir);
    let warnings = get_knowledge_warnings(&json);
    assert!(
        !has_warning(&warnings, "EPISTEMIC_UNGROUNDED"),
        "Hypothesis nodes don't need grounding"
    );
    assert!(
        !has_warning(&warnings, "EPISTEMIC_CONTRADICTED"),
        "Hypothesis nodes can't be contradicted by this check"
    );
}

// ============================================================================
// K6.17: Trace with --show-knowledge across a chain
// ============================================================================

#[test]
fn k6_17_trace_show_knowledge_chain() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let rc = add_node(dir, "Root cause", "RC");
    let int = add_node(dir, "Intermediate", "INT");
    let ude = add_node(dir, "Effect", "UDE");

    let tree_id = add_tree(dir, "crt", "trace-test");
    attach_node(dir, &tree_id, &rc);
    attach_node(dir, &tree_id, &int);
    attach_node(dir, &tree_id, &ude);
    connect_nodes(dir, &tree_id, &rc, &int);
    connect_nodes(dir, &tree_id, &int, &ude);

    // Link knowledge to each node
    let kn_rc = add_knowledge(dir, "RC evidence", "measurement", "src");
    link_knowledge(dir, &kn_rc, &rc, "supports");
    edit_knowledge_status(dir, &kn_rc, "verified");

    let kn_int = add_knowledge(dir, "INT context", "document", "src");
    link_knowledge(dir, &kn_int, &int, "contextualizes");

    let kn_ude = add_knowledge(dir, "UDE observation", "observation", "src");
    link_knowledge(dir, &kn_ude, &ude, "supports");

    let (json, code) = run_ltp(
        dir,
        &[
            "trace",
            &ude,
            "--tree",
            &tree_id,
            "--direction",
            "upstream",
            "--show-knowledge",
        ],
    );
    assert_eq!(code, 0);

    let chain = json["data"]["chain"].as_array().unwrap();

    // Each node in chain should have knowledge array
    for entry in chain {
        let node_id = entry["node"].as_str().unwrap();
        let knowledge = entry["knowledge"].as_array().unwrap();

        if node_id == rc {
            assert_eq!(knowledge.len(), 1);
            assert_eq!(knowledge[0]["id"], kn_rc);
            assert_eq!(knowledge[0]["relation"], "supports");
        } else if node_id == int {
            assert_eq!(knowledge.len(), 1);
            assert_eq!(knowledge[0]["id"], kn_int);
            assert_eq!(knowledge[0]["relation"], "contextualizes");
        } else if node_id == ude {
            assert_eq!(knowledge.len(), 1);
            assert_eq!(knowledge[0]["id"], kn_ude);
            assert_eq!(knowledge[0]["relation"], "supports");
        }
    }
}

// ============================================================================
// K6.18: Trace — knowledge linked to edge does NOT appear on node
// ============================================================================

#[test]
fn k6_18_trace_knowledge_on_edge_not_on_node() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let n1 = add_node(dir, "Cause", "RC");
    let n2 = add_node(dir, "Effect", "UDE");

    let tree_id = add_tree(dir, "crt", "edge-knowledge");
    attach_node(dir, &tree_id, &n1);
    attach_node(dir, &tree_id, &n2);
    let link_id = connect_nodes(dir, &tree_id, &n1, &n2);

    // Link knowledge to the EDGE, not to a node
    let kn = add_knowledge(dir, "Edge evidence", "measurement", "src");
    link_knowledge(dir, &kn, &link_id, "supports");

    let (json, code) = run_ltp(
        dir,
        &[
            "trace",
            &n2,
            "--tree",
            &tree_id,
            "--direction",
            "upstream",
            "--show-knowledge",
        ],
    );
    assert_eq!(code, 0);

    let chain = json["data"]["chain"].as_array().unwrap();
    // Knowledge linked to the edge should NOT appear in node's knowledge array
    for entry in chain {
        let knowledge = entry["knowledge"].as_array().unwrap();
        let has_edge_kn = knowledge.iter().any(|k| k["id"].as_str().unwrap() == kn);
        assert!(
            !has_edge_kn,
            "Knowledge linked to edge should not appear on node in trace"
        );
    }
}

// ============================================================================
// K6.19: Invalidate assumption — KN link still valid
// ============================================================================

#[test]
fn k6_19_invalidate_assumption_kn_link_valid() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let n1 = add_node(dir, "Cause", "RC");
    let n2 = add_node(dir, "Effect", "UDE");
    let tree_id = add_tree(dir, "crt", "inv-test");
    attach_node(dir, &tree_id, &n1);
    attach_node(dir, &tree_id, &n2);
    let link_id = connect_nodes(dir, &tree_id, &n1, &n2);

    // Add assumption to the link
    let (json, code) = run_ltp(
        dir,
        &[
            "assume",
            "add",
            "--tree",
            &tree_id,
            "--link",
            &link_id,
            "--text",
            "Cause is always present",
        ],
    );
    assert_eq!(code, 0, "assume add failed: {:?}", json);
    let asm_id = json["data"]["id"].as_str().unwrap().to_string();

    // Link knowledge to the assumption
    let kn = add_knowledge(dir, "Assumption context", "document", "src");
    link_knowledge(dir, &kn, &asm_id, "contextualizes");

    // Invalidate the assumption
    run_ltp(
        dir,
        &[
            "invalidate",
            "--tree",
            &tree_id,
            "--link",
            &link_id,
            "--asm",
            &asm_id,
        ],
    );

    // Validate: KN link to ASM should NOT be dangling (ASM still exists, just invalidated)
    let json = validate(dir);
    let warnings = get_knowledge_warnings(&json);
    assert!(
        !warnings
            .iter()
            .any(|w| w["code"] == "DANGLING_KNOWLEDGE_REF"
                && w["target"].as_str().map(|s| s == asm_id).unwrap_or(false)),
        "Invalidated assumption still exists, no DANGLING"
    );
}

// ============================================================================
// K6.20: Path collapse — KN linked to interior node is NOT dangling
// ============================================================================

#[test]
fn k6_20_path_collapse_interior_not_dangling() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let a = add_node(dir, "Node A", "RC");
    let b = add_node(dir, "Node B", "INT");
    let c = add_node(dir, "Node C", "INT");
    let d = add_node(dir, "Node D", "UDE");

    let tree_id = add_tree(dir, "crt", "collapse-test");
    attach_node(dir, &tree_id, &a);
    attach_node(dir, &tree_id, &b);
    attach_node(dir, &tree_id, &c);
    attach_node(dir, &tree_id, &d);
    connect_nodes(dir, &tree_id, &a, &b);
    connect_nodes(dir, &tree_id, &b, &c);
    connect_nodes(dir, &tree_id, &c, &d);

    // Link knowledge to interior node B
    let kn = add_knowledge(dir, "Evidence for B", "measurement", "src");
    link_knowledge(dir, &kn, &b, "supports");

    // Collapse path from A to D
    let (json, code) = run_ltp(
        dir,
        &[
            "path",
            "collapse",
            "--tree",
            &tree_id,
            "--from",
            &a,
            "--to",
            &d,
            "--label",
            "Collapsed chain",
        ],
    );
    assert_eq!(code, 0, "path collapse failed: {:?}", json);

    // Validate: B still exists in pool, link not dangling
    let json = validate(dir);
    let warnings = get_knowledge_warnings(&json);
    assert!(
        !warnings
            .iter()
            .any(|w| w["code"] == "DANGLING_KNOWLEDGE_REF"
                && w["target"].as_str().map(|s| s == b).unwrap_or(false)),
        "Interior node B still exists in pool, no DANGLING"
    );
}

// ============================================================================
// K6.21: Path explode — new node gets default hypothesis, no warnings
// ============================================================================

#[test]
fn k6_21_path_explode_new_node_default_epistemic() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let a = add_node(dir, "Node A", "RC");
    let b = add_node(dir, "Node B", "UDE");

    let tree_id = add_tree(dir, "crt", "explode-test");
    attach_node(dir, &tree_id, &a);
    attach_node(dir, &tree_id, &b);
    let link_id = connect_nodes(dir, &tree_id, &a, &b);

    // Add assumption to the link (required for explode)
    let (json, code) = run_ltp(
        dir,
        &[
            "assume",
            "add",
            "--tree",
            &tree_id,
            "--link",
            &link_id,
            "--text",
            "Some assumption",
        ],
    );
    assert_eq!(code, 0, "assume add failed: {:?}", json);
    let asm_id = json["data"]["id"].as_str().unwrap().to_string();

    // Explode: creates intermediate node
    let (json, code) = run_ltp(
        dir,
        &[
            "path",
            "explode",
            "--tree",
            &tree_id,
            "--link",
            &link_id,
            "--asm",
            &asm_id,
            "--label",
            "New intermediate",
        ],
    );
    assert_eq!(code, 0, "path explode failed: {:?}", json);

    let new_node_id = json["data"]["created_node_id"].as_str().unwrap();

    // Validate: new intermediate node has default hypothesis, no UNGROUNDED
    let json = validate(dir);
    let warnings = get_knowledge_warnings(&json);
    assert!(
        !has_warning_for_node(&warnings, "EPISTEMIC_UNGROUNDED", new_node_id),
        "New intermediate node (default hypothesis) should not trigger UNGROUNDED"
    );
}

// ============================================================================
// K6.22: Node split — KN linked to original becomes dangling
// ============================================================================

#[test]
fn k6_22_node_split_creates_dangling() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let n1 = add_node(dir, "Cause", "RC");
    let n_target = add_node(dir, "Mixed idea A and B", "INT");
    let n3 = add_node(dir, "Effect", "UDE");

    let tree_id = add_tree(dir, "crt", "split-test");
    attach_node(dir, &tree_id, &n1);
    attach_node(dir, &tree_id, &n_target);
    attach_node(dir, &tree_id, &n3);
    connect_nodes(dir, &tree_id, &n1, &n_target);
    connect_nodes(dir, &tree_id, &n_target, &n3);

    // Link knowledge to the node that will be split
    let kn = add_knowledge(dir, "Evidence for mixed node", "measurement", "src");
    link_knowledge(dir, &kn, &n_target, "supports");

    // Split the node
    let (json, code) = run_ltp(
        dir,
        &[
            "node", "split", &n_target, "--into", "Idea A", "Idea B", "--tree", &tree_id,
        ],
    );
    assert_eq!(code, 0, "node split failed: {:?}", json);

    // Validate: original node ID no longer exists → DANGLING
    let json = validate(dir);
    let warnings = get_knowledge_warnings(&json);
    assert!(
        warnings
            .iter()
            .any(|w| w["code"] == "DANGLING_KNOWLEDGE_REF"
                && w["target"].as_str().map(|s| s == n_target).unwrap_or(false)),
        "Split removes original node, creating dangling ref"
    );
}

// ============================================================================
// K6.23: Tree rm — nodes stay, edges vanish → DANGLING for edge-linked KN
// ============================================================================

#[test]
fn k6_23_tree_rm_edges_dangling() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let n1 = add_node(dir, "Cause", "RC");
    let n2 = add_node(dir, "Effect", "UDE");

    let tree_id = add_tree(dir, "crt", "doomed-tree");
    attach_node(dir, &tree_id, &n1);
    attach_node(dir, &tree_id, &n2);
    let link_id = connect_nodes(dir, &tree_id, &n1, &n2);

    // Link KN to the edge and to a node
    let kn_edge = add_knowledge(dir, "Edge evidence", "measurement", "src");
    link_knowledge(dir, &kn_edge, &link_id, "supports");

    let kn_node = add_knowledge(dir, "Node evidence", "measurement", "src");
    link_knowledge(dir, &kn_node, &n1, "supports");

    // Remove the tree
    let (json, code) = run_ltp(dir, &["tree", "rm", &tree_id]);
    assert_eq!(code, 0, "tree rm failed: {:?}", json);

    // Validate
    let json = validate(dir);
    let warnings = get_knowledge_warnings(&json);

    // Edge-linked KN should be dangling (edge gone with tree)
    assert!(
        warnings
            .iter()
            .any(|w| w["code"] == "DANGLING_KNOWLEDGE_REF"
                && w["target"].as_str().map(|s| s == link_id).unwrap_or(false)),
        "Edge removed with tree, KN link is dangling"
    );

    // Node-linked KN should NOT be dangling (nodes stay in pool)
    assert!(
        !warnings
            .iter()
            .any(|w| w["code"] == "DANGLING_KNOWLEDGE_REF"
                && w["target"].as_str().map(|s| s == n1).unwrap_or(false)),
        "Node still in pool, not dangling"
    );
}

// ============================================================================
// K6.24: Link group — old edge IDs vanish → DANGLING for linked KN
// ============================================================================

#[test]
fn k6_24_link_group_old_edges_dangling() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let a = add_node(dir, "Cause A", "RC");
    let b = add_node(dir, "Cause B", "RC");
    let c = add_node(dir, "Effect C", "UDE");

    let tree_id = add_tree(dir, "crt", "group-test");
    attach_node(dir, &tree_id, &a);
    attach_node(dir, &tree_id, &b);
    attach_node(dir, &tree_id, &c);
    let l1 = connect_nodes(dir, &tree_id, &a, &c);
    let l2 = connect_nodes(dir, &tree_id, &b, &c);

    // Link KN to one of the edges that will be grouped
    let kn = add_knowledge(dir, "Edge evidence", "measurement", "src");
    link_knowledge(dir, &kn, &l1, "supports");

    // Group the edges
    let (json, code) = run_ltp(
        dir,
        &[
            "link",
            "group",
            "--tree",
            &tree_id,
            "--links",
            &format!("{},{}", l1, l2),
            "--operator",
            "AND",
        ],
    );
    assert_eq!(code, 0, "link group failed: {:?}", json);

    // Validate: old edge l1 no longer exists
    let json = validate(dir);
    let warnings = get_knowledge_warnings(&json);
    assert!(
        warnings
            .iter()
            .any(|w| w["code"] == "DANGLING_KNOWLEDGE_REF"
                && w["target"].as_str().map(|s| s == l1).unwrap_or(false)),
        "Grouped edge ID removed, KN link is dangling"
    );
}

// ============================================================================
// K6.25: Link dissolve restores old edges → DANGLING resolved
// ============================================================================

#[test]
fn k6_25_link_dissolve_restores_edges() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let a = add_node(dir, "Cause A", "RC");
    let b = add_node(dir, "Cause B", "RC");
    let c = add_node(dir, "Effect C", "UDE");

    let tree_id = add_tree(dir, "crt", "dissolve-test");
    attach_node(dir, &tree_id, &a);
    attach_node(dir, &tree_id, &b);
    attach_node(dir, &tree_id, &c);

    // Create AND edge directly
    let (json, code) = run_ltp(
        dir,
        &[
            "link",
            "connect",
            "--tree",
            &tree_id,
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

    // Link KN to the grouped edge
    let kn = add_knowledge(dir, "Group edge evidence", "measurement", "src");
    link_knowledge(dir, &kn, &group_link, "supports");

    // Dissolve the group
    let (json, code) = run_ltp(
        dir,
        &[
            "link",
            "dissolve",
            "--tree",
            &tree_id,
            "--link",
            &group_link,
        ],
    );
    assert_eq!(code, 0, "link dissolve failed: {:?}", json);

    // Validate: the grouped link ID no longer exists → DANGLING
    let json = validate(dir);
    let warnings = get_knowledge_warnings(&json);
    assert!(
        warnings
            .iter()
            .any(|w| w["code"] == "DANGLING_KNOWLEDGE_REF"
                && w["target"]
                    .as_str()
                    .map(|s| s == group_link)
                    .unwrap_or(false)),
        "Dissolved group link ID removed, KN link is dangling"
    );
}

// ============================================================================
// K6.26: NBR rm — edge removed → DANGLING for linked KN
// ============================================================================

#[test]
fn k6_26_nbr_rm_edge_dangling() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    // Setup FRT with injection
    let inj = add_node(dir, "Injection", "INJ");
    let de = add_node(dir, "Desired Effect", "DE");
    let tree_id = add_tree(dir, "frt", "nbr-test");
    attach_node(dir, &tree_id, &inj);
    attach_node(dir, &tree_id, &de);
    connect_nodes(dir, &tree_id, &inj, &de);

    // Add NBR
    let (json, _) = run_ltp(
        dir,
        &["nbr", "add", "--tree", &tree_id, "--source-node", &inj],
    );
    let nbr_id = json["data"]["nbr_id"].as_str().unwrap().to_string();

    // Add edge within NBR
    let nde = add_node(dir, "Negative effect", "UDE");
    let (json, code) = run_ltp(
        dir,
        &[
            "link", "connect", "--tree", &tree_id, "--nbr", &nbr_id, "--from", &inj, "--to", &nde,
        ],
    );
    assert_eq!(code, 0);
    let nbr_link = json["data"]["created_links"][0]
        .as_str()
        .unwrap()
        .to_string();

    // Link KN to the NBR edge
    let kn = add_knowledge(dir, "NBR edge evidence", "measurement", "src");
    link_knowledge(dir, &kn, &nbr_link, "supports");

    // Remove NBR
    let (json, code) = run_ltp(dir, &["nbr", "rm", "--tree", &tree_id, "--nbr", &nbr_id]);
    assert_eq!(code, 0, "nbr rm failed: {:?}", json);

    // Validate: NBR edge gone → DANGLING
    let json = validate(dir);
    let warnings = get_knowledge_warnings(&json);
    assert!(
        warnings
            .iter()
            .any(|w| w["code"] == "DANGLING_KNOWLEDGE_REF"
                && w["target"].as_str().map(|s| s == nbr_link).unwrap_or(false)),
        "NBR edge removed, KN link is dangling"
    );
}

// ============================================================================
// K6.27: Status coherence — knowledge_health always reflects current state
// ============================================================================

#[test]
fn k6_27_status_coherence() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    // Start with 0
    let (json, _) = run_ltp(dir, &["status"]);
    assert_eq!(json["data"]["knowledge_health"]["total"], 0);

    // Add items
    let kn1 = add_knowledge(dir, "Item 1", "measurement", "src");
    let (json, _) = run_ltp(dir, &["status"]);
    assert_eq!(json["data"]["knowledge_health"]["total"], 1);
    assert_eq!(json["data"]["knowledge_health"]["unlinked_items"], 1);

    let _kn2 = add_knowledge(dir, "Item 2", "observation", "src");
    let (json, _) = run_ltp(dir, &["status"]);
    assert_eq!(json["data"]["knowledge_health"]["total"], 2);
    assert_eq!(json["data"]["knowledge_health"]["unlinked_items"], 2);

    // Link one
    let node = add_node(dir, "Target", "UDE");
    link_knowledge(dir, &kn1, &node, "supports");
    let (json, _) = run_ltp(dir, &["status"]);
    assert_eq!(json["data"]["knowledge_health"]["total"], 2);
    assert_eq!(json["data"]["knowledge_health"]["unlinked_items"], 1);

    // Remove one
    run_ltp(dir, &["knowledge", "rm", &kn1]);
    let (json, _) = run_ltp(dir, &["status"]);
    assert_eq!(json["data"]["knowledge_health"]["total"], 1);
}

// ============================================================================
// K6.28: Status coherence — by_status reflects edits
// ============================================================================

#[test]
fn k6_28_status_coherence_by_status() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let kn = add_knowledge(dir, "Item", "measurement", "src");

    // Default = unverified
    let (json, _) = run_ltp(dir, &["status"]);
    assert_eq!(
        json["data"]["knowledge_health"]["by_status"]["unverified"],
        1
    );

    // Change to superseded
    edit_knowledge_status(dir, &kn, "superseded");
    let (json, _) = run_ltp(dir, &["status"]);
    assert_eq!(
        json["data"]["knowledge_health"]["by_status"]["superseded"],
        1
    );
    assert_eq!(
        json["data"]["knowledge_health"]["by_status"]["unverified"],
        0
    );
}

// ============================================================================
// K6.29: Link disconnect — LINK-ID destroyed → DANGLING for linked KN
// ============================================================================

#[test]
fn k6_29_link_disconnect_dangling() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let a = add_node(dir, "Node A", "RC");
    let b = add_node(dir, "Node B", "UDE");

    let tree_id = add_tree(dir, "crt", "disconnect-test");
    attach_node(dir, &tree_id, &a);
    attach_node(dir, &tree_id, &b);
    let link_ab = connect_nodes(dir, &tree_id, &a, &b);

    // Link KN to the edge
    let kn = add_knowledge(dir, "Edge evidence", "measurement", "src");
    link_knowledge(dir, &kn, &link_ab, "supports");

    // Disconnect the edge — this destroys the LINK-ID
    let (json, code) = run_ltp(
        dir,
        &[
            "link",
            "disconnect",
            "--tree",
            &tree_id,
            "--links",
            &link_ab,
        ],
    );
    assert_eq!(code, 0, "link disconnect failed: {:?}", json);

    // Validate: link_ab was removed → DANGLING
    let json = validate(dir);
    let warnings = get_knowledge_warnings(&json);
    assert!(
        warnings
            .iter()
            .any(|w| w["code"] == "DANGLING_KNOWLEDGE_REF"
                && w["target"].as_str().map(|s| s == link_ab).unwrap_or(false)),
        "Disconnected edge removed, KN link is dangling"
    );
}

// ============================================================================
// K6.30: Batch with failed op — only successful ops revert
// ============================================================================

#[test]
fn k6_30_batch_with_failed_op() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    begin_batch(dir, "Mixed batch");

    // This should FAIL (no source)
    let (json, code) = run_ltp(
        dir,
        &["knowledge", "add", "No source", "--type", "measurement"],
    );
    assert_ne!(code, 0, "Should fail without source: {:?}", json);

    // This should succeed
    let kn = add_knowledge(dir, "With source", "measurement", "valid source");

    end_batch(dir);

    // Verify successful op exists
    assert!(dir.join(format!("knowledge/{}.json", kn)).exists());

    // Undo batch reverts only the successful operation
    undo(dir);
    assert!(
        !dir.join(format!("knowledge/{}.json", kn)).exists(),
        "Successful op in batch should be reverted by undo"
    );
}

// ============================================================================
// K6.31: Superseded KN with contradicts — no CONTRADICTED warning
// ============================================================================

#[test]
fn k6_31_superseded_contradiction_no_warning() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node = add_node(dir, "Fact node", "UDE");
    edit_node_epistemic(dir, &node, "fact");

    // Add support to avoid UNGROUNDED
    let kn_support = add_knowledge(dir, "Support", "measurement", "src");
    link_knowledge(dir, &kn_support, &node, "supports");
    edit_knowledge_status(dir, &kn_support, "verified");

    // Add contradiction but mark it as superseded
    let kn_contra = add_knowledge(dir, "Old contradiction", "observation", "src");
    link_knowledge(dir, &kn_contra, &node, "contradicts");
    edit_knowledge_status(dir, &kn_contra, "superseded");

    let json = validate(dir);
    let warnings = get_knowledge_warnings(&json);
    assert!(
        !has_warning_for_node(&warnings, "EPISTEMIC_CONTRADICTED", &node),
        "Superseded contradiction should NOT trigger CONTRADICTED warning"
    );
}
