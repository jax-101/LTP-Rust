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

fn ids(links: &Value) -> Vec<String> {
    links
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l.as_str().unwrap().to_string())
        .collect()
}

/// Logic of `link` exactly as stored on disk (trunk or NBR), bypassing the normalizing read.
fn edge_logic_on_disk(dir: &Path, tree: &str, link: &str) -> String {
    let raw = read_tree(dir, tree);
    let nbr_edges = raw["nbr_branches"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|b| b["edges"].as_array().into_iter().flatten());
    raw["edges"]
        .as_array()
        .unwrap()
        .iter()
        .chain(nbr_edges)
        .find(|e| e["id"] == link)
        .unwrap_or_else(|| panic!("{link} not found in {tree}"))["logic"]
        .as_str()
        .unwrap()
        .to_string()
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

/// T4.1: every `link connect` shape (SINGLE, multi-destination, AND) writes NECESSITY in a GT.
#[test]
fn t4_1_connect_in_gt_writes_necessity_for_every_shape() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup(dir);
    let nc1 = add_node(dir, "Flota disponible", "NC");
    let nc2 = add_node(dir, "Conductores formados", "NC");
    let nc3 = add_node(dir, "Rutas planificadas", "NC");
    let csf1 = add_node(dir, "Entregas fiables", "CSF");
    let csf2 = add_node(dir, "Entregas rápidas", "CSF");
    let goal = add_node(dir, "Clientes satisfechos", "GOAL");
    let tree = create_tree(dir, "gt", "GT Formas");
    attach(dir, &tree, &[&nc1, &nc2, &nc3, &csf1, &csf2, &goal]);

    let single = connect(dir, &tree, &csf1, &goal);
    assert_eq!(
        edge_logic_on_disk(dir, &tree, &single),
        "NECESSITY",
        "SINGLE"
    );

    let to = format!("{csf1},{csf2}");
    let json = run_ok(
        dir,
        &[
            "link", "connect", "--tree", &tree, "--from", &nc1, "--to", &to,
        ],
    );
    let multi = ids(&json["data"]["created_links"]);
    assert_eq!(multi.len(), 2);
    for link in &multi {
        assert_eq!(
            edge_logic_on_disk(dir, &tree, link),
            "NECESSITY",
            "multi-dest {link}"
        );
    }

    let from = format!("{nc2},{nc3}");
    let json = run_ok(
        dir,
        &[
            "link",
            "connect",
            "--tree",
            &tree,
            "--from",
            &from,
            "--to",
            &csf2,
            "--operator",
            "AND",
        ],
    );
    let and = ids(&json["data"]["created_links"]);
    assert_eq!(and.len(), 1);
    assert_eq!(edge_logic_on_disk(dir, &tree, &and[0]), "NECESSITY", "AND");
}

/// T4.2 (review focus): an NBR branch stays SUFFICIENCY even inside a necessity tree (PRT),
/// while the trunk inherits NECESSITY.
#[test]
fn t4_2_nbr_edge_stays_sufficiency_in_prt() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup(dir);
    let obs = add_node(dir, "Falta de stock", "OBS");
    let io = add_node(dir, "Inventario visible", "IO");
    let inj = add_node(dir, "Kanban de reposición", "INJ");
    let ude = add_node(dir, "Sobrecoste de almacén", "UDE");
    let tree = create_tree(dir, "prt", "PRT Stock");
    attach(dir, &tree, &[&obs, &io, &inj]);

    let trunk = connect(dir, &tree, &obs, &io);
    assert_eq!(edge_logic_on_disk(dir, &tree, &trunk), "NECESSITY", "trunk");

    let json = run_ok(dir, &["nbr", "add", "--tree", &tree, "--source-node", &inj]);
    let nbr = json["data"]["nbr_id"].as_str().unwrap().to_string();
    let json = run_ok(
        dir,
        &[
            "link", "connect", "--tree", &tree, "--nbr", &nbr, "--from", &inj, "--to", &ude,
        ],
    );
    let nbr_link = ids(&json["data"]["created_links"]);
    assert_eq!(
        edge_logic_on_disk(dir, &tree, &nbr_link[0]),
        "SUFFICIENCY",
        "NBR edge"
    );
    assert_eq!(
        edge_logic_on_disk(dir, &tree, &trunk),
        "NECESSITY",
        "trunk after NBR"
    );
}

/// T4.3: control — a CRT keeps writing SUFFICIENCY.
#[test]
fn t4_3_connect_in_crt_writes_sufficiency() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup(dir);
    let rc = add_node(dir, "Previsión manual", "RC");
    let ude = add_node(dir, "Roturas de stock", "UDE");
    let tree = create_tree(dir, "crt", "CRT Stock");
    attach(dir, &tree, &[&rc, &ude]);

    let link = connect(dir, &tree, &rc, &ude);
    assert_eq!(edge_logic_on_disk(dir, &tree, &link), "SUFFICIENCY");
}
