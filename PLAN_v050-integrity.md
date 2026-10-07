# PLAN — v0.5.0 (MINOR): integridad global de `node split` / `node rm`

> Restricción vigente: **no stagear, commitear ni editar** `RFC-002_meta-graph.md`, `RFC-002_appendix_20-transitions.md` ni `RFC-002_appendix_use-cases.md`.

## Contexto

PROGRESS.md (entrada v0.4.0) registra dos huecos: `node split` borra el nodo globalmente pero solo reescribe el árbol indicado, y `node rm` ignora `macro_edges`. La investigación (graph-first, `src/node/commands.rs`, `src/validate/`, `src/macro_edge/`, `src/path/`) amplía el diagnóstico:

| Estructura | `split` árbol `--tree` | `split` otros árboles | `rm` (todos) |
|---|---|---|---|
| `nodes[]`, `edges` | ✅ | ❌ | ✅ |
| `nbr_branches` (incl. nodos solo-rama) | ✅ | ❌ | ✅ |
| `feedback_edges` | ❌ | ❌ | ✅ |
| `macro_edges` (`from`/`to`/`interior_*`) | ❌ | ❌ | ❌ |

Consecuencias verificadas:
- `macro expand` (`src/macro_edge/mod.rs` ~L390) no re-valida extremos y `path replace` (`src/path/mod.rs` ~L857) usa `macro_from`/`macro_to` sin comprobarlos ⇒ una macro colgante **materializa edges rotos** (`REFERENTIAL_INTEGRITY_VIOLATION`).
- `validate` solo comprueba integridad sobre `tree.edges` del tronco (`src/validate/integrity.rs`) ⇒ los workspaces ya dañados por v0.4.0 no lo detectan.
- `rm` y `split` saltan en silencio los árboles ilegibles (`Err(_) => continue`) ⇒ dejan referencias colgantes sin avisar.
- Undo hace snapshot de `nodes/` + `trees/` + `knowledge/` completos ⇒ reescribir N árboles es deshacible sin cambios en `history`.

**Versionado (decidido con Six Hats, RELEASE_POLICY §1)**: el warning nuevo `MACRO_EDGE_REMOVED` y el campo nuevo `affected_trees` en `node split` son aditivos ⇒ **MINOR v0.5.0**. El gate de consumidores (`>= 0.4.0`) no cambia.

## 0. codebase-memory (antes de cada tarea)

- Índice fresco al planificar (indexed_at 2026-10-06T11:48:48Z, 0 ficheros parciales).
- Antes de cada tarea: `detect_changes` → `index_repository` incremental si hay deriva; exploración graph-first (`search_graph`, `trace_path`, `get_code_snippet`), grep solo para literales.
- Tras cada commit: reindexar (símbolos nuevos `redirect_split`, `prune_removed`, `PruneReport`, `check_tree_integrity`).

## 1. Decisiones

| D | Decisión | Justificación (Negro obligatorio) |
|---|---|---|
| **D-1** Alcance del split | El split reescribe **todos** los árboles donde aparece el nodo (tronco, ramas NBR, feedback, macros). `--tree` sigue obligatorio como árbol de contexto (`NODE_NOT_IN_TREE` se mantiene). | El nodo es una entidad global del pool y las refs entrantes ya se reescriben globalmente (ADR-015). Alternativa "split local, conservar el original si sigue en otros árboles" contradice ENGINE_SPEC ("Elimina el nodo original") y duplicaría identidad. Riesgo: un analista no ve el impacto en otros árboles ⇒ mitigado con `affected_trees`. |
| **D-2** Dirección | Regla única *entrante → `first`, saliente → `second`*: edge `to`/feedback `to`/macro `to`/NBR `source_node` → `first`; edge `from[]`/feedback `from`/macro `from` → `second`; en `nodes[]` y `macro.interior_nodes` el original se sustituye **en su posición** por los hijos que aún no estén en la lista; una entrada existente nunca se mueve ni se reconstruye (conserva su sitio y su `role`). | Redirigir solo puede eliminar caminos (no hay edge `first→second`) ⇒ no crea ciclos; no se re-ejecuta DFS. El orden por posición mantiene el diff git mínimo y determinista. |
| **D-3** Macros en `rm` | (a) extremo `from`/`to` borrado ⇒ la macro se elimina (con sus `MacroAssumption`), warning `MACRO_EDGE_REMOVED` `reason=endpoint_removed`; (b) nodo interior borrado ⇒ se quita de `interior_nodes`, y los IDs de los edges eliminados se quitan de `interior_links`; (c) un `Overlay` que este `rm` ha tocado (le quitó algún nodo o link interior) y al que no le queda **ningún link interior vivo** (presente en `tree.edges`) se elimina, `reason=interior_emptied`. Las macros no tocadas no se modifican nunca; los IDs fantasma que dejan los comandos `link` no se limpian. Una `Reservation` (interior vacío por construcción) solo puede caer por (a). | Simetría con `NBR_BRANCH_REMOVED`. Se descarta bloquear con `--force` (requiere error nuevo y rompe el patrón "limpiar y avisar"). Un Overlay sin interior no es un estado válido de ADR-013 y degradarlo a `Reservation` sería una transición implícita no autorada. La pérdida de `MacroAssumption` es recuperable con `undo` y queda explícita en el warning (`assumption_ids`). |
| **D-4** Árbol ilegible | `split` y `rm` cargan **todos** los árboles antes de escribir nada; si alguno no se puede leer ⇒ `IO_ERROR {tree_id}` y 0 bytes escritos, 0 contadores consumidos. | Fail-closed: hoy el `continue` silencioso produce justamente la corrupción que este paquete arregla. Coste: un workspace con un árbol corrupto bloquea `rm`/`split` hasta repararlo (aceptable; `validate` lo señala). |
| **D-5** Integridad en `validate` | `REFERENTIAL_INTEGRITY_VIOLATION` (código existente, mismo significado) se amplía a `nodes[]`, `feedback_edges`, edges de `nbr_branches` + `source_node`, y `macro_edges` (`from`, `to`, `interior_nodes`). Contexto: siempre `tree_id`, `node_id`, `location` (`nodes`/`edges`/`feedback_edges`/`nbr_branches`/`macro_edges`) y el ID del contenedor (`edge_id`/`feedback_id`/`nbr_id`/`macro_link`). | Única vía de diagnóstico para workspaces dañados por v0.4.0. Riesgo: workspaces que hoy "pasan" empezarán a fallar ⇒ correcto (están rotos), se documenta en CHANGELOG/INTEGRATION. Un nodo en disco pero ilegible (`NODE_UNREADABLE`) cuenta como existente en el pool ⇒ no duplica error. |
| **D-6** Fuera de alcance | Los hijos de un split mantienen `role: None` (comportamiento actual); los knowledge links al nodo partido no se reescriben ni avisan; `tree detach` de un extremo de macro. | Registrados en §6. |

## 2. Tipos y funciones puras (type-first) — `src/meta/integrity.rs` (semilla `ltp-core`, sin E/S)

```rust
/// Motivo por el que `node rm` elimina una long arrow.
/// `as_str()` es la única fuente del nombre de wire; `impl Serialize` delega en ella.
pub enum MacroRemovalReason { EndpointRemoved, InteriorEmptied }

/// Long arrow eliminada por la poda (campos movidos desde el `MacroEdge`, sin clones).
/// `assumption_ids` en orden de almacenamiento (= creación = numérico); sin `sort()` textual.
pub struct RemovedMacro {
    pub id: String, pub reason: MacroRemovalReason, pub status: MacroEdgeStatus,
    pub from: String, pub to: String, pub assumption_ids: Vec<String>,
}

/// Resultado de podar un árbol tras `node rm`.
pub struct PruneReport {
    pub edges_removed: usize,          // tronco + feedback + NBR (semántica actual de removed_edges_count)
    pub removed_branches: Vec<String>, // NBR cuyo source fue borrado
    pub removed_macros: Vec<RemovedMacro>,
    pub changed: bool,
}

/// Redirige `original` → (`first`, `second`) en todas las estructuras del árbol (D-2). Devuelve si cambió.
pub fn redirect_split(tree: &mut Tree, original: &str, first: &str, second: &str) -> bool;

/// Elimina toda referencia a `ids` en el árbol (tronco, feedback, NBR, macros; D-3).
pub fn prune_removed(tree: &mut Tree, ids: &HashSet<&str>) -> PruneReport;

/// Violaciones de integridad referencial en todas las estructuras del árbol (D-5), en orden fijo:
/// nodes → edges → feedback_edges → nbr_branches → macro_edges.
pub fn check_tree_integrity(tree: &Tree, pool: &HashSet<String>) -> Vec<OutputError>;
```

Sin `clone()` innecesarios: `redirect_split` muta en sitio; `prune_removed` calcula el set de IDs de edges eliminados una sola vez (`HashSet<String>` de los edges filtrados) para podar `interior_links`.

## 3. Tareas (TDD: cada tarea escribe primero sus UATs en rojo)

| T | Contenido | Commit |
|---|---|---|
| **T0** | Este plan + **ADR-016** "Integridad global de mutaciones de nodo" (D-1..D-5). | `docs(v0.5.0): plan + ADR-016` |
| **T1** | `src/meta/integrity.rs` con las 3 funciones + unit tests (§4.1). | `feat(v0.5.0): funciones puras de integridad` |
| **T2** | `execute_node_split`: carga todos los árboles → valida contexto → mintea IDs → `redirect_split` en memoria → guarda solo los que cambian → refs → borra original. `NodeSplitData.affected_trees: Vec<String>` (ordenado, incluye `--tree`). CLI + MCP (mismo `execute_*`). | `feat(v0.5.0): node split global` |
| **T3** | `execute_node_rm` usa `prune_removed`; carga previa fail-closed (D-4); warning `MACRO_EDGE_REMOVED {tree_id, macro_link, reason, status, from, to, assumption_ids}` en el orden (árbol, macro). | `feat(v0.5.0): node rm poda macro_edges` |
| **T4** | Defensivo: `macro expand` y `path replace` re-validan que los extremos están en `tree.nodes` **antes** de mintear contadores (`NODE_NOT_IN_TREE`, código existente). | `fix(v0.5.0): extremos de macro en expand/replace` |
| **T5** | `validate` usa `check_tree_integrity` (sustituye a `check_integrity` sobre el tronco; respeta `--tree`). | `feat(v0.5.0): validate integridad completa` |
| **T6** | Mutation checks (§4.4) ejecutados y registrados. | — (resultado en PROGRESS) |
| **T7** | Docs + release: ENGINE_SPEC (§2.2 `node rm`/`split`, `validate`, `macro expand`, `path replace`, catálogo de warnings), USAGE_GUIDE, INTEGRATION (warning nuevo, validate más estricto), CHANGELOG `[0.5.0]`, README, `Cargo.toml` 0.5.0, PROGRESS (cerrar huecos, registrar los de §6), tag `v0.5.0`, push. | `chore(release): v0.5.0` |

## 4. UATs

Convenciones: `tests/v050_integrity.rs` con los helpers de `tests/rfc002_s1.rs` (`run_ltp`, `mcp_call`, `snapshot` byte a byte de `nodes/`+`trees/`+`knowledge/`, `warnings_with`, `error_codes`). En **todo** caso de fallo se comprueba: `success=false`, código exacto, `snapshot` idéntico al previo **y** `.ltp/counters.json` idéntico (no se queman IDs). En todo caso de éxito se termina con `validate` del workspace completo ⇒ 0 `REFERENTIAL_INTEGRITY_VIOLATION`.

### 4.1 Unit (puras, `src/meta/integrity.rs`)

- U1 `redirect_split` en un `Tree` vacío o sin el nodo ⇒ `false` y el árbol idéntico (comparar serialización).
- U2 sustitución en posición: `nodes = [A, X, B]` ⇒ `[A, X1, X2, B]`; `interior_nodes = [X]` ⇒ `[X1, X2]`. U2b–U2d: un hijo ya presente (antes o después del original) conserva su sitio y su `role`, solo se insertan los que faltan; `[X, X1🎫]` ⇒ `[X2, X1🎫]`; originales repetidos se eliminan todos.
- U3 `from = [X, Y]` (AND) ⇒ `[X2, Y]`, operador y assumptions intactos.
- U4 `prune_removed` sobre macro con extremo y con interior a la vez en el mismo batch ⇒ una sola `RemovedMacro` con `reason=endpoint_removed` (prioridad del extremo).
- U5 diamante A→B→E, A→C→E colapsado; borrar B ⇒ `interior_nodes=[C]`, `interior_links` = solo los de C; `changed=true`, 0 macros eliminadas.
- U6 `check_tree_integrity`: un fallo por estructura, orden fijo, `location` correcto; pool que contiene todo ⇒ vacío.

### 4.2 `node split` (S)

Happy path mínimo, el resto adversarial:

- **S1** nodo X en el tronco de T1 y T2 con edges en ambos; `split X --tree T1` ⇒ T2 reescrito (entrantes → `first`, salientes → `second`), `affected_trees=[T1,T2]` ordenado, `validate` limpio.
- **S2** X **solo** en una rama NBR de T2 (no en su tronco) y en el tronco de T1 ⇒ edges de la rama reescritos; X no aparece en ningún fichero.
- **S3** X `source_node` de una rama NBR en T2 ⇒ `source_node = first`; la rama sigue existiendo.
- **S4** feedback `X→Y` y `Y→X` en **T1** (árbol indicado) y en T2 ⇒ `from=second` / `to=first` en ambos (cubre el hueco del propio árbol).
- **S5** macros: Overlay con X como `from` (→ `second`), otra con X como `to` (→ `first`), otra con X interior (→ `[first, second]` en posición). Después, `macro-assume gather` sobre ellas no reporta `dangling` nuevo.
- **S6** Reservation en T2 con X como extremo ⇒ tras split, `macro promote` crea un edge entre nodos existentes y `validate` queda limpio; `macro expand` igual sobre otra reserva.
- **S7** `split X --tree T3` con X ausente de T3 pero presente en T1 ⇒ `NODE_NOT_IN_TREE`, 0 bytes, contadores intactos (regresión).
- **S8** `--tree` inexistente ⇒ `TREE_NOT_FOUND`, 0 bytes, contadores intactos.
- **S9** un árbol del workspace con JSON corrupto (escrito a mano) ⇒ `IO_ERROR` con `tree_id` del corrupto, 0 bytes en **todos** los demás ficheros, contadores intactos (D-4).
- **S10** número de labels ≠ 2 (0, 1, 3) y label vacío ⇒ error sin escritura.
- **S11** árbol T4 que no contiene X ⇒ bytes idénticos y ausente de `affected_trees` (mutación "guardar siempre").
- **S12** split encadenado: partir `second` de un split anterior en un workspace multi-árbol ⇒ sigue consistente; los IDs son secuenciales sin huecos.
- **S13** undo restaura byte a byte (incluidos T2 y los nodos con refs); redo produce bytes idénticos al post-split.
- **S14** split local en posición: el fichero tras el split es exactamente el previo con X sustituido en su sitio por `X1, X2` (en `nodes[]` e `interior_nodes`) y los IDs redirigidos, sin reordenar nada más; reconstruir el mismo escenario dos veces da bytes idénticos. (Reformulado: `nodes[]` conserva el orden de `attach` y nunca se ordena, así que dos órdenes de construcción distintos dan ficheros distintos ya antes del split.)
- **S15** paridad CLI↔MCP (`ltp_node_split`): mismo `data`, mismos ficheros.
- **S16** split no crea ciclos: X en un camino A→X→B con feedback B→A ⇒ `valid_dag=true` y ningún `CIRCULAR_DEPENDENCY_DETECTED`.

### 4.3 `node rm` y macros (M)

- **M1** borrar el `from` de un Overlay con 2 `MacroAssumption` ⇒ macro eliminada; `MACRO_EDGE_REMOVED` con `reason=endpoint_removed`, `status=overlay` y `assumption_ids` en orden de almacenamiento (creación); undo restaura byte a byte.
- **M2** borrar el `to` de una Reservation pura (sin edges reales) ⇒ macro eliminada con `status=reservation`, árbol en `affected_trees`; `validate` ya no emite `LONG_ARROW_RESERVATION_PENDING` sobre ella.
- **M3** diamante colapsado; borrar B ⇒ macro sobrevive recortada, sin warning de eliminación; si el resumen proyectaba ASMs del link de B ⇒ `LONG_ARROW_SUMMARY_STALE` con exactamente esos IDs en `dangling`; `path replace` sobre la superviviente funciona y `validate` queda limpio.
- **M4** Overlay lineal A→B→E; borrar B ⇒ `reason=interior_emptied`, macro eliminada.
- **M5** batch `rm A,E` (ambos extremos de la misma macro) ⇒ **un** solo warning para esa macro.
- **M6** batch con un ID inexistente (`rm A,NOPE`) ⇒ `NODE_NOT_FOUND`, 0 bytes (regresión: no se poda nada parcialmente).
- **M7** X extremo de una macro en T1 e interior de otra en T2 ⇒ ambas tratadas según D-3; una macro no relacionada en T1 queda byte a byte igual.
- **M8** tras M1, `macro expand`/`macro promote`/`path replace` sobre la macro eliminada ⇒ `MACRO_EDGE_NOT_FOUND`, 0 bytes.
- **M9** árbol corrupto en el workspace ⇒ `IO_ERROR`, 0 bytes, nada borrado del pool (D-4).
- **M10** macro legacy escrita a mano (`"status": "active"`, sin `assumptions`) con X como extremo ⇒ eliminada sin error; otra legacy no relacionada se re-serializa sin cambios de bytes no esperados.
- **M11** orden de warnings determinista: varias macros en varios árboles ⇒ ordenados por (`tree_id`, `macro_link`) y después de `NBR_BRANCH_REMOVED`, antes de `REFS_STRIPPED`.
- **M12** paridad CLI↔MCP (`ltp_node_rm`).
- **M13** links fantasma (D3/F3): Overlay lineal A→B→E; `link disconnect` de A→B (el ID queda en `interior_links`); `node rm B` ⇒ macro eliminada con `reason=interior_emptied`. Variante: un Overlay ya vacío (escrito a mano) y un `rm` de un nodo no relacionado ⇒ bytes de esa macro idénticos y sin warning.

### 4.4 Defensivo y validate (D, V)

- **D1** reserva cuyo extremo se ha desadjuntado (`tree detach`) ⇒ `macro expand` da `NODE_NOT_IN_TREE`, 0 bytes, contador `INT` intacto.
- **D2** igual para `path replace` sobre un Overlay con extremo desadjuntado ⇒ `NODE_NOT_IN_TREE`, 0 bytes, ningún nodo marcado `superseded`.
- **V1** workspace dañado a mano (simula v0.4.0): un ID inexistente en `nodes[]`, en un feedback, en el `source_node` y en un edge de una rama NBR, en `from`/`to`/`interior_nodes` de una macro ⇒ una `REFERENTIAL_INTEGRITY_VIOLATION` por cada uno, con `location` y el ID del contenedor correctos, en el orden fijo, `success=false`.
- **V2** `validate --tree T` solo reporta las violaciones de T.
- **V3** nodo presente en disco pero ilegible ⇒ solo `NODE_UNREADABLE`, ninguna violación de integridad duplicada.
- **V4** fixtures que reproducen exactamente la corrupción de v0.4.0 (escritas a mano, como el round-trip simulado de R17): el estado de S1 tras el split antiguo (T2 apuntando al original borrado) y el de M1 tras el rm antiguo (macro con extremo borrado) ⇒ el `validate` nuevo los detecta y `macro expand`/`path replace` sobre ellos fallan con `NODE_NOT_IN_TREE` sin escribir.
- **V5** golden `contract/validate.json` sin cambios (workspace sano ⇒ salida idéntica).

### 4.5 Mutation checks (cada uno debe tumbar al menos su UAT)

| Mutación | Debe fallar |
|---|---|
| Split solo sobre el árbol `--tree` (quitar el bucle global) | S1, S2 |
| Ignorar membresía solo-rama NBR | S2 |
| No redirigir `source_node` | S3 |
| No redirigir feedback | S4 |
| Intercambiar `first`/`second` en macro `from`/`to` | S5 |
| Añadir hijos al final en vez de en posición | U2, S14 |
| Guardar todos los árboles aunque no cambien | S11 |
| `continue` en árbol ilegible (comportamiento antiguo) | S9, M9 |
| Mintear IDs antes de cargar árboles | S9 (contadores) |
| Sin eliminación por extremo | M1, M2 |
| Sin recorte de interior | M3, U5 |
| Sin eliminar Overlays vacíos | M4 |
| Comprobar lista vacía en vez de links vivos | M13, `d3_touched_overlay_with_only_ghost_links_is_removed` |
| Sin deduplicar por macro | M5, U4 |
| Sin re-validar extremos en expand/replace | D1, D2 |
| `check_tree_integrity` sin macros (o sin feedback/NBR/`nodes[]`) | V1 |
| Tratar nodos ilegibles como ausentes | V3 |

## 5. Verificación (en cada commit con código)

```bash
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace
cargo fmt --all -- --check
```

Más: sin `.unwrap()`/`.expect()` en producción, `///` en todo lo público, sin `dbg!`/`println!`. Revisión Six Hats (Negro obligatorio) antes de T2 y T3.

## 6. Fuera de alcance, registrado (PROGRESS)

- Los hijos de un split pierden el `role` del original (afecta a EC: puede romper reglas de rol).
- El split no reescribe ni avisa de los knowledge links al nodo original (`KNOWLEDGE_ORPHANED` solo lo emite `rm`).
- `tree detach` de un extremo de macro deja la macro apuntando a un nodo desadjuntado (T4 lo bloquea en expand/replace, no lo previene).
- `--force` de `node rm` sigue sin uso.
- Los comandos `link` (`disconnect`, `rm-cause`, `reverse`, `move`…) no mantienen `macro_edges[].interior_links`: dejan IDs de links fantasma. `node rm` los tolera (D-3c usa links vivos), pero no los limpia.
- `validate` no detecta ni un Overlay sin interior ni links fantasma en `interior_links` (D-5 solo comprueba referencias a **nodos**). Arreglarlo requiere un código nuevo: contrato.
- Validación XOR de EC y `--dry-run` en mutaciones (heredados de v0.4.0).
