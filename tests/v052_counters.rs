//! v0.5.2 — A new ID never overwrites an existing one (PLAN_v052.md).
//!
//! `counters.json` is only a monotonicity memory: every mint reconciles it with
//! what is on disk (D-1), scanning only the scope where the prefix lives (D-3)
//! and failing closed when something in that scope cannot be read. Copies
//! (`tree clone`, `link dissolve`) mint fresh IDs for the entities they copy (D-6).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

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

fn error_codes(json: &Value) -> Vec<String> {
    json["errors"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|e| e["code"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn add_node(dir: &Path, label: &str, node_type: &str) -> String {
    let json = run_ok(dir, &["node", "add", label, "--type", node_type]);
    json["data"]["id"].as_str().unwrap().to_string()
}

fn new_tree(dir: &Path, name: &str) -> String {
    run_ok(dir, &["tree", "new", "crt", name])["data"]["id"]
        .as_str()
        .unwrap()
        .to_string()
}

fn attach(dir: &Path, tree: &str, nodes: &[&str]) {
    for node in nodes {
        run_ok(dir, &["tree", "attach", "--tree", tree, "--node", node]);
    }
}

fn connect_args<'a>(tree: &'a str, from: &'a str, to: &'a str) -> Vec<&'a str> {
    vec![
        "link", "connect", "--tree", tree, "--from", from, "--to", to,
    ]
}

fn connect(dir: &Path, tree: &str, from: &str, to: &str) -> String {
    run_ok(dir, &connect_args(tree, from, to))["data"]["created_links"][0]
        .as_str()
        .unwrap()
        .to_string()
}

fn add_assumption(dir: &Path, tree: &str, link: &str, text: &str) -> String {
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

fn add_knowledge(dir: &Path, label: &str) -> Value {
    let (json, _) = run_ltp(
        dir,
        &[
            "knowledge",
            "add",
            label,
            "--type",
            "document",
            "--source-uri",
            "https://example.org",
        ],
    );
    json
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn tree_file(dir: &Path, tree: &str) -> PathBuf {
    dir.join("trees").join(format!("{tree}.json"))
}

fn counters_path(dir: &Path) -> PathBuf {
    dir.join(".ltp").join("counters.json")
}

fn set_counter(dir: &Path, prefix: &str, value: u64) {
    let path = counters_path(dir);
    let mut counters = read_json(&path);
    counters[prefix] = Value::from(value);
    std::fs::write(&path, serde_json::to_string_pretty(&counters).unwrap()).unwrap();
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

/// Every `"id"` inside one tree file except the tree's own slug.
fn ids_in_tree(dir: &Path, tree: &str) -> Vec<String> {
    let mut ids = Vec::new();
    collect_ids(&read_json(&tree_file(dir, tree)), &mut ids);
    ids.retain(|id| !id.starts_with("tree-"));
    ids
}

/// Every issued ID in the workspace (node/knowledge file stems plus tree contents).
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
        let Ok(content) = std::fs::read_to_string(entry.path()) else {
            continue;
        };
        if let Ok(value) = serde_json::from_str::<Value>(&content) {
            let mut inner = Vec::new();
            collect_ids(&value, &mut inner);
            ids.extend(inner.into_iter().filter(|id| !id.starts_with("tree-")));
        }
    }
    ids
}

fn duplicates(ids: &[String]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    ids.iter()
        .filter(|id| !seen.insert(id.as_str()))
        .cloned()
        .collect()
}

fn assert_unique(ids: &[String]) {
    let dups = duplicates(ids);
    assert!(dups.is_empty(), "duplicate IDs issued: {dups:?}");
}

/// Byte-for-byte snapshot of every file in the workspace.
fn snapshot(dir: &Path) -> Vec<(String, Vec<u8>)> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            let rel = path.strip_prefix(root).unwrap().display().to_string();
            if path.is_dir() {
                out.push((format!("{rel}/"), Vec::new()));
                walk(root, &path, out);
            } else {
                out.push((rel, std::fs::read(&path).unwrap()));
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

#[cfg(unix)]
fn running_as_root() -> bool {
    // SAFETY: geteuid has no preconditions and cannot fail.
    unsafe { libc::geteuid() == 0 }
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
}

/// Two CRTs, `a` with `LINK-001` (+ `ASM-001`) and `b` with two attached nodes
/// ready to be connected. Returns (tree_a, tree_b, cause_in_b, effect_in_b).
fn two_trees(dir: &Path) -> (String, String, String, String) {
    run_ok(dir, &["init", "--name", "V052"]);
    let a = new_tree(dir, "a");
    let b = new_tree(dir, "b");
    let rc = add_node(dir, "causa", "RC");
    let ude = add_node(dir, "efecto", "UDE");
    attach(dir, &a, &[&rc, &ude]);
    let link = connect(dir, &a, &rc, &ude);
    assert_eq!(link, "LINK-001");
    add_assumption(dir, &a, &link, "porque sí");
    let rc2 = add_node(dir, "causa b", "RC");
    let ude2 = add_node(dir, "efecto b", "UDE");
    attach(dir, &b, &[&rc2, &ude2]);
    (a, b, rc2, ude2)
}

/// Replaces the tree file with a directory of the same name (EISDIR on read).
fn make_tree_unreadable(dir: &Path, tree: &str) {
    let path = tree_file(dir, tree);
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
}

fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@example.org",
            "-c",
            "init.defaultBranch=main",
        ])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

// R1 — C1: two git clones; a pulled node is never overwritten.
#[test]
fn r1_two_git_clones_do_not_overwrite_a_pulled_node() {
    if !git_available() {
        eprintln!("skipping r1: git not available");
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let bare = root.path().join("origin.git");
    let a = root.path().join("a");
    let b = root.path().join("b");
    std::fs::create_dir(&a).unwrap();
    git(root.path(), &["init", "--bare", bare.to_str().unwrap()]);

    git(&a, &["init"]);
    run_ok(&a, &["init", "--name", "Clones"]);
    assert_eq!(add_node(&a, "de A", "UDE"), "UDE-001");
    git(&a, &["add", "-A"]);
    git(&a, &["commit", "-m", "uno"]);
    git(&a, &["push", bare.to_str().unwrap(), "HEAD:main"]);

    git(
        root.path(),
        &["clone", bare.to_str().unwrap(), b.to_str().unwrap()],
    );
    assert!(!b.join(".ltp").exists(), ".ltp/ must be gitignored");
    // B mints something unrelated, so it persists counters with UDE: 1.
    let kn = add_knowledge(&b, "nota de B");
    assert_eq!(kn["success"], true, "B must be usable after clone: {kn}");

    assert_eq!(add_node(&a, "segundo de A", "UDE"), "UDE-002");
    git(&a, &["add", "-A"]);
    git(&a, &["commit", "-m", "dos"]);
    git(&a, &["push", bare.to_str().unwrap(), "HEAD:main"]);
    let from_a = std::fs::read(a.join("nodes").join("UDE-002.json")).unwrap();

    git(&b, &["pull", "origin", "main"]);
    assert_eq!(add_node(&b, "de B", "UDE"), "UDE-003");
    assert_eq!(
        std::fs::read(b.join("nodes").join("UDE-002.json")).unwrap(),
        from_a,
        "A's UDE-002 must be intact in B"
    );
}

// R1b — C1 without git: counters.json below disk.
#[test]
fn r1b_stale_counter_does_not_overwrite_node_on_disk() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["init", "--name", "Stale"]);
    add_node(dir.path(), "uno", "UDE");
    let nodes = dir.path().join("nodes");
    let mut pulled = read_json(&nodes.join("UDE-001.json"));
    pulled["id"] = Value::from("UDE-002");
    pulled["label"] = Value::from("traído por pull");
    let pulled_bytes = serde_json::to_string_pretty(&pulled).unwrap();
    std::fs::write(nodes.join("UDE-002.json"), &pulled_bytes).unwrap();
    assert_eq!(read_json(&counters_path(dir.path()))["UDE"], 1);

    assert_eq!(add_node(dir.path(), "nuevo", "UDE"), "UDE-003");
    assert_eq!(
        std::fs::read_to_string(nodes.join("UDE-002.json")).unwrap(),
        pulled_bytes
    );
}

// R2 — C2: a tree with merge-conflict markers still counts.
#[test]
fn r2_merge_conflict_markers_do_not_hide_ids() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b, rc2, ude2) = two_trees(dir.path());
    let path = tree_file(dir.path(), &a);
    let original = std::fs::read_to_string(&path).unwrap();
    let conflicted = format!(
        "<<<<<<< HEAD\n{original}\n=======\n{{\"edges\": [{{\"id\": \"LINK-001\"}}], \"nbr_branches\": [{{\"id\": \"NBR-001\"}}]}}\n>>>>>>> otra-rama\n"
    );
    std::fs::write(&path, conflicted).unwrap();
    std::fs::remove_file(counters_path(dir.path())).unwrap();

    let link = connect(dir.path(), &b, &rc2, &ude2);
    assert_ne!(link, "LINK-001", "LINK-001 lives in the conflicted tree");
    let num: u64 = link.trim_start_matches("LINK-").parse().unwrap();
    assert!(num >= 2, "{link}");
}

// R3 — C3 (deterministic): unreadable tree, no counters.json → fail closed.
#[test]
fn r3_unreadable_tree_without_counters_fails_closed() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b, rc2, ude2) = two_trees(dir.path());
    make_tree_unreadable(dir.path(), &a);
    std::fs::remove_file(counters_path(dir.path())).unwrap();
    let before = snapshot(dir.path());

    let (json, code) = run_ltp(dir.path(), &connect_args(&b, &rc2, &ude2));
    assert_ne!(code, 0, "{json}");
    assert_eq!(error_codes(&json), vec!["ID_GENERATION_ERROR"], "{json}");
    let detail = json["errors"][0]["detail"].as_str().unwrap_or_default();
    assert!(
        detail.contains(&format!("{a}.json")),
        "detail must name the unreadable file: {detail}"
    );
    assert_eq!(snapshot(dir.path()), before, "workspace must be untouched");
}

// R4 — C4: `nodes/` write-only (-wx), no counters.json → fail closed.
#[cfg(unix)]
#[test]
fn r4_unlistable_nodes_without_counters_fails_closed() {
    if running_as_root() {
        eprintln!("skipping r4: root ignores permissions");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["init", "--name", "Wx"]);
    add_node(dir.path(), "original", "UDE");
    let nodes = dir.path().join("nodes");
    let original = std::fs::read(nodes.join("UDE-001.json")).unwrap();
    std::fs::remove_file(counters_path(dir.path())).unwrap();

    set_mode(&nodes, 0o300);
    let (json, code) = run_ltp(dir.path(), &["node", "add", "nuevo", "--type", "UDE"]);
    set_mode(&nodes, 0o755);

    assert_ne!(code, 0, "{json}");
    assert_eq!(error_codes(&json), vec!["ID_GENERATION_ERROR"], "{json}");
    assert_eq!(std::fs::read(nodes.join("UDE-001.json")).unwrap(), original);
}

// R5 — C6: a fresh clone (no `.ltp/`) is usable and respects the disk.
#[test]
fn r5_workspace_without_ltp_dir_is_usable() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b, rc2, ude2) = two_trees(dir.path());
    std::fs::remove_dir_all(dir.path().join(".ltp")).unwrap();

    let ude = add_node(dir.path(), "tras clonar", "UDE");
    assert_eq!(ude, "UDE-003", "UDE-001/002 already exist on disk");
    let link = connect(dir.path(), &b, &rc2, &ude2);
    assert_eq!(link, "LINK-002", "LINK-001 lives in {a}");
    assert_unique(&all_ids(dir.path()));
}

// R6 — unreadable tree WITH a valid counters.json: tree-scoped mints fail,
// node/knowledge mints are out of scope and still work.
#[test]
fn r6_unreadable_tree_blocks_only_tree_scoped_mints() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b, rc2, ude2) = two_trees(dir.path());
    make_tree_unreadable(dir.path(), &a);
    assert!(counters_path(dir.path()).exists());

    let (json, code) = run_ltp(dir.path(), &connect_args(&b, &rc2, &ude2));
    assert_ne!(code, 0, "{json}");
    assert_eq!(error_codes(&json), vec!["ID_GENERATION_ERROR"], "{json}");

    assert_eq!(add_node(dir.path(), "fuera de ámbito", "UDE"), "UDE-003");
    let kn = add_knowledge(dir.path(), "fuera de ámbito");
    assert_eq!(kn["success"], true, "{kn}");
    assert_eq!(kn["data"]["created_knowledge_id"], "KN-001");
}

// R6b — `nodes/` write-only WITH a valid counters.json → still fail closed.
#[cfg(unix)]
#[test]
fn r6b_unlistable_nodes_with_counters_fails_closed() {
    if running_as_root() {
        eprintln!("skipping r6b: root ignores permissions");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["init", "--name", "Wx"]);
    add_node(dir.path(), "original", "UDE");
    let nodes = dir.path().join("nodes");
    assert!(counters_path(dir.path()).exists());

    set_mode(&nodes, 0o300);
    let (json, code) = run_ltp(dir.path(), &["node", "add", "nuevo", "--type", "UDE"]);
    set_mode(&nodes, 0o755);

    assert_ne!(code, 0, "{json}");
    assert_eq!(error_codes(&json), vec!["ID_GENERATION_ERROR"], "{json}");
    assert!(!nodes.join("UDE-002.json").exists());
}

// R7 — absent ≠ unreadable: no `knowledge/` directory is fine.
#[test]
fn r7_absent_knowledge_dir_is_not_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let (_a, b, rc2, ude2) = two_trees(dir.path());
    std::fs::remove_dir_all(dir.path().join("knowledge")).unwrap();
    std::fs::remove_file(counters_path(dir.path())).unwrap();

    assert_eq!(add_node(dir.path(), "x", "UDE"), "UDE-003");
    assert_eq!(connect(dir.path(), &b, &rc2, &ude2), "LINK-002");
    let kn = add_knowledge(dir.path(), "primera");
    assert_eq!(kn["success"], true, "{kn}");
    assert_eq!(kn["data"]["created_knowledge_id"], "KN-001");
}

// R8 — stale LINK counter + a tree brought by pull → no duplicate LINK.
#[test]
fn r8_stale_link_counter_does_not_duplicate_across_trees() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b, rc2, ude2) = two_trees(dir.path());
    assert_eq!(connect(dir.path(), &b, &rc2, &ude2), "LINK-002");
    // Simulate a pull: the local counter never saw LINK-002.
    set_counter(dir.path(), "LINK", 1);

    let rc3 = add_node(dir.path(), "otra causa", "RC");
    attach(dir.path(), &a, &[&rc3]);
    let ude_a = read_json(&tree_file(dir.path(), &a))["edges"][0]["to"]
        .as_str()
        .unwrap()
        .to_string();
    let link = connect(dir.path(), &a, &rc3, &ude_a);
    assert_eq!(link, "LINK-003");
    assert_unique(&all_ids(dir.path()));
}

/// CRT with `LINK-001` + `ASM-001` + `FB-001`. Returns the tree id.
fn tree_with_asm_and_fb(dir: &Path) -> String {
    run_ok(dir, &["init", "--name", "Clone"]);
    let t = new_tree(dir, "demo");
    let rc = add_node(dir, "causa", "RC");
    let ude = add_node(dir, "efecto", "UDE");
    attach(dir, &t, &[&rc, &ude]);
    let link = connect(dir, &t, &rc, &ude);
    assert_eq!(add_assumption(dir, &t, &link, "s"), "ASM-001");
    let fb = run_ok(
        dir,
        &[
            "link", "feedback", "--tree", &t, "--from", &ude, "--to", &rc, "--type", "positive",
        ],
    );
    assert_eq!(fb["data"]["id"], "FB-001");
    t
}

fn clone_tree(dir: &Path, tree: &str, name: &str) -> String {
    run_ok(dir, &["tree", "clone", tree, "--name", name])["data"]["new_id"]
        .as_str()
        .unwrap()
        .to_string()
}

// R10 — `tree clone` mints fresh ASM and FB IDs; the original keeps its own.
#[test]
fn r10_tree_clone_mints_fresh_asm_and_fb_ids() {
    let dir = tempfile::tempdir().unwrap();
    let t = tree_with_asm_and_fb(dir.path());
    let original_before = ids_in_tree(dir.path(), &t);
    let copy = clone_tree(dir.path(), &t, "copia");

    assert_eq!(ids_in_tree(dir.path(), &t), original_before);
    let copy_ids = ids_in_tree(dir.path(), &copy);
    assert!(
        copy_ids.iter().any(|id| id.starts_with("ASM-")),
        "{copy_ids:?}"
    );
    assert!(
        copy_ids.iter().any(|id| id.starts_with("FB-")),
        "{copy_ids:?}"
    );
    assert_unique(&all_ids(dir.path()));
}

// R11 — a knowledge link to the original ASM does not silently move to the copy.
#[test]
fn r11_knowledge_link_does_not_migrate_to_the_clone() {
    let dir = tempfile::tempdir().unwrap();
    let t = tree_with_asm_and_fb(dir.path());
    let kn = add_knowledge(dir.path(), "evidencia");
    assert_eq!(kn["data"]["created_knowledge_id"], "KN-001", "{kn}");
    run_ok(
        dir.path(),
        &[
            "knowledge",
            "link",
            "KN-001",
            "--to",
            "ASM-001",
            "--relation",
            "supports",
        ],
    );
    clone_tree(dir.path(), &t, "copia");
    run_ok(dir.path(), &["tree", "rm", &t]);

    let inspect = run_ok(dir.path(), &["knowledge", "inspect", "KN-001"]);
    let link = &inspect["data"]["links"][0];
    assert_eq!(link["target"], "ASM-001");
    assert_eq!(
        link["target_type"], "unknown",
        "ASM-001 was removed with its tree; it must not resolve to the copy: {inspect}"
    );
}

// R12 — `link dissolve` gives each spread assumption its own ID.
#[test]
fn r12_link_dissolve_does_not_duplicate_assumptions() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["init", "--name", "Dissolve"]);
    let t = new_tree(dir.path(), "demo");
    let ude = add_node(dir.path(), "efecto", "UDE");
    let rc1 = add_node(dir.path(), "causa 1", "RC");
    let rc2 = add_node(dir.path(), "causa 2", "RC");
    attach(dir.path(), &t, &[&ude, &rc1, &rc2]);
    let from = format!("{rc1},{rc2}");
    run_ok(
        dir.path(),
        &[
            "link",
            "connect",
            "--tree",
            &t,
            "--from",
            &from,
            "--to",
            &ude,
            "--operator",
            "AND",
        ],
    );
    assert_eq!(add_assumption(dir.path(), &t, "LINK-001", "s"), "ASM-001");

    run_ok(
        dir.path(),
        &["link", "dissolve", "--tree", &t, "--link", "LINK-001"],
    );
    let ids = ids_in_tree(dir.path(), &t);
    assert_unique(&ids);
    assert!(ids.contains(&"ASM-001".to_string()), "{ids:?}");

    run_ok(
        dir.path(),
        &["assume", "rm", "--tree", &t, "--asm", "ASM-001"],
    );
    let left = ids_in_tree(dir.path(), &t);
    assert!(
        !left.contains(&"ASM-001".to_string()),
        "a copy of ASM-001 survived its removal: {left:?}"
    );
}
