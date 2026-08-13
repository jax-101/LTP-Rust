# Plan Fase 11: Historial (Undo/Redo)

## Fuentes consultadas
- ENGINE_SPEC.md §2.13 (Historial — Undo/Redo)
- ENGINE_SPEC.md §3.2-3.3 (Formato undo/redo entries)
- ADR-009 (Undo/Redo por Snapshot con Validacion de Checksums)
- PLAN.md (Fase 11: UATs 11.1–11.18)
- Codigo existente: `src/history/{mod.rs, types.rs, hook.rs}`

## Estado Actual

Ya existen:
- `src/history/types.rs`: `UndoEntry`, `RedoEntry`, `FileSnapshot`, `RedoFileSnapshot`
- `src/history/hook.rs`: trait `SnapshotHook` + `NoOpHook`
- `src/workspace/config.rs`: `HistoryConfig { max_size_mb, enabled }`
- `src/errors.rs`: `UndoStateDiverged`, `RedoStateDiverged`
- `src/main.rs`: CLI ya tiene `Commands::Undo`, `Commands::Redo`, `Commands::History` con subcomandos (stub `NOT_IMPLEMENTED`)
- `.ltp/undo/` y `.ltp/redo/` se crean en `init_workspace`

## Diseno

### Componente Central: `HistoryManager`

Un struct que encapsula la logica de historial, operando sobre el filesystem via paths relativos al workspace root.

```rust
pub struct HistoryManager {
    root: PathBuf,          // workspace root
    undo_dir: PathBuf,      // .ltp/undo/
    redo_dir: PathBuf,      // .ltp/redo/
    tmp_dir: PathBuf,       // .ltp/tmp/
    config: HistoryConfig,
}
```

### Flujo de Captura (SnapshotHook activado)

1. Antes de mutacion: leer contenido actual de ficheros afectados.
2. Despues de mutacion: calcular SHA-256 del nuevo contenido.
3. Construir `UndoEntry` con seq autoincremental y guardar en `.ltp/undo/{seq:03}.json`.
4. Vaciar `.ltp/redo/` (operacion mutante nueva invalida redo stack).
5. Aplicar rotacion si se supera `max_size_mb`.

### Flujo de Undo

1. Cargar entry con seq mas alto del undo stack.
2. Para cada fichero en `affected_files`:
   - Leer contenido actual → calcular SHA-256.
   - Comparar con `after_hash`. Si diverge → error `UNDO_STATE_DIVERGED`.
3. Si --dry-run: reportar sin ejecutar.
4. Restaurar `before` para cada fichero (write-then-rename via tmp).
   - Si `before: null` → eliminar fichero (fue creacion).
5. Construir `RedoEntry` (inverso: `before_hash` = hash del estado pre-undo, `after` = contenido actual pre-undo).
6. Mover entry de undo a redo stack.

### Flujo de Redo

1. Cargar entry con seq mas alto del redo stack.
2. Para cada fichero en `affected_files`:
   - Leer contenido actual → calcular SHA-256.
   - Comparar con `before_hash`. Si diverge → `REDO_STATE_DIVERGED`.
3. Si --dry-run: reportar sin ejecutar.
4. Escribir `after` para cada fichero.
   - Si `after: null` → eliminar fichero.
5. Mover entry de redo a undo stack (restaurar la UndoEntry original).

### Batch

- `begin-batch`: Crear fichero `.ltp/batch_active.json` con `{ label, timestamp, seq_start }`.
  - Si ya existe → error `BATCH_ALREADY_IN_PROGRESS`.
  - Tomar snapshot completo del workspace en ese momento.
- Durante batch: los comandos mutantes NO generan entries individuales.
- `end-batch`: Construir una sola UndoEntry con el snapshot del begin (antes de toda la batch).
  - Eliminar `.ltp/batch_active.json`.
  - Si no existe → error `NO_BATCH_IN_PROGRESS`.

### History List/Check/Invalidate/Clear

- `history [--last N]`: Listar entries del undo stack (seq, timestamp, action, command).
- `history check`: Para cada entry, verificar `after_hash` contra disco actual. Reportar valid/diverged.
- `history invalidate --from <seq>`: Eliminar entries con seq >= N.
- `history clear`: Eliminar todos los ficheros de undo/ y redo/.

### Rotacion FIFO

Al guardar una nueva entry, calcular tamano total de `.ltp/undo/`. Si > `max_size_mb * 1MB`, eliminar entries mas antiguas hasta bajar del limite.

## Pasos de Implementacion

### Paso 1: `HistoryManager` — tipos y constructor
- Archivo: `src/history/manager.rs`
- Struct `HistoryManager` + constructor desde root + config.
- Helpers: `next_seq()`, `undo_entry_path(seq)`, `redo_entry_path(seq)`, `list_undo_entries()`, `list_redo_entries()`.
- Helper: `sha256_hash(content: &str) -> String`.
- Helper: `total_dir_size(dir: &Path) -> u64`.
- Helper: `rotate_if_needed()`.

### Paso 2: Captura — `ActiveSnapshotHook`
- Archivo: `src/history/hook.rs` (extender)
- Implementar `ActiveSnapshotHook` que recibe un `HistoryManager` y una `command: String`.
- `before_mutation`: lee ficheros, almacena en memoria.
- `after_mutation`: calcula hashes, escribe UndoEntry, vacia redo, rota.
- El hook necesita un mecanismo para que el caller informe los paths afectados Y el action/command string.

**Alternativa mas simple (elegida)**: En lugar de usar el trait `SnapshotHook` tal cual, integrar la captura directamente en las funciones `execute_*` existentes via `HistoryManager`. El manager expone:
```rust
fn record_mutation(&self, action: &str, command: &str, affected_files: &[&Path]) -> Result<()>
```
Pero esto requiere capturar ANTES y DESPUES. Mejor API:
```rust
fn begin_capture(&self, affected_paths: &[PathBuf]) -> Result<CaptureContext>;
fn commit_capture(&self, ctx: CaptureContext, action: &str, command: &str) -> Result<()>;
```

Donde `CaptureContext` contiene los `before` snapshots. Esto se invoca desde una funcion wrapper en `main.rs` o directamente en cada comando.

**Decision**: Implementar un wrapper `with_history` que encapsula lock + snapshot + ejecucion + commit para comandos mutantes. Esto minimiza cambios en las funciones `execute_*` existentes.

### Paso 3: Undo/Redo — logica core
- `HistoryManager::execute_undo(dry_run: bool) -> Result<UndoResult>`
- `HistoryManager::execute_redo(dry_run: bool) -> Result<RedoResult>`
- Validacion de checksums, restauracion atomica, movimiento entre stacks.

### Paso 4: History subcommands
- `HistoryManager::list_history(last: Option<usize>) -> Vec<HistoryEntry>`
- `HistoryManager::check_integrity() -> Vec<CheckResult>`
- `HistoryManager::invalidate_from(seq: u64) -> usize`
- `HistoryManager::clear() -> Result<()>`

### Paso 5: Batch
- `HistoryManager::begin_batch(label: &str) -> Result<()>`
- `HistoryManager::end_batch() -> Result<()>`
- `HistoryManager::is_batch_active() -> bool`
- Modificar captura para no escribir entries individuales si batch activa.

### Paso 6: Integracion con CLI (main.rs)
- Implementar handlers para `Commands::Undo`, `Commands::Redo`, `Commands::History`.
- Integrar captura de historial en comandos mutantes existentes.
  - Estrategia: wrapper function que calcula paths afectados pre/post ejecucion.
  - Los comandos mutantes son: node add/edit/rm/split, tree new/rm/attach/detach/clone, link connect/disconnect/feedback/reverse/move/insert-between/group/dissolve/split/reoperator/add-cause/rm-cause, assume add/edit/rm/move, invalidate, path collapse/explode/replace, nbr add/rm.

### Paso 7: Tests de integracion (UATs 11.1–11.18)
- Archivo: `tests/fase_11.rs`
- Patron CLI integration tests (run binary, parse JSON).

## Orden de Ejecucion

1. Paso 1 (HistoryManager core)
2. Paso 3 (Undo/Redo logic) — necesita manager
3. Paso 4 (History subcommands) — necesita manager
4. Paso 5 (Batch) — necesita manager
5. Paso 2 (Captura/hook activo) — conecta todo
6. Paso 6 (CLI integration) — conecta main.rs
7. Paso 7 (Tests UAT) — valida todo

## Mapeo UATs → Funcionalidad

| UAT | Funcionalidad |
|-----|---------------|
| 11.1-11.2 | Undo/Redo basico |
| 11.3 | --dry-run en undo |
| 11.4 | Deteccion de divergencia (UNDO_STATE_DIVERGED) |
| 11.5 | history --last N |
| 11.6 | history check |
| 11.7 | begin-batch / end-batch |
| 11.8 | Undo de invalidate (por snapshot, automatico) |
| 11.9 | Operacion post-undo vacia redo |
| 11.10 | Rotacion FIFO (config `max_size_mb` bajo en test) |
| 11.11 | history invalidate --from |
| 11.12 | history clear |
| 11.13 | Undo con stack vacio |
| 11.14 | Redo con stack vacio |
| 11.15 | end-batch sin begin-batch |
| 11.16 | begin-batch anidado |
| 11.17 | Undo de rm restaura nodo completo |
| 11.18 | Undo de link group restaura edges originales |
| 11.19 | `redo --dry-run` muestra sin ejecutar |
| 11.20 | Redo con divergencia externa → REDO_STATE_DIVERGED |
| 11.21 | Undo de creacion (node add) → fichero eliminado |
| 11.22 | Rotacion con threshold bajo verifica FIFO (reemplaza impl de 11.10) |

## Notas de Diseno

- **Paths relativos**: todas las rutas en entries son relativas al workspace root (portabilidad).
- **Determinismo**: seq es secuencial, ficheros nombrados `{seq:03}.json`.
- **Atomicidad**: write-then-rename via `.ltp/tmp/` para prevenir corrupcion.
- **No se modifica el trait Storage**: el HistoryManager opera directamente sobre filesystem, usando los paths del workspace. Esto evita contaminar la interfaz Storage con concerns de historial.
- **Lock**: undo/redo adquieren lock como cualquier comando mutante.
