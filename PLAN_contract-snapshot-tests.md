# PLAN — Contrato de salida verificable por máquina (golden fixtures + snapshot test)

> Estado: **propuesto** (RPI, fase Plan). Pendiente de implementación.
> Scope: motor (`ltp-engine`). El lado consumidor (UI Tauri) se describe pero **no** se implementa aquí (es otro repo).

## 1. Problema

Hoy el contrato de salida (`CommandOutput` JSON) vive en **prosa**: `ENGINE_SPEC.md`, `INTEGRATION.md`, `CLAUDE.md`. La prosa no se compila. Esta misma sesión encontramos, a mano, tres derivas que ningún test cazaba:

- `ENGINE_SPEC.md` afirmaba que `tree walk` incluye `feedback_edges` → **falso** (`TreeWalkData` solo tiene `tree_id/order/nodes`).
- El shape de `tree list` (`tree_type`, no `type`; enums en minúscula) solo estaba garantizado por lectura manual del código.
- `errors[]` con contexto **aplanado** al nivel raíz (no anidado bajo `context`) — el detalle de mayor valor para el parser de la UI — no estaba documentado en el sitio del consumidor.

Un cambio de shape hoy rompe al consumidor **en runtime, en silencio, delante del usuario**. No hay red que salte antes.

## 2. Objetivo y no-objetivos

**Objetivo**: convertir el contrato de salida de prosa a **fuente de verdad verificada por máquina**: capturar el output real del binario para un conjunto representativo de comandos como *golden files* versionados, y un test que los regenera y compara. Cualquier cambio de shape rompe `cargo test` en rojo con diff exacto, forzando una decisión consciente de versión (RELEASE_POLICY §1/§5).

**No-objetivos** (Sombrero Negro — qué NO es esto):

- **No** es un JSON Schema ni tipos compartidos entre repos. Eso acoplaría el motor y la UI y añadiría peso. Son **ejemplos reales + comparación textual**.
- **No** testea la semántica (eso es del agente LLM, ADR-001). Testea **forma**, no verdad lógica.
- **No** cubre todos los comandos desde el día 1. Empieza por los shapes que la UI ya consume y crece bajo demanda.
- **No** toca el repo de la UI. Su contract test es responsabilidad suya (ver §7).

## 3. Diseño

### 3.1 Golden fixtures en `contract/`

Directorio nuevo `contract/` en la raíz (máxima descubribilidad: el consumidor —o su Claude— puede leerlos directamente como ejemplos siempre-verdaderos). Contiene:

- `contract/README.md` — qué son estos ficheros y cómo se regeneran.
- `contract/*.json` — un `CommandOutput` canónico por comando/caso representativo.

### 3.2 Workspace-fixture determinista

Un helper `build_fixture(dir)` en el test construye el workspace ejecutando una **secuencia fija** de comandos `ltp` (patrón `run_ltp` ya existente en `tests/e2e.rs`). Como los IDs son **secuenciales por tipo** (invariante de determinismo, ADR-001), re-ejecutar la secuencia produce IDs idénticos ⇒ goldens byte-estables. La secuencia mínima:

1. `init` un workspace.
2. Añadir un puñado de nodos (`node add ...`) de tipos variados → `UDE-001`, `UDE-002`, ...
3. Crear un CRT y conectar edges (`tree new`, `link connect ...`).
4. Añadir un knowledge item y enlazarlo (`knowledge add`, `knowledge link`) → para el caso `--show-knowledge`.

### 3.3 Snapshot test `tests/contract.rs`

Spawnea el **binario real** (`env!("CARGO_BIN_EXE_ltp")` + `current_dir(dir)`), captura stdout, y compara con el golden. Se spawnea el binario —no se llaman funciones de librería— para snapshotar **exactamente lo que la UI ve** (arg parsing, resolución de `cwd`, stdout). Reutiliza el patrón de `tests/e2e.rs`.

**Conjunto inicial de casos** (crecer bajo demanda):

| Golden | Comando | Qué garantiza |
|--------|---------|---------------|
| `tree_walk.json` | `tree walk <crtId>` | nodos `{id, role, incoming_edges[], outgoing_edges[]}`; **sin** `feedback_edges` |
| `tree_walk_knowledge.json` | `tree walk <crtId> --show-knowledge` | campo opcional `knowledge{supports,contradicts,contextualizes}` presente |
| `tree_list.json` | `tree list` | `tree_type` (no `type`), enums minúscula, `node_count`/`edge_count` |
| `error_flattened_context.json` | un comando que falla con contexto (p. ej. `tree walk <idInexistente>` o `link connect` sobre nodo fuera del árbol) | `errors[]` = objetos `{code, detail, ...contexto aplanado}` |
| `warning_clr.json` | un comando que emite warning CLR no bloqueante | `warnings[]` shape + no-bloqueo |
| `validate.json` (opcional) | `validate` | `graph_health` + shape de validación |

### 3.4 Normalización de campos volátiles (Sombrero Blanco/Negro)

El snapshot **no** puede ser un `diff` crudo: hay campos no deterministas que romperían el test sin que cambie el contrato.

- **`workspace` (nivel raíz)**: es la ruta absoluta al temp dir del test → redactar a `"<WORKSPACE>"` antes de comparar.
- **Timestamps ISO 8601** (chrono): los comandos elegidos para el conjunto inicial (`walk`, `list`, error, warning) **no** los exponen en su `data`. Si en el futuro se añade un golden de un comando que sí (p. ej. `node inspect` con `created_at`), redactar por nombre de clave a `"<TIMESTAMP>"`.
- **Provenance/SHA**: NO aparece en `CommandOutput` (solo en `--version`), así que no contamina goldens. ✅

La normalización se aplica **igual** al output vivo y (implícitamente) ya está en el golden, comparando JSON canónico normalizado contra golden.

### 3.5 Regeneración intencional

`UPDATE_GOLDEN=1 cargo test --test contract` reescribe los goldens. Es el punto de fricción deliberado: cuando cambias el contrato a propósito, regeneras, y el **diff del golden en el commit ES el registro del cambio de contrato** → tu señal para el bump SemVer.

## 4. Cómo encaja con RELEASE_POLICY

- Un shape cambia sin querer → contract test **rojo** → regresión cazada antes de shippear.
- Un shape cambia a propósito → regeneras golden → el diff aparece en el PR → decides el incremento (RELEASE_POLICY §1: cambiar shape de salida = **MAJOR**; campo nuevo aditivo = **MINOR**) y actualizas `CHANGELOG.md` + `INTEGRATION.md` (§5 ya obliga a esto último).
- El golden test se integra en el gate existente (`cargo test --workspace`), sin paso nuevo.

## 5. Six Thinking Hats (resumen)

- **Blanco**: patrón `run_ltp`/`CARGO_BIN_EXE_ltp` ya existe (`e2e.rs`, `version_provenance.rs`); no hay `contract/` ni `tests/fixtures/` aún; IDs secuenciales garantizan estabilidad.
- **Rojo**: goldens legibles a ojo = documentación que da confianza; regenerar con una variable de entorno es ergonómico.
- **Negro** (obligatorio): riesgo = **cobertura de muestras** (si eliges pocas o poco representativas, no cubres todo → mitigación: empezar por lo que la UI consume, crecer bajo demanda). Riesgo = **campos volátiles** → mitigado con normalización explícita (§3.4). NO introducir dependencia nueva pesada (`insta`, `assert_cmd`): basta `serde_json` + el patrón `run_ltp` existente.
- **Amarillo**: coste inicial modesto (1 fixture + N goldens + 1 test), aprovecha el determinismo que ya se pagó (BTreeMap, IDs secuenciales, JSON canónico); documentación ejecutable.
- **Verde**: preferir comparación de `serde_json::Value` normalizado sobre comparación de strings (robusto a orden de claves, aunque el JSON ya sea canónico); sin crates nuevas.
- **Azul**: coherente con ADR-001 (motor valida forma, no semántica) y con el principio "el binario es la fuente de verdad" (snapshotamos el output del binario real).

## 6. Plan de implementación (pasos)

1. Crear `contract/README.md` (qué son, cómo regenerar).
2. Crear `tests/contract.rs`: helper `build_fixture(dir)` + helper de normalización (`redact_volatile(value)`).
3. Implementar comparación golden ↔ output vivo, con branch `UPDATE_GOLDEN=1` que escribe en vez de comparar.
4. Generar el conjunto inicial de goldens (`UPDATE_GOLDEN=1`), **revisarlos a ojo** (que reflejen el contrato correcto), commitear.
5. Correr el gate completo. Ajustar.
6. (Doc) Añadir `contract/` al mapa de documentación de `README.md` y una nota en `INTEGRATION.md` §2A ("los shapes están fijados por goldens en `contract/`").
7. Commit por hito: `feat(contract): golden fixtures + snapshot test del CommandOutput`.

## 7. Lado UI (otro repo — solo referencia, NO se implementa aquí)

Responsabilidad del repo de la UI, documentado aquí para cerrar el círculo del protocolo inter-proyecto:

- Un contract test propio que spawnea el binario `ltp` real (vía `LTP_BINARY` o el sidecar) contra un workspace mínimo y assertea los shapes que **parsea** (p. ej. `data.nodes[].incoming_edges` es `string[]`, `errors[]` tiene contexto aplanado).
- Doble entrada: el golden del motor responde "¿cambié el contrato?" (productor); el contract test de la UI responde "¿el motor sigue dándome lo que parseo?" (consumidor). Ambos miran el mismo output real; ninguno duplica prosa. Repos ortogonales, coherentes por construcción.

## 8. Verificación (gate obligatorio antes de completar)

```bash
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace
cargo fmt --all -- --check
```

## 9. Decisiones abiertas

- Comando exacto para el caso de warning determinista (elegir uno que emita warning CLR no bloqueante de forma estable).
- Confirmar contra `--help`/`ENGINE_SPEC.md` la firma exacta de cada comando del conjunto inicial antes de codificar la secuencia del fixture.
