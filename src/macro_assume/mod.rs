//! Supuestos de long arrow (macro_edge assumptions) — Slice 1.
//!
//! Ritmo "gather → author": el motor recolecta el union de supuestos interiores y diffea
//! contra el resumen almacenado (funciones puras, storage-agnostic, base para `ltp-core`);
//! el LLM/usuario destila y mapea vía `add`/`rm`. Namespace propio para preparar el Slice 2
//! (creación top-down `macro_*`).

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::link::{Assumption, AssumptionStatus};
use crate::output::{CommandOutput, GraphHealth, OutputError};
use crate::storage::Storage;
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

/// Un supuesto interior proyectado en la vista de `gather` (solo lectura).
#[derive(Debug, Serialize)]
pub struct InteriorAsm {
    /// ID del supuesto interior (`ASM-xxx`).
    pub id: String,
    /// Estado del supuesto interior.
    pub status: AssumptionStatus,
    /// Texto del supuesto interior.
    pub text: String,
}

/// Reconciliación por membresía de conjuntos entre el interior vivo y el resumen almacenado.
///
/// Limitación (D9): detecta cambios de *membresía* (`unmapped`/`dangling`), no cambios de
/// *texto* de un supuesto interior ya mapeado (hash de contenido diferido, YAGNI).
#[derive(Debug, Serialize)]
pub struct GatherDiff {
    /// Supuestos interiores (`ASM-xxx`) que ningún `projection_ref` del resumen cubre.
    pub unmapped: Vec<String>,
    /// `projection_refs` del resumen que ya no existen en el interior vivo.
    pub dangling: Vec<String>,
    /// Número de `MacroAssumption` almacenados en la long arrow.
    pub summary_count: usize,
}

/// Vista viva del interior de una long arrow más su diff contra el resumen almacenado.
#[derive(Debug, Serialize)]
pub struct GatherData {
    /// ID de la long arrow inspeccionada.
    pub macro_link: String,
    /// Supuestos interiores agrupados por `link_id` (ordenado, determinista).
    pub interior: BTreeMap<String, Vec<InteriorAsm>>,
    /// Reconciliación de obsolescencia.
    pub diff: GatherDiff,
}

/// Calcula el diff de obsolescencia entre el interior vivo y el resumen de la long arrow.
///
/// Puro, storage-agnostic; compartido por `macro_assume_gather` (M2) y `validate` (M4).
/// Determinista: todos los conjuntos son `BTreeSet`, así que las salidas van ordenadas.
///
/// Set vivo (R3) = `(interior_links ∩ edges existentes)` ∪ `{asm.id de esos links}`.
/// Un `interior_link` eliminado del grafo (p. ej. por `link split`) deja de estar en el set
/// vivo, de modo que las refs que lo apuntaban pasan a `dangling`.
pub fn compute_diff(tree: &Tree, macro_edge: &MacroEdge) -> GatherDiff {
    let grouped = gather_interior_assumptions(tree, macro_edge);

    let mut live: BTreeSet<String> = BTreeSet::new();
    let mut interior_asm_ids: BTreeSet<String> = BTreeSet::new();
    for (link_id, asms) in &grouped {
        live.insert(link_id.clone());
        for asm in asms {
            live.insert(asm.id.clone());
            interior_asm_ids.insert(asm.id.clone());
        }
    }

    let summary_refs: BTreeSet<String> = macro_edge
        .assumptions
        .iter()
        .flat_map(|ma| ma.projection_refs.iter().cloned())
        .collect();

    let dangling: Vec<String> = summary_refs
        .iter()
        .filter(|r| !live.contains(*r))
        .cloned()
        .collect();
    let unmapped: Vec<String> = interior_asm_ids
        .iter()
        .filter(|a| !summary_refs.contains(*a))
        .cloned()
        .collect();

    GatherDiff {
        unmapped,
        dangling,
        summary_count: macro_edge.assumptions.len(),
    }
}

/// Construye una salida de fallo para `gather` con `GatherData` vacío.
fn gather_failure(
    action: &str,
    ws_name: &str,
    macro_link: &str,
    error: OutputError,
) -> CommandOutput<GatherData> {
    CommandOutput {
        success: false,
        action: action.to_string(),
        workspace: ws_name.to_string(),
        data: GatherData {
            macro_link: macro_link.to_string(),
            interior: BTreeMap::new(),
            diff: GatherDiff {
                unmapped: vec![],
                dangling: vec![],
                summary_count: 0,
            },
        },
        graph_health: GraphHealth {
            valid_dag: true,
            orphan_nodes_count: 0,
        },
        errors: vec![error],
        warnings: vec![],
    }
}

/// Ejecuta `macro-assume gather`: vista viva del interior + reconciliación (solo lectura).
///
/// No adquiere lock ni participa en el historial (invariante #3: vistas al vuelo).
pub fn execute_macro_assume_gather(
    storage: &dyn Storage,
    tree_id: &str,
    macro_link: &str,
) -> CommandOutput<GatherData> {
    let ws_name = storage.workspace_name().unwrap_or_default();
    let action = "macro_assume_gather";

    let tree = match storage.load_tree(tree_id) {
        Ok(t) => t,
        Err(_) => {
            return gather_failure(
                action,
                &ws_name,
                macro_link,
                OutputError::new("TREE_NOT_FOUND", format!("Tree '{tree_id}' not found")),
            );
        }
    };

    let macro_edge = match tree.macro_edges.iter().find(|m| m.id == macro_link) {
        Some(m) => m,
        None => {
            return gather_failure(
                action,
                &ws_name,
                macro_link,
                OutputError::new(
                    "MACRO_EDGE_NOT_FOUND",
                    format!("Macro-edge '{macro_link}' not found in tree '{tree_id}'"),
                ),
            );
        }
    };

    let interior: BTreeMap<String, Vec<InteriorAsm>> =
        gather_interior_assumptions(&tree, macro_edge)
            .into_iter()
            .map(|(link_id, asms)| {
                let entries = asms
                    .into_iter()
                    .map(|a| InteriorAsm {
                        id: a.id.clone(),
                        status: a.status,
                        text: a.text.clone(),
                    })
                    .collect();
                (link_id, entries)
            })
            .collect();

    let diff = compute_diff(&tree, macro_edge);

    CommandOutput::ok(
        action,
        &ws_name,
        GatherData {
            macro_link: macro_link.to_string(),
            interior,
            diff,
        },
    )
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

    fn masm(id: &str, refs: &[&str]) -> MacroAssumption {
        MacroAssumption {
            id: id.to_string(),
            status: AssumptionStatus::Valid,
            text: format!("resumen {id}"),
            projection_refs: refs.iter().map(|s| s.to_string()).collect(),
        }
    }

    // --- compute_diff: resumen vacío => todos los ASM interiores unmapped ---
    #[test]
    fn compute_diff_empty_summary_marks_all_unmapped() {
        let edges = vec![
            edge("LINK-001", "A", "B", vec![asm("ASM-001")]),
            edge("LINK-002", "B", "C", vec![asm("ASM-002")]),
        ];
        let me = macro_edge("MACRO-001", vec!["LINK-001", "LINK-002"]);
        let tree = tree_with(edges, vec![me.clone()]);
        let diff = compute_diff(&tree, &me);
        assert_eq!(diff.summary_count, 0);
        assert_eq!(diff.unmapped, vec!["ASM-001", "ASM-002"]);
        assert!(diff.dangling.is_empty());
    }

    // --- compute_diff: mapeo parcial => solo los no cubiertos quedan unmapped ---
    #[test]
    fn compute_diff_partial_mapping() {
        let edges = vec![
            edge("LINK-001", "A", "B", vec![asm("ASM-001")]),
            edge("LINK-002", "B", "C", vec![asm("ASM-002")]),
        ];
        let mut me = macro_edge("MACRO-001", vec!["LINK-001", "LINK-002"]);
        me.assumptions = vec![masm("MASM-001", &["ASM-001"])];
        let tree = tree_with(edges, vec![me.clone()]);
        let diff = compute_diff(&tree, &me);
        assert_eq!(diff.summary_count, 1);
        assert_eq!(diff.unmapped, vec!["ASM-002"]);
        assert!(diff.dangling.is_empty());
    }

    // --- compute_diff: ref a un ASM interior removido => dangling ---
    #[test]
    fn compute_diff_dangling_when_interior_asm_removed() {
        let edges = vec![edge("LINK-001", "A", "B", vec![])];
        let mut me = macro_edge("MACRO-001", vec!["LINK-001"]);
        me.assumptions = vec![masm("MASM-001", &["ASM-001"])];
        let tree = tree_with(edges, vec![me.clone()]);
        let diff = compute_diff(&tree, &me);
        assert_eq!(diff.dangling, vec!["ASM-001"]);
        assert!(diff.unmapped.is_empty());
    }

    // --- compute_diff R3: ref a un interior_link eliminado del grafo => dangling ---
    #[test]
    fn compute_diff_dangling_when_interior_link_removed_r3() {
        let edges = vec![edge("LINK-001", "A", "B", vec![])];
        let mut me = macro_edge("MACRO-001", vec!["LINK-001", "LINK-002"]);
        me.assumptions = vec![masm("MASM-001", &["LINK-002"])];
        let tree = tree_with(edges, vec![me.clone()]);
        let diff = compute_diff(&tree, &me);
        assert_eq!(diff.dangling, vec!["LINK-002"]);
    }
}
