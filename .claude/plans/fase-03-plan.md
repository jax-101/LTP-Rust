# Plan Ejecutable — Fase 3: Vistas (Trees)

## Objetivo

Implementar `tree new`, `tree list`, `tree rm`, `tree attach`, `tree detach`, `tree clone`, `tree diff` y `tree walk` como operaciones sobre las vistas topológicas.

## Entregables

1. Comando `ltp tree new` con generación de ID slug-based y lógica por tipo
2. Comando `ltp tree list` con resumen por tree
3. Comando `ltp tree rm`
4. Comandos `ltp tree attach/detach`
5. Comando `ltp tree clone`
6. Comando `ltp tree diff`
7. Comando `ltp tree walk` con orden topológico y reverso
8. 11 UATs como tests de integración CLI

## Decisiones de Diseño

1. **Tree ID format**: `tree-<type>-<slug(name)>` (e.g., `tree-crt-logistica`). Slug: lowercase, espacios→guiones, caracteres no-ascii simplificados.
2. **Logic por tipo**: `gt/crt/frt/tt` → sufficiency; `ec/prt` → necessity.
3. **Attach/detach son mutantes**: requieren lock.
4. **Walk topological**: DFS post-order sobre el DAG de edges. Para trees sin edges, simplemente lista nodos attached.
5. **Diff**: reporta nodos y edges added/removed comparando dos trees.
6. **Clone**: copia profunda de edges (IDs nuevos via LINK counter), nodos son referencias compartidas al pool.

## Tareas Ordenadas

### T1: Helpers de slugificación y lógica (`src/tree/commands.rs`)

- `slugify(name) -> String`: lowercase, reemplaza espacios y caracteres especiales por guiones, elimina guiones duplicados/trailing.
- `logic_for_type(tree_type) -> TreeLogic`: gt/crt/frt/tt → Sufficiency; ec/prt → Necessity.
- `parse_tree_type(s) -> Result<TreeType>`: case-insensitive parse.

### T2: Implementar `tree new`

Lógica:
1. Acquire lock.
2. Parsear TreeType.
3. Generar ID: `tree-<type>-<slug(name)>`.
4. Verificar que no exista ya un tree con ese ID.
5. Construir Tree vacío (nodes: [], edges: [], etc.).
6. `storage.save_tree(&tree)`.
7. Release lock.

### T3: Implementar `tree list`

Lógica (lectura, sin lock):
1. `storage.list_tree_ids()`.
2. Para cada tree, cargar y extraer: id, name, type, node_count, edge_count.

### T4: Implementar `tree rm`

Lógica:
1. Acquire lock.
2. Verificar que el tree existe.
3. `storage.delete_tree(id)`.
4. Release lock.
5. Nodos quedan en el pool global.

### T5: Implementar `tree attach` y `tree detach`

Attach:
1. Acquire lock.
2. Verificar que el nodo existe en el pool.
3. Cargar tree.
4. Verificar que el nodo no está ya en el tree.
5. Añadir NodeRef { ref: node_id, role }.
6. Guardar tree.
7. Release lock.

Detach:
1. Acquire lock.
2. Cargar tree.
3. Quitar nodo de `nodes[]`.
4. Quitar todos los edges donde el nodo participe (from o to).
5. Guardar tree.
6. Release lock.

### T6: Implementar `tree clone`

Lógica:
1. Acquire lock.
2. Cargar tree original.
3. Generar nuevo ID con el nuevo nombre.
4. Copiar nodes[] (mismas refs).
5. Copiar edges con nuevos IDs (via storage.next_id("LINK")).
6. Copiar feedback_edges, macro_edges, nbr_branches con nuevos IDs si aplica.
7. Guardar nuevo tree.
8. Release lock.

### T7: Implementar `tree diff`

Lógica (lectura, sin lock):
1. Cargar ambos trees.
2. Comparar nodes: added (en B pero no A), removed (en A pero no B).
3. Comparar edges por (from, to, operator): added, removed.
4. Retornar resumen.

### T8: Implementar `tree walk`

Lógica (lectura, sin lock):
1. Cargar tree.
2. Si no hay edges: listar nodos en orden de ID.
3. Si hay edges: construir grafo de adyacencia.
4. Topological order (Kahn's algorithm): desde nodos sin dependencias entrantes hacia arriba.
5. Reverse: invertir el resultado.
6. Para cada nodo en el recorrido: incluir edges entrantes/salientes y assumptions.

### T9: Conectar comandos al CLI (`src/main.rs`)

Dispatch para todos los TreeAction variants.

### T10: Tests de integración (`tests/fase_03.rs`)

11 UATs (3.1–3.11).

## Criterios de Aceptación

- [ ] Las 4 verificaciones pasan (check, clippy, test, fmt).
- [ ] No hay `.unwrap()` ni `.expect()` en código nuevo de producción.
- [ ] Todos los items públicos nuevos tienen `///` doc comments.
- [ ] 36 tests totales pasando (25 previos + 11 nuevos).

## Archivos a Crear/Modificar

| Archivo | Acción |
|---------|--------|
| `src/tree/commands.rs` | Crear — lógica de todos los tree commands |
| `src/tree/mod.rs` | Modificar — re-exportar commands |
| `src/main.rs` | Modificar — dispatch tree commands |
| `tests/fase_03.rs` | Crear — 11 UATs |

## Riesgos

- `tree walk` con orden topológico requiere Kahn's algorithm (BFS basado en in-degree). Ya existe check_dag con DFS, pero walk necesita orden, no solo validación.
- `tree clone` debe generar IDs nuevos para edges → incrementa contadores LINK. Si el tree original tiene muchos edges, se generan muchos IDs en una sola operación.
