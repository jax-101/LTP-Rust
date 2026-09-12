# RFC-002: Meta-Grafo — Sistema, Relaciones entre Arboles, Escenarios y Marcos Temporales

| Campo       | Valor                              |
|-------------|------------------------------------|
| Status      | **EXPLORING**                      |
| Author      | Javier Asensio                     |
| Created     | 2026-09-11                         |
| Aplica a    | ltp-engine (original) y ltp-engine-max (fork) |
| Relacionado | RFC-001 (persistencia hibrida / fork con Turso) |
| Lifecycle   | EXPLORING → DRAFT → ACCEPTED/REJECTED |

---

## 1. Motivacion

### El problema

ltp-engine modela arboles individuales (GT, CRT, EC, FRT, PRT, TT, NBR) y permite que compartan nodos via `tree_attach`. Pero **no captura la relacion entre los arboles mismos**.

Un analisis LTP no es una coleccion de diagramas sueltos — es una **narrativa estructurada** sobre un sistema real, visto desde multiples angulos temporales y logicos. Los arboles son secciones de una misma realidad. Lo que falta es capturar esa realidad y como las secciones se conectan.

### Que no se captura hoy

| Pregunta | Respuesta actual |
|----------|-----------------|
| Que sistema estamos analizando? | No existe como entidad. Es implicito. |
| Por que existe este CRT? | Porque alguien lo creo. No hay link explicito a brechas del GT. |
| Como se alimentan los arboles entre si? | Via nodos compartidos. Pero no hay "CRT alimenta EC" explicito. |
| Cual es el marco temporal de cada arbol? | Implicito por tipo (CRT=presente, FRT=futuro). No esta modelado. |
| Que pasa si elijo INJ-A vs INJ-B? | Dos FRTs sueltos sin relacion de "alternativa". |
| Por que se paso del CRT a la EC? | Porque la CRC se encontro. Pero esa transicion no esta registrada. |

### Vision

Agregar una capa de abstraccion — el **meta-grafo** — que modela:

1. **El sistema** bajo analisis (fronteras, contexto, stakeholders)
2. **Las relaciones entre arboles** (derivacion, inyeccion, revision, alternativa)
3. **Los escenarios** (futuros alternativos que divergen en un punto de decision)
4. **Los marcos temporales** (presente, deseado, futuro, transicion)

Esto es independiente de la capa de persistencia: funciona sobre JSON (ltp-engine) o sobre libSQL (ltp-engine-max).

---

## 2. Marco Conceptual

### Los arboles como vistas de una realidad

Todos los artefactos LTP son **lentes** sobre el mismo sistema:

```
                        ┌─────────────────────┐
                        │      SISTEMA         │
                        │  (la realidad bajo   │
                        │   analisis)          │
                        └──────┬──────────────┘
                               │
           ┌───────────────────┼────────────────────┐
           │                   │                     │
     Temporal: HOY       Temporal: DESEADO     Temporal: FUTURO
           │                   │                     │
        ┌──┴──┐             ┌──┴──┐           ┌─────┼──────┐
        │ CRT │             │ GT  │           │ Escenario A │
        └──┬──┘             └─────┘           │  ┌─────┐   │
           │                                  │  │ FRT │   │
     ┌─────┴─────┐                           │  └──┬──┘   │
     │ CRC       │                           │     │      │
     ▼           │                           │  ┌──┴──┐   │
  ┌──────┐       │                           │  │ PRT │   │
  │  EC  │───INJ─┤                           │  └──┬──┘   │
  └──────┘       │                           │  ┌──┴──┐   │
                 │                           │  │ TT  │   │
                 │                           │  └─────┘   │
                 │                           └────────────┘
                 │
                 │                           ┌─────────────┐
                 └─────INJ alternativa──────►│ Escenario B │
                                             │  FRT', PRT' │
                                             │  TT', NBR'  │
                                             └─────────────┘
```

### Las 3 preguntas como arco narrativo

El LTP de Dettmer/Fedurko se estructura en 3 preguntas. Cada pregunta tiene arboles asociados y las transiciones entre preguntas son **handoffs** con nodos puente:

| Pregunta | Arboles | Handoff hacia la siguiente |
|----------|---------|---------------------------|
| Que cambiar? | GT → CRT → EC | CRC del CRT alimenta la EC |
| A que cambiar? | EC → FRT (+ NBR) | INJ de la EC se valida en el FRT |
| Como causar el cambio? | PRT → TT | INJ validada del FRT genera IOs en el PRT |

Hoy estos handoffs son **implicitos** — dependen de que un nodo participe en dos arboles via `tree_attach`. No hay entidad que diga "esta transicion existe porque se encontro la CRC" o "este escenario nace porque se eligio INJ-A en vez de INJ-B".

### Linaje de datos completo

```
GT ──(brechas)──► CRT ──(CRC)──► EC ──(INJ)──► FRT ──(INJ validadas)──► PRT ──(IOs)──► TT
                                                  │
                                                  ▼
                                                 NBR (riesgos → trimming)
```

El flujo retrocede: insights del FRT revisan el CRT; NBRs requieren nuevas inyecciones y potencialmente nuevas ECs. Estos retroalimentaciones tampoco se capturan explicitamente.

---

## 3. Entidades Nuevas

### 3.1 Sistema (System)

La realidad bajo analisis. Todo workspace tiene al menos un sistema.

**Campos**:

| Campo | Tipo | Descripcion |
|-------|------|-------------|
| `id` | String | `SYS-001` |
| `name` | String | "Departamento de Logistica — Empresa X" |
| `description` | String | Descripcion del sistema y su contexto |
| `boundary` | String | Que esta dentro y fuera del analisis |
| `stakeholders` | String[] | Quienes son los interesados |
| `tags` | String[] | Etiquetas para clasificacion |

**Representacion JSON** (ltp-engine):
```json
{
  "id": "SYS-001",
  "name": "Departamento de Logística — Empresa X",
  "description": "Operaciones logísticas de última milla en zona metropolitana",
  "boundary": "Desde recepción en almacén hasta entrega al cliente. Excluye producción y compras.",
  "stakeholders": ["Director de Operaciones", "Equipo de Customer Success", "Partners logísticos"],
  "tags": ["logistics", "last-mile"]
}
```

**Representacion SQL** (ltp-engine-max):
```sql
CREATE TABLE systems (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT,
    boundary    TEXT,
    stakeholders TEXT,          -- JSON array
    tags        TEXT,           -- JSON array
    embedding   F32_BLOB(384)  -- solo en ltp-engine-max
);
```

**Relacion con trees**: Cada arbol pertenece a un sistema.

```sql
-- Agregar columna a trees existente
ALTER TABLE trees ADD COLUMN system_id TEXT REFERENCES systems(id);
```

O en JSON, agregar `"system_id": "SYS-001"` al tree.

### 3.2 Relacion entre Arboles (Tree Relation)

Conexion explicita y tipada entre dos arboles.

**Campos**:

| Campo | Tipo | Descripcion |
|-------|------|-------------|
| `id` | String | `TREL-001` |
| `from_tree` | String → Tree.id | Arbol origen |
| `to_tree` | String → Tree.id | Arbol destino |
| `relation_type` | Enum | Tipo de relacion (ver tabla abajo) |
| `handoff_nodes` | HandoffNode[] | Nodos puente entre arboles (ver seccion 10.5) |
| `proof_standard` | Object (nullable) | Personalizacion de pruebas CLR en la transicion (nullable, solo si hay override) |
| `description` | String | Contexto de por que existe esta relacion |

**Tipos de relacion**:

| Tipo | Significado | Direccion tipica | Ejemplo |
|------|------------|------------------|---------|
| `gap_analysis` | Brechas del GT generan UDEs del CRT | GT → CRT | "REQs no cumplidos se convierten en UDEs" |
| `core_conflict` | CRC del CRT alimenta la EC | CRT → EC | "RC-001 es el conflicto central" |
| `injection` | INJ de la EC se valida en el FRT | EC → FRT | "INJ-005 invalido ASM-012" |
| `risk_check` | INJ del FRT se evalua con NBR | FRT → NBR | "Evaluar efectos negativos de INJ-005" |
| `prerequisite` | INJ validada genera PRT | FRT → PRT | "INJ-005 requiere prerequisitos" |
| `tactical` | IOs del PRT generan acciones del TT | PRT → TT | "IO-002 se descompone en acciones" |
| `alternative` | Dos arboles representan escenarios distintos | FRT-A ↔ FRT-B | "GPS tracking vs Partner logistico" |
| `revision` | Insights de un arbol revisan otro | FRT → CRT | "FRT revela UDE no detectada" |
| `challenges` | NBR cuestiona una INJ y fuerza nueva EC | NBR → EC | "Riesgo obliga a buscar otra inyeccion" |
| `extends` | Un arbol profundiza parte de otro | CRT → CRT | "Sub-CRT para rama especifica" |

**HandoffNode**: puede ser un string simple (`"RC-001"`) para backward-compatibility, o un objeto enriquecido cuando el nodo proviene de un edge agrupado:

```json
// Formato simple (sin contexto de operador):
"handoff_nodes": ["RC-001"]

// Formato enriquecido (con contexto de operador — seccion 10.5):
"handoff_nodes": [
  {
    "node_id": "RC-001",
    "source_operator": "AND"
  }
]
```

`source_operator` registra el operador del edge del que provenia el nodo en el arbol origen. Solo se incluye cuando es relevante (el nodo formaba parte de un grupo AND/OR/MAG). Esto captura informacion que se pierde al cruzar a un arbol de necesidad donde esos operadores no existen.

**`logic_transition`** no se almacena — se calcula de `from_tree.logic` y `to_tree.logic`. Valores posibles: `suf_to_nec`, `nec_to_suf`, `same_suf`, `same_nec`.

**Representacion JSON** (ltp-engine):

Nuevo archivo en el workspace: `relations/TREL-001.json`

```json
{
  "id": "TREL-001",
  "from_tree": "tree-crt-logistica",
  "to_tree": "tree-ec-conflicto-entregas",
  "relation_type": "core_conflict",
  "handoff_nodes": [
    {"node_id": "RC-001", "source_operator": "AND"}
  ],
  "proof_standard": null,
  "description": "RC-001 identificada como CRC del CRT. Genera el conflicto central de la EC."
}
```

**Representacion SQL** (ltp-engine-max):

```sql
CREATE TABLE tree_relations (
    id              TEXT PRIMARY KEY,
    from_tree       TEXT REFERENCES trees(id),
    to_tree         TEXT REFERENCES trees(id),
    relation_type   TEXT NOT NULL,
    handoff_nodes   TEXT,           -- JSON array (strings o objects con source_operator)
    proof_standard  TEXT,           -- JSON, nullable. Solo si hay personalizacion CLR.
    description     TEXT
);
```

### 3.3 Escenario (Scenario)

Una rama de analisis que representa un futuro alternativo. Los escenarios divergen en un punto de decision (tipicamente una INJ alternativa).

**Campos**:

| Campo | Tipo | Descripcion |
|-------|------|-------------|
| `id` | String | `SCN-001` |
| `system_id` | String → System.id | A que sistema pertenece |
| `name` | String | "Escenario GPS Tracking" |
| `description` | String | Descripcion del escenario |
| `parent_scenario` | String → Scenario.id (nullable) | De donde bifurca (null = escenario raiz) |
| `branch_node` | String → Node.id (nullable) | La INJ que genera la bifurcacion |
| `status` | Enum | `active` / `discarded` / `selected` |

**Relacion con trees**: Un escenario agrupa arboles que pertenecen a esa rama de futuro.

```sql
CREATE TABLE scenarios (
    id              TEXT PRIMARY KEY,
    system_id       TEXT REFERENCES systems(id),
    name            TEXT NOT NULL,
    description     TEXT,
    parent_scenario TEXT REFERENCES scenarios(id),
    branch_node     TEXT REFERENCES nodes(id),
    status          TEXT DEFAULT 'active'
);

CREATE TABLE scenario_trees (
    scenario_id TEXT REFERENCES scenarios(id),
    tree_id     TEXT REFERENCES trees(id),
    PRIMARY KEY (scenario_id, tree_id)
);
```

**Representacion JSON** (ltp-engine):

Nuevo archivo: `scenarios/SCN-001.json`

```json
{
  "id": "SCN-001",
  "system_id": "SYS-001",
  "name": "Escenario GPS Tracking",
  "description": "Futuro donde se implementa tracking GPS propio",
  "parent_scenario": null,
  "branch_node": "INJ-005",
  "status": "active",
  "trees": ["tree-frt-gps", "tree-prt-gps", "tree-tt-gps-q1"]
}
```

**Semantica de bifurcacion**:

```
Escenario raiz (shared: GT, CRT, EC)
    │
    ├── SCN-001: "GPS Tracking" (branch_node: INJ-005)
    │   ├── tree-frt-gps
    │   ├── tree-nbr-gps-costes
    │   ├── tree-prt-gps
    │   └── tree-tt-gps-q1
    │
    └── SCN-002: "Partner Logistico" (branch_node: INJ-008)
        ├── tree-frt-partner
        ├── tree-nbr-partner-dependencia
        ├── tree-prt-partner
        └── tree-tt-partner-q1
```

Los arboles del presente (GT, CRT, EC) son **compartidos** — no pertenecen a ningun escenario especifico. Los arboles del futuro (FRT, PRT, TT, NBR) divergen por escenario.

### 3.4 Marco Temporal (Temporal Frame)

Cada arbol se situa en un momento de la narrativa.

**Campos**:

| Campo | Tipo | Descripcion |
|-------|------|-------------|
| `tree_id` | String → Tree.id | Arbol |
| `frame` | Enum | Marco temporal |
| `scenario_id` | String → Scenario.id (nullable) | Null para frames compartidos (present, desired) |

**Frames**:

| Frame | Significado | Arboles tipicos | Compartido entre escenarios? |
|-------|------------|-----------------|------|
| `desired` | Lo que deberia ser | GT | Si |
| `present` | Lo que es hoy | CRT, EC | Si |
| `future` | Lo que sera si se aplican las inyecciones | FRT, NBR | **No** — uno por escenario |
| `transition` | Como llegar del presente al futuro | PRT, TT | **No** — uno por escenario |
| `past` | Analisis historico (opcional) | CRT de periodo anterior | Si |

**Representacion SQL** (ltp-engine-max):

```sql
CREATE TABLE temporal_frames (
    tree_id     TEXT REFERENCES trees(id),
    frame       TEXT NOT NULL,
    scenario_id TEXT REFERENCES scenarios(id),
    PRIMARY KEY (tree_id)
);
```

**Representacion JSON** (ltp-engine): Campo adicional en el tree:

```json
{
  "id": "tree-frt-gps",
  "tree_type": "frt",
  "temporal_frame": "future",
  "scenario_id": "SCN-001",
  ...
}
```

---

## 4. Casos de Uso Desbloqueados

### 4.1 Navegacion narrativa en la UI

**Hoy**: La UI mostraria una lista de arboles sueltos: `tree-gt-logistica`, `tree-crt-logistica`, `tree-ec-conflicto`, etc.

**Con meta-grafo**: La UI muestra la **historia del analisis**:

```
Sistema: "Departamento de Logistica — Empresa X"
Boundary: Desde almacen hasta entrega al cliente

Narrativa:
  1. DESEADO: Goal Tree "Logistica de clase mundial"
     ↓ gap_analysis (REQ-001, REQ-003 no cumplidos)
  2. PRESENTE: CRT "Realidad actual de logistica"
     ↓ core_conflict (CRC: RC-001)
  3. CONFLICTO: EC "Velocidad vs Control en entregas"
     ↓ injection (INJ-005 invalido ASM-012)
  4. FUTURO:
     ├── Escenario A: "GPS Tracking" [ACTIVE]
     │   ├── FRT "Con GPS tracking"
     │   ├── NBR "Riesgo: coste de implementacion"
     │   ├── PRT "Prerequisitos para GPS"
     │   └── TT "Plan tactico Q1-Q2"
     └── Escenario B: "Partner Logistico" [ACTIVE]
         ├── FRT "Con partner logistico"
         ├── NBR "Riesgo: dependencia del partner"
         └── PRT "Prerequisitos para outsourcing"
```

El usuario navega por la narrativa, no por una lista de archivos. Cada transicion entre arboles tiene contexto explicito.

### 4.2 Comparacion de escenarios

Con escenarios como entidades de primer nivel:

```sql
-- Comparar dos escenarios: cuantos nodos, supuestos, riesgos tiene cada uno
SELECT s.name,
  COUNT(DISTINCT tn.node_id) as total_nodes,
  COUNT(DISTINCT a.id) as total_assumptions,
  COUNT(DISTINCT nb.id) as total_nbrs,
  SUM(CASE WHEN a.status = 'valid' THEN 1 ELSE 0 END) as unverified_assumptions
FROM scenarios s
JOIN scenario_trees st ON st.scenario_id = s.id
JOIN tree_nodes tn ON tn.tree_id = st.tree_id
LEFT JOIN edges e ON e.tree_id = st.tree_id
LEFT JOIN assumptions a ON a.edge_id = e.id
LEFT JOIN nbr_branches nb ON nb.tree_id = st.tree_id
GROUP BY s.id;
```

Resultado en la UI:

```
Scenario Comparison:
                        Escenario A (GPS)    Escenario B (Partner)
  Nodes:                32                   24
  Tactical actions:     12                   8
  Assumptions:          18 (4 unverified)    14 (7 unverified)
  NBR risks:            2 (1 unresolved)     3 (2 unresolved)
  Knowledge coverage:   72%                  45%

  Recommendation: Scenario A has more work but stronger evidence base.
                  Scenario B has more unverified assumptions and unresolved risks.
```

### 4.3 Trazabilidad con contexto de transicion

**Hoy**: `ltp trace --cross-tree` podria seguir nodos compartidos, pero no sabe POR QUE se paso de un arbol a otro.

**Con tree_relations**: La traza incluye la logica de cada transicion:

```
ltp trace --narrative --from UDE-003

  GT "Logistica de clase mundial"
    REQ-001 "Entregas en 24h con 99% on-time"
      ↓ gap_analysis: "REQ-001 no cumplido — genera UDE-003"
  CRT "Realidad actual"
    UDE-003 "Clientes cancelan pedidos por incertidumbre"
      ← RC-001 "No hay tracking en tiempo real"
        ↓ core_conflict: "RC-001 es la CRC — genera EC"
  EC "Velocidad vs Control"
    D: "Entregar rapido" ↔ D': "Mantener control de costes"
      ASM-012 invalidado por INJ-005
        ↓ injection: "INJ-005 se valida en FRT"
  FRT [Escenario A: GPS]
    DE-003 "Clientes conocen estado de su pedido"
      INJ-005 "Sistema de tracking en tiempo real"
        ↓ prerequisite: "INJ-005 requiere presupuesto e infraestructura"
  PRT [Escenario A: GPS]
    IO-002 "Conseguir presupuesto para GPS"
      ↓ tactical: "IO-002 se descompone en 4 acciones"
  TT [Escenario A: GPS]
    ACTION-003 "Implementar GPS tracking"
```

**Cada flecha tiene nombre y razon.** No es solo "estos nodos estan conectados" — es "esta transicion ocurrio porque se identifico la CRC".

### 4.4 Deteccion de handoffs rotos

**Escenario**: Se creo un CRT pero nadie construyo la EC a partir de su CRC. O se creo una EC pero la INJ no se valido en ningun FRT.

**Hoy**: Invisible. El motor no sabe que falta un handoff.

**Con tree_relations**:

```sql
-- CRTs sin EC derivada
SELECT t.id, t.name FROM trees t
WHERE t.tree_type = 'crt'
AND NOT EXISTS (
    SELECT 1 FROM tree_relations tr
    WHERE tr.from_tree = t.id AND tr.relation_type = 'core_conflict'
);

-- ECs cuya INJ no se ha validado en ningun FRT
SELECT t.id, t.name FROM trees t
WHERE t.tree_type = 'ec'
AND NOT EXISTS (
    SELECT 1 FROM tree_relations tr
    WHERE tr.from_tree = t.id AND tr.relation_type = 'injection'
);

-- FRTs sin NBR (riesgos no evaluados)
SELECT t.id, t.name FROM trees t
WHERE t.tree_type = 'frt'
AND NOT EXISTS (
    SELECT 1 FROM tree_relations tr
    WHERE tr.from_tree = t.id AND tr.relation_type = 'risk_check'
);
```

Nuevo warning en `ltp validate`:

```json
{
  "warnings": [
    {
      "code": "MISSING_HANDOFF",
      "detail": "CRT 'tree-crt-logistica' has CRC (RC-001) but no EC derives from it",
      "suggestion": "Create EC from CRC using: ltp tree_new --type ec --from-crc RC-001"
    },
    {
      "code": "UNVALIDATED_INJECTION",
      "detail": "EC 'tree-ec-conflicto' produced INJ-005 but no FRT validates it",
      "suggestion": "Create FRT to validate INJ-005"
    }
  ]
}
```

**El motor ahora sabe que falta en el proceso, no solo que falta en un arbol individual.**

### 4.5 Knowledge contextualizado por frame temporal

```sql
-- Knowledge que era valido en el PRESENTE pero que las inyecciones hacen obsoleto
SELECT k.id, k.label
FROM knowledge k
JOIN knowledge_links kl ON kl.knowledge_id = k.id
JOIN tree_nodes tn ON tn.node_id = kl.target_id
JOIN temporal_frames tf ON tf.tree_id = tn.tree_id
WHERE tf.frame = 'present' AND k.status = 'verified'
AND EXISTS (
    SELECT 1 FROM tree_nodes tn2
    JOIN temporal_frames tf2 ON tf2.tree_id = tn2.tree_id
    JOIN nodes n ON n.id = tn2.node_id AND n.status = 'invalidated'
    WHERE tf2.frame = 'future' AND tn2.node_id = kl.target_id
);
```

"Este knowledge era verdad HOY pero deja de serlo en el FUTURO si implementamos el escenario A."

### 4.6 Madurez del analisis por sistema

```sql
-- Que tan completo esta el Thinking Process para un sistema?
SELECT
  s.name,
  COUNT(DISTINCT CASE WHEN t.tree_type = 'gt' THEN t.id END) as gt_count,
  COUNT(DISTINCT CASE WHEN t.tree_type = 'crt' THEN t.id END) as crt_count,
  COUNT(DISTINCT CASE WHEN t.tree_type = 'ec' THEN t.id END) as ec_count,
  COUNT(DISTINCT CASE WHEN t.tree_type = 'frt' THEN t.id END) as frt_count,
  COUNT(DISTINCT CASE WHEN t.tree_type = 'prt' THEN t.id END) as prt_count,
  COUNT(DISTINCT CASE WHEN t.tree_type = 'tt' THEN t.id END) as tt_count,
  COUNT(DISTINCT tr.id) as handoffs,
  COUNT(DISTINCT sc.id) as scenarios
FROM systems s
LEFT JOIN trees t ON t.system_id = s.id
LEFT JOIN tree_relations tr ON tr.from_tree = t.id
LEFT JOIN scenarios sc ON sc.system_id = s.id
GROUP BY s.id;
```

Nuevo comando: `ltp system_status`:

```
System: "Departamento de Logistica — Empresa X"

Thinking Process Maturity:
  [✓] Goal Tree (1)
  [✓] Current Reality Tree (1)
  [✓] Evaporating Cloud (1)
  [✓] Future Reality Tree (2 — one per scenario)
  [~] NBR (1 of 2 scenarios covered)
  [✓] Prerequisite Tree (2)
  [✗] Transition Tree (0) ← NEXT STEP

Handoffs: 6 of 8 expected (2 missing)
Scenarios: 2 active, 0 discarded

Missing:
  - TT for both scenarios (PRT → TT handoff missing)
  - NBR for Scenario B (FRT → NBR handoff missing)
```

---

## 5. Comandos MCP Nuevos

### Para Sistema

| Comando | Descripcion |
|---------|-------------|
| `ltp system_new` | Crear un sistema |
| `ltp system_edit` | Editar nombre, boundary, etc. |
| `ltp system_inspect` | Ver sistema con sus arboles, escenarios y madurez |
| `ltp system_status` | Dashboard de madurez del Thinking Process |
| `ltp system_list` | Listar sistemas en el workspace |

### Para Relaciones entre Arboles

| Comando | Descripcion |
|---------|-------------|
| `ltp relation_add` | Crear relacion entre dos arboles |
| `ltp relation_rm` | Eliminar relacion |
| `ltp relation_list` | Listar relaciones (opcionalmente filtrar por arbol o tipo) |
| `ltp relation_inspect` | Ver relacion con sus handoff nodes |

### Para Escenarios

| Comando | Descripcion |
|---------|-------------|
| `ltp scenario_new` | Crear escenario (opcionalmente bifurcando de otro) |
| `ltp scenario_edit` | Editar nombre, descripcion, status |
| `ltp scenario_add_tree` | Asociar arbol a escenario |
| `ltp scenario_rm_tree` | Desasociar arbol |
| `ltp scenario_compare` | Comparar dos escenarios (nodos, riesgos, cobertura) |
| `ltp scenario_select` | Marcar escenario como `selected` |
| `ltp scenario_discard` | Marcar escenario como `discarded` |
| `ltp scenario_list` | Listar escenarios de un sistema |

### Extensiones a Comandos Existentes

| Comando existente | Extension |
|-------------------|-----------|
| `ltp tree_new` | `--system SYS-001 --scenario SCN-001 --frame future` |
| `ltp validate` | Incluye warnings de handoffs rotos (`MISSING_HANDOFF`) |
| `ltp trace` | `--narrative` para incluir contexto de tree_relations |
| `ltp status` | Muestra sistema activo y madurez del proceso |

---

## 6. Impacto en el Workspace

### ltp-engine (JSON)

Nuevos directorios:

```
mi-proyecto-ltp/
├── ltp.config.json
├── nodes/
├── trees/
├── knowledge/
├── systems/              ← NUEVO
│   └── SYS-001.json
├── relations/            ← NUEVO
│   ├── TREL-001.json
│   └── TREL-002.json
├── scenarios/            ← NUEVO
│   ├── SCN-001.json
│   └── SCN-002.json
└── .ltp/
```

### ltp-engine-max (libSQL)

Tablas nuevas en `workspace.db` (ver seccion 3 para DDL):
- `systems`
- `tree_relations`
- `scenarios`
- `scenario_trees`
- `temporal_frames`

Columna nueva en `trees`: `system_id`

---

## 7. Compatibilidad con ltp-engine Actual

El meta-grafo es **aditivo**: no cambia ninguna entidad existente. Todos los campos nuevos son opcionales o viven en entidades nuevas.

| Aspecto | Impacto |
|---------|---------|
| Nodos | Sin cambios |
| Trees | Campo opcional `system_id`, `temporal_frame`, `scenario_id` |
| Edges | Sin cambios |
| Assumptions | Sin cambios |
| Knowledge | Sin cambios |
| Undo/redo | Las nuevas entidades se incluyen en snapshots/journal normalmente |
| MCP tools existentes | Sin cambios — las 54 tools siguen funcionando |

Un workspace sin sistemas, relaciones ni escenarios funciona exactamente como hoy. La activacion es gradual: se empieza creando un sistema, luego se agregan relaciones, luego escenarios.

---

## 8. Analisis Six Thinking Hats

### Blanco (Datos)

- El LTP de Dettmer/Fedurko define explicitamente las 3 preguntas y el flujo entre arboles. Esta RFC modela lo que la metodologia ya prescribe pero el motor no captura.
- ltp-engine ya tiene `tree_attach` para compartir nodos — el meta-grafo agrega la capa de "por que se comparten".
- Un workspace tipico tiene 5-8 arboles. Las relaciones entre ellos son ~6-10 (un arbol por pregunta, ~2 handoffs por transicion).

### Rojo (Intuicion)

Esto se siente como lo que siempre debio estar ahi. Cuando construyes un LTP, la narrativa esta en tu cabeza — "cree el CRT porque vi gaps en el GT, y el CRC me llevo a la EC". El meta-grafo es tu modelo mental hecho explicito y persistente.

### Negro (Critico — Obligatorio)

| Riesgo | Severidad | Mitigacion |
|--------|-----------|------------|
| **Overhead de creacion** — el usuario tiene que crear relaciones manualmente | MEDIA | Auto-sugerencia: al hacer `tree_new --type ec`, el motor sugiere "derivar de CRT X?" |
| **Rigidez** — no todos los analisis siguen el flujo canonico GT→CRT→EC→FRT→PRT→TT | MEDIA | Las relaciones son opcionales y los tipos incluyen `extends` y `revision` para flujos no lineales |
| **Complejidad del modelo** — 4 entidades nuevas para workspaces que hoy tienen 0 | BAJA | Todas opcionales. Activacion gradual. |
| **Escenarios complejos** — bifurcaciones anidadas, escenarios que se fusionan | BAJA | `parent_scenario` permite anidamiento. Fusion no se modela (crear nuevo escenario que tome lo mejor de ambos). |

### Amarillo (Beneficios)

- Captura la narrativa completa del Thinking Process como entidad navegable.
- La UI puede renderizar el flujo entre arboles, no solo los arboles individuales.
- `ltp validate` detecta handoffs rotos — sabes que falta en el proceso.
- Escenarios permiten explorar alternativas sin mezclar arboles de futuros distintos.
- Cross-workspace: comparar como se abordo el mismo tipo de sistema en distintos analisis.

### Verde (Alternativas)

- Alternativa: modelar las relaciones como un tipo especial de edge (meta-edge) entre arboles, no como entidad separada. Mas simple pero mezcla niveles de abstraccion.
- Alternativa: los escenarios podrian ser branches de git. Pero eso mezcla versionado con modelado y no permite comparar escenarios en la UI.
- Alternativa: temporal frames podrian inferirse del tree_type (CRT → present, FRT → future). Pero hay casos donde un CRT es historico (past) o un FRT es del presente (validacion).

### Azul (Coherencia Arquitectonica)

- Mantiene el principio de entidades de primer nivel (ADR-005): sistemas, relaciones y escenarios son direccionables, no embebidos.
- Mantiene IDs secuenciales: `SYS-001`, `TREL-001`, `SCN-001`.
- Funciona en ambas arquitecturas de persistencia (JSON y libSQL).
- No modifica ningun invariante existente del motor.

---

## 9. Preguntas Abiertas

1. **Auto-creacion de relaciones**: cuando `tree_new --type ec` se crea despues de un CRT, el motor deberia sugerir o crear automaticamente la relacion `core_conflict`?
2. **Escenarios como workspaces**: deberian los escenarios ser sub-workspaces independientes (con su propio .db) o entidades dentro del mismo workspace?
3. **Relaciones bidireccionales**: `revision` (FRT → CRT) implica que el CRT se actualize. Debe el motor forzar re-validacion del CRT cuando se agrega una relacion `revision`?
4. **Herencia de knowledge**: si un nodo compartido tiene knowledge en el frame `present`, esa knowledge es valida en el frame `future`? O necesita re-verificarse?
5. **Madurez como metrica**: el `system_status` que muestra madurez del proceso — deberia ser configurable (que fases son obligatorias vs opcionales)?
6. **Nomenclatura de IDs**: `SYS-001`, `TREL-001`, `SCN-001` vs otro patron? Consistente con el esquema actual.
7. **Visualizacion del meta-grafo**: la UI deberia mostrar el meta-grafo como un diagrama de alto nivel (cajas = arboles, flechas = relaciones) ademas de la vista narrativa?
8. **Profundidad de validacion cross-logic**: los warnings de transicion logica (`LOGIC_TRANSITION_UNVALIDATED`, `ASSUMPTION_CROSS_LOGIC`, `OPERATOR_CONTEXT_LOST`) — deberian ser todos opcionales, todos por defecto, o configurables por severidad?
9. **Handoff nodes enriquecidos**: el formato dual (string simple vs objeto con `source_operator`) — deberian los handoff_nodes ser siempre objetos para uniformidad, o mantener el formato dual para backward-compatibility?
10. **EC como proyeccion comprimida**: resuelto en seccion 11. La EC comprime cadenas multi-nivel del GT y CRT en 5 nodos y 5 edges. La proveniencia de esa compresion se modela con `projection_map` en la tree_relation (nodos) y `provenance` en los edges de la EC (caminos comprimidos). Ver seccion 11 para analisis Six Hats completo.
11. **Granularidad de provenance**: el `collapsed_path` dentro de `provenance` — debe incluir todos los nodos intermedios de la cadena fuente, o basta con extremos (source, target)?
12. **Provenance mutable**: si el GT se reestructura (nodo intermedio eliminado), el `collapsed_path` en el edge EC se desincroniza. Debe el motor detectar esto y emitir warning, o la provenance se considera snapshot historico?
13. **Proyeccion automatica vs manual**: al construir una EC, el motor podria inferir el `projection_map` si los nodos ya existen en GT/CRT via `tree_attach`. Debe inferirlo y pedir confirmacion, o esperar creacion manual?
14. **Granularidad de actor_role**: el enum de roles (`causa`, `afectado`, `ejecutor`, `bloqueante`) — es suficiente, o necesitamos un vocabulario mas amplio? Deberia ser un enum cerrado o texto libre?
15. **Actores y stakeholders**: `systems.stakeholders` (String[]) y la nueva tabla `actors` cubren terreno similar. Deberian coexistir (texto libre + estructurado), o `actors` reemplaza a `stakeholders`?
16. **GT satelite vs supuesto**: cuando una INJ depende de un actor externo, el analista puede (a) crear un GT satelite parcial o (b) capturar un supuesto con `actor_dependency`. Cuando es cada uno apropiado? Deberia el motor guiar esta decision?
17. **Atribucion multi-actor**: un nodo como "Retrasos en entregas" afecta a proveedor, cliente, y operaciones. Deberia `actor_id` ser un campo singular o un array? Un campo singular fuerza precision; un array captura realidad pero diluye utilidad.
18. **Transformation type en handoff_nodes**: deberia el meta-grafo anotar la naturaleza de la transformacion (derivation, compression, recontextualization, inversion, promotion, decomposition) en los handoff_nodes? Esto permitiria a `trace --narrative` explicar como se transforma el nodo al cruzar, no solo que cruzo.
19. **Validacion norma-bisagra**: deberia `validate` verificar que cada UDE del CRT se puede trazar a al menos una NC/CSF violada del GT? Y que cada DE del FRT evidencia una NC/CSF cumplida? Esto es la norma-bisagra como invariante validable.
20. **Espacio de fuentes de la EC en el motor**: deberia el motor conocer las combinaciones validas de tipos fuente para A, B, C, D, D' y validar/sugerir cuando el projection_map las viola? O es demasiada rigidez para un marco que necesita flexibilidad?

---

## 10. Analisis Profundo: Logica de Suficiencia vs Necesidad en el Meta-Grafo

### Contexto

Cada arbol LTP opera bajo una de dos logicas fundamentales. Esto no es una anotacion decorativa — define como se lee, se recorre, se valida y se conecta cada arbol. El meta-grafo debe modelar estas diferencias porque **cada handoff canonico entre arboles cruza una frontera logica**.

### 10.1. Las dos logicas: hechos (Sombrero Blanco)

#### Tabla de referencia

| Dimension | Suficiencia (CRT, FRT, TT) | Necesidad (GT, EC, PRT) |
|-----------|---------------------------|------------------------|
| Formato logico | "Si [Causa], entonces [Efecto]" | "Para lograr [Objetivo], debo tener [Condicion]" |
| Direccion causal | Bottom-up: causas → efectos | Top-down: objetivo → requisitos/prerequisitos |
| tree_walk default | `topological` (desde raices hacia hojas) | `reverse` (desde objetivo hacia prerequisitos) |
| Operadores | SINGLE, AND, OR, MAG, XOR | Solo SINGLE (conexiones de necesidad pura) |
| CLR #3 (Causalidad) | "¿Es A suficiente para producir B?" | "¿Es B necesario para lograr A?" |
| CLR #4 (Insuficiencia) | **Aplica**: AND gates, silogismo aristotelico | **No aplica**: no hay AND gates en necesidad |
| CLR #5 (Causa adicional) | **Aplica**: OR/MAG distinction, weights | **No aplica**: no hay OR/MAG en necesidad |
| CLR #6 (Inversion) | Detecta UDE→RC (alto→bajo) | Detecta prerequisito en posicion de objetivo |
| CLR #7 (Efecto predicho) | Causa intangible requiere 2+ edges salientes | Condicion intangible requiere corroboracion |
| CLR #8 (Tautologia) | DFS 3 colores para ciclos dirigidos | Aplica pero la estructura es inherentemente mas lineal |
| Feedback edges | **Disponibles** (positive/negative loops) | **No disponibles** |
| NBR branches | **Disponibles** (en FRT) | **No disponibles** |
| DAG validation | **Bloqueante**: ciclos = error | No aplica de la misma forma (estructura de arbol) |

#### Mapa de transiciones logicas en el flujo canonico

```
GT ──────► CRT ──────► EC ──────► FRT ──────► PRT ──────► TT
(nec)      (suf)       (nec)      (suf)       (nec)       (suf)
   nec→suf    suf→nec    nec→suf    suf→nec     nec→suf

                                   FRT ──────► NBR
                                   (suf)       (suf)
                                      same_suf
```

**Hallazgo critico**: los 5 handoffs principales del LTP canonico alternan sistematicamente entre logicas. Solo FRT→NBR permanece en la misma logica. Esto no es coincidencia — Dettmer diseño el proceso para forzar cambios de perspectiva en cada transicion.

#### Handoffs no canonicos (retroalimentacion)

| Handoff | Logica | Transicion | Trigger |
|---------|--------|------------|---------|
| FRT → CRT (revision) | suf → suf | same_suf | FRT revela UDE no detectada en CRT |
| NBR → EC (challenges) | suf → nec | suf→nec | Riesgo de NBR obliga nueva inyeccion |
| TT → PRT (feedback) | suf → nec | suf→nec | Accion del TT revela prerequisito faltante |
| CRT → GT (feedback) | suf → nec | suf→nec | CRT revela gap no capturado en GT |

### 10.2. Intuicion (Sombrero Rojo)

Las transiciones logicas son los **puntos de mayor fragilidad epistemica** del analisis. Cuando pasas de un CRT (suficiencia) a una EC (necesidad), no solo cambias de arbol — cambias de forma de pensar:

- En el CRT pensabas "esto causa aquello" (deductivo).
- En la EC piensas "para resolver esto, necesito aquello" (teleologico).

Ese cambio de mentalidad es exactamente donde los errores se introducen: el analista arrastra la logica anterior al nuevo arbol. Una CRC (causa raiz del CRT, logica "si-entonces") se convierte en un conflicto (logica "para-necesito"), y el supuesto que la une puede no sobrevivir la traduccion.

Los handoffs se sienten como **puertas entre habitaciones con reglas diferentes**. Si el meta-grafo no marca esas puertas, el usuario (y el agente) caminan de una habitacion a otra sin saber que las reglas cambiaron.

### 10.3. Riesgos (Sombrero Negro — Obligatorio)

| # | Riesgo | Severidad | Detalle | Mitigacion |
|---|--------|-----------|---------|------------|
| B1 | **Validacion cruzada incoherente** — aplicar CLR #4/#5 (AND/OR/MAG) a un arbol de necesidad que no los soporta | ALTA | Si `ltp validate` no distingue logica al evaluar cross-tree, podria advertir "SINGLE edge sin AND" en un PRT donde SINGLE es lo unico valido | `validate` ya distingue por `tree.logic`; el riesgo es que el meta-grafo introduzca validaciones cross-tree que ignoren la frontera |
| B2 | **Direccion de lectura invertida en traza** — `trace --narrative` cruza de FRT (bottom-up) a PRT (top-down) sin cambiar la direccion de recorrido | ALTA | Una traza que sigue el flujo causal bottom-up en FRT y luego entra al PRT deberia invertir a top-down. Si no lo hace, la narrativa es incoherente | Anotar `logic_transition` en tree_relations; `trace` cambia de estrategia al cruzar |
| B3 | **Supuestos que no sobreviven la traduccion logica** — un ASM valido en logica de suficiencia ("si A entonces B asumiendo X") pierde sentido en logica de necesidad ("para B necesito A asumiendo X") | MEDIA | El assumption puede ser verdadero en un contexto logico y falso en otro. Los handoff_nodes comparten assumptions pero la logica que los rodea cambia | Los assumptions en handoff_nodes deberian marcarse `needs_review` al cruzar frontera logica |
| B4 | **Operadores fantasma en handoffs** — un nodo viene de un edge AND en el CRT (suficiencia) y se inserta en un PRT (necesidad) donde AND no existe | MEDIA | El operador del edge original no tiene traduccion en el arbol destino. Si el meta-grafo no lo advierte, se pierde informacion causal | Warning `OPERATOR_CONTEXT_LOST` cuando handoff_node proviene de edge agrupado |
| B5 | **Feedback edges sin frontera** — alguien crea un feedback edge cross-tree entre un FRT y un PRT, pero los feedback edges solo estan definidos para arboles de suficiencia | BAJA | Actualmente el motor no soporta feedback cross-tree, pero el meta-grafo podria sugerir relaciones tipo `revision` que impliquen retroalimentacion hacia arboles de necesidad | Las tree_relations de tipo `revision` o `challenges` son el mecanismo correcto — no feedback edges |
| B6 | **Escenarios con logica mixta** — un escenario agrupa arboles de ambas logicas (FRT+PRT+TT) sin explicitar donde cambia la logica | MEDIA | La comparacion de escenarios (`scenario_compare`) podria mezclar metricas de suficiencia (operadores, weights, feedback loops) con metricas de necesidad (prerequisitos, cobertura de IOs) | Separar metricas por logica en el output de comparacion |
| B7 | **Madurez engañosa** — `system_status` muestra "FRT completo" pero no detecta que la transicion FRT→PRT pierde informacion logica | BAJA | La madurez por tipos de arbol no refleja la calidad de las transiciones entre logicas | Agregar metrica de "handoffs validados" que incluya verificacion de coherencia logica |

### 10.4. Beneficios (Sombrero Amarillo)

1. **Pedagogia explicita**: al modelar `logic_transition` en las tree_relations, tanto el usuario como el agente saben que estan cruzando una frontera logica. Esto es lo que Dettmer enseña en sus cursos — el LTP fuerza cambios de perspectiva — pero hoy eso depende de que el analista lo sepa.

2. **Validacion cross-tree consciente**: el motor puede emitir warnings especificos para transiciones logicas:
   - `LOGIC_TRANSITION_UNVALIDATED`: relacion entre arboles de diferente logica sin que los handoff_nodes hayan sido revisados.
   - `ASSUMPTION_CROSS_LOGIC`: supuesto compartido entre contextos de suficiencia y necesidad que no ha sido re-evaluado.
   - `OPERATOR_CONTEXT_LOST`: nodo proveniente de edge agrupado (AND/OR/MAG) insertado en arbol de necesidad.

3. **Traza narrativa inteligente**: `trace --narrative` anota los cambios de logica:
   ```
   [SUFFICIENCY → NECESSITY] Cruzando de CRT a EC
   Lectura cambia de "Si A entonces B" a "Para B necesito A"
   Direccion: bottom-up → top-down
   ```

4. **Proof standard explicito**: cada tree_relation sabe que tipo de prueba aplica. En la transicion CRT→EC, la prueba de causalidad (CLR#3) cambia de "¿es suficiente?" a "¿es necesario?". Esto permite al agente LLM ajustar su evaluacion.

5. **Completitud del silogismo en fronteras**: los AND gates del CRT que alimentan la EC se "descomponen" en necesidades individuales. El meta-grafo puede advertir cuando un silogismo de suficiencia se traduce a requisitos de necesidad sin verificar que todas las premisas se preservan.

6. **Metricas de escenario ajustadas por logica**: al comparar escenarios, las metricas se segmentan — p.ej. un FRT con muchos AND gates (buena suficiencia) vs un PRT con todos los IOs cubiertos (buena necesidad). No se mezclan manzanas con peras.

### 10.5. Alternativas de diseno (Sombrero Verde)

#### Alternativa A: Campo `logic_transition` en TreeRelation (propuesta principal)

Agregar a `tree_relations`:

```json
{
  "id": "TREL-001",
  "from_tree": "tree-crt-logistica",
  "to_tree": "tree-ec-conflicto",
  "relation_type": "core_conflict",
  "logic_transition": "suf_to_nec",
  "proof_standard": {
    "from": "sufficiency",
    "to": "necessity",
    "clr_changes": ["CLR3_test_changes", "CLR4_not_applicable", "CLR5_not_applicable"]
  },
  "handoff_nodes": ["RC-001"],
  "description": "CRC identificada. Lectura cambia de 'Si [causa] entonces [efecto]' a 'Para [resolver] necesito [condicion]'."
}
```

**SQL**:
```sql
ALTER TABLE tree_relations ADD COLUMN logic_transition TEXT;
-- Enum: 'suf_to_nec' | 'nec_to_suf' | 'same_suf' | 'same_nec'

ALTER TABLE tree_relations ADD COLUMN proof_standard TEXT;
-- JSON con detalle de cambios CLR
```

**Ventaja**: explicito, auditable, consultable.
**Desventaja**: campo redundante (derivable de `from_tree.logic` y `to_tree.logic`).

#### Alternativa B: Derivar la transicion del tipo de arbol (zero storage)

No almacenar `logic_transition` — calcularlo al vuelo:

```rust
fn logic_transition(from: &Tree, to: &Tree) -> LogicTransition {
    match (from.logic, to.logic) {
        (Logic::Sufficiency, Logic::Necessity) => LogicTransition::SufToNec,
        (Logic::Necessity, Logic::Sufficiency) => LogicTransition::NecToSuf,
        (Logic::Sufficiency, Logic::Sufficiency) => LogicTransition::SameSuf,
        (Logic::Necessity, Logic::Necessity) => LogicTransition::SameNec,
    }
}
```

**Ventaja**: zero storage, imposible de desincronizar.
**Desventaja**: el `proof_standard` detallado (que CLRs cambian) requiere logica adicional.

#### Alternativa C: Hibrida — derivar transicion, almacenar solo proof_standard custom

La transicion se calcula; el `proof_standard` solo se almacena si el usuario/agente lo personaliza (p.ej. "en esta transicion concreta, CLR#3 no aplica porque la EC ya fue validada externamente").

**Ventaja**: combina lo mejor de A y B.
**Desventaja**: ligeramente mas complejo de implementar.

#### Alternativa D: Handoff Nodes con "pasaporte logico"

En vez de anotar la relacion, anotar cada handoff_node con metadata de donde viene y adonde va:

```json
{
  "handoff_nodes": [
    {
      "node_id": "RC-001",
      "source_context": {
        "tree": "tree-crt-logistica",
        "logic": "sufficiency",
        "role": "root_cause",
        "edge_operator": "AND"
      },
      "target_context": {
        "tree": "tree-ec-conflicto",
        "logic": "necessity",
        "role": "requirement",
        "edge_operator": null
      }
    }
  ]
}
```

**Ventaja**: granularidad maxima — se sabe exactamente que pierde/gana cada nodo en la transicion.
**Desventaja**: verboso, duplica informacion ya presente en los arboles.

#### Recomendacion

**Alternativa C** (hibrida) como base con **elementos de D** (pasaporte) cuando hay operadores que se pierden. Concretamente:

1. `logic_transition` se **calcula** (no se almacena) — derivado de `from_tree.logic` y `to_tree.logic`.
2. `proof_standard` se almacena **opcionalmente** en la tree_relation — solo si hay personalizacion.
3. `handoff_nodes` se enriquece con `source_operator` cuando el nodo provenia de un edge agrupado — esto es la unica informacion que se pierde genuinamente en la transicion.

### 10.6. Coherencia arquitectonica (Sombrero Azul)

#### Alineacion con invariantes existentes

| Principio | Cumplimiento |
|-----------|-------------|
| Motor determinista, sin semantica bloqueante | Si: `logic_transition` se calcula deterministicamente; los warnings de cross-logic son no-bloqueantes |
| IDs secuenciales | Si: no requiere IDs nuevos — la logica vive en tree_relations existentes |
| JSON canonico (BTreeMap) | Si: `proof_standard` se serializa con claves ordenadas |
| Entidades de primer nivel (ADR-005) | Si: no se crean entidades nuevas, se enriquecen las existentes |
| Compatibilidad aditiva | Si: campos opcionales, backwards-compatible con workspaces sin meta-grafo |

#### Impacto en comandos existentes

| Comando | Cambio |
|---------|--------|
| `validate` | Nuevos warnings: `LOGIC_TRANSITION_UNVALIDATED`, `ASSUMPTION_CROSS_LOGIC`, `OPERATOR_CONTEXT_LOST` |
| `trace --narrative` | Anota cambios de logica y direccion de lectura al cruzar tree_relations |
| `tree_walk` | Sin cambios — ya respeta `tree.logic` para elegir orden por defecto |
| `link connect` | Sin cambios — operadores ya se validan por arbol |
| `scenario_compare` | Segmenta metricas por logica: suficiencia (operadores, weights, feedback) vs necesidad (IOs, prerequisitos) |
| `system_status` | Agrega linea "Logic transitions: N validated / M total" |
| `relation_add` | Calcula y muestra `logic_transition` en el output; warning si handoff_nodes provienen de edges agrupados |

#### Impacto en el esquema de datos

**Cambio minimo en tree_relations**:

```sql
-- ltp-engine-max
ALTER TABLE tree_relations ADD COLUMN proof_standard TEXT;
-- JSON, nullable. Solo se persiste si hay personalizacion.

-- handoff_nodes se enriquece para incluir source_operator cuando aplique.
-- El campo handoff_nodes pasa de ser JSON array de strings a JSON array de objects:
-- Antes:  ["RC-001"]
-- Ahora:  [{"node_id": "RC-001", "source_operator": "AND"}]
--         o ["RC-001"] si no hay contexto de operador (backward-compatible)
```

**ltp-engine (JSON)**: el tree_relation JSON acepta ambos formatos de handoff_nodes.

#### Nuevos warnings para `ltp validate`

```json
{
  "warnings": [
    {
      "code": "LOGIC_TRANSITION_UNVALIDATED",
      "detail": "Relation TREL-001 crosses suf→nec boundary (CRT→EC). Handoff node RC-001 assumptions not reviewed in necessity context.",
      "suggestion": "Review assumptions on RC-001 in the context of EC necessity logic."
    },
    {
      "code": "ASSUMPTION_CROSS_LOGIC",
      "detail": "ASM-003 on RC-001 was validated in sufficiency context (CRT) but RC-001 participates in EC (necessity). Assumption text may not hold under 'To achieve X, I must have Y' framing.",
      "suggestion": "Re-evaluate ASM-003 under necessity logic."
    },
    {
      "code": "OPERATOR_CONTEXT_LOST",
      "detail": "RC-001 was part of AND group (LINK-005) in CRT but enters EC as individual node. AND dependency on INT-002 is not captured in EC.",
      "suggestion": "Verify that the necessity relationship in EC accounts for all co-dependent causes from the CRT."
    }
  ]
}
```

### 10.7. Resumen ejecutivo del analisis

La logica de suficiencia vs necesidad no es una etiqueta pasiva — es una **propiedad fundamental** que cambia como se lee, se valida y se recorre cada arbol. El flujo canonico del LTP alterna sistematicamente entre ambas logicas en cada handoff principal, lo que hace que las transiciones logicas sean el punto mas fragil del analisis.

**Decision de diseno**: no crear nuevas entidades. Enriquecer `tree_relations` con:
1. `logic_transition` calculado (no almacenado) desde `from_tree.logic` / `to_tree.logic`.
2. `proof_standard` opcional (solo si hay personalizacion).
3. `handoff_nodes` que acepta objetos enriquecidos con `source_operator` cuando el nodo provenia de un edge agrupado.

**Impacto**: 3 nuevos warnings en validate, anotaciones de logica en trace, metricas segmentadas en scenario_compare. Zero breaking changes.

---

## 11. La EC como Proyeccion Comprimida: Proveniencia y Arquitectura

### Contexto

La Evaporating Cloud no es un arbol downstream mas — es una **proyeccion comprimida** de cadenas causales que existen en el Goal Tree y en el Current Reality Tree. Los 5 nodos de la EC no nacen ex nihilo: son nodos (o resumen de nodos) que ya viven en otros arboles. Los 5 edges de la EC no modelan relaciones causales simples: cada uno **colapsa uno o muchos niveles** de edges y nodos de los arboles fuente. Los supuestos detras de cada flecha de la EC son poderosos precisamente porque cuestionan toda la cadena comprimida, y al invalidar uno, se "evapora" la logica colapsada.

Esta seccion analiza como modelar esa compresion y donde debe vivir la proveniencia — en el meta-grafo (tree_relations), en los edges de la EC, o en ambos.

### 11.1. Hechos: mapeo de la EC a sus fuentes (Sombrero Blanco)

#### Mapeo de los 5 nodos EC a sus fuentes tipicas

| Nodo EC | Role | Tipos fuente posibles | Arbol fuente | Ejemplo |
|---------|------|-----------------------|-------------|---------|
| **A** (Objetivo comun) | `objective` | GOAL, OBJ, o CSF del GT | GT (necesidad) | GOAL-001 "Logistica de clase mundial" o CSF-002 "Entregas on-time" |
| **B** (Necesidad 1) | `requirement` | CSF o NC del GT | GT (necesidad) | CSF-002 "Entregas on-time" o NC-003 "Control de costes operativos" |
| **C** (Necesidad 2) | `requirement` | CSF o NC del GT | GT (necesidad) | NC-007 "Visibilidad en tiempo real del pedido" o CSF-004 "Satisfaccion del cliente" |
| **D** (Prerequisito 1) | `prerequisite` | RC o INT del CRT, NC del GT, o nodo nuevo | CRT (suficiencia), GT, o nuevo | RC-001 "Usar entregas manuales sin tracking" |
| **D'** (Prerequisito 2) | `prerequisite` | RC o INT del CRT, NC del GT, o nodo nuevo | CRT (suficiencia), GT, o nuevo | PRE-002 "Invertir en sistema de tracking GPS" |

**Matiz critico — la norma como bisagra**: los CSF/NC del GT son la **norma** (lo que deberia ser). Un CSF/NC no se convierte directamente en un UDE ni en un DE:
- Si la norma **se cumple** → se evidencia como **DE** en el FRT (efecto deseable observable).
- Si la norma **no se cumple** → de ahi se **deriva** un **UDE** en el CRT (manifestacion observable de la violacion).

El UDE no ES la NC — es su manifestacion negativa. El DE no ES la NC — es su manifestacion positiva. Son caras de la misma moneda: NC-003 "Entregas on-time ≥95%" genera UDE-003 "El 28% de entregas llegan fuera de plazo" (CRT, violada) o DE-003 "Entregas on-time al 97%" (FRT, cumplida).

**Observacion critica**: A, B, C vienen de arboles de **necesidad** (GT). B y C pueden ser **cualquier combinacion** de CSF y NC (B=CSF + C=NC, B=CSF + C=CSF, B=NC + C=NC). D y D' vienen tipicamente de arboles de **suficiencia** (CRT) o son nuevos. La EC, siendo de necesidad, recontextualiza nodos de suficiencia — un nodo que era "causa de UDEs" en el CRT se convierte en "prerequisito en conflicto" en la EC.

#### Mapeo de los 5 edges EC a lo que comprimen

| Edge EC | Lectura | Que comprime | Depth tipica | Ejemplo |
|---------|---------|-------------|-------------|---------|
| **B→A** | "Para lograr A, necesito B" | Cadena GT: NC/CSF → ... → CSF → GOAL | 1-3 niveles | NC-003 → OBJ-001 → GOAL-001 (2 niveles comprimidos) |
| **C→A** | "Para lograr A, necesito C" | Cadena GT: NC/CSF → ... → CSF → GOAL | 1-3 niveles | NC-007 → CSF-002 → GOAL-001 (2 niveles comprimidos) |
| **D→B** | "Para satisfacer B, debo hacer D" | Cadena cross-tree: RC/INT (CRT) → ... → UDE derivado de NC/CSF violada (GT) | 2-5 niveles | RC-001 → INT-003 → UDE-002 (CRT) donde UDE-002 deriva de NC-003 violada (GT) |
| **D'→C** | "Para satisfacer C, debo hacer D'" | Cadena cross-tree: similar a D→B, conectando con la otra necesidad | 2-5 niveles | La accion/politica opuesta conecta con C via cadena CRT+GT |
| **D↔D'** | "D y D' estan en conflicto porque..." | Exclusion fisica, de recurso o logica | 0 (directo) | No comprime — es una relacion emergente nueva |

**Hallazgos**:
1. Los edges verticales (B→A, C→A) comprimen cadenas **dentro del GT**.
2. Los edges diagonales (D→B, D'→C) comprimen cadenas **cross-tree** (CRT → GT) que incluyen el paso critico "NC/CSF violada → UDE derivado → cadena causal → RC". Son los mas profundos y donde la compresion oculta mas logica.
3. El edge de conflicto (D↔D') es el unico que **NO comprime** — es una relacion genuinamente nueva.
4. La bisagra GT↔CRT esta en el concepto de **norma violada**: el edge diagonal no conecta directamente RC con NC, sino que pasa por la violacion que genera el UDE.

#### Profundidad de compresion

En un analisis LTP tipico:

| Metrica | Valor tipico |
|---------|-------------|
| Niveles del GT (GOAL → CSF → NC) | 3 niveles |
| Niveles del CRT (UDE → INT → RC) | 3-8 niveles |
| Edges EC B→A, C→A | Comprimen 1-3 niveles del GT |
| Edges EC D→B, D'→C | Comprimen 2-5 niveles cross-tree (CRT+GT) |
| Total de nodos "ocultos" en una EC tipica | 8-20 nodos no visibles en la EC |

#### Relacion entre supuestos y cadena comprimida

Los supuestos de la EC (generados segun el protocolo de Fedurko con Reglas 1-4 + 7 Errores Fatales) cuestionan la logica comprimida:

- Un supuesto en el edge **B→A** cuestiona si la cadena GT NC/CSF→...→GOAL es realmente necesaria.
- Un supuesto en el edge **D→B** cuestiona si la cadena cross-tree RC→...→UDE (derivado de NC/CSF violada) es la unica via. Aqui el supuesto puede atacar tres puntos: (a) la cadena causal CRT (¿es RC realmente la causa?), (b) la violacion de la norma (¿realmente NC-003 se viola?), o (c) la necesidad misma (¿NC-003 es realmente necesaria para el CSF?).
- Al invalidar un supuesto en D→B, no se invalida solo la flecha EC — se invalida **toda la cadena comprimida** que esa flecha representa, incluyendo la relacion "norma violada → UDE derivado".

Esto es exactamente el poder de la EC: comprimir para hacer visible lo cuestionable. Pero hoy esa compresion es **invisible para el motor**.

#### Como funciona hoy (sin meta-grafo)

1. El usuario construye la EC con `tree_new --type ec` + `node_add` × 5.
2. Los nodos A, B, C se crean como nodos nuevos O se reutilizan via `tree_attach` si ya existen en el GT.
3. Los nodos D, D' se crean nuevos o reutilizan del CRT.
4. No queda registro de que A era el GOAL-001 del GT, o que D era RC-001 del CRT.
5. No queda registro de que el edge B→A comprime 2 niveles del GT.
6. Los supuestos se generan "a ciegas" — sin saber que cadena cuestionan.

Si los nodos se reutilizan via `tree_attach`, al menos hay un link implicito. Pero:
- No se sabe QUE cadena comprime cada edge.
- No se sabe DE DONDE viene cada nodo (de que role en que arbol).
- El agente LLM no tiene contexto para generar supuestos mas ricos.

### 11.2. Intuicion (Sombrero Rojo)

La EC es la **lente mas potente** del LTP — precisamente porque comprime. Es el momento donde el analista dice "toda esa realidad compleja se reduce a ESTE conflicto". Hacer visible esa compresion tiene un poder pedagogico enorme:

- El usuario ve: "este edge B→A que parece simple en realidad oculta 3 niveles del Goal Tree".
- El agente LLM genera supuestos sabiendo que cadena cuestionar: "dado que B→A comprime NC-003 → OBJ-001 → GOAL-001, un supuesto valido seria cuestionar si OBJ-001 realmente requiere NC-003".
- La UI puede mostrar un "zoom" al hacer clic en un edge EC: se despliega la cadena comprimida del GT o CRT.

La EC sin proveniencia es un cristal opaco — ves el conflicto pero no lo que hay detras. Con proveniencia, el cristal se vuelve transparente.

Pero hay un riesgo emocional: si la proveniencia es demasiado compleja de crear o mantener, el usuario la ignora y pierde la magia. **La barrera de entrada debe ser casi cero.**

### 11.3. Riesgos (Sombrero Negro — Obligatorio)

| # | Riesgo | Severidad | Detalle | Mitigacion |
|---|--------|-----------|---------|------------|
| B1 | **Duplicacion de informacion** — provenance repite datos que ya estan en GT/CRT | ALTA | Si el edge B→A tiene `collapsed_path: [NC-003, OBJ-001, GOAL-001]`, esos nodos y sus conexiones ya existen en el GT. Estamos duplicando la estructura del arbol fuente dentro de la EC. | La provenance es un **puntero** (IDs de nodos), no una copia de sus datos. La duplicacion es de referencia, no de contenido. Aun asi, son datos que el motor puede derivar. |
| B2 | **Desincronizacion con arboles fuente** — si el GT se reestructura, la provenance queda obsoleta | ALTA | El usuario elimina OBJ-001 del GT o lo fusiona con otro. El `collapsed_path` del edge EC sigue referenciando OBJ-001. No hay mecanismo de cascade ni de warning. | Dos opciones: (a) soft references — la provenance se valida al hacer `validate` y emite warning si un nodo del path ya no existe; (b) snapshot semantico — la provenance se trata como "asi era cuando se construyo la EC", no como referencia viva. |
| B3 | **Complejidad de mantenimiento** — el usuario/agente tiene que crear y mantener projection_map + provenance | MEDIA-ALTA | Un agente LLM puede automatizar esto. Pero un usuario humano construyendo una EC manualmente tiene que especificar de donde vienen 5 nodos y que comprimen 5 edges. Son 10 campos adicionales en una entidad que hoy tiene 5 nodos + 5 edges + supuestos. | Creacion automatica: el motor infiere la provenance cuando los nodos EC ya existen en otros arboles (via `tree_attach`). El `projection_map` se pre-rellena; el usuario solo confirma o ajusta. |
| B4 | **Overengineering** — no todo analisis necesita proveniencia explicita | MEDIA | Un analista experto que construye una EC rapida para un dilema simple no necesita que el motor le diga "B→A comprime 2 niveles del GT". La provenance agrega ceremonia sin valor para ECs simples o ECs tipo "dilema" que no derivan de GT/CRT. | Hacer la provenance **completamente opcional**. Default: sin provenance. El agente la crea proactivamente; el humano solo si quiere. `validate` sugiere pero no requiere. |
| B5 | **Impacto en schema** — dos campos nuevos (projection_map en tree_relation, provenance en edge) | MEDIA | En ltp-engine (JSON): un campo extra en el tree_relation JSON y un campo extra en cada edge JSON del EC. En ltp-engine-max (SQL): una columna nullable en `tree_relations` y una columna nullable en `edges`. | Ambos campos son nullable/opcionales. Zero breaking changes. Pero agregan complejidad al modelo mental del schema. |
| B6 | **Profundidad variable** — no siempre esta claro que es "la cadena comprimida" | MEDIA | El edge D→B comprime "RC-001 del CRT + cadena hasta UDE + violacion de NC del GT". Donde empieza y termina la cadena? Si el CRT tiene 8 niveles, el `collapsed_path` incluye todos? Solo los relevantes? Quien decide? | El `collapsed_path` incluye el camino minimo entre el nodo fuente EC y el punto donde conecta con el otro arbol. El agente propone, el usuario puede ajustar. Si es ambiguo, el motor registra solo los extremos. |
| B7 | **Falsa precision** — la provenance sugiere que la compresion es exacta cuando en realidad es una simplificacion | BAJA | La EC deliberadamente simplifica. Afirmar que B→A "comprime [NC-003, OBJ-001, GOAL-001]" implica que esos son los unicos 3 nodos. Pero quiza hay NCs laterales que tambien contribuyen. | Documentar que la provenance es "el camino principal comprimido", no una representacion exhaustiva. El agente puede anotar `"partial": true` si sabe que hay ramas no capturadas. |
| B8 | **Circularidad con supuestos** — los supuestos cuestionan la cadena comprimida, pero la provenance describe esa cadena. Si el supuesto invalida la cadena, la provenance se vuelve irrelevante | BAJA | Esto no es un bug — es el comportamiento esperado. Al invalidar un supuesto, la INJ resultante rompe la cadena. La provenance queda como registro historico de "lo que se cuestiono". | No requiere mitigacion. La provenance es input para generar supuestos, no se invalida con ellos. |

**Valoracion global**: los riesgos B1 (duplicacion) y B2 (desincronizacion) son los mas serios. B1 se mitiga porque la provenance son punteros, no datos. B2 requiere decision de diseno: referencia viva con validacion, o snapshot historico.

### 11.4. Beneficios (Sombrero Amarillo)

#### Para el usuario humano

1. **Transparencia de la compresion**: al ver que el edge B→A comprime 3 niveles del GT, el usuario entiende POR QUE esa flecha es tan cargada de supuestos. La EC deja de ser una abstraccion y se convierte en una ventana al analisis subyacente.

2. **Navegacion drill-down**: la UI puede ofrecer "zoom into source" — clic en el edge B→A y se despliega la cadena GT. El usuario navega entre niveles de abstraccion sin perder contexto.

3. **Pedagogia LTP mejorada**: para un analista novato, ver que "D viene del CRT como causa raiz, y B viene del GT como condicion necesaria" es una leccion viva de como funcionan las transiciones cross-tree.

#### Para el agente LLM

4. **Supuestos mas ricos**: con provenance, el agente sabe que cadena cuestionar. En vez de generar supuestos genericos para B→A, puede inspeccionar los nodos intermedios (OBJ-001, NC-003) y generar supuestos que cuestionen cada eslabon:
   - "Si queremos GOAL-001, y si *OBJ-001 no es el unico camino hacia GOAL-001*, entonces no necesitamos NC-003" (cuestiona OBJ-001→GOAL-001)
   - "Si queremos CSF-002, y si *NC-007 ya se satisface por otro medio*, entonces no necesitamos PRE-002" (cuestiona NC-007→CSF-002)

5. **Validacion cruzada automatica**: el agente puede verificar que la cadena comprimida sigue siendo valida consultando el GT/CRT actual. Si un nodo intermedio fue invalidado o editado, el agente detecta inconsistencia.

6. **Contexto para la INJ**: al invalidar un supuesto, el agente sabe exactamente que eslabon de la cadena cuestiono. La INJ puede ser mas precisa porque ataca un punto especifico de la cadena, no la flecha abstracta.

#### Para validate

7. **Nuevo warning `EC_PROVENANCE_STALE`**: si la provenance referencia nodos que ya no existen o cuyo label cambio significativamente, `validate` emite warning.

8. **Nuevo warning `EC_COMPRESSION_DEPTH`**: si un edge EC comprime mas de 5 niveles, advertir que la compresion puede ocultar logica critica que merece supuestos adicionales.

9. **Cross-check de completitud**: si la EC tiene projection_map y un nodo mapea a un CSF del GT que tiene 4 NCs, pero el edge EC solo captura 1, advertir que la compresion puede ser incompleta.

#### Para trace

10. **Narrativa con zoom**: `trace --narrative` puede expandir los edges EC inline:
    ```
    EC "Velocidad vs Control"
      B→A: "Para Logistica de clase mundial, necesito Control de costes"
            [comprime: NC-003 → OBJ-001 → GOAL-001 en GT]
      D→B: "Para Control de costes, debo usar entregas manuales"
            [comprime: RC-001 → INT-003 → UDE-002 en CRT, que viola NC-003 en GT]
    ```

### 11.5. Alternativas de diseno (Sombrero Verde)

#### Opcion A — Todo en la tree_relation: `projection_map` + `edge_provenance`

Ambos conceptos (mapeo de nodos y compresion de edges) viven en la tree_relation `core_conflict`.

```json
{
  "id": "TREL-002",
  "from_tree": "tree-crt-logistica",
  "to_tree": "tree-ec-conflicto",
  "relation_type": "core_conflict",
  "handoff_nodes": [{"node_id": "RC-001", "source_operator": "AND"}],
  "projection_map": [
    {"ec_node": "REQ-005", "ec_role": "A", "source_node": "GOAL-001", "source_tree": "tree-gt-logistica", "source_role": "goal"},
    {"ec_node": "REQ-006", "ec_role": "B", "source_node": "NC-003", "source_tree": "tree-gt-logistica", "source_role": "nc"},
    {"ec_node": "REQ-007", "ec_role": "C", "source_node": "NC-007", "source_tree": "tree-gt-logistica", "source_role": "nc"},
    {"ec_node": "PRE-001", "ec_role": "D", "source_node": "RC-001", "source_tree": "tree-crt-logistica", "source_role": "root_cause"},
    {"ec_node": "PRE-002", "ec_role": "D'", "source_node": null, "source_tree": null, "source_role": null}
  ],
  "edge_provenance": [
    {"ec_edge": "B-A", "collapsed_path": ["NC-003", "OBJ-001", "GOAL-001"], "source_tree": "tree-gt-logistica", "depth": 2},
    {"ec_edge": "C-A", "collapsed_path": ["NC-007", "CSF-002", "GOAL-001"], "source_tree": "tree-gt-logistica", "depth": 2},
    {"ec_edge": "D-B", "collapsed_path": ["RC-001", "INT-003", "INT-005", "UDE-002"], "source_tree": "tree-crt-logistica", "depth": 3, "cross_tree_link": "UDE-002 viola NC-003 en GT"},
    {"ec_edge": "D'-C", "collapsed_path": ["PRE-002"], "source_tree": null, "depth": 0}
  ],
  "description": "EC como proyeccion comprimida de GT + CRT. CRC RC-001 genera el conflicto central."
}
```

**SQL** (ltp-engine-max):
```sql
ALTER TABLE tree_relations ADD COLUMN projection_map TEXT;  -- JSON array, nullable
-- No se necesita tabla extra. El campo es JSON dentro de tree_relations.
```

| Ventaja | Desventaja |
|---------|-----------|
| Todo en un lugar — una tree_relation tiene el panorama completo | La tree_relation se vuelve muy pesada (~40 lineas JSON para una EC tipica) |
| Facil de consultar: "dame la provenance de esta EC" = 1 query | Mezcla niveles: la relacion entre arboles (meta-grafo) y la estructura interna del arbol (edges) |
| Compatible con el schema actual (campo JSON nullable) | La edge_provenance referencia edges de la EC por nombre ("B-A") en vez de por ID — fragil |

#### Opcion B — `projection_map` en tree_relation, sin `edge_provenance`

Solo se registra de donde vienen los nodos. La compresion de edges se infiere al vuelo.

```json
{
  "id": "TREL-002",
  "from_tree": "tree-crt-logistica",
  "to_tree": "tree-ec-conflicto",
  "relation_type": "core_conflict",
  "handoff_nodes": [{"node_id": "RC-001", "source_operator": "AND"}],
  "projection_map": [
    {"ec_node": "REQ-005", "ec_role": "A", "source_node": "GOAL-001", "source_tree": "tree-gt-logistica"},
    {"ec_node": "REQ-006", "ec_role": "B", "source_node": "NC-003", "source_tree": "tree-gt-logistica"},
    {"ec_node": "REQ-007", "ec_role": "C", "source_node": "NC-007", "source_tree": "tree-gt-logistica"},
    {"ec_node": "PRE-001", "ec_role": "D", "source_node": "RC-001", "source_tree": "tree-crt-logistica"},
    {"ec_node": "PRE-002", "ec_role": "D'", "source_node": null, "source_tree": null}
  ],
  "description": "EC como proyeccion comprimida de GT + CRT."
}
```

**Derivar la compresion al vuelo**: si sabemos que B mapea a NC-003 y A mapea a GOAL-001, y ambos estan en el GT, podemos calcular el camino entre NC-003 y GOAL-001 usando `trace` o `tree_walk` en el GT. No necesitamos almacenar el `collapsed_path`.

```rust
fn derive_collapsed_path(
    ec_edge: &Edge, // edge B→A en la EC
    projection: &ProjectionMap,
    engine: &Engine,
) -> Option<Vec<NodeId>> {
    let source_node = projection.get(ec_edge.from_node)?.source_node?;
    let target_node = projection.get(ec_edge.to_node)?.source_node?;
    let source_tree = projection.get(ec_edge.from_node)?.source_tree?;
    // Solo si ambos vienen del mismo arbol
    if projection.get(ec_edge.to_node)?.source_tree == Some(source_tree) {
        engine.find_path(source_tree, source_node, target_node)
    } else {
        None // cross-tree, no se puede derivar con un solo tree_walk
    }
}
```

| Ventaja | Desventaja |
|---------|-----------|
| Menor duplicacion — el collapsed_path no se almacena | No funciona para edges cross-tree (D→B comprime CRT+GT) — requiere logica de join |
| Siempre sincronizado — se calcula desde el estado actual del GT/CRT | La derivacion requiere que los arboles fuente existan y no hayan cambiado |
| Mas simple de crear y mantener | Pierde la "foto" de lo que se comprimia cuando se construyo la EC |

#### Opcion C — `provenance` en los edges de la EC

Cada edge de la EC (entidad que ya existe en el motor, tabla `edges`) tiene un campo `provenance`:

```json
{
  "id": "LINK-042",
  "tree_id": "tree-ec-conflicto",
  "to_node": "REQ-005",
  "operator": "SINGLE",
  "provenance": {
    "source_tree": "tree-gt-logistica",
    "collapsed_path": ["NC-003", "OBJ-001", "GOAL-001"],
    "depth": 2
  }
}
```

**SQL** (ltp-engine-max):
```sql
ALTER TABLE edges ADD COLUMN provenance TEXT;  -- JSON, nullable
-- Solo aplica a edges de ECs. Null para todo lo demas.
```

| Ventaja | Desventaja |
|---------|-----------|
| La provenance vive donde es natural: en el edge que comprime | Agrega un campo al schema de `edges` que solo aplica a ECs — campo especifico de tipo de arbol en entidad generica |
| El motor actual ya tiene `edge_inspect` — la provenance aparece inline | El meta-grafo no tiene visibilidad directa de la compresion — hay que consultar los edges |
| Facil de crear: `link_connect` podria aceptar `--provenance` | No captura la relacion de mapeo de NODOS (de donde viene A, B, C, D, D') |

#### Opcion B+C — Hibrida: `projection_map` en tree_relation + `provenance` en edges EC

La tree_relation captura **que nodos cruzan** (projection_map). Los edges de la EC capturan **que caminos comprimen** (provenance).

**tree_relation**:
```json
{
  "id": "TREL-002",
  "relation_type": "core_conflict",
  "projection_map": [
    {"ec_node": "REQ-005", "ec_role": "A", "source_node": "GOAL-001", "source_tree": "tree-gt-logistica"},
    {"ec_node": "PRE-001", "ec_role": "D", "source_node": "RC-001", "source_tree": "tree-crt-logistica"}
  ]
}
```

**edge de la EC**:
```json
{
  "id": "LINK-042",
  "tree_id": "tree-ec-conflicto",
  "to_node": "REQ-005",
  "provenance": {
    "source_tree": "tree-gt-logistica",
    "collapsed_path": ["NC-003", "OBJ-001", "GOAL-001"],
    "depth": 2
  }
}
```

| Ventaja | Desventaja |
|---------|-----------|
| Separacion clara: "de donde vienen los nodos" vs "que comprimen los edges" | Dos lugares que mantener — el agente debe crear ambos consistentemente |
| Cada pieza vive donde es natural | Schema mas complejo: un campo nuevo en tree_relations Y un campo nuevo en edges |
| La tree_relation mantiene la vision de alto nivel; el edge tiene el detalle | Si la provenance del edge contradice el projection_map de la relacion, hay inconsistencia |

#### Opcion D — Derivar todo, no almacenar nada

Ni projection_map ni provenance. El motor deriva todo al vuelo:

- Si un nodo EC participa en otro arbol via `tree_attach`, el motor sabe de donde viene.
- El collapsed_path se calcula con `find_path` entre el nodo fuente y el nodo destino en el arbol de origen.

| Ventaja | Desventaja |
|---------|-----------|
| Zero storage, zero desincronizacion, zero mantenimiento | Requiere que los nodos EC esten SIEMPRE `tree_attach`-eados a los arboles fuente — hoy es opcional |
| Imposible de duplicar o contradecir | No funciona si los nodos EC se crearon como copias nuevas en vez de reutilizarse |
| Maxima simplicidad | Pierde historicidad — si el GT cambia, la provenance cambia retroactivamente |

#### Recomendacion

**Opcion B** (projection_map en tree_relation, provenance derivada al vuelo) como **base**, con **escape hatch a C** para edges cross-tree que no se pueden derivar.

Justificacion:

1. **`projection_map` se almacena** en la tree_relation — es lightweight (5 entradas para una EC), captura la decision de "de donde viene cada nodo", y es dificil de derivar automaticamente (el motor no sabe que REQ-005 "es" GOAL-001 a menos que se lo digan).

2. **`provenance` de edges se deriva al vuelo** siempre que sea posible — si B mapea a NC-003 (GT) y A mapea a GOAL-001 (GT), el motor calcula el path entre NC-003 y GOAL-001 en el GT. Zero storage, siempre sincronizado.

3. **`provenance` se almacena en el edge** solo cuando es cross-tree y no derivable — el edge D→B comprime CRT+GT, cruzando arboles. El motor no puede calcular un path que cruza dos arboles. En este caso, se almacena `provenance` en el edge como snapshot.

4. **Todo es opcional** — una EC sin projection_map ni provenance funciona exactamente como hoy. El agente puede crear la provenance proactivamente; el usuario puede ignorarla.

**Esquema de la recomendacion**:

```sql
-- tree_relations (ya existe, campo nuevo)
ALTER TABLE tree_relations ADD COLUMN projection_map TEXT;  -- JSON array, nullable

-- edges (ya existe, campo nuevo)
ALTER TABLE edges ADD COLUMN provenance TEXT;  -- JSON, nullable. Solo para edges EC cross-tree.
```

```json
// Ejemplo projection_map en tree_relation:
"projection_map": [
  {"ec_node": "REQ-005", "ec_role": "A", "source_node": "GOAL-001", "source_tree": "tree-gt-logistica"},
  {"ec_node": "REQ-006", "ec_role": "B", "source_node": "NC-003", "source_tree": "tree-gt-logistica"},
  {"ec_node": "REQ-007", "ec_role": "C", "source_node": "NC-007", "source_tree": "tree-gt-logistica"},
  {"ec_node": "PRE-001", "ec_role": "D", "source_node": "RC-001", "source_tree": "tree-crt-logistica"},
  {"ec_node": "PRE-002", "ec_role": "D'", "source_node": null, "source_tree": null}
]

// Ejemplo provenance en edge (solo cross-tree, no derivable):
"provenance": {
  "source_tree": "tree-crt-logistica",
  "collapsed_path": ["RC-001", "INT-003", "INT-005", "UDE-002"],
  "depth": 3,
  "cross_tree_link": "UDE-002 viola NC-003 en tree-gt-logistica"
}
```

### 11.6. Coherencia arquitectonica (Sombrero Azul)

#### Alineacion con invariantes

| Principio | Cumplimiento | Notas |
|-----------|-------------|-------|
| Motor determinista, sin semantica bloqueante | **Si** | projection_map y provenance son metadata opcional. `validate` emite warnings, no errores |
| IDs secuenciales | **Si** | No requiere IDs nuevos. projection_map usa IDs de nodos existentes |
| JSON canonico (BTreeMap) | **Si** | Ambos campos JSON se serializan con claves ordenadas |
| Entidades de primer nivel (ADR-005) | **Si** | No se crean entidades nuevas. Se enriquecen tree_relations y edges (nullable) |
| Compatibilidad aditiva | **Si** | Campos nullable, backwards-compatible. EC sin projection_map funciona como hoy |

#### Relacion con Seccion 10 (suficiencia vs necesidad)

La proyeccion EC cruza fronteras logicas de forma **bidireccional**:

- Los nodos A, B, C vienen del GT (necesidad) → entran en la EC (necesidad): **same_nec**
- Los nodos D, D' vienen del CRT (suficiencia) → entran en la EC (necesidad): **suf_to_nec**

Esto significa que la EC es el unico arbol LTP donde **coexisten nodos de ambas logicas**, recontextualizados bajo logica de necesidad. El `projection_map` debe capturar esta dualidad:

```json
// D viene de suficiencia (CRT), entra en necesidad (EC)
{"ec_node": "PRE-001", "ec_role": "D", "source_node": "RC-001", "source_tree": "tree-crt-logistica"}
// El source_tree es de suficiencia. El motor sabe que hay un cruce logico.
```

Los 3 warnings de la Seccion 10 (`LOGIC_TRANSITION_UNVALIDATED`, `ASSUMPTION_CROSS_LOGIC`, `OPERATOR_CONTEXT_LOST`) aplican especificamente a los nodos D/D' del projection_map cuando `source_tree.logic == sufficiency`.

#### Impacto en comandos existentes

| Comando | Cambio |
|---------|--------|
| `tree_new --type ec` | Opcionalmente acepta `--from-crc <node-id>` que pre-rellena D del projection_map |
| `tree_attach` | Si el nodo ya participa en GT/CRT y se attach a una EC, sugerir actualizar projection_map |
| `relation_add --type core_conflict` | Opcionalmente acepta `--projection-map` JSON |
| `link_connect` (en EC) | Opcionalmente acepta `--provenance` para edges cross-tree |
| `link_inspect` (en EC) | Muestra provenance si existe (almacenada o derivada) |
| `validate` | Nuevos warnings: `EC_PROVENANCE_STALE`, `EC_COMPRESSION_DEPTH` |
| `trace --narrative` | Si hay provenance, expande edges EC inline con la cadena comprimida |
| `assume_add` (en EC) | Si hay provenance, el output incluye "este supuesto cuestiona la cadena: [collapsed_path]" |

#### Impacto en el schema

**ltp-engine (JSON)**:

- `relations/TREL-XXX.json`: campo opcional `projection_map` (array de objetos)
- `trees/<ec-tree>.json` → edges: campo opcional `provenance` (objeto)
- Ambos nullable. Zero breaking changes.

**ltp-engine-max (SQL)**:

```sql
-- tree_relations: 1 columna nueva
ALTER TABLE tree_relations ADD COLUMN projection_map TEXT;  -- JSON array, nullable

-- edges: 1 columna nueva
ALTER TABLE edges ADD COLUMN provenance TEXT;  -- JSON object, nullable
```

#### Compatibilidad con ambas arquitecturas

| Arquitectura | projection_map | provenance derivada | provenance almacenada |
|-------------|---------------|--------------------|--------------------|
| ltp-engine (JSON) | JSON field en tree_relation | Requiere cargar GT/CRT en memoria y hacer tree_walk | JSON field en edge |
| ltp-engine-max (SQL) | JSON column en tree_relations | Query SQL con CTE recursivo sobre edges del arbol fuente | JSON column en edges |

La derivacion al vuelo es mas eficiente en SQL (CTE recursivo vs cargar arboles en memoria para JSON), lo cual refuerza la ventaja de ltp-engine-max para esta funcionalidad.

### 11.7. Resumen ejecutivo

La EC es una **proyeccion comprimida** del GT y del CRT — no un arbol independiente. Sus nodos provienen de otros arboles y sus edges colapsan cadenas multi-nivel. Los supuestos de la EC cuestionan esas cadenas comprimidas, y su poder reside en hacer cuestionable lo que estaba oculto en la complejidad.

**Decision de diseno**: modelar la proveniencia en dos capas:

1. **`projection_map`** en la tree_relation (campo JSON nullable): mapea los 5 nodos de la EC a sus fuentes en GT/CRT. Se almacena porque no es derivable automaticamente.

2. **`provenance`** en edges de la EC: se **deriva al vuelo** cuando ambos nodos del edge vienen del mismo arbol (calcular path en GT o CRT). Se **almacena en el edge** (campo JSON nullable) solo para edges cross-tree cuya cadena cruza multiples arboles y no es derivable.

3. **Todo es opcional**: una EC sin projection_map ni provenance funciona como hoy. Zero breaking changes.

**Impacto**: 2 campos nullable nuevos (1 en tree_relations, 1 en edges), 2 warnings nuevos en validate, provenance inline en trace narrative, contexto enriquecido para generacion de supuestos.

**Riesgo principal**: desincronizacion (B2) si los arboles fuente cambian. Mitigacion: `validate` detecta nodos referenciados que ya no existen.

---

## 12. Actores como Dimension del Meta-Grafo: Perspectivas Multi-Actor

### Contexto

En LTP, Dettmer prescribe centrarse en el "dueno del sistema" — el actor con autoridad y responsabilidad para cambiar las cosas. El Goal Tree se construye desde SU perspectiva. Pero la realidad es multi-actor: proveedores, clientes, reguladores, departamentos internos, cada uno con su propia logica de necesidad. En la practica, cuando construimos un CRT estamos reflejando la realidad multi-actor de forma **implicita**, y cuando construimos un FRT simulamos como reaccionaran otros actores — sin haber explicitado sus objetivos ni restricciones.

Esta seccion analiza si y como modelar actores como dimension del meta-grafo, y las implicaciones para el LTP y para ltp-engine.

### 12.1. Hechos: los actores en el LTP actual (Sombrero Blanco)

#### Que dice Dettmer sobre actores

Dettmer establece que el analisis LTP se conduce desde la perspectiva de quien tiene **autoridad y responsabilidad** sobre el sistema — el "system owner". Esto no es arbitrario: el LTP necesita un punto de ancla para definir que es un UDE (efecto indeseable para QUIEN?) y que es un Goal (objetivo de QUIEN?). Sin ese ancla, el analisis pierde foco.

Conceptos clave de Dettmer relacionados:

| Concepto | Descripcion | Implicacion para actores |
|----------|-------------|-------------------------|
| **Span of control** | Lo que el dueno puede cambiar directamente | Acciones/inyecciones deben estar dentro del span |
| **Sphere of influence** | Lo que el dueno puede influir pero no controlar | Otros actores viven aqui — influenciables pero autonomos |
| **System boundary** | Que esta dentro y fuera del analisis | Define que actores son "internos" vs "externos" |
| **Stakeholders** (GT) | Quienes son los interesados | Ya capturado en `systems.stakeholders` pero sin estructura |

#### Como aparecen actores implicitamente en cada tipo de arbol

| Arbol | Donde aparecen actores | Que se pierde por no modelarlos |
|-------|----------------------|-------------------------------|
| **GT** | El Goal y los CSFs son del dueno. Pero algunos NCs dependen de que otros actores cooperen: "Proveedores entregan a tiempo" es un NC que el dueno no controla | No se sabe que NCs dependen de actores externos → no se evalua viabilidad |
| **CRT** | Las UDEs afectan a diferentes actores: "Clientes cancelan pedidos" (actor=cliente), "Operarios hacen horas extra" (actor=empleados), "Proveedor incumple SLA" (actor=proveedor) | UDEs sin actor = imposible saber quien sufre, quien causa, y quien puede resolver |
| **EC** | D y D' frecuentemente representan acciones/politicas de actores DISTINTOS: "Reducir costes" (gerencia) vs "Mantener calidad de servicio" (operaciones). El conflicto es inter-actor | Sin modelar actores, la EC pierde la dimension de "conflicto entre quienes" |
| **FRT** | Las INJs asumen comportamiento de otros actores: "El proveedor adoptara el nuevo sistema" es una asuncion sobre el GT implicito del proveedor | INJs que invaden el GT de otro actor sin evaluacion de viabilidad |
| **PRT** | Los IOs frecuentemente requieren accion de otros actores: "Negociar con proveedor" (prerequisito que depende de otro actor) | No se distingue IO bajo control del dueno vs IO que requiere cooperacion externa |
| **TT** | Las acciones tacticas asignan responsabilidad: "Gerente de logistica implementa GPS", "IT configura integracion" | Sin actores, las acciones no tienen responsable → nadie sabe quien ejecuta que |

#### Cuantos actores tiene un sistema tipico

En un analisis LTP de complejidad media:

| Metrica | Valor tipico |
|---------|-------------|
| Actores totales identificables | 8-15 |
| Actores relevantes para el analisis | 3-6 |
| Actores con impacto en INJs | 2-4 |
| Actores con GT implicito propio | 1-3 (ademas del dueno) |

#### Informacion sobre actores que se pierde hoy

1. **Atribucion de UDEs**: no se sabe quien experimenta cada UDE
2. **Dependencia de NCs**: no se sabe que NCs del GT dependen de cooperacion de actores externos
3. **Conflicto inter-actor en la EC**: D y D' no se atribuyen a actores → se pierde la dimension "conflicto entre quienes"
4. **Viabilidad de INJs**: una INJ que requiere cambio de comportamiento de un actor externo tiene menos probabilidad de exito que una bajo control directo
5. **Supuestos ocultos sobre actores**: "El proveedor cumplira" es un supuesto sobre el GT implicito del proveedor — no capturado como ASM
6. **Responsabilidad de acciones**: las acciones del TT no tienen responsable asignado
7. **Impacto de escenarios en actores**: un escenario puede ser optimo para el dueno pero terrible para otro actor (proveedor pierde margen → dejara de cooperar)

#### Relacion con la EC como proyeccion comprimida (Seccion 11)

La seccion 11 mostro que los nodos D y D' de la EC vienen frecuentemente de fuentes distintas (CRT, GT). Lo que no se capturo es que D y D' a menudo representan **actores distintos**:

```
EC "Velocidad vs Control"
  A: "Logistica de clase mundial"           → actor: Empresa
  B: "Control de costes operativos"         → actor: Gerencia financiera
  C: "Visibilidad en tiempo real"           → actor: Operaciones / Clientes
  D: "Usar entregas manuales sin tracking"  → actor: Operaciones (status quo)
  D': "Invertir en GPS tracking"            → actor: IT / Gerencia financiera
  
  Conflicto: D (operaciones quiere simplicidad) vs D' (gerencia quiere inversion)
```

El `projection_map` de la seccion 11 mapea nodos a arboles fuente. La atribucion de actores agrega una dimension ortogonal: mapea nodos a QUIEN los experimenta o ejecuta.

### 12.2. Intuicion (Sombrero Rojo)

El LTP funciona porque **fuerza foco**. Un dueno, un sistema, un analisis. Esa restriccion es lo que lo hace potente: en vez de modelar toda la complejidad organizacional, fuerza a pensar "que puedo YO cambiar".

Pero esa restriccion tiene un coste: **los puntos ciegos sobre otros actores**. El analista piensa implicitamente en los GTs de proveedores, clientes, reguladores — pero esa reflexion se pierde. Las INJs mas fragiles del LTP son las que asumen cooperacion de actores cuya logica de necesidad no se ha examinado.

La tension es real:
- Demasiada estructura multi-actor → se pierde el foco, se convierte en modelado organizacional
- Ninguna estructura → los supuestos sobre actores quedan ocultos y las INJs fallan en la realidad

La respuesta intuitiva es: **no modelar los GTs completos de todos los actores, sino hacer visible DONDE un actor entra en juego y QUE se asume sobre el**. Es la diferencia entre un mapa topografico completo y un mapa de carreteras que marca los cruces peligrosos.

Hay un riesgo emocional: la atribucion de actores puede convertirse en una herramienta politica ("este es TU problema, no el mio"). El LTP no debe ser un arma para asignar culpas sino una herramienta para entender el sistema.

### 12.3. Riesgos (Sombrero Negro — Obligatorio)

| # | Riesgo | Severidad | Detalle | Mitigacion |
|---|--------|-----------|---------|------------|
| B1 | **Explosion combinatoria** — N actores × M arboles = N×M potenciales perspectivas | ALTA | Con 4 actores relevantes y 6 tipos de arbol, hay 24 arboles potenciales. La mayoria seran innecesarios, pero la tentacion de "completar el mapa" es real. Un agente LLM podria sugerir "crear el GT del proveedor" compulsivamente. | Limitar GTs adicionales a actores con impacto directo en INJs. El motor NO sugiere GTs proactivamente — solo cuando detecta INJ que invade GT ajeno. |
| B2 | **Perdida de foco** — el LTP funciona porque fuerza a pensar desde UNA perspectiva. Multi-actor diluye esa fuerza | ALTA | Si el analista empieza a construir el CRT desde la perspectiva del proveedor, luego del cliente, pierde el hilo del analysis del dueno. El LTP se convierte en un ejercicio de empatia multi-stakeholder sin conclusion accionable. | La perspectiva primaria (dueno) es SIEMPRE la principal. Los GTs de otros actores son **satelites** — opcionales, parciales, y solo para validar supuestos. El motor marca claramente: "Este GT es del Actor X — satelite, no principal." |
| B3 | **Scope creep — modelar la organizacion, no el problema** | ALTA | Actores con roles, relaciones de poder, estructuras organizativas... eso es un organigrama, no un LTP. Si empezamos a modelar "quien reporta a quien" o "que departamento controla que presupuesto", nos alejamos del thinking process. | Los actores son una ETIQUETA ligera, no una entidad rica. No modelamos relaciones entre actores, jerarquias, ni poder. Solo: "este nodo/supuesto depende de este actor." |
| B4 | **Complejidad de mantenimiento de la atribucion** | MEDIA | Cada nodo puede potencialmente tener un actor. Mantener esa atribucion actualizada cuando se editan nodos, se mueven entre arboles, o se reescriben labels es overhead significativo. | La atribucion es **opcional** y **inferible**. El agente puede proponer atribucion basandose en el contenido del nodo ("Clientes cancelan pedidos" → actor=cliente). El usuario confirma o ignora. |
| B5 | **Curva de aprendizaje** — el LTP ya es metodologicamente exigente. Agregar actores eleva la barrera | MEDIA | Un analista novato que ya lucha con la diferencia entre suficiencia y necesidad, CLRs, y EC protocol... ahora tiene que pensar en actores. Es una capa mas de complejidad. | Activacion gradual. Nivel 0: sin actores (hoy). Nivel 1: solo atribucion de nodos. Nivel 2: supuestos con dependencia de actor. Nivel 3: GTs satelite. Cada nivel es opcional. |
| B6 | **Analysis paralysis** — "no puedo continuar hasta modelar el GT del proveedor" | MEDIA | Si el motor emite warnings tipo "INJ-005 invade GT del proveedor — GT no modelado", el analista puede sentir que necesita modelar ese GT antes de continuar. Esto bloquea el progreso del analisis principal. | Los warnings de actor son INFORMATIVOS, nunca bloqueantes. Siempre incluyen "sugerencia: capturar supuesto sobre Actor X" como alternativa rapida al GT completo. |
| B7 | **Atribucion ambigua** — muchos nodos involucran multiples actores. "Retrasos en entregas" afecta al proveedor, al cliente, y al equipo de operaciones | BAJA | Si la atribucion permite multiples actores por nodo, se pierde la utilidad de la etiqueta. Si solo permite uno, se pierde precision. | Atribucion con roles: `actor_attributions: [{actor: "proveedor", role: "causa"}, {actor: "cliente", role: "afectado"}]`. Pero esto aumenta la complejidad (ver B4). Alternativa: un solo actor primario por nodo, con nota textual. |
| B8 | **Conflicto con la EC** — la EC modela un conflicto entre D y D'. Si D y D' son de actores distintos, la EC se convierte en un conflicto inter-actor. Pero la metodologia Dettmer trata la EC como conflicto INTERNO del dueno | BAJA-MEDIA | Dettmer dice que el conflicto es del dueno: "YO quiero D pero tambien quiero D'". Si D es "lo que quiere mi equipo de operaciones" y D' es "lo que quiere finanzas", el conflicto sigue siendo del dueno (que gestiona ambos). Pero si D es "lo que quiere mi proveedor" y D' es "lo que quiero yo", es un conflicto donde una parte esta fuera de span of control. | La atribucion de actores en la EC es INFORMATIVA — no cambia la mecanica de la EC. Sigue siendo el conflicto del dueno. Pero saber que D viene de un actor externo ayuda a evaluar la viabilidad de invalidar supuestos en esa flecha. |

**Valoracion global**: los riesgos B1 (explosion), B2 (perdida de foco) y B3 (scope creep) son los mas serios y estan interconectados. Todos se mitigan con el mismo principio: **los actores son etiquetas ligeras y opcionales, no entidades ricas**. Si la implementacion respeta esto, el riesgo es manejable.

### 12.4. Beneficios (Sombrero Amarillo)

#### Para el analisis

1. **Supuestos ocultos se hacen explicitos**: cuando un nodo se atribuye a un actor externo, el motor puede sugerir: "Este nodo depende de Actor X. Hay un supuesto implicito sobre el comportamiento de X. Capturar como ASM?" Esto convierte suposiciones invisibles en supuestos auditables.

2. **Viabilidad de INJs mejorada**: una INJ con `actor_dependency: "proveedor"` tiene menos probabilidad de exito que una con `actor_dependency: null` (bajo control directo). El agente LLM puede ponderar viabilidad usando esta informacion.

3. **Dimension "conflicto entre quienes" en la EC**: saber que D pertenece a un actor y D' a otro enriquece el analisis de supuestos. Los supuestos de la flecha B→D cuestionan por que Actor X necesita D — y eso lleva a inspeccionar el GT implicito de Actor X.

#### Para la validacion

4. **Nuevo warning `ACTOR_DEPENDENCY_UNEXAMINED`**: cuando una INJ tiene atribucion a un actor externo y no hay supuesto capturado sobre ese actor, `validate` emite warning.

5. **Nuevo warning `INJ_INVADES_EXTERNAL_GT`**: cuando una INJ requiere cambio de comportamiento de un actor cuyo GT satelite existe y la INJ contradice uno de sus NCs.

6. **Cross-check de responsabilidad**: si un IO del PRT o una accion del TT depende de un actor externo y no hay mecanismo explicito (contrato, acuerdo, incentivo), emitir warning.

#### Para el agente LLM

7. **Contexto para generacion de supuestos**: al generar supuestos para un edge de la EC, el agente puede inspeccionar el GT satelite del actor involucrado y generar supuestos mas profundos: "Asumimos que el proveedor priorizara nuestro pedido (contradice su NC-002: maximizar margen diversificando clientes)."

8. **Evaluacion de INJs contra GTs satelite**: el agente puede verificar: "INJ-005 requiere que el proveedor cambie su proceso de entregas. Segun el GT satelite del proveedor, esto viola su NC-003. Probabilidad de aceptacion: baja sin incentivo adicional."

#### Para la UI

9. **Stakeholder map visual**: la UI puede mostrar un diagrama de actores con lineas hacia los nodos que les atanen. Clic en un actor → vista filtrada de todos los nodos, supuestos y acciones que le involucran.

10. **Vista por actor**: filtrar el CRT para ver solo UDEs que afectan a un actor especifico. Filtrar el TT para ver solo acciones asignadas a un actor.

#### Para el meta-grafo

11. **Actores enriquecen escenarios**: un escenario puede evaluarse por impacto en cada actor: "Escenario A: bueno para el dueno, malo para el proveedor. Escenario B: neutral para ambos." Esto previene implementar un escenario que sabotea un actor critico.

12. **Actores enriquecen temporal frames**: un mismo actor puede tener estado diferente en `present` vs `future`. "Proveedor: hoy es reactivo (present). Con GPS tracking, necesita ser proactivo (future)."

### 12.5. Alternativas de diseno (Sombrero Verde)

#### Enfoque A — GT completo por actor

Cada actor relevante tiene su propio Goal Tree completo, y se crean tree_relations entre GTs de diferentes actores.

**Ejemplo**:

```json
// GT del proveedor (satelite)
{
  "id": "tree-gt-proveedor",
  "tree_type": "gt",
  "system_id": "SYS-001",
  "actor_id": "ACT-002",
  "name": "Goal Tree — Proveedor Logistico"
}

// Relacion entre el GT del proveedor y el GT del dueno
{
  "id": "TREL-010",
  "from_tree": "tree-gt-logistica",
  "to_tree": "tree-gt-proveedor",
  "relation_type": "actor_dependency",
  "handoff_nodes": [{"node_id": "NC-005"}],
  "description": "NC-005 del dueno depende de cooperacion del proveedor"
}
```

**SQL (actor como entidad)**:
```sql
CREATE TABLE actors (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    actor_type  TEXT NOT NULL,  -- 'owner' | 'internal' | 'external' | 'regulator'
    description TEXT,
    system_id   TEXT REFERENCES systems(id)
);

ALTER TABLE trees ADD COLUMN actor_id TEXT REFERENCES actors(id);
-- Un arbol pertenece a un actor (null = dueno implicito)
```

| Ventaja | Desventaja |
|---------|-----------|
| Maximo rigor: cada actor tiene su logica de necesidad explicitada | Explosion combinatoria (B1): 4 actores × GT = 4 GTs adicionales como minimo |
| Permite validacion cruzada profunda: "INJ contradice NC del proveedor" | Perdida de foco (B2): el analista se pierde en GTs satelite |
| Relaciones inter-GT son entidades del meta-grafo | Scope creep (B3): estamos modelando el ecosistema, no el problema |
| El agente tiene contexto completo para evaluar INJs | Overhead masivo de creacion y mantenimiento |

#### Enfoque B — GT del dueno + actores como restricciones contextuales

Un solo GT (del dueno). Otros actores aparecen solo como texto en `systems.stakeholders` y como contexto en nodos/supuestos.

**Ejemplo** (basicamente el estado actual + texto):

```json
// Sistema con stakeholders textuales
{
  "id": "SYS-001",
  "stakeholders": [
    "Director de Operaciones (dueno)",
    "Proveedores logisticos (externos)",
    "Equipo de Customer Success (interno)",
    "Clientes finales (externos)"
  ]
}

// UDE con actor en el texto
{
  "id": "UDE-003",
  "label": "Clientes cancelan pedidos por incertidumbre en entregas",
  "node_type": "UDE"
}
// El "Clientes" es implicito en el label. No hay metadata.
```

| Ventaja | Desventaja |
|---------|-----------|
| Zero overhead — funciona como hoy | Ningun actor es queryable: no puedo pedir "todos los nodos del proveedor" |
| Sin riesgo de explosion ni scope creep | Los supuestos sobre actores quedan ocultos |
| Simple y rapido | El agente no tiene contexto para evaluar viabilidad de INJs |
| Consistente con Dettmer puro | Informacion que el analista tiene en la cabeza se pierde |

#### Enfoque C — Atribucion ligera a nivel de nodo y supuesto

No se crean GTs separados. Se agregan campos opcionales de atribucion a nodos y supuestos.

**JSON — atribucion en nodos**:

```json
{
  "id": "UDE-003",
  "label": "Clientes cancelan pedidos por incertidumbre en entregas",
  "node_type": "UDE",
  "actor": "cliente",
  "actor_role": "afectado"
}
```

**JSON — dependencia en supuestos**:

```json
{
  "id": "ASM-012",
  "text": "El proveedor adoptara el nuevo sistema de tracking",
  "edge_id": "LINK-025",
  "actor_dependency": "proveedor",
  "actor_assumption": "El proveedor priorizara nuestra integracion sobre la de otros clientes"
}
```

**SQL**:
```sql
-- Tabla ligera de actores (entidad minima)
CREATE TABLE actors (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    actor_type  TEXT NOT NULL,  -- 'owner' | 'internal' | 'external' | 'regulator'
    system_id   TEXT REFERENCES systems(id),
    description TEXT
);

-- Atribucion en nodos (campo nullable)
ALTER TABLE nodes ADD COLUMN actor_id TEXT REFERENCES actors(id);
ALTER TABLE nodes ADD COLUMN actor_role TEXT;  -- 'causa' | 'afectado' | 'ejecutor' | 'bloqueante'

-- Dependencia en supuestos (campo nullable)
ALTER TABLE assumptions ADD COLUMN actor_dependency TEXT REFERENCES actors(id);
```

| Ventaja | Desventaja |
|---------|-----------|
| Lightweight — campos opcionales, zero overhead si no se usan | No captura la logica de necesidad del actor (su GT) |
| Queryable: "todos los nodos del proveedor", "supuestos que dependen de cliente" | El agente no puede validar INJs contra el GT del actor |
| Actor_role distingue causa/afectado/ejecutor | Atribucion puede ser ambigua (B7): multiples actores por nodo |
| Compatible con ambas arquitecturas | Sin GT satelite, la dependencia queda como etiqueta sin profundidad |

#### Enfoque D — Hibrido: atribucion ligera (C) + GTs satelite opcionales (A)

Combina C como base (siempre disponible, bajo overhead) con A como profundizacion (solo cuando el analisis lo justifica).

**Nivel 0**: sin actores (como hoy).
**Nivel 1**: actores como entidades ligeras + atribucion en nodos/supuestos.
**Nivel 2**: GT satelite parcial para un actor especifico, cuando una INJ depende de el.

**Ejemplo del flujo**:

```
1. Analista construye CRT con UDEs
2. Agente detecta: "UDE-003 menciona 'clientes' — atribuir actor?"
   → Si: crea ACT-002 "Clientes finales" + atribuye UDE-003
3. Analista llega a la EC: D' = "Invertir en GPS"
   → Agente detecta: ASM-012 depende del proveedor
   → Crea ACT-003 "Proveedor logistico" + actor_dependency en ASM-012
4. Analista llega al FRT: INJ-005 requiere cooperacion del proveedor
   → Agente: "INJ-005 depende de ACT-003. Quieres explorar su GT para evaluar viabilidad?"
   → Si analista acepta: crea GT satelite parcial (GOAL + 2-3 NCs clave del proveedor)
   → Agente: "INJ-005 contradice NC-003 del proveedor. Capturar como supuesto?"
5. La mayoria de los actores quedan en Nivel 1. Solo el proveedor llego a Nivel 2.
```

**SQL (schema consolidado)**:

```sql
CREATE TABLE actors (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    actor_type  TEXT NOT NULL,  -- 'owner' | 'internal' | 'external' | 'regulator'
    system_id   TEXT REFERENCES systems(id),
    description TEXT
);

-- Nodos: atribucion ligera
ALTER TABLE nodes ADD COLUMN actor_id TEXT REFERENCES actors(id);
ALTER TABLE nodes ADD COLUMN actor_role TEXT;

-- Supuestos: dependencia de actor
ALTER TABLE assumptions ADD COLUMN actor_dependency TEXT REFERENCES actors(id);

-- Trees: actor propietario (para GTs satelite)
ALTER TABLE trees ADD COLUMN actor_id TEXT REFERENCES actors(id);

-- Relaciones: nuevo tipo 'actor_dependency' para inter-GT relations
-- (sin cambio de schema — relation_type ya es TEXT)
```

**JSON (ltp-engine)**:

```json
// actors/ACT-001.json
{
  "id": "ACT-001",
  "name": "Departamento de Operaciones",
  "actor_type": "owner",
  "system_id": "SYS-001",
  "description": "Dueno del sistema — Director de Operaciones"
}

// actors/ACT-003.json
{
  "id": "ACT-003",
  "name": "Proveedor Logistico Principal",
  "actor_type": "external",
  "system_id": "SYS-001",
  "description": "Partner logistico que gestiona ultima milla"
}

// Nodo con atribucion
{
  "id": "UDE-003",
  "label": "Clientes cancelan pedidos por incertidumbre en entregas",
  "node_type": "UDE",
  "actor_id": "ACT-002",
  "actor_role": "afectado"
}

// Supuesto con dependencia
{
  "id": "ASM-012",
  "text": "El proveedor adoptara el nuevo sistema de tracking",
  "edge_id": "LINK-025",
  "status": "valid",
  "actor_dependency": "ACT-003"
}

// GT satelite (arbol parcial del proveedor)
{
  "id": "tree-gt-proveedor",
  "tree_type": "gt",
  "system_id": "SYS-001",
  "actor_id": "ACT-003",
  "name": "Goal Tree satelite — Proveedor Logistico"
}
```

| Ventaja | Desventaja |
|---------|-----------|
| Escalado progresivo: desde cero overhead hasta analisis profundo | Schema mas complejo: tabla `actors` + 3 columnas nuevas en entidades existentes |
| Respeta Dettmer: el GT del dueno sigue siendo principal | Riesgo de confusion: cual es el GT "real" vs los "satelite"? |
| El agente escala: atribucion automatica (nivel 1) → GT satelite (nivel 2) solo cuando importa | Necesita UI/UX clara para distinguir niveles |
| Previene los 3 riesgos principales: no fuerza explosion (B1), mantiene foco (B2), no modela la org (B3) | Ligeramente mas complejo de implementar que C puro |
| Interaccion natural con EC projection (seccion 11): projection_map + actor_id dan vision completa | El "nivel 2" es un judgment call — cuando es "necesario" crear un GT satelite? |

#### Recomendacion

**Enfoque D (hibrido)** con las siguientes restricciones:

1. **Actores son entidades ligeras** — tabla/archivo con `id`, `name`, `actor_type`, `system_id`, `description`. Sin relaciones entre actores, sin jerarquias, sin metadata organizacional.

2. **Atribucion en nodos es opcional** — campo `actor_id` nullable. El agente lo sugiere basandose en el contenido del nodo; el usuario confirma o ignora. `actor_role` es un enum libre (`causa`, `afectado`, `ejecutor`, `bloqueante`, etc.).

3. **Dependencia en supuestos es opcional** — campo `actor_dependency` nullable. El motor emite warning `ACTOR_DEPENDENCY_UNEXAMINED` solo si el supuesto es critico (esta en una INJ o en la EC).

4. **GTs satelite son excepcionales** — solo se crean cuando una INJ depende de un actor externo Y el analista quiere evaluar viabilidad. El arbol se marca con `actor_id` y se distingue claramente como satelite. El motor NUNCA sugiere proactivamente crear GTs satelite.

5. **El dueno tiene un actor implicito** — `actor_type: "owner"`, `actor_id` en el GT principal. No es obligatorio crearlo; si no existe, el GT principal se asume del dueno.

### 12.6. Coherencia arquitectonica (Sombrero Azul)

#### Alineacion con invariantes

| Principio | Cumplimiento | Notas |
|-----------|-------------|-------|
| Motor determinista, sin semantica bloqueante | **Si** | Todos los warnings de actor son no-bloqueantes. El motor no juzga si un actor "cooperara" |
| IDs secuenciales | **Si** | `ACT-001`, `ACT-002`, etc. Patron consistente |
| JSON canonico (BTreeMap) | **Si** | Campos nuevos se serializan con claves ordenadas |
| Entidades de primer nivel (ADR-005) | **Si** | Actores son entidades direccionables con ID propio |
| Compatibilidad aditiva | **Si** | Todos los campos son nullable. Un workspace sin actores funciona como hoy |

#### Relacion con secciones previas

**Seccion 3 — Systems**: la entidad `systems` ya tiene `stakeholders` como String[]. Con actores como entidad, `stakeholders` se convierte en redundante o se reemplaza por una relacion `system_actors` (o simplemente `actors.system_id`). Recomendacion: mantener `stakeholders` como texto libre para notas rapidas; los `actors` son la version estructurada para quienes quieren queryabilidad.

**Seccion 10 — Suficiencia/Necesidad**: los GTs satelite son de necesidad (como todo GT). Las INJs del FRT (suficiencia) que dependen de actores externos cruzan una frontera no solo de arbol sino de "perspectiva". El warning `INJ_INVADES_EXTERNAL_GT` es un caso especifico de cross-logic transition.

**Seccion 11 — EC como proyeccion comprimida**: el `projection_map` de la tree_relation puede enriquecerse con actor_id por nodo mapeado:

```json
"projection_map": [
  {"ec_node": "PRE-001", "ec_role": "D", "source_node": "RC-001", "source_tree": "tree-crt-logistica", "actor_id": "ACT-001"},
  {"ec_node": "PRE-002", "ec_role": "D'", "source_node": null, "source_tree": null, "actor_id": "ACT-003"}
]
```

Esto completa la vision: el nodo D viene del CRT (seccion 11), opera en logica de suficiencia recontextualizada en necesidad (seccion 10), y pertenece al actor Operaciones (seccion 12). Tres dimensiones ortogonales que enriquecen el mismo nodo.

#### Impacto en comandos

| Comando | Cambio |
|---------|--------|
| `actor_new` | **NUEVO**: crear actor ligero |
| `actor_list` | **NUEVO**: listar actores de un sistema |
| `actor_inspect` | **NUEVO**: ver actor con nodos/supuestos que le referencian |
| `node_add` / `node_edit` | Opcionalmente acepta `--actor ACT-002 --actor-role afectado` |
| `assume_add` | Opcionalmente acepta `--actor-dependency ACT-003` |
| `tree_new` | Opcionalmente acepta `--actor ACT-003` (para GTs satelite) |
| `validate` | Nuevos warnings: `ACTOR_DEPENDENCY_UNEXAMINED`, `INJ_INVADES_EXTERNAL_GT` |
| `system_status` | Seccion de actores: cuantos, cuales tienen atribucion, cuales tienen GT satelite |
| `trace --narrative` | Anota actor cuando un nodo cruza de un arbol a otro |
| `scenario_compare` | Columna de impacto por actor en la comparacion |

#### Impacto en el schema

**ltp-engine (JSON)**:

- Nuevo directorio: `actors/` con `ACT-001.json`, etc.
- Nodos: campo opcional `actor_id`, `actor_role`
- Supuestos: campo opcional `actor_dependency`
- Trees: campo opcional `actor_id` (para GTs satelite)

**ltp-engine-max (SQL)**:

```sql
-- Tabla nueva
CREATE TABLE actors (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    actor_type  TEXT NOT NULL,
    system_id   TEXT REFERENCES systems(id),
    description TEXT
);

-- Columnas nuevas (todas nullable)
ALTER TABLE nodes ADD COLUMN actor_id TEXT REFERENCES actors(id);
ALTER TABLE nodes ADD COLUMN actor_role TEXT;
ALTER TABLE assumptions ADD COLUMN actor_dependency TEXT REFERENCES actors(id);
ALTER TABLE trees ADD COLUMN actor_id TEXT REFERENCES actors(id);
```

#### Workspace actualizado

```
mi-proyecto-ltp/
├── ltp.config.json
├── nodes/
├── trees/
├── knowledge/
├── systems/
├── relations/
├── scenarios/
├── actors/              ← NUEVO
│   ├── ACT-001.json
│   ├── ACT-002.json
│   └── ACT-003.json
└── .ltp/
```

### 12.7. Resumen ejecutivo

Los actores son una dimension que el LTP trata como implicita pero que tiene impacto real en la calidad del analisis — especialmente en la viabilidad de INJs, la profundidad de supuestos en la EC, y la asignacion de responsabilidad en el TT.

**Decision de diseno**: enfoque hibrido (D) con escalado progresivo:

1. **Actores como entidades ligeras** — tabla/archivo con campos minimos. Sin relaciones inter-actor, sin jerarquias.
2. **Atribucion opcional en nodos** — campo `actor_id` + `actor_role` nullables. El agente sugiere; el usuario confirma.
3. **Dependencia en supuestos** — campo `actor_dependency` nullable. Warning cuando supuesto critico no tiene dependencia examinada.
4. **GTs satelite excepcionales** — solo cuando una INJ depende de actor externo y se quiere evaluar viabilidad. Marcados con `actor_id` en el arbol.
5. **El dueno es implicito** — no hace falta crear un actor para el dueno a menos que se quiera explicitar.

**Impacto**: 1 tabla nueva (`actors`), 4 columnas nullable (2 en `nodes`, 1 en `assumptions`, 1 en `trees`), 3 comandos nuevos (`actor_new`, `actor_list`, `actor_inspect`), 2 warnings nuevos en validate. Zero breaking changes.

**Riesgo principal**: perdida de foco (B2) y scope creep (B3). Mitigacion: actores son etiquetas ligeras y opcionales; GTs satelite son excepcionales y nunca sugeridos proactivamente.

**Interaccion con secciones previas**: la atribucion de actores es una dimension ortogonal a la proveniencia de arboles (seccion 11) y a la logica de suficiencia/necesidad (seccion 10). Las tres dimensiones se componen: un nodo de la EC viene de un arbol fuente (proveniencia), opera en una logica especifica (suf/nec), y pertenece a un actor (atribucion).

---

## 13. Matriz de Transformacion de Tipos de Nodo entre Arboles

Las 20 transiciones (apendice) describen el nivel MACRO (arbol → arbol). Esta seccion documenta el nivel MICRO: como cada **tipo de nodo** se transforma al cruzar de un arbol a otro. Es el complemento que permite al meta-grafo y al motor saber no solo QUE arboles se conectan, sino QUE tipos de entidad cruzan la frontera y COMO se recontextualizan.

### 13.1. Tipos de nodo por arbol — referencia completa

| Arbol | Logica | Tipos de nodo | Notas |
|-------|--------|---------------|-------|
| **GT** | Necesidad | GOAL, OBJ, CSF, NC | Jerarquia: GOAL → OBJ → CSF → NC. Cada nivel es condicion necesaria del superior. |
| **CRT** | Suficiencia | UDE, INT, RC | UDE = efecto indeseable observable. INT = causa intermedia. RC = causa raiz (CRC si conecta >70% UDEs). |
| **EC** | Necesidad | REQ (roles: A=objective, B/C=requirement), PRE (roles: D/D'=prerequisite) | 5 nodos fijos. D↔D' en conflicto XOR. Ver seccion 13.4 para espacio de fuentes. |
| **FRT** | Suficiencia | DE, INJ | DE = efecto deseable (opuesto a UDE). INJ = inyeccion (nodo raiz que entra del EC). |
| **NBR** | Suficiencia | UDE (colateral), Trimming INJ | UDE = efecto indeseable colateral de la INJ (mismo tipo que en CRT, distinto origen). Trimming INJ = inyeccion de recorte para mitigar la UDE colateral. |
| **PRT** | Necesidad | OBS, IO, INJ (como objetivo) | OBS = obstaculo. IO = objetivo intermedio. INJ validada entra como objetivo a implementar. |
| **TT** | Suficiencia | Nodos de accion tactica | 5 elementos por paso: realidad actual, necesidad, accion, nueva realidad, siguiente necesidad. |

**Entidades transversales** (no pertenecen a un arbol especifico):

| Entidad | Donde vive | Funcion |
|---------|-----------|---------|
| **ASM** (Supuesto) | En edges, vinculado a un edge_id | Cuestiona la logica de un enlace causal. En la EC, cuestiona la cadena comprimida. |
| **KN** (Knowledge item) | Pool independiente, vinculable a cualquier nodo | Evidencia, dato, testimonio, hipotesis. Status epistemico (verified, unverified, refuted). |

### 13.2. La norma como bisagra GT ↔ CRT ↔ FRT

Los CSF y NC del Goal Tree son la **norma** — la definicion de como deberia funcionar el sistema. Esta norma es la bisagra que conecta los tres arboles principales:

```
                         GT (norma)
                        /          \
              [violada]              [cumplida]
                 ↓                       ↓
    UDE derivado (CRT)           DE evidenciado (FRT)
```

| Norma (GT) | Estado | Resultado | Arbol destino | Ejemplo |
|-----------|--------|-----------|--------------|---------|
| NC-003 "Entregas on-time ≥95%" | **Violada** | UDE-003 "El 28% de entregas llegan fuera de plazo" | CRT | La UDE es la manifestacion observable de la violacion |
| NC-003 "Entregas on-time ≥95%" | **Cumplida** | DE-003 "Entregas on-time al 97%" | FRT | El DE es la evidencia observable del cumplimiento |
| CSF-002 "Entregas on-time" | **Violada** | UDE-001 "Indice de satisfaccion del cliente <60%" | CRT | Un CSF violado puede derivar UDEs de nivel mas alto |
| CSF-002 "Entregas on-time" | **Cumplida** | DE-001 "Indice de satisfaccion >90%" | FRT | El DE a nivel CSF es mas estrategico |

**Reglas criticas**:
1. El UDE **no ES** la NC — es su manifestacion negativa observable. "Entregas on-time ≥95%" (NC) ≠ "28% llegan tarde" (UDE).
2. El DE **no ES** la NC — es su manifestacion positiva observable. "Entregas on-time ≥95%" (NC) ≠ "97% on-time" (DE).
3. La relacion no es directa (NC → UDE). Es: NC [violada] → se **deriva** un UDE que captura el efecto observable.
4. Cada UDE del CRT deberia poder trazarse a al menos una NC/CSF violada del GT. Si no puede, el GT esta incompleto (ver T05, CRT→GT feedback).
5. Cada DE del FRT deberia ser el opuesto de un UDE del CRT, y por tanto, evidencia de una NC/CSF cumplida.

### 13.3. Matriz completa de transformaciones cross-tree

#### Flujo canonico (Tier 1: T01, T06, T11, T16)

| Tipo origen | → | Tipo destino | Transicion | Naturaleza | Ejemplo |
|-------------|---|-------------|------------|------------|---------|
| NC/CSF (GT) | → | UDE (CRT) | T01 gap_analysis | **Derivacion por violacion**: la norma no cumplida se manifiesta como efecto indeseable observable | NC-003 "On-time ≥95%" → [violada] → UDE-003 "28% llegan tarde" |
| NC/CSF (GT) | → | DE (FRT) | T01+T11 | **Derivacion por cumplimiento**: la norma cumplida se evidencia como efecto deseable | NC-003 "On-time ≥95%" → [cumplida] → DE-003 "97% on-time" |
| RC/CRC (CRT) | → | D o D' (EC) | T06 core_conflict | **Recontextualizacion**: causa raiz en logica de suficiencia se convierte en prerequisito en logica de necesidad | RC-001 "Entregas sin tracking" → D "Usar entregas manuales" (prerequisite) |
| INT (CRT) | → | D o D' (EC) | T06 core_conflict | **Recontextualizacion**: intermedio cercano al conflicto operativo se convierte en prerequisito | INT-005 "Agrupacion estatica de envios" → D "Mantener agrupacion estatica" |
| GOAL/OBJ/CSF (GT) | → | A (EC) | T06 via proyeccion | **Compresion**: nodo de alto nivel del GT se comprime en el objetivo comun de la EC | GOAL-001 "Logistica de clase mundial" → A (objective) |
| CSF/NC (GT) | → | B, C (EC) | T06 via proyeccion | **Compresion**: necesidades del GT se comprimen en las dos necesidades en tension de la EC | CSF-002 "On-time", NC-003 "Control costes" → B, C (requirements) |
| INJ (EC) | → | INJ (FRT) | T11 injection | **Transferencia directa**: la inyeccion entra al FRT como nodo raiz para simulacion | INJ-001 "Optimizacion dinamica de rutas" → INJ raiz del FRT |
| UDE (CRT) | → | DE (FRT) | T11 implicito | **Inversion**: cada UDE del CRT debe tener un DE opuesto en el FRT que demuestre su eliminacion | UDE-003 "28% tarde" → DE-003 "97% on-time" |
| INJ validada (FRT) | → | Objective (PRT) | T16 prerequisite | **Promocion**: inyeccion inmunizada contra NBRs se convierte en objetivo a implementar | INJ-001 validada → objetivo del PRT "Implementar optimizacion" |
| OBS (PRT) | → | IO (PRT) | interno | **Conversion**: obstaculo se transforma en objetivo intermedio que lo supera | OBS "No hay proveedor" → IO "Seleccionar proveedor" |
| IO (PRT) | → | Accion (TT) | T16 tactical | **Descomposicion**: objetivo intermedio se descompone en pasos tacticos ejecutables | IO "Seleccionar proveedor" → Paso 1: "Solicitar propuestas a 3 proveedores" |
| UDE colateral (NBR) | → | Trimming INJ (NBR) | interno | **Mitigacion**: efecto negativo colateral genera inyeccion de recorte | UDE "Dependencia de proveedor unico" → Trim INJ "Clausula multi-vendor" |

#### Loops correctivos (Tier 2: T05, T09, T10, T14, T15, T18, T20)

| Tipo origen | → | Tipo destino | Transicion | Naturaleza | Ejemplo |
|-------------|---|-------------|------------|------------|---------|
| UDE (CRT) sin ancla | → | NC/CSF nueva (GT) | T05 CRT→GT feedback | **Revelacion**: UDE que no viola ningun CSF/NC revela gap en la norma | UDE-009 "Conductores con fatiga cronica" → nuevo CSF "Bienestar operativo" |
| CRC (CRT) contraproducente | → | NC revisada (GT) | T05 CRT→GT feedback | **Correccion**: CRC revela que una NC del GT es parte del problema | RC que apunta a NC-005 → NC-005 se revisa o elimina |
| Supuesto invalidado (EC) | → | NC/CSF nueva (GT) | T09 EC→GT | **Insight**: supuesto cuestionado revela creencia implicita que permeaba el GT | ASM "Competitivo = barato" invalidado → CSF revisado "Fiabilidad, no coste" |
| D/D' (EC) sin conexion | → | INT/RC nuevos (CRT) | T10 EC→CRT | **Descubrimiento**: la EC revela causas no capturadas en el CRT | D "Agrupar envios" → CRT necesita cadena causal de penalizaciones |
| UDE nueva (FRT) | → | UDE (CRT) | T14 FRT→CRT revision | **Descubrimiento**: la simulacion revela problemas del presente no diagnosticados | FRT revela "conductores desconfian de algoritmos" → UDE-012 en CRT |
| DE no alcanzado (FRT) | → | NC revisada (GT) | T13 FRT→GT | **Ajuste**: NC inalcanzable incluso con la mejor INJ requiere revision de la meta | FRT muestra max 85% on-time → NC-003 se ajusta a ≥85% |
| DE emergente (FRT) | → | CSF nuevo (GT) | T13 FRT→GT | **Oportunidad**: DE inesperado merece convertirse en CSF | DE "Tracking live para clientes" → nuevo CSF "Revenue por servicios premium" |
| UDE colateral grave (NBR) | → | D/D' (EC nueva) | T15 FRT→EC challenges | **Escalacion**: NBR no recortable obliga a buscar INJ alternativa | UDE colateral catastrofica → nueva EC con otro supuesto a invalidar |
| OBS (PRT) | → | UDE (CRT) | T18 PRT/TT→CRT | **Descubrimiento**: obstaculo de implementacion revela problema no diagnosticado | OBS "Sistema sin API" → UDE nueva "Legacy sin integracion" |
| IO (PRT) conflictivo | → | D/D' (EC nueva) | T19 PRT/TT→EC | **Conflicto tactico**: dos IOs mutuamente excluyentes generan EC de implementacion | IO "Exclusividad proveedor" vs IO "Multi-vendor" → EC tactica |
| Restriccion (PRT/TT) | → | cadena FRT | T20 PRT/TT→FRT | **Retroalimentacion**: implementacion revela que el FRT era incompleto | OBS "Sin API" → FRT necesita cadena "Si migramos a sistema con API..." |

#### Patrones avanzados (Tier 3-4: T02, T03, T04, T07, T08, T12)

| Tipo origen | → | Tipo destino | Transicion | Naturaleza | Ejemplo |
|-------------|---|-------------|------------|------------|---------|
| CSF/NC en tension (GT) | → | B/C (EC directa) | T02 GT→EC | **Conflicto estructural**: la norma misma tiene NCs incompatibles | CSF-1 "Crecimiento agresivo" vs CSF-2 "Soporte excepcional" → EC directa |
| NC (GT) | → | DE (FRT benchmark) | T03 GT→FRT | **Simulacion proactiva**: evaluar iniciativa estrategica contra la norma sin diagnostico previo | NC-003 + propuesta cloud → FRT de benchmark |
| NC (GT) | → | OBS (PRT greenfield) | T04 GT→PRT/TT | **Greenfield**: NCs se convierten directamente en objetivos cuyas obstaculos se mapean | NC-003 "Tracking GPS" → OBS "No hay proveedor" → IO → TT |
| RC (CRT) | → | INJ (FRT directo) | T07 CRT→FRT | **Solucion obvia**: causa raiz con solucion conocida salta la EC | RC "Sin tracking" + regulacion obliga GPS → INJ directa al FRT |
| UDE critica (CRT) | → | Accion (TT emergencia) | T08 CRT→PRT/TT | **Emergencia**: UDE de riesgo inmediato requiere accion antes del analisis completo | UDE "Valvulas fallan sin aviso" → TT de emergencia: inspeccion inmediata |
| INJ menor (EC) | → | Accion (TT quick win) | T12 EC→PRT/TT | **Quick win**: INJ de bajo riesgo se implementa directamente | INJ "Eliminar aprobacion manual <5kg" → TT directo |

### 13.4. Tipos de nodo de la EC: espacio completo de fuentes

La EC tiene 5 nodos con roles fijos, pero los tipos de nodo fuente admiten multiples combinaciones:

#### Combinaciones de A, B, C (lado de necesidad — GT)

| A (objective) | B (requirement) | C (requirement) | Nivel del conflicto | Ejemplo |
|---------------|----------------|----------------|--------------------| --------|
| GOAL | CSF-1 | CSF-2 | **Estrategico**: conflicto entre factores criticos de exito | A="Logistica competitiva", B="Minimizar coste", C="Maximizar servicio" |
| CSF | NC-1 | NC-2 | **Tactico**: conflicto entre condiciones necesarias del mismo CSF | A="Entregas on-time", B="Rutas eficientes", C="Flexibilidad ante imprevistos" |
| OBJ | CSF | NC | **Mixto**: conflicto entre niveles jerarquicos del GT | A="Excelencia operativa", B="KPI de coste" (CSF), C="Formacion continua" (NC) |
| CSF | CSF (de otro OBJ) | NC | **Cross-branch**: conflicto entre ramas distintas del GT | A=OBJ comun, B=CSF de rama comercial, C=NC de rama operativa |

#### Combinaciones de D, D' (lado de prerequisitos — CRT/GT/nuevo)

| D | D' | Fuente D | Fuente D' | Ejemplo |
|---|----|---------|-----------| --------|
| RC del CRT | Nodo nuevo | CRT (suficiencia) | Creado para la EC | D="Entregas manuales" (RC-001), D'="Invertir en GPS" (nuevo) |
| RC del CRT | NC del GT | CRT (suficiencia) | GT (necesidad) | D="Politica de agrupacion" (RC), D'="Cumplir SLA contractual" (NC del GT) |
| INT del CRT | INT del CRT | CRT (suficiencia) | CRT (suficiencia) | D="Subcontratar transporte" (INT), D'="Mantener flota propia" (INT) |
| NC del GT | NC del GT | GT (necesidad) | GT (necesidad) | D="Invertir en tecnologia" (NC), D'="Contener CAPEX" (NC) — conflicto puro de norma |
| Nodo nuevo | Nodo nuevo | Creado | Creado | D y D' no existen en ningun arbol previo — surgen del analisis de la EC |

**Implicacion para el projection_map**: el `source_tree` de cada entrada puede ser GT, CRT, o null (nodo nuevo). La combinacion mas rica (y mas comun en analisis reales) es:
- A del GT, B y C del GT (mezcla CSF/NC), D del CRT, D' nuevo o del GT.

**Implicacion para la logica suf/nec** (seccion 10): cuando D viene del CRT (suficiencia) y entra en la EC (necesidad), hay un cruce logico. Cuando D viene del GT (necesidad) y entra en la EC (necesidad), no hay cruce — pero si hay compresion de niveles.

### 13.5. Resumen

La matriz de transformacion de tipos de nodo aporta tres cosas al meta-grafo y al motor:

1. **Precision en los handoffs**: el `projection_map` y los `handoff_nodes` de las tree_relations ahora saben no solo QUE nodo cruza, sino DE QUE TIPO era en el arbol origen y EN QUE TIPO se convierte en el destino. La naturaleza de la transformacion (derivacion, compresion, recontextualizacion, inversion, etc.) es metadata valiosa.

2. **El concepto de norma-bisagra**: los CSF/NC del GT no se convierten directamente en UDEs o DEs. Se **derivan** por violacion (→ UDE) o por cumplimiento (→ DE). Esto es fundamental para que `validate` verifique que cada UDE del CRT tiene ancla en la norma, y que cada DE del FRT evidencia cumplimiento de la norma.

3. **Espacio de fuentes de la EC**: documentar todas las combinaciones posibles de tipos fuente para los 5 nodos EC permite al motor y al agente generar `projection_map` mas precisos, y al agente generar supuestos mas ricos sabiendo exactamente que tipo de nodo cuestiona cada edge.

**Impacto potencial en el motor**: un campo `transformation_type` en los `handoff_nodes` (derivation, compression, recontextualization, inversion, promotion, decomposition) permitiria a `trace --narrative` explicar la naturaleza del cruce, no solo que nodo cruzo.

## 14. Escenarios como Ramas de Futuros Alternativos

### Contexto

La seccion 3.3 definio escenarios como entidades con `branch_node`, `parent_scenario` y `scenario_trees`. Pero el modelo original asumia implicitamente un flujo relativamente lineal: una EC produce una INJ, esa INJ genera un FRT, y se avanza. La practica LTP real es mas compleja:

1. Una EC puede producir **multiples INJs** (invalidando supuestos distintos). Cada INJ genera su propia rama de futuro completa.
2. Un CRT puede producir **multiples ECs** (de diferentes CRCs o conflictos a diferentes niveles).
3. Los NBRs de un FRT pueden requerir **trimming INJs** que a su vez generan nuevos conflictos — nuevas ECs con nuevas INJs.
4. El proceso es **recursivo**: EC → INJ → FRT → NBR → nueva EC → nueva INJ → nuevo FRT → ...

Esto crea un arbol de alternativas que el modelo de escenarios debe capturar:

```
GT ─── CRT ─── EC₁ ──┬── SCN-001: INJ₁ₐ → FRT₁ₐ → NBR → PRT₁ₐ → TT₁ₐ
                      ├── SCN-002: INJ₁ᵦ → FRT₁ᵦ → NBR → PRT₁ᵦ → TT₁ᵦ
                      └── SCN-003: INJ₁꜀ → FRT₁꜀ → NBR₂ → EC₂ → INJ₂ → FRT₁꜀' → ...
       ─── EC₃ ──┬── SCN-004: INJ₃ₐ → FRT₃ₐ → ...
                  └── SCN-005: INJ₃ᵦ → FRT₃ᵦ → ...
```

Esta seccion analiza como modelar esta realidad y sus implicaciones para el meta-grafo.

### 14.1. Hechos: la ramificacion en la practica (Sombrero Blanco)

#### Cuantas INJs alternativas produce una EC tipica

| Metrica | Valor tipico | Fuente |
|---------|-------------|--------|
| Supuestos generados por EC (protocolo Fedurko) | 10-25 | 5 edges × 2-5 supuestos por edge |
| Supuestos invalidables (fragiles, no triviales) | 3-7 | Filtro: 7 errores fatales + ranking por fragilidad |
| INJs concretas derivadas de supuestos invalidados | 2-5 | Un supuesto invalidado puede generar 1-2 INJs |
| INJs que merecen FRT completo | 1-3 | Las demas se descartan por evaluacion rapida |

En la practica, el analista **rankea** los supuestos por fragilidad y trabaja el mas fragil primero. Solo si ese camino falla (NBR catastrofico) se explora el segundo. No se construyen 5 FRTs en paralelo.

#### Cuantos escenarios se exploran realmente

| Nivel de analisis | Escenarios explorados | Escenarios con FRT completo | Escenarios con PRT/TT |
|-------------------|----------------------|---------------------------|----------------------|
| Analisis rapido | 1-2 | 1 | 1 |
| Analisis estandar | 2-4 | 1-2 | 1 |
| Analisis profundo | 3-6 | 2-3 | 1-2 |
| Analisis exhaustivo (raro) | 5-10 | 3-5 | 2-3 |

**Hallazgo**: la mayoria de los analisis LTP trabajan 1-2 escenarios a fondo y esbozan 2-3 mas como referencia. La exhaustividad total es rara y costosa.

#### Que arboles son compartidos y cuales divergen

| Arbol | Compartido entre escenarios? | Por que |
|-------|------------------------------|---------|
| GT | **Si** — siempre | La meta no cambia por elegir una INJ u otra |
| CRT | **Si** — casi siempre | La realidad actual es la misma para todos los escenarios. Excepcion: si un loop correctivo (T14, FRT→CRT) descubre UDEs especificas de un escenario |
| EC | **Si** — normalmente | El conflicto es el mismo. Las INJs cambian, no el conflicto. Excepcion: si una EC hija emerge de un NBR, es especifica del escenario que genero el NBR |
| FRT | **No** — diverge por escenario | Cada INJ produce un futuro diferente |
| NBR | **No** — diverge por escenario | Cada FRT tiene sus propios riesgos |
| PRT | **No** — diverge por escenario | Cada INJ tiene obstaculos diferentes |
| TT | **No** — diverge por escenario | Cada PRT genera acciones diferentes |

**Caso especial — ECs hijas**: cuando un NBR de un FRT genera un conflicto nuevo (T15/T19), esa EC hija es **especifica del escenario** que la genero. No es compartida. Esto rompe la regla "las ECs son compartidas":

```
EC₁ (compartida) → INJ₁ₐ → FRT₁ₐ → NBR₂ → EC₂ (especifica de SCN-001)
                 → INJ₁ᵦ → FRT₁ᵦ → (sin NBR grave, no genera EC hija)
```

#### La recursion en la practica

| Nivel de recursion | Que pasa | Frecuencia |
|-------------------|---------|------------|
| **Nivel 0** | EC → INJ → FRT → OK | Caso ideal. INJ funciona, NBRs son menores. ~40% de analisis |
| **Nivel 1** | EC → INJ → FRT → NBR grave → Trimming INJ → OK | El NBR se recorta sin conflicto nuevo. ~35% |
| **Nivel 2** | EC → INJ → FRT → NBR → Trimming INJ → nuevo conflicto → EC₂ → INJ₂ → FRT' | La trimming INJ genera su propio conflicto. ~20% |
| **Nivel 3+** | Recursion de nivel 2 repetida | Raro (<5%). Si ocurre, indica problema mas profundo: la meta (GT) o el diagnostico (CRT) necesitan revision |

**Regla practica de Dettmer**: si llegas a nivel 3 de recursion, es senal de que el sistema tiene un conflicto estructural profundo que las INJs individuales no pueden resolver. Se recomienda volver al GT (T13/T17).

#### Como funciona hoy sin modelo de escenarios

1. El analista crea FRT-1 para INJ₁ₐ y FRT-2 para INJ₁ᵦ.
2. Ambos FRTs son arboles sueltos sin relacion formal.
3. No queda registro de que ambos vienen de la misma EC.
4. No hay forma de comparar: que UDEs resuelve cada uno, que NBRs genera, que coste de PRT tiene.
5. Si un NBR genera una EC hija, esta EC es otro arbol suelto sin vinculo al NBR que la genero.
6. La decision "elegimos INJ₁ₐ sobre INJ₁ᵦ" no se captura. Se pierde la historia de la deliberacion.

### 14.2. Intuicion (Sombrero Rojo)

La tension fundamental es entre **exploracion** y **avance**:

- La metodologia LTP dice: invalida el supuesto mas fragil, construye el FRT, verifica. Si no funciona, vuelve a la EC y busca otro. Es un proceso **iterativo-secuencial**, no paralelo.
- Pero la buena toma de decisiones dice: antes de comprometerte con un camino, conoce las alternativas. Es dificil elegir INJ₁ₐ con conviccion si no sabes que INJ₁ᵦ existe y que ofrece.

El modelo de escenarios resuelve esto: puedes **esbozar** las alternativas rapidamente (FRT parcial, evaluacion de NBRs de alto nivel) antes de comprometerte con la que vas a desarrollar a fondo (FRT completo + PRT + TT).

Hay un riesgo emocional: los escenarios pueden convertirse en una excusa para no decidir. "Necesitamos explorar un escenario mas" es la version LTP de "necesitamos mas datos antes de actuar". El modelo debe facilitar la decision, no aplazarla.

La metafora correcta no es "explorar todos los caminos" sino **"scout antes de marchar"**: reconocimiento rapido de las alternativas, avance decisivo por una, con capacidad de pivotar si la elegida falla.

### 14.3. Riesgos (Sombrero Negro — Obligatorio)

| # | Riesgo | Severidad | Detalle | Mitigacion |
|---|--------|-----------|---------|------------|
| B1 | **Explosion de arboles** — N escenarios × M arboles por escenario | ALTA | Con 3 INJs alternativas, cada una con FRT + NBR + PRT + TT = 12 arboles adicionales ademas de los compartidos. En un analisis profundo con 2 ECs y 3 INJs cada una: 24 arboles divergentes + 4-5 compartidos = ~30 arboles en un workspace. | Escenarios parciales: no todo escenario necesita PRT/TT. El FRT + evaluacion de NBRs de alto nivel es suficiente para descartar alternativas. Solo el escenario elegido avanza a PRT/TT. |
| B2 | **Cascada de invalidacion** — si el CRT cambia, todos los FRTs de todos los escenarios pueden quedar obsoletos | ALTA | Un loop correctivo (T14) que modifica el CRT impacta el diagnostico base. Si la CRC cambia, la EC cambia, y todas las INJs y FRTs derivados se invalidan en cascada. | El motor emite `SCENARIO_BASE_CHANGED` cuando un arbol compartido (GT, CRT, EC) se modifica. Los escenarios se marcan `needs_review` automaticamente, no se eliminan. El analista decide cuales siguen siendo validos. |
| B3 | **Complejidad cognitiva** — el analista tiene que trackear multiples futuros simultaneamente | MEDIA-ALTA | Mantener en la cabeza "en el escenario A pasa X, en el B pasa Y, en el C pasa Z" es cognitivamente costoso. Sin herramientas de visualizacion/comparacion, los escenarios confunden mas de lo que aclaran. | La UI y el agente son los que gestionan la complejidad, no el humano. `scenario_compare` genera una tabla comparativa. El analista ve resultados, no estructura. |
| B4 | **Project management disfrazado** — los escenarios pueden derivar en gestion de proyecto (timelines, responsables, presupuestos) en vez de thinking process | MEDIA | Si cada escenario tiene PRT + TT con fechas, responsables y costes, el workspace se parece mas a un MS Project que a un analisis causal. El LTP es para pensar, no para gestionar. | Los escenarios son para **deliberar** (elegir la mejor INJ), no para **planificar** (gestionar la implementacion). Solo el escenario elegido (status=`selected`) avanza a PRT/TT detallado. Los demas se quedan a nivel FRT + NBR de alto nivel. |
| B5 | **Falsa exhaustividad** — "hemos explorado 5 escenarios" suena exhaustivo pero puede omitir el mejor | MEDIA | Si los 5 escenarios vienen de la misma EC (invalidando 5 supuestos), pero hay otra EC no construida que tiene una INJ superior, los 5 escenarios son variaciones del mismo enfoque. La exhaustividad real requiere explorar conflictos alternativos, no solo supuestos alternativos. | Distinguir entre ramificacion dentro de una EC (supuestos alternativos) y ramificacion entre ECs (conflictos alternativos). `system_status` puede advertir: "2 ECs con 5 escenarios, pero CRC alternativas del CRT no exploradas". |
| B6 | **Recursion infinita** — EC → FRT → NBR → EC₂ → FRT' → NBR' → EC₃ → ... | MEDIA | Cada ciclo genera mas arboles, mas escenarios, mas complejidad. En teoria puede ser infinito. En la practica deberia converger (cada EC sucesiva es de menor impacto), pero sin mecanismo de parada, un agente LLM podria iterar indefinidamente. | Mecanismo de convergencia: (1) profundidad maxima configurable (default: 3 niveles de recursion); (2) el motor emite `RECURSION_DEPTH_WARNING` al superar el umbral; (3) cada nivel sucesivo debe resolver NBRs de menor severidad — si la severidad no decrece, volver al GT. |
| B7 | **Escenarios zombi** — escenarios descartados que quedan en el workspace sin limpieza | BAJA-MEDIA | Un escenario descartado (status=`discarded`) sigue teniendo arboles, nodos, edges. Si el workspace acumula 10 escenarios descartados, el ruido dificulta la navegacion. | `scenario_archive` para mover escenarios descartados a un estado de solo lectura. Los nodos compartidos con escenarios activos no se eliminan. `system_status` muestra ratio activos/descartados. |
| B8 | **Convergencia de escenarios** — dos INJs que inicialmente parecen alternativas resultan ser complementarias | BAJA | INJ₁ₐ "Optimizacion de rutas" y INJ₁ᵦ "Formacion de conductores" no son mutuamente excluyentes — se pueden combinar. Si el modelo solo permite bifurcacion (OR), no captura la combinacion (AND). | Soportar un tipo de relacion `merge` entre escenarios: SCN-003 = SCN-001 + SCN-002. El FRT de SCN-003 combina ambas INJs. Esto es distinto de `alternative` — es composicion, no eleccion. |

**Valoracion global**: B1 (explosion de arboles) y B2 (cascada de invalidacion) son los riesgos dominantes. Ambos se mitigan con el principio de **escenarios parciales**: solo el escenario elegido se desarrolla a fondo. Los demas son esbozos para deliberar.

### 14.4. Beneficios (Sombrero Amarillo)

#### Para el analisis

1. **Deliberacion explicita**: la decision "elegimos INJ₁ₐ sobre INJ₁ᵦ" se documenta con evidencia: que UDEs resuelve cada una, que NBRs genera, que coste de PRT tiene. No es una intuicion — es una comparacion fundamentada.

2. **Pivote con red de seguridad**: si la INJ elegida falla durante la implementacion (PRT/TT revela obstaculos insuperables), el escenario alternativo ya esta esbozado. No hay que empezar de cero — se retoma el FRT alternativo y se profundiza.

3. **Recursion controlada**: el modelo de escenarios hace visible la profundidad de recursion: "estamos en nivel 2, la trimming INJ genero una EC hija". Sin esta visibilidad, la recursion es invisible y el analista puede perderse.

#### Para la toma de decisiones

4. **Comparacion estructurada**: `scenario_compare` puede generar una tabla:

```
| Criterio              | SCN-001 (GPS)     | SCN-002 (Partner) |
|-----------------------|-------------------|-------------------|
| UDEs resueltas        | 5/6               | 4/6               |
| NBRs residuales       | 1 (bajo)          | 2 (medio, bajo)   |
| Coste PRT estimado    | Alto (infraestr.)  | Medio (contrato)  |
| Impacto Actor-Cliente | Alto positivo      | Medio positivo     |
| Impacto Actor-Proveed.| Neutral            | Negativo (depend.) |
| Tiempo impl. (PRT)    | 6 meses            | 3 meses            |
```

5. **Decision auditable**: la eleccion del escenario queda registrada. Meses despues, si se cuestiona la decision, el workspace tiene la deliberacion completa.

#### Para el agente LLM

6. **Recomendacion fundamentada**: el agente puede evaluar todos los escenarios y recomendar: "SCN-001 resuelve mas UDEs, pero SCN-002 es mas rapido de implementar y tiene menor riesgo. Dada la urgencia del cliente, recomiendo SCN-002."

7. **Exploracion paralela eficiente**: un agente headless puede construir FRTs parciales para todas las INJs simultaneamente, evaluar NBRs de alto nivel, y presentar la comparacion antes de que el humano decida cual profundizar.

#### Para la UI

8. **Vista de ramas**: la UI puede mostrar un diagrama de arbol donde cada rama es un escenario, con indicadores visuales de: completitud (FRT solo vs FRT+PRT+TT), status (active/selected/discarded), y metricas clave (UDEs resueltas, NBRs).

9. **Drill-down por escenario**: clic en una rama despliega los arboles de ese escenario. Los arboles compartidos (GT, CRT, EC) se muestran como base comun.

#### Para el meta-grafo

10. **Los escenarios son el mecanismo natural de branching**: las tree_relations de tipo `injection` y `alternative` crean el grafo de decisiones. Los escenarios agrupan las ramas para navegabilidad.

11. **Temporal frames cobran sentido**: `future` y `transition` no son genericos — son especificos de un escenario. "El futuro segun SCN-001" vs "el futuro segun SCN-002".

### 14.5. Alternativas de diseno (Sombrero Verde)

#### Enfoque A — Escenarios como sub-workspaces

Cada escenario tiene su propio espacio aislado. Los arboles compartidos se copian (snapshot) al crear el escenario.

```json
{
  "id": "SCN-001",
  "name": "Escenario GPS Tracking",
  "workspace_path": "scenarios/scn-001/",
  "snapshot_base": {
    "gt": "tree-gt-logistica@v3",
    "crt": "tree-crt-logistica@v5",
    "ec": "tree-ec-conflicto@v2"
  },
  "own_trees": ["tree-frt-gps", "tree-nbr-gps", "tree-prt-gps"],
  "status": "active"
}
```

| Ventaja | Desventaja |
|---------|-----------|
| Aislamiento total: cambios en un escenario no afectan a otros | Duplicacion masiva: los arboles compartidos se copian N veces |
| Facil de archivar/eliminar: borrar el directorio | Los nodos compartidos divergen: si editas UDE-003 en SCN-001, no se refleja en SCN-002 |
| Cada escenario es autocontenido | Comparacion entre escenarios requiere join entre sub-workspaces |
| Compatible con branch de git (un branch por escenario) | Overhead de storage y sincronizacion |

#### Enfoque B — Escenarios como etiqueta ligera (tag)

Los arboles tienen un campo `scenario_id` nullable. Los arboles compartidos tienen `scenario_id = null`. Los que divergen apuntan a su escenario.

```json
// Arbol compartido (sin escenario)
{
  "id": "tree-crt-logistica",
  "scenario_id": null,
  "temporal_frame": "present"
}

// Arbol de escenario
{
  "id": "tree-frt-gps",
  "scenario_id": "SCN-001",
  "temporal_frame": "future"
}
```

**SQL**:
```sql
-- Sin tabla scenario_trees. Solo columna en trees.
ALTER TABLE trees ADD COLUMN scenario_id TEXT REFERENCES scenarios(id);
-- null = compartido, no-null = especifico del escenario
```

| Ventaja | Desventaja |
|---------|-----------|
| Minimalista: un campo, sin tablas extra | No captura el punto de bifurcacion ni la jerarquia de escenarios |
| Sin duplicacion: arboles compartidos son unicos | Si un arbol compartido cambia, no hay mecanismo de notificacion |
| Queries simples: `WHERE scenario_id = 'SCN-001' OR scenario_id IS NULL` | No soporta escenarios anidados (recursion EC→NBR→EC₂) |
| Compatible con el schema existente | La comparacion de escenarios es manual |

#### Enfoque C — Escenarios implicitos via tree_relations

No existe entidad "escenario". Los FRTs alternativos se conectan a la EC con relaciones `injection`, y entre si con relaciones `alternative`. El escenario emerge de la topologia del grafo.

```json
// Relaciones que implican dos escenarios:
{"id": "TREL-010", "from_tree": "tree-ec-1", "to_tree": "tree-frt-gps", "relation_type": "injection"}
{"id": "TREL-011", "from_tree": "tree-ec-1", "to_tree": "tree-frt-partner", "relation_type": "injection"}
{"id": "TREL-012", "from_tree": "tree-frt-gps", "to_tree": "tree-frt-partner", "relation_type": "alternative"}
```

| Ventaja | Desventaja |
|---------|-----------|
| Zero entidades nuevas: todo se modela con tree_relations | No hay nombre ni descripcion del escenario — es anonimo |
| Maxima flexibilidad: cualquier grafo de relaciones es valido | No soporta status (active/discarded/selected) |
| Compatible con toda la maquinaria de tree_relations existente | No soporta comparacion estructurada |
| Las tree_relations ya capturan la topologia | La recursion (EC hija) se modela pero no se visualiza como jerarquia |

#### Enfoque D — Hibrido: escenario ligero con contexto de bifurcacion

Escenario como entidad lightweight con **informacion de bifurcacion enriquecida**. Los arboles se vinculan via `scenario_trees`. Los compartidos no pertenecen a ningun escenario.

```json
{
  "id": "SCN-001",
  "system_id": "SYS-001",
  "name": "GPS Tracking propio",
  "description": "Futuro donde implementamos tracking GPS con flota propia",
  "branch_point": {
    "ec_id": "tree-ec-conflicto",
    "assumption_id": "ASM-007",
    "injection_id": "INJ-005"
  },
  "parent_scenario": null,
  "depth": 0,
  "status": "active",
  "trees": ["tree-frt-gps", "tree-nbr-gps-costes", "tree-prt-gps", "tree-tt-gps-q1"]
}

// Escenario hijo (recursion: NBR genero nueva EC)
{
  "id": "SCN-003",
  "system_id": "SYS-001",
  "name": "GPS + Descentralizado",
  "description": "Variante de SCN-001 con optimizacion descentralizada tras NBR catastrofico",
  "branch_point": {
    "ec_id": "tree-ec-centralizacion",
    "assumption_id": "ASM-019",
    "injection_id": "INJ-012"
  },
  "parent_scenario": "SCN-001",
  "depth": 1,
  "status": "active",
  "trees": ["tree-frt-gps-descentralizado", "tree-nbr-gps-desc", "tree-prt-gps-desc"]
}
```

**SQL**:
```sql
CREATE TABLE scenarios (
    id              TEXT PRIMARY KEY,
    system_id       TEXT REFERENCES systems(id),
    name            TEXT NOT NULL,
    description     TEXT,
    branch_point    TEXT,          -- JSON: {ec_id, assumption_id, injection_id}
    parent_scenario TEXT REFERENCES scenarios(id),
    depth           INTEGER DEFAULT 0,
    status          TEXT DEFAULT 'active'
);

CREATE TABLE scenario_trees (
    scenario_id TEXT REFERENCES scenarios(id),
    tree_id     TEXT REFERENCES trees(id),
    PRIMARY KEY (scenario_id, tree_id)
);
```

**Escenarios combinados (convergencia)**:

```json
{
  "id": "SCN-004",
  "name": "GPS + Formacion (combinado)",
  "description": "Combinacion de INJ₁ₐ (GPS) + INJ₁ᵦ (Formacion)",
  "branch_point": {
    "ec_id": "tree-ec-conflicto",
    "assumption_ids": ["ASM-007", "ASM-011"],
    "injection_ids": ["INJ-005", "INJ-008"]
  },
  "parent_scenarios": ["SCN-001", "SCN-002"],
  "depth": 0,
  "status": "active"
}
```

| Ventaja | Desventaja |
|---------|-----------|
| `branch_point` captura exactamente de donde nace: EC + supuesto + INJ | Schema mas complejo que B o C |
| `parent_scenario` + `depth` modelan la recursion naturalmente | `parent_scenarios` (plural) para convergencia rompe la referencia simple |
| Status (active/discarded/selected) permite gestion del ciclo de deliberacion | Hay que decidir: `parent_scenario` singular (simple, sin convergencia) o plural (complejo, con convergencia) |
| Comparacion estructurada via `scenario_compare` | El `branch_point` es JSON complejo — dificil de validar en el motor |
| Compatible con escenarios parciales (FRT solo, sin PRT/TT) | |

#### Recomendacion

**Enfoque D** con las siguientes decisiones:

1. **`branch_point` es un objeto con 3 campos**: `ec_id` (la EC de donde nace), `assumption_id` (el supuesto invalidado), `injection_id` (la INJ resultante). Esto captura el "por que" del escenario a nivel atomico — no solo "eligimos INJ-005" sino "porque invalidamos ASM-007 en EC-1".

2. **`parent_scenario` singular** (no plural). La convergencia se modela con un `relation_type: "merge"` entre escenarios en vez de con parent multiple. Esto mantiene la jerarquia como arbol (no DAG) lo que simplifica navegacion y validacion.

3. **`depth` explicito** para mecanismo de convergencia. El motor emite `RECURSION_DEPTH_WARNING` cuando `depth >= 3`.

4. **Status con ciclo de vida claro**: `active` → `selected` o `discarded`. Solo un escenario por EC puede tener status `selected` (no es un constraint del motor sino un warning). Un escenario `discarded` pasa a solo lectura.

5. **Escenarios parciales validos**: un escenario puede tener solo un FRT (sin PRT/TT). El `system_status` muestra completitud por escenario: "SCN-001: FRT ✓, NBR ✓, PRT ✓, TT ✓" vs "SCN-002: FRT ✓, NBR ✓, PRT -, TT -".

6. **Revision de la seccion 3.3**: el schema original tenia `branch_node` (String → Node.id). Se reemplaza por `branch_point` (Object) para capturar EC + supuesto + INJ. Esto es un **cambio de schema** respecto a la seccion 3.3.

### 14.6. Coherencia arquitectonica (Sombrero Azul)

#### Revision de la seccion 3.3 (schema original de escenarios)

| Campo original | Cambio | Razon |
|---------------|--------|-------|
| `branch_node` (String → Node.id) | **Reemplazado** por `branch_point` (Object: {ec_id, assumption_id, injection_id}) | `branch_node` no capturaba el contexto completo de la bifurcacion |
| `parent_scenario` | **Mantiene** | Sigue siendo singular — la convergencia va por `merge` relation |
| `status` | **Mantiene** con ciclo explicitado | `active` → `selected` / `discarded` |
| — | **Nuevo**: `depth` (Integer) | Mecanismo de control de recursion |
| `trees` (array en JSON) / `scenario_trees` (tabla SQL) | **Mantiene** | Los arboles que divergen por escenario |

#### Interaccion con secciones previas

**Seccion 10 (logica suf/nec)**: los escenarios no cambian la logica de transicion — cada arbol dentro del escenario sigue teniendo su logica. Pero la comparacion de escenarios debe ser consciente de la logica: comparar metricas de suficiencia (AND gates, weights) entre FRTs es valido; comparar metricas de suficiencia del FRT con metricas de necesidad del PRT dentro del mismo escenario requiere conversion.

**Seccion 11 (EC como proyeccion)**: el `branch_point` referencia la EC y el supuesto. El `projection_map` de la tree_relation EC→FRT incluye los nodos que cruzan. Combinados: "SCN-001 nace porque en EC₁ (que proyecta GT+CRT) se invalido ASM-007 en el edge D→B (que comprime la cadena RC-001→INT-003→UDE-002) y la INJ-005 interviene en el eslabon RC-001→INT-003 del CRT."

**Seccion 12 (actores)**: la comparacion de escenarios puede incluir impacto por actor. "SCN-001 es mejor para el cliente pero peor para el proveedor." El campo `actor_id` en los nodos del FRT permite agregar por actor.

**Seccion 13 (tipos de nodo)**: las transformaciones cross-tree dentro de un escenario siguen las mismas reglas que el flujo general. INJ→FRT, UDE→DE, etc. La diferencia es que cada escenario tiene su propia instancia de estas transformaciones.

#### Criterios de comparacion de escenarios

`scenario_compare` deberia evaluar como minimo:

| Criterio | Como se calcula | Fuente |
|----------|----------------|--------|
| UDEs resueltas | Contar DEs del FRT que son opuestos a UDEs del CRT | FRT + CRT |
| UDEs no resueltas | UDEs del CRT sin DE opuesto en el FRT | FRT + CRT |
| NBRs residuales | UDEs colaterales del NBR sin trimming INJ efectiva | NBR |
| Severidad de NBRs | Suma ponderada de severidad de UDEs colaterales residuales | NBR |
| Complejidad de PRT | Numero de OBS e IOs | PRT |
| Impacto por actor | UDEs resueltas/generadas que afectan a cada actor | FRT + actores |
| Profundidad de recursion | `depth` del escenario | Escenario |
| Completitud | Que arboles tiene (FRT, NBR, PRT, TT) | scenario_trees |

#### Impacto en comandos

| Comando | Cambio |
|---------|--------|
| `scenario_new` | Acepta `--branch-point` con EC, supuesto e INJ. Calcula `depth` automaticamente de `parent_scenario`. |
| `scenario_compare` | **Enriquecido**: genera tabla comparativa con criterios configurables. Acepta `--by-actor` para segmentar por actor. |
| `scenario_select` | **Nuevo**: marca un escenario como `selected`. Emite warning si otro ya estaba seleccionado para la misma EC. |
| `scenario_discard` | **Nuevo**: marca como `discarded`. Los arboles pasan a solo lectura. |
| `scenario_merge` | **Nuevo**: crea escenario combinado con `parent_scenarios` implicitos via tree_relation `merge`. |
| `validate` | Nuevos warnings: `SCENARIO_BASE_CHANGED` (arbol compartido modificado), `RECURSION_DEPTH_WARNING` (depth ≥ 3), `MULTIPLE_SELECTED_SCENARIOS` (mas de un selected por EC). |
| `system_status` | Seccion de escenarios: activos/descartados/seleccionados, completitud por escenario, profundidad maxima de recursion. |
| `trace --narrative` | Anota el escenario al cruzar a un arbol que pertenece a uno: "[Entrando en SCN-001: GPS Tracking]". |

#### Impacto en el schema

**Cambio en `scenarios`** (respecto a seccion 3.3):

```sql
-- Antes (seccion 3.3):
CREATE TABLE scenarios (
    id              TEXT PRIMARY KEY,
    system_id       TEXT REFERENCES systems(id),
    name            TEXT NOT NULL,
    description     TEXT,
    parent_scenario TEXT REFERENCES scenarios(id),
    branch_node     TEXT REFERENCES nodes(id),    -- ELIMINADO
    status          TEXT DEFAULT 'active'
);

-- Ahora (seccion 14):
CREATE TABLE scenarios (
    id              TEXT PRIMARY KEY,
    system_id       TEXT REFERENCES systems(id),
    name            TEXT NOT NULL,
    description     TEXT,
    branch_point    TEXT,                          -- JSON: {ec_id, assumption_id, injection_id}
    parent_scenario TEXT REFERENCES scenarios(id),
    depth           INTEGER DEFAULT 0,
    status          TEXT DEFAULT 'active'          -- active | selected | discarded
);

-- scenario_trees se mantiene sin cambios
```

**JSON** (ltp-engine): el campo `branch_node` en `scenarios/SCN-001.json` se reemplaza por `branch_point`:

```json
// Antes:
"branch_node": "INJ-005"

// Ahora:
"branch_point": {
  "ec_id": "tree-ec-conflicto",
  "assumption_id": "ASM-007",
  "injection_id": "INJ-005"
}
```

### 14.7. El proceso iterativo: scout-commit-pivot

La practica real del LTP con escenarios sigue un patron de 3 fases:

**Fase 1 — Scout (reconocimiento)**:
- La EC produce N supuestos invalidables.
- Para cada supuesto fragil, se esboza la INJ resultante y se crea un escenario parcial (solo FRT de alto nivel + evaluacion rapida de NBRs).
- Tiempo: horas, no dias.
- Resultado: 2-4 escenarios esbozados con evaluacion preliminar.

**Fase 2 — Commit (compromiso)**:
- Se elige el escenario mas prometedor (via `scenario_compare` o deliberacion).
- Se construye el FRT completo, se evaluan NBRs con rigor, se recortan con trimming INJs.
- Si la recursion genera una EC hija, se crea un escenario hijo y se repite scout-commit.
- Resultado: 1 escenario con FRT completo + NBRs resueltos.

**Fase 3 — Pivot (pivote, si necesario)**:
- Si el escenario elegido falla (NBR catastrofico no recortable, PRT revela obstaculos insuperables), se marca como `discarded`.
- Se retoma el siguiente escenario esbozado de la Fase 1 y se entra en Fase 2 con el.
- El FRT parcial del esbozo se completa, no se reconstruye de cero.
- Resultado: pivote rapido con red de seguridad.

Este patron es natural para el agente LLM headless: la Fase 1 (scout) se puede hacer en paralelo; la Fase 2 (commit) es secuencial; la Fase 3 (pivot) es un fallback.

### 14.8. Resumen ejecutivo

Los escenarios son el mecanismo por el cual el meta-grafo captura la **deliberacion entre futuros alternativos**. Cada EC produce multiples INJs potenciales; cada INJ es un futuro diferente; y la recursion via NBRs crea sub-futuros dentro de cada rama.

**Decisiones de diseno**:

1. **`branch_point`** reemplaza a `branch_node`: captura EC + supuesto + INJ como contexto atomico de la bifurcacion.
2. **`depth`** explicito para control de recursion. Warning a profundidad ≥ 3.
3. **Status con ciclo de vida**: `active` → `selected` / `discarded`.
4. **Escenarios parciales** como norma: solo el escenario elegido avanza a PRT/TT. Los demas se quedan a nivel FRT + NBR.
5. **Convergencia via `merge`**: dos escenarios complementarios se combinan en un tercero.
6. **Scout-commit-pivot** como proceso iterativo recomendado.

**Impacto**: cambio de schema en `scenarios` (branch_node → branch_point, nuevo campo depth), 3 comandos nuevos (scenario_select, scenario_discard, scenario_merge), 3 warnings nuevos en validate, comparacion estructurada de escenarios.

**Riesgo principal**: explosion de arboles (B1). Mitigacion: escenarios parciales — la exploracion es barata (FRT de alto nivel); el compromiso es costoso (PRT + TT). El modelo facilita explorar mucho y comprometerse con poco.

### 14.9. Insights colaborativos pendientes de documentar

Los siguientes insights surgieron en la discusion previa y se recogen aqui para que no se pierdan. Cada uno merece profundizacion futura.

#### 14.9.1. Donde vive el edge de conflicto D↔D'

El edge D↔D' (conflicto) es la **unica relacion genuinamente nueva** de la EC. No es derivada de otro arbol:

| Arbol | Como aparece D↔D' |
|-------|-------------------|
| GT | **Latente**: D y D' son consecuencias de necesidades (B→D, C→D'), pero la exclusion mutua no esta modelada |
| CRT | **Semi-visible**: solo D aparece (como UDE). D' no existe porque es el "deseo no cumplido". La tension es implicita |
| EC | **Explicita**: D↔D' es el edge central. La exclusion mutua se verbaliza |
| FRT | **Resuelta**: la INJ elimina la exclusion. D y D' coexisten (o uno deja de ser necesario) |

**Que desaparece cuando se evapora el conflicto**: no desaparecen D ni D'. Desaparece la **exclusion mutua** — los supuestos que decian "no puedes tener D y D' al mismo tiempo". Las estrategias de evaporacion son:

- **Tiempo**: D ahora, D' despues (o viceversa)
- **Espacio**: D aqui, D' alla
- **Condicion**: D si X, D' si ¬X
- **Elevacion**: subir al nivel de A para encontrar una forma de satisfacer B y C sin requerir D y D' en su forma actual

#### 14.9.2. INJ location en CRT via provenance

Cuando una INJ invalida un supuesto en una flecha del EC (ej: edge B→D), y esa flecha comprime una cadena del CRT (ej: RC-001→INT-003→UDE-002 via el projection_map de la seccion 11), la INJ se puede localizar en el CRT:

1. El `projection_map` de la tree_relation EC→CRT dice que nodos participan
2. El `provenance` del edge B→D dice que cadena comprime
3. La INJ se aplica en el eslabon especifico de esa cadena donde el supuesto invalidado tiene efecto

Esto permite al agente recomendar: "La INJ-005 interviene entre RC-001 e INT-003 en el CRT, invalidando el supuesto ASM-007 que decia que [supuesto]."

#### 14.9.3. EC como generador de hipotesis para el CRT

Si no tenemos un CRT construido pero tenemos una EC, los supuestos detras de cada flecha del EC son **pistas** para construir el CRT:

- Cada supuesto del EC sugiere una cadena causal que deberia existir en el CRT
- Desmenuzar los supuestos de A→B, A→C, B→D, C→D' produce hipotesis sobre eslabones causales
- El CRT resultante puede validar o refutar esas hipotesis

Este es un camino alternativo al flujo canonico (GT→CRT→EC): si empezamos con el conflicto (EC), podemos derivar el diagnostico (CRT) retro-ingenieria de los supuestos.

#### 14.9.4. Recursion EC→INJ→FRT→NBR→EC₂

El proceso LTP es fundamentalmente recursivo:

```
EC₁ → INJ₁ → FRT₁ → NBR → Trimming INJ → nuevo conflicto? 
                                              → SI: EC₂ → INJ₂ → FRT₁' → ...
                                              → NO: PRT → TT
```

Cada nivel de recursion genera un **escenario hijo** (parent_scenario apunta al padre). La profundidad de recursion es un indicador de salud del analisis:

- Profundidad 0-1: saludable
- Profundidad 2: aceptable pero merece atencion
- Profundidad 3+: senal de conflicto estructural — considerar volver al GT

---

## Changelog

| Fecha | Cambio |
|-------|--------|
| 2026-09-11 | Creacion inicial: sistema, relaciones entre arboles, escenarios, marcos temporales |
| 2026-09-11 | Seccion 10: analisis profundo Six Hats de suficiencia vs necesidad en el meta-grafo |
| 2026-09-11 | Seccion 11: Six Hats de la EC como proyeccion comprimida — proveniencia, projection_map, edge provenance |
| 2026-09-11 | Q10 resuelta: EC merece tratamiento especial. Q11-Q13 nuevas (granularidad, mutabilidad, auto-inferencia) |
| 2026-09-11 | Seccion 12: Six Hats de actores como dimension del meta-grafo — atribucion ligera, GTs satelite, escalado progresivo |
| 2026-09-11 | Q14-Q17 nuevas (actor_role granularidad, actores vs stakeholders, GT satelite vs supuesto, atribucion multi-actor) |
| 2026-09-11 | Seccion 11 corregida: matiz norma-bisagra (CSF/NC no son UDE/DE directamente, se derivan por violacion/cumplimiento). B/C pueden ser CSF y/o NC. |
| 2026-09-11 | Seccion 13: Matriz de transformacion de tipos de nodo entre arboles — nivel micro que complementa las 20 transiciones macro. Espacio de fuentes de la EC. |
| 2026-09-11 | Q18-Q20 nuevas (transformation_type, validacion norma-bisagra, espacio de fuentes EC en motor) |
| 2026-09-12 | Seccion 14: Six Hats de escenarios como ramas de futuros alternativos — branch_point enriquecido, recursion controlada, scout-commit-pivot, comparacion estructurada, escenarios parciales |
