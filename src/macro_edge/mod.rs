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

use crate::errors::LtpError;
use crate::link::{Edge, EdgeStatus, Logic, Operator};
use crate::node::types::{EpistemicStatus, Node, NodeMetadata, NodeStatus, NodeType};
use crate::output::{CommandOutput, GraphHealth, OutputError, OutputWarning};
use crate::storage::{LockOutcome, Storage};
use crate::tree::{MacroEdge, MacroEdgeStatus, NodeRef, TreeLogic};
use crate::validate::check_dag;

/// Deriva la lógica de un edge (`Logic`) desde la lógica del árbol (`TreeLogic`).
///
/// Los INT/LINK materializados por `expand` (y el edge atómico de `promote`) heredan la lógica
/// del árbol contenedor: árboles de suficiencia (GT/CRT/FRT/TT) ⇒ `Sufficiency`; de necesidad
/// (EC/PRT) ⇒ `Necessity` (ver `tree::commands::logic_for_type`).
fn edge_logic(tree_logic: TreeLogic) -> Logic {
    match tree_logic {
        TreeLogic::Sufficiency => Logic::Sufficiency,
        TreeLogic::Necessity => Logic::Necessity,
    }
}

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

// --- M3: macro expand (materializar la reserva) ---

/// Data devuelta por `macro expand`.
#[derive(Debug, Serialize)]
pub struct MacroExpandData {
    /// Long arrow materializada (ahora `Overlay`).
    pub macro_link: String,
    /// IDs de los nodos INT creados, en orden de `--steps` (`INT-xxx`).
    pub created_nodes: Vec<String>,
    /// IDs de los edges creados, en orden de la cadena (`LINK-xxx`, n+1).
    pub created_links: Vec<String>,
    /// Estado resultante de la long arrow ([`MacroEdgeStatus::Overlay`]).
    pub status: MacroEdgeStatus,
}

/// Construye una salida de fallo para `macro expand` con `MacroExpandData` vacío.
///
/// `valid_dag` es un parámetro porque el fallo por ciclo (D9) reporta `valid_dag: false`
/// (contrato idéntico a `link connect`), mientras que el resto de fallos no tocan el DAG.
fn expand_failure(
    ws_name: &str,
    macro_link: &str,
    valid_dag: bool,
    error: OutputError,
) -> CommandOutput<MacroExpandData> {
    CommandOutput {
        success: false,
        action: "macro_expand".to_string(),
        workspace: ws_name.to_string(),
        data: MacroExpandData {
            macro_link: macro_link.to_string(),
            created_nodes: vec![],
            created_links: vec![],
            status: MacroEdgeStatus::Reservation,
        },
        graph_health: GraphHealth {
            valid_dag,
            orphan_nodes_count: 0,
        },
        errors: vec![error],
        warnings: vec![],
    }
}

/// Ejecuta `macro expand`: materializa una reserva en una cadena INT explícita (`Overlay`).
///
/// Transición `Reservation → Overlay` (ADR-013): crea `n` nodos INT (uno por label en `steps`,
/// separadas por comas) y `n+1` edges encadenando `from → INT₁ → … → INTₙ → to`, con la lógica
/// derivada del árbol ([`edge_logic`]). La long arrow pasa a `Overlay` y sus `MacroAssumption`
/// **se conservan** (ahora proyectables por `macro-assume`).
///
/// Los edges son reales ⇒ **bloquea ciclos** (D9): pre-valida el DAG (`from→INT…→to` sumado a
/// `tree.edges`) **antes** de persistir; si cerraría un ciclo devuelve
/// `CIRCULAR_DEPENDENCY_DETECTED` con `cycle_path`, sin escribir ningún fichero (los INT/LINK no
/// se materializan; solo se consumen contadores, contrato idéntico a `link connect`). Muta bajo
/// lock; el historial lo captura el llamador.
///
/// Errores: `TREE_NOT_FOUND`, `MACRO_EDGE_NOT_FOUND`, `NOT_A_RESERVATION` (la macro ya es
/// `Overlay`), `STEPS_REQUIRED` (sin labels no vacías), `CIRCULAR_DEPENDENCY_DETECTED`,
/// `ID_GENERATION_ERROR`, `IO_ERROR`.
pub fn execute_macro_expand(
    storage: &dyn Storage,
    tree_id: &str,
    macro_link: &str,
    steps: &str,
) -> CommandOutput<MacroExpandData> {
    let ws_name = storage.workspace_name().unwrap_or_default();

    let lock_outcome = match storage.acquire_lock("macro expand") {
        Ok(o) => o,
        Err(e) => {
            return expand_failure(
                &ws_name,
                macro_link,
                true,
                OutputError::new("LOCK_ERROR", e.to_string()),
            );
        }
    };

    let mut tree = match storage.load_tree(tree_id) {
        Ok(t) => t,
        Err(_) => {
            let _ = storage.release_lock();
            return expand_failure(
                &ws_name,
                macro_link,
                true,
                OutputError::new("TREE_NOT_FOUND", format!("Tree '{tree_id}' not found")),
            );
        }
    };

    let macro_idx = match tree.macro_edges.iter().position(|m| m.id == macro_link) {
        Some(i) => i,
        None => {
            let _ = storage.release_lock();
            return expand_failure(
                &ws_name,
                macro_link,
                true,
                OutputError::new(
                    "MACRO_EDGE_NOT_FOUND",
                    format!("Macro-edge '{macro_link}' not found in tree '{tree_id}'"),
                ),
            );
        }
    };

    if tree.macro_edges[macro_idx].status != MacroEdgeStatus::Reservation {
        let _ = storage.release_lock();
        return expand_failure(
            &ws_name,
            macro_link,
            true,
            OutputError::new(
                "NOT_A_RESERVATION",
                format!("Macro-edge '{macro_link}' is not a reservation (status is overlay)"),
            )
            .with_context("macro_link", macro_link),
        );
    }

    // Parseo de labels: separadas por comas, sin espacios, sin vacías. Duplicadas permitidas
    // (los INT son entidades distintas; los labels no son únicos). Vacío ⇒ STEPS_REQUIRED.
    let labels: Vec<String> = steps
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    if labels.is_empty() {
        let _ = storage.release_lock();
        return expand_failure(
            &ws_name,
            macro_link,
            true,
            OutputError::new(
                "STEPS_REQUIRED",
                "macro expand requires at least one non-empty step label",
            ),
        );
    }

    // Extremos de la reserva (se materializará from → INT₁ → … → INTₙ → to).
    let from = tree.macro_edges[macro_idx].from.clone();
    let to = tree.macro_edges[macro_idx].to.clone();
    let logic = edge_logic(tree.logic);

    // 1) Mintear todos los INT en orden (contadores). Se construyen en memoria; NO se persiste
    //    ningún fichero hasta después del pre-check DAG (D9), de modo que un ciclo bloqueado no
    //    deja nodos huérfanos en disco (solo consume contadores, como `link connect`).
    let mut int_nodes: Vec<Node> = Vec::with_capacity(labels.len());
    for label in &labels {
        let int_id = match storage.next_id("INT") {
            Ok(id) => id,
            Err(e) => {
                let _ = storage.release_lock();
                return expand_failure(
                    &ws_name,
                    macro_link,
                    true,
                    OutputError::new("ID_GENERATION_ERROR", e.to_string()),
                );
            }
        };
        int_nodes.push(Node {
            id: int_id,
            node_type: NodeType::Int,
            label: label.clone(),
            tags: vec![],
            observable: true,
            epistemic: EpistemicStatus::default(),
            metadata: NodeMetadata {
                status: NodeStatus::Active,
                extra: Default::default(),
            },
        });
    }

    // 2) Mintear los n+1 LINK y construir la cadena en memoria.
    let mut new_edges: Vec<Edge> = Vec::with_capacity(int_nodes.len() + 1);
    // Waypoints: from, INT₁, …, INTₙ, to. Cada par consecutivo es un edge.
    let mut waypoints: Vec<String> = Vec::with_capacity(int_nodes.len() + 2);
    waypoints.push(from);
    waypoints.extend(int_nodes.iter().map(|n| n.id.clone()));
    waypoints.push(to);

    for pair in waypoints.windows(2) {
        let link_id = match storage.next_id("LINK") {
            Ok(id) => id,
            Err(e) => {
                let _ = storage.release_lock();
                return expand_failure(
                    &ws_name,
                    macro_link,
                    true,
                    OutputError::new("ID_GENERATION_ERROR", e.to_string()),
                );
            }
        };
        new_edges.push(Edge {
            id: link_id,
            from: vec![pair[0].clone()],
            to: pair[1].clone(),
            operator: Operator::Single,
            weight: None,
            status: EdgeStatus::Active,
            logic,
            assumptions: vec![],
        });
    }

    // 3) Pre-validar DAG (D9) ANTES de persistir: edges reales ⇒ bloquean ciclos.
    let mut all_edges: Vec<Edge> = tree.edges.clone();
    all_edges.extend(new_edges.iter().cloned());
    if let Err(e) = check_dag(&all_edges, tree_id) {
        let _ = storage.release_lock();
        let err = match &e {
            LtpError::CircularDependencyDetected { cycle_path, .. } => {
                OutputError::new("CIRCULAR_DEPENDENCY_DETECTED", e.to_string()).with_context(
                    "cycle_path",
                    serde_json::Value::Array(
                        cycle_path
                            .iter()
                            .map(|n| serde_json::Value::String(n.clone()))
                            .collect(),
                    ),
                )
            }
            _ => OutputError::new("VALIDATION_ERROR", e.to_string()),
        };
        // No se guardó ningún INT ni el tree ⇒ estado en disco intacto (I9).
        return expand_failure(&ws_name, macro_link, false, err);
    }

    // 4) DAG válido: persistir los INT, luego mutar y guardar el tree.
    for node in &int_nodes {
        if let Err(e) = storage.save_node(node) {
            let _ = storage.release_lock();
            return expand_failure(
                &ws_name,
                macro_link,
                true,
                OutputError::new("IO_ERROR", e.to_string()),
            );
        }
    }

    let created_nodes: Vec<String> = int_nodes.iter().map(|n| n.id.clone()).collect();
    let created_links: Vec<String> = new_edges.iter().map(|e| e.id.clone()).collect();

    // Attach INT al tree + sumar edges.
    for id in &created_nodes {
        tree.nodes.push(NodeRef {
            node_ref: id.clone(),
            role: None,
        });
    }
    tree.edges.extend(new_edges);

    // Materializar la long arrow: interior poblado, estado Overlay. Los MacroAssumption se
    // conservan (ahora sobre un Overlay, proyectables por macro-assume).
    {
        let me = &mut tree.macro_edges[macro_idx];
        me.interior_nodes = created_nodes.clone();
        me.interior_links = created_links.clone();
        me.status = MacroEdgeStatus::Overlay;
    }

    if let Err(e) = storage.save_tree(&tree) {
        let _ = storage.release_lock();
        return expand_failure(
            &ws_name,
            macro_link,
            true,
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
        action: "macro_expand".to_string(),
        workspace: ws_name,
        data: MacroExpandData {
            macro_link: macro_link.to_string(),
            created_nodes,
            created_links,
            status: MacroEdgeStatus::Overlay,
        },
        graph_health: GraphHealth {
            valid_dag: true,
            orphan_nodes_count: 0,
        },
        errors: vec![],
        warnings,
    }
}
