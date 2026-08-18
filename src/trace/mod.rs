use std::collections::{BTreeSet, VecDeque};

use serde::Serialize;

use crate::link::types::{EdgeStatus, FeedbackLoopType, Logic, Operator};
use crate::output::{CommandOutput, GraphHealth, OutputError};
use crate::storage::Storage;

// --- Output types ---

/// Summary of a link encountered during trace traversal.
#[derive(Debug, Clone, Serialize)]
pub struct LinkSummary {
    pub id: String,
    pub status: String,
    pub operator: String,
}

/// Knowledge item summary attached to a traced node.
#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeSummary {
    pub id: String,
    pub relation: String,
    pub status: String,
    pub confidence: Option<String>,
}

/// Single entry in the trace chain.
#[derive(Debug, Clone, Serialize)]
pub struct TraceEntry {
    pub node: String,
    pub link_to_next: Option<LinkSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub knowledge: Option<Vec<KnowledgeSummary>>,
}

/// Health summary of the traced chain.
#[derive(Debug, Clone, Serialize)]
pub struct ChainHealth {
    pub fully_connected: bool,
    pub broken_links: Vec<String>,
    pub superseded_links: Vec<String>,
}

/// Feedback edge encountered during trace.
#[derive(Debug, Clone, Serialize)]
pub struct FeedbackSummary {
    pub id: String,
    pub from: String,
    pub to: String,
    pub loop_type: String,
}

/// Full output data for `ltp trace`.
#[derive(Debug, Clone, Serialize)]
pub struct TraceData {
    pub node_id: String,
    pub tree_id: String,
    pub direction: String,
    pub depth: Option<usize>,
    pub chain: Vec<TraceEntry>,
    pub feedback_loops: Vec<FeedbackSummary>,
    pub chain_health: ChainHealth,
}

/// Node label reference for link inspect.
#[derive(Debug, Clone, Serialize)]
pub struct NodeLabel {
    pub id: String,
    pub label: String,
}

/// Assumption detail for link inspect.
#[derive(Debug, Clone, Serialize)]
pub struct AssumptionDetail {
    pub id: String,
    pub text: String,
    pub status: String,
}

/// Full output data for `ltp link inspect`.
#[derive(Debug, Clone, Serialize)]
pub struct LinkInspectData {
    pub id: String,
    pub from: Vec<String>,
    pub from_labels: Vec<NodeLabel>,
    pub to: String,
    pub to_label: String,
    pub operator: String,
    pub weight: Option<f64>,
    pub status: String,
    pub logic: String,
    pub assumptions: Vec<AssumptionDetail>,
}

/// Single match in `ltp link find`.
#[derive(Debug, Clone, Serialize)]
pub struct LinkFindEntry {
    pub id: String,
    pub operator: String,
    pub status: String,
}

/// Full output data for `ltp link find`.
#[derive(Debug, Clone, Serialize)]
pub struct LinkFindData {
    pub from: String,
    pub to: String,
    pub tree_id: String,
    pub links: Vec<LinkFindEntry>,
}

// --- Helpers ---

fn operator_str(op: Operator) -> &'static str {
    match op {
        Operator::Single => "SINGLE",
        Operator::And => "AND",
        Operator::Or => "OR",
        Operator::Mag => "MAG",
        Operator::Xor => "XOR",
    }
}

fn status_str(s: EdgeStatus) -> &'static str {
    match s {
        EdgeStatus::Active => "active",
        EdgeStatus::Broken => "broken",
        EdgeStatus::Superseded => "superseded",
        EdgeStatus::NeedsReview => "needs_review",
    }
}

fn logic_str(l: Logic) -> &'static str {
    match l {
        Logic::Sufficiency => "sufficiency",
        Logic::Necessity => "necessity",
    }
}

fn feedback_type_str(t: FeedbackLoopType) -> &'static str {
    match t {
        FeedbackLoopType::Positive => "positive",
        FeedbackLoopType::Negative => "negative",
    }
}

#[allow(clippy::too_many_arguments)]
fn empty_trace_error(
    action: &str,
    ws: String,
    code: &str,
    detail: String,
    node_id: &str,
    tree_id: &str,
    direction: &str,
    depth: Option<usize>,
) -> CommandOutput<TraceData> {
    CommandOutput {
        success: false,
        action: action.to_string(),
        workspace: ws,
        data: TraceData {
            node_id: node_id.to_string(),
            tree_id: tree_id.to_string(),
            direction: direction.to_string(),
            depth,
            chain: vec![],
            feedback_loops: vec![],
            chain_health: ChainHealth {
                fully_connected: true,
                broken_links: vec![],
                superseded_links: vec![],
            },
        },
        graph_health: GraphHealth {
            valid_dag: true,
            orphan_nodes_count: 0,
        },
        errors: vec![OutputError::new(code, detail)],
        warnings: vec![],
    }
}

// --- Execute functions ---

/// Trace upstream or downstream from a node within a tree.
#[allow(clippy::too_many_arguments)]
pub fn execute_trace(
    storage: &dyn Storage,
    node_id: &str,
    tree_id: &str,
    direction: &str,
    depth: Option<usize>,
    no_feedback: bool,
    include_nbr: bool,
    show_knowledge: bool,
) -> CommandOutput<TraceData> {
    let ws_name = storage.workspace_name().unwrap_or_default();

    let tree = match storage.load_tree(tree_id) {
        Ok(t) => t,
        Err(_) => {
            return empty_trace_error(
                "trace",
                ws_name,
                "TREE_NOT_FOUND",
                format!("Tree '{}' not found", tree_id),
                node_id,
                tree_id,
                direction,
                depth,
            );
        }
    };

    if storage.load_node(node_id).is_err() {
        return empty_trace_error(
            "trace",
            ws_name,
            "NODE_NOT_FOUND",
            format!("Node '{}' not found in pool", node_id),
            node_id,
            tree_id,
            direction,
            depth,
        );
    }

    let is_attached = tree.nodes.iter().any(|nr| nr.node_ref == node_id);
    if !is_attached {
        return empty_trace_error(
            "trace",
            ws_name,
            "NODE_NOT_IN_TREE",
            format!("Node '{}' is not attached to tree '{}'", node_id, tree_id),
            node_id,
            tree_id,
            direction,
            depth,
        );
    }

    // Collect all edges to consider
    let mut all_edges = tree.edges.clone();
    if include_nbr {
        for nbr in &tree.nbr_branches {
            all_edges.extend(nbr.edges.clone());
        }
    }

    // BFS traversal
    let mut chain: Vec<TraceEntry> = Vec::new();
    let mut visited: BTreeSet<String> = BTreeSet::new();
    // Queue: (node_id, current_depth)
    let mut queue: VecDeque<(String, usize)> = VecDeque::new();
    queue.push_back((node_id.to_string(), 0));
    visited.insert(node_id.to_string());

    let max_depth = depth.unwrap_or(usize::MAX);

    match direction {
        "downstream" => {
            while let Some((current_node, current_depth)) = queue.pop_front() {
                if current_depth >= max_depth {
                    chain.push(TraceEntry {
                        node: current_node,
                        link_to_next: None,
                        knowledge: None,
                    });
                    continue;
                }

                // Find edges where current_node is in `from`
                let outgoing: Vec<_> = all_edges
                    .iter()
                    .filter(|e| e.from.contains(&current_node))
                    .collect();

                if outgoing.is_empty() {
                    chain.push(TraceEntry {
                        node: current_node,
                        link_to_next: None,
                        knowledge: None,
                    });
                } else {
                    for edge in &outgoing {
                        chain.push(TraceEntry {
                            node: current_node.clone(),
                            link_to_next: Some(LinkSummary {
                                id: edge.id.clone(),
                                status: status_str(edge.status).to_string(),
                                operator: operator_str(edge.operator).to_string(),
                            }),
                            knowledge: None,
                        });

                        if !visited.contains(&edge.to) {
                            visited.insert(edge.to.clone());
                            queue.push_back((edge.to.clone(), current_depth + 1));
                        }
                    }
                }
            }
        }
        "upstream" => {
            while let Some((current_node, current_depth)) = queue.pop_front() {
                if current_depth >= max_depth {
                    chain.push(TraceEntry {
                        node: current_node,
                        link_to_next: None,
                        knowledge: None,
                    });
                    continue;
                }

                // Find edges where current_node is the `to`
                let incoming: Vec<_> = all_edges.iter().filter(|e| e.to == current_node).collect();

                if incoming.is_empty() {
                    chain.push(TraceEntry {
                        node: current_node,
                        link_to_next: None,
                        knowledge: None,
                    });
                } else {
                    for edge in &incoming {
                        chain.push(TraceEntry {
                            node: current_node.clone(),
                            link_to_next: Some(LinkSummary {
                                id: edge.id.clone(),
                                status: status_str(edge.status).to_string(),
                                operator: operator_str(edge.operator).to_string(),
                            }),
                            knowledge: None,
                        });

                        for from_node in &edge.from {
                            if !visited.contains(from_node) {
                                visited.insert(from_node.clone());
                                queue.push_back((from_node.clone(), current_depth + 1));
                            }
                        }
                    }
                }
            }
        }
        _ => {
            return empty_trace_error(
                "trace",
                ws_name,
                "INVALID_DIRECTION",
                format!(
                    "Direction must be 'upstream' or 'downstream', got '{}'",
                    direction
                ),
                node_id,
                tree_id,
                direction,
                depth,
            );
        }
    }

    // Deduplicate: remove the starting node if it appears as a terminal-only
    // entry AND also has a link_to_next entry (BFS may produce both).
    // The canonical contract: first entry is the start node.

    // Feedback loops — collect those touching any node in the visited set
    let feedback_loops: Vec<FeedbackSummary> = if no_feedback {
        vec![]
    } else {
        tree.feedback_edges
            .iter()
            .filter(|fe| visited.contains(&fe.from) || visited.contains(&fe.to))
            .map(|fe| FeedbackSummary {
                id: fe.id.clone(),
                from: fe.from.clone(),
                to: fe.to.clone(),
                loop_type: feedback_type_str(fe.loop_type).to_string(),
            })
            .collect()
    };

    // Chain health
    let mut broken_links: Vec<String> = Vec::new();
    let mut superseded_links: Vec<String> = Vec::new();
    for entry in &chain {
        if let Some(ref link) = entry.link_to_next {
            if link.status == "broken" {
                broken_links.push(link.id.clone());
            } else if link.status == "superseded" {
                superseded_links.push(link.id.clone());
            }
        }
    }
    // Deduplicate (same link may appear multiple times in branching paths)
    broken_links.sort();
    broken_links.dedup();
    superseded_links.sort();
    superseded_links.dedup();

    let fully_connected = broken_links.is_empty() && superseded_links.is_empty();

    let chain_health = ChainHealth {
        fully_connected,
        broken_links,
        superseded_links,
    };

    // Attach knowledge summaries if requested
    if show_knowledge {
        let kn_ids = storage.list_knowledge_ids().unwrap_or_default();
        let kn_items: Vec<crate::knowledge::KnowledgeItem> = kn_ids
            .iter()
            .filter_map(|id| storage.load_knowledge(id).ok())
            .collect();

        for entry in &mut chain {
            let node_knowledge: Vec<KnowledgeSummary> = kn_items
                .iter()
                .flat_map(|item| {
                    item.links
                        .iter()
                        .filter(|l| l.target == entry.node)
                        .map(move |l| KnowledgeSummary {
                            id: item.id.clone(),
                            relation: format!("{:?}", l.relation).to_lowercase(),
                            status: format!("{:?}", item.status).to_lowercase(),
                            confidence: item.confidence.map(|c| format!("{:?}", c).to_lowercase()),
                        })
                })
                .collect();
            entry.knowledge = Some(node_knowledge);
        }
    }

    CommandOutput::ok(
        "trace",
        &ws_name,
        TraceData {
            node_id: node_id.to_string(),
            tree_id: tree_id.to_string(),
            direction: direction.to_string(),
            depth,
            chain,
            feedback_loops,
            chain_health,
        },
    )
}

/// Inspect a single link with full detail including node labels and assumptions.
pub fn execute_link_inspect(
    storage: &dyn Storage,
    link_id: &str,
    tree_id: &str,
) -> CommandOutput<LinkInspectData> {
    let ws_name = storage.workspace_name().unwrap_or_default();

    let tree = match storage.load_tree(tree_id) {
        Ok(t) => t,
        Err(_) => {
            return CommandOutput {
                success: false,
                action: "link_inspect".to_string(),
                workspace: ws_name,
                data: LinkInspectData {
                    id: link_id.to_string(),
                    from: vec![],
                    from_labels: vec![],
                    to: String::new(),
                    to_label: String::new(),
                    operator: String::new(),
                    weight: None,
                    status: String::new(),
                    logic: String::new(),
                    assumptions: vec![],
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

    // Search in regular edges and NBR edges
    let edge = tree.edges.iter().find(|e| e.id == link_id).or_else(|| {
        tree.nbr_branches
            .iter()
            .flat_map(|nbr| nbr.edges.iter())
            .find(|e| e.id == link_id)
    });

    let edge = match edge {
        Some(e) => e,
        None => {
            return CommandOutput {
                success: false,
                action: "link_inspect".to_string(),
                workspace: ws_name,
                data: LinkInspectData {
                    id: link_id.to_string(),
                    from: vec![],
                    from_labels: vec![],
                    to: String::new(),
                    to_label: String::new(),
                    operator: String::new(),
                    weight: None,
                    status: String::new(),
                    logic: String::new(),
                    assumptions: vec![],
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

    // Resolve labels
    let from_labels: Vec<NodeLabel> = edge
        .from
        .iter()
        .map(|nid| {
            let label = storage.load_node(nid).map(|n| n.label).unwrap_or_default();
            NodeLabel {
                id: nid.clone(),
                label,
            }
        })
        .collect();

    let to_label = storage
        .load_node(&edge.to)
        .map(|n| n.label)
        .unwrap_or_default();

    let assumptions: Vec<AssumptionDetail> = edge
        .assumptions
        .iter()
        .map(|a| AssumptionDetail {
            id: a.id.clone(),
            text: a.text.clone(),
            status: format!("{:?}", a.status).to_lowercase(),
        })
        .collect();

    CommandOutput::ok(
        "link_inspect",
        &ws_name,
        LinkInspectData {
            id: edge.id.clone(),
            from: edge.from.clone(),
            from_labels,
            to: edge.to.clone(),
            to_label,
            operator: operator_str(edge.operator).to_string(),
            weight: edge.weight,
            status: status_str(edge.status).to_string(),
            logic: logic_str(edge.logic).to_string(),
            assumptions,
        },
    )
}

/// Find edges between two nodes in a tree.
pub fn execute_link_find(
    storage: &dyn Storage,
    tree_id: &str,
    from: &str,
    to: &str,
) -> CommandOutput<LinkFindData> {
    let ws_name = storage.workspace_name().unwrap_or_default();

    let tree = match storage.load_tree(tree_id) {
        Ok(t) => t,
        Err(_) => {
            return CommandOutput {
                success: false,
                action: "link_find".to_string(),
                workspace: ws_name,
                data: LinkFindData {
                    from: from.to_string(),
                    to: to.to_string(),
                    tree_id: tree_id.to_string(),
                    links: vec![],
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

    // Search in regular edges and NBR edges
    let mut all_edges = tree.edges.iter().collect::<Vec<_>>();
    for nbr in &tree.nbr_branches {
        all_edges.extend(nbr.edges.iter());
    }

    let links: Vec<LinkFindEntry> = all_edges
        .iter()
        .filter(|e| e.from.contains(&from.to_string()) && e.to == to)
        .map(|e| LinkFindEntry {
            id: e.id.clone(),
            operator: operator_str(e.operator).to_string(),
            status: status_str(e.status).to_string(),
        })
        .collect();

    CommandOutput::ok(
        "link_find",
        &ws_name,
        LinkFindData {
            from: from.to_string(),
            to: to.to_string(),
            tree_id: tree_id.to_string(),
            links,
        },
    )
}
