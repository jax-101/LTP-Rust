//! v0.6.0 — Unreadable ≠ absent (PLAN_v060.md).
//!
//! Something that exists but cannot be read is never reported as missing:
//! - layer 1 (storage): `*NotFound` only on `ErrorKind::NotFound` (D-3);
//! - layer 2 (commands): `*_NOT_FOUND` only for `*NotFound`, else `IO_ERROR` (D-4);
//! - layer 3 (knowledge resolve): an unreadable tree is not "no such target" (D-7);
//! - a pool that cannot be listed warns `KNOWLEDGE_POOL_UNREADABLE` (D-1/D-2);
//! - a rebuilt or reconciled `counters.json` warns `COUNTERS_REBUILT`, at most
//!   once per command (D-5), and only on success outputs (D-5b).
//!
//! Deterministic cases replace a directory by a file (ENOTDIR) or a file by a
//! directory (EISDIR). `chmod` cases are `cfg(unix)` and skipped as root.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{json, Value};

// --- harness -----------------------------------------------------------------

fn run_ltp(dir: &Path, args: &[&str]) -> (Value, i32) {
    let output = Command::new(env!("CARGO_BIN_EXE_ltp"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("failed to execute ltp binary");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap_or_else(|_| {
        panic!(
            "Failed to parse JSON.\nargs: {args:?}\nstdout: {stdout}\nstderr: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    });
    (json, output.status.code().unwrap_or(-1))
}

fn run_ok(dir: &Path, args: &[&str]) -> Value {
    let (json, code) = run_ltp(dir, args);
    assert_eq!(code, 0, "ltp {args:?} failed: {json}");
    json
}

/// A long-lived `ltp-mcp` server: one process, several tool calls.
struct McpServer {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

impl McpServer {
    fn start(dir: &Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_ltp-mcp"))
            .arg("--workspace")
            .arg(dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("failed to spawn ltp-mcp");
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Self {
            child,
            stdin,
            stdout,
            next_id: 1,
        }
    }

    /// CommandOutput of one tool call (waits for its response line).
    fn call(&mut self, tool: &str, arguments: Value) -> Value {
        let request = json!({
            "jsonrpc": "2.0", "id": self.next_id,
            "method": "tools/call",
            "params": { "name": tool, "arguments": arguments }
        });
        self.next_id += 1;
        writeln!(self.stdin, "{request}").unwrap();
        self.stdin.flush().unwrap();
        let mut line = String::new();
        while line.trim().is_empty() {
            line.clear();
            assert!(
                self.stdout.read_line(&mut line).unwrap() > 0,
                "server closed"
            );
        }
        let response: Value = serde_json::from_str(&line).unwrap();
        let text = response["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_else(|| panic!("no tool result: {response}"));
        serde_json::from_str(text).unwrap()
    }
}

impl Drop for McpServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// One tool call on a fresh server.
fn mcp_call(dir: &Path, tool: &str, arguments: Value) -> Value {
    McpServer::start(dir).call(tool, arguments)
}

fn codes(list: &Value) -> Vec<String> {
    list.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|e| e["code"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn error_codes(json: &Value) -> Vec<String> {
    codes(&json["errors"])
}

fn warning_codes(json: &Value) -> Vec<String> {
    codes(&json["warnings"])
}

fn warnings_with<'a>(json: &'a Value, code: &str) -> Vec<&'a Value> {
    json["warnings"]
        .as_array()
        .map(|a| a.iter().filter(|w| w["code"] == code).collect())
        .unwrap_or_default()
}

/// `validate` details entry for `tree_id` (panics if absent).
fn detail<'a>(validate: &'a Value, tree_id: &str) -> &'a Value {
    validate["data"]["details"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["tree_id"] == tree_id)
        .unwrap_or_else(|| panic!("no details entry for {tree_id}: {validate}"))
}

/// Every warning code of every `validate` details entry.
fn all_detail_warning_codes(validate: &Value) -> Vec<String> {
    validate["data"]["details"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|d| codes(&d["warnings"]))
        .collect()
}

fn pool_warnings(validate: &Value) -> Value {
    validate["data"]["details"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["tree_id"] == "_knowledge_pool")
        .map(|d| json!({ "warnings": d["warnings"].clone() }))
        .unwrap_or_else(|| json!({ "warnings": [] }))
}

#[cfg(unix)]
fn running_as_root() -> bool {
    // SAFETY: geteuid has no preconditions and cannot fail.
    unsafe { libc::geteuid() == 0 }
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

/// Replaces a directory with a regular file of the same name (ENOTDIR inside).
fn dir_to_file(path: &Path) {
    fs::remove_dir_all(path).unwrap();
    fs::write(path, "not a directory").unwrap();
}

/// Replaces a file with a directory of the same name (EISDIR on read).
fn file_to_dir(path: &Path) {
    fs::remove_file(path).unwrap();
    fs::create_dir(path).unwrap();
}

fn tree_file(dir: &Path, tree: &str) -> PathBuf {
    dir.join("trees").join(format!("{tree}.json"))
}

fn node_file(dir: &Path, node: &str) -> PathBuf {
    dir.join("nodes").join(format!("{node}.json"))
}

fn counters_path(dir: &Path) -> PathBuf {
    dir.join(".ltp").join("counters.json")
}

/// Fixture. Nodes RC-001, UDE-001 (fact), RC-002, UDE-002; CRT `tree-crt-a`
/// with all four attached, LINK-001 RC-001 → UDE-001 carrying ASM-001.
/// KN-001 supports UDE-001; KN-002 supports ASM-001; KN-003 supports UDE-002.
fn fixture(dir: &Path) -> String {
    run_ok(dir, &["init", "--name", "V060"]);
    run_ok(dir, &["node", "add", "causa", "--type", "RC"]);
    run_ok(
        dir,
        &[
            "node",
            "add",
            "efecto",
            "--type",
            "UDE",
            "--epistemic",
            "fact",
        ],
    );
    run_ok(dir, &["node", "add", "causa dos", "--type", "RC"]);
    run_ok(dir, &["node", "add", "efecto dos", "--type", "UDE"]);
    let tree = "tree-crt-a".to_string();
    run_ok(dir, &["tree", "new", "crt", "a"]);
    for node in ["RC-001", "UDE-001", "RC-002", "UDE-002"] {
        run_ok(dir, &["tree", "attach", "--tree", &tree, "--node", node]);
    }
    run_ok(
        dir,
        &[
            "link", "connect", "--tree", &tree, "--from", "RC-001", "--to", "UDE-001",
        ],
    );
    run_ok(
        dir,
        &[
            "assume", "add", "--tree", &tree, "--link", "LINK-001", "--text", "porque",
        ],
    );
    for (label, target) in [("uno", "UDE-001"), ("dos", "ASM-001"), ("tres", "UDE-002")] {
        let added = run_ok(
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
        let id = added["data"]["created_knowledge_id"].as_str().unwrap();
        run_ok(
            dir,
            &[
                "knowledge",
                "link",
                id,
                "--to",
                target,
                "--relation",
                "supports",
            ],
        );
    }
    tree
}

fn first_error(json: &Value) -> &Value {
    &json["errors"][0]
}

fn assert_io_error(json: &Value, what: &str) {
    assert_eq!(json["success"], false, "{what}: {json}");
    assert_eq!(error_codes(json), vec!["IO_ERROR"], "{what}: {json}");
}

// --- layer 1: storage (D-3) -----------------------------------------------------

#[test]
fn l1_nodes_dir_enotdir_node_inspect_is_io_error() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    fixture(dir);
    dir_to_file(&dir.join("nodes"));
    let (json, _) = run_ltp(dir, &["node", "inspect", "UDE-001"]);
    assert_io_error(&json, "node inspect with nodes/ as a file");
}

#[cfg(unix)]
#[test]
fn l1_nodes_dir_000_node_inspect_is_io_error() {
    if running_as_root() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    fixture(dir);
    set_mode(&dir.join("nodes"), 0o000);
    let (json, _) = run_ltp(dir, &["node", "inspect", "UDE-001"]);
    set_mode(&dir.join("nodes"), 0o755);
    assert_io_error(&json, "node inspect with nodes/ in 000");
}

#[test]
fn l1_trees_dir_enotdir_tree_walk_is_io_error() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let tree = fixture(dir);
    dir_to_file(&dir.join("trees"));
    let (json, _) = run_ltp(dir, &["tree", "walk", &tree]);
    assert_io_error(&json, "tree walk with trees/ as a file");
}

#[cfg(unix)]
#[test]
fn l1_trees_dir_000_tree_walk_is_io_error() {
    if running_as_root() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let tree = fixture(dir);
    set_mode(&dir.join("trees"), 0o000);
    let (json, _) = run_ltp(dir, &["tree", "walk", &tree]);
    set_mode(&dir.join("trees"), 0o755);
    assert_io_error(&json, "tree walk with trees/ in 000");
}

#[test]
fn l1_knowledge_dir_enotdir_inspect_is_io_error() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    fixture(dir);
    dir_to_file(&dir.join("knowledge"));
    let (json, _) = run_ltp(dir, &["knowledge", "inspect", "KN-001"]);
    assert_io_error(&json, "knowledge inspect with knowledge/ as a file");
}

#[cfg(unix)]
#[test]
fn l1_knowledge_dir_000_inspect_is_io_error() {
    if running_as_root() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    fixture(dir);
    set_mode(&dir.join("knowledge"), 0o000);
    let (json, _) = run_ltp(dir, &["knowledge", "inspect", "KN-001"]);
    set_mode(&dir.join("knowledge"), 0o755);
    assert_io_error(&json, "knowledge inspect with knowledge/ in 000");
}

/// Already correct before v0.6.0 ("correcto" rows): listing an unreadable pool.
#[cfg(unix)]
#[test]
fn l1_nodes_dir_000_node_list_and_status_stay_io_error() {
    if running_as_root() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    fixture(dir);
    set_mode(&dir.join("nodes"), 0o000);
    let (list, _) = run_ltp(dir, &["node", "list"]);
    let (status, _) = run_ltp(dir, &["status"]);
    set_mode(&dir.join("nodes"), 0o755);
    assert_io_error(&list, "node list");
    assert_io_error(&status, "status");
}

/// A broken symlink is listed by `read_dir` but opens as `NotFound`; saying
/// "does not exist" would contradict the listing (D-3, rev 3.1).
#[cfg(unix)]
#[test]
fn l1_broken_symlink_is_io_error_not_not_found() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    fixture(dir);
    std::os::unix::fs::symlink(dir.join("nowhere.json"), tree_file(dir, "tree-crt-ghost")).unwrap();
    std::os::unix::fs::symlink(dir.join("nowhere.json"), node_file(dir, "UDE-009")).unwrap();

    let (walk, _) = run_ltp(dir, &["tree", "walk", "tree-crt-ghost"]);
    assert_io_error(&walk, "tree walk on a broken symlink");
    let (inspect, _) = run_ltp(dir, &["node", "inspect", "UDE-009"]);
    assert_io_error(&inspect, "node inspect on a broken symlink");

    let (validate, _) = run_ltp(dir, &["validate"]);
    assert_eq!(validate["success"], false, "{validate}");
    assert_eq!(
        codes(&detail(&validate, "tree-crt-ghost")["errors"]),
        vec!["TREE_LOAD_ERROR"]
    );
}

/// Legitimate `*_NOT_FOUND` stay as they were.
#[test]
fn l1_l2_legit_not_found_unchanged() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let tree = fixture(dir);
    let cases: Vec<(Vec<&str>, &str)> = vec![
        (vec!["node", "inspect", "UDE-999"], "NODE_NOT_FOUND"),
        (vec!["tree", "walk", "tree-crt-nope"], "TREE_NOT_FOUND"),
        (
            vec!["knowledge", "inspect", "KN-999"],
            "KNOWLEDGE_NOT_FOUND",
        ),
        (
            vec![
                "link", "connect", "--tree", &tree, "--from", "RC-999", "--to", "UDE-001",
            ],
            "REFERENTIAL_INTEGRITY_VIOLATION",
        ),
        (
            vec![
                "knowledge",
                "link",
                "KN-001",
                "--to",
                "ASM-999",
                "--relation",
                "contradicts",
            ],
            "TARGET_NOT_FOUND",
        ),
    ];
    for (args, code) in cases {
        let (json, _) = run_ltp(dir, &args);
        assert_eq!(error_codes(&json), vec![code], "{args:?}: {json}");
    }
}

// --- layer 2: commands (D-4) ------------------------------------------------------

#[test]
fn l2_tree_file_eisdir_tree_walk_is_io_error() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let tree = fixture(dir);
    file_to_dir(&tree_file(dir, &tree));
    let (json, _) = run_ltp(dir, &["tree", "walk", &tree]);
    assert_io_error(&json, "tree walk on a directory-as-tree");
}

#[cfg(unix)]
#[test]
fn l2_tree_file_000_tree_walk_is_io_error() {
    if running_as_root() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let tree = fixture(dir);
    set_mode(&tree_file(dir, &tree), 0o000);
    let (json, _) = run_ltp(dir, &["tree", "walk", &tree]);
    set_mode(&tree_file(dir, &tree), 0o644);
    assert_io_error(&json, "tree walk on a 000 tree file");
}

#[test]
fn l2_node_file_eisdir_link_connect_is_io_error() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let tree = fixture(dir);
    file_to_dir(&node_file(dir, "RC-002"));
    let (json, _) = run_ltp(
        dir,
        &[
            "link", "connect", "--tree", &tree, "--from", "RC-002", "--to", "UDE-001",
        ],
    );
    assert_io_error(&json, "link connect from an unreadable node");
}

#[cfg(unix)]
#[test]
fn l2_node_file_000_link_connect_is_io_error() {
    if running_as_root() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let tree = fixture(dir);
    set_mode(&node_file(dir, "RC-002"), 0o000);
    let (json, _) = run_ltp(
        dir,
        &[
            "link", "connect", "--tree", &tree, "--from", "RC-002", "--to", "UDE-001",
        ],
    );
    set_mode(&node_file(dir, "RC-002"), 0o644);
    assert_io_error(&json, "link connect from a 000 node");
}

// --- layer 3: knowledge resolve (D-7) -----------------------------------------------

#[test]
fn l3_validate_unreadable_tree_is_not_a_dangling_ref() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let tree = fixture(dir);
    file_to_dir(&tree_file(dir, &tree));
    let (json, _) = run_ltp(dir, &["validate"]);
    assert_eq!(json["success"], false, "{json}");
    assert_eq!(
        codes(&detail(&json, &tree)["errors"]),
        vec!["TREE_LOAD_ERROR"]
    );
    let pool = pool_warnings(&json);
    assert!(
        !warning_codes(&pool).contains(&"DANGLING_KNOWLEDGE_REF".to_string()),
        "ASM-001 lives in an unreadable tree, it is not dangling: {pool}"
    );
}

#[test]
fn l3_knowledge_inspect_warns_tree_load_error() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let tree = fixture(dir);
    file_to_dir(&tree_file(dir, &tree));
    let json = run_ok(dir, &["knowledge", "inspect", "KN-002"]);
    assert_eq!(json["data"]["links"][0]["target"], "ASM-001");
    assert_eq!(json["data"]["links"][0]["target_type"], "unknown");
    let w = warnings_with(&json, "TREE_LOAD_ERROR");
    assert_eq!(w.len(), 1, "{json}");
    assert_eq!(w[0]["tree_id"], tree.as_str());
}

#[test]
fn l3_knowledge_link_to_unverifiable_target_is_io_error() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let tree = fixture(dir);
    file_to_dir(&tree_file(dir, &tree));
    let (json, _) = run_ltp(
        dir,
        &[
            "knowledge",
            "link",
            "KN-001",
            "--to",
            "ASM-001",
            "--relation",
            "contradicts",
        ],
    );
    assert_io_error(&json, "knowledge link to a target in an unreadable tree");
    assert_eq!(first_error(&json)["tree_id"], tree.as_str(), "{json}");
}

/// (d) The target is in a readable tree while another tree is unreadable: it
/// resolves, with no false warning.
#[test]
fn l3_target_in_readable_tree_resolves_despite_other_unreadable_tree() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    fixture(dir);
    // Sorts before `tree-crt-a`, so it is visited first.
    run_ok(dir, &["tree", "new", "crt", "0x"]);
    file_to_dir(&tree_file(dir, "tree-crt-0x"));

    let inspect = run_ok(dir, &["knowledge", "inspect", "KN-002"]);
    assert_eq!(inspect["data"]["links"][0]["target_type"], "assumption");
    assert!(
        warnings_with(&inspect, "TREE_LOAD_ERROR").is_empty(),
        "{inspect}"
    );

    let (validate, _) = run_ltp(dir, &["validate"]);
    assert!(
        !all_detail_warning_codes(&validate).contains(&"DANGLING_KNOWLEDGE_REF".to_string()),
        "{validate}"
    );
    run_ok(
        dir,
        &[
            "knowledge",
            "link",
            "KN-001",
            "--to",
            "ASM-001",
            "--relation",
            "contradicts",
        ],
    );
}

/// A legitimate dangling ref (after `tree rm`) is still reported.
#[test]
fn l3_legit_dangling_after_tree_rm_unchanged() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let tree = fixture(dir);
    run_ok(dir, &["tree", "rm", &tree]);
    let (validate, _) = run_ltp(dir, &["validate"]);
    let pool = pool_warnings(&validate);
    let dangling = warnings_with(&pool, "DANGLING_KNOWLEDGE_REF");
    assert_eq!(dangling.len(), 1, "{validate}");
    assert_eq!(dangling[0]["knowledge_id"], "KN-002");
    assert_eq!(dangling[0]["target"], "ASM-001");
    let inspect = run_ok(dir, &["knowledge", "inspect", "KN-002"]);
    assert!(
        warnings_with(&inspect, "TREE_LOAD_ERROR").is_empty(),
        "{inspect}"
    );
}

/// G1 — corrupt JSON and unreadable are both reported, never as missing nor as
/// clean, and their `detail` tells them apart (decision: codes follow D-4/D-7).
#[test]
fn g1_corrupt_vs_unreadable_tree() {
    let corrupt = tempfile::tempdir().unwrap();
    let unreadable = tempfile::tempdir().unwrap();
    let tree = fixture(corrupt.path());
    fixture(unreadable.path());
    fs::write(tree_file(corrupt.path(), &tree), "{broken").unwrap();
    file_to_dir(&tree_file(unreadable.path(), &tree));

    let mut walk_details = Vec::new();
    for dir in [corrupt.path(), unreadable.path()] {
        let (walk, _) = run_ltp(dir, &["tree", "walk", &tree]);
        assert_io_error(&walk, "tree walk");
        walk_details.push(first_error(&walk)["detail"].clone());

        let (validate, _) = run_ltp(dir, &["validate"]);
        assert_eq!(validate["success"], false);
        assert_eq!(
            codes(&detail(&validate, &tree)["errors"]),
            vec!["TREE_LOAD_ERROR"]
        );
        assert!(
            !all_detail_warning_codes(&validate).contains(&"DANGLING_KNOWLEDGE_REF".to_string()),
            "{validate}"
        );

        let inspect = run_ok(dir, &["knowledge", "inspect", "KN-002"]);
        let w = warnings_with(&inspect, "TREE_LOAD_ERROR");
        assert_eq!(w.len(), 1, "{inspect}");
        assert_eq!(w[0]["tree_id"], tree.as_str());
    }
    assert_ne!(
        walk_details[0], walk_details[1],
        "detail must tell them apart"
    );
}

/// G7 — `trees/` corner cases: an empty `.json` is a load error, never an empty
/// tree; files that are not `.json` are ignored without a warning.
#[test]
fn g7_trees_dir_corner_cases() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    fixture(dir);
    fs::write(tree_file(dir, "tree-crt-empty"), "").unwrap();
    fs::write(dir.join("trees").join("notas.txt"), "hola").unwrap();
    fs::write(dir.join("trees").join(".DS_Store"), [0u8, 1, 2]).unwrap();

    let (walk, _) = run_ltp(dir, &["tree", "walk", "tree-crt-empty"]);
    assert_io_error(&walk, "tree walk on an empty file");

    let (validate, _) = run_ltp(dir, &["validate"]);
    assert_eq!(
        codes(&detail(&validate, "tree-crt-empty")["errors"]),
        vec!["TREE_LOAD_ERROR"]
    );
    let text = validate.to_string();
    assert!(!text.contains("notas"), "{text}");
    assert!(!text.contains("DS_Store"), "{text}");
    let list = run_ok(dir, &["tree", "list"]).to_string();
    assert!(
        !list.contains("notas") && !list.contains("DS_Store"),
        "{list}"
    );
}

// --- B1: knowledge pool that cannot be listed (D-1/D-2) -------------------------------

fn pool_unreadable_count(json: &Value) -> usize {
    warnings_with(json, "KNOWLEDGE_POOL_UNREADABLE").len()
}

#[test]
fn b1_status_warns_and_keeps_counts() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    fixture(dir);
    let before = run_ok(dir, &["status"]);
    dir_to_file(&dir.join("knowledge"));
    let json = run_ok(dir, &["status"]);
    assert_eq!(pool_unreadable_count(&json), 1, "{json}");
    assert_eq!(json["data"]["node_count"], before["data"]["node_count"]);
    assert_eq!(json["data"]["tree_count"], before["data"]["tree_count"]);
    assert_eq!(json["data"]["knowledge_health"]["total"], 0);

    let mcp = mcp_call(dir, "ltp/status", json!({}));
    assert_eq!(mcp["warnings"], json["warnings"]);
}

#[cfg(unix)]
#[test]
fn b1_status_warns_with_knowledge_dir_000() {
    if running_as_root() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    fixture(dir);
    set_mode(&dir.join("knowledge"), 0o000);
    let json = run_ltp(dir, &["status"]).0;
    set_mode(&dir.join("knowledge"), 0o755);
    assert_eq!(json["success"], true, "{json}");
    assert_eq!(pool_unreadable_count(&json), 1, "{json}");
}

#[test]
fn b1_validate_skips_epistemic_analysis() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    fixture(dir);
    let (clean, _) = run_ltp(dir, &["validate"]);
    assert!(
        !all_detail_warning_codes(&clean).contains(&"EPISTEMIC_UNGROUNDED".to_string()),
        "UDE-001 is a supported fact: {clean}"
    );
    dir_to_file(&dir.join("knowledge"));
    let (json, _) = run_ltp(dir, &["validate"]);
    let pool = pool_warnings(&json);
    assert_eq!(
        warning_codes(&pool),
        vec!["KNOWLEDGE_POOL_UNREADABLE"],
        "no false EPISTEMIC_UNGROUNDED: {json}"
    );
}

#[test]
fn b1_node_rm_warns_without_blocking() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    fixture(dir);
    run_ok(dir, &["node", "add", "suelto", "--type", "UDE"]);
    dir_to_file(&dir.join("knowledge"));
    let json = run_ok(dir, &["node", "rm", "UDE-003"]);
    assert_eq!(pool_unreadable_count(&json), 1, "{json}");
    assert!(!dir.join("nodes/UDE-003.json").exists());
}

#[test]
fn b1_trace_and_walk_warn_only_with_show_knowledge() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let tree = fixture(dir);
    dir_to_file(&dir.join("knowledge"));
    let trace = |extra: &[&str]| {
        let mut args = vec![
            "trace",
            "--tree",
            &tree,
            "--direction",
            "upstream",
            "UDE-001",
        ];
        args.extend_from_slice(extra);
        run_ok(dir, &args)
    };
    assert_eq!(pool_unreadable_count(&trace(&["--show-knowledge"])), 1);
    assert_eq!(pool_unreadable_count(&trace(&[])), 0);
    let walk_k = run_ok(dir, &["tree", "walk", &tree, "--show-knowledge"]);
    assert_eq!(pool_unreadable_count(&walk_k), 1, "{walk_k}");
    let walk = run_ok(dir, &["tree", "walk", &tree]);
    assert_eq!(pool_unreadable_count(&walk), 0, "{walk}");
}

/// (f) `knowledge/` replaced by a file is never an empty pool.
#[test]
fn b1_knowledge_dir_as_file_is_never_an_empty_pool() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    fixture(dir);
    dir_to_file(&dir.join("knowledge"));
    let (list, _) = run_ltp(dir, &["knowledge", "list"]);
    assert_io_error(&list, "knowledge list");
    let status = run_ok(dir, &["status"]);
    assert_eq!(pool_unreadable_count(&status), 1, "{status}");
}

// --- adjacent: validate with an unlistable node pool (D-3) ------------------------------

#[test]
fn adj_validate_unlistable_nodes_is_io_error() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    fixture(dir);
    dir_to_file(&dir.join("nodes"));
    let (json, _) = run_ltp(dir, &["validate"]);
    assert_io_error(&json, "validate with nodes/ as a file");
}

// --- COUNTERS_REBUILT (D-5) ---------------------------------------------------------------

fn rebuilt(json: &Value) -> Vec<&Value> {
    warnings_with(json, "COUNTERS_REBUILT")
}

fn assert_one_rebuilt(json: &Value, reason: &str) {
    let w = rebuilt(json);
    assert_eq!(w.len(), 1, "exactly one COUNTERS_REBUILT: {json}");
    assert_eq!(w[0]["reason"], reason, "{json}");
}

fn set_counter(dir: &Path, prefix: &str, value: u64) {
    let path = counters_path(dir);
    let mut counters: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    counters[prefix] = Value::from(value);
    fs::write(&path, serde_json::to_string_pretty(&counters).unwrap()).unwrap();
}

fn rm_counters(dir: &Path) {
    fs::remove_file(counters_path(dir)).unwrap();
}

/// PID of a process that has already exited.
fn dead_pid() -> u32 {
    let mut child = Command::new("true").spawn().unwrap();
    let pid = child.id();
    child.wait().unwrap();
    pid
}

fn write_stale_lock(dir: &Path) {
    let lock = json!({
        "pid": dead_pid(),
        "timestamp": "2026-10-09T00:00:00+00:00",
        "command": "node add",
    });
    fs::write(dir.join(".ltp/lock"), lock.to_string()).unwrap();
}

#[test]
fn c1_cli_missing_counters_warns_once_then_never() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    fixture(dir);
    rm_counters(dir);
    let json = run_ok(dir, &["node", "add", "nuevo", "--type", "UDE"]);
    assert_eq!(json["data"]["id"], "UDE-003");
    assert_one_rebuilt(&json, "missing");
    assert_eq!(warning_codes(&json)[0], "COUNTERS_REBUILT", "first: {json}");
    let again = run_ok(dir, &["node", "add", "otro", "--type", "UDE"]);
    assert!(rebuilt(&again).is_empty(), "{again}");
}

#[test]
fn c2_cli_corrupt_counters_warns() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    fixture(dir);
    fs::write(counters_path(dir), "{").unwrap();
    let json = run_ok(dir, &["node", "add", "nuevo", "--type", "UDE"]);
    assert_eq!(json["data"]["id"], "UDE-003");
    assert_one_rebuilt(&json, "corrupt");
    let again = run_ok(dir, &["node", "add", "otro", "--type", "UDE"]);
    assert!(rebuilt(&again).is_empty(), "{again}");
}

#[test]
fn c3_cli_stale_counter_warns_with_prefix_and_range() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    fixture(dir);
    set_counter(dir, "UDE", 0);
    let json = run_ok(dir, &["node", "add", "nuevo", "--type", "UDE"]);
    assert_eq!(json["data"]["id"], "UDE-003");
    assert_one_rebuilt(&json, "stale");
    let w = rebuilt(&json)[0];
    assert_eq!(w["prefix"], "UDE");
    assert_eq!(w["from"], 0);
    assert_eq!(w["to"], 2);
}

#[test]
fn c4_mcp_missing_corrupt_stale() {
    for case in ["missing", "corrupt", "stale"] {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        fixture(dir);
        match case {
            "missing" => rm_counters(dir),
            "corrupt" => fs::write(counters_path(dir), "{").unwrap(),
            _ => set_counter(dir, "UDE", 0),
        }
        let json = mcp_call(
            dir,
            "ltp/node_add",
            json!({ "label": "nuevo", "type": "UDE" }),
        );
        assert_eq!(json["success"], true, "{case}: {json}");
        assert_one_rebuilt(&json, case);
    }
}

/// Two scopes in one command (INT in nodes, LINK in trees): one notice and no
/// false `stale` from the second scope.
#[test]
fn c5_macro_expand_without_counters_warns_once() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let tree = fixture(dir);
    run_ok(
        dir,
        &[
            "macro", "add", "--tree", &tree, "--from", "RC-002", "--to", "UDE-002", "--label",
            "salto",
        ],
    );
    rm_counters(dir);
    let json = run_ok(
        dir,
        &[
            "macro",
            "expand",
            "--tree",
            &tree,
            "--macro-link",
            "MACRO-001",
            "--steps",
            "uno,dos",
        ],
    );
    assert_one_rebuilt(&json, "missing");
}

#[test]
fn c5_path_explode_and_replace_without_counters_warn_once() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let tree = fixture(dir);
    rm_counters(dir);
    let explode = run_ok(
        dir,
        &[
            "path", "explode", "--tree", &tree, "--link", "LINK-001", "--asm", "ASM-001",
            "--label", "medio",
        ],
    );
    assert_one_rebuilt(&explode, "missing");

    let tmp2 = tempfile::tempdir().unwrap();
    let dir2 = tmp2.path();
    let tree2 = fixture(dir2);
    run_ok(
        dir2,
        &[
            "link", "connect", "--tree", &tree2, "--from", "UDE-001", "--to", "UDE-002",
        ],
    );
    let collapse = run_ok(
        dir2,
        &[
            "path", "collapse", "--tree", &tree2, "--from", "RC-001", "--to", "UDE-002", "--label",
            "resumen",
        ],
    );
    let macro_id = collapse["data"]["macro_edge_id"]
        .as_str()
        .or_else(|| collapse["data"]["macro_link"].as_str())
        .unwrap_or("MACRO-001")
        .to_string();
    rm_counters(dir2);
    let replace = run_ok(
        dir2,
        &[
            "path",
            "replace",
            "--tree",
            &tree2,
            "--macro-link",
            &macro_id,
            "--by-node",
            "RC-002",
        ],
    );
    assert_one_rebuilt(&replace, "missing");
}

/// D-5b — an error output carries no notice, but the counters are repaired.
#[test]
fn c6_error_output_has_no_notice() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let tree = fixture(dir);
    rm_counters(dir);
    let (json, code) = run_ltp(
        dir,
        &[
            "link", "connect", "--tree", &tree, "--from", "UDE-001", "--to", "RC-001",
        ],
    );
    assert_ne!(code, 0);
    assert_eq!(error_codes(&json), vec!["CIRCULAR_DEPENDENCY_DETECTED"]);
    assert!(rebuilt(&json).is_empty(), "{json}");
    assert!(counters_path(dir).exists(), "counters repaired on disk");
}

/// (b) One MCP server, several commands: the notice is per command (the lock
/// session resets), not per server.
#[test]
fn c7_mcp_same_server_notice_per_command() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    fixture(dir);
    rm_counters(dir);
    let mut server = McpServer::start(dir);
    let add = json!({ "label": "x", "type": "UDE" });
    let first = server.call("ltp/node_add", add.clone());
    assert_one_rebuilt(&first, "missing");
    let second = server.call("ltp/node_add", add.clone());
    assert!(rebuilt(&second).is_empty(), "{second}");
    rm_counters(dir);
    let third = server.call("ltp/node_add", add);
    assert_one_rebuilt(&third, "missing");
    assert_eq!(third["data"]["id"], "UDE-005");
}

fn snapshot(dir: &Path) -> Vec<(String, Vec<u8>)> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let rel = path.strip_prefix(root).unwrap().display().to_string();
            if path.is_dir() {
                out.push((format!("{rel}/"), Vec::new()));
                walk(root, &path, out);
            } else {
                out.push((rel, fs::read(&path).unwrap()));
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

/// (c) `--dry-run` shows the notice and leaves the real `.ltp/` untouched.
#[test]
fn c8_dry_run_shows_notice_and_leaves_ltp_untouched() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    fixture(dir);
    rm_counters(dir);
    let before = snapshot(&dir.join(".ltp"));
    let json = run_ok(dir, &["--dry-run", "node", "add", "x", "--type", "UDE"]);
    assert_one_rebuilt(&json, "missing");
    assert_eq!(snapshot(&dir.join(".ltp")), before);
    assert!(!counters_path(dir).exists());
}

/// G3 — stale lock and missing counters together: both, once each, lock first.
#[test]
fn g3_stale_lock_then_counters_notice_cli_and_mcp() {
    for via_mcp in [false, true] {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        fixture(dir);
        rm_counters(dir);
        write_stale_lock(dir);
        let json = if via_mcp {
            mcp_call(dir, "ltp/node_add", json!({ "label": "x", "type": "UDE" }))
        } else {
            run_ok(dir, &["node", "add", "x", "--type", "UDE"])
        };
        assert_eq!(
            warning_codes(&json),
            vec!["STALE_LOCK_REMOVED", "COUNTERS_REBUILT"],
            "mcp={via_mcp}: {json}"
        );
    }
}

/// G4 — inside a batch the notice is still per command.
#[test]
fn g4_batch_notice_per_command() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    fixture(dir);
    run_ok(dir, &["history", "begin-batch", "--label", "lote"]);
    rm_counters(dir);
    let add = |label: &str| run_ok(dir, &["node", "add", label, "--type", "UDE"]);
    assert_one_rebuilt(&add("uno"), "missing");
    assert!(rebuilt(&add("dos")).is_empty());
    assert!(rebuilt(&add("tres")).is_empty());
    rm_counters(dir);
    let back = add("cuatro");
    assert_one_rebuilt(&back, "missing");
    assert_eq!(back["data"]["id"], "UDE-006");
    run_ok(dir, &["history", "end-batch"]);
}

/// G5 — `undo` never emits the notice, and the undone ID is not reissued.
#[test]
fn g5_undo_no_notice_and_monotonic() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    fixture(dir);
    rm_counters(dir);
    let add = run_ok(dir, &["node", "add", "x", "--type", "UDE"]);
    assert_one_rebuilt(&add, "missing");
    assert_eq!(add["data"]["id"], "UDE-003");
    let undo = run_ok(dir, &["undo"]);
    assert!(rebuilt(&undo).is_empty(), "{undo}");
    let next = run_ok(dir, &["node", "add", "y", "--type", "UDE"]);
    assert_eq!(next["data"]["id"], "UDE-004", "ADR-009 monotonicity");
    assert!(rebuilt(&next).is_empty(), "{next}");
}

/// D-5 — every command that mints reports the rebuild (the compiler cannot
/// force a site to forward its notice: `&mut notice` already counts as a use).
#[test]
fn c9_every_minting_command_reports_the_rebuild() {
    const T: &str = "tree-crt-a";
    let grouped: &[&[&str]] = &[
        &[
            "link", "connect", "--tree", T, "--from", "RC-002", "--to", "UDE-001",
        ],
        &[
            "link",
            "group",
            "--tree",
            T,
            "--links",
            "LINK-001,LINK-002",
            "--operator",
            "AND",
        ],
    ];
    let reserved: &[&[&str]] = &[&[
        "macro", "add", "--tree", T, "--from", "RC-002", "--to", "UDE-002", "--label", "salto",
    ]];
    let chain: &[&[&str]] = &[&[
        "link", "connect", "--tree", T, "--from", "UDE-001", "--to", "UDE-002",
    ]];
    let collapsed: &[&[&str]] = &[
        chain[0],
        &[
            "path", "collapse", "--tree", T, "--from", "RC-001", "--to", "UDE-002", "--label", "r",
        ],
    ];
    let cases: &[(&[&[&str]], &[&str])] = &[
        (&[], &["node", "add", "x", "--type", "UDE"]),
        (
            &[],
            &["node", "split", "--tree", T, "UDE-002", "--into", "a", "b"],
        ),
        (
            &[],
            &[
                "knowledge",
                "add",
                "k",
                "--type",
                "observation",
                "--source-excerpt",
                "s",
            ],
        ),
        (
            &[],
            &[
                "assume", "add", "--tree", T, "--link", "LINK-001", "--text", "t",
            ],
        ),
        (
            &[],
            &[
                "invalidate",
                "--tree",
                T,
                "--link",
                "LINK-001",
                "--asm",
                "ASM-001",
                "--injection",
                "i",
            ],
        ),
        (
            &[],
            &[
                "link", "connect", "--tree", T, "--from", "RC-002", "--to", "UDE-002",
            ],
        ),
        (
            &[],
            &[
                "link", "feedback", "--tree", T, "--from", "UDE-001", "--to", "RC-001", "--type",
                "positive",
            ],
        ),
        (
            &[],
            &[
                "link",
                "insert-between",
                "--tree",
                T,
                "--link",
                "LINK-001",
                "--node",
                "RC-002",
            ],
        ),
        (&grouped[..1], grouped[1]),
        (
            grouped,
            &["link", "dissolve", "--tree", T, "--link", "LINK-003"],
        ),
        (
            grouped,
            &[
                "link",
                "split",
                "--tree",
                T,
                "--link",
                "LINK-003",
                "--extract",
                "RC-002",
            ],
        ),
        (
            &[],
            &["nbr", "add", "--tree", T, "--source-node", "UDE-001"],
        ),
        (&[], reserved[0]),
        (
            reserved,
            &[
                "macro",
                "expand",
                "--tree",
                T,
                "--macro-link",
                "MACRO-001",
                "--steps",
                "uno",
            ],
        ),
        (
            reserved,
            &["macro", "promote", "--tree", T, "--macro-link", "MACRO-001"],
        ),
        (
            reserved,
            &[
                "macro-assume",
                "add",
                "--tree",
                T,
                "--macro-link",
                "MACRO-001",
                "--text",
                "m",
            ],
        ),
        (chain, collapsed[1]),
        (
            &[],
            &[
                "path", "explode", "--tree", T, "--link", "LINK-001", "--asm", "ASM-001",
                "--label", "m",
            ],
        ),
        (
            collapsed,
            &[
                "path",
                "replace",
                "--tree",
                T,
                "--macro-link",
                "MACRO-001",
                "--by-node",
                "RC-002",
            ],
        ),
        (&[], &["tree", "clone", T, "--name", "copia"]),
    ];
    for (setup, cmd) in cases {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        fixture(dir);
        for step in *setup {
            run_ok(dir, step);
        }
        rm_counters(dir);
        let json = run_ok(dir, cmd);
        assert_eq!(rebuilt(&json).len(), 1, "{cmd:?}: {json}");
        assert_eq!(rebuilt(&json)[0]["reason"], "missing", "{cmd:?}: {json}");
    }
}
