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
    pub status: String,
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
