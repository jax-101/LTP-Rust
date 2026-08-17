use serde::Serialize;

use crate::link::types::{Assumption, AssumptionStatus, EdgeStatus};
use crate::node::{EpistemicStatus, Node, NodeMetadata, NodeStatus, NodeType};
use crate::output::{CommandOutput, GraphHealth, OutputError, OutputWarning};
use crate::storage::{LockOutcome, Storage};

// --- Helpers ---

fn stale_lock_warning(outcome: &LockOutcome) -> Option<OutputWarning> {
    match outcome {
        LockOutcome::StaleLockRemoved { pid } => Some(OutputWarning::new(
            "STALE_LOCK_REMOVED",
            format!("Stale lock from PID {} was removed", pid),
        )),
        LockOutcome::Acquired => None,
    }
}

fn parse_assumption_status(s: &str) -> Option<AssumptionStatus> {
    match s.to_lowercase().as_str() {
        "valid" => Some(AssumptionStatus::Valid),
        "invalid" => Some(AssumptionStatus::Invalid),
        "needs_review" => Some(AssumptionStatus::NeedsReview),
        _ => None,
    }
}

// --- Output data types ---

/// Data returned by `assume add`.
#[derive(Debug, Serialize)]
pub struct AssumeAddData {
    pub id: String,
    pub link_id: String,
    pub tree_id: String,
    pub text: String,
}

/// Data returned by `assume edit`.
#[derive(Debug, Serialize)]
pub struct AssumeEditData {
    pub id: String,
    pub tree_id: String,
    pub text: String,
}

/// Data returned by `assume list`.
#[derive(Debug, Serialize)]
pub struct AssumeListData {
    pub tree_id: String,
    pub assumptions: Vec<AssumeListEntry>,
}

/// Single entry in the `assume list` result.
#[derive(Debug, Serialize)]
pub struct AssumeListEntry {
    pub id: String,
    pub text: String,
    pub status: AssumptionStatus,
    pub link_id: String,
}

/// Data returned by `assume move`.
#[derive(Debug, Serialize)]
pub struct AssumeMoveData {
    pub id: String,
    pub from_link: String,
    pub to_link: String,
    pub tree_id: String,
}

/// Data returned by `assume rm`.
#[derive(Debug, Serialize)]
pub struct AssumeRmData {
    pub id: String,
    pub tree_id: String,
}

/// Data returned by `invalidate`.
#[derive(Debug, Serialize)]
pub struct InvalidateData {
    pub asm_id: String,
    pub link_id: String,
    pub link_status: EdgeStatus,
    pub injection_id: Option<String>,
    pub changed: bool,
}

// --- Command implementations ---

/// Execute `assume add`: creates a new assumption on an edge.
pub fn execute_assume_add(
    storage: &dyn Storage,
    tree_id: &str,
    link_id: &str,
    text: &str,
) -> CommandOutput<AssumeAddData> {
    let ws_name = storage.workspace_name().unwrap_or_default();
    let action = "assume_add";

    let lock_outcome = match storage.acquire_lock("assume add") {
        Ok(o) => o,
        Err(e) => {
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: AssumeAddData {
                    id: String::new(),
                    link_id: link_id.to_string(),
                    tree_id: tree_id.to_string(),
                    text: String::new(),
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
        Err(_) => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: AssumeAddData {
                    id: String::new(),
                    link_id: link_id.to_string(),
                    tree_id: tree_id.to_string(),
                    text: String::new(),
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

    let edge = match tree.edges.iter_mut().find(|e| e.id == link_id) {
        Some(e) => e,
        None => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: AssumeAddData {
                    id: String::new(),
                    link_id: link_id.to_string(),
                    tree_id: tree_id.to_string(),
                    text: String::new(),
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new(
                    "LINK_NOT_FOUND",
                    format!("Edge '{}' not found in tree '{}'", link_id, tree_id),
                )],
                warnings: vec![],
            };
        }
    };

    let asm_id = match storage.next_id("ASM") {
        Ok(id) => id,
        Err(e) => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: AssumeAddData {
                    id: String::new(),
                    link_id: link_id.to_string(),
                    tree_id: tree_id.to_string(),
                    text: String::new(),
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

    edge.assumptions.push(Assumption {
        id: asm_id.clone(),
        status: AssumptionStatus::Valid,
        text: text.to_string(),
    });

    if let Err(e) = storage.save_tree(&tree) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: AssumeAddData {
                id: String::new(),
                link_id: link_id.to_string(),
                tree_id: tree_id.to_string(),
                text: String::new(),
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
        action: action.to_string(),
        workspace: ws_name,
        data: AssumeAddData {
            id: asm_id,
            link_id: link_id.to_string(),
            tree_id: tree_id.to_string(),
            text: text.to_string(),
        },
        graph_health: GraphHealth {
            valid_dag: true,
            orphan_nodes_count: 0,
        },
        errors: vec![],
        warnings,
    }
}

/// Execute `assume edit`: updates an assumption's text.
pub fn execute_assume_edit(
    storage: &dyn Storage,
    tree_id: &str,
    asm_id: &str,
    new_text: &str,
) -> CommandOutput<AssumeEditData> {
    let ws_name = storage.workspace_name().unwrap_or_default();
    let action = "assume_edit";

    let lock_outcome = match storage.acquire_lock("assume edit") {
        Ok(o) => o,
        Err(e) => {
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: AssumeEditData {
                    id: asm_id.to_string(),
                    tree_id: tree_id.to_string(),
                    text: String::new(),
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
        Err(_) => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: AssumeEditData {
                    id: asm_id.to_string(),
                    tree_id: tree_id.to_string(),
                    text: String::new(),
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

    let mut found = false;
    for edge in &mut tree.edges {
        if let Some(asm) = edge.assumptions.iter_mut().find(|a| a.id == asm_id) {
            asm.text = new_text.to_string();
            found = true;
            break;
        }
    }

    if !found {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: AssumeEditData {
                id: asm_id.to_string(),
                tree_id: tree_id.to_string(),
                text: String::new(),
            },
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![OutputError::new(
                "ASSUMPTION_NOT_FOUND",
                format!("Assumption '{}' not found in tree '{}'", asm_id, tree_id),
            )],
            warnings: vec![],
        };
    }

    if let Err(e) = storage.save_tree(&tree) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: AssumeEditData {
                id: asm_id.to_string(),
                tree_id: tree_id.to_string(),
                text: String::new(),
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
        action: action.to_string(),
        workspace: ws_name,
        data: AssumeEditData {
            id: asm_id.to_string(),
            tree_id: tree_id.to_string(),
            text: new_text.to_string(),
        },
        graph_health: GraphHealth {
            valid_dag: true,
            orphan_nodes_count: 0,
        },
        errors: vec![],
        warnings,
    }
}

/// Execute `assume list`: lists assumptions, optionally filtered by status.
pub fn execute_assume_list(
    storage: &dyn Storage,
    tree_id: &str,
    status_filter: Option<&str>,
) -> CommandOutput<AssumeListData> {
    let ws_name = storage.workspace_name().unwrap_or_default();
    let action = "assume_list";

    let tree = match storage.load_tree(tree_id) {
        Ok(t) => t,
        Err(_) => {
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: AssumeListData {
                    tree_id: tree_id.to_string(),
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

    let filter = status_filter.and_then(parse_assumption_status);

    let mut assumptions = vec![];
    for edge in &tree.edges {
        for asm in &edge.assumptions {
            if let Some(ref f) = filter {
                if asm.status != *f {
                    continue;
                }
            }
            assumptions.push(AssumeListEntry {
                id: asm.id.clone(),
                text: asm.text.clone(),
                status: asm.status,
                link_id: edge.id.clone(),
            });
        }
    }

    CommandOutput::ok(
        action,
        &ws_name,
        AssumeListData {
            tree_id: tree_id.to_string(),
            assumptions,
        },
    )
}

/// Execute `assume move`: moves an assumption from one edge to another.
pub fn execute_assume_move(
    storage: &dyn Storage,
    tree_id: &str,
    asm_id: &str,
    to_link_id: &str,
) -> CommandOutput<AssumeMoveData> {
    let ws_name = storage.workspace_name().unwrap_or_default();
    let action = "assume_move";

    let lock_outcome = match storage.acquire_lock("assume move") {
        Ok(o) => o,
        Err(e) => {
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: AssumeMoveData {
                    id: asm_id.to_string(),
                    from_link: String::new(),
                    to_link: to_link_id.to_string(),
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
        Err(_) => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: AssumeMoveData {
                    id: asm_id.to_string(),
                    from_link: String::new(),
                    to_link: to_link_id.to_string(),
                    tree_id: tree_id.to_string(),
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

    // Find and remove the assumption from its current edge
    let mut from_link = String::new();
    let mut assumption: Option<Assumption> = None;
    for edge in &mut tree.edges {
        if let Some(pos) = edge.assumptions.iter().position(|a| a.id == asm_id) {
            from_link = edge.id.clone();
            assumption = Some(edge.assumptions.remove(pos));
            break;
        }
    }

    let asm = match assumption {
        Some(a) => a,
        None => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: AssumeMoveData {
                    id: asm_id.to_string(),
                    from_link: String::new(),
                    to_link: to_link_id.to_string(),
                    tree_id: tree_id.to_string(),
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new(
                    "ASSUMPTION_NOT_FOUND",
                    format!("Assumption '{}' not found in tree '{}'", asm_id, tree_id),
                )],
                warnings: vec![],
            };
        }
    };

    // Find target edge and push assumption
    let target = match tree.edges.iter_mut().find(|e| e.id == to_link_id) {
        Some(e) => e,
        None => {
            // Restore assumption to source edge before returning error
            if let Some(src_edge) = tree.edges.iter_mut().find(|e| e.id == from_link) {
                src_edge.assumptions.push(asm);
            }
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: AssumeMoveData {
                    id: asm_id.to_string(),
                    from_link,
                    to_link: to_link_id.to_string(),
                    tree_id: tree_id.to_string(),
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new(
                    "LINK_NOT_FOUND",
                    format!(
                        "Target edge '{}' not found in tree '{}'",
                        to_link_id, tree_id
                    ),
                )],
                warnings: vec![],
            };
        }
    };
    target.assumptions.push(asm);

    if let Err(e) = storage.save_tree(&tree) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: AssumeMoveData {
                id: asm_id.to_string(),
                from_link,
                to_link: to_link_id.to_string(),
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
        action: action.to_string(),
        workspace: ws_name,
        data: AssumeMoveData {
            id: asm_id.to_string(),
            from_link,
            to_link: to_link_id.to_string(),
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

/// Execute `assume rm`: removes an assumption from its edge.
pub fn execute_assume_rm(
    storage: &dyn Storage,
    tree_id: &str,
    asm_id: &str,
) -> CommandOutput<AssumeRmData> {
    let ws_name = storage.workspace_name().unwrap_or_default();
    let action = "assume_rm";

    let lock_outcome = match storage.acquire_lock("assume rm") {
        Ok(o) => o,
        Err(e) => {
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: AssumeRmData {
                    id: asm_id.to_string(),
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
        Err(_) => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: AssumeRmData {
                    id: asm_id.to_string(),
                    tree_id: tree_id.to_string(),
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

    let mut found = false;
    for edge in &mut tree.edges {
        if let Some(pos) = edge.assumptions.iter().position(|a| a.id == asm_id) {
            edge.assumptions.remove(pos);
            found = true;
            break;
        }
    }

    if !found {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: AssumeRmData {
                id: asm_id.to_string(),
                tree_id: tree_id.to_string(),
            },
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![OutputError::new(
                "ASSUMPTION_NOT_FOUND",
                format!("Assumption '{}' not found in tree '{}'", asm_id, tree_id),
            )],
            warnings: vec![],
        };
    }

    if let Err(e) = storage.save_tree(&tree) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: AssumeRmData {
                id: asm_id.to_string(),
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
        action: action.to_string(),
        workspace: ws_name,
        data: AssumeRmData {
            id: asm_id.to_string(),
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

/// Execute `invalidate`: marks an assumption as invalid, breaks its edge,
/// and optionally creates an injection node.
pub fn execute_invalidate(
    storage: &dyn Storage,
    tree_id: &str,
    link_id: &str,
    asm_id: &str,
    injection_label: Option<&str>,
) -> CommandOutput<InvalidateData> {
    let ws_name = storage.workspace_name().unwrap_or_default();
    let action = "invalidate";

    let lock_outcome = match storage.acquire_lock("invalidate") {
        Ok(o) => o,
        Err(e) => {
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: InvalidateData {
                    asm_id: asm_id.to_string(),
                    link_id: link_id.to_string(),
                    link_status: EdgeStatus::Active,
                    injection_id: None,
                    changed: false,
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
        Err(_) => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: InvalidateData {
                    asm_id: asm_id.to_string(),
                    link_id: link_id.to_string(),
                    link_status: EdgeStatus::Active,
                    injection_id: None,
                    changed: false,
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

    let edge = match tree.edges.iter_mut().find(|e| e.id == link_id) {
        Some(e) => e,
        None => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: InvalidateData {
                    asm_id: asm_id.to_string(),
                    link_id: link_id.to_string(),
                    link_status: EdgeStatus::Active,
                    injection_id: None,
                    changed: false,
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new(
                    "LINK_NOT_FOUND",
                    format!("Edge '{}' not found in tree '{}'", link_id, tree_id),
                )],
                warnings: vec![],
            };
        }
    };

    let asm = match edge.assumptions.iter_mut().find(|a| a.id == asm_id) {
        Some(a) => a,
        None => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: InvalidateData {
                    asm_id: asm_id.to_string(),
                    link_id: link_id.to_string(),
                    link_status: edge.status,
                    injection_id: None,
                    changed: false,
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new(
                    "ASSUMPTION_NOT_IN_LINK",
                    format!("Assumption '{}' not found in edge '{}'", asm_id, link_id),
                )],
                warnings: vec![],
            };
        }
    };

    // Idempotency check (ADR-010)
    if asm.status == AssumptionStatus::Invalid && edge.status == EdgeStatus::Broken {
        let _ = storage.release_lock();
        let mut warnings = vec![OutputWarning::new(
            "ALREADY_INVALIDATED",
            format!(
                "Assumption '{}' is already invalid and edge '{}' is already broken",
                asm_id, link_id
            ),
        )];
        if let Some(w) = stale_lock_warning(&lock_outcome) {
            warnings.push(w);
        }
        return CommandOutput {
            success: true,
            action: action.to_string(),
            workspace: ws_name,
            data: InvalidateData {
                asm_id: asm_id.to_string(),
                link_id: link_id.to_string(),
                link_status: EdgeStatus::Broken,
                injection_id: None,
                changed: false,
            },
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![],
            warnings,
        };
    }

    // Auto-repair: handle inconsistent states with warning
    let mut warnings: Vec<OutputWarning> = vec![];
    if asm.status == AssumptionStatus::Invalid && edge.status != EdgeStatus::Broken {
        warnings.push(OutputWarning::new(
            "STATE_REPAIRED",
            format!(
                "Assumption '{}' was invalid but edge '{}' was not broken — repaired",
                asm_id, link_id
            ),
        ));
    } else if asm.status != AssumptionStatus::Invalid && edge.status == EdgeStatus::Broken {
        warnings.push(OutputWarning::new(
            "STATE_REPAIRED",
            format!(
                "Edge '{}' was broken but assumption '{}' was not invalid — repaired",
                link_id, asm_id
            ),
        ));
    }

    // Apply the invalidation
    asm.status = AssumptionStatus::Invalid;
    edge.status = EdgeStatus::Broken;

    // Create injection node if label provided
    let injection_id = if let Some(label) = injection_label {
        let inj_id = match storage.next_id("INJ") {
            Ok(id) => id,
            Err(e) => {
                let _ = storage.release_lock();
                return CommandOutput {
                    success: false,
                    action: action.to_string(),
                    workspace: ws_name,
                    data: InvalidateData {
                        asm_id: asm_id.to_string(),
                        link_id: link_id.to_string(),
                        link_status: EdgeStatus::Broken,
                        injection_id: None,
                        changed: false,
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

        let inj_node = Node {
            id: inj_id.clone(),
            node_type: NodeType::Inj,
            label: label.to_string(),
            tags: vec![],
            observable: true,
            epistemic: EpistemicStatus::default(),
            metadata: NodeMetadata {
                status: NodeStatus::Active,
                extra: Default::default(),
            },
        };

        if let Err(e) = storage.save_node(&inj_node) {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: InvalidateData {
                    asm_id: asm_id.to_string(),
                    link_id: link_id.to_string(),
                    link_status: EdgeStatus::Broken,
                    injection_id: None,
                    changed: false,
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new("IO_ERROR", e.to_string())],
                warnings: vec![],
            };
        }

        Some(inj_id)
    } else {
        None
    };

    if let Err(e) = storage.save_tree(&tree) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: InvalidateData {
                asm_id: asm_id.to_string(),
                link_id: link_id.to_string(),
                link_status: EdgeStatus::Broken,
                injection_id: None,
                changed: false,
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
        warnings.push(w);
    }

    CommandOutput {
        success: true,
        action: action.to_string(),
        workspace: ws_name,
        data: InvalidateData {
            asm_id: asm_id.to_string(),
            link_id: link_id.to_string(),
            link_status: EdgeStatus::Broken,
            injection_id,
            changed: true,
        },
        graph_health: GraphHealth {
            valid_dag: true,
            orphan_nodes_count: 0,
        },
        errors: vec![],
        warnings,
    }
}
