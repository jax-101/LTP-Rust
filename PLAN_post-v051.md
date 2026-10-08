# PLAN — Después de v0.5.1: búsqueda única de la NBR (ya) y "ilegible ≠ ausente" (v0.6.0)

## Contexto

PROGRESS (v0.5.1, "Fuera de alcance") dejó dos huecos abiertos. Se investigaron el 2026-10-08 con mediciones sobre el binario de `9706710` y se revisaron con Six Hats. Decisiones del usuario:

- **Hueco 2** (`let-else` de `link connect --nbr`): refactor sin cambio de contrato. Entra en `main` **sin tag propio** y viaja en la siguiente release. Una v0.5.2 con solo esto sería una release vacía para el consumidor.
- **Hueco 1** (`load_pool` se calla si falla `list_knowledge_ids`): necesita un warning nuevo, así que va en **v0.6.0 (MINOR)** con la opción A + D. Se agrupa con el hueco sistémico "ilegible = ausente" de `fs_storage` y con el `dry_run` de MCP ya previsto.
- **Descartado como bug**: que `knowledge add` queme el contador `KN` cuando falla con `IO_ERROR`. Es el patrón de todo el motor (`node add` igual: `node/commands.rs:328` → `:368`). Un fallo de escritura después de reservar el ID quema el contador, y ADR-009 lo acepta (los IDs no retroceden). Comprobar antes de escribir dejaría una ventana de carrera (TOCTOU). Solo se registra en PROGRESS.

---

## Parte A — Hueco 2: buscar la NBR una sola vez (en `main`, sin tag)

### Datos

- `src/link/commands.rs`: la rama se busca dos veces. Primero en la validación temprana (`:211`, `any`), antes de reservar IDs. Después en el `let-else` (`:346`), tras `next_id("LINK")` (`:274`/`:308`).
- Sonda T2b (v0.5.1): si se quita la validación temprana, el `let-else` quema un `LINK` y los 3 E2E de `tests/v051_no_expect.rs` fallan. El `let-else` protege mal.
- Medido: un ciclo dentro de una NBR quema `LINK` (1 → 2) con `CIRCULAR_DEPENDENCY_DETECTED`. Está documentado (ADR-013/ADR-009) y **queda fuera de alcance**. No hay que venderlo como "ya no se queman contadores".
- CLI (`main.rs:1407`) y MCP (`mcp/dispatch.rs:671`) llaman a la misma función.
- **Prototipo validado** en un worktree desechable: `cargo check` + `clippy -D warnings` limpios y **751/751 tests en verde**. Mueve el cálculo de `edge_logic` antes de la búsqueda (es puro: solo depende de `nbr_id.is_some()` y `tree.logic`), busca la rama una vez con `iter_mut().find` y guarda la referencia `&mut NbrBranch` hasta la inserción. Son 20 líneas añadidas y 18 quitadas, en un solo fichero.

### Tareas

| T | Qué | Verificación |
|---|---|---|
| A1 | Aplicar el refactor con estas dos mejoras sobre el prototipo: (a) un enum de destino `Trunk` / `Nbr(&mut NbrBranch)` en vez de `Option<&mut NbrBranch>`; (b) tomar `nid` de `nbr_id`, no de `nbr_branch.id`. | Orden de errores intacto: `LOCK_ERROR` → `TREE_NOT_FOUND` → errores de nodo → `NBR_NOT_FOUND` → `INVALID_OPERATOR`. Sin `clone()` nuevo |
| A2 | Mutación: buscar la rama **después** de reservar IDs | Debe detectarla `v051_no_expect` (compara `.ltp/counters.json`). Revertir con `git checkout` |
| A3 | Las 4 verificaciones (`check`, `clippy -D warnings`, `test --workspace`, `fmt --check`) | Verde |
| A4 | PROGRESS: cerrar el hueco 2 y dejar la sonda T2b sin efecto (ya no hay dos guardias). Registrar el adyacente "KN quemado en `IO_ERROR`" como comportamiento de ADR-009. CHANGELOG `[Unreleased]`: refactor interno. | — |
| A5 | Commit `refactor(link): …` y push. **Sin tag.** | — |

---

## Parte B — v0.6.0 (MINOR): "ilegible ≠ ausente"

> Esto **no es todavía un plan implementable**. Recoge lo investigado y lo que falta investigar. Antes de implementar hay que redactar `PLAN_v060.md` (RPI), consultando ENGINE_SPEC, ADR, KNOWLEDGE_SPEC y RELEASE_POLICY.

### B1. Hueco 1 — `load_pool` sin listado

**Medido** con `knowledge/` en `chmod 000` o sustituido por un fichero (ENOTDIR). El fixture tiene un nodo `fact` UDE-001 y KN-001 con `supports` hacia él:

| Comando | Pool legible | Listado fallando |
|---|---|---|
| `status` | `knowledge_health.total: 1` | `total: 0`, 0 warnings |
| `validate` | sin avisos | **`EPISTEMIC_UNGROUNDED` falso** sobre UDE-001 |
| `node rm UDE-001` | `KNOWLEDGE_ORPHANED [KN-001]` | `success: true`, 0 warnings: referencia colgante invisible |
| `knowledge list` | — | `IO_ERROR` (correcto) |

- Punto de fallo único: `src/knowledge/pool.rs:35`, `list_knowledge_ids().unwrap_or_default()`. Lo usan 6 consumidores: `status` (CLI `main.rs:989` y MCP `mcp/dispatch.rs:389`), `validate`, `node rm`, `trace` y `tree walk` con `--show-knowledge`. Todos reenvían ya `pool.warnings`.
- Nota para los tests: los avisos de `validate` van anidados en `details[_knowledge_pool]`, no en los `warnings` del nivel superior.

**Opción elegida: A + D.**
- **A**: `KnowledgePool` lleva un `enum PoolScope { Complete, Unlisted }`, no un `bool`, para que el compilador obligue a cada consumidor a decidir. Se añade un warning nuevo (nombre provisional `KNOWLEDGE_POOL_UNREADABLE`). Cada consumidor:
  - `validate` suprime los `EPISTEMIC_*` que dependen de que algo **falte** (`UNGROUNDED`, `UPGRADEABLE`). Avisar sin suprimirlos seguiría siendo mentir.
  - `node rm` avisa de que no pudo calcular `KNOWLEDGE_ORPHANED`, sin bloquear (D-K5: `rm` no escribe knowledge).
  - `status` mantiene los números a 0 con el aviso. Devolver `null` cambiaría la forma del JSON, y eso sería MAJOR.
- **D**: una entrada mala no tumba el listado entero (`entry?` en `fs_storage.rs:313`).
- Descartadas: reusar `KNOWLEDGE_LOAD_ERROR` (le cambiaría el significado: MAJOR), usar `IO_ERROR` como warning (sería, de hecho, un código de warning nuevo) y hacer que los lectores fallen (contradice D-K5).

### B2. Sistémico — `exists()` confunde ilegible con ausente

- `path.exists()` devuelve `false` cuando el error es de permisos. Hay 14 llamadas en `src/workspace/fs_storage.rs`: `load_node` (`:98`), knowledge (`:284`, `:301`), árboles, config (`:238`) y **lock (`:191`, `:223`)**.
- Medido: `knowledge inspect KN-001` con `knowledge/` en `chmod 000` responde `KNOWLEDGE_NOT_FOUND`, que es falso.
- **Hipótesis sin medir (prioridad alta en la investigación de v0.6.0):** con `.ltp/` ilegible, `lock_path.exists()` da `false` y el motor podría adquirir el lock aunque otro proceso lo tenga.
- Dirección: usar `try_exists()` o comprobar `ErrorKind::NotFound`. Pasar de `*_NOT_FOUND` a `IO_ERROR` en esos casos va en una MINOR con nota de migración, porque roza "cambiar el significado de un código" (RELEASE_POLICY).

### B3. `dry_run` en MCP

Ya estaba previsto para v0.6.0 (PROGRESS v0.5.1, §5). Reutilizaría el mecanismo de ADR-017 D-1. Si v0.6.0 se hace demasiado grande, puede ir en un paquete separado.

### Documentación prevista para v0.6.0

Adenda D-K6 a ADR-016; KNOWLEDGE_SPEC §318; ENGINE_SPEC (orden de warnings de `rm`, `status`, `validate`, `walk` y `trace`); INTEGRATION (gate `>= 0.6.0` y nota de migración `*_NOT_FOUND` → `IO_ERROR`); snapshot nuevo en `contract/`. Tests: ENOTDIR como caso determinista y `chmod 000` con guardia para `uid == 0`. Mutaciones al estilo de T4: quitar el aviso y no suprimir `EPISTEMIC_UNGROUNDED`.
