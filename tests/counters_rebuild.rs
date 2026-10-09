//! v0.3.1 — `Counters::rebuild` must recover IDs that only live inside tree JSON
//! (LINK, ASM, NBR, MACRO, MASM, FB). Otherwise a missing or corrupt
//! `.ltp/counters.json` makes the engine reissue IDs that already exist.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

use serde_json::Value;

const EMBEDDED: &[&str] = &["LINK", "ASM", "NBR", "MACRO", "MASM", "FB"];

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

fn add_node(dir: &Path, label: &str, node_type: &str) -> String {
    let json = run_ok(dir, &["node", "add", label, "--type", node_type]);
    json["data"]["id"].as_str().unwrap().to_string()
}

fn connect(dir: &Path, tree: &str, from: &str, to: &str) -> String {
    let json = run_ok(
        dir,
        &[
            "link", "connect", "--tree", tree, "--from", from, "--to", to,
        ],
    );
    json["data"]["created_links"][0]
        .as_str()
        .unwrap()
        .to_string()
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn tree_file(dir: &Path, tree: &str) -> std::path::PathBuf {
    dir.join("trees").join(format!("{tree}.json"))
}

fn counters_path(dir: &Path) -> std::path::PathBuf {
    dir.join(".ltp").join("counters.json")
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

/// Every issued ID in the workspace: file stems of nodes/knowledge plus every
/// `"id"` value inside tree files. Duplicates are kept so callers can detect them.
fn all_ids(dir: &Path) -> Vec<String> {
    let mut ids = Vec::new();
    for sub in ["nodes", "knowledge"] {
        let Ok(entries) = std::fs::read_dir(dir.join(sub)) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(stem) = name.strip_suffix(".json") {
                ids.push(stem.to_string());
            }
        }
    }
    for entry in std::fs::read_dir(dir.join("trees")).unwrap().flatten() {
        if let Ok(value) = serde_json::from_str::<Value>(
            &std::fs::read_to_string(entry.path()).unwrap_or_default(),
        ) {
            let mut inner = Vec::new();
            collect_ids(&value, &mut inner);
            // The tree's own id is the file stem, not a counter-issued ID.
            ids.extend(inner.into_iter().filter(|id| !id.starts_with("tree-")));
        }
    }
    ids
}

fn max_by_prefix(ids: &[String]) -> BTreeMap<String, u64> {
    let mut max = BTreeMap::new();
    for id in ids {
        if let Some((prefix, num)) = id.split_once('-') {
            if let Ok(n) = num.parse::<u64>() {
                let entry = max.entry(prefix.to_string()).or_insert(0);
                *entry = (*entry).max(n);
            }
        }
    }
    max
}

fn assert_unique(ids: &[String]) {
    let mut seen = BTreeSet::new();
    let dups: Vec<&String> = ids.iter().filter(|id| !seen.insert(id.as_str())).collect();
    assert!(dups.is_empty(), "duplicate IDs issued: {dups:?}");
}

/// Builds a CRT that has at least 2 IDs of every embedded prefix.
fn build_rich_workspace(dir: &Path) -> String {
    run_ok(dir, &["init", "--name", "Counters"]);
    let tree = run_ok(dir, &["tree", "new", "crt", "rebuild"])["data"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let nodes: Vec<String> = ["UDE", "RC", "RC", "UDE", "INJ", "INJ"]
        .iter()
        .enumerate()
        .map(|(i, t)| add_node(dir, &format!("n{i}"), t))
        .collect();
    for node in &nodes {
        run_ok(dir, &["tree", "attach", "--tree", &tree, "--node", node]);
    }
    let l1 = connect(dir, &tree, &nodes[1], &nodes[0]);
    let l2 = connect(dir, &tree, &nodes[2], &nodes[3]);
    for link in [&l1, &l2] {
        run_ok(
            dir,
            &[
                "assume",
                "add",
                "--tree",
                &tree,
                "--link",
                link,
                "--text",
                "porque sí",
            ],
        );
    }
    for (from, to) in [(&nodes[0], &nodes[1]), (&nodes[3], &nodes[2])] {
        run_ok(
            dir,
            &[
                "link", "feedback", "--tree", &tree, "--from", from, "--to", to, "--type",
                "positive",
            ],
        );
    }
    for inj in [&nodes[4], &nodes[5]] {
        run_ok(dir, &["nbr", "add", "--tree", &tree, "--source-node", inj]);
    }
    let nbr_id = read_json(&tree_file(dir, &tree))["nbr_branches"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let extra = add_node(dir, "efecto colateral", "UDE");
    run_ok(
        dir,
        &[
            "link", "connect", "--tree", &tree, "--nbr", &nbr_id, "--from", &nodes[4], "--to",
            &extra,
        ],
    );
    for (from, to) in [(&nodes[1], &nodes[3]), (&nodes[2], &nodes[0])] {
        run_ok(
            dir,
            &[
                "macro", "add", "--tree", &tree, "--from", from, "--to", to, "--label", "salto",
            ],
        );
    }
    let macro_id = read_json(&tree_file(dir, &tree))["macro_edges"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    for text in ["resumen uno", "resumen dos"] {
        run_ok(
            dir,
            &[
                "macro-assume",
                "add",
                "--tree",
                &tree,
                "--macro-link",
                &macro_id,
                "--text",
                text,
            ],
        );
    }
    let max = max_by_prefix(&all_ids(dir));
    for prefix in EMBEDDED {
        assert!(
            max.get(*prefix).copied().unwrap_or(0) >= 2,
            "fixture must have >=2 {prefix} IDs, got {max:?}"
        );
    }
    tree
}

/// Triggers a mint in every scan scope (v0.5.2 D-3: a mint only reconciles the
/// scope its prefix lives in), so the stored counters are rebuilt for all of them.
fn trigger_next_id(dir: &Path, tree: &str) {
    add_node(dir, "disparador", "UDE");
    let link = read_json(&tree_file(dir, tree))["edges"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    run_ok(
        dir,
        &[
            "assume",
            "add",
            "--tree",
            tree,
            "--link",
            &link,
            "--text",
            "disparador",
        ],
    );
}

fn assert_counters_cover_disk(dir: &Path, before: &BTreeMap<String, u64>) {
    let counters = read_json(&counters_path(dir));
    for prefix in EMBEDDED {
        let stored = counters[*prefix].as_u64().unwrap_or(0);
        assert!(
            stored >= before[*prefix],
            "{prefix}: rebuilt counter {stored} must cover the max ID on disk {}",
            before[*prefix]
        );
    }
}

/// Creates one more of every embedded type, then asserts no ID was reissued.
fn create_one_of_each_and_assert_unique(dir: &Path, tree: &str) {
    let a = add_node(dir, "nuevo a", "RC");
    let b = add_node(dir, "nuevo b", "UDE");
    for n in [&a, &b] {
        run_ok(dir, &["tree", "attach", "--tree", tree, "--node", n]);
    }
    let link = connect(dir, tree, &a, &b);
    run_ok(
        dir,
        &[
            "assume", "add", "--tree", tree, "--link", &link, "--text", "nuevo",
        ],
    );
    run_ok(
        dir,
        &[
            "link", "feedback", "--tree", tree, "--from", &b, "--to", &a, "--type", "negative",
        ],
    );
    run_ok(dir, &["nbr", "add", "--tree", tree, "--source-node", &a]);
    let c = add_node(dir, "nuevo c", "UDE");
    run_ok(dir, &["tree", "attach", "--tree", tree, "--node", &c]);
    run_ok(
        dir,
        &[
            "macro", "add", "--tree", tree, "--from", &c, "--to", &b, "--label", "nuevo",
        ],
    );
    let macros = read_json(&tree_file(dir, tree))["macro_edges"]
        .as_array()
        .unwrap()
        .clone();
    let last_macro = macros.last().unwrap()["id"].as_str().unwrap().to_string();
    run_ok(
        dir,
        &[
            "macro-assume",
            "add",
            "--tree",
            tree,
            "--macro-link",
            &last_macro,
            "--text",
            "nuevo",
        ],
    );
    assert_unique(&all_ids(dir));
}

// U1
#[test]
fn u1_missing_counters_recovers_every_embedded_prefix() {
    let dir = tempfile::tempdir().unwrap();
    let tree = build_rich_workspace(dir.path());
    let before = max_by_prefix(&all_ids(dir.path()));

    std::fs::remove_file(counters_path(dir.path())).unwrap();
    trigger_next_id(dir.path(), &tree);

    assert_counters_cover_disk(dir.path(), &before);
    create_one_of_each_and_assert_unique(dir.path(), &tree);
}

// U2
#[test]
fn u2_corrupt_counters_recovers_every_embedded_prefix() {
    for garbage in ["", "garbage", "[]", r#"{"UDE":"x"}"#, "{\"LINK\": -1}"] {
        let dir = tempfile::tempdir().unwrap();
        let tree = build_rich_workspace(dir.path());
        let before = max_by_prefix(&all_ids(dir.path()));

        std::fs::write(counters_path(dir.path()), garbage).unwrap();
        trigger_next_id(dir.path(), &tree);

        assert_counters_cover_disk(dir.path(), &before);
        create_one_of_each_and_assert_unique(dir.path(), &tree);
    }
}

// U3
#[test]
fn u3_corrupt_tree_file_is_scanned_as_text_without_panic() {
    let dir = tempfile::tempdir().unwrap();
    let tree = build_rich_workspace(dir.path());
    let before = max_by_prefix(&all_ids(dir.path()));
    std::fs::write(
        dir.path().join("trees").join("tree-crt-roto.json"),
        "{ not json",
    )
    .unwrap();
    std::fs::write(dir.path().join("trees").join("notas.txt"), "LINK-999").unwrap();

    std::fs::remove_file(counters_path(dir.path())).unwrap();
    trigger_next_id(dir.path(), &tree);

    assert_counters_cover_disk(dir.path(), &before);
    let counters = read_json(&counters_path(dir.path()));
    assert_ne!(counters["LINK"], 999, "non-json files must not be scanned");
}

/// Hand-writes an extra edge with `id` into the tree file.
fn inject_edge_id(dir: &Path, tree: &str, id: &str) {
    let path = tree_file(dir, tree);
    let mut raw = read_json(&path);
    let mut edge = raw["edges"][0].clone();
    edge["id"] = Value::String(id.to_string());
    edge["assumptions"] = Value::Array(vec![]);
    raw["edges"].as_array_mut().unwrap().push(edge);
    std::fs::write(&path, serde_json::to_string_pretty(&raw).unwrap()).unwrap();
}

// U4
#[test]
fn u4_three_digit_boundary_rolls_over_to_four_digits() {
    let dir = tempfile::tempdir().unwrap();
    let tree = build_rich_workspace(dir.path());
    inject_edge_id(dir.path(), &tree, "LINK-999");
    std::fs::remove_file(counters_path(dir.path())).unwrap();

    let a = add_node(dir.path(), "a", "RC");
    let b = add_node(dir.path(), "b", "UDE");
    for n in [&a, &b] {
        run_ok(
            dir.path(),
            &["tree", "attach", "--tree", &tree, "--node", n],
        );
    }
    assert_eq!(connect(dir.path(), &tree, &a, &b), "LINK-1000");

    std::fs::remove_file(counters_path(dir.path())).unwrap();
    let c = add_node(dir.path(), "c", "RC");
    run_ok(
        dir.path(),
        &["tree", "attach", "--tree", &tree, "--node", &c],
    );
    assert_eq!(connect(dir.path(), &tree, &c, &b), "LINK-1001");
}

// U5
#[test]
fn u5_numeric_tree_slug_and_odd_filenames_do_not_create_junk_keys() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["init", "--name", "Slug"]);
    run_ok(dir.path(), &["tree", "new", "crt", "2024"]);
    let nodes = dir.path().join("nodes");
    std::fs::write(nodes.join("ude-050.json"), "{}").unwrap();
    std::fs::write(nodes.join("README.json"), "{}").unwrap();
    std::fs::write(nodes.join("UDE-abc.json"), "{}").unwrap();

    std::fs::remove_file(counters_path(dir.path())).unwrap();
    add_node(dir.path(), "disparador", "UDE");

    let counters = read_json(&counters_path(dir.path()));
    let keys: Vec<&String> = counters.as_object().unwrap().keys().collect();
    assert!(
        keys.iter()
            .all(|k| k.bytes().all(|b| b.is_ascii_uppercase())),
        "junk counter keys: {keys:?}"
    );
    assert_eq!(counters["UDE"], 1, "lowercase ude-050 must be ignored");
    assert_eq!(counters["FB"], 0, "FB is a tracked entity type");
}

// U6
#[test]
fn u6_rebuild_is_deterministic() {
    let dir = tempfile::tempdir().unwrap();
    build_rich_workspace(dir.path());
    // Read-only commands never call next_id, so a rebuild needs a mutation;
    // compare two rebuilds from the same disk state by restoring the node file.
    std::fs::remove_file(counters_path(dir.path())).unwrap();
    let id = add_node(dir.path(), "x", "UDE");
    let first = std::fs::read(counters_path(dir.path())).unwrap();

    std::fs::remove_file(dir.path().join("nodes").join(format!("{id}.json"))).unwrap();
    std::fs::remove_file(counters_path(dir.path())).unwrap();
    let again = add_node(dir.path(), "x", "UDE");
    assert_eq!(again, id);
    assert_eq!(first, std::fs::read(counters_path(dir.path())).unwrap());
}

// U7
#[test]
fn u7_deeply_nested_ids_are_recovered() {
    let dir = tempfile::tempdir().unwrap();
    let tree = build_rich_workspace(dir.path());
    let path = tree_file(dir.path(), &tree);
    let mut raw = read_json(&path);
    raw["nbr_branches"][0]["edges"][0]["assumptions"] =
        serde_json::json!([{"id": "ASM-077", "text": "anidado", "status": "active"}]);
    raw["macro_edges"][0]["assumptions"][0]["id"] = Value::String("MASM-033".into());
    std::fs::write(&path, serde_json::to_string_pretty(&raw).unwrap()).unwrap();

    std::fs::remove_file(counters_path(dir.path())).unwrap();
    // The hand-edited tree no longer deserializes, so mint a LINK in another
    // tree: any tree-scoped mint reconciles every tree prefix (D-3).
    let other = run_ok(dir.path(), &["tree", "new", "crt", "otro"])["data"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let a = add_node(dir.path(), "a", "RC");
    let b = add_node(dir.path(), "b", "UDE");
    for n in [&a, &b] {
        run_ok(
            dir.path(),
            &["tree", "attach", "--tree", &other, "--node", n],
        );
    }
    connect(dir.path(), &other, &a, &b);

    let counters = read_json(&counters_path(dir.path()));
    assert_eq!(counters["ASM"], 77);
    assert_eq!(counters["MASM"], 33);
}

// U8
#[test]
fn u8_references_are_not_counted_as_issued_ids() {
    let dir = tempfile::tempdir().unwrap();
    let tree = build_rich_workspace(dir.path());
    let path = tree_file(dir.path(), &tree);
    let mut raw = read_json(&path);
    raw["edges"][0]["from"] = serde_json::json!(["RC-900"]);
    raw["nodes"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"ref": "UDE-900", "role": null}));
    std::fs::write(&path, serde_json::to_string_pretty(&raw).unwrap()).unwrap();

    std::fs::remove_file(counters_path(dir.path())).unwrap();
    trigger_next_id(dir.path(), &tree);

    let counters = read_json(&counters_path(dir.path()));
    assert!(counters["UDE"].as_u64().unwrap() < 900);
    assert!(counters["RC"].as_u64().unwrap() < 900);
}
