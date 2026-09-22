//! Supuestos de long arrow (macro_edge assumptions) — Slice 1.
//!
//! Ritmo "gather → author": el motor recolecta el union de supuestos interiores y diffea
//! contra el resumen almacenado (funciones puras, storage-agnostic, base para `ltp-core`);
//! el LLM/usuario destila y mapea vía `add`/`rm`. Namespace propio para preparar el Slice 2
//! (creación top-down `macro_*`).

use std::collections::BTreeMap;

use crate::link::Assumption;
use crate::tree::{MacroEdge, Tree};

/// Recolecta las assumptions de los `interior_links` de una long arrow, agrupadas por link.
///
/// No muta. Determinista (`BTreeMap` ordenado por `link_id`). Devuelve las assumptions por
/// referencia para evitar clonaciones. Los `interior_links` que ya no existen en
/// `tree.edges` (p. ej. removidos por operaciones destructivas) se omiten del resultado
/// —cuentan como `dangling` en el diff de M2— mientras que los links existentes sin
/// assumptions aparecen con un `Vec` vacío.
pub fn gather_interior_assumptions<'a>(
    tree: &'a Tree,
    macro_edge: &MacroEdge,
) -> BTreeMap<String, Vec<&'a Assumption>> {
    let mut grouped: BTreeMap<String, Vec<&'a Assumption>> = BTreeMap::new();
    for link_id in &macro_edge.interior_links {
        if let Some(edge) = tree.edges.iter().find(|e| &e.id == link_id) {
            grouped.insert(link_id.clone(), edge.assumptions.iter().collect());
        }
    }
    grouped
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::link::{Assumption, AssumptionStatus, Edge, Logic, Operator};
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
            status: crate::link::EdgeStatus::Active,
            logic: Logic::Sufficiency,
            assumptions,
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

    fn macro_edge(id: &str, interior_links: Vec<&str>) -> MacroEdge {
        MacroEdge {
            id: id.to_string(),
            from: "RC-001".to_string(),
            to: "UDE-001".to_string(),
            label: "Long arrow".to_string(),
            interior_nodes: vec![],
            interior_links: interior_links.into_iter().map(String::from).collect(),
            status: "active".to_string(),
            assumptions: vec![],
        }
    }

    // --- UAT C7: macro_edge legacy (sin campo `assumptions`) deserializa OK ---
    #[test]
    fn uat_c7_legacy_macro_edge_deserializes_without_assumptions() {
        // JSON de un MacroEdge previo al feature (sin la clave "assumptions").
        let legacy = r#"{
            "id": "MACRO-001",
            "from": "RC-001",
            "to": "UDE-001",
            "label": "Cadena",
            "interior_nodes": ["INT-001"],
            "interior_links": ["LINK-001", "LINK-002"],
            "status": "active"
        }"#;
        let me: MacroEdge = serde_json::from_str(legacy).expect("legacy debe deserializar");
        assert!(
            me.assumptions.is_empty(),
            "assumptions ausente => Vec vacío"
        );

        // Roundtrip con assumptions presentes.
        let with = MacroEdge {
            assumptions: vec![MacroAssumption {
                id: "MASM-001".to_string(),
                status: AssumptionStatus::Valid,
                text: "resumen".to_string(),
                projection_refs: vec!["ASM-001".to_string()],
            }],
            ..macro_edge("MACRO-002", vec!["LINK-001"])
        };
        let json = serde_json::to_string(&with).expect("serializa");
        let back: MacroEdge = serde_json::from_str(&json).expect("roundtrip");
        assert_eq!(back.assumptions.len(), 1);
        assert_eq!(back.assumptions[0].id, "MASM-001");
        assert_eq!(back.assumptions[0].projection_refs, vec!["ASM-001"]);

        // skip_serializing_if: macro sin assumptions no emite la clave.
        let empty = macro_edge("MACRO-003", vec!["LINK-001"]);
        let empty_json = serde_json::to_string(&empty).expect("serializa");
        assert!(
            !empty_json.contains("assumptions"),
            "assumptions vacío debe omitirse del JSON: {empty_json}"
        );
    }

    // --- UAT B3: gather con interior_links vacío -> BTreeMap vacío, sin panic ---
    #[test]
    fn uat_b3_gather_empty_interior_links() {
        let me = macro_edge("MACRO-001", vec![]);
        let tree = tree_with(vec![], vec![me.clone()]);
        let result = gather_interior_assumptions(&tree, &me);
        assert!(result.is_empty());
    }

    // --- UAT I4: gather sobre interior tipo diamond agrupa sin romper ---
    #[test]
    fn uat_i4_gather_diamond_groups_by_link() {
        // Diamond: A->B (L1), A->C (L2), B->D (L3), C->D (L4). Interior = L1..L4.
        let edges = vec![
            edge("LINK-001", "A", "B", vec![asm("ASM-001")]),
            edge("LINK-002", "A", "C", vec![asm("ASM-002"), asm("ASM-003")]),
            edge("LINK-003", "B", "D", vec![]),
            edge("LINK-004", "C", "D", vec![asm("ASM-004")]),
        ];
        let me = macro_edge(
            "MACRO-001",
            vec!["LINK-001", "LINK-002", "LINK-003", "LINK-004"],
        );
        let tree = tree_with(edges, vec![me.clone()]);
        let result = gather_interior_assumptions(&tree, &me);

        // 4 links presentes (incluido L3 con vec vacío), orden determinista por link_id.
        let keys: Vec<&String> = result.keys().collect();
        assert_eq!(keys, vec!["LINK-001", "LINK-002", "LINK-003", "LINK-004"]);
        assert_eq!(result["LINK-001"].len(), 1);
        assert_eq!(result["LINK-002"].len(), 2);
        assert!(result["LINK-003"].is_empty());
        assert_eq!(result["LINK-004"][0].id, "ASM-004");
    }

    // --- Interior link ausente en tree.edges se omite del gather ---
    #[test]
    fn gather_omits_missing_interior_link() {
        let edges = vec![edge("LINK-001", "A", "B", vec![asm("ASM-001")])];
        // interior_links referencia LINK-002 que ya no existe.
        let me = macro_edge("MACRO-001", vec!["LINK-001", "LINK-002"]);
        let tree = tree_with(edges, vec![me.clone()]);
        let result = gather_interior_assumptions(&tree, &me);
        assert_eq!(result.len(), 1);
        assert!(result.contains_key("LINK-001"));
        assert!(!result.contains_key("LINK-002"));
    }
}
