//! v0.6.0 D-8 — Inherited duplicate IDs (PLAN_v060.md).
//!
//! Before v0.5.2 `tree clone` copied ASM/FB IDs and `link dissolve` spread one
//! ASM over several edges. `validate` reports each repeated ID once, as the
//! warning `DUPLICATE_ENTITY_ID {id, occurrences: [{tree_id, location}]}` in
//! the `_workspace` pseudo-tree, ordered by ID. It is a warning, not an error:
//! clone duplicates have no repair command.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};

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

fn tree_file(dir: &Path, tree: &str) -> PathBuf {
    dir.join("trees").join(format!("{tree}.json"))
}

fn read_tree(dir: &Path, tree: &str) -> Value {
    serde_json::from_str(&std::fs::read_to_string(tree_file(dir, tree)).unwrap()).unwrap()
}

fn write_tree(dir: &Path, tree: &Value) {
    let id = tree["id"].as_str().unwrap();
    std::fs::write(
        tree_file(dir, id),
        serde_json::to_string_pretty(tree).unwrap(),
    )
    .unwrap();
}

/// Copies `tree-crt-<from>` byte-for-byte as `tree-crt-<name>`, keeping every
/// entity ID (what `tree clone` did before v0.5.2).
fn copy_tree(dir: &Path, from: &str, name: &str) -> String {
    let mut tree = read_tree(dir, &format!("tree-crt-{from}"));
    let id = format!("tree-crt-{name}");
    tree["id"] = json!(id);
    tree["name"] = json!(name);
    write_tree(dir, &tree);
    id
}

/// Fixture. CRT `tree-crt-a` holding one entity of every kind:
/// LINK-001 (+ ASM-001), LINK-002, FB-001, NBR-001 (+ LINK-003), MACRO-001 (+ MASM-001).
fn fixture(dir: &Path) -> String {
    run_ok(dir, &["init", "--name", "DUP"]);
    for (label, kind) in [
        ("c1", "RC"),
        ("e1", "UDE"),
        ("c2", "RC"),
        ("e2", "UDE"),
        ("e3", "UDE"),
    ] {
        run_ok(dir, &["node", "add", label, "--type", kind]);
    }
    let tree = "tree-crt-a".to_string();
    run_ok(dir, &["tree", "new", "crt", "a"]);
    for node in ["RC-001", "UDE-001", "RC-002", "UDE-002", "UDE-003"] {
        run_ok(dir, &["tree", "attach", "--tree", &tree, "--node", node]);
    }
    let connect = |from: &str, to: &str, extra: &[&str]| {
        let mut args = vec![
            "link", "connect", "--tree", &tree, "--from", from, "--to", to,
        ];
        args.extend_from_slice(extra);
        run_ok(dir, &args);
    };
    connect("RC-001", "UDE-001", &[]);
    connect("UDE-001", "UDE-002", &[]);
    run_ok(
        dir,
        &[
            "assume", "add", "--tree", &tree, "--link", "LINK-001", "--text", "t",
        ],
    );
    run_ok(
        dir,
        &[
            "link", "feedback", "--tree", &tree, "--from", "UDE-002", "--to", "RC-001", "--type",
            "positive",
        ],
    );
    run_ok(
        dir,
        &["nbr", "add", "--tree", &tree, "--source-node", "RC-001"],
    );
    connect("RC-001", "UDE-003", &["--nbr", "NBR-001"]);
    run_ok(
        dir,
        &[
            "macro", "add", "--tree", &tree, "--from", "RC-002", "--to", "UDE-002", "--label",
            "salto",
        ],
    );
    run_ok(
        dir,
        &[
            "macro-assume",
            "add",
            "--tree",
            &tree,
            "--macro-link",
            "MACRO-001",
            "--text",
            "s",
        ],
    );
    tree
}

/// Warnings of the `_workspace` entry (empty if the entry is absent).
fn workspace_warnings(validate: &Value) -> Vec<Value> {
    validate["data"]["details"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["tree_id"] == "_workspace")
        .and_then(|d| d["warnings"].as_array().cloned())
        .unwrap_or_default()
}

fn duplicates(validate: &Value) -> Vec<Value> {
    workspace_warnings(validate)
        .into_iter()
        .filter(|w| w["code"] == "DUPLICATE_ENTITY_ID")
        .collect()
}

fn duplicate_ids(validate: &Value) -> Vec<String> {
    duplicates(validate)
        .iter()
        .map(|w| w["id"].as_str().unwrap().to_string())
        .collect()
}

fn duplicate<'a>(dups: &'a [Value], id: &str) -> &'a Value {
    dups.iter()
        .find(|w| w["id"] == id)
        .unwrap_or_else(|| panic!("no DUPLICATE_ENTITY_ID for {id}: {dups:?}"))
}

fn occurrences(warning: &Value) -> Vec<(String, String)> {
    warning["occurrences"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| {
            (
                o["tree_id"].as_str().unwrap().to_string(),
                o["location"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

fn occ(tree: &str, location: &str) -> (String, String) {
    (tree.to_string(), location.to_string())
}

/// Every warning code anywhere in a `validate` output.
fn all_warning_codes(validate: &Value) -> Vec<String> {
    validate["data"]["details"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|d| d["warnings"].as_array().cloned().unwrap_or_default())
        .map(|w| w["code"].as_str().unwrap().to_string())
        .collect()
}

const ALL_IDS: [&str; 8] = [
    "ASM-001",
    "FB-001",
    "LINK-001",
    "LINK-002",
    "LINK-003",
    "MACRO-001",
    "MASM-001",
    "NBR-001",
];

#[test]
fn d0_healthy_workspace_has_no_duplicates() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let tree = fixture(dir);
    run_ok(dir, &["tree", "clone", &tree, "--name", "copia"]);
    let (json, _) = run_ltp(dir, &["validate"]);
    assert!(
        !all_warning_codes(&json).contains(&"DUPLICATE_ENTITY_ID".to_string()),
        "{json}"
    );
    assert!(workspace_warnings(&json).is_empty(), "{json}");
}

#[test]
fn d1_cross_tree_duplicates_every_kind() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let a = fixture(dir);
    let (clean, clean_code) = run_ltp(dir, &["validate"]);
    let b = copy_tree(dir, "a", "b");
    let (json, code) = run_ltp(dir, &["validate"]);
    assert_eq!(code, clean_code, "a warning never changes the exit code");
    assert_eq!(json["success"], clean["success"]);

    assert_eq!(duplicate_ids(&json), ALL_IDS);
    let dups = duplicates(&json);
    let expected = [
        ("ASM-001", "edges[0].assumptions[0]"),
        ("FB-001", "feedback_edges[0]"),
        ("LINK-001", "edges[0]"),
        ("LINK-002", "edges[1]"),
        ("LINK-003", "nbr_branches[0].edges[0]"),
        ("MACRO-001", "macro_edges[0]"),
        ("MASM-001", "macro_edges[0].assumptions[0]"),
        ("NBR-001", "nbr_branches[0]"),
    ];
    for (id, location) in expected {
        let w = duplicate(&dups, id);
        assert_eq!(
            occurrences(w),
            vec![occ(&a, location), occ(&b, location)],
            "{w}"
        );
        assert!(
            w["detail"].as_str().unwrap().contains("tree rm"),
            "cross-tree repair hint: {w}"
        );
    }
}

/// What `link dissolve` did: one ASM spread over two edges of the same tree.
#[test]
fn d2_within_tree_duplicate() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let a = fixture(dir);
    let mut tree = read_tree(dir, &a);
    let asm = tree["edges"][0]["assumptions"][0].clone();
    tree["edges"][1]["assumptions"] = json!([asm]);
    write_tree(dir, &tree);

    let (json, _) = run_ltp(dir, &["validate"]);
    assert_eq!(duplicate_ids(&json), vec!["ASM-001"]);
    let dups = duplicates(&json);
    let w = duplicate(&dups, "ASM-001");
    assert_eq!(
        occurrences(w),
        vec![
            occ(&a, "edges[0].assumptions[0]"),
            occ(&a, "edges[1].assumptions[0]")
        ]
    );
    assert!(
        w["detail"].as_str().unwrap().contains("assume rm"),
        "within-tree repair hint: {w}"
    );
}

/// (e) An ASM repeated inside an NBR branch.
#[test]
fn d3_duplicate_inside_nbr_branch() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let a = fixture(dir);
    let mut tree = read_tree(dir, &a);
    let asm = json!({ "id": "ASM-050", "status": "valid", "text": "nbr" });
    tree["nbr_branches"][0]["edges"][0]["assumptions"] = json!([asm.clone(), asm]);
    write_tree(dir, &tree);

    let (json, _) = run_ltp(dir, &["validate"]);
    assert_eq!(duplicate_ids(&json), vec!["ASM-050"]);
    let dups = duplicates(&json);
    assert_eq!(
        occurrences(duplicate(&dups, "ASM-050")),
        vec![
            occ(&a, "nbr_branches[0].edges[0].assumptions[0]"),
            occ(&a, "nbr_branches[0].edges[0].assumptions[1]")
        ]
    );
}

#[test]
fn d4_feedback_duplicate_across_trees() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let a = fixture(dir);
    run_ok(dir, &["tree", "new", "crt", "b"]);
    for node in ["RC-001", "UDE-002"] {
        run_ok(
            dir,
            &["tree", "attach", "--tree", "tree-crt-b", "--node", node],
        );
    }
    let mut b = read_tree(dir, "tree-crt-b");
    b["feedback_edges"] = read_tree(dir, &a)["feedback_edges"].clone();
    write_tree(dir, &b);

    let (json, _) = run_ltp(dir, &["validate"]);
    assert_eq!(duplicate_ids(&json), vec!["FB-001"]);
    let dups = duplicates(&json);
    assert_eq!(
        occurrences(duplicate(&dups, "FB-001")),
        vec![
            occ(&a, "feedback_edges[0]"),
            occ("tree-crt-b", "feedback_edges[0]")
        ]
    );
}

/// With `--tree T`: only IDs present in T, crossed with the whole workspace.
#[test]
fn d5_tree_filter_scopes_ids_but_crosses_workspace() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let a = fixture(dir);
    copy_tree(dir, "a", "b");
    let c = run_ok(dir, &["tree", "clone", &a, "--name", "c"])["data"]["new_id"]
        .as_str()
        .unwrap()
        .to_string();
    copy_tree(dir, "c", "d");
    let c_ids: Vec<String> = {
        let mut ids = Vec::new();
        collect_ids(&read_tree(dir, &c), &mut ids);
        ids.retain(|id| id != &c);
        ids.sort();
        ids
    };

    let (only_a, _) = run_ltp(dir, &["validate", "--tree", &a]);
    assert_eq!(duplicate_ids(&only_a), ALL_IDS);
    for w in duplicates(&only_a) {
        let trees: Vec<String> = occurrences(&w).into_iter().map(|(t, _)| t).collect();
        assert_eq!(trees, vec![a.clone(), "tree-crt-b".to_string()], "{w}");
    }

    let (only_c, _) = run_ltp(dir, &["validate", "--tree", &c]);
    let ids = duplicate_ids(&only_c);
    assert!(!ids.is_empty(), "{only_c}");
    for id in &ids {
        assert!(!ALL_IDS.contains(&id.as_str()), "{id} is not in {c}");
        assert!(c_ids.contains(id), "{id} not in {c_ids:?}");
    }
}

fn collect_ids(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                match (key.as_str(), child) {
                    ("id", Value::String(id)) => out.push(id.clone()),
                    _ => collect_ids(child, out),
                }
            }
        }
        Value::Array(items) => items.iter().for_each(|i| collect_ids(i, out)),
        _ => {}
    }
}

/// (e) One ID three times gives one entry with three occurrences; a MASM
/// repeated between two long arrows is caught.
#[test]
fn d6_corners_three_occurrences_and_masm_between_macros() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let a = fixture(dir);
    run_ok(
        dir,
        &[
            "macro", "add", "--tree", &a, "--from", "RC-001", "--to", "UDE-003", "--label", "otro",
        ],
    );
    let mut tree = read_tree(dir, &a);
    tree["macro_edges"][1]["assumptions"] = tree["macro_edges"][0]["assumptions"].clone();
    write_tree(dir, &tree);
    let (json, _) = run_ltp(dir, &["validate"]);
    let dups = duplicates(&json);
    assert_eq!(duplicate_ids(&json), vec!["MASM-001"]);
    assert_eq!(
        occurrences(duplicate(&dups, "MASM-001")),
        vec![
            occ(&a, "macro_edges[0].assumptions[0]"),
            occ(&a, "macro_edges[1].assumptions[0]")
        ]
    );

    copy_tree(dir, "a", "b");
    copy_tree(dir, "a", "c");
    let (json, _) = run_ltp(dir, &["validate"]);
    let dups = duplicates(&json);
    let link = duplicate(&dups, "LINK-001");
    assert_eq!(
        occurrences(link),
        vec![
            occ(&a, "edges[0]"),
            occ("tree-crt-b", "edges[0]"),
            occ("tree-crt-c", "edges[0]")
        ]
    );
    assert_eq!(
        duplicate_ids(&json)
            .iter()
            .filter(|id| *id == "LINK-001")
            .count(),
        1
    );
    assert_eq!(occurrences(duplicate(&dups, "MASM-001")).len(), 6);
}

/// G2 — an unreadable tree holding the duplicate: reported as TREE_LOAD_ERROR,
/// never as a clean result nor as a duplicate computed as if all was read.
#[test]
fn d7_unreadable_tree_holding_the_duplicate() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    fixture(dir);
    let b = copy_tree(dir, "a", "b");
    let path = tree_file(dir, &b);
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();

    let (json, _) = run_ltp(dir, &["validate"]);
    assert_eq!(json["success"], false, "{json}");
    let entry = json["data"]["details"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["tree_id"] == b.as_str())
        .unwrap_or_else(|| panic!("{json}"));
    assert_eq!(entry["errors"][0]["code"], "TREE_LOAD_ERROR");
    assert!(duplicates(&json).is_empty(), "{json}");
}

/// G6 — deterministic order: IDs ascending, occurrences by tree, byte-identical
/// across runs, whatever the creation order of the trees.
#[test]
fn d8_deterministic_order() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let a = fixture(dir);
    copy_tree(dir, "a", "zeta");
    copy_tree(dir, "a", "mid");
    let (first, _) = run_raw(dir, &["validate"]);
    let (second, _) = run_raw(dir, &["validate"]);
    assert_eq!(first, second);

    let json: Value = serde_json::from_str(&first).unwrap();
    assert_eq!(duplicate_ids(&json), ALL_IDS);
    for w in duplicates(&json) {
        let trees: Vec<String> = occurrences(&w).into_iter().map(|(t, _)| t).collect();
        assert_eq!(
            trees,
            vec![a.clone(), "tree-crt-mid".into(), "tree-crt-zeta".into()],
            "{w}"
        );
    }
}
