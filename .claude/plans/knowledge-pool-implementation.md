# Plan de Implementacion — Knowledge Pool (ADR-012)

**Fecha**: 2026-08-17
**Spec**: [KNOWLEDGE_SPEC.md](../../KNOWLEDGE_SPEC.md)
**ADR**: ADR-012
**Prerequisito**: Motor completo (F1-F12 al 100%)
**UATs detalladas**: [knowledge-pool-uats-deep.md](knowledge-pool-uats-deep.md) (224 UATs)

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

- **K1** (fundacion): schema, storage, init, counters — base minima. **[COMPLETADA]**
- **K2** (CRUD): knowledge add/edit/rm/inspect/list — operaciones standalone.
- **K3** (linking): knowledge link/unlink + validate refs — conecta con el grafo.
- **K4** (epistemic): campo en nodos + node add/edit/list extensions.
- **K5** (integracion): status/validate/trace extensions — reportes y queries.
- **K6** (E2E): workflows completos hypothesis-driven.
- **K7** (MCP): exposicion de los nuevos tools en ltp-mcp.

---

## Decisiones de Diseno (Resueltas por Analisis de UATs)

| # | Decision | Resolucion | Razon |
|---|----------|-----------|-------|
| D1 | Mismo target con different relations | **Permitido** | Caso legitimo: "dato refuerza existencia pero contradice magnitud" |
| D2 | Link a nodo huerfano (en pool, no en tree) | **Permitido** | Validacion de attachment es concern del tree, no del knowledge |
| D3 | `unlink --from X` con multiples links al target | **Elimina TODOS** | Unlink es por target, no por relation. Simplifica la API |
| D4 | `list --target X` con KN que tiene multiples links a X | **Una vez** con array de relations | Evita duplicados confusos en el output |
| D5 | UNGROUNDED threshold | **0 supports totales** (cualquier status) | Cualquier support (incluso unverified) es grounding. Sin support = no grounded |
| D6 | UPGRADEABLE con contradiccion activa | **No se emite** | No sugieres promover algo que esta contradecido por evidencia verified |
| D7 | Validate con tree filter y knowledge | **Solo nodos en ese tree** | Consistente con validate existente que es per-tree |
| D8 | `node split` y knowledge refs | **Deja dangling** | El ID original desaparece; es responsabilidad del usuario re-linkear |

---

## Fase K1: Fundacion Knowledge (Schema, Storage, Init) — COMPLETADA

**Scope**: Definir structs/enums, extender Storage trait, crear `knowledge/` en init, extender counters.

**Archivos creados/modificados**: `src/knowledge/mod.rs`, `src/knowledge/types.rs`, `src/storage.rs`, `src/errors.rs`, `src/workspace/fs_storage.rs`, `src/workspace/counters.rs`, `src/main.rs`, `src/mcp/dispatch.rs`

**Peso**: 8% | **UATs**: 16 | **Estado**: COMPLETADA

### Diseno Implementado

```rust
pub enum KnowledgeType { Measurement, Testimony, Hypothesis, Document, Observation, Derived }
pub enum KnowledgeStatus { Unverified, Verified, Refuted, Superseded }
pub enum Confidence { High, Medium, Low }
pub enum KnowledgeRelation { Supports, Contradicts, Contextualizes }
pub struct KnowledgeSource { uri: Option<String>, excerpt: Option<String> }
pub struct KnowledgeLink { target: String, relation: KnowledgeRelation }
pub struct KnowledgeItem { id, knowledge_type, label, status, confidence, source, captured, links, tags }
```

Storage trait extendido con: `load_knowledge`, `save_knowledge`, `delete_knowledge`, `list_knowledge_ids`, `ensure_knowledge_dir`.

### UATs (16) — Ver K1.1-K1.16 en [knowledge-pool-uats-deep.md](knowledge-pool-uats-deep.md)

---

## Fase K2: CRUD de Knowledge Items — COMPLETADA

**Scope**: `knowledge add/edit/rm/inspect/list` — operaciones standalone sin interaccion con el grafo.

**Archivos**: `src/knowledge/commands.rs`, CLI dispatch en `src/main.rs`

**Peso**: 18% | **UATs**: 47 | **Estado**: COMPLETADA

### Diseno

Subcommand `knowledge` con subcomandos `add`, `edit`, `rm`, `inspect`, `list`.

```rust
#[derive(Subcommand)]
enum KnowledgeCommands {
    Add { label: String, #[arg(long)] r#type: KnowledgeType, ... },
    Edit { id: String, #[arg(long)] label: Option<String>, ... },
    Rm { ids: String },  // comma-separated
    Inspect { id: String },
    List { #[arg(long)] r#type: Option<KnowledgeType>, ... },
}
```

Validaciones en add:
- `label` no vacio (error `LABEL_REQUIRED`)
- `source` con al menos uri o excerpt no vacios (error `SOURCE_REQUIRED`)
- Counter no incrementa si validacion falla

Validaciones en edit:
- `--label ""` rechazado (LABEL_REQUIRED)
- `--source-uri "" --source-excerpt ""` rechazado si ambos vaciarian source (SOURCE_REQUIRED)
- `--add-tag` deduplica
- `--rm-tag` de tag inexistente: no-op silencioso (o warning leve)

Batch rm:
- Parcial: si un ID no existe, reporta error para ese ID pero borra los demas

Undo/redo:
- Cada operacion (add/edit/rm) genera entry en undo
- Counter NO retrocede con undo (IDs son monotonicos)

### UATs (47) — Ver K2.1-K2.47 en [knowledge-pool-uats-deep.md](knowledge-pool-uats-deep.md)

Categorias: 12 happy, 14 boundary, 8 interaction, 1 corrupt, 4 idempotent, 4 ordering.

---

## Fase K3: Linking (Vinculos al Grafo)

**Scope**: `knowledge link/unlink`, filtro `--target` en list, validacion de integridad referencial de targets.

**Archivos**: `src/knowledge/commands.rs` (extension), resolucion de targets contra pool+trees

**Peso**: 15% | **UATs**: 37

### Diseno

```
ltp knowledge link KN-001 --to UDE-003 --relation supports
ltp knowledge unlink KN-001 --from UDE-003
ltp knowledge list --target UDE-003 [--relation supports]
```

Resolucion de targets:
- Nodos: buscar en node pool (`nodes/`)
- Edges (LINK-XXX): buscar en todos los trees (`.edges[]`)
- Assumptions (ASM-XXX): buscar en todos los trees (edges con assumptions)

Reglas de linking (D1, D2):
- Mismo target + misma relation = DUPLICATE_LINK (warning, idempotente)
- Mismo target + diferente relation = PERMITIDO (D1)
- Target huerfano (en pool, no en tree) = PERMITIDO (D2)
- Target con status invalidated/broken = PERMITIDO (motor no juzga)
- Target interior a macro_edge = PERMITIDO (nodo sigue en pool)
- Target es MACRO-XXX = NOT FOUND (macro_edges no son entidades standalone)

Unlink (D3):
- `--from X` elimina TODOS los links con target=X (sin filtrar por relation)

List con --target (D4):
- KN aparece una vez con array de relations matching

### UATs (37) — Ver K3.1-K3.37 en [knowledge-pool-uats-deep.md](knowledge-pool-uats-deep.md)

Categorias: 6 happy, 6 boundary, 8 interaction, 1 idempotent, 9 referential.

---

## Fase K4: Campo Epistemico en Nodos

**Scope**: Anadir campo `epistemic` a struct Node, extender `node add/edit/list/inspect` con `--epistemic`.

**Archivos**: `src/node/types.rs`, `src/node/commands.rs`, CLI dispatch

**Peso**: 10% | **UATs**: 16

### Diseno

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
#[serde(default = "default_epistemic", skip_serializing_if = "is_hypothesis")]
pub epistemic: EpistemicStatus,
```

- Default: `Hypothesis`
- Se OMITE del JSON si es hypothesis (backwards-compatible con nodos existentes)
- `node inspect` muestra el valor efectivo (incluyendo default)
- `node list --epistemic hypothesis` incluye nodos sin campo explicito

### UATs (16) — Ver K4.1-K4.16 en [knowledge-pool-uats-deep.md](knowledge-pool-uats-deep.md)

Categorias: 5 happy, 4 boundary, 2 interaction, 2 corrupt, 1 idempotent.

---

## Fase K5: Integracion con Comandos Existentes

**Scope**: Extender `status`, `validate`, `trace`, `node rm`, `tree walk` con awareness de knowledge pool.

**Archivos**: `src/workspace/status.rs` (o donde viva status), `src/validate/knowledge.rs`, `src/trace/mod.rs`, `src/node/commands.rs`, `src/tree/commands.rs`

**Peso**: 20% | **UATs**: 47

### Diseno

#### Status — `knowledge_health`
```json
{
  "knowledge_health": {
    "total": 10,
    "unlinked_items": 2,
    "contradictions": 1,
    "by_status": { "unverified": 3, "verified": 5, "refuted": 1, "superseded": 1 },
    "epistemic_coverage": { "fact": 3, "hypothesis": 5, "assumption": 1, "derived": 1 }
  }
}
```

#### Validate — 4 warnings nuevos

| Warning | Condicion | Aplica a |
|---------|-----------|----------|
| `DANGLING_KNOWLEDGE_REF` | KN.link.target no existe en pool ni trees | Cualquier link |
| `EPISTEMIC_UNGROUNDED` | Nodo `fact` con 0 supports totales (D5) | Solo nodos `fact` |
| `EPISTEMIC_CONTRADICTED` | Nodo `fact` con >= 1 KN `contradicts` verified | Solo nodos `fact` |
| `EPISTEMIC_UPGRADEABLE` | Nodo `hypothesis/assumption` con >= 2 KN `supports` verified Y 0 contradicts verified (D6) | Solo `hypothesis`/`assumption` |

Validate con `--tree T` (D7): solo reporta para nodos attached a T.

#### Trace — `--show-knowledge`
Cada nodo en chain incluye `knowledge: [{id, relation, status, confidence}]`.
Solo knowledge linked directamente al nodo (no a sus edges).
Sin flag = output normal (backwards-compatible).

#### Node rm — Warning `KNOWLEDGE_ORPHANED`
Warning informativo con IDs de KN afectados. El nodo se borra igualmente. Los KN quedan con dangling refs.

#### Tree walk — `--show-knowledge`
Cada nodo muestra count por relation: `knowledge: {supports: 2, contradicts: 1, contextualizes: 0}`.

### UATs (47) — Ver K5.1-K5.47 en [knowledge-pool-uats-deep.md](knowledge-pool-uats-deep.md)

Categorias: 8 happy, 22 boundary, 4 interaction, 5 referential.

---

## Fase K6: Tests End-to-End (Workflows)

**Scope**: Workflows completos que combinan multiples features y verifican coherencia end-to-end.

**Archivos**: `tests/e2e_knowledge.rs`

**Peso**: 12% | **UATs**: 28

### Workflows Criticos

1. **Hypothesis-driven cycle** (K6.1-K6.2): add → link → promote → validate
2. **Refutation cascade** (K6.3-K6.4): evidencia refutada deja nodo sin grounding
3. **Inbox management** (K6.5): unlinked items como inbox que se vacia
4. **Contradiction detection** (K6.6-K6.7): supports vs contradicts en mismo nodo
5. **Undo roundtrip** (K6.10-K6.11): granularidad correcta de undo/redo
6. **Batch + undo** (K6.12): atomicidad de batch con knowledge
7. **Cross-feature interactions** (K6.19-K6.26): invalidate, collapse, split, group, dissolve, nbr rm
8. **Status coherence** (K6.27-K6.28): status siempre refleja estado real

### UATs (28) — Ver K6.1-K6.28 en [knowledge-pool-uats-deep.md](knowledge-pool-uats-deep.md)

Categorias: 7 happy, 3 boundary, 15 interaction, 3 ordering.

---

## Fase K7: MCP Server (Knowledge Tools)

**Scope**: Exponer los 7 comandos knowledge como tools MCP + extensiones de tools existentes.

**Archivos**: `src/mcp/tools.rs`, `src/mcp/dispatch.rs`

**Peso**: 17% | **UATs**: 33

### Tools Nuevos

| Tool | Params principales |
|------|-------------------|
| `ltp/knowledge_add` | label, type, source_uri, source_excerpt, status, confidence, tags |
| `ltp/knowledge_edit` | id, label?, status?, confidence?, source_uri?, source_excerpt?, add_tags?, rm_tags? |
| `ltp/knowledge_rm` | ids (comma-separated) |
| `ltp/knowledge_inspect` | id |
| `ltp/knowledge_list` | type?, status?, confidence?, unlinked?, target?, relation?, tag? |
| `ltp/knowledge_link` | id, target, relation |
| `ltp/knowledge_unlink` | id, target |

### Extensiones a Tools Existentes

| Tool | Param nuevo |
|------|-------------|
| `ltp/node_add` | `epistemic?: string` |
| `ltp/node_edit` | `epistemic?: string` |
| `ltp/node_list` | `epistemic?: string` |
| `ltp/trace` | `show_knowledge?: bool` |
| `ltp/tree_walk` | `show_knowledge?: bool` |
| `ltp/status` | (sin params, output extendido automatico) |
| `ltp/validate` | (sin params, warnings extendidos automatico) |

### UATs (33) — Ver K7.1-K7.33 en [knowledge-pool-uats-deep.md](knowledge-pool-uats-deep.md)

Categorias: 16 happy, 6 boundary, 3 interaction, 1 idempotent, 1 ordering, 1 referential.

---

## Estimacion de Complejidad (Revisada)

| Fase | Archivos principales | UATs | Complejidad | Peso |
|------|---------------------|:----:|-------------|:----:|
| K1 | knowledge/types.rs, storage.rs, workspace/ | 16 | Baja | 8% |
| K2 | knowledge/commands.rs, main.rs | 47 | Media | 18% |
| K3 | knowledge/commands.rs (link/unlink), target resolution | 37 | Media-Alta | 15% |
| K4 | node/types.rs, node/commands.rs | 16 | Baja | 10% |
| K5 | validate/knowledge.rs, trace, status, node/rm | 47 | Alta | 20% |
| K6 | tests/e2e_knowledge.rs | 28 | Media | 12% |
| K7 | mcp/tools.rs, mcp/dispatch.rs | 33 | Media | 17% |
| | **TOTAL** | **224** | | **100%** |

---

## Estimacion Temporal

| Metrica | Valor |
|---------|-------|
| UATs totales Knowledge Pool | 224 |
| Complejidad principal | K3 (target resolution across trees) y K5 (validate matrix) |
| Estimacion optimista | 2 dias de trabajo intensivo |
| Estimacion conservadora | 3 dias |
| Factor de riesgo principal | K5 validate interactions con operaciones destructivas existentes |

---

## Error Codes Nuevos

| Code | Fase | Contexto |
|------|------|----------|
| `LABEL_REQUIRED` | K2 | add/edit con label vacio |
| `SOURCE_REQUIRED` | K2 | add/edit sin uri ni excerpt validos |
| `KNOWLEDGE_NOT_FOUND` | K2+ | ID no existe en pool |
| `TARGET_NOT_FOUND` | K3 | link target no existe en pool/trees |
| `LINK_NOT_FOUND` | K3 | unlink target no esta en KN.links |
| `DUPLICATE_LINK` | K3 | link ya existe (warning, no error) |
| `DANGLING_KNOWLEDGE_REF` | K5 | validate: KN.link.target no resolvible |
| `EPISTEMIC_UNGROUNDED` | K5 | validate: fact sin supports |
| `EPISTEMIC_CONTRADICTED` | K5 | validate: fact con contradiccion verified |
| `EPISTEMIC_UPGRADEABLE` | K5 | validate: hypothesis con >=2 supports verified |
| `KNOWLEDGE_ORPHANED` | K5 | node rm: KN apuntaba al nodo eliminado |
| `KNOWLEDGE_DIR_CREATED` | K1 | auto-creacion de knowledge/ on demand |
