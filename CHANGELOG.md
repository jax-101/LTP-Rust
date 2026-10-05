# Changelog

Todos los cambios notables de `ltp-engine` se documentan aquí.

El formato sigue [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/) y el proyecto se adhiere a [Semantic Versioning](https://semver.org/lang/es/) según [RELEASE_POLICY.md](RELEASE_POLICY.md).

## [Unreleased]

### Added

- **`tree rename`**: renombra el `name` (label) de una instancia de tree existente sin tocar su `id` ni el fichero `trees/<id>.json`, preservando la integridad referencial (`attach`, refs, consumidores externos). Análogo a `node edit` sobre `node.label`. Expuesto en CLI (`ltp tree rename <TREE_ID> --name "<nuevo>"`) y MCP (`ltp/tree_rename`) — 71 tools MCP en total. Errores tipados `TREE_NOT_FOUND` e `INVALID_TREE_NAME` (nombre vacío); idempotente al renombrar al mismo nombre.
- **Tipo de nodo `CSF`** (Critical Success Factor), nivel intermedio del Goal Tree (`GOAL ← CSF ← NC`): `ltp node add --type CSF` / `ltp/node_add`, IDs `CSF-xxx`. Los workspaces previos (sin la clave `CSF` en `counters.json`) arrancan en `CSF-001` sin migración. Valor de enum nuevo ⇒ MINOR (RELEASE_POLICY §1).

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
