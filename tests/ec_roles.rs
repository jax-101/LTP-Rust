//! v0.3.1 — the advertised `role` vocabulary for `tree attach` (MCP schema and CLI
//! help) must match what the EC validator enforces. Before the fix the MCP schema
//! said "root, leaf, intermediate", which always fails EC validation.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use ltp_engine::validate::ec::{EC_ROLES, ROLE_HELP};
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

fn add_node(dir: &Path, label: &str, node_type: &str) -> String {
    run_ok(dir, &["node", "add", label, "--type", node_type])["data"]["id"]
        .as_str()
        .unwrap()
        .to_string()
}

fn tools_list() -> Vec<Value> {
    let dir = tempfile::tempdir().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_ltp-mcp"))
        .arg("--workspace")
        .arg(dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn ltp-mcp");
    let mut stdin = child.stdin.take().unwrap();
    writeln!(
        stdin,
        "{}",
        json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"})
    )
    .unwrap();
    drop(stdin);
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.lines().find(|l| !l.trim().is_empty()).unwrap();
    let response: Value = serde_json::from_str(line).unwrap();
    response["result"]["tools"].as_array().unwrap().clone()
}

/// Builds a structurally correct EC (OBJ <- REQ x2 <- PRE x2, PRE XOR) whose
/// attachments use `roles` = [objective, requirement, prerequisite] labels.
fn build_ec(dir: &Path, roles: [&str; 3]) -> String {
    run_ok(dir, &["init", "--name", "EC"]);
    let tree = run_ok(dir, &["tree", "new", "ec", "dilema"])["data"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let obj = add_node(dir, "objetivo", "OBJ");
    let reqs = [add_node(dir, "b", "REQ"), add_node(dir, "c", "REQ")];
    let pres = [add_node(dir, "d", "PRE"), add_node(dir, "d'", "PRE")];
    let attach = |node: &str, role: &str| {
        run_ok(
            dir,
            &[
                "tree", "attach", "--tree", &tree, "--node", node, "--role", role,
            ],
        );
    };
    attach(&obj, roles[0]);
    for (req, pre) in reqs.iter().zip(&pres) {
        attach(req, roles[1]);
        attach(pre, roles[2]);
    }
    for (req, pre) in reqs.iter().zip(&pres) {
        run_ok(
            dir,
            &[
                "link", "connect", "--tree", &tree, "--from", req, "--to", &obj,
            ],
        );
        run_ok(
            dir,
            &[
                "link", "connect", "--tree", &tree, "--from", pre, "--to", req,
            ],
        );
    }
    tree
}

fn validate(dir: &Path, tree: &str) -> (Value, i32) {
    run_ltp(dir, &["validate", "--tree", tree])
}

fn sub_rules(json: &Value) -> Vec<String> {
    json["data"]["details"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|d| d["errors"].as_array().unwrap().clone())
        .filter(|e| e["code"] == "EC_VALIDATION")
        .map(|e| e["sub_rule"].as_str().unwrap_or_default().to_string())
        .collect()
}

#[test]
fn role_help_names_every_enforced_role() {
    for role in EC_ROLES {
        assert!(ROLE_HELP.contains(role), "ROLE_HELP misses '{role}'");
    }
}

// U10
#[test]
fn u10_mcp_tree_attach_role_description_matches_validator() {
    let tools = tools_list();
    let attach = tools
        .iter()
        .find(|t| t["name"] == "ltp/tree_attach")
        .expect("ltp/tree_attach missing");
    let description = attach["inputSchema"]["properties"]["role"]["description"]
        .as_str()
        .unwrap();
    for role in EC_ROLES {
        assert!(
            description.contains(role),
            "missing '{role}': {description}"
        );
    }
    for wrong in ["root", "leaf", "intermediate"] {
        assert!(
            !description.contains(wrong),
            "description still advertises '{wrong}': {description}"
        );
    }
}

// U11
#[test]
fn u11_ec_with_advertised_legacy_roles_fails_and_correct_roles_pass() {
    let dir = tempfile::tempdir().unwrap();
    let tree = build_ec(dir.path(), ["root", "intermediate", "leaf"]);
    let (json, code) = validate(dir.path(), &tree);
    assert_ne!(code, 0);
    assert_eq!(json["success"], false);
    let rules = sub_rules(&json);
    assert!(
        rules.contains(&"missing_objective".to_string()),
        "{rules:?}"
    );
    assert!(
        rules.contains(&"minimum_2_requirements".to_string()),
        "{rules:?}"
    );

    let dir = tempfile::tempdir().unwrap();
    let tree = build_ec(dir.path(), EC_ROLES);
    let (json, code) = validate(dir.path(), &tree);
    assert_eq!(code, 0, "{json}");
    assert_eq!(json["success"], true);
    assert!(sub_rules(&json).is_empty());
}

// U12
#[test]
fn u12_near_miss_roles_are_rejected() {
    for roles in [
        ["Objective", "requirement", "prerequisite"],
        [" objective", "requirement", "prerequisite"],
        ["objective", "requirements", "prerequisite"],
        ["objective", "requirement", "PREREQUISITE"],
    ] {
        let dir = tempfile::tempdir().unwrap();
        let tree = build_ec(dir.path(), roles);
        let (json, _) = validate(dir.path(), &tree);
        assert_eq!(json["success"], false, "{roles:?} must fail: {json}");
        assert!(!sub_rules(&json).is_empty(), "{roles:?}");
    }
}

// U13
#[test]
fn u13_cli_attach_help_lists_the_roles() {
    let output = Command::new(env!("CARGO_BIN_EXE_ltp"))
        .args(["tree", "attach", "--help"])
        .output()
        .unwrap();
    let help = String::from_utf8_lossy(&output.stdout);
    for role in EC_ROLES {
        assert!(help.contains(role), "--help misses '{role}':\n{help}");
    }
}
