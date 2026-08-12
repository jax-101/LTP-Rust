# Plan Ejecutable — Fase 2a: Nodos Standalone

## Objetivo

Implementar `node add`, `node edit`, `node list` y `node search` como operaciones standalone sobre el pool global (`nodes/`). Incluye linter CLR#2 (advertencia por conjunciones causales).

## Entregables

1. Comando `ltp node add` con generación de ID, persistencia y warning CLR#2
2. Comando `ltp node edit` (label, add-tag, rm-tag, observable)
3. Comando `ltp node list` sobre pool global (sin requerir tree)
4. Comando `ltp node search` por substring en label (sin requerir tree)
5. Linter CLR#2: detección de conjunciones causales ("porque", "para", "in order to", "because")
6. 9 UATs como tests de integración CLI

## Decisiones de Diseño

1. **`--tree` será opcional en `list` y `search`**: sin `--tree`, operan sobre el pool global completo. El filtrado por tree se añadirá en F3 cuando existan trees.
2. **Linter CLR#2 es un warning, no un error**: el nodo se crea siempre; el warning se incluye en el output JSON.
3. **Lock se adquiere en add/edit** (son mutantes): el pipeline es lock → mutación atómica → release.
4. **Node file format**: `nodes/<ID>.json` donde ID = `<TYPE>-<SEQ>` (e.g., `UDE-001`).

## Tareas Ordenadas

### T1: Módulo CLR linter (`src/node/clr_lint.rs`)

Función que recibe un label y retorna `Vec<OutputWarning>` con las conjunciones detectadas:
- Palabras clave: `"porque"`, `"para "` (con espacio, evitar falsos positivos con "paralelismo"), `"because"`, `"in order to"`, `"y"` como causal (heurística: " y " entre frases verbales).
- Cada match genera warning con code `CLR2_CONJUNCTION_DETECTED`.

### T2: Implementar `node add` (`src/node/commands.rs`)

Lógica:
1. Acquire lock ("node add").
2. Parsear `NodeType` desde string (case-insensitive).
3. Generar ID via `storage.next_id(type)`.
4. Construir `Node` con status `active`, tags opcionales, observable por defecto `true`.
5. Ejecutar CLR#2 linter sobre el label → warnings.
6. `storage.save_node(&node)` (escritura atómica).
7. Release lock.
8. Retornar `CommandOutput` con `data.id`, `data.node_type`, `data.label`.

### T3: Implementar `node edit` (`src/node/commands.rs`)

Lógica:
1. Acquire lock ("node edit").
2. `storage.load_node(id)` — error si no existe.
3. Aplicar mutaciones opcionales: label, add-tag (append sin duplicar), rm-tag (filter out), observable.
4. Si label cambiado → ejecutar CLR#2 linter → warnings.
5. `storage.save_node(&node)`.
6. Release lock.
7. Retornar `CommandOutput` con nodo actualizado.

### T4: Implementar `node list` (`src/node/commands.rs`)

Lógica:
1. No requiere lock (lectura).
2. `storage.list_node_ids()` → para cada ID, `storage.load_node(id)`.
3. Aplicar filtros opcionales: `--type` (Vec<NodeType>), `--status` (Vec<NodeStatus>).
4. Retornar `CommandOutput` con `data.nodes: [...]` (id, type, label, status).

### T5: Implementar `node search` (`src/node/commands.rs`)

Lógica:
1. No requiere lock (lectura).
2. Cargar todos los nodos del pool.
3. Filtrar por `label.to_lowercase().contains(query.to_lowercase())`.
4. Retornar matches con id, type, label.

### T6: Conectar comandos al CLI (`src/main.rs`)

- Modificar `NodeAction::List` para que `--tree` sea `Option<String>` (no requerido).
- Modificar `NodeAction::Search` para que `--tree` sea `Option<String>`.
- Dispatch `NodeAction::Add`, `Edit`, `List`, `Search` a las funciones en `src/node/commands.rs`.
- Los comandos mutantes (`add`, `edit`) adquieren/liberan lock.

### T7: Tests de integración (`tests/fase_02a.rs`)

9 UATs:
- **2a.1**: `node add "..." --type UDE --tags logistica,critico` → crea `UDE-001.json`, counter incrementa.
- **2a.2**: `node add "Vendemos poco porque no hay marketing" --type UDE` → warning CLR2.
- **2a.3**: `node add "Falta de capacidad" --type RC` → crea `RC-001.json`.
- **2a.4**: `node edit UDE-001 --label "Nuevo texto"` → label actualizado en disco.
- **2a.5**: `node edit UDE-001 --add-tag urgente` → tag añadido.
- **2a.6**: `node edit UDE-001 --rm-tag critico` → tag eliminado.
- **2a.7**: `node edit UDE-001 --observable false` → marcado no observable.
- **2a.8**: `node list` → lista todos los nodos del pool.
- **2a.9**: `node search --query "entrega"` → encuentra por substring.

## Criterios de Aceptación

- [ ] `cargo check --all-targets --all-features` pasa.
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` pasa.
- [ ] `cargo test --workspace` pasa (11 tests previos + 9 nuevos = 20).
- [ ] `cargo fmt --all -- --check` pasa.
- [ ] No hay `.unwrap()` ni `.expect()` en código nuevo de producción.
- [ ] No queda `dbg!()` ni `println!()` en código de producción.
- [ ] Todos los items públicos nuevos tienen `///` doc comments.

## Archivos a Crear/Modificar

| Archivo | Acción |
|---------|--------|
| `src/node/clr_lint.rs` | Crear — linter CLR#2 |
| `src/node/commands.rs` | Crear — lógica de add/edit/list/search |
| `src/node/mod.rs` | Modificar — re-exportar nuevos módulos |
| `src/main.rs` | Modificar — dispatch node commands, hacer --tree opcional |
| `tests/fase_02a.rs` | Crear — 9 UATs de integración |

## Dependencias Externas

Ninguna nueva.

## Riesgos

- El CLI actual tiene `--tree` como obligatorio en `list` y `search`. Se cambiará a `Option<String>` — no rompe nada porque aún no se usa desde tests.
