# PLAN v0.5.2 (PATCH) — La reconstrucción de contadores no subestima nunca

## Contexto

Al investigar la Parte B de `PLAN_post-v051.md` (2026-10-09, binario de `46a69a9`) apareció un bug más grave que los huecos de la Parte B: **pérdida de datos silenciosa**. Decisión del usuario: sale ya como **v0.5.2 PATCH**, junto con el refactor de la Parte A (que así no viaja en una release vacía). Lo MINOR sigue en `PLAN_v060.md`.

### Datos (medidos)

Si `.ltp/counters.json` falta o está corrupto, `Counters::load` llama a `Counters::rebuild` (`src/workspace/counters.rs:48`), que escanea `nodes/`, `trees/` y `knowledge/`. El escaneo **se salta en silencio** lo que no puede leer (`Err(_) => return` en `read_dir`, `entries.flatten()`, `let Ok(content) = … else { continue }`). Un contador subestimado vuelve a emitir un ID que ya existe.

| Caso | Resultado |
|---|---|
| E2: `counters.json` ausente y `trees/tree-crt-a.json` en `chmod 000`; `link connect` en `tree-crt-b` | `success: true`, 0 warnings, **`LINK-001` duplicado** (en `a` y en `b`) |
| E5: `counters.json` ausente y `nodes/` en `0300` (`-wx`: se escribe, no se lista); `node add "NUEVO"` | `success: true`, `id: UDE-001`, 0 warnings: **sobrescribe `UDE-001`** ("efecto ORIGINAL" → "NUEVO") |
| E2b: `counters.json` en `chmod 000` | `ID_GENERATION_ERROR` (fail-closed por accidente: `save` falla) |

Escenario realista: `.ltp/` está en `.gitignore`, así que un clon nuevo **siempre** reconstruye. Basta un fichero ilegible (permisos, un directorio con nombre `.json`, un fichero a medio sincronizar) para que se reutilicen IDs.

Además, `next_id` descarta el warning `COUNTERS_REBUILT` (`fs_storage.rs:231`, `let (mut counters, _)`). Exponerlo es MINOR (warning nuevo en el output): va en `PLAN_v060.md`, no aquí.

### Principio

**Sobrestimar un contador es inocuo; subestimarlo destruye.** ADR-009 ya acepta huecos en los IDs (no retroceden), así que un contador demasiado alto solo deja un hueco. Por eso:
- Lo que **no se puede leer** → la reconstrucción falla (fail-closed). Es D-K5 aplicado a los contadores: `next_id` alimenta una escritura, y lo ilegible que una mutación escribe, bloquea.
- Lo que **se lee pero no se puede parsear** (JSON corrupto) → escaneo textual de todo lo que tenga forma `PREFIX-NNN`. Sobrestima, y eso es seguro. Así se mantiene U3 (`tests/counters_rebuild.rs:318`, "corrupt tree file is skipped without panic"), pero ahora además no puede subestimar.

### Contrato

PATCH (RELEASE_POLICY §1): sin códigos, campos ni flags nuevos. Un fallo de la reconstrucción sale como `ID_GENERATION_ERROR`, el código que ya emiten los 33 sitios que llaman a `next_id`, con su significado de siempre ("no se pudo generar el ID"). Un workspace sano no cambia de comportamiento.

---

## Tareas

| T | Qué | Verificación |
|---|---|---|
| T0 | **Tests primero** (`tests/v052_counters_rebuild.rs`), en rojo antes de T1. **R1** (determinista): `counters.json` ausente + `trees/tree-crt-a.json` sustituido por un **directorio** (EISDIR) → `link connect` en `b` da `ID_GENERATION_ERROR`, y el workspace queda intacto byte a byte (helper de `v051_no_expect`). **R2** (`cfg(unix)`, se salta si `uid == 0`): E5 con `nodes/` en `0300` → `ID_GENERATION_ERROR` y `UDE-001` intacto. **R3**: E2 con `chmod 000` en el árbol. **R4**: `knowledge/` ausente (se crea bajo demanda) → la reconstrucción **sigue funcionando** (ausente ≠ ilegible). **R5** (unit): árbol con JSON corrupto que contiene `"LINK-007"` → contador `LINK >= 7`. | R1–R3 y R5 en rojo, R4 en verde sobre `main` |
| T1 | `Counters::rebuild` y `Counters::load` devuelven `Result<(Self, Vec<OutputWarning>)>`. `scan_directory` y `scan_tree_contents` devuelven `Result<()>`: `read_dir` con `ErrorKind::NotFound` → `Ok` (directorio ausente = vacío); cualquier otro error, un `entry` en error o un `read_to_string` fallido → `Err(LtpError::Io)`. JSON no parseable → `observe_text(&str)`, un escáner de bytes sin regex que llama a `observe` para cada token `[A-Z]+-[0-9]+`. `next_id` y `load_counters` propagan con `?`. | Sin `unwrap`/`expect`/`clone` nuevos. U1–U4 de `counters_rebuild.rs` siguen en verde |
| T2 | Mutaciones al estilo de T4 (v0.5.1), revertidas a mano sobre el árbol de trabajo (no con `git checkout`, que se llevaría el cambio sin commitear): (M1) `read_dir` en error → `Ok` → muere R2; (M2) `read_to_string` en error → `continue` → mueren R1 y R3; (M3) quitar `observe_text` → muere R5; (M4) tratar `NotFound` como error → muere R4. | 4/4 detectadas |
| T3 | Las 4 verificaciones | Verde |
| T4 | Docs: adenda a **ADR-009** (sobrestimar es seguro, subestimar no; la reconstrucción es fail-closed ante lo ilegible). ENGINE_SPEC §3.1: un párrafo sobre la reconstrucción de `counters.json` (hoy solo está en PLAN.md:94). CHANGELOG `[0.5.2]`: `Fixed` (contadores) + `Changed` (refactor de la Parte A, que ya está en `[Unreleased]`). INTEGRATION §4: entrada v0.5.2 (gate `>= 0.5.2` recomendado si se clona un workspace con `.ltp/` ignorado). PROGRESS: entrada de release, dashboard, contador de tests. | — |
| T5 | Release según RELEASE_POLICY §5: `Cargo.toml` → `0.5.2`, commit `chore(release): v0.5.2`, tag anotado, `git push --follow-tags`. | `ltp --version` = `0.5.2+<sha>` sin `dirty` |

Commits: `test(v0.5.2)` (T0), `fix(v0.5.2)` (T1), `docs(v0.5.2)` (T2 + T4), `chore(release)` (T5).

## Fuera de alcance (registrado)

- `COUNTERS_REBUILT` sigue sin salir al output: es MINOR → `PLAN_v060.md`.
- Escritura con `create_new` (O_EXCL) al crear entidades, como segunda defensa frente a IDs repetidos. Con T1 no hace falta para cerrar E2/E5; se puede valorar en v0.6.0.
- Todo lo de "ilegible ≠ ausente" en lectores (`exists()`, `*_NOT_FOUND` falsos, `load_pool`) → `PLAN_v060.md`.
