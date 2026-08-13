# Plan Fase 8: Navegación (Trace & Inspección)

## Scope

Implementar `ltp trace`, `ltp link inspect` y `ltp link find` — comandos de navegación read-only del grafo causal.

**Decisiones vinculantes**:
- ADR-010 Decisión 3: trace incluye broken/superseded/needs_review links sin detenerse. Cada entry tiene `link_status`. Output incluye `chain_health`.
- ADR-001: el motor informa, no interpreta.
- Feedback edges incluidos por defecto; excluir con `--no-feedback`.
- NBR edges excluidos por defecto; incluir con `--nbr`.

## Archivos a crear/modificar

| Archivo | Acción |
|---------|--------|
| `src/trace/mod.rs` | Implementar lógica completa (reemplazar placeholder) |
| `src/main.rs` | Wire dispatch `Commands::Trace`, `LinkAction::Inspect`, `LinkAction::Find` |
| `src/lib.rs` | Ya exporta `pub mod trace` — sin cambios |
| `tests/trace_tests.rs` | Integration tests cubriendo 15 UATs |

## Estructuras de datos (Type-First)

### TraceEntry (cada nodo en la cadena)

```rust
#[derive(Debug, Clone, Serialize)]
pub struct TraceEntry {
    pub node: String,
    pub link_to_next: Option<LinkSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LinkSummary {
    pub id: String,
    pub status: String,
    pub operator: String,
}
```

### ChainHealth

```rust
#[derive(Debug, Clone, Serialize)]
pub struct ChainHealth {
    pub fully_connected: bool,
    pub broken_links: Vec<String>,
    pub superseded_links: Vec<String>,
}
```

### TraceData (output completo)

```rust
#[derive(Debug, Clone, Serialize)]
pub struct TraceData {
    pub node_id: String,
    pub tree_id: String,
    pub direction: String,
    pub depth: Option<usize>,
    pub chain: Vec<TraceEntry>,
    pub feedback_loops: Vec<FeedbackSummary>,
    pub chain_health: ChainHealth,
}

#[derive(Debug, Clone, Serialize)]
pub struct FeedbackSummary {
    pub id: String,
    pub from: String,
    pub to: String,
    pub loop_type: String,
}
```

### LinkInspectData

```rust
#[derive(Debug, Clone, Serialize)]
pub struct LinkInspectData {
    pub id: String,
    pub from: Vec<String>,
    pub from_labels: Vec<NodeLabel>,
    pub to: String,
    pub to_label: String,
    pub operator: String,
    pub weight: Option<f64>,
    pub status: String,
    pub logic: String,
    pub assumptions: Vec<AssumptionDetail>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NodeLabel {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AssumptionDetail {
    pub id: String,
    pub text: String,
    pub status: String,
}
```

### LinkFindData

```rust
#[derive(Debug, Clone, Serialize)]
pub struct LinkFindData {
    pub from: String,
    pub to: String,
    pub tree_id: String,
    pub links: Vec<LinkFindEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LinkFindEntry {
    pub id: String,
    pub operator: String,
    pub status: String,
}
```

## Algoritmo de Trace

### Downstream (BFS desde node_id siguiendo `from → to`)
1. Cargar tree. Verificar que node_id existe en pool y está attached.
2. Construir adjacency map: para cada edge, `from[i] → (edge, to)`.
3. BFS desde node_id:
   - Por cada edge que tenga node_id en `from[]`, visitar `edge.to`.
   - Registrar TraceEntry con link_to_next = info del edge usado.
   - Respetar `--depth` (nivel max de BFS).
4. El último nodo de cada rama tiene `link_to_next: null`.

### Upstream (BFS inverso: desde node_id siguiendo `to → from`)
1. Construir reverse adjacency: para cada edge, `to → (edge, from[])`.
2. BFS desde node_id siguiendo aristas inversas.
3. Si un edge tiene multiples `from`, todos se agregan como entries al mismo depth.

### Feedback edges
- Por defecto, recorrer `feedback_edges` que tocan nodos de la cadena ya encontrada.
- Reportar en sección separada `feedback_loops`.
- Con `--no-feedback`: ignorar completamente.

### NBR edges
- Por defecto NO se incluyen.
- Con `--nbr`: incluir `nbr_branches[].edges` en el grafo de búsqueda.

### Chain health
- `fully_connected`: true si ningún link en la cadena tiene status != active.
- `broken_links`: IDs de links con status `broken`.
- `superseded_links`: IDs de links con status `superseded`.

## Pasos de Implementación

### Paso 1: Types + función `execute_trace`
- Definir structs en `src/trace/mod.rs`.
- Implementar `execute_trace(storage, node_id, tree_id, direction, depth, no_feedback, nbr)`.
- Validaciones: TREE_NOT_FOUND, NODE_NOT_FOUND, NODE_NOT_IN_TREE.
- Algoritmo BFS bidireccional.

### Paso 2: `execute_link_inspect` y `execute_link_find`
- Añadir en `src/trace/mod.rs` (misma familia de navegación).
- `execute_link_inspect`: carga tree, busca edge por ID, enriquece con labels de nodos.
- `execute_link_find`: filtra edges donde `from` contiene el nodo A y `to == B` (o viceversa con feedback).

### Paso 3: Wire en `main.rs`
- Reemplazar el wildcard `_ =>` de `Commands::Trace` con dispatch real.
- Reemplazar el wildcard `_ =>` de `LinkAction::Inspect` y `LinkAction::Find`.
- Añadir `use ltp_engine::trace::{execute_trace, execute_link_inspect, execute_link_find};`

### Paso 4: Integration tests
- Crear `tests/trace_tests.rs` con setup helpers.
- Cubrir las 15 UATs (8.1–8.15).

### Paso 5: Verificación
- `cargo check --all-targets --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- `cargo fmt --all -- --check`
- Actualizar PROGRESS.md

## UATs Mapping

| UAT | Función | Test case |
|-----|---------|-----------|
| 8.1 | execute_trace downstream | trace_downstream_full_chain |
| 8.2 | execute_trace upstream | trace_upstream_to_roots |
| 8.3 | execute_trace upstream --depth 2 | trace_upstream_depth_limit |
| 8.4 | execute_trace con feedback | trace_includes_feedback_by_default |
| 8.5 | execute_trace --no-feedback | trace_excludes_feedback |
| 8.6 | execute_trace --nbr | trace_includes_nbr_edges |
| 8.7 | execute_link_inspect | link_inspect_full_detail |
| 8.8 | execute_link_find | link_find_between_nodes |
| 8.9 | execute_trace nodo inexistente | trace_node_not_found |
| 8.10 | execute_trace tree inexistente | trace_tree_not_found |
| 8.11 | execute_trace nodo no attached | trace_node_not_in_tree |
| 8.12 | execute_trace nodo hoja | trace_leaf_node_empty_chain |
| 8.13 | execute_trace nodo raíz | trace_root_node_empty_chain |
| 8.14 | execute_trace broken links | trace_broken_link_chain_health |
| 8.15 | execute_link_inspect con assumptions | link_inspect_with_assumptions |
