//! Whole-pool reads that never drop an unreadable item in silence (ADR-016 D-K5).

use std::fmt::Display;

use crate::knowledge::KnowledgeItem;
use crate::output::OutputWarning;
use crate::storage::Storage;

/// How much of the knowledge pool a [`KnowledgePool`] covers (PLAN_v060 D-1).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum PoolScope {
    /// The pool directory was listed: every item is either read or warned about.
    #[default]
    Complete,
    /// The pool directory could not be listed: there are no items, and their
    /// absence says nothing about the workspace.
    Unlisted,
}

/// Every readable knowledge item of the workspace plus one
/// `KNOWLEDGE_LOAD_ERROR` warning per item that could not be read.
#[derive(Debug, Default)]
pub struct KnowledgePool {
    /// Readable items, in `list_knowledge_ids` order (ascending ID).
    pub items: Vec<KnowledgeItem>,
    /// `KNOWLEDGE_POOL_UNREADABLE` when the pool could not be listed, else one
    /// `KNOWLEDGE_LOAD_ERROR {id}` per unreadable item, in ID order.
    pub warnings: Vec<OutputWarning>,
    /// Whether `items` is the whole readable pool or nothing at all.
    pub scope: PoolScope,
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
/// A pool directory that cannot be listed yields an empty pool with scope
/// [`PoolScope::Unlisted`] and a single `KNOWLEDGE_POOL_UNREADABLE` warning,
/// so no reader mistakes "unreadable" for "empty".
pub fn load_pool(storage: &dyn Storage) -> KnowledgePool {
    let mut pool = KnowledgePool::default();
    let ids = match storage.list_knowledge_ids() {
        Ok(ids) => ids,
        Err(e) => {
            pool.scope = PoolScope::Unlisted;
            pool.warnings.push(OutputWarning::new(
                "KNOWLEDGE_POOL_UNREADABLE",
                format!("Failed to list the knowledge pool: {}", e),
            ));
            return pool;
        }
    };
    for id in ids {
        match storage.load_knowledge(&id) {
            Ok(item) => pool.items.push(item),
            Err(e) => pool.warnings.push(load_error_warning(&id, &e)),
        }
    }
    pool
}
