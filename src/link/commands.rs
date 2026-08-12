use serde::Serialize;

use crate::errors::LtpError;
use crate::link::types::{Edge, EdgeStatus, FeedbackEdge, FeedbackLoopType, Logic, Operator};
use crate::output::{CommandOutput, GraphHealth, OutputError, OutputWarning};
use crate::storage::{LockOutcome, Storage};
use crate::validate::check_dag;

// --- Helpers ---

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

fn parse_feedback_type(s: &str) -> Option<FeedbackLoopType> {
    match s.to_lowercase().as_str() {
        "positive" => Some(FeedbackLoopType::Positive),
        "negative" => Some(FeedbackLoopType::Negative),
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

// --- Output data types ---

/// Data returned by `link connect`.
#[derive(Debug, Serialize)]
pub struct LinkConnectData {
    pub created_links: Vec<String>,
    pub tree_id: String,
}

/// Data returned by `link disconnect`.
#[derive(Debug, Serialize)]
pub struct LinkDisconnectData {
    pub removed_links: Vec<String>,
    pub tree_id: String,
}

/// Data returned by `link feedback`.
#[derive(Debug, Serialize)]
pub struct LinkFeedbackData {
    pub id: String,
    pub tree_id: String,
    pub from: String,
    pub to: String,
    pub loop_type: FeedbackLoopType,
}

// --- Command implementations ---

/// Execute `link connect`.
pub fn execute_link_connect(
    storage: &dyn Storage,
    tree_id: &str,
    from: &[String],
    to: &[String],
    operator: Option<&str>,
    weight: Option<f64>,
) -> CommandOutput<LinkConnectData> {
    let ws_name = storage.workspace_name().unwrap_or_default();

    let lock_outcome = match storage.acquire_lock("link connect") {
        Ok(o) => o,
        Err(e) => {
            return CommandOutput {
                success: false,
                action: "link_connect".to_string(),
                workspace: ws_name,
                data: LinkConnectData {
                    created_links: vec![],
                    tree_id: tree_id.to_string(),
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

    // Load tree
    let mut tree = match storage.load_tree(tree_id) {
        Ok(t) => t,
        Err(e) => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: "link_connect".to_string(),
                workspace: ws_name,
                data: LinkConnectData {
                    created_links: vec![],
                    tree_id: tree_id.to_string(),
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new("TREE_NOT_FOUND", e.to_string())],
                warnings: vec![],
            };
        }
    };

    // Validate all nodes exist in pool and are attached to tree
    let all_node_ids: Vec<&String> = from.iter().chain(to.iter()).collect();
    for node_id in &all_node_ids {
        if storage.load_node(node_id).is_err() {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: "link_connect".to_string(),
                workspace: ws_name,
                data: LinkConnectData {
                    created_links: vec![],
                    tree_id: tree_id.to_string(),
                },
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

        if !tree.nodes.iter().any(|n| &n.node_ref == *node_id) {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: "link_connect".to_string(),
                workspace: ws_name,
                data: LinkConnectData {
                    created_links: vec![],
                    tree_id: tree_id.to_string(),
                },
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

    // Determine operator
    let op = match operator {
        Some(s) => match parse_operator(s) {
            Some(o) => o,
            None => {
                let _ = storage.release_lock();
                return CommandOutput {
                    success: false,
                    action: "link_connect".to_string(),
                    workspace: ws_name,
                    data: LinkConnectData {
                        created_links: vec![],
                        tree_id: tree_id.to_string(),
                    },
                    graph_health: GraphHealth {
                        valid_dag: true,
                        orphan_nodes_count: 0,
                    },
                    errors: vec![OutputError::new(
                        "INVALID_OPERATOR",
                        format!("Unknown operator: {}", s),
                    )],
                    warnings: vec![],
                };
            }
        },
        None => {
            if from.len() > 1 {
                Operator::And
            } else {
                Operator::Single
            }
        }
    };

    let mut warnings: Vec<OutputWarning> = vec![];

    // MAG without weight warning
    if op == Operator::Mag && weight.is_none() {
        warnings.push(OutputWarning::new(
            "MAG_WEIGHT_MISSING",
            "Operator MAG without --weight; magnitude estimation pending",
        ));
    }

    // Build edges based on to[] cardinality
    let mut new_edges: Vec<Edge> = Vec::new();

    if to.len() > 1 {
        // Multiple destinations: create one SINGLE edge per destination
        for dest in to {
            let link_id = match storage.next_id("LINK") {
                Ok(id) => id,
                Err(e) => {
                    let _ = storage.release_lock();
                    return CommandOutput {
                        success: false,
                        action: "link_connect".to_string(),
                        workspace: ws_name,
                        data: LinkConnectData {
                            created_links: vec![],
                            tree_id: tree_id.to_string(),
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
            new_edges.push(Edge {
                id: link_id,
                from: from.to_vec(),
                to: dest.clone(),
                operator: Operator::Single,
                weight: None,
                status: EdgeStatus::Active,
                logic: Logic::Sufficiency,
                assumptions: vec![],
            });
        }
    } else {
        // Single destination
        let link_id = match storage.next_id("LINK") {
            Ok(id) => id,
            Err(e) => {
                let _ = storage.release_lock();
                return CommandOutput {
                    success: false,
                    action: "link_connect".to_string(),
                    workspace: ws_name,
                    data: LinkConnectData {
                        created_links: vec![],
                        tree_id: tree_id.to_string(),
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
        new_edges.push(Edge {
            id: link_id,
            from: from.to_vec(),
            to: to[0].clone(),
            operator: op,
            weight,
            status: EdgeStatus::Active,
            logic: Logic::Sufficiency,
            assumptions: vec![],
        });
    }

    // Validate DAG with new edges
    let mut all_edges: Vec<Edge> = tree.edges.clone();
    all_edges.extend(new_edges.iter().cloned());

    if let Err(e) = check_dag(&all_edges, tree_id) {
        let _ = storage.release_lock();
        let err = match &e {
            LtpError::CircularDependencyDetected { .. } => {
                OutputError::new("CIRCULAR_DEPENDENCY_DETECTED", e.to_string())
            }
            _ => OutputError::new("VALIDATION_ERROR", e.to_string()),
        };
        return CommandOutput {
            success: false,
            action: "link_connect".to_string(),
            workspace: ws_name,
            data: LinkConnectData {
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

    let created_ids: Vec<String> = new_edges.iter().map(|e| e.id.clone()).collect();
    tree.edges.extend(new_edges);

    if let Err(e) = storage.save_tree(&tree) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: "link_connect".to_string(),
            workspace: ws_name,
            data: LinkConnectData {
                created_links: vec![],
                tree_id: tree_id.to_string(),
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

    if let Some(w) = stale_lock_warning(&lock_outcome) {
        warnings.insert(0, w);
    }

    CommandOutput {
        success: true,
        action: "link_connect".to_string(),
        workspace: ws_name,
        data: LinkConnectData {
            created_links: created_ids,
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

/// Execute `link disconnect`.
pub fn execute_link_disconnect(
    storage: &dyn Storage,
    tree_id: &str,
    link_ids: &[String],
) -> CommandOutput<LinkDisconnectData> {
    let ws_name = storage.workspace_name().unwrap_or_default();

    let lock_outcome = match storage.acquire_lock("link disconnect") {
        Ok(o) => o,
        Err(e) => {
            return CommandOutput {
                success: false,
                action: "link_disconnect".to_string(),
                workspace: ws_name,
                data: LinkDisconnectData {
                    removed_links: vec![],
                    tree_id: tree_id.to_string(),
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

    let mut tree = match storage.load_tree(tree_id) {
        Ok(t) => t,
        Err(e) => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: "link_disconnect".to_string(),
                workspace: ws_name,
                data: LinkDisconnectData {
                    removed_links: vec![],
                    tree_id: tree_id.to_string(),
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new("TREE_NOT_FOUND", e.to_string())],
                warnings: vec![],
            };
        }
    };

    let mut removed = Vec::new();
    for link_id in link_ids {
        if tree.edges.iter().any(|e| e.id == *link_id) {
            tree.edges.retain(|e| e.id != *link_id);
            removed.push(link_id.clone());
        }
    }

    if let Err(e) = storage.save_tree(&tree) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: "link_disconnect".to_string(),
            workspace: ws_name,
            data: LinkDisconnectData {
                removed_links: vec![],
                tree_id: tree_id.to_string(),
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

    let mut warnings = vec![];
    if let Some(w) = stale_lock_warning(&lock_outcome) {
        warnings.push(w);
    }

    CommandOutput {
        success: true,
        action: "link_disconnect".to_string(),
        workspace: ws_name,
        data: LinkDisconnectData {
            removed_links: removed,
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

/// Execute `link feedback`.
pub fn execute_link_feedback(
    storage: &dyn Storage,
    tree_id: &str,
    from: &str,
    to: &str,
    loop_type_str: &str,
    label: Option<&str>,
) -> CommandOutput<LinkFeedbackData> {
    let ws_name = storage.workspace_name().unwrap_or_default();

    let loop_type = match parse_feedback_type(loop_type_str) {
        Some(t) => t,
        None => {
            return CommandOutput {
                success: false,
                action: "link_feedback".to_string(),
                workspace: ws_name,
                data: LinkFeedbackData {
                    id: String::new(),
                    tree_id: tree_id.to_string(),
                    from: from.to_string(),
                    to: to.to_string(),
                    loop_type: FeedbackLoopType::Positive,
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new(
                    "INVALID_FEEDBACK_TYPE",
                    format!("Expected 'positive' or 'negative', got '{}'", loop_type_str),
                )],
                warnings: vec![],
            };
        }
    };

    let lock_outcome = match storage.acquire_lock("link feedback") {
        Ok(o) => o,
        Err(e) => {
            return CommandOutput {
                success: false,
                action: "link_feedback".to_string(),
                workspace: ws_name,
                data: LinkFeedbackData {
                    id: String::new(),
                    tree_id: tree_id.to_string(),
                    from: from.to_string(),
                    to: to.to_string(),
                    loop_type,
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

    let mut tree = match storage.load_tree(tree_id) {
        Ok(t) => t,
        Err(e) => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: "link_feedback".to_string(),
                workspace: ws_name,
                data: LinkFeedbackData {
                    id: String::new(),
                    tree_id: tree_id.to_string(),
                    from: from.to_string(),
                    to: to.to_string(),
                    loop_type,
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new("TREE_NOT_FOUND", e.to_string())],
                warnings: vec![],
            };
        }
    };

    let fb_id = match storage.next_id("FB") {
        Ok(id) => id,
        Err(e) => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: "link_feedback".to_string(),
                workspace: ws_name,
                data: LinkFeedbackData {
                    id: String::new(),
                    tree_id: tree_id.to_string(),
                    from: from.to_string(),
                    to: to.to_string(),
                    loop_type,
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

    tree.feedback_edges.push(FeedbackEdge {
        id: fb_id.clone(),
        from: from.to_string(),
        to: to.to_string(),
        loop_type,
        label: label.map(String::from),
    });

    if let Err(e) = storage.save_tree(&tree) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: "link_feedback".to_string(),
            workspace: ws_name,
            data: LinkFeedbackData {
                id: String::new(),
                tree_id: tree_id.to_string(),
                from: from.to_string(),
                to: to.to_string(),
                loop_type,
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

    let mut warnings = vec![];
    if let Some(w) = stale_lock_warning(&lock_outcome) {
        warnings.push(w);
    }

    CommandOutput {
        success: true,
        action: "link_feedback".to_string(),
        workspace: ws_name,
        data: LinkFeedbackData {
            id: fb_id,
            tree_id: tree_id.to_string(),
            from: from.to_string(),
            to: to.to_string(),
            loop_type,
        },
        graph_health: GraphHealth {
            valid_dag: true,
            orphan_nodes_count: 0,
        },
        errors: vec![],
        warnings,
    }
}
