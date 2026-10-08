//! `knowledge_health` block of `status`, shared by the CLI and the MCP server.

use serde::Serialize;

use crate::knowledge::pool::load_pool;
use crate::knowledge::{KnowledgeRelation, KnowledgeStatus};
use crate::node::types::EpistemicStatus;
use crate::output::OutputWarning;
use crate::storage::Storage;

/// Aggregate health of the knowledge pool, as reported by `status`.
#[derive(Debug, Serialize)]
pub struct KnowledgeHealth {
    /// Number of readable knowledge items.
    pub total: usize,
    /// Readable items without any link.
    pub unlinked_items: usize,
    /// Verified `contradicts` links whose target is a `fact` node.
    pub contradictions: usize,
    /// Readable items per status.
    pub by_status: KnowledgeByStatus,
    /// Nodes of the pool per epistemic status.
    pub epistemic_coverage: EpistemicCoverage,
}

/// Knowledge item count per [`KnowledgeStatus`].
#[derive(Debug, Serialize)]
pub struct KnowledgeByStatus {
    /// Items with status `unverified`.
    pub unverified: usize,
    /// Items with status `verified`.
    pub verified: usize,
    /// Items with status `refuted`.
    pub refuted: usize,
    /// Items with status `superseded`.
    pub superseded: usize,
}

/// Node count per [`EpistemicStatus`].
#[derive(Debug, Serialize)]
pub struct EpistemicCoverage {
    /// Nodes declared as `fact`.
    pub fact: usize,
    /// Nodes declared as `hypothesis`.
    pub hypothesis: usize,
    /// Nodes declared as `assumption`.
    pub assumption: usize,
    /// Nodes declared as `derived`.
    pub derived: usize,
}

/// Computes `knowledge_health` over the pool and the given nodes.
///
/// Returns the `KNOWLEDGE_LOAD_ERROR` warnings of the unreadable items
/// alongside; unreadable items are not counted. Unreadable nodes are skipped
/// in `epistemic_coverage` and as contradiction targets.
pub fn knowledge_health(
    storage: &dyn Storage,
    node_ids: &[String],
) -> (KnowledgeHealth, Vec<OutputWarning>) {
    let pool = load_pool(storage);
    let items = &pool.items;

    let contradictions = items
        .iter()
        .filter(|i| i.status == KnowledgeStatus::Verified)
        .flat_map(|i| &i.links)
        .filter(|l| l.relation == KnowledgeRelation::Contradicts)
        .filter(|l| {
            storage
                .load_node(&l.target)
                .is_ok_and(|n| n.epistemic == EpistemicStatus::Fact)
        })
        .count();

    let count_status = |s: KnowledgeStatus| items.iter().filter(|i| i.status == s).count();
    let by_status = KnowledgeByStatus {
        unverified: count_status(KnowledgeStatus::Unverified),
        verified: count_status(KnowledgeStatus::Verified),
        refuted: count_status(KnowledgeStatus::Refuted),
        superseded: count_status(KnowledgeStatus::Superseded),
    };

    let mut epistemic_coverage = EpistemicCoverage {
        fact: 0,
        hypothesis: 0,
        assumption: 0,
        derived: 0,
    };
    for node in node_ids.iter().filter_map(|id| storage.load_node(id).ok()) {
        match node.epistemic {
            EpistemicStatus::Fact => epistemic_coverage.fact += 1,
            EpistemicStatus::Hypothesis => epistemic_coverage.hypothesis += 1,
            EpistemicStatus::Assumption => epistemic_coverage.assumption += 1,
            EpistemicStatus::Derived => epistemic_coverage.derived += 1,
        }
    }

    let health = KnowledgeHealth {
        total: items.len(),
        unlinked_items: items.iter().filter(|i| i.links.is_empty()).count(),
        contradictions,
        by_status,
        epistemic_coverage,
    };
    (health, pool.warnings)
}
