# Plan: Tests End-to-End (E2E) — 19 UATs

## Objetivo

Implementar 19 UATs de workflows completos (13 originales + 6 Six Hats). Validan cohesión cross-feature con flujos reales del LTP.

## Archivo destino

`tests/e2e.rs`

## Helpers necesarios

Reutilizados de fases previas: `ltp_bin`, `run_ltp`, `setup_workspace`, `add_node`, `create_tree`, `attach_node`, `connect_nodes`.

Nuevos para E2E: `assume_add`, `invalidate`, `undo`, `redo`, `validate`, `trace`, `nbr_add`, `nbr_connect`, `path_collapse`, `path_replace`, `begin_batch`, `end_batch`, `tree_clone`, `tree_diff`, `link_add_cause`, `link_reoperator`, `link_disconnect`, `node_edit`, `node_rm`, `history_check`, `history_invalidate`, `link_inspect`.

## UATs (19 total)

### Bloque 1: Flujos base (E2E.1–E2E.5)
- E2E.1: CRT completo (init → build → validate → status)
- E2E.2: Insuficiencia → corrección (validate warning → fix → validate clean)
- E2E.3: Invalidación completa (assume → invalidate → undo roundtrip)
- E2E.4: EC validation (roles correctos, reglas EC pasan)
- E2E.5: CRT→EC→FRT cycle (multi-tree, nodos compartidos, NBR)

### Bloque 2: Undo/History (E2E.6, E2E.15, E2E.18)
- E2E.6: Batch undo (begin-batch → N ops → end-batch → undo atómico)
- E2E.15: Undo cross-tree cascade (node rm multi-tree → undo restaura todo)
- E2E.18: History divergence recovery (edición manual → check → invalidate)

### Bloque 3: Navegación (E2E.7, E2E.8, E2E.11, E2E.14)
- E2E.7: Clone + diff
- E2E.8: Trace depth (8 niveles, depth 3 vs sin límite)
- E2E.11: Invalidate + trace lifecycle
- E2E.14: Agent simulation (navegación intercalada con mutaciones)

### Bloque 4: Abstracción y NBR (E2E.12, E2E.13, E2E.17)
- E2E.12: Path collapse + validate (no falsos positivos)
- E2E.13: NBR + invalidate + undo (aislamiento trunk/NBR)
- E2E.17: Path replace + undo roundtrip

### Bloque 5: Cross-cutting (E2E.9, E2E.10, E2E.16, E2E.19)
- E2E.9: Nodo compartido multi-tree (edit visible en ambos)
- E2E.10: Counters recovery
- E2E.16: EC incremental construction (error → fix → error → fix → clean)
- E2E.19: Multi-warning iterative fix (CLR#4 + CLR#6 + CLR#7 → fix secuencial)

## Verificación

```bash
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace
cargo fmt --all -- --check
```
