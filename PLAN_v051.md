# PLAN — v0.5.1 (PATCH): el motor no calla — `--dry-run` real y knowledge ilegible visible

## Contexto

Este PATCH reúne dos fallos con la misma raíz: el motor da por completa una información que no lo es.
- (1) `--dry-run` escribe aunque promete simular (§1–§2).
- (2) Un knowledge ilegible se descarta en silencio en seis sitios (§1b).

Se añaden además los huecos de tests del Knowledge Pool que destapó la auditoría del 2026-10-08.

### (1) `--dry-run`

`--dry-run` es un flag global del CLI (`src/main.rs:64`, "Simulate the operation without writing to disk"), y INTEGRATION §2A promete que "simula la operación sin escribir a disco". El código solo lo consulta en `init`, `undo` y `redo`. Se comprobó el 2026-10-08: en un workspace nuevo, `ltp node add "Prueba" --type UDE --dry-run` devuelve `success: true` y escribe `nodes/UDE-001.json`. Cualquier consumidor del CLI que lo use para previsualizar modifica el workspace sin saberlo. El riesgo es mayor desde v0.5.0, porque lo natural es previsualizar un `node rm` o un `node split` destructivos.

La revisión Six Hats (2026-10-08) descartó dos alternativas:
- (A) Rechazar el flag: arregla la mentira, pero deja sin previsualización justo los comandos destructivos.
- (B) Un `Storage` simulado en memoria: es una segunda implementación de la persistencia que puede divergir de la real.

El usuario eligió la **opción C**: ejecutar el comando real sobre una copia temporal del workspace y descartarla.

Datos de la investigación (graph-first + lectura de código):
- **Por qué un proceso hijo:** `main` termina con `process::exit` en más de 70 brazos, y `process::exit` se salta cualquier `Drop`. Una copia limpiada por un guard dentro del mismo proceso dejaría basura en cada fallo. Por eso el padre relanza el **propio binario** (`std::env::current_exe()`) sobre la copia y limpia siempre al terminar.
- **El campo `workspace` es el nombre de la configuración** (`Storage::workspace_name`), no una ruta. El output del hijo es idéntico al de una ejecución real sin reescribir nada.
- **Qué escribe y dónde:** fuera de `Storage` solo escriben el lock (`.ltp/lock`), los contadores (`.ltp/counters.json`) y el historial (`.ltp/undo|redo`), y todo vive bajo `.ltp/`. El historial está limitado (`max_size_mb`, 5 MB por defecto).
- **MCP:** no tiene `dry_run`, así que la UI no está afectada.

## 1. Decisiones

| D | Decisión | Justificación (Negro obligatorio) |
|---|---|---|
| **D-1** Mecanismo | Si llega `--dry-run` y el comando no es `init`, `undo` ni `redo`: (1) copiar el workspace a un directorio temporal; (2) relanzar el binario con `current_dir` = la copia, los mismos argumentos sin el token `--dry-run` y la variable `LTP_DRY_RUN_CHILD=1`; (3) reenviar stdout y stderr byte a byte; (4) borrar la copia; (5) salir con el código del hijo. | Fidelidad por construcción: se ejecuta el mismo binario y el mismo código, así que la simulación no puede divergir de la ejecución real. Cuesta lanzar un proceso y copiar unos KB o MB de JSON, y en LTP es despreciable. |
| **D-2** Qué se copia | Solo lo que gestiona LTP: `ltp.config.json`, `nodes/`, `trees/`, `knowledge/` y `.ltp/`, todo salvo `.ltp/tmp/`. Se copian solo ficheros regulares; los symlinks no se siguen. El resto del `cwd` (repos, documentos del usuario) no se copia. | El workspace es el `cwd` y puede contener ficheros ajenos y grandes. Copiar `.ltp/` entero (lock, contadores, historial, estado de batch) hace que el hijo vea exactamente el mismo estado: con un lock activo da `WORKSPACE_LOCKED`; con uno obsoleto, el aviso de lock obsoleto; dentro de un batch, no captura historial. |
| **D-3** Cero bytes en el workspace real | En modo simulación el padre **no adquiere el lock real** (adquirirlo ya es escribir) ni crea nada en el workspace. Todo lo que escriba el hijo, incluido su historial, cae en la copia. | Es la promesa del flag. Hay una carrera con otro proceso que escriba durante la copia: es la misma ventana que ya tiene hoy un comando de solo lectura, y una mutación concurrente viva hará que el hijo vea su lock y falle igual que la ejecución real. Se documenta. |
| **D-4** Output | Idéntico al de la ejecución real: sin campos, warnings ni códigos nuevos. `--human` y el código de salida se respetan. | Añadir `dry_run: true` o un warning sería contrato nuevo (MINOR, RELEASE_POLICY §1). La versión se queda en PATCH: hace cumplir una promesa ya documentada. Quien pasa el flag ya sabe que simula. |
| **D-5** Excepciones | `init`, `undo` y `redo` conservan su `--dry-run` nativo, ya probado (UAT 11.3). Los comandos de solo lectura también pasan por la copia: es inofensivo, uniforme y evita mantener una lista de qué comandos mutan. | Una lista blanca de "comandos que mutan" sería otra fuente de verdad que se desfasaría con cada comando nuevo. |
| **D-6** Fallos del mecanismo | Si no se puede crear o copiar el directorio temporal, o lanzar el hijo, la salida es un `CommandOutput` JSON con `IO_ERROR` (código existente), `success: false` y código 1, **sin ejecutar nada sobre el workspace real**. La copia se borra también cuando el hijo falla o muere. | Fail-closed: nunca degradar a una ejecución real. `IO_ERROR` ya existe, así que no hay contrato nuevo. |
| **D-7** Recursión | Si `LTP_DRY_RUN_CHILD` está activa y llega `--dry-run` de todos modos, no se vuelve a lanzar nada: es un error `IO_ERROR` interno. | Defensa contra un bucle de procesos si quitar el token fallara. |
| **D-8** Deuda incluida | (a) Quitar los dos `.expect()` de producción: `src/output.rs:102` (`to_json`, con un JSON de error de respaldo) y `src/link/commands.rs:345` (pasa a `ok_or` → error). (b) `#![warn(missing_docs)]` en `lib.rs` con `#[allow(missing_docs)]` explícito en los 15 módulos con deuda; se exigen ya `meta`, `macro_edge` y `macro_assume`, que tienen 0 elementos sin documentar. | El objetivo es que no crezca la deuda (hoy son 655 elementos), no documentarla ahora. Si se activara sin los `allow`, `-D warnings` rompería el gate. |

## 1b. Decisiones — knowledge ilegible (Six Hats 2026-10-08)

Hechos de la auditoría (verificados en el código): estos seis sitios hacen `.filter_map(|id| load_knowledge(id).ok())` y descartan en silencio un `knowledge/KN-xxx.json` ilegible:
- `status` en el CLI (`src/main.rs:1039`);
- `status` en el MCP (`src/mcp/dispatch.rs:394`), una implementación **duplicada**;
- `validate` (`src/validate/knowledge.rs:28`);
- `tree walk --show-knowledge` (`src/tree/commands.rs:1268`);
- `trace --show-knowledge` (`src/trace/mod.rs:424`);
- `node rm` (`src/node/commands.rs:1244`), donde se pierde `KNOWLEDGE_ORPHANED`.

`knowledge list` ya emite `KNOWLEDGE_LOAD_ERROR {id}` y continúa.

| D | Decisión | Justificación (Negro obligatorio) |
|---|---|---|
| **D-K1** Código | Se reutiliza `KNOWLEDGE_LOAD_ERROR` con la forma de `list`: `detail` "Failed to load {id}: {e}" y contexto `id`, un aviso por item ilegible, en orden de `list_knowledge_ids`. Sin código nuevo ⇒ PATCH. | RELEASE_POLICY §1: un código existente con el mismo significado es un bugfix. Cambiar la clave de contexto a `knowledge_id` sería MAJOR. |
| **D-K2** Dónde | `status` (CLI y MCP), `validate` (en `data.details` de la entrada sintética `_knowledge_pool` si existe, o en la misma que hoy usan los avisos epistémicos; se decide leyendo el código en T-K1 y se fija con el UAT), y `tree walk`/`trace` **solo con `--show-knowledge`**. | Un aviso en vistas que no han pedido knowledge sería ruido. |
| **D-K3** `node rm` | **Avisa y no bloquea**: si un item no se puede leer, se emite `KNOWLEDGE_LOAD_ERROR` (no se sabe si apuntaba al nodo borrado), después de `KNOWLEDGE_ORPHANED`. | `rm` **no escribe** knowledge: no lo deja inconsistente, solo pierde precisión en el aviso. Fail-closed (ADR-016 D-4) se reserva para lo que la mutación escribe. Si bloqueara, un solo KN corrupto paralizaría todo `node rm`. |
| **D-K4** Una sola fuente | El cálculo de `knowledge_health` pasa a una función de librería (`pub fn`, documentada) que llaman el CLI y el MCP. | La duplicación actual es la que permitió que el fallo existiera en dos sitios. Mismo principio que los `execute_*` compartidos. |
| **D-K5** Regla (adenda a ADR-016) | *Lo ilegible que la mutación escribe, bloquea; lo ilegible que solo lee, se avisa; nunca se calla.* | Generaliza D-4 sin contradecirlo. |

## 2. Tipos y funciones (type-first) — `src/dry_run.rs` (módulo nuevo del binario, documentado)

```rust
/// Copia temporal del workspace para `--dry-run`; se borra al soltarse (el padre nunca hace `process::exit` con ella viva).
struct DryRunCopy { root: PathBuf }
impl DryRunCopy {
    /// Copia lo gestionado por LTP (D-2) a `temp_dir()/ltp-dry-run-<pid>-<n>`.
    fn create(workspace: &Path) -> io::Result<Self>;
}
impl Drop for DryRunCopy { fn drop(&mut self); } // remove_dir_all, errores ignorados

/// Argumentos del hijo: los originales sin el token `--dry-run` (solo la primera aparición global).
fn child_args(args: &[OsString]) -> Vec<OsString>;

/// Ejecuta el hijo sobre la copia y devuelve su `Output` (stdout, stderr, status).
fn run_child(copy: &DryRunCopy, args: &[OsString]) -> io::Result<Output>;

/// Punto de entrada desde `main` antes del `match`: `Some(exit_code)` si atendió el dry-run.
pub fn intercept(cli: &Cli, cwd: &Path, human: bool) -> Option<i32>;
```

Sin `.clone()` de estructuras del dominio: se copian ficheros. `child_args` trabaja sobre `OsString` para no perder argumentos que no sean UTF-8.

## 3. Tareas (TDD: cada tarea escribe primero sus UATs y los ve en rojo)

| T | Contenido | Commit |
|---|---|---|
| **T0** | Este plan + **ADR-017** "Semántica de `--dry-run`: ejecución real sobre una copia descartable" (D-1..D-7) + adenda D-K5 a ADR-016. | `docs(v0.5.1): plan + ADR-017` |
| **T-K1** | D-K1..D-K4: `KNOWLEDGE_LOAD_ERROR` en los seis sitios + `knowledge_health` compartido. UATs KL1–KL7 (§4.0). | `fix(v0.5.1): knowledge ilegible visible` |
| **T-K2** | Solo tests: los 14 UATs del Knowledge Pool sin test o con aserción incompleta (§4.0b), y quitar los `if let Some(... "_knowledge_pool")` de `k5_15`, `k5_32`, `k5_34` y `k5_44`. `k5_32` debe comprobar además que el nodo del árbol sí aparece. Se investiga si `depth: null` en `trace` y el `--depth 1` de K5.40 son fallos; si lo son, **se para y se consulta**, sin arreglarlos dentro de T-K2. | `test(v0.5.1): cobertura Knowledge Pool` |
| **T1** | `src/dry_run.rs` + `intercept` en `main`. UATs DR1–DR12 (§4). | `fix(v0.5.1): --dry-run no escribe en disco` |
| **T2** | D-8a: los dos `.expect()`. Unit para `to_json` de respaldo; regresión del error de `link connect --nbr`. | `fix(v0.5.1): sin expect en producción` |
| **T3** | D-8b: `missing_docs` en modo aviso con `allow` por módulo con deuda. | `chore(v0.5.1): lint missing_docs` |
| **T4** | Mutation checks (§4.2) ejecutados de verdad, revertidos y registrados en PROGRESS. | — (resultado en PROGRESS) |
| **T5** | Docs + release: ENGINE_SPEC (§ flags globales: semántica de `--dry-run` y excepciones; `KNOWLEDGE_LOAD_ERROR` en `status`/`validate`/`walk`/`trace`/`node rm`), KNOWLEDGE_SPEC (tratamiento de lo ilegible), INTEGRATION §2A (precisar la promesa y la carrera de D-3), USAGE_GUIDE (previsualizar `rm`/`split`), CHANGELOG `[0.5.1]` en *Fixed*, README, `Cargo.toml` 0.5.1, PROGRESS (dashboard, cerrar el hueco "`--dry-run` en mutaciones", registrar §5), tag `v0.5.1`, push. | `chore(release): v0.5.1` |

## 4.0 UATs knowledge ilegible (`tests/v051_knowledge_unreadable.rs`)

Fixture: workspace con KN-001..KN-003 válidos y KN-004 corrupto (`{broken`), enlazado a UDE-001.
- **KL1 `status`** (CLI): `KNOWLEDGE_LOAD_ERROR {id: KN-004}`, `knowledge_health.total` = 3, `success: true`.
- **KL2 `status` en MCP**: el mismo aviso y los mismos conteos que el CLI (paridad).
- **KL3 `validate`**: el aviso aparece, `success` no cambia por él, y los avisos epistémicos de los items válidos siguen saliendo.
- **KL4 `tree walk` / `trace`**: con `--show-knowledge`, el aviso aparece; sin el flag, no hay aviso y el output es idéntico al de v0.5.0.
- **KL5 `node rm UDE-001`**: el nodo se borra, salen `KNOWLEDGE_ORPHANED` (por los válidos que apunten a UDE-001) y `KNOWLEDGE_LOAD_ERROR {id: KN-004}` en ese orden, y KN-004 queda byte-idéntico.
- **KL6 Varios ilegibles**: KN-004 y KN-006 dan dos avisos en orden de ID.
- **KL7 Regresión**: sin items ilegibles, el output de los seis comandos es idéntico al de v0.5.0 (sin avisos nuevos).

## 4.0b UATs del Knowledge Pool a completar (T-K2, solo tests)

K3.12, K3.13 (con `invalidate` real), K3.14, K3.17 (`FB-xxx`), K3.21 (con `inspect`), K3.23 (link a un LINK interior + `inspect`), K5.7 (vía `assume rm`), K5.16, K5.19, K5.22, K5.25, K5.33 (dos árboles), K5.38 y K5.49 (vía `feedback-rm`). K5.40 se fija con un test una vez decidida su semántica. El comportamiento de cada uno se verificó con el binario en la auditoría (BEHAVIOR_OK).

## 4. UATs `--dry-run` (`tests/v051_dry_run.rs`, adversariales)

Helper `fingerprint(dir)`: SHA-256 de cada fichero bajo el workspace, **incluido `.ltp/`** y los ficheros ajenos, más la lista de rutas. Fixture `rich_workspace()` con nodos, dos árboles, edges, feedback, una NBR, una macro Overlay con `macro_assume`, una Reservation, supuestos y knowledge vinculado.

### 4.1 Funcionales

- **DR1 — Tabla de mutaciones.** Una fila por cada mutación del CLI (node add/edit/rm/split, tree new/rename/rm/attach/detach/clone, link connect/disconnect/feedback/reverse/move/insert-between/group/dissolve/split/reoperator/add-cause/rm-cause, assume add/edit/rm/move, macro-assume add/rm/gather, macro add/expand/promote, path collapse/explode/replace, invalidate, nbr add/rm, knowledge add/edit/rm/link/unlink). Para cada una: `fingerprint` antes = después de `--dry-run`, y el stdout del dry-run es **byte-idéntico** al de la ejecución real sobre una copia gemela del fixture. Lo único que se normaliza es `captured` (fecha) de knowledge, por si el test cruza la medianoche.
- **DR2 — Avisos destructivos visibles.** `node rm` de un extremo de macro con `--dry-run` muestra `MACRO_EDGE_REMOVED` y `affected_trees`, y el workspace no cambia. `node split --dry-run` muestra los `affected_trees` reales.
- **DR3 — Una simulación que falla tampoco escribe.** Ciclo en `link connect`, `NODE_NOT_FOUND` en `rm` y árbol ilegible en `split`: el error JSON es igual al real, el código de salida es 1 y `fingerprint` no cambia.
- **DR4 — Los contadores reales no se consumen.** `node add --dry-run` ×3 y después `node add` real ⇒ el ID real es `UDE-001`.
- **DR5 — Sin historial.** Tras un `--dry-run`, `history list` es igual y `undo` deshace la última mutación real, no la simulada.
- **DR6 — Lock activo.** Con un `.ltp/lock` del PID del propio test (vivo), `--dry-run` da el mismo `WORKSPACE_LOCKED` que la ejecución real, y el fichero de lock sigue igual byte a byte.
- **DR7 — Lock obsoleto.** Con un lock de un PID muerto, el dry-run muestra el aviso de lock obsoleto, pero **el lock real sigue en disco** (la simulación no repara nada).
- **DR8 — Dentro de un batch.** `history begin-batch` real, después una mutación con `--dry-run` ⇒ no cambia nada, y `end-batch` real agrupa solo las mutaciones reales.
- **DR9 — Limpieza.** Con `TMPDIR` apuntando a un directorio propio del test: tras éxito, tras error y con `--human`, no queda ningún `ltp-dry-run-*`.
- **DR10 — Ficheros ajenos.** Un fichero grande ajeno en el `cwd` (p. ej. `notes.bin` de 5 MB) no se copia (no aparece en `TMPDIR` durante la ejecución, comprobable vía D-2 en un unit test de `DryRunCopy`) y sigue intacto.
- **DR11 — Excepciones intactas.** `undo --dry-run`, `redo --dry-run` e `init --dry-run` se comportan como en v0.5.0 (UAT 11.3 sigue en verde), y `--dry-run` en un comando de solo lectura (`tree list`, `validate`) da un output idéntico.
- **DR12 — Workspace sin inicializar.** `--dry-run` fuera de un workspace da el mismo error que la ejecución real y no crea nada en el `cwd`.

Unit (`src/dry_run.rs`): `child_args` (el flag en cualquier posición, aparece una sola vez, conserva el resto y los `OsString`); `DryRunCopy` copia solo D-2, no sigue symlinks y se borra en `Drop`; recursión D-7.

### 4.2 Mutation checks (cada uno debe tumbar al menos su UAT)

| Mutación | Debe fallar |
|---|---|
| `KNOWLEDGE_LOAD_ERROR` eliminado de `status` (CLI o MCP) | KL1 o KL2 |
| `rm` bloquea ante un KN ilegible | KL5 |
| Aviso también sin `--show-knowledge` | KL4, KL7 |
| `k5_32` vuelve a `if let` (solo T-K2) | — (se verifica a mano que hoy falla al quitar la sección) |
| `intercept` devuelve `None` siempre (vuelve el fallo) | DR1, DR2, DR4 |
| No excluir `.ltp/` de la copia (el hijo crea historial y lock vírgenes) | DR6, DR7, DR8 |
| Copiar con `rename` o mover en vez de copiar | DR1 (`fingerprint`) |
| No borrar la copia (sin `Drop`) | DR9 |
| Copiar el `cwd` entero | DR10 |
| Adquirir el lock real antes de copiar | DR6 (lock cambiado), DR1 |
| Quitar todos los `--dry-run` (también el propio de `undo`) | DR11 |
| Degradar a ejecución real si falla la copia | unit D-6 |
| Mezclar stderr en stdout | DR1 (byte-idéntico) |

## 5. Fuera de alcance, registrado (PROGRESS)

- `dry_run` en MCP (los 72 tools): es contrato nuevo, va como MINOR (v0.6.0) y reutiliza el mecanismo de D-1.
- Documentar los 655 elementos públicos sin `///` (la deuda queda congelada por T3, no saldada).
- Dividir `main.rs` (2373 líneas).
- Carrera de D-3 con escritores concurrentes durante la copia (la misma ventana que una lectura de hoy).
- Los heredados de v0.5.0 (§6 de `PLAN_v050-integrity.md`).

## 6. Verificación (en cada commit con código)

`cargo check --all-targets --all-features` → `cargo clippy --all-targets --all-features -- -D warnings` → `cargo test --workspace` → `cargo fmt --all -- --check`. codebase-memory: `detect_changes` antes de cada tarea y reindexación si hay deriva.
