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
    let (json, code) = run_ltp(dir, &["init", "--name", "TestK5"]);
    assert_eq!(code, 0, "init failed: {:?}", json);
}

/// Helper: create a node and return its ID.
fn add_node(dir: &std::path::Path, label: &str, ntype: &str) -> String {
    let (json, code) = run_ltp(dir, &["node", "add", label, "--type", ntype]);
    assert_eq!(code, 0, "node add failed: {:?}", json);
    json["data"]["id"].as_str().unwrap().to_string()
}

/// Helper: create a tree and return its ID.
fn add_tree(dir: &std::path::Path, ttype: &str, name: &str) -> String {
    let (json, code) = run_ltp(dir, &["tree", "new", ttype, name]);
    assert_eq!(code, 0, "tree new failed: {:?}", json);
    json["data"]["id"].as_str().unwrap().to_string()
}

/// Helper: add a knowledge item and return its ID.
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

/// Helper: link knowledge to target.
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

/// Helper: edit knowledge status.
fn edit_knowledge_status(dir: &std::path::Path, kn_id: &str, status: &str) {
    let (json, code) = run_ltp(dir, &["knowledge", "edit", kn_id, "--status", status]);
    assert_eq!(code, 0, "knowledge edit failed: {:?}", json);
}

// ========================================================================
// Status tests (K5.1 - K5.4)
// ========================================================================

#[test]
fn k5_1_status_with_knowledge_items() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    add_node(dir, "Test node", "UDE");
    add_knowledge(dir, "Evidence A", "measurement", "ERP");
    add_knowledge(dir, "Evidence B", "testimony", "Interview");

    let (json, code) = run_ltp(dir, &["status"]);
    assert_eq!(code, 0);

    let kh = &json["data"]["knowledge_health"];
    assert_eq!(kh["total"], 2);
    assert!(kh["by_status"]["unverified"].as_u64().unwrap() >= 2);
}

#[test]
fn k5_2_status_empty_knowledge_pool() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let (json, code) = run_ltp(dir, &["status"]);
    assert_eq!(code, 0);

    let kh = &json["data"]["knowledge_health"];
    assert_eq!(kh["total"], 0);
    assert_eq!(kh["unlinked_items"], 0);
    assert_eq!(kh["contradictions"], 0);
}

#[test]
fn k5_3_status_unlinked_and_contradictions() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node_id = add_node(dir, "Fact node", "UDE");
    // Set node as fact
    run_ltp(dir, &["node", "edit", &node_id, "--epistemic", "fact"]);

    // 5 items total
    let kn1 = add_knowledge(dir, "Evidence 1", "measurement", "src1");
    let kn2 = add_knowledge(dir, "Evidence 2", "measurement", "src2");
    let kn3 = add_knowledge(dir, "Contradiction", "observation", "src3");
    let _kn4 = add_knowledge(dir, "Unlinked 1", "testimony", "src4");
    let _kn5 = add_knowledge(dir, "Unlinked 2", "testimony", "src5");

    // Link some
    link_knowledge(dir, &kn1, &node_id, "supports");
    link_knowledge(dir, &kn2, &node_id, "supports");
    link_knowledge(dir, &kn3, &node_id, "contradicts");

    // Edit contradiction to verified
    edit_knowledge_status(dir, &kn3, "verified");

    let (json, code) = run_ltp(dir, &["status"]);
    assert_eq!(code, 0);

    let kh = &json["data"]["knowledge_health"];
    assert_eq!(kh["total"], 5);
    assert_eq!(kh["unlinked_items"], 2);
    assert_eq!(kh["contradictions"], 1);
}

#[test]
fn k5_4_status_epistemic_coverage() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    // 3 fact, 2 hypothesis (default), 1 assumption, 1 derived
    add_node(dir, "Node 1", "UDE"); // default = hypothesis
    add_node(dir, "Node 2", "RC"); // default = hypothesis

    let n3 = add_node(dir, "Node 3", "UDE");
    run_ltp(dir, &["node", "edit", &n3, "--epistemic", "fact"]);

    let n4 = add_node(dir, "Node 4", "UDE");
    run_ltp(dir, &["node", "edit", &n4, "--epistemic", "fact"]);

    let n5 = add_node(dir, "Node 5", "UDE");
    run_ltp(dir, &["node", "edit", &n5, "--epistemic", "fact"]);

    let n6 = add_node(dir, "Node 6", "UDE");
    run_ltp(dir, &["node", "edit", &n6, "--epistemic", "assumption"]);

    let n7 = add_node(dir, "Node 7", "UDE");
    run_ltp(dir, &["node", "edit", &n7, "--epistemic", "derived"]);

    let (json, code) = run_ltp(dir, &["status"]);
    assert_eq!(code, 0);

    let ec = &json["data"]["knowledge_health"]["epistemic_coverage"];
    assert_eq!(ec["fact"], 3);
    assert_eq!(ec["hypothesis"], 2);
    assert_eq!(ec["assumption"], 1);
    assert_eq!(ec["derived"], 1);
}

// ========================================================================
// Validate — DANGLING_KNOWLEDGE_REF (K5.5 - K5.9)
// ========================================================================

#[test]
fn k5_5_validate_dangling_ref_to_nonexistent_node() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    // Create KN and manually set a dangling link by linking then removing the node
    let node_id = add_node(dir, "Will be deleted", "UDE");
    let kn_id = add_knowledge(dir, "Evidence", "measurement", "src");
    link_knowledge(dir, &kn_id, &node_id, "supports");

    // Remove the node (creates dangling ref)
    run_ltp(dir, &["node", "rm", &node_id, "--force"]);

    let (json, code) = run_ltp(dir, &["validate"]);
    assert_eq!(code, 0); // validate returns success=true (warnings only)

    let details = json["data"]["details"].as_array().unwrap();
    let kp = details
        .iter()
        .find(|d| d["tree_id"] == "_knowledge_pool")
        .expect("knowledge_pool section missing");

    let warnings = kp["warnings"].as_array().unwrap();
    assert!(
        warnings
            .iter()
            .any(|w| w["code"] == "DANGLING_KNOWLEDGE_REF"
                && w["target"].as_str().unwrap() == node_id),
        "Expected DANGLING_KNOWLEDGE_REF warning for {}",
        node_id
    );
}

#[test]
fn k5_8_validate_no_warning_for_valid_link() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node_id = add_node(dir, "Exists", "UDE");
    let kn_id = add_knowledge(dir, "Evidence", "measurement", "src");
    link_knowledge(dir, &kn_id, &node_id, "supports");

    let (json, code) = run_ltp(dir, &["validate"]);
    assert_eq!(code, 0);

    // No knowledge pool warnings
    let details = json["data"]["details"].as_array().unwrap();
    let kp = details.iter().find(|d| d["tree_id"] == "_knowledge_pool");
    if let Some(kp) = kp {
        let warnings = kp["warnings"].as_array().unwrap();
        assert!(
            !warnings
                .iter()
                .any(|w| w["code"] == "DANGLING_KNOWLEDGE_REF"),
            "Should not have DANGLING warning for valid link"
        );
    }
}

#[test]
fn k5_9_validate_multiple_dangling_refs() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let n1 = add_node(dir, "Will die 1", "UDE");
    let n2 = add_node(dir, "Will die 2", "RC");
    let kn1 = add_knowledge(dir, "Ev1", "measurement", "s1");
    let kn2 = add_knowledge(dir, "Ev2", "measurement", "s2");
    link_knowledge(dir, &kn1, &n1, "supports");
    link_knowledge(dir, &kn2, &n2, "contradicts");

    // Remove both nodes
    run_ltp(dir, &["node", "rm", &format!("{},{}", n1, n2), "--force"]);

    let (json, _) = run_ltp(dir, &["validate"]);
    let details = json["data"]["details"].as_array().unwrap();
    let kp = details
        .iter()
        .find(|d| d["tree_id"] == "_knowledge_pool")
        .unwrap();
    let warnings = kp["warnings"].as_array().unwrap();
    let dangling_count = warnings
        .iter()
        .filter(|w| w["code"] == "DANGLING_KNOWLEDGE_REF")
        .count();
    assert_eq!(
        dangling_count, 2,
        "Should have one warning per dangling link"
    );
}

// ========================================================================
// Validate — EPISTEMIC_UNGROUNDED (K5.10 - K5.16)
// ========================================================================

#[test]
fn k5_10_validate_ungrounded_fact_no_supports() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node_id = add_node(dir, "Ungrounded fact", "UDE");
    run_ltp(dir, &["node", "edit", &node_id, "--epistemic", "fact"]);

    let (json, _) = run_ltp(dir, &["validate"]);
    let details = json["data"]["details"].as_array().unwrap();
    let kp = details
        .iter()
        .find(|d| d["tree_id"] == "_knowledge_pool")
        .unwrap();
    let warnings = kp["warnings"].as_array().unwrap();
    assert!(warnings
        .iter()
        .any(|w| w["code"] == "EPISTEMIC_UNGROUNDED" && w["node_id"].as_str().unwrap() == node_id));
}

#[test]
fn k5_11_validate_ungrounded_refuted_support_doesnt_count() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node_id = add_node(dir, "Fact", "UDE");
    run_ltp(dir, &["node", "edit", &node_id, "--epistemic", "fact"]);

    let kn = add_knowledge(dir, "Refuted support", "measurement", "src");
    link_knowledge(dir, &kn, &node_id, "supports");
    edit_knowledge_status(dir, &kn, "refuted");

    let (json, _) = run_ltp(dir, &["validate"]);
    let details = json["data"]["details"].as_array().unwrap();
    let kp = details
        .iter()
        .find(|d| d["tree_id"] == "_knowledge_pool")
        .unwrap();
    let warnings = kp["warnings"].as_array().unwrap();
    assert!(
        warnings.iter().any(|w| w["code"] == "EPISTEMIC_UNGROUNDED"),
        "Refuted support should not count as grounding"
    );
}

#[test]
fn k5_12_validate_no_ungrounded_with_verified_support() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node_id = add_node(dir, "Grounded fact", "UDE");
    run_ltp(dir, &["node", "edit", &node_id, "--epistemic", "fact"]);

    let kn = add_knowledge(dir, "Verified support", "measurement", "src");
    link_knowledge(dir, &kn, &node_id, "supports");
    edit_knowledge_status(dir, &kn, "verified");

    let (json, _) = run_ltp(dir, &["validate"]);
    let details = json["data"]["details"].as_array().unwrap();
    let kp = details.iter().find(|d| d["tree_id"] == "_knowledge_pool");
    if let Some(kp) = kp {
        let warnings = kp["warnings"].as_array().unwrap();
        assert!(
            !warnings.iter().any(|w| w["code"] == "EPISTEMIC_UNGROUNDED"
                && w["node_id"].as_str().unwrap() == node_id),
            "Verified support should count as grounding"
        );
    }
}

#[test]
fn k5_13_validate_unverified_support_counts_as_active() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node_id = add_node(dir, "Fact", "UDE");
    run_ltp(dir, &["node", "edit", &node_id, "--epistemic", "fact"]);

    // Support with default status = unverified
    let kn = add_knowledge(dir, "Unverified support", "measurement", "src");
    link_knowledge(dir, &kn, &node_id, "supports");

    let (json, _) = run_ltp(dir, &["validate"]);
    let details = json["data"]["details"].as_array().unwrap();
    let kp = details.iter().find(|d| d["tree_id"] == "_knowledge_pool");
    if let Some(kp) = kp {
        let warnings = kp["warnings"].as_array().unwrap();
        assert!(
            !warnings.iter().any(|w| w["code"] == "EPISTEMIC_UNGROUNDED"
                && w["node_id"].as_str().unwrap() == node_id),
            "Unverified support is still active grounding"
        );
    }
}

#[test]
fn k5_14_validate_contextualizes_not_support() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node_id = add_node(dir, "Fact", "UDE");
    run_ltp(dir, &["node", "edit", &node_id, "--epistemic", "fact"]);

    let kn1 = add_knowledge(dir, "Context 1", "document", "src");
    let kn2 = add_knowledge(dir, "Context 2", "document", "src");
    link_knowledge(dir, &kn1, &node_id, "contextualizes");
    link_knowledge(dir, &kn2, &node_id, "contextualizes");

    let (json, _) = run_ltp(dir, &["validate"]);
    let details = json["data"]["details"].as_array().unwrap();
    let kp = details
        .iter()
        .find(|d| d["tree_id"] == "_knowledge_pool")
        .unwrap();
    let warnings = kp["warnings"].as_array().unwrap();
    assert!(
        warnings.iter().any(|w| w["code"] == "EPISTEMIC_UNGROUNDED"),
        "Contextualizes does not count as support"
    );
}

#[test]
fn k5_15_validate_hypothesis_not_ungrounded() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    // Default epistemic = hypothesis
    add_node(dir, "Hypothesis node", "UDE");

    let (json, _) = run_ltp(dir, &["validate"]);
    let details = json["data"]["details"].as_array().unwrap();
    let kp = details.iter().find(|d| d["tree_id"] == "_knowledge_pool");
    if let Some(kp) = kp {
        let warnings = kp["warnings"].as_array().unwrap();
        assert!(
            !warnings.iter().any(|w| w["code"] == "EPISTEMIC_UNGROUNDED"),
            "UNGROUNDED only applies to fact nodes"
        );
    }
}

#[test]
fn k5_51_validate_superseded_support_doesnt_count() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node_id = add_node(dir, "Fact", "UDE");
    run_ltp(dir, &["node", "edit", &node_id, "--epistemic", "fact"]);

    let kn = add_knowledge(dir, "Superseded support", "measurement", "src");
    link_knowledge(dir, &kn, &node_id, "supports");
    edit_knowledge_status(dir, &kn, "superseded");

    let (json, _) = run_ltp(dir, &["validate"]);
    let details = json["data"]["details"].as_array().unwrap();
    let kp = details
        .iter()
        .find(|d| d["tree_id"] == "_knowledge_pool")
        .unwrap();
    let warnings = kp["warnings"].as_array().unwrap();
    assert!(
        warnings.iter().any(|w| w["code"] == "EPISTEMIC_UNGROUNDED"),
        "Superseded support should not count as grounding"
    );
}

// ========================================================================
// Validate — EPISTEMIC_CONTRADICTED (K5.17 - K5.22)
// ========================================================================

#[test]
fn k5_17_validate_contradicted_fact() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node_id = add_node(dir, "Contradicted fact", "UDE");
    run_ltp(dir, &["node", "edit", &node_id, "--epistemic", "fact"]);

    let kn_support = add_knowledge(dir, "Support", "measurement", "src");
    link_knowledge(dir, &kn_support, &node_id, "supports");
    edit_knowledge_status(dir, &kn_support, "verified");

    let kn_contra = add_knowledge(dir, "Contradiction", "observation", "src");
    link_knowledge(dir, &kn_contra, &node_id, "contradicts");
    edit_knowledge_status(dir, &kn_contra, "verified");

    let (json, _) = run_ltp(dir, &["validate"]);
    let details = json["data"]["details"].as_array().unwrap();
    let kp = details
        .iter()
        .find(|d| d["tree_id"] == "_knowledge_pool")
        .unwrap();
    let warnings = kp["warnings"].as_array().unwrap();
    assert!(warnings.iter().any(
        |w| w["code"] == "EPISTEMIC_CONTRADICTED" && w["node_id"].as_str().unwrap() == node_id
    ));
}

#[test]
fn k5_18_validate_unverified_contradiction_no_warning() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node_id = add_node(dir, "Fact", "UDE");
    run_ltp(dir, &["node", "edit", &node_id, "--epistemic", "fact"]);

    let kn_support = add_knowledge(dir, "Support", "measurement", "src");
    link_knowledge(dir, &kn_support, &node_id, "supports");

    // Unverified contradiction — should NOT trigger warning
    let kn_contra = add_knowledge(dir, "Contradiction", "observation", "src");
    link_knowledge(dir, &kn_contra, &node_id, "contradicts");

    let (json, _) = run_ltp(dir, &["validate"]);
    let details = json["data"]["details"].as_array().unwrap();
    let kp = details.iter().find(|d| d["tree_id"] == "_knowledge_pool");
    if let Some(kp) = kp {
        let warnings = kp["warnings"].as_array().unwrap();
        assert!(
            !warnings
                .iter()
                .any(|w| w["code"] == "EPISTEMIC_CONTRADICTED"
                    && w["node_id"].as_str().unwrap() == node_id),
            "Unverified contradiction should not trigger warning"
        );
    }
}

#[test]
fn k5_21_validate_hypothesis_not_contradicted() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    // Hypothesis (default), with verified contradiction
    let node_id = add_node(dir, "Hypothesis", "UDE");

    let kn = add_knowledge(dir, "Contradiction", "observation", "src");
    link_knowledge(dir, &kn, &node_id, "contradicts");
    edit_knowledge_status(dir, &kn, "verified");

    let (json, _) = run_ltp(dir, &["validate"]);
    let details = json["data"]["details"].as_array().unwrap();
    let kp = details.iter().find(|d| d["tree_id"] == "_knowledge_pool");
    if let Some(kp) = kp {
        let warnings = kp["warnings"].as_array().unwrap();
        assert!(
            !warnings
                .iter()
                .any(|w| w["code"] == "EPISTEMIC_CONTRADICTED"),
            "CONTRADICTED only applies to fact nodes"
        );
    }
}

// ========================================================================
// Validate — EPISTEMIC_UPGRADEABLE (K5.23 - K5.28)
// ========================================================================

#[test]
fn k5_23_validate_upgradeable_hypothesis() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    // Hypothesis with 2 verified supports
    let node_id = add_node(dir, "Hypothesis", "RC");

    let kn1 = add_knowledge(dir, "Support 1", "measurement", "src");
    link_knowledge(dir, &kn1, &node_id, "supports");
    edit_knowledge_status(dir, &kn1, "verified");

    let kn2 = add_knowledge(dir, "Support 2", "measurement", "src");
    link_knowledge(dir, &kn2, &node_id, "supports");
    edit_knowledge_status(dir, &kn2, "verified");

    let (json, _) = run_ltp(dir, &["validate"]);
    let details = json["data"]["details"].as_array().unwrap();
    let kp = details
        .iter()
        .find(|d| d["tree_id"] == "_knowledge_pool")
        .unwrap();
    let warnings = kp["warnings"].as_array().unwrap();
    assert!(
        warnings
            .iter()
            .any(|w| w["code"] == "EPISTEMIC_UPGRADEABLE"
                && w["node_id"].as_str().unwrap() == node_id)
    );
}

#[test]
fn k5_24_validate_not_upgradeable_one_verified() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node_id = add_node(dir, "Hypothesis", "RC");

    let kn1 = add_knowledge(dir, "Support 1 verified", "measurement", "src");
    link_knowledge(dir, &kn1, &node_id, "supports");
    edit_knowledge_status(dir, &kn1, "verified");

    let kn2 = add_knowledge(dir, "Support 2 unverified", "measurement", "src");
    link_knowledge(dir, &kn2, &node_id, "supports");

    let (json, _) = run_ltp(dir, &["validate"]);
    let details = json["data"]["details"].as_array().unwrap();
    let kp = details.iter().find(|d| d["tree_id"] == "_knowledge_pool");
    if let Some(kp) = kp {
        let warnings = kp["warnings"].as_array().unwrap();
        assert!(
            !warnings.iter().any(|w| w["code"] == "EPISTEMIC_UPGRADEABLE"
                && w["node_id"].as_str().unwrap() == node_id),
            "Needs >=2 VERIFIED supports"
        );
    }
}

#[test]
fn k5_26_validate_not_upgradeable_already_fact() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node_id = add_node(dir, "Already fact", "UDE");
    run_ltp(dir, &["node", "edit", &node_id, "--epistemic", "fact"]);

    let kn1 = add_knowledge(dir, "S1", "measurement", "src");
    link_knowledge(dir, &kn1, &node_id, "supports");
    edit_knowledge_status(dir, &kn1, "verified");

    let kn2 = add_knowledge(dir, "S2", "measurement", "src");
    link_knowledge(dir, &kn2, &node_id, "supports");
    edit_knowledge_status(dir, &kn2, "verified");

    let (json, _) = run_ltp(dir, &["validate"]);
    let details = json["data"]["details"].as_array().unwrap();
    let kp = details.iter().find(|d| d["tree_id"] == "_knowledge_pool");
    if let Some(kp) = kp {
        let warnings = kp["warnings"].as_array().unwrap();
        assert!(
            !warnings.iter().any(|w| w["code"] == "EPISTEMIC_UPGRADEABLE"
                && w["node_id"].as_str().unwrap() == node_id),
            "Already fact, nothing to upgrade"
        );
    }
}

#[test]
fn k5_27_validate_not_upgradeable_with_contradiction() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node_id = add_node(dir, "Hypothesis", "RC");

    let kn1 = add_knowledge(dir, "Support 1", "measurement", "src");
    link_knowledge(dir, &kn1, &node_id, "supports");
    edit_knowledge_status(dir, &kn1, "verified");

    let kn2 = add_knowledge(dir, "Support 2", "measurement", "src");
    link_knowledge(dir, &kn2, &node_id, "supports");
    edit_knowledge_status(dir, &kn2, "verified");

    // Add verified contradiction — should suppress UPGRADEABLE
    let kn3 = add_knowledge(dir, "Contradiction", "observation", "src");
    link_knowledge(dir, &kn3, &node_id, "contradicts");
    edit_knowledge_status(dir, &kn3, "verified");

    let (json, _) = run_ltp(dir, &["validate"]);
    let details = json["data"]["details"].as_array().unwrap();
    let kp = details.iter().find(|d| d["tree_id"] == "_knowledge_pool");
    if let Some(kp) = kp {
        let warnings = kp["warnings"].as_array().unwrap();
        assert!(
            !warnings.iter().any(|w| w["code"] == "EPISTEMIC_UPGRADEABLE"
                && w["node_id"].as_str().unwrap() == node_id),
            "Contradiction suppresses UPGRADEABLE"
        );
    }
    // If _knowledge_pool section doesn't exist, that also means no UPGRADEABLE was emitted
}

#[test]
fn k5_28_validate_upgradeable_assumption() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node_id = add_node(dir, "Assumption node", "UDE");
    run_ltp(
        dir,
        &["node", "edit", &node_id, "--epistemic", "assumption"],
    );

    let kn1 = add_knowledge(dir, "Support 1", "measurement", "src");
    link_knowledge(dir, &kn1, &node_id, "supports");
    edit_knowledge_status(dir, &kn1, "verified");

    let kn2 = add_knowledge(dir, "Support 2", "measurement", "src");
    link_knowledge(dir, &kn2, &node_id, "supports");
    edit_knowledge_status(dir, &kn2, "verified");

    let (json, _) = run_ltp(dir, &["validate"]);
    let details = json["data"]["details"].as_array().unwrap();
    let kp = details
        .iter()
        .find(|d| d["tree_id"] == "_knowledge_pool")
        .unwrap();
    let warnings = kp["warnings"].as_array().unwrap();
    assert!(
        warnings
            .iter()
            .any(|w| w["code"] == "EPISTEMIC_UPGRADEABLE"
                && w["node_id"].as_str().unwrap() == node_id),
        "UPGRADEABLE applies to assumption too"
    );
}

// ========================================================================
// Validate — combinatorics (K5.29 - K5.34)
// ========================================================================

#[test]
fn k5_29_validate_contradicted_but_not_ungrounded() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node_id = add_node(dir, "Fact", "UDE");
    run_ltp(dir, &["node", "edit", &node_id, "--epistemic", "fact"]);

    let kn_s = add_knowledge(dir, "Support", "measurement", "src");
    link_knowledge(dir, &kn_s, &node_id, "supports");
    edit_knowledge_status(dir, &kn_s, "verified");

    let kn_c = add_knowledge(dir, "Contradict", "observation", "src");
    link_knowledge(dir, &kn_c, &node_id, "contradicts");
    edit_knowledge_status(dir, &kn_c, "verified");

    let (json, _) = run_ltp(dir, &["validate"]);
    let details = json["data"]["details"].as_array().unwrap();
    let kp = details
        .iter()
        .find(|d| d["tree_id"] == "_knowledge_pool")
        .unwrap();
    let warnings = kp["warnings"].as_array().unwrap();
    assert!(warnings
        .iter()
        .any(|w| w["code"] == "EPISTEMIC_CONTRADICTED"));
    assert!(
        !warnings
            .iter()
            .any(|w| w["code"] == "EPISTEMIC_UNGROUNDED"
                && w["node_id"].as_str().unwrap() == node_id),
        "Has support, not ungrounded"
    );
}

#[test]
fn k5_30_validate_ungrounded_and_contradicted() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node_id = add_node(dir, "Fact no support", "UDE");
    run_ltp(dir, &["node", "edit", &node_id, "--epistemic", "fact"]);

    let kn_c = add_knowledge(dir, "Contradict", "observation", "src");
    link_knowledge(dir, &kn_c, &node_id, "contradicts");
    edit_knowledge_status(dir, &kn_c, "verified");

    let (json, _) = run_ltp(dir, &["validate"]);
    let details = json["data"]["details"].as_array().unwrap();
    let kp = details
        .iter()
        .find(|d| d["tree_id"] == "_knowledge_pool")
        .unwrap();
    let warnings = kp["warnings"].as_array().unwrap();
    assert!(warnings
        .iter()
        .any(|w| w["code"] == "EPISTEMIC_UNGROUNDED" && w["node_id"].as_str().unwrap() == node_id));
    assert!(warnings.iter().any(
        |w| w["code"] == "EPISTEMIC_CONTRADICTED" && w["node_id"].as_str().unwrap() == node_id
    ));
}

#[test]
fn k5_32_validate_tree_filter_only_reports_tree_nodes() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let n1 = add_node(dir, "In tree", "UDE");
    let n2 = add_node(dir, "Not in tree", "RC");
    run_ltp(dir, &["node", "edit", &n1, "--epistemic", "fact"]);
    run_ltp(dir, &["node", "edit", &n2, "--epistemic", "fact"]);

    let tree_id = add_tree(dir, "crt", "test-tree");
    run_ltp(dir, &["tree", "attach", "--tree", &tree_id, "--node", &n1]);

    let (json, _) = run_ltp(dir, &["validate", "--tree", &tree_id]);
    let details = json["data"]["details"].as_array().unwrap();
    let kp = details.iter().find(|d| d["tree_id"] == "_knowledge_pool");
    if let Some(kp) = kp {
        let warnings = kp["warnings"].as_array().unwrap();
        // n1 should appear, n2 should NOT
        let has_n2 = warnings
            .iter()
            .any(|w| w["node_id"].as_str().map(|s| s == n2).unwrap_or(false));
        assert!(
            !has_n2,
            "Node not in tree should not appear in tree-filtered validate"
        );
    }
}

#[test]
fn k5_34_validate_healthy_pool_no_warnings() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node_id = add_node(dir, "Grounded fact", "UDE");
    run_ltp(dir, &["node", "edit", &node_id, "--epistemic", "fact"]);

    let kn1 = add_knowledge(dir, "S1", "measurement", "src");
    link_knowledge(dir, &kn1, &node_id, "supports");
    edit_knowledge_status(dir, &kn1, "verified");

    let kn2 = add_knowledge(dir, "S2", "measurement", "src");
    link_knowledge(dir, &kn2, &node_id, "supports");
    edit_knowledge_status(dir, &kn2, "verified");

    let (json, _) = run_ltp(dir, &["validate"]);
    let details = json["data"]["details"].as_array().unwrap();
    let kp = details.iter().find(|d| d["tree_id"] == "_knowledge_pool");
    if let Some(kp) = kp {
        let warnings = kp["warnings"].as_array().unwrap();
        // Should only be EPISTEMIC_UPGRADEABLE (fact with 2 supports is fine)
        assert!(
            !warnings.iter().any(|w| w["code"] == "EPISTEMIC_UNGROUNDED"
                || w["code"] == "EPISTEMIC_CONTRADICTED"
                || w["code"] == "DANGLING_KNOWLEDGE_REF"),
            "Healthy pool should have no negative warnings"
        );
    }
}

// ========================================================================
// Trace — --show-knowledge (K5.35 - K5.40)
// ========================================================================

#[test]
fn k5_35_trace_show_knowledge() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let n1 = add_node(dir, "Root cause", "RC");
    let n2 = add_node(dir, "Effect", "UDE");

    let tree_id = add_tree(dir, "crt", "test-crt");
    run_ltp(dir, &["tree", "attach", "--tree", &tree_id, "--node", &n1]);
    run_ltp(dir, &["tree", "attach", "--tree", &tree_id, "--node", &n2]);
    run_ltp(
        dir,
        &[
            "link", "connect", "--tree", &tree_id, "--from", &n1, "--to", &n2,
        ],
    );

    let kn = add_knowledge(dir, "Evidence", "measurement", "src");
    link_knowledge(dir, &kn, &n2, "supports");
    edit_knowledge_status(dir, &kn, "verified");

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

    // Find the entry for n2
    let chain = json["data"]["chain"].as_array().unwrap();
    let n2_entry = chain.iter().find(|e| e["node"].as_str().unwrap() == n2);
    assert!(n2_entry.is_some(), "n2 should be in chain");

    let knowledge = n2_entry.unwrap()["knowledge"].as_array().unwrap();
    assert_eq!(knowledge.len(), 1);
    assert_eq!(knowledge[0]["id"], kn);
    assert_eq!(knowledge[0]["relation"], "supports");
    assert_eq!(knowledge[0]["status"], "verified");
}

#[test]
fn k5_36_trace_without_show_knowledge() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let n1 = add_node(dir, "Root", "RC");
    let n2 = add_node(dir, "Effect", "UDE");

    let tree_id = add_tree(dir, "crt", "test-crt");
    run_ltp(dir, &["tree", "attach", "--tree", &tree_id, "--node", &n1]);
    run_ltp(dir, &["tree", "attach", "--tree", &tree_id, "--node", &n2]);
    run_ltp(
        dir,
        &[
            "link", "connect", "--tree", &tree_id, "--from", &n1, "--to", &n2,
        ],
    );

    let kn = add_knowledge(dir, "Evidence", "measurement", "src");
    link_knowledge(dir, &kn, &n2, "supports");

    let (json, code) = run_ltp(
        dir,
        &["trace", &n2, "--tree", &tree_id, "--direction", "upstream"],
    );
    assert_eq!(code, 0);

    // Without --show-knowledge, no "knowledge" field
    let chain = json["data"]["chain"].as_array().unwrap();
    for entry in chain {
        assert!(
            entry.get("knowledge").is_none(),
            "knowledge field should be absent without --show-knowledge"
        );
    }
}

#[test]
fn k5_37_trace_show_knowledge_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let n1 = add_node(dir, "Root", "RC");
    let n2 = add_node(dir, "Effect", "UDE");

    let tree_id = add_tree(dir, "crt", "test-crt");
    run_ltp(dir, &["tree", "attach", "--tree", &tree_id, "--node", &n1]);
    run_ltp(dir, &["tree", "attach", "--tree", &tree_id, "--node", &n2]);
    run_ltp(
        dir,
        &[
            "link", "connect", "--tree", &tree_id, "--from", &n1, "--to", &n2,
        ],
    );

    // No knowledge linked to any node
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
    for entry in chain {
        let knowledge = entry["knowledge"].as_array().unwrap();
        assert_eq!(knowledge.len(), 0, "Empty array when no knowledge linked");
    }
}

// ========================================================================
// Node rm — KNOWLEDGE_ORPHANED (K5.41 - K5.44)
// ========================================================================

#[test]
fn k5_41_node_rm_knowledge_orphaned_warning() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node_id = add_node(dir, "Will be removed", "UDE");
    let kn1 = add_knowledge(dir, "Ev1", "measurement", "src");
    let kn2 = add_knowledge(dir, "Ev2", "observation", "src");
    link_knowledge(dir, &kn1, &node_id, "supports");
    link_knowledge(dir, &kn2, &node_id, "contradicts");

    let (json, code) = run_ltp(dir, &["node", "rm", &node_id, "--force"]);
    assert_eq!(code, 0);
    assert_eq!(json["success"], true);

    let warnings = json["warnings"].as_array().unwrap();
    let orphan_warning = warnings.iter().find(|w| w["code"] == "KNOWLEDGE_ORPHANED");
    assert!(
        orphan_warning.is_some(),
        "Should warn about orphaned knowledge"
    );

    let kn_ids = orphan_warning.unwrap()["knowledge_ids"].as_array().unwrap();
    assert!(kn_ids.iter().any(|id| id.as_str().unwrap() == kn1));
    assert!(kn_ids.iter().any(|id| id.as_str().unwrap() == kn2));
}

#[test]
fn k5_42_node_rm_no_knowledge_no_warning() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node_id = add_node(dir, "No knowledge linked", "UDE");

    let (json, code) = run_ltp(dir, &["node", "rm", &node_id, "--force"]);
    assert_eq!(code, 0);

    let warnings = json["warnings"].as_array().unwrap();
    assert!(
        !warnings.iter().any(|w| w["code"] == "KNOWLEDGE_ORPHANED"),
        "No warning when no knowledge is orphaned"
    );
}

#[test]
fn k5_43_node_rm_batch_multiple_orphaned() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let n1 = add_node(dir, "Node A", "UDE");
    let n2 = add_node(dir, "Node B", "RC");
    let kn1 = add_knowledge(dir, "Ev for A", "measurement", "src");
    let kn2 = add_knowledge(dir, "Ev for B", "measurement", "src");
    link_knowledge(dir, &kn1, &n1, "supports");
    link_knowledge(dir, &kn2, &n2, "supports");

    let (json, code) = run_ltp(dir, &["node", "rm", &format!("{},{}", n1, n2), "--force"]);
    assert_eq!(code, 0);

    let warnings = json["warnings"].as_array().unwrap();
    let orphan_warning = warnings
        .iter()
        .find(|w| w["code"] == "KNOWLEDGE_ORPHANED")
        .unwrap();
    let kn_ids = orphan_warning["knowledge_ids"].as_array().unwrap();
    assert_eq!(kn_ids.len(), 2);
}

#[test]
fn k5_44_node_rm_undo_resolves_dangling() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let node_id = add_node(dir, "To remove and restore", "UDE");
    let kn = add_knowledge(dir, "Evidence", "measurement", "src");
    link_knowledge(dir, &kn, &node_id, "supports");

    // Remove + undo
    run_ltp(dir, &["node", "rm", &node_id, "--force"]);
    run_ltp(dir, &["undo"]);

    // Validate should show no dangling
    let (json, _) = run_ltp(dir, &["validate"]);
    let details = json["data"]["details"].as_array().unwrap();
    let kp = details.iter().find(|d| d["tree_id"] == "_knowledge_pool");
    if let Some(kp) = kp {
        let warnings = kp["warnings"].as_array().unwrap();
        assert!(
            !warnings
                .iter()
                .any(|w| w["code"] == "DANGLING_KNOWLEDGE_REF"),
            "After undo, ref should resolve again"
        );
    }
}

// ========================================================================
// Tree walk — --show-knowledge (K5.45 - K5.47)
// ========================================================================

#[test]
fn k5_45_tree_walk_show_knowledge() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let n1 = add_node(dir, "Node A", "UDE");
    let n2 = add_node(dir, "Node B", "RC");

    let tree_id = add_tree(dir, "crt", "walk-test");
    run_ltp(dir, &["tree", "attach", "--tree", &tree_id, "--node", &n1]);
    run_ltp(dir, &["tree", "attach", "--tree", &tree_id, "--node", &n2]);
    run_ltp(
        dir,
        &[
            "link", "connect", "--tree", &tree_id, "--from", &n2, "--to", &n1,
        ],
    );

    let kn1 = add_knowledge(dir, "Support", "measurement", "src");
    let kn2 = add_knowledge(dir, "Contradict", "observation", "src");
    link_knowledge(dir, &kn1, &n1, "supports");
    link_knowledge(dir, &kn2, &n1, "contradicts");

    let (json, code) = run_ltp(dir, &["tree", "walk", &tree_id, "--show-knowledge"]);
    assert_eq!(code, 0);

    let nodes = json["data"]["nodes"].as_array().unwrap();
    let n1_entry = nodes
        .iter()
        .find(|n| n["id"].as_str().unwrap() == n1)
        .unwrap();
    let knowledge = &n1_entry["knowledge"];
    assert_eq!(knowledge["supports"], 1);
    assert_eq!(knowledge["contradicts"], 1);
    assert_eq!(knowledge["contextualizes"], 0);
}

#[test]
fn k5_46_tree_walk_without_show_knowledge() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let n1 = add_node(dir, "Node A", "UDE");
    let tree_id = add_tree(dir, "crt", "walk-test");
    run_ltp(dir, &["tree", "attach", "--tree", &tree_id, "--node", &n1]);

    let (json, code) = run_ltp(dir, &["tree", "walk", &tree_id]);
    assert_eq!(code, 0);

    let nodes = json["data"]["nodes"].as_array().unwrap();
    for node in nodes {
        assert!(
            node.get("knowledge").is_none(),
            "knowledge should be absent without flag"
        );
    }
}

#[test]
fn k5_47_tree_walk_show_knowledge_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    let n1 = add_node(dir, "No KN", "UDE");
    let tree_id = add_tree(dir, "crt", "walk-test");
    run_ltp(dir, &["tree", "attach", "--tree", &tree_id, "--node", &n1]);

    let (json, code) = run_ltp(dir, &["tree", "walk", &tree_id, "--show-knowledge"]);
    assert_eq!(code, 0);

    let nodes = json["data"]["nodes"].as_array().unwrap();
    let n1_entry = nodes
        .iter()
        .find(|n| n["id"].as_str().unwrap() == n1)
        .unwrap();
    let knowledge = &n1_entry["knowledge"];
    assert_eq!(knowledge["supports"], 0);
    assert_eq!(knowledge["contradicts"], 0);
    assert_eq!(knowledge["contextualizes"], 0);
}

// ========================================================================
// Additional (K5.48 - K5.50)
// ========================================================================

#[test]
fn k5_48_validate_no_knowledge_dir_backwards_compatible() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    // Remove knowledge/ dir to simulate pre-K1 workspace
    std::fs::remove_dir(dir.join("knowledge")).ok();

    let (json, code) = run_ltp(dir, &["validate"]);
    assert_eq!(code, 0, "Should not crash without knowledge dir");
    assert_eq!(json["success"], true);
}

#[test]
fn k5_50_status_with_corrupt_knowledge_file() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    init_workspace(dir);

    // Create a valid item
    add_knowledge(dir, "Valid", "measurement", "src");

    // Write corrupt file
    let corrupt_path = dir.join("knowledge").join("KN-999.json");
    std::fs::write(&corrupt_path, "{broken json").unwrap();

    let (json, code) = run_ltp(dir, &["status"]);
    // Should not crash; should count the valid one
    assert_eq!(code, 0);
    let kh = &json["data"]["knowledge_health"];
    assert_eq!(kh["total"], 1, "Only valid item counted");
}
