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
│    Entidades:   node add/edit/rm/split, tree new/rm/clone/rename       │
│    Vistas:      tree attach/detach                                     │
│    Enlaces:     link connect/disconnect/reverse/move/insert-between    │
│    Agrupación:  link group/dissolve/split/reoperator/add-cause/rm-cause│
│    Supuestos:   assume add/edit/rm/list/move, invalidate               │
│    Abstracción: path collapse/explode/replace                          │
│    NBR:         nbr add/list/inspect                                   │
│    Feedback:    link feedback/feedback-list/feedback-rm                 │
│                                                                        │
│  CAPA DE ANÁLISIS SEMÁNTICO (LLM / Humano — fuera del motor)          │
│    El motor NUNCA juzga causalidad, claridad ni suficiencia.           │
│    Solo sirve datos estructurados para que el auditor evalúe.          │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ Modifica / Valida
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│                   WORKSPACE CANÓNICO EN DISCO                           │
│  • /nodes/<ID>.json  • /trees/<ID>.json  • /knowledge/<ID>.json         │
│  • ltp.config.json                                                     │
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

#### `ltp node add "<label>" --type <TYPE> [--tags t1,t2] [--observable true|false] [--ref NODE[@TREE]]…`

Tipos: `UDE | RC | INJ | NC | GOAL | OBJ | WANT | OBS | IO | INT | DE | REQ | PRE | CSF`

`CSF` (Critical Success Factor) es el nivel intermedio del Goal Tree: `GOAL ← CSF ← NC`. Como el resto de tipos, el motor no le impone reglas de rol ni semántica (ADR-001).

"Con Dientes": ejecuta un linter sintáctico suave. Advierte si el texto contiene conjunciones causales prohibidas por CLR #2 (`because`, `in order to`, `para`), sugiriendo dividir la idea.

**Refs entre nodos (RFC-002 S1, ADR-015, desde v0.4.0).** `--ref` (repetible) declara que este nodo se refiere a otro nodo del pool, opcionalmente fijado a un árbol (`NODE@TREE`). En disco: `metadata.refs: [{"node": "NC-001", "tree": "tree-gt-x" | null}]`, set ordenado por `(node, tree)` (un duplicado es no-op). Se omite si está vacío (byte-compatible con v0.3.0). MCP: `refs: [{node, tree?}]` (parser estricto: claves desconocidas o tipos erróneos → `-32602`).

Validación bloqueante, previa al minteo del ID: `NODE_NOT_FOUND` (destino inexistente), `TREE_NOT_FOUND` (árbol del pin inexistente), `NODE_NOT_IN_TREE` (destino ausente del árbol, tronco o ramas NBR), `SELF_REF` (auto-referencia). Contexto: `ref_node`, `ref_tree`. En CLI, un `--ref` léxicamente inválido (`@`, `NC-001@`, espacios, dos `@`) → `INVALID_REF`.

#### `ltp node edit <ID> [--label "<texto>"] [--add-tag <tag>] [--rm-tag <tag>] [--observable true|false] [--epistemic <fact|hypothesis|assumption|derived>] [--add-ref NODE[@TREE]]… [--rm-ref NODE[@TREE]]…`

`--rm-ref` se aplica antes que `--add-ref`; elimina por coincidencia exacta `(node, tree)` y no valida el destino (permite limpiar refs colgantes). Una ref ausente → warning `REF_NOT_PRESENT {node_id, ref_node, ref_tree}` (no-op). `--add-ref` aplica la misma validación que `node add`. MCP: `add_refs` / `rm_refs`.

Cuando se modifica `--epistemic`, el motor emite warnings de cascada epistémica:
- `EPISTEMIC_UNBOUNDED_FACT`: al promover a `fact` o `derived`, si alguna causa upstream (en cualquier tree donde participe) tiene status `hypothesis` o `assumption`.
- `EPISTEMIC_CASCADE_REVIEW`: al degradar de `fact`/`derived` a `hypothesis`/`assumption`, lista los efectos downstream que deberían revisarse.

#### `ltp node rm <ID>[,<ID2>,<ID3>] [--force]`

Elimina nodos del pool global y todos sus edges asociados en todas las vistas, incluidos los edges de ramas NBR. Batch: acepta lista separada por comas.

- Si un nodo eliminado es el `source_node` de una rama NBR, la rama se elimina: warning `NBR_BRANCH_REMOVED {tree_id, nbr_id}`.
- Las refs entrantes desde otros nodos se eliminan: un warning `REFS_STRIPPED {referencing, node_ids}` por nodo afectado (`node_ids` ordenado). Deshacible con `undo`.
- *(Desde v0.5.0, ADR-016 D-3.)* Long arrows (`macro_edges`) de todos los árboles:
  - Si se borra un extremo (`from`/`to`), la macro se elimina con sus `MacroAssumption`.
  - Si se borra un nodo interior, se quita de `interior_nodes`, y los edges eliminados se quitan de `interior_links`.
  - Un `overlay` al que este `rm` ha tocado y que se queda sin ningún link interior vivo (presente en `edges`) se elimina.
  - Cada macro eliminada emite `MACRO_EDGE_REMOVED {tree_id, macro_link, reason, status, from, to, assumption_ids}`, con `reason` = `endpoint_removed` | `interior_emptied`. `assumption_ids` va en orden de almacenamiento y la eliminación se puede deshacer con `undo`.
  - Las macros que el `rm` no toca no se modifican. Las `projection_refs` de una superviviente tampoco: `validate` las señala con `LONG_ARROW_SUMMARY_STALE`. Regla: *una mutación avisa de lo que destruye; `validate`, de lo que queda inconsistente*.
- *(Desde v0.5.0.)* **Fail-closed**: se cargan todos los árboles antes de escribir nada. Si alguno no se puede leer, se devuelve `IO_ERROR {tree_id}` sin escribir ningún byte. Los IDs repetidos en la entrada se deduplican.
- Orden de warnings: lock obsoleto → `NBR_BRANCH_REMOVED` → `MACRO_EDGE_REMOVED` (por árbol y después en orden de almacenamiento) → `REFS_STRIPPED` → `KNOWLEDGE_ORPHANED`. `data`: `{ removed_nodes, removed_edges_count, affected_trees }`. Si `affected_trees` no está vacío, conviene ejecutar `validate` sobre esos árboles.

#### `ltp node inspect <ID>`

Muestra en qué árboles participa, con qué rol en cada uno, y sus conexiones. Desde v0.4.0, `data` incluye `refs` (salientes) y `referenced_by` (IDs de nodos que lo referencian, ordenados y sin duplicados).

#### `ltp node list --tree <TREE_ID> [--type UDE,RC,...] [--status active,draft,...]`

Lista nodos del tree filtrable por tipo y/o status.

#### `ltp node search --tree <TREE_ID> --query "<texto>"`

Busca nodos por contenido de label (substring match).

#### `ltp node split <ID> --into "<label_1>" "<label_2>" --tree <TREE_ID>`

Divide una entidad con dos ideas en dos nodos. Hereda conexiones entrantes al primer nodo y salientes al segundo. Elimina el nodo original. Ambos hijos heredan `refs` y metadata extra (status `active`). Las refs entrantes de otros nodos se reescriben para apuntar a **ambos** hijos (conservando el pin). Caso de uso principal: CLR #2 (entidad con ideas mezcladas).

**Global desde v0.5.0 (ADR-016 D-1/D-2).** El nodo pertenece al pool, así que el split reescribe **todos** los árboles donde aparece, no solo `--tree`. `--tree` sigue siendo obligatorio como árbol de contexto: si el nodo no está en su `nodes[]`, devuelve `NODE_NOT_IN_TREE`. La redirección sigue una regla única: lo **entrante** va al primer hijo y lo **saliente**, al segundo.
- Al primer hijo van: el `to` de los edges del tronco y de las ramas NBR, el `to` del feedback, el `to` de las macros y el `source_node` de las NBR.
- Al segundo hijo van: el `from[]` de los edges, el `from` del feedback y el `from` de las macros.
- En `nodes[]` y en `interior_nodes`, el original se sustituye **en su posición** por los hijos que aún no estén. Una entrada existente nunca se mueve y conserva su `role`.
- Un nodo que solo aparece en ramas NBR también se reescribe.

Solo se guardan los árboles que cambian. Es **fail-closed**: carga todos los árboles antes de mintear IDs o escribir. Un árbol ilegible devuelve `IO_ERROR {tree_id}` sin escribir nada y sin consumir contadores, también si es el de `--tree`. `data`: `{ original_id, new_nodes, tree_id, affected_trees }`, donde `affected_trees` está ordenado e incluye siempre `tree_id`. Conviene ejecutar `validate` sobre esos árboles.

---

### 2.3. Vistas (Trees)

#### `ltp tree new <gt|crt|ec|frt|prt|tt> "<nombre>"`

La lógica del árbol se deriva del tipo (CLR_SPEC §1.2, ADR-014): `necessity` en GT/EC/PRT y `sufficiency` en CRT/FRT/TT. No es configurable.

#### `ltp tree list`

Lista todos los trees del workspace. `data`: `{ "trees": [ { "id", "name", "tree_type", "logic", "node_count", "edge_count" } ], "count" }`. El campo se llama **`tree_type`** (no `type`) y su valor es un string en minúscula: `gt` | `crt` | `ec` | `frt` | `prt` | `tt`. `logic` es `sufficiency` | `necessity`.

#### `ltp tree rm <TREE_ID>`

#### `ltp tree attach --tree <TREE_ID> --node <ID> [--role "<role>"]`

Añade un nodo del pool a un tree sin conectarlo (staging). Aparece como huérfano dentro del tree. `status` lo reporta.

`--role` es texto libre salvo en los EC, donde `validate` exige el vocabulario exacto (case-sensitive): `objective` (exactamente 1), `requirement` (≥2) y `prerequisite` (≥1 por requirement, conectado `prerequisite → requirement`). Cualquier otro valor en un EC (p. ej. `root`, `leaf`, `intermediate`, `Objective`) hace fallar `validate` con `EC_VALIDATION`.

#### `ltp tree detach --tree <TREE_ID> --node <ID>`

Quita un nodo de una vista (y sus edges en ese tree), pero lo preserva en el pool global `/nodes/`.

#### `ltp tree clone <TREE_ID> --name "<nuevo_nombre>"`

Crea una copia del tree con nuevo ID. Los nodos son referencias compartidas al pool (mismo ref). Los edges son independientes — se puede reorganizar sin afectar el original. Para exploración "what-if".

#### `ltp tree rename <TREE_ID> --name "<nuevo_nombre>"`

Renombra el `name` (label) de una instancia de tree existente. Muta **solo** el nombre: el `id` y el fichero `trees/<id>.json` permanecen estables, preservando la integridad referencial (`attach`, refs, consumidores externos). Es el análogo de `node edit` sobre `node.label`. `data`: `{ "id", "old_name", "new_name" }`. Errores: `TREE_NOT_FOUND` (id inexistente), `INVALID_TREE_NAME` (nombre vacío o de solo espacios; el nombre válido se almacena verbatim, sin trim). Renombrar al mismo nombre es un no-op idempotente exitoso.

> **Interacción conocida**: como el `id` se deriva del slug del nombre en `tree new`, tras renombrar el slug original sigue ocupado por ese `id`. Un `tree new` posterior con el nombre original chocará con `TREE_ALREADY_EXISTS` (no hay sobrescritura silenciosa).

#### `ltp tree diff <TREE_A> <TREE_B>`

Reporta diferencias entre dos trees: nodos añadidos/quitados, edges añadidos/quitados/modificados, cambios de operador.

#### `ltp tree relation list [--tree <TREE_ID>]`

*(RFC-002 S1, ADR-015, desde v0.4.0. MCP: `ltp/tree_relation_list`.)* Meta-grafo **inferido al vuelo** desde las `refs` de los nodos; nunca se persiste y es solo lectura. Para cada nodo N y cada ref r: extremos(N) × extremos(r.node), filtrando por `r.tree` si hay pin. Un extremo es `{tree, nbr}` (`nbr: null` = tronco). Se descartan los pares con extremos idénticos (refs intra-árbol). Las relaciones son **estructurales y sin tipo** (no hay `relation_type`: inferirlo afirmaría la intención del analista, ADR-001).

```json
{ "relations": [ { "referencing": {"tree": "tree-crt-x", "nbr": null},
                   "referenced":  {"tree": "tree-gt-y",  "nbr": null},
                   "logic": {"referencing": "sufficiency", "referenced": "necessity"},
                   "basis": [{"node": "UDE-001", "ref": "NC-003"}],
                   "inferred": true } ],
  "count": 1 }
```

`logic` se deriva del tipo de cada árbol (ADR-014). Salida ordenada por `(referencing, referenced)`. `--tree` filtra las relaciones que tocan ese árbol; si no existe → `TREE_NOT_FOUND`.

#### `ltp tree walk <TREE_ID> [--order topological|reverse] [--show-knowledge]`

Recorrido ordenado del árbol completo para auditoría sistemática (JSON por defecto; `--human` para texto).

`data`: `{ "tree_id", "order", "nodes": [ ... ] }`. Cada nodo:

```json
{ "id": "UDE-001", "role": "core_problem", "incoming_edges": ["LINK-003"], "outgoing_edges": ["LINK-004", "LINK-005"] }
```

- `role` puede ser `null` (solo obligatorio en EC).
- `incoming_edges` / `outgoing_edges` son arrays de **IDs de edge (LINK)**, no objetos. Para operator/assumptions/from/to de un edge concreto: `ltp link inspect <id>`. Un mismo edge AND aparece en el `outgoing_edges` de cada causa.
- Sin `--order`, el orden se deriva de la lógica del árbol (ADR-014): `topological` en suficiencia (CRT/FRT/TT), desde causas raíz hacia efectos; `reverse` en necesidad (GT/EC/PRT), desde el objetivo hacia los prerrequisitos. `--order` explícito siempre manda y `data.order` informa del orden aplicado. Solo se aceptan `topological` y `reverse` (sensible a mayúsculas); cualquier otro valor falla con `INVALID_ORDER` antes de buscar el árbol (`data.order` devuelve el valor recibido y `nodes` vacío).
- `--show-knowledge`: añade a cada nodo `"knowledge": { "supports", "contradicts", "contextualizes" }` (conteos). Sin el flag, el campo se **omite**.
- **No incluye feedback edges** (viven en `feedback_edges`, fuera del DAG): obtenlas con `ltp link feedback-list`.

> Flags reservados **sin efecto actual** (se parsean pero se ignoran en el dispatch): `--show-origin`, `--expand-nbr`.

---

### 2.4. Enlaces — Conexión y Desconexión

#### `ltp link connect --tree <TREE_ID> --from <ID1>[,<ID2>] --to <ID3>[,<ID4>] [--operator SINGLE|AND|OR|MAG|XOR] [--weight 0.0-1.0]`

Establece una conexión causa-efecto en una vista:
- Si `--from` recibe múltiples IDs con `--operator AND`, genera la elipse de suficiencia conjuntiva.
- Si `--to` recibe múltiples IDs, crea un edge SINGLE de la causa hacia cada destino.
- `--weight` solo aplica con operator MAG; warning si se omite con MAG.
- La lógica del edge la hereda del árbol (ADR-014): `NECESSITY` en GT/EC/PRT, `SUFFICIENCY` en CRT/FRT/TT. Con `--nbr`, el edge es siempre `SUFFICIENCY` (una NBR es una rama "si-entonces").
- Valida integridad referencial (falla si un nodo no existe en `/nodes/`).

#### `ltp link disconnect --tree <TREE_ID> --links <LINK_ID>[,<LINK2>,<LINK3>]`

Elimina uno o más edges. Batch: acepta lista separada por comas.

#### `ltp link feedback --tree <TREE_ID> --from <ID1> --to <ID2> --type <positive|negative> [--label "<texto>"]`

Crea una arista de retroalimentación (feedback loop) en el pool `feedback_edges` del tree. No participa en la validación DAG. Disponible en árboles de suficiencia (CRT, FRT, TT).

#### `ltp link feedback-list --tree <TREE_ID>`

Lista todas las feedback edges del tree. Solo lectura, sin historial. Acción: `link_feedback_list`. Data: `{ "tree_id", "feedback_edges": [{ "id", "from", "to", "loop_type", "label" }] }`. Si el tree no existe: error `TREE_NOT_FOUND`.

#### `ltp link feedback-rm --tree <TREE_ID> --feedback <FB_ID>`

Elimina una feedback edge por ID. Genera entrada de historial (undo la restaura). Acción: `link_feedback_rm`. Data: `{ "removed_id", "tree_id" }`. Si la feedback edge no existe: error `FEEDBACK_EDGE_NOT_FOUND`.

---

### 2.5. Enlaces — Inspección y Búsqueda

#### `ltp link inspect <LINK_ID> --tree <TREE_ID>`

Muestra el detalle completo de un edge: from (con labels, `node_type`, `epistemic`), to (con label, `to_type`, `to_epistemic`), operator, weight, status, logic, y la lista completa de assumptions con su status. Los campos `node_type`/`epistemic` y `to_type`/`to_epistemic` se omiten del JSON si son `null` (nodo no encontrado).

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
- `--insert-after-cause <ID>`: extrae esa causa del grupo, crea CauseX→Nodo, y Nodo se añade al grupo original en su lugar. El edge original se modifica in-place; assumptions se conservan intactas.
- `--insert-before-effect`: `[A, B] --AND--> C` se convierte en `[A, B] --AND--> Nodo` + `Nodo → C`.

**Assumptions**: en los casos SINGLE e `--insert-before-effect`, los assumptions del edge original se copian al edge que preserva las causas originales (edge1) con `status: "needs_review"`. El edge hacia el destino original (edge2) no hereda assumptions. Se emite warning `ASSUMPTIONS_MOVED_NEED_REVIEW` si había assumptions. En `--insert-after-cause`, el edge original sobrevive modificado y sus assumptions se conservan sin cambios.

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

Warning `COLLAPSE_HIDES_EXECUTION_NODES`: si algún nodo interior es de tipo `OBS`, `IO` o `PRE` (nodos de ejecución crítica), el motor advierte que el colapso los oculta. El warning incluye `hidden_nodes` (array de IDs afectados) en el contexto.

#### `ltp path explode --tree <ID> --link <LINK_ID> --asm <ASM_ID> --label "<texto_nuevo_nodo>"`

Desglosa un supuesto convirtiéndolo en un nodo intermedio explícito (INT) dentro de la cadena causa-efecto.

#### `ltp path replace --tree <ID> --macro-link <MACRO_ID> --by-node <NODE_ID>`

Reemplaza un sub-grafo colapsado por una Inyección, marcando la cadena táctica previa como `superseded`.

*(Desde v0.5.0.)* Antes de marcar nada o mintear IDs, comprueba los extremos de la macro. Si uno no está en `nodes[]` del árbol, devuelve `NODE_NOT_IN_TREE {node_id}`; si está adjunto pero no existe en el pool, `NODE_NOT_FOUND {node_id}`. Se comprueba primero `from`. Así una macro colgante, dañada por versiones anteriores, no materializa edges rotos.

#### `ltp macro add --tree <ID> --from <ID1> --to <ID2> --label "<label>"`

Declara **top-down** un salto lógico (CLR #1, "flecha larga"): crea un `macro_edge` en estado `reservation` con interior vacío (`MACRO-xxx`), representando una relación `from → to` que el analista cree válida pero cuyos pasos intermedios aún no ha articulado (ADR-013). No valida topología (la reserva es independiente del grafo táctico) y **no** afecta `valid_dag` (ADR-010: fuera del DAG). Es la operación inversa a `path collapse` (que resume una cadena real existente, `overlay`).

Errores: `TREE_NOT_FOUND`, `LABEL_REQUIRED` (label vacío), `RESERVATION_SELF_LOOP` (`from == to`, contexto `node_id`), `NODE_NOT_IN_TREE` (extremo no attached, contexto `node_id`). Todas las validaciones preceden al minteo del ID ⇒ el contador `MACRO` no se consume en un fallo de validación. `data`: `{ macro_edge_id, from, to, label }`.

#### `ltp macro expand --tree <ID> --macro-link <MACRO_ID> --steps "<s1,s2,…>"`

Materializa una reserva en una cadena `INT` explícita (transición `reservation → overlay`, ADR-013): crea `n` nodos `INT` (uno por label separada por comas; labels duplicadas permitidas, IDs distintos) y `n+1` edges encadenando `from → INT₁ → … → INTₙ → to`, con la lógica derivada del árbol contenedor (`SUFFICIENCY` en CRT/FRT/TT; `NECESSITY` en GT/EC/PRT). Los `macro_assume` de la reserva se conservan (ahora proyectables sobre el interior real).

Los edges son reales ⇒ **bloquea ciclos**: pre-valida el DAG antes de persistir; si la cadena cerraría un ciclo devuelve `CIRCULAR_DEPENDENCY_DETECTED` (contexto `cycle_path`, `valid_dag: false`) sin crear ningún `INT`/`LINK` ni mutar el estado en disco (mismo contrato que `link connect`).

Errores: `TREE_NOT_FOUND`, `MACRO_EDGE_NOT_FOUND`, `NOT_A_RESERVATION` (la macro ya es `overlay`), `NODE_NOT_IN_TREE` / `NODE_NOT_FOUND` (desde v0.5.0: el extremo no está adjunto, o está adjunto pero no existe en el pool; contexto `node_id`; se comprueba antes de crear nada), `STEPS_REQUIRED` (sin labels no vacías), `CIRCULAR_DEPENDENCY_DETECTED`. `data`: `{ macro_link, created_nodes: [INT…], created_links: [LINK…], status: "overlay" }`.

#### `ltp macro promote --tree <ID> --macro-link <MACRO_ID>`

Acepta el salto como causalidad directa y consume la reserva (transición `reservation → edge atómico + macro eliminada`, ADR-013): crea un edge `from → to` (`SINGLE`, lógica derivada del árbol) y **migra** los `macro_assume` de la reserva a `Assumption` del edge (preserva `status`/`text`, `ASM-xxx`; descarta `projection_refs`). El `macro_edge` se elimina (espeja `path replace`). Para consumir un `overlay` úsese `path replace`, no `promote`.

El edge es real ⇒ **bloquea ciclos** con el mismo contrato que `expand`: el pre-check DAG precede al minteo de los `ASM` migrados, de modo que un ciclo bloqueado no consume la reserva ni quema el contador `ASM`.

Errores: `TREE_NOT_FOUND`, `MACRO_EDGE_NOT_FOUND`, `NOT_A_RESERVATION` (la macro es `overlay`; usar `path replace`), `NODE_NOT_IN_TREE`, `NODE_NOT_FOUND` (desde v0.5.0: extremo adjunto pero ausente del pool), `CIRCULAR_DEPENDENCY_DETECTED`. `data`: `{ macro_link, created_link, migrated_assumptions: [ASM…], from, to }`.

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
- DFS de 3 colores sobre `edges` (excluye `feedback_edges`): verifica que los árboles de suficiencia (CRT, FRT, TT) sean DAGs puros. Retorna `CIRCULAR_DEPENDENCY_DETECTED` si hay ciclos. El error incluye `cycle_path` (array de IDs de nodos formando el ciclo exacto).
- Valida edges dentro de cada `nbr_branches[]` como DAGs independientes (con `cycle_path` en caso de ciclo).
- Integridad referencial: todo nodo referenciado existe en `/nodes/`. Desde v0.5.0 (ADR-016 D-5) se comprueban todas las estructuras del árbol, en este orden fijo: `nodes[]` → `edges` → `feedback_edges` → `nbr_branches` (edges y `source_node`) → `macro_edges` (`from`, `to`, `interior_nodes`). Así se detectan también los workspaces dañados por `node split`/`node rm` de v0.4.x. Se emite `REFERENTIAL_INTEGRITY_VIOLATION` por cada hueco, en `data.details[].errors`, con este contexto:
  - `tree_id` y `node_id` (el nodo ausente).
  - `location`: `nodes` | `edges` | `feedback_edges` | `nbr_branches` | `macro_edges`.
  - `field`: `ref` | `from` | `to` | `source_node` | `interior_nodes`.
  - El ID del contenedor: `edge_id`, `feedback_id`, `nbr_id` (más `edge_id` si es un edge de rama) o `macro_link`.

  Un nodo que está en disco pero no se puede leer cuenta como existente: lo señala `NODE_UNREADABLE` y no se duplica como violación. Los comandos `link` emiten el mismo código **sin contexto** (solo `detail`).
- EC: exactamente 1 nodo con role `"objective"`, al menos 2 con role `"requirement"` vinculados al objective, al menos 1 `"prerequisite"` por cada requirement, al menos 1 conector XOR entre prerrequisitos incompatibles. Soporta N ramas. Los nodos referenciados pueden ser de cualquier tipo del pool global — el role es contextual a la vista.

**Advertencias (warnings):**
- Linter CLR #2: conjunciones causales prohibidas en labels (`because`, `in order to`, `para`, `y` como causal). Se ejecuta sobre todos los nodos del tree, no solo al crearlos.
- *(Solo suficiencia: CRT/FRT/TT.)* Nodos con solo 1 entrada SINGLE: candidatos a insuficiencia (CLR #4).
- *(Solo suficiencia: CRT/FRT/TT.)* Nodos con ≥2 entradas SINGLE sin operador declarado: OR implícito (CLR #4/#5). `CLR4_5_IMPLICIT_OR_REVIEW` advierte que se confirme que cada causa basta sola, o que se agrupe con AND/MAG si son co-dependientes.
- *(Solo suficiencia: CRT/FRT/TT.)* Elipses AND con >4 entradas: posible mezcla de causas independientes (CLR #4/#5).
- En árboles de necesidad (GT/EC/PRT) los tres lints CLR #4 anteriores no se evalúan: cada condición necesaria es insuficiente por sí sola por construcción (CLR_SPEC §1.2, ADR-014). El resto de lints semánticos sí se aplican.
- Nodos con `observable: false` y <2 edges salientes: candidatos a CLR #7 (causa intangible sin efecto predicho).
- Inversión de tipos sospechosa (CLR #6): nodo de nivel alto (UDE, DE) en posición `from` apuntando a nodo de nivel bajo (RC, INT).
- CLR #5 (MAG weights): `CLR5_MAG_WEIGHTS_NOT_NORMALIZED` si las weights de edges MAG al mismo nodo no suman ~1.0 (tolerancia ±0.01). `CLR5_MAG_WEIGHT_UNDEFINED` si un edge MAG no tiene weight definido.
- Nodos huérfanos dentro del tree (attached pero sin edges). Excepción (ADR-013): los extremos de un `macro_edge` en estado `reservation` se consideran conectados (el salto lógico ya los relaciona), por lo que no disparan `ORPHAN_NODE_IN_TREE`.
- Flecha larga en estado `reservation` sin resolver: `LONG_ARROW_RESERVATION_PENDING` (CLR #1, contexto `macro_link`/`from`/`to`) recuerda que el salto está pendiente de `macro expand` o `macro promote`. No bloquea (ADR-010).
- *(Meta-grafo, entrada sintética `_meta_graph`, desde v0.4.0.)* `DANGLING_NODE_REF {node_id, ref_node, ref_tree, reason}` con `reason` = `node_missing` | `tree_missing` | `not_in_tree` (refs rotas por edición manual o binarios antiguos). `NORM_REF_MISSING {node_id, trees}`: solo si el workspace tiene ≥1 GT; una UDE adjunta a un CRT o presente en una rama NBR sin ref a una norma (NC, CSF u OBJ) adjunta a un GT (respetando el pin). Un warning por nodo, `trees` ordenado. `NODE_UNREADABLE {node_id}`: nodo listado en disco que no se puede cargar (antes se saltaba en silencio). Con `--tree`, solo nodos de ese árbol (tronco o ramas NBR). La entrada solo aparece si hay warnings.
- Higiene de resumen de flecha larga (Slice 1): `LONG_ARROW_SUMMARY_STALE` (proyecciones colgantes o supuestos interiores sin mapear) y `MACRO_ASSUMPTION_UNGROUNDED` (macro-assume sobre un `overlay` con interior no vacío pero sin `projection_refs`).

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
├── knowledge/               # Pool de knowledge items (ADR-012)
│   ├── KN-001.json
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
  "epistemic": "fact",
  "metadata": {
    "status": "active"
  }
}
```

Campo `epistemic` (opcional): `fact | hypothesis | assumption | derived`. Default: `hypothesis`. Declarativo del usuario — el motor persiste y valida consistencia, nunca promueve/degrada automáticamente. Ver KNOWLEDGE_SPEC.md §3 para detalle.

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
      "status": "overlay",
      "assumptions": [
        {
          "id": "MASM-001",
          "status": "valid",
          "text": "El resumen supone estabilidad de la demanda en el tramo.",
          "projection_refs": ["ASM-001", "LINK-002"]
        }
      ]
    },
    {
      "id": "MACRO-002",
      "from": "CRT-RC-002",
      "to": "CRT-UDE-004",
      "label": "Salto lógico pendiente de articular",
      "interior_nodes": [],
      "interior_links": [],
      "status": "reservation"
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

Se deriva de `tree_type` (ADR-014): `necessity` en GT/EC/PRT, `sufficiency` en CRT/FRT/TT. Los edges del tronco llevan la misma lógica (`NECESSITY`/`SUFFICIENCY`); los edges de `nbr_branches` son siempre `SUFFICIENCY`. Un fichero legacy inconsistente se normaliza al leer y se persiste corregido en la siguiente mutación.

Vocabulario de `status` en assumptions: `valid | invalid | needs_review`

Vocabulario de `status` en macro_edges: `reservation | overlay` (ADR-013). `overlay` = resume una cadena causa-efecto real coexistente (creado por `path collapse` o resultante de `macro expand`); `reservation` = salto lógico top-down con interior vacío pendiente de `macro expand`/`macro promote`. Retrocompat: los `macro_edges` previos con `"status": "active"` (o sin campo `status`) deserializan como `overlay` vía `#[serde(alias)]`.

Campo `assumptions` en macro_edges (opcional): supuestos-resumen autorados (`MASM-xxx`) que cuelgan de la flecha larga, con `projection_refs` (IDs interiores `LINK-xxx`/`ASM-xxx` que resumen). Se omite del JSON cuando está vacío (`skip_serializing_if`); los ficheros legacy sin el campo deserializan con lista vacía. Gestionados por los comandos `macro-assume gather|add|rm|list` (Slice 1).

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
