# Plan F10: NBR (Negative Branch Reservations)

## Estado Actual

- `NbrBranch` struct ya definida en `src/tree/types.rs` (id, source_node, edges, trim_injection)
- `nbr_branches: Vec<NbrBranch>` ya en `Tree`
- Validate ya chequea DAG de cada NBR (validate/mod.rs:111-120)
- Trace ya soporta `--nbr` flag (trace/mod.rs:233-235)
- CLI: `NbrAction` enum tiene Add/List/Inspect (falta Rm)
- `src/nbr/mod.rs` = placeholder vacío
- `link connect` NO soporta `--nbr` flag aún
- `Commands::Nbr` cae al catch-all `_ => NOT_IMPLEMENTED`

## Tareas

### T1: `nbr add` (UAT 10.1, 10.5, 10.7, 10.8)

Implementar en `src/nbr/mod.rs`:
- `execute_nbr_add(storage, tree_id, source_node, trim) -> CommandOutput<NbrAddData>`
- Validaciones: tree existe, source_node existe en pool, source_node attached al tree
- Genera `NBR-<SEQ>` via `storage.next_id("NBR")`
- Crea `NbrBranch { id, source_node, edges: vec![], trim_injection }`
- Push a `tree.nbr_branches`, save tree

### T2: `nbr rm` (UAT 10.10, 10.11) — ADR-010 Decisión 2

- `execute_nbr_rm(storage, tree_id, nbr_id) -> CommandOutput<NbrRmData>`
- Busca NBR en `tree.nbr_branches`, error `NBR_NOT_FOUND` si no existe
- Elimina la entrada. Nodos permanecen en pool. trim_injection no se toca.
- Añadir `Rm` al enum `NbrAction` en main.rs

### T3: `nbr list` (UAT 10.3)

- `execute_nbr_list(storage, tree_id) -> CommandOutput<NbrListData>`
- Retorna: id, source_node, edge_count, has_trim

### T4: `nbr inspect` (UAT 10.4)

- `execute_nbr_inspect(storage, tree_id, nbr_id) -> CommandOutput<NbrInspectData>`
- Retorna: edges de la NBR, nodos involucrados, trim_injection

### T5: Extender `link connect` con `--nbr` (UAT 10.2, 10.9)

- Añadir `#[arg(long)] nbr: Option<String>` a `LinkAction::Connect` en main.rs
- Si `--nbr` presente: buscar la NBR, validar nodos en pool (error `REFERENTIAL_INTEGRITY_VIOLATION` si no), crear edge dentro de `nbr.edges` en vez de `tree.edges`
- DAG check sobre `nbr.edges` antes de guardar

### T6: Wiring en main.rs

- Importar funciones de `ltp_engine::nbr`
- Conectar NbrAction::Add/Rm/List/Inspect a las funciones
- Pasar `nbr` arg a `execute_link_connect`

### T7: Tests de integración (12 UATs)

- Archivo: `tests/fase_10.rs`
- UATs 10.1-10.12 como tests de integración CLI

### T8: UATs expandidas (Six Hats)

| ID | Comando | Resultado esperado | Gap |
|----|---------|--------------------|-----|
| 10.13 | `link connect --nbr NBR-001` creando ciclo A→B→A | Error `CIRCULAR_DEPENDENCY_DETECTED` |
| 10.14 | `ltp nbr inspect NBR-999 --tree T` | Error `NBR_NOT_FOUND` |
| 10.15 | `ltp nbr add --tree T --source-node INJ-001 --trim NODO-999` | Error `NODE_NOT_FOUND` |
| 10.16 | `ltp nbr add --tree T --source-node INJ-001` (ya hay NBR con ese source) | Éxito: múltiples NBRs por source válido |
| 10.17 | `ltp trace INJ-001 --tree T --direction downstream --nbr` | Trace incluye edges de NBR branches |

Decisiones diferidas:
- `link disconnect --nbr` → extensión natural, no bloquea F10
- `tree detach` de source_node con NBR activa → warning/rechazo, diferido

## Orden de Ejecución

T1 → T2 → T3 → T4 → T5 → T6 → T7 → verify

## Verificación

```bash
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace
cargo fmt --all -- --check
```
