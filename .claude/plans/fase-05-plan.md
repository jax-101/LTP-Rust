# Plan F5 — Validación Completa

## Objetivo

Implementar `ltp validate [--tree <TREE_ID>]` con todas las reglas de validación: DAG, integridad referencial, validación EC, linter CLR#2, y warnings CLR#4/#5/#6/#7. Cubrir los 14 UATs definidos en PLAN.md.

---

## Arquitectura

### Módulo `src/validate/`

```
src/validate/
├── mod.rs          # Orquestador: execute_validate() + ValidationResult
├── dag.rs          # (existente) check_dag()
├── integrity.rs    # Integridad referencial de edges
├── ec.rs           # Reglas específicas de Evaporating Cloud
├── clr.rs          # Linter CLR: #2, #4, #5, #6, #7
└── orphans.rs      # Nodos huérfanos dentro del tree
```

### Flujo de `execute_validate`

```
1. Cargar workspace (config + nodes + trees)
2. Si --tree: filtrar a un solo tree; si no: iterar todos
3. Por cada tree:
   a. check_dag(edges)                → errors: CIRCULAR_DEPENDENCY_DETECTED
   b. check_dag(nbr.edges) per NBR    → errors: CIRCULAR_DEPENDENCY_DETECTED (NBR)
   c. check_integrity(edges, pool)    → errors: REFERENTIAL_INTEGRITY_VIOLATION
   d. if tree_type == EC:
      check_ec_rules(tree)            → errors: EC_VALIDATION (3 sub-reglas)
   e. lint_clr2(nodes en tree)        → warnings: CLR2_CONJUNCTION_DETECTED
   f. lint_clr4(edges)                → warnings: CLR4_INSUFFICIENT_CAUSE
   g. lint_clr4_5(edges)              → warnings: CLR4_5_EXCESSIVE_AND_INPUTS
   h. lint_clr6(edges, nodes)         → warnings: CLR6_TYPE_INVERSION
   i. lint_clr7(nodes, edges)         → warnings: CLR7_INTANGIBLE_NO_PREDICTED
   j. check_orphans(tree.nodes, tree.edges) → warnings: ORPHAN_NODE_IN_TREE
4. Agregar todos los errors/warnings
5. Retornar CommandOutput con graph_health actualizado
```

---

## Reglas de Validación (mapeo a UATs)

### Bloqueantes (errors)

| Regla | Código | Descripción | UAT |
|-------|--------|-------------|-----|
| DAG | `CIRCULAR_DEPENDENCY_DETECTED` | DFS 3 colores sobre `edges` (excluye feedback_edges) | 5.2 |
| DAG NBR | `CIRCULAR_DEPENDENCY_DETECTED` | DFS 3 colores sobre cada `nbr_branches[].edges` | 5.14 |
| Integridad | `REFERENTIAL_INTEGRITY_VIOLATION` | Nodo en edge.from/edge.to no existe en pool `/nodes/` | 5.3 |
| EC objective | `EC_VALIDATION` | EC sin exactamente 1 nodo con role `objective` | 5.4 |
| EC requirements | `EC_VALIDATION` | EC con <2 nodos role `requirement` | 5.5 |
| EC prerequisites | `EC_VALIDATION` | Requirement sin al menos 1 prerequisite conectado | 5.6 |

### Advertencias (warnings)

| Regla | Código | Descripción | UAT |
|-------|--------|-------------|-----|
| CLR#2 | `CLR2_CONJUNCTION_DETECTED` | Conjunciones causales en labels | 5.7 |
| CLR#4 | `CLR4_INSUFFICIENT_CAUSE` | Nodo con solo 1 entrada SINGLE | 5.8 |
| CLR#4/#5 | `CLR4_5_EXCESSIVE_AND_INPUTS` | AND con >4 entradas | 5.9 |
| CLR#7 | `CLR7_INTANGIBLE_NO_PREDICTED` | Nodo observable:false con <2 salientes | 5.10 |
| CLR#6 | `CLR6_TYPE_INVERSION` | UDE/DE en from apuntando a RC/INT | 5.11 |
| Huérfanos | `ORPHAN_NODE_IN_TREE` | Nodo attached sin edges | 5.12 |

---

## Pasos de Implementación

### Paso 1: Tipos y estructura del módulo validate

**Archivos**: `src/validate/mod.rs`

- Definir `ValidateResult` (struct con errors + warnings)
- Crear firma pública `execute_validate(storage, tree_id: Option<&str>) -> CommandOutput<ValidateData>`
- Definir `ValidateData` con campos: `trees_validated: usize`, `total_errors: usize`, `total_warnings: usize`, `details: Vec<TreeValidation>`
- Struct `TreeValidation`: `tree_id`, `errors`, `warnings`

### Paso 2: Integridad referencial (`integrity.rs`)

**Lógica**:
- Para cada edge en el tree: verificar que cada nodo en `edge.from[]` y `edge.to` existe en el pool global (storage.list_node_ids() o un HashSet precalculado)
- Retornar `Vec<OutputError>` con code `REFERENTIAL_INTEGRITY_VIOLATION` y context `node_id`, `edge_id`, `tree_id`

**UAT cubierto**: 5.3

### Paso 3: Validación EC (`ec.rs`)

**Lógica** (solo aplica si `tree_type == Ec`):
1. Contar nodos con `role == Some("objective")` → debe ser exactamente 1
2. Contar nodos con `role == Some("requirement")` → debe ser >= 2
3. Para cada requirement: verificar que existe al menos un edge entrante desde un nodo con role `prerequisite` (o sin role pero que tenga un edge apuntando al requirement)
   - Alternativa más simple según spec: cada requirement debe tener al menos 1 `prerequisite` conectado como from en algún edge donde to sea el requirement

**Decisión de diseño sobre "prerequisite por requirement"**:
- Un nodo prerequisite se identifica por `role == Some("prerequisite")` dentro del tree
- La conexión se verifica: debe existir un edge con `from` conteniendo un prerequisite y `to` == el requirement

**UATs cubiertos**: 5.4, 5.5, 5.6

### Paso 4: Linter CLR#2 reutilizado (`clr.rs`)

**Lógica**:
- Reutilizar `node::clr_lint::lint_clr2()` ya existente
- Para cada nodo attached en el tree, cargar el nodo del pool y ejecutar lint_clr2 sobre su label
- Agregar a warnings

**UAT cubierto**: 5.7

### Paso 5: Linters CLR#4, #5, #6, #7 (`clr.rs`)

**CLR#4 — Insuficiencia** (UAT 5.8):
- Contar edges entrantes por nodo-destino (to)
- Si un nodo tiene exactamente 1 edge entrante con operator SINGLE → warning `CLR4_INSUFFICIENT_CAUSE`
- Context: `node_id`, `edge_id`

**CLR#4/#5 — AND excesivo** (UAT 5.9):
- Buscar edges con operator AND cuyo `from.len() > 4`
- Warning `CLR4_5_EXCESSIVE_AND_INPUTS`
- Context: `edge_id`, `input_count`

**CLR#6 — Inversión de tipos** (UAT 5.11):
- Definir "nivel alto": UDE, DE
- Definir "nivel bajo": RC, INT
- Para cada edge: si todos los from son de nivel alto y el to es de nivel bajo → warning
- Context: `edge_id`, `from_types`, `to_type`

**CLR#7 — Intangible sin efecto predicho** (UAT 5.10):
- Buscar nodos con `observable == false`
- Contar sus edges salientes (aparecen en `edge.from[]`)
- Si <2 → warning `CLR7_INTANGIBLE_NO_PREDICTED`
- Context: `node_id`, `outgoing_edge_count`

### Paso 6: Detección de huérfanos dentro del tree (`orphans.rs`)

**Lógica** (UAT 5.12):
- Construir set de nodos que participan en algún edge (from o to)
- Para cada nodo en tree.nodes: si su ref no aparece en ningún edge → warning `ORPHAN_NODE_IN_TREE`
- Context: `node_id`, `tree_id`

### Paso 7: Orquestación y DAG NBR (`mod.rs`)

**Lógica**:
- Iterar `tree.nbr_branches` y ejecutar `check_dag()` sobre cada `nbr.edges`
- Enriquecer el error con context `nbr_id`

**UAT cubierto**: 5.14

### Paso 8: Integración en `main.rs`

- En el match de `Commands::Validate { tree }`: llamar a `execute_validate(&storage, tree.as_deref())`
- Eliminar el fallback `_ => NOT_IMPLEMENTED` para Validate

**UAT cubierto**: 5.1, 5.13 (--tree filter)

### Paso 9: Tests de integración CLI

**Archivo**: `tests/validate_tests.rs`

Crear 14 tests de integración que invoquen el binario `ltp` como proceso hijo en un tempdir y verifiquen stdout JSON + estado correcto:

| Test | Escenario | Verificación |
|------|-----------|--------------|
| 5.1 | workspace sano (nodos + tree + edges válidos) | errors=[], warnings=[] |
| 5.2 | Inyectar ciclo manualmente en tree JSON | error CIRCULAR_DEPENDENCY_DETECTED |
| 5.3 | Edge referencia nodo inexistente | error REFERENTIAL_INTEGRITY_VIOLATION |
| 5.4 | EC sin nodo role objective | error EC_VALIDATION: missing objective |
| 5.5 | EC con 1 solo requirement | error EC_VALIDATION: minimum 2 requirements |
| 5.6 | EC sin prerequisite para requirement | error EC_VALIDATION: requirement without prerequisite |
| 5.7 | Nodo con "porque" en label | warning CLR2_CONJUNCTION_DETECTED |
| 5.8 | Nodo con 1 sola entrada SINGLE | warning CLR4_INSUFFICIENT_CAUSE |
| 5.9 | AND con >4 entradas | warning CLR4_5_EXCESSIVE_AND_INPUTS |
| 5.10 | Nodo observable:false con <2 salientes | warning CLR7_INTANGIBLE_NO_PREDICTED |
| 5.11 | UDE en from apuntando a RC | warning CLR6_TYPE_INVERSION |
| 5.12 | Nodo attached sin edges | warning ORPHAN_NODE_IN_TREE |
| 5.13 | validate --tree (solo valida ese tree) | Solo ese tree en results |
| 5.14 | NBR con ciclo interno | error CIRCULAR_DEPENDENCY_DETECTED con nbr context |

---

## Decisiones Técnicas

1. **Pool de nodos como HashSet precalculado**: Al inicio de validate, cargar todos los node IDs en un `HashSet<String>` para O(1) lookup en integridad referencial.

2. **Carga de nodos solo cuando necesario**: Para CLR#6 y CLR#7 necesitamos `node_type` y `observable`. Cargar los nodos referenciados en el tree bajo demanda (o precache todos los nodos del tree en un `HashMap<String, Node>`).

3. **Reutilización de `check_dag`**: La función existente en `dag.rs` ya sirve tanto para edges del tree como para edges de NBR branches.

4. **EC validation solo para tree_type == Ec**: No aplicar reglas EC a CRT/FRT/etc.

5. **Validate es operación de navegación**: No genera undo entry, no adquiere lock file.

6. **Salida**: `success: true` si no hay errors (puede haber warnings). `success: false` solo si hay al menos un error bloqueante.

---

## Orden de Ejecución

```
Paso 1 (tipos)
  → Paso 2 (integridad) + Paso 3 (EC) + Paso 4-5 (CLR) + Paso 6 (orphans)  [parallelizables]
    → Paso 7 (orquestación + NBR DAG)
      → Paso 8 (integración main.rs)
        → Paso 9 (tests CLI)
          → cargo check + clippy + test + fmt
            → Actualizar PROGRESS.md
```

---

## Estimación

- Complejidad: **Media** (PLAN.md)
- Archivos nuevos: 4 (`integrity.rs`, `ec.rs`, `clr.rs`, `orphans.rs`)
- Archivos modificados: 3 (`validate/mod.rs`, `main.rs`, `lib.rs`)
- Tests de integración: 14
- Peso: 8% del total
