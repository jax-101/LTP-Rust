use crate::storage::Storage;

/// Information about a resolved target entity in the graph.
#[derive(Debug, Clone)]
pub struct ResolvedTarget {
    pub id: String,
    pub label: Option<String>,
    pub target_type: String,
}

/// Attempts to resolve a target ID against the workspace graph.
///
/// Resolution order:
/// - MACRO-XXX: always NOT FOUND (macro_edges are not standalone entities)
/// - LINK-XXX: search all trees' edges (trunk + nbr_branches)
/// - FB-XXX: search all trees' feedback_edges
/// - ASM-XXX: search all trees' edges' assumptions (trunk + nbr_branches)
/// - Anything else: treated as a node ID, checked in node pool
pub fn resolve_target(storage: &dyn Storage, target: &str) -> Option<ResolvedTarget> {
    if target.starts_with("MACRO-") {
        return None;
    }

    if target.starts_with("LINK-") {
        return resolve_edge(storage, target);
    }

    if target.starts_with("FB-") {
        return resolve_feedback_edge(storage, target);
    }

    if target.starts_with("ASM-") {
        return resolve_assumption(storage, target);
    }

    resolve_node(storage, target)
}

fn resolve_node(storage: &dyn Storage, id: &str) -> Option<ResolvedTarget> {
    match storage.load_node(id) {
        Ok(node) => Some(ResolvedTarget {
            id: id.to_string(),
            label: Some(node.label),
            target_type: "node".to_string(),
        }),
        Err(_) => None,
    }
}

fn resolve_edge(storage: &dyn Storage, id: &str) -> Option<ResolvedTarget> {
    let tree_ids = storage.list_tree_ids().unwrap_or_default();

    for tree_id in &tree_ids {
        let tree = match storage.load_tree(tree_id) {
            Ok(t) => t,
            Err(_) => continue,
        };

        for edge in &tree.edges {
            if edge.id == id {
                let label = format!("{} -> {}", edge.from.join("+"), edge.to);
                return Some(ResolvedTarget {
                    id: id.to_string(),
                    label: Some(label),
                    target_type: "edge".to_string(),
                });
            }
        }

        for branch in &tree.nbr_branches {
            for edge in &branch.edges {
                if edge.id == id {
                    let label = format!("{} -> {}", edge.from.join("+"), edge.to);
                    return Some(ResolvedTarget {
                        id: id.to_string(),
                        label: Some(label),
                        target_type: "edge".to_string(),
                    });
                }
            }
        }
    }

    None
}

fn resolve_feedback_edge(storage: &dyn Storage, id: &str) -> Option<ResolvedTarget> {
    let tree_ids = storage.list_tree_ids().unwrap_or_default();

    for tree_id in &tree_ids {
        let tree = match storage.load_tree(tree_id) {
            Ok(t) => t,
            Err(_) => continue,
        };

        for fb in &tree.feedback_edges {
            if fb.id == id {
                let label = fb
                    .label
                    .clone()
                    .unwrap_or_else(|| format!("{} -> {}", fb.from, fb.to));
                return Some(ResolvedTarget {
                    id: id.to_string(),
                    label: Some(label),
                    target_type: "feedback_edge".to_string(),
                });
            }
        }
    }

    None
}

fn resolve_assumption(storage: &dyn Storage, id: &str) -> Option<ResolvedTarget> {
    let tree_ids = storage.list_tree_ids().unwrap_or_default();

    for tree_id in &tree_ids {
        let tree = match storage.load_tree(tree_id) {
            Ok(t) => t,
            Err(_) => continue,
        };

        for edge in &tree.edges {
            for asm in &edge.assumptions {
                if asm.id == id {
                    return Some(ResolvedTarget {
                        id: id.to_string(),
                        label: Some(asm.text.clone()),
                        target_type: "assumption".to_string(),
                    });
                }
            }
        }

        for branch in &tree.nbr_branches {
            for edge in &branch.edges {
                for asm in &edge.assumptions {
                    if asm.id == id {
                        return Some(ResolvedTarget {
                            id: id.to_string(),
                            label: Some(asm.text.clone()),
                            target_type: "assumption".to_string(),
                        });
                    }
                }
            }
        }
    }

    None
}

/// Checks whether a target ID exists in the graph without resolving label.
/// More efficient when label is not needed.
pub fn target_exists(storage: &dyn Storage, target: &str) -> bool {
    resolve_target(storage, target).is_some()
}
