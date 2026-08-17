# Especificación del Knowledge Pool (KNOWLEDGE_SPEC.md)

**Estado**: Diseño Aprobado (pendiente de implementación)
**ADR**: ADR-012

## 1. Principio de Diseño

El Knowledge Pool gestiona información epistemológica del sistema bajo análisis — hechos, hipótesis, mediciones, testimonios — separada del grafo causal pero vinculada a sus entidades.

```
┌─────────────────────────────────────────────────────────────────────┐
│                    GRAFO CAUSAL (existente)                          │
│  nodes/ ──── edges ──── trees/ ──── assumptions                     │
│     ▲            ▲                        ▲                         │
│     │            │                        │                         │
│  supports    supports              supports/contradicts             │
│     │            │                        │                         │
└─────┼────────────┼────────────────────────┼─────────────────────────┘
      │            │                        │
┌─────┼────────────┼────────────────────────┼─────────────────────────┐
│     └────────────┴────────────────────────┘                         │
│                   KNOWLEDGE POOL (nuevo)                             │
│  knowledge/KN-001.json, KN-002.json, ...                            │
│  • Ciclo de vida independiente                                      │
│  • Links no-causales (supports, contradicts, contextualizes)        │
│  • Puede existir sin vincularse (conocimiento huérfano)             │
└─────────────────────────────────────────────────────────────────────┘
```

**Separación de responsabilidades (ADR-001):**
- El motor persiste, linkea, filtra y reporta consistencia.
- El motor NUNCA juzga si la evidencia es suficiente, si una hipótesis debería promoverse a hecho, o si una contradicción invalida un nodo.
- Esas evaluaciones las hace el agente LLM o el humano.

## 2. Schema del Knowledge Item

```json
{
  "id": "KN-001",
  "type": "measurement",
  "label": "Media de entrega 18.3 días en Q2-2026",
  "status": "verified",
  "confidence": "high",
  "source": {
    "uri": "obsidian://vault/MyBrain/notas/kpi-logistica-q2.md",
    "excerpt": "El tiempo medio de entrega fue 18.3 días, vs 12.1 en Q1"
  },
  "captured": "2026-08-17",
  "links": [
    {"target": "UDE-003", "relation": "supports"},
    {"target": "LINK-007", "relation": "supports"},
    {"target": "ASM-002", "relation": "contradicts"}
  ]
}
```

### 2.1. Campos

| Campo | Tipo | Requerido | Descripción |
|-------|------|-----------|-------------|
| `id` | String | Sí | ID secuencial único (`KN-001`, `KN-002`...) |
| `type` | Enum | Sí | Qué ES este item |
| `label` | String | Sí | Descripción atómica del conocimiento |
| `status` | Enum | Sí | Qué SABEMOS de este item |
| `confidence` | Enum | No | Nivel de confianza declarado (default: `medium`) |
| `source` | Object | Sí* | Origen del conocimiento (*al menos `uri` o `excerpt`) |
| `source.uri` | String | No* | Puntero a la fuente original |
| `source.excerpt` | String | No* | Extracto textual de la fuente |
| `captured` | String (ISO 8601) | Sí | Fecha de captura |
| `links` | Array | No | Vínculos a entidades del grafo (default: `[]`) |
| `links[].target` | String | Sí | ID del target (nodo, link o assumption) |
| `links[].relation` | Enum | Sí | Tipo de relación |
| `tags` | Array[String] | No | Tags libres para categorización |

### 2.2. Enums

**`type`** — Naturaleza del knowledge item:

| Valor | Descripción |
|-------|-------------|
| `measurement` | Dato cuantitativo medido (KPI, métrica, conteo) |
| `testimony` | Declaración de una persona (entrevista, mensaje, reunión) |
| `hypothesis` | Suposición formulada pendiente de verificar |
| `document` | Referencia a documento formal (reporte, política, contrato) |
| `observation` | Observación directa de comportamiento del sistema |
| `derived` | Conclusión derivada de otros knowledge items |

**`status`** — Estado epistémico del item:

| Valor | Descripción |
|-------|-------------|
| `unverified` | Capturado pero no confirmado (default) |
| `verified` | Confirmado por evidencia o validación cruzada |
| `refuted` | Demostrado como falso o incorrecto |
| `superseded` | Reemplazado por información más reciente |

**`confidence`** — Nivel de confianza del declarante:

| Valor | Descripción |
|-------|-------------|
| `high` | Alta seguridad (múltiples fuentes, dato duro) |
| `medium` | Confianza razonable (fuente creíble, sin contradicción) |
| `low` | Baja seguridad (fuente no verificada, dato parcial, rumor) |

**`relation`** — Tipo de vínculo con entidad del grafo:

| Valor | Descripción |
|-------|-------------|
| `supports` | Este knowledge respalda la existencia/veracidad del target |
| `contradicts` | Este knowledge contradice o debilita el target |
| `contextualizes` | Información de fondo relevante, no directamente probatoria |

### 2.3. Targets válidos para links

| Prefijo del ID | Entidad target | Ejemplo |
|----------------|----------------|---------|
| `UDE-`, `RC-`, `INJ-`, `NC-`, etc. | Nodo del pool global | `"target": "UDE-003"` |
| `LINK-` | Edge de un tree | `"target": "LINK-007"` |
| `ASM-` | Assumption de un edge | `"target": "ASM-002"` |

## 3. Campo Epistémico en Nodos

Nuevo campo opcional en nodos existentes (`nodes/*.json`):

```json
{
  "id": "UDE-003",
  "type": "UDE",
  "label": "Los tiempos de entrega superan 15 días",
  "epistemic": "fact"
}
```

**Enum `epistemic`:**

| Valor | Descripción |
|-------|-------------|
| `fact` | Hecho establecido — el usuario lo declara como verdadero |
| `hypothesis` | Hipótesis por confirmar (default para nodos nuevos) |
| `assumption` | Supuesto aceptado sin verificación explícita |
| `derived` | Conclusión derivada de la estructura lógica del grafo |

**Reglas:**
- Default: `hypothesis` si no se especifica.
- El motor persiste y filtra. Nunca promueve ni degrada automáticamente.
- `validate` emite warnings de consistencia (ver sección 7).

## 4. Estructura en Disco

```
mi-proyecto-ltp/
├── nodes/              # Pool de nodos (existente)
├── trees/              # Vistas topológicas (existente)
├── knowledge/          # ← NUEVO: Pool de knowledge items
│   ├── KN-001.json
│   ├── KN-002.json
│   └── KN-003.json
├── ltp.config.json     # Config global (existente)
└── .ltp/
    ├── counters.json   # Añade contador "knowledge": N
    ├── undo/           # Knowledge participa en snapshots
    └── ...
```

`ltp init` crea la carpeta `knowledge/` vacía junto a `nodes/` y `trees/`.

## 5. Comandos

### 5.1. `ltp knowledge add "<label>" --type <TYPE> [opciones]`

Crea un knowledge item en `knowledge/`.

| Flag | Requerido | Descripción |
|------|-----------|-------------|
| `--type <TYPE>` | Sí | measurement, testimony, hypothesis, document, observation, derived |
| `--source-uri "<uri>"` | No* | URI de la fuente |
| `--source-excerpt "<texto>"` | No* | Extracto textual |
| `--status <STATUS>` | No | Default: `unverified` |
| `--confidence <LEVEL>` | No | Default: `medium` |
| `--tags t1,t2` | No | Tags libres |

*Al menos `--source-uri` o `--source-excerpt` es requerido.

**Respuesta:**
```json
{
  "success": true,
  "action": "knowledge_add",
  "data": {
    "created_knowledge_id": "KN-004",
    "type": "testimony",
    "status": "unverified"
  }
}
```

### 5.2. `ltp knowledge edit <KN-ID> [opciones]`

Modifica campos de un knowledge item existente.

| Flag | Descripción |
|------|-------------|
| `--label "<texto>"` | Nuevo label |
| `--status <STATUS>` | Nuevo status |
| `--confidence <LEVEL>` | Nuevo confidence |
| `--source-uri "<uri>"` | Nuevo URI |
| `--source-excerpt "<texto>"` | Nuevo excerpt |
| `--add-tag <tag>` | Añadir tag |
| `--rm-tag <tag>` | Quitar tag |

### 5.3. `ltp knowledge rm <KN-ID>[,<KN-ID2>]`

Elimina knowledge items del pool. Batch: acepta lista separada por comas.

No afecta a los nodos/edges/assumptions que eran targets de sus links.

### 5.4. `ltp knowledge link <KN-ID> --to <TARGET_ID> --relation <RELATION>`

Vincula un knowledge item a una entidad del grafo.

| Flag | Requerido | Descripción |
|------|-----------|-------------|
| `--to <ID>` | Sí | Node ID, LINK ID, o ASM ID |
| `--relation <R>` | Sí | supports, contradicts, contextualizes |

**Validaciones:**
- Error `KNOWLEDGE_NOT_FOUND` si KN-ID no existe.
- Error `TARGET_NOT_FOUND` si el target no existe en el workspace.
- Warning `DUPLICATE_LINK` si ya existe un link con mismo target y relation (no bloquea, es idempotente).

### 5.5. `ltp knowledge unlink <KN-ID> --from <TARGET_ID>`

Elimina el vínculo entre un knowledge item y un target.

Error `LINK_NOT_FOUND` si no existe esa combinación.

### 5.6. `ltp knowledge list [opciones]`

Lista knowledge items con filtros.

| Flag | Descripción |
|------|-------------|
| `--type <TYPE>` | Filtrar por tipo |
| `--status <STATUS>` | Filtrar por status |
| `--confidence <LEVEL>` | Filtrar por confidence |
| `--unlinked` | Solo items sin links (inbox) |
| `--target <ID>` | Items vinculados a un target específico |
| `--relation <R>` | Filtrar por tipo de relación (requiere --target) |
| `--tag <tag>` | Filtrar por tag |

**Respuesta (ejemplo con `--target UDE-003`):**
```json
{
  "success": true,
  "action": "knowledge_list",
  "data": {
    "items": [
      {
        "id": "KN-001",
        "type": "measurement",
        "label": "Media de entrega 18.3 días en Q2-2026",
        "status": "verified",
        "confidence": "high",
        "relation": "supports"
      },
      {
        "id": "KN-007",
        "type": "testimony",
        "label": "El director dice que mejoraron en julio",
        "status": "unverified",
        "confidence": "medium",
        "relation": "contradicts"
      }
    ],
    "total": 2
  }
}
```

### 5.7. `ltp knowledge inspect <KN-ID>`

Muestra detalle completo de un knowledge item incluyendo todos sus campos y links resueltos (con labels de los targets).

**Respuesta:**
```json
{
  "success": true,
  "action": "knowledge_inspect",
  "data": {
    "id": "KN-001",
    "type": "measurement",
    "label": "Media de entrega 18.3 días en Q2-2026",
    "status": "verified",
    "confidence": "high",
    "source": {
      "uri": "obsidian://vault/MyBrain/notas/kpi-logistica-q2.md",
      "excerpt": "El tiempo medio de entrega fue 18.3 días, vs 12.1 en Q1"
    },
    "captured": "2026-08-17",
    "tags": ["logística", "q2-2026"],
    "links": [
      {
        "target": "UDE-003",
        "target_label": "Los tiempos de entrega superan 15 días",
        "target_type": "UDE",
        "relation": "supports"
      }
    ]
  }
}
```

## 6. Interacción con Comandos Existentes

### 6.1. `ltp status`

Añade sección de knowledge al reporte de salud:

```json
{
  "knowledge_health": {
    "total_items": 12,
    "unlinked_items": 3,
    "contradictions": 1,
    "by_status": {"verified": 5, "unverified": 6, "refuted": 1, "superseded": 0},
    "epistemic_coverage": {
      "nodes_as_fact": 8,
      "nodes_as_hypothesis": 4,
      "nodes_without_epistemic": 2
    }
  }
}
```

### 6.2. `ltp validate`

Nuevos warnings (no bloqueantes):

| Código | Condición | Mensaje |
|--------|-----------|---------|
| `DANGLING_KNOWLEDGE_REF` | Knowledge item linkea a target que no existe | "KN-003 references UDE-005 which does not exist" |
| `EPISTEMIC_UNGROUNDED` | Nodo con `epistemic: "fact"` y 0 knowledge items `supports` | "UDE-003 declared as fact but has no supporting knowledge" |
| `EPISTEMIC_CONTRADICTED` | Nodo con `epistemic: "fact"` y ≥1 knowledge `contradicts` con status `verified` | "UDE-003 declared as fact but contradicted by KN-007 (verified)" |
| `EPISTEMIC_UPGRADEABLE` | Nodo con `epistemic: "hypothesis"` y ≥2 knowledge `supports` con status `verified` | "RC-001 has 2+ verified supports — consider promoting to fact" |

### 6.3. `ltp node rm`

Nuevo warning al eliminar nodo:

```json
{
  "warnings": [
    {"code": "KNOWLEDGE_ORPHANED", "detail": "2 knowledge items reference UDE-003: KN-001, KN-007. Their links are now dangling."}
  ]
}
```

No bloquea la eliminación. Los knowledge items conservan el link (se reportará como `DANGLING_KNOWLEDGE_REF` en validate).

### 6.4. `ltp trace` (extensión)

Nuevo flag `--show-knowledge`:

```
ltp trace UDE-003 --tree tree-crt --direction upstream --show-knowledge
```

Añade a cada entry del trace un campo `knowledge`:

```json
{
  "chain": [
    {
      "node": "RC-001",
      "knowledge": [
        {"id": "KN-005", "relation": "supports", "status": "verified", "confidence": "high"}
      ],
      "link_to_next": {"id": "LINK-002", "status": "active"}
    },
    {
      "node": "UDE-003",
      "knowledge": [
        {"id": "KN-001", "relation": "supports", "status": "verified", "confidence": "high"},
        {"id": "KN-007", "relation": "contradicts", "status": "unverified", "confidence": "medium"}
      ],
      "link_to_next": null
    }
  ]
}
```

### 6.5. `ltp node add` / `ltp node edit`

Nuevo flag `--epistemic <fact|hypothesis|assumption|derived>`:

```
ltp node add "Los tiempos superan 15 días" --type UDE --epistemic fact
ltp node edit UDE-003 --epistemic fact
```

Default: `hypothesis` si no se especifica en `node add`.

### 6.6. `ltp node list` (extensión)

Nuevo filtro `--epistemic`:

```
ltp node list --tree tree-crt --epistemic hypothesis
```

### 6.7. `ltp init`

Crea carpeta `knowledge/` vacía. Añade `"knowledge": 0` a `.ltp/counters.json`.

## 7. Undo/Redo

Las operaciones sobre `knowledge/` participan en el stack de undo/redo (ADR-009):
- `knowledge add` → snapshot con `before: null` (fichero creado).
- `knowledge edit` → snapshot del estado previo del fichero.
- `knowledge rm` → snapshot del fichero completo.
- `knowledge link` / `unlink` → snapshot del fichero (el link vive en el KN).

Batches (`history begin-batch` / `end-batch`) incluyen operaciones de knowledge si están mezcladas con otras mutaciones.

## 8. Contrato de Salida

Todos los comandos `knowledge *` siguen el contrato estándar del motor:

```json
{
  "success": bool,
  "action": "knowledge_<verb>",
  "workspace": "string",
  "data": { ... },
  "graph_health": { "valid_dag": bool, "orphan_nodes_count": int },
  "errors": [{"code": "...", "detail": "...", ...context}],
  "warnings": [{"code": "...", "detail": "...", ...context}]
}
```

## 9. Workflow de Hypothesis-Driven Analysis

Este workflow es orquestado por el agente (skill `ltp-mcp`), no por el motor:

```
1. Capturar hipótesis     → knowledge add "..." --type hypothesis
2. Vincular a nodo        → knowledge link KN-X --to RC-005 --relation supports
3. Construir sobre ella   → (cadena causal normal en CRT/FRT)
4. Buscar evidencia       → (fuentes externas: MCP, Obsidian, entrevistas)
5a. Confirmar             → knowledge edit KN-X --status verified
                          → node edit RC-005 --epistemic fact
5b. Refutar              → knowledge edit KN-X --status refuted
                          → (status reporta nodos debilitados)
                          → (revisar cadena downstream)
```

El motor habilita este workflow con primitivas; la lógica de decisión vive en el agente.
