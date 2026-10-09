# PLAN v0.6.0 (MINOR) — Ilegible ≠ ausente

## Contexto

> **Revisión 3 (2026-10-09, tras publicar v0.5.2 y una pasada de Six Hats; decisiones del usuario).** Cambios respecto a la rev 2.1:
> - Referencias actualizadas al código de v0.5.2: **34** sitios de `next_id`, no 31, y `workspace_exists() -> bool` es del trait. Hay un adyacente nuevo, `validate/mod.rs:327`.
> - **Capa 3 nueva** (`knowledge/resolve.rs`), medida: un árbol ilegible produce un `DANGLING_KNOWLEDGE_REF` **falso**, `target_type: "unknown"` en `knowledge inspect` y `TARGET_NOT_FOUND` en `knowledge link` → **D-7**.
> - D-5 se encaja con la D-7 de v0.5.2: hay **como mucho un aviso por cada vez que se toma el lock**, guardado en el mismo estado `LockSession`. Así se evita el `stale` falso en los comandos que mintean en dos ámbitos. **D-5b**: el aviso sale solo en salidas de éxito.
> - **D-8 nueva**: warning `DUPLICATE_ENTITY_ID` en `validate`, para los duplicados que crearon `tree clone` y `link dissolve` antes de v0.5.2. Es warning, no error, porque no hay reparación por comandos.
> - El riesgo para la UI se corrige con datos: la UI lanza el **CLI** y **no ramifica por códigos**. El riesgo real es que **no ve los avisos** (ver D-6).
> - Los avisos de knowledge links colgantes en las mutaciones (unos 15 comandos) **salen de v0.6.0**: `validate` ya los detecta con `DANGLING_KNOWLEDGE_REF`.
>
> **Revisión 2.1 (2026-10-09).** D-5 pierde `CounterNotice::Unreadable` por la D-3 de `PLAN_v052.md` rev 3.
>
> **Revisión 2 (2026-10-09, tras los Six Hats).** Se descarta la opción D (YAGNI) y el scope vuelve a `Unlisted`. En D-3, abrir y mirar `NotFound` en vez de `try_exists()`. D-5 se amplía con la reconciliación de v0.5.2.

Sustituye a la Parte B de `PLAN_post-v051.md`. Arranca después de v0.5.2, que salió el 2026-10-09 (`0f0f476`). Las mediciones de la rev 3 son sobre el binario `0.5.2+0f0f4767`, en workspaces temporales con `uid` 501 (no root). Las de la rev 2, sobre `46a69a9`.

El `dry_run` de MCP va en una **release propia (v0.7.0)** con su plan.

Fuentes consultadas: ENGINE_SPEC §2.0/§2.1/§2.12/§3.1, ADR-009 (y su adenda v0.5.2), ADR-005 (adenda v0.5.2), ADR-016 (D-4, D-5, D-K5), ADR-017, KNOWLEDGE_SPEC §6.0, RELEASE_POLICY §1/§5, INTEGRATION §2A/§4, y el repo de la UI `LTP-Rust-UI` (`c3fe87e`). CLR_SPEC no aplica: no se toca ningún lint CLR.

### Mediciones

**Hipótesis del lock: refutada (rev 2).** Con un lock de otro proceso vivo y `.ltp/` en cada modo:

| `.ltp/` | Resultado | Lock ajeno |
|---|---|---|
| `000` | `LOCK_ERROR` (`exists()` da `false`, pero `fs::write` falla) | intacto |
| `100`, `300`, `500` | `WORKSPACE_LOCKED` | intacto |

El lock es fail-closed en todos los casos. Desde v0.5.2 (D-4), `acquire_lock` crea `.ltp/` si falta. Sus dos `exists()` (`fs_storage.rs:267` y `:301`) siguen ahí y adoptan la regla de D-3 por coherencia.

**`*_NOT_FOUND` falsos: tres capas.**

| Caso | Hoy | Capa |
|---|---|---|
| `nodes/` en 000 → `node inspect UDE-001` | `NODE_NOT_FOUND` | 1 storage (`exists()`) |
| `trees/` en 000 → `tree walk` | `TREE_NOT_FOUND` | 1 storage |
| **fichero** de árbol en 000 (el directorio sí es legible) → `tree walk` | `TREE_NOT_FOUND` | 2 **comando**: traduce cualquier `Err` de `load_tree` |
| nodo en 000 → `link connect` | `REFERENTIAL_INTEGRITY_VIOLATION` | 2 comando |
| `knowledge/` en 000 → `knowledge inspect` | `KNOWLEDGE_NOT_FOUND` | 1 storage |
| árbol ilegible con `ASM-001` + `KN-001 → ASM-001` → `validate` | `TREE_LOAD_ERROR` (correcto) **más un `DANGLING_KNOWLEDGE_REF` falso** ("does not exist") | **3 resolve** (rev 3, medido) |
| lo mismo → `knowledge inspect KN-001` | `target_type: "unknown"`, sin aviso | 3 resolve |
| lo mismo → `knowledge link KN-002 --to ASM-001` | `TARGET_NOT_FOUND` (por lectura del código, `knowledge/commands.rs:831`) | 3 resolve |
| `nodes/` en 000 → `node list`, `status` | `IO_ERROR` | correcto |

- Capa 1: 14 llamadas a `exists()` en `src/workspace/fs_storage.rs` (`:169 :185 :194 :212 :231 :240 :267 :301 :334 :367 :380 :397 :406 :424`). `:334` es `workspace_exists() -> bool` del trait `Storage`.
- Capa 2: los comandos emiten `TREE_NOT_FOUND` en 49 sitios (11 ficheros), `NODE_NOT_FOUND` en 14 (6), `KNOWLEDGE_NOT_FOUND` en 5 (1) y `REFERENTIAL_INTEGRITY_VIOLATION` en 6 (3). Hay que revisar cuáles traducen un `Err` arbitrario.
- Capa 3: `src/knowledge/resolve.rs` hace `list_tree_ids().unwrap_or_default()` y `Err(_) => continue` en `resolve_edge`, `resolve_feedback_edge` y `resolve_assumption`, y `resolve_node` hace `Err(_) => None`. Tiene tres consumidores: `validate_knowledge` (`target_exists`), `knowledge inspect` (`:599`) y `knowledge link` (`:831`).
- Arreglar solo la capa 1 no cierra ni la fila del fichero de árbol ni la capa 3.

**`load_pool` sin listado (B1, ya medido en `PLAN_post-v051.md`)**: falso `EPISTEMIC_UNGROUNDED` en `validate`, `KNOWLEDGE_ORPHANED` perdido en `node rm`, `status` a 0 sin aviso.

**Adyacentes en `validate`:**
- `validate_knowledge` (`src/validate/knowledge.rs:46`) devuelve en silencio si falla `list_node_ids`, y se salta con `continue` los nodos ilegibles. Lo segundo ya lo cubre `NODE_UNREADABLE` (`validate/mod.rs:330`).
- `validate/mod.rs:327` hace `list_node_ids().unwrap_or_default()`, también en silencio (rev 3).

**IDs duplicados heredados (rev 3, medido).**
- Con un `ASM-001` copiado a mano entre dos árboles, igual que hacía `tree clone` antes de v0.5.2: `validate` no avisa de nada. Tras `tree rm` del original, `knowledge inspect` resuelve `ASM-001` contra la copia (`target_type: "assumption"`), también sin aviso.
- Con el `ASM` duplicado dentro de un árbol, igual que hacía `dissolve`: `assume rm --asm ASM-001` borra **una** copia y responde `success`. Queda una sola, y eso repara el caso.
- Los duplicados de `clone` **no tienen reparación por comandos**: hay que hacer `tree rm` del clon y volver a clonar (se pierde lo editado en él) o editar el JSON a mano.

**Knowledge links colgantes (rev 3, medido).** Tras `tree rm`, `validate` **ya emite** `DANGLING_KNOWLEDGE_REF {knowledge_id, target}` en el pseudo-árbol `_knowledge_pool`. Lo que no existe es el aviso en el momento de la mutación. Unos 12–15 comandos quitan `LINK`, `ASM` o `FB`: `link disconnect`, `dissolve`, `insert-between`, `split`, `path replace`/`explode`, `tree rm`, `tree detach`, `assume rm`, `link feedback-rm`… → fuera de alcance (ver abajo).

**Consumidor UI (rev 3, medido en `LTP-Rust-UI` `c3fe87e`).**
- La UI lanza el **CLI**, un proceso por comando; MCP es solo para agentes sin cabeza.
- **No ramifica por códigos de error.** `src/engine/parser.ts` convierte cada error en `"CODE: detail"` y lanza el primero. Ningún flujo hace "si `TREE_NOT_FOUND`, créalo". Los `TREE_NOT_FOUND` de sus tests son mocks que siguen siendo válidos.
- **Descarta los `warnings` de las mutaciones**: solo `runValidate` los guarda.
- **El panel de `validate` lee `errors`/`warnings` del nivel superior**, pero el motor pone los hallazgos en `data.details[]`. Hoy la UI muestra 0 errores aunque `validate` falle.
- Consecuencia: todo aviso que añada v0.6.0 (`KNOWLEDGE_POOL_UNREADABLE`, `COUNTERS_REBUILT`, `DUPLICATE_ENTITY_ID`) será invisible en la UI hasta que se arregle allí.

**Salidas de error sin avisos (rev 3).** Hay 233 `warnings: vec![]` escritos a mano en `src`. Ninguna salida de error lleva avisos hoy, ni siquiera `STALE_LOCK_REMOVED`.

---

## Decisiones

- **D-1 (B1, solo la opción A)**: `KnowledgePool` lleva `scope: PoolScope`, con `enum PoolScope { Complete, Unlisted }`. `Unlisted` significa que `list_knowledge_ids` falló entero, así que no hay items. Warning nuevo `KNOWLEDGE_POOL_UNREADABLE` (`detail` con el error de E/S), una vez por comando y antes de cualquier `KNOWLEDGE_LOAD_ERROR`. El trait `Storage` no cambia.
  - **Se descarta la opción D** ("una entrada mala no tumba el listado"). Un `DirEntry` en error es un fallo de `readdir(3)` a mitad de recorrido (EIO), no un fichero malo, y los ficheros malos ya los cubre D-K5 item a item. Implementar D exigiría un método nuevo en el trait y un tercer estado de scope, que reintroduciría falsos `EPISTEMIC_UPGRADEABLE`.
- **D-2 Consumidores con `Unlisted`**:
  - `validate` se salta **todo** el análisis epistémico por nodo, porque el único aviso posible sería el falso `EPISTEMIC_UNGROUNDED`.
  - `node rm` emite `KNOWLEDGE_POOL_UNREADABLE` sin bloquear (D-K5: `rm` no escribe knowledge).
  - `status` mantiene los números (cuenta lo legible) y añade el aviso. Pasar a `null` cambiaría la forma del JSON, y eso es MAJOR.
  - `trace` y `tree walk` emiten el aviso solo con `--show-knowledge`.
- **D-3 (capa 1)**: en `fs_storage` se eliminan los `exists()` previos. Se abre o se borra directamente y se hace `match` sobre `ErrorKind::NotFound` → `*NotFound`; cualquier otro error → `LtpError::Io`. Se descarta `try_exists()`: es una llamada al sistema más y deja una ventana entre comprobar y abrir.
  - En `list_*`, un directorio ausente sigue siendo una lista vacía, y uno ilegible pasa a ser `Err`.
  - ENOTDIR (`ErrorKind::NotADirectory`) **no** es `NotFound` → `IO_ERROR`, y es lo correcto: el workspace está roto.
  - `workspace_exists()` pasa a devolver `Result<bool>` (cambia la firma del trait; la API Rust no es contrato). Sus llamadores tratan `Err` como `IO_ERROR`, no como "no hay workspace".
  - Los dos `exists()` del lock adoptan la misma regla.
  - Los adyacentes `validate/knowledge.rs:46` y `validate/mod.rs:327` dejan de callar: un `Err` de `list_node_ids` da `IO_ERROR` en `validate`.
- **D-4 (capa 2)**: una única función, `fn load_error_code(e: &LtpError, not_found: &'static str) -> &'static str`. Las variantes `*NotFound` devuelven el código de "no encontrado" del sitio, y todo lo demás devuelve `IO_ERROR` con el contexto que ya tenga (`tree_id`, `node_id`…). Es la regla de ADR-016 D-4 en `rm` y `split`, extendida a todo el motor.
- **D-5 Avisos de contadores** (sobre v0.5.2, D-1 a D-3 y D-7):
  - **Tipo.** `Storage::next_id` devuelve `Result<MintedId>`, con `MintedId { id: String, notice: Option<CounterNotice> }`. `CounterNotice` es un enum:
    - `Rebuilt { reason: Missing | Corrupt }`: `counters.json` faltaba o no parseaba (sale de `StoredCounters`);
    - `Reconciled { prefix, from, to }`: el contador estaba por debajo del disco (caso C1 de v0.5.2).

    No hay variante `Unreadable`, porque desde v0.5.2 lo ilegible dentro del ámbito ya es `ID_GENERATION_ERROR`.
  - **Como mucho un aviso por cada vez que se toma el lock (rev 3).** El estado de D-7 de v0.5.2 (`ReconciledScopes`) pasa a ser `LockSession { scopes: ReconciledScopes, notice_emitted: bool }`, que es `Copy`. Mantiene las mismas reglas: se borra en `acquire_lock` y en `release_lock`, y solo vale con el lock tomado. `next_id` devuelve `Some(notice)` solo si todavía no se emitió ninguno en esta sesión de lock.
    - Motivo, medido por lectura del código: `macro expand` y `path replace` mintean `INT` (ámbito Nodos) y `LINK`/`ASM` (ámbito Árboles). Si falta `counters.json`, el primer minteo da `Missing` y guarda un fichero con los prefijos de árbol a 0. El segundo ámbito vería entonces un fichero válido por debajo del disco, y saldría un `stale` **falso**.
    - Gana el primer motivo. `Missing`/`Corrupt` se detectan en el primer minteo del comando, sea del ámbito que sea. Un `stale` legítimo del segundo ámbito, con `counters.json` válido y sin aviso previo, sí sale.
  - **Salida.** Un solo warning `COUNTERS_REBUILT` con `context.reason` = `missing` | `corrupt` | `stale` (y `prefix`, `from`, `to` si es `stale`). Va justo después de `STALE_LOCK_REMOVED` si existe; si no, en primer lugar. Un helper compartido sustituye a las unas 13 copias de `stale_lock_warning` y coloca los dos avisos en orden.
  - **Descartados**: un buzón de avisos en `FsStorage` que el comando vacía (el compilador no obliga a mirarlo) y `(String, Vec<OutputWarning>)`. Con `MintedId`, el compilador lleva a los **34** sitios.
  - **D-5b (rev 3): solo en salidas de éxito.** Si un comando falla después de mintear (por ejemplo, `CIRCULAR_DEPENDENCY_DETECTED`), el aviso se pierde, pero los contadores ya quedaron reparados en disco. No hay riesgo de datos: es informativo, y D-K5 trata de lo ilegible saltado en silencio. Hoy **ninguna** salida de error lleva avisos (233 `warnings: vec![]`, `STALE_LOCK_REMOVED` incluido). Poner este aviso solo en algunas ramas de error dejaría la política incoherente. Se documenta en ENGINE_SPEC ("una salida de error no lleva warnings"), y llevar los avisos también a los errores queda registrado como refactor uniforme aparte.
  - `--dry-run` (ADR-017) muestra el aviso igual que la ejecución real, porque la copia tiene el mismo `counters.json`.
- **D-6 Contrato**: MINOR, con **tres** warnings nuevos: `KNOWLEDGE_POOL_UNREADABLE`, `COUNTERS_REBUILT` y `DUPLICATE_ENTITY_ID`.
  - Que `*_NOT_FOUND`, `REFERENTIAL_INTEGRITY_VIOLATION` y `TARGET_NOT_FOUND` pasen a `IO_ERROR` cuando algo existe pero no se puede leer **restringe esos códigos a su significado documentado**, no se lo cambia. Por eso es MINOR con nota de migración en INTEGRATION, no MAJOR. Para un consumidor que reintentaba ante `*_NOT_FOUND`, la acción correcta pasa a ser "revisar el disco".
  - **Consumidor UI (corregido en la rev 3)**: no ramifica por códigos, así que el cambio a `IO_ERROR` solo cambia el mensaje que ve el usuario. El riesgo es el contrario: los avisos nuevos no se ven en la UI (descarta los avisos de las mutaciones y lee `validate` del nivel equivocado). La nota de migración lo dice explícitamente. El arreglo va en `LTP-Rust-UI`, fuera de este plan, y conviene hacerlo antes o junto a v0.6.0.
- **D-7 (capa 3, rev 3)**: `resolve_target` pasa a devolver `Result<Option<ResolvedTarget>>` y `target_exists`, `Result<bool>`. `Ok(None)` significa "no existe": todos los árboles se leyeron y no estaba. `Err` significa que algún árbol, o el nodo, no se pudo leer y no se encontró en lo legible. Si se encuentra en un árbol legible, es `Ok(Some)` aunque otro árbol sea ilegible. Consumidores:
  - `validate`: ante un `Err`, **no** emite `DANGLING_KNOWLEDGE_REF`. El árbol ilegible ya sale como `TREE_LOAD_ERROR` en sus `details`, así que no se calla nada.
  - `knowledge inspect` (solo lee; D-K5: se avisa): el link sale con `target_type: "unknown"` como hoy, más un warning `TREE_LOAD_ERROR {tree_id}`. El código ya existe (lo usa `validate`); aquí se emite como aviso.
  - `knowledge link` (escribe un link cuyo destino no puede verificar): `IO_ERROR {tree_id}` en lugar de `TARGET_NOT_FOUND`. Es fail-closed, como ADR-016 D-4.
- **D-8 IDs duplicados heredados (rev 3)**: `validate` emite el warning **`DUPLICATE_ENTITY_ID {id, occurrences: [{tree_id, location}]}`**, uno por ID repetido y ordenado por ID, en el pseudo-árbol `_workspace`.
  - **Qué recorre**: un recorrido tipado, no el escáner de `"id"` de `counters.rs`, por `edges[]` (y sus `assumptions[]`), `feedback_edges[]`, `nbr_branches[]` (su ID y sus aristas con sus supuestos), `macro_edges[]` y sus `MASM`.
  - **Con `--tree T`**: solo los IDs que aparecen en `T`, pero cruzados con todo el workspace.
  - **Un árbol ilegible no cuenta**: ya sale como `TREE_LOAD_ERROR`, y el recorrido no lo da por vacío en silencio.
  - **Warning, no error**: los duplicados los creó el propio motor antes de v0.5.2, y los de `clone` no se pueden reparar con comandos. Un error dejaría en rojo para siempre cualquier workspace con clones antiguos, y un agente al que se le pide "deja `validate` en verde" entraría en bucle.
  - **Reparación en el `detail`**: para un duplicado dentro de un árbol, `assume rm` (deja una sola copia); para uno entre árboles, `tree rm` del clon y volver a clonar, o editar el JSON. Pasarlo a error cuando exista un comando de reparación queda registrado; endurecer `validate` así ya tiene precedente en v0.5.0 (MINOR).

## Tareas

| T | Qué | Verificación |
|---|---|---|
| T0 | Tests primero (`tests/v060_unreadable.rs` y `tests/v060_duplicates.rs`), en rojo. Casos deterministas con ENOTDIR/EISDIR (un fichero en lugar de un directorio, o al revés). Casos con `chmod`: `cfg(unix)` y salto si `uid == 0`. Uno por fila de la tabla de capas, más: B1 (4 comandos); `COUNTERS_REBUILT` en CLI y MCP con `missing`, `corrupt` y `stale`; **un solo aviso en `macro expand` y en `path replace` sin `counters.json`** (D-5, sin `stale` falso); ninguno en la segunda ejecución; ninguno en una salida de error (D-5b); los adyacentes `validate` `:46` y `:327`; D-8 (duplicado entre árboles, dentro de un árbol, en NBR, en feedback, con `--tree`, y sin falso positivo en un workspace sano). | Todos en rojo salvo los casos "correcto" y el de D-5b |
| T1 | D-1 + D-2 (`src/knowledge/pool.rs` y sus 6 consumidores) | Tests B1 en verde. `v051_knowledge_unreadable` intacto |
| T2 | D-3 (`fs_storage.rs`, 14 sitios, `workspace_exists() -> Result<bool>` y los adyacentes de `validate`) | Filas "storage" en verde |
| T3 | D-4 (unos 74 sitios en los comandos; primero inventariar cuáles traducen un `Err` arbitrario) | Fila "comando" en verde. Las suites de contrato de los `*_NOT_FOUND` legítimos, intactas |
| T4 | D-7 (`resolve.rs` y sus 3 consumidores) | Filas "resolve" en verde. `DANGLING_KNOWLEDGE_REF` legítimo (tras `tree rm`) intacto, incluido el UAT D10.8 de la UI |
| T5 | D-5 (`MintedId` en los 34 sitios, `LockSession`, helper de avisos de lock y contadores) | `COUNTERS_REBUILT` en CLI y MCP, con un solo aviso por comando. R13–R16 de v0.5.2 intactos. E2E.10 de `PLAN.md:452` cumplido por fin |
| T6 | D-8 (`validate`, recorrido tipado) | Tests de duplicados en verde. Sin falsos positivos en `contract/` ni en las suites existentes |
| T7 | Mutaciones: quitar el aviso de pool; no saltar el análisis epistémico con `Unlisted`; volver a `exists()` en `load_tree`; que `load_error_code` devuelva siempre `not_found`; descartar `notice`; no marcar `notice_emitted` (dos avisos en `macro expand`); no borrar `notice_emitted` en `acquire_lock`; que `resolve` vuelva a tragarse los `Err` (vuelve el `DANGLING` falso); que D-8 ignore las `nbr_branches`; que D-8 cuente un árbol ilegible como vacío | 10/10 detectadas |
| T8 | Rendimiento: `validate` en el workspace de 4 MB de v0.5.2 T3 (5.000 aristas y 5.000 supuestos), antes y después de D-7 y D-8 | Menos de un 20 % de aumento sobre la medida de antes. Números en PROGRESS |
| T9 | Docs: adenda **D-K6** a ADR-016 ("ilegible ≠ ausente", tres capas). KNOWLEDGE_SPEC §6.0 (fila `KNOWLEDGE_POOL_UNREADABLE` + scope; `TREE_LOAD_ERROR` en `inspect`; `IO_ERROR` en `link`). ENGINE_SPEC (orden de warnings en `rm`/`status`/`validate`/`walk`/`trace`; `COUNTERS_REBUILT` y "una salida de error no lleva warnings"; regla `*_NOT_FOUND` frente a `IO_ERROR`; `DUPLICATE_ENTITY_ID`). INTEGRATION (gate `>= 0.6.0`, nota de migración y nota de la UI). Snapshot nuevo en `contract/`, CHANGELOG, PROGRESS | — |
| T10 | Las 4 verificaciones + release (RELEASE_POLICY §5) | `0.6.0+<sha>` sin `dirty` |

Un commit por tarea (`test(v0.6.0)`, `feat(v0.6.0)`, `docs(v0.6.0)`, `chore(release)`). Las mutaciones (T7) y el rendimiento (T8) van en el commit de docs (T9).

## Fuera de alcance (registrado)

- **`dry_run` en MCP** → v0.7.0, con plan propio. Reutilizaría la copia de ADR-017 D-2 (`dry_run::DryRunCopy`) ejecutando en el mismo proceso sobre un `FsStorage` apuntado a la copia. Con la UI sobre el CLI, su prioridad baja: solo lo usan agentes sin cabeza.
- **Avisos de knowledge links colgantes en las mutaciones** (`KNOWLEDGE_ORPHANED` o similar, unos 12–15 comandos que quitan `LINK`/`ASM`/`FB`) → un MINOR posterior. Hoy los detecta `validate` (`DANGLING_KNOWLEDGE_REF`).
- **Avisos en las salidas de error** (233 `warnings: vec![]`): un refactor uniforme que arregle a la vez `STALE_LOCK_REMOVED` y `COUNTERS_REBUILT`.
- **Comando de reparación de duplicados** y, cuando exista, pasar `DUPLICATE_ENTITY_ID` a error.
- **Ambigüedad en `knowledge inspect`** cuando un destino aparece en varios árboles: se resuelve contra el primero. Con D-8, `validate` ya lo hace visible.
- **UI (`LTP-Rust-UI`)**: leer `data.details[]` en el panel de `validate` y mostrar los avisos de las mutaciones. Es otro repo, y conviene hacerlo antes o junto a v0.6.0.
- **`src/history/manager.rs`**: unas 25 llamadas a `exists()`. Medido: `undo` con `nodes/` en 000 responde `UNDO_STATE_DIVERGED`. Es fail-closed, pero el código engaña (no hay divergencia, sino un fallo de lectura). Va con su propio análisis, porque toca ADR-009.
- `create_new` (O_EXCL) al crear entidades como segunda defensa frente a IDs repetidos (ver `PLAN_v052.md`).
