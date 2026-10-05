//! ADR-014: la lógica de un árbol se deriva de su tipo (CLR_SPEC §1.2).
//!
//! Invariante: `tree.logic == tree_type.logic()`, los edges del tronco llevan esa lógica y
//! los edges de NBR son siempre `SUFFICIENCY`. Los ficheros legacy se normalizan al leer
//! (en memoria) y se persisten corregidos en la siguiente mutación.
//!
//! Every read normalizes, so `link inspect` (or any later mutation) would mask a broken
//! edge creator: assertions about what a command wrote read the raw `trees/<id>.json`
//! right after that command.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};

fn ltp_bin() -> String {
    env!("CARGO_BIN_EXE_ltp").to_string()
}

fn run_ltp(dir: &Path, args: &[&str]) -> (Value, i32) {
    let output = Command::new(ltp_bin())
        .args(args)
        .current_dir(dir)
        .output()
        .expect("failed to execute ltp binary");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let code = output.status.code().unwrap_or(-1);
    let json: Value = serde_json::from_str(&stdout).unwrap_or_else(|_| {
        panic!(
            "Failed to parse JSON.\nargs: {args:?}\nstdout: {stdout}\nstderr: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    });
    (json, code)
}

fn run_ok(dir: &Path, args: &[&str]) -> Value {
    let (json, code) = run_ltp(dir, args);
    assert_eq!(code, 0, "ltp {args:?} failed: {json}");
    json
}

fn setup(dir: &Path) {
    run_ok(dir, &["init", "--name", "TreeLogic"]);
}

fn add_node(dir: &Path, label: &str, node_type: &str) -> String {
    let json = run_ok(dir, &["node", "add", label, "--type", node_type]);
    json["data"]["id"].as_str().unwrap().to_string()
}

fn create_tree(dir: &Path, tree_type: &str, name: &str) -> String {
    let json = run_ok(dir, &["tree", "new", tree_type, name]);
    json["data"]["id"].as_str().unwrap().to_string()
}

fn attach(dir: &Path, tree: &str, nodes: &[&str]) {
    for node in nodes {
        run_ok(dir, &["tree", "attach", "--tree", tree, "--node", node]);
    }
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

fn tree_path(dir: &Path, tree: &str) -> PathBuf {
    dir.join("trees").join(format!("{tree}.json"))
}

fn read_tree(dir: &Path, tree: &str) -> Value {
    serde_json::from_str(&std::fs::read_to_string(tree_path(dir, tree)).unwrap()).unwrap()
}

/// Rewrites the tree file the way a pre-ADR-014 workspace stored a GT: the tree and every
/// trunk edge as sufficiency. Returns the exact bytes written.
fn write_legacy_sufficiency(dir: &Path, tree: &str) -> String {
    let mut raw = read_tree(dir, tree);
    raw["logic"] = json!("sufficiency");
    for edge in raw["edges"].as_array_mut().unwrap() {
        edge["logic"] = json!("SUFFICIENCY");
    }
    let content = serde_json::to_string_pretty(&raw).unwrap();
    std::fs::write(tree_path(dir, tree), &content).unwrap();
    content
}

fn walk_ids(json: &Value) -> Vec<String> {
    json["data"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["id"].as_str().unwrap().to_string())
        .collect()
}

/// Minimal Dettmer GT: `NC → CSF → GOAL` (edges point child → parent).
struct GtChain {
    tree: String,
    nc: String,
    csf: String,
    goal: String,
    links: [String; 2],
}

fn gt_chain(dir: &Path) -> GtChain {
    let nc = add_node(dir, "Hay capacidad de transporte suficiente", "NC");
    let csf = add_node(dir, "Entregas fiables", "CSF");
    let goal = add_node(dir, "Entregamos en menos de 10 días", "GOAL");
    let tree = create_tree(dir, "gt", "GT Logistica");
    attach(dir, &tree, &[&nc, &csf, &goal]);
    let l1 = connect(dir, &tree, &nc, &csf);
    let l2 = connect(dir, &tree, &csf, &goal);
    GtChain {
        tree,
        nc,
        csf,
        goal,
        links: [l1, l2],
    }
}

/// T3.1: `tree new` derives the logic from the type, in the response and on disk.
#[test]
fn t3_1_tree_new_derives_logic_from_type() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup(dir);

    for (tree_type, logic) in [
        ("gt", "necessity"),
        ("ec", "necessity"),
        ("prt", "necessity"),
        ("crt", "sufficiency"),
        ("frt", "sufficiency"),
        ("tt", "sufficiency"),
    ] {
        let json = run_ok(
            dir,
            &["tree", "new", tree_type, &format!("Arbol {tree_type}")],
        );
        assert_eq!(json["data"]["logic"], logic, "{tree_type}: response");
        let id = json["data"]["id"].as_str().unwrap();
        assert_eq!(read_tree(dir, id)["logic"], logic, "{tree_type}: disk");
    }
}

/// T3.2 (review focus): a legacy GT reads as necessity on every read-only surface, and
/// none of them rewrites the file (an out-of-band write would break undo, ADR-009).
#[test]
fn t3_2_legacy_gt_reads_as_necessity_without_touching_disk() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup(dir);
    let gt = gt_chain(dir);
    let legacy = write_legacy_sufficiency(dir, &gt.tree);

    let list = run_ok(dir, &["tree", "list"]);
    let entry = list["data"]["trees"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["id"] == gt.tree.as_str())
        .unwrap();
    assert_eq!(entry["logic"], "necessity");

    let inspect = run_ok(dir, &["link", "inspect", &gt.links[0], "--tree", &gt.tree]);
    assert_eq!(inspect["data"]["logic"], "necessity");

    let walk = run_ok(dir, &["tree", "walk", &gt.tree, "--order", "topological"]);
    assert_eq!(
        walk_ids(&walk),
        [gt.nc.clone(), gt.csf.clone(), gt.goal.clone()]
    );

    run_ok(dir, &["validate", "--tree", &gt.tree]);

    assert_eq!(
        std::fs::read_to_string(tree_path(dir, &gt.tree)).unwrap(),
        legacy,
        "a read-only command rewrote the tree file"
    );
}

/// T3.3 (review focus): the first mutation persists the normalized tree, and undo
/// restores the legacy file byte for byte.
#[test]
fn t3_3_first_mutation_persists_normalized_and_undo_restores_legacy() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup(dir);
    let gt = gt_chain(dir);
    let legacy = write_legacy_sufficiency(dir, &gt.tree);

    let extra = add_node(dir, "Flota mantenida", "NC");
    attach(dir, &gt.tree, &[&extra]);

    let raw = read_tree(dir, &gt.tree);
    assert_eq!(raw["logic"], "necessity");
    let edges = raw["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 2);
    assert!(edges.iter().all(|e| e["logic"] == "NECESSITY"), "{edges:?}");

    run_ok(dir, &["undo"]);
    assert_eq!(
        std::fs::read_to_string(tree_path(dir, &gt.tree)).unwrap(),
        legacy,
        "undo must restore the legacy bytes exactly"
    );
}

/// T3.4: cloning a legacy GT writes the clone already normalized.
#[test]
fn t3_4_clone_of_legacy_gt_is_normalized() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup(dir);
    let gt = gt_chain(dir);
    write_legacy_sufficiency(dir, &gt.tree);

    let json = run_ok(dir, &["tree", "clone", &gt.tree, "--name", "GT Clon"]);
    let clone = read_tree(dir, json["data"]["new_id"].as_str().unwrap());
    assert_eq!(clone["logic"], "necessity");
    let edges = clone["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 2);
    assert!(edges.iter().all(|e| e["logic"] == "NECESSITY"), "{edges:?}");
}
