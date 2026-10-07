# REVIEW — v0.5.0 T1: decisiones implícitas a validar

> Contexto: T1 (`a5a5b2b`) implementó `src/meta/integrity.rs` según `PLAN_v050-integrity.md` §2. Hay 5 puntos que el plan no fijaba. Ya están implementados, pero **no hay que avanzar a T2 hasta revisar cada uno**.
>
> Cómo usar: revisa las decisiones una a una y marca el veredicto: ✅ aceptar · ✏️ cambiar (anota cómo) · ❓ discutir. Si una decisión cambia, se corrige en un commit `fix(v0.5.0):` antes de la revisión Six Hats de T2.

| # | Decisión | Afecta a | Veredicto |
|---|---|---|---|
| 1 | `RemovedMacro` lleva `from`/`to` + `MacroRemovalReason::as_str()` | T3 (warning) | ✅ con cambios |
| 2 | Regla de "sin duplicar" al sustituir hijos del split | T2 (split) | ☐ |
| 3 | `interior_emptied` solo si este `rm` recortó la macro | T3 (rm) | ☐ |
| 4 | Contexto de errores NBR: `nbr_id` (+ `edge_id`) | T5 (validate) | ☐ |
| 5 | `projection_refs` de `MacroAssumption` no se tocan en `rm` | T3 (rm), M3 | ☐ |

---

## D1 — `RemovedMacro` incluye `from` y `to`; `as_str()` en el motivo

**Qué dice el plan (§2):** `RemovedMacro { id, reason, assumption_ids }`. T3 pide el warning `MACRO_EDGE_REMOVED {tree_id, macro_link, reason, from, to, assumption_ids}`.

**Qué hice:** añadí `from: String` y `to: String` a `RemovedMacro` (`src/meta/integrity.rs:37`), y `MacroRemovalReason::as_str() -> &'static str` (`:27`).

**Por qué:**
- Sin `from`/`to`, T3 tendría que conservar una copia de la macro antes de podar (un `clone()` o un segundo recorrido) solo para construir el warning. `RemovedMacro::new` **mueve** los campos de la macro eliminada, así que no hay clones.
- `as_str()` evita `serde_json::to_value(reason)`, que es falible y obligaría a gestionar un `Result` imposible en producción.

**Alternativas:**
- (a) Ceñirse al plan y que T3 construya el warning con la macro antes de podarla. Mezcla E/S y lógica pura, en contra de la semilla `ltp-core`.
- (b) Solo `Serialize`, sin `as_str()`. Una fuente de verdad menos, pero con un `Result` en el camino de producción.

**Riesgo:** `as_str()` y serde podrían divergir si se añade una variante. Lo cubre el test `reason_wire_names_match_serde` (`:794`).

**Veredicto:** ✅ aceptado con cambios (revisión Six Hats, 2026-10-07):
- **N1:** una sola fuente de verdad. Se quita `derive(Serialize)` e `impl Serialize` delega en `as_str()`. El test fija los nombres de wire literales (`reason_wire_names_are_pinned`).
- **N2:** se añade `status: MacroEdgeStatus` a `RemovedMacro` y al warning `MACRO_EDGE_REMOVED` (aditivo, MINOR). Perder una `Reservation` es perder una intención top-down sin rastro en el grafo; perder un `Overlay` deja la cadena interior.
- **N3:** se quita el `sort()` de `assumption_ids`. El orden de almacenamiento ya es el de creación (contador `MASM` global y creciente, `push` al final, `rm` con `retain`), así que es numérico. El `sort()` textual ponía `MASM-1000` antes que `MASM-998`; lo reproduce `u4_…` con IDs a ambos lados de 999. M1 se reformula como "orden de almacenamiento". El orden textual de IDs en el resto del repo no se toca (determinista, fuera de alcance).
- **Para T3:** el warning necesita `status` como string. `MacroEdgeStatus` solo tiene `derive(Serialize)`; aplicar el mismo patrón de N1 o decidirlo en T3. También: ordenar los warnings explícitamente por `(tree_id, macro_link)` (M11), sin fiarse del orden del árbol.

---

## D2 — "Sin duplicar" al sustituir el original por `[first, second]`

**Qué dice el plan (D-2):** "el original se sustituye **en su posición** por `[first, second]` (sin duplicar si un hijo ya estuviera)". No dice *qué posición gana* si el hijo ya está.

**Qué hice** (`splice_children`, `:132`):
- Los hijos se insertan en la posición de la **primera** aparición del original.
- Si un hijo ya estaba en la lista, se queda solo en su **primera** posición (antes o después del original), y no se repite.
- Las apariciones repetidas del original (un estado ya corrupto) se eliminan todas.

| Entrada | Resultado |
|---|---|
| `nodes = [A, X, B]` | `[A, X1, X2, B]` (U2, caso normal) |
| `nodes = [X1, X, B]` | `[X1, X2, B]` (X1 conserva su posición y su `role`) |
| `interior_nodes = [X, X1]` | `[X1, X2]` (X1 se coloca donde estaba X; la copia posterior se descarta) |

**Por qué:** es determinista, nunca duplica IDs y, si el hijo ya estaba en `nodes[]`, conserva su `role`. En la práctica no ocurre, porque los hijos son IDs recién minteados. Es una garantía de la función pura, no un caso real del CLI.

**Alternativas:**
- (a) El hijo ya existente gana siempre su posición original, aunque esté después del original. `[X, X1]` daría `[X2, X1]`, con el orden de los hijos invertido.
- (b) Los hijos siempre van en la posición del original y se borran las copias previas. `[X1, X, B]` daría `[X1, X2, B]` igualmente, pero se perdería el `role` de X1.

**Test:** `u2b_children_not_duplicated` (`:492`).

**Veredicto:** ☐ ✅ ☐ ✏️ ☐ ❓ — Notas:

---

## D3 — `interior_emptied` solo cuando *este* `rm` recortó la macro

**Qué dice el plan (D-3c):** "Un `Overlay` cuyo `interior_links` queda vacío se elimina".

**Qué hice** (`:217`): el Overlay se elimina por `interior_emptied` solo si este `rm` ha quitado algo de su interior y, como consecuencia, `interior_links` queda vacío. Un Overlay que **ya tenía** `interior_links: []` antes del `rm` (un legacy raro o escrito a mano) no se toca.

**Por qué:** si no, borrar un nodo cualquiera del árbol eliminaría en silencio (con warning, pero sin relación causal) una macro que no tiene nada que ver con el nodo borrado. Eso es una transición no autorada, justo lo que ADR-016 quiere evitar. Ese estado inválido previo es asunto de `validate`, no de `rm`.

**Alternativas:**
- (a) Lectura literal: eliminar todo Overlay con `interior_links` vacío tras cualquier `rm`. Limpia estados inválidos, pero mezcla responsabilidades y produce warnings sorprendentes.

**Hueco de test:** no hay un unit test específico para "Overlay legacy vacío + `rm` no relacionado ⇒ intacto". Si se acepta, lo añado en T3, junto a M10 (macros legacy).

**Veredicto:** ☐ ✅ ☐ ✏️ ☐ ❓ — Notas:

---

## D4 — Contexto de errores en ramas NBR

**Qué dice el plan (D-5):** el contexto lleva siempre `tree_id`, `node_id`, `location` y "el ID del contenedor (`edge_id`/`feedback_id`/`nbr_id`/`macro_link`)". Con un único contenedor por caso.

**Qué hice** (`:317`, `:328`):

| Violación | `location` | Contenedor |
|---|---|---|
| `source_node` de una rama | `nbr_branches` | `nbr_id` |
| Nodo en un edge de una rama | `nbr_branches` | `nbr_id` **+ `edge_id`** |

**Por qué:** con solo `nbr_id`, el consumidor (LLM o UI) no sabe qué edge de la rama reparar. Con solo `edge_id`, se pierde que el edge vive en una rama y no en el tronco. Añadir claves de contexto es aditivo según RELEASE_POLICY §1.

**Alternativas:**
- (a) Solo `nbr_id` (lectura literal).
- (b) `location: "nbr_edges"` como valor distinto de `nbr_branches`. Más preciso, pero añade un valor al enum de `location` que el plan no contempla.

**Test:** `u6_one_violation_per_structure_in_fixed_order` (`:727`).

**Veredicto:** ☐ ✅ ☐ ✏️ ☐ ❓ — Notas:

---

## D5 — `projection_refs` de `MacroAssumption` no se tocan en `rm`

**Qué dice el plan:** M3 espera que, si el resumen de una macro superviviente proyectaba ASMs del link borrado, aparezca `LONG_ARROW_SUMMARY_STALE` con exactamente esos IDs en `dangling`. No dice si `rm` debe limpiar `projection_refs`.

**Qué hice:** `prune_removed` recorta `interior_nodes` e `interior_links`, pero **no** modifica `assumptions[].projection_refs`. Las referencias colgantes quedan para que las detecte el mecanismo que ya existe (`macro-assume gather` / `validate`, warning `LONG_ARROW_SUMMARY_STALE`).

**Por qué:**
- M3 solo es verificable si las refs siguen ahí. Si `rm` las limpiara, `dangling` saldría vacío y el analista no sabría que su resumen ha perdido sustento.
- Cumple el invariante 2 de CLAUDE.md: la semántica (si el resumen sigue siendo válido) es no bloqueante y corresponde al agente o analista. El motor solo avisa.

**Alternativas:**
- (a) Limpiar `projection_refs` y emitir un warning nuevo. Duplica `LONG_ARROW_SUMMARY_STALE` y añade un código al contrato.
- (b) Marcar la `MacroAssumption` como `needs_review`. Es una mutación semántica implícita que nadie ha autorizado.

**Hueco de test:** lo cubrirá M3 en T3 (integración). No hay unit test, porque es una ausencia de comportamiento.

**Veredicto:** ☐ ✅ ☐ ✏️ ☐ ❓ — Notas:

---

## Siguiente paso

Cuando las 5 tengan veredicto:
1. Aplicar los ✏️ (commit `fix(v0.5.0): ...`) y los tests pendientes de D3, si procede.
2. Revisión Six Hats (Negro obligatorio) de T2.
3. T2: `node split` global.
