//! Validación no bloqueante de long arrows (`macro_edge`) — Slice 1, Fase M4.
//!
//! Hace visibles las long arrows a `validate` mediante warnings (nunca errores; no afectan
//! `valid_dag`, ADR-010). Reutiliza el diff puro de M2 ([`compute_diff`]) para la detección de
//! obsolescencia, de modo que `gather` y `validate` comparten exactamente la misma lógica.

use serde_json::Value;

use crate::macro_assume::{compute_diff, gather_interior_assumptions};
use crate::output::OutputWarning;
use crate::tree::Tree;

/// Estado textual que marca una long arrow como activa (elegible para auditoría).
const ACTIVE_STATUS: &str = "active";

/// Audita las long arrows activas de un árbol y emite warnings de higiene del resumen.
///
/// - `LONG_ARROW_UNSUMMARIZED`: macro activa sin resumen alguno pese a tener supuestos
///   interiores (`interior_asm_count > 0`).
/// - `LONG_ARROW_SUMMARY_STALE`: el resumen existe pero su diff (`unmapped` ∪ `dangling`) no
///   está vacío (reusa [`compute_diff`]). Mutuamente excluyente con `UNSUMMARIZED`.
/// - `MACRO_ASSUMPTION_UNGROUNDED`: un `MacroAssumption` sin `projection_refs` teniendo interior
///   (mismo criterio que `macro-assume add`).
///
/// Nunca produce errores; solo `macro_edges` con `status == "active"`.
pub fn check_macro_edges(tree: &Tree) -> Vec<OutputWarning> {
    let mut warnings = Vec::new();

    for me in &tree.macro_edges {
        if me.status != ACTIVE_STATUS {
            continue;
        }

        let grouped = gather_interior_assumptions(tree, me);
        let interior_non_empty = !grouped.is_empty();
        let interior_asm_count: usize = grouped.values().map(|v| v.len()).sum();

        if me.assumptions.is_empty() {
            // Sin resumen: solo se avisa si hay supuestos interiores que resumir.
            if interior_asm_count > 0 {
                warnings.push(
                    OutputWarning::new(
                        "LONG_ARROW_UNSUMMARIZED",
                        format!(
                            "Long arrow '{}' has {interior_asm_count} interior assumption(s) but no summary",
                            me.id
                        ),
                    )
                    .with_context("macro_link", me.id.as_str())
                    .with_context("interior_asm_count", interior_asm_count as u64),
                );
            }
            continue;
        }

        // Con resumen: detectar obsolescencia (idéntico diff que M2 / gather).
        let diff = compute_diff(tree, me);
        if !diff.unmapped.is_empty() || !diff.dangling.is_empty() {
            warnings.push(
                OutputWarning::new(
                    "LONG_ARROW_SUMMARY_STALE",
                    format!(
                        "Long arrow '{}' summary is stale ({} unmapped, {} dangling)",
                        me.id,
                        diff.unmapped.len(),
                        diff.dangling.len()
                    ),
                )
                .with_context("macro_link", me.id.as_str())
                .with_context("unmapped", string_array_value(&diff.unmapped))
                .with_context("dangling", string_array_value(&diff.dangling)),
            );
        }

        // Supuestos-resumen sin anclar (mismo criterio que `macro-assume add`).
        if interior_non_empty {
            for ma in &me.assumptions {
                if ma.projection_refs.is_empty() {
                    warnings.push(
                        OutputWarning::new(
                            "MACRO_ASSUMPTION_UNGROUNDED",
                            format!(
                                "Macro-assumption '{}' has no projection_refs while the interior is non-empty",
                                ma.id
                            ),
                        )
                        .with_context("macro_link", me.id.as_str())
                        .with_context("assumption_id", ma.id.as_str()),
                    );
                }
            }
        }
    }

    warnings
}

/// Convierte un slice de IDs en un `serde_json::Value::Array` para el contexto de warnings.
fn string_array_value(items: &[String]) -> Value {
    Value::Array(items.iter().map(|s| Value::String(s.clone())).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::link::{Assumption, AssumptionStatus, Edge, EdgeStatus, Logic, Operator};
    use crate::tree::{MacroAssumption, MacroEdge, Tree, TreeLogic, TreeType};

    fn asm(id: &str) -> Assumption {
        Assumption {
            id: id.to_string(),
            status: AssumptionStatus::Valid,
            text: format!("assumption {id}"),
        }
    }

    fn edge(id: &str, from: &str, to: &str, assumptions: Vec<Assumption>) -> Edge {
        Edge {
            id: id.to_string(),
            from: vec![from.to_string()],
            to: to.to_string(),
            operator: Operator::Single,
            weight: None,
            status: EdgeStatus::Active,
            logic: Logic::Sufficiency,
            assumptions,
        }
    }

    fn macro_edge(
        id: &str,
        interior_links: Vec<&str>,
        assumptions: Vec<MacroAssumption>,
    ) -> MacroEdge {
        MacroEdge {
            id: id.to_string(),
            from: "RC-001".to_string(),
            to: "UDE-001".to_string(),
            label: "Long arrow".to_string(),
            interior_nodes: vec![],
            interior_links: interior_links.into_iter().map(String::from).collect(),
            status: "active".to_string(),
            assumptions,
        }
    }

    fn masm(id: &str, refs: &[&str]) -> MacroAssumption {
        MacroAssumption {
            id: id.to_string(),
            status: AssumptionStatus::Valid,
            text: format!("resumen {id}"),
            projection_refs: refs.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn tree_with(edges: Vec<Edge>, macro_edges: Vec<MacroEdge>) -> Tree {
        Tree {
            id: "TREE-001".to_string(),
            name: "Test".to_string(),
            tree_type: TreeType::Crt,
            logic: TreeLogic::Sufficiency,
            nodes: vec![],
            edges,
            macro_edges,
            feedback_edges: vec![],
            nbr_branches: vec![],
        }
    }

    fn codes(warnings: &[OutputWarning]) -> Vec<&str> {
        warnings.iter().map(|w| w.code.as_str()).collect()
    }

    #[test]
    fn unsummarized_when_interior_asms_but_no_summary() {
        let edges = vec![edge("LINK-001", "A", "B", vec![asm("ASM-001")])];
        let me = macro_edge("MACRO-001", vec!["LINK-001"], vec![]);
        let tree = tree_with(edges, vec![me]);
        let warnings = check_macro_edges(&tree);
        assert_eq!(codes(&warnings), vec!["LONG_ARROW_UNSUMMARIZED"]);
        assert_eq!(warnings[0].context["interior_asm_count"], 1);
    }

    #[test]
    fn no_unsummarized_when_interior_has_no_asms() {
        let edges = vec![edge("LINK-001", "A", "B", vec![])];
        let me = macro_edge("MACRO-001", vec!["LINK-001"], vec![]);
        let tree = tree_with(edges, vec![me]);
        assert!(check_macro_edges(&tree).is_empty());
    }

    #[test]
    fn stale_when_summary_has_unmapped() {
        // Interior ASM-001 exists but summary maps nothing to it.
        let edges = vec![edge("LINK-001", "A", "B", vec![asm("ASM-001")])];
        let me = macro_edge(
            "MACRO-001",
            vec!["LINK-001"],
            vec![masm("MASM-001", &["LINK-001"])],
        );
        let tree = tree_with(edges, vec![me]);
        let warnings = check_macro_edges(&tree);
        // ASM-001 unmapped => stale. MASM-001 is grounded (points at LINK-001) => no ungrounded.
        assert_eq!(codes(&warnings), vec!["LONG_ARROW_SUMMARY_STALE"]);
        assert_eq!(
            warnings[0].context["unmapped"],
            serde_json::json!(["ASM-001"])
        );
    }

    #[test]
    fn ungrounded_when_masm_has_no_refs() {
        let edges = vec![edge("LINK-001", "A", "B", vec![])];
        // Interior has a live link (non-empty) but the summary MASM has no refs.
        let me = macro_edge("MACRO-001", vec!["LINK-001"], vec![masm("MASM-001", &[])]);
        let tree = tree_with(edges, vec![me]);
        let warnings = check_macro_edges(&tree);
        assert!(codes(&warnings).contains(&"MACRO_ASSUMPTION_UNGROUNDED"));
        let ungrounded = warnings
            .iter()
            .find(|w| w.code == "MACRO_ASSUMPTION_UNGROUNDED")
            .unwrap();
        assert_eq!(ungrounded.context["assumption_id"], "MASM-001");
    }

    #[test]
    fn inactive_macro_edge_is_skipped() {
        let edges = vec![edge("LINK-001", "A", "B", vec![asm("ASM-001")])];
        let mut me = macro_edge("MACRO-001", vec!["LINK-001"], vec![]);
        me.status = "exploded".to_string();
        let tree = tree_with(edges, vec![me]);
        assert!(check_macro_edges(&tree).is_empty());
    }
}
