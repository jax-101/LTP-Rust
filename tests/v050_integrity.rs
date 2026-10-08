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

// --- M: node rm and macros ---------------------------------------------------

fn rm(dir: &Path, ids: &str) -> (Value, i32) {
    run_ltp(dir, &["node", "rm", ids])
}

fn rm_ok(dir: &Path, ids: &str) -> Value {
    let (out, code) = rm(dir, ids);
    assert_eq!(code, 0, "rm failed: {out}");
    out
}

fn assume_add(dir: &Path, tree: &str, link: &str, text: &str) -> String {
    run_ok(
        dir,
        &[
            "assume", "add", "--tree", tree, "--link", link, "--text", text,
        ],
    )["data"]["id"]
        .as_str()
        .unwrap()
        .to_string()
}

fn macro_assume_add(dir: &Path, tree: &str, m: &str, text: &str, projection: &[&str]) -> String {
    let mut args = vec![
        "macro-assume",
        "add",
        "--tree",
        tree,
        "--macro-link",
        m,
        "--text",
        text,
    ];
    for p in projection {
        args.push("--projection");
        args.push(p);
    }
    run_ok(dir, &args)["data"]["created_assumption_id"]
        .as_str()
        .unwrap()
        .to_string()
}

fn codes_of(warnings: &Value) -> Vec<String> {
    warnings
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["code"].as_str().unwrap().to_string())
        .collect()
}

/// Rewrites a tree file by hand (simulates legacy data / v0.4.0 damage).
fn edit_tree(dir: &Path, id: &str, f: impl FnOnce(&mut Value)) {
    let mut tree = tree_json(dir, id);
    f(&mut tree);
    std::fs::write(
        tree_path(dir, id),
        serde_json::to_string_pretty(&tree).unwrap(),
    )
    .unwrap();
}

/// CRT with the chain A → B → E collapsed into an overlay A ⇒ E.
/// Returns `(tree, [a, b, e], [l1, l2], macro)`.
fn linear_overlay(d: &Path) -> (String, [String; 3], [String; 2], String) {
    init(d);
    let t = new_tree(d, "crt", "lineal");
    let a = add_node(d, "a", "UDE");
    let b = add_node(d, "b", "UDE");
    let e = add_node(d, "e", "UDE");
    attach(d, &t, &[&a, &b, &e]);
    let l1 = connect(d, &t, &a, &b);
    let l2 = connect(d, &t, &b, &e);
    let m = collapse(d, &t, &a, &e);
    (t, [a, b, e], [l1, l2], m)
}

// M1 — removing the `from` of an overlay with 2 MacroAssumptions removes it, with
// MACRO_EDGE_REMOVED {endpoint_removed, overlay, assumption_ids in storage order}.
#[test]
fn m1_endpoint_removal_drops_overlay_with_warning() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let (t, [a, _, e], _, m) = linear_overlay(d);
    let masm1 = macro_assume_add(d, &t, &m, "primero", &[]);
    let masm2 = macro_assume_add(d, &t, &m, "segundo", &[]);
    let before = snapshot(d);

    let out = rm_ok(d, &a);

    let removed = warnings_with(&out, "MACRO_EDGE_REMOVED");
    assert_eq!(removed.len(), 1, "{out}");
    let w = &removed[0];
    assert_eq!(w["tree_id"], t.as_str());
    assert_eq!(w["macro_link"], m.as_str());
    assert_eq!(w["reason"], "endpoint_removed");
    assert_eq!(w["status"], "overlay");
    assert_eq!(w["from"], a.as_str());
    assert_eq!(w["to"], e.as_str());
    assert_eq!(w["assumption_ids"], json!([masm1, masm2]));
    assert_eq!(tree_json(d, &t)["macro_edges"], json!([]));
    assert_validate_clean(d);

    run_ok(d, &["undo"]);
    assert_eq!(snapshot(d), before, "undo must restore every byte");
}

// M2 — removing the `to` of a pure reservation removes it (status=reservation).
#[test]
fn m2_reservation_endpoint_removal() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    init(d);
    let t = new_tree(d, "crt", "reserva");
    let a = add_node(d, "a", "UDE");
    let e = add_node(d, "e", "UDE");
    attach(d, &t, &[&a, &e]);
    let m = macro_add(d, &t, &a, &e);

    let out = rm_ok(d, &e);

    let removed = warnings_with(&out, "MACRO_EDGE_REMOVED");
    assert_eq!(removed.len(), 1, "{out}");
    assert_eq!(removed[0]["macro_link"], m.as_str());
    assert_eq!(removed[0]["status"], "reservation");
    assert_eq!(removed[0]["assumption_ids"], json!([]));
    assert_eq!(out["data"]["affected_trees"], json!([t]));
    assert!(validate_warnings(d, "LONG_ARROW_RESERVATION_PENDING").is_empty());
}

/// Diamond A→B→E, A→C→E collapsed into A ⇒ E; an ASM on A→B projected by the summary.
/// Returns `(tree, [a, b, c, e], macro, asm)`.
fn diamond_overlay(d: &Path) -> (String, [String; 4], String, String) {
    init(d);
    let t = new_tree(d, "crt", "diamante");
    let a = add_node(d, "a", "UDE");
    let b = add_node(d, "b", "UDE");
    let c = add_node(d, "c", "UDE");
    let e = add_node(d, "e", "UDE");
    attach(d, &t, &[&a, &b, &c, &e]);
    let ab = connect(d, &t, &a, &b);
    connect(d, &t, &b, &e);
    connect(d, &t, &a, &c);
    connect(d, &t, &c, &e);
    let asm = assume_add(d, &t, &ab, "b siempre ocurre");
    let m = collapse(d, &t, &a, &e);
    (t, [a, b, c, e], m, asm)
}

// M3 — an interior removal trims the overlay; rm itself is silent about the stale
// summary, `validate` reports it with exactly the dangling ASM (ADR-016 D-3d).
#[test]
fn m3_interior_trim_keeps_overlay_and_validate_reports_stale() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let (t, [_, b, c, _], m, asm) = diamond_overlay(d);
    macro_assume_add(d, &t, &m, "resumen", &[&asm]);
    let refs_before = tree_json(d, &t)["macro_edges"][0]["assumptions"].clone();

    let out = rm_ok(d, &b);

    assert!(
        warnings_with(&out, "MACRO_EDGE_REMOVED").is_empty(),
        "{out}"
    );
    assert!(
        warnings_with(&out, "LONG_ARROW_SUMMARY_STALE").is_empty(),
        "{out}"
    );
    assert_eq!(out["data"]["affected_trees"], json!([t]));
    let macro_edge = &tree_json(d, &t)["macro_edges"][0];
    assert_eq!(macro_edge["interior_nodes"], json!([c]));
    assert_eq!(
        macro_edge["assumptions"], refs_before,
        "projection_refs untouched"
    );

    let stale = validate_warnings(d, "LONG_ARROW_SUMMARY_STALE");
    assert_eq!(stale.len(), 1, "{stale:?}");
    assert_eq!(stale[0]["dangling"], json!([asm]));
    assert_validate_clean(d);

    let inj = add_node(d, "inyeccion", "INJ");
    run_ok(
        d,
        &[
            "path",
            "replace",
            "--tree",
            &t,
            "--macro-link",
            &m,
            "--by-node",
            &inj,
        ],
    );
    assert_validate_clean(d);
}

// M4 — linear overlay; removing its only interior node ⇒ interior_emptied.
#[test]
fn m4_linear_overlay_interior_emptied() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let (t, [_, b, _], _, m) = linear_overlay(d);

    let out = rm_ok(d, &b);

    let removed = warnings_with(&out, "MACRO_EDGE_REMOVED");
    assert_eq!(removed.len(), 1, "{out}");
    assert_eq!(removed[0]["macro_link"], m.as_str());
    assert_eq!(removed[0]["reason"], "interior_emptied");
    assert_eq!(tree_json(d, &t)["macro_edges"], json!([]));
    assert_validate_clean(d);
}

// M5 — removing both endpoints in one batch ⇒ one warning for that macro.
#[test]
fn m5_batch_with_both_endpoints_warns_once() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let (_, [a, _, e], _, m) = linear_overlay(d);

    let out = rm_ok(d, &format!("{a},{e}"));

    let removed = warnings_with(&out, "MACRO_EDGE_REMOVED");
    assert_eq!(removed.len(), 1, "{out}");
    assert_eq!(removed[0]["macro_link"], m.as_str());
    assert_eq!(removed[0]["reason"], "endpoint_removed");
}

// M6 — an unknown ID in the batch ⇒ NODE_NOT_FOUND, nothing pruned.
#[test]
fn m6_unknown_id_prunes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let (_, [a, _, _], _, _) = linear_overlay(d);
    let (before, ctr) = (snapshot(d), counters(d));

    let (out, _) = rm(d, &format!("{a},UDE-999"));
    assert_eq!(out["success"], false);
    assert_eq!(error_codes(&out), vec!["NODE_NOT_FOUND"]);
    assert_untouched(d, &before, &ctr);
}

// M7 — X is an endpoint of a macro in T1 and interior of another in T2; an
// unrelated macro in T1 stays identical.
#[test]
fn m7_each_macro_follows_its_rule_across_trees() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    init(d);
    let t1 = new_tree(d, "crt", "uno");
    let t2 = new_tree(d, "crt", "dos");
    let [x, b, c, p, q, r, a, e] =
        ["x", "b", "c", "p", "q", "r", "a", "e"].map(|l| add_node(d, l, "UDE"));
    attach(d, &t1, &[&x, &b, &c, &p, &q, &r]);
    connect(d, &t1, &x, &b);
    connect(d, &t1, &b, &c);
    let m_end = collapse(d, &t1, &x, &c);
    connect(d, &t1, &p, &q);
    connect(d, &t1, &q, &r);
    collapse(d, &t1, &p, &r);
    attach(d, &t2, &[&a, &x, &c, &e]);
    connect(d, &t2, &a, &x);
    connect(d, &t2, &x, &e);
    connect(d, &t2, &a, &c);
    connect(d, &t2, &c, &e);
    let m_int = collapse(d, &t2, &a, &e);
    let unrelated = tree_json(d, &t1)["macro_edges"][1].clone();

    let out = rm_ok(d, &x);

    let removed = warnings_with(&out, "MACRO_EDGE_REMOVED");
    assert_eq!(removed.len(), 1, "{out}");
    assert_eq!(removed[0]["macro_link"], m_end.as_str());
    let tree1 = tree_json(d, &t1);
    assert_eq!(tree1["macro_edges"], json!([unrelated]));
    let tree2 = tree_json(d, &t2);
    assert_eq!(tree2["macro_edges"][0]["id"], m_int.as_str());
    assert_eq!(tree2["macro_edges"][0]["interior_nodes"], json!([c]));
    assert_validate_clean(d);
}

// M8 — after M1, every macro command on the removed macro ⇒ MACRO_EDGE_NOT_FOUND.
#[test]
fn m8_removed_macro_is_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let (t, [a, _, _], _, m) = linear_overlay(d);
    rm_ok(d, &a);
    let inj = add_node(d, "inyeccion", "INJ");
    let (before, ctr) = (snapshot(d), counters(d));

    let attempts: [Vec<&str>; 3] = [
        vec![
            "macro",
            "expand",
            "--tree",
            &t,
            "--macro-link",
            &m,
            "--steps",
            "s",
        ],
        vec!["macro", "promote", "--tree", &t, "--macro-link", &m],
        vec![
            "path",
            "replace",
            "--tree",
            &t,
            "--macro-link",
            &m,
            "--by-node",
            &inj,
        ],
    ];
    for args in attempts {
        let (out, _) = run_ltp(d, &args);
        assert_eq!(
            error_codes(&out),
            vec!["MACRO_EDGE_NOT_FOUND"],
            "{args:?}: {out}"
        );
        assert_untouched(d, &before, &ctr);
    }
}

// M9 — a corrupt tree ⇒ IO_ERROR {tree_id}, nothing removed from the pool (D-4).
#[test]
fn m9_corrupt_tree_fails_closed() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let (_, [a, _, _], _, _) = linear_overlay(d);
    let corrupt = new_tree(d, "crt", "zzz-roto");
    std::fs::write(tree_path(d, &corrupt), "{ not json").unwrap();
    let (before, ctr) = (snapshot(d), counters(d));

    let (out, _) = rm(d, &a);
    assert_eq!(out["success"], false);
    assert_eq!(error_codes(&out), vec!["IO_ERROR"], "{out}");
    assert_eq!(out["errors"][0]["tree_id"], corrupt.as_str(), "{out}");
    assert_untouched(d, &before, &ctr);
}

// M10 — legacy macros (`"active"`, no `assumptions`) written by hand.
#[test]
fn m10_legacy_macros() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    init(d);
    let t = new_tree(d, "crt", "legacy");
    let quiet = new_tree(d, "crt", "quieto");
    let [a, b, e, o] = ["a", "b", "e", "o"].map(|l| add_node(d, l, "UDE"));
    attach(d, &t, &[&a, &b, &e]);
    let l1 = connect(d, &t, &a, &b);
    let l2 = connect(d, &t, &b, &e);
    attach(d, &quiet, &[&o]);
    let legacy = |id: &str, from: &str, to: &str, nodes: Value, links: Value| {
        json!({"id": id, "from": from, "to": to, "label": "viejo",
               "interior_nodes": nodes, "interior_links": links, "status": "active"})
    };
    let doomed = legacy("MACRO-050", &a, &e, json!([b]), json!([l1, l2]));
    let bystander = legacy("MACRO-051", &b, &e, json!([]), json!([]));
    edit_tree(d, &t, |tr| tr["macro_edges"] = json!([doomed, bystander]));
    let far = legacy("MACRO-052", &o, &o, json!([]), json!([]));
    edit_tree(d, &quiet, |tr| tr["macro_edges"] = json!([far]));
    let quiet_bytes = std::fs::read(tree_path(d, &quiet)).unwrap();

    let out = rm_ok(d, &a);

    let removed = warnings_with(&out, "MACRO_EDGE_REMOVED");
    assert_eq!(removed.len(), 1, "{out}");
    assert_eq!(removed[0]["macro_link"], "MACRO-050");
    assert_eq!(removed[0]["status"], "overlay");
    let mut expected = bystander;
    expected["status"] = json!("overlay");
    assert_eq!(tree_json(d, &t)["macro_edges"], json!([expected]));
    assert_eq!(std::fs::read(tree_path(d, &quiet)).unwrap(), quiet_bytes);
}

// M11 — deterministic warning order: by tree, then storage order (MACRO-998 before
// MACRO-1000, no textual sort); after NBR_BRANCH_REMOVED, before REFS_STRIPPED.
#[test]
fn m11_warning_order_is_deterministic() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    init(d);
    let ta = new_tree(d, "frt", "a");
    let tb = new_tree(d, "frt", "b");
    let x = add_node(d, "x", "INJ");
    let y = add_node(d, "y", "DE");
    let side = add_node(d, "colateral", "UDE");
    let referrer = add_node(d, "referente", "UDE");
    attach(d, &ta, &[&x, &y]);
    attach(d, &tb, &[&x, &y]);
    let nbr = nbr_add(d, &ta, &x);
    nbr_connect(d, &ta, &nbr, &x, &side);
    let m_a = macro_add(d, &ta, &x, &y);
    let reservation = |id: &str| {
        json!({"id": id, "from": x, "to": y, "label": "r",
               "interior_nodes": [], "interior_links": [], "status": "reservation"})
    };
    edit_tree(d, &tb, |tr| {
        tr["macro_edges"] = json!([reservation("MACRO-998"), reservation("MACRO-1000")])
    });
    let path = d.join("nodes").join(format!("{referrer}.json"));
    let mut raw = read_json(&path);
    raw["metadata"]["refs"] = json!([{"node": x, "tree": null}]);
    std::fs::write(&path, serde_json::to_string_pretty(&raw).unwrap()).unwrap();

    let out = rm_ok(d, &x);

    assert_eq!(
        codes_of(&out["warnings"]),
        vec![
            "NBR_BRANCH_REMOVED",
            "MACRO_EDGE_REMOVED",
            "MACRO_EDGE_REMOVED",
            "MACRO_EDGE_REMOVED",
            "REFS_STRIPPED"
        ],
        "{out}"
    );
    let order: Vec<(String, String)> = warnings_with(&out, "MACRO_EDGE_REMOVED")
        .iter()
        .map(|w| {
            (
                s(w["tree_id"].as_str().unwrap()),
                s(w["macro_link"].as_str().unwrap()),
            )
        })
        .collect();
    assert_eq!(
        order,
        vec![
            (ta, m_a),
            (tb.clone(), s("MACRO-998")),
            (tb, s("MACRO-1000")),
        ]
    );
}

// M12 — CLI ↔ MCP parity for `node rm`.
#[test]
fn m12_cli_mcp_parity() {
    let cli = tempfile::tempdir().unwrap();
    let mcp = tempfile::tempdir().unwrap();
    let (_, [a, _, _], _, _) = linear_overlay(cli.path());
    linear_overlay(mcp.path());

    let cli_out = rm_ok(cli.path(), &a);
    let mcp_out = mcp_call(mcp.path(), "ltp/node_rm", json!({"ids": [a]}));
    assert_eq!(mcp_out["success"], true, "{mcp_out}");
    assert_eq!(cli_out["data"], mcp_out["data"]);
    assert_eq!(cli_out["warnings"], mcp_out["warnings"]);
    assert_eq!(snapshot(cli.path()), snapshot(mcp.path()));
}

// M13 — ghost links (D3/F3): after `link disconnect` the ID stays in interior_links;
// removing B leaves no live interior link ⇒ interior_emptied. An empty overlay
// written by hand and not touched by the rm stays identical, without warning.
#[test]
fn m13_ghost_links_do_not_keep_an_overlay_alive() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let (t, [a, b, e], [l1, _], m) = linear_overlay(d);
    let empty = json!({"id": "MACRO-077", "from": a, "to": e, "label": "vacía",
                       "interior_nodes": [], "interior_links": [], "status": "overlay"});
    edit_tree(d, &t, |tr| {
        tr["macro_edges"]
            .as_array_mut()
            .unwrap()
            .push(empty.clone())
    });
    run_ok(d, &["link", "disconnect", "--tree", &t, "--links", &l1]);
    assert!(
        tree_json(d, &t)["macro_edges"][0]["interior_links"]
            .as_array()
            .unwrap()
            .contains(&json!(l1)),
        "precondition: link disconnect leaves a ghost"
    );

    let out = rm_ok(d, &b);

    let removed = warnings_with(&out, "MACRO_EDGE_REMOVED");
    assert_eq!(removed.len(), 1, "{out}");
    assert_eq!(removed[0]["macro_link"], m.as_str());
    assert_eq!(removed[0]["reason"], "interior_emptied");
    assert_eq!(tree_json(d, &t)["macro_edges"], json!([empty]));
}

// M14 — repeated IDs behave like a single one (CLI and MCP); undo is exact.
#[test]
fn m14_duplicate_ids_are_deduplicated() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let (_, [a, _, _], _, _) = linear_overlay(d);
    let before = snapshot(d);

    let out = rm_ok(d, &format!("{a},{a}"));
    assert_eq!(out["data"]["removed_nodes"], json!([a]));
    run_ok(d, &["undo"]);
    assert_eq!(snapshot(d), before, "undo must restore every byte");

    let mcp_out = mcp_call(d, "ltp/node_rm", json!({"ids": [a, a]}));
    assert_eq!(mcp_out["success"], true, "{mcp_out}");
    assert_eq!(mcp_out["data"]["removed_nodes"], json!([a]));
}

fn warnings_with(out: &Value, code: &str) -> Vec<Value> {
    out["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|w| w["code"] == code)
        .cloned()
        .collect()
}

/// Warnings with `code` from a `validate` run, across every tree in `data.details`.
fn validate_warnings(dir: &Path, code: &str) -> Vec<Value> {
    let (out, exit) = run_ltp(dir, &["validate"]);
    assert_eq!(exit, 0, "validate must pass: {out}");
    out["data"]["details"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|d| d["warnings"].as_array().unwrap().clone())
        .filter(|w| w["code"] == code)
        .collect()
}

// --- D: macro endpoints in expand / replace ----------------------------------

fn detach(dir: &Path, tree: &str, node: &str) {
    run_ok(dir, &["tree", "detach", "--tree", tree, "--node", node]);
}

/// The failure contract shared with `macro promote`: one `NODE_NOT_IN_TREE {node_id}`.
fn assert_endpoint_not_in_tree(out: &Value, node: &str) {
    assert_eq!(out["success"], false, "{out}");
    assert_eq!(error_codes(out), vec!["NODE_NOT_IN_TREE"], "{out}");
    assert_eq!(out["errors"][0]["node_id"], node, "{out}");
}

// D1 — reservation whose endpoint was detached ⇒ `macro expand` fails before minting
// (INT/LINK counters intact, 0 bytes). Both endpoints are covered; promote is the reference.
#[test]
fn d1_expand_rejects_detached_endpoint() {
    for detached_to in [true, false] {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        init(d);
        let t = new_tree(d, "crt", "reserva");
        let a = add_node(d, "a", "UDE");
        let e = add_node(d, "e", "UDE");
        attach(d, &t, &[&a, &e]);
        let m = macro_add(d, &t, &a, &e);
        let gone = if detached_to { &e } else { &a };
        detach(d, &t, gone);
        let (before, ctr) = (snapshot(d), counters(d));

        let attempts: [Vec<&str>; 2] = [
            vec![
                "macro",
                "expand",
                "--tree",
                &t,
                "--macro-link",
                &m,
                "--steps",
                "p,q",
            ],
            vec!["macro", "promote", "--tree", &t, "--macro-link", &m],
        ];
        for args in attempts {
            let (out, _) = run_ltp(d, &args);
            assert_endpoint_not_in_tree(&out, gone);
            assert_untouched(d, &before, &ctr);
        }
    }
}

// D2 — overlay with a detached endpoint ⇒ `path replace` fails before touching the pool:
// no node marked superseded, counters intact, 0 bytes.
#[test]
fn d2_replace_rejects_detached_endpoint() {
    for detached_to in [true, false] {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        let (t, [a, b, e], _, m) = linear_overlay(d);
        let inj = add_node(d, "inyeccion", "INJ");
        let gone = if detached_to { &e } else { &a };
        detach(d, &t, gone);
        let (before, ctr) = (snapshot(d), counters(d));

        let (out, _) = run_ltp(
            d,
            &[
                "path",
                "replace",
                "--tree",
                &t,
                "--macro-link",
                &m,
                "--by-node",
                &inj,
            ],
        );
        assert_endpoint_not_in_tree(&out, gone);
        assert_untouched(d, &before, &ctr);
        let interior = read_json(&d.join("nodes").join(format!("{b}.json")));
        assert_ne!(interior["metadata"]["status"], "superseded");
    }
}

// --- V: validate, full referential integrity ---------------------------------

/// `validate` (optionally `--tree`) ⇒ `(output, REFERENTIAL_INTEGRITY_VIOLATION errors)`.
fn integrity_errors(dir: &Path, tree: Option<&str>) -> (Value, Vec<Value>) {
    let mut args = vec!["validate"];
    if let Some(t) = tree {
        args.extend(["--tree", t]);
    }
    let (out, _) = run_ltp(dir, &args);
    let errors = out["data"]["details"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|d| d["errors"].as_array().unwrap().clone())
        .filter(|e| e["code"] == "REFERENTIAL_INTEGRITY_VIOLATION")
        .collect();
    (out, errors)
}

/// `(node_id, location, field)` of each violation, in output order.
fn slots(errors: &[Value]) -> Vec<(String, String, String)> {
    errors
        .iter()
        .map(|e| {
            let f = |k: &str| e[k].as_str().unwrap_or("").to_string();
            (f("node_id"), f("location"), f("field"))
        })
        .collect()
}

/// FRT with every structure: trunk A→B→C, feedback C→A, NBR on B with B→C, overlay A ⇒ C.
fn full_tree(d: &Path, name: &str) -> String {
    let t = new_tree(d, "frt", name);
    let a = add_node(d, "a", "INJ");
    let b = add_node(d, "b", "DE");
    let c = add_node(d, "c", "UDE");
    attach(d, &t, &[&a, &b, &c]);
    connect(d, &t, &a, &b);
    connect(d, &t, &b, &c);
    feedback(d, &t, &c, &a);
    let nbr = nbr_add(d, &t, &b);
    nbr_connect(d, &t, &nbr, &b, &c);
    collapse(d, &t, &a, &c);
    t
}

/// Hand-written damage (simulates v0.4.0): one distinct ghost per slot.
fn damage(d: &Path, t: &str) {
    edit_tree(d, t, |tr| {
        tr["nodes"]
            .as_array_mut()
            .unwrap()
            .push(json!({"ref": "UDE-901", "role": null}));
        tr["edges"][0]["from"] = json!(["UDE-902"]);
        tr["edges"][1]["to"] = json!("UDE-903");
        tr["feedback_edges"][0]["from"] = json!("UDE-904");
        tr["nbr_branches"][0]["source_node"] = json!("UDE-905");
        tr["nbr_branches"][0]["edges"][0]["to"] = json!("UDE-906");
        tr["macro_edges"][0]["from"] = json!("UDE-907");
        tr["macro_edges"][0]["to"] = json!("UDE-908");
        tr["macro_edges"][0]["interior_nodes"] = json!(["UDE-909"]);
    });
}

// V1 — one violation per damaged slot, with location/field/container, in fixed order.
#[test]
fn v1_every_structure_is_checked() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    init(d);
    let t = full_tree(d, "danado");
    damage(d, &t);

    let (out, errors) = integrity_errors(d, None);

    assert_eq!(out["success"], false, "{out}");
    let expect = |n: &str, l: &str, f: &str| (s(n), s(l), s(f));
    assert_eq!(
        slots(&errors),
        vec![
            expect("UDE-901", "nodes", "ref"),
            expect("UDE-902", "edges", "from"),
            expect("UDE-903", "edges", "to"),
            expect("UDE-904", "feedback_edges", "from"),
            expect("UDE-905", "nbr_branches", "source_node"),
            expect("UDE-906", "nbr_branches", "to"),
            expect("UDE-907", "macro_edges", "from"),
            expect("UDE-908", "macro_edges", "to"),
            expect("UDE-909", "macro_edges", "interior_nodes"),
        ],
        "{errors:?}"
    );
    for e in &errors {
        assert_eq!(e["tree_id"], t.as_str());
    }
    assert_eq!(errors[1]["edge_id"], "LINK-001");
    assert_eq!(errors[3]["feedback_id"], "FB-001");
    assert_eq!(errors[4]["nbr_id"], "NBR-001");
    assert_eq!(errors[5]["nbr_id"], "NBR-001");
    assert_eq!(errors[5]["edge_id"], "LINK-003");
    assert_eq!(errors[8]["macro_link"], "MACRO-001");
}

// V2 — `validate --tree T` only reports T's violations.
#[test]
fn v2_tree_filter_is_respected() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    init(d);
    let t1 = full_tree(d, "uno");
    let t2 = full_tree(d, "dos");
    damage(d, &t1);
    damage(d, &t2);

    let (_, errors) = integrity_errors(d, Some(&t2));
    assert_eq!(errors.len(), 9, "{errors:?}");
    assert!(errors.iter().all(|e| e["tree_id"] == t2.as_str()));

    let (_, all) = integrity_errors(d, None);
    assert_eq!(all.len(), 18);
}

// V3 — a node on disk but unreadable counts as existing: only NODE_UNREADABLE.
#[test]
fn v3_unreadable_node_is_not_a_violation() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    init(d);
    let t = full_tree(d, "ilegible");
    let node = tree_json(d, &t)["nodes"][1]["ref"]
        .as_str()
        .unwrap()
        .to_string();
    std::fs::write(d.join("nodes").join(format!("{node}.json")), "{ roto").unwrap();

    let (out, errors) = integrity_errors(d, None);
    assert!(errors.is_empty(), "{errors:?}");
    let unreadable = validate_warnings_of(&out, "NODE_UNREADABLE");
    assert_eq!(unreadable.len(), 1, "{out}");
    assert_eq!(unreadable[0]["node_id"], node.as_str());
}

fn validate_warnings_of(out: &Value, code: &str) -> Vec<Value> {
    out["data"]["details"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|d| d["warnings"].as_array().unwrap().clone())
        .filter(|w| w["code"] == code)
        .collect()
}

// V4a — exact v0.4.0 split damage (S1): T2 still points at the deleted original.
// `validate` detects it; X is still attached, so the materializers refuse it with
// NODE_NOT_FOUND (attached but absent from the pool).
#[test]
fn v4a_legacy_split_damage_is_detected() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    init(d);
    let t1 = new_tree(d, "crt", "uno");
    let t2 = new_tree(d, "crt", "dos");
    let [x, y, z] = ["x", "y", "z"].map(|l| add_node(d, l, "UDE"));
    attach(d, &t1, &[&x, &y]);
    attach(d, &t2, &[&x, &y, &z]);
    connect(d, &t2, &x, &y);
    let m = macro_add(d, &t2, &x, &z);
    // Old split: X deleted from the pool, only T1 rewritten.
    edit_tree(d, &t1, |tr| tr["nodes"] = json!([{"ref": y, "role": null}]));
    std::fs::remove_file(d.join("nodes").join(format!("{x}.json"))).unwrap();

    let (_, errors) = integrity_errors(d, None);
    let expect = |l: &str, f: &str| (x.clone(), s(l), s(f));
    assert_eq!(
        slots(&errors),
        vec![
            expect("nodes", "ref"),
            expect("edges", "from"),
            expect("macro_edges", "from"),
        ],
        "{errors:?}"
    );

    let (before, ctr) = (snapshot(d), counters(d));
    for args in [
        vec![
            "macro",
            "expand",
            "--tree",
            &t2,
            "--macro-link",
            &m,
            "--steps",
            "p",
        ],
        vec!["macro", "promote", "--tree", &t2, "--macro-link", &m],
    ] {
        let (out, _) = run_ltp(d, &args);
        assert_eq!(out["success"], false, "{args:?}: {out}");
        assert_eq!(error_codes(&out), vec!["NODE_NOT_FOUND"], "{out}");
        assert_eq!(out["errors"][0]["node_id"], x.as_str(), "{out}");
        assert_untouched(d, &before, &ctr);
    }
}

// V4b — exact v0.4.0 rm damage (M1): overlay whose `from` was deleted everywhere.
#[test]
fn v4b_legacy_rm_damage_is_detected() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let (t, [a, _, _], [l1, _], m) = linear_overlay(d);
    let inj = add_node(d, "inyeccion", "INJ");
    // Old rm: A gone from pool, nodes[] and edges; the macro is left untouched.
    edit_tree(d, &t, |tr| {
        tr["nodes"].as_array_mut().unwrap().remove(0);
        tr["edges"].as_array_mut().unwrap().remove(0);
    });
    std::fs::remove_file(d.join("nodes").join(format!("{a}.json"))).unwrap();
    assert!(tree_json(d, &t)["macro_edges"][0]["interior_links"]
        .as_array()
        .unwrap()
        .contains(&json!(l1)));

    let (_, errors) = integrity_errors(d, None);
    assert_eq!(
        slots(&errors),
        vec![(a.clone(), s("macro_edges"), s("from"))]
    );

    let (before, ctr) = (snapshot(d), counters(d));
    let (out, _) = run_ltp(
        d,
        &[
            "path",
            "replace",
            "--tree",
            &t,
            "--macro-link",
            &m,
            "--by-node",
            &inj,
        ],
    );
    assert_endpoint_not_in_tree(&out, &a);
    assert_untouched(d, &before, &ctr);
}
