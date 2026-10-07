//! v0.5.0 (ADR-016) — global integrity of `node split` / `node rm`.
//!
//! Every failure case checks `success=false`, the exact code, an identical
//! `nodes/`+`trees/`+`knowledge/` snapshot **and** identical `.ltp/counters.json`.
//! Every success case ends with a clean `validate` and no file mentioning the
//! removed node.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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

fn error_codes(json: &Value) -> Vec<String> {
    json["errors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["code"].as_str().unwrap().to_string())
        .collect()
}

fn add_node(dir: &Path, label: &str, node_type: &str) -> String {
    run_ok(dir, &["node", "add", label, "--type", node_type])["data"]["id"]
        .as_str()
        .unwrap()
        .to_string()
}

fn new_tree(dir: &Path, kind: &str, name: &str) -> String {
    run_ok(dir, &["tree", "new", kind, name])["data"]["id"]
        .as_str()
        .unwrap()
        .to_string()
}

fn attach(dir: &Path, tree: &str, nodes: &[&str]) {
    for node in nodes {
        run_ok(dir, &["tree", "attach", "--tree", tree, "--node", node]);
    }
}

fn connect(dir: &Path, tree: &str, from: &str, to: &str) -> String {
    run_ok(
        dir,
        &[
            "link", "connect", "--tree", tree, "--from", from, "--to", to,
        ],
    )["data"]["created_links"][0]
        .as_str()
        .unwrap()
        .to_string()
}

fn feedback(dir: &Path, tree: &str, from: &str, to: &str) {
    run_ok(
        dir,
        &[
            "link", "feedback", "--tree", tree, "--from", from, "--to", to, "--type", "positive",
        ],
    );
}

fn nbr_add(dir: &Path, tree: &str, source: &str) -> String {
    run_ok(
        dir,
        &["nbr", "add", "--tree", tree, "--source-node", source],
    );
    let branches = read_json(&tree_path(dir, tree))["nbr_branches"]
        .as_array()
        .unwrap()
        .clone();
    branches.last().unwrap()["id"].as_str().unwrap().to_string()
}

fn nbr_connect(dir: &Path, tree: &str, nbr: &str, from: &str, to: &str) {
    run_ok(
        dir,
        &[
            "link", "connect", "--tree", tree, "--nbr", nbr, "--from", from, "--to", to,
        ],
    );
}

fn collapse(dir: &Path, tree: &str, from: &str, to: &str) -> String {
    run_ok(
        dir,
        &[
            "path", "collapse", "--tree", tree, "--from", from, "--to", to, "--label", "salto",
        ],
    )["data"]["macro_edge_id"]
        .as_str()
        .unwrap()
        .to_string()
}

fn macro_add(dir: &Path, tree: &str, from: &str, to: &str) -> String {
    run_ok(
        dir,
        &[
            "macro", "add", "--tree", tree, "--from", from, "--to", to, "--label", "reserva",
        ],
    )["data"]["macro_edge_id"]
        .as_str()
        .unwrap()
        .to_string()
}

fn split(dir: &Path, id: &str, tree: &str) -> (Value, i32) {
    run_ltp(
        dir,
        &[
            "node", "split", id, "--into", "primero", "segundo", "--tree", tree,
        ],
    )
}

/// Successful split: returns `(output, first, second)`.
fn split_ok(dir: &Path, id: &str, tree: &str) -> (Value, String, String) {
    let (out, code) = split(dir, id, tree);
    assert_eq!(code, 0, "split failed: {out}");
    let child = |i: usize| {
        out["data"]["new_nodes"][i]["id"]
            .as_str()
            .unwrap()
            .to_string()
    };
    let (first, second) = (child(0), child(1));
    (out, first, second)
}

fn tree_path(dir: &Path, id: &str) -> PathBuf {
    dir.join("trees").join(format!("{id}.json"))
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn tree_json(dir: &Path, id: &str) -> Value {
    read_json(&tree_path(dir, id))
}

/// Snapshot of every file under nodes/, trees/ and knowledge/ (path → bytes).
fn snapshot(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    for sub in ["nodes", "trees", "knowledge"] {
        let Ok(read) = std::fs::read_dir(dir.join(sub)) else {
            continue;
        };
        let mut entries: Vec<_> = read.flatten().map(|e| e.path()).collect();
        entries.sort();
        for p in entries {
            out.push((
                p.strip_prefix(dir).unwrap().display().to_string(),
                std::fs::read(&p).unwrap(),
            ));
        }
    }
    out
}

fn counters(dir: &Path) -> Vec<u8> {
    std::fs::read(dir.join(".ltp").join("counters.json")).unwrap_or_default()
}

/// Asserts a failed command left every byte (and every counter) untouched.
fn assert_untouched(dir: &Path, before: &[(String, Vec<u8>)], counters_before: &[u8]) {
    assert_eq!(snapshot(dir), before, "a failed command must write 0 bytes");
    assert_eq!(counters(dir), counters_before, "no ID may be burnt");
}

/// No file under nodes/ or trees/ mentions `id` any more.
fn assert_gone(dir: &Path, id: &str) {
    let needle = format!("\"{id}\"");
    for (path, bytes) in snapshot(dir) {
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            !text.contains(&needle),
            "{path} still mentions {id}:\n{text}"
        );
    }
}

fn assert_validate_clean(dir: &Path) {
    let (out, code) = run_ltp(dir, &["validate"]);
    assert_eq!(code, 0, "validate must pass: {out}");
    assert!(
        !error_codes(&out).contains(&"REFERENTIAL_INTEGRITY_VIOLATION".to_string()),
        "{out}"
    );
}

fn edge_pairs(edges: &Value) -> Vec<(Vec<String>, String)> {
    edges
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            let from = e["from"]
                .as_array()
                .unwrap()
                .iter()
                .map(|f| f.as_str().unwrap().to_string())
                .collect();
            (from, e["to"].as_str().unwrap().to_string())
        })
        .collect()
}

fn s(v: &str) -> String {
    v.to_string()
}

fn init(dir: &Path) {
    run_ok(dir, &["init", "--name", "v050"]);
}

// --- S: node split -----------------------------------------------------------

// S1 — X in the trunk of T1 and T2; split from T1 rewrites T2 too.
#[test]
fn s1_split_rewrites_every_tree() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    init(d);
    let t1 = new_tree(d, "crt", "uno");
    let t2 = new_tree(d, "crt", "dos");
    let a = add_node(d, "a", "UDE");
    let x = add_node(d, "x", "UDE");
    let b = add_node(d, "b", "UDE");
    let c = add_node(d, "c", "UDE");
    attach(d, &t1, &[&a, &x, &b]);
    connect(d, &t1, &a, &x);
    connect(d, &t1, &x, &b);
    attach(d, &t2, &[&a, &x, &c]);
    connect(d, &t2, &a, &x);
    connect(d, &t2, &x, &c);

    let (out, first, second) = split_ok(d, &x, &t1);

    let mut expected = vec![t1.clone(), t2.clone()];
    expected.sort();
    assert_eq!(out["data"]["affected_trees"], json!(expected), "{out}");
    assert_eq!(out["data"]["tree_id"], t1.as_str());
    assert_eq!(
        edge_pairs(&tree_json(d, &t2)["edges"]),
        vec![(vec![a], first), (vec![second], c)]
    );
    assert_gone(d, &x);
    assert_validate_clean(d);
}

// S2 — X only in an NBR branch of T2 (never in its trunk) and in the trunk of T1.
#[test]
fn s2_branch_only_membership_is_rewritten() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    init(d);
    let t1 = new_tree(d, "crt", "uno");
    let t2 = new_tree(d, "frt", "dos");
    let x = add_node(d, "x", "UDE");
    let inj = add_node(d, "inyeccion", "INJ");
    let z = add_node(d, "z", "UDE");
    attach(d, &t1, &[&x]);
    attach(d, &t2, &[&inj]);
    let nbr = nbr_add(d, &t2, &inj);
    nbr_connect(d, &t2, &nbr, &inj, &x);
    nbr_connect(d, &t2, &nbr, &x, &z);

    let (out, first, second) = split_ok(d, &x, &t1);

    assert!(
        out["data"]["affected_trees"]
            .as_array()
            .unwrap()
            .contains(&json!(t2)),
        "{out}"
    );
    let tree = tree_json(d, &t2);
    assert_eq!(
        edge_pairs(&tree["nbr_branches"][0]["edges"]),
        vec![(vec![inj], first), (vec![second], z)]
    );
    assert_eq!(
        tree["nodes"].as_array().unwrap().len(),
        1,
        "trunk untouched"
    );
    assert_gone(d, &x);
    assert_validate_clean(d);
}

// S3 — X is the source_node of an NBR branch in T2 ⇒ source = first, branch kept.
#[test]
fn s3_nbr_source_goes_to_first() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    init(d);
    let t1 = new_tree(d, "frt", "uno");
    let t2 = new_tree(d, "frt", "dos");
    let x = add_node(d, "x", "INJ");
    let side = add_node(d, "colateral", "UDE");
    attach(d, &t1, &[&x]);
    attach(d, &t2, &[&x]);
    let nbr = nbr_add(d, &t2, &x);
    nbr_connect(d, &t2, &nbr, &x, &side);

    let (_, first, second) = split_ok(d, &x, &t1);

    let branch = &tree_json(d, &t2)["nbr_branches"][0];
    assert_eq!(branch["id"], nbr.as_str());
    assert_eq!(branch["source_node"], first.as_str());
    assert_eq!(
        edge_pairs(&branch["edges"]),
        vec![(vec![second], side)],
        "outbound branch edge leaves from second"
    );
    assert_gone(d, &x);
    assert_validate_clean(d);
}

// S4 — feedback X→W and Y→X in the --tree itself and in T2.
#[test]
fn s4_feedback_is_redirected_in_every_tree() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    init(d);
    let t1 = new_tree(d, "crt", "uno");
    let t2 = new_tree(d, "crt", "dos");
    let w = add_node(d, "w", "UDE");
    let x = add_node(d, "x", "UDE");
    let y = add_node(d, "y", "UDE");
    for t in [&t1, &t2] {
        attach(d, t, &[&w, &x, &y]);
        connect(d, t, &w, &x);
        connect(d, t, &x, &y);
        feedback(d, t, &y, &x);
        feedback(d, t, &x, &w);
    }

    let (_, first, second) = split_ok(d, &x, &t1);

    for t in [&t1, &t2] {
        let fb: Vec<(String, String)> = tree_json(d, t)["feedback_edges"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| (s(f["from"].as_str().unwrap()), s(f["to"].as_str().unwrap())))
            .collect();
        assert_eq!(
            fb,
            vec![(y.clone(), first.clone()), (second.clone(), w.clone())],
            "tree {t}"
        );
    }
    assert_gone(d, &x);
    assert_validate_clean(d);
}

// S5 — overlays with X as from, as to and as interior; gather reports nothing dangling.
#[test]
fn s5_overlays_are_redirected() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    init(d);
    let t_from = new_tree(d, "crt", "desde");
    let t_to = new_tree(d, "crt", "hacia");
    let t_in = new_tree(d, "crt", "interior");
    let a = add_node(d, "a", "UDE");
    let x = add_node(d, "x", "UDE");
    let b = add_node(d, "b", "UDE");
    let c = add_node(d, "c", "UDE");

    attach(d, &t_from, &[&x, &b, &c]);
    connect(d, &t_from, &x, &b);
    connect(d, &t_from, &b, &c);
    let m_from = collapse(d, &t_from, &x, &c);

    attach(d, &t_to, &[&a, &b, &x]);
    connect(d, &t_to, &a, &b);
    connect(d, &t_to, &b, &x);
    let m_to = collapse(d, &t_to, &a, &x);

    attach(d, &t_in, &[&a, &x, &b]);
    connect(d, &t_in, &a, &x);
    connect(d, &t_in, &x, &b);
    let m_in = collapse(d, &t_in, &a, &b);

    let (_, first, second) = split_ok(d, &x, &t_from);

    let m = &tree_json(d, &t_from)["macro_edges"][0];
    assert_eq!(m["from"], second.as_str(), "macro from ⇒ second");
    let m = &tree_json(d, &t_to)["macro_edges"][0];
    assert_eq!(m["to"], first.as_str(), "macro to ⇒ first");
    let m = &tree_json(d, &t_in)["macro_edges"][0];
    assert_eq!(m["interior_nodes"], json!([first, second]));

    for (t, m) in [(&t_from, &m_from), (&t_to, &m_to), (&t_in, &m_in)] {
        let out = run_ok(
            d,
            &["macro-assume", "gather", "--tree", t, "--macro-link", m],
        );
        assert_eq!(out["data"]["diff"]["dangling"], json!([]), "{out}");
    }
    assert_gone(d, &x);
    assert_validate_clean(d);
}

// S6 — reservations with X as endpoint keep working after the split.
#[test]
fn s6_reservations_stay_usable() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    init(d);
    let t1 = new_tree(d, "crt", "uno");
    let t2 = new_tree(d, "crt", "dos");
    let w = add_node(d, "w", "UDE");
    let x = add_node(d, "x", "UDE");
    let z = add_node(d, "z", "UDE");
    attach(d, &t1, &[&x]);
    attach(d, &t2, &[&w, &x, &z]);
    let out_res = macro_add(d, &t2, &x, &z);
    let in_res = macro_add(d, &t2, &w, &x);

    let (_, first, second) = split_ok(d, &x, &t1);

    let promoted = run_ok(
        d,
        &["macro", "promote", "--tree", &t2, "--macro-link", &out_res],
    );
    let link = promoted["data"]["created_link"].as_str().unwrap();
    run_ok(
        d,
        &[
            "macro",
            "expand",
            "--tree",
            &t2,
            "--macro-link",
            &in_res,
            "--steps",
            "paso",
        ],
    );
    let tree = tree_json(d, &t2);
    let edges = tree["edges"].as_array().unwrap();
    let promoted_edge = edges.iter().find(|e| e["id"] == link).unwrap();
    assert_eq!(promoted_edge["from"], json!([second]));
    assert_eq!(promoted_edge["to"], z.as_str());
    assert!(
        edges.iter().any(|e| e["to"] == first.as_str()),
        "expand chain ends at first: {tree}"
    );
    assert_gone(d, &x);
    assert_validate_clean(d);
}

// S7 — X absent from --tree but present elsewhere ⇒ NODE_NOT_IN_TREE (regression).
#[test]
fn s7_node_not_in_context_tree() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    init(d);
    let t1 = new_tree(d, "crt", "uno");
    let t3 = new_tree(d, "crt", "tres");
    let x = add_node(d, "x", "UDE");
    attach(d, &t1, &[&x]);
    let (before, ctr) = (snapshot(d), counters(d));

    let (out, code) = split(d, &x, &t3);
    assert_ne!(code, 0);
    assert_eq!(out["success"], false);
    assert_eq!(error_codes(&out), vec!["NODE_NOT_IN_TREE"]);
    assert_untouched(d, &before, &ctr);
}

// S8 — unknown --tree ⇒ TREE_NOT_FOUND.
#[test]
fn s8_unknown_tree() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    init(d);
    let t1 = new_tree(d, "crt", "uno");
    let x = add_node(d, "x", "UDE");
    attach(d, &t1, &[&x]);
    let (before, ctr) = (snapshot(d), counters(d));

    let (out, _) = split(d, &x, "tree-crt-nope");
    assert_eq!(out["success"], false);
    assert_eq!(error_codes(&out), vec!["TREE_NOT_FOUND"]);
    assert_untouched(d, &before, &ctr);
}

/// T1 and T2 both contain X with edges; returns `(t1, t2, x)`.
fn two_trees_with_x(d: &Path) -> (String, String, String) {
    init(d);
    let t1 = new_tree(d, "crt", "uno");
    let t2 = new_tree(d, "crt", "dos");
    let a = add_node(d, "a", "UDE");
    let x = add_node(d, "x", "UDE");
    for t in [&t1, &t2] {
        attach(d, t, &[&a, &x]);
        connect(d, t, &a, &x);
    }
    (t1, t2, x)
}

// S9 — a corrupt tree anywhere ⇒ IO_ERROR {tree_id}, 0 bytes, counters intact (D-4).
#[test]
fn s9_corrupt_tree_fails_closed() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let (t1, _, x) = two_trees_with_x(d);
    let corrupt = new_tree(d, "crt", "zzz-roto");
    std::fs::write(tree_path(d, &corrupt), "{ not json").unwrap();
    let (before, ctr) = (snapshot(d), counters(d));

    let (out, _) = split(d, &x, &t1);
    assert_eq!(out["success"], false);
    assert_eq!(error_codes(&out), vec!["IO_ERROR"], "{out}");
    assert_eq!(out["errors"][0]["tree_id"], corrupt.as_str(), "{out}");
    assert_untouched(d, &before, &ctr);
}

// S9b — the corrupt tree is --tree itself ⇒ IO_ERROR, not TREE_NOT_FOUND.
#[test]
fn s9b_corrupt_context_tree_is_io_error() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let (t1, _, x) = two_trees_with_x(d);
    std::fs::write(tree_path(d, &t1), "{ not json").unwrap();
    let (before, ctr) = (snapshot(d), counters(d));

    let (out, _) = split(d, &x, &t1);
    assert_eq!(error_codes(&out), vec!["IO_ERROR"], "{out}");
    assert_eq!(out["errors"][0]["tree_id"], t1.as_str(), "{out}");
    assert_untouched(d, &before, &ctr);
}

// S10 — label count ≠ 2 via MCP ⇒ INVALID_ARGS without writing.
#[test]
fn s10_label_count_must_be_two() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let (t1, _, x) = two_trees_with_x(d);
    let (before, ctr) = (snapshot(d), counters(d));

    for into in [json!([]), json!(["solo"]), json!(["a", "b", "c"])] {
        let out = mcp_call(
            d,
            "ltp/node_split",
            json!({"id": x, "into": into, "tree": t1}),
        );
        assert_eq!(out["success"], false, "{into}: {out}");
        assert_eq!(error_codes(&out), vec!["INVALID_ARGS"], "{into}: {out}");
        assert_untouched(d, &before, &ctr);
    }
}

// S11 — a tree without X is not rewritten (bytes identical even if non-canonical).
#[test]
fn s11_untouched_tree_is_not_rewritten() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let (t1, t2, x) = two_trees_with_x(d);
    let t4 = new_tree(d, "crt", "cuatro");
    let other = add_node(d, "otro", "UDE");
    attach(d, &t4, &[&other]);
    // Compact (non-canonical) JSON: any save would change its bytes.
    let compact = serde_json::to_string(&tree_json(d, &t4)).unwrap();
    std::fs::write(tree_path(d, &t4), &compact).unwrap();

    let (out, _, _) = split_ok(d, &x, &t1);

    assert_eq!(std::fs::read_to_string(tree_path(d, &t4)).unwrap(), compact);
    let mut expected = vec![t1, t2];
    expected.sort();
    assert_eq!(out["data"]["affected_trees"], json!(expected));
}

// S12 — chained split of `second` across trees; IDs sequential without gaps.
#[test]
fn s12_chained_split_stays_consistent() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let (t1, t2, x) = two_trees_with_x(d);
    let b = add_node(d, "b", "UDE");
    for t in [&t1, &t2] {
        attach(d, t, &[&b]);
        connect(d, t, &x, &b);
    }

    let (_, first, second) = split_ok(d, &x, &t1);
    let (_, third, fourth) = split_ok(d, &second, &t2);

    assert_eq!(
        [&first, &second, &third, &fourth].map(|s| s.as_str()),
        ["UDE-004", "UDE-005", "UDE-006", "UDE-007"]
    );
    for t in [&t1, &t2] {
        let pairs = edge_pairs(&tree_json(d, t)["edges"]);
        assert!(
            pairs.contains(&(vec![fourth.clone()], b.clone())),
            "{t}: {pairs:?}"
        );
        assert!(pairs.iter().any(|(_, to)| *to == first), "{t}: {pairs:?}");
    }
    assert_gone(d, &x);
    assert_gone(d, &second);
    assert_validate_clean(d);
}

// S13 — undo restores every byte (all trees and referencing nodes); redo is exact.
#[test]
fn s13_undo_redo_are_byte_exact() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let (t1, _, x) = two_trees_with_x(d);
    let referrer = add_node(d, "referente", "UDE");
    let path = d.join("nodes").join(format!("{referrer}.json"));
    let mut raw = read_json(&path);
    raw["metadata"]["refs"] = json!([{"node": x, "tree": null}]);
    std::fs::write(&path, serde_json::to_string_pretty(&raw).unwrap()).unwrap();
    // Canonicalise the hand-edited node so the snapshot compares like with like.
    run_ok(d, &["node", "edit", &referrer, "--label", "referente"]);
    let before = snapshot(d);

    split_ok(d, &x, &t1);
    let after = snapshot(d);
    assert_ne!(before, after);

    run_ok(d, &["undo"]);
    assert_eq!(snapshot(d), before, "undo must restore every byte");
    run_ok(d, &["redo"]);
    assert_eq!(snapshot(d), after, "redo must reproduce the split");
}

/// T with nodes [A, X, B], A→X→B, a feedback B→A and an overlay A⇒B over X.
fn local_fixture(d: &Path) -> (String, String) {
    init(d);
    let t = new_tree(d, "crt", "local");
    let a = add_node(d, "a", "UDE");
    let x = add_node(d, "x", "UDE");
    let b = add_node(d, "b", "UDE");
    attach(d, &t, &[&a, &x, &b]);
    connect(d, &t, &a, &x);
    connect(d, &t, &x, &b);
    feedback(d, &t, &b, &a);
    collapse(d, &t, &a, &b);
    (t, x)
}

// S14 — the split is local and in position: the file is the previous one with X
// replaced in place, nothing else reordered; the same scenario gives identical bytes.
#[test]
fn s14_split_is_local_and_in_position() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let (t, x) = local_fixture(d);
    let mut expected = tree_json(d, &t);

    let (_, first, second) = split_ok(d, &x, &t);

    let children = || {
        vec![
            json!({"ref": first, "role": null}),
            json!({"ref": second, "role": null}),
        ]
    };
    let nodes = expected["nodes"].as_array_mut().unwrap();
    let pos = nodes.iter().position(|n| n["ref"] == x.as_str()).unwrap();
    nodes.splice(pos..=pos, children());
    for e in expected["edges"].as_array_mut().unwrap() {
        if e["to"] == x.as_str() {
            e["to"] = json!(first);
        }
        for f in e["from"].as_array_mut().unwrap() {
            if *f == x.as_str() {
                *f = json!(second);
            }
        }
    }
    expected["macro_edges"][0]["interior_nodes"] = json!([first, second]);
    let actual = tree_json(d, &t);
    // `role` is omitted when None; compare the trees field by field-normalised JSON.
    assert_eq!(actual, expected);

    let twin = tempfile::tempdir().unwrap();
    let (t2, x2) = local_fixture(twin.path());
    split_ok(twin.path(), &x2, &t2);
    assert_eq!(
        snapshot(twin.path()),
        snapshot(d),
        "same scenario ⇒ same bytes"
    );
}

// S15 — CLI ↔ MCP parity: same data, same files.
#[test]
fn s15_cli_mcp_parity() {
    let cli = tempfile::tempdir().unwrap();
    let mcp = tempfile::tempdir().unwrap();
    let (t1, _, x) = two_trees_with_x(cli.path());
    two_trees_with_x(mcp.path());

    let (cli_out, _, _) = split_ok(cli.path(), &x, &t1);
    let mcp_out = mcp_call(
        mcp.path(),
        "ltp/node_split",
        json!({"id": x, "into": ["primero", "segundo"], "tree": t1}),
    );
    assert_eq!(mcp_out["success"], true, "{mcp_out}");
    assert_eq!(cli_out["data"], mcp_out["data"]);
    assert_eq!(snapshot(cli.path()), snapshot(mcp.path()));
}

// S16 — the split never creates cycles: A→X→B with feedback B→A.
#[test]
fn s16_split_creates_no_cycle() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let (t, x) = local_fixture(d);

    let (out, _, _) = split_ok(d, &x, &t);
    assert_eq!(out["graph_health"]["valid_dag"], true);

    let (v, code) = run_ltp(d, &["validate"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["graph_health"]["valid_dag"], true);
    assert!(!error_codes(&v).contains(&"CIRCULAR_DEPENDENCY_DETECTED".to_string()));
}
