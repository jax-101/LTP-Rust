//! Ciclo de vida top-down de long arrows (`macro add` / `macro expand` / `macro promote`) — Slice 2.
//!
//! Dirección inversa a `path collapse`: primero se declara el salto lógico (CLR#1 "flecha larga")
//! como una **reserva** (`macro add`, interior vacío, fuera del DAG) y luego se resuelve
//! materializándolo en una cadena INT explícita (`macro expand`) o promoviéndolo a un edge
//! atómico directo (`macro promote`). Coherente con ADR-004 (mutaciones por intención),
//! ADR-010 (semántica no bloqueante: las reservas nunca afectan `valid_dag`) y ADR-013 (máquina
//! de estados sin tombstones; `expand`/`promote` materializan edges reales ⇒ sí bloquean ciclos).
//!
//! Las funciones `execute_*` mutan bajo lock pero **no** capturan historial: el snapshot
//! undo/redo (ADR-009) lo envuelve el llamador (CLI en `main.rs`, MCP en `dispatch.rs`).

use serde::Serialize;

use crate::output::{CommandOutput, GraphHealth, OutputError, OutputWarning};
use crate::storage::{LockOutcome, Storage};
use crate::tree::{MacroEdge, MacroEdgeStatus};

/// Advertencia por lock obsoleto retirado (paridad con el resto de comandos mutadores).
fn stale_lock_warning(outcome: &LockOutcome) -> Option<OutputWarning> {
    match outcome {
        LockOutcome::StaleLockRemoved { pid } => Some(OutputWarning::new(
            "STALE_LOCK_REMOVED",
            format!("Stale lock from PID {pid} was removed"),
        )),
        LockOutcome::Acquired => None,
    }
}

// --- M2: macro add (reserva top-down) ---

/// Data devuelta por `macro add`.
#[derive(Debug, Serialize)]
pub struct MacroAddData {
    /// ID de la reserva recién creada (`MACRO-xxx`).
    pub macro_edge_id: String,
    /// Nodo origen del salto lógico declarado.
    pub from: String,
    /// Nodo destino del salto lógico declarado.
    pub to: String,
    /// Etiqueta descriptiva del salto.
    pub label: String,
}

/// Construye una salida de fallo para `macro add` con `MacroAddData` de eco (sin ID minted).
fn add_failure(
    ws_name: &str,
    from: &str,
    to: &str,
    label: &str,
    error: OutputError,
) -> CommandOutput<MacroAddData> {
    CommandOutput {
        success: false,
        action: "macro_add".to_string(),
        workspace: ws_name.to_string(),
        data: MacroAddData {
            macro_edge_id: String::new(),
            from: from.to_string(),
            to: to.to_string(),
            label: label.to_string(),
        },
        graph_health: GraphHealth {
            valid_dag: true,
            orphan_nodes_count: 0,
        },
        errors: vec![error],
        warnings: vec![],
    }
}

/// Ejecuta `macro add`: crea una reserva (long arrow con interior vacío) entre dos nodos.
///
/// Declaración de intención top-down (CLR#1 "flecha larga"): registra un salto lógico
/// `from → to` sin materializar los pasos intermedios (estado [`MacroEdgeStatus::Reservation`]).
/// **No** valida que exista o no un camino real `from → to` (ADR-004: la reserva es independiente
/// de la topología táctica; la resolverán `macro expand` o `macro promote`), y **no** afecta
/// `valid_dag` (ADR-010: fuera del DAG). Muta bajo lock; el historial lo captura el llamador.
///
/// Validaciones (en orden; todas preceden al minteo del ID ⇒ el contador `MACRO` no se consume
/// en un fallo de validación):
/// - `TREE_NOT_FOUND`: el árbol no existe.
/// - `LABEL_REQUIRED`: `label` vacío o solo espacios.
/// - `RESERVATION_SELF_LOOP`: `from == to`.
/// - `NODE_NOT_IN_TREE`: algún extremo no está attached al árbol.
pub fn execute_macro_add(
    storage: &dyn Storage,
    tree_id: &str,
    from: &str,
    to: &str,
    label: &str,
) -> CommandOutput<MacroAddData> {
    let ws_name = storage.workspace_name().unwrap_or_default();

    let lock_outcome = match storage.acquire_lock("macro add") {
        Ok(o) => o,
        Err(e) => {
            return add_failure(
                &ws_name,
                from,
                to,
                label,
                OutputError::new("LOCK_ERROR", e.to_string()),
            );
        }
    };

    let mut tree = match storage.load_tree(tree_id) {
        Ok(t) => t,
        Err(_) => {
            let _ = storage.release_lock();
            return add_failure(
                &ws_name,
                from,
                to,
                label,
                OutputError::new("TREE_NOT_FOUND", format!("Tree '{tree_id}' not found")),
            );
        }
    };

    if label.trim().is_empty() {
        let _ = storage.release_lock();
        return add_failure(
            &ws_name,
            from,
            to,
            label,
            OutputError::new("LABEL_REQUIRED", "Macro-edge label must not be empty"),
        );
    }

    if from == to {
        let _ = storage.release_lock();
        return add_failure(
            &ws_name,
            from,
            to,
            label,
            OutputError::new(
                "RESERVATION_SELF_LOOP",
                format!("Reservation cannot loop node '{from}' onto itself"),
            )
            .with_context("node_id", from),
        );
    }

    let from_attached = tree.nodes.iter().any(|nr| nr.node_ref == from);
    let to_attached = tree.nodes.iter().any(|nr| nr.node_ref == to);
    if !from_attached {
        let _ = storage.release_lock();
        return add_failure(
            &ws_name,
            from,
            to,
            label,
            OutputError::new(
                "NODE_NOT_IN_TREE",
                format!("Node '{from}' is not attached to tree '{tree_id}'"),
            )
            .with_context("node_id", from),
        );
    }
    if !to_attached {
        let _ = storage.release_lock();
        return add_failure(
            &ws_name,
            from,
            to,
            label,
            OutputError::new(
                "NODE_NOT_IN_TREE",
                format!("Node '{to}' is not attached to tree '{tree_id}'"),
            )
            .with_context("node_id", to),
        );
    }

    let macro_id = match storage.next_id("MACRO") {
        Ok(id) => id,
        Err(e) => {
            let _ = storage.release_lock();
            return add_failure(
                &ws_name,
                from,
                to,
                label,
                OutputError::new("ID_GENERATION_ERROR", e.to_string()),
            );
        }
    };

    tree.macro_edges.push(MacroEdge {
        id: macro_id.clone(),
        from: from.to_string(),
        to: to.to_string(),
        label: label.to_string(),
        interior_nodes: vec![],
        interior_links: vec![],
        status: MacroEdgeStatus::Reservation,
        assumptions: vec![],
    });

    if let Err(e) = storage.save_tree(&tree) {
        let _ = storage.release_lock();
        return add_failure(
            &ws_name,
            from,
            to,
            label,
            OutputError::new("IO_ERROR", e.to_string()),
        );
    }

    let _ = storage.release_lock();

    let mut warnings = vec![];
    if let Some(w) = stale_lock_warning(&lock_outcome) {
        warnings.push(w);
    }

    CommandOutput {
        success: true,
        action: "macro_add".to_string(),
        workspace: ws_name,
        data: MacroAddData {
            macro_edge_id: macro_id,
            from: from.to_string(),
            to: to.to_string(),
            label: label.to_string(),
        },
        graph_health: GraphHealth {
            valid_dag: true,
            orphan_nodes_count: 0,
        },
        errors: vec![],
        warnings,
    }
}
