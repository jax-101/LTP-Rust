# Plan de Implementación — Knowledge Pool (ADR-012)

**Fecha**: 2026-08-17
**Spec**: [KNOWLEDGE_SPEC.md](../../KNOWLEDGE_SPEC.md)
**ADR**: ADR-012
**Prerequisito**: Motor completo (F1–F12 al 100%)

## Dependencias entre Fases

```
K1 ──→ K2 ──→ K3 ──→ K4 ──→ K5
                              │
                              ▼
                          K6 (E2E)
                              │
                              ▼
                          K7 (MCP)
```

- **K1** (fundación): schema, storage, init, counters — base mínima.
- **K2** (CRUD): knowledge add/edit/rm/inspect/list — operaciones standalone.
- **K3** (linking): knowledge link/unlink + validate refs — conecta con el grafo.
- **K4** (epistemic): campo en nodos + node add/edit/list extensions.
- **K5** (integración): status/validate/trace extensions — reportes y queries.
- **K6** (E2E): workflows completos hypothesis-driven.
- **K7** (MCP): exposición de los nuevos tools en ltp-mcp.

---

## Fase K1: Fundación Knowledge (Schema, Storage, Init)

**Scope**: Definir structs/enums, extender Storage trait, crear `knowledge/` en init, extender counters.

**Archivos**: `src/knowledge/mod.rs`, `src/knowledge/types.rs`, `src/storage.rs`, `src/workspace/`

**Peso estimado**: 8%

### Diseño

```rust
/// Tipo del knowledge item
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeType {
    Measurement,
    Testimony,
    Hypothesis,
    Document,
    Observation,
    Derived,
}

/// Estado epistémico del item
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeStatus {
    Unverified,
    Verified,
    Refuted,
    Superseded,
}

/// Nivel de confianza
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    High,
    Medium,
    Low,
}

/// Tipo de relación con entidad del grafo
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeRelation {
    Supports,
    Contradicts,
    Contextualizes,
}

/// Fuente del conocimiento
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excerpt: Option<String>,
}

/// Vínculo a entidad del grafo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeLink {
    pub target: String,
    pub relation: KnowledgeRelation,
}

/// Entidad de conocimiento
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeItem {
    pub id: String,
    #[serde(rename = "type")]
    pub knowledge_type: KnowledgeType,
    pub label: String,
    pub status: KnowledgeStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<Confidence>,
    pub source: KnowledgeSource,
    pub captured: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<KnowledgeLink>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}
```

Extender `Storage` trait:
```rust
fn load_knowledge(&self, id: &str) -> Result<KnowledgeItem>;
fn save_knowledge(&self, item: &KnowledgeItem) -> Result<()>;
fn delete_knowledge(&self, id: &str) -> Result<()>;
fn list_knowledge_ids(&self) -> Result<Vec<String>>;
```

### UATs

| ID | Acción | Resultado esperado |
|----|--------|-------------------|
| K1.1 | `ltp init --name "Test"` (versión nueva) | Crea carpeta `knowledge/` vacía. `counters.json` incluye `"KN": 0`. |
| K1.2 | Abrir workspace existente (sin `knowledge/`) → ejecutar cualquier comando | Auto-crea `knowledge/` + añade `"KN": 0` a counters. Warning `KNOWLEDGE_DIR_CREATED`. |
| K1.3 | Serializar/deserializar `KnowledgeItem` round-trip | JSON canónico (BTreeMap-ordered keys, indent 2). Todos los campos opcionales omitidos cuando None/empty. |
| K1.4 | `KnowledgeSource` con `uri: None, excerpt: None` | Error de validación `SOURCE_REQUIRED` (al menos uno debe estar presente). |

---

## Fase K2: CRUD de Knowledge Items

**Scope**: `knowledge add/edit/rm/inspect/list` — operaciones standalone sin interacción con el grafo.

**Archivos**: `src/knowledge/add.rs`, `src/knowledge/edit.rs`, `src/knowledge/rm.rs`, `src/knowledge/inspect.rs`, `src/knowledge/list.rs`, CLI dispatch.

**Peso estimado**: 18%

### UATs

| ID | Comando | Resultado esperado |
|----|---------|-------------------|
| K2.1 | `ltp knowledge add "Media entrega 18.3 días" --type measurement --source-excerpt "ERP Q2" --status verified --confidence high` | Crea `knowledge/KN-001.json`. Output: `success: true, data.created_knowledge_id: "KN-001"`. Counter KN incrementa. |
| K2.2 | `ltp knowledge add "Director dice mejoraron" --type testimony --source-uri "slack://C04/p123"` | Crea KN-002. Status default `unverified`, confidence default `medium`. |
| K2.3 | `ltp knowledge add "Sin fuente" --type observation` (sin --source-uri ni --source-excerpt) | Error `SOURCE_REQUIRED`. |
| K2.4 | `ltp knowledge add "Hipótesis cambio proveedor" --type hypothesis --source-excerpt "Entrevista con ops" --tags logistica,proveedor` | Crea con tags. |
| K2.5 | `ltp knowledge edit KN-001 --label "Nuevo texto"` | Actualiza label en disco. |
| K2.6 | `ltp knowledge edit KN-001 --status refuted` | Actualiza status. |
| K2.7 | `ltp knowledge edit KN-001 --confidence low` | Actualiza confidence. |
| K2.8 | `ltp knowledge edit KN-001 --add-tag q2-2026` | Añade tag. |
| K2.9 | `ltp knowledge edit KN-001 --rm-tag logistica` | Quita tag. |
| K2.10 | `ltp knowledge edit KN-999 --label "..."` | Error `KNOWLEDGE_NOT_FOUND`. |
| K2.11 | `ltp knowledge rm KN-001` | Elimina fichero. |
| K2.12 | `ltp knowledge rm KN-001,KN-002` (batch) | Elimina ambos. |
| K2.13 | `ltp knowledge rm KN-999` | Error `KNOWLEDGE_NOT_FOUND`. |
| K2.14 | `ltp knowledge inspect KN-001` | Detalle completo: todos los campos, links con target_label resuelto. |
| K2.15 | `ltp knowledge inspect KN-999` | Error `KNOWLEDGE_NOT_FOUND`. |
| K2.16 | `ltp knowledge list` | Lista todos los items (id, type, label, status, confidence, link_count). |
| K2.17 | `ltp knowledge list --type measurement` | Filtra por tipo. |
| K2.18 | `ltp knowledge list --status unverified` | Filtra por status. |
| K2.19 | `ltp knowledge list --unlinked` | Solo items con `links: []`. |
| K2.20 | `ltp knowledge list --tag logistica` | Filtra por tag. |
| K2.21 | `ltp knowledge list --confidence high` | Filtra por confidence. |

---

## Fase K3: Linking (Vínculos al Grafo)

**Scope**: `knowledge link/unlink`, filtro `--target` en list, validación de integridad referencial en targets.

**Archivos**: `src/knowledge/link.rs`, `src/knowledge/unlink.rs`, extensión de `list.rs`

**Peso estimado**: 15%

### UATs

| ID | Comando | Resultado esperado |
|----|---------|-------------------|
| K3.1 | `ltp knowledge link KN-001 --to UDE-003 --relation supports` | Añade link a KN-001. Output: `success: true`. |
| K3.2 | `ltp knowledge link KN-001 --to LINK-007 --relation supports` | Link a un edge. Valida que LINK-007 existe en algún tree. |
| K3.3 | `ltp knowledge link KN-001 --to ASM-002 --relation contradicts` | Link a un assumption. Valida que ASM-002 existe en algún tree. |
| K3.4 | `ltp knowledge link KN-001 --to NODO-999 --relation supports` | Error `TARGET_NOT_FOUND`. |
| K3.5 | `ltp knowledge link KN-999 --to UDE-003 --relation supports` | Error `KNOWLEDGE_NOT_FOUND`. |
| K3.6 | `ltp knowledge link KN-001 --to UDE-003 --relation supports` (duplicado) | Warning `DUPLICATE_LINK`. Idempotente: no añade segundo link. |
| K3.7 | `ltp knowledge unlink KN-001 --from UDE-003` | Elimina el link. |
| K3.8 | `ltp knowledge unlink KN-001 --from RC-005` (no existe ese link) | Error `LINK_NOT_FOUND`. |
| K3.9 | `ltp knowledge list --target UDE-003` | Lista items vinculados a UDE-003 (incluye campo `relation`). |
| K3.10 | `ltp knowledge list --target UDE-003 --relation supports` | Filtra por target + relation. |
| K3.11 | `ltp knowledge list --target NODO-999` | Lista vacía (no es error). |
| K3.12 | KN-001 linked a UDE-003 y LINK-007. `ltp knowledge inspect KN-001` | Ambos links con `target_label` resuelto desde disco. |
| K3.13 | KN-001 linked a UDE-003 (borrado previamente). `ltp knowledge inspect KN-001` | Link mostrado con `target_label: null` (dangling ref, no crash). |

---

## Fase K4: Campo Epistémico en Nodos

**Scope**: Añadir campo `epistemic` a struct Node, extender `node add/edit/list` con `--epistemic`.

**Archivos**: `src/node/types.rs`, `src/node/add.rs`, `src/node/edit.rs`, `src/node/list.rs`

**Peso estimado**: 10%

### Diseño

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EpistemicStatus {
    Fact,
    Hypothesis,
    Assumption,
    Derived,
}
```

Campo en Node:
```rust
pub struct Node {
    // ... campos existentes ...
    #[serde(default = "default_epistemic", skip_serializing_if = "is_hypothesis")]
    pub epistemic: EpistemicStatus,
}
```

Default: `Hypothesis`. Se omite del JSON si es hypothesis (backwards-compatible con nodos existentes).

### UATs

| ID | Comando | Resultado esperado |
|----|---------|-------------------|
| K4.1 | `ltp node add "Test" --type UDE --epistemic fact` | Crea nodo con `"epistemic": "fact"` en JSON. |
| K4.2 | `ltp node add "Test" --type UDE` (sin --epistemic) | Crea nodo SIN campo epistemic en JSON (default hypothesis, omitido por serde). |
| K4.3 | `ltp node edit UDE-001 --epistemic fact` | Añade/actualiza campo epistemic. |
| K4.4 | `ltp node edit UDE-001 --epistemic hypothesis` | Vuelve a default → campo omitido del JSON. |
| K4.5 | `ltp node list --tree T --epistemic hypothesis` | Filtra por epistemic status. Incluye nodos sin campo (implícit hypothesis). |
| K4.6 | `ltp node list --tree T --epistemic fact` | Solo nodos explícitamente marcados como fact. |
| K4.7 | `ltp node inspect UDE-001` (con epistemic fact) | Output incluye `"epistemic": "fact"`. |
| K4.8 | Cargar nodo existente (sin campo epistemic en JSON) | Deserializa correctamente con default `Hypothesis`. Backwards-compatible. |

---

## Fase K5: Integración con Comandos Existentes

**Scope**: Extender `status`, `validate`, `trace`, `node rm` con awareness de knowledge pool.

**Archivos**: `src/workspace/status.rs`, `src/validate/`, `src/trace/`, `src/node/rm.rs`

**Peso estimado**: 20%

### UATs

| ID | Comando | Resultado esperado |
|----|---------|-------------------|
| **status** | | |
| K5.1 | `ltp status` (con knowledge items) | Output incluye `knowledge_health`: total, unlinked, contradictions, by_status, epistemic_coverage. |
| K5.2 | `ltp status` (sin knowledge items) | `knowledge_health` con todos los conteos a 0. |
| **validate** | | |
| K5.3 | KN-001 linked a UDE-999 (borrado). `ltp validate` | Warning `DANGLING_KNOWLEDGE_REF`. |
| K5.4 | UDE-003 con `epistemic: fact`, 0 knowledge items `supports`. `ltp validate` | Warning `EPISTEMIC_UNGROUNDED`. |
| K5.5 | UDE-003 con `epistemic: fact`, KN-007 `contradicts` con status `verified`. `ltp validate` | Warning `EPISTEMIC_CONTRADICTED`. |
| K5.6 | RC-001 con `epistemic: hypothesis`, 2+ knowledge `supports` con status `verified`. `ltp validate` | Warning `EPISTEMIC_UPGRADEABLE`. |
| K5.7 | `ltp validate` con knowledge pool sano | 0 warnings de knowledge. |
| **trace** | | |
| K5.8 | `ltp trace UDE-003 --tree T --direction upstream --show-knowledge` | Cada nodo en chain incluye campo `knowledge: [{id, relation, status, confidence}]`. |
| K5.9 | `ltp trace UDE-003 --tree T --direction upstream` (sin flag) | Output normal sin knowledge (backwards-compatible). |
| K5.10 | `ltp trace` con `--show-knowledge`, nodo sin knowledge | Campo `knowledge: []` (no se omite). |
| **node rm** | | |
| K5.11 | `ltp node rm UDE-003` (tiene knowledge apuntando) | Warning `KNOWLEDGE_ORPHANED` con IDs de los KN afectados. Nodo se borra igual. |
| K5.12 | `ltp node rm UDE-003` (sin knowledge apuntando) | Sin warning extra. Comportamiento normal. |
| **tree walk** | | |
| K5.13 | `ltp tree walk T --show-knowledge` | Cada nodo en output incluye resumen de knowledge (count por relation). |

---

## Fase K6: Tests End-to-End (Workflows Knowledge)

**Scope**: Workflows completos de hypothesis-driven analysis, inbox management, contradicciones.

**Archivos**: `tests/e2e_knowledge.rs`

**Peso estimado**: 12%

### UATs

| ID | Workflow | Resultado esperado |
|----|---------|-------------------|
| K6.1 | **Hypothesis-driven cycle**: knowledge add (hypothesis) → link to RC → node edit --epistemic hypothesis → knowledge edit --status verified → node edit --epistemic fact → validate clean | Ciclo completo de promoción funciona. |
| K6.2 | **Refutation cascade**: knowledge add → link → build chain on RC → knowledge edit --status refuted → status reports weakened node | Status detecta nodo sostenido solo por evidencia refutada. |
| K6.3 | **Inbox workflow**: add 5 KN sin links → list --unlinked (5) → link 3 → list --unlinked (2) → link 2 → list --unlinked (0) | Inbox se vacía correctamente. |
| K6.4 | **Contradiction detection**: KN-001 supports UDE-003, KN-002 contradicts UDE-003. `validate` | Warning `EPISTEMIC_CONTRADICTED` (UDE-003 es fact). |
| K6.5 | **Multi-target link**: KN-001 supports UDE-003, KN-001 supports RC-001. `knowledge inspect KN-001` | Ambos links visibles con labels resueltos. |
| K6.6 | **Undo roundtrip**: knowledge add → knowledge link → undo → verify link gone → undo → verify KN gone | Undo granular funciona sobre knowledge. |
| K6.7 | **Batch + undo**: begin-batch → knowledge add + link + node edit epistemic → end-batch → undo | Todo el batch se revierte en un undo. |
| K6.8 | **Node rm + dangling**: link KN to UDE → node rm UDE → validate → DANGLING_KNOWLEDGE_REF | Warning correcto tras eliminación de target. |
| K6.9 | **Backwards compatibility**: workspace con nodos sin campo epistemic → node list --epistemic hypothesis | Nodos existentes se tratan como hypothesis (default). |
| K6.10 | **Knowledge + trace integration**: build chain RC→INT→UDE, link knowledge a cada nodo, trace --show-knowledge | Cada entry muestra su knowledge con status/confidence. |

---

## Fase K7: MCP Server (Knowledge Tools)

**Scope**: Exponer los 7 comandos knowledge como tools MCP + extensiones de tools existentes.

**Archivos**: `src/mcp/tools.rs`, `src/mcp/dispatch.rs`

**Peso estimado**: 17%

### UATs

| ID | Acción | Resultado esperado |
|----|--------|-------------------|
| K7.1 | `tools/list` | Incluye 7 nuevos tools: `ltp/knowledge_add`, `ltp/knowledge_edit`, `ltp/knowledge_rm`, `ltp/knowledge_inspect`, `ltp/knowledge_list`, `ltp/knowledge_link`, `ltp/knowledge_unlink`. |
| K7.2 | Invocar `ltp/knowledge_add` via JSON-RPC | Mismo resultado que CLI. Genera undo entry. |
| K7.3 | Invocar `ltp/knowledge_link` via JSON-RPC | Link creado. Target validado. |
| K7.4 | Invocar `ltp/knowledge_list` con filtro `unlinked: true` | Retorna items sin links. |
| K7.5 | Invocar `ltp/knowledge_list` con filtro `target: "UDE-003"` | Retorna items linked a ese nodo. |
| K7.6 | `ltp/node_add` con campo `epistemic` en params | Nodo creado con epistemic status. |
| K7.7 | `ltp/node_edit` con campo `epistemic` en params | Epistemic actualizado. |
| K7.8 | `ltp/node_list` con filtro `epistemic` en params | Filtra correctamente. |
| K7.9 | `ltp/trace` con `show_knowledge: true` en params | Incluye knowledge por nodo. |
| K7.10 | `ltp/tree_walk` con `show_knowledge: true` en params | Incluye resumen knowledge por nodo. |
| K7.11 | `ltp/status` | Incluye `knowledge_health` en output. |
| K7.12 | `ltp/validate` | Reporta warnings de knowledge (DANGLING_KNOWLEDGE_REF, EPISTEMIC_*). |
| K7.13 | `ltp/knowledge_add` con params inválidos (sin source) | Error JSON-RPC -32602 con detalle `SOURCE_REQUIRED`. |
| K7.14 | `ltp/knowledge_link` con target inexistente | isError: true, error `TARGET_NOT_FOUND`. |

---

## Estimación de Complejidad

| Fase | Archivos principales | UATs | Complejidad | Peso |
|------|---------------------|:----:|-------------|:----:|
| K1 | knowledge/types.rs, storage.rs, workspace/ | 4 | Baja | 8% |
| K2 | knowledge/add,edit,rm,inspect,list.rs | 21 | Media | 18% |
| K3 | knowledge/link,unlink.rs | 13 | Media | 15% |
| K4 | node/types.rs, node/add,edit,list.rs | 8 | Baja | 10% |
| K5 | status, validate, trace, node/rm | 13 | Media-Alta | 20% |
| K6 | tests/e2e_knowledge.rs | 10 | Media | 12% |
| K7 | mcp/tools.rs, mcp/dispatch.rs | 14 | Media | 17% |
| | **TOTAL** | **83** | | **100%** |

---

## Estimación Temporal

Basado en la velocity del proyecto original (191 UATs completadas en ~2 días de trabajo intensivo con factor 1.0x):

| Métrica | Valor |
|---------|-------|
| UATs totales Knowledge Pool | 83 |
| Ratio vs proyecto original | 83/191 = 43% |
| Complejidad relativa | Más baja (sin DFS, sin DAG validation, sin macro_edges — primitivas CRUD + queries) |
| Estimación optimista | 1 día de trabajo intensivo |
| Estimación conservadora | 1.5 días |
| Factor de riesgo | K5 (integración) puede requerir refactoring de validate/trace |

---

## Decisiones Técnicas

1. **Backwards compatibility**: El campo `epistemic` usa `#[serde(default)]` → nodos existentes sin el campo deserializan como `Hypothesis`. No se requiere migración.
2. **Knowledge en undo/redo**: Participa del mismo pipeline. `before_mutation` captura `knowledge/KN-XXX.json`. Sin cambios en HistoryManager.
3. **Init migration**: Si workspace existe sin `knowledge/`, se crea on-demand al primer acceso (K1.2). No requiere comando de migración manual.
4. **Scan para queries inversas**: `list --target X` escanea todos los KN files. Con <1000 items, <10ms. YAGNI un índice.
5. **Error codes nuevos**: `SOURCE_REQUIRED`, `KNOWLEDGE_NOT_FOUND`, `TARGET_NOT_FOUND`, `LINK_NOT_FOUND`, `DUPLICATE_LINK`, `DANGLING_KNOWLEDGE_REF`, `EPISTEMIC_UNGROUNDED`, `EPISTEMIC_CONTRADICTED`, `EPISTEMIC_UPGRADEABLE`, `KNOWLEDGE_ORPHANED`, `KNOWLEDGE_DIR_CREATED`.
