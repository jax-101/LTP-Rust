use serde::Serialize;

use crate::link::types::{AssumptionStatus, Operator};
use crate::output::{CommandOutput, GraphHealth, OutputError, OutputWarning};
use crate::storage::{LockOutcome, Storage};
use crate::validate::check_dag;

// NOTE: reserved for `link group` (Task 5) and `link reoperator` (Task 6),
// which parse an `--operator` string argument. Not yet called from this
// module until those subcommands land.
#[allow(dead_code)]
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

// --- Command implementations ---

/// Execute `link reverse`.
///
/// Swaps the cause and effect ends of a single-source edge: the previous
/// `to` becomes the sole `from`, and the previous `from[0]` becomes `to`.
/// Edges with more than one cause (grouped AND/OR/XOR) cannot be reversed
/// directly; `link dissolve` must be used first.
///
/// If the edge carries assumptions, reversing it invalidates their original
/// causal framing, so the caller must pass `force = true`. When forced, all
/// assumptions on the edge are marked `needs_review`.
///
/// Always followed by a DAG check on the mutated edge set; if the reversal
/// would introduce a cycle, the mutation is rolled back and the tree is not
/// persisted.
pub fn execute_link_reverse(
    storage: &dyn Storage,
    tree_id: &str,
    link_id: &str,
    force: bool,
) -> CommandOutput<LinkReverseData> {
    let ws_name = storage.workspace_name().unwrap_or_default();
    let action = "link_reverse";

    let empty_data = || LinkReverseData {
        link_id: link_id.to_string(),
        tree_id: tree_id.to_string(),
        new_from: vec![],
        new_to: String::new(),
    };

    let lock_outcome = match storage.acquire_lock("link reverse") {
        Ok(o) => o,
        Err(e) => {
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: empty_data(),
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
                action: action.to_string(),
                workspace: ws_name,
                data: empty_data(),
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
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
                data: empty_data(),
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new(
                    "LINK_NOT_FOUND",
                    format!("Edge '{}' not found in tree '{}'", link_id, tree_id),
                )],
                warnings: vec![],
            };
        }
    };

    // Edges with assumptions require --force: reversing changes the causal
    // framing the assumption was written against.
    if !tree.edges[edge_idx].assumptions.is_empty() && !force {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: empty_data(),
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![OutputError::new(
                "REVERSE_REQUIRES_FORCE",
                "Edge has assumptions; use --force to reverse (assumptions will be marked needs_review)",
            )],
            warnings: vec![],
        };
    }

    // Only single-source edges can be reversed unambiguously: from becomes
    // [old_to] and to becomes old_from[0]. Grouped edges (from.len() > 1)
    // must be dissolved first.
    let edge = &tree.edges[edge_idx];
    if edge.from.len() > 1 {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: empty_data(),
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
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

    tree.edges[edge_idx].from = new_from.clone();
    tree.edges[edge_idx].to = new_to.clone();

    if force {
        for asm in &mut tree.edges[edge_idx].assumptions {
            asm.status = AssumptionStatus::NeedsReview;
        }
    }

    if let Err(e) = check_dag(&tree.edges, tree_id) {
        // Roll back the mutation; nothing is persisted on failure.
        tree.edges[edge_idx].from = old_from;
        tree.edges[edge_idx].to = old_to;
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: empty_data(),
            graph_health: GraphHealth {
                valid_dag: false,
                orphan_nodes_count: 0,
            },
            errors: vec![OutputError::new(
                "CIRCULAR_DEPENDENCY_DETECTED",
                e.to_string(),
            )],
            warnings: vec![],
        };
    }

    if let Err(e) = storage.save_tree(&tree) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: empty_data(),
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
        action: action.to_string(),
        workspace: ws_name,
        data: LinkReverseData {
            link_id: link_id.to_string(),
            tree_id: tree_id.to_string(),
            new_from,
            new_to,
        },
        graph_health: GraphHealth {
            valid_dag: true,
            orphan_nodes_count: 0,
        },
        errors: vec![],
        warnings,
    }
}
