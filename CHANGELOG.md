# Changelog

Todos los cambios notables de `ltp-engine` se documentan aquí.

El formato sigue [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/) y el proyecto se adhiere a [Semantic Versioning](https://semver.org/lang/es/) según [RELEASE_POLICY.md](RELEASE_POLICY.md).

## [Unreleased]

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
