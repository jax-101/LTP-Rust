use serde::Serialize;

use crate::errors::{LtpError, Result};
use crate::node::clr_lint::lint_clr2;
use crate::node::types::{Node, NodeMetadata, NodeStatus, NodeType};
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
}

/// Data returned by `node edit`.
#[derive(Debug, Serialize)]
pub struct NodeEditData {
    pub id: String,
    pub node_type: NodeType,
    pub label: String,
    pub tags: Vec<String>,
    pub observable: bool,
}

/// Summary of a node for listing.
#[derive(Debug, Serialize)]
pub struct NodeSummary {
    pub id: String,
    pub node_type: NodeType,
    pub label: String,
    pub status: NodeStatus,
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

fn node_to_summary(node: &Node) -> NodeSummary {
    NodeSummary {
        id: node.id.clone(),
        node_type: node.node_type,
        label: node.label.clone(),
        status: node.metadata.status,
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
) -> CommandOutput<NodeEditData> {
    let ws_name = storage.workspace_name().unwrap_or_default();

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
pub fn execute_node_list(
    storage: &dyn Storage,
    type_filter: Option<&[String]>,
    status_filter: Option<&[String]>,
) -> CommandOutput<NodeListData> {
    let ws_name = storage.workspace_name().unwrap_or_default();

    let node_ids = match storage.list_node_ids() {
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
