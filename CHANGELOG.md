# Changelog

Todos los cambios notables de `ltp-engine` se documentan aquí.

El formato sigue [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/) y el proyecto se adhiere a [Semantic Versioning](https://semver.org/lang/es/) según [RELEASE_POLICY.md](RELEASE_POLICY.md).

## [Unreleased]

## [0.6.0] - 2026-10-10

"Ilegible ≠ ausente" (plan `PLAN_v060.md`, adenda D-K6 a ADR-016). Es MINOR: tres warnings nuevos, y los códigos de "no encontrado" quedan restringidos a su significado documentado. Ningún campo ni forma cambia.

### Added

- **`KNOWLEDGE_POOL_UNREADABLE`**: si `knowledge/` no se puede listar, `status`, `validate`, `node rm` y `tree walk`/`trace --show-knowledge` emiten un único aviso, antes de cualquier `KNOWLEDGE_LOAD_ERROR`. `validate` se salta el análisis epistémico, que antes daba `EPISTEMIC_UNGROUNDED` falsos, y `node rm` avisa sin bloquear.
- **`COUNTERS_REBUILT {reason: missing|corrupt|stale}`** (con `prefix`/`from`/`to` si es `stale`): la reparación de `.ltp/counters.json` al mintear deja de ser silenciosa. Sale como mucho uno por comando, siempre detrás de `STALE_LOCK_REMOVED`, y solo en salidas de éxito. Lo emiten los 20 comandos que mintean IDs.
- **`DUPLICATE_ENTITY_ID {id, occurrences: [{tree_id, location}]}`** en la entrada `_workspace` de `validate`: IDs repetidos que dejaron `tree clone`/`link dissolve` antes de v0.5.2, con la reparación en `detail`. Es warning: no cambia `success`. Con `--tree T` se cruza con todo el workspace, y un árbol ilegible da `TREE_LOAD_ERROR {tree_id}` en `_workspace` en vez de contar como vacío.

### Changed

- **Lo ilegible ya no se reporta como inexistente.** `NODE_NOT_FOUND`, `TREE_NOT_FOUND`, `KNOWLEDGE_NOT_FOUND`, `TARGET_NOT_FOUND` y `REFERENTIAL_INTEGRITY_VIOLATION` salen solo si la entidad no existe. Si existe y no se puede leer (permisos, ENOTDIR, symlink colgante, JSON corrupto), sale `IO_ERROR` con la causa en `detail`. Se corrigieron tres capas: el storage (sin `exists()`), 60 sitios de los comandos (una única función de mapeo) y la resolución de destinos de knowledge links. Nota de migración en INTEGRATION §4.
- `knowledge link` hacia un destino en un árbol ilegible da `IO_ERROR {tree_id}` (antes `TARGET_NOT_FOUND`). `knowledge inspect` avisa con `TREE_LOAD_ERROR {tree_id}`, y `validate` ya no emite un `DANGLING_KNOWLEDGE_REF` falso.
- Los avisos de sesión van siempre al principio de `warnings` (`STALE_LOCK_REMOVED` y después `COUNTERS_REBUILT`). En `invalidate`, `STALE_LOCK_REMOVED` iba detrás de `ALREADY_INVALIDATED`/`STATE_REPAIRED`.
- **Rendimiento**: `validate` carga cada árbol una sola vez (antes, dos). En un workspace de 4,3 MB (50 árboles, 5.000 aristas, 5.000 supuestos) baja de 66 a 61 ms.
- Tests: suites `v060_unreadable` (40 E2E, incluida una tabla con los 20 comandos que mintean) y `v060_duplicates` (10 E2E), unit de D-3 y D-5, y dos goldens nuevos en `contract/`.

### Fixed

- `validate` callaba si fallaba el listado de nodos; ahora da `IO_ERROR`.
- Un symlink colgante en `nodes/` o `trees/` se reportaba como inexistente aunque `tree list` lo mostrara.

## [0.5.2] - 2026-10-09

"Un ID nuevo nunca pisa uno existente" (plan `PLAN_v052.md`, adendas a ADR-009 y ADR-005). Es PATCH: sin campos, códigos ni flags nuevos.

### Fixed

- **Un `git pull` seguido de `node add` sobrescribía el nodo recibido** (C1): `.ltp/` no se versiona, así que el contador local podía estar por debajo de lo que había en disco. Ahora cada ID nuevo es el máximo entre `counters.json` y lo observado en disco, más uno. Lo mismo para cualquier contador desactualizado (por ejemplo `LINK` con un árbol traído por pull).
- **Reconstruir contadores se saltaba lo ilegible** y volvía a emitir IDs existentes, con `success: true` y sin avisos: un árbol con `chmod 000` duplicaba `LINK-001` (C3), y `nodes/` en `-wx` hacía que `node add` sobrescribiera `UDE-001` (C4). Ahora lo ilegible dentro del ámbito del prefijo da `ID_GENERATION_ERROR` con la ruta en `detail`, haya o no `counters.json`. Fuera del ámbito no se lee nada: un árbol ilegible no bloquea `node add` ni `knowledge add`.
- **Un árbol con marcas de conflicto de merge ocultaba sus IDs** (C2): ahora se escanea como texto y el contador no se queda corto.
- **Un clon recién hecho no se podía usar** (C6): toda mutación daba `LOCK_ERROR` porque faltaba `.ltp/`. `acquire_lock` lo crea.
- **`counters.json` sin permiso de lectura** (C5) daba `ID_GENERATION_ERROR` por accidente; ahora es la regla, con la ruta en `detail`.
- **`tree clone` copiaba los IDs de `ASM` y `FB`**, y **`link dissolve` repartía los mismos `ASM` entre las aristas nuevas** (D-6). `assume rm` borraba una sola copia y un knowledge link pasaba en silencio a la copia al borrar el original. Ahora la copia recibe IDs nuevos (en `dissolve`, la primera arista conserva los originales).

### Changed

- **Rendimiento**: cada comando escanea cada ámbito una sola vez mientras tiene el lock. En un workspace de 4 MB, `node add` tarda unos 51 ms (53 ms en v0.5.1) y `tree clone` de 5.000 aristas y 5.000 supuestos, 0,75 s.
- **Interno**: `link connect --nbr` busca la rama NBR una sola vez, antes de reservar IDs, y la guarda hasta insertar las aristas. Desaparece una segunda búsqueda que, de alcanzarse, habría quemado un contador `LINK`. Sin cambios en el contrato: mismos códigos, mismo orden de errores, mismo output.
- Tests: suite `v052_counters` (13 E2E, incluidos dos clones git reales) y 7 unit de contadores y memoización.

## [0.5.1] - 2026-10-08

"El motor no calla" (plan `PLAN_v051.md`, ADR-017, adenda D-K5 a ADR-016). Es PATCH: hace cumplir promesas ya documentadas, sin campos, códigos ni flags nuevos.

### Fixed

- **`--dry-run` escribía en disco** en todos los comandos salvo `init`, `undo` y `redo` (`node add --dry-run` creaba el nodo). Ahora el CLI ejecuta el comando real sobre una copia temporal de lo que gestiona LTP (`ltp.config.json`, `nodes/`, `trees/`, `knowledge/`, `.ltp/` sin `tmp/`) y la descarta. El output y el código de salida son idénticos a los de la ejecución real, y el workspace no cambia ni un byte: ni contadores, ni historial, ni lock. Si la simulación no se puede montar, devuelve `IO_ERROR` (`action: "dry_run"`) sin ejecutar nada. `init`/`undo`/`redo` conservan su `--dry-run` nativo.
- **Un knowledge ilegible se descartaba en silencio** en `status` (CLI y MCP), `validate`, `tree walk --show-knowledge`, `trace --show-knowledge` y `node rm` (que perdía `KNOWLEDGE_ORPHANED`). Ahora cada uno emite `KNOWLEDGE_LOAD_ERROR {id}`, el código que ya usaba `knowledge list`, en orden de ID. `node rm` avisa y no bloquea. `knowledge_health` tiene una sola implementación para CLI y MCP.
- **No-determinismo entre ejecuciones** (invariante 1): `cycle_path` de `CIRCULAR_DEPENDENCY_DETECTED` salía como una rotación arbitraria del ciclo, y el orden de los warnings CLR4, CLR4/5, CLR5 y CLR7 de `validate` cambiaba de un proceso a otro. Ahora el DFS arranca por el ID menor y los avisos salen ordenados por nodo.
- Sin `.expect()` en producción: si la serialización falla, `to_json` devuelve el contrato de error (`INTERNAL_ERROR`) en vez de abortar, y `link connect --nbr` devuelve `NBR_NOT_FOUND` por un camino tipado.

### Changed

- Lint `missing_docs` activo en la librería: los módulos nuevos deben documentarse; la deuda existente (654 elementos en 15 módulos) queda congelada con `allow` explícitos.
- Tests: 14 UATs del Knowledge Pool que faltaban o tenían aserciones vacuas (`if let` que nunca fallaba), y las suites `v051_dry_run`, `v051_knowledge_unreadable` y `v051_no_expect`.

## [0.5.0] - 2026-10-08

Integridad referencial global de las mutaciones de nodo (ADR-016, plan `PLAN_v050-integrity.md`). Es MINOR: el warning `MACRO_EDGE_REMOVED` y el campo `affected_trees` de `node split` son aditivos. `validate` se vuelve más estricto con el código que ya existía.

### Added

- **`node split` → `data.affected_trees`**: los árboles reescritos, ordenados; incluye siempre `--tree`.
- **Warning `MACRO_EDGE_REMOVED {tree_id, macro_link, reason, status, from, to, assumption_ids}`** en `node rm`, con `reason` = `endpoint_removed` | `interior_emptied`.

### Changed

- **`node split` es global**: reescribe todos los árboles donde aparece el nodo (tronco, ramas NBR, feedback y flechas largas). Lo entrante va al primer hijo y lo saliente, al segundo. En `nodes[]`/`interior_nodes` los hijos ocupan la posición del original. `--tree` sigue siendo el árbol de contexto obligatorio.
- **`validate`**: `REFERENTIAL_INTEGRITY_VIOLATION` cubre `nodes[]`, `feedback_edges`, ramas NBR (`source_node` y edges) y `macro_edges` (`from`, `to`, `interior_nodes`), con contexto `tree_id`, `node_id`, `location`, `field` y el ID del contenedor. Los workspaces dañados por versiones anteriores que antes pasaban ahora fallan: estaban rotos. Un nodo ilegible no cuenta como violación.
- **`node rm` y `node split` son fail-closed**: cargan todos los árboles antes de escribir, y un árbol ilegible da `IO_ERROR {tree_id}` sin escribir nada ni consumir contadores.
- Descripciones MCP y ayuda CLI de `node split`/`node rm` actualizadas.

### Fixed

- **`node split` dejaba referencias colgantes**: borraba el nodo del pool pero solo reescribía `--tree`, y ni siquiera ahí tocaba el feedback ni las flechas largas.
- **`node rm` ignoraba `macro_edges`**: las flechas largas quedaban con extremos o interiores colgantes. Ahora se podan o se eliminan con aviso.
- **`macro expand`, `macro promote` y `path replace`** materializaban edges rotos sobre una flecha larga colgante. Ahora comprueban los extremos antes de crear nada: `NODE_NOT_IN_TREE` si no está adjunto y `NODE_NOT_FOUND` si está adjunto pero ausente del pool.
- **`node rm A,A`** borraba el nodo y después fallaba sin entrada de undo. Ahora los IDs repetidos se deduplican.
- `node rm`/`node split` se saltaban en silencio los árboles ilegibles (`continue`), lo que producía justamente esta corrupción.

## [0.4.0] - 2026-10-06

RFC-002 Slice 1 (ADR-015): refs entre nodos y meta-grafo inferido. Cambio aditivo (MINOR): los nodos sin refs se serializan byte-idénticos a v0.3.x.

### Added

- **Refs entre nodos** (`metadata.refs: [{node, tree}]`, set ordenado): `node add --ref NODE[@TREE]`, `node edit --add-ref/--rm-ref` (repetibles); MCP `refs` en `ltp/node_add` y `add_refs`/`rm_refs` en `ltp/node_edit` (parser estricto, `-32602` ante shape inválido). Validación bloqueante previa al minteo de ID: `NODE_NOT_FOUND`, `TREE_NOT_FOUND`, `NODE_NOT_IN_TREE` y el código nuevo `SELF_REF`. CLI: `INVALID_REF` para un `--ref` léxicamente inválido. `--rm-ref` de una ref ausente → warning `REF_NOT_PRESENT`.
- **`node inspect`** expone `refs` y `referenced_by`.
- **`tree relation list [--tree]`** / `ltp/tree_relation_list` (tool nº 72): meta-grafo inferido al vuelo, relaciones **estructurales y sin tipo** entre extremos `{tree, nbr}` con `logic` por extremo y `basis` ordenado. Nunca se persiste.
- **`validate`, entrada sintética `_meta_graph`**: `DANGLING_NODE_REF` (`node_missing|tree_missing|not_in_tree`), `NORM_REF_MISSING` (UDE de CRT/NBR sin ref a una norma de un GT; solo con ≥1 GT) y `NODE_UNREADABLE`. Todos son warnings.
- Golden de contrato `contract/tree_relation_list.json`.

### Fixed

- **`node rm` y ramas NBR**: los edges de rama que tocaban el nodo quedaban colgando; ahora se eliminan, y si el nodo era el `source_node` la rama entera se borra (warning `NBR_BRANCH_REMOVED`). Las refs entrantes se limpian (warning `REFS_STRIPPED`).
- **`node split`**: perdía la metadata del nodo; ahora ambos hijos heredan `refs` y claves extra, las refs entrantes se reescriben a ambos hijos y los edges NBR se redirigen.
- **`validate`** saltaba en silencio los nodos ilegibles; ahora emite `NODE_UNREADABLE`.

### Changed

- Golden `contract/warning_root.json`: `data` de `node edit` incluye `refs` (aditivo).

## [0.3.1] - 2026-10-06

### Fixed

- **Reconstrucción de contadores** (`.ltp/counters.json` ausente o corrupto): el rebuild solo leía nombres de fichero, así que los IDs que viven dentro del JSON de los árboles (`LINK`, `ASM`, `NBR`, `MACRO`, `MASM`, `FB`) volvían a 0 y el motor podía **reemitir IDs ya existentes**. Ahora también se recorren las claves `"id"` de `trees/*.json` (los ficheros ilegibles se saltan). `FB` pasa a ser un tipo contado desde `init`. Un tree con slug numérico (`tree-crt-2024`) ya no crea la clave basura `TREE-CRT` en `counters.json`.
- **Vocabulario de roles de EC**: el schema MCP de `ltp/tree_attach` anunciaba `root, leaf, intermediate`, que siempre fallan `validate` con `EC_VALIDATION`. La descripción (MCP y `ltp tree attach --help`) ahora sale de la misma constante que usa el validador: `objective` (1), `requirement` (≥2), `prerequisite` (≥1 por requirement, `prerequisite → requirement`). Solo cambia la documentación; el comportamiento del validador no varía.

## [0.3.0] - 2026-10-06

### Added

- **`tree rename`**: renombra el `name` (label) de una instancia de tree existente sin tocar su `id` ni el fichero `trees/<id>.json`, preservando la integridad referencial (`attach`, refs, consumidores externos). Análogo a `node edit` sobre `node.label`. Expuesto en CLI (`ltp tree rename <TREE_ID> --name "<nuevo>"`) y MCP (`ltp/tree_rename`) — 71 tools MCP en total. Errores tipados `TREE_NOT_FOUND` e `INVALID_TREE_NAME` (nombre vacío); idempotente al renombrar al mismo nombre.
- **Tipo de nodo `CSF`** (Critical Success Factor), nivel intermedio del Goal Tree (`GOAL ← CSF ← NC`): `ltp node add --type CSF` / `ltp/node_add`, IDs `CSF-xxx`. Los workspaces previos (sin la clave `CSF` en `counters.json`) arrancan en `CSF-001` sin migración. Valor de enum nuevo ⇒ MINOR (RELEASE_POLICY §1).
- **Error `INVALID_ORDER` en `tree walk`**: `--order` (CLI) / `"order"` (MCP) solo acepta `topological` o `reverse` (sensible a mayúsculas). Antes un valor desconocido se aceptaba en silencio y se recorría en `topological`. Se valida antes de buscar el árbol; `data.order` devuelve el valor recibido. Error code nuevo ⇒ MINOR.
- **Test de contrato de `Storage`** (`tests/storage_contract.rs`): fija que `load_tree` devuelve el árbol normalizado (ADR-014) para los 6 tipos, incluidas las ramas NBR. Todo backend futuro (Turso) debe pasarlo.

### Fixed

- **Orden por defecto de `tree walk`** (ADR-014): sin `--order`, CLI y MCP recorrían siempre en `topological`, en contra de ENGINE_SPEC. Ahora GT/EC/PRT usan `reverse` (desde el objetivo) y CRT/FRT/TT siguen en `topological`. `data.order` informa del orden aplicado. **Consumidores**: para conservar el comportamiento anterior en árboles de necesidad, pasad `--order topological` (o `"order": "topological"` en MCP) explícitamente.
- **Lógica de los Goal Trees** (ADR-014): `tree new gt` creaba el árbol con lógica `sufficiency`; ahora es `necessity`, como EC y PRT (CLR_SPEC §1.2). La lógica de un árbol se deriva siempre de su tipo. Los GT ya guardados se leen corregidos y se persisten así en su siguiente mutación (el `undo` posterior restaura el fichero original byte a byte).
- **Lógica de los edges** (ADR-014): `link connect` escribía siempre `SUFFICIENCY`, incluso en GT/EC/PRT. Ahora el edge hereda la lógica del árbol; los edges de una rama NBR (`--nbr`) son siempre `SUFFICIENCY`.
- **Lógica de los edges creados por `link insert-between`, `link group` y `path replace`** (ADR-014): escribían siempre `SUFFICIENCY`. Ahora heredan la lógica del árbol, igual que `link connect` y `macro expand`/`macro promote`.
- **Falsos positivos de CLR #4 en árboles de necesidad** (ADR-014): `validate` emitía `CLR4_INSUFFICIENT_CAUSE`, `CLR4_5_IMPLICIT_OR_REVIEW` y `CLR4_5_EXCESSIVE_AND_INPUTS` en GT/EC/PRT, donde cada condición necesaria es insuficiente por construcción. Ahora solo se evalúan en CRT/FRT/TT.

## [0.2.0] - 2026-09-23

Primer release bajo la política de versionado formal ([RELEASE_POLICY.md](RELEASE_POLICY.md)).

### Added

- **Ciclo de vida de la flecha larga** (Slice 2, ADR-013): comandos `macro add` (reserva top-down), `macro expand` (materializar la reserva en una cadena INT) y `macro promote` (promover a edge atómico con migración de supuestos). Expuestos en CLI y MCP (`ltp/macro_add`, `ltp/macro_expand`, `ltp/macro_promote`) — 70 tools MCP en total.
- `MacroEdgeStatus` tipado (`reservation` | `overlay`) con migración serde retrocompatible (alias legacy `active`).
- Warning `LONG_ARROW_RESERVATION_PENDING` en `validate` para reservas pendientes de resolver (CLR#1), no bloqueante (ADR-010).
- **Provenance de build**: el binario reporta su versión con el SHA de git embebido (`MAJOR.MINOR.PATCH+<sha>[.dirty]`), idéntica en `ltp --version` y en el `initialize` del MCP (`serverInfo.version`).

### Changed

- `serverInfo.version` (MCP) y `ltp --version` (CLI) pasan de reportar el SemVer pelado a incluir la provenance de git.

### Fixed

- Descripción del tool MCP `ltp/path_explode`: decía "Explode a macro-link back to its original sub-graph" cuando desglosa un supuesto de un edge normal en un nodo intermedio (INT). Alineada con `ENGINE_SPEC` y el contrato real.

## [0.1.0] - baseline

Estado del motor previo a la política de versionado formal: núcleo determinista (grafo causal, DAG, integridad referencial, IDs secuenciales), Knowledge Pool, historial undo/redo, Slice 1 de la flecha larga (`macro-assume`) y servidor MCP. La versión permaneció congelada en `0.1.0` durante todo este periodo; se documenta aquí como línea base.
