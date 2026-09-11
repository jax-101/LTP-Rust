# RFC-001: ltp-engine-max — Rediseno de Persistencia con libSQL/Turso

| Campo       | Valor                              |
|-------------|------------------------------------|
| Status      | **EXPLORING**                      |
| Author      | Javier Asensio                     |
| Created     | 2026-09-10                         |
| Updated     | 2026-09-11                         |
| Target      | Friendly fork: `ltp-engine-max`    |
| Lifecycle   | EXPLORING → DRAFT → ACCEPTED/REJECTED |

---

## 1. Motivacion

### Limitaciones del modelo actual (JSON en disco)

ltp-engine persiste su estado como archivos JSON individuales en disco. Este diseno es simple, determinista, legible a ojo y git-diffable. Sin embargo, presenta limitaciones estructurales:

- **Sin back-references**: un nodo no sabe en que arboles participa. Descubrirlo requiere escanear todos los trees.
- **Relaciones M:N denormalizadas**: arrays de string IDs sin integridad referencial a nivel de storage.
- **Queries cross-tree imposibles sin LLM**: responder "que nodos comparten CRT y FRT" requiere cargar todo en contexto del agente.
- **Sin busqueda semantica headless**: la unica forma de buscar por significado es a traves de un LLM, con coste en tokens y no-determinismo.
- **Assumptions profundamente anidados**: Tree → Edge → Assumption (3 niveles) dificulta queries directas.
- **Sin atomicidad real**: `history_begin_batch` es una convencion, no una transaccion ACID. Un crash a mitad del batch deja estado parcial.
- **Feedback en vivo imposible**: la UI que se desarrolla en paralelo necesita queries rapidas y ricas para dar feedback semantico mientras el usuario construye los arboles.

### Vision: ltp-engine-max

Un friendly fork de ltp-engine donde:

- **libSQL/Turso** es la fuente de verdad (un `workspace.db` por workspace)
- **JSON** pasa a ser formato de exportacion legible por humanos
- **Vectores** estan integrados nativamente (columnas F32_BLOB, indice DiskANN)
- **Tres consumidores** comparten el mismo backend: UI (humano), MCP (agente), CLI

El motor nace async (`tokio`) porque toda la I/O pasa por el DB — no es un parche, es el fundamento.

---

## 2. Contexto Tecnologico: libSQL / Turso

### Que es

[libSQL](https://github.com/tursodatabase/libsql) es un fork open-source de SQLite creado por Turso. Extiende SQLite con:

- Extensibilidad (UDFs en Rust/WASM)
- Replicacion (replicas embebidas)
- Acceso HTTP
- **Columnas vectoriales nativas** con indice DiskANN

### Capacidades Vectoriales

| Tipo de vector | Storage por dimension | Caso de uso |
|----------------|----------------------|-------------|
| `F32_BLOB`     | 4 bytes              | Default recomendado |
| `F64_BLOB`     | 8 bytes              | Maxima precision |
| `F16_BLOB`     | 2 bytes              | Balanceado |
| `F8_BLOB`      | 1 byte               | Compacto |
| `F1BIT_BLOB`   | 1/8 byte             | Compresion extrema |

- **Max dimensiones**: 65,536
- **Funciones de distancia**: `vector_distance_cos()` (coseno), `vector_distance_l2()` (euclidea)
- **Indice**: DiskANN (busqueda aproximada k-NN), configurable (`max_neighbors`, `search_l`, `insert_l`)

### Rust Crate

- Crate: `libsql` v0.10.x con feature flag `core` para modo embebido
- API **async** — `execute()`, `query()`, `prepare()`, `transaction()` son todos async
- `Database::open()` y `Database::connect()` son sync
- Modo local: `Builder::new_local("workspace.db")` — un archivo `.db`, sin servidor
- Modo replica: `Builder::new_remote_replica("local.db", url, token)` — sync con Turso
- Vectores via SQL raw (`execute`), no hay API Rust nativa para vectores
- Ejemplo basico:

```rust
let db = Builder::new_local("workspace.db").build().await?;
let conn = db.connect()?;
conn.execute("CREATE TABLE nodes (id TEXT PRIMARY KEY, embedding F32_BLOB(384))", ()).await?;
conn.execute("INSERT INTO nodes VALUES (?, vector32(?))", [id, json_vec]).await?;
```

---

## 3. Recorrido de Exploracion

Esta RFC evoluciono a traves de tres arquitecturas evaluadas antes de converger en la recomendada.

### Arquitectura A: Hibrida (JSON + indice derivado)

**Concepto**: JSON sigue como fuente de verdad. Se agrega un `.ltp/index.db` (rusqlite) como indice derivado para queries relacionales. Vectores como fase posterior (sqlite-vec).

**Ventajas**: Preserva transparencia de archivos, git-friendly, incremental. Desbloquea 5 capacidades relacionales.

**Drawbacks identificados (analisis Six Hats)**:

| Drawback | Severidad |
|----------|-----------|
| Sincronizacion JSON↔DB — dos stores que pueden divergir | ALTA |
| La promesa de "derivado" se erosiona con el tiempo (comandos que solo usan DB) | MEDIA-ALTA |
| Edicion manual de JSON rompe indice silenciosamente | MEDIA |
| Crash entre JSON write e index sync → inconsistencia silenciosa | MEDIA |
| Superficie de testing x2-3 (verificar ambos stores) | MEDIA |
| sqlite-vec en alpha (0.1.9) con bloque `unsafe` para cargarlo | MEDIA |

**Veredicto**: Funcional pero la complejidad de sincronizacion es un coste recurrente, no puntual.

### Arquitectura B: Hibrida con rusqlite (resolviendo async)

**Concepto**: Igual que A pero con rusqlite (sync, 147M downloads) en vez de libsql (async). Vectores diferidos a cuando sqlite-vec madure.

**Analisis sync vs async**:

| | libsql | rusqlite | rusqlite + sqlite-vec |
|---|---|---|---|
| API sync | **No** | **Si** | **Si** |
| Requiere tokio | Si | No | No |
| Vectores nativos | Si (DiskANN) | No | Via extension (alpha, unsafe) |
| Madurez | pre-v1 | **Estable, 147M downloads** | alpha |
| Binary delta | +10-15 MB | +3-5 MB | +4-6 MB |

**Veredicto**: Resuelve el problema async pero los vectores quedan como ciudadanos de segunda clase (extension alpha, tablas separadas, unsafe). Y el drawback de sincronizacion persiste.

### Arquitectura C: Fork con libSQL/Turso (DB como fuente de verdad) — RECOMENDADA

**Concepto**: El DB **es** la fuente de verdad. JSON es formato de exportacion. Vectores nativos. Motor async desde el diseno.

**Por que funciona ahora y no antes**:

1. **Existe un proyecto de UI** — la transparencia humana la provee la UI, no los archivos JSON.
2. **El async esta justificado** — toda la I/O es contra el DB, no es un parche para un indice auxiliar.
3. **Vectores inline** — columnas `F32_BLOB` en la misma tabla, no tablas virtuales separadas ni unsafe.
4. **Cero sincronizacion** — un solo store, cero divergencia posible.
5. **ACID real** — batches son transacciones SQL, no convenciones.
6. **Turso-ready** — replica embebida disponible cuando haya home server.

---

## 4. Modelo de Datos Actual — Inventario Completo

### Entidades (ltp-engine actual)

| Entidad | Archivo(s) | Struct Rust | Patron de ID |
|---------|-----------|-------------|--------------|
| **Node** | `nodes/<ID>.json` | `Node` | `UDE-001`, `RC-002`, `INJ-003`... (13 prefijos) |
| **Tree** | `trees/<ID>.json` | `Tree` | `tree-<type>-<slug>` |
| **Edge** | Embebido en `Tree.edges[]` | `Edge` | `LINK-001` |
| **Assumption** | Embebido en `Edge.assumptions[]` | `Assumption` | `ASM-001` |
| **FeedbackEdge** | Embebido en `Tree.feedback_edges[]` | `FeedbackEdge` | `FB-001` |
| **MacroEdge** | Embebido en `Tree.macro_edges[]` | `MacroEdge` | `MACRO-001` |
| **NbrBranch** | Embebido en `Tree.nbr_branches[]` | `NbrBranch` | `NBR-001` |
| **KnowledgeItem** | `knowledge/<ID>.json` | `KnowledgeItem` | `KN-001` |
| **UndoEntry** | `.ltp/undo/<seq>.json` | `UndoEntry` | seq `1,2,3...` |
| **RedoEntry** | `.ltp/redo/<seq>.json` | `RedoEntry` | seq |
| **WorkspaceConfig** | `ltp.config.json` | `WorkspaceConfig` | singleton |
| **Counters** | `.ltp/counters.json` | `BTreeMap<String,u64>` | singleton |
| **Lock** | `.ltp/lock` | ad-hoc JSON | singleton |

### Campos por Entidad

**Node**: `id`, `node_type` (13 variantes enum), `label`, `tags[]`, `observable: bool`, `epistemic` (fact|hypothesis|assumption|derived), `metadata.status` (active|draft|invalidated|superseded), `metadata.extra` (BTreeMap catch-all)

**Tree**: `id`, `name`, `tree_type` (gt|crt|ec|frt|prt|tt), `logic` (sufficiency|necessity), `nodes[]`, `edges[]`, `macro_edges[]`, `feedback_edges[]`, `nbr_branches[]`

**NodeRef** (en Tree.nodes): `node_ref` (→ Node.id), `role: Option<String>`

**Edge**: `id`, `from: Vec<String>` (→ Node.id[]), `to: String` (→ Node.id), `operator` (SINGLE|AND|OR|MAG|XOR), `weight: Option<f64>`, `status` (active|broken|superseded|needs_review), `logic` (SUFFICIENCY|NECESSITY), `assumptions[]`

**Assumption**: `id`, `status` (valid|invalid|needs_review), `text`

**FeedbackEdge**: `id`, `from` (→ Node.id), `to` (→ Node.id), `loop_type` (positive|negative), `label: Option<String>`

**MacroEdge**: `id`, `from` (→ Node.id), `to` (→ Node.id), `label`, `interior_nodes[]` (→ Node.id[]), `interior_links[]` (→ Edge.id[]), `status`

**NbrBranch**: `id`, `source_node` (→ Node.id), `edges[]` (mismo struct Edge), `trim_injection: Option<String>` (→ Node.id)

**KnowledgeItem**: `id`, `knowledge_type` (measurement|testimony|hypothesis|document|observation|derived), `label`, `status` (unverified|verified|refuted|superseded), `confidence: Option` (high|medium|low), `source.uri: Option`, `source.excerpt: Option`, `captured` (ISO date), `links[]`, `tags[]`

**KnowledgeLink** (en KnowledgeItem.links): `target` (→ Node.id o Edge.id o Assumption.id), `relation` (supports|contradicts|contextualizes)

**UndoEntry**: `seq`, `action`, `command`, `timestamp`, `batch: Option<String>`, `affected_files: BTreeMap<path, FileSnapshot{before, after_hash}>`

### Relaciones (con cardinalidad)

| From | To | Relacion | Cardinalidad |
|------|----|----------|-------------|
| Tree.nodes[].ref | Node.id | participacion | M:N (nodo en muchos arboles) |
| Edge.from[] | Node.id | causa(s) | N:1 por edge (1+ causas → 1 efecto) |
| Edge.to | Node.id | efecto | N:1 |
| Assumption → Edge | embebido | ownership | N:1 |
| FeedbackEdge.from/to | Node.id | referencia feedback | N:1 cada uno |
| MacroEdge.interior_nodes | Node.id | miembros colapsados | M:N |
| MacroEdge.interior_links | Edge.id | miembros colapsados | M:N |
| NbrBranch.source_node | Node.id | fuente de riesgo | N:1 |
| NbrBranch.trim_injection | Node.id | inyeccion mitigante | N:1 opcional |
| NbrBranch.edges[] | Edge (mismo struct) | cadena causal NBR | 1:N |
| KnowledgeLink.target | Node.id / Edge.id / Assumption.id | link epistemico | M:N |

### Observaciones Estructurales Clave

1. **Nodos son archivos top-level** — pool global, referenciados por ID desde trees.
2. **Trees son la capa de vista** — edges, assumptions, feedback edges, macro edges y NBR branches estan TODOS embebidos dentro del JSON del tree.
3. **Knowledge items son archivos top-level** — pool separado con links transversales al grafo via string IDs.
4. **Assumptions estan profundamente anidados** — Tree → Edge → Assumption (3 niveles).
5. **Sin tablas de join** — todas las relaciones M:N son arrays de string IDs (denormalizadas).
6. **Sin back-references** — un Node no sabe en que trees esta; eso se descubre escaneando todos los trees.

---

## 5. Arquitectura de ltp-engine-max

### Principio Central

> **`workspace.db` es la unica fuente de verdad. JSON es un formato de exportacion para humanos y git. Vectores son ciudadanos de primera clase.**

### Tres Consumidores, Un Backend

```
           ┌─── UI (humano) ───────┐
           │                       │
           ▼                       ▼
    workspace.db  ◄────────  Agent (MCP)
   (source of truth)
           │
           └─── CLI (humano/scripts)
           │
           ▼
    JSON export (git, backup, interop)
```

### Espectro de Autonomia

El diseno sirve a todo el espectro, no a un solo modo:

```
Humano puro ◄───────────────────────────────────────────► Agente autonomo
     │              │                    │                      │
  UI only     UI + asistencia       MCP + approval       MCP headless
              semantica IA en       humano en UI
              tiempo real
```

| Modo | Interaccion | Que necesita del backend |
|------|------------|-------------------------|
| Humano puro | UI | Queries rapidas, estado completo, visualizacion |
| Humano + IA asistente | UI + sugerencias en vivo | Queries vectoriales en tiempo real, feedback semantico |
| IA + humano supervisor | MCP + UI para aprobar | ACID, completitud garantizada, impact analysis |
| Agente autonomo | MCP headless | Tool calls eficientes, determinismo, vectores |

### Asistencia Semantica en Vivo (UI)

El caso mas impactante: el humano construye arboles en la UI y la IA asiste **sin ser autonoma**:

| El humano esta... | La IA sugiere... | Requiere vectores? |
|-------------------|------------------|--------------------|
| Creando una UDE | "Ya existe UDE-007 con wording similar" | **Si** |
| Escribiendo un supuesto | "Este supuesto contradice KN-014" | **Si** |
| Conectando causa→efecto | "Conexion debil segun CLR#4, falta AND?" | No |
| Buscando donde insertar nodo | "3 cadenas causales hablan de entregas" | **Si** |
| Revisando el CRT | "Estas 4 UDEs son variaciones del mismo tema" | **Si** |
| Decidiendo que investigar | "RC-005 tiene 0% cobertura epistemica" | No |
| Navegando entre arboles | "Este nodo tambien aparece en FRT y PRT" | No |

Con Turso, estas sugerencias son queries de milisegundos contra `workspace.db`. Sin LLM call, sin tokens, feedback en tiempo real mientras el usuario escribe.

### Workspace del Fork

```
mi-proyecto-ltp/
├── ltp.config.json          # Config (sigue siendo JSON legible)
├── workspace.db             # Source of truth (libSQL)
└── .ltp/
    ├── lock                 # Lock de concurrencia
    └── backups/             # Snapshots automaticos pre-operacion destructiva
```

Opcionalmente, con exports:
```
mi-proyecto-ltp/
├── ltp.config.json
├── workspace.db
├── export/                  # Generado por ltp export
│   ├── nodes/
│   ├── trees/
│   └── knowledge/
└── .ltp/
```

---

## 6. Schema Relacional

```sql
-- Pool global de nodos
CREATE TABLE nodes (
    id          TEXT PRIMARY KEY,   -- 'UDE-001'
    node_type   TEXT NOT NULL,      -- enum 13 variantes
    label       TEXT NOT NULL,
    observable  INTEGER DEFAULT 1,
    epistemic   TEXT DEFAULT 'hypothesis',
    status      TEXT DEFAULT 'active',
    extra       TEXT,               -- JSON catch-all
    embedding   F32_BLOB(384)       -- VECTOR (NULL si embeddings deshabilitados)
);
CREATE INDEX nodes_vec ON nodes(libsql_vector_idx(embedding, 'metric=cosine'));

-- Arboles (metadata plana, sin embeber edges)
CREATE TABLE trees (
    id          TEXT PRIMARY KEY,   -- 'tree-crt-logistica'
    name        TEXT NOT NULL,
    tree_type   TEXT NOT NULL,      -- gt|crt|ec|frt|prt|tt
    logic       TEXT NOT NULL       -- sufficiency|necessity
);

-- Participacion nodo-arbol (M:N explicita con back-references nativas)
CREATE TABLE tree_nodes (
    tree_id     TEXT REFERENCES trees(id),
    node_id     TEXT REFERENCES nodes(id),
    role        TEXT,               -- objetivo, root_cause, injection...
    PRIMARY KEY (tree_id, node_id)
);

-- Edges como entidad de primer nivel
CREATE TABLE edges (
    id          TEXT PRIMARY KEY,   -- 'LINK-001'
    tree_id     TEXT REFERENCES trees(id),
    to_node     TEXT REFERENCES nodes(id),
    operator    TEXT DEFAULT 'SINGLE',
    weight      REAL,
    status      TEXT DEFAULT 'active',
    logic       TEXT                -- SUFFICIENCY|NECESSITY
);

-- Causes (1 edge → N from-nodes)
CREATE TABLE edge_sources (
    edge_id     TEXT REFERENCES edges(id),
    node_id     TEXT REFERENCES nodes(id),
    PRIMARY KEY (edge_id, node_id)
);

-- Assumptions como entidad de primer nivel
CREATE TABLE assumptions (
    id          TEXT PRIMARY KEY,   -- 'ASM-001'
    edge_id     TEXT REFERENCES edges(id),
    text        TEXT NOT NULL,
    status      TEXT DEFAULT 'valid',
    embedding   F32_BLOB(384)       -- VECTOR
);
CREATE INDEX asm_vec ON assumptions(libsql_vector_idx(embedding, 'metric=cosine'));

-- Feedback edges
CREATE TABLE feedback_edges (
    id          TEXT PRIMARY KEY,
    tree_id     TEXT REFERENCES trees(id),
    from_node   TEXT REFERENCES nodes(id),
    to_node     TEXT REFERENCES nodes(id),
    loop_type   TEXT NOT NULL,      -- positive|negative
    label       TEXT
);

-- Macro edges
CREATE TABLE macro_edges (
    id          TEXT PRIMARY KEY,
    tree_id     TEXT REFERENCES trees(id),
    from_node   TEXT REFERENCES nodes(id),
    to_node     TEXT REFERENCES nodes(id),
    label       TEXT NOT NULL,
    status      TEXT DEFAULT 'active'
);

CREATE TABLE macro_edge_nodes (
    macro_id    TEXT REFERENCES macro_edges(id),
    node_id     TEXT REFERENCES nodes(id),
    PRIMARY KEY (macro_id, node_id)
);

CREATE TABLE macro_edge_links (
    macro_id    TEXT REFERENCES macro_edges(id),
    edge_id     TEXT REFERENCES edges(id),
    PRIMARY KEY (macro_id, edge_id)
);

-- NBR branches
CREATE TABLE nbr_branches (
    id              TEXT PRIMARY KEY,
    tree_id         TEXT REFERENCES trees(id),
    source_node     TEXT REFERENCES nodes(id),
    trim_injection  TEXT REFERENCES nodes(id)
);

CREATE TABLE nbr_edges (
    nbr_id      TEXT REFERENCES nbr_branches(id),
    edge_id     TEXT PRIMARY KEY,
    to_node     TEXT REFERENCES nodes(id),
    operator    TEXT DEFAULT 'SINGLE',
    status      TEXT DEFAULT 'active',
    logic       TEXT
);

CREATE TABLE nbr_edge_sources (
    edge_id     TEXT REFERENCES nbr_edges(edge_id),
    node_id     TEXT REFERENCES nodes(id),
    PRIMARY KEY (edge_id, node_id)
);

-- Knowledge pool
CREATE TABLE knowledge (
    id              TEXT PRIMARY KEY,
    ktype           TEXT NOT NULL,
    label           TEXT NOT NULL,
    status          TEXT DEFAULT 'unverified',
    confidence      TEXT,
    source_uri      TEXT,
    source_excerpt  TEXT,
    captured        TEXT,               -- ISO 8601
    embedding       F32_BLOB(384)       -- VECTOR
);
CREATE INDEX kn_vec ON knowledge(libsql_vector_idx(embedding, 'metric=cosine'));

-- Knowledge links (M:N polimorfica)
CREATE TABLE knowledge_links (
    knowledge_id TEXT REFERENCES knowledge(id),
    target_id    TEXT NOT NULL,
    target_type  TEXT NOT NULL,      -- 'node'|'edge'|'assumption'
    relation     TEXT NOT NULL,      -- supports|contradicts|contextualizes
    PRIMARY KEY (knowledge_id, target_id, relation)
);

-- Tags normalizadas
CREATE TABLE node_tags (
    node_id     TEXT REFERENCES nodes(id),
    tag         TEXT NOT NULL,
    PRIMARY KEY (node_id, tag)
);

CREATE TABLE knowledge_tags (
    knowledge_id TEXT REFERENCES knowledge(id),
    tag          TEXT NOT NULL,
    PRIMARY KEY (knowledge_id, tag)
);

-- Counters (reemplaza counters.json)
CREATE TABLE counters (
    prefix      TEXT PRIMARY KEY,    -- 'UDE', 'RC', 'LINK', etc.
    next_seq    INTEGER NOT NULL DEFAULT 1
);

-- Undo journal (reemplaza .ltp/undo/*.json)
CREATE TABLE undo_journal (
    seq         INTEGER PRIMARY KEY AUTOINCREMENT,
    batch_id    TEXT,                -- NULL si no es batch
    table_name  TEXT NOT NULL,
    row_id      TEXT NOT NULL,
    action      TEXT NOT NULL,       -- INSERT, UPDATE, DELETE
    before_data TEXT,                -- JSON del estado anterior (NULL para INSERT)
    after_data  TEXT,                -- JSON del estado nuevo (NULL para DELETE)
    timestamp   TEXT NOT NULL        -- ISO 8601
);
```

---

## 7. Flujo Operativo

### Mutacion — Ejemplo: `node_add`

```
ltp node_add --type UDE --label "Entregas llegan tarde el 40% de las veces"
```

```sql
BEGIN TRANSACTION;
  -- 1. Generar ID
  UPDATE counters SET next_seq = next_seq + 1 WHERE prefix = 'UDE' RETURNING next_seq;
  -- → UDE-001

  -- 2. Insertar nodo (+ embedding si habilitado)
  INSERT INTO nodes (id, node_type, label, embedding)
  VALUES ('UDE-001', 'UDE', 'Entregas llegan tarde el 40%', vector32(?));

  -- 3. Journal para undo
  INSERT INTO undo_journal (table_name, row_id, action, after_data, timestamp)
  VALUES ('nodes', 'UDE-001', 'INSERT', '{"id":"UDE-001",...}', '2026-09-11T10:00:00Z');
COMMIT;  -- todo o nada, ACID
```

Si el proceso muere entre el BEGIN y el COMMIT: **nada cambio**. Sin estado parcial.

### Batch atomico

```sql
BEGIN TRANSACTION;
  -- Marca inicio de batch en journal
  INSERT INTO undo_journal (...) VALUES (..., batch_id='batch-001', ...);

  -- N operaciones...
  INSERT INTO nodes ...;
  INSERT INTO edges ...;
  INSERT INTO tree_nodes ...;

  -- Todas las entradas del batch comparten batch_id
COMMIT;
```

`ltp undo` revierte todas las entradas con el mismo `batch_id` en una sola transaccion.

### Query cross-tree — `ltp impact`

```sql
WITH RECURSIVE impact AS (
    SELECT e.to_node AS nid, e.tree_id, 1 AS depth
    FROM edges e JOIN edge_sources es ON es.edge_id = e.id
    WHERE es.node_id = 'RC-001'
  UNION
    SELECT e.to_node, e.tree_id, i.depth + 1
    FROM edges e JOIN edge_sources es ON es.edge_id = e.id
    JOIN impact i ON es.node_id = i.nid
)
SELECT nid, tree_id, depth FROM impact ORDER BY depth;
```

### Query semantica — feedback en vivo

Mientras el usuario escribe en la UI:
```sql
SELECT n.id, n.label, vector_distance_cos(n.embedding, vector32(?)) as dist
FROM nodes n
WHERE dist < 0.3 AND n.id != ? AND n.status = 'active'
ORDER BY dist LIMIT 5;
```

Latencia: milisegundos. Sin LLM. Sin tokens.

### Undo / Redo

```sql
-- Undo: leer ultima entrada (o batch) del journal, revertir
BEGIN TRANSACTION;
  -- Para cada entrada del batch, en orden inverso:
  -- INSERT → DELETE WHERE id = row_id
  -- UPDATE → UPDATE SET ... = before_data WHERE id = row_id
  -- DELETE → INSERT con before_data
  
  -- Mover entradas a redo_journal
COMMIT;
```

### Export a JSON

```bash
ltp export --format json --dir ./export/
```

Genera la estructura clasica legible:
```
export/
├── nodes/UDE-001.json
├── nodes/RC-001.json
├── trees/tree-crt-logistica.json
└── knowledge/KN-001.json
```

Util para: git commits, compartir con humanos, interoperabilidad, backup legible.

### Import desde JSON (migracion)

```bash
ltp import --dir ./mi-workspace-ltp-engine-original/
```

Lee los JSON del workspace de ltp-engine original y los carga en `workspace.db`. Path de migracion desde el motor actual.

---

## 8. Casos de Uso Desbloqueados

### Resumen rapido

| Capacidad | Tipo | Hoy |
|-----------|------|-----|
| Back-references | Relacional | Imposible sin escanear todos los trees |
| Impact analysis cross-tree | Relacional | Imposible sin N tree_walks |
| Health epistemica | Relacional | Imposible sin cruce manual LLM |
| Nodos huerfanos/sin evidencia | Relacional | Invisible |
| Travesia cross-tree full lineage | Relacional | Imposible |
| Queries arbitrarias | Relacional | No existe |
| Busqueda semantica headless | Vectorial | Solo con LLM |
| Deteccion automatica duplicados | Vectorial | Solo con LLM |
| Clustering por tema | Vectorial | Imposible |
| Feedback semantico en vivo (UI) | Vectorial | Imposible |
| Atomicidad ACID | Operacional | Convencion (batch) |
| Undo/redo transaccional | Operacional | Snapshots de archivos |
| Export legible | Operacional | Los JSON SON el storage |
| Replica multi-dispositivo | Operacional | No existe |

A continuacion, cada caso de uso en profundidad, organizado por consumidor.

---

### 8.1 Feedback Semantico en Vivo (Humano en UI)

#### 8.1.1 Deteccion de duplicados mientras escribes

**Escenario**: El usuario construye un CRT de logistica. Escribe la UDE "Los clientes reciben pedidos danados frecuentemente". Lleva 30 nodos en el workspace.

**Hoy**: Nada avisa. Se crea la UDE. Semanas despues se descubre que ya existia UDE-012 "Productos llegan con danos al cliente final" — esencialmente lo mismo con distinto wording. Dos cadenas causales para el mismo efecto.

**Con ltp-engine-max**: Mientras se escribe en el campo de texto de la UI, a cada pausa de 300ms:

```sql
SELECT id, label, vector_distance_cos(embedding, vector32(?)) as dist
FROM nodes WHERE node_type = 'UDE' AND status = 'active' AND dist < 0.25
ORDER BY dist LIMIT 3;
```

La UI muestra un tooltip: *"Similar a UDE-012: Productos llegan con danos al cliente final (92% similitud). Es la misma?"*

El humano decide: fusionar, distinguir explicitamente, o continuar. **La decision la toma el humano, el motor solo ilumina.**

#### 8.1.2 Contradicciones knowledge-assumption en tiempo real

**Escenario**: El usuario trabaja en una EC sobre conflicto de entregas. Escribe el supuesto "El proveedor tiene capacidad para entregar en 24h".

**Hoy**: El supuesto se guarda. Nadie recuerda que hace dos semanas se agrego KN-008: "Auditoria de capacidad del proveedor: lead time minimo 72h, verificado febrero 2026". El supuesto contradice evidencia verificada y nadie lo sabe.

**Con ltp-engine-max**:

```sql
SELECT k.id, k.label, k.status, kl.relation,
       vector_distance_cos(k.embedding, vector32(?)) as dist
FROM knowledge k
WHERE dist < 0.3 AND k.status = 'verified'
ORDER BY dist LIMIT 5;
```

La UI muestra un warning: *"KN-008 (verified) podria contradecir este supuesto: 'Lead time minimo 72h'. Confidence: high."*

El humano investiga, y o bien invalida el supuesto (la EC se resuelve), o bien cuestiona el knowledge (datos desactualizados?). **Sin vectores, esta conexion es invisible salvo que un LLM cargue todo en contexto y la detecte.**

#### 8.1.3 Sugerencia de agrupacion AND/OR

**Escenario**: Se conecta RC-005 → UDE-003 como SINGLE. Pero hay otras dos causas (RC-002, RC-007) que ya apuntan a UDE-003 con wording semanticamente cercano al de RC-005.

**Hoy**: El motor lanza un warning CLR#4 generico ("SINGLE edge, consider AND/OR"). No sabe si las causas son realmente co-dependientes o independientes.

**Con ltp-engine-max**: El motor compara embeddings de las tres causas:

- RC-002 <-> RC-005: distancia 0.7 (distintas semanticamente)
- RC-007 <-> RC-005: distancia 0.15 (casi identicas)

La UI sugiere: *"RC-005 y RC-007 son semanticamente muy similares. Son realmente dos causas distintas o deberian fusionarse? Si son distintas e independientes, considera agrupar como OR."*

#### 8.1.4 Mapa de calor epistemico en el diagrama

**Escenario**: Un CRT con 25 nodos renderizado en la UI.

**Hoy**: Todos los nodos se ven igual. No se sabe cuales estan bien fundamentados y cuales son castillos de arena.

**Con ltp-engine-max**:

```sql
SELECT n.id, n.node_type,
  COUNT(CASE WHEN k.status = 'verified' THEN 1 END) as verified_count,
  COUNT(CASE WHEN k.status = 'refuted' THEN 1 END) as refuted_count,
  COUNT(CASE WHEN a.status = 'invalid' THEN 1 END) as invalid_assumptions
FROM nodes n
JOIN tree_nodes tn ON tn.node_id = n.id
LEFT JOIN knowledge_links kl ON kl.target_id = n.id
LEFT JOIN knowledge k ON k.id = kl.knowledge_id
LEFT JOIN edges e ON e.to_node = n.id
LEFT JOIN assumptions a ON a.edge_id = e.id
WHERE tn.tree_id = 'tree-crt-logistica'
GROUP BY n.id;
```

Cada nodo en el diagrama tiene un color de fondo:
- **Verde**: respaldado por knowledge verificada
- **Amarillo**: sin knowledge, solo hipotesis
- **Rojo**: knowledge refutada o supuestos invalidados
- **Gris pulsante**: nodo con supuestos que contradicen knowledge (deteccion vectorial)

**El humano ve la salud de su arbol de un vistazo.** Las zonas rojas son donde debe investigar. Las verdes son solidas.

---

### 8.2 Inteligencia Cross-Tree

#### 8.2.1 Linaje completo: de UDE a accion tactica

**Escenario**: Un stakeholder pregunta "por que tenemos que hacer la accion X del plan de implementacion?"

**Hoy**: Reconstruir la cadena mentalmente o con el agente haciendo 5-6 tree_walks encadenados. Lento, propenso a error, no queda registrado.

**Con ltp-engine-max**:

```bash
ltp trace --cross-tree --from ACTION-003 --direction upstream
```

```sql
WITH RECURSIVE lineage AS (
    SELECT 'ACTION-003' as nid, 'tree-tt-plan' as tree_id, 0 as depth
  UNION
    SELECT es.node_id, e.tree_id, l.depth + 1
    FROM lineage l
    JOIN edges e ON e.to_node = l.nid
    JOIN edge_sources es ON es.edge_id = e.id
  UNION
    SELECT tn.node_id, tn.tree_id, l.depth + 1
    FROM lineage l
    JOIN tree_nodes tn ON tn.node_id = l.nid AND tn.tree_id != l.tree_id
)
SELECT * FROM lineage ORDER BY depth DESC;
```

Resultado en la UI — un camino visual iluminado que cruza todos los arboles:

```
TT: ACTION-003 "Implementar GPS tracking"
  ↑ PRT: IO-002 "Conseguir presupuesto para GPS"
    ↑ PRT: INJ-005 "Sistema de tracking en tiempo real"  ← shared with FRT
      ↑ FRT: DE-003 "Los clientes conocen el estado de su pedido"
        ↑ EC: INJ-005 (invalido supuesto ASM-012)
          ↑ CRT: RC-001 "No hay tracking en tiempo real"
            ↑ CRT: UDE-003 "Clientes cancelan pedidos por incertidumbre"
              ↑ GT: GOAL "Reducir cancelaciones al 5%"
```

**El stakeholder ve la trazabilidad completa de por que cada accion existe.** No es una explicacion narrativa — es una cadena logica verificable.

#### 8.2.2 Deteccion de nodos que deberian compartirse pero no lo estan

**Escenario**: En el CRT hay RC-003 "Falta de visibilidad en la cadena de suministro". En el FRT hay DE-008 "Se logra visibilidad completa de la cadena de suministro" — creado independientemente, sin relacion explicita.

**Hoy**: Nadie sabe que estos dos nodos hablan de lo mismo (uno como problema, otro como estado deseado). Son islas desconectadas.

**Con ltp-engine-max**:

```sql
SELECT a.id, a.label, a.node_type, b.id, b.label, b.node_type,
       vector_distance_cos(a.embedding, b.embedding) as dist
FROM nodes a, nodes b
WHERE a.id < b.id AND dist < 0.2
AND NOT EXISTS (
    SELECT 1 FROM tree_nodes t1
    JOIN tree_nodes t2 ON t1.tree_id = t2.tree_id
    WHERE t1.node_id = a.id AND t2.node_id = b.id
);
```

Nuevo comando: `ltp suggest_links`:

```
Candidates for cross-tree linkage:
  RC-003 (CRT) <-> DE-008 (FRT) — 0.89 similarity
    "Falta de visibilidad..." <-> "Se logra visibilidad..."
    Suggestion: DE-008 should be the resolution of RC-003
```

#### 8.2.3 Analisis de convergencia/divergencia entre CRTs

**Escenario**: 3 CRTs de distintos departamentos. Cuanto se solapan los root causes? Hay una causa raiz sistemica que nadie ha conectado?

**Hoy**: Un humano o LLM tiene que cargar los 3 CRTs, leer todas las RCs, y cruzar mentalmente.

**Con ltp-engine-max**:

```sql
SELECT a.id, b.id, a.label, b.label,
       vector_distance_cos(a.embedding, b.embedding) as dist
FROM nodes a
JOIN tree_nodes ta ON ta.node_id = a.id
JOIN trees t1 ON t1.id = ta.tree_id AND t1.tree_type = 'crt'
JOIN nodes b ON b.node_type = 'RC' AND b.id > a.id
JOIN tree_nodes tb ON tb.node_id = b.id
JOIN trees t2 ON t2.id = tb.tree_id AND t2.tree_type = 'crt' AND t2.id != t1.id
WHERE a.node_type = 'RC'
AND vector_distance_cos(a.embedding, b.embedding) < 0.2;
```

Resultado:

```
Cross-CRT Root Cause Convergence:
  Cluster: "informacion en tiempo real"
    RC-003 (CRT Logistica): "Falta tracking en tiempo real"
    RC-011 (CRT Ventas): "Vendedores no ven stock actualizado"
    RC-019 (CRT Produccion): "Planta no recibe demand signal"
    → Possible systemic root cause: data visibility gap across the org
```

**Esto es discovery que un humano tardaria horas en hacer y que un agente haria gastando cientos de tokens. Con vectores, es una query.**

---

### 8.3 Inteligencia Epistemica

#### 8.3.1 Vulnerabilidad de supuestos: "que pasa si estoy equivocado?"

**Escenario**: 30 supuestos en las ECs. Cual, si resulta falso, rompe mas cadenas causales?

**Hoy**: No existe forma de saberlo sin recorrer manualmente cada supuesto y trazar sus dependencias.

**Con ltp-engine-max**:

```sql
SELECT a.id, a.text, a.status,
  (SELECT COUNT(DISTINCT i.nid) FROM (
    WITH RECURSIVE impact AS (
      SELECT e.to_node as nid FROM edges e WHERE e.id = a.edge_id
      UNION
      SELECT e2.to_node FROM edges e2
      JOIN edge_sources es ON es.edge_id = e2.id
      JOIN impact imp ON es.node_id = imp.nid
    ) SELECT nid FROM impact
  ) i) as blast_radius
FROM assumptions a
WHERE a.status = 'valid'
ORDER BY blast_radius DESC;
```

Resultado:

```
Assumption Vulnerability Ranking:
  1. ASM-005 "El board aprobara presupuesto en Q1" — blast radius: 14 nodes
  2. ASM-012 "Proveedor puede escalar a 10K/mes" — blast radius: 11 nodes
  3. ASM-003 "Regulacion no cambiara en 2 anos" — blast radius: 9 nodes
```

La UI muestra los supuestos como un ranking de riesgo. **Se sabe exactamente donde investigar primero.** ASM-005 es donde se deberia pedir knowledge de soporte — si cae, se lleva 14 nodos.

#### 8.3.2 Gaps epistemicos con priorizacion automatica

**Escenario**: 47 nodos. Donde invertir el proximo esfuerzo de investigacion?

**Con ltp-engine-max**:

```sql
SELECT n.id, n.label, n.node_type,
  COUNT(DISTINCT kl.knowledge_id) as knowledge_count,
  (SELECT COUNT(*) FROM edges e
   JOIN edge_sources es ON es.edge_id = e.id
   WHERE es.node_id = n.id) as downstream_edges,
  (SELECT COUNT(*) FROM assumptions a
   JOIN edges e ON e.id = a.edge_id
   JOIN edge_sources es ON es.edge_id = e.id
   WHERE es.node_id = n.id AND a.status = 'valid') as unverified_assumptions
FROM nodes n
LEFT JOIN knowledge_links kl ON kl.target_id = n.id
WHERE n.status = 'active'
GROUP BY n.id
HAVING knowledge_count = 0 AND downstream_edges > 0
ORDER BY downstream_edges DESC, unverified_assumptions DESC;
```

Resultado — lista de prioridades de investigacion:

```
Investigation Priority Queue:
  ! CRITICAL: RC-001 "No hay tracking en tiempo real"
    0 knowledge items, 5 downstream edges, 2 unverified assumptions
    → Critical node with zero evidence. Investigate first.

  ! HIGH: INJ-003 "Implementar sistema GPS"
    0 knowledge items, 3 downstream edges, 1 unverified assumption
    → Injection with no feasibility evidence.

  ~ MEDIUM: UDE-007 "Retrasos en ultima milla"
    0 knowledge items, 1 downstream edge
    → Leaf-ish node, lower blast radius.
```

#### 8.3.3 Red de evidencia por conexion causal

**Escenario**: Un stakeholder cuestiona el CRT. "En que te basas para decir que X causa Y?"

**Con ltp-engine-max**:

```sql
SELECT k.id, k.label, k.knowledge_type, k.status, k.confidence,
       kl.relation, k.source_uri
FROM knowledge k
JOIN knowledge_links kl ON kl.knowledge_id = k.id
WHERE kl.target_id IN (
    SELECT 'LINK-005'
    UNION SELECT node_id FROM edge_sources WHERE edge_id = 'LINK-005'
    UNION SELECT to_node FROM edges WHERE id = 'LINK-005'
    UNION SELECT id FROM assumptions WHERE edge_id = 'LINK-005'
)
ORDER BY k.status, kl.relation;
```

La UI renderiza un panel de evidencia para cada conexion causal:

```
Edge LINK-005: RC-001 → UDE-003
  Evidence:
    ✓ KN-002 (verified, high): "GPS tracking reduces uncertainty 60%"
      Source: McKinsey Supply Chain Report 2025
      Relation: supports
    ✓ KN-008 (verified, medium): "Customer survey: 73% cite uncertainty"
      Relation: supports
    ~ KN-014 (unverified): "Competitor X reduced cancellations 40% with tracking"
      Relation: contextualizes
  Assumptions:
    ASM-003 (valid): "Customers check tracking at least 2x per order"
      No supporting knowledge ← gap
```

---

### 8.4 Agente Autonomo Optimizado

#### 8.4.1 Auto-correccion durante construccion de CRT

**Escenario**: Un agente headless construye un CRT a partir de entrevistas transcritas.

**Hoy**: El agente crea nodos, conecta, valida, y repite. Cada ciclo gasta 20-30 tool calls en redescubrir el estado.

**Con ltp-engine-max**:

```
Loop del agente:
  1. node_add UDE → respuesta incluye: similar nodes (vector), blast radius (relational)
  2. link_connect → respuesta incluye: CLR warnings + knowledge contradictions (vector)
  3. health → una query: cobertura epistemica global
  4. Si cobertura < 60% en RCs → buscar knowledge relevante
  5. Si hay duplicados semanticos → proponer merge

Cada iteracion: 3-5 tool calls (vs 20-30 hoy)
El agente dedica contexto a PENSAR, no a DESCUBRIR ESTADO
```

#### 8.4.2 EC con validacion cruzada de supuestos

**Escenario**: El agente construye una Evaporating Cloud. Al generar supuestos por flecha, el motor automaticamente:

1. Busca supuestos similares en otras ECs (dedup vectorial)
2. Busca knowledge que soporte o contradiga cada supuesto (cruce vectorial)
3. Rankea por fragilidad (impacto relacional)

**Hoy**: El agente genera supuestos, pero no puede cruzarlos contra knowledge ni contra supuestos previos sin cargar todo en contexto.

**Con ltp-engine-max**: El `assume_add` devuelve:

```json
{
  "success": true,
  "data": {
    "created_assumption_id": "ASM-025",
    "text": "El proveedor puede escalar produccion en 30 dias"
  },
  "warnings": [
    {
      "code": "SIMILAR_ASSUMPTION",
      "detail": "ASM-012 (0.91 cosine): 'Proveedor puede escalar a 10K/mes'",
      "tree": "tree-ec-capacidad"
    },
    {
      "code": "CONTRADICTING_KNOWLEDGE",
      "detail": "KN-008 (verified): 'Lead time minimo del proveedor: 72h para cambios de volumen'",
      "relation": "contradicts"
    }
  ]
}
```

El agente recibe la contradiccion **en la misma respuesta**, sin hacer queries adicionales. Puede decidir inmediatamente si invalidar el supuesto.

#### 8.4.3 Multi-agente coordinado

**Escenario futuro**: Tres agentes trabajan en paralelo sobre el mismo workspace:
- Agente A: construye el CRT
- Agente B: enriquece el knowledge pool desde documentos
- Agente C: audita CLR en los edges existentes

**Hoy**: Imposible. Un workspace con archivos JSON y lock file no soporta escritura concurrente segura.

**Con ltp-engine-max**: SQLite WAL mode permite un writer y multiples readers simultaneamente. Con transacciones ACID:
- Agente B puede leer nodos mientras A escribe nuevos
- Agente C puede leer edges mientras B agrega knowledge
- Writes se serializan via WAL pero cada transaccion es atomica

El knowledge que B descubre aparece inmediatamente en las queries de A (contradicciones, soporte). Los warnings de C informan las decisiones de A. **El workspace es un espacio de trabajo compartido en tiempo real.**

---

### 8.5 Analisis de Portafolio (Cross-Workspace)

#### 8.5.1 Patrones recurrentes entre proyectos

**Escenario**: 6 meses usando ltp-engine-max. 5 workspaces de distintos analisis LTP. Hay patrones recurrentes?

**Con multiples `.db` adjuntados**:

```sql
ATTACH 'workspace_logistica.db' AS w1;
ATTACH 'workspace_ventas.db' AS w2;
ATTACH 'workspace_produccion.db' AS w3;

SELECT a.label, b.label, vector_distance_cos(a.embedding, b.embedding) as dist
FROM w1.nodes a, w2.nodes b
WHERE a.node_type = 'RC' AND b.node_type = 'RC'
AND dist < 0.2;
```

Resultado: *"La falta de datos en tiempo real aparece como root cause en 4 de 5 analisis. Es un problema sistemico de la organizacion, no de un departamento."*

**Meta-analisis que hoy es completamente manual.**

#### 8.5.2 Libreria de inyecciones probadas

**Escenario**: Se construye un nuevo FRT. Hay inyecciones de analisis anteriores que apliquen?

```sql
SELECT inj.label, inj.id, k.label as evidence,
       vector_distance_cos(inj.embedding, ?) as relevance
FROM other_workspace.nodes inj
JOIN other_workspace.knowledge_links kl ON kl.target_id = inj.id
JOIN other_workspace.knowledge k ON k.id = kl.knowledge_id
WHERE inj.node_type = 'INJ' AND k.status = 'verified'
ORDER BY relevance LIMIT 5;
```

"En el analisis de ventas de hace 3 meses, la inyeccion 'Implementar dashboard de datos en tiempo real' fue validada con knowledge verificada. Aplica aqui?"

---

### 8.6 Tabla Resumen: Antes Imposible, Ahora Posible

| Caso de uso | Sin ltp-engine-max | Con ltp-engine-max |
|-------------|-------------------|-------------------|
| Deteccion de duplicados mientras escribes | Imposible | Milisegundos, cada keystroke |
| Contradiccion knowledge↔assumption en vivo | Imposible sin LLM cargando todo | Query vectorial instantanea |
| Sugerencia AND/OR semantica | Warning generico sin contexto | Comparacion vectorial entre causas |
| Mapa de calor epistemico visual | Imposible | Una query SQL por arbol |
| Linaje completo cross-tree (UDE→accion) | 5-6 tree_walks manuales, propenso a error | Una query recursiva, completa |
| Nodos que deberian compartirse pero no lo estan | Invisible | Deteccion vectorial automatica |
| Convergencia de root causes entre CRTs | Horas de analisis manual | Una query de clustering |
| Ranking de vulnerabilidad de supuestos | Imposible | JOIN recursivo + blast radius |
| Priorizacion de investigacion epistemica | Juicio humano sin datos | Query con scoring automatico |
| Red de evidencia por conexion causal | Cargar todo en contexto LLM | Panel instantaneo |
| Agente con auto-correccion semantica | 20-30 tool calls por ciclo | 3-5 tool calls con warnings inline |
| EC con validacion cruzada de supuestos | Tool calls extra por cada supuesto | Incluido en la respuesta de assume_add |
| Multi-agente coordinado | Imposible (lock file) | WAL mode + ACID |
| Meta-analisis cross-workspace | Completamente manual | ATTACH + queries vectoriales |
| Libreria de inyecciones probadas | No existe | Busqueda semantica cross-workspace |

La mayoria de estos no son "lo mismo pero mas rapido" — son capacidades que **no existen** en ninguna forma en la arquitectura actual.

---

## 9. Configuracion por Capacidades

```json
{
  "workspace": "mi-proyecto-ltp",
  "index": {
    "enabled": true
  },
  "vectors": {
    "enabled": true,
    "provider": "api",
    "api": {
      "model": "openai:text-embedding-3-small",
      "dimensions": 1536
    }
  },
  "sync": {
    "enabled": false
  }
}
```

Cada seccion es independiente con defaults seguros:

| Seccion | Default | Controla |
|---------|---------|----------|
| `index` | `enabled: true` | Indice relacional. Si `false`, el DB existe pero sin indices avanzados. |
| `vectors` | `enabled: false` | Busqueda semantica. Requiere provider configurado. |
| `vectors.provider` | — | `"local"` (modelo en maquina), `"api"` (OpenAI/Voyage) |
| `sync` | `enabled: false` | Replicacion Turso |

### Perfiles por maquina

**Mac (desarrollo principal)**:
```json
{
  "index": { "enabled": true },
  "vectors": {
    "enabled": true,
    "provider": "local",
    "local": { "model": "all-MiniLM-L6-v2", "dimensions": 384 }
  }
}
```

**Dell XPS 9343 (secundario, Linux Mint)**:
```json
{
  "index": { "enabled": true },
  "vectors": {
    "enabled": true,
    "provider": "api",
    "api": { "model": "openai:text-embedding-3-small", "dimensions": 1536 }
  }
}
```

**Maquina minima / CI**:
```json
{
  "index": { "enabled": true },
  "vectors": { "enabled": false }
}
```

**Futuro con home server**:
```json
{
  "index": { "enabled": true },
  "vectors": { "enabled": true, "provider": "api", "api": { "model": "..." } },
  "sync": {
    "enabled": true,
    "turso_url": "libsql://mi-workspace.turso.io",
    "turso_token": "eyJ..."
  }
}
```

### Degradacion elegante

```
┌─────────────────────────────────────────┐
│  Sync (Turso)                           │  ← desactivar: trabajas solo local
│  ┌───────────────────────────────────┐  │
│  │  Vectors (local/api)              │  │  ← desactivar: pierdes semantica
│  │  ┌─────────────────────────────┐  │  │
│  │  │  Relational (queries, JOINs)│  │  │  ← siempre presente con el DB
│  │  │  ┌───────────────────────┐  │  │  │
│  │  │  │  workspace.db (core)  │  │  │  │  ← siempre funciona
│  │  │  └───────────────────────┘  │  │  │
│  │  └─────────────────────────────┘  │  │
│  └───────────────────────────────────┘  │
└─────────────────────────────────────────┘
```

### Diagnostico: `ltp status` enriquecido

```
Workspace: mi-proyecto-ltp (workspace.db — 1.2 MB)
  Nodes: 47  Trees: 5  Knowledge: 12

Capabilities:
  ✓ Relational index (always on)
  ✓ Vector search (provider: local, model: all-MiniLM-L6-v2)
    Embeddings: 47/47 nodes, 23/23 assumptions, 12/12 knowledge
  ✗ Sync (disabled)

Epistemic Health:
  UDE: 12/15 (80%) backed    RC: 4/8 (50%) ← gap
  INJ: 2/6  (33%) ← critical ASM: 18/30 (60%)
```

---

## 10. Requisitos de Hardware

### Por capa

| Capa | CPU | RAM | Disco | GPU | Red |
|------|-----|-----|-------|-----|-----|
| Core (workspace.db) | Cualquiera | < 100 MB | ~2 MB/1000 nodos | No | No |
| + Vectores API | Cualquiera | < 100 MB | +1.5 MB vectores/1000 nodos | No | Si |
| + Vectores local | x86_64/ARM64 moderno | 500 MB - 1 GB | +100-200 MB modelo | Opcional (10-50x aceleracion) | Solo descarga inicial |
| + Turso sync | Cualquiera | < 200 MB | Replica local | No | Si |

### En tu hardware

| Maquina | Config recomendada | Rendimiento |
|---------|-------------------|-------------|
| **Mac (Apple Silicon)** | Todo local (modelo + vectores) | Excelente — Metal acelera embeddings a ~5ms |
| **Dell XPS 9343 (8 GB)** | DB local + vectores via API | Bueno — DB vuela en SSD, embeddings via red |
| **Dell XPS 9343 (4 GB)** | DB local + vectores via API | Bueno — mismo que 8 GB (API no consume RAM local) |
| **Home server (futuro)** | Hub Turso (sqld self-hosted) | Sync entre Mac y Dell |

---

## 11. Analisis Six Thinking Hats — Fork Completo

### Blanco (Datos)

- La UI en desarrollo paralelo elimina la necesidad de inspeccionar JSON a mano.
- libSQL es async-only — el motor nace async con tokio. Es decision de diseno, no parche.
- Un `workspace.db` con 1000 nodos + vectores: ~5 MB. Trivial.
- SQLite es el formato de archivo mas desplegado del mundo.
- libsql es open source (MIT). sqld es open source. Si Turso cierra, el software sigue.

### Rojo (Intuicion)

Se siente como lo correcto para donde va el proyecto. ltp-engine empezo como CLI tool para un desarrollador; ltp-engine-max es el motor para un producto con UI, agentes, y potencialmente multiples usuarios. El JSON-first era la decision correcta para el v1; el DB-first es la decision correcta para el v2.

### Negro (Critico — Obligatorio)

| Riesgo | Severidad | Mitigacion |
|--------|-----------|------------|
| **Dependencia pre-v1 en el core** | MEDIA-ALTA | Schema SQL es compatible con SQLite. Migrar a rusqlite = cambiar crate, no schema. Export JSON como escape hatch. |
| **Reescritura de persistencia completa** | ALTA (esfuerzo) | Es un fork, no migracion in-place. ltp-engine original sigue funcionando. |
| **Undo/redo rediseñado** | MEDIA (esfuerzo) | Journal table es mas robusto que snapshots. ACID > SHA-256 checksums. |
| **Git no trackea .db** | MEDIA | `ltp export` + pre-commit hook. O journal interno como changelog. |
| **Turso (empresa) podria cerrar** | BAJA | Open source (MIT). Datos locales. sqld self-hosteable. |
| **Binario ~20-25 MB** | BAJA | Aceptable para un motor con estas capacidades. |
| **Tokio como dependencia** | BAJA en fork | El motor nace async. No es contaminacion de un motor sync. |
| **Concurrencia (SQLite single writer)** | BAJA | WAL mode. Single-user es el caso principal. |
| **Corrupcion del .db** | BAJA | `PRAGMA integrity_check`. Backups automaticos. Export como safety net. |

### Amarillo (Beneficios)

| Beneficio | Impacto |
|-----------|---------|
| Cero sincronizacion — un solo store | Elimina el drawback #1 del hibrido |
| ACID real — batches son transacciones | Elimina estado parcial |
| Vectores inline — F32_BLOB en columna | Una insert = dato + embedding, atomico |
| DiskANN integrado | Mejor k-NN que sqlite-vec, sin unsafe |
| Replica embebida | Path directo a multi-dispositivo |
| 6 precisiones de vector | Optimizar storage/precision por caso |
| Un solo crate | No rusqlite + sqlite-vec + unsafe loading |
| UI + Agent + CLI sobre mismo backend | Consistencia garantizada |
| Feedback semantico en vivo | Queries vectoriales en ms, sin LLM |
| Agente headless optimo | ~60-80 tool calls vs ~400-500 |

### Verde (Alternativas y diseno)

**Patron Rust**:
```rust
pub struct Engine {
    db: libsql::Connection,
    embedder: Box<dyn EmbeddingProvider>,
}

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EngineError>;
    fn dimensions(&self) -> usize;
}

pub struct NoOpEmbedder;          // Default — columnas vector quedan NULL
pub struct LocalEmbedder { ... }  // all-MiniLM via ort/candle
pub struct ApiEmbedder { ... }    // OpenAI/Voyage via HTTP

// CLI entry point
#[tokio::main]
async fn main() {
    let args = Cli::parse();
    let engine = Engine::open(&args.workspace).await?;
    let result = engine.execute(args.command).await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
}
```

### Azul (Coherencia Arquitectonica)

| Principio original | Se mantiene en el fork? |
|-------------------|------------------------|
| Motor determinista, semantica en el agente | **Si** — IDs secuenciales, queries deterministas. Agente sigue haciendo semantica CLR. |
| Type-first | **Si** — traits `EmbeddingProvider`, structs antes de logica. |
| Sin `.unwrap()` | **Si** — `libsql::Error` se mapea a `EngineError` via `thiserror`. |
| Errores tipados | **Si** — mismo enum `EngineError`, mas variantes para DB. |
| IDs secuenciales | **Si** — tabla `counters` reemplaza `counters.json` con misma semantica. |
| Contrato de salida JSON | **Si** — los commands siguen devolviendo `{success, action, data, errors, warnings}`. |
| Undo/redo | **Rediseñado** — journal transaccional reemplaza snapshots. Mas robusto. |
| Persistencia unidireccional | **Evolucionado** — de JSON unidireccional a DB ACID. El principio (un sitio canonical) se mantiene reforzado. |

---

## 12. Relacion con ltp-engine Original

ltp-engine-max es un **friendly fork**, no un reemplazo:

| Aspecto | ltp-engine (original) | ltp-engine-max (fork) |
|---------|----------------------|----------------------|
| Persistencia | JSON en disco | workspace.db (libSQL) |
| Runtime | Sync | Async (tokio) |
| Vectores | No | Nativos (F32_BLOB, DiskANN) |
| Transparencia | Alta (archivos legibles) | Via UI + export |
| Complejidad | Baja | Media-alta |
| Target | CLI headless, simplicidad maxima | UI + agente + CLI, capacidades avanzadas |

**Compatibilidad**:
- `ltp import` migra workspaces de ltp-engine original a ltp-engine-max
- `ltp export` genera la estructura de archivos clasica
- Las 54 herramientas MCP mantienen la misma interfaz — el agente no nota la diferencia
- El contrato de salida JSON no cambia

ltp-engine original sigue siendo valido para uso CLI puro donde la simplicidad es prioridad.

---

## 13. Preguntas Abiertas

1. **Nombre definitivo**: ltp-engine-max, ltp-engine-v2, ltp-core, otro?
2. **Modelo de embeddings por defecto**: local (all-MiniLM, zero-config) vs API (mas preciso, requiere key)?
3. **Dimension de embeddings**: 384 (MiniLM) vs 1536 (OpenAI)? Afecta tamano y precision. Configurable?
4. **Threshold de similaridad**: configurable o hardcoded? El umbral de "duplicado" puede variar por dominio.
5. **Git strategy para el fork**: nuevo repo, o branch del existente?
6. **Schema migrations**: como manejar cambios de schema entre versiones del fork?
7. **Undo depth**: limite de entradas en el journal? Purgar automaticamente?
8. **MCP interface**: mismas 54 tools + nuevas, o reorganizar?
9. **Runtime de embeddings local**: `ort` (ONNX Runtime, C++ deps) vs `candle` (Rust puro, HuggingFace)?
10. **Test strategy**: como testear sin depender de un modelo de embeddings real?

---

## 14. Cronologia de Exploracion

| Fecha | Evento |
|-------|--------|
| 2026-09-10 | Exploracion inicial: Turso como BBDD vectorial para ltp-engine |
| 2026-09-10 | Diseno de arquitectura hibrida (JSON + indice derivado) |
| 2026-09-10 | Six Hats sobre el hibrido: sincronizacion como drawback principal |
| 2026-09-10 | Inventario completo del modelo de datos actual (13 entidades) |
| 2026-09-10 | 8 capacidades nuevas identificadas (5 relacionales + 3 vectoriales) |
| 2026-09-11 | Analisis profundo de drawbacks (10 riesgos identificados) |
| 2026-09-11 | Analisis sync vs async: rusqlite vs libsql vs rusqlite+sqlite-vec |
| 2026-09-11 | Decision: rusqlite para hibrido (descartando libsql async para indice) |
| 2026-09-11 | Evaluacion de ventajas para agente headless: indice como infraestructura |
| 2026-09-11 | Pivot: fork con DB como source of truth (libsql/Turso full power) |
| 2026-09-11 | Contexto UI: tres consumidores (UI + MCP + CLI) eliminan tension human/agent |
| 2026-09-11 | Espectro de autonomia: feedback semantico en vivo en la UI |
| 2026-09-11 | Requisitos de hardware: Mac + Dell XPS 9343 + futuro home server |
| 2026-09-11 | Configuracion por capacidades: degradacion elegante por maquina |
| 2026-09-11 | Convergencia: ltp-engine-max como friendly fork con Turso completo |

---

## Changelog

| Fecha | Cambio |
|-------|--------|
| 2026-09-10 | Creacion inicial: arquitectura hibrida (JSON + indice derivado) |
| 2026-09-11 | Reescritura completa: convergencia en fork con Turso (ltp-engine-max) |
| 2026-09-11 | Seccion 8 expandida: 15 casos de uso en profundidad con queries SQL concretas |
