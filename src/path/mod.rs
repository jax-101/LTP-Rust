use std::collections::{BTreeSet, VecDeque};

use serde::Serialize;

use crate::link::types::{Edge, EdgeStatus, Logic, Operator};
use crate::node::types::{EpistemicStatus, Node, NodeMetadata, NodeStatus, NodeType};
use crate::output::{CommandOutput, GraphHealth, OutputError};
use crate::storage::Storage;
use crate::tree::types::{MacroEdge, NodeRef};

// --- Output types ---

/// Output data for `ltp path collapse`.
#[derive(Debug, Clone, Serialize)]
pub struct CollapseData {
    pub macro_edge_id: String,
    pub from: String,
    pub to: String,
    pub label: String,
    pub interior_nodes: Vec<String>,
    pub interior_links: Vec<String>,
}

/// Output data for `ltp path explode`.
#[derive(Debug, Clone, Serialize)]
pub struct ExplodeData {
    pub created_node_id: String,
    pub created_links: Vec<String>,
    pub removed_assumption: String,
    pub original_link_removed: String,
}

/// Output data for `ltp path replace`.
#[derive(Debug, Clone, Serialize)]
pub struct ReplaceData {
    pub macro_link: String,
    pub by_node: String,
    pub superseded_links: Vec<String>,
    pub superseded_nodes: Vec<String>,
    pub new_links: Vec<String>,
}

// --- Execute functions ---

/// Collapse a sub-graph between two nodes into a macro_edge.
pub fn execute_path_collapse(
    storage: &dyn Storage,
    tree_id: &str,
    from: &str,
    to: &str,
    label: &str,
) -> CommandOutput<CollapseData> {
    let ws_name = storage.workspace_name().unwrap_or_default();
    let action = "path_collapse";

    let lock_outcome = match storage.acquire_lock("path collapse") {
        Ok(o) => o,
        Err(e) => {
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: CollapseData {
                    macro_edge_id: String::new(),
                    from: from.to_string(),
                    to: to.to_string(),
                    label: label.to_string(),
                    interior_nodes: vec![],
                    interior_links: vec![],
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
    if let crate::storage::LockOutcome::StaleLockRemoved { pid } = lock_outcome {
        warnings.push(crate::output::OutputWarning::new(
            "STALE_LOCK_REMOVED",
            format!("Removed stale lock from PID {}", pid),
        ));
    }

    let mut tree = match storage.load_tree(tree_id) {
        Ok(t) => t,
        Err(_) => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: CollapseData {
                    macro_edge_id: String::new(),
                    from: from.to_string(),
                    to: to.to_string(),
                    label: label.to_string(),
                    interior_nodes: vec![],
                    interior_links: vec![],
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

    // Verify from and to are attached
    let from_attached = tree.nodes.iter().any(|nr| nr.node_ref == from);
    let to_attached = tree.nodes.iter().any(|nr| nr.node_ref == to);

    if !from_attached {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: CollapseData {
                macro_edge_id: String::new(),
                from: from.to_string(),
                to: to.to_string(),
                label: label.to_string(),
                interior_nodes: vec![],
                interior_links: vec![],
            },
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![OutputError::new(
                "NODE_NOT_IN_TREE",
                format!("Node '{}' is not attached to tree '{}'", from, tree_id),
            )],
            warnings: vec![],
        };
    }

    if !to_attached {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: CollapseData {
                macro_edge_id: String::new(),
                from: from.to_string(),
                to: to.to_string(),
                label: label.to_string(),
                interior_nodes: vec![],
                interior_links: vec![],
            },
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![OutputError::new(
                "NODE_NOT_IN_TREE",
                format!("Node '{}' is not attached to tree '{}'", to, tree_id),
            )],
            warnings: vec![],
        };
    }

    // BFS forward from `from` to find all nodes reachable that lead to `to`
    // Step 1: find all nodes reachable from `from` (downstream)
    let mut reachable_from_start: BTreeSet<String> = BTreeSet::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    queue.push_back(from.to_string());
    reachable_from_start.insert(from.to_string());

    while let Some(current) = queue.pop_front() {
        for edge in &tree.edges {
            if edge.from.contains(&current) && !reachable_from_start.contains(&edge.to) {
                reachable_from_start.insert(edge.to.clone());
                queue.push_back(edge.to.clone());
            }
        }
    }

    if !reachable_from_start.contains(to) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: CollapseData {
                macro_edge_id: String::new(),
                from: from.to_string(),
                to: to.to_string(),
                label: label.to_string(),
                interior_nodes: vec![],
                interior_links: vec![],
            },
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![OutputError::new(
                "NO_DIRECTED_PATH",
                format!("No directed path from '{}' to '{}'", from, to),
            )],
            warnings: vec![],
        };
    }

    // Step 2: BFS backward from `to` to find nodes that can reach `to`
    let mut reachable_to_end: BTreeSet<String> = BTreeSet::new();
    let mut queue_back: VecDeque<String> = VecDeque::new();
    queue_back.push_back(to.to_string());
    reachable_to_end.insert(to.to_string());

    while let Some(current) = queue_back.pop_front() {
        for edge in &tree.edges {
            if edge.to == current {
                for f in &edge.from {
                    if !reachable_to_end.contains(f) {
                        reachable_to_end.insert(f.clone());
                        queue_back.push_back(f.clone());
                    }
                }
            }
        }
    }

    // Sub-graph = intersection of forward-reachable from `from` AND backward-reachable from `to`
    let subgraph_nodes: BTreeSet<String> = reachable_from_start
        .intersection(&reachable_to_end)
        .cloned()
        .collect();

    // Interior nodes = subgraph - {from, to}
    let interior_nodes: Vec<String> = subgraph_nodes
        .iter()
        .filter(|n| *n != from && *n != to)
        .cloned()
        .collect();

    // Interior links = edges whose from[] nodes AND to are all within the subgraph
    let interior_links: Vec<String> = tree
        .edges
        .iter()
        .filter(|e| {
            e.from.iter().all(|f| subgraph_nodes.contains(f)) && subgraph_nodes.contains(&e.to)
        })
        .map(|e| e.id.clone())
        .collect();

    // Check for nested macro_edges
    let existing_macro_links: BTreeSet<String> = tree
        .macro_edges
        .iter()
        .flat_map(|me| me.interior_links.iter().cloned())
        .collect();

    let has_nested = interior_links
        .iter()
        .any(|il| existing_macro_links.contains(il));

    if has_nested {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: CollapseData {
                macro_edge_id: String::new(),
                from: from.to_string(),
                to: to.to_string(),
                label: label.to_string(),
                interior_nodes: vec![],
                interior_links: vec![],
            },
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![OutputError::new(
                "NESTED_MACRO_NOT_ALLOWED",
                "Sub-graph already contains a macro_edge",
            )],
            warnings: vec![],
        };
    }

    // Generate macro_edge ID
    let macro_id = match storage.next_id("MACRO") {
        Ok(id) => id,
        Err(e) => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: CollapseData {
                    macro_edge_id: String::new(),
                    from: from.to_string(),
                    to: to.to_string(),
                    label: label.to_string(),
                    interior_nodes: vec![],
                    interior_links: vec![],
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

    let macro_edge = MacroEdge {
        id: macro_id.clone(),
        from: from.to_string(),
        to: to.to_string(),
        label: label.to_string(),
        interior_nodes: interior_nodes.clone(),
        interior_links: interior_links.clone(),
        status: "active".to_string(),
    };

    tree.macro_edges.push(macro_edge);

    if let Err(e) = storage.save_tree(&tree) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: CollapseData {
                macro_edge_id: macro_id,
                from: from.to_string(),
                to: to.to_string(),
                label: label.to_string(),
                interior_nodes,
                interior_links,
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
        CollapseData {
            macro_edge_id: macro_id,
            from: from.to_string(),
            to: to.to_string(),
            label: label.to_string(),
            interior_nodes,
            interior_links,
        },
    );
    output.warnings = warnings;
    output
}

/// Explode an assumption into an intermediate node, splitting the edge in two.
pub fn execute_path_explode(
    storage: &dyn Storage,
    tree_id: &str,
    link_id: &str,
    asm_id: &str,
    label: &str,
) -> CommandOutput<ExplodeData> {
    let ws_name = storage.workspace_name().unwrap_or_default();
    let action = "path_explode";

    let lock_outcome = match storage.acquire_lock("path explode") {
        Ok(o) => o,
        Err(e) => {
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: ExplodeData {
                    created_node_id: String::new(),
                    created_links: vec![],
                    removed_assumption: asm_id.to_string(),
                    original_link_removed: String::new(),
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
    if let crate::storage::LockOutcome::StaleLockRemoved { pid } = lock_outcome {
        warnings.push(crate::output::OutputWarning::new(
            "STALE_LOCK_REMOVED",
            format!("Removed stale lock from PID {}", pid),
        ));
    }

    let mut tree = match storage.load_tree(tree_id) {
        Ok(t) => t,
        Err(_) => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: ExplodeData {
                    created_node_id: String::new(),
                    created_links: vec![],
                    removed_assumption: asm_id.to_string(),
                    original_link_removed: String::new(),
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

    // Find the edge
    let edge_idx = tree.edges.iter().position(|e| e.id == link_id);
    let edge_idx = match edge_idx {
        Some(i) => i,
        None => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: ExplodeData {
                    created_node_id: String::new(),
                    created_links: vec![],
                    removed_assumption: asm_id.to_string(),
                    original_link_removed: String::new(),
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new(
                    "LINK_NOT_FOUND",
                    format!("Link '{}' not found in tree '{}'", link_id, tree_id),
                )],
                warnings: vec![],
            };
        }
    };

    // Verify assumption exists in this edge
    let asm_exists = tree.edges[edge_idx]
        .assumptions
        .iter()
        .any(|a| a.id == asm_id);

    if !asm_exists {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: ExplodeData {
                created_node_id: String::new(),
                created_links: vec![],
                removed_assumption: asm_id.to_string(),
                original_link_removed: String::new(),
            },
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![OutputError::new(
                "ASSUMPTION_NOT_IN_LINK",
                format!("Assumption '{}' not found in link '{}'", asm_id, link_id),
            )],
            warnings: vec![],
        };
    }

    // Create INT node
    let int_id = match storage.next_id("INT") {
        Ok(id) => id,
        Err(e) => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: ExplodeData {
                    created_node_id: String::new(),
                    created_links: vec![],
                    removed_assumption: asm_id.to_string(),
                    original_link_removed: String::new(),
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

    let int_node = Node {
        id: int_id.clone(),
        node_type: NodeType::Int,
        label: label.to_string(),
        tags: vec![],
        observable: true,
        epistemic: EpistemicStatus::default(),
        metadata: NodeMetadata {
            status: NodeStatus::Active,
            extra: Default::default(),
        },
    };

    if let Err(e) = storage.save_node(&int_node) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: ExplodeData {
                created_node_id: String::new(),
                created_links: vec![],
                removed_assumption: asm_id.to_string(),
                original_link_removed: String::new(),
            },
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![OutputError::new("SAVE_ERROR", e.to_string())],
            warnings: vec![],
        };
    }

    // Generate 2 new link IDs
    let link_a_id = match storage.next_id("LINK") {
        Ok(id) => id,
        Err(e) => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: ExplodeData {
                    created_node_id: int_id,
                    created_links: vec![],
                    removed_assumption: asm_id.to_string(),
                    original_link_removed: String::new(),
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

    let link_b_id = match storage.next_id("LINK") {
        Ok(id) => id,
        Err(e) => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: ExplodeData {
                    created_node_id: int_id,
                    created_links: vec![link_a_id],
                    removed_assumption: asm_id.to_string(),
                    original_link_removed: String::new(),
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

    // Extract info from the original edge before removing it
    let original_from = tree.edges[edge_idx].from.clone();
    let original_to = tree.edges[edge_idx].to.clone();
    let original_logic = tree.edges[edge_idx].logic;

    // Create edge A: original.from → INT
    let edge_a = Edge {
        id: link_a_id.clone(),
        from: original_from,
        to: int_id.clone(),
        operator: Operator::Single,
        weight: None,
        status: EdgeStatus::Active,
        logic: original_logic,
        assumptions: vec![],
    };

    // Create edge B: INT → original.to
    let edge_b = Edge {
        id: link_b_id.clone(),
        from: vec![int_id.clone()],
        to: original_to,
        operator: Operator::Single,
        weight: None,
        status: EdgeStatus::Active,
        logic: original_logic,
        assumptions: vec![],
    };

    // Remove the assumption from the original edge, then remove the edge
    let original_link_id = tree.edges[edge_idx].id.clone();
    tree.edges.remove(edge_idx);

    // Add new edges
    tree.edges.push(edge_a);
    tree.edges.push(edge_b);

    // Attach INT node to the tree
    tree.nodes.push(NodeRef {
        node_ref: int_id.clone(),
        role: None,
    });

    if let Err(e) = storage.save_tree(&tree) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: ExplodeData {
                created_node_id: int_id,
                created_links: vec![link_a_id, link_b_id],
                removed_assumption: asm_id.to_string(),
                original_link_removed: original_link_id,
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
        ExplodeData {
            created_node_id: int_id,
            created_links: vec![link_a_id, link_b_id],
            removed_assumption: asm_id.to_string(),
            original_link_removed: original_link_id,
        },
    );
    output.warnings = warnings;
    output
}

/// Replace a macro_edge's sub-graph with a single node, marking interior as superseded.
pub fn execute_path_replace(
    storage: &dyn Storage,
    tree_id: &str,
    macro_link_id: &str,
    by_node_id: &str,
) -> CommandOutput<ReplaceData> {
    let ws_name = storage.workspace_name().unwrap_or_default();
    let action = "path_replace";

    let lock_outcome = match storage.acquire_lock("path replace") {
        Ok(o) => o,
        Err(e) => {
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: ReplaceData {
                    macro_link: macro_link_id.to_string(),
                    by_node: by_node_id.to_string(),
                    superseded_links: vec![],
                    superseded_nodes: vec![],
                    new_links: vec![],
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
    if let crate::storage::LockOutcome::StaleLockRemoved { pid } = lock_outcome {
        warnings.push(crate::output::OutputWarning::new(
            "STALE_LOCK_REMOVED",
            format!("Removed stale lock from PID {}", pid),
        ));
    }

    let mut tree = match storage.load_tree(tree_id) {
        Ok(t) => t,
        Err(_) => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: ReplaceData {
                    macro_link: macro_link_id.to_string(),
                    by_node: by_node_id.to_string(),
                    superseded_links: vec![],
                    superseded_nodes: vec![],
                    new_links: vec![],
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

    // Find the macro_edge
    let macro_idx = tree
        .macro_edges
        .iter()
        .position(|me| me.id == macro_link_id);

    let macro_idx = match macro_idx {
        Some(i) => i,
        None => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: ReplaceData {
                    macro_link: macro_link_id.to_string(),
                    by_node: by_node_id.to_string(),
                    superseded_links: vec![],
                    superseded_nodes: vec![],
                    new_links: vec![],
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new(
                    "MACRO_EDGE_NOT_FOUND",
                    format!(
                        "Macro edge '{}' not found in tree '{}'",
                        macro_link_id, tree_id
                    ),
                )],
                warnings: vec![],
            };
        }
    };

    // Verify by_node exists in pool
    if storage.load_node(by_node_id).is_err() {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: ReplaceData {
                macro_link: macro_link_id.to_string(),
                by_node: by_node_id.to_string(),
                superseded_links: vec![],
                superseded_nodes: vec![],
                new_links: vec![],
            },
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![OutputError::new(
                "NODE_NOT_FOUND",
                format!("Node '{}' not found in pool", by_node_id),
            )],
            warnings: vec![],
        };
    }

    let macro_from = tree.macro_edges[macro_idx].from.clone();
    let macro_to = tree.macro_edges[macro_idx].to.clone();
    let interior_links_set: BTreeSet<String> = tree.macro_edges[macro_idx]
        .interior_links
        .iter()
        .cloned()
        .collect();
    let interior_nodes_list = tree.macro_edges[macro_idx].interior_nodes.clone();

    // Mark interior links as superseded
    let mut superseded_links: Vec<String> = Vec::new();
    for edge in &mut tree.edges {
        if interior_links_set.contains(&edge.id) {
            edge.status = EdgeStatus::Superseded;
            superseded_links.push(edge.id.clone());
        }
    }

    // Mark interior nodes as superseded (update metadata.status in pool)
    let mut superseded_nodes: Vec<String> = Vec::new();
    for node_id in &interior_nodes_list {
        if let Ok(mut node) = storage.load_node(node_id) {
            node.metadata.status = NodeStatus::Superseded;
            if storage.save_node(&node).is_ok() {
                superseded_nodes.push(node_id.clone());
            }
        }
    }

    // Attach by_node to tree if not already
    let already_attached = tree.nodes.iter().any(|nr| nr.node_ref == by_node_id);
    if !already_attached {
        tree.nodes.push(NodeRef {
            node_ref: by_node_id.to_string(),
            role: None,
        });
    }

    // Create 2 new edges: from → by_node, by_node → to
    let link_a_id = match storage.next_id("LINK") {
        Ok(id) => id,
        Err(e) => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: ReplaceData {
                    macro_link: macro_link_id.to_string(),
                    by_node: by_node_id.to_string(),
                    superseded_links,
                    superseded_nodes,
                    new_links: vec![],
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

    let link_b_id = match storage.next_id("LINK") {
        Ok(id) => id,
        Err(e) => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: ReplaceData {
                    macro_link: macro_link_id.to_string(),
                    by_node: by_node_id.to_string(),
                    superseded_links,
                    superseded_nodes,
                    new_links: vec![link_a_id],
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

    let edge_a = Edge {
        id: link_a_id.clone(),
        from: vec![macro_from],
        to: by_node_id.to_string(),
        operator: Operator::Single,
        weight: None,
        status: EdgeStatus::Active,
        logic: Logic::Sufficiency,
        assumptions: vec![],
    };

    let edge_b = Edge {
        id: link_b_id.clone(),
        from: vec![by_node_id.to_string()],
        to: macro_to,
        operator: Operator::Single,
        weight: None,
        status: EdgeStatus::Active,
        logic: Logic::Sufficiency,
        assumptions: vec![],
    };

    tree.edges.push(edge_a);
    tree.edges.push(edge_b);

    // Remove the macro_edge
    tree.macro_edges.remove(macro_idx);

    if let Err(e) = storage.save_tree(&tree) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: ReplaceData {
                macro_link: macro_link_id.to_string(),
                by_node: by_node_id.to_string(),
                superseded_links,
                superseded_nodes,
                new_links: vec![link_a_id, link_b_id],
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
        ReplaceData {
            macro_link: macro_link_id.to_string(),
            by_node: by_node_id.to_string(),
            superseded_links,
            superseded_nodes,
            new_links: vec![link_a_id, link_b_id],
        },
    );
    output.warnings = warnings;
    output
}
