pub mod clr;
pub mod dag;
pub mod ec;
pub mod knowledge;
pub mod macro_edge;
pub mod orphans;

pub use dag::check_dag;

use std::collections::{BTreeMap, BTreeSet, HashSet};

use serde::Serialize;
use tracing::{debug, info};

use crate::meta;
use crate::output::{CommandOutput, GraphHealth, OutputError, OutputWarning};
use crate::storage::Storage;
use crate::tree::types::{MacroEdgeStatus, TreeLogic, TreeType};

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
        if let Err(crate::errors::LtpError::CircularDependencyDetected { cycle_path, .. }) =
            check_dag(&tree.edges, &tree.id)
        {
            all_valid_dag = false;
            tree_errors.push(
                OutputError::new(
                    "CIRCULAR_DEPENDENCY_DETECTED",
                    format!(
                        "Cycle detected in tree '{}': {}",
                        tree.id,
                        cycle_path.join(" -> ")
                    ),
                )
                .with_context("tree_id", serde_json::Value::String(tree.id.clone()))
                .with_context(
                    "cycle_path",
                    serde_json::Value::Array(
                        cycle_path
                            .iter()
                            .map(|n| serde_json::Value::String(n.clone()))
                            .collect(),
                    ),
                ),
            );
        }

        // DAG check on each NBR branch
        for nbr in &tree.nbr_branches {
            if let Err(crate::errors::LtpError::CircularDependencyDetected { cycle_path, .. }) =
                check_dag(&nbr.edges, &tree.id)
            {
                all_valid_dag = false;
                tree_errors.push(
                    OutputError::new(
                        "CIRCULAR_DEPENDENCY_DETECTED",
                        format!(
                            "Cycle detected in NBR '{}' of tree '{}': {}",
                            nbr.id,
                            tree.id,
                            cycle_path.join(" -> ")
                        ),
                    )
                    .with_context("tree_id", serde_json::Value::String(tree.id.clone()))
                    .with_context("nbr_id", serde_json::Value::String(nbr.id.clone()))
                    .with_context(
                        "cycle_path",
                        serde_json::Value::Array(
                            cycle_path
                                .iter()
                                .map(|n| serde_json::Value::String(n.clone()))
                                .collect(),
                        ),
                    ),
                );
            }
        }

        // Referential integrity in every structure: nodes, edges, feedback, NBR, macros (ADR-016 D-5)
        tree_errors.extend(meta::integrity::check_tree_integrity(&tree, &node_pool));

        // EC-specific rules
        if tree.tree_type == TreeType::Ec {
            let ec_errors = ec::check_ec_rules(&tree.nodes, &tree.edges, &tree.id);
            tree_errors.extend(ec_errors);
        }

        // Load nodes referenced in this tree for CLR checks
        let tree_node_ids: Vec<&str> = tree.nodes.iter().map(|n| n.node_ref.as_str()).collect();
        let mut node_map: BTreeMap<String, crate::node::Node> = BTreeMap::new();
        let mut nodes_for_clr2 = Vec::new();

        for nid in &tree_node_ids {
            if let Ok(node) = storage.load_node(nid) {
                nodes_for_clr2.push(node.clone());
                node_map.insert(node.id.clone(), node);
            }
        }

        // CLR#2: Conjunctions
        tree_warnings.extend(clr::lint_clr2(&nodes_for_clr2));

        // CLR#4 and CLR#4/#5 only apply to sufficiency trees (CRT/FRT/TT). In necessity
        // trees (GT/EC/PRT) each necessary condition is insufficient on its own by
        // construction, so these lints would be pure noise (CLR_SPEC §1.2, ADR-014).
        if tree.logic == TreeLogic::Sufficiency {
            // CLR#4: Insufficiency
            tree_warnings.extend(clr::lint_clr4_insufficiency(&tree.edges));

            // CLR#4/#5: Implicit OR (multiple ungrouped SINGLE edges to same node)
            tree_warnings.extend(clr::lint_clr4_5_implicit_or(&tree.edges));

            // CLR#4/#5: Excessive AND inputs
            tree_warnings.extend(clr::lint_clr4_5_excessive_and(&tree.edges));
        }

        // CLR#6: Type inversion
        tree_warnings.extend(clr::lint_clr6_type_inversion(&tree.edges, &node_map));

        // CLR#7: Intangible without predicted effect
        tree_warnings.extend(clr::lint_clr7_intangible(&tree.edges, &node_map));

        // CLR#5: MAG weights normalization
        tree_warnings.extend(clr::lint_clr5_mag_weights(&tree.edges));

        // Orphan nodes in tree. D6: los extremos de reservas (long arrows Reservation) están
        // conectados lógicamente aunque aún no tengan edge real, así que se siembran para no
        // reportarlos como huérfanos (los Overlay ya están conectados por su interior real).
        let reserved_endpoints: Vec<&str> = tree
            .macro_edges
            .iter()
            .filter(|m| m.status == MacroEdgeStatus::Reservation)
            .flat_map(|m| [m.from.as_str(), m.to.as_str()])
            .collect();
        let orphan_warnings =
            orphans::check_orphans(&tree.nodes, &tree.edges, &reserved_endpoints, &tree.id);
        total_orphans += orphan_warnings.len();
        tree_warnings.extend(orphan_warnings);

        // Long arrow (macro_edge) summary hygiene — non-blocking, never affects valid_dag
        tree_warnings.extend(macro_edge::check_macro_edges(&tree));

        details.push(TreeValidation {
            tree_id: tree.id,
            errors: tree_errors,
            warnings: tree_warnings,
        });
    }

    // Knowledge pool validation
    let knowledge_node_filter: Option<HashSet<String>> = if tree_filter.is_some() {
        // When validating a specific tree, only check nodes in that tree
        let filter: HashSet<String> = details
            .iter()
            .flat_map(|d| {
                storage
                    .load_tree(&d.tree_id)
                    .map(|t| {
                        t.nodes
                            .iter()
                            .map(|n| n.node_ref.clone())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            })
            .collect();
        Some(filter)
    } else {
        None
    };

    let knowledge_warnings = knowledge::validate_knowledge(storage, knowledge_node_filter.as_ref());

    // Add knowledge warnings to a synthetic "knowledge_pool" validation entry
    if !knowledge_warnings.is_empty() {
        details.push(TreeValidation {
            tree_id: "_knowledge_pool".to_string(),
            errors: vec![],
            warnings: knowledge_warnings,
        });
    }

    // Meta-graph validation (RFC-002 S1): ref integrity and norm coverage
    let meta_warnings = validate_meta_graph(storage, tree_filter);
    if !meta_warnings.is_empty() {
        details.push(TreeValidation {
            tree_id: "_meta_graph".to_string(),
            errors: vec![],
            warnings: meta_warnings,
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

/// Workspace-wide ref checks (ADR-015) for the synthetic `_meta_graph` entry.
///
/// Loads every readable tree and node. Nodes listed on disk that fail to load
/// yield `NODE_UNREADABLE {node_id}` (they used to be skipped silently), followed
/// by `meta::check_refs`. With `tree_filter`, only nodes present in that tree
/// (trunk or NBR branch) are checked.
fn validate_meta_graph<S: Storage>(storage: &S, tree_filter: Option<&str>) -> Vec<OutputWarning> {
    let trees: Vec<_> = storage
        .list_tree_ids()
        .unwrap_or_default()
        .iter()
        .filter_map(|id| storage.load_tree(id).ok())
        .collect();
    let scope: Option<BTreeSet<String>> = tree_filter.map(|id| {
        meta::tree_memberships(&trees)
            .into_iter()
            .filter(|(_, ends)| ends.iter().any(|e| e.tree == id))
            .map(|(node, _)| node)
            .collect()
    });
    let in_scope = |id: &str| scope.as_ref().is_none_or(|s| s.contains(id));

    let mut warnings = Vec::new();
    let mut nodes = Vec::new();
    for id in storage.list_node_ids().unwrap_or_default() {
        match storage.load_node(&id) {
            Ok(node) => nodes.push(node),
            Err(_) if in_scope(&id) => warnings.push(
                OutputWarning::new("NODE_UNREADABLE", format!("Node '{id}' cannot be loaded"))
                    .with_context("node_id", serde_json::Value::String(id)),
            ),
            Err(_) => {}
        }
    }
    warnings.extend(meta::check_refs(&nodes, &trees, scope.as_ref()));
    warnings
}
