# Fase 2b: Nodos Cross-Tree (rm/split/inspect) — Plan de Implementación

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implementar los subcomandos `node rm`, `node inspect`, `node split` y extender `node list` con filtrado por tree — operaciones que cruzan la frontera nodo↔tree↔edge.

**Architecture:** Cada comando nuevo se implementa como función pública en `src/node/commands.rs`, siguiendo el patrón establecido (acquire lock → load → mutate → save → release lock → return `CommandOutput<T>`). Los comandos de lectura (`inspect`, `list --tree`) no necesitan lock. El wiring en `main.rs` conecta los nuevos handlers al CLI ya definido en `NodeAction`.

**Tech Stack:** Rust, clap (derive), serde/serde_json, thiserror, tempfile (tests)

## Global Constraints

- JSON canónico (BTreeMap, indent 2 espacios) en todo output.
- Prohibido `.unwrap()`/`.expect()` en código de producción.
- Documentación `///` en todos los items públicos.
- Verificación obligatoria: `cargo check` + `clippy -D warnings` + `test` + `fmt --check`.
- Tests de integración CLI invocan binario real en tempdir, verifican stdout JSON + estado disco.

---

## Estructura de Archivos

| Archivo | Responsabilidad | Acción |
|---------|----------------|--------|
| `src/node/commands.rs` | Funciones `execute_node_rm`, `execute_node_inspect`, `execute_node_split`; extensión de `execute_node_list` | Modificar |
| `src/main.rs` | Wiring de `NodeAction::Rm`, `NodeAction::Inspect`, `NodeAction::Split`, fix `NodeAction::List` | Modificar |
| `tests/fase_2b.rs` | 7 UATs de integración CLI | Crear |

---

### Task 1: `node rm` — Eliminación cross-tree con limpieza de edges

**Files:**
- Modify: `src/node/commands.rs` (añadir `execute_node_rm` + struct `NodeRmData`)
- Modify: `src/main.rs:860` (reemplazar catch-all `_ =>` por handler de `Rm`)
- Create: `tests/fase_2b.rs`

**Interfaces:**
- Consumes: `Storage::load_node`, `Storage::delete_node`, `Storage::list_tree_ids`, `Storage::load_tree`, `Storage::save_tree`, `Storage::acquire_lock`, `Storage::release_lock`
- Produces: `execute_node_rm(storage: &dyn Storage, ids: &[String], force: bool) -> CommandOutput<NodeRmData>`

- [ ] **Step 1: Crear test file y escribir UAT 2b.1 (rm nodo con edges)**

```rust
// tests/fase_2b.rs
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
            "Failed to parse JSON output.\nstdout: {}\nstderr: {}",
            stdout,
            String::from_utf8_lossy(&output.stderr)
        )
    });

    (json, code)
}

fn setup_workspace(dir: &std::path::Path) {
    let (_, code) = run_ltp(dir, &["init", "--name", "F2bTest"]);
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
    let (_, code) = run_ltp(
        dir,
        &["tree", "attach", "--tree", tree_id, "--node", node_id],
    );
    assert_eq!(code, 0);
}

fn connect_nodes(dir: &std::path::Path, tree_id: &str, from: &str, to: &str) -> String {
    let (json, code) = run_ltp(
        dir,
        &["link", "connect", "--tree", tree_id, "--from", from, "--to", to],
    );
    assert_eq!(code, 0);
    json["data"]["created_links"][0].as_str().unwrap().to_string()
}

/// UAT 2b.1: node rm elimina nodo del pool Y todos sus edges en todos los trees.
#[test]
fn uat_2b_1_node_rm_with_edges() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let node_a = add_node(dir, "Cause A", "ude");
    let node_b = add_node(dir, "Effect B", "ude");
    let node_c = add_node(dir, "Effect C", "rc");
    let tree_id = create_tree(dir, "crt", "TestTree");
    attach_node(dir, &tree_id, &node_a);
    attach_node(dir, &tree_id, &node_b);
    attach_node(dir, &tree_id, &node_c);
    connect_nodes(dir, &tree_id, &node_a, &node_b);
    connect_nodes(dir, &tree_id, &node_b, &node_c);

    // Remove node_b which has edges in both directions
    let (json, code) = run_ltp(dir, &["node", "rm", &node_b]);

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["action"], "node_rm");
    assert_eq!(json["data"]["removed_nodes"][0], node_b);
    assert!(json["data"]["removed_edges_count"].as_u64().unwrap() >= 2);

    // Verify node is gone from pool
    let (json_list, _) = run_ltp(dir, &["node", "list"]);
    let nodes = json_list["data"]["nodes"].as_array().unwrap();
    assert!(!nodes.iter().any(|n| n["id"].as_str().unwrap() == node_b));

    // Verify edges referencing node_b are removed from tree
    let tree_file = dir.join("trees").join(format!("{}.json", tree_id));
    let tree_content: Value =
        serde_json::from_str(&std::fs::read_to_string(&tree_file).unwrap()).unwrap();
    let edges = tree_content["edges"].as_array().unwrap();
    for edge in edges {
        let from_arr = edge["from"].as_array().unwrap();
        assert!(!from_arr.iter().any(|f| f.as_str().unwrap() == node_b));
        assert_ne!(edge["to"].as_str().unwrap(), node_b.as_str());
    }

    // Verify node_b is removed from tree's nodes[]
    let tree_nodes = tree_content["nodes"].as_array().unwrap();
    assert!(!tree_nodes.iter().any(|n| n["ref"].as_str().unwrap() == node_b));
}
```

- [ ] **Step 2: Ejecutar test para verificar que falla**

Run: `cargo test --test fase_2b uat_2b_1 -- --nocapture`
Expected: FAIL (actualmente devuelve NOT_IMPLEMENTED)

- [ ] **Step 3: Implementar `NodeRmData` y `execute_node_rm` en commands.rs**

```rust
/// Data returned by `node rm`.
#[derive(Debug, Serialize)]
pub struct NodeRmData {
    pub removed_nodes: Vec<String>,
    pub removed_edges_count: usize,
    pub affected_trees: Vec<String>,
}

/// Execute `node rm` command.
///
/// Removes nodes from the global pool and cleans up all references
/// in every tree: removes from `nodes[]`, removes edges where the node
/// appears in `from[]` or `to`, and removes feedback edges referencing it.
pub fn execute_node_rm(
    storage: &dyn Storage,
    ids: &[String],
    _force: bool,
) -> CommandOutput<NodeRmData> {
    let ws_name = storage.workspace_name().unwrap_or_default();

    if ids.is_empty() {
        return CommandOutput {
            success: false,
            action: "node_rm".to_string(),
            workspace: ws_name,
            data: NodeRmData {
                removed_nodes: vec![],
                removed_edges_count: 0,
                affected_trees: vec![],
            },
            graph_health: GraphHealth { valid_dag: true, orphan_nodes_count: 0 },
            errors: vec![OutputError::new("INVALID_ARGS", "No node IDs provided")],
            warnings: vec![],
        };
    }

    // Validate all nodes exist before acquiring lock
    for id in ids {
        if storage.load_node(id).is_err() {
            return CommandOutput {
                success: false,
                action: "node_rm".to_string(),
                workspace: ws_name,
                data: NodeRmData {
                    removed_nodes: vec![],
                    removed_edges_count: 0,
                    affected_trees: vec![],
                },
                graph_health: GraphHealth { valid_dag: true, orphan_nodes_count: 0 },
                errors: vec![OutputError::new(
                    "NODE_NOT_FOUND",
                    format!("Node '{}' not found in pool", id),
                )],
                warnings: vec![],
            };
        }
    }

    let lock_outcome = match storage.acquire_lock("node rm") {
        Ok(o) => o,
        Err(e) => {
            return CommandOutput {
                success: false,
                action: "node_rm".to_string(),
                workspace: ws_name,
                data: NodeRmData {
                    removed_nodes: vec![],
                    removed_edges_count: 0,
                    affected_trees: vec![],
                },
                graph_health: GraphHealth { valid_dag: true, orphan_nodes_count: 0 },
                errors: vec![OutputError::new("LOCK_ERROR", e.to_string())],
                warnings: vec![],
            };
        }
    };

    let id_set: std::collections::HashSet<&str> = ids.iter().map(|s| s.as_str()).collect();

    // Scan all trees and remove references
    let tree_ids = match storage.list_tree_ids() {
        Ok(t) => t,
        Err(e) => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: "node_rm".to_string(),
                workspace: ws_name,
                data: NodeRmData {
                    removed_nodes: vec![],
                    removed_edges_count: 0,
                    affected_trees: vec![],
                },
                graph_health: GraphHealth { valid_dag: true, orphan_nodes_count: 0 },
                errors: vec![OutputError::new("IO_ERROR", e.to_string())],
                warnings: vec![],
            };
        }
    };

    let mut total_removed_edges = 0usize;
    let mut affected_trees = Vec::new();

    for tree_id in &tree_ids {
        let mut tree = match storage.load_tree(tree_id) {
            Ok(t) => t,
            Err(_) => continue,
        };

        let before_nodes = tree.nodes.len();
        let before_edges = tree.edges.len();
        let before_fb = tree.feedback_edges.len();

        // Remove from nodes[]
        tree.nodes.retain(|nr| !id_set.contains(nr.node_ref.as_str()));

        // Remove edges where node is in from[] or is the to target
        tree.edges.retain(|edge| {
            let to_removed = id_set.contains(edge.to.as_str());
            let from_has_removed = edge.from.iter().any(|f| id_set.contains(f.as_str()));
            !to_removed && !from_has_removed
        });

        // Remove feedback edges referencing removed nodes
        tree.feedback_edges.retain(|fb| {
            !id_set.contains(fb.from.as_str()) && !id_set.contains(fb.to.as_str())
        });

        let edges_removed = (before_edges - tree.edges.len()) + (before_fb - tree.feedback_edges.len());
        let tree_changed = tree.nodes.len() != before_nodes
            || tree.edges.len() != before_edges
            || tree.feedback_edges.len() != before_fb;

        if tree_changed {
            if let Err(e) = storage.save_tree(&tree) {
                let _ = storage.release_lock();
                return CommandOutput {
                    success: false,
                    action: "node_rm".to_string(),
                    workspace: ws_name,
                    data: NodeRmData {
                        removed_nodes: vec![],
                        removed_edges_count: 0,
                        affected_trees: vec![],
                    },
                    graph_health: GraphHealth { valid_dag: true, orphan_nodes_count: 0 },
                    errors: vec![OutputError::new("IO_ERROR", e.to_string())],
                    warnings: vec![],
                };
            }
            total_removed_edges += edges_removed;
            affected_trees.push(tree_id.clone());
        }
    }

    // Delete nodes from pool
    let mut removed_nodes = Vec::new();
    for id in ids {
        if let Err(e) = storage.delete_node(id) {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: "node_rm".to_string(),
                workspace: ws_name,
                data: NodeRmData {
                    removed_nodes,
                    removed_edges_count: total_removed_edges,
                    affected_trees,
                },
                graph_health: GraphHealth { valid_dag: true, orphan_nodes_count: 0 },
                errors: vec![OutputError::new("IO_ERROR", e.to_string())],
                warnings: vec![],
            };
        }
        removed_nodes.push(id.clone());
    }

    let _ = storage.release_lock();

    let mut warnings = vec![];
    if let Some(w) = stale_lock_warning(&lock_outcome) {
        warnings.push(w);
    }

    CommandOutput {
        success: true,
        action: "node_rm".to_string(),
        workspace: ws_name,
        data: NodeRmData {
            removed_nodes,
            removed_edges_count: total_removed_edges,
            affected_trees,
        },
        graph_health: GraphHealth { valid_dag: true, orphan_nodes_count: 0 },
        errors: vec![],
        warnings,
    }
}
```

- [ ] **Step 4: Wire `NodeAction::Rm` en main.rs**

Reemplazar el bloque `_ =>` (línea ~860) por handlers específicos. Para `Rm`:

```rust
NodeAction::Rm { ids, force } => {
    let output = execute_node_rm(&storage, &ids, force);
    render_output(&output, cli.human);
    if !output.success {
        process::exit(1);
    }
}
```

Añadir al import en main.rs:
```rust
use ltp_engine::node::commands::{
    execute_node_add, execute_node_edit, execute_node_list, execute_node_search,
    execute_node_rm,
};
```

- [ ] **Step 5: Ejecutar test UAT 2b.1**

Run: `cargo test --test fase_2b uat_2b_1 -- --nocapture`
Expected: PASS

- [ ] **Step 6: Escribir UAT 2b.2 (rm nodo inexistente)**

```rust
/// UAT 2b.2: node rm de nodo inexistente retorna NODE_NOT_FOUND.
#[test]
fn uat_2b_2_node_rm_not_found() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let (json, code) = run_ltp(dir, &["node", "rm", "UDE-999"]);

    assert_eq!(code, 1);
    assert_eq!(json["success"], false);
    assert_eq!(json["errors"][0]["code"], "NODE_NOT_FOUND");
}
```

- [ ] **Step 7: Ejecutar test UAT 2b.2**

Run: `cargo test --test fase_2b uat_2b_2 -- --nocapture`
Expected: PASS (ya cubierto por la validación previa en el impl)

- [ ] **Step 8: Escribir UAT 2b.3 (rm batch)**

```rust
/// UAT 2b.3: node rm batch — elimina múltiples nodos con todos sus edges.
#[test]
fn uat_2b_3_node_rm_batch() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let node_a = add_node(dir, "Node A", "ude");
    let node_b = add_node(dir, "Node B", "rc");
    let node_c = add_node(dir, "Node C", "ude");
    let tree_id = create_tree(dir, "crt", "BatchTree");
    attach_node(dir, &tree_id, &node_a);
    attach_node(dir, &tree_id, &node_b);
    attach_node(dir, &tree_id, &node_c);
    connect_nodes(dir, &tree_id, &node_a, &node_c);
    connect_nodes(dir, &tree_id, &node_b, &node_c);

    // Remove A and B in batch
    let ids_arg = format!("{},{}", node_a, node_b);
    let (json, code) = run_ltp(dir, &["node", "rm", &ids_arg]);

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    let removed = json["data"]["removed_nodes"].as_array().unwrap();
    assert_eq!(removed.len(), 2);
    assert!(json["data"]["removed_edges_count"].as_u64().unwrap() >= 2);

    // Verify pool only has node_c
    let (json_list, _) = run_ltp(dir, &["node", "list"]);
    let nodes = json_list["data"]["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0]["id"].as_str().unwrap(), node_c);
}
```

- [ ] **Step 9: Ejecutar test UAT 2b.3**

Run: `cargo test --test fase_2b uat_2b_3 -- --nocapture`
Expected: PASS

- [ ] **Step 10: Verificación parcial**

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: Sin errores ni warnings

---

### Task 2: `node inspect` — Inspección cross-tree

**Files:**
- Modify: `src/node/commands.rs` (añadir `execute_node_inspect` + structs)
- Modify: `src/main.rs` (añadir handler para `NodeAction::Inspect`)

**Interfaces:**
- Consumes: `Storage::load_node`, `Storage::list_tree_ids`, `Storage::load_tree`
- Produces: `execute_node_inspect(storage: &dyn Storage, id: &str) -> CommandOutput<NodeInspectData>`

- [ ] **Step 1: Escribir UAT 2b.4 (node inspect)**

```rust
/// UAT 2b.4: node inspect muestra trees, rol y conexiones.
#[test]
fn uat_2b_4_node_inspect() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let node_a = add_node(dir, "Cause A", "ude");
    let node_b = add_node(dir, "Effect B", "ude");
    let node_c = add_node(dir, "Effect C", "rc");
    let tree_id = create_tree(dir, "crt", "InspectTree");
    attach_node(dir, &tree_id, &node_a);
    attach_node(dir, &tree_id, &node_b);
    attach_node(dir, &tree_id, &node_c);

    // Attach node_a with a role in a second tree
    let tree_id_2 = create_tree(dir, "ec", "SecondTree");
    let (_, code) = run_ltp(
        dir,
        &["tree", "attach", "--tree", &tree_id_2, "--node", &node_a, "--role", "objective"],
    );
    assert_eq!(code, 0);

    // Connect A->B and B->C in first tree
    connect_nodes(dir, &tree_id, &node_a, &node_b);
    connect_nodes(dir, &tree_id, &node_b, &node_c);

    let (json, code) = run_ltp(dir, &["node", "inspect", &node_b]);

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["action"], "node_inspect");
    assert_eq!(json["data"]["id"], node_b.as_str());
    assert_eq!(json["data"]["label"], "Effect B");

    // Verify trees participation
    let trees = json["data"]["trees"].as_array().unwrap();
    assert_eq!(trees.len(), 1);
    assert_eq!(trees[0]["tree_id"].as_str().unwrap(), tree_id);

    // Verify connections (edges inbound and outbound)
    let connections = &trees[0]["connections"];
    let inbound = connections["inbound"].as_array().unwrap();
    let outbound = connections["outbound"].as_array().unwrap();
    assert_eq!(inbound.len(), 1);
    assert_eq!(outbound.len(), 1);
    assert!(inbound[0]["from"].as_array().unwrap().iter().any(|f| f.as_str().unwrap() == node_a));
    assert_eq!(outbound[0]["to"].as_str().unwrap(), node_c.as_str());

    // Inspect node_a which is in 2 trees
    let (json_a, _) = run_ltp(dir, &["node", "inspect", &node_a]);
    let trees_a = json_a["data"]["trees"].as_array().unwrap();
    assert_eq!(trees_a.len(), 2);

    // Verify the second tree has role "objective"
    let second = trees_a.iter().find(|t| t["tree_id"].as_str().unwrap() == tree_id_2).unwrap();
    assert_eq!(second["role"].as_str().unwrap(), "objective");
}
```

- [ ] **Step 2: Ejecutar test para verificar que falla**

Run: `cargo test --test fase_2b uat_2b_4 -- --nocapture`
Expected: FAIL

- [ ] **Step 3: Implementar structs y `execute_node_inspect`**

```rust
/// Summary of a node's participation in one tree.
#[derive(Debug, Serialize)]
pub struct NodeTreeParticipation {
    pub tree_id: String,
    pub tree_name: String,
    pub role: Option<String>,
    pub connections: NodeConnections,
}

/// Inbound and outbound connections of a node within a tree.
#[derive(Debug, Serialize)]
pub struct NodeConnections {
    pub inbound: Vec<EdgeSummary>,
    pub outbound: Vec<EdgeSummary>,
}

/// Compact representation of an edge for inspect output.
#[derive(Debug, Serialize)]
pub struct EdgeSummary {
    pub id: String,
    pub from: Vec<String>,
    pub to: String,
    pub operator: crate::link::Operator,
}

/// Data returned by `node inspect`.
#[derive(Debug, Serialize)]
pub struct NodeInspectData {
    pub id: String,
    pub node_type: NodeType,
    pub label: String,
    pub tags: Vec<String>,
    pub observable: bool,
    pub status: NodeStatus,
    pub trees: Vec<NodeTreeParticipation>,
}

/// Execute `node inspect` command.
///
/// Shows which trees a node participates in, its role in each,
/// and all inbound/outbound edges per tree.
pub fn execute_node_inspect(storage: &dyn Storage, id: &str) -> CommandOutput<NodeInspectData> {
    let ws_name = storage.workspace_name().unwrap_or_default();

    let node = match storage.load_node(id) {
        Ok(n) => n,
        Err(_) => {
            return CommandOutput {
                success: false,
                action: "node_inspect".to_string(),
                workspace: ws_name,
                data: NodeInspectData {
                    id: id.to_string(),
                    node_type: NodeType::Ude,
                    label: String::new(),
                    tags: vec![],
                    observable: true,
                    status: NodeStatus::Active,
                    trees: vec![],
                },
                graph_health: GraphHealth { valid_dag: true, orphan_nodes_count: 0 },
                errors: vec![OutputError::new("NODE_NOT_FOUND", format!("Node '{}' not found", id))],
                warnings: vec![],
            };
        }
    };

    let tree_ids = storage.list_tree_ids().unwrap_or_default();
    let mut participations = Vec::new();

    for tree_id in &tree_ids {
        let tree = match storage.load_tree(tree_id) {
            Ok(t) => t,
            Err(_) => continue,
        };

        let node_ref = tree.nodes.iter().find(|nr| nr.node_ref == id);
        if let Some(nr) = node_ref {
            let inbound: Vec<EdgeSummary> = tree
                .edges
                .iter()
                .filter(|e| e.to == id)
                .map(|e| EdgeSummary {
                    id: e.id.clone(),
                    from: e.from.clone(),
                    to: e.to.clone(),
                    operator: e.operator,
                })
                .collect();

            let outbound: Vec<EdgeSummary> = tree
                .edges
                .iter()
                .filter(|e| e.from.contains(&id.to_string()))
                .map(|e| EdgeSummary {
                    id: e.id.clone(),
                    from: e.from.clone(),
                    to: e.to.clone(),
                    operator: e.operator,
                })
                .collect();

            participations.push(NodeTreeParticipation {
                tree_id: tree.id.clone(),
                tree_name: tree.name.clone(),
                role: nr.role.clone(),
                connections: NodeConnections { inbound, outbound },
            });
        }
    }

    CommandOutput::ok(
        "node_inspect",
        &ws_name,
        NodeInspectData {
            id: node.id,
            node_type: node.node_type,
            label: node.label,
            tags: node.tags,
            observable: node.observable,
            status: node.metadata.status,
            trees: participations,
        },
    )
}
```

- [ ] **Step 4: Wire `NodeAction::Inspect` en main.rs**

```rust
NodeAction::Inspect { id } => {
    let output = execute_node_inspect(&storage, &id);
    render_output(&output, cli.human);
    if !output.success {
        process::exit(1);
    }
}
```

Añadir al import: `execute_node_inspect`

- [ ] **Step 5: Ejecutar test UAT 2b.4**

Run: `cargo test --test fase_2b uat_2b_4 -- --nocapture`
Expected: PASS

- [ ] **Step 6: Verificación parcial**

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: Sin errores

---

### Task 3: `node split` — División de nodo con herencia de edges

**Files:**
- Modify: `src/node/commands.rs` (añadir `execute_node_split` + struct `NodeSplitData`)
- Modify: `src/main.rs` (añadir handler para `NodeAction::Split`)

**Interfaces:**
- Consumes: `Storage::load_node`, `Storage::save_node`, `Storage::delete_node`, `Storage::load_tree`, `Storage::save_tree`, `Storage::next_id`, `Storage::acquire_lock`, `Storage::release_lock`
- Produces: `execute_node_split(storage: &dyn Storage, id: &str, labels: &[String], tree_id: &str) -> CommandOutput<NodeSplitData>`

- [ ] **Step 1: Escribir UAT 2b.5 (node split)**

```rust
/// UAT 2b.5: node split divide un nodo, hereda edges entrantes al primero y salientes al segundo.
#[test]
fn uat_2b_5_node_split() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let node_a = add_node(dir, "Root Cause", "rc");
    let node_int = add_node(dir, "Mixed idea A and B", "int");
    let node_c = add_node(dir, "Effect", "ude");
    let tree_id = create_tree(dir, "crt", "SplitTree");
    attach_node(dir, &tree_id, &node_a);
    attach_node(dir, &tree_id, &node_int);
    attach_node(dir, &tree_id, &node_c);
    connect_nodes(dir, &tree_id, &node_a, &node_int);
    connect_nodes(dir, &tree_id, &node_int, &node_c);

    let (json, code) = run_ltp(
        dir,
        &[
            "node", "split", &node_int,
            "--into", "Idea A", "Idea B",
            "--tree", &tree_id,
        ],
    );

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["action"], "node_split");

    let new_ids = json["data"]["new_nodes"].as_array().unwrap();
    assert_eq!(new_ids.len(), 2);
    let first_id = new_ids[0]["id"].as_str().unwrap();
    let second_id = new_ids[1]["id"].as_str().unwrap();

    // Verify original node removed from pool
    let (json_list, _) = run_ltp(dir, &["node", "list"]);
    let nodes = json_list["data"]["nodes"].as_array().unwrap();
    assert!(!nodes.iter().any(|n| n["id"].as_str().unwrap() == node_int));
    assert!(nodes.iter().any(|n| n["id"].as_str().unwrap() == first_id));
    assert!(nodes.iter().any(|n| n["id"].as_str().unwrap() == second_id));

    // Verify edges: A->first_id and second_id->C
    let tree_file = dir.join("trees").join(format!("{}.json", tree_id));
    let tree_content: Value =
        serde_json::from_str(&std::fs::read_to_string(&tree_file).unwrap()).unwrap();
    let edges = tree_content["edges"].as_array().unwrap();

    let has_a_to_first = edges.iter().any(|e| {
        e["from"].as_array().unwrap().iter().any(|f| f.as_str().unwrap() == node_a)
            && e["to"].as_str().unwrap() == first_id
    });
    let has_second_to_c = edges.iter().any(|e| {
        e["from"].as_array().unwrap().iter().any(|f| f.as_str().unwrap() == second_id)
            && e["to"].as_str().unwrap() == node_c
    });

    assert!(has_a_to_first, "Expected edge from A to first new node");
    assert!(has_second_to_c, "Expected edge from second new node to C");

    // Verify no edges reference original node
    for edge in edges {
        let from_arr = edge["from"].as_array().unwrap();
        assert!(!from_arr.iter().any(|f| f.as_str().unwrap() == node_int));
        assert_ne!(edge["to"].as_str().unwrap(), node_int.as_str());
    }
}
```

- [ ] **Step 2: Ejecutar test para verificar que falla**

Run: `cargo test --test fase_2b uat_2b_5 -- --nocapture`
Expected: FAIL

- [ ] **Step 3: Implementar `NodeSplitData` y `execute_node_split`**

```rust
/// Summary of a newly created node after split.
#[derive(Debug, Serialize)]
pub struct NewNodeSummary {
    pub id: String,
    pub label: String,
    pub node_type: NodeType,
}

/// Data returned by `node split`.
#[derive(Debug, Serialize)]
pub struct NodeSplitData {
    pub original_id: String,
    pub new_nodes: Vec<NewNodeSummary>,
    pub tree_id: String,
}

/// Execute `node split` command.
///
/// Splits a node into two new nodes within a specific tree.
/// Incoming edges of the original are redirected to the first new node.
/// Outgoing edges of the original are redirected from the second new node.
/// The original node is removed from pool and tree.
pub fn execute_node_split(
    storage: &dyn Storage,
    id: &str,
    labels: &[String],
    tree_id: &str,
) -> CommandOutput<NodeSplitData> {
    let ws_name = storage.workspace_name().unwrap_or_default();

    if labels.len() != 2 {
        return CommandOutput {
            success: false,
            action: "node_split".to_string(),
            workspace: ws_name,
            data: NodeSplitData {
                original_id: id.to_string(),
                new_nodes: vec![],
                tree_id: tree_id.to_string(),
            },
            graph_health: GraphHealth { valid_dag: true, orphan_nodes_count: 0 },
            errors: vec![OutputError::new("INVALID_ARGS", "Split requires exactly 2 labels")],
            warnings: vec![],
        };
    }

    let original = match storage.load_node(id) {
        Ok(n) => n,
        Err(_) => {
            return CommandOutput {
                success: false,
                action: "node_split".to_string(),
                workspace: ws_name,
                data: NodeSplitData {
                    original_id: id.to_string(),
                    new_nodes: vec![],
                    tree_id: tree_id.to_string(),
                },
                graph_health: GraphHealth { valid_dag: true, orphan_nodes_count: 0 },
                errors: vec![OutputError::new("NODE_NOT_FOUND", format!("Node '{}' not found", id))],
                warnings: vec![],
            };
        }
    };

    let lock_outcome = match storage.acquire_lock("node split") {
        Ok(o) => o,
        Err(e) => {
            return CommandOutput {
                success: false,
                action: "node_split".to_string(),
                workspace: ws_name,
                data: NodeSplitData {
                    original_id: id.to_string(),
                    new_nodes: vec![],
                    tree_id: tree_id.to_string(),
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
                action: "node_split".to_string(),
                workspace: ws_name,
                data: NodeSplitData {
                    original_id: id.to_string(),
                    new_nodes: vec![],
                    tree_id: tree_id.to_string(),
                },
                graph_health: GraphHealth { valid_dag: true, orphan_nodes_count: 0 },
                errors: vec![OutputError::new("TREE_NOT_FOUND", e.to_string())],
                warnings: vec![],
            };
        }
    };

    // Verify node is in the tree
    if !tree.nodes.iter().any(|nr| nr.node_ref == id) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: "node_split".to_string(),
            workspace: ws_name,
            data: NodeSplitData {
                original_id: id.to_string(),
                new_nodes: vec![],
                tree_id: tree_id.to_string(),
            },
            graph_health: GraphHealth { valid_dag: true, orphan_nodes_count: 0 },
            errors: vec![OutputError::new(
                "NODE_NOT_IN_TREE",
                format!("Node '{}' is not attached to tree '{}'", id, tree_id),
            )],
            warnings: vec![],
        };
    }

    // Generate IDs for new nodes (same type as original)
    let type_prefix = format!("{:?}", original.node_type).to_uppercase();
    let id_first = match storage.next_id(&type_prefix) {
        Ok(id) => id,
        Err(e) => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: "node_split".to_string(),
                workspace: ws_name,
                data: NodeSplitData {
                    original_id: id.to_string(),
                    new_nodes: vec![],
                    tree_id: tree_id.to_string(),
                },
                graph_health: GraphHealth { valid_dag: true, orphan_nodes_count: 0 },
                errors: vec![OutputError::new("ID_GENERATION_ERROR", e.to_string())],
                warnings: vec![],
            };
        }
    };
    let id_second = match storage.next_id(&type_prefix) {
        Ok(id) => id,
        Err(e) => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: "node_split".to_string(),
                workspace: ws_name,
                data: NodeSplitData {
                    original_id: id.to_string(),
                    new_nodes: vec![],
                    tree_id: tree_id.to_string(),
                },
                graph_health: GraphHealth { valid_dag: true, orphan_nodes_count: 0 },
                errors: vec![OutputError::new("ID_GENERATION_ERROR", e.to_string())],
                warnings: vec![],
            };
        }
    };

    // Create two new nodes inheriting type, tags, observable from original
    let node_first = Node {
        id: id_first.clone(),
        node_type: original.node_type,
        label: labels[0].clone(),
        tags: original.tags.clone(),
        observable: original.observable,
        metadata: NodeMetadata {
            status: NodeStatus::Active,
            extra: Default::default(),
        },
    };
    let node_second = Node {
        id: id_second.clone(),
        node_type: original.node_type,
        label: labels[1].clone(),
        tags: original.tags.clone(),
        observable: original.observable,
        metadata: NodeMetadata {
            status: NodeStatus::Active,
            extra: Default::default(),
        },
    };

    if let Err(e) = storage.save_node(&node_first) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: "node_split".to_string(),
            workspace: ws_name,
            data: NodeSplitData {
                original_id: id.to_string(),
                new_nodes: vec![],
                tree_id: tree_id.to_string(),
            },
            graph_health: GraphHealth { valid_dag: true, orphan_nodes_count: 0 },
            errors: vec![OutputError::new("IO_ERROR", e.to_string())],
            warnings: vec![],
        };
    }
    if let Err(e) = storage.save_node(&node_second) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: "node_split".to_string(),
            workspace: ws_name,
            data: NodeSplitData {
                original_id: id.to_string(),
                new_nodes: vec![],
                tree_id: tree_id.to_string(),
            },
            graph_health: GraphHealth { valid_dag: true, orphan_nodes_count: 0 },
            errors: vec![OutputError::new("IO_ERROR", e.to_string())],
            warnings: vec![],
        };
    }

    // Update tree: replace original node ref with two new refs
    tree.nodes.retain(|nr| nr.node_ref != id);
    tree.nodes.push(crate::tree::NodeRef { node_ref: id_first.clone(), role: None });
    tree.nodes.push(crate::tree::NodeRef { node_ref: id_second.clone(), role: None });

    // Redirect edges: inbound (to == original) -> to = first
    for edge in &mut tree.edges {
        if edge.to == id {
            edge.to = id_first.clone();
        }
    }
    // Redirect edges: outbound (from contains original) -> replace with second
    for edge in &mut tree.edges {
        for from_ref in &mut edge.from {
            if from_ref == id {
                *from_ref = id_second.clone();
            }
        }
    }

    if let Err(e) = storage.save_tree(&tree) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: "node_split".to_string(),
            workspace: ws_name,
            data: NodeSplitData {
                original_id: id.to_string(),
                new_nodes: vec![],
                tree_id: tree_id.to_string(),
            },
            graph_health: GraphHealth { valid_dag: true, orphan_nodes_count: 0 },
            errors: vec![OutputError::new("IO_ERROR", e.to_string())],
            warnings: vec![],
        };
    }

    // Delete original node from pool
    if let Err(e) = storage.delete_node(id) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: "node_split".to_string(),
            workspace: ws_name,
            data: NodeSplitData {
                original_id: id.to_string(),
                new_nodes: vec![],
                tree_id: tree_id.to_string(),
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
        action: "node_split".to_string(),
        workspace: ws_name,
        data: NodeSplitData {
            original_id: id.to_string(),
            new_nodes: vec![
                NewNodeSummary { id: id_first, label: labels[0].clone(), node_type: original.node_type },
                NewNodeSummary { id: id_second, label: labels[1].clone(), node_type: original.node_type },
            ],
            tree_id: tree_id.to_string(),
        },
        graph_health: GraphHealth { valid_dag: true, orphan_nodes_count: 0 },
        errors: vec![],
        warnings,
    }
}
```

- [ ] **Step 4: Wire `NodeAction::Split` en main.rs**

```rust
NodeAction::Split { id, into, tree } => {
    let output = execute_node_split(&storage, &id, &into, &tree);
    render_output(&output, cli.human);
    if !output.success {
        process::exit(1);
    }
}
```

Añadir al import: `execute_node_split`

- [ ] **Step 5: Ejecutar test UAT 2b.5**

Run: `cargo test --test fase_2b uat_2b_5 -- --nocapture`
Expected: PASS

- [ ] **Step 6: Verificación parcial**

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: Sin errores

---

### Task 4: `node list --tree` y `--status` — Filtrado por tree

**Files:**
- Modify: `src/node/commands.rs` (extender `execute_node_list` para aceptar `tree_filter`)
- Modify: `src/main.rs:846-858` (pasar `tree` al handler en vez de ignorarlo)

**Interfaces:**
- Consumes: `Storage::load_tree`, parámetros existentes de `execute_node_list`
- Produces: `execute_node_list(storage: &dyn Storage, tree_filter: Option<&str>, type_filter: Option<&[String]>, status_filter: Option<&[String]>) -> CommandOutput<NodeListData>` (firma extendida)

- [ ] **Step 1: Escribir UATs 2b.6 y 2b.7**

```rust
/// UAT 2b.6: node list --tree --type filtra por tree y tipo.
#[test]
fn uat_2b_6_node_list_tree_filter() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let node_a = add_node(dir, "UDE in tree", "ude");
    let node_b = add_node(dir, "RC in tree", "rc");
    let node_c = add_node(dir, "UDE not in tree", "ude");
    let tree_id = create_tree(dir, "crt", "FilterTree");
    attach_node(dir, &tree_id, &node_a);
    attach_node(dir, &tree_id, &node_b);
    // node_c NOT attached

    // List with --tree filter
    let (json, code) = run_ltp(dir, &["node", "list", "--tree", &tree_id]);
    assert_eq!(code, 0);
    let nodes = json["data"]["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 2);
    assert!(nodes.iter().any(|n| n["id"].as_str().unwrap() == node_a));
    assert!(nodes.iter().any(|n| n["id"].as_str().unwrap() == node_b));
    assert!(!nodes.iter().any(|n| n["id"].as_str().unwrap() == node_c));

    // List with --tree AND --type UDE
    let (json2, code2) = run_ltp(dir, &["node", "list", "--tree", &tree_id, "--type", "UDE"]);
    assert_eq!(code2, 0);
    let nodes2 = json2["data"]["nodes"].as_array().unwrap();
    assert_eq!(nodes2.len(), 1);
    assert_eq!(nodes2[0]["id"].as_str().unwrap(), node_a);
}

/// UAT 2b.7: node list --tree --status filtra por status.
#[test]
fn uat_2b_7_node_list_status_filter() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_workspace(dir);

    let node_a = add_node(dir, "Active node", "ude");
    let node_b = add_node(dir, "Another node", "ude");
    let tree_id = create_tree(dir, "crt", "StatusTree");
    attach_node(dir, &tree_id, &node_a);
    attach_node(dir, &tree_id, &node_b);

    // Both are active by default
    let (json, code) = run_ltp(
        dir,
        &["node", "list", "--tree", &tree_id, "--status", "active"],
    );
    assert_eq!(code, 0);
    let nodes = json["data"]["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 2);

    // Filter by draft (should be empty)
    let (json2, code2) = run_ltp(
        dir,
        &["node", "list", "--tree", &tree_id, "--status", "draft"],
    );
    assert_eq!(code2, 0);
    let nodes2 = json2["data"]["nodes"].as_array().unwrap();
    assert_eq!(nodes2.len(), 0);
}
```

- [ ] **Step 2: Ejecutar tests para verificar que fallan**

Run: `cargo test --test fase_2b uat_2b_6 uat_2b_7 -- --nocapture`
Expected: FAIL (list ignora `--tree`)

- [ ] **Step 3: Extender `execute_node_list` para filtrar por tree**

Cambiar la firma de `execute_node_list`:

```rust
/// Execute `node list` command.
///
/// Lists nodes from the pool, optionally filtered by tree membership,
/// node type, and/or status.
pub fn execute_node_list(
    storage: &dyn Storage,
    tree_filter: Option<&str>,
    type_filter: Option<&[String]>,
    status_filter: Option<&[String]>,
) -> CommandOutput<NodeListData> {
    let ws_name = storage.workspace_name().unwrap_or_default();

    // Determine which node IDs to consider
    let candidate_ids: Vec<String> = if let Some(tid) = tree_filter {
        match storage.load_tree(tid) {
            Ok(tree) => tree.nodes.iter().map(|nr| nr.node_ref.clone()).collect(),
            Err(e) => {
                return CommandOutput {
                    success: false,
                    action: "node_list".to_string(),
                    workspace: ws_name,
                    data: NodeListData { nodes: vec![], count: 0 },
                    graph_health: GraphHealth { valid_dag: true, orphan_nodes_count: 0 },
                    errors: vec![OutputError::new("TREE_NOT_FOUND", e.to_string())],
                    warnings: vec![],
                };
            }
        }
    } else {
        match storage.list_node_ids() {
            Ok(ids) => ids,
            Err(e) => {
                return CommandOutput {
                    success: false,
                    action: "node_list".to_string(),
                    workspace: ws_name,
                    data: NodeListData { nodes: vec![], count: 0 },
                    graph_health: GraphHealth { valid_dag: true, orphan_nodes_count: 0 },
                    errors: vec![OutputError::new("IO_ERROR", e.to_string())],
                    warnings: vec![],
                };
            }
        }
    };

    // (rest of filtering logic unchanged — type_filters + status_filters applied on candidate_ids)
    // ...
}
```

- [ ] **Step 4: Actualizar call-site en main.rs**

Cambiar la línea que actualmente ignora `tree` (`tree: _tree`):

```rust
NodeAction::List { tree, r#type, status } => {
    let output = execute_node_list(&storage, tree.as_deref(), r#type.as_deref(), status.as_deref());
    render_output(&output, cli.human);
    if !output.success {
        process::exit(1);
    }
}
```

- [ ] **Step 5: Ejecutar tests UATs 2b.6 y 2b.7**

Run: `cargo test --test fase_2b uat_2b_6 uat_2b_7 -- --nocapture`
Expected: PASS

- [ ] **Step 6: Ejecutar tests previos para verificar no-regresión**

Run: `cargo test --workspace`
Expected: PASS (tests de F1, F2a, F3, F4 siguen pasando)

---

### Task 5: Verificación Final y Limpieza

**Files:**
- All modified files from tasks 1-4

- [ ] **Step 1: Ejecutar suite completa de verificación**

```bash
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace
cargo fmt --all -- --check
```

Expected: Todo PASS, 0 warnings

- [ ] **Step 2: Verificar que los 7 UATs de F2b pasan**

Run: `cargo test --test fase_2b -- --nocapture`
Expected: 7 tests PASS (uat_2b_1 through uat_2b_7)

- [ ] **Step 3: Commit**

```bash
git add src/node/commands.rs src/main.rs tests/fase_2b.rs
git commit -m "feat(F2b): implement node rm/inspect/split and list --tree filter

Cross-tree operations: node rm cleans edges in all trees,
inspect shows participation and connections, split divides
with edge inheritance, list supports --tree/--status filters.

7 UATs passing (2b.1-2b.7)."
```

---

## Notas de Implementación

1. **`NodeType` a string para `next_id`**: el trait `Storage::next_id` recibe `&str` (ej. `"UDE"`, `"INT"`). En `node split`, se necesita convertir `NodeType` enum a su representación string. Usar el serialization rename: `format!("{:?}", node_type).to_uppercase()` no funciona directamente porque el enum usa variantes como `Ude` que serializa a `"UDE"` via serde. Mejor usar un helper `node_type_prefix(nt: NodeType) -> &'static str` que mapee explícitamente.

2. **Feedback edges en `node rm`**: además de `tree.edges`, se deben limpiar `tree.feedback_edges` que referencien al nodo eliminado (ya cubierto en el impl).

3. **Signature change en `execute_node_list`**: al añadir `tree_filter: Option<&str>` como primer parámetro nuevo, los tests existentes en `fase_02a.rs` que llaman a esta función necesitan actualización. Revisar el call-site en `fase_02a.rs` — como son tests CLI (invocan binario), NO se ven afectados. Solo el call-site en `main.rs` necesita cambio.
