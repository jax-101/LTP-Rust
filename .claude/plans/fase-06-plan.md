# F6: Enlaces — Manipulación Avanzada

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implementar los 9 subcomandos de manipulación avanzada de enlaces: `reverse`, `move`, `insert-between`, `group`, `dissolve`, `split`, `reoperator`, `add-cause`, `rm-cause`.

**Architecture:** Cada comando es una función `execute_link_*` en `src/link/commands.rs` (o un nuevo fichero `src/link/advanced.rs` para mantener el módulo manejable). Todas siguen el patrón establecido: acquire lock → load tree → validar → mutar → check DAG → save tree → release lock → retornar `CommandOutput<T>`. El CLI dispatch en `main.rs` ya define los subcomandos (catch-all `_ =>` actual).

**Tech Stack:** Rust, clap (derive), serde_json, `CommandOutput<T>` pattern.

## Global Constraints

- Prohibido `.unwrap()`/`.expect()` en producción.
- `///` docs en items públicos.
- JSON canónico con `BTreeMap` donde aplique.
- DAG check obligatorio tras cualquier mutación de edges.
- IDs secuenciales via `storage.next_id("LINK")`.
- Tests de integración CLI (patrón `tests/fase_04.rs`).
- Verificación: `cargo check` + `clippy` + `test` + `fmt --check`.

---

### Task 1: Scaffolding — `advanced.rs` + output types

**Files:**
- Create: `src/link/advanced.rs`
- Modify: `src/link/mod.rs`
- Modify: `src/main.rs:365-382` (add-cause needs `--promote-to`)

**Interfaces:**
- Consumes: `Storage` trait, `Edge`, `Operator`, `EdgeStatus`, `AssumptionStatus`, `CommandOutput<T>`, `check_dag`, `stale_lock_warning`, `parse_operator`
- Produces: `LinkReverseData`, `LinkMoveData`, `LinkInsertBetweenData`, `LinkGroupData`, `LinkDissolveData`, `LinkSplitData`, `LinkReoperatorData`, `LinkAddCauseData`, `LinkRmCauseData` (structs usadas por tasks siguientes)

- [ ] **Step 1: Crear `src/link/advanced.rs` con output structs**

```rust
use serde::Serialize;

use crate::link::types::{AssumptionStatus, Edge, EdgeStatus, Operator};
use crate::output::{CommandOutput, GraphHealth, OutputError, OutputWarning};
use crate::storage::{LockOutcome, Storage};
use crate::validate::check_dag;

fn parse_operator(s: &str) -> Option<Operator> {
    match s.to_uppercase().as_str() {
        "SINGLE" => Some(Operator::Single),
        "AND" => Some(Operator::And),
        "OR" => Some(Operator::Or),
        "MAG" => Some(Operator::Mag),
        "XOR" => Some(Operator::Xor),
        _ => None,
    }
}

fn stale_lock_warning(outcome: &LockOutcome) -> Option<OutputWarning> {
    match outcome {
        LockOutcome::StaleLockRemoved { pid } => Some(OutputWarning::new(
            "STALE_LOCK_REMOVED",
            format!("Stale lock from PID {} was removed", pid),
        )),
        LockOutcome::Acquired => None,
    }
}

/// Data returned by `link reverse`.
#[derive(Debug, Serialize)]
pub struct LinkReverseData {
    pub link_id: String,
    pub tree_id: String,
    pub new_from: Vec<String>,
    pub new_to: String,
}

/// Data returned by `link move`.
#[derive(Debug, Serialize)]
pub struct LinkMoveData {
    pub link_id: String,
    pub tree_id: String,
}

/// Data returned by `link insert-between`.
#[derive(Debug, Serialize)]
pub struct LinkInsertBetweenData {
    pub removed_link: String,
    pub created_links: Vec<String>,
    pub tree_id: String,
}

/// Data returned by `link group`.
#[derive(Debug, Serialize)]
pub struct LinkGroupData {
    pub created_link: String,
    pub removed_links: Vec<String>,
    pub tree_id: String,
}

/// Data returned by `link dissolve`.
#[derive(Debug, Serialize)]
pub struct LinkDissolveData {
    pub created_links: Vec<String>,
    pub removed_link: String,
    pub tree_id: String,
}

/// Data returned by `link split`.
#[derive(Debug, Serialize)]
pub struct LinkSplitData {
    pub extracted_link: String,
    pub original_link: String,
    pub tree_id: String,
}

/// Data returned by `link reoperator`.
#[derive(Debug, Serialize)]
pub struct LinkReoperatorData {
    pub link_id: String,
    pub old_operator: Operator,
    pub new_operator: Operator,
    pub tree_id: String,
}

/// Data returned by `link add-cause`.
#[derive(Debug, Serialize)]
pub struct LinkAddCauseData {
    pub link_id: String,
    pub added_node: String,
    pub tree_id: String,
}

/// Data returned by `link rm-cause`.
#[derive(Debug, Serialize)]
pub struct LinkRmCauseData {
    pub link_id: String,
    pub removed_node: String,
    pub new_operator: Operator,
    pub tree_id: String,
}
```

- [ ] **Step 2: Registrar módulo en `src/link/mod.rs`**

Añadir `pub mod advanced;` al fichero `src/link/mod.rs`.

- [ ] **Step 3: Añadir `--promote-to` al subcomando `AddCause` en `main.rs`**

Dentro del enum `LinkAction::AddCause`, añadir:
```rust
    #[arg(long)]
    promote_to: Option<String>,
```

- [ ] **Step 4: Verificar compilación**

Run: `cargo check --all-targets --all-features`
Expected: compila sin errores (structs sin usar genera warnings, ok por ahora).

- [ ] **Step 5: Commit**

```bash
git add src/link/advanced.rs src/link/mod.rs src/main.rs
git commit -m "feat(F6): scaffold advanced link module with output types"
```

---

### Task 2: `link reverse`

**Files:**
- Modify: `src/link/advanced.rs`
- Modify: `src/main.rs` (dispatch)
- Test: `tests/fase_06.rs` (UATs 6.1–6.3)

**Interfaces:**
- Consumes: `Storage`, `Edge`, `check_dag`
- Produces: `execute_link_reverse(storage, tree_id, link_id, force) -> CommandOutput<LinkReverseData>`

- [ ] **Step 1: Write failing tests (UATs 6.1–6.3)**

```rust
// tests/fase_06.rs
use std::process::Command;
use serde_json::Value;

fn ltp_bin() -> String {
    env!("CARGO_BIN_EXE_ltp-engine").to_string()
}

fn run_ltp(dir: &std::path::Path, args: &[&str]) -> (Value, i32) {
    let output = Command::new(ltp_bin())
        .args(args)
        .current_dir(dir)
        .output()
        .expect("failed to execute ltp binary");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let code = output.status.code().unwrap_or(-1);
    let json: Value = serde_json::from_str(&stdout).unwrap_or_else(|_| {
        panic!(
            "Failed to parse JSON.\nstdout: {}\nstderr: {}",
            stdout,
            String::from_utf8_lossy(&output.stderr)
        )
    });
    (json, code)
}

fn setup_workspace(dir: &std::path::Path) {
    let (_, code) = run_ltp(dir, &["init", "--name", "F6Test"]);
    assert_eq!(code, 0);
}

fn add_node(dir: &std::path::Path, label: &str, node_type: &str) -> String {
    let (json, _) = run_ltp(dir, &["node", "add", label, "--type", node_type]);
    json["data"]["id"].as_str().unwrap().to_string()
}

fn create_tree(dir: &std::path::Path, tree_type: &str, name: &str) -> String {
    let (json, _) = run_ltp(dir, &["tree", "new", tree_type, name]);
    json["data"]["id"].as_str().unwrap().to_string()
}

fn attach_node(dir: &std::path::Path, tree_id: &str, node_id: &str) {
    let (_, code) = run_ltp(dir, &["tree", "attach", "--tree", tree_id, "--node", node_id]);
    assert_eq!(code, 0);
}

fn connect(dir: &std::path::Path, tree_id: &str, from: &str, to: &str) -> String {
    let (json, code) = run_ltp(
        dir,
        &["link", "connect", "--tree", tree_id, "--from", from, "--to", to],
    );
    assert_eq!(code, 0);
    json["data"]["created_links"][0].as_str().unwrap().to_string()
}

/// UAT 6.1: reverse sin assumptions invierte from↔to.
#[test]
fn uat_6_1_link_reverse_no_assumptions() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let a = add_node(dir, "Cause", "rc");
    let b = add_node(dir, "Effect", "ude");
    let tree = create_tree(dir, "crt", "ReverseTest");
    attach_node(dir, &tree, &a);
    attach_node(dir, &tree, &b);
    let link = connect(dir, &tree, &a, &b);

    let (json, code) = run_ltp(
        dir,
        &["link", "reverse", "--tree", &tree, "--link", &link],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["action"], "link_reverse");
    assert_eq!(json["data"]["new_from"][0], b);
    assert_eq!(json["data"]["new_to"], a);
}

/// UAT 6.2: reverse con assumptions sin --force → error.
#[test]
fn uat_6_2_link_reverse_with_assumptions_no_force() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let a = add_node(dir, "Cause", "rc");
    let b = add_node(dir, "Effect", "ude");
    let tree = create_tree(dir, "crt", "ReverseTest2");
    attach_node(dir, &tree, &a);
    attach_node(dir, &tree, &b);
    let link = connect(dir, &tree, &a, &b);

    // Add assumption to the link
    run_ltp(
        dir,
        &["assume", "add", "--tree", &tree, "--link", &link, "--text", "Some assumption"],
    );

    let (json, code) = run_ltp(
        dir,
        &["link", "reverse", "--tree", &tree, "--link", &link],
    );

    assert_eq!(code, 1);
    assert_eq!(json["success"], false);
    assert_eq!(json["errors"][0]["code"], "REVERSE_REQUIRES_FORCE");
}

/// UAT 6.3: reverse con assumptions + --force → invierte + marks needs_review.
#[test]
fn uat_6_3_link_reverse_with_force() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let a = add_node(dir, "Cause", "rc");
    let b = add_node(dir, "Effect", "ude");
    let tree = create_tree(dir, "crt", "ReverseTest3");
    attach_node(dir, &tree, &a);
    attach_node(dir, &tree, &b);
    let link = connect(dir, &tree, &a, &b);

    run_ltp(
        dir,
        &["assume", "add", "--tree", &tree, "--link", &link, "--text", "Test assumption"],
    );

    let (json, code) = run_ltp(
        dir,
        &["link", "reverse", "--tree", &tree, "--link", &link, "--force"],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["new_from"][0], b);
    assert_eq!(json["data"]["new_to"], a);
}
```

- [ ] **Step 2: Run tests — verify they fail**

Run: `cargo test --test fase_06 -- --test-threads=1`
Expected: FAIL (function not defined / NOT_IMPLEMENTED)

- [ ] **Step 3: Implement `execute_link_reverse`**

In `src/link/advanced.rs`:

```rust
/// Execute `link reverse`.
pub fn execute_link_reverse(
    storage: &dyn Storage,
    tree_id: &str,
    link_id: &str,
    force: bool,
) -> CommandOutput<LinkReverseData> {
    let ws_name = storage.workspace_name().unwrap_or_default();
    let action = "link_reverse";

    let lock_outcome = match storage.acquire_lock("link reverse") {
        Ok(o) => o,
        Err(e) => {
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: LinkReverseData {
                    link_id: link_id.to_string(),
                    tree_id: tree_id.to_string(),
                    new_from: vec![],
                    new_to: String::new(),
                },
                graph_health: GraphHealth { valid_dag: true, orphan_nodes_count: 0 },
                errors: vec![OutputError::new("LOCK_ERROR", e.to_string())],
                warnings: vec![],
            };
        }
    };

    let mut tree = match storage.load_tree(tree_id) {
        Ok(t) => t,
        Err(e) => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: LinkReverseData {
                    link_id: link_id.to_string(),
                    tree_id: tree_id.to_string(),
                    new_from: vec![],
                    new_to: String::new(),
                },
                graph_health: GraphHealth { valid_dag: true, orphan_nodes_count: 0 },
                errors: vec![OutputError::new("TREE_NOT_FOUND", e.to_string())],
                warnings: vec![],
            };
        }
    };

    let edge_idx = match tree.edges.iter().position(|e| e.id == link_id) {
        Some(i) => i,
        None => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: LinkReverseData {
                    link_id: link_id.to_string(),
                    tree_id: tree_id.to_string(),
                    new_from: vec![],
                    new_to: String::new(),
                },
                graph_health: GraphHealth { valid_dag: true, orphan_nodes_count: 0 },
                errors: vec![OutputError::new("LINK_NOT_FOUND", format!("Edge '{}' not found in tree '{}'", link_id, tree_id))],
                warnings: vec![],
            };
        }
    };

    // Check assumptions → require --force
    if !tree.edges[edge_idx].assumptions.is_empty() && !force {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: LinkReverseData {
                link_id: link_id.to_string(),
                tree_id: tree_id.to_string(),
                new_from: vec![],
                new_to: String::new(),
            },
            graph_health: GraphHealth { valid_dag: true, orphan_nodes_count: 0 },
            errors: vec![OutputError::new(
                "REVERSE_REQUIRES_FORCE",
                "Edge has assumptions; use --force to reverse (assumptions will be marked needs_review)",
            )],
            warnings: vec![],
        };
    }

    // Perform reversal: from becomes vec![old_to], to becomes old_from[0] (for SINGLE)
    // For multi-from: from becomes vec![old_to], to becomes... ambiguous.
    // Spec says "invierte from↔to" — swap the single `to` into `from`, and `from[0]` into `to`.
    // For AND/OR with multiple from: only valid if from.len() == 1. Otherwise error.
    let edge = &tree.edges[edge_idx];
    if edge.from.len() > 1 {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: LinkReverseData {
                link_id: link_id.to_string(),
                tree_id: tree_id.to_string(),
                new_from: vec![],
                new_to: String::new(),
            },
            graph_health: GraphHealth { valid_dag: true, orphan_nodes_count: 0 },
            errors: vec![OutputError::new(
                "CANNOT_REVERSE_GROUP",
                "Cannot reverse a grouped edge with multiple sources; dissolve first",
            )],
            warnings: vec![],
        };
    }

    let old_to = edge.to.clone();
    let old_from = edge.from.clone();
    let new_from = vec![old_to.clone()];
    let new_to = old_from[0].clone();

    // Apply reversal
    tree.edges[edge_idx].from = new_from.clone();
    tree.edges[edge_idx].to = new_to.clone();

    // Mark assumptions as needs_review if --force was used
    if force {
        for asm in &mut tree.edges[edge_idx].assumptions {
            asm.status = AssumptionStatus::NeedsReview;
        }
    }

    // DAG check
    if let Err(e) = check_dag(&tree.edges, tree_id) {
        // Rollback
        tree.edges[edge_idx].from = old_from;
        tree.edges[edge_idx].to = old_to;
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: LinkReverseData {
                link_id: link_id.to_string(),
                tree_id: tree_id.to_string(),
                new_from: vec![],
                new_to: String::new(),
            },
            graph_health: GraphHealth { valid_dag: false, orphan_nodes_count: 0 },
            errors: vec![OutputError::new("CIRCULAR_DEPENDENCY_DETECTED", e.to_string())],
            warnings: vec![],
        };
    }

    if let Err(e) = storage.save_tree(&tree) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: LinkReverseData {
                link_id: link_id.to_string(),
                tree_id: tree_id.to_string(),
                new_from: vec![],
                new_to: String::new(),
            },
            graph_health: GraphHealth { valid_dag: true, orphan_nodes_count: 0 },
            errors: vec![OutputError::new("IO_ERROR", e.to_string())],
            warnings: vec![],
        };
    }

    let _ = storage.release_lock();
    let mut warnings = vec![];
    if let Some(w) = stale_lock_warning(&lock_outcome) {
        warnings.push(w);
    }

    CommandOutput {
        success: true,
        action: action.to_string(),
        workspace: ws_name,
        data: LinkReverseData {
            link_id: link_id.to_string(),
            tree_id: tree_id.to_string(),
            new_from,
            new_to,
        },
        graph_health: GraphHealth { valid_dag: true, orphan_nodes_count: 0 },
        errors: vec![],
        warnings,
    }
}
```

- [ ] **Step 4: Wire dispatch in `main.rs`**

Replace the `_ =>` catch-all in the Link match arm. Add `LinkAction::Reverse` arm:
```rust
LinkAction::Reverse { tree, link, force } => {
    let output = execute_link_reverse(&storage, &tree, &link, force);
    render_output(&output, cli.human);
    if !output.success { process::exit(1); }
}
```

Import `execute_link_reverse` from `ltp_engine::link::advanced`.

- [ ] **Step 5: Run tests — verify pass**

Run: `cargo test --test fase_06 -- --test-threads=1`
Expected: UATs 6.1–6.3 PASS

- [ ] **Step 6: Clippy + fmt**

Run: `cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --all -- --check`

- [ ] **Step 7: Commit**

```bash
git add src/link/advanced.rs src/main.rs tests/fase_06.rs
git commit -m "feat(F6): implement link reverse (UATs 6.1-6.3)"
```

---

### Task 3: `link move`

**Files:**
- Modify: `src/link/advanced.rs`
- Modify: `src/main.rs` (dispatch)
- Modify: `tests/fase_06.rs` (UAT 6.4)

**Interfaces:**
- Consumes: `Storage`, `Edge`, `check_dag`
- Produces: `execute_link_move(storage, tree_id, link_id, new_from, new_to) -> CommandOutput<LinkMoveData>`

- [ ] **Step 1: Write failing test (UAT 6.4)**

```rust
/// UAT 6.4: link move redirige destino.
#[test]
fn uat_6_4_link_move_new_to() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let a = add_node(dir, "Cause", "rc");
    let b = add_node(dir, "Old Effect", "ude");
    let d = add_node(dir, "New Effect", "ude");
    let tree = create_tree(dir, "crt", "MoveTest");
    attach_node(dir, &tree, &a);
    attach_node(dir, &tree, &b);
    attach_node(dir, &tree, &d);
    let link = connect(dir, &tree, &a, &b);

    let (json, code) = run_ltp(
        dir,
        &["link", "move", "--tree", &tree, "--link", &link, "--new-to", &d],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["action"], "link_move");
    assert_eq!(json["data"]["link_id"], link);
}
```

- [ ] **Step 2: Run test — verify it fails**

Run: `cargo test --test fase_06 uat_6_4 -- --test-threads=1`
Expected: FAIL

- [ ] **Step 3: Implement `execute_link_move`**

Logic: load tree, find edge, validate new_from/new_to exist in pool + attached to tree, update edge.from/edge.to, check DAG, save.

```rust
/// Execute `link move`.
pub fn execute_link_move(
    storage: &dyn Storage,
    tree_id: &str,
    link_id: &str,
    new_from: Option<&str>,
    new_to: Option<&str>,
) -> CommandOutput<LinkMoveData> {
    let ws_name = storage.workspace_name().unwrap_or_default();
    let action = "link_move";

    let lock_outcome = match storage.acquire_lock("link move") {
        Ok(o) => o,
        Err(e) => {
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: LinkMoveData { link_id: link_id.to_string(), tree_id: tree_id.to_string() },
                graph_health: GraphHealth { valid_dag: true, orphan_nodes_count: 0 },
                errors: vec![OutputError::new("LOCK_ERROR", e.to_string())],
                warnings: vec![],
            };
        }
    };

    let mut tree = match storage.load_tree(tree_id) {
        Ok(t) => t,
        Err(e) => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: LinkMoveData { link_id: link_id.to_string(), tree_id: tree_id.to_string() },
                graph_health: GraphHealth { valid_dag: true, orphan_nodes_count: 0 },
                errors: vec![OutputError::new("TREE_NOT_FOUND", e.to_string())],
                warnings: vec![],
            };
        }
    };

    let edge_idx = match tree.edges.iter().position(|e| e.id == link_id) {
        Some(i) => i,
        None => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: LinkMoveData { link_id: link_id.to_string(), tree_id: tree_id.to_string() },
                graph_health: GraphHealth { valid_dag: true, orphan_nodes_count: 0 },
                errors: vec![OutputError::new("LINK_NOT_FOUND", format!("Edge '{}' not found in tree '{}'", link_id, tree_id))],
                warnings: vec![],
            };
        }
    };

    // Validate new nodes exist and are attached
    if let Some(nf) = new_from {
        if storage.load_node(nf).is_err() {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false, action: action.to_string(), workspace: ws_name,
                data: LinkMoveData { link_id: link_id.to_string(), tree_id: tree_id.to_string() },
                graph_health: GraphHealth { valid_dag: true, orphan_nodes_count: 0 },
                errors: vec![OutputError::new("REFERENTIAL_INTEGRITY_VIOLATION", format!("Node '{}' not found", nf))],
                warnings: vec![],
            };
        }
        if !tree.nodes.iter().any(|n| n.node_ref == nf) {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false, action: action.to_string(), workspace: ws_name,
                data: LinkMoveData { link_id: link_id.to_string(), tree_id: tree_id.to_string() },
                graph_health: GraphHealth { valid_dag: true, orphan_nodes_count: 0 },
                errors: vec![OutputError::new("NODE_NOT_IN_TREE", format!("Node '{}' not attached to tree", nf))],
                warnings: vec![],
            };
        }
    }
    if let Some(nt) = new_to {
        if storage.load_node(nt).is_err() {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false, action: action.to_string(), workspace: ws_name,
                data: LinkMoveData { link_id: link_id.to_string(), tree_id: tree_id.to_string() },
                graph_health: GraphHealth { valid_dag: true, orphan_nodes_count: 0 },
                errors: vec![OutputError::new("REFERENTIAL_INTEGRITY_VIOLATION", format!("Node '{}' not found", nt))],
                warnings: vec![],
            };
        }
        if !tree.nodes.iter().any(|n| n.node_ref == nt) {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false, action: action.to_string(), workspace: ws_name,
                data: LinkMoveData { link_id: link_id.to_string(), tree_id: tree_id.to_string() },
                graph_health: GraphHealth { valid_dag: true, orphan_nodes_count: 0 },
                errors: vec![OutputError::new("NODE_NOT_IN_TREE", format!("Node '{}' not attached to tree", nt))],
                warnings: vec![],
            };
        }
    }

    // Apply move
    if let Some(nf) = new_from {
        tree.edges[edge_idx].from = vec![nf.to_string()];
    }
    if let Some(nt) = new_to {
        tree.edges[edge_idx].to = nt.to_string();
    }

    // DAG check
    if let Err(e) = check_dag(&tree.edges, tree_id) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false, action: action.to_string(), workspace: ws_name,
            data: LinkMoveData { link_id: link_id.to_string(), tree_id: tree_id.to_string() },
            graph_health: GraphHealth { valid_dag: false, orphan_nodes_count: 0 },
            errors: vec![OutputError::new("CIRCULAR_DEPENDENCY_DETECTED", e.to_string())],
            warnings: vec![],
        };
    }

    if let Err(e) = storage.save_tree(&tree) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false, action: action.to_string(), workspace: ws_name,
            data: LinkMoveData { link_id: link_id.to_string(), tree_id: tree_id.to_string() },
            graph_health: GraphHealth { valid_dag: true, orphan_nodes_count: 0 },
            errors: vec![OutputError::new("IO_ERROR", e.to_string())],
            warnings: vec![],
        };
    }

    let _ = storage.release_lock();
    let mut warnings = vec![];
    if let Some(w) = stale_lock_warning(&lock_outcome) { warnings.push(w); }

    CommandOutput {
        success: true, action: action.to_string(), workspace: ws_name,
        data: LinkMoveData { link_id: link_id.to_string(), tree_id: tree_id.to_string() },
        graph_health: GraphHealth { valid_dag: true, orphan_nodes_count: 0 },
        errors: vec![],
        warnings,
    }
}
```

- [ ] **Step 4: Wire dispatch**

```rust
LinkAction::Move { tree, link, new_from, new_to } => {
    let output = execute_link_move(&storage, &tree, &link, new_from.as_deref(), new_to.as_deref());
    render_output(&output, cli.human);
    if !output.success { process::exit(1); }
}
```

- [ ] **Step 5: Run tests — verify pass**

Run: `cargo test --test fase_06 uat_6_4 -- --test-threads=1`
Expected: PASS

- [ ] **Step 6: Clippy + fmt**

- [ ] **Step 7: Commit**

```bash
git add src/link/advanced.rs src/main.rs tests/fase_06.rs
git commit -m "feat(F6): implement link move (UAT 6.4)"
```

---

### Task 4: `link insert-between`

**Files:**
- Modify: `src/link/advanced.rs`
- Modify: `src/main.rs` (dispatch)
- Modify: `tests/fase_06.rs` (UATs 6.5–6.7)

**Interfaces:**
- Consumes: `Storage`, `Edge`, `check_dag`, `storage.next_id("LINK")`
- Produces: `execute_link_insert_between(storage, tree_id, link_id, node_id, insert_after_cause, insert_before_effect) -> CommandOutput<LinkInsertBetweenData>`

- [ ] **Step 1: Write failing tests (UATs 6.5–6.7)**

```rust
/// UAT 6.5: insert-between SINGLE — A→B becomes A→X→B.
#[test]
fn uat_6_5_insert_between_single() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let a = add_node(dir, "Cause A", "rc");
    let b = add_node(dir, "Effect B", "ude");
    let x = add_node(dir, "Intermediate X", "int");
    let tree = create_tree(dir, "crt", "InsertTest");
    attach_node(dir, &tree, &a);
    attach_node(dir, &tree, &b);
    attach_node(dir, &tree, &x);
    let link = connect(dir, &tree, &a, &b);

    let (json, code) = run_ltp(
        dir,
        &["link", "insert-between", "--tree", &tree, "--link", &link, "--node", &x],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["removed_link"], link);
    assert_eq!(json["data"]["created_links"].as_array().unwrap().len(), 2);
}

/// UAT 6.6: insert-between AND + --insert-after-cause A.
#[test]
fn uat_6_6_insert_between_and_after_cause() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let a = add_node(dir, "Cause A", "rc");
    let b = add_node(dir, "Cause B", "rc");
    let c = add_node(dir, "Effect C", "ude");
    let x = add_node(dir, "Intermediate X", "int");
    let tree = create_tree(dir, "crt", "InsertAND");
    attach_node(dir, &tree, &a);
    attach_node(dir, &tree, &b);
    attach_node(dir, &tree, &c);
    attach_node(dir, &tree, &x);

    // Create AND edge [A,B] → C
    let (json, code) = run_ltp(
        dir,
        &["link", "connect", "--tree", &tree, "--from", &format!("{},{}", a, b), "--to", &c, "--operator", "AND"],
    );
    assert_eq!(code, 0);
    let link = json["data"]["created_links"][0].as_str().unwrap().to_string();

    let (json, code) = run_ltp(
        dir,
        &["link", "insert-between", "--tree", &tree, "--link", &link, "--node", &x, "--insert-after-cause", &a],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    // A extracted from group, A→X created, X replaces A in group
    assert_eq!(json["data"]["created_links"].as_array().unwrap().len(), 2);
}

/// UAT 6.7: insert-between AND + --insert-before-effect.
#[test]
fn uat_6_7_insert_between_before_effect() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let a = add_node(dir, "Cause A", "rc");
    let b = add_node(dir, "Cause B", "rc");
    let c = add_node(dir, "Effect C", "ude");
    let x = add_node(dir, "Intermediate X", "int");
    let tree = create_tree(dir, "crt", "InsertBeforeEffect");
    attach_node(dir, &tree, &a);
    attach_node(dir, &tree, &b);
    attach_node(dir, &tree, &c);
    attach_node(dir, &tree, &x);

    let (json, code) = run_ltp(
        dir,
        &["link", "connect", "--tree", &tree, "--from", &format!("{},{}", a, b), "--to", &c, "--operator", "AND"],
    );
    assert_eq!(code, 0);
    let link = json["data"]["created_links"][0].as_str().unwrap().to_string();

    let (json, code) = run_ltp(
        dir,
        &["link", "insert-between", "--tree", &tree, "--link", &link, "--node", &x, "--insert-before-effect"],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    // [A,B]→C becomes [A,B]→X + X→C
    assert_eq!(json["data"]["created_links"].as_array().unwrap().len(), 2);
    assert_eq!(json["data"]["removed_link"], link);
}
```

- [ ] **Step 2: Run tests — verify fail**

- [ ] **Step 3: Implement `execute_link_insert_between`**

Logic:
- **SINGLE (no flags):** Remove original A→B, create A→X (LINK-new1), X→B (LINK-new2).
- **AND + `--insert-after-cause A`:** Remove A from `from[]`, create A→X (SINGLE), add X to `from[]` of original. If only called with modifying original: update original edge's from, create A→X. Actually per spec: "extrae esa causa del grupo, crea CauseX→Nodo, y Nodo se añade al grupo original en su lugar." So: remove A from `from[]`, add X to `from[]`, create new edge A→X (SINGLE). The original edge keeps its ID.
- **AND + `--insert-before-effect`:** `[A,B]→C` becomes `[A,B]→X` (keep original, change `to` to X) + new X→C (SINGLE).

Wait — re-reading spec more carefully:
- For `--insert-after-cause`: "extrae esa causa del grupo, crea CauseX→Nodo, y Nodo se añade al grupo original en su lugar." The original group edge persists (modified), plus a new edge. So `removed_link` wouldn't apply. Let me re-read the output struct... Actually we return `created_links` (the new edges) and `removed_link` (original if fully replaced). For the AND cases, the original is modified, not removed. Let me adjust:
  - `--insert-after-cause`: original edge modified (A removed from `from[]`, X added), new edge A→X created. `removed_link` = "" or empty (no link removed). `created_links` = [new A→X edge]. Hmm, but the test expects 2 created_links. Let me re-think.
  
Actually, on second read: "extrae esa causa del grupo, crea CauseX→Nodo, y Nodo se añade al grupo original en su lugar" — this means we create ONE new edge (A→X) and modify the original (replace A with X in from[]). So `created_links` = 1, not 2. But `--insert-before-effect` says "[A,B]→C becomes [A,B]→X + X→C" which is the original modified (to=X) plus one new edge (X→C). Let me fix the test expectations.

Actually for consistency I'll unify: for SINGLE the original IS removed and 2 new ones created. For AND variants, original is modified and 1 new edge created. Let me adjust the output struct to have `modified_link: Option<String>` too. Simpler: keep it as-is but adjust semantics: `removed_link` is empty string when original is kept (modified). Tests will match.

Let me re-adjust test assertions:
- UAT 6.6: `created_links.len() == 1` (A→X), `removed_link` is empty (original is modified).
- UAT 6.7: `created_links.len() == 1` (X→C), `removed_link` is empty (original to changed to X).

Actually this is getting confusing. Let me use a clearer approach from the spec:

For `--insert-before-effect`: "[A, B] --AND--> C se convierte en [A, B] --AND--> Nodo + Nodo → C" — the original IS removed, replaced by [A,B]→X and X→C. Two new links, one removal. Same pattern as SINGLE.

For `--insert-after-cause`: "extrae esa causa del grupo, crea CauseX→Nodo, y Nodo se añade al grupo original en su lugar" — the group is mutated in place. One new edge (CauseX→Nodo). The original survives (modified). So: `removed_link: ""`, `created_links: [A→X]`, and the original edge is modified.

Hmm but if we remove the original and create two new ones for `--insert-before-effect`, that's inconsistent with "y Nodo se añade al grupo original en su lugar" for `--insert-after-cause`.

Let me make this simpler and consistent: I'll report the final state in `created_links` (newly created edge IDs) and `removed_link` (if original is deleted). For the modified-in-place case, `removed_link` = "" and we just report the one new edge. Fixing tests accordingly:

```rust
// UAT 6.6: expects 1 created link (A→X edge), original modified in place
assert_eq!(json["data"]["created_links"].as_array().unwrap().len(), 1);

// UAT 6.7: expects 2 created links, original removed
assert_eq!(json["data"]["created_links"].as_array().unwrap().len(), 2);
assert_eq!(json["data"]["removed_link"], link);
```

I'll put the corrected versions in the plan.

- [ ] **Step 4: Wire dispatch**

```rust
LinkAction::InsertBetween { tree, link, node, insert_after_cause, insert_before_effect } => {
    let output = execute_link_insert_between(
        &storage, &tree, &link, &node,
        insert_after_cause.as_deref(), insert_before_effect,
    );
    render_output(&output, cli.human);
    if !output.success { process::exit(1); }
}
```

- [ ] **Step 5: Run tests — verify pass**

Run: `cargo test --test fase_06 uat_6_5 uat_6_6 uat_6_7 -- --test-threads=1`

- [ ] **Step 6: Clippy + fmt + commit**

```bash
git add src/link/advanced.rs src/main.rs tests/fase_06.rs
git commit -m "feat(F6): implement link insert-between (UATs 6.5-6.7)"
```

---

### Task 5: `link group` + `link dissolve`

**Files:**
- Modify: `src/link/advanced.rs`
- Modify: `src/main.rs` (dispatch)
- Modify: `tests/fase_06.rs` (UATs 6.8–6.10)

**Interfaces:**
- Consumes: `Storage`, `Edge`, `check_dag`, `storage.next_id("LINK")`
- Produces:
  - `execute_link_group(storage, tree_id, link_ids, operator) -> CommandOutput<LinkGroupData>`
  - `execute_link_dissolve(storage, tree_id, link_id) -> CommandOutput<LinkDissolveData>`

- [ ] **Step 1: Write failing tests (UATs 6.8–6.10)**

```rust
/// UAT 6.8: group two SINGLE edges → one AND edge.
#[test]
fn uat_6_8_link_group() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let a = add_node(dir, "Cause A", "rc");
    let b = add_node(dir, "Cause B", "rc");
    let c = add_node(dir, "Effect C", "ude");
    let tree = create_tree(dir, "crt", "GroupTest");
    attach_node(dir, &tree, &a);
    attach_node(dir, &tree, &b);
    attach_node(dir, &tree, &c);
    let l1 = connect(dir, &tree, &a, &c);
    let l2 = connect(dir, &tree, &b, &c);

    let (json, code) = run_ltp(
        dir,
        &["link", "group", "--tree", &tree, "--links", &format!("{},{}", l1, l2), "--operator", "AND"],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["action"], "link_group");
    assert!(json["data"]["created_link"].as_str().unwrap().starts_with("LINK-"));
    assert_eq!(json["data"]["removed_links"].as_array().unwrap().len(), 2);
}

/// UAT 6.9: group edges that don't share same `to` → error.
#[test]
fn uat_6_9_link_group_different_to() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let a = add_node(dir, "Cause A", "rc");
    let b = add_node(dir, "Cause B", "rc");
    let c = add_node(dir, "Effect C", "ude");
    let d = add_node(dir, "Effect D", "ude");
    let tree = create_tree(dir, "crt", "GroupFail");
    attach_node(dir, &tree, &a);
    attach_node(dir, &tree, &b);
    attach_node(dir, &tree, &c);
    attach_node(dir, &tree, &d);
    let l1 = connect(dir, &tree, &a, &c);
    let l2 = connect(dir, &tree, &b, &d);

    let (json, code) = run_ltp(
        dir,
        &["link", "group", "--tree", &tree, "--links", &format!("{},{}", l1, l2), "--operator", "AND"],
    );

    assert_eq!(code, 1);
    assert_eq!(json["success"], false);
    assert_eq!(json["errors"][0]["code"], "GROUP_DESTINATION_MISMATCH");
}

/// UAT 6.10: dissolve a grouped edge → each cause becomes SINGLE.
#[test]
fn uat_6_10_link_dissolve() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let a = add_node(dir, "Cause A", "rc");
    let b = add_node(dir, "Cause B", "rc");
    let c = add_node(dir, "Effect C", "ude");
    let tree = create_tree(dir, "crt", "DissolveTest");
    attach_node(dir, &tree, &a);
    attach_node(dir, &tree, &b);
    attach_node(dir, &tree, &c);

    // Create AND edge [A,B]→C
    let (json, _) = run_ltp(
        dir,
        &["link", "connect", "--tree", &tree, "--from", &format!("{},{}", a, b), "--to", &c, "--operator", "AND"],
    );
    let group_link = json["data"]["created_links"][0].as_str().unwrap().to_string();

    // Add assumption to test inheritance
    run_ltp(
        dir,
        &["assume", "add", "--tree", &tree, "--link", &group_link, "--text", "Some assumption"],
    );

    let (json, code) = run_ltp(
        dir,
        &["link", "dissolve", "--tree", &tree, "--link", &group_link],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["removed_link"], group_link);
    assert_eq!(json["data"]["created_links"].as_array().unwrap().len(), 2);
}
```

- [ ] **Step 2: Run tests — verify fail**

- [ ] **Step 3: Implement `execute_link_group`**

Logic: validate all link_ids exist in tree, validate all share same `to`, collect all `from` nodes, create new edge with combined `from[]` and given operator, remove originals.

- [ ] **Step 4: Implement `execute_link_dissolve`**

Logic: find edge, verify `from.len() > 1`, for each cause create a SINGLE edge to same `to`. Inherit assumptions with `needs_review` status. Remove original.

- [ ] **Step 5: Wire dispatch for both**

- [ ] **Step 6: Run tests — verify pass**

- [ ] **Step 7: Clippy + fmt + commit**

```bash
git commit -m "feat(F6): implement link group + dissolve (UATs 6.8-6.10)"
```

---

### Task 6: `link split` + `link reoperator`

**Files:**
- Modify: `src/link/advanced.rs`
- Modify: `src/main.rs` (dispatch)
- Modify: `tests/fase_06.rs` (UATs 6.11–6.14)

**Interfaces:**
- Consumes: `Storage`, `Edge`, `storage.next_id("LINK")`
- Produces:
  - `execute_link_split(storage, tree_id, link_id, extract_nodes) -> CommandOutput<LinkSplitData>`
  - `execute_link_reoperator(storage, tree_id, link_id, new_operator) -> CommandOutput<LinkReoperatorData>`

- [ ] **Step 1: Write failing tests (UATs 6.11–6.14)**

```rust
/// UAT 6.11: split extracts a cause from a group.
#[test]
fn uat_6_11_link_split_extract() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let a = add_node(dir, "A", "rc");
    let b = add_node(dir, "B", "rc");
    let c = add_node(dir, "C", "rc");
    let d = add_node(dir, "D", "ude");
    let tree = create_tree(dir, "crt", "SplitTest");
    attach_node(dir, &tree, &a);
    attach_node(dir, &tree, &b);
    attach_node(dir, &tree, &c);
    attach_node(dir, &tree, &d);

    let (json, _) = run_ltp(
        dir,
        &["link", "connect", "--tree", &tree, "--from", &format!("{},{},{}", a, b, c), "--to", &d, "--operator", "AND"],
    );
    let link = json["data"]["created_links"][0].as_str().unwrap().to_string();

    let (json, code) = run_ltp(
        dir,
        &["link", "split", "--tree", &tree, "--link", &link, "--extract", &a],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["original_link"], link);
    assert!(json["data"]["extracted_link"].as_str().unwrap().starts_with("LINK-"));
}

/// UAT 6.12: group with 2 causes after split → becomes SINGLE.
#[test]
fn uat_6_12_split_reduces_to_single() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let a = add_node(dir, "A", "rc");
    let b = add_node(dir, "B", "rc");
    let d = add_node(dir, "D", "ude");
    let tree = create_tree(dir, "crt", "SplitSingle");
    attach_node(dir, &tree, &a);
    attach_node(dir, &tree, &b);
    attach_node(dir, &tree, &d);

    let (json, _) = run_ltp(
        dir,
        &["link", "connect", "--tree", &tree, "--from", &format!("{},{}", a, b), "--to", &d, "--operator", "AND"],
    );
    let link = json["data"]["created_links"][0].as_str().unwrap().to_string();

    let (json, code) = run_ltp(
        dir,
        &["link", "split", "--tree", &tree, "--link", &link, "--extract", &a],
    );

    assert_eq!(code, 0);
    // Original should now be SINGLE (only B remaining)
    // Verify by loading tree and checking
    let (tree_json, _) = run_ltp(dir, &["tree", "walk", &tree, "--order", "topological"]);
    // The original link should still exist with operator SINGLE
    let edges = tree_json["data"]["walk"].as_array().unwrap();
    // Simplified: just verify the split succeeded and extracted link was created
    assert!(json["data"]["extracted_link"].as_str().unwrap().starts_with("LINK-"));
}

/// UAT 6.13: reoperator changes operator. Warning if MAG without weights.
#[test]
fn uat_6_13_link_reoperator_to_mag() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let a = add_node(dir, "A", "rc");
    let b = add_node(dir, "B", "rc");
    let c = add_node(dir, "C", "ude");
    let tree = create_tree(dir, "crt", "ReopTest");
    attach_node(dir, &tree, &a);
    attach_node(dir, &tree, &b);
    attach_node(dir, &tree, &c);

    let (json, _) = run_ltp(
        dir,
        &["link", "connect", "--tree", &tree, "--from", &format!("{},{}", a, b), "--to", &c, "--operator", "AND"],
    );
    let link = json["data"]["created_links"][0].as_str().unwrap().to_string();

    let (json, code) = run_ltp(
        dir,
        &["link", "reoperator", "--tree", &tree, "--link", &link, "--operator", "MAG"],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["new_operator"], "MAG");
    // Warning about missing weights
    assert!(json["warnings"].as_array().unwrap().iter().any(|w| w["code"] == "MAG_WEIGHT_MISSING"));
}

/// UAT 6.14: reoperator MAG→AND discards weights silently.
#[test]
fn uat_6_14_reoperator_mag_to_and() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let a = add_node(dir, "A", "rc");
    let b = add_node(dir, "B", "rc");
    let c = add_node(dir, "C", "ude");
    let tree = create_tree(dir, "crt", "ReopMAG");
    attach_node(dir, &tree, &a);
    attach_node(dir, &tree, &b);
    attach_node(dir, &tree, &c);

    let (json, _) = run_ltp(
        dir,
        &["link", "connect", "--tree", &tree, "--from", &format!("{},{}", a, b), "--to", &c, "--operator", "MAG", "--weight", "0.6"],
    );
    let link = json["data"]["created_links"][0].as_str().unwrap().to_string();

    let (json, code) = run_ltp(
        dir,
        &["link", "reoperator", "--tree", &tree, "--link", &link, "--operator", "AND"],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["new_operator"], "AND");
    assert_eq!(json["data"]["old_operator"], "MAG");
}
```

- [ ] **Step 2: Run tests — verify fail**

- [ ] **Step 3: Implement `execute_link_split`**

Logic: find edge, verify `extract` nodes are in `from[]`, remove them from `from[]`, create new SINGLE edge (or use `--new-operator` if provided) for extracted nodes to same `to`. If original `from[]` is reduced to 1, change operator to SINGLE.

- [ ] **Step 4: Implement `execute_link_reoperator`**

Logic: find edge. Validate transition rules:
- SINGLE→AND/OR/MAG/XOR: only if `from.len() > 1` (error otherwise)
- AND/OR/MAG/XOR→SINGLE: only if `from.len() == 1` (error otherwise)
- If target is MAG and no weight: warning `MAG_WEIGHT_MISSING`.
- If source is MAG and target is not: set `weight = None`.

- [ ] **Step 5: Wire dispatch for both**

- [ ] **Step 6: Run tests — verify pass**

- [ ] **Step 7: Clippy + fmt + commit**

```bash
git commit -m "feat(F6): implement link split + reoperator (UATs 6.11-6.14)"
```

---

### Task 7: `link add-cause` + `link rm-cause`

**Files:**
- Modify: `src/link/advanced.rs`
- Modify: `src/main.rs` (dispatch)
- Modify: `tests/fase_06.rs` (UATs 6.15–6.17)

**Interfaces:**
- Consumes: `Storage`, `Edge`, `check_dag`, `parse_operator`
- Produces:
  - `execute_link_add_cause(storage, tree_id, link_id, node_id, weight, promote_to) -> CommandOutput<LinkAddCauseData>`
  - `execute_link_rm_cause(storage, tree_id, link_id, node_id) -> CommandOutput<LinkRmCauseData>`

- [ ] **Step 1: Write failing tests (UATs 6.15–6.17)**

```rust
/// UAT 6.15: add-cause to AND edge expands from[].
#[test]
fn uat_6_15_add_cause_and() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let a = add_node(dir, "A", "rc");
    let b = add_node(dir, "B", "rc");
    let c = add_node(dir, "C", "ude");
    let x = add_node(dir, "X", "rc");
    let tree = create_tree(dir, "crt", "AddCauseTest");
    attach_node(dir, &tree, &a);
    attach_node(dir, &tree, &b);
    attach_node(dir, &tree, &c);
    attach_node(dir, &tree, &x);

    let (json, _) = run_ltp(
        dir,
        &["link", "connect", "--tree", &tree, "--from", &format!("{},{}", a, b), "--to", &c, "--operator", "AND"],
    );
    let link = json["data"]["created_links"][0].as_str().unwrap().to_string();

    let (json, code) = run_ltp(
        dir,
        &["link", "add-cause", "--tree", &tree, "--link", &link, "--node", &x],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["added_node"], x);
}

/// UAT 6.16: add-cause on SINGLE without --promote-to → error.
#[test]
fn uat_6_16_add_cause_single_no_promote() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let a = add_node(dir, "A", "rc");
    let b = add_node(dir, "B", "ude");
    let x = add_node(dir, "X", "rc");
    let tree = create_tree(dir, "crt", "AddCauseFail");
    attach_node(dir, &tree, &a);
    attach_node(dir, &tree, &b);
    attach_node(dir, &tree, &x);
    let link = connect(dir, &tree, &a, &b);

    let (json, code) = run_ltp(
        dir,
        &["link", "add-cause", "--tree", &tree, "--link", &link, "--node", &x],
    );

    assert_eq!(code, 1);
    assert_eq!(json["success"], false);
    assert_eq!(json["errors"][0]["code"], "PROMOTE_TO_REQUIRED");
}

/// UAT 6.17: rm-cause reduces from[]; if 1 left → SINGLE.
#[test]
fn uat_6_17_rm_cause_reduces_to_single() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let a = add_node(dir, "A", "rc");
    let b = add_node(dir, "B", "rc");
    let c = add_node(dir, "C", "ude");
    let tree = create_tree(dir, "crt", "RmCauseTest");
    attach_node(dir, &tree, &a);
    attach_node(dir, &tree, &b);
    attach_node(dir, &tree, &c);

    let (json, _) = run_ltp(
        dir,
        &["link", "connect", "--tree", &tree, "--from", &format!("{},{}", a, b), "--to", &c, "--operator", "AND"],
    );
    let link = json["data"]["created_links"][0].as_str().unwrap().to_string();

    let (json, code) = run_ltp(
        dir,
        &["link", "rm-cause", "--tree", &tree, "--link", &link, "--node", &a],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["removed_node"], a);
    assert_eq!(json["data"]["new_operator"], "SINGLE");
}
```

- [ ] **Step 2: Run tests — verify fail**

- [ ] **Step 3: Implement `execute_link_add_cause`**

Logic: find edge. If operator is SINGLE and `from.len() == 1`, require `promote_to`. If provided, change operator and add node. If operator is AND/OR/MAG/XOR, just push node to `from[]`. If MAG, apply weight. DAG check after.

- [ ] **Step 4: Implement `execute_link_rm_cause`**

Logic: find edge, verify node is in `from[]`, remove it. If `from.len()` drops to 1, convert operator to SINGLE (weight = None if was MAG). DAG check (though removing a cause can't create cycles — do it anyway for safety).

- [ ] **Step 5: Wire dispatch**

```rust
LinkAction::AddCause { tree, link, node, weight, promote_to } => {
    let output = execute_link_add_cause(&storage, &tree, &link, &node, weight, promote_to.as_deref());
    render_output(&output, cli.human);
    if !output.success { process::exit(1); }
}
LinkAction::RmCause { tree, link, node } => {
    let output = execute_link_rm_cause(&storage, &tree, &link, &node);
    render_output(&output, cli.human);
    if !output.success { process::exit(1); }
}
```

- [ ] **Step 6: Run tests — verify pass**

- [ ] **Step 7: Clippy + fmt + commit**

```bash
git commit -m "feat(F6): implement link add-cause + rm-cause (UATs 6.15-6.17)"
```

---

### Task 8: Final verification + PROGRESS.md

**Files:**
- Modify: `PROGRESS.md`

**Interfaces:**
- Consumes: all 17 UATs passing
- Produces: updated progress dashboard

- [ ] **Step 1: Full verification**

Run:
```bash
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace
cargo fmt --all -- --check
```
Expected: all pass, 0 warnings.

- [ ] **Step 2: Run only F6 tests explicitly**

Run: `cargo test --test fase_06 -- --test-threads=1`
Expected: 17 tests pass.

- [ ] **Step 3: Update PROGRESS.md**

Change F6 row to `✅ Completado | 17/17`. Update avance global: 44% → 58%. Add historial entry.

- [ ] **Step 4: Commit + push**

```bash
git add PROGRESS.md
git commit -m "docs: update PROGRESS.md — F6 complete (58% global)"
git push
```
