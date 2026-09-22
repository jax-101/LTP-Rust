# Plan de Implementación — Supuestos de Long Arrow (agregación/mapeo)

**Fecha**: 2026-09-22
**Specs consultadas**: ENGINE_SPEC.md §2.10 / §3.2, ADR.md (001, 004, 005, 010), CLR_SPEC.md §2.1 (Clarity / Long Arrow)
**RFC relacionado**: RFC-002 (meta-grafo, patrón `projection_map`)
**Prerequisito**: Motor completo (F1–F14 + Knowledge K1–K7). `MacroEdge` ya existe (creado por `path_collapse`).
**Alcance**: Slice 1 del feature "long arrow". La creación top-down (reserva) queda para Slice 2 (fuera de este plan).

## Contexto y Decisión de Diseño (6-Hats, aprobado)

Una long arrow (`macro_edge`) resume una cadena causa-efecto que **coexiste** con ella (collapse es no-destructivo). Este slice añade la capacidad de **llenar/actualizar los supuestos-resumen de la long arrow a partir de la cadena interior, con mapeo (`projection_refs`) y detección de obsolescencia** — como **operación separada, nunca automática**.

Ritmo de dos tiempos **"gather → author"**:
- **Motor** (determinista, ADR-001): recolecta el union de supuestos interiores y diffea contra el resumen almacenado.
- **LLM/usuario** (semántico): destila y mapea vía `add`/`rm`.

Decisiones confirmadas:

| # | Decisión | Resolución |
|---|----------|-----------|
| D1 | Scope Slice 1 | Agregación/mapeo sobre `macro_edges` **existentes** (creados por collapse). Reserva top-down → Slice 2. |
| D2 | Modelo de datos | **Enfoque C (híbrido)**: `gather` es vista viva; el resumen destilado se **almacena** opcionalmente en `MacroEdge.assumptions`. |
| D3 | Superficie de comandos | `macro_assume_gather / add / rm / list`. **`edit` diferido** (edit = rm + add; MASM-id no es referenciado externamente en Slice 1). |
| D4 | `projection_refs` | `Vec<String>` plano de IDs interiores (`LINK-xxx` y/o `ASM-xxx`), validados contra el interior. Vacío permitido (warned en overlay). |
| D5 | Recolección | On-the-fly, **no se persiste** (invariante #3: vistas ejecutivas al vuelo). Solo persiste el resumen autorado. |
| D6 | Tipo de supuesto-resumen | Tipo **propio** `MacroAssumption` (no extender `Assumption`, evita campo siempre-vacío en edges — ADR-005). |
| D7 | Granularidad refs | Links interiores y assumptions interiores. **No** `interior_nodes` (YAGNI). |
| D8 | Interacción con `validate` | `macro_edges` entran en validate **solo como warnings**, nunca bloqueante (invariante #2, ADR-010). |
| D9 | Staleness Slice 1 | Por **membresía de conjuntos** (`unmapped`/`dangling`). Cambio de *texto* de un supuesto interior NO se detecta — limitación documentada, hash de contenido diferido (YAGNI). |
| D10 | Prefijo ID | `MASM-xxx` secuencial vía `next_id("MASM")` (determinista, direccionable). |

Alineación arquitectónica: ADR-001 (motor informa, no interpreta) · ADR-004 (`projection_refs` apuntan **hacia abajo, solo-lectura** → no reintroduce View Update Problem) · ADR-005 (assumptions direccionables por ID, `MacroAssumption` replica el patrón) · ADR-010 (no-bloqueante) · CLR#1 (la "Flecha Larga" es una categoría Clarity; los warnings viven en esa familia). Regla dos-velocidades: la lógica de `gather` es una **función pura storage-agnostic** (futuro `ltp-core`); los campos nuevos son **contrato first-class** (sobreviven a Turso), no hacks de `warnings[]`.

## Dependencias entre Fases

```
M1 (types + gather puro) ──→ M2 (gather cmd) ──→ M3 (add/rm/list) ──→ M4 (validate) ──→ M5 (E2E)
```

---

## Fase M1: Fundación — `MacroAssumption` + gather puro

**Scope**: Definir el tipo, extender `MacroEdge`, función pura de recolección, compat serde.

**Archivos**: `src/tree/types.rs`, `src/path/mod.rs` (o nuevo `src/path/gather.rs`), `src/errors.rs` (si hace falta variante nueva).

### Diseño

```rust
/// Supuesto-resumen destilado que cuelga de una long arrow (macro_edge).
/// Mapea (via projection_refs) a elementos de la cadena causa-efecto interior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroAssumption {
    pub id: String,                     // MASM-xxx
    pub status: String,                 // valid | invalid | needs_review (paridad con Assumption)
    pub text: String,
    #[serde(default)]
    pub projection_refs: Vec<String>,   // IDs interiores: LINK-xxx y/o ASM-xxx (ordenados, dedup)
}
```

Extensión de `MacroEdge`:
```rust
#[serde(default, skip_serializing_if = "Vec::is_empty")]
pub assumptions: Vec<MacroAssumption>,
```
- `#[serde(default)]` → los `macro_edges` existentes (sin el campo) deserializan con `Vec` vacío. Backwards-compatible.
- `skip_serializing_if` → no ensucia el JSON de macros sin resumen.

Función pura (storage-agnostic, base para `ltp-core`):
```rust
/// Recolecta las assumptions de los interior_links de una macro_edge,
/// agrupadas por link. No muta. Determinista (BTreeMap).
pub fn gather_interior_assumptions<'a>(
    tree: &'a Tree,
    macro_edge: &MacroEdge,
) -> BTreeMap<String, Vec<&'a Assumption>>
```
- Recorre `macro_edge.interior_links`, resuelve cada uno en `tree.edges`, devuelve sus `assumptions` por referencia (evitar `.clone()`).
- Links interiores que ya no existen en `tree.edges` (p. ej. removidos) se omiten del union pero cuentan para `dangling` en gather.

**Verificación**: `cargo check` + `clippy` + roundtrip serde de un tree con macro_edge sin/con `assumptions`.

---

## Fase M2: Comando `macro_assume_gather` (lectura + diff)

**Scope**: Vista viva del interior + reconciliación contra el resumen almacenado. NO muta, NO genera undo.

**Archivos**: `src/path/mod.rs` (o `src/macro_assume/mod.rs` nuevo), `src/main.rs` (CLI), `src/mcp/tools.rs` + `src/mcp/dispatch.rs`.

### Diseño

CLI: `ltp macro-assume gather --tree <ID> --macro-link <MACRO_ID>`
MCP: `ltp/macro_assume_gather { tree, macro_link }`

```rust
pub struct GatherData {
    pub macro_link: String,
    pub interior: BTreeMap<String, Vec<InteriorAsm>>, // por link_id → assumptions {id,status,text}
    pub diff: GatherDiff,
}
pub struct GatherDiff {
    pub unmapped: Vec<String>,   // ASM interiores que ningún projection_ref cubre
    pub dangling: Vec<String>,   // projection_refs del resumen que ya no existen en el interior
    pub summary_count: usize,    // nº de MacroAssumption almacenados
}
```

Lógica:
1. Cargar tree, localizar `macro_edge` (error `MACRO_EDGE_NOT_FOUND`).
2. `interior` = `gather_interior_assumptions` (M1), agrupado y ordenado.
3. Construir set vivo de IDs interiores válidos = `interior_links ∪ {asm.id de cada link interior existente}`.
4. Set de refs del resumen = unión de `projection_refs` de `macro_edge.assumptions`.
5. `dangling` = refs del resumen ∉ set vivo (ordenado).
6. `unmapped` = ASM interiores ∉ refs del resumen (ordenado).

**Errores**: `TREE_NOT_FOUND`, `MACRO_EDGE_NOT_FOUND`.

---

## Fase M3: Comandos `add` / `rm` / `list`

**Scope**: Autoría del resumen (mutaciones) + introspección.

**Archivos**: `src/path/mod.rs` (o módulo nuevo), `src/main.rs`, `src/mcp/{tools,dispatch}.rs`, `src/workspace/counters.rs` (prefijo MASM vía `next_id`).

### Diseño

```
ltp macro-assume add  --tree <ID> --macro-link <MACRO_ID> --text "<...>" [--projection <ID>]...
ltp macro-assume rm   --tree <ID> --macro-link <MACRO_ID> --asm <MASM_ID>
ltp macro-assume list --tree <ID> --macro-link <MACRO_ID> [--status <s>]
```

`add`:
- Validar `text` no vacío (`TEXT_REQUIRED`).
- Resolver cada `--projection <ID>` contra el interior de ESTA macro:
  - `LINK-xxx` ∈ `interior_links`, o `ASM-xxx` ∈ assumptions de un interior_link. Si no → error `PROJECTION_REF_NOT_IN_INTERIOR` (con contexto `ref`).
  - Ref a un `MASM-xxx` (auto/lateral) → error `PROJECTION_REF_INVALID`.
  - Dedup con BTreeSet; orden canónico.
- `next_id("MASM")` → nuevo `MacroAssumption { status: "valid", ... }`.
- Push a `macro_edge.assumptions`; `save_tree`; `history_commit("macro_assume_add", ...)`.
- Output `MacroAssumeAddData { created_assumption_id, macro_link, projection_refs }`.
- Warning `MACRO_ASSUMPTION_UNGROUNDED` si `projection_refs` vacío **y** el interior no está vacío (overlay).

`rm`:
- Localizar `MASM_ID` en `macro_edge.assumptions` (error `MACRO_ASSUMPTION_NOT_FOUND`).
- Remover; `save_tree`; `history_commit`. Idempotencia: si no existe → error (no silencioso; paridad con `assume_rm`).
- Output `MacroAssumeRmData { removed_assumption, macro_link }`.

`list`:
- Devolver `macro_edge.assumptions` (filtrable por `--status`). Solo lectura.
- Output `MacroAssumeListData { macro_link, assumptions: Vec<MacroAssumption>, count }`.

---

## Fase M4: Integración con `validate`

**Scope**: Hacer visibles las long arrows a `validate` mediante **warnings no-bloqueantes**.

**Archivos**: `src/validate/macro_edge.rs` (nuevo, mirror de `validate/knowledge.rs`), `src/validate/mod.rs` (wiring).

### Warnings nuevos

| Warning | Condición | Contexto |
|---------|-----------|----------|
| `LONG_ARROW_UNSUMMARIZED` | `macro_edge` activo con `assumptions` vacío (interior no vacío) | `macro_link`, `interior_asm_count` |
| `LONG_ARROW_SUMMARY_STALE` | `unmapped` ∪ `dangling` ≠ ∅ (reusa lógica de M2) | `macro_link`, `unmapped[]`, `dangling[]` |
| `MACRO_ASSUMPTION_UNGROUNDED` | Un `MacroAssumption` con `projection_refs` vacío e interior no vacío | `macro_link`, `assumption_id` |

- Solo `macro_edges` con `status == "active"`.
- **Nunca errores** — no afectan `valid_dag`. Reusa el diff de M2 (extraer helper puro compartido).

---

## Fase M5: Tests End-to-End + Cierre

**Scope**: Workflows completos + verificación final + PROGRESS.md.

**Archivos**: `tests/macro_assume.rs` (nuevo), `PROGRESS.md`.

### Workflows críticos
1. collapse → gather (vacío) → add×2 con projection → list → validate (sin stale).
2. add dejando un ASM interior sin mapear → gather muestra `unmapped` → validate `LONG_ARROW_SUMMARY_STALE`.
3. add con projection válido → invalidar/borrar ese ASM interior (`assume_rm`) → gather muestra `dangling` → validate stale.
4. `path_replace` sobre la macro (interior superseded) → refs quedan dangling → validate.
5. Undo/redo roundtrip de `add`/`rm` (macro_assume participa en undo; counter no retrocede).
6. Batch `history_begin_batch` … add×N … `history_end_batch` → un undo revierte todo.

---

## UATs (adversariales — edge cases, boundaries, corrupt, illegal transitions)

**Happy (H)**
- H1: gather de macro recién colapsada → `interior` poblado, `diff.summary_count = 0`, `unmapped` = todos los ASM interiores.
- H2: add con 2 projection_refs válidos → MASM-001, JSON canónico.
- H3: list tras 2 adds → 2 items ordenados.
- H4: add sin `--projection` con interior vacío (macro degenerada) → sin warning ungrounded.

**Boundary (B)**
- B1: add `--text ""` → `TEXT_REQUIRED`, counter MASM no incrementa.
- B2: add con projection_refs duplicados (`--projection ASM-007 --projection ASM-007`) → dedup a uno.
- B3: gather de macro con `interior_links` vacío → `interior` vacío, sin panic.
- B4: add con interior no vacío pero sin projection → warning `MACRO_ASSUMPTION_UNGROUNDED`, pero **success=true**.
- B5: rm del último MASM → `assumptions` vuelve a `[]`, se omite del JSON (skip_serializing_if).

**Corrupt / referential (C)**
- C1: gather/add/rm con `--macro-link` inexistente → `MACRO_EDGE_NOT_FOUND`.
- C2: gather/add con `--tree` inexistente → `TREE_NOT_FOUND`.
- C3: add `--projection LINK-999` (no en interior) → `PROJECTION_REF_NOT_IN_INTERIOR`, no muta.
- C4: add `--projection ASM-020` donde ASM-020 existe pero en un link **periférico** (no interior) → `PROJECTION_REF_NOT_IN_INTERIOR`.
- C5: add `--projection MASM-001` (auto-referencia) → `PROJECTION_REF_INVALID`.
- C6: rm de `MASM_ID` inexistente → `MACRO_ASSUMPTION_NOT_FOUND`.
- C7: tree con `macro_edge` legacy (sin campo `assumptions`) → deserializa OK, gather funciona.

**Illegal transitions / interaction (I)**
- I1: add projection a ASM interior → borrar ese ASM con `assume_rm` → validate `LONG_ARROW_SUMMARY_STALE` con `dangling=[ASM-x]`.
- I2: `path_replace` de la macro → macro_edge desaparece → gather posterior → `MACRO_EDGE_NOT_FOUND` (la macro ya no existe).
- I3: `path_explode` añade un INT y parte un interior_link → nuevos ASM interiores → gather muestra `unmapped` de los nuevos; refs previos que apuntaban al link partido → `dangling`.
- I4: NESTED (no aplica — collapse ya prohíbe anidar; verificar que gather no rompe con interiores complejos tipo diamond).

**Idempotencia / ordering (O)**
- O1: undo de `macro_assume_add` → el MASM desaparece; redo lo restaura con **mismo ID** (snapshot).
- O2: dos adds → gather → orden estable (BTreeSet) independiente del orden de inserción.
- O3: batch con 3 adds + 1 rm → un `undo` revierte los 4.

---

## Códigos de Error / Warning Nuevos

| Code | Tipo | Fase | Contexto |
|------|------|------|----------|
| `MACRO_EDGE_NOT_FOUND` | error | M2 | macro_link no existe en el tree (ya existe en path_replace; reusar) |
| `TEXT_REQUIRED` | error | M3 | add con text vacío |
| `PROJECTION_REF_NOT_IN_INTERIOR` | error | M3 | projection ref no está en el interior de la macro |
| `PROJECTION_REF_INVALID` | error | M3 | projection ref apunta a un MASM (lateral/auto) |
| `MACRO_ASSUMPTION_NOT_FOUND` | error | M3 | rm de MASM inexistente |
| `MACRO_ASSUMPTION_UNGROUNDED` | warning | M3/M4 | resumen sin projection_refs (interior no vacío) |
| `LONG_ARROW_UNSUMMARIZED` | warning | M4 | macro activa sin resumen |
| `LONG_ARROW_SUMMARY_STALE` | warning | M4 | unmapped/dangling ≠ ∅ |

---

## MCP Tools Nuevos

| Tool | Params |
|------|--------|
| `ltp/macro_assume_gather` | `tree`, `macro_link` |
| `ltp/macro_assume_add` | `tree`, `macro_link`, `text`, `projection?` (array) |
| `ltp/macro_assume_rm` | `tree`, `macro_link`, `asm` |
| `ltp/macro_assume_list` | `tree`, `macro_link`, `status?` |

(Actualizar el conteo real de tools en SKILL.md como cierre; hoy declara 54, real 61 → 65 tras este slice.)

---

## Estimación

| Fase | Archivos | UATs | Complejidad |
|------|----------|:----:|-------------|
| M1 | tree/types.rs, path/gather | 3 | Baja |
| M2 | gather cmd + CLI/MCP | 6 | Media |
| M3 | add/rm/list + CLI/MCP | 12 | Media-Alta |
| M4 | validate/macro_edge.rs | 6 | Media |
| M5 | tests/macro_assume.rs | ~10 workflows | Media |

**Riesgo principal**: reconciliación de `dangling` cuando el interior cambia por operaciones destructivas existentes (`path_replace`, `path_explode`, `assume_rm`). Mitigado con UATs I1–I3.

**Gate obligatorio antes de "completado"**: `cargo check --all-targets --all-features` + `clippy --all-targets --all-features -- -D warnings` + `test --workspace` + `fmt --all -- --check`. Commit por fase; push al cerrar el slice; actualizar PROGRESS.md.

---

## Fuera de Alcance (Slice 2 / observaciones)

- Creación top-down (`macro_add` reserva con interior vacío), expansión vía `explode` generalizado, promoción a edge atómico, participación DAG de reservas.
- `macro_assume_edit` (revisitar si algo pasa a referenciar `MASM-id` de forma estable).
- Hash de contenido para staleness de *texto*.
- **Observación** (no en scope): el tool MCP `ltp/path_explode` (tools.rs:557) se describe como "Explode a macro-link back to its original sub-graph", pero la implementación explota un **assumption** de un edge normal en un nodo INT. Doc desalineada — fix trivial aparte.
