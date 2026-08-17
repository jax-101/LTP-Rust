use serde::Serialize;

use crate::knowledge::types::{
    Confidence, KnowledgeItem, KnowledgeLink, KnowledgeSource, KnowledgeStatus, KnowledgeType,
};
use crate::output::{CommandOutput, GraphHealth, OutputError, OutputWarning};
use crate::storage::{LockOutcome, Storage};

/// Data returned by `knowledge add`.
#[derive(Debug, Serialize)]
pub struct KnowledgeAddData {
    pub created_knowledge_id: String,
    #[serde(rename = "type")]
    pub knowledge_type: KnowledgeType,
    pub label: String,
    pub status: KnowledgeStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<Confidence>,
}

/// Data returned by `knowledge edit`.
#[derive(Debug, Serialize)]
pub struct KnowledgeEditData {
    pub id: String,
    pub label: String,
    pub status: KnowledgeStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<Confidence>,
    pub tags: Vec<String>,
}

/// Data returned by `knowledge rm`.
#[derive(Debug, Serialize)]
pub struct KnowledgeRmData {
    pub removed: Vec<String>,
    pub failed: Vec<String>,
}

/// Data returned by `knowledge inspect`.
#[derive(Debug, Serialize)]
pub struct KnowledgeInspectData {
    pub id: String,
    #[serde(rename = "type")]
    pub knowledge_type: KnowledgeType,
    pub label: String,
    pub status: KnowledgeStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<Confidence>,
    pub source: KnowledgeSource,
    pub captured: String,
    pub links: Vec<KnowledgeLink>,
    pub tags: Vec<String>,
}

/// Summary of a knowledge item for listing.
#[derive(Debug, Serialize)]
pub struct KnowledgeSummary {
    pub id: String,
    #[serde(rename = "type")]
    pub knowledge_type: KnowledgeType,
    pub label: String,
    pub status: KnowledgeStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<Confidence>,
    pub link_count: usize,
    pub tags: Vec<String>,
}

/// Data returned by `knowledge list`.
#[derive(Debug, Serialize)]
pub struct KnowledgeListData {
    pub items: Vec<KnowledgeSummary>,
    pub total: usize,
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

fn empty_add_data() -> KnowledgeAddData {
    KnowledgeAddData {
        created_knowledge_id: String::new(),
        knowledge_type: KnowledgeType::Observation,
        label: String::new(),
        status: KnowledgeStatus::Unverified,
        confidence: None,
    }
}

/// Execute `knowledge add`.
#[allow(clippy::too_many_arguments)]
pub fn execute_knowledge_add(
    storage: &dyn Storage,
    label: &str,
    knowledge_type: KnowledgeType,
    source_uri: Option<&str>,
    source_excerpt: Option<&str>,
    status: Option<KnowledgeStatus>,
    confidence: Option<Confidence>,
    tags: Option<Vec<String>>,
) -> CommandOutput<KnowledgeAddData> {
    let ws_name = storage.workspace_name().unwrap_or_default();

    if label.is_empty() {
        return CommandOutput {
            success: false,
            action: "knowledge_add".to_string(),
            workspace: ws_name,
            data: empty_add_data(),
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![OutputError::new("LABEL_REQUIRED", "Label cannot be empty")],
            warnings: vec![],
        };
    }

    let uri = source_uri.and_then(|s| {
        if s.is_empty() {
            None
        } else {
            Some(s.to_string())
        }
    });
    let excerpt = source_excerpt.and_then(|s| {
        if s.is_empty() {
            None
        } else {
            Some(s.to_string())
        }
    });

    let source = KnowledgeSource { uri, excerpt };
    if !source.is_valid() {
        return CommandOutput {
            success: false,
            action: "knowledge_add".to_string(),
            workspace: ws_name,
            data: empty_add_data(),
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![OutputError::new(
                "SOURCE_REQUIRED",
                "At least one of source-uri or source-excerpt must be provided",
            )],
            warnings: vec![],
        };
    }

    let lock_outcome = match storage.acquire_lock("knowledge add") {
        Ok(outcome) => outcome,
        Err(e) => {
            return CommandOutput {
                success: false,
                action: "knowledge_add".to_string(),
                workspace: ws_name,
                data: empty_add_data(),
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new("LOCK_ERROR", e.to_string())],
                warnings: vec![],
            };
        }
    };

    let id = match storage.next_id("KN") {
        Ok(id) => id,
        Err(e) => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: "knowledge_add".to_string(),
                workspace: ws_name,
                data: empty_add_data(),
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new("ID_GENERATION_ERROR", e.to_string())],
                warnings: vec![],
            };
        }
    };

    let effective_status = status.unwrap_or(KnowledgeStatus::Unverified);
    let effective_confidence = confidence.or(Some(Confidence::Medium));
    let effective_tags = tags.unwrap_or_default();
    let captured = chrono::Utc::now().format("%Y-%m-%d").to_string();

    let item = KnowledgeItem {
        id: id.clone(),
        knowledge_type,
        label: label.to_string(),
        status: effective_status,
        confidence: effective_confidence,
        source,
        captured,
        links: vec![],
        tags: effective_tags,
    };

    if let Err(e) = storage.save_knowledge(&item) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: "knowledge_add".to_string(),
            workspace: ws_name,
            data: empty_add_data(),
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![OutputError::new("IO_ERROR", e.to_string())],
            warnings: vec![],
        };
    }

    let _ = storage.release_lock();

    let mut warnings = Vec::new();
    if let Some(w) = stale_lock_warning(&lock_outcome) {
        warnings.push(w);
    }

    CommandOutput {
        success: true,
        action: "knowledge_add".to_string(),
        workspace: ws_name,
        data: KnowledgeAddData {
            created_knowledge_id: id,
            knowledge_type,
            label: label.to_string(),
            status: effective_status,
            confidence: effective_confidence,
        },
        graph_health: GraphHealth {
            valid_dag: true,
            orphan_nodes_count: 0,
        },
        errors: vec![],
        warnings,
    }
}

/// Execute `knowledge edit`.
#[allow(clippy::too_many_arguments)]
pub fn execute_knowledge_edit(
    storage: &dyn Storage,
    id: &str,
    label: Option<&str>,
    status: Option<KnowledgeStatus>,
    confidence: Option<Confidence>,
    source_uri: Option<&str>,
    source_excerpt: Option<&str>,
    add_tags: Option<Vec<String>>,
    rm_tags: Option<Vec<String>>,
) -> CommandOutput<KnowledgeEditData> {
    let ws_name = storage.workspace_name().unwrap_or_default();

    let empty_edit = || KnowledgeEditData {
        id: id.to_string(),
        label: String::new(),
        status: KnowledgeStatus::Unverified,
        confidence: None,
        tags: vec![],
    };

    if let Some(l) = label {
        if l.is_empty() {
            return CommandOutput {
                success: false,
                action: "knowledge_edit".to_string(),
                workspace: ws_name,
                data: empty_edit(),
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new("LABEL_REQUIRED", "Label cannot be empty")],
                warnings: vec![],
            };
        }
    }

    let lock_outcome = match storage.acquire_lock("knowledge edit") {
        Ok(outcome) => outcome,
        Err(e) => {
            return CommandOutput {
                success: false,
                action: "knowledge_edit".to_string(),
                workspace: ws_name,
                data: empty_edit(),
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new("LOCK_ERROR", e.to_string())],
                warnings: vec![],
            };
        }
    };

    let mut item = match storage.load_knowledge(id) {
        Ok(item) => item,
        Err(_) => {
            let _ = storage.release_lock();
            return CommandOutput {
                success: false,
                action: "knowledge_edit".to_string(),
                workspace: ws_name,
                data: empty_edit(),
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new(
                    "KNOWLEDGE_NOT_FOUND",
                    format!("Knowledge item '{}' not found", id),
                )],
                warnings: vec![],
            };
        }
    };

    if let Some(l) = label {
        item.label = l.to_string();
    }
    if let Some(s) = status {
        item.status = s;
    }
    if let Some(c) = confidence {
        item.confidence = Some(c);
    }

    let uri_provided = source_uri.is_some();
    let excerpt_provided = source_excerpt.is_some();

    if uri_provided {
        let val = source_uri.unwrap_or("");
        item.source.uri = if val.is_empty() {
            None
        } else {
            Some(val.to_string())
        };
    }
    if excerpt_provided {
        let val = source_excerpt.unwrap_or("");
        item.source.excerpt = if val.is_empty() {
            None
        } else {
            Some(val.to_string())
        };
    }

    if (uri_provided || excerpt_provided) && !item.source.is_valid() {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: "knowledge_edit".to_string(),
            workspace: ws_name,
            data: empty_edit(),
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![OutputError::new(
                "SOURCE_REQUIRED",
                "Cannot clear both source-uri and source-excerpt",
            )],
            warnings: vec![],
        };
    }

    let mut warnings = Vec::new();

    if let Some(tags_to_add) = add_tags {
        for tag in tags_to_add {
            if !tag.is_empty() && !item.tags.contains(&tag) {
                item.tags.push(tag);
            }
        }
    }

    if let Some(tags_to_rm) = rm_tags {
        for tag in &tags_to_rm {
            if !item.tags.contains(tag) {
                warnings.push(OutputWarning::new(
                    "TAG_NOT_FOUND",
                    format!("Tag '{}' not found on item {}", tag, id),
                ));
            }
        }
        item.tags.retain(|t| !tags_to_rm.contains(t));
    }

    if let Err(e) = storage.save_knowledge(&item) {
        let _ = storage.release_lock();
        return CommandOutput {
            success: false,
            action: "knowledge_edit".to_string(),
            workspace: ws_name,
            data: empty_edit(),
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
        action: "knowledge_edit".to_string(),
        workspace: ws_name,
        data: KnowledgeEditData {
            id: item.id,
            label: item.label,
            status: item.status,
            confidence: item.confidence,
            tags: item.tags,
        },
        graph_health: GraphHealth {
            valid_dag: true,
            orphan_nodes_count: 0,
        },
        errors: vec![],
        warnings,
    }
}

/// Execute `knowledge rm`.
pub fn execute_knowledge_rm(
    storage: &dyn Storage,
    ids: &[String],
) -> CommandOutput<KnowledgeRmData> {
    let ws_name = storage.workspace_name().unwrap_or_default();

    let lock_outcome = match storage.acquire_lock("knowledge rm") {
        Ok(outcome) => outcome,
        Err(e) => {
            return CommandOutput {
                success: false,
                action: "knowledge_rm".to_string(),
                workspace: ws_name,
                data: KnowledgeRmData {
                    removed: vec![],
                    failed: vec![],
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

    let mut removed = Vec::new();
    let mut failed = Vec::new();
    let mut errors = Vec::new();

    for id in ids {
        match storage.delete_knowledge(id) {
            Ok(()) => removed.push(id.clone()),
            Err(_) => {
                failed.push(id.clone());
                errors.push(
                    OutputError::new(
                        "KNOWLEDGE_NOT_FOUND",
                        format!("Knowledge item '{}' not found", id),
                    )
                    .with_context("id", serde_json::Value::String(id.clone())),
                );
            }
        }
    }

    let _ = storage.release_lock();

    let success = !removed.is_empty() || errors.is_empty();

    let mut warnings = Vec::new();
    if let Some(w) = stale_lock_warning(&lock_outcome) {
        warnings.push(w);
    }

    CommandOutput {
        success,
        action: "knowledge_rm".to_string(),
        workspace: ws_name,
        data: KnowledgeRmData { removed, failed },
        graph_health: GraphHealth {
            valid_dag: true,
            orphan_nodes_count: 0,
        },
        errors,
        warnings,
    }
}

/// Execute `knowledge inspect`.
pub fn execute_knowledge_inspect(
    storage: &dyn Storage,
    id: &str,
) -> CommandOutput<KnowledgeInspectData> {
    let ws_name = storage.workspace_name().unwrap_or_default();

    let empty_inspect = || KnowledgeInspectData {
        id: id.to_string(),
        knowledge_type: KnowledgeType::Observation,
        label: String::new(),
        status: KnowledgeStatus::Unverified,
        confidence: None,
        source: KnowledgeSource {
            uri: None,
            excerpt: None,
        },
        captured: String::new(),
        links: vec![],
        tags: vec![],
    };

    let item = match storage.load_knowledge(id) {
        Ok(item) => item,
        Err(_) => {
            return CommandOutput {
                success: false,
                action: "knowledge_inspect".to_string(),
                workspace: ws_name,
                data: empty_inspect(),
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new(
                    "KNOWLEDGE_NOT_FOUND",
                    format!("Knowledge item '{}' not found", id),
                )],
                warnings: vec![],
            };
        }
    };

    CommandOutput::ok(
        "knowledge_inspect",
        &ws_name,
        KnowledgeInspectData {
            id: item.id,
            knowledge_type: item.knowledge_type,
            label: item.label,
            status: item.status,
            confidence: item.confidence,
            source: item.source,
            captured: item.captured,
            links: item.links,
            tags: item.tags,
        },
    )
}

/// Execute `knowledge list`.
pub fn execute_knowledge_list(
    storage: &dyn Storage,
    type_filter: Option<KnowledgeType>,
    status_filter: Option<KnowledgeStatus>,
    confidence_filter: Option<Confidence>,
    unlinked: bool,
    tag_filter: Option<&str>,
) -> CommandOutput<KnowledgeListData> {
    let ws_name = storage.workspace_name().unwrap_or_default();

    let ids = match storage.list_knowledge_ids() {
        Ok(ids) => ids,
        Err(e) => {
            return CommandOutput {
                success: false,
                action: "knowledge_list".to_string(),
                workspace: ws_name,
                data: KnowledgeListData {
                    items: vec![],
                    total: 0,
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

    let mut items = Vec::new();
    let mut warnings = Vec::new();

    for id in &ids {
        let item = match storage.load_knowledge(id) {
            Ok(item) => item,
            Err(e) => {
                warnings.push(
                    OutputWarning::new(
                        "KNOWLEDGE_LOAD_ERROR",
                        format!("Failed to load {}: {}", id, e),
                    )
                    .with_context("id", serde_json::Value::String(id.clone())),
                );
                continue;
            }
        };

        if let Some(ref tf) = type_filter {
            if item.knowledge_type != *tf {
                continue;
            }
        }
        if let Some(ref sf) = status_filter {
            if item.status != *sf {
                continue;
            }
        }
        if let Some(ref cf) = confidence_filter {
            match item.confidence {
                Some(ref c) if c == cf => {}
                _ => continue,
            }
        }
        if unlinked && !item.links.is_empty() {
            continue;
        }
        if let Some(tag) = tag_filter {
            if !item.tags.contains(&tag.to_string()) {
                continue;
            }
        }

        items.push(KnowledgeSummary {
            id: item.id,
            knowledge_type: item.knowledge_type,
            label: item.label,
            status: item.status,
            confidence: item.confidence,
            link_count: item.links.len(),
            tags: item.tags,
        });
    }

    let total = items.len();

    let mut output = CommandOutput::ok(
        "knowledge_list",
        &ws_name,
        KnowledgeListData { items, total },
    );
    if !warnings.is_empty() {
        output.warnings = warnings;
    }
    output
}
