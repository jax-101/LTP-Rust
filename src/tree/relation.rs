//! `tree relation list`: the meta-graph inferred from node refs (RFC-002 S1, ADR-015).

use serde::Serialize;

use crate::meta::{infer_relations, InferredRelation};
use crate::output::{CommandOutput, GraphHealth, OutputError};
use crate::storage::Storage;

/// Data returned by `tree relation list`.
#[derive(Debug, Serialize)]
pub struct TreeRelationListData {
    /// Inferred relations, sorted by `(referencing, referenced)`.
    pub relations: Vec<InferredRelation>,
    /// Number of relations.
    pub count: usize,
}

/// Execute `tree relation list`: computes relations between trees (and NBR
/// branches) from node refs. Read-only; nothing is persisted.
///
/// With `tree_filter`, keeps only relations with either endpoint in that tree
/// (`TREE_NOT_FOUND` if it does not exist). Unreadable nodes and trees are skipped.
pub fn execute_tree_relation_list(
    storage: &dyn Storage,
    tree_filter: Option<&str>,
) -> CommandOutput<TreeRelationListData> {
    let ws_name = storage.workspace_name().unwrap_or_default();
    let action = "tree_relation_list";

    if let Some(tree_id) = tree_filter {
        if storage.load_tree(tree_id).is_err() {
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: TreeRelationListData {
                    relations: vec![],
                    count: 0,
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
    }

    let trees: Vec<_> = storage
        .list_tree_ids()
        .unwrap_or_default()
        .iter()
        .filter_map(|id| storage.load_tree(id).ok())
        .collect();
    let nodes: Vec<_> = storage
        .list_node_ids()
        .unwrap_or_default()
        .iter()
        .filter_map(|id| storage.load_node(id).ok())
        .collect();

    let relations: Vec<InferredRelation> = infer_relations(&nodes, &trees)
        .into_iter()
        .filter(|r| tree_filter.is_none_or(|t| r.referencing.tree == t || r.referenced.tree == t))
        .collect();
    let count = relations.len();
    CommandOutput::ok(action, &ws_name, TreeRelationListData { relations, count })
}
