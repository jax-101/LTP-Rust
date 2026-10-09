# PLAN v0.6.0 (MINOR) — Ilegible ≠ ausente

## Contexto

> **Revisión 2 (2026-10-09, tras los Six Hats).** Cambios: se descarta la opción D (YAGNI) y el scope vuelve a `Unlisted`. En D-3, abrir y mirar `NotFound` en vez de `try_exists()`. D-5 se amplía con la reconciliación de v0.5.2. Son 31 sitios, no 33. Se añade el riesgo para el consumidor UI.

Sustituye a la Parte B de `PLAN_post-v051.md`. Investigación y mediciones del 2026-10-09 sobre el binario de `46a69a9`, en un workspace temporal con `uid` 501 (no root). Decisiones del usuario (2026-10-09):
- La pérdida de datos en la reconstrucción de contadores sale antes, como **v0.5.2 PATCH** (`PLAN_v052.md`). **Este plan arranca después de v0.5.2.**
- El `dry_run` de MCP (antes B3) sale de v0.6.0 y pasa a una **release propia (v0.7.0)** con su plan. No tiene relación con este tema y v0.6.0 ya es grande.

Fuentes consultadas: ENGINE_SPEC §2.0/§2.1/§2.12, ADR-009, ADR-016 (D-4, D-5, D-K5), ADR-017, KNOWLEDGE_SPEC §6.0, RELEASE_POLICY §1/§5, INTEGRATION §2A/§4. CLR_SPEC no aplica (no se toca ningún lint CLR).

### Mediciones

**Hipótesis del lock: refutada.** La Parte B la daba como prioridad alta. Con un lock de otro proceso vivo y `.ltp/` en cada modo:

| `.ltp/` | Resultado | Lock ajeno |
|---|---|---|
| `000` | `LOCK_ERROR` (`exists()` da `false`, pero `fs::write` falla) | intacto |
| `100`, `300`, `500` | `WORKSPACE_LOCKED` | intacto |

El lock es fail-closed en todos los casos, así que no hay bug. `acquire_lock` adopta la regla de D-3 por coherencia. Ya en v0.5.2 (D-4) crea `.ltp/` si falta, para que un clon funcione.

**`*_NOT_FOUND` falsos: dos capas, no una.**

| Caso | Hoy | Capa |
|---|---|---|
| `nodes/` en 000 → `node inspect UDE-001` | `NODE_NOT_FOUND` | storage (`exists()`) |
| `trees/` en 000 → `tree walk` | `TREE_NOT_FOUND` | storage |
| **fichero** de árbol en 000 (el directorio sí es legible) → `tree walk` | `TREE_NOT_FOUND` | **comando**: traduce cualquier `Err` de `load_tree` |
| nodo en 000 → `link connect` | `REFERENTIAL_INTEGRITY_VIOLATION` | comando |
| `knowledge/` en 000 → `knowledge inspect` | `KNOWLEDGE_NOT_FOUND` | storage |
| `nodes/` en 000 → `node list`, `status` | `IO_ERROR` | correcto |

Capa 1: 14 llamadas a `exists()` en `src/workspace/fs_storage.rs`. Capa 2: los comandos emiten `TREE_NOT_FOUND` en 49 sitios (11 ficheros), `NODE_NOT_FOUND` en 14 (6), `KNOWLEDGE_NOT_FOUND` en 5 (1) y `REFERENTIAL_INTEGRITY_VIOLATION` en 6 (3). Hay que revisar cuáles traducen un `Err` arbitrario. Arreglar solo la capa 1 no cierra la fila del fichero de árbol.

**`load_pool` sin listado (B1, ya medido en `PLAN_post-v051.md`)**: falso `EPISTEMIC_UNGROUNDED` en `validate`, `KNOWLEDGE_ORPHANED` perdido en `node rm`, `status` a 0 sin aviso.

**Adyacente:** `validate_knowledge` (`src/validate/knowledge.rs:46`) devuelve en silencio si falla `list_node_ids`, y se salta los nodos ilegibles con `continue`. Lo segundo ya lo cubre `NODE_UNREADABLE` en `validate/mod.rs:330`. Lo primero hay que comprobarlo en T0.

---

## Decisiones

- **D-1 (B1, solo la opción A)**: `KnowledgePool` lleva `scope: PoolScope`, con `enum PoolScope { Complete, Unlisted }`. `Unlisted` significa que `list_knowledge_ids` falló entero, así que no hay items. Warning nuevo `KNOWLEDGE_POOL_UNREADABLE` (`detail` con el error de E/S), una vez por comando y antes de cualquier `KNOWLEDGE_LOAD_ERROR`. El trait `Storage` no cambia.
  - **Se descarta la opción D** ("una entrada mala no tumba el listado"). Un `DirEntry` en error es un fallo de `readdir(3)` a mitad de recorrido (EIO), no un fichero malo. Los ficheros malos ya los cubre D-K5 item a item. Para implementar D haría falta un método nuevo en el trait y un tercer estado de scope (listado parcial con items), que reintroduciría falsos `EPISTEMIC_UPGRADEABLE`: se contarían supports y faltarían contradicts. No compensa por un caso que no se ha visto.
- **D-2 Consumidores con `Unlisted`**:
  - `validate` se salta **todo** el análisis epistémico por nodo. Con `Unlisted` no hay items, así que el único aviso posible sería el falso `EPISTEMIC_UNGROUNDED`.
  - `node rm` emite `KNOWLEDGE_POOL_UNREADABLE` sin bloquear (D-K5: `rm` no escribe knowledge).
  - `status` mantiene los números (cuenta lo legible) y añade el aviso. Pasar a `null` cambiaría la forma del JSON, y eso es MAJOR.
  - `trace` y `tree walk` emiten el aviso solo con `--show-knowledge`.
- **D-3 (B2, capa 1)**: en `fs_storage` se eliminan los `exists()` previos. Se abre o se borra directamente y se hace `match` sobre `ErrorKind::NotFound` → `*NotFound`; cualquier otro error → `LtpError::Io`. Se descarta `try_exists()`: es una llamada al sistema más y deja una ventana entre comprobar y abrir. En `list_*`, un directorio ausente sigue siendo una lista vacía y uno ilegible pasa a ser `Err`. Ojo: ENOTDIR (`ErrorKind::NotADirectory`) **no** es `NotFound` → `IO_ERROR`, y es lo correcto, porque el workspace está roto.
- **D-4 (B2, capa 2)**: una única función, `fn load_error_code(e: &LtpError, not_found: &'static str) -> &'static str`. Las variantes `*NotFound` devuelven el código de "no encontrado" del sitio y todo lo demás devuelve `IO_ERROR`, con el contexto que ya tenga (`tree_id`, `node_id`…). Es la regla que ADR-016 D-4 ya aplica en `rm`/`split` ("un árbol inexistente da `TREE_NOT_FOUND`; uno que existe pero no se puede leer da `IO_ERROR {tree_id}`"), extendida a todo el motor.
- **D-5 Avisos de contadores** (requiere v0.5.2, D-1 a D-3): `Storage::next_id` devuelve `Result<MintedId>` con `MintedId { id: String, notice: Option<CounterNotice> }`. `CounterNotice` es un enum: `Rebuilt { reason: Missing | Corrupt }`, `Reconciled { from, to }` (el contador estaba por debajo del disco; caso C1 de v0.5.2) y `Unreadable { paths }` (el doble fallo residual de v0.5.2 D-3, que ya no se calla). Sale un solo warning `COUNTERS_REBUILT` con `context.reason` (`missing` / `corrupt` / `stale`) y `unreadable` si aplica, justo después de `STALE_LOCK_REMOVED` si existe, y si no en primer lugar. Un comando que mintea varias veces lo emite una vez: tras el primer minteo, `counters.json` ya está al día y los siguientes no traen aviso. Se descarta un buzón de avisos con `RefCell` en `FsStorage` (estado oculto) y también `(String, Vec<OutputWarning>)` (el compilador no obliga a mirarlo). Con `MintedId`, el compilador lleva a los **31** sitios.
- **D-6 Contrato**: MINOR, con dos warnings nuevos (`KNOWLEDGE_POOL_UNREADABLE` y `COUNTERS_REBUILT`). Que `*_NOT_FOUND` y `REFERENTIAL_INTEGRITY_VIOLATION` pasen a `IO_ERROR` cuando algo existe pero no se puede leer **restringe esos códigos a su significado documentado**: no se lo cambia. Por eso es MINOR con nota de migración en INTEGRATION, no MAJOR. Para un consumidor que reintentaba ante `*_NOT_FOUND`, la acción correcta pasa a ser "revisar el disco". **Riesgo para el consumidor UI** (los 36 flujos sobre MCP): hay que revisar qué flujos ramifican con `TREE_NOT_FOUND` o `NODE_NOT_FOUND`, por ejemplo "crear el árbol si no existe". Con un fichero ilegible, ese flujo antes intentaba crear y ahora se para con `IO_ERROR`. Es el comportamiento correcto, pero se tiene que poder ver en la nota de migración.

## Tareas

| T | Qué | Verificación |
|---|---|---|
| T0 | Tests primero (`tests/v060_unreadable.rs`), en rojo. Casos deterministas con ENOTDIR o EISDIR (un fichero en lugar de un directorio, o al revés). Casos con `chmod`: `cfg(unix)` y salto si `uid == 0`. Uno por fila de las tablas de arriba, más B1 (4 comandos) y `COUNTERS_REBUILT` (CLI y MCP). Medir el adyacente de `validate_knowledge` con `list_node_ids` en error. | Todos en rojo salvo los casos "correcto" |
| T1 | D-1 + D-2 (`src/knowledge/pool.rs` y sus 6 consumidores) | Tests B1 en verde. Tests de v0.5.1 `v051_knowledge_unreadable` intactos |
| T2 | D-3 (`fs_storage.rs`, 14 sitios; el lock ya cambió en v0.5.2 D-4) | Las filas "storage" en verde |
| T3 | D-4 (unos 74 sitios en los comandos; inventariar primero cuáles traducen un `Err` arbitrario) | La fila "comando" en verde. Las suites de contrato de los `*_NOT_FOUND` legítimos, intactas |
| T4 | D-5 (`MintedId`, 31 sitios) | `COUNTERS_REBUILT` con `reason` `missing`/`corrupt`/`stale` en CLI y MCP. E2E.10 de PLAN.md:452 cumplido por fin |
| T5 | Mutaciones: quitar el aviso de pool; no saltar el análisis epistémico con `Unlisted`; volver a `exists()` en `load_tree`; que `load_error_code` devuelva siempre `not_found`; descartar `rebuilt` | 5/5 detectadas |
| T6 | Docs: adenda **D-K6** a ADR-016 ("ilegible ≠ ausente"), KNOWLEDGE_SPEC §6.0 (fila `KNOWLEDGE_POOL_UNREADABLE` + scope), ENGINE_SPEC (orden de warnings en `rm`/`status`/`validate`/`walk`/`trace`; `COUNTERS_REBUILT`; regla `*_NOT_FOUND` vs `IO_ERROR`), INTEGRATION (gate `>= 0.6.0` y nota de migración), snapshot nuevo en `contract/`, CHANGELOG, PROGRESS | — |
| T7 | Las 4 verificaciones + release (RELEASE_POLICY §5) | `0.6.0+<sha>` sin `dirty` |

Un commit por tarea (`test(v0.6.0)`, `feat(v0.6.0)`, `docs(v0.6.0)`, `chore(release)`).

## Fuera de alcance (registrado)

- **`dry_run` en MCP** → v0.7.0, plan propio. Reutilizaría la copia de ADR-017 D-2 (`dry_run::DryRunCopy`) ejecutando en el mismo proceso sobre un `FsStorage` apuntado a la copia: MCP no termina con `process::exit`, así que no necesita relanzar un proceso hijo como D-1.
- **`src/history/manager.rs`**: unas 25 llamadas a `exists()`. Medido: `undo` con `nodes/` en 000 responde `UNDO_STATE_DIVERGED`. Es fail-closed, pero el código engaña (no hay divergencia, sino un fallo de lectura). Va con su propio análisis, porque toca ADR-009.
- `create_new` (O_EXCL) al crear entidades como segunda defensa frente a IDs repetidos (ver `PLAN_v052.md`).
