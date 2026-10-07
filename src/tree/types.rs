use serde::{Deserialize, Serialize};

use crate::link::{AssumptionStatus, Edge, FeedbackEdge, Logic};

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

impl TreeType {
    /// Lógica canónica del tipo de árbol (CLR_SPEC §1.2).
    ///
    /// Necesidad: GT, EC, PRT ("para lograr X, necesitamos Y"). Suficiencia: CRT, FRT, TT
    /// ("si X, entonces Y"). Única fuente de verdad: `Tree::logic` y la lógica de los edges
    /// del tronco se derivan de aquí (ADR-014).
    pub fn logic(self) -> TreeLogic {
        match self {
            Self::Gt | Self::Ec | Self::Prt => TreeLogic::Necessity,
            Self::Crt | Self::Frt | Self::Tt => TreeLogic::Sufficiency,
        }
    }
}

impl From<TreeLogic> for Logic {
    fn from(logic: TreeLogic) -> Self {
        match logic {
            TreeLogic::Sufficiency => Logic::Sufficiency,
            TreeLogic::Necessity => Logic::Necessity,
        }
    }
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MacroEdgeStatus {
    /// Reserva top-down: salto lógico con interior vacío, pendiente de resolución.
    Reservation,
    /// Resumen no-destructivo de una cadena causa-efecto real coexistente.
    #[serde(alias = "active")] // retrocompat: workspaces previos a Slice 2 escribían "active".
    Overlay,
}

impl MacroEdgeStatus {
    /// Nombre en el wire (`snake_case`). Única fuente: `Serialize` delega aquí.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Reservation => "reservation",
            Self::Overlay => "overlay",
        }
    }
}

impl Serialize for MacroEdgeStatus {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
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

impl Tree {
    /// Re-deriva la lógica del árbol y de sus edges desde `tree_type` (ADR-014).
    ///
    /// Invariante: `logic == tree_type.logic()`, cada edge del tronco lleva esa lógica y los
    /// edges de `nbr_branches` son siempre `Sufficiency` (una NBR es una rama "si-entonces").
    /// Se aplica al leer (`Storage::load_tree`), en memoria: un fichero legacy (p. ej. un GT
    /// guardado como suficiencia) se corrige al vuelo y se persiste corregido en la siguiente
    /// mutación, sin reescrituras fuera del historial que romperían el undo (ADR-009).
    /// Idempotente.
    pub fn normalize_logic(&mut self) {
        self.logic = self.tree_type.logic();
        let trunk = Logic::from(self.logic);
        for edge in &mut self.edges {
            edge.logic = trunk;
        }
        for edge in self
            .nbr_branches
            .iter_mut()
            .flat_map(|b| b.edges.iter_mut())
        {
            edge.logic = Logic::Sufficiency;
        }
    }
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

    // (d) `as_str` es el nombre en el wire: serializa igual y deserializa de vuelta.
    #[test]
    fn as_str_pins_wire_names() {
        for (status, wire) in [
            (MacroEdgeStatus::Reservation, "reservation"),
            (MacroEdgeStatus::Overlay, "overlay"),
        ] {
            assert_eq!(status.as_str(), wire);
            assert_eq!(serde_json::to_value(status).unwrap(), wire);
            let back: MacroEdgeStatus = serde_json::from_value(wire.into()).unwrap();
            assert_eq!(back, status);
        }
    }

    use crate::link::{EdgeStatus, Operator};

    fn edge(id: &str, logic: Logic) -> Edge {
        Edge {
            id: id.to_string(),
            from: vec!["NC-001".to_string()],
            to: "GOAL-001".to_string(),
            operator: Operator::Single,
            weight: None,
            status: EdgeStatus::Active,
            logic,
            assumptions: vec![],
        }
    }

    fn tree_with(
        tree_type: TreeType,
        logic: TreeLogic,
        edges: Vec<Edge>,
        nbr_edges: Vec<Edge>,
    ) -> Tree {
        Tree {
            id: "tree-x".to_string(),
            name: "x".to_string(),
            tree_type,
            logic,
            nodes: vec![],
            edges,
            macro_edges: vec![],
            feedback_edges: vec![],
            nbr_branches: vec![NbrBranch {
                id: "NBR-001".to_string(),
                source_node: "INJ-001".to_string(),
                edges: nbr_edges,
                trim_injection: None,
            }],
        }
    }

    // ADR-014: la lógica canónica de cada tipo sigue CLR_SPEC §1.2.
    #[test]
    fn tree_type_logic_follows_clr_spec() {
        for t in [TreeType::Gt, TreeType::Ec, TreeType::Prt] {
            assert_eq!(t.logic(), TreeLogic::Necessity, "{t:?}");
        }
        for t in [TreeType::Crt, TreeType::Frt, TreeType::Tt] {
            assert_eq!(t.logic(), TreeLogic::Sufficiency, "{t:?}");
        }
    }

    #[test]
    fn tree_logic_maps_to_edge_logic() {
        assert_eq!(Logic::from(TreeLogic::Sufficiency), Logic::Sufficiency);
        assert_eq!(Logic::from(TreeLogic::Necessity), Logic::Necessity);
    }

    // Legacy: GT guardado como suficiencia (pre-ADR-014) ⇒ tronco a necesidad; la NBR no cambia.
    #[test]
    fn normalize_logic_fixes_legacy_gt() {
        let mut t = tree_with(
            TreeType::Gt,
            TreeLogic::Sufficiency,
            vec![
                edge("LINK-001", Logic::Sufficiency),
                edge("LINK-002", Logic::Sufficiency),
            ],
            vec![edge("LINK-003", Logic::Sufficiency)],
        );
        t.normalize_logic();
        assert_eq!(t.logic, TreeLogic::Necessity);
        assert!(t.edges.iter().all(|e| e.logic == Logic::Necessity));
        assert_eq!(t.nbr_branches[0].edges[0].logic, Logic::Sufficiency);
    }

    // Bidireccional: un edge NECESSITY colado a mano en un CRT vuelve a SUFFICIENCY, y una rama
    // NBR mal etiquetada vuelve a SUFFICIENCY aunque el árbol sea de necesidad.
    #[test]
    fn normalize_logic_is_bidirectional_and_forces_nbr_sufficiency() {
        let mut crt = tree_with(
            TreeType::Crt,
            TreeLogic::Necessity,
            vec![edge("LINK-001", Logic::Necessity)],
            vec![],
        );
        crt.normalize_logic();
        assert_eq!(crt.logic, TreeLogic::Sufficiency);
        assert_eq!(crt.edges[0].logic, Logic::Sufficiency);

        let mut prt = tree_with(
            TreeType::Prt,
            TreeLogic::Necessity,
            vec![],
            vec![edge("LINK-002", Logic::Necessity)],
        );
        prt.normalize_logic();
        assert_eq!(prt.nbr_branches[0].edges[0].logic, Logic::Sufficiency);
    }

    #[test]
    fn normalize_logic_is_idempotent() {
        let mut t = tree_with(
            TreeType::Ec,
            TreeLogic::Sufficiency,
            vec![edge("LINK-001", Logic::Sufficiency)],
            vec![edge("LINK-002", Logic::Necessity)],
        );
        t.normalize_logic();
        let first = serde_json::to_string(&t).unwrap();
        t.normalize_logic();
        assert_eq!(serde_json::to_string(&t).unwrap(), first);
    }
}
