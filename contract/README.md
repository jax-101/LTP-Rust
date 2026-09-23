# `contract/` — Golden fixtures del `CommandOutput`

Estos `*.json` son **ejemplos reales y siempre-verdaderos** del contrato de salida
del motor: cada uno es el `CommandOutput` canónico que el binario `ltp` emite para
un comando concreto sobre un workspace-fixture determinista. Son a la vez
**documentación ejecutable** (un consumidor —o su IA— puede leerlos como muestras
del shape exacto) y **red de seguridad** (el snapshot test los regenera y compara).

No son un JSON Schema ni tipos compartidos entre repos: son ejemplos + comparación.
Fijan **forma**, no semántica (ADR-001: la verdad lógica es del agente LLM).

## Cómo se generan y verifican

El test `tests/contract.rs` construye un workspace temporal ejecutando una secuencia
fija de comandos (`build_fixture`) y captura el stdout del **binario real**. Como los
IDs son secuenciales por tipo (ADR-001) y los tree-ids derivan del slug del nombre,
la secuencia es byte-estable.

```bash
# Verificar (parte del gate `cargo test --workspace`):
cargo test --test contract

# Regenerar tras un cambio de contrato intencional:
UPDATE_GOLDEN=1 cargo test --test contract
```

Campos volátiles normalizados antes de comparar (ver `redact_volatile`):

- `workspace` → `"<WORKSPACE>"`.
- claves de timestamp (`created_at`, `updated_at`, `timestamp`) → `"<TIMESTAMP>"`
  (defensivo; el conjunto inicial no las expone).

## Flujo de cambio de contrato

1. Cambias el shape de salida a propósito.
2. `UPDATE_GOLDEN=1 cargo test --test contract` reescribe los goldens.
3. **Revisas el diff a ojo** — es el registro del cambio de contrato.
4. Decides el incremento SemVer (RELEASE_POLICY §1: cambiar shape = MAJOR;
   campo nuevo aditivo = MINOR) y actualizas `CHANGELOG.md` + `INTEGRATION.md`.

Si el diff aparece **sin** que lo esperaras → regresión de contrato cazada antes de
llegar al consumidor.

## Casos actuales

| Golden | Comando | Qué fija |
|--------|---------|----------|
| `tree_walk.json` | `tree walk <crt>` | nodos `{id, role, incoming_edges[], outgoing_edges[]}`; **sin** `feedback_edges` |
| `tree_walk_knowledge.json` | `tree walk <crt> --show-knowledge` | campo opcional `knowledge{supports,contradicts,contextualizes}` |
| `tree_list.json` | `tree list` | `tree_type` (no `type`), enums minúscula, `logic`, `node_count`/`edge_count` |
| `validate.json` | `validate` | `graph_health` + warnings CLR anidados en `data.details[]` (contexto aplanado: `edge_id`, `node_id`) |
| `error_flattened_context.json` | `macro add` con `--from X --to X` (self-loop) | `errors[]` = `{code, detail, ...contexto aplanado}` (aquí `node_id` al nivel raíz); `success:false` |

> El shape de `warnings[]`/`errors[]` (`OutputWarning`/`OutputError`: `code`, `detail`
> y contexto **aplanado** al nivel raíz) es idéntico a nivel superior o anidado; los
> goldens `validate` y `error_circular_dependency` lo fijan por ambas vías.

El conjunto crece **bajo demanda**: al añadir un golden de un comando que exponga
timestamps u otros campos volátiles, amplía `redact_volatile` en `tests/contract.rs`.

## Nota: por qué el error golden es un self-loop y no un ciclo

El caso de error demuestra el contexto aplanado con `macro add --from X --to X`
(`RESERVATION_SELF_LOOP`, `node_id` al nivel raíz), no con un ciclo
(`CIRCULAR_DEPENDENCY_DETECTED`). Razón: el `cycle_path` que reporta la detección
de ciclos (DFS) **rota su nodo de inicio entre ejecuciones idénticas** (mismo ciclo,
distinta rotación) — su *forma* es estable (array de strings) pero su *contenido* no
es determinista, así que no sirve como golden byte-estable. El self-loop tiene
contenido fijo y demuestra el mismo contrato de `errors[]` aplanado.

