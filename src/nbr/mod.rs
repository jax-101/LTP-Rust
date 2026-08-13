use serde::Serialize;

use crate::output::{CommandOutput, GraphHealth, OutputError, OutputWarning};
use crate::storage::{LockOutcome, Storage};
use crate::tree::types::NbrBranch;

// --- Output data types ---

/// Data returned by `nbr add`.
#[derive(Debug, Serialize)]
pub struct NbrAddData {
    pub nbr_id: String,
    pub tree_id: String,
    pub source_node: String,
    pub trim_injection: Option<String>,
}

/// Data returned by `nbr rm`.
#[derive(Debug, Serialize)]
pub struct NbrRmData {
    pub nbr_id: String,
    pub tree_id: String,
    pub edges_removed: usize,
}

/// Summary of a single NBR for list output.
#[derive(Debug, Serialize)]
pub struct NbrSummary {
    pub id: String,
    pub source_node: String,
    pub edge_count: usize,
    pub has_trim: bool,
}

/// Data returned by `nbr list`.
#[derive(Debug, Serialize)]
pub struct NbrListData {
    pub tree_id: String,
    pub nbr_count: usize,
    pub branches: Vec<NbrSummary>,
}

/// Data returned by `nbr inspect`.
#[derive(Debug, Serialize)]
pub struct NbrInspectData {
    pub nbr_id: String,
    pub tree_id: String,
    pub source_node: String,
    pub trim_injection: Option<String>,
    pub edge_count: usize,
    pub edges: Vec<NbrEdgeInfo>,
    pub nodes_involved: Vec<String>,
}

/// Edge info within an NBR for inspect output.
#[derive(Debug, Serialize)]
pub struct NbrEdgeInfo {
    pub id: String,
    pub from: Vec<String>,
    pub to: String,
    pub operator: String,
}

// --- Helpers ---

fn stale_lock_warning(outcome: &LockOutcome) -> Option<OutputWarning> {
    match outcome {
        LockOutcome::StaleLockRemoved { pid } => Some(OutputWarning::new(
            "STALE_LOCK_REMOVED",
            format!("Stale lock from PID {} was removed", pid),
        )),
        LockOutcome::Acquired => None,
    }
}

// --- Command implementations ---

/// Execute `nbr add`: create an empty NBR branch on a tree.
pub fn execute_nbr_add(
    storage: &dyn Storage,
    tree_id: &str,
    source_node: &str,
    trim: Option<&str>,
) -> CommandOutput<NbrAddData> {
    let ws_name = storage.workspace_name().unwrap_or_default();
    let action = "nbr_add";

    let lock_outcome = match storage.acquire_lock("nbr add") {
        Ok(o) => o,
        Err(e) => {
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: NbrAddData {
                    nbr_id: String::new(),
                    tree_id: tree_id.to_string(),
                    source_node: source_node.to_string(),
                    trim_injection: trim.map(|s| s.to_string()),
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

    let mut warnings = vec![];
    if let Some(w) = stale_lock_warning(&lock_outcome) {
        warnings.push(w);
    }

    // Load tree
    let mut tree = match storage.load_tree(tree_id) {
        Ok(t) => t,
        Err(_) => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: NbrAddData {
                    nbr_id: String::new(),
                    tree_id: tree_id.to_string(),
                    source_node: source_node.to_string(),
                    trim_injection: trim.map(|s| s.to_string()),
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new(
                    "TREE_NOT_FOUND",
                    format!("Tree '{}' not found", tree_id),
                )],
                warnings: vec![],
            };
        }
    };

    // Validate source_node exists in pool
    if storage.load_node(source_node).is_err() {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: NbrAddData {
                nbr_id: String::new(),
                tree_id: tree_id.to_string(),
                source_node: source_node.to_string(),
                trim_injection: trim.map(|s| s.to_string()),
            },
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![OutputError::new(
                "NODE_NOT_FOUND",
                format!("Node '{}' not found in pool", source_node),
            )],
            warnings: vec![],
        };
    }

    // Validate source_node is attached to tree
    if !tree.nodes.iter().any(|n| n.node_ref == source_node) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: NbrAddData {
                nbr_id: String::new(),
                tree_id: tree_id.to_string(),
                source_node: source_node.to_string(),
                trim_injection: trim.map(|s| s.to_string()),
            },
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![OutputError::new(
                "NODE_NOT_IN_TREE",
                format!(
                    "Node '{}' is not attached to tree '{}'",
                    source_node, tree_id
                ),
            )],
            warnings: vec![],
        };
    }

    // Validate trim_injection exists in pool (if provided)
    if let Some(trim_id) = trim {
        if storage.load_node(trim_id).is_err() {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: NbrAddData {
                    nbr_id: String::new(),
                    tree_id: tree_id.to_string(),
                    source_node: source_node.to_string(),
                    trim_injection: Some(trim_id.to_string()),
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new(
                    "NODE_NOT_FOUND",
                    format!("Trim injection node '{}' not found in pool", trim_id),
                )],
                warnings: vec![],
            };
        }
    }

    // Generate NBR ID
    let nbr_id = match storage.next_id("NBR") {
        Ok(id) => id,
        Err(e) => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: NbrAddData {
                    nbr_id: String::new(),
                    tree_id: tree_id.to_string(),
                    source_node: source_node.to_string(),
                    trim_injection: trim.map(|s| s.to_string()),
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

    let nbr_branch = NbrBranch {
        id: nbr_id.clone(),
        source_node: source_node.to_string(),
        edges: vec![],
        trim_injection: trim.map(|s| s.to_string()),
    };

    tree.nbr_branches.push(nbr_branch);

    if let Err(e) = storage.save_tree(&tree) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: NbrAddData {
                nbr_id,
                tree_id: tree_id.to_string(),
                source_node: source_node.to_string(),
                trim_injection: trim.map(|s| s.to_string()),
            },
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![OutputError::new("SAVE_ERROR", e.to_string())],
            warnings: vec![],
        };
    }

    let _ = storage.release_lock();

    let mut output = CommandOutput::ok(
        action,
        &ws_name,
        NbrAddData {
            nbr_id,
            tree_id: tree_id.to_string(),
            source_node: source_node.to_string(),
            trim_injection: trim.map(|s| s.to_string()),
        },
    );
    output.warnings = warnings;
    output
}

/// Execute `nbr rm`: remove an NBR branch from a tree (ADR-010: nodes stay in pool).
pub fn execute_nbr_rm(
    storage: &dyn Storage,
    tree_id: &str,
    nbr_id: &str,
) -> CommandOutput<NbrRmData> {
    let ws_name = storage.workspace_name().unwrap_or_default();
    let action = "nbr_rm";

    let lock_outcome = match storage.acquire_lock("nbr rm") {
        Ok(o) => o,
        Err(e) => {
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: NbrRmData {
                    nbr_id: nbr_id.to_string(),
                    tree_id: tree_id.to_string(),
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

    let mut warnings = vec![];
    if let Some(w) = stale_lock_warning(&lock_outcome) {
        warnings.push(w);
    }

    let mut tree = match storage.load_tree(tree_id) {
        Ok(t) => t,
        Err(_) => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: NbrRmData {
                    nbr_id: nbr_id.to_string(),
                    tree_id: tree_id.to_string(),
                    edges_removed: 0,
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new(
                    "TREE_NOT_FOUND",
                    format!("Tree '{}' not found", tree_id),
                )],
                warnings: vec![],
            };
        }
    };

    let nbr_idx = tree.nbr_branches.iter().position(|b| b.id == nbr_id);
    let nbr_idx = match nbr_idx {
        Some(i) => i,
        None => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: NbrRmData {
                    nbr_id: nbr_id.to_string(),
                    tree_id: tree_id.to_string(),
                    edges_removed: 0,
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new(
                    "NBR_NOT_FOUND",
                    format!("NBR '{}' not found in tree '{}'", nbr_id, tree_id),
                )],
                warnings: vec![],
            };
        }
    };

    let edges_removed = tree.nbr_branches[nbr_idx].edges.len();
    tree.nbr_branches.remove(nbr_idx);

    if let Err(e) = storage.save_tree(&tree) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: NbrRmData {
                nbr_id: nbr_id.to_string(),
                tree_id: tree_id.to_string(),
                edges_removed,
            },
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![OutputError::new("SAVE_ERROR", e.to_string())],
            warnings: vec![],
        };
    }

    let _ = storage.release_lock();

    let mut output = CommandOutput::ok(
        action,
        &ws_name,
        NbrRmData {
            nbr_id: nbr_id.to_string(),
            tree_id: tree_id.to_string(),
            edges_removed,
        },
    );
    output.warnings = warnings;
    output
}

/// Execute `nbr list`: list all NBR branches in a tree.
pub fn execute_nbr_list(storage: &dyn Storage, tree_id: &str) -> CommandOutput<NbrListData> {
    let ws_name = storage.workspace_name().unwrap_or_default();
    let action = "nbr_list";

    let tree = match storage.load_tree(tree_id) {
        Ok(t) => t,
        Err(_) => {
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: NbrListData {
                    tree_id: tree_id.to_string(),
                    nbr_count: 0,
                    branches: vec![],
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new(
                    "TREE_NOT_FOUND",
                    format!("Tree '{}' not found", tree_id),
                )],
                warnings: vec![],
            };
        }
    };

    let branches: Vec<NbrSummary> = tree
        .nbr_branches
        .iter()
        .map(|b| NbrSummary {
            id: b.id.clone(),
            source_node: b.source_node.clone(),
            edge_count: b.edges.len(),
            has_trim: b.trim_injection.is_some(),
        })
        .collect();

    CommandOutput::ok(
        action,
        &ws_name,
        NbrListData {
            tree_id: tree_id.to_string(),
            nbr_count: branches.len(),
            branches,
        },
    )
}

/// Execute `nbr inspect`: show full details of a specific NBR branch.
pub fn execute_nbr_inspect(
    storage: &dyn Storage,
    tree_id: &str,
    nbr_id: &str,
) -> CommandOutput<NbrInspectData> {
    let ws_name = storage.workspace_name().unwrap_or_default();
    let action = "nbr_inspect";

    let tree = match storage.load_tree(tree_id) {
        Ok(t) => t,
        Err(_) => {
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: NbrInspectData {
                    nbr_id: nbr_id.to_string(),
                    tree_id: tree_id.to_string(),
                    source_node: String::new(),
                    trim_injection: None,
                    edge_count: 0,
                    edges: vec![],
                    nodes_involved: vec![],
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new(
                    "TREE_NOT_FOUND",
                    format!("Tree '{}' not found", tree_id),
                )],
                warnings: vec![],
            };
        }
    };

    let nbr = tree.nbr_branches.iter().find(|b| b.id == nbr_id);
    let nbr = match nbr {
        Some(b) => b,
        None => {
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: NbrInspectData {
                    nbr_id: nbr_id.to_string(),
                    tree_id: tree_id.to_string(),
                    source_node: String::new(),
                    trim_injection: None,
                    edge_count: 0,
                    edges: vec![],
                    nodes_involved: vec![],
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new(
                    "NBR_NOT_FOUND",
                    format!("NBR '{}' not found in tree '{}'", nbr_id, tree_id),
                )],
                warnings: vec![],
            };
        }
    };

    let edges: Vec<NbrEdgeInfo> = nbr
        .edges
        .iter()
        .map(|e| NbrEdgeInfo {
            id: e.id.clone(),
            from: e.from.clone(),
            to: e.to.clone(),
            operator: format!("{:?}", e.operator).to_uppercase(),
        })
        .collect();

    // Collect all unique nodes involved in the NBR edges
    let mut nodes_involved: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for edge in &nbr.edges {
        for f in &edge.from {
            nodes_involved.insert(f.clone());
        }
        nodes_involved.insert(edge.to.clone());
    }

    CommandOutput::ok(
        action,
        &ws_name,
        NbrInspectData {
            nbr_id: nbr_id.to_string(),
            tree_id: tree_id.to_string(),
            source_node: nbr.source_node.clone(),
            trim_injection: nbr.trim_injection.clone(),
            edge_count: nbr.edges.len(),
            edges,
            nodes_involved: nodes_involved.into_iter().collect(),
        },
    )
}
