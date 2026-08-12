# Especificación del Motor Determinista (ENGINE_SPEC.md)

**Estado**: Especificación de Producción

**Filosofía**: Núcleo reducido de primitivas atómicas de grafo + Componibilidad total para Agentes y Scripts + Determinismo absoluto en Rust.

## 1. Principio de Diseño: Primitivas vs. Composición

El motor `ltp-engine` NO intenta adivinar flujos de trabajo ni empaquetar comandos complejos de orquestación. En su lugar, proporciona un conjunto de primitivas de grafo y algoritmos de recorrido.

```
┌────────────────────────────────────────────────────────────────────────┐
│                      AGENTE LLM / SCRIPT USER                          │
│  • Compone flujos de trabajo (CRT -> EC -> FRT) usando primitivas      │
│  • Evalúa la semántica del dominio (Dettmer CLR)                       │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ Invoca Primitivas (JSON)
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│                   MOTOR CLI EN RUST (`ltp-engine`)                        │
│                                                                        │
│  CAPA DE NAVEGACIÓN (determinista, sirve datos)                        │
│    Selección:   node inspect, link inspect, link find                  │
│    Recorrido:   tree walk (topological / reverse)                      │
│    Vecindad:    trace --depth 1 (upstream / downstream)                │
│    Estructura:  validate (ciclos, integridad, linter CLR sintáctico)   │
│    Contexto:    status, tree list, nbr inspect                         │
│                                                                        │
│  CAPA DE MANIPULACIÓN (determinista, muta estado)                      │
│    Entidades:   node add/edit/rm/split, tree new/rm/clone              │
│    Vistas:      tree attach/detach                                     │
│    Enlaces:     link connect/disconnect/reverse/move/insert-between    │
│    Agrupación:  link group/dissolve/split/reoperator/add-cause/rm-cause│
│    Supuestos:   assume add/edit/rm/list/move, invalidate               │
│    Abstracción: path collapse/explode/replace                          │
│    NBR:         nbr add/list/inspect                                   │
│    Feedback:    link feedback                                          │
│                                                                        │
│  CAPA DE ANÁLISIS SEMÁNTICO (LLM / Humano — fuera del motor)          │
│    El motor NUNCA juzga causalidad, claridad ni suficiencia.           │
│    Solo sirve datos estructurados para que el auditor evalúe.          │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ Modifica / Valida
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│                   WORKSPACE CANÓNICO EN DISCO                           │
│  • /nodes/<ID>.json  • /trees/<ID>.json  • ltp.config.json             │
└────────────────────────────────────────────────────────────────────────┘
```

## 2. Comandos del Motor

Toda la interacción con el sistema se expone tanto en la CLI de Rust como en el Servidor MCP. Todos los comandos soportan `--json`.

---

### 2.1. Workspace

#### `ltp init [--name <nombre>]`

Inicializa la estructura del workspace (`nodes/`, `trees/`, `ltp.config.json`) e inicializa Git si no existe.

#### `ltp status [--json]`

Diagnóstico de salud determinista del workspace:
- Reporta nodos huérfanos (no conectados a ninguna vista).
- Reporta causas raíz sin resolver o supuestos invalidados.
- Retorna el recuento global de entidades por tipo.
- Reporta feedback loops (cantidad, tipo positive/negative).

---

### 2.2. Entidades (Pool Global)

#### `ltp node add "<label>" --type <TYPE> [--tags t1,t2] [--observable true|false]`

Tipos: `UDE | RC | INJ | NC | GOAL | OBJ | WANT | OBS | IO | INT | DE | REQ | PRE`

"Con Dientes": ejecuta un linter sintáctico suave. Advierte si el texto contiene conjunciones causales prohibidas por CLR #2 (`because`, `in order to`, `para`), sugiriendo dividir la idea.

#### `ltp node edit <ID> [--label "<texto>"] [--add-tag <tag>] [--rm-tag <tag>] [--observable true|false]`

#### `ltp node rm <ID>[,<ID2>,<ID3>] [--force]`

Elimina nodos del pool global y todos sus edges asociados en todas las vistas. Batch: acepta lista separada por comas.

#### `ltp node inspect <ID>`

Muestra en qué árboles participa, con qué rol en cada uno, y sus conexiones.

#### `ltp node list --tree <TREE_ID> [--type UDE,RC,...] [--status active,draft,...]`

Lista nodos del tree filtrable por tipo y/o status.

#### `ltp node search --tree <TREE_ID> --query "<texto>"`

Busca nodos por contenido de label (substring match).

#### `ltp node split <ID> --into "<label_1>" "<label_2>" --tree <TREE_ID>`

Divide una entidad con dos ideas en dos nodos. Hereda conexiones entrantes al primer nodo y salientes al segundo. Elimina el nodo original. Caso de uso principal: CLR #2 (entidad con ideas mezcladas).

---

### 2.3. Vistas (Trees)

#### `ltp tree new <gt|crt|ec|frt|prt|tt> "<nombre>"`

#### `ltp tree list`

#### `ltp tree rm <TREE_ID>`

#### `ltp tree attach --tree <TREE_ID> --node <ID> [--role "<role>"]`

Añade un nodo del pool a un tree sin conectarlo (staging). Aparece como huérfano dentro del tree. `status` lo reporta.

#### `ltp tree detach --tree <TREE_ID> --node <ID>`

Quita un nodo de una vista (y sus edges en ese tree), pero lo preserva en el pool global `/nodes/`.

#### `ltp tree clone <TREE_ID> --name "<nuevo_nombre>"`

Crea una copia del tree con nuevo ID. Los nodos son referencias compartidas al pool (mismo ref). Los edges son independientes — se puede reorganizar sin afectar el original. Para exploración "what-if".

#### `ltp tree diff <TREE_A> <TREE_B>`

Reporta diferencias entre dos trees: nodos añadidos/quitados, edges añadidos/quitados/modificados, cambios de operador.

#### `ltp tree walk <TREE_ID> [--order topological|reverse] [--show-origin] [--expand-nbr] [--json]`

Recorrido ordenado del árbol completo para auditoría sistemática. Devuelve cada nodo con su contexto inmediato (edges entrantes/salientes, operator, assumptions) en orden de recorrido:

- `--order topological` (default en árboles de suficiencia): desde causas raíz hacia efectos (bottom-up).
- `--order reverse` (default en árboles de necesidad): desde objetivo hacia prerrequisitos (top-down).
- `--show-origin`: añade al output de cada nodo en qué otros trees participa y con qué rol.
- `--expand-nbr`: expande nbr_branches inline en vez de mostrarlas colapsadas.

Incluye feedback_edges al final como sección separada.

---

### 2.4. Enlaces — Conexión y Desconexión

#### `ltp link connect --tree <TREE_ID> --from <ID1>[,<ID2>] --to <ID3>[,<ID4>] [--operator SINGLE|AND|OR|MAG|XOR] [--weight 0.0-1.0]`

Establece una conexión causa-efecto en una vista:
- Si `--from` recibe múltiples IDs con `--operator AND`, genera la elipse de suficiencia conjuntiva.
- Si `--to` recibe múltiples IDs, crea un edge SINGLE de la causa hacia cada destino.
- `--weight` solo aplica con operator MAG; warning si se omite con MAG.
- Valida integridad referencial (falla si un nodo no existe en `/nodes/`).

#### `ltp link disconnect --tree <TREE_ID> --links <LINK_ID>[,<LINK2>,<LINK3>]`

Elimina uno o más edges. Batch: acepta lista separada por comas.

#### `ltp link feedback --tree <TREE_ID> --from <ID1> --to <ID2> --type <positive|negative> [--label "<texto>"]`

Crea una arista de retroalimentación (feedback loop) en el pool `feedback_edges` del tree. No participa en la validación DAG. Disponible en árboles de suficiencia (CRT, FRT, TT).

---

### 2.5. Enlaces — Inspección y Búsqueda

#### `ltp link inspect <LINK_ID> --tree <TREE_ID>`

Muestra el detalle completo de un edge: from (con labels), to (con label), operator, weight, status, logic, y la lista completa de assumptions con su status.

#### `ltp link find --tree <TREE_ID> --from <NODE_ID> --to <NODE_ID>`

Encuentra edge(s) entre dos nodos. Devuelve detalle completo de cada match.

---

### 2.6. Enlaces — Manipulación

#### `ltp link reverse --tree <TREE_ID> --link <LINK_ID> [--force]`

Invierte la dirección de un edge (from↔to). Preserva operator y assumptions pero las marca con `status: "needs_review"`. Si el edge tiene assumptions, requiere `--force` (safety: la semántica del supuesto puede cambiar al invertir la dirección).

#### `ltp link move --tree <TREE_ID> --link <LINK_ID> [--new-from <ID>] [--new-to <ID>]`

Redirige un edge existente a otro nodo origen y/o destino.

#### `ltp link insert-between --tree <TREE_ID> --link <LINK_ID> --node <ID> [--insert-after-cause <CAUSE_ID>] [--insert-before-effect]`

Inserta un nodo intermedio en un edge existente. Para edges SINGLE: A→B se convierte en A→C→B (dos edges nuevos, se elimina el original).

Para edges AND:
- `--insert-after-cause <ID>`: extrae esa causa del grupo, crea CauseX→Nodo, y Nodo se añade al grupo original en su lugar.
- `--insert-before-effect`: `[A, B] --AND--> C` se convierte en `[A, B] --AND--> Nodo` + `Nodo → C`.

---

### 2.7. Enlaces — Agrupación de Operadores

#### `ltp link group --tree <TREE_ID> --links <L1>,<L2>[,<L3>] --operator <AND|OR|MAG|XOR>`

Agrupa edges SINGLE independientes que van al mismo nodo destino bajo un operador. Falla si los edges no comparten el mismo `to`. Crea un solo edge con múltiples entradas en `from[]`. Elimina los edges originales.

#### `ltp link dissolve --tree <TREE_ID> --link <LINK_ID>`

Disuelve un grupo: cada causa vuelve a ser un arrow SINGLE independiente. Assumptions se heredan a cada nuevo edge con `status: "needs_review"`.

#### `ltp link split --tree <TREE_ID> --link <LINK_ID> --extract <ID1>[,<ID2>]`

Extrae causas de un grupo sin disolverlo completamente. El grupo original queda reducido. Las causas extraídas forman un nuevo edge independiente al mismo destino (operator SINGLE si es 1 causa, se puede especificar con `--new-operator`). Si el grupo queda con 1 sola causa, se convierte automáticamente en SINGLE.

#### `ltp link reoperator --tree <TREE_ID> --link <LINK_ID> --operator <NUEVO>`

Cambia el operador de un edge. Reglas:
- Si destino es MAG: warning si no hay weights; asigna `null` con advertencia "pendiente de estimar".
- Si origen es MAG y destino no: descarta weights silenciosamente.
- SINGLE→AND/OR/MAG/XOR: solo válido si el edge ya tiene múltiples `from`.
- AND/OR/MAG/XOR→SINGLE: solo válido si el edge tiene un solo `from` (usar `dissolve` primero si tiene múltiples).

#### `ltp link add-cause --tree <TREE_ID> --link <LINK_ID> --node <ID> [--weight 0.0-1.0]`

Añade un nodo a un edge AND/OR/MAG existente (expande el `from[]`). Si el edge es SINGLE, lo convierte automáticamente al operator especificado (requiere `--promote-to <AND|OR|MAG|XOR>`).

#### `ltp link rm-cause --tree <TREE_ID> --link <LINK_ID> --node <ID>`

Saca un nodo de un edge AND/OR/MAG (reduce el `from[]`). Si queda 1 sola causa, convierte a SINGLE automáticamente.

---

### 2.8. Supuestos (Assumptions)

#### `ltp assume add --tree <TREE_ID> --link <LINK_ID> --text "<texto_supuesto>"`

Adjunta un supuesto explícito a un edge. Genera un ID único (ej. `ASM-001`).

#### `ltp assume edit --tree <TREE_ID> --asm <ASM_ID> --text "<nuevo_texto>"`

Edita el texto de un supuesto existente.

#### `ltp assume rm --tree <TREE_ID> --asm <ASM_ID>`

Elimina un supuesto.

#### `ltp assume list --tree <TREE_ID> [--status valid|invalid|needs_review]`

Lista todos los supuestos de un tree, filtrable por status.

#### `ltp assume move --tree <TREE_ID> --asm <ASM_ID> --to-link <LINK_ID>`

Mueve un supuesto de un edge a otro dentro del mismo tree.

#### `ltp invalidate --tree <TREE_ID> --link <LINK_ID> --asm <ASM_ID> [--injection "<label_nueva_inj>"]`

Operación de ruptura lógica:
- Marca el supuesto como `invalid`.
- Marca el enlace como `broken`.
- Opcionalmente, crea en `/nodes/` el borrador de una nueva Inyección (INJ) vinculada a la ruptura.

---

### 2.9. Exploración y Trazado

#### `ltp trace <NODE_ID> --tree <TREE_ID> --direction <upstream|downstream> [--depth N] [--no-feedback] [--nbr]`

Motor de exploración del grafo:
- `--direction upstream`: recorre los predecesores buscando causas raíz (RC / INJ).
- `--direction downstream`: recorre los sucesores midiendo el impacto hasta los síntomas (UDE / DE).
- `--depth N`: filtra por profundidad.
- Incluye `feedback_edges` por defecto; excluir con `--no-feedback`.
- Con `--nbr` incluye también los edges de las NBR branches.

---

### 2.10. Abstracción y Mutación

#### `ltp path collapse --tree <ID> --from <ID1> --to <ID2> --label "<macro_label>"`

Calcula la ruta entre los dos nodos, identifica todos los nodos y links interiores, y genera una entrada `macro_edge` en la vista ejecutiva sin alterar los nodos tácticos en disco. El macro_edge incluye `interior_nodes` e `interior_links` para que un renderer pueda detectar edges periféricos (aquellos cuyo from o to toca un nodo interior desde fuera del bloque colapsado).

#### `ltp path explode --tree <ID> --link <LINK_ID> --asm <ASM_ID> --label "<texto_nuevo_nodo>"`

Desglosa un supuesto convirtiéndolo en un nodo intermedio explícito (INT) dentro de la cadena causa-efecto.

#### `ltp path replace --tree <ID> --macro-link <MACRO_ID> --by-node <NODE_ID>`

Reemplaza un sub-grafo colapsado por una Inyección, marcando la cadena táctica previa como `superseded`.

---

### 2.11. Negative Branch Reservations

#### `ltp nbr add --tree <TREE_ID> --source-node <NODE_ID> [--trim <INJ_NODE_ID>]`

Crea una NBR vacía vinculada a un nodo fuente (típicamente una inyección del FRT). Opcionalmente asigna la inyección de trimming. Los edges de la NBR se crean con `ltp link connect --tree <TREE_ID> --nbr <NBR_ID> ...`.

#### `ltp nbr list --tree <TREE_ID>`

Lista las NBR de un tree con su nodo fuente, cantidad de edges y si tiene trim_injection asignada.

#### `ltp nbr inspect <NBR_ID> --tree <TREE_ID>`

Muestra la cadena causal completa de una NBR: edges, nodos involucrados y trim injection.

---

### 2.12. Validación (Linter Determinista)

#### `ltp validate [--tree <TREE_ID>]`

Ejecuta validaciones en dos niveles:

**Bloqueantes (errors):**
- DFS de 3 colores sobre `edges` (excluye `feedback_edges`): verifica que los árboles de suficiencia (CRT, FRT, TT) sean DAGs puros. Retorna `CIRCULAR_DEPENDENCY_DETECTED` si hay ciclos.
- Valida edges dentro de cada `nbr_branches[]` como DAGs independientes.
- Integridad referencial: todo nodo referenciado en edges existe en `/nodes/`.
- EC: exactamente 1 nodo con role `"objective"`, al menos 2 con role `"requirement"` vinculados al objective, al menos 1 `"prerequisite"` por cada requirement, al menos 1 conector XOR entre prerrequisitos incompatibles. Soporta N ramas. Los nodos referenciados pueden ser de cualquier tipo del pool global — el role es contextual a la vista.

**Advertencias (warnings):**
- Linter CLR #2: conjunciones causales prohibidas en labels (`because`, `in order to`, `para`, `y` como causal). Se ejecuta sobre todos los nodos del tree, no solo al crearlos.
- Nodos con solo 1 entrada SINGLE: candidatos a insuficiencia (CLR #4).
- Elipses AND con >4 entradas: posible mezcla de causas independientes (CLR #4/#5).
- Nodos con `observable: false` y <2 edges salientes: candidatos a CLR #7 (causa intangible sin efecto predicho).
- Inversión de tipos sospechosa (CLR #6): nodo de nivel alto (UDE, DE) en posición `from` apuntando a nodo de nivel bajo (RC, INT).
- Nodos huérfanos dentro del tree (attached pero sin edges).

---

### 2.13. Historial — Undo / Redo

El motor mantiene un stack lineal (LIFO) de snapshots en `.ltp/undo/` que permite deshacer y rehacer operaciones con garantía de correctitud.

**Principios de diseño:**
- Todo comando de la Capa de Manipulación genera una entrada de historial (los de Navegación no).
- El restore es por snapshot de ficheros — no hay lógica de inversas calculadas.
- Checksums SHA-256 detectan divergencias externas (Git, edición manual). El motor rechaza undo/redo si el estado actual no coincide con lo esperado.
- Undo es todo-o-nada (atómico cross-file). Write-then-rename: se escriben ficheros restaurados a `.ltp/tmp/`, y solo cuando todos están listos se renombran al destino.
- Stack estrictamente lineal: no se puede deshacer operación N sin deshacer N+1 primero. Esto garantiza integridad referencial del grafo en todo momento.

#### `ltp undo [--dry-run]`

Restaura el estado previo a la última operación.
- Verifica `after_hash` de cada fichero afectado contra el estado actual en disco.
- Si coincide: restaura `before`. Si `before: null`: elimina el fichero (fue una creación).
- Si no coincide: `UNDO_STATE_DIVERGED` con detalle del fichero divergente. Sugiere `ltp history check`.
- `--dry-run`: muestra qué se deshará sin ejecutar.
- Mueve la entrada del undo stack al redo stack.
- Cualquier operación mutante nueva vacía el redo stack.

#### `ltp redo [--dry-run]`

Re-aplica la última operación deshecha.
- Verifica `before_hash` de cada fichero antes de reaplicar.
- Si diverge: `REDO_STATE_DIVERGED`.
- `--dry-run`: muestra qué se reharía.

#### `ltp history [--last N]`

Muestra el historial de operaciones (undo stack). Para cada entrada: seq, timestamp, action, command.

#### `ltp history check`

Valida la integridad del stack completo contra el estado actual en disco. Reporta qué entradas siguen siendo válidas (checksum cuadra) y cuáles están rotas (divergencia). Útil después de operaciones Git o ediciones manuales.

#### `ltp history invalidate [--from <seq>]`

Descarta entradas del undo stack desde el punto de divergencia. Las anteriores que sigan cuadrando se preservan.

#### `ltp history begin-batch --label "<descripción>"`

Inicia un batch: guarda un snapshot completo de todos los ficheros del workspace ANTES de la primera operación. Las operaciones dentro del batch NO generan entradas individuales.

#### `ltp history end-batch`

Cierra el batch. Genera una sola entrada en el undo stack con el snapshot del `begin-batch`. Un solo `ltp undo` deshace todo el batch entero.

#### `ltp history clear`

Limpia el historial completo (libera storage).

**Concurrencia — Lock file:**

Todo comando mutante adquiere `.ltp/lock` antes de ejecutar:

```json
{"pid": 12345, "timestamp": "2026-08-11T10:30:00Z", "command": "link connect ..."}
```

- Si el lock existe y el PID sigue vivo: `WORKSPACE_LOCKED` (espera o falla).
- Si el lock existe y el PID no está vivo: lock stale → auto-break con warning `STALE_LOCK_REMOVED`.
- El lock se libera al finalizar la operación (incluyendo escritura de undo entry).

**Configuración (`ltp.config.json`):**

```json
{
  "history": {
    "max_size_mb": 5,
    "enabled": true
  }
}
```

- `max_size_mb`: tamaño máximo del undo stack. Cuando se supera, se descartan las entradas más antiguas (FIFO de rotación). Default: 5MB.
- `enabled`: permite desactivar historial completamente (workspaces descartables).

**Reglas:**
- `ltp init` no genera entrada de undo (el sistema no existe antes de init). Sí genera `.ltp/` en `.gitignore`.
- Rutas en las entradas de undo son relativas al workspace root (portabilidad).
- Undo de `invalidate` resucita assumptions: restaura el assumption a `valid`, el link a `active`, y elimina la inyección borrador si fue creada por esa operación.

---

## 3. Formato Canónico del Estado en Disco

### 3.1. Estructura del Workspace

```
mi-proyecto-ltp/
├── ltp.config.json          # Configuración global
├── nodes/                   # Pool global de nodos atómicos
│   ├── CRT-UDE-001.json
│   └── ...
├── trees/                   # Vistas topológicas
│   ├── tree-crt-logistica.json
│   └── ...
└── .ltp/                    # Estado interno del motor (en .gitignore)
    ├── lock                 # Lock file de concurrencia
    ├── undo/                # Stack de undo
    │   ├── 001.json
    │   ├── 002.json
    │   └── ...
    ├── redo/                # Stack de redo
    │   └── ...
    └── tmp/                 # Escritura atómica temporal
```

### 3.2. Entrada de Undo (`.ltp/undo/001.json`)

```json
{
  "seq": 1,
  "action": "link_reverse",
  "command": "ltp link reverse --tree tree-crt-logistica --link LINK-001 --force",
  "timestamp": "2026-08-11T10:30:00Z",
  "batch": null,
  "affected_files": {
    "trees/tree-crt-logistica.json": {
      "before": "<contenido previo completo>",
      "after_hash": "sha256:a1b2c3d4..."
    }
  }
}
```

Casos especiales:
- `"before": null` → el fichero fue creado por esta operación. Undo lo elimina.
- `"after_hash": null` → el fichero fue eliminado por esta operación. Undo lo recrea desde `before`.
- `"batch": "Construcción CRT"` → esta entrada es un batch (snapshot de begin-batch). Contiene todos los ficheros afectados durante el batch completo.

### 3.3. Entrada de Redo (`.ltp/redo/001.json`)

Mismo formato que undo, con campo adicional:

```json
{
  "seq": 1,
  "action": "link_reverse",
  "command": "...",
  "timestamp": "...",
  "affected_files": {
    "trees/tree-crt-logistica.json": {
      "before_hash": "sha256:x1y2z3...",
      "after": "<contenido a restaurar>"
    }
  }
}
```

`before_hash` se valida contra el estado actual antes de reaplicar. Si diverge → `REDO_STATE_DIVERGED`.

### 3.4. Archivo de Nodo Atómico (`nodes/CRT-UDE-001.json`)

```json
{
  "id": "CRT-UDE-001",
  "type": "UDE",
  "label": "El tiempo de entrega al cliente supera los 15 días laborables",
  "tags": ["logistica", "critico"],
  "observable": true,
  "metadata": {
    "status": "active"
  }
}
```

Vocabulario de `status`: `active | draft | invalidated | superseded`

### 3.2. Archivo de Vista Topológica (`trees/tree-crt-logistica.json`)

`nodes[]`: Array de objetos con `ref` (ID del nodo en el pool global) y `role` (opcional, contextual a esta vista). El `role` es obligatorio en árboles EC (`objective`, `requirement`, `prerequisite`). En otros árboles es `null` o se usa para anotar roles como `core_problem`, `injection_target`, etc. Un nodo del pool puede participar en múltiples vistas con distintos roles — no hay restricción de tipo de origen.

```json
{
  "id": "tree-crt-logistica",
  "name": "Current Reality Tree - Logística",
  "type": "CRT",
  "logic": "sufficiency",
  "nodes": [
    {"ref": "CRT-RC-001", "role": null},
    {"ref": "CRT-INT-001", "role": null},
    {"ref": "CRT-UDE-001", "role": null}
  ],
  "edges": [
    {
      "id": "LINK-001",
      "from": ["CRT-RC-001", "CRT-INT-001"],
      "to": "CRT-UDE-001",
      "operator": "AND",
      "weight": null,
      "status": "active",
      "logic": "SUFFICIENCY",
      "assumptions": [
        {
          "id": "ASM-001",
          "status": "valid",
          "text": "La capacidad de transporte no se incrementa en temporada alta."
        }
      ]
    }
  ],
  "macro_edges": [
    {
      "id": "MACRO-001",
      "from": "CRT-RC-001",
      "to": "CRT-UDE-005",
      "label": "Cadena logística completa",
      "interior_nodes": ["CRT-INT-001", "CRT-INT-002", "CRT-INT-003"],
      "interior_links": ["LINK-001", "LINK-002", "LINK-003"],
      "status": "active"
    }
  ],
  "feedback_edges": [
    {
      "id": "FB-001",
      "from": "CRT-UDE-003",
      "to": "CRT-RC-001",
      "loop_type": "positive",
      "label": "La pérdida de clientes reduce ingresos, lo que agrava la causa raíz"
    }
  ],
  "nbr_branches": [
    {
      "id": "NBR-001",
      "source_node": "FRT-INJ-001",
      "edges": [
        {
          "id": "NBR-LINK-001",
          "from": ["FRT-INJ-001"],
          "to": "FRT-NDE-001",
          "operator": "SINGLE",
          "weight": null,
          "status": "active",
          "logic": "SUFFICIENCY",
          "assumptions": []
        }
      ],
      "trim_injection": "FRT-INJ-003"
    }
  ]
}
```

Vocabulario de `status` en edges: `active | broken | superseded | needs_review`

Vocabulario de `logic` en tree: `sufficiency | necessity`

Vocabulario de `status` en assumptions: `valid | invalid | needs_review`

---

## 4. Ejemplo de Salida JSON Determinista

Cualquier comando invocado con `--json` retorna este contrato:

```json
{
  "success": true,
  "action": "path_explode",
  "workspace": "Transformacion-2026",
  "data": {
    "tree_id": "tree-crt-logistica",
    "created_node_id": "CRT-INT-002",
    "removed_link_id": "LINK-001",
    "created_link_ids": ["LINK-001a", "LINK-001b"]
  },
  "graph_health": {
    "valid_dag": true,
    "orphan_nodes_count": 0
  },
  "errors": [],
  "warnings": []
}
```
