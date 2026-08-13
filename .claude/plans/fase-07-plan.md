# Plan F7 — Supuestos (Assumptions)

## Scope

Implementar `ltp assume add/edit/rm/list/move` y `ltp invalidate`.
Los supuestos (assumptions) son entidades de primer nivel que viven dentro de los edges (ADR-005).

**Archivos principales**: `src/assume/mod.rs` (lógica), `src/main.rs` (dispatch)
**Prerequisitos**: F5 completada (ya tenemos assumptions como Vec<Assumption> en Edge)
**UATs**: 15 (7.1–7.15) — incluye error paths e idempotencia (ADR-010)

---

## Infraestructura Existente

| Componente | Estado | Ubicación |
|-----------|--------|-----------|
| `Assumption` struct | ✅ Definido | `src/link/types.rs:38` |
| `AssumptionStatus` enum (Valid/Invalid/NeedsReview) | ✅ Definido | `src/link/types.rs:31` |
| `Edge.assumptions: Vec<Assumption>` | ✅ Definido | `src/link/types.rs:54` |
| `AssumeAction` enum (CLI args) | ✅ Definido | `src/main.rs:393` |
| `Commands::Invalidate` (CLI args) | ✅ Definido | `src/main.rs:150` |
| `storage.next_id("ASM")` | ✅ Funcional | `src/storage.rs:50` |
| `LtpError::AssumptionNotFound` | ✅ Definido | `src/errors.rs:14` |
| `src/assume/mod.rs` | ⬜ Stub vacío | Solo comentario |
| Dispatch en match principal | ⬜ Cae en wildcard `_` | `src/main.rs:1132` |

---

## Pasos de Implementación

### Paso 1: Funciones de lógica en `src/assume/mod.rs`

Implementar 6 funciones públicas:

#### 1.1 `assume_add(storage, tree_id, link_id, text) -> Result<AssumeAddData>`
- Cargar tree, encontrar edge por `link_id` → error `LINK_NOT_FOUND` si no existe
- Generar `ASM-XXX` via `storage.next_id("ASM")`
- Crear `Assumption { id, status: Valid, text }`
- Push al `Vec<Assumption>` del edge
- Guardar tree
- Retornar id del assumption creado

#### 1.2 `assume_edit(storage, tree_id, asm_id, new_text) -> Result<AssumeEditData>`
- Cargar tree, buscar assumption por ID en todos los edges → error `ASSUMPTION_NOT_FOUND`
- Actualizar `text`
- Guardar tree
- Retornar id + texto actualizado

#### 1.3 `assume_list(storage, tree_id, status_filter) -> Result<AssumeListData>`
- Cargar tree
- Iterar todos los edges, recopilar assumptions
- Filtrar por status si se proporciona
- Retornar lista con: id, text, status, **link_id** (edge al que pertenece)

#### 1.4 `assume_move(storage, tree_id, asm_id, to_link_id) -> Result<AssumeMoveData>`
- Cargar tree
- Encontrar assumption en su edge actual (source) → error `ASSUMPTION_NOT_FOUND`
- Verificar que `to_link_id` existe en el tree → error `LINK_NOT_FOUND`
- Remover del source, push al target
- Guardar tree
- Retornar id + from_link + to_link

#### 1.5 `assume_rm(storage, tree_id, asm_id) -> Result<AssumeRmData>`
- Cargar tree
- Encontrar y remover assumption del edge → error `ASSUMPTION_NOT_FOUND`
- Guardar tree
- Retornar id removido

#### 1.6 `invalidate(storage, tree_id, link_id, asm_id, injection_label) -> Result<InvalidateData>`
- Cargar tree, encontrar edge por link_id → error `LINK_NOT_FOUND`
- Encontrar assumption por asm_id DENTRO de ese edge → error `ASSUMPTION_NOT_IN_LINK`
- **Idempotencia (ADR-010)**: si ASM ya `invalid` + edge ya `broken` → retornar success, `changed: false`, warning `ALREADY_INVALIDATED`. No crear INJ. Return early.
- **Auto-reparación**: si estado inconsistente (ASM invalid/edge active o viceversa), reparar con warning.
- Marcar assumption como `Invalid`
- Marcar edge como `Broken`
- Si `injection_label` es Some:
  - Crear nodo INJ directamente: `Node { id: next_id("INJ"), node_type: Inj, label, ... }` + `storage.save_node()` (patrón de node_split)
  - Retornar el ID del INJ creado
- Guardar tree
- Retornar: asm_id, link_id, link_status, injection_id (Option), **changed: bool**

---

### Paso 2: Dispatch en `src/main.rs`

Reemplazar el wildcard `_ =>` por los arms explícitos:

```rust
Commands::Assume { action } => match action {
    AssumeAction::Add { tree, link, text } => { ... }
    AssumeAction::Edit { tree, asm, text } => { ... }
    AssumeAction::Rm { tree, asm } => { ... }
    AssumeAction::List { tree, status } => { ... }
    AssumeAction::Move { tree, asm, to_link } => { ... }
},
Commands::Invalidate { tree, link, asm, injection } => { ... }
```

Cada arm:
1. Llama a la función de lógica correspondiente
2. Construye `CommandOutput<T>` con el pattern establecido
3. `render_output` + `process::exit(1)` si error

---

### Paso 3: Tests de integración (UATs 7.1–7.15)

Crear `tests/assume_test.rs` con 15 tests:

| UAT | Test | Validación |
|-----|------|-----------|
| 7.1 | `test_assume_add` | Crea ASM-001 dentro del edge; output JSON correcto |
| 7.2 | `test_assume_edit` | Actualiza texto del assumption |
| 7.3 | `test_assume_list` | Lista todos con status + link_id |
| 7.4 | `test_assume_list_filter` | Filtra por status `valid` |
| 7.5 | `test_assume_move` | Mueve assumption de un edge a otro |
| 7.6 | `test_assume_rm` | Elimina assumption del edge |
| 7.7 | `test_invalidate_basic` | Marca ASM invalid + edge broken + `changed: true` |
| 7.8 | `test_invalidate_with_injection` | Además crea nodo INJ en pool |
| 7.9 | `test_assume_add_link_not_found` | Error LINK_NOT_FOUND |
| 7.10 | `test_assume_add_tree_not_found` | Error TREE_NOT_FOUND |
| 7.11 | `test_assume_edit_asm_not_found` | Error ASSUMPTION_NOT_FOUND |
| 7.12 | `test_assume_move_target_not_found` | Error LINK_NOT_FOUND, ASM no se mueve |
| 7.13 | `test_invalidate_asm_not_in_link` | Error ASSUMPTION_NOT_IN_LINK |
| 7.14 | `test_invalidate_idempotent` | success + changed:false + warning, no crea INJ |
| 7.15 | `test_assume_list_multi_edge` | 3 ASMs en 2 edges, cada entry muestra link_id |

Cada test:
1. `ltp init` + setup (add nodes, tree, attach, connect)
2. Ejecuta el comando bajo test
3. Verifica stdout JSON (success, data fields, **changed** si aplica)
4. Verifica estado en disco (tree file, node file si INJ)
5. Para error tests: verifica `success: false` + error code correcto

---

## Decisiones Técnicas

1. **Búsqueda de ASM por ID**: scan lineal de `tree.edges[*].assumptions[*]`. Con ~50 edges y ~5 ASMs por edge, esto es instantáneo. No requiere índice.

2. **Invalidate como comando top-level** (no subcomando de `assume`): ya está definido así en el CLI. Semánticamente es una operación que afecta edge + assumption + potencialmente crea nodo. Es correcto que sea independiente.

3. **Creación de INJ en invalidate**: construir `Node` struct directamente + `storage.next_id("INJ")` + `storage.save_node()` (patrón de `node_split`, línea 1067). El nodo queda en pool global, sin attach a ningún tree (borrador).

4. **Idempotencia de invalidate (ADR-010)**: si estado final ya alcanzado → success + `changed: false` + warning. Sin side-effects (no crea INJ en retry). Auto-repara estados inconsistentes.

5. **Error codes**:
   - `ASSUMPTION_NOT_FOUND` — ASM-XXX no existe en el tree
   - `LINK_NOT_FOUND` — edge no existe en el tree
   - `TREE_NOT_FOUND` — tree no existe
   - `ASSUMPTION_NOT_IN_LINK` — para invalidate, el ASM no pertenece al link especificado
   - `ALREADY_INVALIDATED` — warning (no error) cuando re-invalidate

---

## Orden de Ejecución

1. Implementar funciones en `src/assume/mod.rs` (tipos de retorno + lógica)
2. Añadir dispatch en `src/main.rs`
3. Escribir tests de integración
4. Verificación: `cargo check` + `clippy` + `test` + `fmt`
5. Actualizar PROGRESS.md

---

## Riesgos / Consideraciones

- **node_add para INJ en invalidate**: El lock NO es reentrant (mismo PID → `WORKSPACE_LOCKED`). No puedo llamar `execute_node_add` desde `invalidate` porque ambos adquieren lock. **Solución**: construir el `Node` struct directamente + `storage.next_id("INJ")` + `storage.save_node()`, tal como hace `node_split` (línea 1067 de `node/commands.rs`).
- **Counter ASM**: ya existe en counters.json (inicializado a 0 en F1). Solo necesito `storage.next_id("ASM")`.
- **Lock strategy**: cada `execute_assume_*` y `execute_invalidate` adquieren/liberan su propio lock (patrón estándar de todos los comandos mutantes).
- **Localización de ASM por ID**: para `assume edit/rm/move`, el usuario pasa `--tree T --asm ASM-001` (sin link). La función debe escanear todos los edges del tree para encontrarlo. Para `invalidate`, el usuario pasa `--tree T --link L1 --asm ASM-001` — verificar que el ASM pertenece a ESE link (error `ASSUMPTION_NOT_IN_LINK` si no).

---

## Estimación

- Complejidad: **Media** (plan original, no cambia con +7 UATs — los nuevos son error paths simples)
- UATs: 15
- Peso estimado: 6% del total
