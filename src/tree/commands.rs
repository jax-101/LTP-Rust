use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

use serde::Serialize;

use crate::errors::{LtpError, Result};
use crate::link::Edge;
use crate::output::{CommandOutput, GraphHealth, OutputError, OutputWarning};
use crate::storage::{LockOutcome, Storage};
use crate::tree::types::{NodeRef, Tree, TreeLogic, TreeType};

// --- Helpers ---

/// Convert a name to a URL-friendly slug.
fn slugify(name: &str) -> String {
    let lower = name.to_lowercase();
    let slug: String = lower
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();

    slug.split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Determine the logic type for a given tree type.
fn logic_for_type(tree_type: TreeType) -> TreeLogic {
    match tree_type {
        TreeType::Gt | TreeType::Crt | TreeType::Frt | TreeType::Tt => TreeLogic::Sufficiency,
        TreeType::Ec | TreeType::Prt => TreeLogic::Necessity,
    }
}

/// Parse a tree type string (case-insensitive).
fn parse_tree_type(s: &str) -> Result<TreeType> {
    match s.to_lowercase().as_str() {
        "gt" => Ok(TreeType::Gt),
        "crt" => Ok(TreeType::Crt),
        "ec" => Ok(TreeType::Ec),
        "frt" => Ok(TreeType::Frt),
        "prt" => Ok(TreeType::Prt),
        "tt" => Ok(TreeType::Tt),
        other => Err(LtpError::EcValidation(format!(
            "Unknown tree type: {}",
            other
        ))),
    }
}

fn tree_type_str(t: TreeType) -> &'static str {
    match t {
        TreeType::Gt => "gt",
        TreeType::Crt => "crt",
        TreeType::Ec => "ec",
        TreeType::Frt => "frt",
        TreeType::Prt => "prt",
        TreeType::Tt => "tt",
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

// --- Output data types ---

/// Data returned by `tree new`.
#[derive(Debug, Serialize)]
pub struct TreeNewData {
    pub id: String,
    pub name: String,
    pub tree_type: TreeType,
    pub logic: TreeLogic,
}

/// Summary for tree listing.
#[derive(Debug, Serialize)]
pub struct TreeSummary {
    pub id: String,
    pub name: String,
    pub tree_type: TreeType,
    pub logic: TreeLogic,
    pub node_count: usize,
    pub edge_count: usize,
}

/// Data returned by `tree list`.
#[derive(Debug, Serialize)]
pub struct TreeListData {
    pub trees: Vec<TreeSummary>,
    pub count: usize,
}

/// Data returned by `tree rm`.
#[derive(Debug, Serialize)]
pub struct TreeRmData {
    pub id: String,
}

/// Data returned by `tree attach`.
#[derive(Debug, Serialize)]
pub struct TreeAttachData {
    pub tree_id: String,
    pub node_id: String,
    pub role: Option<String>,
}

/// Data returned by `tree detach`.
#[derive(Debug, Serialize)]
pub struct TreeDetachData {
    pub tree_id: String,
    pub node_id: String,
    pub edges_removed: usize,
}

/// Data returned by `tree clone`.
#[derive(Debug, Serialize)]
pub struct TreeCloneData {
    pub original_id: String,
    pub new_id: String,
    pub new_name: String,
    pub edges_cloned: usize,
}

/// A single diff entry.
#[derive(Debug, Serialize)]
pub struct DiffEntry {
    pub id: String,
    pub change: String,
}

/// Data returned by `tree diff`.
#[derive(Debug, Serialize)]
pub struct TreeDiffData {
    pub tree_a: String,
    pub tree_b: String,
    pub nodes_added: Vec<String>,
    pub nodes_removed: Vec<String>,
    pub edges_added: Vec<DiffEntry>,
    pub edges_removed: Vec<DiffEntry>,
}

/// A node in the walk result with its immediate context.
#[derive(Debug, Serialize)]
pub struct WalkNode {
    pub id: String,
    pub role: Option<String>,
    pub incoming_edges: Vec<String>,
    pub outgoing_edges: Vec<String>,
}

/// Data returned by `tree walk`.
#[derive(Debug, Serialize)]
pub struct TreeWalkData {
    pub tree_id: String,
    pub order: String,
    pub nodes: Vec<WalkNode>,
}

// --- Command implementations ---

/// Execute `tree new`.
pub fn execute_tree_new(
    storage: &dyn Storage,
    type_str: &str,
    name: &str,
) -> CommandOutput<TreeNewData> {
    let ws_name = storage.workspace_name().unwrap_or_default();

    let tree_type = match parse_tree_type(type_str) {
        Ok(t) => t,
        Err(e) => {
            return CommandOutput {
                success: false,
                action: "tree_new".to_string(),
                workspace: ws_name,
                data: TreeNewData {
                    id: String::new(),
                    name: String::new(),
                    tree_type: TreeType::Crt,
                    logic: TreeLogic::Sufficiency,
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new("INVALID_TREE_TYPE", e.to_string())],
                warnings: vec![],
            };
        }
    };

    let lock_outcome = match storage.acquire_lock("tree new") {
        Ok(o) => o,
        Err(e) => {
            return CommandOutput {
                success: false,
                action: "tree_new".to_string(),
                workspace: ws_name,
                data: TreeNewData {
                    id: String::new(),
                    name: String::new(),
                    tree_type,
                    logic: logic_for_type(tree_type),
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new("LOCK_ERROR", e.to_string())],
                warnings: vec![],
            };
        }
    };

    let slug = slugify(name);
    let id = format!("tree-{}-{}", tree_type_str(tree_type), slug);
    let logic = logic_for_type(tree_type);

    if storage.load_tree(&id).is_ok() {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: "tree_new".to_string(),
            workspace: ws_name,
            data: TreeNewData {
                id: id.clone(),
                name: name.to_string(),
                tree_type,
                logic,
            },
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![OutputError::new(
                "TREE_ALREADY_EXISTS",
                format!("Tree with id '{}' already exists", id),
            )],
            warnings: vec![],
        };
    }

    let tree = Tree {
        id: id.clone(),
        name: name.to_string(),
        tree_type,
        logic,
        nodes: vec![],
        edges: vec![],
        macro_edges: vec![],
        feedback_edges: vec![],
        nbr_branches: vec![],
    };

    if let Err(e) = storage.save_tree(&tree) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: "tree_new".to_string(),
            workspace: ws_name,
            data: TreeNewData {
                id: String::new(),
                name: String::new(),
                tree_type,
                logic,
            },
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
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
        action: "tree_new".to_string(),
        workspace: ws_name,
        data: TreeNewData {
            id,
            name: name.to_string(),
            tree_type,
            logic,
        },
        graph_health: GraphHealth {
            valid_dag: true,
            orphan_nodes_count: 0,
        },
        errors: vec![],
        warnings,
    }
}

/// Execute `tree list`.
pub fn execute_tree_list(storage: &dyn Storage) -> CommandOutput<TreeListData> {
    let ws_name = storage.workspace_name().unwrap_or_default();

    let tree_ids = match storage.list_tree_ids() {
        Ok(ids) => ids,
        Err(e) => {
            return CommandOutput {
                success: false,
                action: "tree_list".to_string(),
                workspace: ws_name,
                data: TreeListData {
                    trees: vec![],
                    count: 0,
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new("IO_ERROR", e.to_string())],
                warnings: vec![],
            };
        }
    };

    let mut trees = Vec::new();
    for id in &tree_ids {
        if let Ok(tree) = storage.load_tree(id) {
            trees.push(TreeSummary {
                id: tree.id,
                name: tree.name,
                tree_type: tree.tree_type,
                logic: tree.logic,
                node_count: tree.nodes.len(),
                edge_count: tree.edges.len(),
            });
        }
    }

    let count = trees.len();
    CommandOutput::ok("tree_list", &ws_name, TreeListData { trees, count })
}

/// Execute `tree rm`.
pub fn execute_tree_rm(storage: &dyn Storage, tree_id: &str) -> CommandOutput<TreeRmData> {
    let ws_name = storage.workspace_name().unwrap_or_default();

    let lock_outcome = match storage.acquire_lock("tree rm") {
        Ok(o) => o,
        Err(e) => {
            return CommandOutput {
                success: false,
                action: "tree_rm".to_string(),
                workspace: ws_name,
                data: TreeRmData {
                    id: tree_id.to_string(),
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new("LOCK_ERROR", e.to_string())],
                warnings: vec![],
            };
        }
    };

    if let Err(e) = storage.delete_tree(tree_id) {
        let _ = storage.release_lock();
        let err = match &e {
            LtpError::TreeNotFound(_) => OutputError::new("TREE_NOT_FOUND", e.to_string()),
            _ => OutputError::new("IO_ERROR", e.to_string()),
        };
        return CommandOutput {
            success: false,
            action: "tree_rm".to_string(),
            workspace: ws_name,
            data: TreeRmData {
                id: tree_id.to_string(),
            },
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![err],
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
        action: "tree_rm".to_string(),
        workspace: ws_name,
        data: TreeRmData {
            id: tree_id.to_string(),
        },
        graph_health: GraphHealth {
            valid_dag: true,
            orphan_nodes_count: 0,
        },
        errors: vec![],
        warnings,
    }
}

/// Execute `tree attach`.
pub fn execute_tree_attach(
    storage: &dyn Storage,
    tree_id: &str,
    node_id: &str,
    role: Option<&str>,
) -> CommandOutput<TreeAttachData> {
    let ws_name = storage.workspace_name().unwrap_or_default();

    let lock_outcome = match storage.acquire_lock("tree attach") {
        Ok(o) => o,
        Err(e) => {
            return CommandOutput {
                success: false,
                action: "tree_attach".to_string(),
                workspace: ws_name,
                data: TreeAttachData {
                    tree_id: tree_id.to_string(),
                    node_id: node_id.to_string(),
                    role: role.map(String::from),
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new("LOCK_ERROR", e.to_string())],
                warnings: vec![],
            };
        }
    };

    // Verify node exists in pool
    if storage.load_node(node_id).is_err() {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: "tree_attach".to_string(),
            workspace: ws_name,
            data: TreeAttachData {
                tree_id: tree_id.to_string(),
                node_id: node_id.to_string(),
                role: role.map(String::from),
            },
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![OutputError::new(
                "NODE_NOT_FOUND",
                format!("Node '{}' not found in pool", node_id),
            )],
            warnings: vec![],
        };
    }

    let mut tree = match storage.load_tree(tree_id) {
        Ok(t) => t,
        Err(e) => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: "tree_attach".to_string(),
                workspace: ws_name,
                data: TreeAttachData {
                    tree_id: tree_id.to_string(),
                    node_id: node_id.to_string(),
                    role: role.map(String::from),
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new("TREE_NOT_FOUND", e.to_string())],
                warnings: vec![],
            };
        }
    };

    // Check if already attached
    if tree.nodes.iter().any(|n| n.node_ref == node_id) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: "tree_attach".to_string(),
            workspace: ws_name,
            data: TreeAttachData {
                tree_id: tree_id.to_string(),
                node_id: node_id.to_string(),
                role: role.map(String::from),
            },
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![OutputError::new(
                "NODE_ALREADY_ATTACHED",
                format!("Node '{}' is already in tree '{}'", node_id, tree_id),
            )],
            warnings: vec![],
        };
    }

    tree.nodes.push(NodeRef {
        node_ref: node_id.to_string(),
        role: role.map(String::from),
    });

    if let Err(e) = storage.save_tree(&tree) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: "tree_attach".to_string(),
            workspace: ws_name,
            data: TreeAttachData {
                tree_id: tree_id.to_string(),
                node_id: node_id.to_string(),
                role: role.map(String::from),
            },
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
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
        action: "tree_attach".to_string(),
        workspace: ws_name,
        data: TreeAttachData {
            tree_id: tree_id.to_string(),
            node_id: node_id.to_string(),
            role: role.map(String::from),
        },
        graph_health: GraphHealth {
            valid_dag: true,
            orphan_nodes_count: 0,
        },
        errors: vec![],
        warnings,
    }
}

/// Execute `tree detach`.
pub fn execute_tree_detach(
    storage: &dyn Storage,
    tree_id: &str,
    node_id: &str,
) -> CommandOutput<TreeDetachData> {
    let ws_name = storage.workspace_name().unwrap_or_default();

    let lock_outcome = match storage.acquire_lock("tree detach") {
        Ok(o) => o,
        Err(e) => {
            return CommandOutput {
                success: false,
                action: "tree_detach".to_string(),
                workspace: ws_name,
                data: TreeDetachData {
                    tree_id: tree_id.to_string(),
                    node_id: node_id.to_string(),
                    edges_removed: 0,
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
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
                action: "tree_detach".to_string(),
                workspace: ws_name,
                data: TreeDetachData {
                    tree_id: tree_id.to_string(),
                    node_id: node_id.to_string(),
                    edges_removed: 0,
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new("TREE_NOT_FOUND", e.to_string())],
                warnings: vec![],
            };
        }
    };

    let before_edges = tree.edges.len();
    tree.nodes.retain(|n| n.node_ref != node_id);
    tree.edges
        .retain(|e| !e.from.contains(&node_id.to_string()) && e.to != node_id);
    let edges_removed = before_edges - tree.edges.len();

    if let Err(e) = storage.save_tree(&tree) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: "tree_detach".to_string(),
            workspace: ws_name,
            data: TreeDetachData {
                tree_id: tree_id.to_string(),
                node_id: node_id.to_string(),
                edges_removed: 0,
            },
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
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
        action: "tree_detach".to_string(),
        workspace: ws_name,
        data: TreeDetachData {
            tree_id: tree_id.to_string(),
            node_id: node_id.to_string(),
            edges_removed,
        },
        graph_health: GraphHealth {
            valid_dag: true,
            orphan_nodes_count: 0,
        },
        errors: vec![],
        warnings,
    }
}

/// Execute `tree clone`.
pub fn execute_tree_clone(
    storage: &dyn Storage,
    tree_id: &str,
    new_name: &str,
) -> CommandOutput<TreeCloneData> {
    let ws_name = storage.workspace_name().unwrap_or_default();

    let lock_outcome = match storage.acquire_lock("tree clone") {
        Ok(o) => o,
        Err(e) => {
            return CommandOutput {
                success: false,
                action: "tree_clone".to_string(),
                workspace: ws_name,
                data: TreeCloneData {
                    original_id: tree_id.to_string(),
                    new_id: String::new(),
                    new_name: String::new(),
                    edges_cloned: 0,
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new("LOCK_ERROR", e.to_string())],
                warnings: vec![],
            };
        }
    };

    let original = match storage.load_tree(tree_id) {
        Ok(t) => t,
        Err(e) => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: "tree_clone".to_string(),
                workspace: ws_name,
                data: TreeCloneData {
                    original_id: tree_id.to_string(),
                    new_id: String::new(),
                    new_name: String::new(),
                    edges_cloned: 0,
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new("TREE_NOT_FOUND", e.to_string())],
                warnings: vec![],
            };
        }
    };

    let slug = slugify(new_name);
    let new_id = format!("tree-{}-{}", tree_type_str(original.tree_type), slug);

    // Clone edges with new IDs
    let mut new_edges = Vec::new();
    for edge in &original.edges {
        let link_id = match storage.next_id("LINK") {
            Ok(id) => id,
            Err(e) => {
                let _ = storage.release_lock();
                return CommandOutput {
                    success: false,
                    action: "tree_clone".to_string(),
                    workspace: ws_name,
                    data: TreeCloneData {
                        original_id: tree_id.to_string(),
                        new_id: String::new(),
                        new_name: String::new(),
                        edges_cloned: 0,
                    },
                    graph_health: GraphHealth {
                        valid_dag: true,
                        orphan_nodes_count: 0,
                    },
                    errors: vec![OutputError::new("ID_GENERATION_ERROR", e.to_string())],
                    warnings: vec![],
                };
            }
        };
        new_edges.push(Edge {
            id: link_id,
            from: edge.from.clone(),
            to: edge.to.clone(),
            operator: edge.operator,
            weight: edge.weight,
            status: edge.status,
            logic: edge.logic,
            assumptions: edge.assumptions.clone(),
        });
    }

    let edges_cloned = new_edges.len();

    let new_tree = Tree {
        id: new_id.clone(),
        name: new_name.to_string(),
        tree_type: original.tree_type,
        logic: original.logic,
        nodes: original.nodes.clone(),
        edges: new_edges,
        macro_edges: vec![],
        feedback_edges: original.feedback_edges,
        nbr_branches: vec![],
    };

    if let Err(e) = storage.save_tree(&new_tree) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: "tree_clone".to_string(),
            workspace: ws_name,
            data: TreeCloneData {
                original_id: tree_id.to_string(),
                new_id: String::new(),
                new_name: String::new(),
                edges_cloned: 0,
            },
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
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
        action: "tree_clone".to_string(),
        workspace: ws_name,
        data: TreeCloneData {
            original_id: tree_id.to_string(),
            new_id,
            new_name: new_name.to_string(),
            edges_cloned,
        },
        graph_health: GraphHealth {
            valid_dag: true,
            orphan_nodes_count: 0,
        },
        errors: vec![],
        warnings,
    }
}

/// Execute `tree diff`.
pub fn execute_tree_diff(
    storage: &dyn Storage,
    tree_a_id: &str,
    tree_b_id: &str,
) -> CommandOutput<TreeDiffData> {
    let ws_name = storage.workspace_name().unwrap_or_default();

    let tree_a = match storage.load_tree(tree_a_id) {
        Ok(t) => t,
        Err(e) => {
            return CommandOutput {
                success: false,
                action: "tree_diff".to_string(),
                workspace: ws_name,
                data: TreeDiffData {
                    tree_a: tree_a_id.to_string(),
                    tree_b: tree_b_id.to_string(),
                    nodes_added: vec![],
                    nodes_removed: vec![],
                    edges_added: vec![],
                    edges_removed: vec![],
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new("TREE_NOT_FOUND", e.to_string())],
                warnings: vec![],
            };
        }
    };

    let tree_b = match storage.load_tree(tree_b_id) {
        Ok(t) => t,
        Err(e) => {
            return CommandOutput {
                success: false,
                action: "tree_diff".to_string(),
                workspace: ws_name,
                data: TreeDiffData {
                    tree_a: tree_a_id.to_string(),
                    tree_b: tree_b_id.to_string(),
                    nodes_added: vec![],
                    nodes_removed: vec![],
                    edges_added: vec![],
                    edges_removed: vec![],
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new("TREE_NOT_FOUND", e.to_string())],
                warnings: vec![],
            };
        }
    };

    let nodes_a: BTreeSet<&str> = tree_a.nodes.iter().map(|n| n.node_ref.as_str()).collect();
    let nodes_b: BTreeSet<&str> = tree_b.nodes.iter().map(|n| n.node_ref.as_str()).collect();

    let nodes_added: Vec<String> = nodes_b
        .difference(&nodes_a)
        .map(|s| s.to_string())
        .collect();
    let nodes_removed: Vec<String> = nodes_a
        .difference(&nodes_b)
        .map(|s| s.to_string())
        .collect();

    // Edge signature: (from sorted, to, operator)
    let edge_sig = |e: &Edge| -> String {
        let mut from = e.from.clone();
        from.sort();
        format!("{}->{}[{:?}]", from.join(","), e.to, e.operator)
    };

    let sigs_a: BTreeMap<String, &Edge> = tree_a.edges.iter().map(|e| (edge_sig(e), e)).collect();
    let sigs_b: BTreeMap<String, &Edge> = tree_b.edges.iter().map(|e| (edge_sig(e), e)).collect();

    let edges_added: Vec<DiffEntry> = sigs_b
        .keys()
        .filter(|k| !sigs_a.contains_key(*k))
        .map(|k| DiffEntry {
            id: sigs_b[k].id.clone(),
            change: "added".to_string(),
        })
        .collect();

    let edges_removed: Vec<DiffEntry> = sigs_a
        .keys()
        .filter(|k| !sigs_b.contains_key(*k))
        .map(|k| DiffEntry {
            id: sigs_a[k].id.clone(),
            change: "removed".to_string(),
        })
        .collect();

    CommandOutput::ok(
        "tree_diff",
        &ws_name,
        TreeDiffData {
            tree_a: tree_a_id.to_string(),
            tree_b: tree_b_id.to_string(),
            nodes_added,
            nodes_removed,
            edges_added,
            edges_removed,
        },
    )
}

/// Execute `tree walk`.
pub fn execute_tree_walk(
    storage: &dyn Storage,
    tree_id: &str,
    order: &str,
) -> CommandOutput<TreeWalkData> {
    let ws_name = storage.workspace_name().unwrap_or_default();

    let tree = match storage.load_tree(tree_id) {
        Ok(t) => t,
        Err(e) => {
            return CommandOutput {
                success: false,
                action: "tree_walk".to_string(),
                workspace: ws_name,
                data: TreeWalkData {
                    tree_id: tree_id.to_string(),
                    order: order.to_string(),
                    nodes: vec![],
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new("TREE_NOT_FOUND", e.to_string())],
                warnings: vec![],
            };
        }
    };

    let node_ids: Vec<&str> = tree.nodes.iter().map(|n| n.node_ref.as_str()).collect();
    let roles: HashMap<&str, Option<&str>> = tree
        .nodes
        .iter()
        .map(|n| (n.node_ref.as_str(), n.role.as_deref()))
        .collect();

    // Build adjacency for topological sort (Kahn's algorithm)
    let mut in_degree: HashMap<&str, usize> = node_ids.iter().map(|&id| (id, 0)).collect();
    let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();

    for edge in &tree.edges {
        for from in &edge.from {
            adjacency
                .entry(from.as_str())
                .or_default()
                .push(edge.to.as_str());
        }
        if let Some(deg) = in_degree.get_mut(edge.to.as_str()) {
            *deg += edge.from.len();
        }
    }

    // Incoming/outgoing edges for context
    let mut incoming: HashMap<&str, Vec<String>> = HashMap::new();
    let mut outgoing: HashMap<&str, Vec<String>> = HashMap::new();

    for edge in &tree.edges {
        incoming
            .entry(edge.to.as_str())
            .or_default()
            .push(edge.id.clone());
        for from in &edge.from {
            outgoing
                .entry(from.as_str())
                .or_default()
                .push(edge.id.clone());
        }
    }

    // Kahn's topological sort
    let mut sorted = Vec::new();
    let mut queue: VecDeque<&str> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(&id, _)| id)
        .collect();

    // Sort queue for determinism
    let mut queue_vec: Vec<&str> = queue.drain(..).collect();
    queue_vec.sort();
    queue = queue_vec.into_iter().collect();

    while let Some(node) = queue.pop_front() {
        sorted.push(node);
        if let Some(neighbors) = adjacency.get(node) {
            let mut next_ready = Vec::new();
            for &neighbor in neighbors {
                if let Some(deg) = in_degree.get_mut(neighbor) {
                    *deg -= 1;
                    if *deg == 0 {
                        next_ready.push(neighbor);
                    }
                }
            }
            next_ready.sort();
            for n in next_ready {
                queue.push_back(n);
            }
        }
    }

    // Add any nodes not in edges (orphans within the tree)
    for &id in &node_ids {
        if !sorted.contains(&id) {
            sorted.push(id);
        }
    }

    if order == "reverse" {
        sorted.reverse();
    }

    let walk_nodes: Vec<WalkNode> = sorted
        .iter()
        .map(|&id| WalkNode {
            id: id.to_string(),
            role: roles.get(id).and_then(|r| r.map(String::from)),
            incoming_edges: incoming.get(id).cloned().unwrap_or_default(),
            outgoing_edges: outgoing.get(id).cloned().unwrap_or_default(),
        })
        .collect();

    CommandOutput::ok(
        "tree_walk",
        &ws_name,
        TreeWalkData {
            tree_id: tree_id.to_string(),
            order: order.to_string(),
            nodes: walk_nodes,
        },
    )
}
