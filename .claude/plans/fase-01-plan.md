# Plan Ejecutable — Fase 1: Fundacion

## Objetivo

Implementar la infraestructura base: `ltp init`, `ltp status`, trait `Storage` con implementacion filesystem, trait `SnapshotHook` (no-op), lock file, config, contadores de IDs, y pipeline de escritura atomica.

## Entregables

1. Trait `Storage` + implementacion `FsStorage`
2. Trait `SnapshotHook` + implementacion `NoOpHook`
3. Modulo de ID generation con auto-reconstruccion
4. Comando `ltp init` funcional
5. Comando `ltp status` funcional
6. Lock file con deteccion de stale
7. 6 UATs pasando como tests de integracion CLI

## Tareas Ordenadas

### T1: Definir trait Storage (`src/storage.rs`)

```rust
pub trait Storage {
    fn load_config(&self) -> Result<WorkspaceConfig>;
    fn save_config(&self, config: &WorkspaceConfig) -> Result<()>;
    fn load_node(&self, id: &str) -> Result<Node>;
    fn save_node(&self, node: &Node) -> Result<()>;
    fn delete_node(&self, id: &str) -> Result<()>;
    fn list_node_ids(&self) -> Result<Vec<String>>;
    fn load_tree(&self, id: &str) -> Result<Tree>;
    fn save_tree(&self, tree: &Tree) -> Result<()>;
    fn delete_tree(&self, id: &str) -> Result<()>;
    fn list_tree_ids(&self) -> Result<Vec<String>>;
    fn acquire_lock(&self, command: &str) -> Result<()>;
    fn release_lock(&self) -> Result<()>;
    fn next_id(&self, entity_type: &str) -> Result<String>;
    fn workspace_exists(&self) -> bool;
    fn workspace_name(&self) -> Result<String>;
}
```

### T2: Implementar `FsStorage` (`src/workspace/fs_storage.rs`)

- Recibe `root: PathBuf` (raiz del workspace).
- `load_*` / `save_*`: lee/escribe JSON con serde + BTreeMap para orden canonico.
- `save_*` usa pipeline atomico: write to `.ltp/tmp/<file>` -> rename to destino.
- `acquire_lock` / `release_lock`: implementacion del lock file con PID check.
- `next_id`: lee `.ltp/counters.json`, incrementa, escribe, retorna ID formateado.
- `workspace_exists`: verifica que `ltp.config.json` existe.

### T3: Definir trait SnapshotHook (`src/history/hook.rs`)

```rust
pub trait SnapshotHook {
    fn before_mutation(&self, affected_files: &[&Path]) -> Result<()>;
    fn after_mutation(&self, affected_files: &[&Path]) -> Result<()>;
}

pub struct NoOpHook;
impl SnapshotHook for NoOpHook {
    fn before_mutation(&self, _: &[&Path]) -> Result<()> { Ok(()) }
    fn after_mutation(&self, _: &[&Path]) -> Result<()> { Ok(()) }
}
```

### T4: Modulo de contadores (`src/workspace/counters.rs`)

- Struct `Counters` con `BTreeMap<String, u64>`.
- `load(path)`: lee fichero, si falla o no existe -> `rebuild()`.
- `rebuild(root)`: escanea `nodes/` y `trees/`, parsea IDs, extrae max por tipo.
- `next(entity_type) -> String`: incrementa contador, formatea con padding 3 digitos.
- Emite warning `COUNTERS_REBUILT` si se reconstruye.

### T5: Implementar comando `ltp init`

Logica:
1. Verificar que no existe `ltp.config.json` (sino: error `WORKSPACE_ALREADY_EXISTS`).
2. Crear directorios: `nodes/`, `trees/`, `.ltp/`, `.ltp/undo/`, `.ltp/redo/`, `.ltp/tmp/`.
3. Escribir `ltp.config.json` con nombre y config por defecto.
4. Escribir `.ltp/counters.json` con todos los tipos a 0.
5. Crear `.gitignore` con `.ltp/` si no existe.
6. Retornar output JSON con `success: true`.

### T6: Implementar comando `ltp status`

Logica:
1. Verificar que workspace existe (sino: error `WORKSPACE_NOT_INITIALIZED`).
2. Contar ficheros en `nodes/` y `trees/`.
3. Para cada tree: cargar y ejecutar `check_dag` sobre edges.
4. Contar nodos huerfanos (en pool pero sin edges en ningun tree).
5. Contar feedback loops.
6. Retornar output JSON con data + graph_health.

### T7: Conectar comandos al CLI (`src/main.rs`)

- Instanciar `FsStorage` con el directorio actual.
- Dispatch `Commands::Init` y `Commands::Status` a la logica.
- Formatear output: JSON por defecto, human-readable con `--human`.
- Respetar `--dry-run` (en init: verificar que se puede crear pero no escribir).

### T8: Tests de integracion CLI (`tests/fase_01.rs`)

Implementar los 6 UATs como tests que:
1. Crean un `tempdir`.
2. Invocan el binario `ltp` via `Command::new(env!("CARGO_BIN_EXE_ltp-engine"))`.
3. Parsean stdout como JSON.
4. Verifican assertions sobre el JSON y el estado del filesystem.

UATs:
- **1.1**: `ltp init --name "Test"` -> success, directorios creados, counters.json existe.
- **1.2**: `ltp init` repetido -> error code `WORKSPACE_ALREADY_EXISTS`.
- **1.3**: `ltp status` en workspace vacio -> 0 nodos, 0 trees, valid_dag true.
- **1.4**: Lock concurrente -> un proceso falla con `WORKSPACE_LOCKED`.
- **1.5**: Lock stale (PID muerto) -> auto-break con warning `STALE_LOCK_REMOVED`.
- **1.6**: Verificar `counters.json` existe tras init con todos los tipos a 0.

## Criterios de Aceptacion

- [ ] `cargo check` pasa.
- [ ] `cargo clippy -- -D warnings` pasa.
- [ ] `cargo test` pasa (3 tests existentes + 6 nuevos UATs).
- [ ] `ltp init --name Test` en un directorio vacio produce la estructura correcta.
- [ ] `ltp status` en un workspace inicializado retorna JSON valido.
- [ ] No hay `.unwrap()` ni `.expect()` en codigo de produccion.
- [ ] Todos los items publicos tienen `///` doc comments.

## Archivos a Crear/Modificar

| Archivo | Accion |
|---------|--------|
| `src/storage.rs` | Crear — trait Storage |
| `src/workspace/fs_storage.rs` | Crear — implementacion filesystem |
| `src/workspace/counters.rs` | Crear — gestion de contadores |
| `src/workspace/mod.rs` | Modificar — re-exportar nuevos modulos |
| `src/history/hook.rs` | Crear — trait SnapshotHook + NoOpHook |
| `src/history/mod.rs` | Modificar — re-exportar hook |
| `src/main.rs` | Modificar — conectar init/status al storage |
| `src/lib.rs` | Modificar — exportar storage |
| `src/output.rs` | Sin cambios (ya listo) |
| `src/errors.rs` | Sin cambios (ya listo) |
| `tests/fase_01.rs` | Crear — 6 UATs de integracion |

## Dependencias Externas

Ninguna nueva. Todo cubierto por las crates actuales en Cargo.toml.

## Riesgos

- El hook PostToolUse de `.claude/settings.json` ejecutara `cargo fmt && cargo check` tras cada edicion. Si el codigo esta en estado intermedio (compilacion rota), el hook fallara. Solucion: implementar en orden de dependencias (traits primero, impl despues, main al final).
