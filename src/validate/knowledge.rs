use std::collections::HashSet;

use crate::knowledge::{KnowledgeItem, KnowledgeRelation, KnowledgeStatus};
use crate::node::types::EpistemicStatus;
use crate::output::OutputWarning;
use crate::storage::Storage;

/// Validates knowledge pool integrity and epistemic grounding.
///
/// Returns warnings for:
/// - DANGLING_KNOWLEDGE_REF: KN link target doesn't exist
/// - EPISTEMIC_UNGROUNDED: fact node with 0 active supports
/// - EPISTEMIC_CONTRADICTED: fact node with verified contradiction
/// - EPISTEMIC_UPGRADEABLE: hypothesis/assumption with >=2 verified supports and 0 verified contradicts
pub fn validate_knowledge(
    storage: &dyn Storage,
    node_filter: Option<&HashSet<String>>,
) -> Vec<OutputWarning> {
    let mut warnings = Vec::new();

    let kn_ids = match storage.list_knowledge_ids() {
        Ok(ids) => ids,
        Err(_) => return warnings,
    };

    let items: Vec<KnowledgeItem> = kn_ids
        .iter()
        .filter_map(|id| storage.load_knowledge(id).ok())
        .collect();

    // DANGLING_KNOWLEDGE_REF: check each link target resolves
    for item in &items {
        for link in &item.links {
            if !crate::knowledge::resolve::target_exists(storage, &link.target) {
                warnings.push(
                    OutputWarning::new(
                        "DANGLING_KNOWLEDGE_REF",
                        format!(
                            "Knowledge item '{}' links to '{}' which does not exist",
                            item.id, link.target
                        ),
                    )
                    .with_context("knowledge_id", serde_json::Value::String(item.id.clone()))
                    .with_context("target", serde_json::Value::String(link.target.clone())),
                );
            }
        }
    }

    // Epistemic warnings: per-node analysis
    let node_ids = match storage.list_node_ids() {
        Ok(ids) => ids,
        Err(_) => return warnings,
    };

    for node_id in &node_ids {
        if let Some(filter) = node_filter {
            if !filter.contains(node_id) {
                continue;
            }
        }

        let node = match storage.load_node(node_id) {
            Ok(n) => n,
            Err(_) => continue,
        };

        let epistemic = node.epistemic;

        // Count supports and contradictions targeting this node
        let mut active_supports = 0u32;
        let mut verified_contradicts = 0u32;

        for item in &items {
            for link in &item.links {
                if link.target != *node_id {
                    continue;
                }

                match link.relation {
                    KnowledgeRelation::Supports => {
                        // Active support: status ∈ {unverified, verified}
                        match item.status {
                            KnowledgeStatus::Unverified | KnowledgeStatus::Verified => {
                                active_supports += 1;
                            }
                            KnowledgeStatus::Refuted | KnowledgeStatus::Superseded => {}
                        }
                    }
                    KnowledgeRelation::Contradicts => {
                        if item.status == KnowledgeStatus::Verified {
                            verified_contradicts += 1;
                        }
                    }
                    KnowledgeRelation::Contextualizes => {}
                }
            }
        }

        match epistemic {
            EpistemicStatus::Fact => {
                // EPISTEMIC_UNGROUNDED: fact with 0 active supports
                if active_supports == 0 {
                    warnings.push(
                        OutputWarning::new(
                            "EPISTEMIC_UNGROUNDED",
                            format!(
                                "Node '{}' is declared as fact but has no active supporting evidence",
                                node_id
                            ),
                        )
                        .with_context("node_id", serde_json::Value::String(node_id.clone())),
                    );
                }

                // EPISTEMIC_CONTRADICTED: fact with verified contradicts
                if verified_contradicts > 0 {
                    let contradicting_kns: Vec<String> = items
                        .iter()
                        .filter(|item| {
                            item.status == KnowledgeStatus::Verified
                                && item.links.iter().any(|l| {
                                    l.target == *node_id
                                        && l.relation == KnowledgeRelation::Contradicts
                                })
                        })
                        .map(|item| item.id.clone())
                        .collect();

                    for kn_id in &contradicting_kns {
                        warnings.push(
                            OutputWarning::new(
                                "EPISTEMIC_CONTRADICTED",
                                format!(
                                    "Node '{}' (fact) is contradicted by verified evidence '{}'",
                                    node_id, kn_id
                                ),
                            )
                            .with_context("node_id", serde_json::Value::String(node_id.clone()))
                            .with_context("knowledge_id", serde_json::Value::String(kn_id.clone())),
                        );
                    }
                }
            }
            EpistemicStatus::Hypothesis | EpistemicStatus::Assumption => {
                // EPISTEMIC_UPGRADEABLE: >=2 verified supports AND 0 verified contradicts
                let verified_supports = items
                    .iter()
                    .filter(|item| {
                        item.status == KnowledgeStatus::Verified
                            && item.links.iter().any(|l| {
                                l.target == *node_id && l.relation == KnowledgeRelation::Supports
                            })
                    })
                    .count();

                if verified_supports >= 2 && verified_contradicts == 0 {
                    warnings.push(
                        OutputWarning::new(
                            "EPISTEMIC_UPGRADEABLE",
                            format!(
                                "Node '{}' ({:?}) has {} verified supports and could be promoted to fact",
                                node_id, epistemic, verified_supports
                            ),
                        )
                        .with_context("node_id", serde_json::Value::String(node_id.clone()))
                        .with_context(
                            "verified_supports",
                            serde_json::Value::Number(serde_json::Number::from(verified_supports)),
                        ),
                    );
                }
            }
            EpistemicStatus::Derived => {}
        }
    }

    warnings
}
