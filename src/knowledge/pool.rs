//! Whole-pool reads that never drop an unreadable item in silence (ADR-016 D-K5).

use std::fmt::Display;

use crate::knowledge::KnowledgeItem;
use crate::output::OutputWarning;
use crate::storage::Storage;

/// Every readable knowledge item of the workspace plus one
/// `KNOWLEDGE_LOAD_ERROR` warning per item that could not be read.
#[derive(Debug, Default)]
pub struct KnowledgePool {
    /// Readable items, in `list_knowledge_ids` order (ascending ID).
    pub items: Vec<KnowledgeItem>,
    /// One `KNOWLEDGE_LOAD_ERROR {id}` per unreadable item, in ID order.
    pub warnings: Vec<OutputWarning>,
}

/// `KNOWLEDGE_LOAD_ERROR` warning for item `id` (context key `id`), shared by
/// every reader so the `detail` format cannot diverge.
pub fn load_error_warning(id: &str, error: &impl Display) -> OutputWarning {
    OutputWarning::new(
        "KNOWLEDGE_LOAD_ERROR",
        format!("Failed to load {}: {}", id, error),
    )
    .with_context("id", serde_json::Value::String(id.to_string()))
}

/// Loads the whole knowledge pool, warning about each unreadable item.
///
/// A pool directory that cannot be listed yields an empty pool, as every
/// reader did before v0.5.1.
pub fn load_pool(storage: &dyn Storage) -> KnowledgePool {
    let mut pool = KnowledgePool::default();
    for id in storage.list_knowledge_ids().unwrap_or_default() {
        match storage.load_knowledge(&id) {
            Ok(item) => pool.items.push(item),
            Err(e) => pool.warnings.push(load_error_warning(&id, &e)),
        }
    }
    pool
}
