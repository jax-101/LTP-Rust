//! RFC-002 Slice 1 (ADR-015) — cross-tree refs, inferred meta-graph and the
//! integrity of `node rm` / `node split` over NBR branches and refs.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{json, Value};

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

/// Raw JSON-RPC response of one `tools/call`.
fn mcp_raw(dir: &Path, tool: &str, arguments: Value) -> Value {
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
    serde_json::from_str(line).unwrap()
}

/// CommandOutput returned by an MCP tool call.
fn mcp_call(dir: &Path, tool: &str, arguments: Value) -> Value {
    let response = mcp_raw(dir, tool, arguments);
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

fn attach(dir: &Path, tree: &str, node: &str) {
    run_ok(dir, &["tree", "attach", "--tree", tree, "--node", node]);
}

fn connect(dir: &Path, tree: &str, from: &str, to: &str) {
    run_ok(
        dir,
        &[
            "link", "connect", "--tree", tree, "--from", from, "--to", to,
        ],
    );
}

fn node_path(dir: &Path, id: &str) -> PathBuf {
    dir.join("nodes").join(format!("{id}.json"))
}

fn tree_path(dir: &Path, id: &str) -> PathBuf {
    dir.join("trees").join(format!("{id}.json"))
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// Writes `refs` straight into a node file (simulates a hand edit / future CLI).
fn set_refs(dir: &Path, id: &str, refs: Value) {
    let path = node_path(dir, id);
    let mut raw = read_json(&path);
    raw["metadata"]["refs"] = refs;
    std::fs::write(&path, serde_json::to_string_pretty(&raw).unwrap()).unwrap();
}

fn refs_of(dir: &Path, id: &str) -> Value {
    read_json(&node_path(dir, id))["metadata"]
        .get("refs")
        .cloned()
        .unwrap_or(Value::Null)
}

fn warnings_with(json: &Value, code: &str) -> Vec<Value> {
    json["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|w| w["code"] == code)
        .cloned()
        .collect()
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

/// Snapshot of every file under nodes/ and trees/ (path → bytes).
fn snapshot(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    for sub in ["nodes", "trees"] {
        let mut entries: Vec<_> = std::fs::read_dir(dir.join(sub))
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .collect();
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

/// GT with NC-001, CRT with two UDE that (by hand) reference NC-001.
fn gt_crt_with_refs(dir: &Path) -> (String, String, String, [String; 2]) {
    run_ok(dir, &["init", "--name", "S1"]);
    let gt = new_tree(dir, "gt", "meta");
    let crt = new_tree(dir, "crt", "realidad");
    let nc = add_node(dir, "norma", "NC");
    attach(dir, &gt, &nc);
    let udes = [
        add_node(dir, "efecto a", "UDE"),
        add_node(dir, "efecto b", "UDE"),
    ];
    for ude in &udes {
        attach(dir, &crt, ude);
        set_refs(dir, ude, json!([{"node": nc, "tree": null}]));
    }
    (gt, crt, nc, udes)
}

// R9 — node rm strips inbound refs, warns once per referencing node, undo is exact.
#[test]
fn r9_node_rm_strips_inbound_refs_and_undo_restores_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let (_, _, nc, udes) = gt_crt_with_refs(dir.path());
    // A second, unrelated ref must survive the strip.
    let other = add_node(dir.path(), "otra", "NC");
    set_refs(
        dir.path(),
        &udes[0],
        json!([{"node": nc, "tree": null}, {"node": other, "tree": null}]),
    );
    let before = snapshot(dir.path());

    let out = run_ok(dir.path(), &["node", "rm", &nc]);
    let stripped = warnings_with(&out, "REFS_STRIPPED");
    assert_eq!(stripped.len(), 2, "{out}");
    let referencing: Vec<&str> = stripped
        .iter()
        .map(|w| w["referencing"].as_str().unwrap())
        .collect();
    assert_eq!(referencing, vec![udes[0].as_str(), udes[1].as_str()]);
    assert_eq!(stripped[0]["node_ids"], json!([nc]));

    assert_eq!(
        refs_of(dir.path(), &udes[0]),
        json!([{"node": other, "tree": null}])
    );
    assert_eq!(
        refs_of(dir.path(), &udes[1]),
        Value::Null,
        "empty refs are not serialized"
    );

    run_ok(dir.path(), &["undo"]);
    assert_eq!(snapshot(dir.path()), before, "undo must restore every byte");
}

// R9b — removing several nodes at once (including a referencing one) never
// rewrites a node that is itself being removed.
#[test]
fn r9b_multi_rm_including_referencing_node() {
    let dir = tempfile::tempdir().unwrap();
    let (_, _, nc, udes) = gt_crt_with_refs(dir.path());
    let out = run_ok(dir.path(), &["node", "rm", &nc, &udes[0]]);
    let stripped = warnings_with(&out, "REFS_STRIPPED");
    assert_eq!(stripped.len(), 1, "{out}");
    assert_eq!(stripped[0]["referencing"], udes[1].as_str());
    assert!(!node_path(dir.path(), &udes[0]).exists());
    assert!(!node_path(dir.path(), &nc).exists());
}

// R9c — a node with no inbound refs produces no REFS_STRIPPED noise.
#[test]
fn r9c_rm_without_inbound_refs_is_silent() {
    let dir = tempfile::tempdir().unwrap();
    let (_, _, _, udes) = gt_crt_with_refs(dir.path());
    let out = run_ok(dir.path(), &["node", "rm", &udes[0]]);
    assert!(warnings_with(&out, "REFS_STRIPPED").is_empty(), "{out}");
}

/// FRT with INJ-001 (trunk) → NBR branch INJ-001 → UDE (branch only) → UDE2.
fn frt_with_nbr(dir: &Path) -> (String, String, String, [String; 2]) {
    run_ok(dir, &["init", "--name", "NBR"]);
    let frt = new_tree(dir, "frt", "futuro");
    let inj = add_node(dir, "inyeccion", "INJ");
    let de = add_node(dir, "deseado", "DE");
    attach(dir, &frt, &inj);
    attach(dir, &frt, &de);
    connect(dir, &frt, &inj, &de);
    let nbr = nbr_add(dir, &frt, &inj);
    let side = [
        add_node(dir, "colateral", "UDE"),
        add_node(dir, "peor", "UDE"),
    ];
    nbr_connect(dir, &frt, &nbr, &inj, &side[0]);
    nbr_connect(dir, &frt, &nbr, &side[0], &side[1]);
    (frt, inj, nbr, side)
}

// R10a — removing the NBR source removes the branch, with NBR_BRANCH_REMOVED.
#[test]
fn r10a_rm_nbr_source_removes_branch() {
    let dir = tempfile::tempdir().unwrap();
    let (frt, inj, nbr, _) = frt_with_nbr(dir.path());
    let before = snapshot(dir.path());

    let out = run_ok(dir.path(), &["node", "rm", &inj]);
    let removed = warnings_with(&out, "NBR_BRANCH_REMOVED");
    assert_eq!(removed.len(), 1, "{out}");
    assert_eq!(removed[0]["tree_id"], frt.as_str());
    assert_eq!(removed[0]["nbr_id"], nbr.as_str());
    let tree = read_json(&tree_path(dir.path(), &frt));
    assert_eq!(tree["nbr_branches"], json!([]));
    // trunk edge INJ→DE + 2 branch edges
    assert_eq!(out["data"]["removed_edges_count"], 3, "{out}");

    let (validate, code) = run_ltp(dir.path(), &["validate"]);
    assert_eq!(code, 0, "{validate}");

    run_ok(dir.path(), &["undo"]);
    assert_eq!(snapshot(dir.path()), before);
}

// R10b — removing an interior branch node only drops the branch edges touching it.
#[test]
fn r10b_rm_interior_branch_node_drops_its_edges_only() {
    let dir = tempfile::tempdir().unwrap();
    let (frt, _, nbr, side) = frt_with_nbr(dir.path());

    let out = run_ok(dir.path(), &["node", "rm", &side[1]]);
    assert!(warnings_with(&out, "NBR_BRANCH_REMOVED").is_empty());
    assert_eq!(out["data"]["removed_edges_count"], 1, "{out}");
    assert_eq!(out["data"]["affected_trees"], json!([frt]));
    let tree = read_json(&tree_path(dir.path(), &frt));
    let branch = &tree["nbr_branches"][0];
    assert_eq!(branch["id"], nbr.as_str());
    let edges = branch["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0]["to"], side[0].as_str());
}

// R10c — a branch-only node (never attached to the trunk) still counts as affecting the tree.
#[test]
fn r10c_rm_branch_only_node_marks_tree_affected() {
    let dir = tempfile::tempdir().unwrap();
    let (frt, _, _, side) = frt_with_nbr(dir.path());
    let out = run_ok(dir.path(), &["node", "rm", &side[0]]);
    assert_eq!(out["data"]["affected_trees"], json!([frt]), "{out}");
    let tree = read_json(&tree_path(dir.path(), &frt));
    assert_eq!(tree["nbr_branches"][0]["edges"], json!([]));
}

// R11a — split of a referenced NC: inbound refs point at both children, tree pin kept.
#[test]
fn r11a_split_rewrites_inbound_refs_to_both_children() {
    let dir = tempfile::tempdir().unwrap();
    let (gt, _, nc, udes) = gt_crt_with_refs(dir.path());
    set_refs(dir.path(), &udes[1], json!([{"node": nc, "tree": gt}]));

    let out = run_ok(
        dir.path(),
        &[
            "node", "split", &nc, "--into", "norma a", "norma b", "--tree", &gt,
        ],
    );
    let kids: Vec<String> = out["data"]["new_nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        refs_of(dir.path(), &udes[0]),
        json!([{"node": kids[0], "tree": null}, {"node": kids[1], "tree": null}])
    );
    assert_eq!(
        refs_of(dir.path(), &udes[1]),
        json!([{"node": kids[0], "tree": gt}, {"node": kids[1], "tree": gt}])
    );
}

// R11b — split of a node with outbound refs and extra metadata: both children inherit.
#[test]
fn r11b_split_children_inherit_outbound_refs_and_extra() {
    let dir = tempfile::tempdir().unwrap();
    let (_, crt, nc, udes) = gt_crt_with_refs(dir.path());
    let path = node_path(dir.path(), &udes[0]);
    let mut raw = read_json(&path);
    raw["metadata"]["owner"] = json!("ana");
    std::fs::write(&path, serde_json::to_string_pretty(&raw).unwrap()).unwrap();

    let out = run_ok(
        dir.path(),
        &[
            "node", "split", &udes[0], "--into", "a1", "a2", "--tree", &crt,
        ],
    );
    for kid in out["data"]["new_nodes"].as_array().unwrap() {
        let id = kid["id"].as_str().unwrap();
        let meta = &read_json(&node_path(dir.path(), id))["metadata"];
        assert_eq!(meta["refs"], json!([{"node": nc, "tree": null}]), "{id}");
        assert_eq!(meta["owner"], "ana", "{id}");
        assert_eq!(meta["status"], "active", "{id}");
    }
}

// R11c — split redirects NBR branch edges and the branch source.
#[test]
fn r11c_split_redirects_nbr_edges_and_source() {
    let dir = tempfile::tempdir().unwrap();
    let (frt, inj, _, side) = frt_with_nbr(dir.path());

    let out = run_ok(
        dir.path(),
        &["node", "split", &inj, "--into", "i1", "i2", "--tree", &frt],
    );
    let kids: Vec<String> = out["data"]["new_nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["id"].as_str().unwrap().to_string())
        .collect();
    let tree = read_json(&tree_path(dir.path(), &frt));
    let branch = &tree["nbr_branches"][0];
    assert_eq!(branch["source_node"], kids[0].as_str());
    assert_eq!(branch["edges"][0]["from"], json!([kids[1]]));
    assert_eq!(branch["edges"][0]["to"], side[0].as_str());
    let raw = tree.to_string();
    assert!(
        !raw.contains(&format!("\"{inj}\"")),
        "no stale {inj}: {raw}"
    );
}

/// GT with NC-001 and CSF-001, CRT with UDE-001 (no refs yet).
fn gt_crt(dir: &Path) -> (String, String, String, String) {
    run_ok(dir, &["init", "--name", "S1"]);
    let gt = new_tree(dir, "gt", "meta");
    let crt = new_tree(dir, "crt", "realidad");
    let nc = add_node(dir, "norma", "NC");
    attach(dir, &gt, &nc);
    let ude = add_node(dir, "efecto", "UDE");
    attach(dir, &crt, &ude);
    (gt, crt, nc, ude)
}

// R2 — refs to a missing node / missing tree / node outside that tree are
// blocking errors; nothing is written (pool byte-identical, no ID consumed).
#[test]
fn r2_invalid_ref_targets_are_rejected_and_nothing_is_written() {
    let dir = tempfile::tempdir().unwrap();
    let (gt, crt, nc, ude) = gt_crt(dir.path());
    let cases: [(String, &str); 3] = [
        ("NC-099".to_string(), "NODE_NOT_FOUND"),
        (format!("{nc}@tree-gt-nope"), "TREE_NOT_FOUND"),
        (format!("{nc}@{crt}"), "NODE_NOT_IN_TREE"),
    ];
    for (r, code) in &cases {
        let before = snapshot(dir.path());
        let counters = std::fs::read(dir.path().join(".ltp/counters.json")).unwrap();

        let (out, exit) = run_ltp(
            dir.path(),
            &["node", "add", "nuevo", "--type", "UDE", "--ref", r],
        );
        assert_ne!(exit, 0, "{r}: {out}");
        assert_eq!(error_codes(&out), vec![*code], "{r}: {out}");
        assert_eq!(out["errors"][0]["ref_node"], r.split('@').next().unwrap());

        let (out, exit) = run_ltp(dir.path(), &["node", "edit", &ude, "--add-ref", r]);
        assert_ne!(exit, 0);
        assert_eq!(error_codes(&out), vec![*code], "{r}: {out}");

        assert_eq!(snapshot(dir.path()), before, "{r}: disk must be untouched");
        assert_eq!(
            std::fs::read(dir.path().join(".ltp/counters.json")).unwrap(),
            counters,
            "{r}: a rejected add must not consume an ID"
        );
    }
    // Pinned to the right tree is fine.
    let ok = run_ok(
        dir.path(),
        &["node", "edit", &ude, "--add-ref", &format!("{nc}@{gt}")],
    );
    assert_eq!(ok["data"]["refs"], json!([{"node": nc, "tree": gt}]));
}

// R3 — self-ref is SELF_REF in edit; in add the not-yet-issued ID does not exist.
#[test]
fn r3_self_ref_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let (gt, _, nc, _) = gt_crt(dir.path());
    let before = snapshot(dir.path());
    for r in [nc.clone(), format!("{nc}@{gt}")] {
        let (out, exit) = run_ltp(dir.path(), &["node", "edit", &nc, "--add-ref", &r]);
        assert_ne!(exit, 0);
        assert_eq!(error_codes(&out), vec!["SELF_REF"], "{out}");
    }
    let out = mcp_call(
        dir.path(),
        "ltp/node_edit",
        json!({"id": nc, "add_refs": [{"node": nc}]}),
    );
    assert_eq!(error_codes(&out), vec!["SELF_REF"], "{out}");
    assert_eq!(snapshot(dir.path()), before);

    let (out, _) = run_ltp(
        dir.path(),
        &["node", "add", "yo", "--type", "UDE", "--ref", "UDE-002"],
    );
    assert_eq!(error_codes(&out), vec!["NODE_NOT_FOUND"], "{out}");
}

// R4 — duplicates collapse; on-disk order is canonical regardless of input order.
#[test]
fn r4_duplicate_refs_collapse_and_order_is_canonical() {
    let dir = tempfile::tempdir().unwrap();
    let (gt, _, nc, _) = gt_crt(dir.path());
    let csf = add_node(dir.path(), "factor", "CSF");
    attach(dir.path(), &gt, &csf);
    let pinned = format!("{nc}@{gt}");

    let a = run_ok(
        dir.path(),
        &[
            "node", "add", "a", "--type", "UDE", "--ref", &pinned, "--ref", &nc, "--ref", &csf,
            "--ref", &nc,
        ],
    )["data"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let b = run_ok(
        dir.path(),
        &[
            "node", "add", "b", "--type", "UDE", "--ref", &csf, "--ref", &nc, "--ref", &pinned,
        ],
    )["data"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let expected = json!([
        {"node": csf, "tree": null},
        {"node": nc, "tree": null},
        {"node": nc, "tree": gt}
    ]);
    assert_eq!(refs_of(dir.path(), &a), expected);
    assert_eq!(refs_of(dir.path(), &b), expected);

    let again = run_ok(dir.path(), &["node", "edit", &a, "--add-ref", &nc]);
    assert_eq!(again["data"]["refs"], expected, "re-adding is a no-op");
    assert!(warnings_with(&again, "REF_NOT_PRESENT").is_empty());
}

// R4b — rm_ref: exact match only; absent refs warn REF_NOT_PRESENT; dangling refs can be removed.
#[test]
fn r4b_rm_ref_exact_match_and_dangling_removal() {
    let dir = tempfile::tempdir().unwrap();
    let (gt, _, nc, ude) = gt_crt(dir.path());
    run_ok(
        dir.path(),
        &["node", "edit", &ude, "--add-ref", &format!("{nc}@{gt}")],
    );
    // Unpinned form does not match the pinned ref.
    let out = run_ok(dir.path(), &["node", "edit", &ude, "--rm-ref", &nc]);
    let missing = warnings_with(&out, "REF_NOT_PRESENT");
    assert_eq!(missing.len(), 1, "{out}");
    assert_eq!(missing[0]["ref_tree"], Value::Null);
    assert_eq!(out["data"]["refs"], json!([{"node": nc, "tree": gt}]));

    // A hand-written dangling ref (target gone) is still removable.
    set_refs(
        dir.path(),
        &ude,
        json!([{"node": "NC-404", "tree": null}, {"node": nc, "tree": gt}]),
    );
    let out = run_ok(dir.path(), &["node", "edit", &ude, "--rm-ref", "NC-404"]);
    assert!(warnings_with(&out, "REF_NOT_PRESENT").is_empty(), "{out}");
    assert_eq!(out["data"]["refs"], json!([{"node": nc, "tree": gt}]));

    // Remove + re-add in one call: removal first, then add.
    let out = run_ok(
        dir.path(),
        &[
            "node",
            "edit",
            &ude,
            "--rm-ref",
            &format!("{nc}@{gt}"),
            "--add-ref",
            &nc,
        ],
    );
    assert_eq!(out["data"]["refs"], json!([{"node": nc, "tree": null}]));
}

// R5 — MCP: malformed refs are protocol errors (invalid params); nothing written.
#[test]
fn r5_mcp_malformed_refs_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let (_, _, nc, ude) = gt_crt(dir.path());
    let before = snapshot(dir.path());
    let bad = [
        json!("NC-001"),
        json!({"node": nc}),
        json!(["NC-001"]),
        json!([{}]),
        json!([{"node": 5}]),
        json!([{"tree": "x"}]),
        json!([{"node": nc, "tree": 7}]),
        json!([{"node": nc, "kind": "gap"}]),
        json!([{"node": ""}]),
        json!([{"node": "NC 001"}]),
        json!([{"node": "NC-001@x"}]),
        json!([{"node": nc, "tree": ""}]),
    ];
    for refs in &bad {
        let response = mcp_raw(
            dir.path(),
            "ltp/node_add",
            json!({"label": "x", "type": "UDE", "refs": refs}),
        );
        assert_eq!(response["error"]["code"], -32602, "{refs}: {response}");
        for key in ["add_refs", "rm_refs"] {
            let response = mcp_raw(dir.path(), "ltp/node_edit", json!({"id": ude, key: refs}));
            assert_eq!(
                response["error"]["code"], -32602,
                "{key} {refs}: {response}"
            );
        }
    }
    assert_eq!(snapshot(dir.path()), before);
    // `tree: null` and omitted tree are both accepted.
    let ok = mcp_call(
        dir.path(),
        "ltp/node_add",
        json!({"label": "x", "type": "UDE", "refs": [{"node": nc, "tree": null}]}),
    );
    assert_eq!(ok["success"], true, "{ok}");
}

// R5b — CLI: malformed NODE[@TREE] is a JSON INVALID_REF error; nothing written.
#[test]
fn r5b_cli_malformed_ref_is_invalid_ref() {
    let dir = tempfile::tempdir().unwrap();
    let (_, _, _, ude) = gt_crt(dir.path());
    let before = snapshot(dir.path());
    for bad in ["@", "@tree-gt-meta", "NC-001@", "a@b@c", ""] {
        let (out, exit) = run_ltp(
            dir.path(),
            &["node", "add", "x", "--type", "UDE", "--ref", bad],
        );
        assert_eq!(exit, 1, "{bad:?}");
        assert_eq!(error_codes(&out), vec!["INVALID_REF"], "{bad:?}: {out}");
        let (out, _) = run_ltp(dir.path(), &["node", "edit", &ude, "--rm-ref", bad]);
        assert_eq!(error_codes(&out), vec!["INVALID_REF"], "{bad:?}: {out}");
    }
    assert_eq!(snapshot(dir.path()), before);
}

// R7a — a node only present in an NBR branch is a valid pinned target.
#[test]
fn r7a_branch_only_node_is_a_valid_pinned_target() {
    let dir = tempfile::tempdir().unwrap();
    let (frt, _, _, side) = frt_with_nbr(dir.path());
    let crt = new_tree(dir.path(), "crt", "ahora");
    let rc = add_node(dir.path(), "causa", "RC");
    attach(dir.path(), &crt, &rc);
    let out = run_ok(
        dir.path(),
        &[
            "node",
            "edit",
            &rc,
            "--add-ref",
            &format!("{}@{frt}", side[0]),
        ],
    );
    assert_eq!(out["data"]["refs"], json!([{"node": side[0], "tree": frt}]));
}

// R19 — CLI ↔ MCP parity: same data/errors/warnings for valid and invalid refs.
#[test]
fn r19_cli_mcp_parity_for_refs() {
    let cli = tempfile::tempdir().unwrap();
    let mcp = tempfile::tempdir().unwrap();
    let (gt, _, nc, ude) = gt_crt(cli.path());
    gt_crt(mcp.path());

    let pinned = format!("{nc}@{gt}");
    let c = run_ltp(
        cli.path(),
        &["node", "add", "a", "--type", "UDE", "--ref", &pinned],
    )
    .0;
    let m = mcp_call(
        mcp.path(),
        "ltp/node_add",
        json!({"label": "a", "type": "UDE", "refs": [{"node": nc, "tree": gt}]}),
    );
    assert_eq!(c["data"], m["data"]);

    for (cli_ref, mcp_ref) in [
        ("NC-099".to_string(), json!({"node": "NC-099"})),
        (
            format!("{nc}@tree-gt-x"),
            json!({"node": nc, "tree": "tree-gt-x"}),
        ),
    ] {
        let c = run_ltp(cli.path(), &["node", "edit", &ude, "--add-ref", &cli_ref]).0;
        let m = mcp_call(
            mcp.path(),
            "ltp/node_edit",
            json!({"id": ude, "add_refs": [mcp_ref]}),
        );
        assert_eq!(c["errors"], m["errors"], "{cli_ref}");
        assert_eq!(c["data"], m["data"], "{cli_ref}");
    }

    let c = run_ltp(cli.path(), &["node", "edit", &ude, "--rm-ref", &nc]).0;
    let m = mcp_call(
        mcp.path(),
        "ltp/node_edit",
        json!({"id": ude, "rm_refs": [{"node": nc}]}),
    );
    assert_eq!(c["warnings"], m["warnings"]);

    let c = run_ok(cli.path(), &["node", "rm", &nc]);
    let m = mcp_call(mcp.path(), "ltp/node_rm", json!({"ids": [nc]}));
    assert_eq!(c["warnings"], m["warnings"]);
    assert_eq!(c["data"], m["data"]);
}

// R-inspect — inspect exposes refs and referenced_by (sorted, deduplicated).
#[test]
fn inspect_exposes_refs_and_referenced_by() {
    let dir = tempfile::tempdir().unwrap();
    let (gt, _, nc, udes) = gt_crt_with_refs(dir.path());
    // Two refs (pinned + unpinned) from the same node count once.
    set_refs(
        dir.path(),
        &udes[1],
        json!([{"node": nc, "tree": null}, {"node": nc, "tree": gt}]),
    );
    let out = run_ok(dir.path(), &["node", "inspect", &nc]);
    assert_eq!(out["data"]["referenced_by"], json!([udes[0], udes[1]]));
    assert_eq!(out["data"]["refs"], json!([]));
    let out = run_ok(dir.path(), &["node", "inspect", &udes[1]]);
    assert_eq!(out["data"]["referenced_by"], json!([]));
    assert_eq!(out["data"]["refs"].as_array().unwrap().len(), 2);
}
