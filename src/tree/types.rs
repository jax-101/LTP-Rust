use serde::{Deserialize, Serialize};

use crate::link::{AssumptionStatus, Edge, FeedbackEdge};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TreeType {
    Gt,
    Crt,
    Ec,
    Frt,
    Prt,
    Tt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TreeLogic {
    Sufficiency,
    Necessity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRef {
    #[serde(rename = "ref")]
    pub node_ref: String,
    pub role: Option<String>,
}

/// Supuesto-resumen destilado que cuelga de una long arrow (`MacroEdge`).
///
/// Mapea (via `projection_refs`) a elementos de la cadena causa-efecto interior
/// (`LINK-xxx` y/o `ASM-xxx`). Es una entidad de primer nivel direccionable por ID
/// (`MASM-xxx`), replicando el patrón de `Assumption` (ADR-005) pero como tipo propio
/// para no contaminar los edges normales con un campo siempre vacío.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroAssumption {
    /// Identificador secuencial (`MASM-xxx`).
    pub id: String,
    /// Estado del supuesto (reutiliza el enum de `Assumption`).
    pub status: AssumptionStatus,
    /// Texto destilado del supuesto-resumen.
    pub text: String,
    /// IDs interiores mapeados (`LINK-xxx` y/o `ASM-xxx`), ordenados y sin duplicados.
    #[serde(default)]
    pub projection_refs: Vec<String>,
}

/// Ciclo de vida de una long arrow (`MacroEdge`).
///
/// - `Overlay`: resume una cadena causa-efecto real que coexiste con ella (creada por
///   `path collapse` o resultante de `macro expand`). Retrocompat: los `macro_edges` previos
///   con `"status": "active"` deserializan aquí vía `#[serde(alias)]`.
/// - `Reservation`: salto lógico declarado top-down (CLR#1 "flecha larga"), con interior
///   vacío, pendiente de `macro expand` (materializar) o `macro promote` (edge atómico).
///
/// Máquina de estados sin estados muertos (ADR-013): la trazabilidad histórica la cubren
/// ADR-009 (snapshots undo) y ADR-002 (JSON git-diffable), por lo que no hay tombstones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MacroEdgeStatus {
    /// Reserva top-down: salto lógico con interior vacío, pendiente de resolución.
    Reservation,
    /// Resumen no-destructivo de una cadena causa-efecto real coexistente.
    #[serde(alias = "active")] // retrocompat: workspaces previos a Slice 2 escribían "active".
    Overlay,
}

/// Long arrow: resume una cadena causa-efecto colapsada (`path collapse`).
///
/// Coexiste de forma no-destructiva con la cadena interior. Sus `assumptions` son el
/// resumen autorado (opcional) de los supuestos interiores.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroEdge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub label: String,
    pub interior_nodes: Vec<String>,
    pub interior_links: Vec<String>,
    /// Estado del ciclo de vida de la long arrow (ver [`MacroEdgeStatus`]).
    pub status: MacroEdgeStatus,
    /// Supuestos-resumen autorados que cuelgan de esta long arrow.
    ///
    /// `#[serde(default)]` permite que los `macro_edges` previos (sin el campo)
    /// deserialicen con un `Vec` vacío; `skip_serializing_if` mantiene limpio el JSON
    /// de las macros sin resumen.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assumptions: Vec<MacroAssumption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NbrBranch {
    pub id: String,
    pub source_node: String,
    pub edges: Vec<Edge>,
    pub trim_injection: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tree {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub tree_type: TreeType,
    pub logic: TreeLogic,
    #[serde(default)]
    pub nodes: Vec<NodeRef>,
    #[serde(default)]
    pub edges: Vec<Edge>,
    #[serde(default)]
    pub macro_edges: Vec<MacroEdge>,
    #[serde(default)]
    pub feedback_edges: Vec<FeedbackEdge>,
    #[serde(default)]
    pub nbr_branches: Vec<NbrBranch>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn macro_edge(status: MacroEdgeStatus) -> MacroEdge {
        MacroEdge {
            id: "MACRO-001".to_string(),
            from: "RC-001".to_string(),
            to: "UDE-001".to_string(),
            label: "Long arrow".to_string(),
            interior_nodes: vec![],
            interior_links: vec![],
            status,
            assumptions: vec![],
        }
    }

    // (a) Retrocompat: JSON legacy `"status":"active"` (sin `assumptions`) => Overlay + vacío.
    #[test]
    fn legacy_active_status_deserializes_as_overlay() {
        let legacy = r#"{
            "id": "MACRO-001",
            "from": "RC-001",
            "to": "UDE-001",
            "label": "Cadena",
            "interior_nodes": [],
            "interior_links": ["LINK-001"],
            "status": "active"
        }"#;
        let me: MacroEdge = serde_json::from_str(legacy).expect("legacy debe deserializar");
        assert_eq!(me.status, MacroEdgeStatus::Overlay);
        assert!(me.assumptions.is_empty());
    }

    // (b) Reservation serializa como `"reservation"` y hace roundtrip.
    #[test]
    fn reservation_serializes_snake_case() {
        let json = serde_json::to_string(&macro_edge(MacroEdgeStatus::Reservation)).unwrap();
        assert!(
            json.contains("\"status\":\"reservation\""),
            "esperaba status reservation en {json}"
        );
        let back: MacroEdge = serde_json::from_str(&json).unwrap();
        assert_eq!(back.status, MacroEdgeStatus::Reservation);
    }

    // (c) Overlay serializa como `"overlay"` (nueva escritura canónica, no `"active"`).
    #[test]
    fn overlay_serializes_snake_case() {
        let json = serde_json::to_string(&macro_edge(MacroEdgeStatus::Overlay)).unwrap();
        assert!(
            json.contains("\"status\":\"overlay\""),
            "esperaba status overlay en {json}"
        );
        assert!(
            !json.contains("\"active\""),
            "la escritura canónica migra fuera de 'active': {json}"
        );
        let back: MacroEdge = serde_json::from_str(&json).unwrap();
        assert_eq!(back.status, MacroEdgeStatus::Overlay);
    }
}
