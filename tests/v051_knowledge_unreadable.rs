//! v0.5.1 (ADR-016 adenda D-K5) — an unreadable knowledge item is never
//! silently dropped: every reader emits `KNOWLEDGE_LOAD_ERROR {id}` (the code
//! `knowledge list` already used), one per unreadable file, in ID order.
//! Views that did not ask for knowledge stay byte-identical.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::{json, Value};

// --- harness -----------------------------------------------------------------

fn run_raw(dir: &Path, args: &[&str]) -> (String, i32) {
    let output = Command::new(env!("CARGO_BIN_EXE_ltp"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("failed to execute ltp binary");
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        output.status.code().unwrap_or(-1),
    )
}

fn run_ltp(dir: &Path, args: &[&str]) -> (Value, i32) {
    let (stdout, code) = run_raw(dir, args);
    let json: Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_| panic!("Failed to parse JSON.\nargs: {args:?}\nstdout: {stdout}"));
    (json, code)
}

fn run_ok(dir: &Path, args: &[&str]) -> Value {
    let (json, code) = run_ltp(dir, args);
    assert_eq!(code, 0, "ltp {args:?} failed: {json}");
    json
}

/// CommandOutput returned by an MCP tool call.
fn mcp_call(dir: &Path, tool: &str, arguments: Value) -> Value {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ltp-mcp"))
        .arg("--workspace")
        .arg(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn ltp-mcp");
    let request = json!({
        "jsonrpc": "2.0", "id": 1,
        "method": "tools/call",
        "params": { "name": tool, "arguments": arguments }
    });
    let mut stdin = child.stdin.take().unwrap();
    writeln!(stdin, "{request}").unwrap();
    drop(stdin);
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.lines().find(|l| !l.trim().is_empty()).unwrap();
    let response: Value = serde_json::from_str(line).unwrap();
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("no tool result: {response}"));
    serde_json::from_str(text).unwrap()
}

/// Warnings with the given code, in output order.
fn warnings_with<'a>(warnings: &'a Value, code: &str) -> Vec<&'a Value> {
    warnings
        .as_array()
        .unwrap()
        .iter()
        .filter(|w| w["code"] == code)
        .collect()
}

/// IDs carried by every `KNOWLEDGE_LOAD_ERROR` in `warnings`, in order.
fn load_error_ids(warnings: &Value) -> Vec<String> {
    warnings_with(warnings, "KNOWLEDGE_LOAD_ERROR")
        .iter()
        .map(|w| w["id"].as_str().unwrap().to_string())
        .collect()
}

fn warning_codes(warnings: &Value) -> Vec<String> {
    warnings
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["code"].as_str().unwrap().to_string())
        .collect()
}

/// `_knowledge_pool` warnings of a `validate` output (empty if the entry is absent).
fn pool_warnings(validate: &Value) -> Value {
    validate["data"]["details"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["tree_id"] == "_knowledge_pool")
        .map(|d| d["warnings"].clone())
        .unwrap_or_else(|| json!([]))
}

fn add_knowledge(dir: &Path, label: &str) -> String {
    let json = run_ok(
        dir,
        &[
            "knowledge",
            "add",
            label,
            "--type",
            "observation",
            "--source-excerpt",
            "src",
        ],
    );
    json["data"]["created_knowledge_id"]
        .as_str()
        .unwrap()
        .to_string()
}

fn link(dir: &Path, kn: &str, target: &str, relation: &str) {
    run_ok(
        dir,
        &[
            "knowledge",
            "link",
            kn,
            "--to",
            target,
            "--relation",
            relation,
        ],
    );
}

fn corrupt(dir: &Path, kn: &str) {
    fs::write(dir.join("knowledge").join(format!("{kn}.json")), "{broken").unwrap();
}

/// Fixture: RC-001 → UDE-001 in a CRT; KN-001 supports UDE-001 (verified),
/// KN-002 contextualizes RC-001, KN-003 unlinked, KN-004 linked to UDE-001
/// and then corrupted. Returns the tree ID.
fn fixture(dir: &Path) -> String {
    run_ok(dir, &["init", "--name", "KL"]);
    run_ok(dir, &["node", "add", "Effect", "--type", "UDE"]);
    run_ok(dir, &["node", "add", "Cause", "--type", "RC"]);
    let tree = run_ok(dir, &["tree", "new", "crt", "kl"])["data"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    for node in ["UDE-001", "RC-001"] {
        run_ok(dir, &["tree", "attach", "--tree", &tree, "--node", node]);
    }
    run_ok(
        dir,
        &[
            "link", "connect", "--tree", &tree, "--from", "RC-001", "--to", "UDE-001",
        ],
    );
    for label in ["One", "Two", "Three", "Four"] {
        add_knowledge(dir, label);
    }
    link(dir, "KN-001", "UDE-001", "supports");
    run_ok(
        dir,
        &["knowledge", "edit", "KN-001", "--status", "verified"],
    );
    link(dir, "KN-002", "RC-001", "contextualizes");
    link(dir, "KN-004", "UDE-001", "supports");
    tree
}

fn corrupt_fixture(dir: &Path) -> String {
    let tree = fixture(dir);
    corrupt(dir, "KN-004");
    tree
}

// --- KL1 / KL2: status ---------------------------------------------------------

#[test]
fn kl1_status_warns_and_counts_only_readable() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    corrupt_fixture(dir);

    let json = run_ok(dir, &["status"]);
    assert_eq!(json["success"], true);
    assert_eq!(load_error_ids(&json["warnings"]), vec!["KN-004"]);
    let w = warnings_with(&json["warnings"], "KNOWLEDGE_LOAD_ERROR")[0];
    assert!(
        w["detail"]
            .as_str()
            .unwrap()
            .starts_with("Failed to load KN-004: "),
        "detail must follow `knowledge list`: {w}"
    );
    assert_eq!(json["data"]["knowledge_health"]["total"], 3);
}

#[test]
fn kl2_status_mcp_parity_with_cli() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    corrupt_fixture(dir);

    let cli = run_ok(dir, &["status"]);
    let mcp = mcp_call(dir, "ltp/status", json!({}));
    assert_eq!(mcp["success"], true);
    assert_eq!(load_error_ids(&mcp["warnings"]), vec!["KN-004"]);
    assert_eq!(mcp["warnings"], cli["warnings"]);
    assert_eq!(
        mcp["data"]["knowledge_health"],
        cli["data"]["knowledge_health"]
    );
    assert_eq!(mcp["data"]["knowledge_health"]["total"], 3);
}

// --- KL3: validate ---------------------------------------------------------------

#[test]
fn kl3_validate_warns_in_knowledge_pool_entry() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    corrupt_fixture(dir);
    let clean = tempfile::tempdir().unwrap();
    fixture(clean.path());

    let (json, code) = run_ltp(dir, &["validate"]);
    let (clean_json, clean_code) = run_ltp(clean.path(), &["validate"]);
    assert_eq!(
        code, clean_code,
        "the warning must not change the exit code"
    );
    assert_eq!(json["success"], clean_json["success"]);

    let pool = pool_warnings(&json);
    assert_eq!(load_error_ids(&pool), vec!["KN-004"]);
    // Every warning the clean pool produced (except those about KN-004 itself)
    // still appears.
    let clean_codes: Vec<String> = pool_warnings(&clean_json)
        .as_array()
        .unwrap()
        .iter()
        .filter(|w| w["knowledge_id"] != "KN-004")
        .map(|w| w["code"].as_str().unwrap().to_string())
        .collect();
    let codes = warning_codes(&pool);
    for c in &clean_codes {
        assert!(codes.contains(c), "missing {c} in {pool}");
    }
}

#[test]
fn kl3b_validate_epistemic_warnings_survive_unreadable_item() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    corrupt_fixture(dir);
    // RC-001 has no support: a fact with 0 active supports ⇒ EPISTEMIC_UNGROUNDED.
    run_ok(dir, &["node", "edit", "RC-001", "--epistemic", "fact"]);

    let json = run_ltp(dir, &["validate"]).0;
    let pool = pool_warnings(&json);
    let codes = warning_codes(&pool);
    assert_eq!(
        codes[0], "KNOWLEDGE_LOAD_ERROR",
        "load errors go first: {pool}"
    );
    assert!(
        codes.contains(&"EPISTEMIC_UNGROUNDED".to_string()),
        "{pool}"
    );
}

// --- KL4: tree walk / trace -------------------------------------------------------

#[test]
fn kl4_walk_and_trace_warn_only_with_show_knowledge() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let tree = corrupt_fixture(dir);

    let walk = run_ok(dir, &["tree", "walk", &tree, "--show-knowledge"]);
    assert_eq!(load_error_ids(&walk["warnings"]), vec!["KN-004"]);
    let trace = run_ok(
        dir,
        &[
            "trace",
            "UDE-001",
            "--tree",
            &tree,
            "--direction",
            "upstream",
            "--show-knowledge",
        ],
    );
    assert_eq!(load_error_ids(&trace["warnings"]), vec!["KN-004"]);

    // Without the flag the corrupt file is irrelevant: the output is
    // byte-identical to the one with KN-004 gone.
    let walk_args = ["tree", "walk", tree.as_str()];
    let trace_args = [
        "trace",
        "UDE-001",
        "--tree",
        tree.as_str(),
        "--direction",
        "upstream",
    ];
    let walk_corrupt = run_raw(dir, &walk_args);
    let trace_corrupt = run_raw(dir, &trace_args);
    fs::remove_file(dir.join("knowledge/KN-004.json")).unwrap();
    assert_eq!(walk_corrupt, run_raw(dir, &walk_args));
    assert_eq!(trace_corrupt, run_raw(dir, &trace_args));
}

#[test]
fn kl4b_mcp_walk_and_trace_warn_with_show_knowledge() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let tree = corrupt_fixture(dir);

    let walk = mcp_call(
        dir,
        "ltp/tree_walk",
        json!({"tree_id": tree, "show_knowledge": true}),
    );
    assert_eq!(load_error_ids(&walk["warnings"]), vec!["KN-004"], "{walk}");
    let trace = mcp_call(
        dir,
        "ltp/trace",
        json!({"node_id": "UDE-001", "tree": tree, "direction": "upstream", "show_knowledge": true}),
    );
    assert_eq!(
        load_error_ids(&trace["warnings"]),
        vec!["KN-004"],
        "{trace}"
    );
}

// --- KL5: node rm -------------------------------------------------------------------

#[test]
fn kl5_node_rm_warns_after_orphaned_and_does_not_block() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    corrupt_fixture(dir);
    let kn4 = dir.join("knowledge/KN-004.json");
    let before = fs::read(&kn4).unwrap();

    let (json, code) = run_ltp(dir, &["node", "rm", "UDE-001", "--force"]);
    assert_eq!(code, 0, "{json}");
    assert_eq!(json["success"], true);
    assert!(!dir.join("nodes/UDE-001.json").exists());

    let codes = warning_codes(&json["warnings"]);
    let orphaned = codes
        .iter()
        .position(|c| c == "KNOWLEDGE_ORPHANED")
        .expect("KNOWLEDGE_ORPHANED");
    let load = codes
        .iter()
        .position(|c| c == "KNOWLEDGE_LOAD_ERROR")
        .expect("KNOWLEDGE_LOAD_ERROR");
    assert!(orphaned < load, "order: {codes:?}");
    assert_eq!(
        warnings_with(&json["warnings"], "KNOWLEDGE_ORPHANED")[0]["knowledge_ids"],
        json!(["KN-001"])
    );
    assert_eq!(load_error_ids(&json["warnings"]), vec!["KN-004"]);
    assert_eq!(fs::read(&kn4).unwrap(), before, "rm must not touch KN-004");
}

#[test]
fn kl5b_node_rm_load_error_without_orphans() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    corrupt_fixture(dir);
    run_ok(dir, &["node", "add", "Lonely", "--type", "UDE"]);

    let json = run_ok(dir, &["node", "rm", "UDE-002"]);
    assert!(warnings_with(&json["warnings"], "KNOWLEDGE_ORPHANED").is_empty());
    assert_eq!(load_error_ids(&json["warnings"]), vec!["KN-004"]);
}

// --- KL6: several unreadable items --------------------------------------------------

#[test]
fn kl6_several_unreadable_in_id_order() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let tree = fixture(dir);
    add_knowledge(dir, "Five");
    add_knowledge(dir, "Six");
    corrupt(dir, "KN-006");
    corrupt(dir, "KN-004");

    let expected = vec!["KN-004", "KN-006"];
    let status = run_ok(dir, &["status"]);
    assert_eq!(load_error_ids(&status["warnings"]), expected);
    assert_eq!(status["data"]["knowledge_health"]["total"], 4);
    assert_eq!(
        load_error_ids(&pool_warnings(&run_ltp(dir, &["validate"]).0)),
        expected
    );
    let walk = run_ok(dir, &["tree", "walk", &tree, "--show-knowledge"]);
    assert_eq!(load_error_ids(&walk["warnings"]), expected);
    assert_eq!(
        load_error_ids(&mcp_call(dir, "ltp/status", json!({}))["warnings"]),
        expected
    );
}

// --- KL7: regression without unreadable items ----------------------------------------

#[test]
fn kl7_no_unreadable_no_new_warnings() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let tree = fixture(dir);

    let status = run_ok(dir, &["status"]);
    assert_eq!(status["warnings"], json!([]));
    assert_eq!(
        status["data"]["knowledge_health"],
        json!({
            "total": 4,
            "unlinked_items": 1,
            "contradictions": 0,
            "by_status": {"unverified": 3, "verified": 1, "refuted": 0, "superseded": 0},
            "epistemic_coverage": {"fact": 0, "hypothesis": 2, "assumption": 0, "derived": 0}
        })
    );
    assert_eq!(
        mcp_call(dir, "ltp/status", json!({}))["data"]["knowledge_health"],
        status["data"]["knowledge_health"]
    );
    assert!(load_error_ids(&pool_warnings(&run_ltp(dir, &["validate"]).0)).is_empty());
    let walk = run_ok(dir, &["tree", "walk", &tree, "--show-knowledge"]);
    assert_eq!(walk["warnings"], json!([]));
    let trace = run_ok(
        dir,
        &[
            "trace",
            "UDE-001",
            "--tree",
            &tree,
            "--direction",
            "upstream",
            "--show-knowledge",
        ],
    );
    assert_eq!(trace["warnings"], json!([]));
    let rm = run_ok(dir, &["node", "rm", "UDE-001", "--force"]);
    assert_eq!(
        warning_codes(&rm["warnings"])
            .iter()
            .filter(|c| c.starts_with("KNOWLEDGE_"))
            .collect::<Vec<_>>(),
        vec!["KNOWLEDGE_ORPHANED"]
    );
}
