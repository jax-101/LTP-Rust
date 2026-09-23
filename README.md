# ltp-engine

Motor **determinista** (Rust) para el *Logical Thinking Process* (LTP) de H. William Dettmer. Expone **CLI + servidor MCP** y gestiona un Grafo Causal Global en disco mediante operaciones *headless*, sin capa visual.

El motor valida **topología** (DAG), **integridad referencial** e **IDs únicos**; **nunca evalúa semántica** — eso lo hace el consumidor (una UI o un agente LLM). Separación estricta de responsabilidades (ADR-001).

**Estado**: `v0.2.0` (ver [CHANGELOG.md](CHANGELOG.md)). Estables: núcleo determinista, Knowledge Pool, historial undo/redo y la flecha larga (macro-edges). **RFC-002 (meta-grafo) es diseño, NO implementado.**

## Empieza aquí según quién eres

| Si eres… | Empieza por… |
|----------|--------------|
| **Un consumidor** (UI, app, IA) que quiere *usar* el motor | **[INTEGRATION.md](INTEGRATION.md)** — la puerta: CLI vs MCP, contrato de salida, versionado, estable/WIP |
| Una IA *headless* vía MCP | [MCP_QUICKSTART.md](MCP_QUICKSTART.md) + [INTEGRATION.md](INTEGRATION.md) §2B |
| Quien va a *modificar* el motor | [CLAUDE.md](CLAUDE.md) (reglas) → [PLAN.md](PLAN.md) → [PROGRESS.md](PROGRESS.md) |

## Build

```bash
cargo build --release   # → target/release/ltp (CLI) y target/release/ltp-mcp (servidor MCP)
```

Gate de verificación (obligatorio antes de dar algo por completado):

```bash
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace
cargo fmt --all -- --check
```

## Mapa de documentación

**Consumir el motor**
- [INTEGRATION.md](INTEGRATION.md) — puerta de entrada para consumidores (CLI y MCP).
- [MCP_QUICKSTART.md](MCP_QUICKSTART.md) — arranque rápido del servidor MCP.

**Contrato / especificación**
- [ENGINE_SPEC.md](ENGINE_SPEC.md) — comandos, argumentos, esquemas JSON y contrato de salida.
- [CLR_SPEC.md](CLR_SPEC.md) — las 8 Categorías de Legitimación Lógica (validación / linting).
- [KNOWLEDGE_SPEC.md](KNOWLEDGE_SPEC.md) — Knowledge Pool: schema, comandos y workflow epistémico.

**Versionado y evolución**
- [RELEASE_POLICY.md](RELEASE_POLICY.md) — SemVer anclado al contrato + provenance de build.
- [CHANGELOG.md](CHANGELOG.md) — qué cambió en cada versión.

**Arquitectura y decisiones**
- [ADR.md](ADR.md) — registro de decisiones de arquitectura.
- [RFC-001_hybrid-persistence.md](RFC-001_hybrid-persistence.md) — persistencia híbrida (JSON → Turso).
- [RFC-002_meta-graph.md](RFC-002_meta-graph.md) — meta-grafo (**diseño, NO implementado**).

**Planificación y progreso**
- [PLAN.md](PLAN.md) — plan de desarrollo por fases con UATs.
- [PLAN_long-arrow-assumptions.md](PLAN_long-arrow-assumptions.md) — plan de la flecha larga.
- [PROGRESS.md](PROGRESS.md) — dashboard de avance y telemetría.

**Para agentes / Claude**
- [CLAUDE.md](CLAUDE.md) — instrucciones del proyecto y reglas de desarrollo.

## Principio: el binario es la fuente de verdad

La versión se lee del binario (`ltp --version` en CLI, `initialize` en MCP) y el catálogo de comandos también (`ltp --help` / `tools/list`). La documentación describe el **contrato**; no hardcodea recuentos ni listas volátiles. Un consumidor que quiera evolucionar con el motor solo tiene que hacer `git pull`, releer [INTEGRATION.md](INTEGRATION.md) + [CHANGELOG.md](CHANGELOG.md) y verificar la versión — el resto lo lee del binario en marcha.
