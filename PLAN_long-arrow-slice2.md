# Plan de Implementación — Long Arrow Slice 2 (reserva top-down · status tipado · expand/promote)

**Fecha**: 2026-09-22
**Specs consultadas**: ENGINE_SPEC.md §2.10 / §3.1 / §3.2, ADR.md (001, 004, 005, 009, 010), CLR_SPEC.md §2.1 (Clarity / Long Arrow), PLAN_long-arrow-assumptions.md (Slice 1, SHIPPED).
**RFC relacionado**: RFC-002 (meta-grafo). Cierra los gaps 1/4/6 del modelo `MacroEdge`.
**Prerequisito**: Slice 1 completo (`macro-assume gather/add/rm/list` + `validate` no-bloqueante). `MacroEdge` con `assumptions: Vec<MacroAssumption>`.
**Alcance**: creación **top-down** de long arrows (reserva con interior vacío), `status` como **enum tipado de ciclo de vida**, materialización de la reserva (`expand` → cadena INT) y promoción a edge atómico (`promote`).

---

## Contexto y Decisión de Diseño (Six-Hats, aprobado)

Slice 1 llenaba/mapeaba los supuestos-resumen de long arrows **ya existentes** (creadas bottom-up por `path collapse`). Slice 2 añade la dirección **inversa**: declarar primero el salto lógico (la "flecha larga" de CLR#1 — *pasos intermedios no expresados*, `CLR_SPEC.md §2.1`) como una **reserva**, y luego resolverla de una de dos formas: **expandirla** en una cadena causal explícita (INT), o **promoverla** a un edge atómico directo cuando el salto resulta ser una causalidad directa legítima.

Esto respeta ADR-004 (mutaciones por intención; ahora en dirección top-down objetivo→detalle) y ADR-001 (el motor materializa/promueve de forma determinista; el LLM decide *qué* reservar, *con cuántos pasos* expandir y *cuándo* promover).

### Hechos de arquitectura confirmados (Sombrero Blanco)

| Hecho | Implicación para Slice 2 |
|-------|--------------------------|
| `check_dag` (`validate/dag.rs`) corre **solo** sobre `tree.edges` y `nbr.edges`; nunca sobre `macro_edges`. | Las reservas **no participan en el DAG** — encajan como overlay no-bloqueante (invariante #2). Sin cambios en `check_dag`. |
| `check_orphans` (`orphans.rs`) solo considera conectividad de `edges`. | Una reserva entre dos nodos sin edges reales marcaría **ambos extremos como huérfanos** (falso). Debe corregirse. |
| Producción solo emite `status: "active"` (collapse, `path/mod.rs:364`). `"exploded"`/`"superseded"` solo existen en tests. | La migración a enum tipado tiene blast radius pequeño y `"active"` mapea 1:1 a `Overlay`. |
| Slice-1 `resolve_projection_refs` exige refs en interior vivo; `LONG_ARROW_UNSUMMARIZED` solo dispara si `interior_asm_count > 0`. | Una reserva (interior vacío) **no produce hoy ningún warning de macro** → hace falta `LONG_ARROW_RESERVATION_PENDING`. |
| `path explode` opera sobre **edge normal + assumption → INT**, NO sobre macros. | El `expand` de Slice 2 es una operación **nueva y distinta**; el nombre `macro expand` evita el footgun `expand`/`explode`. |
| Prefijo de contador `MACRO` ya existe (`counters.rs:13`). Patrón `history_begin/commit` para undo (ADR-009). | Reutilizables sin cambios estructurales. |

### Tabla de decisiones (Six-Hats)

| # | Decisión | Resolución | Justificación (hat dominante) |
|---|----------|-----------|-------------------------------|
| D1 | Superficie de comandos | **Grupo nuevo `macro`**: `macro add` / `macro expand` / `macro promote`. `path {collapse,explode,replace}` intactos. | ⚫ Evita colisión cognitiva `expand`/`explode`; 🔵 separa nav Slice-1 de ciclo-de-vida; simetría con `macro-assume`. |
| D2 | Tipo de `status` | **enum `MacroEdgeStatus { Reservation, Overlay }`**, `#[serde(rename_all="snake_case")]` + `#[serde(alias="active")]` sobre `Overlay`. | 🔵 Type-First, replica patrón de `EdgeStatus`; ⚫ retrocompat de lectura documentada (migración de una vía en la próxima escritura). |
| D3 | Semántica `expand` | `Reservation → Overlay`: crea cadena `from→INT₁→…→INTₙ→to` (n = nº de `--steps`), puebla `interior_nodes`/`interior_links`. Los nuevos edges **entran al DAG**. | 🟢 Reutiliza patrón `next_id("INT")`/`next_id("LINK")` de `path_explode`; 🟡 la reserva pasa a respaldar una cadena real → deviene Overlay. |
| D4 | Semántica `promote` | Crea edge atómico `from→to` (SINGLE) y **consume la macro** (`.remove()`, espeja `path replace`). **Migra cada `MacroAssumption` → `Assumption`** en el nuevo edge (nuevo `ASM-id`, preserva `text`/`status`, descarta `projection_refs`). | ⚫ "promote conserva overlay sobre 1 edge" es un resumen que no resume → incoherente; "consume y punto" pierde datos → 🟡 la migración de supuestos es la síntesis sin pérdida. |
| D5 | Validez de transición | `expand` y `promote` **solo válidas sobre `Reservation`** (`NOT_A_RESERVATION` si `Overlay`). Para overlays creados por collapse ya existe `path replace`. | ⚫ Evita solapamiento semántico con `path replace`; mantiene la máquina de estados sin ramas ambiguas. |
| D6 | Reserva ↔ huérfanos | **Extender `check_orphans`** con un parámetro desacoplado `reserved_endpoints: &[&str]` (calculado por el caller desde `tree.macro_edges`: `from`+`to` de todos); siembra `connected` con esos extremos. `orphans.rs` **no** importa el modelo macro. | ⚫ Un nodo reservado no es huérfano, está *pendiente*; ⚫ pasar `&[MacroEdge]` acoplaría `orphans.rs` al tipo macro → firma genérica (responsabilidad única). |
| D7 | Warning de reserva | Nuevo **`LONG_ARROW_RESERVATION_PENDING`** (familia CLR#1 Clarity), no-bloqueante. `check_macro_edges` migra de `if status != "active"` a `match` sobre el enum. | 🔵 CLR#1 = "flecha larga con pasos intermedios no expresados" = definición literal de reserva. |
| D8 | `macro_assume_edit` + content-hash | **Diferidos ambos** (sin cambio respecto a Slice-1 D3/D9). | ⚫ El content-hash impone mantenimiento transversal sobre todos los mutadores de interior por beneficio marginal → YAGNI; `edit` = rm+add sigue cubriendo el caso. |
| D9 | DAG en expand/promote | **Reservas fuera del DAG** (no tienen edges reales; sin cambio en `check_dag`). Pero `expand`/`promote` **materializan edges reales** en `tree.edges` → pre-validan con `check_dag(tree.edges + nuevos)` y **BLOQUEAN ciclos** (`CIRCULAR_DEPENDENCY_DETECTED` + `cycle_path`, liberan lock, **no guardan**). | ⚪ Hecho verificado: `link connect` (`commands.rs:383`) y todas las ops de `link/advanced.rs` (split/move/insert/reverse) ya bloquean así; 🔵 invariante #2 (topología estricta bloqueante); ⚫ un `promote` no-bloqueante persistiría `valid_dag:false` con la info para prevenirlo → regresión de integridad. |
| D10 | Ubicación de código | Módulo nuevo **`src/macro_edge/mod.rs`** para el ciclo de vida (reserve/expand/promote). `path/mod.rs` (1014 líneas) sin crecer. | 🟡 Mantenibilidad + cohesión por grupo de comando; paralelo a `src/macro_assume/`. Regla dos-velocidades: funciones puras storage-agnostic donde aplique (base `ltp-core`). |

### Máquina de estados resultante

```
                        path collapse (bottom-up)
   nodos ───────────────────────────────────────────► Overlay ──(path replace)──► [macro eliminada]
                                                          ▲
                                              expand      │  (materializa interior = cadena INT)
   macro add (top-down) ──► Reservation ──────────────────┘
                                 │
                                 └── promote ──► [macro eliminada] + edge atómico from→to
                                                 (MacroAssumption → Assumption en el edge)
```

Sin estados muertos: la trazabilidad histórica ya la cubren ADR-009 (snapshots undo) y ADR-002 (JSON git-diffable), por lo que tombstones `Expanded`/`Promoted` serían redundantes.

---

## Dependencias entre Fases

```
M1 (enum + migración serde) ──► M2 (macro add / reserve)
                                    │
                                    ├──► M3 (macro expand)
                                    └──► M4 (macro promote)
                                              │
M1 ──► M5 (validate: RESERVATION_PENDING + orphans) ◄─────┘
M2,M3,M4 ──► M6 (CLI + MCP wiring) ──► M7 (E2E + docs + cierre)
```

---

## Fase M1: Fundación — `MacroEdgeStatus` tipado + migración serde

**Scope**: enum de ciclo de vida, cambio de `MacroEdge.status`, retrocompat de lectura, actualización de constructores existentes.

**Archivos**: `src/tree/types.rs`, `src/path/mod.rs` (collapse), `src/validate/macro_edge.rs`, `src/macro_assume/mod.rs` (test helper).

### Diseño (Type-First)

```rust
/// Ciclo de vida de una long arrow (`MacroEdge`).
///
/// - `Overlay`: resume una cadena causa-efecto real que coexiste con ella
///   (creada por `path collapse` o resultante de `macro expand`). Retrocompat:
///   los `macro_edges` previos con `"status": "active"` deserializan aquí.
/// - `Reservation`: salto lógico declarado top-down (CLR#1 "flecha larga"),
///   con interior vacío, pendiente de `expand` o `promote`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MacroEdgeStatus {
    Reservation,
    #[serde(alias = "active")] // retrocompat: workspaces previos a Slice 2
    Overlay,
}
```

Cambio en `MacroEdge`:
```rust
pub status: MacroEdgeStatus,   // antes: String
```

- `path_collapse` (`path/mod.rs:364`): `status: "active".to_string()` → `MacroEdgeStatus::Overlay`.
- `validate/macro_edge.rs`: eliminar `const ACTIVE_STATUS`; sustituir `if me.status != ACTIVE_STATUS { continue }` por un `match` (ver M5). Actualizar el test `inactive_macro_edge_is_skipped` (ya no hay estado "inactivo" — ambos estados se auditan; reemplazar por casos `Reservation`/`Overlay`).
- Test helpers que construyen `MacroEdge { status: "active".into() }` → `MacroEdgeStatus::Overlay`.

**Verificación**: `cargo check` + roundtrip serde: (a) JSON legacy `"status":"active"` sin `assumptions` → `Overlay` + `assumptions` vacío; (b) `Reservation` serializa `"status":"reservation"`; (c) `Overlay` serializa `"status":"overlay"`.

---

## Fase M2: `macro add` — reserva top-down

**Scope**: crear un `MacroEdge` con `status: Reservation` e interior vacío entre dos nodos del tree.

**Archivos**: `src/macro_edge/mod.rs` (nuevo), `src/lib.rs` (`pub mod macro_edge;`).

### Diseño

CLI: `ltp macro add --tree <ID> --from <NODE> --to <NODE> --label "<...>"`

```rust
#[derive(Debug, Serialize)]
pub struct MacroAddData {
    pub macro_edge_id: String,
    pub from: String,
    pub to: String,
    pub label: String,
}

pub fn execute_macro_add(
    storage: &dyn Storage,
    tree_id: &str,
    from: &str,
    to: &str,
    label: &str,
) -> CommandOutput<MacroAddData>
```

Lógica (bajo lock, participa en undo vía el llamador):
1. `load_tree` (`TREE_NOT_FOUND`).
2. Validar `label` no vacío (`LABEL_REQUIRED`).
3. Validar `from != to` (`RESERVATION_SELF_LOOP`).
4. Validar `from` y `to` attached al tree (`NODE_NOT_IN_TREE`, paridad con `collapse`).
5. `next_id("MACRO")` → `MacroEdge { interior_nodes: [], interior_links: [], status: Reservation, assumptions: [] }`.
6. `save_tree`; output `MacroAddData`.
- **No** valida que exista/no exista camino real `from→to`: la reserva es una declaración de intención independiente de la topología táctica (ADR-004). Documentado.

**Errores**: `TREE_NOT_FOUND`, `LABEL_REQUIRED`, `RESERVATION_SELF_LOOP`, `NODE_NOT_IN_TREE`.

---

## Fase M3: `macro expand` — materializar la reserva

**Scope**: transformar una `Reservation` en `Overlay` creando una cadena de nodos INT explícitos.

**Archivos**: `src/macro_edge/mod.rs`.

### Diseño

CLI: `ltp macro expand --tree <ID> --macro-link <MACRO_ID> --steps "<label1>,<label2>,..."`

```rust
#[derive(Debug, Serialize)]
pub struct MacroExpandData {
    pub macro_link: String,
    pub created_nodes: Vec<String>,   // INT-xxx en orden
    pub created_links: Vec<String>,   // LINK-xxx en orden (n+1)
    pub status: MacroEdgeStatus,      // Overlay
}
```

Lógica (bajo lock, undo):
1. `load_tree`; localizar macro (`MACRO_EDGE_NOT_FOUND`).
2. Exigir `status == Reservation` (`NOT_A_RESERVATION`).
3. Parsear `--steps` (n ≥ 1 labels no vacías); vacío → `STEPS_REQUIRED`. Labels duplicadas permitidas (los INT son entidades distintas; los labels no son únicos).
4. Por cada step: `next_id("INT")` → `Node { node_type: Int, label, observable: true, ... }`; `save_node`; `NodeRef` attach al tree.
5. Crear (n+1) edges vía `next_id("LINK")`: cadena `from→INT₁→…→INTₙ→to`, `Operator::Single`, `logic` **derivado de `tree.logic`** (`Sufficiency`/`Necessity`), `status: Active`.
6. Poblar `macro.interior_nodes = [INT…]`, `macro.interior_links = [LINK…]`, `macro.status = Overlay`.
7. Los `MacroAssumption` de la reserva **se conservan** (ahora sobre un Overlay); pasan a ser proyectables por `macro-assume gather/add` (pueden quedar ungrounded hasta mapear — consistente con Slice-1).
8. **Pre-validar DAG** (D9): `check_dag(tree.edges + nuevos edges)`; si cierra un ciclo (ya existe camino real `to→…→from`) → `CIRCULAR_DEPENDENCY_DETECTED` + `cycle_path`, libera lock, **no guarda** (contrato idéntico a `link connect`).
9. `save_tree`; output.

**Errores**: `TREE_NOT_FOUND`, `MACRO_EDGE_NOT_FOUND`, `NOT_A_RESERVATION`, `STEPS_REQUIRED`, `CIRCULAR_DEPENDENCY_DETECTED`, `ID_GENERATION_ERROR`, `IO_ERROR`.
**Determinismo**: IDs INT/LINK secuenciales en orden de `--steps`.

---

## Fase M4: `macro promote` — promover a edge atómico

**Scope**: aceptar el salto como causalidad directa: crear un edge atómico `from→to` y consumir la reserva, migrando sus supuestos.

**Archivos**: `src/macro_edge/mod.rs`.

### Diseño

CLI: `ltp macro promote --tree <ID> --macro-link <MACRO_ID>`

```rust
#[derive(Debug, Serialize)]
pub struct MacroPromoteData {
    pub macro_link: String,            // consumido
    pub created_link: String,          // nuevo edge atómico
    pub migrated_assumptions: Vec<String>, // ASM-xxx migrados (en orden)
    pub from: String,
    pub to: String,
}
```

Lógica (bajo lock, undo):
1. `load_tree`; localizar macro (`MACRO_EDGE_NOT_FOUND`).
2. Exigir `status == Reservation` (`NOT_A_RESERVATION`). (Overlays de collapse → usar `path replace`.)
3. Re-validar `from`/`to` attached (defensivo; `NODE_NOT_IN_TREE`).
4. `next_id("LINK")` → `Edge { from: [macro.from], to: macro.to, operator: Single, logic: derivado de tree.logic, status: Active, assumptions: [] }` (supuestos vacíos por ahora).
5. **Pre-validar DAG** (D9): `check_dag(tree.edges + edge)`; si cierra un ciclo (existe camino real `to→…→from`) → `CIRCULAR_DEPENDENCY_DETECTED` + `cycle_path`, libera lock, **no guarda** (contrato idéntico a `link connect`, `commands.rs:383`). *(Se valida **antes** de mintear los `ASM-id` de la migración para no quemar contadores en el caso bloqueado — mismo orden que `link connect` respecto a sus IDs.)*
6. Migrar: por cada `MacroAssumption` (en orden) → `Assumption { id: next_id("ASM"), status, text }` sobre el edge. Se descartan `projection_refs` (interior vacío).
7. `tree.macro_edges.remove(idx)` + push del edge atómico (espeja `path replace`).
8. `save_tree`; output.
- **DAG bloqueante** (verificado, D9): toda mutación de ltp-engine que crea/recablea edges pre-valida el DAG y bloquea ciclos; `promote` no es excepción. NO se delega en `validate`.

**Errores**: `TREE_NOT_FOUND`, `MACRO_EDGE_NOT_FOUND`, `NOT_A_RESERVATION`, `NODE_NOT_IN_TREE`, `CIRCULAR_DEPENDENCY_DETECTED`, `ID_GENERATION_ERROR`, `IO_ERROR`.

---

## Fase M5: Integración con `validate`

**Scope**: warning de reserva pendiente + reinterpretación de huérfanos.

**Archivos**: `src/validate/macro_edge.rs`, `src/validate/orphans.rs`, `src/validate/mod.rs`.

### `check_macro_edges` → `match me.status`

```rust
match me.status {
    MacroEdgeStatus::Reservation => {
        // CLR#1: salto lógico con pasos intermedios no expresados.
        warnings.push(OutputWarning::new(
            "LONG_ARROW_RESERVATION_PENDING",
            format!("Long arrow '{}' is a reservation pending expansion or promotion", me.id))
            .with_context("macro_link", me.id.as_str())
            .with_context("from", me.from.as_str())
            .with_context("to", me.to.as_str()));
        // Interior vacío ⇒ ni UNSUMMARIZED ni UNGROUNDED aplican.
    }
    MacroEdgeStatus::Overlay => {
        // Higiene existente de Slice 1: UNSUMMARIZED / STALE / UNGROUNDED (sin cambios).
    }
}
```

### `check_orphans` (D6)

Añadir a `check_orphans` un parámetro **desacoplado** `reserved_endpoints: &[&str]` (no `&[MacroEdge]`, para no acoplar `orphans.rs` al modelo macro) y sembrar `connected` con esos extremos antes del barrido. El caller (`validate/mod.rs`, **único** call site de producción) calcula el set desde `tree.macro_edges` (`from`+`to` de todos). Actualizar los 2 tests unitarios de `orphans.rs` (pasan `&[]`).

**Nunca errores** — ninguno afecta `valid_dag` (ADR-010).

---

## Fase M6: Wiring CLI + MCP

**Archivos**: `src/main.rs`, `src/mcp/tools.rs`, `src/mcp/dispatch.rs`.

- `main.rs`: `enum MacroAction { Add, Expand, Promote }`, variante `Commands::Macro { action }`, dispatch con `history_begin`/`history_commit("macro_add"|"macro_expand"|"macro_promote")` (patrón idéntico a `MacroAssume`/`Path`).
- `mcp/tools.rs`: 3 tool defs nuevas (`ltp/macro_add`, `ltp/macro_expand`, `ltp/macro_promote`) con sus schemas.
- `mcp/dispatch.rs`: 3 `dispatch_*` + routing en el `match`.

---

## Fase M7: E2E + documentación + cierre

**Archivos**: `tests/macro_lifecycle.rs` (nuevo), `ENGINE_SPEC.md`, `ADR.md`, `PROGRESS.md`.

- **ENGINE_SPEC.md**: §2.10 documentar `macro add/expand/promote`; §3.2 actualizar schema de `macro_edge` (vocabulario `status: reservation | overlay`, nota del alias legacy `active`; documentar el campo `assumptions` que Slice-1 añadió al código pero nunca al spec — deuda menor saldada aquí).
- **ADR-013** (nuevo): ciclo de vida tipado de `macro_edge`; reservas fuera del DAG pero `expand`/`promote` **bloquean ciclos** al materializar edges reales (contrato uniforme con `link connect`/`link advanced`); reinterpretación de huérfanos; `promote` migra supuestos. Es el hogar correcto de la decisión de migración serde (Sombrero Negro: debe quedar documentada, no ser efecto colateral).
- **PROGRESS.md**: factor de escala + avance global según sus reglas.

---

## UATs — matriz adversarial completa (integration tests AUTOMÁTICOS en `tests/macro_lifecycle.rs`)

> Regla del proyecto (memoria `feedback_deep-uats`): nada de solo happy-path. Cada UAT es un test `#[test]` automático que construye un workspace temporal, ejecuta comandos y asserta salida/estado. Se cubren boundaries, datos corruptos, transiciones ilegales y ordering/idempotencia.

### Happy (H)
- **H1** `macro add` → `MACRO-001`, `status:"reservation"`, interior vacío, JSON canónico (claves ordenadas).
- **H2** `macro expand --steps "a,b"` sobre reserva → 2 INT + 3 LINK, `status:"overlay"`, cadena `from→INT-001→INT-002→to` verificada por `trace`.
- **H3** `macro promote` sobre reserva sin supuestos → 1 edge atómico SINGLE `from→to`, macro eliminada, `migrated_assumptions: []`.
- **H4** `macro promote` sobre reserva con 2 `MacroAssumption` → edge atómico con 2 `Assumption` (ASM-xxx, `text`/`status` preservados), macro eliminada.
- **H5** `macro expand --steps "único"` (n=1) → 1 INT + 2 LINK.

### Boundary (B)
- **B1** `macro add --label ""` → `LABEL_REQUIRED`; contador MACRO **no** incrementa.
- **B2** `macro add --from X --to X` → `RESERVATION_SELF_LOOP`.
- **B3** `macro expand --steps ""` → `STEPS_REQUIRED`; sin INT/LINK creados; status sigue `Reservation`.
- **B4** `macro expand --steps "l1,l1"` (labels duplicadas) → 2 INT distintos con mismo label; sin error.
- **B5** `macro expand` sobre árbol de **necesidad** (GT/EC/PRT) → edges creados con `logic:"NECESSITY"` (derivado de `tree.logic`), no Sufficiency.
- **B6** `macro promote` de reserva cuyos extremos son los únicos nodos del tree → edge atómico conecta ambos; `validate` sin `ORPHAN_NODE_IN_TREE`.

### Corrupt / referential (C)
- **C1** add/expand/promote con `--tree` inexistente → `TREE_NOT_FOUND`.
- **C2** expand/promote con `--macro-link` inexistente → `MACRO_EDGE_NOT_FOUND`.
- **C3** `macro add --from LINK-999` (nodo no attached) → `NODE_NOT_IN_TREE`; no muta.
- **C4** `macro expand` sobre un **Overlay** (creado por `path collapse`) → `NOT_A_RESERVATION`; no muta.
- **C5** `macro promote` sobre un **Overlay** → `NOT_A_RESERVATION`.
- **C6** deserializar `macro_edge` legacy (`"status":"active"`, sin `assumptions`) → `Overlay` + `assumptions` vacío; `validate` lo trata como overlay (extiende UAT C7 de Slice-1).

### Illegal transitions / interaction (I)
- **I1** `expand` → `expand` de nuevo sobre la misma macro → segundo `NOT_A_RESERVATION` (ya es Overlay).
- **I2** `promote` → `promote`/`expand` de nuevo → `MACRO_EDGE_NOT_FOUND` (consumida).
- **I3** reserva entre dos nodos sin edges reales → `validate`: **NO** `ORPHAN_NODE_IN_TREE` en los extremos, **SÍ** `LONG_ARROW_RESERVATION_PENDING` (D6+D7).
- **I4** `path collapse` → macro resultante es `Overlay` (migración D1/D2); `validate` no emite `RESERVATION_PENDING`.
- **I5** `macro-assume add --projection LINK-xxx` sobre una **reserva** → `PROJECTION_REF_NOT_IN_INTERIOR` (interior vacío; confirma que el guard de Slice-1 se sostiene).
- **I6** `macro-assume add` **sin** projection sobre reserva → success, **sin** `MACRO_ASSUMPTION_UNGROUNDED` (interior vacío). Luego `macro expand` → `validate` ahora **sí** emite `UNGROUNDED` (interior no vacío, refs vacías). *(cadena de transición que descubre el edge case del supuesto que "envejece" al expandir)*.
- **I7** `macro promote` que **cerraría un ciclo** (existe camino real `to→…→from`) → `CIRCULAR_DEPENDENCY_DETECTED` + `cycle_path`, `success:false`, **sin mutación** (mismo contrato que `link connect`: el edge atómico no se crea, la macro **no** se consume, el estado en disco queda intacto).
- **I8** `macro expand` de una reserva cuyos extremos estaban huérfanos → tras expand, `validate` no emite `ORPHAN_NODE_IN_TREE` (ahora conectados por edges reales) ni `RESERVATION_PENDING` (ahora Overlay).
- **I9** `macro expand` de una reserva `from→to` cuando ya existe camino real `to→…→from` → `CIRCULAR_DEPENDENCY_DETECTED` + `cycle_path`; INT/LINK **no** creados, `status` sigue `Reservation` (el pre-check DAG del paso 8 bloquea antes de persistir).

### Ordering / idempotencia (O)
- **O1** `undo` de `macro add` → macro desaparece; `redo` la restaura con **mismo `MACRO-id`** (snapshot ADR-009); contador no retrocede.
- **O2** `undo` de `macro expand` → INT/LINK creados desaparecen, macro vuelve a `Reservation`, INT desatachados del tree; `redo` restaura con mismos IDs.
- **O3** `undo` de `macro promote` → edge atómico + `Assumption` migrados desaparecen, macro `Reservation` reaparece con sus `MacroAssumption` intactos; `redo` reaplica.
- **O4** `history begin-batch` … `macro add` + `macro expand` + N `macro-assume add` … `end-batch` → un solo `undo` revierte todo el batch.
- **O5** `macro expand --steps "a,b,c"` → IDs `INT-001,INT-002,INT-003` y `LINK-001..004` en orden estable, independiente de reintentos.

---

## Códigos de Error / Warning Nuevos

| Code | Tipo | Fase | Contexto |
|------|------|------|----------|
| `LABEL_REQUIRED` | error | M2 | `macro add` con label vacío |
| `RESERVATION_SELF_LOOP` | error | M2 | `macro add` con `from == to` |
| `NOT_A_RESERVATION` | error | M3/M4 | `expand`/`promote` sobre un `Overlay` |
| `STEPS_REQUIRED` | error | M3 | `macro expand` sin steps |
| `LONG_ARROW_RESERVATION_PENDING` | warning | M5 | reserva pendiente de expand/promote (CLR#1) |

Reutilizados: `TREE_NOT_FOUND`, `MACRO_EDGE_NOT_FOUND`, `NODE_NOT_IN_TREE`, `CIRCULAR_DEPENDENCY_DETECTED` (con contexto `cycle_path`, mismo contrato que `link connect`; bloquea `expand`/`promote`), `ID_GENERATION_ERROR`, `IO_ERROR`, `LOCK_ERROR`, `STALE_LOCK_REMOVED`.

---

## MCP Tools Nuevos

| Tool | Params |
|------|--------|
| `ltp/macro_add` | `tree`, `from`, `to`, `label` |
| `ltp/macro_expand` | `tree`, `macro_link`, `steps` (array) |
| `ltp/macro_promote` | `tree`, `macro_link` |

(Actualizar el conteo real de tools en el cierre — Slice-1 dejó 65; este slice suma 3 → 68.)

---

## Estimación

| Fase | Archivos | UATs | Complejidad |
|------|----------|:----:|-------------|
| M1 | tree/types.rs, path/mod.rs, validate/macro_edge.rs | 3 | Baja (migración mecánica + serde) |
| M2 | macro_edge/mod.rs, lib.rs | 5 | Baja-Media |
| M3 | macro_edge/mod.rs | 9 | Media (cadena INT determinista + pre-check DAG) |
| M4 | macro_edge/mod.rs | 6 | Media (migración de supuestos + pre-check DAG) |
| M5 | validate/{macro_edge,orphans,mod}.rs | 5 | Media (firma de `check_orphans` + tests) |
| M6 | main.rs, mcp/{tools,dispatch}.rs | — | Baja (patrón replicado) |
| M7 | tests/macro_lifecycle.rs, specs, PROGRESS.md | ~28 | Media |

**Riesgo principal**: la migración `String → MacroEdgeStatus` toca serde y varios test helpers; mitigado por el alias `active` + UAT C6. Riesgo secundario: la firma de `check_orphans` ripple a call sites/tests (contenido, 1 call site de producción). **Resuelto** (D9): `expand`/`promote` bloquean ciclos con `check_dag` reutilizado — verificado que `link connect`/`link advanced` ya lo hacen; sin riesgo de persistir DAG inválido.

**Gate obligatorio antes de "completado"** (vibe-coding-rules): `cargo check --all-targets --all-features` + `clippy --all-targets --all-features -- -D warnings` + `test --workspace` + `fmt --all -- --check`. Commit por fase (`feat(long-arrow-s2):`), push al cerrar el slice, actualizar PROGRESS.md.

---

## Fuera de Alcance (observaciones / slices futuros)

- `macro_assume_edit` (in-place preservando `MASM-id`) — diferido (D8); revisitar si surge fricción real editando supuestos de reserva.
- Content-hash para staleness de *texto* de supuestos interiores — diferido (D8/YAGNI).
- `promote`/`expand` sobre **Overlays** (no solo reservas) — deliberadamente fuera (D5); `path replace` ya cubre la reestructuración de overlays de collapse.
- **Observación heredada de Slice-1** (fuera de scope, fix trivial aparte): la descripción del tool MCP `ltp/path_explode` sigue diciendo "Explode a macro-link back to its original sub-graph" cuando explota un assumption de un edge normal en un INT. Doc desalineada.
