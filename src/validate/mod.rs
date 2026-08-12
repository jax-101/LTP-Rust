pub mod clr;
pub mod dag;
pub mod ec;
pub mod integrity;
pub mod orphans;

pub use dag::check_dag;

use std::collections::{HashMap, HashSet};

use serde::Serialize;
use tracing::{debug, info};

use crate::output::{CommandOutput, GraphHealth, OutputError, OutputWarning};
use crate::storage::Storage;
use crate::tree::types::TreeType;

/// Per-tree validation results.
#[derive(Debug, Serialize)]
pub struct TreeValidation {
    pub tree_id: String,
    pub errors: Vec<OutputError>,
    pub warnings: Vec<OutputWarning>,
}

/// Top-level validate output data.
#[derive(Debug, Serialize)]
pub struct ValidateData {
    pub trees_validated: usize,
    pub total_errors: usize,
    pub total_warnings: usize,
    pub details: Vec<TreeValidation>,
}

/// Execute full validation on the workspace (or a single tree if specified).
pub fn execute_validate<S: Storage>(
    storage: &S,
    tree_filter: Option<&str>,
) -> CommandOutput<ValidateData> {
    let ws_name = storage.workspace_name().unwrap_or_default();

    let tree_ids = match tree_filter {
        Some(id) => vec![id.to_string()],
        None => match storage.list_tree_ids() {
            Ok(ids) => ids,
            Err(e) => {
                return CommandOutput {
                    success: false,
                    action: "validate".to_string(),
                    workspace: ws_name,
                    data: ValidateData {
                        trees_validated: 0,
                        total_errors: 1,
                        total_warnings: 0,
                        details: vec![],
                    },
                    graph_health: GraphHealth {
                        valid_dag: true,
                        orphan_nodes_count: 0,
                    },
                    errors: vec![OutputError::new("IO_ERROR", e.to_string())],
                    warnings: vec![],
                };
            }
        },
    };

    let node_pool: HashSet<String> = storage
        .list_node_ids()
        .unwrap_or_default()
        .into_iter()
        .collect();

    info!(tree_count = tree_ids.len(), "starting validation");

    let mut details = Vec::new();
    let mut all_valid_dag = true;
    let mut total_orphans = 0usize;

    for tree_id in &tree_ids {
        let tree = match storage.load_tree(tree_id) {
            Ok(t) => t,
            Err(e) => {
                details.push(TreeValidation {
                    tree_id: tree_id.clone(),
                    errors: vec![OutputError::new("TREE_LOAD_ERROR", e.to_string())],
                    warnings: vec![],
                });
                continue;
            }
        };

        debug!(tree_id = %tree.id, "validating tree");

        let mut tree_errors: Vec<OutputError> = Vec::new();
        let mut tree_warnings: Vec<OutputWarning> = Vec::new();

        // DAG check on main edges
        if check_dag(&tree.edges, &tree.id).is_err() {
            all_valid_dag = false;
            tree_errors.push(
                OutputError::new(
                    "CIRCULAR_DEPENDENCY_DETECTED",
                    format!("Cycle detected in tree '{}'", tree.id),
                )
                .with_context("tree_id", serde_json::Value::String(tree.id.clone())),
            );
        }

        // DAG check on each NBR branch
        for nbr in &tree.nbr_branches {
            if check_dag(&nbr.edges, &tree.id).is_err() {
                all_valid_dag = false;
                tree_errors.push(
                    OutputError::new(
                        "CIRCULAR_DEPENDENCY_DETECTED",
                        format!("Cycle detected in NBR '{}' of tree '{}'", nbr.id, tree.id),
                    )
                    .with_context("tree_id", serde_json::Value::String(tree.id.clone()))
                    .with_context("nbr_id", serde_json::Value::String(nbr.id.clone())),
                );
            }
        }

        // Referential integrity
        let integrity_errors = integrity::check_integrity(&tree.edges, &node_pool, &tree.id);
        tree_errors.extend(integrity_errors);

        // EC-specific rules
        if tree.tree_type == TreeType::Ec {
            let ec_errors = ec::check_ec_rules(&tree.nodes, &tree.edges, &tree.id);
            tree_errors.extend(ec_errors);
        }

        // Load nodes referenced in this tree for CLR checks
        let tree_node_ids: Vec<&str> = tree.nodes.iter().map(|n| n.node_ref.as_str()).collect();
        let mut node_map: HashMap<String, crate::node::Node> = HashMap::new();
        let mut nodes_for_clr2 = Vec::new();

        for nid in &tree_node_ids {
            if let Ok(node) = storage.load_node(nid) {
                nodes_for_clr2.push(node.clone());
                node_map.insert(node.id.clone(), node);
            }
        }

        // CLR#2: Conjunctions
        tree_warnings.extend(clr::lint_clr2(&nodes_for_clr2));

        // CLR#4: Insufficiency
        tree_warnings.extend(clr::lint_clr4_insufficiency(&tree.edges));

        // CLR#4/#5: Excessive AND inputs
        tree_warnings.extend(clr::lint_clr4_5_excessive_and(&tree.edges));

        // CLR#6: Type inversion
        tree_warnings.extend(clr::lint_clr6_type_inversion(&tree.edges, &node_map));

        // CLR#7: Intangible without predicted effect
        tree_warnings.extend(clr::lint_clr7_intangible(&tree.edges, &node_map));

        // Orphan nodes in tree
        let orphan_warnings = orphans::check_orphans(&tree.nodes, &tree.edges, &tree.id);
        total_orphans += orphan_warnings.len();
        tree_warnings.extend(orphan_warnings);

        details.push(TreeValidation {
            tree_id: tree.id,
            errors: tree_errors,
            warnings: tree_warnings,
        });
    }

    let total_errors: usize = details.iter().map(|d| d.errors.len()).sum();
    let total_warnings: usize = details.iter().map(|d| d.warnings.len()).sum();
    let success = total_errors == 0;

    info!(
        trees = details.len(),
        errors = total_errors,
        warnings = total_warnings,
        "validation complete"
    );

    CommandOutput {
        success,
        action: "validate".to_string(),
        workspace: ws_name,
        data: ValidateData {
            trees_validated: details.len(),
            total_errors,
            total_warnings,
            details,
        },
        graph_health: GraphHealth {
            valid_dag: all_valid_dag,
            orphan_nodes_count: total_orphans,
        },
        errors: vec![],
        warnings: vec![],
    }
}
