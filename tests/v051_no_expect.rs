//! v0.5.1 T2 (D-8a) — `link connect --nbr` with an NBR that is not in the
//! tree must be a typed `NBR_NOT_FOUND` error: exit 1, workspace untouched,
//! lock released. Regression guard for the removed `.expect()`.

use std::path::Path;
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

fn id(json: &Value) -> String {
    json["data"]["id"].as_str().unwrap().to_string()
}

/// Every file under `dir` with its contents, sorted by path.
fn snapshot(dir: &Path) -> Vec<(String, Vec<u8>)> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                let rel = path.strip_prefix(root).unwrap().display().to_string();
                out.push((rel, std::fs::read(&path).unwrap()));
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

/// Two trees; `tree-a` has an NBR, `tree-b` has none. Returns
/// (tree_a, tree_b, nbr_of_a, inj, ude).
fn setup(dir: &Path) -> (String, String, String, String, String) {
    run_ok(dir, &["init", "--name", "T2"]);
    let tree_a = id(&run_ok(dir, &["tree", "new", "frt", "a"]));
    let tree_b = id(&run_ok(dir, &["tree", "new", "frt", "b"]));
    let inj = id(&run_ok(dir, &["node", "add", "inyeccion", "--type", "INJ"]));
    let ude = id(&run_ok(dir, &["node", "add", "efecto", "--type", "UDE"]));
    for tree in [&tree_a, &tree_b] {
        for node in [&inj, &ude] {
            run_ok(dir, &["tree", "attach", "--tree", tree, "--node", node]);
        }
    }
    run_ok(
        dir,
        &["nbr", "add", "--tree", &tree_a, "--source-node", &inj],
    );
    let tree_json: Value = serde_json::from_str(
        &std::fs::read_to_string(dir.join("trees").join(format!("{tree_a}.json"))).unwrap(),
    )
    .unwrap();
    let nbr = tree_json["nbr_branches"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    (tree_a, tree_b, nbr, inj, ude)
}

fn assert_nbr_not_found(dir: &Path, tree: &str, nbr: &str, from: &str, to: &str) {
    let before = snapshot(dir);
    let (json, code) = run_ltp(
        dir,
        &[
            "link", "connect", "--tree", tree, "--nbr", nbr, "--from", from, "--to", to,
        ],
    );
    assert_eq!(code, 1, "{json}");
    assert_eq!(json["success"], false);
    assert_eq!(json["action"], "link_connect");
    assert_eq!(json["errors"][0]["code"], "NBR_NOT_FOUND", "{json}");
    assert_eq!(
        json["errors"][0]["detail"],
        format!("NBR '{nbr}' not found in tree '{tree}'")
    );
    assert_eq!(json["data"]["created_links"], serde_json::json!([]));
    assert_eq!(
        snapshot(dir),
        before,
        "workspace modified by a failed connect"
    );
    assert!(!dir.join(".ltp/lock").exists(), "lock not released");
}

#[test]
fn t2_connect_unknown_nbr_is_typed_error() {
    let ws = tempfile::tempdir().unwrap();
    let dir = ws.path();
    let (tree_a, _, _, inj, ude) = setup(dir);
    assert_nbr_not_found(dir, &tree_a, "NBR-999", &inj, &ude);
}

#[test]
fn t2_connect_nbr_of_another_tree_is_typed_error() {
    let ws = tempfile::tempdir().unwrap();
    let dir = ws.path();
    let (_, tree_b, nbr, inj, ude) = setup(dir);
    assert_nbr_not_found(dir, &tree_b, &nbr, &inj, &ude);
}

#[test]
fn t2_failed_connect_leaves_workspace_usable() {
    let ws = tempfile::tempdir().unwrap();
    let dir = ws.path();
    let (tree_a, _, nbr, inj, ude) = setup(dir);
    assert_nbr_not_found(dir, &tree_a, "NBR-999", &inj, &ude);
    let json = run_ok(
        dir,
        &[
            "link", "connect", "--tree", &tree_a, "--nbr", &nbr, "--from", &inj, "--to", &ude,
        ],
    );
    assert_eq!(json["data"]["created_links"].as_array().unwrap().len(), 1);
}
