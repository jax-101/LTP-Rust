use serde::Serialize;

use std::collections::{BTreeSet, HashSet};

use crate::errors::{LtpError, Result};
use crate::link::Operator;
use crate::meta::integrity::{prune_removed, redirect_split};
use crate::meta::node_in_tree;
use crate::node::clr_lint::lint_clr2;
use crate::node::types::{CrossRef, EpistemicStatus, Node, NodeMetadata, NodeStatus, NodeType};
use crate::output::{
    prepend_session_warnings, CommandOutput, GraphHealth, OutputError, OutputWarning,
};
use crate::storage::Storage;

/// Data returned by `node add`.
#[derive(Debug, Serialize)]
pub struct NodeAddData {
    pub id: String,
    pub node_type: NodeType,
    pub label: String,
    pub tags: Vec<String>,
    pub observable: bool,
    pub epistemic: EpistemicStatus,
    /// Outbound cross-tree refs (ADR-015), sorted.
    pub refs: Vec<CrossRef>,
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
    /// Outbound cross-tree refs (ADR-015), sorted.
    pub refs: Vec<CrossRef>,
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
        "CSF" => Ok(NodeType::Csf),
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

/// Validates refs being written to node `self_id` (ADR-015, blocking integrity):
/// no self-ref, target node exists, pinned tree exists and contains the target
/// (trunk or NBR branch).
fn validate_new_refs(
    storage: &dyn Storage,
    self_id: Option<&str>,
    refs: &[CrossRef],
) -> std::result::Result<(), OutputError> {
    let ref_context = |err: OutputError, r: &CrossRef| {
        err.with_context("ref_node", serde_json::Value::String(r.node.clone()))
            .with_context(
                "ref_tree",
                r.tree
                    .clone()
                    .map_or(serde_json::Value::Null, serde_json::Value::String),
            )
    };
    for r in refs {
        if self_id == Some(r.node.as_str()) {
            return Err(ref_context(
                OutputError::new(
                    "SELF_REF",
                    format!("Node '{}' cannot reference itself", r.node),
                ),
                r,
            ));
        }
        if let Err(e) = storage.load_node(&r.node) {
            return Err(ref_context(
                OutputError::load_failed(
                    &e,
                    "NODE_NOT_FOUND",
                    format!("Referenced node '{}' not found in pool", r.node),
                ),
                r,
            ));
        }
        if let Some(tree_id) = &r.tree {
            let tree = match storage.load_tree(tree_id) {
                Ok(t) => t,
                Err(e) => {
                    return Err(ref_context(
                        OutputError::load_failed(
                            &e,
                            "TREE_NOT_FOUND",
                            format!("Referenced tree '{}' not found", tree_id),
                        ),
                        r,
                    ))
                }
            };
            if !node_in_tree(&tree, &r.node) {
                return Err(ref_context(
                    OutputError::new(
                        "NODE_NOT_IN_TREE",
                        format!("Referenced node '{}' is not in tree '{}'", r.node, tree_id),
                    ),
                    r,
                ));
            }
        }
    }
    Ok(())
}

/// Execute `node add` command.
pub fn execute_node_add(
    storage: &dyn Storage,
    label: &str,
    type_str: &str,
    tags: Option<Vec<String>>,
    observable: Option<bool>,
    epistemic: Option<&str>,
    refs: &[CrossRef],
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
                    refs: vec![],
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
                        refs: vec![],
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

    if let Err(err) = validate_new_refs(storage, None, refs) {
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
                refs: vec![],
            },
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![err],
            warnings: vec![],
        };
    }
    let mut metadata = NodeMetadata::new(NodeStatus::Active);
    for r in refs {
        metadata.add_ref(r.clone());
    }

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
                    refs: vec![],
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
    let mut notice = None;

    let type_prefix = type_str.to_uppercase();
    let id = match storage.next_id(&type_prefix) {
        Ok(m) => m.into_id(&mut notice),
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
                    refs: vec![],
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
        metadata,
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
                refs: vec![],
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
    prepend_session_warnings(&mut warnings, &lock_outcome, notice.as_ref());

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
            refs: node.metadata.refs,
        },
        graph_health: GraphHealth {
            valid_dag: true,
            orphan_nodes_count: 0,
        },
        errors: vec![],
        warnings,
    }
}

/// Check for epistemic cascade issues when a node's epistemic status changes.
///
/// - Promote to fact/derived: warns if upstream causes are hypothesis/assumption.
/// - Degrade from fact/derived: warns that downstream effects should be reviewed.
fn check_epistemic_cascade(
    storage: &dyn Storage,
    node_id: &str,
    old_status: EpistemicStatus,
    new_status: EpistemicStatus,
) -> Vec<OutputWarning> {
    let mut warnings = Vec::new();

    let is_promotion = matches!(
        (old_status, new_status),
        (
            EpistemicStatus::Hypothesis | EpistemicStatus::Assumption,
            EpistemicStatus::Fact
        ) | (
            EpistemicStatus::Hypothesis | EpistemicStatus::Assumption,
            EpistemicStatus::Derived,
        )
    );

    let is_degradation = matches!(
        (old_status, new_status),
        (
            EpistemicStatus::Fact | EpistemicStatus::Derived,
            EpistemicStatus::Hypothesis
        ) | (
            EpistemicStatus::Fact | EpistemicStatus::Derived,
            EpistemicStatus::Assumption,
        )
    );

    if !is_promotion && !is_degradation {
        return warnings;
    }

    let tree_ids = match storage.list_tree_ids() {
        Ok(ids) => ids,
        Err(_) => return warnings,
    };

    for tree_id in &tree_ids {
        let tree = match storage.load_tree(tree_id) {
            Ok(t) => t,
            Err(_) => continue,
        };

        let node_in_tree = tree.nodes.iter().any(|nr| nr.node_ref == node_id);
        if !node_in_tree {
            continue;
        }

        if is_promotion {
            // Check upstream causes: edges where `to == node_id`
            for edge in &tree.edges {
                if edge.to != node_id {
                    continue;
                }
                for from_id in &edge.from {
                    if let Ok(upstream_node) = storage.load_node(from_id) {
                        if matches!(
                            upstream_node.epistemic,
                            EpistemicStatus::Hypothesis | EpistemicStatus::Assumption
                        ) {
                            warnings.push(
                                OutputWarning::new(
                                    "EPISTEMIC_UNBOUNDED_FACT",
                                    format!(
                                        "Node '{}' promoted to {:?} but upstream cause '{}' is {:?}",
                                        node_id, new_status, from_id, upstream_node.epistemic
                                    ),
                                )
                                .with_context(
                                    "node_id",
                                    serde_json::Value::String(node_id.to_string()),
                                )
                                .with_context(
                                    "upstream_node",
                                    serde_json::Value::String(from_id.clone()),
                                )
                                .with_context(
                                    "upstream_epistemic",
                                    serde_json::Value::String(format!(
                                        "{:?}",
                                        upstream_node.epistemic
                                    )),
                                )
                                .with_context(
                                    "tree_id",
                                    serde_json::Value::String(tree_id.clone()),
                                ),
                            );
                        }
                    }
                }
            }
        }

        if is_degradation {
            // Check downstream effects: edges where `from` contains node_id
            let downstream_edges: Vec<&str> = tree
                .edges
                .iter()
                .filter(|e| e.from.iter().any(|f| f == node_id))
                .map(|e| e.to.as_str())
                .collect();

            if !downstream_edges.is_empty() {
                warnings.push(
                    OutputWarning::new(
                        "EPISTEMIC_CASCADE_REVIEW",
                        format!(
                            "Node '{}' degraded to {:?}; {} downstream effect(s) in tree '{}' should be reviewed: {}",
                            node_id,
                            new_status,
                            downstream_edges.len(),
                            tree_id,
                            downstream_edges.join(", ")
                        ),
                    )
                    .with_context(
                        "node_id",
                        serde_json::Value::String(node_id.to_string()),
                    )
                    .with_context(
                        "downstream_nodes",
                        serde_json::Value::Array(
                            downstream_edges
                                .iter()
                                .map(|n| serde_json::Value::String(n.to_string()))
                                .collect(),
                        ),
                    )
                    .with_context(
                        "tree_id",
                        serde_json::Value::String(tree_id.clone()),
                    ),
                );
            }
        }
    }

    warnings
}

/// Execute `node edit` command.
///
/// `add_refs` are validated like `node add` refs; `rm_refs` are removed by
/// exact match, emitting `REF_NOT_PRESENT` for absent ones (removals apply first).
#[allow(clippy::too_many_arguments)]
pub fn execute_node_edit(
    storage: &dyn Storage,
    id: &str,
    label: Option<&str>,
    add_tag: Option<&str>,
    rm_tag: Option<&str>,
    observable: Option<bool>,
    epistemic: Option<&str>,
    add_refs: &[CrossRef],
    rm_refs: &[CrossRef],
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
                        refs: vec![],
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
                    refs: vec![],
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
                    refs: vec![],
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

    if let Err(err) = validate_new_refs(storage, Some(id), add_refs) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: "node_edit".to_string(),
            workspace: ws_name,
            data: NodeEditData {
                id: id.to_string(),
                node_type: node.node_type,
                label: node.label,
                tags: node.tags,
                observable: node.observable,
                epistemic: node.epistemic,
                refs: node.metadata.refs,
            },
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![err],
            warnings: vec![],
        };
    }

    let mut ref_warnings = Vec::new();
    for r in rm_refs {
        if !node.metadata.remove_ref(r) {
            ref_warnings.push(
                OutputWarning::new(
                    "REF_NOT_PRESENT",
                    format!("Node '{}' has no ref to '{}'", id, r.node),
                )
                .with_context("node_id", serde_json::Value::String(id.to_string()))
                .with_context("ref_node", serde_json::Value::String(r.node.clone()))
                .with_context(
                    "ref_tree",
                    r.tree
                        .clone()
                        .map_or(serde_json::Value::Null, serde_json::Value::String),
                ),
            );
        }
    }
    for r in add_refs {
        node.metadata.add_ref(r.clone());
    }

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

    let old_epistemic = node.epistemic;
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
                refs: vec![],
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

    prepend_session_warnings(&mut warnings, &lock_outcome, None);
    warnings.extend(ref_warnings);

    // Epistemic cascade warnings when status changes
    if epistemic_status.is_some() && old_epistemic != node.epistemic {
        warnings.extend(check_epistemic_cascade(
            storage,
            id,
            old_epistemic,
            node.epistemic,
        ));
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
            refs: node.metadata.refs,
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
                    errors: vec![OutputError::load_failed(
                        &e,
                        "TREE_NOT_FOUND",
                        e.to_string(),
                    )],
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
/// Removes nodes from the global pool and cleans up all references in every tree
/// via [`prune_removed`]: `nodes[]`, trunk, feedback and NBR edges, NBR branches whose
/// source is removed, and `macro_edges` (ADR-016 D-3). Repeated IDs are deduplicated.
///
/// Fail-closed (D-4): every tree is loaded before any write; an unreadable tree aborts
/// with `IO_ERROR {tree_id}`. Warnings: stale lock, `NBR_BRANCH_REMOVED`,
/// `MACRO_EDGE_REMOVED` (by tree, then storage order), `REFS_STRIPPED`, `KNOWLEDGE_ORPHANED`.
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

    // Repeated IDs behave like a single one (first occurrence wins the order).
    let mut seen = HashSet::new();
    let ids: Vec<&str> = ids
        .iter()
        .map(String::as_str)
        .filter(|id| seen.insert(*id))
        .collect();

    for id in &ids {
        if let Err(e) = storage.load_node(id) {
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
                errors: vec![OutputError::load_failed(
                    &e,
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

    let id_set: HashSet<&str> = ids.iter().copied().collect();
    let fail = |error: OutputError| {
        let _ = storage.release_lock();
        CommandOutput {
            success: false,
            action: "node_rm".to_string(),
            workspace: storage.workspace_name().unwrap_or_default(),
            data: NodeRmData {
                removed_nodes: vec![],
                removed_edges_count: 0,
                affected_trees: vec![],
            },
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![error],
            warnings: vec![],
        }
    };

    let tree_ids = match storage.list_tree_ids() {
        Ok(t) => t,
        Err(e) => return fail(OutputError::new("IO_ERROR", e.to_string())),
    };

    // Fail-closed (ADR-016 D-4): every tree is loaded before anything is written.
    let mut trees = Vec::with_capacity(tree_ids.len());
    for tree_id in tree_ids {
        match storage.load_tree(&tree_id) {
            Ok(tree) => trees.push((tree_id, tree)),
            Err(e) => {
                return fail(
                    OutputError::new("IO_ERROR", e.to_string()).with_context("tree_id", tree_id),
                )
            }
        }
    }

    let mut total_removed_edges = 0usize;
    let mut affected_trees = Vec::new();
    let mut nbr_branch_warnings: Vec<OutputWarning> = Vec::new();
    let mut macro_warnings: Vec<OutputWarning> = Vec::new();

    for (tree_id, mut tree) in trees {
        let report = prune_removed(&mut tree, &id_set);
        if !report.changed {
            continue;
        }
        if let Err(e) = storage.save_tree(&tree) {
            return fail(
                OutputError::new("IO_ERROR", e.to_string()).with_context("tree_id", tree_id),
            );
        }
        nbr_branch_warnings.extend(report.removed_branches.into_iter().map(|nbr_id| {
            OutputWarning::new(
                "NBR_BRANCH_REMOVED",
                format!(
                    "NBR branch '{}' of tree '{}' removed: its source node was deleted",
                    nbr_id, tree.id
                ),
            )
            .with_context("tree_id", tree.id.as_str())
            .with_context("nbr_id", nbr_id)
        }));
        macro_warnings.extend(
            report
                .removed_macros
                .into_iter()
                .map(|m| m.into_warning(&tree.id)),
        );
        total_removed_edges += report.edges_removed;
        affected_trees.push(tree_id);
    }

    // Inbound cross-tree refs (ADR-015): strip refs that point at removed nodes.
    let mut refs_stripped_warnings: Vec<OutputWarning> = Vec::new();
    for other_id in storage.list_node_ids().unwrap_or_default() {
        if id_set.contains(other_id.as_str()) {
            continue;
        }
        let Ok(mut other) = storage.load_node(&other_id) else {
            continue;
        };
        let stripped: BTreeSet<String> = other
            .metadata
            .refs
            .iter()
            .filter(|r| id_set.contains(r.node.as_str()))
            .map(|r| r.node.clone())
            .collect();
        if stripped.is_empty() {
            continue;
        }
        other
            .metadata
            .refs
            .retain(|r| !id_set.contains(r.node.as_str()));
        if let Err(e) = storage.save_node(&other) {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: "node_rm".to_string(),
                workspace: ws_name,
                data: NodeRmData {
                    removed_nodes: vec![],
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
        refs_stripped_warnings.push(
            OutputWarning::new(
                "REFS_STRIPPED",
                format!(
                    "Refs from '{}' to removed node(s) stripped: {}",
                    other_id,
                    stripped.iter().cloned().collect::<Vec<_>>().join(", ")
                ),
            )
            .with_context("referencing", serde_json::Value::String(other_id.clone()))
            .with_context(
                "node_ids",
                serde_json::Value::Array(
                    stripped
                        .into_iter()
                        .map(serde_json::Value::String)
                        .collect(),
                ),
            ),
        );
    }

    let mut removed_nodes = Vec::new();
    for id in &ids {
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
        removed_nodes.push(id.to_string());
    }

    let _ = storage.release_lock();

    let mut warnings = vec![];
    prepend_session_warnings(&mut warnings, &lock_outcome, None);
    warnings.extend(nbr_branch_warnings);
    warnings.extend(macro_warnings);
    warnings.extend(refs_stripped_warnings);

    // KNOWLEDGE_ORPHANED: check if any knowledge items link to the removed nodes.
    // An unreadable item warns after it and never blocks: `rm` does not write
    // knowledge (ADR-016 D-K5).
    let kn_pool = crate::knowledge::pool::load_pool(storage);
    let orphaned_kn_ids: Vec<String> = kn_pool
        .items
        .into_iter()
        .filter(|item| {
            item.links
                .iter()
                .any(|l| id_set.contains(l.target.as_str()))
        })
        .map(|item| item.id)
        .collect();

    if !orphaned_kn_ids.is_empty() {
        warnings.push(
            OutputWarning::new(
                "KNOWLEDGE_ORPHANED",
                format!(
                    "Knowledge items with dangling references after node removal: {}",
                    orphaned_kn_ids.join(", ")
                ),
            )
            .with_context(
                "knowledge_ids",
                serde_json::Value::Array(
                    orphaned_kn_ids
                        .iter()
                        .map(|id| serde_json::Value::String(id.clone()))
                        .collect(),
                ),
            ),
        );
    }
    warnings.extend(kn_pool.warnings);

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
    /// Outbound cross-tree refs (ADR-015), sorted.
    pub refs: Vec<CrossRef>,
    /// IDs of nodes whose refs point at this node, sorted.
    pub referenced_by: Vec<String>,
}

/// Execute `node inspect` command.
///
/// Shows which trees a node participates in, its role in each,
/// and all inbound/outbound edges per tree.
pub fn execute_node_inspect(storage: &dyn Storage, id: &str) -> CommandOutput<NodeInspectData> {
    let ws_name = storage.workspace_name().unwrap_or_default();

    let node = match storage.load_node(id) {
        Ok(n) => n,
        Err(e) => {
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
                    refs: vec![],
                    referenced_by: vec![],
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::load_failed(
                    &e,
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

    let referenced_by: Vec<String> = storage
        .list_node_ids()
        .unwrap_or_default()
        .into_iter()
        .filter(|other_id| other_id != id)
        .filter(|other_id| {
            storage
                .load_node(other_id)
                .is_ok_and(|other| other.metadata.refs.iter().any(|r| r.node == id))
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

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
            refs: node.metadata.refs,
            referenced_by,
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
    /// Trees rewritten by the split (ADR-016 D-1), sorted; always includes `tree_id`.
    pub affected_trees: Vec<String>,
}

/// Metadata inherited by each child of a split: the original's refs and extra
/// keys, with a fresh `active` status.
fn split_child_metadata(original: &NodeMetadata) -> NodeMetadata {
    NodeMetadata {
        refs: original.refs.clone(),
        extra: original.extra.clone(),
        ..NodeMetadata::new(NodeStatus::Active)
    }
}

/// Execute `node split` command.
///
/// Splits a node into two new nodes in **every** tree where it appears (ADR-016 D-1);
/// `tree_id` is the context tree and must have the node attached.
/// Inbound references (edge/feedback/macro `to`, NBR `source_node`) go to the first
/// child; outbound ones (edge `from`, feedback/macro `from`) leave from the second; in
/// `nodes[]` and macro interiors the children replace the original in place (D-2).
/// All trees are loaded before anything is written or any ID is minted (D-4).
/// Both children inherit the original's refs and extra metadata, and inbound
/// refs from other nodes are rewritten to point at both children (ADR-015).
/// The original node is removed from the pool last, so no intermediate state
/// leaves a dangling reference.
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
        affected_trees: vec![],
    };
    let fail = |error: OutputError| CommandOutput {
        success: false,
        action: "node_split".to_string(),
        workspace: ws_name.clone(),
        data: empty_data(),
        graph_health: GraphHealth {
            valid_dag: true,
            orphan_nodes_count: 0,
        },
        errors: vec![error],
        warnings: vec![],
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
                errors: vec![OutputError::load_failed(
                    &e,
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
    let mut notice = None;

    // D-4: load every tree before writing anything or minting IDs.
    let tree_ids = match storage.list_tree_ids() {
        Ok(ids) => ids,
        Err(e) => {
            let _ = storage.release_lock();
            return fail(OutputError::new("IO_ERROR", e.to_string()));
        }
    };
    if !tree_ids.iter().any(|t| t == tree_id) {
        let _ = storage.release_lock();
        return fail(OutputError::new(
            "TREE_NOT_FOUND",
            format!("Tree '{tree_id}' not found"),
        ));
    }
    let mut trees = Vec::with_capacity(tree_ids.len());
    for tid in tree_ids {
        match storage.load_tree(&tid) {
            Ok(t) => trees.push((tid, t)),
            Err(e) => {
                let _ = storage.release_lock();
                return fail(
                    OutputError::new("IO_ERROR", e.to_string()).with_context("tree_id", tid),
                );
            }
        }
    }

    let attached = trees
        .iter()
        .find(|(tid, _)| tid == tree_id)
        .is_some_and(|(_, t)| t.nodes.iter().any(|nr| nr.node_ref == id));
    if !attached {
        let _ = storage.release_lock();
        return fail(OutputError::new(
            "NODE_NOT_IN_TREE",
            format!("Node '{}' is not attached to tree '{}'", id, tree_id),
        ));
    }

    let type_prefix = original.node_type.prefix();
    let id_first = match storage.next_id(type_prefix) {
        Ok(m) => m.into_id(&mut notice),
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
        Ok(m) => m.into_id(&mut notice),
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
        metadata: split_child_metadata(&original.metadata),
    };
    let node_second = Node {
        id: id_second.clone(),
        node_type: original.node_type,
        label: labels[1].clone(),
        tags: original.tags.clone(),
        observable: original.observable,
        epistemic: EpistemicStatus::default(),
        metadata: split_child_metadata(&original.metadata),
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

    // D-2: redirect in memory; only the trees that change are written.
    let mut affected_trees = Vec::new();
    for (tid, mut tree) in trees {
        if !redirect_split(&mut tree, id, &id_first, &id_second) {
            continue;
        }
        if let Err(e) = storage.save_tree(&tree) {
            let _ = storage.release_lock();
            return fail(OutputError::new("IO_ERROR", e.to_string()).with_context("tree_id", tid));
        }
        affected_trees.push(tid);
    }

    // Inbound cross-tree refs (ADR-015): a ref to the split node now points at both children.
    for other_id in storage.list_node_ids().unwrap_or_default() {
        if other_id == id || other_id == id_first || other_id == id_second {
            continue;
        }
        let Ok(mut other) = storage.load_node(&other_id) else {
            continue;
        };
        let inbound: Vec<CrossRef> = other
            .metadata
            .refs
            .iter()
            .filter(|r| r.node == id)
            .cloned()
            .collect();
        if inbound.is_empty() {
            continue;
        }
        other.metadata.refs.retain(|r| r.node != id);
        for r in inbound {
            for child in [&id_first, &id_second] {
                other.metadata.add_ref(CrossRef {
                    node: child.clone(),
                    tree: r.tree.clone(),
                });
            }
        }
        if let Err(e) = storage.save_node(&other) {
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
    prepend_session_warnings(&mut warnings, &lock_outcome, notice.as_ref());

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
