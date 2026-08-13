# Plan Fase 9: Abstracción (Path)

## Scope

Implementar `ltp path collapse`, `ltp path explode`, `ltp path replace` en `src/path/mod.rs`.

## Fuentes

- **ENGINE_SPEC.md §2.10** — Semántica de cada comando.
- **ADR-010 Decisión 1** — Collapse opera sobre sub-grafos completos (from→to DAG), no solo cadenas lineales. Error `NO_DIRECTED_PATH` y `NESTED_MACRO_NOT_ALLOWED`.
- **PLAN.md F9** — 12 UATs (9.1–9.12).
- **Patrón** — `src/trace/mod.rs` (output types + execute functions que reciben `&dyn Storage`).

## Diseño

### Tipos de salida (`src/path/mod.rs`)

```rust
pub struct CollapseData {
    pub macro_edge_id: String,
    pub from: String,
    pub to: String,
    pub label: String,
    pub interior_nodes: Vec<String>,
    pub interior_links: Vec<String>,
}

pub struct ExplodeData {
    pub created_node_id: String,
    pub created_links: Vec<String>,
    pub removed_assumption: String,
    pub original_link_removed: bool,
}

pub struct ReplaceData {
    pub macro_link: String,
    pub by_node: String,
    pub superseded_links: Vec<String>,
    pub superseded_nodes: Vec<String>,
    pub new_links: Vec<String>,
}
```

### Algoritmos

#### `path collapse`

1. Cargar tree. Validar que `from` y `to` están attached.
2. **Sub-grafo dirigido (BFS forward desde `from`)**:
   - Desde `from`, expandir downstream por edges del tree.
   - Recolectar nodos alcanzables hasta `to`.
   - Si `to` no es alcanzable → `NO_DIRECTED_PATH`.
3. **Interior**: nodos del sub-grafo excluyendo from y to. Links: edges cuyo `from[]` y `to` estén ambos en el sub-grafo.
4. **Validar**: si algún edge interior es de tipo macro_edge (ya existe macro_edge cuyo `interior_links` intersecta) → `NESTED_MACRO_NOT_ALLOWED`.
5. Generar ID `MACRO-NNN` vía `storage.next_id("MACRO")`.
6. Crear `MacroEdge { id, from, to, label, interior_nodes, interior_links, status: "active" }` y pushear a `tree.macro_edges`.
7. `save_tree`. Release lock.

#### `path explode`

1. Cargar tree. Buscar edge `link_id` (en edges o nbr_branches).
2. Verificar que `asm_id` existe en el edge → `ASSUMPTION_NOT_IN_LINK` si no.
3. Crear nodo INT con `storage.next_id("INT")` y `label`. Guardar nodo.
4. Attach nodo al tree (push `NodeRef`).
5. Crear 2 edges nuevos (IDs vía `next_id("LINK")`):
   - `edge_a`: from = original.from, to = INT, operator = SINGLE, logic = original.logic, status = active.
   - `edge_b`: from = [INT], to = original.to, operator = SINGLE, logic = original.logic, status = active.
6. Eliminar el assumption del edge original.
7. Eliminar el edge original del tree. Insertar los 2 nuevos.
8. `save_node`, `save_tree`. Release lock.

#### `path replace`

1. Cargar tree. Buscar macro_edge por ID → `MACRO_EDGE_NOT_FOUND`.
2. Verificar que `by_node` existe en pool → `NODE_NOT_FOUND`.
3. Marcar como `superseded` todos los edges del tree cuyo ID esté en `interior_links`.
4. Marcar como `superseded` el status de los nodos interiores (`interior_nodes`) — NO los elimina, solo metadata.status.
5. Attach `by_node` al tree si no está ya.
6. Crear 2 nuevos edges:
   - from = macro_edge.from → by_node
   - from = by_node → macro_edge.to
7. Eliminar el macro_edge de `tree.macro_edges`.
8. `save_tree`, `save_node` (update metadata de nodos). Release lock.

### Lock

Los 3 comandos son mutantes → `acquire_lock` / `release_lock`.

### Errores específicos

| Código | Comando | Condición |
|--------|---------|-----------|
| `NO_DIRECTED_PATH` | collapse | No existe camino from→to |
| `NESTED_MACRO_NOT_ALLOWED` | collapse | Sub-grafo contiene macro_edge interior |
| `ASSUMPTION_NOT_IN_LINK` | explode | ASM no existe en el edge |
| `MACRO_EDGE_NOT_FOUND` | replace | Macro edge ID inexistente |
| `NODE_NOT_FOUND` | replace/explode | Nodo no existe en pool |
| `TREE_NOT_FOUND` | todos | Tree no existe |
| `NODE_NOT_IN_TREE` | collapse | from/to no attached al tree |

## Pasos de implementación

1. **Types + stubs** — Definir structs de output y firmas de `execute_path_collapse`, `execute_path_explode`, `execute_path_replace` en `src/path/mod.rs`.
2. **Collapse** — Implementar BFS + creación de macro_edge.
3. **Explode** — Implementar partición de edge + creación de nodo INT.
4. **Replace** — Implementar supersede + reconexión.
5. **Wire CLI** — Añadir match arm en `src/main.rs` para `Commands::Path { action }`.
6. **Tests** — Crear `tests/fase_09.rs` con UATs 9.1–9.12.
7. **Verificación** — `cargo check` + `clippy` + `test` + `fmt`.
8. **PROGRESS.md** — Actualizar avance global.

## UATs cubiertas

9.1–9.12 según PLAN.md.
