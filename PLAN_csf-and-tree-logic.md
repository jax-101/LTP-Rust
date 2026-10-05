# PLAN — `CSF` como tipo de nodo + lógica de árbol derivada del tipo (GT = necesidad)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

> Estado: **aprobado** (2026-10-06) — ejecución **Subagent-driven** (`superpowers:subagent-driven-development`), empezando por la Tarea 1.

**Objetivo:** añadir `CSF` (Critical Success Factor) al pool de tipos de nodo y hacer que la lógica de cada árbol (y de sus edges) se derive siempre de su tipo — GT/EC/PRT = necesidad, CRT/FRT/TT = suficiencia —, con lo que se corrigen los falsos positivos CLR #4 y el orden por defecto de `tree walk`.

**Arquitectura:** `TreeType::logic()` pasa a ser la única fuente de verdad. `Tree::normalize_logic()` (dominio puro e idempotente) se invoca en el único punto de deserialización (`FsStorage::load_tree`): los GT legacy guardados como suficiencia se corrigen **en memoria al leer** y se persisten corregidos en la **siguiente mutación**, sin escrituras fuera del historial (ADR-009). Todos los creadores de edges heredan `tree.logic`; los edges de `nbr_branches` son siempre `SUFFICIENCY`.

**Tech Stack:** Rust (edición del crate), `serde`/`serde_json`, `clap`, tests E2E que spawnean `CARGO_BIN_EXE_ltp` / `CARGO_BIN_EXE_ltp-mcp` + `tempfile`.

**Spec:** [CLR_SPEC.md](CLR_SPEC.md) §1.2 (árboles de suficiencia vs. necesidad), [ENGINE_SPEC.md](ENGINE_SPEC.md) §2.2 / §2.3 / §2.4 / §3 (validate) / vocabulario de `logic`, [ADR.md](ADR.md) ADR-009 (undo con SHA-256), ADR-010, ADR-013 (precedente de migración serde sin reescritura). Decisiones de esta sesión en §"Decisiones" más abajo; el ADR-014 nuevo (Tarea 3) las fija.

## Restricciones globales

- Prohibido `.unwrap()` / `.expect()` en código de producción (en tests sí).
- `///` en todo ítem `pub` nuevo. Comentarios en el idioma del fichero que se toca (dominio `tree/types.rs`, `macro_edge/` en español; `link/`, `validate/`, `tree/commands.rs` y `tests/` en inglés).
- Sin crates nuevas. Sin `HashMap` en nada que se serialice. Sin `.clone()` nuevos para esquivar al borrow checker (`Logic`/`TreeLogic`/`TreeType` son `Copy`).
- **Ningún camino de lectura escribe en disco** (ADR-009: una reescritura fuera del historial provoca `UNDO_STATE_DIVERGED`).
- El shape de `CommandOutput` no cambia. Los goldens `contract/*.json` (todos CRT) **no deben cambiar**: si `cargo test --test contract` falla, es una regresión, no un golden a regenerar.
- El número de tools MCP sigue en **71** (no se añade ninguno).
- Docs en español. Planes/docs en la raíz del proyecto.
- Gate de verificación antes de cerrar cada tarea con código:
  ```bash
  cargo check --all-targets --all-features
  cargo clippy --all-targets --all-features -- -D warnings
  cargo test --workspace
  cargo fmt --all -- --check
  ```
- Commit al final de cada tarea, solo con los ficheros de esa tarea. **No** añadir al stage los 5 docs pendientes ajenos a este plan (`RFC-002_meta-graph.md`, `RFC-002_appendix_20-transitions.md`, `RFC-002_appendix_use-cases.md`, `PLAN_long-arrow-slice2.md`, `USAGE_GUIDE.md`). Prohibido `git add -A` / `git add .`.
- Mensajes: `feat(FX):` / `fix(FX):` / `docs:` y terminar con la línea `Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>`.

## Foco de revisión

Los cinco modos de fallo que la spec implica y que un usuario encontraría antes, con la tarea cuyo test los fija:

1. **GT legacy + comando de solo lectura** (`tree list`, `tree walk`, `validate`, `link inspect`): debe verse como necesidad **y** los bytes del fichero no deben cambiar → T3.2 (Tarea 3).
2. **GT legacy + mutación + `undo`**: la mutación persiste la versión corregida; el `undo` restaura byte a byte el fichero legacy, sin `UNDO_STATE_DIVERGED` → T3.3 (Tarea 3).
3. **`link connect --nbr` en un árbol de necesidad**: el tronco hereda `NECESSITY`, pero la rama NBR sigue en `SUFFICIENCY` → T4.2 (Tarea 4).
4. **Workspace legacy sin la clave `CSF` en `counters.json`, o con contadores perdidos**: `node add --type CSF` arranca en `CSF-001` y la reconstrucción por escaneo no reutiliza IDs → UAT 2a.12 / 2a.13 (Tarea 1).
5. **MCP `ltp/tree_walk` sin `order`**: mismo default que la CLI (hoy cada shell tiene su propio `"topological"` en duro) → T7.4 (Tarea 7).

> **Trampa de test (léelo antes de escribir asertos de persistencia):** como `load_tree` normaliza al leer, cualquier comando del motor (incluido `link inspect`) **enmascara** un creador de edges roto, y la mutación siguiente también lo “arregla” al guardar. Los asertos de lógica de edges recién creados leen el fichero `trees/<id>.json` **crudo** y **justo después** del comando que los creó.

## Decisiones (sesión de diseño, Six Thinking Hats)

- **CSF** = nuevo `NodeType::Csf` (prefijo `CSF`). Jerarquía GT de Dettmer: `GOAL ← CSF ← NC` (edges hijo → padre, como en los borradores reales). Sin reglas de rol nuevas (el motor no evalúa semántica, ADR-001). Cambio aditivo (MINOR en el próximo release).
- **GT = necesidad**: corrige `logic_for_type`, que agrupaba GT con suficiencia en contra de CLR_SPEC §1.2.
- **`link connect` (y todo creador de edges) hereda la lógica del árbol**, tal como pidió el usuario. Las ramas NBR son siempre suficiencia.
- **GT ya guardados en disco** (borradores, no críticos): opción **C** — normalizar al leer en memoria y persistir de forma perezosa en la siguiente mutación. Descartadas: **A** (solo hacia delante → árboles mixtos para siempre), **B** (warning de lógica inconsistente en `validate` → con C nunca se dispara: código muerto), **D** (comando de migración → tool MCP nº 72; YAGNI para borradores), **E** (reescribir al leer → rompe el undo, ADR-009).
- **CLR #4** (`CLR4_INSUFFICIENT_CAUSE`, `CLR4_5_IMPLICIT_OR_REVIEW`, `CLR4_5_EXCESSIVE_AND_INPUTS`) solo en árboles de suficiencia.
- **`tree walk`** sin `--order`: `reverse` en necesidad y `topological` en suficiencia (lo que ENGINE_SPEC ya documenta). `--order` explícito manda.
- **Diferido** (fuera de alcance): re-tipar los OBJ/REQ de los borradores a CSF/NC (`node edit` no tiene `--type`); rechazar valores inválidos de `--order` (hoy se aceptan en silencio); eliminar el campo almacenado `edge.logic` (RFC-002 Capa 1, sería MAJOR); el `.expect` preexistente de `link/commands.rs:339`; CLR #5/MAG en árboles de necesidad; CLR #4 sobre los edges de `nbr_branches` (hoy `validate/mod.rs:184-190` solo lintea `tree.edges`, así que una rama NBR, que siempre es suficiencia, nunca se audita por CLR #4, ni antes ni después de este plan); la redacción de ENGINE_SPEC sobre DAG (:363).

## Mapa de ficheros

| Fichero | Tarea | Cambio |
|---------|-------|--------|
| `src/node/types.rs` | 1 | variante `Csf` + prefijo `CSF` |
| `src/node/commands.rs` | 1 | `parse_node_type` acepta `CSF` |
| `src/workspace/counters.rs` | 1 | `"CSF"` en `ENTITY_TYPES` |
| `src/mcp/tools.rs` | 1, 7 | descripción de tipos de nodo; descripción de `order` |
| `tests/fase_02a.rs`, `tests/fase_01.rs` | 1 | UATs 2a.10–2a.14; `CSF` en UAT 1.6 |
| `src/tree/types.rs` | 2 | `TreeType::logic()`, `From<TreeLogic> for Logic`, `Tree::normalize_logic()` + unit tests |
| `src/tree/commands.rs` | 3, 7 | `tree new` usa `tree_type.logic()` (se borra `logic_for_type`); default de `walk` |
| `src/storage.rs`, `src/workspace/fs_storage.rs` | 3 | contrato + normalización en `load_tree` |
| `tests/tree_logic.rs` (nuevo) | 3–7 | suite E2E adversarial del invariante |
| `ADR.md`, `INTEGRATION.md` | 3 | ADR-014; enums de `tree_type` |
| `src/link/commands.rs` | 4 | `link connect` hereda lógica (tronco) / `SUFFICIENCY` (NBR) |
| `src/link/advanced.rs`, `src/path/mod.rs`, `src/macro_edge/mod.rs` | 5 | `insert-between`, `group`, `path replace` heredan; se borra `edge_logic` |
| `src/validate/mod.rs`, `CLR_SPEC.md` | 6 | gate de CLR #4 por lógica |
| `src/main.rs`, `src/mcp/dispatch.rs` | 7 | `order` opcional extremo a extremo |
| `ENGINE_SPEC.md`, `CHANGELOG.md` | 1–7 | cada tarea documenta lo suyo |
| `PROGRESS.md` | 8 | dashboard + historial + telemetría |

---

### Tarea 1: `NodeType::Csf`

**Files:**
- Modify: `src/node/types.rs:38-75` (enum `NodeType` + `prefix()`)
- Modify: `src/node/commands.rs:61-81` (`parse_node_type`)
- Modify: `src/workspace/counters.rs:11-14` (`ENTITY_TYPES`)
- Modify: `src/mcp/tools.rs:34` (descripción del param `type` de `ltp/node_add`)
- Modify: `ENGINE_SPEC.md:78`, `CHANGELOG.md` (`[Unreleased]` → `### Added`)
- Test: `tests/fase_02a.rs` (añadir al final), `tests/fase_01.rs:203-206`

**Interfaces:**
- Consumes: nada.
- Produces: tipo de nodo `CSF` aceptado por `ltp node add --type CSF|csf` y `ltp/node_add`; IDs `CSF-001…`; `"type": "CSF"` en `nodes/CSF-xxx.json`; `"node_type": "CSF"` en la salida. Las tareas 3–7 crean nodos `CSF` en sus fixtures.

- [ ] **Step 1: Escribir los tests que fallan** — añadir al final de `tests/fase_02a.rs` (usa los helpers `run_ltp` y `setup_workspace` del propio fichero):

```rust
/// UAT 2a.10: node add --type CSF creates CSF-001 (Critical Success Factor, GT middle level).
#[test]
fn uat_2a_10_node_add_csf() {
    let tmp = tempfile::tempdir().expect("failed to create tempdir");
    let dir = tmp.path();
    setup_workspace(dir);

    let (json, code) = run_ltp(dir, &["node", "add", "Entregas fiables", "--type", "CSF"]);
    assert_eq!(code, 0, "{json:?}");
    assert_eq!(json["data"]["id"], "CSF-001");
    assert_eq!(json["data"]["node_type"], "CSF");

    let on_disk: Value = serde_json::from_str(
        &std::fs::read_to_string(dir.join("nodes/CSF-001.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(on_disk["type"], "CSF");

    // Case-insensitive, like every other node type.
    let (json, code) = run_ltp(dir, &["node", "add", "Costes controlados", "--type", "csf"]);
    assert_eq!(code, 0, "{json:?}");
    assert_eq!(json["data"]["id"], "CSF-002");
}

/// UAT 2a.11: node list --type CSF returns only CSF nodes.
#[test]
fn uat_2a_11_node_list_filter_csf() {
    let tmp = tempfile::tempdir().expect("failed to create tempdir");
    let dir = tmp.path();
    setup_workspace(dir);

    for (label, ty) in [
        ("Meta", "GOAL"),
        ("Entregas fiables", "CSF"),
        ("Hay transporte suficiente", "NC"),
        ("Costes controlados", "CSF"),
    ] {
        let (json, code) = run_ltp(dir, &["node", "add", label, "--type", ty]);
        assert_eq!(code, 0, "{json:?}");
    }

    let (json, code) = run_ltp(dir, &["node", "list", "--type", "CSF"]);
    assert_eq!(code, 0, "{json:?}");
    assert_eq!(json["data"]["count"], 2);
    assert!(json["data"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .all(|n| n["node_type"] == "CSF"));
}

/// UAT 2a.12: legacy workspace whose counters.json predates CSF (no "CSF" key):
/// node add CSF starts at CSF-001 and persists the new key.
#[test]
fn uat_2a_12_csf_legacy_counters_without_key() {
    let tmp = tempfile::tempdir().expect("failed to create tempdir");
    let dir = tmp.path();
    setup_workspace(dir);

    let counters_path = dir.join(".ltp/counters.json");
    let mut counters: Value =
        serde_json::from_str(&std::fs::read_to_string(&counters_path).unwrap()).unwrap();
    counters.as_object_mut().unwrap().remove("CSF");
    std::fs::write(&counters_path, serde_json::to_string_pretty(&counters).unwrap()).unwrap();

    let (json, code) = run_ltp(dir, &["node", "add", "Entregas fiables", "--type", "CSF"]);
    assert_eq!(code, 0, "{json:?}");
    assert_eq!(json["data"]["id"], "CSF-001");

    let after: Value =
        serde_json::from_str(&std::fs::read_to_string(&counters_path).unwrap()).unwrap();
    assert_eq!(after["CSF"], 1);
}

/// UAT 2a.13: lost counters.json — the rebuild scan recognises the CSF prefix and never
/// reuses an existing ID.
#[test]
fn uat_2a_13_csf_counter_rebuild_from_scan() {
    let tmp = tempfile::tempdir().expect("failed to create tempdir");
    let dir = tmp.path();
    setup_workspace(dir);

    run_ltp(dir, &["node", "add", "CSF uno", "--type", "CSF"]);
    run_ltp(dir, &["node", "add", "CSF dos", "--type", "CSF"]);
    std::fs::remove_file(dir.join(".ltp/counters.json")).unwrap();

    let (json, code) = run_ltp(dir, &["node", "add", "CSF tres", "--type", "CSF"]);
    assert_eq!(code, 0, "{json:?}");
    assert_eq!(json["data"]["id"], "CSF-003");
}

/// UAT 2a.14: parser boundary — a near-miss type is still rejected and creates nothing.
#[test]
fn uat_2a_14_unknown_type_near_csf_rejected() {
    let tmp = tempfile::tempdir().expect("failed to create tempdir");
    let dir = tmp.path();
    setup_workspace(dir);

    let (json, code) = run_ltp(dir, &["node", "add", "X", "--type", "CSFX"]);
    assert_eq!(code, 1);
    assert_eq!(json["success"], false);
    assert_eq!(json["errors"][0]["code"], "INVALID_NODE_TYPE");
    assert_eq!(std::fs::read_dir(dir.join("nodes")).unwrap().count(), 0);
}
```

Y en `tests/fase_01.rs:203-206` (UAT 1.6), añadir `"CSF"` a `expected_types`:

```rust
    let expected_types = [
        "UDE", "RC", "INJ", "NC", "GOAL", "OBJ", "WANT", "OBS", "IO", "INT", "DE", "REQ", "PRE",
        "CSF", "TREE", "LINK", "ASM", "NBR", "MACRO",
    ];
```

- [ ] **Step 2: Ejecutar y verificar que fallan**

Run: `cargo test --test fase_02a uat_2a_1 && cargo test --test fase_01 uat_1_6`
Expected: FALLAN `uat_2a_10`–`uat_2a_13` (`INVALID_NODE_TYPE` / `code` 1) y `uat_1_6` (`Counter for CSF should be 0`). `uat_2a_14` ya pasa (es una guarda de frontera).

- [ ] **Step 3: Implementación mínima**

`src/node/types.rs` — añadir la variante al final del enum y su prefijo:

```rust
    Req,
    Pre,
    /// Critical Success Factor: nivel intermedio del Goal Tree (`GOAL ← CSF ← NC`).
    Csf,
}
```

```rust
            Self::Pre => "PRE",
            Self::Csf => "CSF",
        }
```

`src/node/commands.rs` — en `parse_node_type`, tras `"PRE" => Ok(NodeType::Pre),`:

```rust
        "CSF" => Ok(NodeType::Csf),
```

`src/workspace/counters.rs` — `ENTITY_TYPES`:

```rust
const ENTITY_TYPES: &[&str] = &[
    "UDE", "RC", "INJ", "NC", "GOAL", "OBJ", "WANT", "OBS", "IO", "INT", "DE", "REQ", "PRE",
    "CSF", "TREE", "LINK", "ASM", "NBR", "MACRO", "KN", "MASM",
];
```

`src/mcp/tools.rs:34`:

```rust
                "type": { "type": "string", "description": "Node type (UDE, RC, INJ, NC, GOAL, OBJ, WANT, OBS, IO, INT, DE, REQ, PRE, CSF)" },
```

- [ ] **Step 4: Ejecutar y verificar que pasan**

Run: `cargo test --test fase_02a && cargo test --test fase_01 && cargo test --lib counters`
Expected: PASS (incluido el unit test existente `new_zeroed_has_all_types`, que itera `ENTITY_TYPES`).

- [ ] **Step 5: Docs**

`ENGINE_SPEC.md:78`:

```markdown
Tipos: `UDE | RC | INJ | NC | GOAL | OBJ | WANT | OBS | IO | INT | DE | REQ | PRE | CSF`

`CSF` (Critical Success Factor) es el nivel intermedio del Goal Tree: `GOAL ← CSF ← NC`. Como el resto de tipos, el motor no le impone reglas de rol ni semántica (ADR-001).
```

`CHANGELOG.md`, bajo `## [Unreleased]` → `### Added`, tras la entrada de `tree rename`:

```markdown
- **Tipo de nodo `CSF`** (Critical Success Factor), nivel intermedio del Goal Tree (`GOAL ← CSF ← NC`): `ltp node add --type CSF` / `ltp/node_add`, IDs `CSF-xxx`. Los workspaces previos (sin la clave `CSF` en `counters.json`) arrancan en `CSF-001` sin migración. Valor de enum nuevo ⇒ MINOR (RELEASE_POLICY §1).
```

- [ ] **Step 6: Gate completo** (los 4 comandos de "Restricciones globales"). Expected: verde.

- [ ] **Step 7: Commit**

```bash
git add src/node/types.rs src/node/commands.rs src/workspace/counters.rs src/mcp/tools.rs \
        tests/fase_02a.rs tests/fase_01.rs ENGINE_SPEC.md CHANGELOG.md
git commit -m "feat(F2a): tipo de nodo CSF (Critical Success Factor del Goal Tree)

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

- [ ] **Step 8: Actualizar el skill consumidor `ltp-mcp`** (fuera del repo: `~/.claude/skills/ltp-mcp/`; **no** se commitea). Hoy el skill enseña al agente LLM a construir el GT con `OBJ`/`REQ`; si no se cambia, `CSF` nunca se usará. Tres reemplazos exactos:

`~/.claude/skills/ltp-mcp/SKILL.md:130`:
```markdown
| GT | Necesidad | GOAL -> OBJ -> REQ | Definir norma |
```
→
```markdown
| GT | Necesidad | GOAL <- CSF <- NC | Definir norma |
```

`~/.claude/skills/ltp-mcp/SKILL.md:164`:
```markdown
| GT | `tree_new(gt)` -> `node_add(GOAL,OBJ,REQ)` -> `tree_attach` -> `link_connect` -> `validate` |
```
→
```markdown
| GT | `tree_new(gt)` -> `node_add(GOAL,CSF,NC)` -> `tree_attach` -> `link_connect(NC->CSF, CSF->GOAL)` -> `validate` |
```

`~/.claude/skills/ltp-mcp/trees-reference.md:7`:
```markdown
- **Estructura:** 1 GOAL (cuspide) -> 3-5 OBJ (CSFs) -> multiples REQ (NCs).
```
→
```markdown
- **Estructura:** 1 GOAL (cuspide) <- 3-5 CSF <- multiples NC (tipos de nodo `GOAL`, `CSF`, `NC`; los edges van hijo -> padre). La logica (`necessity`) la fija el motor (ADR-014).
```

La línea `:166` del EC (`node_add(REQ×2,PRE×2)`) **no cambia**: REQ/PRE son los tipos correctos del Evaporating Cloud. Verificación: `rg -n "OBJ" ~/.claude/skills/ltp-mcp/` → sin resultados.

---

### Tarea 2: Dominio — lógica derivada del tipo (`TreeType::logic`, `From<TreeLogic> for Logic`, `Tree::normalize_logic`)

Tarea pura de dominio: no cambia ningún comportamiento observable todavía (nadie llama a lo nuevo hasta la Tarea 3).

**Files:**
- Modify: `src/tree/types.rs:3` (import), tras `pub enum TreeLogic` (~:21) y tras `pub struct Tree` (~:116)
- Test: `src/tree/types.rs` (módulo `#[cfg(test)] mod tests` existente)

**Interfaces:**
- Consumes: `crate::link::{Edge, Logic}` (`Logic` es `Copy`, `#[serde(rename_all = "UPPERCASE")]`).
- Produces (las usan las tareas 3–7):
  - `pub fn TreeType::logic(self) -> TreeLogic`
  - `impl From<TreeLogic> for Logic` → `Logic::from(tree.logic)`
  - `pub fn Tree::normalize_logic(&mut self)` — idempotente.

- [ ] **Step 1: Escribir los unit tests que fallan** — añadir dentro del `mod tests` existente de `src/tree/types.rs` (ya tiene `use super::*;`):

```rust
    use crate::link::{EdgeStatus, Operator};

    fn edge(id: &str, logic: Logic) -> Edge {
        Edge {
            id: id.to_string(),
            from: vec!["NC-001".to_string()],
            to: "GOAL-001".to_string(),
            operator: Operator::Single,
            weight: None,
            status: EdgeStatus::Active,
            logic,
            assumptions: vec![],
        }
    }

    fn tree_with(
        tree_type: TreeType,
        logic: TreeLogic,
        edges: Vec<Edge>,
        nbr_edges: Vec<Edge>,
    ) -> Tree {
        Tree {
            id: "tree-x".to_string(),
            name: "x".to_string(),
            tree_type,
            logic,
            nodes: vec![],
            edges,
            macro_edges: vec![],
            feedback_edges: vec![],
            nbr_branches: vec![NbrBranch {
                id: "NBR-001".to_string(),
                source_node: "INJ-001".to_string(),
                edges: nbr_edges,
                trim_injection: None,
            }],
        }
    }

    // ADR-014: la lógica canónica de cada tipo sigue CLR_SPEC §1.2.
    #[test]
    fn tree_type_logic_follows_clr_spec() {
        for t in [TreeType::Gt, TreeType::Ec, TreeType::Prt] {
            assert_eq!(t.logic(), TreeLogic::Necessity, "{t:?}");
        }
        for t in [TreeType::Crt, TreeType::Frt, TreeType::Tt] {
            assert_eq!(t.logic(), TreeLogic::Sufficiency, "{t:?}");
        }
    }

    #[test]
    fn tree_logic_maps_to_edge_logic() {
        assert_eq!(Logic::from(TreeLogic::Sufficiency), Logic::Sufficiency);
        assert_eq!(Logic::from(TreeLogic::Necessity), Logic::Necessity);
    }

    // Legacy: GT guardado como suficiencia (pre-ADR-014) ⇒ tronco a necesidad; la NBR no cambia.
    #[test]
    fn normalize_logic_fixes_legacy_gt() {
        let mut t = tree_with(
            TreeType::Gt,
            TreeLogic::Sufficiency,
            vec![
                edge("LINK-001", Logic::Sufficiency),
                edge("LINK-002", Logic::Sufficiency),
            ],
            vec![edge("LINK-003", Logic::Sufficiency)],
        );
        t.normalize_logic();
        assert_eq!(t.logic, TreeLogic::Necessity);
        assert!(t.edges.iter().all(|e| e.logic == Logic::Necessity));
        assert_eq!(t.nbr_branches[0].edges[0].logic, Logic::Sufficiency);
    }

    // Bidireccional: un edge NECESSITY colado a mano en un CRT vuelve a SUFFICIENCY, y una rama
    // NBR mal etiquetada vuelve a SUFFICIENCY aunque el árbol sea de necesidad.
    #[test]
    fn normalize_logic_is_bidirectional_and_forces_nbr_sufficiency() {
        let mut crt = tree_with(
            TreeType::Crt,
            TreeLogic::Necessity,
            vec![edge("LINK-001", Logic::Necessity)],
            vec![],
        );
        crt.normalize_logic();
        assert_eq!(crt.logic, TreeLogic::Sufficiency);
        assert_eq!(crt.edges[0].logic, Logic::Sufficiency);

        let mut prt = tree_with(
            TreeType::Prt,
            TreeLogic::Necessity,
            vec![],
            vec![edge("LINK-002", Logic::Necessity)],
        );
        prt.normalize_logic();
        assert_eq!(prt.nbr_branches[0].edges[0].logic, Logic::Sufficiency);
    }

    #[test]
    fn normalize_logic_is_idempotent() {
        let mut t = tree_with(
            TreeType::Ec,
            TreeLogic::Sufficiency,
            vec![edge("LINK-001", Logic::Sufficiency)],
            vec![edge("LINK-002", Logic::Necessity)],
        );
        t.normalize_logic();
        let first = serde_json::to_string(&t).unwrap();
        t.normalize_logic();
        assert_eq!(serde_json::to_string(&t).unwrap(), first);
    }
```

- [ ] **Step 2: Ejecutar y verificar que fallan**

Run: `cargo test --lib tree::types`
Expected: FALLA la compilación con `no method named `logic` found for enum `TreeType``, `the trait `From<TreeLogic>` is not implemented for `Logic`` y `no method named `normalize_logic``.

- [ ] **Step 3: Implementación mínima** en `src/tree/types.rs`.

Import (línea 3):

```rust
use crate::link::{AssumptionStatus, Edge, FeedbackEdge, Logic};
```

Tras `pub enum TreeLogic { … }`:

```rust
impl TreeType {
    /// Lógica canónica del tipo de árbol (CLR_SPEC §1.2).
    ///
    /// Necesidad: GT, EC, PRT ("para lograr X, necesitamos Y"). Suficiencia: CRT, FRT, TT
    /// ("si X, entonces Y"). Única fuente de verdad: `Tree::logic` y la lógica de los edges
    /// del tronco se derivan de aquí (ADR-014).
    pub fn logic(self) -> TreeLogic {
        match self {
            Self::Gt | Self::Ec | Self::Prt => TreeLogic::Necessity,
            Self::Crt | Self::Frt | Self::Tt => TreeLogic::Sufficiency,
        }
    }
}

impl From<TreeLogic> for Logic {
    fn from(logic: TreeLogic) -> Self {
        match logic {
            TreeLogic::Sufficiency => Logic::Sufficiency,
            TreeLogic::Necessity => Logic::Necessity,
        }
    }
}
```

Tras `pub struct Tree { … }`:

```rust
impl Tree {
    /// Re-deriva la lógica del árbol y de sus edges desde `tree_type` (ADR-014).
    ///
    /// Invariante: `logic == tree_type.logic()`, cada edge del tronco lleva esa lógica y los
    /// edges de `nbr_branches` son siempre `Sufficiency` (una NBR es una rama "si-entonces").
    /// Se aplica al leer (`Storage::load_tree`), en memoria: un fichero legacy (p. ej. un GT
    /// guardado como suficiencia) se corrige al vuelo y se persiste corregido en la siguiente
    /// mutación, sin reescrituras fuera del historial que romperían el undo (ADR-009).
    /// Idempotente.
    pub fn normalize_logic(&mut self) {
        self.logic = self.tree_type.logic();
        let trunk = Logic::from(self.logic);
        for edge in &mut self.edges {
            edge.logic = trunk;
        }
        for edge in self.nbr_branches.iter_mut().flat_map(|b| b.edges.iter_mut()) {
            edge.logic = Logic::Sufficiency;
        }
    }
}
```

- [ ] **Step 4: Ejecutar y verificar que pasan**

Run: `cargo test --lib tree::types`
Expected: PASS (5 tests nuevos + los de migración serde de `MacroEdgeStatus` existentes).

- [ ] **Step 5: Gate completo.** Expected: verde. (`clippy` no debe quejarse de dead code: los ítems son `pub`.)

- [ ] **Step 6: Commit**

```bash
git add src/tree/types.rs
git commit -m "feat(F3): lógica de árbol derivada del tipo (TreeType::logic + Tree::normalize_logic)

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Tarea 3: `tree new` deriva la lógica + normalización al leer (`load_tree`) + ADR-014

Corrige el bug raíz (GT nace como suficiencia) y hace que **toda** lectura entregue el árbol normalizado. `FsStorage::load_tree` es el **único** punto de deserialización de `Tree` en el crate (verificado: `rg "Tree = serde_json"` → solo `src/workspace/fs_storage.rs:145`); `tree list`, `link inspect`, `validate`, `tree walk` y el MCP pasan todos por él.

**Files:**
- Modify: `src/tree/commands.rs:27-33` (borrar `logic_for_type`), `:229` y `:243` (callers)
- Modify: `src/workspace/fs_storage.rs:139-147` (`load_tree`)
- Modify: `src/storage.rs:32` (doc del trait)
- Create: `tests/tree_logic.rs`
- Modify (docs, Step 6): `ADR.md` (append ADR-014), `ENGINE_SPEC.md` (:112, :116, :324, :652), `INTEGRATION.md:47`, `CHANGELOG.md`

**Interfaces:**
- Consumes (Tarea 2): `TreeType::logic(self) -> TreeLogic`, `Tree::normalize_logic(&mut self)`.
- Produces: helpers de `tests/tree_logic.rs` que reutilizan las tareas 4–7 — `run_ltp`, `run_ok`, `setup`, `add_node`, `create_tree`, `attach`, `connect`, `tree_path`, `read_tree`, `write_legacy_sufficiency`, `walk_ids`, `GtChain { tree, nc, csf, goal, links: [String; 2] }`, `gt_chain(dir) -> GtChain`.

> ⚠️ **Trampa de test**: como toda lectura normaliza, `link inspect` o cualquier mutación posterior **enmascaran** un creador roto. Las aserciones sobre lo que escribe un comando deben leer el `trees/<id>.json` **crudo** (`read_tree`) inmediatamente después de ese comando.

- [ ] **Step 1: Crear `tests/tree_logic.rs` con helpers + tests que fallan**

```rust
//! ADR-014: la lógica de un árbol se deriva de su tipo (CLR_SPEC §1.2).
//!
//! Invariante: `tree.logic == tree_type.logic()`, los edges del tronco llevan esa lógica y
//! los edges de NBR son siempre `SUFFICIENCY`. Los ficheros legacy se normalizan al leer
//! (en memoria) y se persisten corregidos en la siguiente mutación.
//!
//! Every read normalizes, so `link inspect` (or any later mutation) would mask a broken
//! edge creator: assertions about what a command wrote read the raw `trees/<id>.json`
//! right after that command.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};

fn ltp_bin() -> String {
    env!("CARGO_BIN_EXE_ltp").to_string()
}

fn run_ltp(dir: &Path, args: &[&str]) -> (Value, i32) {
    let output = Command::new(ltp_bin())
        .args(args)
        .current_dir(dir)
        .output()
        .expect("failed to execute ltp binary");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let code = output.status.code().unwrap_or(-1);
    let json: Value = serde_json::from_str(&stdout).unwrap_or_else(|_| {
        panic!(
            "Failed to parse JSON.\nargs: {args:?}\nstdout: {stdout}\nstderr: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    });
    (json, code)
}

fn run_ok(dir: &Path, args: &[&str]) -> Value {
    let (json, code) = run_ltp(dir, args);
    assert_eq!(code, 0, "ltp {args:?} failed: {json}");
    json
}

fn setup(dir: &Path) {
    run_ok(dir, &["init", "--name", "TreeLogic"]);
}

fn add_node(dir: &Path, label: &str, node_type: &str) -> String {
    let json = run_ok(dir, &["node", "add", label, "--type", node_type]);
    json["data"]["id"].as_str().unwrap().to_string()
}

fn create_tree(dir: &Path, tree_type: &str, name: &str) -> String {
    let json = run_ok(dir, &["tree", "new", tree_type, name]);
    json["data"]["id"].as_str().unwrap().to_string()
}

fn attach(dir: &Path, tree: &str, nodes: &[&str]) {
    for node in nodes {
        run_ok(dir, &["tree", "attach", "--tree", tree, "--node", node]);
    }
}

fn connect(dir: &Path, tree: &str, from: &str, to: &str) -> String {
    let json = run_ok(
        dir,
        &["link", "connect", "--tree", tree, "--from", from, "--to", to],
    );
    json["data"]["created_links"][0].as_str().unwrap().to_string()
}

fn tree_path(dir: &Path, tree: &str) -> PathBuf {
    dir.join("trees").join(format!("{tree}.json"))
}

fn read_tree(dir: &Path, tree: &str) -> Value {
    serde_json::from_str(&std::fs::read_to_string(tree_path(dir, tree)).unwrap()).unwrap()
}

/// Rewrites the tree file the way a pre-ADR-014 workspace stored a GT: the tree and every
/// trunk edge as sufficiency. Returns the exact bytes written.
fn write_legacy_sufficiency(dir: &Path, tree: &str) -> String {
    let mut raw = read_tree(dir, tree);
    raw["logic"] = json!("sufficiency");
    for edge in raw["edges"].as_array_mut().unwrap() {
        edge["logic"] = json!("SUFFICIENCY");
    }
    let content = serde_json::to_string_pretty(&raw).unwrap();
    std::fs::write(tree_path(dir, tree), &content).unwrap();
    content
}

fn walk_ids(json: &Value) -> Vec<String> {
    json["data"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["id"].as_str().unwrap().to_string())
        .collect()
}

/// Minimal Dettmer GT: `NC → CSF → GOAL` (edges point child → parent).
struct GtChain {
    tree: String,
    nc: String,
    csf: String,
    goal: String,
    links: [String; 2],
}

fn gt_chain(dir: &Path) -> GtChain {
    let nc = add_node(dir, "Hay capacidad de transporte suficiente", "NC");
    let csf = add_node(dir, "Entregas fiables", "CSF");
    let goal = add_node(dir, "Entregamos en menos de 10 días", "GOAL");
    let tree = create_tree(dir, "gt", "GT Logistica");
    attach(dir, &tree, &[&nc, &csf, &goal]);
    let l1 = connect(dir, &tree, &nc, &csf);
    let l2 = connect(dir, &tree, &csf, &goal);
    GtChain {
        tree,
        nc,
        csf,
        goal,
        links: [l1, l2],
    }
}

/// T3.1: `tree new` derives the logic from the type, in the response and on disk.
#[test]
fn t3_1_tree_new_derives_logic_from_type() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup(dir);

    for (tree_type, logic) in [
        ("gt", "necessity"),
        ("ec", "necessity"),
        ("prt", "necessity"),
        ("crt", "sufficiency"),
        ("frt", "sufficiency"),
        ("tt", "sufficiency"),
    ] {
        let json = run_ok(dir, &["tree", "new", tree_type, &format!("Arbol {tree_type}")]);
        assert_eq!(json["data"]["logic"], logic, "{tree_type}: response");
        let id = json["data"]["id"].as_str().unwrap();
        assert_eq!(read_tree(dir, id)["logic"], logic, "{tree_type}: disk");
    }
}

/// T3.2 (review focus): a legacy GT reads as necessity on every read-only surface, and
/// none of them rewrites the file (an out-of-band write would break undo, ADR-009).
#[test]
fn t3_2_legacy_gt_reads_as_necessity_without_touching_disk() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup(dir);
    let gt = gt_chain(dir);
    let legacy = write_legacy_sufficiency(dir, &gt.tree);

    let list = run_ok(dir, &["tree", "list"]);
    let entry = list["data"]["trees"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["id"] == gt.tree.as_str())
        .unwrap();
    assert_eq!(entry["logic"], "necessity");

    let inspect = run_ok(dir, &["link", "inspect", &gt.links[0], "--tree", &gt.tree]);
    assert_eq!(inspect["data"]["logic"], "necessity");

    let walk = run_ok(dir, &["tree", "walk", &gt.tree, "--order", "topological"]);
    assert_eq!(walk_ids(&walk), [gt.nc.clone(), gt.csf.clone(), gt.goal.clone()]);

    run_ok(dir, &["validate", "--tree", &gt.tree]);

    assert_eq!(
        std::fs::read_to_string(tree_path(dir, &gt.tree)).unwrap(),
        legacy,
        "a read-only command rewrote the tree file"
    );
}

/// T3.3 (review focus): the first mutation persists the normalized tree, and undo
/// restores the legacy file byte for byte.
#[test]
fn t3_3_first_mutation_persists_normalized_and_undo_restores_legacy() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup(dir);
    let gt = gt_chain(dir);
    let legacy = write_legacy_sufficiency(dir, &gt.tree);

    let extra = add_node(dir, "Flota mantenida", "NC");
    attach(dir, &gt.tree, &[&extra]);

    let raw = read_tree(dir, &gt.tree);
    assert_eq!(raw["logic"], "necessity");
    let edges = raw["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 2);
    assert!(edges.iter().all(|e| e["logic"] == "NECESSITY"), "{edges:?}");

    run_ok(dir, &["undo"]);
    assert_eq!(
        std::fs::read_to_string(tree_path(dir, &gt.tree)).unwrap(),
        legacy,
        "undo must restore the legacy bytes exactly"
    );
}

/// T3.4: cloning a legacy GT writes the clone already normalized.
#[test]
fn t3_4_clone_of_legacy_gt_is_normalized() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup(dir);
    let gt = gt_chain(dir);
    write_legacy_sufficiency(dir, &gt.tree);

    let json = run_ok(dir, &["tree", "clone", &gt.tree, "--name", "GT Clon"]);
    let clone = read_tree(dir, json["data"]["new_id"].as_str().unwrap());
    assert_eq!(clone["logic"], "necessity");
    let edges = clone["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 2);
    assert!(edges.iter().all(|e| e["logic"] == "NECESSITY"), "{edges:?}");
}
```

- [ ] **Step 2: Ejecutar y verificar que fallan**

Run: `cargo test --test tree_logic`
Expected: FALLAN `t3_1` (`gt: response` → `"sufficiency"`), `t3_2` (`tree list` da `sufficiency` para el GT legacy), `t3_3` (el GT persiste `sufficiency`) y `t3_4` (clon `sufficiency`).

- [ ] **Step 3: Implementación**

`src/tree/commands.rs` — borrar entera la función errónea (líneas 27-33):

```rust
/// Determine the logic type for a given tree type.
fn logic_for_type(tree_type: TreeType) -> TreeLogic {
    match tree_type {
        TreeType::Gt | TreeType::Crt | TreeType::Frt | TreeType::Tt => TreeLogic::Sufficiency,
        TreeType::Ec | TreeType::Prt => TreeLogic::Necessity,
    }
}
```

y sustituir sus dos callers: `:229` `logic: logic_for_type(tree_type),` → `logic: tree_type.logic(),`; `:243` `let logic = logic_for_type(tree_type);` → `let logic = tree_type.logic();`. (`TreeLogic` sigue importado: se usa en `:80`, `:89`, `:206`.)

`src/workspace/fs_storage.rs` — `load_tree`:

```rust
    fn load_tree(&self, id: &str) -> Result<Tree> {
        let path = self.trees_dir().join(format!("{}.json", id));
        if !path.exists() {
            return Err(LtpError::TreeNotFound(id.to_string()));
        }
        let content = fs::read_to_string(&path)?;
        let mut tree: Tree = serde_json::from_str(&content)?;
        // ADR-014: logic is derived from the tree type. Legacy files are fixed in memory
        // only; the corrected tree reaches disk on the next mutation (never on a read).
        tree.normalize_logic();
        Ok(tree)
    }
```

`save_tree` **no cambia**. `src/storage.rs:32`:

```rust
    /// Load a tree by its ID.
    ///
    /// Implementations must return the tree normalized via `Tree::normalize_logic`
    /// (ADR-014), without writing back to storage.
    fn load_tree(&self, id: &str) -> Result<Tree>;
```

- [ ] **Step 4: Ejecutar y verificar que pasan**

Run: `cargo test --test tree_logic && cargo test --test fase_03`
Expected: PASS (4 tests nuevos; `uat_3_1`/`uat_3_2` siguen verdes: CRT suficiencia, EC necesidad).

- [ ] **Step 5: Gate completo.** Expected: verde. (El doc de `macro_edge/mod.rs:27` menciona `logic_for_type` en texto plano, no como intra-doc link; la Tarea 5 borra ese doc.)

- [ ] **Step 6: Docs**

`ADR.md` — añadir al final, mismo formato de texto plano que ADR-013 (línea de título, luego `Contexto` / `Decisión` / `Justificación` / `Consecuencias`):

```markdown
ADR-014: Lógica de Árbol Derivada del Tipo (Normalización al Leer, Persistencia Perezosa)

Contexto

CLR_SPEC §1.2 fija la lógica de cada árbol: necesidad en GT, EC y PRT ("para lograr X, necesitamos Y"); suficiencia en CRT, FRT y TT ("si X, entonces Y"). El motor derivaba mal esa lógica: `tree new gt` creaba el árbol como suficiencia, y todos los creadores de edges (`link connect`, `insert-between`, `group`, `path replace`) escribían `SUFFICIENCY` fijo, incluso dentro de un EC o un PRT. Consecuencias observadas sobre datos reales: `validate` emitía falsos positivos de CLR #4 en árboles de necesidad (un GT con `CLR4_5_IMPLICIT_OR_REVIEW` ×5; un EC con `CLR4_INSUFFICIENT_CAUSE` ×2 + `CLR4_5_IMPLICIT_OR_REVIEW` ×1), y `tree walk` sin `--order` recorría siempre en `topological`, en contra del default por lógica que ENGINE_SPEC ya documentaba. Además existen borradores GT en disco guardados como suficiencia.

Se evaluaron cinco opciones para los árboles ya guardados:
- A. Corregir solo hacia delante: los árboles existentes quedan mixtos para siempre.
- B. Warning de lógica inconsistente en `validate`: con la opción C nunca se dispararía (código muerto).
- C. Normalizar al leer, en memoria, y persistir de forma perezosa en la siguiente mutación.
- D. Comando de migración explícito: añade el tool MCP nº 72; YAGNI para borradores no críticos.
- E. Reescribir el fichero al leer: una escritura fuera del historial rompe el undo (ADR-009, `UNDO_STATE_DIVERGED`).

Decisión

Se adopta C. La lógica deja de ser un dato independiente y pasa a derivarse del tipo:
- Invariante: `tree.logic == tree_type.logic()`; los edges del tronco llevan `Logic::from(tree.logic)`; los edges de `nbr_branches` son siempre `SUFFICIENCY` (una NBR es una rama "si-entonces" en cualquier árbol).
- `TreeType::logic()` es la única fuente de verdad. Todos los creadores de edges heredan la lógica del árbol.
- `Tree::normalize_logic()` (dominio puro, idempotente) se invoca desde `Storage::load_tree`, el único punto de deserialización. Las lecturas nunca escriben; el árbol corregido llega a disco con la siguiente mutación, dentro del historial. "Siguiente mutación" significa cualquier comando que guarde ese árbol, aunque no vaya dirigido a él (p. ej. un comando de nodo que reescribe varios árboles): la normalización viaja en esa misma entrada de undo, que es correcta porque ambos shells capturan el workspace completo (`snapshot_workspace_paths`), pero cuyo diff incluye los cambios de lógica.
- CLR #4 (`CLR4_INSUFFICIENT_CAUSE`, `CLR4_5_IMPLICIT_OR_REVIEW`, `CLR4_5_EXCESSIVE_AND_INPUTS`) solo se evalúa en árboles de suficiencia.
- `tree walk` sin `--order` usa `reverse` en necesidad y `topological` en suficiencia; `--order` explícito manda.

Justificación
- CLR_SPEC §1.2 es la fuente de verdad semántica; el motor solo la aplica de forma determinista (ADR-001).
- ADR-009: normalizar en memoria preserva los checksums del undo; el primer undo tras la mutación restaura el fichero legacy byte a byte.
- ADR-010: CLR #4 es semántica no bloqueante; aplicarla donde no corresponde solo genera ruido.
- Precedente ADR-013: migración retrocompatible en la deserialización, sin migraciones manuales.
- Arquitectura de dos velocidades: la regla vive en el dominio (`Tree`), no en `FsStorage`; un backend futuro solo tiene que llamar a `normalize_logic` en su `load_tree`.

Consecuencias
- Positivas: invariante total (ningún camino de lectura entrega un árbol inconsistente); ningún tool MCP nuevo; cero falsos positivos de CLR #4 en GT/EC/PRT; los borradores legacy se arreglan solos al primer uso.
- Negativas: el disco conserva el contenido legacy hasta la siguiente mutación, y esa primera mutación produce un ruido puntual en el `git diff` (todos los edges cambian de lógica). Coste O(E) por cada `load_tree`. El campo almacenado `edge.logic` se mantiene como deuda (redundante con `tree_type`; su eliminación es RFC-002 Capa 1 y sería MAJOR). El cambio de default de `tree walk` es visible para consumidores que no pasan `--order` en GT/EC/PRT (mitigación: pasar `--order topological` explícito).
```

`ENGINE_SPEC.md`:
- Tras el encabezado de `:112` (`#### \`ltp tree new <gt|crt|ec|frt|prt|tt> "<nombre>"\``), añadir el párrafo: `La lógica del árbol se deriva del tipo (CLR_SPEC §1.2, ADR-014): \`necessity\` en GT/EC/PRT y \`sufficiency\` en CRT/FRT/TT. No es configurable.`
- `:116` (`tree list`): `` `crt` | `ec` | `frt` | `prt` `` → `` `gt` | `crt` | `ec` | `frt` | `prt` | `tt` ``.
- `:324` (`macro expand`): `(\`SUFFICIENCY\` en GT/CRT/FRT/TT; \`NECESSITY\` en EC/PRT)` → `(\`SUFFICIENCY\` en CRT/FRT/TT; \`NECESSITY\` en GT/EC/PRT)`.
- `:652`: `Vocabulario de \`logic\` en tree: \`sufficiency | necessity\`` → añadir a continuación: `Se deriva de \`tree_type\` (ADR-014): \`necessity\` en GT/EC/PRT, \`sufficiency\` en CRT/FRT/TT. Los edges del tronco llevan la misma lógica (\`NECESSITY\`/\`SUFFICIENCY\`); los edges de \`nbr_branches\` son siempre \`SUFFICIENCY\`. Un fichero legacy inconsistente se normaliza al leer y se persiste corregido en la siguiente mutación.`

`INTEGRATION.md:47`: `` `tree_type` → `crt|ec|frt|prt` `` → `` `tree_type` → `gt|crt|ec|frt|prt|tt` ``.

`CHANGELOG.md` — bajo `## [Unreleased]`, tras `### Added`, crear la sección:

```markdown
### Fixed

- **Lógica de los Goal Trees** (ADR-014): `tree new gt` creaba el árbol con lógica `sufficiency`; ahora es `necessity`, como EC y PRT (CLR_SPEC §1.2). La lógica de un árbol se deriva siempre de su tipo. Los GT ya guardados se leen corregidos y se persisten así en su siguiente mutación (el `undo` posterior restaura el fichero original byte a byte).
```

- [ ] **Step 7: Gate completo.** Expected: verde.

- [ ] **Step 8: Commit**

```bash
git add src/tree/commands.rs src/workspace/fs_storage.rs src/storage.rs tests/tree_logic.rs ADR.md ENGINE_SPEC.md INTEGRATION.md CHANGELOG.md
git commit -m "fix(F3): GT nace como necesidad; normalización de lógica al leer (ADR-014)

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Tarea 4: `link connect` hereda la lógica del árbol (NBR siempre suficiencia)

**Files:**
- Modify: `src/link/commands.rs:257` (comentario `// Build edges based on to[] cardinality`), `:291` (multi-destino), `:325` (destino único)
- Test: `tests/tree_logic.rs`
- Modify (docs): `ENGINE_SPEC.md:164-170`, `CHANGELOG.md`

**Interfaces:**
- Consumes: `impl From<TreeLogic> for Logic` (Tarea 2); `tree` ya cargado (y normalizado) en `:116` como `let mut tree`; `nbr_id: Option<&str>` (parámetro, `:90`). `Logic` ya está importado (`:4`).
- Produces: helpers de test `edge_logic_on_disk(dir, tree, link) -> String` e `ids(&Value) -> Vec<String>` (los usan las tareas 5 y 6).

- [ ] **Step 1: Añadir helpers y tests que fallan** a `tests/tree_logic.rs` (helpers junto al resto de helpers, tests al final):

```rust
fn ids(links: &Value) -> Vec<String> {
    links
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l.as_str().unwrap().to_string())
        .collect()
}

/// Logic of `link` exactly as stored on disk (trunk or NBR), bypassing the normalizing read.
fn edge_logic_on_disk(dir: &Path, tree: &str, link: &str) -> String {
    let raw = read_tree(dir, tree);
    let nbr_edges = raw["nbr_branches"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|b| b["edges"].as_array().into_iter().flatten());
    raw["edges"]
        .as_array()
        .unwrap()
        .iter()
        .chain(nbr_edges)
        .find(|e| e["id"] == link)
        .unwrap_or_else(|| panic!("{link} not found in {tree}"))["logic"]
        .as_str()
        .unwrap()
        .to_string()
}

/// T4.1: every `link connect` shape (SINGLE, multi-destination, AND) writes NECESSITY in a GT.
#[test]
fn t4_1_connect_in_gt_writes_necessity_for_every_shape() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup(dir);
    let nc1 = add_node(dir, "Flota disponible", "NC");
    let nc2 = add_node(dir, "Conductores formados", "NC");
    let nc3 = add_node(dir, "Rutas planificadas", "NC");
    let csf1 = add_node(dir, "Entregas fiables", "CSF");
    let csf2 = add_node(dir, "Entregas rápidas", "CSF");
    let goal = add_node(dir, "Clientes satisfechos", "GOAL");
    let tree = create_tree(dir, "gt", "GT Formas");
    attach(dir, &tree, &[&nc1, &nc2, &nc3, &csf1, &csf2, &goal]);

    let single = connect(dir, &tree, &csf1, &goal);
    assert_eq!(edge_logic_on_disk(dir, &tree, &single), "NECESSITY", "SINGLE");

    let to = format!("{csf1},{csf2}");
    let json = run_ok(
        dir,
        &["link", "connect", "--tree", &tree, "--from", &nc1, "--to", &to],
    );
    let multi = ids(&json["data"]["created_links"]);
    assert_eq!(multi.len(), 2);
    for link in &multi {
        assert_eq!(edge_logic_on_disk(dir, &tree, link), "NECESSITY", "multi-dest {link}");
    }

    let from = format!("{nc2},{nc3}");
    let json = run_ok(
        dir,
        &[
            "link", "connect", "--tree", &tree, "--from", &from, "--to", &csf2, "--operator",
            "AND",
        ],
    );
    let and = ids(&json["data"]["created_links"]);
    assert_eq!(and.len(), 1);
    assert_eq!(edge_logic_on_disk(dir, &tree, &and[0]), "NECESSITY", "AND");
}

/// T4.2 (review focus): an NBR branch stays SUFFICIENCY even inside a necessity tree (PRT),
/// while the trunk inherits NECESSITY.
#[test]
fn t4_2_nbr_edge_stays_sufficiency_in_prt() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup(dir);
    let obs = add_node(dir, "Falta de stock", "OBS");
    let io = add_node(dir, "Inventario visible", "IO");
    let inj = add_node(dir, "Kanban de reposición", "INJ");
    let ude = add_node(dir, "Sobrecoste de almacén", "UDE");
    let tree = create_tree(dir, "prt", "PRT Stock");
    attach(dir, &tree, &[&obs, &io, &inj]);

    let trunk = connect(dir, &tree, &obs, &io);
    assert_eq!(edge_logic_on_disk(dir, &tree, &trunk), "NECESSITY", "trunk");

    let json = run_ok(dir, &["nbr", "add", "--tree", &tree, "--source-node", &inj]);
    let nbr = json["data"]["nbr_id"].as_str().unwrap().to_string();
    let json = run_ok(
        dir,
        &[
            "link", "connect", "--tree", &tree, "--nbr", &nbr, "--from", &inj, "--to", &ude,
        ],
    );
    let nbr_link = ids(&json["data"]["created_links"]);
    assert_eq!(edge_logic_on_disk(dir, &tree, &nbr_link[0]), "SUFFICIENCY", "NBR edge");
    assert_eq!(edge_logic_on_disk(dir, &tree, &trunk), "NECESSITY", "trunk after NBR");
}

/// T4.3: control — a CRT keeps writing SUFFICIENCY.
#[test]
fn t4_3_connect_in_crt_writes_sufficiency() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup(dir);
    let rc = add_node(dir, "Previsión manual", "RC");
    let ude = add_node(dir, "Roturas de stock", "UDE");
    let tree = create_tree(dir, "crt", "CRT Stock");
    attach(dir, &tree, &[&rc, &ude]);

    let link = connect(dir, &tree, &rc, &ude);
    assert_eq!(edge_logic_on_disk(dir, &tree, &link), "SUFFICIENCY");
}
```

- [ ] **Step 2: Ejecutar y verificar que fallan**

Run: `cargo test --test tree_logic t4_`
Expected: FALLAN `t4_1` (`SINGLE`: `"SUFFICIENCY"` ≠ `"NECESSITY"`) y `t4_2` (`trunk`); `t4_3` PASA (control).

- [ ] **Step 3: Implementación** en `src/link/commands.rs`. Insertar justo después del comentario `// Build edges based on to[] cardinality` (antes de `let mut new_edges`):

```rust
    // ADR-014: trunk edges inherit the tree's logic; an NBR branch is always sufficiency.
    let edge_logic = if nbr_id.is_some() {
        Logic::Sufficiency
    } else {
        Logic::from(tree.logic)
    };
```

y en los dos `Edge { … }` (`:291` y `:325`): `logic: Logic::Sufficiency,` → `logic: edge_logic,`.

- [ ] **Step 4: Ejecutar y verificar que pasan**

Run: `cargo test --test tree_logic && cargo test --test fase_10 && cargo test --test macro_lifecycle`
Expected: PASS (las UATs de NBR de `fase_10` y el B5 de `macro_lifecycle` siguen verdes).

- [ ] **Step 5: Docs.** `ENGINE_SPEC.md` — en la lista de `link connect` (tras el bullet `--weight`), añadir:

```markdown
- La lógica del edge la hereda del árbol (ADR-014): `NECESSITY` en GT/EC/PRT, `SUFFICIENCY` en CRT/FRT/TT. Con `--nbr`, el edge es siempre `SUFFICIENCY` (una NBR es una rama "si-entonces").
```

`CHANGELOG.md` — en `### Fixed`:

```markdown
- **Lógica de los edges** (ADR-014): `link connect` escribía siempre `SUFFICIENCY`, incluso en GT/EC/PRT. Ahora el edge hereda la lógica del árbol; los edges de una rama NBR (`--nbr`) son siempre `SUFFICIENCY`.
```

- [ ] **Step 6: Gate completo.** Expected: verde.

- [ ] **Step 7: Commit**

```bash
git add src/link/commands.rs tests/tree_logic.rs ENGINE_SPEC.md CHANGELOG.md
git commit -m "fix(F4): link connect hereda la lógica del árbol (NBR siempre suficiencia)

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Tarea 5: resto de creadores de edges (`insert-between`, `group`, `path replace`) + limpieza de `macro_edge`

Mismo bug que la Tarea 4 en los demás comandos que **acuñan** edges nuevos. Los que **copian** un edge existente (`logic: edge.logic` en `advanced.rs:817`, `:1242`, `:1463`; `original_logic` en `path/mod.rs:660/672`) ya son correctos: copian de un árbol normalizado al leer.

**Files:**
- Modify: `src/link/advanced.rs:560-572` (`single_edge`), `:636` aprox. (tras cargar el árbol en `execute_link_insert_between`), `:732`, `:735`, `:777`, `:820`, `:1076`
- Modify: `src/path/mod.rs:956`, `:967`
- Modify: `src/macro_edge/mod.rs:20` (import), `:23-33` (borrar `edge_logic`), `:305` (doc), `:404`, `:741`
- Test: `tests/tree_logic.rs`
- Modify (docs): `CHANGELOG.md`

**Interfaces:**
- Consumes: `impl From<TreeLogic> for Logic` (Tarea 2); helpers `ids`, `edge_logic_on_disk` (Tarea 4), `gt_chain`, `connect`, `attach` (Tarea 3).
- Produces: `fn single_edge(id: String, from: String, to: String, logic: Logic) -> Edge` (privada de `advanced.rs`). Helper de test `assert_all_necessity(dir, tree, links: &Value, ctx: &str)`.

- [ ] **Step 1: Añadir helper y tests que fallan** a `tests/tree_logic.rs`:

```rust
/// Asserts that every link id in `links` (a JSON array) is stored as NECESSITY on disk.
fn assert_all_necessity(dir: &Path, tree: &str, links: &Value, ctx: &str) {
    let created = ids(links);
    assert!(!created.is_empty(), "{ctx}: no links created");
    for link in &created {
        assert_eq!(edge_logic_on_disk(dir, tree, link), "NECESSITY", "{ctx}: {link}");
    }
}

/// T5.1: the three `link insert-between` modes write NECESSITY in a GT.
#[test]
fn t5_1_insert_between_in_gt_writes_necessity_in_every_mode() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup(dir);
    let nc1 = add_node(dir, "Flota disponible", "NC");
    let nc2 = add_node(dir, "Conductores formados", "NC");
    let csf = add_node(dir, "Entregas fiables", "CSF");
    let goal = add_node(dir, "Clientes satisfechos", "GOAL");
    let x1 = add_node(dir, "Mantenimiento preventivo", "NC");
    let x2 = add_node(dir, "Operación estable", "NC");
    let x3 = add_node(dir, "Cumplimiento de plazos", "CSF");
    let tree = create_tree(dir, "gt", "GT Insertar");
    attach(dir, &tree, &[&nc1, &nc2, &csf, &goal, &x1, &x2, &x3]);

    let from = format!("{nc1},{nc2}");
    let json = run_ok(
        dir,
        &[
            "link", "connect", "--tree", &tree, "--from", &from, "--to", &csf, "--operator",
            "AND",
        ],
    );
    let and = ids(&json["data"]["created_links"]).remove(0);
    let single = connect(dir, &tree, &csf, &goal);

    // AND edge, --insert-after-cause: mints `nc1 → x1`.
    let json = run_ok(
        dir,
        &[
            "link", "insert-between", "--tree", &tree, "--link", &and, "--node", &x1,
            "--insert-after-cause", &nc1,
        ],
    );
    assert_all_necessity(dir, &tree, &json["data"]["created_links"], "after-cause");

    // Same AND edge, --insert-before-effect: mints `(x1, nc2) → x2` and `x2 → csf`.
    let json = run_ok(
        dir,
        &[
            "link", "insert-between", "--tree", &tree, "--link", &and, "--node", &x2,
            "--insert-before-effect",
        ],
    );
    assert_all_necessity(dir, &tree, &json["data"]["created_links"], "before-effect");

    // SINGLE edge: `csf → goal` becomes `csf → x3 → goal`.
    let json = run_ok(
        dir,
        &[
            "link", "insert-between", "--tree", &tree, "--link", &single, "--node", &x3,
        ],
    );
    assert_all_necessity(dir, &tree, &json["data"]["created_links"], "single");
}

/// T5.2: `link group` mints the AND edge as NECESSITY in a GT.
#[test]
fn t5_2_group_in_gt_writes_necessity() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup(dir);
    let nc1 = add_node(dir, "Flota disponible", "NC");
    let nc2 = add_node(dir, "Conductores formados", "NC");
    let csf = add_node(dir, "Entregas fiables", "CSF");
    let tree = create_tree(dir, "gt", "GT Agrupar");
    attach(dir, &tree, &[&nc1, &nc2, &csf]);
    let l1 = connect(dir, &tree, &nc1, &csf);
    let l2 = connect(dir, &tree, &nc2, &csf);

    let links = format!("{l1},{l2}");
    let json = run_ok(
        dir,
        &["link", "group", "--tree", &tree, "--links", &links, "--operator", "AND"],
    );
    let grouped = json["data"]["created_link"].as_str().unwrap();
    assert_eq!(edge_logic_on_disk(dir, &tree, grouped), "NECESSITY");
}

/// T5.3: `path replace` mints both new edges as NECESSITY in a GT.
#[test]
fn t5_3_path_replace_in_gt_writes_necessity() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup(dir);
    let gt = gt_chain(dir);
    let by = add_node(dir, "Entregas trazables", "CSF");

    let json = run_ok(
        dir,
        &[
            "path", "collapse", "--tree", &gt.tree, "--from", &gt.nc, "--to", &gt.goal,
            "--label", "Resumen",
        ],
    );
    let macro_id = json["data"]["macro_edge_id"].as_str().unwrap().to_string();

    let json = run_ok(
        dir,
        &[
            "path", "replace", "--tree", &gt.tree, "--macro-link", &macro_id, "--by-node", &by,
        ],
    );
    let new_links = &json["data"]["new_links"];
    assert_eq!(ids(new_links).len(), 2);
    assert_all_necessity(dir, &gt.tree, new_links, "path replace");
}
```

- [ ] **Step 2: Ejecutar y verificar que fallan**

Run: `cargo test --test tree_logic t5_`
Expected: FALLAN los tres (`"SUFFICIENCY"` ≠ `"NECESSITY"`; en `t5_1` el primer fallo es `after-cause`).

- [ ] **Step 3: Implementación**

`src/link/advanced.rs` — `single_edge` recibe la lógica:

```rust
/// A freshly minted SINGLE edge with no operator-specific metadata, carrying the
/// tree's edge logic (ADR-014).
fn single_edge(id: String, from: String, to: String, logic: Logic) -> Edge {
    Edge {
        id,
        from: vec![from],
        to,
        operator: Operator::Single,
        weight: None,
        status: EdgeStatus::Active,
        logic,
        assumptions: vec![],
    }
}
```

En `execute_link_insert_between`, justo después del `match storage.load_tree(tree_id) { … };` (~`:636`):

```rust
    // ADR-014: freshly minted edges inherit the tree's logic.
    let edge_logic = Logic::from(tree.logic);
```

y en ese mismo cuerpo:
- `:732` (`edge1` de la rama SINGLE): `logic: Logic::Sufficiency,` → `logic: edge_logic,`
- `:735`: `single_edge(new2_id.clone(), node_id.to_string(), b)` → `single_edge(new2_id.clone(), node_id.to_string(), b, edge_logic)`
- `:777`: `single_edge(new_id.clone(), cause_id.to_string(), node_id.to_string())` → `single_edge(new_id.clone(), cause_id.to_string(), node_id.to_string(), edge_logic)`
- `:820`: `single_edge(new2_id.clone(), node_id.to_string(), edge.to)` → `single_edge(new2_id.clone(), node_id.to_string(), edge.to, edge_logic)`
- `:817` (`edge1` de `--insert-before-effect`, `logic: edge.logic`) **no cambia**.

En `execute_link_group`, `:1076`: `logic: Logic::Sufficiency,` → `logic: Logic::from(tree.logic),`.

`src/path/mod.rs` — en `execute_path_replace`, `edge_a` (`:956`) y `edge_b` (`:967`): `logic: Logic::Sufficiency,` → `logic: Logic::from(tree.logic),`.

`src/macro_edge/mod.rs` — borrar la función duplicada (su doc dice, además, que GT es suficiencia):

```rust
/// Deriva la lógica de un edge (`Logic`) desde la lógica del árbol (`TreeLogic`).
///
/// Los INT/LINK materializados por `expand` (y el edge atómico de `promote`) heredan la lógica
/// del árbol contenedor: árboles de suficiencia (GT/CRT/FRT/TT) ⇒ `Sufficiency`; de necesidad
/// (EC/PRT) ⇒ `Necessity` (ver `tree::commands::logic_for_type`).
fn edge_logic(tree_logic: TreeLogic) -> Logic {
    match tree_logic {
        TreeLogic::Sufficiency => Logic::Sufficiency,
        TreeLogic::Necessity => Logic::Necessity,
    }
}
```

y además:
- `:20`: `use crate::tree::{MacroEdge, MacroEdgeStatus, NodeRef, TreeLogic};` → `use crate::tree::{MacroEdge, MacroEdgeStatus, NodeRef};`
- `:305` (doc de `execute_macro_expand`): `derivada del árbol ([\`edge_logic\`])` → `derivada del árbol (\`Logic::from(tree.logic)\`, ADR-014)`
- `:404`: `let logic = edge_logic(tree.logic);` → `let logic = Logic::from(tree.logic);`
- `:741`: `logic: edge_logic(tree.logic),` → `logic: Logic::from(tree.logic),`

- [ ] **Step 4: Ejecutar y verificar que pasan**

Run: `cargo test --test tree_logic && cargo test --test fase_06 && cargo test --test macro_lifecycle`
Expected: PASS (`fase_06` cubre `insert-between`/`group` en CRT; `macro_lifecycle` B5 sigue dando `NECESSITY` en PRT).

- [ ] **Step 5: Docs.** `CHANGELOG.md` — en `### Fixed`:

```markdown
- **Lógica de los edges creados por `link insert-between`, `link group` y `path replace`** (ADR-014): escribían siempre `SUFFICIENCY`. Ahora heredan la lógica del árbol, igual que `link connect` y `macro expand`/`macro promote`.
```

- [ ] **Step 6: Gate completo.** Expected: verde. Comprobar también `cargo doc --no-deps 2>&1 | rg "unresolved link"` → sin resultados (el enlace intra-doc a `edge_logic` ya no existe).

- [ ] **Step 7: Commit**

```bash
git add src/link/advanced.rs src/path/mod.rs src/macro_edge/mod.rs tests/tree_logic.rs CHANGELOG.md
git commit -m "fix(F6): insert-between, group y path replace heredan la lógica del árbol

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Tarea 6: CLR #4 solo en árboles de suficiencia

En un árbol de necesidad ("para lograr X, necesitamos Y") cada condición necesaria es, por construcción, insuficiente por sí sola y suele haber varias flechas sin operador al mismo padre: CLR #4 tal como está implementado (`CLR4_INSUFFICIENT_CAUSE`, `CLR4_5_IMPLICIT_OR_REVIEW`, `CLR4_5_EXCESSIVE_AND_INPUTS`) solo genera falsos positivos ahí. El resto de linters semánticos (CLR #2, #5, #6, #7) no cambian.

**Files:**
- Modify: `src/validate/mod.rs:18` (import), `:183-190` (las tres llamadas CLR #4)
- Test: `tests/tree_logic.rs`
- Modify (docs): `ENGINE_SPEC.md:370-372`, `CLR_SPEC.md` (una frase), `CHANGELOG.md`

**Interfaces:**
- Consumes: `tree.logic` ya normalizado por `load_tree` (Tarea 3); helpers `gt_chain`, `write_legacy_sufficiency`, `connect`, `attach`, `ids` (tareas 3–4).
- Produces: helpers de test `warning_codes(&Value) -> Vec<String>`, `validate_tree(dir, tree) -> Value`, `CLR4_CODES`, `assert_no_clr4(&Value, ctx)`, `fan_in_tree(dir, tree_type, [&str; 4]) -> String`.

- [ ] **Step 1: Añadir helpers y tests que fallan** a `tests/tree_logic.rs`:

```rust
/// Warning codes from the root `warnings[]` and from validate's `data.details[].warnings[]`.
fn warning_codes(json: &Value) -> Vec<String> {
    let root = json["warnings"].as_array().into_iter().flatten();
    let nested = json["data"]["details"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|d| d["warnings"].as_array().into_iter().flatten());
    root.chain(nested)
        .map(|w| w["code"].as_str().unwrap().to_string())
        .collect()
}

fn validate_tree(dir: &Path, tree: &str) -> Value {
    run_ok(dir, &["validate", "--tree", tree])
}

const CLR4_CODES: [&str; 3] = [
    "CLR4_INSUFFICIENT_CAUSE",
    "CLR4_5_IMPLICIT_OR_REVIEW",
    "CLR4_5_EXCESSIVE_AND_INPUTS",
];

fn assert_no_clr4(json: &Value, ctx: &str) {
    let codes = warning_codes(json);
    for code in CLR4_CODES {
        assert!(!codes.iter().any(|c| c == code), "{ctx}: unexpected {code} in {codes:?}");
    }
}

/// `a → m`, `b → m`, `m → top`: an implicit OR on `m` and a lone SINGLE cause on `top`,
/// i.e. both CLR #4 triggers if the tree were a sufficiency tree.
fn fan_in_tree(dir: &Path, tree_type: &str, types: [&str; 4]) -> String {
    let [ta, tb, tm, ttop] = types;
    let a = add_node(dir, "Causa A", ta);
    let b = add_node(dir, "Causa B", tb);
    let m = add_node(dir, "Efecto intermedio", tm);
    let top = add_node(dir, "Efecto final", ttop);
    let tree = create_tree(dir, tree_type, &format!("Fan {tree_type}"));
    attach(dir, &tree, &[&a, &b, &m, &top]);
    connect(dir, &tree, &a, &m);
    connect(dir, &tree, &b, &m);
    connect(dir, &tree, &m, &top);
    tree
}

/// T6.1: a GT with an implicit OR and a lone SINGLE cause emits no CLR #4.
#[test]
fn t6_1_gt_emits_no_clr4() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup(dir);
    let tree = fan_in_tree(dir, "gt", ["NC", "NC", "CSF", "GOAL"]);
    assert_no_clr4(&validate_tree(dir, &tree), "GT");
}

/// T6.2: a PRT with a 5-input AND emits no CLR4_5_EXCESSIVE_AND_INPUTS.
#[test]
fn t6_2_prt_wide_and_emits_no_clr4() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup(dir);
    let obstacles: Vec<String> = (1..=5)
        .map(|i| add_node(dir, &format!("Obstáculo {i}"), "OBS"))
        .collect();
    let io = add_node(dir, "Objetivo intermedio", "IO");
    let tree = create_tree(dir, "prt", "PRT Ancho");
    let mut nodes: Vec<&str> = obstacles.iter().map(String::as_str).collect();
    nodes.push(&io);
    attach(dir, &tree, &nodes);
    let from = obstacles.join(",");
    run_ok(
        dir,
        &[
            "link", "connect", "--tree", &tree, "--from", &from, "--to", &io, "--operator", "AND",
        ],
    );
    assert_no_clr4(&validate_tree(dir, &tree), "PRT");
}

/// T6.3: control — the same shape in a CRT still emits both CLR #4 warnings.
#[test]
fn t6_3_crt_still_emits_clr4() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup(dir);
    let tree = fan_in_tree(dir, "crt", ["RC", "RC", "INT", "UDE"]);
    let codes = warning_codes(&validate_tree(dir, &tree));
    assert!(codes.iter().any(|c| c == "CLR4_5_IMPLICIT_OR_REVIEW"), "{codes:?}");
    assert!(codes.iter().any(|c| c == "CLR4_INSUFFICIENT_CAUSE"), "{codes:?}");
}

/// T6.4: a legacy GT (stored as sufficiency) emits no CLR #4 either.
#[test]
fn t6_4_legacy_gt_emits_no_clr4() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup(dir);
    let gt = gt_chain(dir);
    write_legacy_sufficiency(dir, &gt.tree);
    assert_no_clr4(&validate_tree(dir, &gt.tree), "legacy GT");
}

/// T6.5: the gate is CLR #4 only — CLR #2 (conjunctions) still runs on a GT.
#[test]
fn t6_5_gt_still_emits_clr2() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup(dir);
    let nc = add_node(dir, "Bajamos costes porque subimos volumen", "NC");
    let goal = add_node(dir, "Somos rentables", "GOAL");
    let tree = create_tree(dir, "gt", "GT Conjuncion");
    attach(dir, &tree, &[&nc, &goal]);
    connect(dir, &tree, &nc, &goal);
    let codes = warning_codes(&validate_tree(dir, &tree));
    assert!(codes.iter().any(|c| c == "CLR2_CONJUNCTION_DETECTED"), "{codes:?}");
}
```

- [ ] **Step 2: Ejecutar y verificar que fallan**

Run: `cargo test --test tree_logic t6_`
Expected: FALLAN `t6_1` (`CLR4_5_IMPLICIT_OR_REVIEW`), `t6_2` (`CLR4_5_EXCESSIVE_AND_INPUTS`) y `t6_4` (`CLR4_INSUFFICIENT_CAUSE`). PASAN ya `t6_3` y `t6_5` (controles de regresión).

- [ ] **Step 3: Implementación** en `src/validate/mod.rs`.

Import (`:18`):

```rust
use crate::tree::types::{MacroEdgeStatus, TreeLogic, TreeType};
```

Sustituir las tres llamadas CLR #4 (`:183-190`) por:

```rust
        // CLR#4 and CLR#4/#5 only apply to sufficiency trees (CRT/FRT/TT). In necessity
        // trees (GT/EC/PRT) each necessary condition is insufficient on its own by
        // construction, so these lints would be pure noise (CLR_SPEC §1.2, ADR-014).
        if tree.logic == TreeLogic::Sufficiency {
            // CLR#4: Insufficiency
            tree_warnings.extend(clr::lint_clr4_insufficiency(&tree.edges));

            // CLR#4/#5: Implicit OR (multiple ungrouped SINGLE edges to same node)
            tree_warnings.extend(clr::lint_clr4_5_implicit_or(&tree.edges));

            // CLR#4/#5: Excessive AND inputs
            tree_warnings.extend(clr::lint_clr4_5_excessive_and(&tree.edges));
        }
```

- [ ] **Step 4: Ejecutar y verificar que pasan**

Run: `cargo test --test tree_logic && cargo test --test fase_05 && cargo test --test e2e`
Expected: PASS (las UATs de CLR4 de `fase_05` y `e2e` usan CRT).

- [ ] **Step 5: Docs**

`ENGINE_SPEC.md` — en **Advertencias** de `validate`, prefijar los tres bullets CLR #4 (`:370`, `:371`, `:372`) con `*(Solo suficiencia: CRT/FRT/TT.)* ` y añadir tras ellos:

```markdown
- En árboles de necesidad (GT/EC/PRT) los tres lints CLR #4 anteriores no se evalúan: cada condición necesaria es insuficiente por sí sola por construcción (CLR_SPEC §1.2, ADR-014). El resto de lints semánticos sí se aplican.
```

`CLR_SPEC.md` — es una sola línea larga; justo después del ancla única `Si hay múltiples flechas sin operador al mismo nodo (OR implícito), ¿cada causa basta sola?` añadir:

```text
 (Solo árboles de suficiencia —CRT, FRT, TT—: en GT/EC/PRT cada condición necesaria es insuficiente por sí sola por construcción y el motor no emite CLR #4, ADR-014.)
```

`CHANGELOG.md` — en `### Fixed`:

```markdown
- **Falsos positivos de CLR #4 en árboles de necesidad** (ADR-014): `validate` emitía `CLR4_INSUFFICIENT_CAUSE`, `CLR4_5_IMPLICIT_OR_REVIEW` y `CLR4_5_EXCESSIVE_AND_INPUTS` en GT/EC/PRT, donde cada condición necesaria es insuficiente por construcción. Ahora solo se evalúan en CRT/FRT/TT.
```

- [ ] **Step 6: Gate completo.** Expected: verde.

- [ ] **Step 7: Commit**

```bash
git add src/validate/mod.rs tests/tree_logic.rs ENGINE_SPEC.md CLR_SPEC.md CHANGELOG.md
git commit -m "fix(F5): CLR #4 solo en árboles de suficiencia (sin falsos positivos en GT/EC/PRT)

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Tarea 7: `tree walk` sin `--order` usa el default de la lógica (CLI y MCP)

ENGINE_SPEC:154 ya documenta `topological` como default en suficiencia y `reverse` en necesidad, pero las dos shells fijan `topological` (clap `default_value` en `main.rs:312`; `unwrap_or("topological")` en `dispatch.rs:641`). La decisión se mueve al dominio: el default solo se conoce tras cargar el árbol.

**Files:**
- Modify: `src/tree/commands.rs:1098-1104` (doc + firma), `:1116` (rama de error), tras el `load_tree` (~`:1123`)
- Modify: `src/main.rs:310-321` (`Walk`), `:1408`
- Modify: `src/mcp/dispatch.rs:641`, `src/mcp/tools.rs:217`
- Test: `tests/tree_logic.rs`
- Modify (docs): `ENGINE_SPEC.md:154`, `CHANGELOG.md`

**Interfaces:**
- Consumes: `tree.logic` normalizado (Tarea 3); helpers `gt_chain`, `write_legacy_sufficiency`, `walk_ids`, `connect`, `attach` (Tarea 3).
- Produces: `pub fn execute_tree_walk(storage: &dyn Storage, tree_id: &str, order: Option<&str>, show_knowledge: bool) -> CommandOutput<TreeWalkData>` (antes `order: &str`; solo la llaman las dos shells del workspace). `fn default_walk_order(logic: TreeLogic) -> &'static str` (privada).

- [ ] **Step 1: Añadir helpers MCP y tests que fallan** a `tests/tree_logic.rs`. Ampliar los imports de cabecera:

```rust
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
```

Helpers:

```rust
fn mcp_bin() -> String {
    env!("CARGO_BIN_EXE_ltp-mcp").to_string()
}

/// One `tools/call` over a fresh `ltp-mcp` stdio session; returns the parsed CommandOutput.
fn mcp_call(dir: &Path, tool: &str, arguments: Value) -> Value {
    let mut child = Command::new(mcp_bin())
        .arg("--workspace")
        .arg(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn ltp-mcp");
    let request = json!({
        "jsonrpc": "2.0", "id": 1,
        "method": "tools/call",
        "params": { "name": tool, "arguments": arguments }
    });
    let mut stdin = child.stdin.take().unwrap();
    writeln!(stdin, "{request}").unwrap();
    drop(stdin);
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.lines().find(|l| !l.trim().is_empty()).unwrap_or_else(|| {
        panic!("no MCP response; stderr: {}", String::from_utf8_lossy(&output.stderr))
    });
    let response: Value = serde_json::from_str(line).unwrap();
    serde_json::from_str(response["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
}
```

Tests:

```rust
/// T7.1: a GT walks `reverse` by default (goal first).
#[test]
fn t7_1_gt_walk_defaults_to_reverse() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup(dir);
    let gt = gt_chain(dir);
    let json = run_ok(dir, &["tree", "walk", &gt.tree]);
    assert_eq!(json["data"]["order"], "reverse");
    assert_eq!(walk_ids(&json), [gt.goal.clone(), gt.csf.clone(), gt.nc.clone()]);
}

/// T7.2: control — a CRT keeps walking `topological` by default.
#[test]
fn t7_2_crt_walk_defaults_to_topological() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup(dir);
    let rc = add_node(dir, "Previsión manual", "RC");
    let ude = add_node(dir, "Roturas de stock", "UDE");
    let tree = create_tree(dir, "crt", "CRT Walk");
    attach(dir, &tree, &[&rc, &ude]);
    connect(dir, &tree, &rc, &ude);
    let json = run_ok(dir, &["tree", "walk", &tree]);
    assert_eq!(json["data"]["order"], "topological");
    assert_eq!(walk_ids(&json), [rc, ude]);
}

/// T7.3: an explicit `--order` always wins over the logic default.
#[test]
fn t7_3_explicit_order_wins() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup(dir);
    let gt = gt_chain(dir);
    let json = run_ok(dir, &["tree", "walk", &gt.tree, "--order", "topological"]);
    assert_eq!(json["data"]["order"], "topological");
    assert_eq!(walk_ids(&json), [gt.nc.clone(), gt.csf.clone(), gt.goal.clone()]);
}

/// T7.4 (review focus): the MCP shell applies the same default as the CLI.
#[test]
fn t7_4_mcp_walk_default_matches_cli() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup(dir);
    let gt = gt_chain(dir);
    let mcp = mcp_call(dir, "ltp/tree_walk", json!({ "tree_id": gt.tree }));
    assert_eq!(mcp["data"]["order"], "reverse");
    let cli = run_ok(dir, &["tree", "walk", &gt.tree]);
    assert_eq!(walk_ids(&mcp), walk_ids(&cli));
}

/// T7.5: a legacy GT (stored as sufficiency) also walks `reverse` by default.
#[test]
fn t7_5_legacy_gt_walk_defaults_to_reverse() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup(dir);
    let gt = gt_chain(dir);
    write_legacy_sufficiency(dir, &gt.tree);
    let json = run_ok(dir, &["tree", "walk", &gt.tree]);
    assert_eq!(json["data"]["order"], "reverse");
}
```

- [ ] **Step 2: Ejecutar y verificar que fallan**

Run: `cargo test --test tree_logic t7_`
Expected: FALLAN `t7_1`, `t7_4` y `t7_5` (`"topological"` ≠ `"reverse"`). PASAN ya `t7_2` y `t7_3` (controles).

- [ ] **Step 3: Implementación**

`src/tree/commands.rs` — justo encima de `/// Execute \`tree walk\`.`:

```rust
/// Default `tree walk` order for a tree logic (ENGINE_SPEC §2.3, ADR-014): necessity trees
/// read top-down from the goal, sufficiency trees bottom-up from the root causes.
fn default_walk_order(logic: TreeLogic) -> &'static str {
    match logic {
        TreeLogic::Sufficiency => "topological",
        TreeLogic::Necessity => "reverse",
    }
}
```

Doc y firma de `execute_tree_walk`:

```rust
/// Execute `tree walk`.
///
/// `order: None` applies the tree's logic default (`reverse` for GT/EC/PRT, `topological`
/// for CRT/FRT/TT); an explicit order always wins.
pub fn execute_tree_walk(
    storage: &dyn Storage,
    tree_id: &str,
    order: Option<&str>,
    show_knowledge: bool,
) -> CommandOutput<TreeWalkData> {
```

En la rama de error del `load_tree` (`:1116`): `order: order.to_string(),` → 

```rust
                    // Tree not found ⇒ logic unknown; keep the historical default.
                    order: order.unwrap_or("topological").to_string(),
```

Justo después del `match storage.load_tree(tree_id) { … };`:

```rust
    let order = order.unwrap_or_else(|| default_walk_order(tree.logic));
```

(Los usos posteriores `if order == "reverse"` (`:1208`) y `order: order.to_string()` (`:1271`) no cambian: `order` vuelve a ser `&str`.)

`src/main.rs` — variante `Walk` (`:310-321`):

```rust
    Walk {
        tree_id: String,
        /// Walk order: topological | reverse (default: reverse for GT/EC/PRT, topological for CRT/FRT/TT)
        #[arg(long)]
        order: Option<String>,
```

y `:1408`: `execute_tree_walk(&storage, &tree_id, &order, show_knowledge)` → `execute_tree_walk(&storage, &tree_id, order.as_deref(), show_knowledge)`.

`src/mcp/dispatch.rs:641`: `let order = get_str_opt(args, "order").unwrap_or("topological");` → `let order = get_str_opt(args, "order");`.

`src/mcp/tools.rs:217`:

```rust
                "order": { "type": "string", "description": "Walk order: topological or reverse (default: reverse for GT/EC/PRT, topological for CRT/FRT/TT)" },
```

- [ ] **Step 4: Ejecutar y verificar que pasan**

Run: `cargo test --test tree_logic && cargo test --test fase_03 && cargo test --test contract && cargo test --test knowledge_k5 && cargo test --test knowledge_k7 && cargo test --test fase_12`
Expected: PASS (los goldens `tree_walk*.json` y los walks de `knowledge_k5`/`k7` son sobre CRT ⇒ siguen en `topological`; el número de tools MCP sigue en 71).

- [ ] **Step 5: Docs**

`ENGINE_SPEC.md:154` — sustituir el bullet de `--order` por:

```markdown
- Sin `--order`, el orden se deriva de la lógica del árbol (ADR-014): `topological` en suficiencia (CRT/FRT/TT), desde causas raíz hacia efectos; `reverse` en necesidad (GT/EC/PRT), desde el objetivo hacia los prerrequisitos. `--order` explícito siempre manda, y `data.order` informa siempre del orden aplicado.
```

`CHANGELOG.md` — en `### Fixed`:

```markdown
- **Orden por defecto de `tree walk`** (ADR-014): sin `--order`, CLI y MCP recorrían siempre en `topological`, en contra de ENGINE_SPEC. Ahora GT/EC/PRT usan `reverse` (desde el objetivo) y CRT/FRT/TT siguen en `topological`. `data.order` informa del orden aplicado. **Consumidores**: para conservar el comportamiento anterior en árboles de necesidad, pasad `--order topological` (o `"order": "topological"` en MCP) explícitamente.
```

- [ ] **Step 6: Gate completo.** Expected: verde.

- [ ] **Step 7: Commit**

```bash
git add src/tree/commands.rs src/main.rs src/mcp/dispatch.rs src/mcp/tools.rs tests/tree_logic.rs ENGINE_SPEC.md CHANGELOG.md
git commit -m "fix(F3): tree walk sin --order usa el default de la lógica del árbol (CLI y MCP)

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Tarea 8: Cierre — PROGRESS, gate final y push

**Files:**
- Modify: `PROGRESS.md` (Dashboard `:3-23`, `## Historial de Avance` `:68`, `## Histórico de Reestimaciones` `:751`)

**Interfaces:**
- Consumes: las tareas 1–7 commiteadas y en verde.

- [ ] **Step 1: Contar los tests reales**

Run: `cargo test --workspace 2>&1 | rg -o "[0-9]+ passed" | awk '{s+=$1} END {print s}'`
Expected: `561` (531 + 30 nuevos: 5 UATs CSF de la Tarea 1, 5 unit de la Tarea 2, 4 + 3 + 3 + 5 + 5 E2E de `tree_logic.rs` en las tareas 3–7). Si sale otro número, usar **el real** en los pasos siguientes y explicar la diferencia en el historial.

- [ ] **Step 2: Dashboard.** En la tabla `## Dashboard`:
- `| **Último bugfix** | … |` → `| **Último bugfix** | Lógica de árbol derivada del tipo: GT = necesidad, edges heredan, CLR #4 y walk por lógica (ADR-014) |`
- `| **Último añadido** | … |` → `| **Último añadido** | Tipo de nodo \`CSF\` (Critical Success Factor del Goal Tree) |`
- Añadir, antes de `**Tests totales**`, la fila `| **Tests CSF + lógica de árbol** | 30/30 |`.
- `| **Tests totales** | 531 |` → `| **Tests totales** | 561 |`.

- [ ] **Step 3: Historial.** Insertar justo debajo de `## Historial de Avance` (la entrada más reciente va primero):

```markdown
### [Bugfix + CSF] — Lógica de árbol derivada del tipo + `NodeType::Csf` (ADR-014)
**Fecha**: 2026-10-05
**Naturaleza**: Bugfix + tipo de nodo aditivo sobre el motor ya completo. **No** altera el % del motor ni del Knowledge Pool.
**Avance**: 8 paquetes completados, 30 tests nuevos ✅ (5 UATs CSF en `fase_02a`, 5 unit en `tree/types.rs`, 20 E2E en `tests/tree_logic.rs`).
**Tests totales**: 531 → 561
**Plan**: `PLAN_csf-and-tree-logic.md` | **Specs**: ENGINE_SPEC §2.2/§2.3/§2.4/§2.12/validate, ADR-014 (nuevo), ADR-009/010/013, CLR_SPEC §1.2
**Factor de escala**: 1.0x (8 paquetes, esfuerzo ≈ estimado)
**Origen**: el GT de Dettmer (`GOAL ← CSF ← NC`) no tenía tipo `CSF`, y el motor trataba el GT como árbol de suficiencia; además todos los creadores de edges escribían `SUFFICIENCY` fijo. Resultado: falsos positivos de CLR #4 en GT/EC y `tree walk` en orden contrario al documentado.

#### Entregables
- **`NodeType::Csf`** (T1): prefijo `CSF`, aceptado en CLI y MCP, contadores retrocompatibles.
- **Lógica derivada del tipo** (T2–T3): `TreeType::logic()` como única fuente de verdad; `Tree::normalize_logic()` llamado en `load_tree` (normalización al leer, persistencia perezosa, undo intacto).
- **Creadores de edges** (T4–T5): `link connect`, `insert-between`, `group` y `path replace` heredan la lógica del árbol; NBR siempre `SUFFICIENCY`. Eliminadas las dos funciones duplicadas (`logic_for_type`, `macro_edge::edge_logic`).
- **CLR #4 por lógica** (T6): solo en CRT/FRT/TT.
- **`tree walk`** (T7): default por lógica en CLI y MCP; `--order` explícito manda.

#### Decisiones
- Opción C para los borradores GT legacy (normalizar al leer + persistir en la siguiente mutación). Descartadas A, B, D y E: ver ADR-014.
- Ningún tool MCP nuevo (siguen 71). Goldens `contract/` sin cambios.

#### Siguiente
- Diferido: re-tipar los OBJ/REQ de los borradores a CSF/NC; validar valores de `--order`; eliminar `edge.logic` almacenado (RFC-002 Capa 1, MAJOR).
```

Si el esfuerzo real se desvió >30% del estimado, recalcular el factor de escala según `## Reglas de Reestimación` y reflejarlo aquí y en el dashboard.

- [ ] **Step 4: Reestimaciones.** Añadir al final de la tabla de `## Histórico de Reestimaciones`:

```markdown
| 2026-10-05 | CSF + lógica de árbol derivada del tipo (ADR-014) | +30 tests (5 UATs CSF + 5 unit + 20 E2E `tree_logic`). Bugfix + tipo aditivo sobre el motor completo (no altera % del motor base ni del Knowledge Pool). | 531 → 561 tests. Factor de escala 1.0x. |
```

- [ ] **Step 5: Gate final completo** (las cuatro órdenes de las Restricciones globales). Expected: verde. Comprobar además `git status --short` → solo deben aparecer como pendientes los 5 docs ajenos a este plan.

- [ ] **Step 6: Commit y push** (fin de fase)

```bash
git add PROGRESS.md
git commit -m "docs: PROGRESS — CSF + lógica de árbol derivada del tipo (ADR-014)

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
git push
```
