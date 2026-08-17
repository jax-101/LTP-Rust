use serde::Serialize;

use std::collections::HashSet;

use crate::errors::{LtpError, Result};
use crate::link::Operator;
use crate::node::clr_lint::lint_clr2;
use crate::node::types::{EpistemicStatus, Node, NodeMetadata, NodeStatus, NodeType};
use crate::output::{CommandOutput, GraphHealth, OutputError, OutputWarning};
use crate::storage::{LockOutcome, Storage};

/// Data returned by `node add`.
#[derive(Debug, Serialize)]
pub struct NodeAddData {
    pub id: String,
    pub node_type: NodeType,
    pub label: String,
    pub tags: Vec<String>,
    pub observable: bool,
    pub epistemic: EpistemicStatus,
}

/// Data returned by `node edit`.
#[derive(Debug, Serialize)]
pub struct NodeEditData {
    pub id: String,
    pub node_type: NodeType,
    pub label: String,
    pub tags: Vec<String>,
    pub observable: bool,
    pub epistemic: EpistemicStatus,
}

/// Summary of a node for listing.
#[derive(Debug, Serialize)]
pub struct NodeSummary {
    pub id: String,
    pub node_type: NodeType,
    pub label: String,
    pub status: NodeStatus,
    pub epistemic: EpistemicStatus,
    pub tags: Vec<String>,
}

/// Data returned by `node list`.
#[derive(Debug, Serialize)]
pub struct NodeListData {
    pub nodes: Vec<NodeSummary>,
    pub count: usize,
}

/// Data returned by `node search`.
#[derive(Debug, Serialize)]
pub struct NodeSearchData {
    pub query: String,
    pub matches: Vec<NodeSummary>,
    pub count: usize,
}

/// Parse a node type string (case-insensitive) into `NodeType`.
fn parse_node_type(s: &str) -> Result<NodeType> {
    match s.to_uppercase().as_str() {
        "UDE" => Ok(NodeType::Ude),
        "RC" => Ok(NodeType::Rc),
        "INJ" => Ok(NodeType::Inj),
        "NC" => Ok(NodeType::Nc),
        "GOAL" => Ok(NodeType::Goal),
        "OBJ" => Ok(NodeType::Obj),
        "WANT" => Ok(NodeType::Want),
        "OBS" => Ok(NodeType::Obs),
        "IO" => Ok(NodeType::Io),
        "INT" => Ok(NodeType::Int),
        "DE" => Ok(NodeType::De),
        "REQ" => Ok(NodeType::Req),
        "PRE" => Ok(NodeType::Pre),
        other => Err(LtpError::EcValidation(format!(
            "Unknown node type: {}",
            other
        ))),
    }
}

/// Parse a node status string (case-insensitive) into `NodeStatus`.
fn parse_node_status(s: &str) -> Result<NodeStatus> {
    match s.to_lowercase().as_str() {
        "active" => Ok(NodeStatus::Active),
        "draft" => Ok(NodeStatus::Draft),
        "invalidated" => Ok(NodeStatus::Invalidated),
        "superseded" => Ok(NodeStatus::Superseded),
        other => Err(LtpError::EcValidation(format!("Unknown status: {}", other))),
    }
}

/// Parse an epistemic status string (case-insensitive) into `EpistemicStatus`.
fn parse_epistemic(s: &str) -> Result<EpistemicStatus> {
    match s.to_lowercase().as_str() {
        "fact" => Ok(EpistemicStatus::Fact),
        "hypothesis" => Ok(EpistemicStatus::Hypothesis),
        "assumption" => Ok(EpistemicStatus::Assumption),
        "derived" => Ok(EpistemicStatus::Derived),
        other => Err(LtpError::EcValidation(format!(
            "Unknown epistemic status: {}",
            other
        ))),
    }
}

fn node_to_summary(node: &Node) -> NodeSummary {
    NodeSummary {
        id: node.id.clone(),
        node_type: node.node_type,
        label: node.label.clone(),
        status: node.metadata.status,
        epistemic: node.epistemic,
        tags: node.tags.clone(),
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

/// Execute `node add` command.
pub fn execute_node_add(
    storage: &dyn Storage,
    label: &str,
    type_str: &str,
    tags: Option<Vec<String>>,
    observable: Option<bool>,
    epistemic: Option<&str>,
) -> CommandOutput<NodeAddData> {
    let ws_name = storage.workspace_name().unwrap_or_default();

    let node_type = match parse_node_type(type_str) {
        Ok(t) => t,
        Err(e) => {
            return CommandOutput {
                success: false,
                action: "node_add".to_string(),
                workspace: ws_name,
                data: NodeAddData {
                    id: String::new(),
                    node_type: NodeType::Ude,
                    label: String::new(),
                    tags: vec![],
                    observable: true,
                    epistemic: EpistemicStatus::default(),
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new("INVALID_NODE_TYPE", e.to_string())],
                warnings: vec![],
            };
        }
    };

    let epistemic_status = match epistemic {
        Some(s) => match parse_epistemic(s) {
            Ok(e) => e,
            Err(e) => {
                return CommandOutput {
                    success: false,
                    action: "node_add".to_string(),
                    workspace: ws_name,
                    data: NodeAddData {
                        id: String::new(),
                        node_type,
                        label: String::new(),
                        tags: vec![],
                        observable: true,
                        epistemic: EpistemicStatus::default(),
                    },
                    graph_health: GraphHealth {
                        valid_dag: true,
                        orphan_nodes_count: 0,
                    },
                    errors: vec![OutputError::new("INVALID_EPISTEMIC", e.to_string())],
                    warnings: vec![],
                };
            }
        },
        None => EpistemicStatus::default(),
    };

    let lock_outcome = match storage.acquire_lock("node add") {
        Ok(outcome) => outcome,
        Err(e) => {
            let err = match &e {
                LtpError::WorkspaceLocked { pid, timestamp } => {
                    OutputError::new("WORKSPACE_LOCKED", e.to_string())
                        .with_context("pid", serde_json::Value::from(*pid))
                        .with_context("timestamp", serde_json::Value::String(timestamp.clone()))
                }
                _ => OutputError::new("LOCK_ERROR", e.to_string()),
            };
            return CommandOutput {
                success: false,
                action: "node_add".to_string(),
                workspace: ws_name,
                data: NodeAddData {
                    id: String::new(),
                    node_type,
                    label: String::new(),
                    tags: vec![],
                    observable: true,
                    epistemic: epistemic_status,
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![err],
                warnings: vec![],
            };
        }
    };

    let type_prefix = type_str.to_uppercase();
    let id = match storage.next_id(&type_prefix) {
        Ok(id) => id,
        Err(e) => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: "node_add".to_string(),
                workspace: ws_name,
                data: NodeAddData {
                    id: String::new(),
                    node_type,
                    label: String::new(),
                    tags: vec![],
                    observable: true,
                    epistemic: epistemic_status,
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

    let obs = observable.unwrap_or(true);
    let node_tags = tags.unwrap_or_default();

    let node = Node {
        id: id.clone(),
        node_type,
        label: label.to_string(),
        tags: node_tags.clone(),
        observable: obs,
        epistemic: epistemic_status,
        metadata: NodeMetadata {
            status: NodeStatus::Active,
            extra: Default::default(),
        },
    };

    if let Err(e) = storage.save_node(&node) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: "node_add".to_string(),
            workspace: ws_name,
            data: NodeAddData {
                id: String::new(),
                node_type,
                label: String::new(),
                tags: vec![],
                observable: true,
                epistemic: epistemic_status,
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

    let mut warnings = lint_clr2(label);
    if let Some(w) = stale_lock_warning(&lock_outcome) {
        warnings.insert(0, w);
    }

    CommandOutput {
        success: true,
        action: "node_add".to_string(),
        workspace: ws_name,
        data: NodeAddData {
            id,
            node_type,
            label: label.to_string(),
            tags: node_tags,
            observable: obs,
            epistemic: epistemic_status,
        },
        graph_health: GraphHealth {
            valid_dag: true,
            orphan_nodes_count: 0,
        },
        errors: vec![],
        warnings,
    }
}

/// Execute `node edit` command.
pub fn execute_node_edit(
    storage: &dyn Storage,
    id: &str,
    label: Option<&str>,
    add_tag: Option<&str>,
    rm_tag: Option<&str>,
    observable: Option<bool>,
    epistemic: Option<&str>,
) -> CommandOutput<NodeEditData> {
    let ws_name = storage.workspace_name().unwrap_or_default();

    let epistemic_status = match epistemic {
        Some(s) => match parse_epistemic(s) {
            Ok(e) => Some(e),
            Err(e) => {
                return CommandOutput {
                    success: false,
                    action: "node_edit".to_string(),
                    workspace: ws_name,
                    data: NodeEditData {
                        id: id.to_string(),
                        node_type: NodeType::Ude,
                        label: String::new(),
                        tags: vec![],
                        observable: true,
                        epistemic: EpistemicStatus::default(),
                    },
                    graph_health: GraphHealth {
                        valid_dag: true,
                        orphan_nodes_count: 0,
                    },
                    errors: vec![OutputError::new("INVALID_EPISTEMIC", e.to_string())],
                    warnings: vec![],
                };
            }
        },
        None => None,
    };

    let lock_outcome = match storage.acquire_lock("node edit") {
        Ok(outcome) => outcome,
        Err(e) => {
            return CommandOutput {
                success: false,
                action: "node_edit".to_string(),
                workspace: ws_name,
                data: NodeEditData {
                    id: id.to_string(),
                    node_type: NodeType::Ude,
                    label: String::new(),
                    tags: vec![],
                    observable: true,
                    epistemic: EpistemicStatus::default(),
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

    let mut node = match storage.load_node(id) {
        Ok(n) => n,
        Err(e) => {
            let _ = storage.release_lock();
            let err = match &e {
                LtpError::NodeNotFound(_) => OutputError::new("NODE_NOT_FOUND", e.to_string()),
                _ => OutputError::new("IO_ERROR", e.to_string()),
            };
            return CommandOutput {
                success: false,
                action: "node_edit".to_string(),
                workspace: ws_name,
                data: NodeEditData {
                    id: id.to_string(),
                    node_type: NodeType::Ude,
                    label: String::new(),
                    tags: vec![],
                    observable: true,
                    epistemic: EpistemicStatus::default(),
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![err],
                warnings: vec![],
            };
        }
    };

    if let Some(new_label) = label {
        node.label = new_label.to_string();
    }

    if let Some(tag) = add_tag {
        if !node.tags.contains(&tag.to_string()) {
            node.tags.push(tag.to_string());
        }
    }

    if let Some(tag) = rm_tag {
        node.tags.retain(|t| t != tag);
    }

    if let Some(obs) = observable {
        node.observable = obs;
    }

    if let Some(ep) = epistemic_status {
        node.epistemic = ep;
    }

    if let Err(e) = storage.save_node(&node) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: "node_edit".to_string(),
            workspace: ws_name,
            data: NodeEditData {
                id: id.to_string(),
                node_type: node.node_type,
                label: node.label.clone(),
                tags: node.tags.clone(),
                observable: node.observable,
                epistemic: node.epistemic,
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

    let mut warnings = if label.is_some() {
        lint_clr2(&node.label)
    } else {
        vec![]
    };

    if let Some(w) = stale_lock_warning(&lock_outcome) {
        warnings.insert(0, w);
    }

    CommandOutput {
        success: true,
        action: "node_edit".to_string(),
        workspace: ws_name,
        data: NodeEditData {
            id: node.id,
            node_type: node.node_type,
            label: node.label,
            tags: node.tags,
            observable: node.observable,
            epistemic: node.epistemic,
        },
        graph_health: GraphHealth {
            valid_dag: true,
            orphan_nodes_count: 0,
        },
        errors: vec![],
        warnings,
    }
}

/// Execute `node list` command.
///
/// Lists nodes from the pool, optionally filtered by tree membership,
/// node type, status, and/or epistemic status.
pub fn execute_node_list(
    storage: &dyn Storage,
    tree_filter: Option<&str>,
    type_filter: Option<&[String]>,
    status_filter: Option<&[String]>,
    epistemic_filter: Option<&str>,
) -> CommandOutput<NodeListData> {
    let ws_name = storage.workspace_name().unwrap_or_default();

    let node_ids: Vec<String> = if let Some(tid) = tree_filter {
        match storage.load_tree(tid) {
            Ok(tree) => tree.nodes.iter().map(|nr| nr.node_ref.clone()).collect(),
            Err(e) => {
                return CommandOutput {
                    success: false,
                    action: "node_list".to_string(),
                    workspace: ws_name,
                    data: NodeListData {
                        nodes: vec![],
                        count: 0,
                    },
                    graph_health: GraphHealth {
                        valid_dag: true,
                        orphan_nodes_count: 0,
                    },
                    errors: vec![OutputError::new("TREE_NOT_FOUND", e.to_string())],
                    warnings: vec![],
                };
            }
        }
    } else {
        match storage.list_node_ids() {
            Ok(ids) => ids,
            Err(e) => {
                return CommandOutput {
                    success: false,
                    action: "node_list".to_string(),
                    workspace: ws_name,
                    data: NodeListData {
                        nodes: vec![],
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
        }
    };

    let type_filters: Option<Vec<NodeType>> = type_filter.map(|types| {
        types
            .iter()
            .filter_map(|t| parse_node_type(t).ok())
            .collect()
    });

    let status_filters: Option<Vec<NodeStatus>> = status_filter.map(|statuses| {
        statuses
            .iter()
            .filter_map(|s| parse_node_status(s).ok())
            .collect()
    });

    let ep_filter: Option<EpistemicStatus> = epistemic_filter.and_then(|s| parse_epistemic(s).ok());

    let mut nodes = Vec::new();
    for id in &node_ids {
        let node = match storage.load_node(id) {
            Ok(n) => n,
            Err(_) => continue,
        };

        if let Some(ref types) = type_filters {
            if !types.contains(&node.node_type) {
                continue;
            }
        }

        if let Some(ref statuses) = status_filters {
            if !statuses.contains(&node.metadata.status) {
                continue;
            }
        }

        if let Some(ep) = ep_filter {
            if node.epistemic != ep {
                continue;
            }
        }

        nodes.push(node_to_summary(&node));
    }

    let count = nodes.len();
    CommandOutput::ok("node_list", &ws_name, NodeListData { nodes, count })
}

/// Execute `node search` command.
pub fn execute_node_search(storage: &dyn Storage, query: &str) -> CommandOutput<NodeSearchData> {
    let ws_name = storage.workspace_name().unwrap_or_default();

    let node_ids = match storage.list_node_ids() {
        Ok(ids) => ids,
        Err(e) => {
            return CommandOutput {
                success: false,
                action: "node_search".to_string(),
                workspace: ws_name,
                data: NodeSearchData {
                    query: query.to_string(),
                    matches: vec![],
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

    let query_lower = query.to_lowercase();
    let mut matches = Vec::new();

    for id in &node_ids {
        let node = match storage.load_node(id) {
            Ok(n) => n,
            Err(_) => continue,
        };

        if node.label.to_lowercase().contains(&query_lower) {
            matches.push(node_to_summary(&node));
        }
    }

    let count = matches.len();
    CommandOutput::ok(
        "node_search",
        &ws_name,
        NodeSearchData {
            query: query.to_string(),
            matches,
            count,
        },
    )
}

/// Data returned by `node rm`.
#[derive(Debug, Serialize)]
pub struct NodeRmData {
    pub removed_nodes: Vec<String>,
    pub removed_edges_count: usize,
    pub affected_trees: Vec<String>,
}

/// Execute `node rm` command.
///
/// Removes nodes from the global pool and cleans up all references
/// in every tree: removes from `nodes[]`, removes edges where the node
/// appears in `from[]` or `to`, and removes feedback edges referencing it.
pub fn execute_node_rm(
    storage: &dyn Storage,
    ids: &[String],
    _force: bool,
) -> CommandOutput<NodeRmData> {
    let ws_name = storage.workspace_name().unwrap_or_default();

    if ids.is_empty() {
        return CommandOutput {
            success: false,
            action: "node_rm".to_string(),
            workspace: ws_name,
            data: NodeRmData {
                removed_nodes: vec![],
                removed_edges_count: 0,
                affected_trees: vec![],
            },
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![OutputError::new("INVALID_ARGS", "No node IDs provided")],
            warnings: vec![],
        };
    }

    for id in ids {
        if storage.load_node(id).is_err() {
            return CommandOutput {
                success: false,
                action: "node_rm".to_string(),
                workspace: ws_name,
                data: NodeRmData {
                    removed_nodes: vec![],
                    removed_edges_count: 0,
                    affected_trees: vec![],
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new(
                    "NODE_NOT_FOUND",
                    format!("Node '{}' not found in pool", id),
                )],
                warnings: vec![],
            };
        }
    }

    let lock_outcome = match storage.acquire_lock("node rm") {
        Ok(o) => o,
        Err(e) => {
            return CommandOutput {
                success: false,
                action: "node_rm".to_string(),
                workspace: ws_name,
                data: NodeRmData {
                    removed_nodes: vec![],
                    removed_edges_count: 0,
                    affected_trees: vec![],
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

    let id_set: HashSet<&str> = ids.iter().map(|s| s.as_str()).collect();

    let tree_ids = match storage.list_tree_ids() {
        Ok(t) => t,
        Err(e) => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: "node_rm".to_string(),
                workspace: ws_name,
                data: NodeRmData {
                    removed_nodes: vec![],
                    removed_edges_count: 0,
                    affected_trees: vec![],
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

    let mut total_removed_edges = 0usize;
    let mut affected_trees = Vec::new();

    for tree_id in &tree_ids {
        let mut tree = match storage.load_tree(tree_id) {
            Ok(t) => t,
            Err(_) => continue,
        };

        let before_nodes = tree.nodes.len();
        let before_edges = tree.edges.len();
        let before_fb = tree.feedback_edges.len();

        tree.nodes
            .retain(|nr| !id_set.contains(nr.node_ref.as_str()));

        tree.edges.retain(|edge| {
            let to_removed = id_set.contains(edge.to.as_str());
            let from_has_removed = edge.from.iter().any(|f| id_set.contains(f.as_str()));
            !to_removed && !from_has_removed
        });

        tree.feedback_edges
            .retain(|fb| !id_set.contains(fb.from.as_str()) && !id_set.contains(fb.to.as_str()));

        let edges_removed =
            (before_edges - tree.edges.len()) + (before_fb - tree.feedback_edges.len());
        let tree_changed = tree.nodes.len() != before_nodes
            || tree.edges.len() != before_edges
            || tree.feedback_edges.len() != before_fb;

        if tree_changed {
            if let Err(e) = storage.save_tree(&tree) {
                let _ = storage.release_lock();
                return CommandOutput {
                    success: false,
                    action: "node_rm".to_string(),
                    workspace: ws_name,
                    data: NodeRmData {
                        removed_nodes: vec![],
                        removed_edges_count: 0,
                        affected_trees: vec![],
                    },
                    graph_health: GraphHealth {
                        valid_dag: true,
                        orphan_nodes_count: 0,
                    },
                    errors: vec![OutputError::new("IO_ERROR", e.to_string())],
                    warnings: vec![],
                };
            }
            total_removed_edges += edges_removed;
            affected_trees.push(tree_id.clone());
        }
    }

    let mut removed_nodes = Vec::new();
    for id in ids {
        if let Err(e) = storage.delete_node(id) {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: "node_rm".to_string(),
                workspace: ws_name,
                data: NodeRmData {
                    removed_nodes,
                    removed_edges_count: total_removed_edges,
                    affected_trees,
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new("IO_ERROR", e.to_string())],
                warnings: vec![],
            };
        }
        removed_nodes.push(id.clone());
    }

    let _ = storage.release_lock();

    let mut warnings = vec![];
    if let Some(w) = stale_lock_warning(&lock_outcome) {
        warnings.push(w);
    }

    CommandOutput {
        success: true,
        action: "node_rm".to_string(),
        workspace: ws_name,
        data: NodeRmData {
            removed_nodes,
            removed_edges_count: total_removed_edges,
            affected_trees,
        },
        graph_health: GraphHealth {
            valid_dag: true,
            orphan_nodes_count: 0,
        },
        errors: vec![],
        warnings,
    }
}

/// Summary of a node's participation in one tree.
#[derive(Debug, Serialize)]
pub struct NodeTreeParticipation {
    pub tree_id: String,
    pub tree_name: String,
    pub role: Option<String>,
    pub connections: NodeConnections,
}

/// Inbound and outbound connections of a node within a tree.
#[derive(Debug, Serialize)]
pub struct NodeConnections {
    pub inbound: Vec<EdgeSummary>,
    pub outbound: Vec<EdgeSummary>,
}

/// Compact representation of an edge for inspect output.
#[derive(Debug, Serialize)]
pub struct EdgeSummary {
    pub id: String,
    pub from: Vec<String>,
    pub to: String,
    pub operator: Operator,
}

/// Data returned by `node inspect`.
#[derive(Debug, Serialize)]
pub struct NodeInspectData {
    pub id: String,
    pub node_type: NodeType,
    pub label: String,
    pub tags: Vec<String>,
    pub observable: bool,
    pub epistemic: EpistemicStatus,
    pub status: NodeStatus,
    pub trees: Vec<NodeTreeParticipation>,
}

/// Execute `node inspect` command.
///
/// Shows which trees a node participates in, its role in each,
/// and all inbound/outbound edges per tree.
pub fn execute_node_inspect(storage: &dyn Storage, id: &str) -> CommandOutput<NodeInspectData> {
    let ws_name = storage.workspace_name().unwrap_or_default();

    let node = match storage.load_node(id) {
        Ok(n) => n,
        Err(_) => {
            return CommandOutput {
                success: false,
                action: "node_inspect".to_string(),
                workspace: ws_name,
                data: NodeInspectData {
                    id: id.to_string(),
                    node_type: NodeType::Ude,
                    label: String::new(),
                    tags: vec![],
                    observable: true,
                    epistemic: EpistemicStatus::default(),
                    status: NodeStatus::Active,
                    trees: vec![],
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new(
                    "NODE_NOT_FOUND",
                    format!("Node '{}' not found", id),
                )],
                warnings: vec![],
            };
        }
    };

    let tree_ids = storage.list_tree_ids().unwrap_or_default();
    let mut participations = Vec::new();

    for tree_id in &tree_ids {
        let tree = match storage.load_tree(tree_id) {
            Ok(t) => t,
            Err(_) => continue,
        };

        let node_ref = tree.nodes.iter().find(|nr| nr.node_ref == id);
        if let Some(nr) = node_ref {
            let inbound: Vec<EdgeSummary> = tree
                .edges
                .iter()
                .filter(|e| e.to == id)
                .map(|e| EdgeSummary {
                    id: e.id.clone(),
                    from: e.from.clone(),
                    to: e.to.clone(),
                    operator: e.operator,
                })
                .collect();

            let outbound: Vec<EdgeSummary> = tree
                .edges
                .iter()
                .filter(|e| e.from.contains(&id.to_string()))
                .map(|e| EdgeSummary {
                    id: e.id.clone(),
                    from: e.from.clone(),
                    to: e.to.clone(),
                    operator: e.operator,
                })
                .collect();

            participations.push(NodeTreeParticipation {
                tree_id: tree.id.clone(),
                tree_name: tree.name.clone(),
                role: nr.role.clone(),
                connections: NodeConnections { inbound, outbound },
            });
        }
    }

    CommandOutput::ok(
        "node_inspect",
        &ws_name,
        NodeInspectData {
            id: node.id,
            node_type: node.node_type,
            label: node.label,
            tags: node.tags,
            observable: node.observable,
            epistemic: node.epistemic,
            status: node.metadata.status,
            trees: participations,
        },
    )
}

/// Summary of a newly created node after split.
#[derive(Debug, Serialize)]
pub struct NewNodeSummary {
    pub id: String,
    pub label: String,
    pub node_type: NodeType,
}

/// Data returned by `node split`.
#[derive(Debug, Serialize)]
pub struct NodeSplitData {
    pub original_id: String,
    pub new_nodes: Vec<NewNodeSummary>,
    pub tree_id: String,
}

/// Execute `node split` command.
///
/// Splits a node into two new nodes within a specific tree.
/// Incoming edges of the original are redirected to the first new node.
/// Outgoing edges of the original are redirected from the second new node.
/// The original node is removed from pool and tree.
pub fn execute_node_split(
    storage: &dyn Storage,
    id: &str,
    labels: &[String],
    tree_id: &str,
) -> CommandOutput<NodeSplitData> {
    let ws_name = storage.workspace_name().unwrap_or_default();

    let empty_data = || NodeSplitData {
        original_id: id.to_string(),
        new_nodes: vec![],
        tree_id: tree_id.to_string(),
    };

    if labels.len() != 2 {
        return CommandOutput {
            success: false,
            action: "node_split".to_string(),
            workspace: ws_name,
            data: empty_data(),
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![OutputError::new(
                "INVALID_ARGS",
                "Split requires exactly 2 labels",
            )],
            warnings: vec![],
        };
    }

    let original = match storage.load_node(id) {
        Ok(n) => n,
        Err(_) => {
            return CommandOutput {
                success: false,
                action: "node_split".to_string(),
                workspace: ws_name,
                data: empty_data(),
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new(
                    "NODE_NOT_FOUND",
                    format!("Node '{}' not found", id),
                )],
                warnings: vec![],
            };
        }
    };

    let lock_outcome = match storage.acquire_lock("node split") {
        Ok(o) => o,
        Err(e) => {
            return CommandOutput {
                success: false,
                action: "node_split".to_string(),
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
                action: "node_split".to_string(),
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

    if !tree.nodes.iter().any(|nr| nr.node_ref == id) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: "node_split".to_string(),
            workspace: ws_name,
            data: empty_data(),
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![OutputError::new(
                "NODE_NOT_IN_TREE",
                format!("Node '{}' is not attached to tree '{}'", id, tree_id),
            )],
            warnings: vec![],
        };
    }

    let type_prefix = original.node_type.prefix();
    let id_first = match storage.next_id(type_prefix) {
        Ok(new_id) => new_id,
        Err(e) => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: "node_split".to_string(),
                workspace: ws_name,
                data: empty_data(),
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new("ID_GENERATION_ERROR", e.to_string())],
                warnings: vec![],
            };
        }
    };
    let id_second = match storage.next_id(type_prefix) {
        Ok(new_id) => new_id,
        Err(e) => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: "node_split".to_string(),
                workspace: ws_name,
                data: empty_data(),
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new("ID_GENERATION_ERROR", e.to_string())],
                warnings: vec![],
            };
        }
    };

    let node_first = Node {
        id: id_first.clone(),
        node_type: original.node_type,
        label: labels[0].clone(),
        tags: original.tags.clone(),
        observable: original.observable,
        epistemic: EpistemicStatus::default(),
        metadata: NodeMetadata {
            status: NodeStatus::Active,
            extra: Default::default(),
        },
    };
    let node_second = Node {
        id: id_second.clone(),
        node_type: original.node_type,
        label: labels[1].clone(),
        tags: original.tags.clone(),
        observable: original.observable,
        epistemic: EpistemicStatus::default(),
        metadata: NodeMetadata {
            status: NodeStatus::Active,
            extra: Default::default(),
        },
    };

    if let Err(e) = storage.save_node(&node_first) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: "node_split".to_string(),
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
    if let Err(e) = storage.save_node(&node_second) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: "node_split".to_string(),
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

    // Update tree: replace original node ref with two new refs
    tree.nodes.retain(|nr| nr.node_ref != id);
    tree.nodes.push(crate::tree::NodeRef {
        node_ref: id_first.clone(),
        role: None,
    });
    tree.nodes.push(crate::tree::NodeRef {
        node_ref: id_second.clone(),
        role: None,
    });

    // Redirect inbound edges (to == original) -> to = first
    for edge in &mut tree.edges {
        if edge.to == id {
            edge.to = id_first.clone();
        }
    }
    // Redirect outbound edges (from contains original) -> replace with second
    for edge in &mut tree.edges {
        for from_ref in &mut edge.from {
            if *from_ref == id {
                *from_ref = id_second.clone();
            }
        }
    }

    if let Err(e) = storage.save_tree(&tree) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: "node_split".to_string(),
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

    if let Err(e) = storage.delete_node(id) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: "node_split".to_string(),
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
        action: "node_split".to_string(),
        workspace: ws_name,
        data: NodeSplitData {
            original_id: id.to_string(),
            new_nodes: vec![
                NewNodeSummary {
                    id: id_first,
                    label: labels[0].clone(),
                    node_type: original.node_type,
                },
                NewNodeSummary {
                    id: id_second,
                    label: labels[1].clone(),
                    node_type: original.node_type,
                },
            ],
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
