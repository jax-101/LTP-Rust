# Progreso del Proyecto — `ltp-engine`

## Dashboard

| Métrica | Valor |
|---------|-------|
| **Avance global (motor base)** | 100% ✅ |
| **Avance Knowledge Pool** | 100% ✅ |
| **Enriquecimientos (F13)** | 100% ✅ |
| **Fase actual** | Completado |
| **Última fase completada** | v0.6.0 — ilegible ≠ ausente (adenda D-K6 a ADR-016) |
| **Último release** | v0.6.0 (2026-10-10, commit de release local; tag y push pendientes del usuario) — MINOR: `IO_ERROR` en vez de `*_NOT_FOUND` ante lo ilegible, warnings `KNOWLEDGE_POOL_UNREADABLE`, `COUNTERS_REBUILT` y `DUPLICATE_ENTITY_ID` |
| **Último bugfix** | v0.6.0: lo ilegible se reportaba como inexistente en storage, comandos y resolución de knowledge links (falsos `DANGLING_KNOWLEDGE_REF`/`TARGET_NOT_FOUND`/`EPISTEMIC_UNGROUNDED`). Antes, v0.5.2: contadores por debajo del disco sobrescribían nodos tras `git pull`; reconstrucción que se saltaba lo ilegible; clon sin `.ltp/` inutilizable; `tree clone`/`link dissolve` copiaban IDs de `ASM`/`FB` |
| **Último añadido** | Tool nº 72 `ltp/tree_relation_list` (meta-grafo inferido, sin tipo) |
| **Factor de escala (velocity)** | 1.0x |
| **UATs motor base** | 199/199 |
| **UATs Knowledge Pool** | 234/239 |
| **Tests F13** | 11/11 |
| **Tests F14** | 6/6 |
| **Tests Slice 1 (macro-assume)** | 40/40 |
| **Tests Slice 2 (macro lifecycle)** | 36/36 |
| **Tests versión/provenance** | 4/4 |
| **Tests CSF + lógica de árbol** | 36/36 |
| **Tests v0.3.1 (contadores + roles EC)** | 15/15 |
| **Tests RFC-002 Slice 1** | 46/46 |
| **Tests v0.5.0 (integridad)** | 61/61 (38 E2E + 23 unit) |
| **Tests v0.5.1** | 50/50 (17 E2E dry-run + 10 E2E knowledge ilegible + 3 E2E no-expect + 13 unit `dry_run` + 7 unit determinismo/`to_json`), más 14 UATs KP |
| **Tests v0.5.2** | 20/20 (13 E2E `v052_counters` + 3 unit contadores + 4 unit memoización R13–R16) |
| **Tests v0.6.0** | 62/62 (40 E2E `v060_unreadable` + 10 E2E `v060_duplicates` + 7 unit D-3 + 5 unit D-5), más 2 goldens de `contract/` |
| **Tests totales** | 833 |

---

## Knowledge Pool — Estimación por Paquetes (ADR-012)

Plan: `.claude/plans/knowledge-pool-implementation.md` | Spec: `KNOWLEDGE_SPEC.md`

| Fase | Paquete | Peso Est. | Peso Ajust. | Estado | UATs | Notas |
|------|---------|-----------|-------------|--------|------|-------|
| K1 | Fundación (schema, storage, init, counters) | 8% | 8% | ✅ Completada | 16/16 | |
| K2 | CRUD (add/edit/rm/inspect/list) | 18% | 18% | ✅ Completada | 47/47 | 40 integration tests |
| K3 | Linking (link/unlink, validación refs) | 15% | 15% | ✅ Completada | 37/37 | 37 integration tests (6 añadidos en v0.5.1 T-K2) |
| K4 | Campo epistémico en nodos | 10% | 10% | ✅ Completada | 19/19 | 19 integration tests |
| K5 | Integración (status/validate/trace/node rm) | 20% | 20% | ✅ Completada | 46/51 | 46 integration tests (8 añadidos en v0.5.1 T-K2; K5.40 fijado) |
| K6 | Tests E2E (workflows hypothesis-driven) | 12% | 12% | ✅ Completada | 31/31 | 31 E2E tests |
| K7 | MCP Server (knowledge tools) | 17% | 17% | ✅ Completada | 38/38 | 38 integration tests |
| | **TOTAL** | **100%** | **100%** | | **234/239** | |

---

## Motor Base — Resumen Final (completado)

| Fase | Paquete | Peso | Estado | UATs |
|------|---------|:----:|--------|------|
| F1 | Fundación (workspace, traits, IDs, pipeline) | 10% | ✅ | 6/6 |
| F2a | Nodos standalone (add/edit/list/search) | 4% | ✅ | 9/9 |
| F3 | Vistas (trees) | 8% | ✅ | 19/19 |
| F4 | Enlaces básicos (connect/disconnect/feedback/feedback-list/feedback-rm) | 9% | ✅ | 17/17 |
| F2b | Nodos cross-tree (rm/split/inspect) | 5% | ✅ | 7/7 |
| F5 | Validación completa | 8% | ✅ | 14/14 |
| F6 | Enlaces avanzados | 14% | ✅ | 20/20 |
| F7 | Supuestos (assumptions) | 6% | ✅ | 15/15 |
| F8 | Navegación (trace) | 6% | ✅ | 15/15 |
| F9 | Abstracción (path) | 8% | ✅ | 12/12 |
| F10 | NBR | 5% | ✅ | 17/17 |
| F11 | Historial (undo/redo) | 6% | ✅ | 22/22 |
| E2E | Tests end-to-end | 4% | ✅ | 19/19 |
| F12 | MCP Server | 7% | ✅ | 18/18 |
| F13 | Validation Enrichments | — | ✅ | 11/11 |
| F14 | Feedback Edge Primitives | — | ✅ | 6/6 |
| | **TOTAL** | **100%** | **✅** | **211/211** |

---

## Historial de Avance

### [Release v0.6.0] — Ilegible ≠ ausente (MINOR) — Sesión 2 (T5–T10) completada
**Fecha**: 2026-10-10 (sesión nocturna desatendida; dudas resueltas con Six Hats + recomendación, por decisión del usuario).
**Commits**: `3f797f3` T5 (D-5) → `80882fe` T6 (D-8) → `91ed842` T7 (test que mata la mutación 7) → `6d4c23c` T9 (docs) → T10 `chore(release): v0.6.0`. **Sin tag y sin push**: los hace el usuario (`git tag -a v0.6.0` + `git push --follow-tags`).
**T5 — D-5**: `next_id -> Result<MintedId {id, notice}>`; `LockSession {scopes, notice_emitted}`; helper `prepend_session_warnings`, que sustituye a las 13 copias de `stale_lock_warning` y a las 3 en línea de `path`. La tabla c9 cubre los 20 comandos que mintean, porque el compilador no puede obligar a reenviar el aviso (`&mut notice` ya cuenta como uso). Destapó que `nbr add` calculaba los avisos antes de mintear.
**T6 — D-8**: `validate/duplicates.rs`, un recorrido tipado. `validate` carga los árboles una vez y los comparte entre `_meta_graph`, el filtro de knowledge y D-8.
**Decisiones (Six Hats, sesión 2)**:
- (a) Los avisos de sesión van **siempre al principio** (`STALE_LOCK_REMOVED` y después `COUNTERS_REBUILT`). Cambia el orden en `invalidate`, que ponía el de lock detrás de `ALREADY_INVALIDATED`/`STATE_REPAIRED`. Es un orden documentado en ENGINE_SPEC §4.1, no un campo nuevo.
- (b) `validate --tree T` con **otro** árbol ilegible: warning `TREE_LOAD_ERROR {tree_id}` en `_workspace`, sin cambiar `success`. Se descarta la propuesta de la sesión 1 (error): fallar `validate` de T por un árbol que no se pidió validar castigaría el filtro, pero el chequeo cruzado incompleto no se calla (test d9).
- (c) `stale` con varios prefijos subidos en un mismo escaneo: se nombra el que se mintea si se subió y, si no, el primero.
- (d) Árboles cargados una sola vez en `validate`: es una mejora de rendimiento (T8) y garantiza que las tres pasadas vean el mismo estado.
**T7 — Mutaciones (13/13 detectadas)**, cada una con la suite completa y el contenido del fichero restaurado desde una copia guardada (nunca `git checkout`):

| # | Mutación | Tests que la detectan |
|---|---|---|
| 1 | Quitar el aviso de pool | 6 (`b1_*`) |
| 2 | No saltar el análisis epistémico con `Unlisted` | `b1_validate_skips_epistemic_analysis` |
| 3 | Volver a `exists()` en `load_tree` | 4 (`l1_*`, `d3_broken_symlink…`) |
| 4 | `load_error_code` siempre `not_found` | 8 o más (`g1`, `g7`, `l1_*`) |
| 5 | Descartar `notice` (`into_id`) | 8 o más (`c1`–`c5`, `c7`, `c8`…) |
| 6 | No marcar `notice_emitted` | `d5_one_notice_per_session_across_scopes` |
| 7 | No borrar `notice_emitted` en `acquire_lock` | **Sobrevivía** → test nuevo `d5_acquire_forgets_a_notice_left_by_a_missing_release` (estilo R14). Nota: `acquire_lock` resetea la sesión dos veces (al entrar y tras escribir el lock); la mutación tiene que anular las dos, y si solo anula una no es una mutación real |
| 8 | `resolve` vuelve a tragarse los `Err` | 4 (`g1`, `l3_*`) |
| 9 | D-8 ignora `nbr_branches` | `d1`, `d3`, `d5`, `d8` |
| 10 | D-8 cuenta un árbol ilegible como vacío | `d9` |
| 11 | Symlink roto como ausente (sin `symlink_metadata`) | 3 (`l1_broken_symlink…`, 2 unit D-3) |
| 12a | Helper: contadores antes que lock | `g3` |
| 12b | Helper: pierde el aviso de lock | `g3`, `dr7` |
| 13 | `undo` deja `counters.json` por debajo del disco (lo borra) | `g5`, `k2_46`, `dr11`, `o1` |

**T7b — Código muerto**: no se elimina nada (va en este commit de docs). `rg stale_lock_warning` = 0. `target_exists` sigue en uso (`validate/knowledge.rs`). Sin `allow(dead_code|unused)` nuevos. No quedan ramas `Err(_) => *_NOT_FOUND`. Los tests que fijaban el comportamiento falso se corrigieron en T0–T4 (`e2e_10_counters_recovery` ahora exige `COUNTERS_REBUILT`).
**T8 — Rendimiento** (binarios release `0.5.2+0f0f4767` frente a `91ed842`, workspace sintético de 4,27 MB: 500 nodos, 50 árboles, 5.000 aristas, 5.000 `ASM`; mediana de 15 ejecuciones alternas):

| Comando | v0.5.2 | v0.6.0 | Δ |
|---|---|---|---|
| `validate` | 66,2 ms | 61,0 ms | **−7,9 %** (árboles cargados una vez) |
| `validate --tree tree-crt-t00` | 32,8 ms | 33,6 ms | +2,4 % |
| `status` | 32,2 ms | 36,0 ms | +11,7 % |
| `validate` con 100 árboles y 10.000 IDs duplicados (peor caso de D-8; mediana de 9) | 90,6 ms | 101,7 ms | +12,2 % |

Todo por debajo del umbral del 20 %.
**T9 — Docs**: adenda D-K6 a ADR-016 (incluida la desviación `ResolveError`), KNOWLEDGE_SPEC §6.0, ENGINE_SPEC (`_workspace` en §2.12, aviso de contadores en §3.1, lock, orden de warnings en `rm`/`status`/`validate`/`walk`/`trace`, y §4.1 nueva con `*_NOT_FOUND` frente a `IO_ERROR`, el orden de los avisos de sesión y "una salida de error no lleva warnings" con su excepción previa, `MAG_WEIGHT_MISSING` en el ciclo de `link connect`), INTEGRATION (gate `>= 0.6.0`, nota de migración y nota de la UI), RELEASE_POLICY (feature-gating), CHANGELOG `[0.6.0]` y goldens `warning_counters_rebuilt` y `validate_duplicates`. Los goldens anteriores no cambian.
**Fuera de alcance (registrado)**: avisos en las salidas de error (refactor uniforme); pasar `DUPLICATE_ENTITY_ID` a error cuando exista un comando de reparación; avisos de knowledge links colgantes en las mutaciones; `dry_run` en MCP (v0.7.0); que la UI muestre los warnings de las mutaciones y lea `validate` de `data.details[]` (repo `LTP-Rust-UI`).
**Factor de escala**: 1.0x.

### [v0.6.0] — Ilegible ≠ ausente (MINOR) — Sesión 1 (T0–T4)
**Fecha**: 2026-10-10
**Plan**: `PLAN_v060.md` rev 3.3. Sesión 1 = T0 → T4 (hecha). **Sesión 2 = T5 → T9**, parar antes de T10. Para retomar: "Continúa PLAN_v060.md". Sin push ni tag.
**Commits**: `2ca1731` T0 (tests en rojo) → `61d1e86` T1 (D-1/D-2) → `74e7a24` T2 (D-3) → `0f2d248` T3 (D-4) → `f93bc72` T4 (D-7).
**Estado de los tests**: 808 en verde, 18 en rojo, todos pendientes de la sesión 2: 11 de `v060_unreadable` (D-5: c1–c5, c7, c8, g3, g4, g5) y 7 de `v060_duplicates` (D-8). Las suites de contrato de los `*_NOT_FOUND` legítimos, intactas.
**T0**: `tests/v060_unreadable.rs` (39 casos: capas 1–3, B1, adyacentes, D-5, G1, G3–G5, G7) y `tests/v060_duplicates.rs` (9 casos D-8 con G2 y G6). Rojo según lo previsto; los "correctos", D-5b y (d) ya en verde.
**T1**: `PoolScope { Complete, Unlisted }` y `KNOWLEDGE_POOL_UNREADABLE`. El aviso va en `pool.warnings`, así que status, node rm, trace y walk lo heredan sin tocarlos. validate se salta el análisis epistémico.
**T2**: fs_storage sin `exists()`. Los helpers `read_entity`, `remove_entity`, `list_json_ids` e `is_absent` aplican la regla del symlink colgante con `symlink_metadata`, solo en el camino `NotFound`. Además:
- `workspace_exists -> Result<bool>`; en CLI y MCP, un `Err` da `IO_ERROR`. En MCP sale como tool result, no como error JSON-RPC.
- El lock sigue la misma regla.
- `.gitignore` se escribe con `create_new` (sin TOCTOU).
- `ensure_knowledge_dir` usa `create_dir` y trata `AlreadyExists` como ya existente.
- validate lista los nodos una sola vez (un `Err` da `IO_ERROR`) y los pasa a `validate_knowledge` y a `validate_meta_graph`. Así se cierran `:46`, `:69` y `:327`.
- 7 tests unitarios D-3 en `fs_storage`.

**T3 — Inventario**: 70 sitios con `TREE/NODE/KNOWLEDGE_NOT_FOUND` o `REFERENTIAL_INTEGRITY_VIOLATION`.
- 60 traducían cualquier `Err`: se corrigen.
- 3 ya eran correctos: node edit `:646`, tree rm `:390` y tree rename `:913`.
- 7 son legítimos: la comprobación de pertenencia a lista en node split `:1585` y 6 comprobaciones de pool en `meta/integrity`.
- Helper `OutputError::load_failed(e, not_found, detail)` sobre `load_error_code` (D-4). Con `IO_ERROR`, el `detail` lleva la causa, no el "not found" del sitio.

**T4**: `resolve_target -> Result<Option<ResolvedTarget>, ResolveError>`.
- `ResolveError` tiene tres variantes: `TreesUnlisted`, `TreeUnreadable{tree_id}` y `NodeUnreadable{node_id}`.
- **Desviación de la firma literal del plan**: el plan pone `Result<_>` con `LtpError`, pero se usa un error propio para conservar el `tree_id` que exigen inspect y link. Es API Rust, no contrato JSON.
- Un árbol que desaparece entre el listado y la carga cuenta como ausente.
- inspect deduplica el aviso por causa.
- Un nodo ilegible como destino da `NODE_UNREADABLE {node_id}` en inspect e `IO_ERROR {node_id}` en link.

**Descubrimientos**:
- **G1 (decisión del usuario)**: se ajusta a D-4/D-7. Corrupto e ilegible dan los mismos códigos: `IO_ERROR` en walk, `TREE_LOAD_ERROR` en validate y un aviso `TREE_LOAD_ERROR` en inspect. El `detail` los distingue. No hay códigos nuevos.
- **`path replace` solo mintea `LINK`** (un único ámbito). El caso entre ámbitos de D-5 se prueba con `path explode` y `macro expand` (INT + LINK); `path replace` queda como caso de un solo ámbito.
- **G2 (interpretación)**: un duplicado entre un árbol legible y uno ilegible da `TREE_LOAD_ERROR` para el ilegible, sin `DUPLICATE_ENTITY_ID` y con `success:false`.
- **Formato de `location` para D-8**: `edges[i]`, `edges[i].assumptions[j]`, `feedback_edges[i]`, `nbr_branches[i]`, `nbr_branches[i].edges[j](.assumptions[k])`, `macro_edges[i](.assumptions[j])`. Las `occurrences` van ordenadas por `tree_id` y luego por orden de recorrido.
- **`tree clone` (v0.5.2) no copia `nbr_branches` ni `macro_edges`**: el clon solo lleva aristas, supuestos y feedback, con IDs nuevos. Por eso d0 (workspace sano con clon) no tiene duplicados.
- `assume add` no admite aristas de NBR (da `LINK_NOT_FOUND`), así que los tests de D-8 con supuestos en NBR editan el JSON a mano.
- La salida de error del ciclo en `link connect` lleva `warnings` (`MAG_WEIGHT_MISSING`), así que la regla "una salida de error no lleva warnings" tiene excepciones previas. D-5b solo exige que no salga el aviso de contadores.

**Pendiente en T5/T6**: decidir qué hace `--tree T` en D-8 cuando hay otro árbol ilegible. Propuesta: ese árbol sale como `TREE_LOAD_ERROR`, igual que sin filtro.

### [Release v0.5.2] — Un ID nuevo nunca pisa uno existente (PATCH)
**Fecha**: 2026-10-09
**Plan**: `PLAN_v052.md` (rev 3 + D-7). T0–T6 completadas (T6 confirmada por el usuario el 2026-10-09).
**Commits**: `2b78a5c` T0 (R1–R12 en rojo, como preveía el plan; R7 ya en verde) → `050913c` T1 (reconciliación por ámbito, fail-closed) → `7663a7c` T1b (D-6: `clone`/`dissolve` mintean IDs nuevos) → `d51fa8b` T1c (D-7: una reconciliación por ámbito y por comando).
**T1**: `counters.rs` reescrito (`ScanScope`, `scope_of`, `StoredCounters`, `observe_scope`, `reconcile`, `observe_text`). Se eliminan `rebuild`, `scan_directory`, `scan_tree_contents` y `FsStorage::load_counters`. Nueva variante interna `LtpError::CounterScan { path, source }`. `acquire_lock` crea `.ltp/`. `tests/counters_rebuild.rs` ajustado: `node add` ya no lee los árboles, así que U1/U2/U7/U8 disparan también un minteo de árbol, y U3 ahora sí ve los IDs del árbol en conflicto.
**T2 — Mutaciones (13/13 detectadas)**, aplicadas y revertidas por script sobre el árbol de trabajo (sin `git checkout`):

| M | Mutación | Muere |
|---|---|---|
| M1 | No reconciliar | R1, R1b, R8 (y R2, R5, R7, R13–R16) |
| M2 | Saltarse lo ilegible | R3, R4 (y R6, R6b, R16) |
| M3 | Quitar `observe_text` | R2 |
| M4 | Tolerar lo ilegible con `counters.json` válido (regla T) | R6, R6b, R16 |
| M5 | `NotFound` como ilegible | R7, R1 |
| M6 | Quitar `create_dir_all` | R5, R1 |
| M7 | `scope_of` siempre `All` | R6, R9, R13–R16 |
| M8 | `tree clone` copia los `ASM` | R10, R11 |
| M9 | `dissolve` reparte los mismos `ASM` | R12 |
| M10 | Reconciliar siempre (sin memoria) | R13 |
| M11 | Borrar la memoria solo en `release_lock` | R14 |
| M12 | Marcar el ámbito antes de escanear | R16 |
| M13 | Subir solo el prefijo pedido y marcar todo el ámbito | R13 (`ASM` tras `LINK`) |

**T3 — Rendimiento** (binario release, macOS, workspace sintético de 3,9 MB: 500 nodos, 50 árboles, 5.000 aristas, 5.000 `ASM`; mediana de 15 ejecuciones, 5 en los casos grandes):

| Comando | v0.5.2 | Referencia |
|---|---|---|
| `node add` | 50,6 ms | 53 ms en v0.5.1 (con `counters.json`), 90 ms reconstruyendo |
| `knowledge add` | 48,7 ms | — |
| `link connect` (1 destino) | 55,2 ms | — |
| `assume add` | 60,0 ms | — |
| `link connect` con 50 destinos (5,8 MB, con el árbol de 5.000 aristas) | 87,0 ms | — |
| `tree clone` de 5.000 aristas + 5.000 `ASM` (5,8 MB) | **749 ms** | **149 s** sin D-7 (binario de `7663a7c`) |

Todo por debajo de los umbrales del plan (< 100 ms por minteo; `tree clone` < 1 s). D-7 divide el coste del clone por unas 200. El resto del coste del clone son los ~10.000 `save` de `counters.json`, uno por minteo. Queda dentro del umbral, así que no se toca.
**T4**: `check`, `clippy -D warnings`, `fmt --check` limpios; **771/771 tests** en verde.
**T5**: adendas a ADR-009 (D-1 a D-3, D-7) y ADR-005 (D-6), ENGINE_SPEC §3.1 (contadores, clon sin `.ltp/`), INTEGRATION (gate `>= 0.5.2` con varios clones, límite de clones concurrentes), CHANGELOG `[0.5.2]`.
**Fuera de alcance (registrado en el plan)**: aviso `COUNTERS_REBUILT`/contador desactualizado y detección de duplicados ya existentes (MINOR, `PLAN_v060.md`); knowledge links colgantes tras `tree rm`/`assume rm`; dos clones que mintean a la vez (conflicto add/add, documentado); `create_new` en `save_node`.
**Factor de escala**: 1.0x (4 paquetes de código + docs, según el plan).

### [main, sin tag] — `link connect --nbr` busca la rama una sola vez (refactor)
**Fecha**: 2026-10-09
**Plan**: `PLAN_post-v051.md`, Parte A (A1–A5). Sin cambio de contrato: viaja en la siguiente release (**v0.5.2**, decisión del 2026-10-09), sin tag propio.
**A1 — Refactor** (`src/link/commands.rs`, un solo fichero): la rama NBR se resuelve una vez, antes de reservar IDs, en un `enum ConnectTarget { Trunk, Nbr { id, branch: &mut NbrBranch } }` que se guarda hasta la inserción. Desaparecen la validación temprana con `any` y el `let-else` posterior. `edge_logic` se calcula antes de la búsqueda (es puro: `nbr_id.is_some()` y `tree.logic`). `nid` sale de `nbr_id`, no de `branch.id`. Orden de errores intacto (`LOCK_ERROR` → `TREE_NOT_FOUND` → errores de nodo → `NBR_NOT_FOUND` → `INVALID_OPERATOR`). Ningún `clone()` nuevo.
**A2 — Mutación**: mover la resolución de la rama a después de `next_id("LINK")` → los 3 E2E de `tests/v051_no_expect.rs` mueren (`.ltp/counters.json` pasa a `"LINK": 1`). Detectada y revertida.
**A3 — Verificación**: `check`, `clippy -D warnings`, `fmt --check` limpios; **751/751 tests** en verde.
**Huecos**: cierra el hueco 2 de v0.5.1 (el `let-else` que quemaría un `LINK`). La sonda T2b queda sin efecto: ya no hay dos guardias, solo una, y es la que vigila `v051_no_expect`.
**Registrado (no es bug)**: `knowledge add` quema el contador `KN` si falla con `IO_ERROR` después de reservar el ID. Es el patrón de todo el motor (`node add` igual) y ADR-009 lo acepta: los IDs no retroceden, y comprobar antes de escribir abriría una ventana TOCTOU.
**Sigue fuera de alcance**: un ciclo dentro de una NBR sigue quemando `LINK` con `CIRCULAR_DEPENDENCY_DETECTED` (ADR-013/ADR-009, documentado).
**Factor de escala**: 1.0x (1 paquete, prototipo previo validado).

### [Release v0.5.1] — `--dry-run` real + knowledge ilegible visible (PATCH)
**Fecha**: 2026-10-08
**Plan**: `PLAN_v051.md` (ADR-017, adenda D-K5 a ADR-016). T0–T5 completadas, más T1b (aprobada durante T1). Cierra el hueco heredado "`--dry-run` en mutaciones" de v0.4.0/v0.5.0.
**T1b (añadida con aprobación del usuario, 2026-10-08)**: la comparación byte a byte de T1 destapó un no-determinismo anterior a v0.5.1, que violaba el invariante 1:
- `check_dag` (`src/validate/dag.rs`) arrancaba el DFS recorriendo un `HashSet`, así que `cycle_path` y el `detail` de `CIRCULAR_DEPENDENCY_DETECTED` salían como una rotación arbitraria del ciclo en todo comando que informa de un ciclo.
- Cuatro lints de `src/validate/clr.rs` (CLR4, CLR4/5, CLR5, CLR7) recorrían un `HashMap` para emitir sus avisos, así que el orden de los warnings de `validate` cambiaba entre ejecuciones (se midió 10/20).

**Fix**: `BTreeMap`/`BTreeSet`. El DFS arranca por el ID menor y los avisos salen ordenados por nodo. El resto de `HashMap` del motor se revisó: solo hacen búsquedas, y `tree walk` ya ordenaba su cola.
**Tests**: 5 unit (50 iteraciones en el mismo proceso) + el E2E `t1b_*` (20 procesos). DR3 y DR11b vuelven a comparar byte a byte. Los tres E2E fallaban con el código anterior.
**T2 (D-8a)**: no queda ningún `.expect()`/`.unwrap()` en código de producción (comprobado con un escaneo de `src/` fuera de `#[cfg(test)]`).
- `CommandOutput::to_json`: si `data` no se puede serializar, devuelve el contrato de error (`INTERNAL_ERROR`, claves canónicas, indent 2) en vez de un panic. El unit `to_json_falls_back_*` fallaba con el panic antes del fix.
- `link connect --nbr`: el `expect` pasa a `let … else` → `NBR_NOT_FOUND` (helper `nbr_not_found`, compartido con la validación previa). Ese `expect` no era alcanzable, así que la regresión `tests/v051_no_expect.rs` (3 E2E: NBR inexistente, NBR de otro árbol, workspace usable después) ya pasaba antes del fix; queda como guardia del contrato. 749 tests en verde.
**T3 (D-8b)**: `#![warn(missing_docs)]` en `lib.rs` (con doc `//!` del crate) y `#[allow(missing_docs)]` en los 15 módulos con deuda. La deuda medida al activarlo sin `allow` es de **654 elementos** (655 del plan − `to_json`, documentado en T2). `meta`, `macro_edge` y `macro_assume` quedan exigidos. Sonda: un `pub fn` sin doc en `meta` o `macro_edge` rompe `clippy -D warnings`; en `node` (con `allow`) no. Pagar la deuda de un módulo = quitar su `allow`.
**T4 — Mutation checks (§4.2 + T1b + T2)**: 20 mutaciones aplicadas de verdad sobre `src/` (y `tests/` en K4b). Cada una se ejecutó con `cargo test --no-fail-fast` sobre las suites indicadas y se revirtió con `git checkout` (árbol limpio al final). El runner está en `target/t4_mutate.py`. **18/18 mutaciones reales detectadas.** Las 2 sondas (K4b y T2b) daban información sobre los tests, no sobre el código.

| # | Mutación | Previsto | Detectada por |
|---|----------|----------|---------------|
| K1a | Sin `KNOWLEDGE_LOAD_ERROR` en `status` (CLI) | KL1 | KL1, KL2, KL6 |
| K1b | Sin `KNOWLEDGE_LOAD_ERROR` en `status` (MCP) | KL2 | KL2, KL6 |
| K2 | `node rm` bloquea ante un KN ilegible | KL5 | KL5, KL5b |
| K3 | Aviso también sin `--show-knowledge` (`tree walk`) | KL4, KL7 | KL4. KL7 no puede verlo: su fixture no tiene items ilegibles, así que el aviso extra sale vacío |
| K4a | Sin la entrada `_knowledge_pool` en `validate` | k5_32 | k5_32 y otros 21 tests de K5 |
| K4b | Igual que K4a, con `k5_32` en su forma `if let` previa a T-K2 | (sonda) | **Sobrevive**, como se esperaba: prueba que el `if let` era vacuo y justifica T-K2 |
| D1 | `intercept` devuelve siempre `None` | DR1, DR2, DR4 | DR1, DR1b, DR2, DR3, DR4, DR5, DR7, DR8, DR10, D6, D7 |
| D2 | Excluir `.ltp/` de la copia | DR6, DR7, DR8 | DR6, DR7, DR8, DR1–DR5, DR10, unit copia |
| D3 | `rename` en vez de copiar | DR1 | DR1 (huella), DR1b, DR2, DR3, DR5–DR8, DR10, DR11b, 2 unit |
| D4 | `Drop` sin borrar la copia | DR9 | DR9, DR10, unit `copy_is_deleted_on_drop`, unit D-6 |
| D5 | Copiar el `cwd` entero | DR10 | Solo el unit `copy_contains_only_managed_files`. El E2E DR10 no puede ver la copia, como ya preveía el plan |
| D6 | Adquirir el lock real antes de copiar | DR6, DR1 | DR1 (huella) y 10 E2E más. DR6 no: con un lock vivo la adquisición falla sin tocar el fichero |
| D7a | Sin excepciones nativas (`init`/`undo`/`redo` también se simulan) | DR11 | DR11 |
| D7b | `child_args` quita todos los `--dry-run` | unit | unit `child_args_removes_only_first_and_stops_at_separator` |
| D8 | Degradar a ejecución real si falla la simulación | D-6 | E2E D6, D7 |
| D9 | Mezclar stderr del hijo en stdout | DR1 | **Sobrevivió en la primera pasada**: el hijo nunca escribe en stderr (el único `eprintln!` es el fallo de `current_dir`, imposible en la copia), así que ningún E2E lo ve. Se cerró en `9f48032`: `forward_to` sobre writers explícitos más 2 unit. Re-ejecutada: detectada por `forward_keeps_streams_apart_and_returns_code` |
| B1 | `check_dag` vuelve a `HashSet` | unit, DR3, t1b | unit `cycle_path_is_deterministic`, E2E `t1b_*` |
| B2 | Lint CLR4 vuelve a `HashMap` | unit, DR11b, t1b | unit `clr4_warnings_ordered_by_node_id`, E2E `t1b_*` |
| T2a | `to_json` vuelve al panic | unit | unit `to_json_falls_back_*` |
| T2b | Quitar la validación temprana de `--nbr` (solo queda el `let-else`) | (sonda: ¿equivalente?) | **No es equivalente**: los 3 E2E de `v051_no_expect` mueren, porque al llegar al `let-else` ya se ha consumido un contador `LINK` y `.ltp/counters.json` cambia. La validación temprana es la guardia real y la regresión la vigila. El `let-else` es una red de seguridad para un camino inalcanzable hoy. *Sin efecto desde el refactor de la Parte A de `PLAN_post-v051.md`: solo queda una búsqueda* |

Ni DR1 ni DR3 ni DR11b detectaron B1/B2 en una sola pasada: su comparación byte a byte es entre dos procesos, y una sola pasada no basta para ver el desorden. Lo detectan el unit de 50 iteraciones y el E2E de 20 procesos, que se añadieron precisamente por eso.

**Fuera de alcance, registrado (§5 del plan)**:
- `dry_run` en MCP (los 72 tools; hoy solo `ltp/undo` y `ltp/redo`): es contrato nuevo, va como MINOR (v0.6.0) y reutilizaría el mecanismo de ADR-017 D-1.
- Documentar los 654 elementos públicos sin `///`: la deuda queda congelada por T3, no saldada.
- Dividir `main.rs` (2280 líneas).
- Carrera de ADR-017 D-3: un escritor concurrente durante la copia puede dar una simulación de un estado intermedio (la misma ventana que una lectura de hoy). Documentada en ENGINE_SPEC §2.0 e INTEGRATION §2A.
- Los heredados de v0.5.0 (§6 de `PLAN_v050-integrity.md`), listados arriba.
- **Hueco nuevo**: `load_pool` sigue callando si falla `list_knowledge_ids` (por ejemplo, `knowledge/` ilegible como directorio): devuelve un pool vacío sin aviso. D-K5 cubre los items ilegibles, no el listado.
- El `let-else` de `link connect --nbr` (T2) consumiría un contador `LINK` si llegara a alcanzarse (sonda T2b). Hoy es inalcanzable gracias a la validación temprana. **Cerrado en `main` (2026-10-09, `PLAN_post-v051.md` Parte A)**: la rama se busca una sola vez, antes de reservar IDs.

### [Release v0.5.0] — Integridad referencial global (MINOR)
**Fecha**: 2026-10-08
**Naturaleza**: Cierra los huecos registrados en v0.4.0 (plan `PLAN_v050-integrity.md`, ADR-016). T0–T7 completadas (8 paquetes). Funciones puras `redirect_split`, `prune_removed` y `check_tree_integrity` en `src/meta/integrity.rs` (semilla de `ltp-core`); `node split` global con `affected_trees`; `node rm` poda macros con `MACRO_EDGE_REMOVED`; ambos son fail-closed; los extremos de macro se comprueban en expand/promote/replace; `validate` revisa la integridad de todas las estructuras.
**Mutation checks (T6, §4.5)**: 24 mutaciones aplicadas de verdad sobre `src/`. Cada una se ejecutó contra `cargo test --lib --test v050_integrity` y después se revirtió (`git status src/` limpio al final). **24/24 detectadas, 0 supervivientes**.

| # | Mutación | Detectada por (UATs E2E / unit) |
|---|----------|---------------------------------|
| 1 | Split solo sobre el árbol `--tree` | S1, S2, S3, S4, S5, S6, S11, S12 |
| 2 | Ignorar la membresía solo-rama NBR | S2 |
| 3 | No redirigir `source_node` | S3 + unit `redirect_covers_*` |
| 4 | No redirigir feedback | S4 + unit `redirect_covers_*` |
| 5 | Intercambiar first/second en macro from/to | S5, S6 + unit `redirect_covers_*` |
| 6 | Hijos al final en vez de en posición | S14 + U2, U2b, U2c |
| 7 | Guardar todos los árboles aunque no cambien | S11 |
| 8 | `continue` en árbol ilegible (split y rm) | S9, S9b, M9 |
| 9 | Mintear IDs antes de cargar árboles | S7, S9, S9b, S12 |
| 10 | Sin eliminación por extremo | M1, M2, M5, M7, M8, M10, M11 + U4 |
| 11 | Sin recorte de interior | M3, M4, M7, M13 + U5, D3 ×3 |
| 12 | Sin eliminar Overlays vacíos | M4, M13 + D3 ×2 |
| 13 | Limpiar `projection_refs` en la poda | M3 |
| 14 | Comprobar lista vacía en vez de links vivos | M13 + D3 (solo links fantasma) |
| 15 | Sin deduplicar por macro | M1, M2, M5, M7, M8, M10 + U4 |
| 16 | Ordenar warnings de macro textualmente | M11 |
| 17 | Sin deduplicar IDs de entrada en `rm` | M14 |
| 18 | Sin re-validar extremos en expand/replace/promote | D1, D2, V4a, V4b |
| 19 | Extremos sin comprobar el pool (T5) | V4a + unit `macro_endpoints_*` |
| 20 | `check_tree_integrity` sin macros | V1, V2, V4a, V4b + U6 |
| 21 | … sin feedback | V1, V2 + U6 |
| 22 | … sin NBR | V1, V2 + U6 |
| 23 | … sin `nodes[]` | V1, V2, V4a + U6 |
| 24 | Tratar nodos ilegibles como ausentes en validate | V3 |

**Lectura**: las mutaciones 2, 7, 13, 16, 17 y 24 solo las detecta un UAT cada una (S2, S11, M3, M11, M14, V3). Esos tests no se pueden quitar sin perder cobertura.
**UATs**: S1–S16 (split), M1–M14 (rm y macros), D1–D2 (comprobación de extremos), V1–V5 (validate; V4a/V4b reproducen la corrupción exacta de v0.4.0) + 23 unit. Decisión tomada en T5: un extremo adjunto pero ausente del pool ⇒ `NODE_NOT_FOUND`.
**Tests totales**: 628 → 687
**Factor de escala**: 1.0x (8 paquetes; las correcciones fueron fixtures de tests y un hueco detectado por V4a que se resolvió dentro del paquete)
**Docs**: ADR-016, ENGINE_SPEC (`node rm`/`split`, `path replace`, `macro expand`/`promote`, `validate`), USAGE_GUIDE §5.1/§9/§11, INTEGRATION §4, RELEASE_POLICY (gate `>= 0.5.0`), CHANGELOG `[0.5.0]`, README, descripción MCP y ayuda CLI de `node split`/`node rm`, tag `v0.5.0`.
**Huecos cerrados**: los dos de v0.4.0 (`split` solo reescribía `--tree`; `rm` ignoraba `macro_edges`).
**Huecos registrados (fuera de alcance, plan §6)**:
- Los hijos de un split pierden el `role` del original (en EC puede romper reglas de rol).
- El split no reescribe ni avisa de los knowledge links al original.
- `tree detach` de un extremo de macro deja la macro apuntando a un nodo desadjuntado (expand/replace lo bloquean, pero no se previene).
- `--force` de `node rm` sigue sin uso.
- Los comandos `link` no mantienen `interior_links` (dejan links fantasma), y `validate` no detecta Overlays sin interior ni links fantasma. Arreglarlo requiere un código nuevo.
- `link` emite `REFERENTIAL_INTEGRITY_VIOLATION` sin contexto.
- Ninguna mutación emite `LONG_ARROW_SUMMARY_STALE`.
- Escrituras parciales sin rollback ante un fallo de E/S a mitad.
- Labels vacíos sin validar al crear nodos.
- Un nodo que solo vive en ramas NBR no se puede partir.
- Los nodos ilegibles se saltan al reescribir refs.
- Validación XOR de EC y `--dry-run` (heredados). `--dry-run`: **cerrado en v0.5.1**.

### [Release v0.4.0] — RFC-002 Slice 1: refs + meta-grafo inferido (MINOR)
**Fecha**: 2026-10-06
**Naturaleza**: Primer slice de RFC-002 sobre JSON (arquitectura de dos velocidades). Nueva línea de trabajo fuera del 100% del motor base; **no** altera ese %. Plan `PLAN_v031-and-rfc002-slice1.md`, decisiones D-1..D-10 en ADR-015 (D-4: relaciones sin tipo).
**Avance**: T-S1.0→T-S1.5 completadas (6 paquetes). `CrossRef` + `NodeMetadata::new`; `src/meta/` puro (semilla de `ltp-core`): `tree_memberships`, `infer_relations`, `check_refs`; refs en `node add/edit/inspect` (CLI + MCP estricto); tool nº 72 `tree_relation_list`; `_meta_graph` en `validate`; integridad NBR/refs en `node rm`/`node split`.
**UATs**: R1–R22 en `tests/rfc002_s1.rs` (31 E2E adversariales: refs a destinos inválidos sin escritura, auto-ref, duplicados, MCP malformado, fan-out multi-attach y pin, endpoints NBR, `node rm` + undo byte a byte, split, refs colgantes por motivo, nodo ilegible, reglas de norma, filtro `--tree`, round-trip con binario v0.3.0, determinismo entre órdenes de construcción, paridad CLI↔MCP) + 15 unit. Mutation checks R23: sin filtro de pin falla R6; sin condición ≥1 GT falla R14; sin limpieza de refs en `node rm` falla R9; además sin `NODE_UNREADABLE` falla R13, sin `NODE_NOT_IN_TREE` falla R2, sin `SELF_REF` falla R3, sin borrado de rama falla R10a.
**Tests totales**: 582 → 628
**Factor de escala**: 1.0x (6 paquetes, esfuerzo ≈ estimado; correcciones limitadas a lints de clippy y drift aditivo de goldens)
**Docs**: ENGINE_SPEC (refs, `tree relation list`, códigos nuevos, `_meta_graph`), USAGE_GUIDE §6.6, INTEGRATION (mínima `v0.4.0`, §4), RELEASE_POLICY (gate `>= 0.4.0`), CHANGELOG `[0.4.0]`, README, contract/README, tag `v0.4.0`.
**Huecos registrados (fuera de alcance)**: `node split` borra el nodo globalmente pero solo reescribe el árbol indicado (preexistente); `node rm` ignora `macro_edges`; validación XOR de EC; `--dry-run` en mutaciones (cerrado en v0.5.1).

### [Release v0.3.1] — Rebuild de contadores + roles EC (PATCH)
**Fecha**: 2026-10-06
**Naturaleza**: PATCH sin cambio de contrato, previo a RFC-002 Slice 1 (plan `PLAN_v031-and-rfc002-slice1.md`). **No** altera el % del motor ni el factor de escala.
**Avance**: `Counters::rebuild` recorre las claves `"id"` de `trees/*.json` (antes reemitía IDs embebidos tras perder `counters.json`); prefijos solo en mayúsculas; `FB` en `ENTITY_TYPES`. `EC_ROLES`/`ROLE_HELP` como fuente única del vocabulario EC para validador, MCP y CLI. UATs adversariales: `counters.json` ausente o corrupto (5 variantes), tree corrupto, frontera `LINK-999 → LINK-1000`, IDs anidados, referencias que no cuentan, determinismo, roles legacy y casi correctos. Mutation checks: sin el escaneo de árboles fallan 5 UATs; con la descripción antigua falla U10.
**Tests totales**: 567 → 582
**Docs**: CHANGELOG `[0.3.1]`, ENGINE_SPEC (`tree attach`), USAGE_GUIDE §3.3, tag `v0.3.1`.

### [Release v0.3.0] — `INVALID_ORDER` + contrato de `Storage::load_tree`
**Fecha**: 2026-10-06
**Naturaleza**: Cierre de diferidos del paquete ADR-014 + release MINOR (tipo `CSF` y error code nuevos). **No** altera el % del motor ni el factor de escala.
**Avance**: `tree walk` valida `--order` con un `WalkOrder` tipado (`INVALID_ORDER` antes de buscar el árbol, CLI y MCP); `tests/storage_contract.rs` fija que `load_tree` normaliza los 6 tipos y las ramas NBR, reutilizable por futuros backends (Turso). Verificado por mutación: sin `normalize_logic` el contrato falla.
**Tests totales**: 562 → 567
**Docs**: CHANGELOG `[0.3.0]`, ENGINE_SPEC (`tree walk`), INTEGRATION (versión mínima `v0.3.0`), README, RELEASE_POLICY (feature-gating), tag `v0.3.0`.

### [Bugfix + CSF] — Lógica de árbol derivada del tipo + `NodeType::Csf` (ADR-014)
**Fecha**: 2026-10-06
**Naturaleza**: Bugfix + tipo de nodo aditivo sobre el motor ya completo. **No** altera el % del motor ni del Knowledge Pool.
**Avance**: 8 paquetes completados, 30 tests nuevos ✅ (5 UATs CSF en `fase_02a`, 5 unit en `tree/types.rs`, 20 E2E en `tests/tree_logic.rs`).
**Tests totales**: 531 → 562 (conteo real verificado con `cargo test --workspace`; el plan esperaba 561; +1 por la regresión t3_5 de la revisión final)
**Plan**: `PLAN_csf-and-tree-logic.md` | **Specs**: ENGINE_SPEC §2.2/§2.3/§2.4/§2.12/validate, ADR-014 (nuevo), ADR-009/010/013, CLR_SPEC §1.2
**Factor de escala**: 1.0x (8 paquetes, esfuerzo ≈ estimado; las 7 tareas de código sin rondas de corrección)
**Origen**: el GT de Dettmer (`GOAL ← CSF ← NC`) no tenía tipo `CSF`, y el motor trataba el GT como árbol de suficiencia; además todos los creadores de edges escribían `SUFFICIENCY` fijo. Resultado: falsos positivos de CLR #4 en GT/EC y `tree walk` en orden contrario al documentado.

#### Entregables
- **`NodeType::Csf`** (T1): prefijo `CSF`, aceptado en CLI y MCP, contadores retrocompatibles.
- **Lógica derivada del tipo** (T2–T3): `TreeType::logic()` como única fuente de verdad; `Tree::normalize_logic()` llamado en `load_tree` (normalización al leer, persistencia perezosa, undo intacto).
- **Creadores de edges** (T4–T5): `link connect`, `insert-between`, `group` y `path replace` heredan la lógica del árbol; NBR siempre `SUFFICIENCY`. Eliminadas las dos funciones duplicadas (`logic_for_type`, `macro_edge::edge_logic`).
- **CLR #4 por lógica** (T6): solo en CRT/FRT/TT.
- **`tree walk`** (T7): default por lógica en CLI y MCP; `--order` explícito manda.

#### Decisiones
- Opción C para los borradores GT legacy (normalizar al leer + persistir en la siguiente mutación). Descartadas A, B, D y E: ver ADR-014.
- Ningún tool MCP nuevo (siguen 71). Goldens `contract/` sin cambios.

#### Siguiente
- Diferido: re-tipar los OBJ/REQ de los borradores a CSF/NC; validar valores de `--order`; eliminar `edge.logic` almacenado (RFC-002 Capa 1, MAJOR).

### [Release v0.2.0] — Política de versionado + provenance de build (infra)
**Fecha**: 2026-09-23
**Naturaleza**: Infraestructura de release. **No** altera el % del motor ni el factor de escala.
**Avance**: `build.rs` (cero deps) embebe SHA de git + flag `dirty`; `ltp --version` ≡ MCP `serverInfo.version` = `MAJOR.MINOR.PATCH+<sha>[.dirty]`. 4 UATs nuevos (`tests/version_provenance.rs`): forma SemVer+provenance, `core == CARGO_PKG_VERSION` y CLI↔MCP idénticos.
**Tests totales**: 516 → 520
**Docs**: `RELEASE_POLICY.md` (nuevo), `CHANGELOG.md` (nuevo — log canónico por versión), tag `v0.2.0`. El detalle por versión vive en el CHANGELOG, no aquí.

### [Slice 2] — Long Arrow Lifecycle (`macro add` / `macro expand` / `macro promote`)
**Fecha**: 2026-09-22
**Avance fase**: M1→M7 completadas, 36 tests nuevos ✅ (31 E2E `macro_lifecycle` + 5 unit: migración serde `MacroEdgeStatus` + huérfanos con reservas)
**Tests totales**: 480 → 516
**Plan**: `PLAN_long-arrow-slice2.md` | **Specs**: ENGINE_SPEC §2.10/§2.12/§3.2, ADR-013 (nuevo), ADR 004/005/009/010, CLR_SPEC §1
**Factor de escala**: 1.0x (7 paquetes, esfuerzo ≈ estimado)
**Origen**: Creación **top-down** de flechas largas (dirección inversa a `path collapse`): declarar un salto lógico (CLR #1) como reserva y luego resolverlo articulando la cadena (`expand`) o aceptándolo como causalidad directa (`promote`).

#### Entregables
- **`MacroEdgeStatus { Reservation, Overlay }`** (M1, ADR-013): enum tipado `#[serde(rename_all="snake_case")]`, máquina de estados sin tombstones. Migración serde no destructiva: alias `"active"` → `Overlay`, `status` ausente → `Overlay` (default histórico de `path collapse`).
- **`macro add`** (M2): reserva top-down con interior vacío (`MACRO-xxx`, estado `reservation`). Validaciones pre-minteo (`LABEL_REQUIRED`, `RESERVATION_SELF_LOOP`, `NODE_NOT_IN_TREE`) → el contador no se quema en fallo. Fuera del DAG (ADR-010).
- **`macro expand`** (M3): `reservation → overlay`. Materializa `n` INT + `n+1` LINK (`from→INT₁→…→INTₙ→to`), lógica derivada del árbol. **Bloquea ciclos** (pre-check DAG en memoria antes de `save_node`/`save_tree` → sin INT huérfanos en disco). `STEPS_REQUIRED` si no hay labels.
- **`macro promote`** (M4): `reservation → edge atómico + macro eliminada`. Migra `MacroAssumption → Assumption` (preserva `status`/`text`). Pre-check DAG antes de mintear ASM (no quema contador en camino bloqueado). Para `overlay` → `path replace`.
- **Integración `validate`** (M5): warning `LONG_ARROW_RESERVATION_PENDING` (CLR #1); reinterpretación de huérfanos (extremos de reservas sembrados como conectados → sin `ORPHAN_NODE_IN_TREE`).
- **Wiring CLI + MCP** (M6): `ltp macro add|expand|promote` + 3 MCP tools (`ltp/macro_add|expand|promote`, total **67 → 70**), con captura de historial en el llamador (ADR-009).
- **E2E + docs** (M7): `tests/macro_lifecycle.rs` (28 UATs adversariales: H1-H5, B1-B6, C1-C6, I1-I9, O1-O5, incl. bloqueos de ciclo I7/I9 e idempotencia undo/redo/batch); ADR-013; ENGINE_SPEC §2.10/§2.12/§3.2.

#### Decisiones
- **Frontera del bloqueo de ciclos** (Sombrero Negro): la reserva es no bloqueante (fuera del DAG, ADR-010), pero `expand`/`promote` crean edges reales ⇒ recuperan el bloqueo topológico con contrato idéntico a `link connect` (`CIRCULAR_DEPENDENCY_DETECTED` + `cycle_path`, sin mutación). La frontera es "¿es un edge real en `tree.edges`?", no "¿es un `macro_edge`?".
- **Orden de operaciones**: construir INT/edges en memoria → `check_dag` → persistir (evita huérfanos en disco); en `promote`, `check_dag` antes del minteo de ASM (no quema contadores en el camino bloqueado).
- **Sin tombstones**: `promote` elimina el `macro_edge`; la trazabilidad histórica la cubren ADR-009 (undo) + ADR-002 (git-diff). Registrada como decisión serde en ADR-013 (no efecto colateral).
- **Los `execute_*` no capturan historial**: lo envuelve el llamador (CLI/MCP), invariante heredada de Slice 1.

#### Siguiente
- Slice 2 completado. Fuera de alcance: `macro_assume_edit`, `explode` generalizado, reserva con extremos aún no attached (auto-attach), hash de contenido para staleness de texto.

---

### [Slice 1] — Long Arrow Assumptions (`macro-assume`)
**Fecha**: 2026-09-22
**Avance fase**: M1→M5 completadas, 40/40 tests ✅ (27 integración + 8 unit `macro_assume` + 5 unit `validate::macro_edge`)
**Tests totales**: 440 → 480
**Plan**: `PLAN_long-arrow-assumptions.md` | **Specs**: ENGINE_SPEC §2.10/§3.2, ADR 001/004/005/010, CLR_SPEC §2.1
**Origen**: Llenar/actualizar los supuestos-resumen de una long arrow (`macro_edge`) a partir de su cadena interior — ritmo "gather → author", como operación separada nunca automática.

#### Entregables
- **`MacroAssumption` (`MASM-xxx`)**: tipo propio (no extiende `Assumption`, ADR-005) con `projection_refs: Vec<String>` apuntando hacia abajo (ADR-004, solo-lectura). Vive en `MacroEdge.assumptions` con `#[serde(default, skip_serializing_if = "Vec::is_empty")]` → compat con `macro_edges` legacy. Counter `MASM` en `ENTITY_TYPES`.
- **`macro-assume gather`** (M2): vista viva del interior + diff contra el resumen (lectura pura, **sin lock de mutación ni historial**). Núcleo `compute_diff` = función pura storage-agnostic (futuro `ltp-core`), compartida con `validate`.
- **`macro-assume add / rm / list`** (M3): `add` valida `TEXT_REQUIRED`, resuelve/dedup+ordena `--projection` (BTreeSet) contra el interior vivo (`PROJECTION_REF_NOT_IN_INTERIOR`, `PROJECTION_REF_INVALID` si apunta a un MASM); `rm` → `MACRO_ASSUMPTION_NOT_FOUND`. `add`/`rm` participan en undo/redo; `list` es lectura pura.
- **Integración `validate`** (M4): `src/validate/macro_edge.rs` audita solo `macro_edges` activos y emite **warnings no-bloqueantes** (nunca afectan `valid_dag`, ADR-010): `LONG_ARROW_UNSUMMARIZED`, `LONG_ARROW_SUMMARY_STALE` (reusa `compute_diff`), `MACRO_ASSUMPTION_UNGROUNDED`.
- **4 MCP tools**: `ltp/macro_assume_gather|add|rm|list` (total tools **63 → 67**).
- **E2E** (M5): workflows collapse→gather→add×2→list→validate limpio; unmapped→stale; dangling vía `assume rm`; `path_replace` elimina la macro → `MACRO_EDGE_NOT_FOUND`; undo/redo roundtrip (mismo ID); batch (3 add + 1 rm → un undo revierte todo); diamante; macro legacy sin campo `assumptions`.

#### Decisiones
- **Namespace propio** `src/macro_assume/mod.rs` (prepara `macro_*` de Slice 2), no dentro de `path/`.
- **`gather` es lectura pura**: NO genera undo; `add`/`rm` sí (invariante de historial).
- **Staleness por membresía de conjuntos** (D9): conjunto vivo = (`interior_links` ∩ `tree.edges` existentes) ∪ {`asm.id` de esos links}. `unmapped` = supuestos interiores que ningún `projection_ref` cubre; `dangling` = refs del resumen que ya no están en el interior vivo. Cambio de *texto* NO se detecta (hash diferido, YAGNI).
- **`interior_links` es estático** (no se recomputa en mutaciones del interior): el "half unmapped" de I3 se realiza con un supuesto fresco sobre un link superviviente; el "half dangling" vía eliminación/`insert-between` del link — limitación documentada de Slice 1.

#### Siguiente
- Slice 1 completado. Slice 2 (fuera de alcance): creación top-down (`macro_add` reserva con interior vacío), `explode` generalizado, `macro_assume_edit`, hash de contenido para staleness de texto.

---

### [Bugfix] — insert-between pierde assumptions
**Fecha**: 2026-09-10
**Tests**: 437 → 440 (+3 UATs en F6: 6.18, 6.19, 6.20)

#### Problema
`link insert-between` creaba edges nuevos con `single_edge()` que inicializa `assumptions: vec![]`. Los assumptions del edge original se descartaban al filtrar el edge en los casos SINGLE e `--insert-before-effect`. El caso `--insert-after-cause` ya estaba OK (edge modificado in-place).

#### Fix
Análisis Six Thinking Hats → regla: "el edge que conserva las causas originales hereda assumptions con `needs_review`". Consistente con `link dissolve`, `link reverse --force`, y `link group`.

- **SINGLE** (A→B → A→C + C→B): assumptions → edge1 (A→C), status `needs_review`
- **insert-before-effect** (AND(A,B)→D → AND(A,B)→C + C→D): assumptions → edge1 (AND(A,B)→C), status `needs_review`
- **insert-after-cause**: sin cambios (ya correcto)
- Warning `ASSUMPTIONS_MOVED_NEED_REVIEW` emitido cuando hay assumptions transferidos

#### Archivos modificados
- `src/link/advanced.rs`: `inherited_assumptions` con `NeedsReview`, aplicadas a edge1 en ambos casos, warning condicional
- `tests/fase_06.rs`: 3 tests nuevos (SINGLE con assumptions, after-cause preserva intactos, before-effect con assumptions)
- `ENGINE_SPEC.md`: documentado comportamiento de assumptions en insert-between

---

### [F14] — Feedback Edge Primitives
**Fecha**: 2026-09-08
**Avance fase**: 6/6 tests ✅
**Tests totales**: 431 → 437
**Origen**: Completar CRUD de feedback edges (faltaban list y rm)

#### Entregables
- **`link feedback-list --tree <ID>`**: lista todas las feedback edges del tree. Solo lectura, sin historial. Reutiliza `FeedbackEdge` directamente en la respuesta (sin struct intermedio).
- **`link feedback-rm --tree <ID> --feedback <FB-ID>`**: elimina feedback edge por ID con historial (undo la restaura). Error `FEEDBACK_EDGE_NOT_FOUND` si no existe.
- **MCP tools**: `ltp/link_feedback_list` y `ltp/link_feedback_rm` (63 tools totales).
- 6 tests de integración CLI (UATs 4.12–4.17): list normal, list vacío, list tree not found, rm normal, rm not found, rm + undo roundtrip.

#### Decisiones
- Six Hats aprobó propuesta sin cambios. Dangling knowledge refs a FB-* eliminados ya cubiertos por `validate` (`DANGLING_KNOWLEDGE_REF`), sin duplicar lógica en feedback-rm.
- Sin batch rm: se compone con `history begin-batch` + N llamadas.

#### Siguiente
- Fase completada. Sin tareas pendientes.

---

### [F13] — Validation Enrichments
**Fecha**: 2026-09-01 (actualizado 2026-09-05)
**Avance fase**: 11/11 tests ✅
**Tests totales**: 420 → 431
**Origen**: Revisión Six Hats de propuestas externas (Gemini analysis PDF) + análisis SINGLE vs OR

#### Entregables
- **Cycle path en errores DAG**: `CIRCULAR_DEPENDENCY_DETECTED` ahora incluye `cycle_path` (array de IDs formando el ciclo exacto) en trunk y NBR branches. `find_cycle()` reemplaza a `has_cycle()` con DFS que traza el camino.
- **CLR#5 MAG weight normalization**: `lint_clr5_mag_weights()` valida que edges MAG al mismo destino tengan weights sumando ~1.0 (±0.01). Warnings: `CLR5_MAG_WEIGHTS_NOT_NORMALIZED`, `CLR5_MAG_WEIGHT_UNDEFINED`.
- **CLR#4/#5 Implicit OR review**: `lint_clr4_5_implicit_or()` detecta ≥2 edges SINGLE al mismo nodo destino (OR implícito). Warning `CLR4_5_IMPLICIT_OR_REVIEW` advierte que se confirme que cada causa basta sola o se agrupe con AND/MAG. Surgió del análisis Six Hats sobre la distinción SINGLE vs OR: bajo suficiencia son semánticamente equivalentes; SINGLE es el estado por defecto, no un operador lógico distinto.
- **Epistemic cascade warnings**: `node edit --epistemic` emite `EPISTEMIC_UNBOUNDED_FACT` (promoción con causas upstream débiles) y `EPISTEMIC_CASCADE_REVIEW` (degradación con efectos downstream). Chequeo cross-tree.
- **Link inspect enrichment**: `from_labels` incluye `node_type` y `epistemic` por nodo. Respuesta incluye `to_type` y `to_epistemic` para el nodo destino.
- **Collapse execution node warning**: `path collapse` emite `COLLAPSE_HIDES_EXECUTION_NODES` cuando nodos interiores son OBS/IO/PRE, con `hidden_nodes` en contexto.
- 5 tests CLR#5 (unit) + 2 tests DAG cycle path (unit) + 4 tests implicit OR (unit) = 11 tests nuevos

#### Decisiones
- 8 operaciones propuestas del PDF rechazadas (violaban ADR-001 o eran composición de primitivas existentes): `obstacle add`, `intermediate-objective add`, `action add`, `node promote`, `node degrade`, `link balance`, `predicted-effect add`, `node sanitize`.
- Todo F13 son warnings/enrichments — no se añadieron nuevos comandos CLI.
- SINGLE vs OR: bajo suficiencia, múltiples SINGLE = OR implícito. SINGLE existe como estado por defecto (neutro en ambas lógicas), no como operador semántico distinto de OR. El lint `CLR4_5_IMPLICIT_OR_REVIEW` cierra el gap de auditoría.

#### Siguiente
- Fase completada. Sin tareas pendientes.

---

### [K7] — MCP Server (Knowledge Tools)
**Fecha**: 2026-08-18
**Avance fase**: 38/38 UATs ✅
**Avance Knowledge Pool**: 81% → 100%
**Esfuerzo estimado**: 17% | **Esfuerzo real (percibido)**: 17%
**Factor de escala acumulado**: 1.0x

#### Entregables
- 7 nuevos MCP tools: `ltp/knowledge_add`, `ltp/knowledge_edit`, `ltp/knowledge_rm`, `ltp/knowledge_inspect`, `ltp/knowledge_list`, `ltp/knowledge_link`, `ltp/knowledge_unlink`
- Extensiones a tools existentes: `show_knowledge` en schemas de `ltp/trace` y `ltp/tree_walk`
- `ltp/status` via MCP ahora incluye `knowledge_health` (total, unlinked_items, contradictions, by_status, epistemic_coverage)
- `ltp/validate` via MCP reporta warnings epistémicos (DANGLING_KNOWLEDGE_REF, EPISTEMIC_UNGROUNDED, EPISTEMIC_CONTRADICTED, EPISTEMIC_UPGRADEABLE)
- `snapshot_workspace_paths` incluye `knowledge/` para undo/redo correcto via MCP
- Enum parsers para knowledge types/status/confidence/relation con errores JSON-RPC -32602
- Undo/redo integrado en todas las operaciones mutantes de knowledge via MCP
- Tool count: 54 → 61
- 38 tests de integración MCP (UATs K7.1–K7.38)

#### Siguiente
- Knowledge Pool completado al 100%

---

### [K6] — Tests End-to-End (Workflows)
**Fecha**: 2026-08-18
**Avance fase**: 31/31 UATs ✅
**Avance Knowledge Pool**: 69% → 81%
**Esfuerzo estimado**: 12% | **Esfuerzo real (percibido)**: 12%
**Factor de escala acumulado**: 1.0x

#### Entregables
- `tests/e2e_knowledge.rs` — 31 tests E2E cubriendo workflows completos:
  - Hypothesis-driven cycle (K6.1–K6.2): add → link → promote → validate
  - Refutation cascade (K6.3–K6.4): support refutado deja nodo ungrounded, sin propagación
  - Inbox management (K6.5): unlinked items como inbox que se vacía
  - Contradiction detection (K6.6–K6.7): supports vs contradicts, solo aplica a facts
  - Multi-target/multi-relation (K6.8–K6.9): un KN a múltiples targets
  - Undo roundtrip (K6.10–K6.11): granularidad correcta de undo/redo
  - Batch + undo (K6.12): atomicidad de batch con knowledge
  - Node rm + dangling + undo (K6.13–K6.14): dangling refs se resuelven con undo
  - Backwards compatibility (K6.15–K6.16): nodos sin epistemic = hypothesis
  - Trace integration (K6.17–K6.18): --show-knowledge en cadenas, edge KN no aparece en nodo
  - Cross-feature: invalidate (K6.19), collapse (K6.20), explode (K6.21), split (K6.22), tree rm (K6.23), group (K6.24), dissolve (K6.25), nbr rm (K6.26)
  - Status coherence (K6.27–K6.28): siempre refleja estado actual
  - Link disconnect (K6.29), batch con fallo parcial (K6.30), superseded contradiction (K6.31)

#### Nota técnica
- K6.29 (UAT original: `path replace` destruye LINK-IDs) adaptado a `link disconnect` — `path replace` mantiene edges interiores en el tree, no los destruye.

#### Siguiente
- K7: MCP Server (knowledge tools)

---

### [K5] — Integración con Comandos Existentes
**Fecha**: 2026-08-18
**Avance fase**: 38/51 UATs ✅
**Avance Knowledge Pool**: 49% → 69%
**Esfuerzo estimado**: 20% | **Esfuerzo real (percibido)**: 20%
**Factor de escala acumulado**: 1.0x

#### Entregables
- `status` extendido con `knowledge_health`: total, unlinked_items, contradictions, by_status, epistemic_coverage
- `validate` con 4 nuevos warnings: DANGLING_KNOWLEDGE_REF, EPISTEMIC_UNGROUNDED, EPISTEMIC_CONTRADICTED, EPISTEMIC_UPGRADEABLE
- `validate --tree T` filtra epistemic warnings solo para nodos del tree (D7)
- `trace --show-knowledge` incluye knowledge items por nodo con id/relation/status/confidence
- `node rm` emite warning KNOWLEDGE_ORPHANED con IDs de KN afectados
- `tree walk --show-knowledge` muestra conteo por relation (supports/contradicts/contextualizes)
- MCP tools `ltp/trace` y `ltp/tree_walk` extendidos con param `show_knowledge`
- Módulo `src/validate/knowledge.rs` para validación del pool epistémico
- 38 tests de integración CLI (UATs K5.1–K5.51 parcial)

#### Siguiente
- K6: Tests E2E (workflows hypothesis-driven)

---

### [K4] — Campo Epistémico en Nodos
**Fecha**: 2026-08-17
**Avance fase**: 19/19 UATs ✅
**Avance Knowledge Pool**: 41% → 49%
**Esfuerzo estimado**: 10% | **Esfuerzo real (percibido)**: 10%
**Factor de escala acumulado**: 1.0x

#### Entregables
- Enum `EpistemicStatus` {Fact, Hypothesis, Assumption, Derived} con default Hypothesis
- Campo `epistemic` en `Node` struct con skip_serializing_if hypothesis (backwards-compatible)
- Custom deserializer que trata `null` como default
- `node add --epistemic` para crear nodos con status epistémico
- `node edit --epistemic` para modificar status epistémico
- `node list --epistemic` filtro por status epistémico (incluye nodos sin campo explícito)
- `node inspect` muestra epistemic efectivo
- `node split` genera nuevos nodos con default hypothesis (no hereda)
- MCP tools `node_add`, `node_edit`, `node_list` extendidos con param `epistemic`
- 19 tests de integración CLI (UATs K4.1–K4.19)

#### Siguiente
- K5: Integración con comandos existentes (status/validate/trace/node rm/tree walk)

---

### [F1] — Fundación (workspace, traits, IDs, pipeline)
**Fecha**: 2026-08-12
**Avance fase**: 6/6 UATs ✅
**Avance global**: 0% → 10%
**Esfuerzo estimado**: 10% | **Esfuerzo real (percibido)**: 10%
**Factor de escala acumulado**: 1.0x

#### Entregables
- Trait `Storage` + impl `FsStorage` con escritura atómica (tmp → rename)
- Trait `SnapshotHook` + `NoOpHook`
- Módulo `Counters` con auto-reconstrucción desde filesystem
- `ltp init` funcional con output JSON canónico
- `ltp status` con conteo de nodos/trees y validación DAG
- Lock file con detección de stale PID
- 6 tests de integración CLI (UATs 1.1–1.6)

#### Siguiente
- F2a: Nodos standalone (add/edit/list/search)

---

### [F2a] — Nodos standalone (add/edit/list/search)
**Fecha**: 2026-08-12
**Avance fase**: 9/9 UATs ✅
**Avance global**: 10% → 14%
**Esfuerzo estimado**: 4% | **Esfuerzo real (percibido)**: 4%
**Factor de escala acumulado**: 1.0x

#### Entregables
- `ltp node add` con generación de ID secuencial y linter CLR#2
- `ltp node edit` (label, add-tag, rm-tag, observable)
- `ltp node list` sobre pool global con filtros --type/--status
- `ltp node search` por substring case-insensitive
- Linter CLR#2: detección de conjunciones causales (warning no-bloqueante)
- 9 tests de integración CLI (UATs 2a.1–2a.9)

#### Siguiente
- F3: Vistas (trees)

---

### [F3] — Vistas (trees)
**Fecha**: 2026-08-12
**Avance fase**: 11/11 UATs ✅
**Avance global**: 14% → 22%
**Esfuerzo estimado**: 8% | **Esfuerzo real (percibido)**: 8%
**Factor de escala acumulado**: 1.0x

#### Entregables
- `ltp tree new` con ID slug-based y lógica por tipo (sufficiency/necessity)
- `ltp tree list/rm/attach/detach`
- `ltp tree clone` con edges independientes y nodos compartidos
- `ltp tree diff` entre dos trees (nodes/edges added/removed)
- `ltp tree walk` con Kahn's algorithm (topological/reverse)
- 11 tests de integración CLI (UATs 3.1–3.11)

#### Siguiente
- F4: Enlaces básicos (connect/disconnect/feedback)

---

### [F4] — Enlaces básicos (connect/disconnect/feedback)
**Fecha**: 2026-08-12
**Avance fase**: 11/11 UATs ✅
**Avance global**: 22% → 31%
**Esfuerzo estimado**: 9% | **Esfuerzo real (percibido)**: 9%
**Factor de escala acumulado**: 1.0x

#### Entregables
- `ltp link connect` con operadores SINGLE/AND/OR/MAG/XOR y weight opcional
- Multi-destination: `--to A,B` genera un edge SINGLE por destino
- Multi-source: `--from A,B` con operador inferido (AND) o explícito
- Validación de integridad referencial (nodo existe en pool + attached al tree)
- Validación DAG (ciclo detectado = error bloqueante, no se persiste)
- Warning `MAG_WEIGHT_MISSING` cuando operador MAG sin weight
- `ltp link disconnect` elimina edges por ID
- `ltp link feedback` crea feedback loops (positive/negative) en `feedback_edges[]`
- Feedback edges excluidos de validación DAG
- `ltp status` reporta `feedback_edge_count` por tree
- 11 tests de integración CLI (UATs 4.1–4.11)

#### Siguiente
- F2b: Nodos cross-tree (rm/split/inspect)

---

### [F2b] — Nodos cross-tree (rm/split/inspect)
**Fecha**: 2026-08-13
**Avance fase**: 7/7 UATs ✅
**Avance global**: 31% → 36%
**Esfuerzo estimado**: 5% | **Esfuerzo real (percibido)**: 5%
**Factor de escala acumulado**: 1.0x

#### Entregables
- `ltp node rm` con limpieza cross-tree (edges, feedback_edges, node refs)
- `ltp node rm` batch (múltiples IDs separados por coma)
- `ltp node inspect` muestra participación en trees, roles, edges inbound/outbound
- `ltp node split` divide nodo en dos, hereda edges entrantes→primero, salientes→segundo
- `ltp node list --tree` filtra por membership en tree (antes se ignoraba el parámetro)
- `NodeType::prefix()` helper para generación de IDs desde el enum
- 7 tests de integración CLI (UATs 2b.1–2b.7)

#### Siguiente
- F5: Validación completa

---

### [F5] — Validación completa
**Fecha**: 2026-08-13
**Avance fase**: 14/14 UATs ✅
**Avance global**: 36% → 44%
**Esfuerzo estimado**: 8% | **Esfuerzo real (percibido)**: 8%
**Factor de escala acumulado**: 1.0x

#### Entregables
- `ltp validate [--tree]` con orquestación completa de reglas
- Infraestructura de tracing (`tracing` + `tracing-subscriber`, activable con `LTP_LOG`)
- DAG check sobre edges del tree y cada `nbr_branches[].edges`
- Integridad referencial: nodos en edges deben existir en pool global
- EC validation: objective (=1), requirements (>=2), prerequisite por requirement
- CLR#2 lint: conjunciones causales en labels de nodos del tree
- CLR#4: nodo con 1 sola entrada SINGLE (candidato a insuficiencia)
- CLR#4/#5: ≥2 SINGLE al mismo nodo sin operador declarado (OR implícito — confirmar o agrupar)
- CLR#4/#5: AND con >4 entradas (mezcla de causas independientes)
- CLR#6: inversión de tipos (UDE/DE → RC/INT)
- CLR#7: nodo intangible con <2 salientes (falta efecto predicho)
- Huérfanos: nodos attached sin edges en el tree
- `NodeType` ahora deriva `Hash`
- 14 tests de integración CLI (UATs 5.1–5.14)

#### Siguiente
- F6: Enlaces avanzados / F7: Supuestos / F8: Navegación (parallelizables)

---

### [F6] — Enlaces avanzados
**Fecha**: 2026-08-13
**Avance fase**: 17/17 UATs ✅
**Avance global**: 44% → 58%
**Esfuerzo estimado**: 14% | **Esfuerzo real (percibido)**: 14%
**Factor de escala acumulado**: 1.0x

#### Entregables
- `ltp link reverse` con gate de --force para assumptions, marca needs_review
- `ltp link move` redirige from/to con validación de integridad
- `ltp link insert-between` con 3 variantes (SINGLE, AND+after-cause, AND+before-effect)
- `ltp link group` agrupa edges SINGLE bajo operador AND/OR/MAG/XOR
- `ltp link dissolve` deshace grupo, hereda assumptions con needs_review
- `ltp link split` extrae causas de un grupo, auto-downgrade a SINGLE
- `ltp link reoperator` cambia operador con reglas de cardinalidad y weight
- `ltp link add-cause` expande from[] con promote-to para SINGLE
- `ltp link rm-cause` reduce from[], auto-downgrade a SINGLE
- Fix: serde rename `NeedsReview` → `needs_review` (snake_case)
- 17 tests de integración CLI (UATs 6.1–6.17)

#### Siguiente
- F7: Supuestos / F8: Navegación (parallelizables)

---

### [F7] — Supuestos (assumptions)
**Fecha**: 2026-08-13
**Avance fase**: 15/15 UATs ✅
**Avance global**: 58% → 64%
**Esfuerzo estimado**: 6% | **Esfuerzo real (percibido)**: 6%
**Factor de escala acumulado**: 1.0x

#### Entregables
- `ltp assume add` crea assumption con ID secuencial (ASM-XXX) en un edge
- `ltp assume edit` actualiza texto de assumption (scan lineal por ID)
- `ltp assume list [--status]` lista assumptions con filtro opcional por status
- `ltp assume move` mueve assumption entre edges (rollback si target no existe)
- `ltp assume rm` elimina assumption de su edge
- `ltp invalidate` marca ASM invalid + edge broken + crea INJ opcional
- Idempotencia (ADR-010): re-invalidate retorna success + changed:false + warning
- Auto-reparación de estados inconsistentes (ASM invalid/edge active o viceversa)
- Error codes: ASSUMPTION_NOT_FOUND, ASSUMPTION_NOT_IN_LINK, LINK_NOT_FOUND, TREE_NOT_FOUND
- 15 tests de integración CLI (UATs 7.1–7.15)

#### Siguiente
- F8: Navegación (trace)

---

### [F8] — Navegación (trace)
**Fecha**: 2026-08-13
**Avance fase**: 15/15 UATs ✅
**Avance global**: 64% → 70%
**Esfuerzo estimado**: 6% | **Esfuerzo real (percibido)**: 6%
**Factor de escala acumulado**: 1.0x

#### Entregables
- `ltp trace` BFS upstream/downstream con chain_health (broken_links, superseded_links)
- Soporte feedback_edges incluidos por defecto, excluidos con `--no-feedback`
- Soporte NBR edges con flag `--nbr` (incluye nbr_branches[].edges en traversal)
- `--depth N` limita profundidad de BFS
- ADR-010: trace no se detiene ante broken/superseded/needs_review links
- `ltp link inspect` detalle completo: from con labels, to, operator, weight, status, logic, assumptions
- `ltp link find --from A --to B` encuentra edges entre dos nodos (array vacío si no hay)
- Error codes: TREE_NOT_FOUND, NODE_NOT_FOUND, NODE_NOT_IN_TREE, LINK_NOT_FOUND
- 15 tests de integración CLI (UATs 8.1–8.15)

#### Siguiente
- F9: Abstracción (path collapse/explode/replace)

---

### [F9] — Abstracción (path)
**Fecha**: 2026-08-13
**Avance fase**: 12/12 UATs ✅
**Avance global**: 70% → 78%
**Esfuerzo estimado**: 8% | **Esfuerzo real (percibido)**: 8%
**Factor de escala acumulado**: 1.0x

#### Entregables
- `ltp path collapse` colapsa sub-grafo completo (from→to) en macro_edge (ADR-010 Decisión 1)
- BFS bidireccional (forward+backward) para calcular interior_nodes/interior_links del DAG
- Soporte para diamonds: A→B→D→E, A→C→D→E → interior_nodes: [B,C,D]
- Caso degenerado: single edge directo → interior_nodes: [], interior_links: [link]
- Validación: `NESTED_MACRO_NOT_ALLOWED` si sub-grafo ya contiene macro_edge
- `ltp path explode` convierte assumption en nodo INT intermedio, split edge en 2
- Edges nuevos heredan `logic` del original, status: active, operator: SINGLE
- `ltp path replace` marca sub-grafo táctico como superseded, conecta nodo inyección
- Error codes: NO_DIRECTED_PATH, NESTED_MACRO_NOT_ALLOWED, ASSUMPTION_NOT_IN_LINK, MACRO_EDGE_NOT_FOUND, NODE_NOT_FOUND
- 12 tests de integración CLI (UATs 9.1–9.12)

#### Siguiente
- F10: NBR (Negative Branch Reservations)

---

### [F10] — NBR (Negative Branch Reservations)
**Fecha**: 2026-08-13
**Avance fase**: 17/17 UATs ✅
**Avance global**: 78% → 83%
**Esfuerzo estimado**: 5% | **Esfuerzo real (percibido)**: 5%
**Factor de escala acumulado**: 1.0x

#### Entregables
- `ltp nbr add` crea NBR vacía con source_node + optional trim_injection
- Validación: source_node existe en pool, attached al tree, trim_injection existe si se proporciona
- `ltp nbr rm` elimina NBR branch (ADR-010 Decisión 2: nodos permanecen en pool)
- `ltp nbr list` muestra resumen: id, source_node, edge_count, has_trim
- `ltp nbr inspect` detalle completo: edges, nodes_involved, trim_injection
- `ltp link connect --nbr NBR-XXX` crea edges dentro de la NBR (no en trunk)
- Validación DAG independiente por NBR branch (ciclo en NBR = error bloqueante)
- Nodos en NBR edges solo requieren existencia en pool (no attached al tree)
- Soporte para múltiples NBRs con mismo source_node (una inyección genera N ramas)
- Recursión por referencia: NBR-002.source_node = NBR-001.trim_injection
- Error codes: NODE_NOT_FOUND, NODE_NOT_IN_TREE, NBR_NOT_FOUND, REFERENTIAL_INTEGRITY_VIOLATION, CIRCULAR_DEPENDENCY_DETECTED
- 17 tests de integración CLI (UATs 10.1–10.17, expandidos con Six Hats)

#### Siguiente
- F11: Historial (undo/redo)

---

### [F11] — Historial (Undo/Redo)
**Fecha**: 2026-08-13
**Avance fase**: 22/22 UATs ✅
**Avance global**: 83% → 89%
**Esfuerzo estimado**: 6% | **Esfuerzo real (percibido)**: 6%
**Factor de escala acumulado**: 1.0x

#### Entregables
- `HistoryManager` como componente central con snapshot-based undo/redo (ADR-009)
- SHA-256 checksums para detección de divergencias externas (edición manual, Git)
- `ltp undo [--dry-run]` con restauración atómica cross-file (write-then-rename)
- `ltp redo [--dry-run]` con verificación de before_hash
- `ltp history [--last N]` lista el stack con seq, timestamp, action, command
- `ltp history check` valida integridad de cada entry contra disco
- `ltp history invalidate --from <seq>` descarta entries desde punto de divergencia
- `ltp history clear` limpia ambos stacks
- `ltp history begin-batch --label` / `end-batch` colapsa N operaciones en una sola entry
- Rotación FIFO por `max_size_mb` (configurable en ltp.config.json)
- Captura integrada en todos los comandos mutantes (30+ commands) sin modificar el trait Storage
- Error codes: UNDO_STACK_EMPTY, REDO_STACK_EMPTY, UNDO_STATE_DIVERGED, REDO_STATE_DIVERGED, BATCH_ALREADY_IN_PROGRESS, NO_BATCH_IN_PROGRESS, HISTORY_DISABLED
- Paths relativos en entries (portabilidad entre máquinas)
- 22 tests de integración CLI (UATs 11.1–11.22, expandidos con Six Hats)

#### Siguiente
- E2E Tests: Workflows completos

---

### [E2E] — Tests End-to-End (Workflows Completos)
**Fecha**: 2026-08-13
**Avance fase**: 19/19 UATs ✅
**Avance global**: 89% → 93%
**Esfuerzo estimado**: 4% | **Esfuerzo real (percibido)**: 4%
**Factor de escala acumulado**: 1.0x

#### Entregables
- 19 tests E2E en `tests/e2e.rs` (13 originales + 6 Six Hats)
- E2E.1: CRT completo (10 nodos, cadena causal, validate + status)
- E2E.2: Insuficiencia → corrección (CLR#4 cycle)
- E2E.3: Invalidación completa (assume → invalidate → undo roundtrip)
- E2E.4: EC validation (roles, requirements, prerequisites, XOR)
- E2E.5: CRT→EC→FRT cycle (multi-tree, nodos compartidos, NBR con trim)
- E2E.6: Batch undo (10 ops atómicas)
- E2E.7: Clone + diff (edges independientes, link find en clone)
- E2E.8: Trace depth (8 niveles, depth-limited vs full)
- E2E.9: Nodo compartido multi-tree (edit visible en ambos trees)
- E2E.10: Counters recovery (auto-rebuild tras borrado)
- E2E.11: Invalidate + trace lifecycle (broken links en cadena, undo restaura)
- E2E.12: Path collapse + validate (macro_edges sin falsos positivos)
- E2E.13: NBR + invalidate + undo (aislamiento trunk/NBR)
- E2E.14: Agent simulation (navegación intercalada con mutaciones + undo/redo)
- E2E.15: Undo cross-tree cascade (node rm multi-tree → undo restaura ambos)
- E2E.16: EC incremental construction (error → fix iterativo)
- E2E.17: Path replace + undo roundtrip (superseded → active)
- E2E.18: History divergence recovery (edición externa → check → invalidate)
- E2E.19: Multi-warning iterative fix (CLR#4, CLR#6, CLR#7 → fix secuencial)

#### Siguiente
- F12: MCP Server

---

### [F12] — MCP Server
**Fecha**: 2026-08-13
**Avance fase**: 16/16 UATs ✅
**Avance global**: 93% → 100%
**Esfuerzo estimado**: 7% | **Esfuerzo real (percibido)**: 7%
**Factor de escala acumulado**: 1.0x

#### Entregables
- Binario `ltp-mcp` como servidor MCP (JSON-RPC 2.0 sobre stdin/stdout)
- Módulo `src/mcp/` con 4 archivos: types.rs, tools.rs, dispatch.rs, server.rs
- 54 tools expuestos con inputSchema JSON Schema (paridad total con CLI)
- Protocolo: initialize, tools/list, tools/call + notifications ignoradas
- History hooks integrados: mutaciones generan undo entries (paridad con CLI)
- Error codes: -32700 (parse), -32600 (invalid request), -32601 (method not found), -32602 (invalid params), -32001 (workspace not initialized)
- `isError: true` en tool result cuando CommandOutput.success es false (no JSON-RPC error)
- Graceful shutdown en EOF (exit 0, sin panic)
- Zero dependencias nuevas (solo serde_json ya existente)
- 16 tests de integración (UATs 12.1–12.16, +6 Six Hats robustness)

#### Siguiente
- Knowledge Pool (K1-K7)

---

### [K1] — Fundación Knowledge (Schema, Storage, Init)
**Fecha**: 2026-08-17
**Avance fase**: 16/16 UATs ✅
**Avance Knowledge Pool**: 0% → 8%
**Esfuerzo estimado**: 8% | **Esfuerzo real (percibido)**: 8%

#### Entregables
- Enums: KnowledgeType, KnowledgeStatus, Confidence, KnowledgeRelation
- Structs: KnowledgeItem, KnowledgeLink, KnowledgeSource
- Storage trait extendido: load/save/delete/list_knowledge, ensure_knowledge_dir
- FsStorage impl con escritura atómica y path traversal protection
- Counter "KN" integrado con rebuild desde knowledge/
- `ltp init` crea knowledge/ y counter KN=0
- Round-trip serialization con skip_serializing_if para campos opcionales

---

### [K2] — CRUD de Knowledge Items
**Fecha**: 2026-08-17
**Avance fase**: 47/47 UATs ✅
**Avance Knowledge Pool**: 8% → 26%
**Esfuerzo estimado**: 18% | **Esfuerzo real (percibido)**: 18%

#### Entregables
- `knowledge add`: validación label/source, generación ID, defaults (unverified/medium)
- `knowledge edit`: actualización parcial, validación source integrity, dedup tags
- `knowledge rm`: batch con partial success, error reporting per-ID
- `knowledge inspect`: detalle completo del item
- `knowledge list`: filtros por type/status/confidence/unlinked/tag (AND combinado)
- Undo/redo integrado (snapshot_workspace_paths incluye knowledge/)
- History manager actualizado para detectar nuevos archivos en knowledge/
- Error codes: LABEL_REQUIRED, SOURCE_REQUIRED, KNOWLEDGE_NOT_FOUND, TAG_NOT_FOUND
- 40 tests de integración CLI (UATs K2.1-K2.47)

#### Siguiente
- K3: Linking (link/unlink, target resolution, validate refs)

---

### [K3] — Linking (Vínculos al Grafo)
**Fecha**: 2026-08-17
**Avance fase**: 31/37 UATs ✅
**Avance Knowledge Pool**: 26% → 41%
**Esfuerzo estimado**: 15% | **Esfuerzo real (percibido)**: 15%

#### Entregables
- `knowledge link`: vincula KN a nodos, edges (LINK-), assumptions (ASM-), feedback edges (FB-)
- `knowledge unlink`: elimina TODOS los links a un target (D3)
- Target resolution module (`src/knowledge/resolve.rs`): resolución contra pool+trees+nbr_branches
- `knowledge list --target X [--relation R]`: filtro por target con matching_relations (D4)
- `knowledge inspect`: links resueltos con target_label y target_type (dangling = null)
- MACRO-XXX → TARGET_NOT_FOUND (macro_edges no son entidades standalone)
- Duplicate link → DUPLICATE_LINK warning (idempotente, no error)
- Orphan nodes, broken edges, invalid assumptions → linking permitido (D2)
- Batch history system extendido para incluir knowledge/ en snapshots
- Error codes: TARGET_NOT_FOUND, LINK_NOT_FOUND, TARGET_REQUIRED, DUPLICATE_LINK
- 31 tests de integración CLI (K3.1-K3.37 parcial)

#### Descubrimientos
- Bug fix: batch begin/end no incluía `knowledge/` en snapshot, causando UNDO_STATE_DIVERGED

#### Siguiente
- K4: Campo epistémico en nodos (fact/hypothesis/assumption/derived)

---

### Plantilla de reporte (se copia tras cada paquete completado)

```
### [Fase X] — [Nombre del paquete]
**Fecha**: YYYY-MM-DD
**Avance fase**: X/Y UATs ✅
**Avance global**: NN% → MM%
**Esfuerzo estimado**: N% | **Esfuerzo real (percibido)**: M%
**Factor de escala acumulado**: X.Xx

#### Descubrimientos
- (nuevos paquetes, complejidad no prevista, simplificaciones)

#### En curso
- (qué se está haciendo ahora)

#### Siguiente
- (próximo paquete según el plan)
```

---

## Reglas de Reestimación

1. **Tras completar un paquete**: si el esfuerzo real difiere significativamente del estimado (>30%), se recalcula el factor de escala:
   ```
   velocity = peso_estimado / peso_real
   ```
   Los paquetes restantes se escalan por `1/velocity` para ajustar la predicción.

2. **Descubrimiento de nuevo trabajo**: se añade como nueva fila con peso estimado. Se rebalancea el % total a 100% redistribuyendo proporcionalmente entre todos los paquetes no completados. Los completados mantienen su % real.

3. **Eliminación de trabajo**: si un paquete se simplifica o se descarta, su peso se redistribuye entre los restantes.

4. **El avance global** se calcula como:
   ```
   avance = Σ (peso_ajustado × progreso_fase)
   donde progreso_fase = UATs_pasando / UATs_total de esa fase
   ```

---

## Histórico de Reestimaciones

| Fecha | Motivo | Cambio | Impacto en total |
|-------|--------|--------|-----------------|
| — | Plan inicial | 14 paquetes, 128 UATs | 100% baseline |
| 2026-08-13 | Expansión de UATs (ADR-010) | +42 UATs en F7–F12+E2E (error paths, edge cases, nbr rm, trace broken, invalidate idempotente) | 128 → 170 UATs. Avance global sigue 58% (pesos por fase sin cambio; fases completadas mantienen 100% de su peso). |
| 2026-09-22 | Slice 2 — Long Arrow Lifecycle (ADR-013) | +36 tests (31 E2E `macro_lifecycle` + 5 unit). Enriquecimiento sobre el motor base ya completo (no altera % del motor base ni del Knowledge Pool). | 480 → 516 tests. Factor de escala 1.0x. |
| 2026-09-23 | Release v0.2.0 — versionado + provenance (infra) | +4 UATs (`version_provenance`). Infraestructura de release; no altera % del motor ni factor de escala. | 516 → 520 tests. |
| 2026-10-06 | CSF + lógica de árbol derivada del tipo (ADR-014) | +31 tests (5 UATs CSF + 5 unit + 21 E2E `tree_logic`). Bugfix + tipo aditivo sobre el motor completo (no altera % del motor base ni del Knowledge Pool). | 531 → 562 tests. Factor de escala 1.0x. |
| 2026-10-06 | Release v0.3.0 — `INVALID_ORDER` + contrato `Storage` | +5 tests (3 E2E `tree_logic` + 2 `storage_contract`). Cierre de diferidos; no altera % del motor ni factor de escala. | 562 → 567 tests. |
| 2026-10-06 | Release v0.3.1 — rebuild de contadores + roles EC | +15 tests (8 E2E `counters_rebuild` + 2 unit + 5 `ec_roles`). PATCH; no altera % del motor ni factor de escala. | 567 → 582 tests. |
