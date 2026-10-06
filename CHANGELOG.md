# Changelog

Todos los cambios notables de `ltp-engine` se documentan aquí.

El formato sigue [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/) y el proyecto se adhiere a [Semantic Versioning](https://semver.org/lang/es/) según [RELEASE_POLICY.md](RELEASE_POLICY.md).

## [Unreleased]

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
