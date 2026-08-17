# CLAUDE.md

## Proyecto

`ltp-engine` — Motor determinista CLI + servidor MCP (en Rust) para el Logical Thinking Process (LTP) de H. William Dettmer. Gestiona un Grafo Causal Global en disco mediante operaciones headless, sin capa visual.

## Stack Tecnologico

| Crate | Proposito |
|-------|-----------|
| `clap` (derive) | CLI parsing |
| `serde` + `serde_json` | Serializacion JSON canonica (BTreeMap) |
| `sha2` | SHA-256 para checksums undo/redo |
| `thiserror` | Enum de errores tipados |
| `chrono` | Timestamps ISO 8601 |
| `libc` | PID check para lock file (unix) |

## Comandos de Verificacion

```bash
cargo check --all-targets --all-features        # Compilacion completa (lib + bins + tests + benches)
cargo clippy --all-targets --all-features -- -D warnings  # Lints estrictos sobre todo el workspace
cargo test --workspace                           # Unit + integration tests
cargo fmt --all -- --check                       # Verificar formateo sin modificar
```

## Arquitectura (ADR-001)

Separacion estricta de responsabilidades:

- **Motor Determinista (este repo, Rust)**: valida topologia (DAG), integridad referencial, IDs unicos, prevencion de ciclos (DFS 3 colores). Nunca evalua semantica.
- **Agente LLM (consumidor externo)**: compone flujos de trabajo invocando primitivas del motor, evalua CLR semanticas (causalidad, claridad, efecto predicho).

## Invariantes de Implementacion

1. **Determinismo absoluto**: IDs secuenciales por tipo (`UDE-001`, `RC-003`). JSON canonico: claves ordenadas alfabeticamente (BTreeMap), indent 2 espacios. Output JSON por defecto.
2. **Topologia estricta, semantica no-bloqueante**: el motor valida DAG e integridad referencial de forma bloqueante; las advertencias semanticas (CLR #2: conjunciones prohibidas) son warnings, nunca errores.
3. **Persistencia unidireccional**: disco = grafo tactico completo. Vistas ejecutivas (`collapse`) se calculan al vuelo. Mutaciones de alto nivel via comandos con intencion (`replace`, `explode`, `invalidate`).
4. **Supuestos como entidades de primer nivel** (ADR-005): viven dentro del edge con ID propio (`ASM-001`), son direccionables para invalidacion.

## Contrato de Salida JSON

Todo comando retorna (JSON por defecto, `--human` para legible):

```json
{
  "success": bool,
  "action": "string",
  "workspace": "string",
  "data": { ... },
  "graph_health": { "valid_dag": bool, "orphan_nodes_count": int },
  "errors": [{"code": "...", "detail": "...", ...context}],
  "warnings": [{"code": "...", "detail": "...", ...context}]
}
```

## Fuentes de Verdad

| Documento | Contenido |
|-----------|-----------|
| [ENGINE_SPEC.md](ENGINE_SPEC.md) | Especificacion completa de comandos, esquemas JSON, contrato de salida |
| [PLAN.md](PLAN.md) | Plan de desarrollo por fases con UATs y decisiones tecnicas |
| [PROGRESS.md](PROGRESS.md) | Dashboard de avance, estimaciones y telemetria |
| [ADR.md](ADR.md) | Registro de Decisiones de Arquitectura (12 ADRs) |
| [CLR_SPEC.md](CLR_SPEC.md) | Las 8 Categorias de Legitimacion Logica y matriz de auditoria |
| [KNOWLEDGE_SPEC.md](KNOWLEDGE_SPEC.md) | Knowledge Pool: schema, comandos, interacciones, workflow epistemico |

## Estructura del Workspace generado por el CLI

```
mi-proyecto-ltp/
├── ltp.config.json       # Config global
├── nodes/                # Pool de nodos atomicos (UDE-001.json)
├── trees/                # Vistas topologicas (tree-crt-logistica.json)
├── knowledge/            # Pool de knowledge items (KN-001.json)
└── .ltp/                 # Estado interno del motor (en .gitignore)
    ├── lock              # Lock de concurrencia (PID + timestamp)
    ├── counters.json     # Contadores secuenciales por tipo
    ├── undo/             # Stack de undo (snapshots + checksums)
    ├── redo/             # Stack de redo
    └── tmp/              # Escritura atomica temporal
```

## Reglas de Desarrollo

- Metodologia RPI: Research -> Plan (`.claude/plans/`) -> Implement.
- Verificacion obligatoria antes de marcar como completado: `cargo check` + `clippy` + `test`.
- Progreso registrado en `PROGRESS.md` tras cada paquete de trabajo.
- Prohibido `.unwrap()`/`.expect()` en codigo de produccion.
- Documentacion `///` en todos los items publicos.

## Six Thinking Hats — Revision Interna para Rust

Aplicar esta revision mentalmente en refactorizaciones o tareas de arquitectura complejas antes de generar el codigo final.

| Sombrero | Enfoque en Rust |
|----------|----------------|
| **Blanco (Datos)** | Revision estricta de errores de `rustc`, warnings de `cargo clippy` y coherencia de firmas de tipos. Solo hechos: que dice el compilador, que dice el linter. |
| **Rojo (Intuicion)** | Ergonomia de la API y legibilidad del codigo. Si una firma se siente incomoda de usar o un bloque cuesta leer en 5 segundos, es senal de rediseno. |
| **Negro (Critico — Obligatorio)** | Deteccion de `.clone()` injustificados, `Arc<Mutex<T>>` como parche al borrow checker, asignaciones innecesarias en el heap (`Box` donde basta un stack value), y cualquier workaround que esquive ownership en vez de resolverlo. |
| **Amarillo (Beneficios)** | Rendimiento, mantenibilidad a largo plazo y abstracciones de coste cero (`impl Trait`, generics, newtypes). Evaluar que se gana con la solucion propuesta. |
| **Verde (Alternativas)** | Buscar soluciones idiomaticas basadas en lifetimes, referencias y sistema de tipos antes de duplicar memoria. Preferir `&str` sobre `String`, iteradores sobre colecciones intermedias, enums sobre flags booleanos. |
| **Azul (Proceso)** | Coherencia con la arquitectura de `ltp-engine`: enfoque Type-First (definir structs/enums/traits antes de logica), determinismo (BTreeMap, IDs secuenciales), y alineacion con ADRs vigentes. |

El Sombrero Negro es obligatorio en toda revision. Los demas se activan segun la complejidad de la tarea.

## Git Workflow

- **Commit tras cada hito**: al completar un paso significativo (plan creado, sub-modulo funcional, UATs pasando, fase completa), hacer commit inmediatamente. No acumular trabajo sin commitear.
- **Push al completar cada fase**: tras terminar una fase y actualizar PROGRESS.md, hacer `git push` para sincronizar con origin.
- **Convencion de mensajes**: `docs(FX):` para planes/docs, `feat(FX):` para implementacion, `test(FX):` para tests, `fix(FX):` para correcciones.
- **Branch strategy**: trabajo directo en `main` (single developer). Si una fase es grande, crear branch `feat/fX-nombre` y merge al completar.
