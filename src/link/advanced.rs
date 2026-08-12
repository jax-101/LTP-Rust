use serde::Serialize;

use crate::errors::LtpError;
use crate::link::types::{Assumption, AssumptionStatus, Edge, EdgeStatus, Logic, Operator};
use crate::output::{CommandOutput, GraphHealth, OutputError, OutputWarning};
use crate::storage::{LockOutcome, Storage};
use crate::validate::check_dag;

/// Parses a CLI `--operator` string into an [`Operator`], case-insensitively.
/// Shared by `link group` and `link reoperator`.
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

/// Execute `link move`.
///
/// Redirects an existing edge to a new origin (`new_from`) and/or a new
/// destination (`new_to`). At least one of the two must be provided by the
/// caller (enforced by the CLI dispatch layer). A redirected origin always
/// replaces the entire `from` vector with a single node, since `link move`
/// targets a single-node redirect rather than a grouped-cause edge.
///
/// New endpoints must exist in the node pool and be attached to the target
/// tree, mirroring the checks performed by `link connect`. The mutation is
/// followed by a DAG check; if the move would introduce a cycle, the tree is
/// not persisted and the original edge is left untouched on disk.
pub fn execute_link_move(
    storage: &dyn Storage,
    tree_id: &str,
    link_id: &str,
    new_from: Option<&str>,
    new_to: Option<&str>,
) -> CommandOutput<LinkMoveData> {
    let ws_name = storage.workspace_name().unwrap_or_default();
    let action = "link_move";

    let empty_data = || LinkMoveData {
        link_id: link_id.to_string(),
        tree_id: tree_id.to_string(),
    };

    if new_from.is_none() && new_to.is_none() {
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
                "INVALID_ARGS",
                "At least one of --new-from or --new-to must be provided",
            )],
            warnings: vec![],
        };
    }

    let lock_outcome = match storage.acquire_lock("link move") {
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

    // Validate new endpoints exist in the node pool and are attached to the
    // tree, mirroring `link connect`'s referential-integrity checks.
    for node_id in new_from.into_iter().chain(new_to) {
        if storage.load_node(node_id).is_err() {
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
                    "REFERENTIAL_INTEGRITY_VIOLATION",
                    format!("Node '{}' not found in pool", node_id),
                )],
                warnings: vec![],
            };
        }

        if !tree.nodes.iter().any(|n| n.node_ref == node_id) {
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
                    "NODE_NOT_IN_TREE",
                    format!("Node '{}' is not attached to tree '{}'", node_id, tree_id),
                )],
                warnings: vec![],
            };
        }
    }

    let old_from = tree.edges[edge_idx].from.clone();
    let old_to = tree.edges[edge_idx].to.clone();

    if let Some(nf) = new_from {
        tree.edges[edge_idx].from = vec![nf.to_string()];
    }
    if let Some(nt) = new_to {
        tree.edges[edge_idx].to = nt.to_string();
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
        data: empty_data(),
        graph_health: GraphHealth {
            valid_dag: true,
            orphan_nodes_count: 0,
        },
        errors: vec![],
        warnings,
    }
}

/// Builds a failed `link insert-between` output with empty data, sharing the
/// boilerplate common to every early-exit branch below.
fn insert_between_error(
    ws_name: &str,
    tree_id: &str,
    code: &str,
    detail: impl Into<String>,
) -> CommandOutput<LinkInsertBetweenData> {
    CommandOutput {
        success: false,
        action: "link_insert_between".to_string(),
        workspace: ws_name.to_string(),
        data: LinkInsertBetweenData {
            removed_link: String::new(),
            created_links: vec![],
            tree_id: tree_id.to_string(),
        },
        graph_health: GraphHealth {
            valid_dag: true,
            orphan_nodes_count: 0,
        },
        errors: vec![OutputError::new(code, detail)],
        warnings: vec![],
    }
}

/// A freshly minted SINGLE edge with no operator-specific metadata.
fn single_edge(id: String, from: String, to: String) -> Edge {
    Edge {
        id,
        from: vec![from],
        to,
        operator: Operator::Single,
        weight: None,
        status: EdgeStatus::Active,
        logic: Logic::Sufficiency,
        assumptions: vec![],
    }
}

/// Execute `link insert-between`.
///
/// Inserts an intermediate node into an existing edge. Behavior depends on
/// the edge being split:
///
/// - **SINGLE edge** (`operator == Single`, i.e. a single cause): `A -> B`
///   is replaced by two brand-new SINGLE edges, `A -> X` and `X -> B`; the
///   original edge is removed. This is the only path available to a
///   single-cause edge, so `--insert-after-cause` and
///   `--insert-before-effect` are both ignored in this case — extracting
///   the sole cause from a "group" of one and replacing it is equivalent to
///   the plain split.
/// - **Grouped edge (AND/OR/MAG/XOR) with `--insert-after-cause <CAUSE_ID>`**:
///   `CAUSE_ID` is extracted from the edge's `from[]` and replaced in place
///   by the intermediate node; a new SINGLE edge `CAUSE_ID -> X` is created.
///   The original edge survives (only its `from[]` changes), so
///   `removed_link` is empty and `created_links` holds just the one new
///   edge.
/// - **Grouped edge with `--insert-before-effect`**: `[A, B] -> C` becomes
///   `[A, B] -> X` (a new edge that keeps the original operator/weight/
///   logic) plus a new SINGLE edge `X -> C`. The original edge is removed;
///   both new edges are reported in `created_links`.
///
/// A grouped edge must pick exactly one of the two flags — passing both, or
/// neither, fails with `INVALID_ARGS` before any state is touched.
///
/// The intermediate node must already exist in the node pool and be
/// attached to the target tree. Every branch is followed by a DAG check
/// against the *resulting* edge set; if it would introduce a cycle, nothing
/// is persisted and the tree on disk is left untouched.
pub fn execute_link_insert_between(
    storage: &dyn Storage,
    tree_id: &str,
    link_id: &str,
    node_id: &str,
    insert_after_cause: Option<&str>,
    insert_before_effect: bool,
) -> CommandOutput<LinkInsertBetweenData> {
    let ws_name = storage.workspace_name().unwrap_or_default();
    let action = "link_insert_between";

    if insert_after_cause.is_some() && insert_before_effect {
        return insert_between_error(
            &ws_name,
            tree_id,
            "INVALID_ARGS",
            "Cannot combine --insert-after-cause and --insert-before-effect",
        );
    }

    let lock_outcome = match storage.acquire_lock("link insert-between") {
        Ok(o) => o,
        Err(e) => return insert_between_error(&ws_name, tree_id, "LOCK_ERROR", e.to_string()),
    };

    let mut tree = match storage.load_tree(tree_id) {
        Ok(t) => t,
        Err(e) => {
            let _ = storage.release_lock();
            return insert_between_error(&ws_name, tree_id, "TREE_NOT_FOUND", e.to_string());
        }
    };

    let edge_idx = match tree.edges.iter().position(|e| e.id == link_id) {
        Some(i) => i,
        None => {
            let _ = storage.release_lock();
            return insert_between_error(
                &ws_name,
                tree_id,
                "LINK_NOT_FOUND",
                format!("Edge '{}' not found in tree '{}'", link_id, tree_id),
            );
        }
    };

    if storage.load_node(node_id).is_err() {
        let _ = storage.release_lock();
        return insert_between_error(
            &ws_name,
            tree_id,
            "REFERENTIAL_INTEGRITY_VIOLATION",
            format!("Node '{}' not found in pool", node_id),
        );
    }
    if !tree.nodes.iter().any(|n| n.node_ref == node_id) {
        let _ = storage.release_lock();
        return insert_between_error(
            &ws_name,
            tree_id,
            "NODE_NOT_IN_TREE",
            format!("Node '{}' is not attached to tree '{}'", node_id, tree_id),
        );
    }

    let edge = tree.edges[edge_idx].clone();

    // Compute the edge list the tree would have *after* the mutation, plus
    // the created/removed link IDs to report. Nothing touches `tree.edges`
    // until the DAG check below passes.
    let (new_edges, removed_link, created_links): (Vec<Edge>, String, Vec<String>) =
        if edge.operator == Operator::Single {
            let a = match edge.from.first() {
                Some(a) => a.clone(),
                None => {
                    let _ = storage.release_lock();
                    return insert_between_error(
                        &ws_name,
                        tree_id,
                        "INVALID_EDGE_STATE",
                        format!("Edge '{}' has no source node", link_id),
                    );
                }
            };
            let b = edge.to;

            let new1_id = match storage.next_id("LINK") {
                Ok(id) => id,
                Err(e) => {
                    let _ = storage.release_lock();
                    return insert_between_error(
                        &ws_name,
                        tree_id,
                        "ID_GENERATION_ERROR",
                        e.to_string(),
                    );
                }
            };
            let new2_id = match storage.next_id("LINK") {
                Ok(id) => id,
                Err(e) => {
                    let _ = storage.release_lock();
                    return insert_between_error(
                        &ws_name,
                        tree_id,
                        "ID_GENERATION_ERROR",
                        e.to_string(),
                    );
                }
            };

            let edge1 = single_edge(new1_id.clone(), a, node_id.to_string());
            let edge2 = single_edge(new2_id.clone(), node_id.to_string(), b);

            let mut edges: Vec<Edge> = tree
                .edges
                .iter()
                .filter(|e| e.id != link_id)
                .cloned()
                .collect();
            edges.push(edge1);
            edges.push(edge2);

            (edges, link_id.to_string(), vec![new1_id, new2_id])
        } else if let Some(cause_id) = insert_after_cause {
            let pos = match edge.from.iter().position(|f| f == cause_id) {
                Some(p) => p,
                None => {
                    let _ = storage.release_lock();
                    return insert_between_error(
                        &ws_name,
                        tree_id,
                        "CAUSE_NOT_IN_GROUP",
                        format!(
                            "Cause '{}' is not in the from[] of edge '{}'",
                            cause_id, link_id
                        ),
                    );
                }
            };

            let new_id = match storage.next_id("LINK") {
                Ok(id) => id,
                Err(e) => {
                    let _ = storage.release_lock();
                    return insert_between_error(
                        &ws_name,
                        tree_id,
                        "ID_GENERATION_ERROR",
                        e.to_string(),
                    );
                }
            };

            let new_edge = single_edge(new_id.clone(), cause_id.to_string(), node_id.to_string());

            let mut edges = tree.edges.clone();
            edges[edge_idx].from[pos] = node_id.to_string();
            edges.push(new_edge);

            (edges, String::new(), vec![new_id])
        } else if insert_before_effect {
            let new1_id = match storage.next_id("LINK") {
                Ok(id) => id,
                Err(e) => {
                    let _ = storage.release_lock();
                    return insert_between_error(
                        &ws_name,
                        tree_id,
                        "ID_GENERATION_ERROR",
                        e.to_string(),
                    );
                }
            };
            let new2_id = match storage.next_id("LINK") {
                Ok(id) => id,
                Err(e) => {
                    let _ = storage.release_lock();
                    return insert_between_error(
                        &ws_name,
                        tree_id,
                        "ID_GENERATION_ERROR",
                        e.to_string(),
                    );
                }
            };

            let edge1 = Edge {
                id: new1_id.clone(),
                from: edge.from.clone(),
                to: node_id.to_string(),
                operator: edge.operator,
                weight: edge.weight,
                status: EdgeStatus::Active,
                logic: edge.logic,
                assumptions: vec![],
            };
            let edge2 = single_edge(new2_id.clone(), node_id.to_string(), edge.to);

            let mut edges: Vec<Edge> = tree
                .edges
                .iter()
                .filter(|e| e.id != link_id)
                .cloned()
                .collect();
            edges.push(edge1);
            edges.push(edge2);

            (edges, link_id.to_string(), vec![new1_id, new2_id])
        } else {
            let _ = storage.release_lock();
            return insert_between_error(
                &ws_name,
                tree_id,
                "INVALID_ARGS",
                "Grouped edge requires --insert-after-cause or --insert-before-effect",
            );
        };

    if let Err(e) = check_dag(&new_edges, tree_id) {
        let _ = storage.release_lock();
        let err = match &e {
            LtpError::CircularDependencyDetected { .. } => {
                OutputError::new("CIRCULAR_DEPENDENCY_DETECTED", e.to_string())
            }
            _ => OutputError::new("VALIDATION_ERROR", e.to_string()),
        };
        return CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: LinkInsertBetweenData {
                removed_link: String::new(),
                created_links: vec![],
                tree_id: tree_id.to_string(),
            },
            graph_health: GraphHealth {
                valid_dag: false,
                orphan_nodes_count: 0,
            },
            errors: vec![err],
            warnings: vec![],
        };
    }

    tree.edges = new_edges;

    if let Err(e) = storage.save_tree(&tree) {
        let _ = storage.release_lock();
        return insert_between_error(&ws_name, tree_id, "IO_ERROR", e.to_string());
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
        data: LinkInsertBetweenData {
            removed_link,
            created_links,
            tree_id: tree_id.to_string(),
        },
        graph_health: GraphHealth {
            valid_dag: true,
            orphan_nodes_count: 0,
        },
        errors: vec![],
        warnings,
    }
}

/// Builds a failed `link group` output with empty data, sharing the
/// boilerplate common to every early-exit branch below.
fn group_error(
    ws_name: &str,
    tree_id: &str,
    code: &str,
    detail: impl Into<String>,
) -> CommandOutput<LinkGroupData> {
    CommandOutput {
        success: false,
        action: "link_group".to_string(),
        workspace: ws_name.to_string(),
        data: LinkGroupData {
            created_link: String::new(),
            removed_links: vec![],
            tree_id: tree_id.to_string(),
        },
        graph_health: GraphHealth {
            valid_dag: true,
            orphan_nodes_count: 0,
        },
        errors: vec![OutputError::new(code, detail)],
        warnings: vec![],
    }
}

/// Execute `link group`.
///
/// Merges two or more independent SINGLE edges that share the same
/// destination into a single edge with a combined `from[]` under the given
/// operator (AND/OR/MAG/XOR). Each input edge must itself be SINGLE (a
/// single cause); an edge that is already a group must be dissolved or
/// split first. This mirrors `link connect`'s own restriction that grouped
/// causes only ever combine at edge-creation time — `link group` is the
/// retroactive equivalent for edges that were connected independently.
///
/// Fails with `GROUP_DESTINATION_MISMATCH` if the input edges do not all
/// point to the same `to` node — grouping only makes sense when the causes
/// converge on one effect.
///
/// The new edge gets a fresh ID via `storage.next_id("LINK")`; the original
/// edges are removed. Followed by a DAG check on the resulting edge set,
/// though changing `from[]` cardinality on an edge whose direction is
/// unchanged cannot introduce a cycle that didn't already exist.
pub fn execute_link_group(
    storage: &dyn Storage,
    tree_id: &str,
    link_ids: &[String],
    operator: &str,
) -> CommandOutput<LinkGroupData> {
    let ws_name = storage.workspace_name().unwrap_or_default();
    let action = "link_group";

    if link_ids.len() < 2 {
        return group_error(
            &ws_name,
            tree_id,
            "INVALID_ARGS",
            "At least two links are required for `link group`",
        );
    }

    let op = match parse_operator(operator) {
        Some(Operator::Single) => {
            return group_error(
                &ws_name,
                tree_id,
                "INVALID_OPERATOR",
                "`link group` requires AND, OR, MAG or XOR; SINGLE cannot combine multiple causes",
            );
        }
        Some(o) => o,
        None => {
            return group_error(
                &ws_name,
                tree_id,
                "INVALID_OPERATOR",
                format!("Unknown operator: {}", operator),
            );
        }
    };

    let lock_outcome = match storage.acquire_lock("link group") {
        Ok(o) => o,
        Err(e) => return group_error(&ws_name, tree_id, "LOCK_ERROR", e.to_string()),
    };

    let mut tree = match storage.load_tree(tree_id) {
        Ok(t) => t,
        Err(e) => {
            let _ = storage.release_lock();
            return group_error(&ws_name, tree_id, "TREE_NOT_FOUND", e.to_string());
        }
    };

    // Resolve each link_id to its edge, preserving caller order so the
    // resulting from[] lines up with the order links were listed in.
    let mut source_edges: Vec<Edge> = Vec::with_capacity(link_ids.len());
    for link_id in link_ids {
        match tree.edges.iter().find(|e| &e.id == link_id) {
            Some(e) => source_edges.push(e.clone()),
            None => {
                let _ = storage.release_lock();
                return group_error(
                    &ws_name,
                    tree_id,
                    "LINK_NOT_FOUND",
                    format!("Edge '{}' not found in tree '{}'", link_id, tree_id),
                );
            }
        }
    }

    if let Some(bad) = source_edges.iter().find(|e| e.operator != Operator::Single) {
        let _ = storage.release_lock();
        return group_error(
            &ws_name,
            tree_id,
            "EDGE_NOT_SINGLE",
            format!(
                "Edge '{}' is not a SINGLE edge; dissolve or split it before grouping",
                bad.id
            ),
        );
    }

    let to = source_edges[0].to.clone();
    if source_edges.iter().any(|e| e.to != to) {
        let _ = storage.release_lock();
        return group_error(
            &ws_name,
            tree_id,
            "GROUP_DESTINATION_MISMATCH",
            "All links passed to `link group` must share the same destination node",
        );
    }

    let new_id = match storage.next_id("LINK") {
        Ok(id) => id,
        Err(e) => {
            let _ = storage.release_lock();
            return group_error(&ws_name, tree_id, "ID_GENERATION_ERROR", e.to_string());
        }
    };

    let mut warnings: Vec<OutputWarning> = vec![];
    if op == Operator::Mag {
        warnings.push(OutputWarning::new(
            "MAG_WEIGHT_MISSING",
            "Operator MAG without --weight; magnitude estimation pending",
        ));
    }

    // `from[0]` is safe: every source edge was confirmed SINGLE above, so
    // each has exactly one cause.
    let combined_from: Vec<String> = source_edges.iter().map(|e| e.from[0].clone()).collect();
    let new_edge = Edge {
        id: new_id.clone(),
        from: combined_from,
        to,
        operator: op,
        weight: None,
        status: EdgeStatus::Active,
        logic: Logic::Sufficiency,
        assumptions: vec![],
    };

    let mut new_edges: Vec<Edge> = tree
        .edges
        .iter()
        .filter(|e| !link_ids.contains(&e.id))
        .cloned()
        .collect();
    new_edges.push(new_edge);

    if let Err(e) = check_dag(&new_edges, tree_id) {
        let _ = storage.release_lock();
        let code = match &e {
            LtpError::CircularDependencyDetected { .. } => "CIRCULAR_DEPENDENCY_DETECTED",
            _ => "VALIDATION_ERROR",
        };
        let mut out = group_error(&ws_name, tree_id, code, e.to_string());
        out.graph_health.valid_dag = false;
        return out;
    }

    tree.edges = new_edges;

    if let Err(e) = storage.save_tree(&tree) {
        let _ = storage.release_lock();
        return group_error(&ws_name, tree_id, "IO_ERROR", e.to_string());
    }

    let _ = storage.release_lock();

    if let Some(w) = stale_lock_warning(&lock_outcome) {
        warnings.insert(0, w);
    }

    CommandOutput {
        success: true,
        action: action.to_string(),
        workspace: ws_name,
        data: LinkGroupData {
            created_link: new_id,
            removed_links: link_ids.to_vec(),
            tree_id: tree_id.to_string(),
        },
        graph_health: GraphHealth {
            valid_dag: true,
            orphan_nodes_count: 0,
        },
        errors: vec![],
        warnings,
    }
}

/// Builds a failed `link dissolve` output with empty data, sharing the
/// boilerplate common to every early-exit branch below.
fn dissolve_error(
    ws_name: &str,
    tree_id: &str,
    code: &str,
    detail: impl Into<String>,
) -> CommandOutput<LinkDissolveData> {
    CommandOutput {
        success: false,
        action: "link_dissolve".to_string(),
        workspace: ws_name.to_string(),
        data: LinkDissolveData {
            created_links: vec![],
            removed_link: String::new(),
            tree_id: tree_id.to_string(),
        },
        graph_health: GraphHealth {
            valid_dag: true,
            orphan_nodes_count: 0,
        },
        errors: vec![OutputError::new(code, detail)],
        warnings: vec![],
    }
}

/// Execute `link dissolve`.
///
/// The inverse of `link group`: takes a grouped edge (`from.len() > 1`) and
/// splits it back into one independent SINGLE edge per cause, each pointing
/// to the same `to` node. Fails with `EDGE_ALREADY_SINGLE` if the edge has
/// only one cause — there is nothing to dissolve.
///
/// Any assumptions attached to the original edge are copied onto *every*
/// new SINGLE edge (per ENGINE_SPEC.md §2.7), each forced to
/// `AssumptionStatus::NeedsReview` since dissolving changes the causal
/// framing the assumption was written against — the same rationale
/// `link reverse --force` uses for its own assumptions.
///
/// Each new edge gets a fresh ID via `storage.next_id("LINK")`; the
/// original grouped edge is removed. Followed by a DAG check on the
/// resulting edge set.
pub fn execute_link_dissolve(
    storage: &dyn Storage,
    tree_id: &str,
    link_id: &str,
) -> CommandOutput<LinkDissolveData> {
    let ws_name = storage.workspace_name().unwrap_or_default();
    let action = "link_dissolve";

    let lock_outcome = match storage.acquire_lock("link dissolve") {
        Ok(o) => o,
        Err(e) => return dissolve_error(&ws_name, tree_id, "LOCK_ERROR", e.to_string()),
    };

    let mut tree = match storage.load_tree(tree_id) {
        Ok(t) => t,
        Err(e) => {
            let _ = storage.release_lock();
            return dissolve_error(&ws_name, tree_id, "TREE_NOT_FOUND", e.to_string());
        }
    };

    let edge = match tree.edges.iter().find(|e| e.id == link_id) {
        Some(e) => e.clone(),
        None => {
            let _ = storage.release_lock();
            return dissolve_error(
                &ws_name,
                tree_id,
                "LINK_NOT_FOUND",
                format!("Edge '{}' not found in tree '{}'", link_id, tree_id),
            );
        }
    };

    if edge.from.len() <= 1 {
        let _ = storage.release_lock();
        return dissolve_error(
            &ws_name,
            tree_id,
            "EDGE_ALREADY_SINGLE",
            format!("Edge '{}' has a single cause; nothing to dissolve", link_id),
        );
    }

    let inherited_assumptions: Vec<Assumption> = edge
        .assumptions
        .iter()
        .cloned()
        .map(|mut a| {
            a.status = AssumptionStatus::NeedsReview;
            a
        })
        .collect();

    let mut created_edges: Vec<Edge> = Vec::with_capacity(edge.from.len());
    for cause in &edge.from {
        let new_id = match storage.next_id("LINK") {
            Ok(id) => id,
            Err(e) => {
                let _ = storage.release_lock();
                return dissolve_error(&ws_name, tree_id, "ID_GENERATION_ERROR", e.to_string());
            }
        };
        created_edges.push(Edge {
            id: new_id,
            from: vec![cause.clone()],
            to: edge.to.clone(),
            operator: Operator::Single,
            weight: None,
            status: EdgeStatus::Active,
            logic: edge.logic,
            assumptions: inherited_assumptions.clone(),
        });
    }

    let mut new_edges: Vec<Edge> = tree
        .edges
        .iter()
        .filter(|e| e.id != link_id)
        .cloned()
        .collect();
    new_edges.extend(created_edges.iter().cloned());

    if let Err(e) = check_dag(&new_edges, tree_id) {
        let _ = storage.release_lock();
        let code = match &e {
            LtpError::CircularDependencyDetected { .. } => "CIRCULAR_DEPENDENCY_DETECTED",
            _ => "VALIDATION_ERROR",
        };
        let mut out = dissolve_error(&ws_name, tree_id, code, e.to_string());
        out.graph_health.valid_dag = false;
        return out;
    }

    let created_ids: Vec<String> = created_edges.iter().map(|e| e.id.clone()).collect();
    tree.edges = new_edges;

    if let Err(e) = storage.save_tree(&tree) {
        let _ = storage.release_lock();
        return dissolve_error(&ws_name, tree_id, "IO_ERROR", e.to_string());
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
        data: LinkDissolveData {
            created_links: created_ids,
            removed_link: link_id.to_string(),
            tree_id: tree_id.to_string(),
        },
        graph_health: GraphHealth {
            valid_dag: true,
            orphan_nodes_count: 0,
        },
        errors: vec![],
        warnings,
    }
}

/// Builds a failed `link split` output with empty data, sharing the
/// boilerplate common to every early-exit branch below.
fn split_error(
    ws_name: &str,
    tree_id: &str,
    link_id: &str,
    code: &str,
    detail: impl Into<String>,
) -> CommandOutput<LinkSplitData> {
    CommandOutput {
        success: false,
        action: "link_split".to_string(),
        workspace: ws_name.to_string(),
        data: LinkSplitData {
            extracted_link: String::new(),
            original_link: link_id.to_string(),
            tree_id: tree_id.to_string(),
        },
        graph_health: GraphHealth {
            valid_dag: true,
            orphan_nodes_count: 0,
        },
        errors: vec![OutputError::new(code, detail)],
        warnings: vec![],
    }
}

/// Execute `link split`.
///
/// Extracts one or more causes from a grouped edge (`from.len() > 1`)
/// without fully dissolving it, per ENGINE_SPEC.md §2.7. The extracted
/// causes form a brand-new independent edge to the same `to` node: SINGLE
/// if exactly one cause is extracted, otherwise the original edge's own
/// operator (this interface has no `--new-operator` override; use
/// `link reoperator` afterwards to change it).
///
/// The original edge keeps whatever causes were not extracted. If that
/// leaves it with exactly one cause, it is automatically downgraded to
/// SINGLE — and any MAG `weight` is dropped, since SINGLE edges never carry
/// one. Extracting every cause — leaving the original empty — is rejected
/// with `CANNOT_EXTRACT_ALL_CAUSES`; `link dissolve` is the command for
/// breaking a group apart entirely.
///
/// Fails with `CAUSE_NOT_IN_GROUP` if any requested node is not present in
/// the edge's `from[]`. Followed by a DAG check for consistency with every
/// other topology-mutating command, though extracting causes into an edge
/// that keeps the same direction can never introduce a cycle.
pub fn execute_link_split(
    storage: &dyn Storage,
    tree_id: &str,
    link_id: &str,
    extract_nodes: &[String],
) -> CommandOutput<LinkSplitData> {
    let ws_name = storage.workspace_name().unwrap_or_default();
    let action = "link_split";

    if extract_nodes.is_empty() {
        return split_error(
            &ws_name,
            tree_id,
            link_id,
            "INVALID_ARGS",
            "`link split` requires at least one node in --extract",
        );
    }

    let lock_outcome = match storage.acquire_lock("link split") {
        Ok(o) => o,
        Err(e) => return split_error(&ws_name, tree_id, link_id, "LOCK_ERROR", e.to_string()),
    };

    let mut tree = match storage.load_tree(tree_id) {
        Ok(t) => t,
        Err(e) => {
            let _ = storage.release_lock();
            return split_error(&ws_name, tree_id, link_id, "TREE_NOT_FOUND", e.to_string());
        }
    };

    let edge_idx = match tree.edges.iter().position(|e| e.id == link_id) {
        Some(i) => i,
        None => {
            let _ = storage.release_lock();
            return split_error(
                &ws_name,
                tree_id,
                link_id,
                "LINK_NOT_FOUND",
                format!("Edge '{}' not found in tree '{}'", link_id, tree_id),
            );
        }
    };

    let edge = tree.edges[edge_idx].clone();

    if let Some(missing) = extract_nodes.iter().find(|n| !edge.from.contains(n)) {
        let _ = storage.release_lock();
        return split_error(
            &ws_name,
            tree_id,
            link_id,
            "CAUSE_NOT_IN_GROUP",
            format!(
                "Cause '{}' is not in the from[] of edge '{}'",
                missing, link_id
            ),
        );
    }

    let remaining: Vec<String> = edge
        .from
        .iter()
        .filter(|n| !extract_nodes.contains(n))
        .cloned()
        .collect();

    if remaining.is_empty() {
        let _ = storage.release_lock();
        return split_error(
            &ws_name,
            tree_id,
            link_id,
            "CANNOT_EXTRACT_ALL_CAUSES",
            "Cannot extract every cause from a group; use `link dissolve` instead",
        );
    }

    let new_id = match storage.next_id("LINK") {
        Ok(id) => id,
        Err(e) => {
            let _ = storage.release_lock();
            return split_error(
                &ws_name,
                tree_id,
                link_id,
                "ID_GENERATION_ERROR",
                e.to_string(),
            );
        }
    };

    let mut warnings: Vec<OutputWarning> = vec![];

    // Exactly one extracted cause always becomes a SINGLE edge; extracting
    // several keeps the original operator (no `--new-operator` override in
    // this interface).
    let extracted_operator = if extract_nodes.len() == 1 {
        Operator::Single
    } else {
        edge.operator
    };
    if extracted_operator == Operator::Mag {
        warnings.push(OutputWarning::new(
            "MAG_WEIGHT_MISSING",
            "Operator MAG without --weight; magnitude estimation pending",
        ));
    }
    let extracted_edge = Edge {
        id: new_id.clone(),
        from: extract_nodes.to_vec(),
        to: edge.to.clone(),
        operator: extracted_operator,
        weight: None,
        status: EdgeStatus::Active,
        logic: edge.logic,
        assumptions: vec![],
    };

    // Remaining causes reduce the original edge; a single leftover cause
    // auto-converts it to SINGLE (dropping any MAG weight, which only
    // applies to grouped edges).
    let (original_operator, original_weight) = if remaining.len() == 1 {
        (Operator::Single, None)
    } else {
        (edge.operator, edge.weight)
    };

    let mut new_edges = tree.edges.clone();
    new_edges[edge_idx].from = remaining;
    new_edges[edge_idx].operator = original_operator;
    new_edges[edge_idx].weight = original_weight;
    new_edges.push(extracted_edge);

    if let Err(e) = check_dag(&new_edges, tree_id) {
        let _ = storage.release_lock();
        let code = match &e {
            LtpError::CircularDependencyDetected { .. } => "CIRCULAR_DEPENDENCY_DETECTED",
            _ => "VALIDATION_ERROR",
        };
        let mut out = split_error(&ws_name, tree_id, link_id, code, e.to_string());
        out.graph_health.valid_dag = false;
        return out;
    }

    tree.edges = new_edges;

    if let Err(e) = storage.save_tree(&tree) {
        let _ = storage.release_lock();
        return split_error(&ws_name, tree_id, link_id, "IO_ERROR", e.to_string());
    }

    let _ = storage.release_lock();

    if let Some(w) = stale_lock_warning(&lock_outcome) {
        warnings.insert(0, w);
    }

    CommandOutput {
        success: true,
        action: action.to_string(),
        workspace: ws_name,
        data: LinkSplitData {
            extracted_link: new_id,
            original_link: link_id.to_string(),
            tree_id: tree_id.to_string(),
        },
        graph_health: GraphHealth {
            valid_dag: true,
            orphan_nodes_count: 0,
        },
        errors: vec![],
        warnings,
    }
}

/// Builds a failed `link reoperator` output with empty data, sharing the
/// boilerplate common to every early-exit branch below.
fn reoperator_error(
    ws_name: &str,
    tree_id: &str,
    link_id: &str,
    code: &str,
    detail: impl Into<String>,
) -> CommandOutput<LinkReoperatorData> {
    CommandOutput {
        success: false,
        action: "link_reoperator".to_string(),
        workspace: ws_name.to_string(),
        data: LinkReoperatorData {
            link_id: link_id.to_string(),
            old_operator: Operator::Single,
            new_operator: Operator::Single,
            tree_id: tree_id.to_string(),
        },
        graph_health: GraphHealth {
            valid_dag: true,
            orphan_nodes_count: 0,
        },
        errors: vec![OutputError::new(code, detail)],
        warnings: vec![],
    }
}

/// Execute `link reoperator`.
///
/// Changes the operator of an existing edge in place, per ENGINE_SPEC.md
/// §2.7. The transition is constrained by `from[]` cardinality rather than
/// by the specific old/new operator pair:
///
/// - Switching *to* SINGLE requires `from.len() == 1`; a grouped edge must
///   be `link dissolve`d first (`CANNOT_REOPERATE_TO_SINGLE`).
/// - Switching *to* AND/OR/MAG/XOR requires `from.len() > 1`; a lone-cause
///   edge has nothing to combine (`CANNOT_REOPERATE_SINGLE_CAUSE`).
///
/// Weight handling follows the edge's relationship with MAG:
/// - Moving *to* MAG without an existing `weight` emits a
///   `MAG_WEIGHT_MISSING` warning (the estimate is left for a later step).
/// - Moving *away* from MAG silently discards `weight`, since it is only
///   meaningful for magnitude-weighted causes.
///
/// Followed by a DAG check for consistency with every other
/// topology-mutating command, though changing only `operator`/`weight`
/// while leaving `from`/`to` untouched can never introduce a cycle.
pub fn execute_link_reoperator(
    storage: &dyn Storage,
    tree_id: &str,
    link_id: &str,
    new_operator: &str,
) -> CommandOutput<LinkReoperatorData> {
    let ws_name = storage.workspace_name().unwrap_or_default();
    let action = "link_reoperator";

    let new_op = match parse_operator(new_operator) {
        Some(o) => o,
        None => {
            return reoperator_error(
                &ws_name,
                tree_id,
                link_id,
                "INVALID_OPERATOR",
                format!("Unknown operator: {}", new_operator),
            );
        }
    };

    let lock_outcome = match storage.acquire_lock("link reoperator") {
        Ok(o) => o,
        Err(e) => return reoperator_error(&ws_name, tree_id, link_id, "LOCK_ERROR", e.to_string()),
    };

    let mut tree = match storage.load_tree(tree_id) {
        Ok(t) => t,
        Err(e) => {
            let _ = storage.release_lock();
            return reoperator_error(&ws_name, tree_id, link_id, "TREE_NOT_FOUND", e.to_string());
        }
    };

    let edge_idx = match tree.edges.iter().position(|e| e.id == link_id) {
        Some(i) => i,
        None => {
            let _ = storage.release_lock();
            return reoperator_error(
                &ws_name,
                tree_id,
                link_id,
                "LINK_NOT_FOUND",
                format!("Edge '{}' not found in tree '{}'", link_id, tree_id),
            );
        }
    };

    let old_operator = tree.edges[edge_idx].operator;
    let from_len = tree.edges[edge_idx].from.len();

    if new_op == Operator::Single && from_len != 1 {
        let _ = storage.release_lock();
        return reoperator_error(
            &ws_name,
            tree_id,
            link_id,
            "CANNOT_REOPERATE_TO_SINGLE",
            format!(
                "Edge '{}' has {} causes; dissolve it before switching to SINGLE",
                link_id, from_len
            ),
        );
    }
    if new_op != Operator::Single && from_len <= 1 {
        let _ = storage.release_lock();
        return reoperator_error(
            &ws_name,
            tree_id,
            link_id,
            "CANNOT_REOPERATE_SINGLE_CAUSE",
            format!(
                "Edge '{}' has a single cause; add more causes before switching to {:?}",
                link_id, new_op
            ),
        );
    }

    let mut warnings: Vec<OutputWarning> = vec![];
    let new_weight = if new_op == Operator::Mag {
        let existing = tree.edges[edge_idx].weight;
        if existing.is_none() {
            warnings.push(OutputWarning::new(
                "MAG_WEIGHT_MISSING",
                "Operator MAG without --weight; magnitude estimation pending",
            ));
        }
        existing
    } else if old_operator == Operator::Mag {
        None
    } else {
        tree.edges[edge_idx].weight
    };

    tree.edges[edge_idx].operator = new_op;
    tree.edges[edge_idx].weight = new_weight;

    if let Err(e) = check_dag(&tree.edges, tree_id) {
        let _ = storage.release_lock();
        let code = match &e {
            LtpError::CircularDependencyDetected { .. } => "CIRCULAR_DEPENDENCY_DETECTED",
            _ => "VALIDATION_ERROR",
        };
        let mut out = reoperator_error(&ws_name, tree_id, link_id, code, e.to_string());
        out.graph_health.valid_dag = false;
        return out;
    }

    if let Err(e) = storage.save_tree(&tree) {
        let _ = storage.release_lock();
        return reoperator_error(&ws_name, tree_id, link_id, "IO_ERROR", e.to_string());
    }

    let _ = storage.release_lock();

    if let Some(w) = stale_lock_warning(&lock_outcome) {
        warnings.insert(0, w);
    }

    CommandOutput {
        success: true,
        action: action.to_string(),
        workspace: ws_name,
        data: LinkReoperatorData {
            link_id: link_id.to_string(),
            old_operator,
            new_operator: new_op,
            tree_id: tree_id.to_string(),
        },
        graph_health: GraphHealth {
            valid_dag: true,
            orphan_nodes_count: 0,
        },
        errors: vec![],
        warnings,
    }
}
