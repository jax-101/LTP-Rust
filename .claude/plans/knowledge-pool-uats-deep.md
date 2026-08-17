# UATs Profundas — Knowledge Pool (K1–K7)

**Fecha**: 2026-08-17
**Criterio**: Adversarial — cada UAT apunta a un bug real que pasaría inadvertido con testing superficial.

**Categorías**: `[happy]` `[boundary]` `[interaction]` `[corrupt]` `[idempotent]` `[ordering]` `[referential]`

---

## Fase K1: Fundación (Schema, Storage, Init, Counters)

| ID | Escenario | Resultado esperado | Cat |
|----|-----------|-------------------|-----|
| K1.1 | `ltp init --name "Test"` | Crea `knowledge/` vacía. `counters.json` incluye `"KN": 0`. Output `directories_created` incluye `"knowledge/"`. | `[happy]` |
| K1.2 | Workspace existente sin `knowledge/` → `save_knowledge(item)` | Auto-crea `knowledge/`. Operación completa sin error. | `[happy]` |
| K1.3 | Serializar KnowledgeItem con todos los campos → deserializar | Round-trip perfecto. JSON con claves ordenadas, indent 2. | `[happy]` |
| K1.4 | KnowledgeSource con `uri: None, excerpt: None` → `is_valid()` | Retorna `false`. | `[happy]` |
| K1.5 | KnowledgeItem con `confidence: None`, `links: []`, `tags: []` → serializar | JSON NO contiene claves "confidence", "links", "tags" (skip_serializing_if). | `[boundary]` |
| K1.6 | `load_knowledge("KN-001")` con JSON corrupto `{broken` en disco | Error `Json(...)` — no panic, no crash. | `[corrupt]` |
| K1.7 | `load_knowledge("KN-001")` con JSON válido pero falta campo `type` | Error de deserialización tipado, no crash genérico. | `[corrupt]` |
| K1.8 | `list_knowledge_ids()` con `.DS_Store` y `notes.txt` en `knowledge/` | Solo retorna IDs de archivos `.json` (ignora otros). | `[boundary]` |
| K1.9 | `load_knowledge("../nodes/UDE-001")` (path traversal) | Error `KnowledgeNotFound` — no lee fuera de `knowledge/`. | `[boundary]` |
| K1.10 | `ensure_knowledge_dir()` llamado 2 veces consecutivas | Primera retorna `true`, segunda `false`. Sin error. | `[idempotent]` |
| K1.11 | `next_id("KN")` × 3 | Genera `KN-001`, `KN-002`, `KN-003` secuencialmente. | `[happy]` |
| K1.12 | Counters rebuild con `knowledge/KN-005.json` existente | Counter KN arranca en 5. Siguiente ID es `KN-006`. | `[interaction]` |
| K1.13 | KnowledgeItem con label UTF-8 complejo: `"Entrega média: 日本語 📦"` | Serializa/deserializa sin corrupción. | `[boundary]` |
| K1.14 | `delete_knowledge("KN-001")` → `load_knowledge("KN-001")` | Segundo falla con `KnowledgeNotFound`. | `[ordering]` |
| K1.15 | `save_knowledge` + `list_knowledge_ids` | ID aparece en la lista. Lista ordenada alfabéticamente. | `[ordering]` |
| K1.16 | KnowledgeItem con `links` conteniendo 50 entries → serializar | JSON válido, no truncado. No hay límite artificial. | `[boundary]` |

---

## Fase K2: CRUD de Knowledge Items

| ID | Escenario | Resultado esperado | Cat |
|----|-----------|-------------------|-----|
| **Add — happy** | | | |
| K2.1 | `knowledge add "Media 18.3 días" --type measurement --source-excerpt "ERP Q2" --status verified --confidence high` | Crea `KN-001.json`. Output: `success: true, data.created_knowledge_id: "KN-001"`. Counter KN incrementa. | `[happy]` |
| K2.2 | `knowledge add "Director dice mejoraron" --type testimony --source-uri "slack://C04/p123"` | Crea KN-002. Status default `unverified`, confidence default `medium`. | `[happy]` |
| K2.3 | `knowledge add "Hipótesis" --type hypothesis --source-excerpt "Entrevista" --tags logistica,proveedor` | Tags almacenados correctamente como array. | `[happy]` |
| **Add — boundary/error** | | | |
| K2.4 | `knowledge add "Sin fuente" --type observation` (sin --source-uri ni --source-excerpt) | Error `SOURCE_REQUIRED`. No crea archivo. Counter NO incrementa. | `[boundary]` |
| K2.5 | `knowledge add "" --type measurement --source-excerpt "x"` (label vacío) | Error `LABEL_REQUIRED`. | `[boundary]` |
| K2.6 | `knowledge add "Test" --type INVALID --source-excerpt "x"` (type no válido) | Error de parsing CLI. | `[boundary]` |
| K2.7 | `knowledge add "Test" --type measurement --source-uri "" --source-excerpt ""` (ambos vacíos) | Error `SOURCE_REQUIRED` (string vacío no cuenta como presente). | `[boundary]` |
| K2.8 | `knowledge add "Test" --type measurement --source-uri "valid" --confidence invalid_value` | Error de parsing CLI para enum inválido. | `[boundary]` |
| K2.9 | `knowledge add` con label de 5000 caracteres + --source-excerpt de 10000 chars | Se acepta y persiste completo. Sin truncamiento. | `[boundary]` |
| K2.10 | `knowledge add "A" --type measurement --source-uri "x"` × 3 rápido | IDs secuenciales `KN-001`, `KN-002`, `KN-003`. Sin colisión. | `[ordering]` |
| **Edit — happy** | | | |
| K2.11 | `knowledge edit KN-001 --label "Nuevo texto"` | Label actualizado en disco. Otros campos intactos. | `[happy]` |
| K2.12 | `knowledge edit KN-001 --status refuted` | Status cambia. `captured` NO cambia (inmutable). | `[happy]` |
| K2.13 | `knowledge edit KN-001 --confidence low` | Confidence actualizado. | `[happy]` |
| K2.14 | `knowledge edit KN-001 --add-tag q2-2026` | Tag añadido. Array ordenado o append. | `[happy]` |
| K2.15 | `knowledge edit KN-001 --rm-tag logistica` | Tag eliminado. | `[happy]` |
| K2.16 | `knowledge edit KN-001 --source-uri "new://uri" --source-excerpt "new excerpt"` | Ambos campos actualizados. | `[happy]` |
| **Edit — boundary/error** | | | |
| K2.17 | `knowledge edit KN-999 --label "..."` | Error `KNOWLEDGE_NOT_FOUND`. | `[boundary]` |
| K2.18 | `knowledge edit KN-001 --label ""` (vaciar label) | Error `LABEL_REQUIRED`. Label no se vacía. | `[boundary]` |
| K2.19 | `knowledge edit KN-001 --status verified` cuando ya es `verified` | Idempotente: success, sin cambio real en disco (o cambio idéntico). | `[idempotent]` |
| K2.20 | `knowledge edit KN-001 --add-tag "a" --add-tag "a"` en misma invocación | Tag "a" aparece UNA sola vez (dedup). | `[idempotent]` |
| K2.21 | `knowledge edit KN-001 --rm-tag "inexistente"` | Warning `TAG_NOT_FOUND` o no-op silencioso. No error bloqueante. | `[boundary]` |
| K2.22 | `knowledge edit KN-001 --source-uri "" --source-excerpt ""` (vaciar ambas fuentes) | Error `SOURCE_REQUIRED`. No deja source inválido. | `[boundary]` |
| K2.23 | `knowledge edit KN-001 --source-uri ""` (vaciar uri pero excerpt existe) | OK: excerpt sigue presente, source sigue válido. uri se vuelve None. | `[boundary]` |
| **Edit — interaction** | | | |
| K2.24 | `knowledge add` → `knowledge edit --status refuted` → `undo` | Edit deshecho: status vuelve al valor pre-edit. | `[interaction]` |
| K2.25 | `knowledge edit KN-001 --label "X"` dentro de batch → `undo` | Todo el batch (incluyendo edit) se revierte. | `[interaction]` |
| **Rm — happy** | | | |
| K2.26 | `knowledge rm KN-001` | Archivo eliminado. | `[happy]` |
| K2.27 | `knowledge rm KN-001,KN-002` (batch) | Ambos eliminados. Output reporta ambos IDs. | `[happy]` |
| **Rm — boundary/error** | | | |
| K2.28 | `knowledge rm KN-999` | Error `KNOWLEDGE_NOT_FOUND`. | `[boundary]` |
| K2.29 | `knowledge rm KN-001,KN-999` (batch con uno inexistente) | Parcial: KN-001 se borra. Error reportado para KN-999. | `[boundary]` |
| K2.30 | `knowledge rm KN-001` → `knowledge rm KN-001` (doble delete) | Segundo falla con `KNOWLEDGE_NOT_FOUND`. | `[idempotent]` |
| **Rm — interaction** | | | |
| K2.31 | `knowledge add` → `knowledge rm KN-001` → `undo` | Undo restaura KN-001 completo con todos sus campos. | `[interaction]` |
| K2.32 | `knowledge rm KN-001` (tiene links a UDE-003) | Se elimina. Los links desaparecen con el item. UDE-003 no se modifica. | `[interaction]` |
| **Inspect** | | | |
| K2.33 | `knowledge inspect KN-001` | Detalle completo: todos los campos, formato spec. | `[happy]` |
| K2.34 | `knowledge inspect KN-999` | Error `KNOWLEDGE_NOT_FOUND`. | `[boundary]` |
| **List** | | | |
| K2.35 | `knowledge list` (con 3 items) | Lista 3 items con id, type, label, status, confidence, link_count. | `[happy]` |
| K2.36 | `knowledge list` (pool vacío) | Lista vacía `items: [], total: 0`. No error. | `[boundary]` |
| K2.37 | `knowledge list --type measurement` | Solo items con type=measurement. | `[happy]` |
| K2.38 | `knowledge list --status unverified` | Filtra por status. | `[happy]` |
| K2.39 | `knowledge list --unlinked` | Solo items con `links: []`. | `[happy]` |
| K2.40 | `knowledge list --tag logistica` | Filtra por tag. | `[happy]` |
| K2.41 | `knowledge list --confidence high` | Filtra por confidence. | `[happy]` |
| K2.42 | `knowledge list --type measurement --status verified --confidence high` (filtros combinados) | Intersección (AND) de todos los filtros. | `[boundary]` |
| K2.43 | `knowledge list --type measurement` con 0 matches | Lista vacía. No error. | `[boundary]` |
| K2.44 | `knowledge list --unlinked` después de linkear todos los items | Lista vacía (todos tienen links). | `[ordering]` |
| **List — corrupt** | | | |
| K2.45 | `knowledge list` con `knowledge/KN-003.json` corrupto en disco | Error claro identificando el archivo problemático. Items válidos aún listados (o error global — decisión de diseño). | `[corrupt]` |
| **Undo integration** | | | |
| K2.46 | `knowledge add` → `undo` → `knowledge add` | Segundo add genera KN-002 (counter NO retrocede con undo). | `[interaction]` |
| K2.47 | `knowledge add` → `undo` → `redo` | KN-001 reaparece exactamente como estaba. | `[interaction]` |

---

## Fase K3: Linking (Vínculos al Grafo)

| ID | Escenario | Resultado esperado | Cat |
|----|-----------|-------------------|-----|
| **Link — happy paths** | | | |
| K3.1 | `knowledge link KN-001 --to UDE-003 --relation supports` | Link añadido a KN-001.links. Output success. | `[happy]` |
| K3.2 | `knowledge link KN-001 --to LINK-007 --relation supports` | Link a edge. Valida que LINK-007 existe en algún tree. | `[happy]` |
| K3.3 | `knowledge link KN-001 --to ASM-002 --relation contradicts` | Link a assumption. Valida que ASM-002 existe en algún tree edge. | `[happy]` |
| K3.4 | `knowledge link KN-001 --to RC-005 --relation contextualizes` | Link con relation contextualizes funciona. | `[happy]` |
| **Link — error / boundary** | | | |
| K3.5 | `knowledge link KN-001 --to NODO-999 --relation supports` | Error `TARGET_NOT_FOUND`. | `[referential]` |
| K3.6 | `knowledge link KN-999 --to UDE-003 --relation supports` | Error `KNOWLEDGE_NOT_FOUND`. | `[boundary]` |
| K3.7 | `knowledge link KN-001 --to UDE-003 --relation supports` (duplicado exacto) | Warning `DUPLICATE_LINK`. Idempotente: NO añade segundo link. | `[idempotent]` |
| K3.8 | `knowledge link KN-001 --to UDE-003 --relation contradicts` (mismo target, diferente relation) | ÉXITO: se permite. Un KN puede supports Y contradicts al mismo target (caso legítimo: "este dato refuerza la existencia pero contradice la magnitud"). | `[boundary]` |
| K3.9 | `knowledge link KN-001 --to ""` (target vacío) | Error. | `[boundary]` |
| K3.10 | `knowledge link KN-001 --to MACRO-001 --relation supports` (target es macro_edge) | Error `TARGET_NOT_FOUND` — macro_edges no son targets válidos (no existen como entidad standalone). | `[referential]` |
| **Link — referential integrity con estados especiales** | | | |
| K3.11 | Link a nodo en pool pero NO attached a ningún tree (nodo huérfano) | ¿Se permite o no? **Decisión**: se permite. El nodo EXISTE en el pool, la validación de attachment es concern del tree, no del knowledge. | `[referential]` |
| K3.12 | Link a nodo con status `invalidated` en metadata | Se permite. El motor no juzga, solo linkea. | `[referential]` |
| K3.13 | Link a edge con status `broken` | Se permite. Knowledge puede contradecir un edge roto. | `[referential]` |
| K3.14 | Link a assumption con status `invalid` | Se permite. Knowledge puede contextualizar un supuesto invalidado. | `[referential]` |
| K3.15 | Link a nodo que es `interior_node` de un macro_edge (colapsado) | Se permite. El nodo sigue existiendo en pool aunque esté "oculto" por el collapse. | `[referential]` |
| K3.16 | Link a LINK-ID que es un edge dentro de `nbr_branches` (no trunk) | Se permite. validate no distingue — el edge existe en el tree. | `[referential]` |
| K3.17 | Link a edge de `feedback_edges` (ID tipo LINK con prefijo FB?) | Depende del esquema de IDs de feedback edges. Si usan LINK-XXX, se permite. Si no existen con ese prefijo, `TARGET_NOT_FOUND`. | `[referential]` |
| **Link — interaction con undo/operaciones destructivas** | | | |
| K3.18 | `knowledge link KN-001 --to UDE-003` → `undo` | Link eliminado del array. KN-001 vuelve a estado pre-link. | `[interaction]` |
| K3.19 | `knowledge link` → `node rm UDE-003` → `knowledge inspect KN-001` | Link sigue en KN-001 con `target_label: null` (dangling ref). No crash. | `[interaction]` |
| K3.20 | `knowledge link KN-001 --to LINK-007` → `link disconnect` que elimina LINK-007 → `validate` | Warning `DANGLING_KNOWLEDGE_REF`. | `[interaction]` |
| K3.21 | `knowledge link KN-001 --to ASM-002` → `invalidate ASM-002` → `knowledge inspect KN-001` | Link sigue. Target resolvible (ASM aún existe, solo cambió status). Label del ASM mostrado. | `[interaction]` |
| K3.22 | `knowledge link` dentro de `begin-batch`/`end-batch` → `undo` | Todo el batch se revierte: link desaparece. | `[interaction]` |
| K3.23 | `path collapse` que oculta LINK-007 dentro de macro_edge → `knowledge inspect KN-001` (linked a LINK-007) | Link sigue visible. `target_label` resuelto (el LINK aún existe dentro del tree.edges, solo está "contenido" en macro_edge.interior_links). | `[interaction]` |
| **Unlink — happy** | | | |
| K3.24 | `knowledge unlink KN-001 --from UDE-003` | Elimina el link. Array reducido. | `[happy]` |
| K3.25 | KN-001 tiene 2 links a UDE-003 (different relations). `unlink --from UDE-003` | ¿Elimina TODOS los links a ese target, o solo el primero? **Decisión**: elimina todos los links con ese target (unlink no filtra por relation). | `[boundary]` |
| **Unlink — error** | | | |
| K3.26 | `knowledge unlink KN-001 --from RC-005` (no existe ese link) | Error `LINK_NOT_FOUND`. | `[boundary]` |
| K3.27 | `knowledge unlink KN-999 --from UDE-003` | Error `KNOWLEDGE_NOT_FOUND`. | `[boundary]` |
| **Unlink — interaction** | | | |
| K3.28 | `knowledge unlink KN-001 --from UDE-003` → `undo` | Link restaurado. | `[interaction]` |
| **List con --target** | | | |
| K3.29 | `knowledge list --target UDE-003` | Lista items vinculados a UDE-003 con campo `relation`. | `[happy]` |
| K3.30 | `knowledge list --target UDE-003 --relation supports` | Filtra por target + relation. | `[happy]` |
| K3.31 | `knowledge list --target NODO-999` | Lista vacía. No error (target podría no existir, el filtro es sobre knowledge items). | `[boundary]` |
| K3.32 | `knowledge list --target UDE-003` con KN-001 que tiene 2 links a UDE-003 (supports + contradicts) | KN-001 aparece DOS veces (una por cada link) o UNA vez con ambas relations? **Decisión**: una vez, con campo que muestre las relations relevantes. | `[boundary]` |
| K3.33 | `knowledge list --relation supports` (sin --target) | Filtra todos los KN que tengan al menos un link con relation=supports. | `[boundary]` |
| **Inspect con links** | | | |
| K3.34 | KN-001 linked a UDE-003 y LINK-007. `knowledge inspect KN-001` | Ambos links con `target_label` resuelto desde disco. | `[happy]` |
| K3.35 | KN-001 linked a UDE-003 (borrado previamente). `knowledge inspect KN-001` | Link mostrado con `target_label: null` (dangling ref, no crash). | `[referential]` |
| K3.36 | KN-001 linked a ASM-002. `knowledge inspect KN-001` | `target_label` muestra el texto del assumption. `target_type` muestra "ASM". | `[happy]` |
| K3.37 | KN-001 linked a LINK-007. `knowledge inspect KN-001` | `target_label` muestra algo significativo del edge (ej: "from→to" o el operator). | `[happy]` |

---

## Fase K4: Campo Epistémico en Nodos

| ID | Escenario | Resultado esperado | Cat |
|----|-----------|-------------------|-----|
| **node add con --epistemic** | | | |
| K4.1 | `node add "Test" --type UDE --epistemic fact` | Nodo creado con `"epistemic": "fact"` en JSON. | `[happy]` |
| K4.2 | `node add "Test" --type UDE` (sin --epistemic) | Nodo SIN campo epistemic en JSON (default hypothesis, omitido por serde). | `[happy]` |
| K4.3 | `node add "Test" --type UDE --epistemic invalid_value` | Error de parsing CLI. | `[boundary]` |
| **node edit con --epistemic** | | | |
| K4.4 | `node edit UDE-001 --epistemic fact` | Añade campo epistemic. Verificar en disco. | `[happy]` |
| K4.5 | `node edit UDE-001 --epistemic hypothesis` | Vuelve a default → campo OMITIDO del JSON (no "epistemic": "hypothesis"). | `[happy]` |
| K4.6 | `node edit UDE-001 --epistemic fact` cuando ya es `fact` | Idempotente. Success. Sin cambio material. | `[idempotent]` |
| K4.7 | `node edit UDE-001 --epistemic fact` → `undo` | Epistemic vuelve al valor anterior (o se omite si era default). | `[interaction]` |
| **node list con --epistemic** | | | |
| K4.8 | `node list --tree T --epistemic hypothesis` | Incluye nodos sin campo (implícit hypothesis) + nodos explícitamente hypothesis. | `[happy]` |
| K4.9 | `node list --tree T --epistemic fact` | Solo nodos explícitamente marcados como fact. | `[happy]` |
| K4.10 | `node list --tree T --epistemic fact` con 0 matches | Lista vacía. | `[boundary]` |
| K4.11 | `node list --tree T --epistemic hypothesis --type UDE` (combinado) | Intersección de ambos filtros. | `[boundary]` |
| **Backwards compatibility** | | | |
| K4.12 | Cargar nodo existente (sin campo epistemic en JSON) | Deserializa correctamente con default `Hypothesis`. No error. | `[corrupt]` |
| K4.13 | Nodo con campo extra desconocido `"epistemic": "unknown_value"` en JSON | Error de deserialización (enum strict). | `[corrupt]` |
| **node inspect** | | | |
| K4.14 | `node inspect UDE-001` (con epistemic fact) | Output incluye `"epistemic": "fact"`. | `[happy]` |
| K4.15 | `node inspect UDE-001` (sin epistemic = default) | Output muestra `"epistemic": "hypothesis"` (o lo omite — decisión de diseño para inspect). | `[boundary]` |
| **Interaction con tree clone** | | | |
| K4.16 | `tree clone` de un tree cuyos nodos tienen epistemic → nodos en el nuevo tree | Nodos son compartidos (pool global). Epistemic es propiedad del nodo, no del tree. Clone no duplica nodos. | `[interaction]` |

---

## Fase K5: Integración con Comandos Existentes

### Status

| ID | Escenario | Resultado esperado | Cat |
|----|-----------|-------------------|-----|
| K5.1 | `status` con knowledge items | Output incluye `knowledge_health`: total, unlinked, contradictions, by_status, epistemic_coverage. | `[happy]` |
| K5.2 | `status` sin knowledge items (pool vacío) | `knowledge_health` con todos los conteos a 0. | `[boundary]` |
| K5.3 | `status` con 5 items, 2 unlinked, 1 con relation=contradicts a un nodo fact | `unlinked_items: 2`, `contradictions: 1`. | `[happy]` |
| K5.4 | `status` epistemic_coverage con mix: 3 fact, 2 hypothesis, 1 assumption, 1 derived, 2 sin campo | `nodes_as_fact: 3, nodes_as_hypothesis: 4` (2 sin campo = hypothesis). | `[boundary]` |

### Validate — DANGLING_KNOWLEDGE_REF

| ID | Escenario | Resultado esperado | Cat |
|----|-----------|-------------------|-----|
| K5.5 | KN-001 linked a UDE-005 (no existe en pool). `validate` | Warning `DANGLING_KNOWLEDGE_REF` con id KN-001 y target UDE-005. | `[referential]` |
| K5.6 | KN-001 linked a LINK-099 (no existe en ningún tree). `validate` | Warning `DANGLING_KNOWLEDGE_REF`. | `[referential]` |
| K5.7 | KN-001 linked a ASM-099 (no existe en ningún tree edge). `validate` | Warning `DANGLING_KNOWLEDGE_REF`. | `[referential]` |
| K5.8 | KN-001 linked a UDE-003 (existe). `validate` | Sin warning de knowledge para este link. | `[happy]` |
| K5.9 | Múltiples KN con dangling refs. `validate` | UN warning por cada dangling ref (no por KN, por link). | `[boundary]` |

### Validate — EPISTEMIC_UNGROUNDED

| ID | Escenario | Resultado esperado | Cat |
|----|-----------|-------------------|-----|
| K5.10 | UDE-003 `epistemic: fact`, 0 knowledge items con relation `supports` targeting it. `validate` | Warning `EPISTEMIC_UNGROUNDED`. | `[happy]` |
| K5.11 | UDE-003 `epistemic: fact`, 1 KN `supports` con status `refuted`. `validate` | Warning `EPISTEMIC_UNGROUNDED` (refuted support no cuenta como grounding). | `[boundary]` |
| K5.12 | UDE-003 `epistemic: fact`, 1 KN `supports` con status `verified`. `validate` | Sin warning UNGROUNDED (al menos 1 verified support). | `[happy]` |
| K5.13 | UDE-003 `epistemic: fact`, 1 KN `supports` con status `unverified`. `validate` | ¿Warning o no? **Decisión**: Sin warning UNGROUNDED (tiene support, aunque no verified). UNGROUNDED = 0 supports totales. | `[boundary]` |
| K5.14 | UDE-003 `epistemic: fact`, 2 KN `contextualizes` pero 0 `supports`. `validate` | Warning `EPISTEMIC_UNGROUNDED` (contextualizes no es support). | `[boundary]` |
| K5.15 | UDE-003 `epistemic: hypothesis`, 0 supports. `validate` | Sin warning UNGROUNDED (solo aplica a `fact`). | `[boundary]` |
| K5.16 | UDE-003 `epistemic: assumption`, 0 supports. `validate` | Sin warning UNGROUNDED (solo aplica a `fact`). | `[boundary]` |

### Validate — EPISTEMIC_CONTRADICTED

| ID | Escenario | Resultado esperado | Cat |
|----|-----------|-------------------|-----|
| K5.17 | UDE-003 `epistemic: fact`, KN-007 `contradicts` con status `verified`. `validate` | Warning `EPISTEMIC_CONTRADICTED` nombra UDE-003 y KN-007. | `[happy]` |
| K5.18 | UDE-003 `epistemic: fact`, KN-007 `contradicts` con status `unverified`. `validate` | Sin warning CONTRADICTED (contradicción no verificada no cuenta). | `[boundary]` |
| K5.19 | UDE-003 `epistemic: fact`, KN-007 `contradicts` con status `refuted`. `validate` | Sin warning CONTRADICTED (la contradicción misma fue refutada). | `[boundary]` |
| K5.20 | UDE-003 `epistemic: fact`, KN-007 `contradicts` con status `superseded`. `validate` | Sin warning CONTRADICTED (superseded no es activo). | `[boundary]` |
| K5.21 | UDE-003 `epistemic: hypothesis`, KN-007 `contradicts` verified. `validate` | Sin warning CONTRADICTED (solo aplica a nodos declarados `fact`). | `[boundary]` |
| K5.22 | UDE-003 `epistemic: fact`, 3 KN `contradicts` verified + 5 KN `supports` verified. `validate` | Warning CONTRADICTED para CADA contradicción verified (3 warnings). El tener supports no anula contradictions. | `[boundary]` |

### Validate — EPISTEMIC_UPGRADEABLE

| ID | Escenario | Resultado esperado | Cat |
|----|-----------|-------------------|-----|
| K5.23 | RC-001 `epistemic: hypothesis`, 2 KN `supports` con status `verified`. `validate` | Warning `EPISTEMIC_UPGRADEABLE`. | `[happy]` |
| K5.24 | RC-001 `epistemic: hypothesis`, 1 KN `supports` verified + 1 `supports` unverified. `validate` | Sin warning (necesita ≥2 verified). | `[boundary]` |
| K5.25 | RC-001 `epistemic: hypothesis`, 3 KN `supports` verified. `validate` | Warning UPGRADEABLE (≥2 threshold cumplido). | `[happy]` |
| K5.26 | RC-001 `epistemic: fact`, 5 KN `supports` verified. `validate` | Sin warning UPGRADEABLE (ya es fact, no hay upgrade). | `[boundary]` |
| K5.27 | RC-001 `epistemic: hypothesis`, 2 KN `supports` verified + 1 `contradicts` verified. `validate` | Warning UPGRADEABLE Y warning CONTRADICTED? **Decisión**: UPGRADEABLE no aplica si hay contradicción verified activa (no sugieres promover algo contradecido). | `[boundary]` |
| K5.28 | RC-001 `epistemic: assumption`, 2 KN `supports` verified. `validate` | Warning UPGRADEABLE (aplica a hypothesis Y assumption). | `[boundary]` |

### Validate — combinatoria completa

| ID | Escenario | Resultado esperado | Cat |
|----|-----------|-------------------|-----|
| K5.29 | Nodo fact + 1 support verified + 1 contradict verified. `validate` | CONTRADICTED pero NO UNGROUNDED (tiene support). | `[boundary]` |
| K5.30 | Nodo fact + 0 supports + 1 contradict verified. `validate` | UNGROUNDED + CONTRADICTED (ambos warnings). | `[boundary]` |
| K5.31 | Nodo fact + 2 supports verified + 0 contradicts. `validate` | Limpio. Ni UNGROUNDED ni CONTRADICTED. | `[happy]` |
| K5.32 | `validate --tree T1` (tree filter). Knowledge links a nodos de T1 y T2. | Solo reporta warnings de epistemic para nodos EN T1 (no globals). | `[boundary]` |
| K5.33 | `validate` sin tree filter. Nodos con epistemic en múltiples trees. | Warnings a nivel global (knowledge pool es global, no per-tree). | `[boundary]` |
| K5.34 | Knowledge pool completamente sano (todos linked, no contradictions, facts grounded). `validate` | 0 warnings de knowledge. | `[happy]` |

### Trace

| ID | Escenario | Resultado esperado | Cat |
|----|-----------|-------------------|-----|
| K5.35 | `trace UDE-003 --tree T --direction upstream --show-knowledge` | Cada nodo en chain incluye `knowledge: [{id, relation, status, confidence}]`. | `[happy]` |
| K5.36 | `trace UDE-003 --tree T --direction upstream` (sin --show-knowledge) | Output normal sin campo knowledge (backwards-compatible). | `[happy]` |
| K5.37 | `trace` con `--show-knowledge`, nodo sin knowledge | Campo `knowledge: []` (se incluye vacío, no se omite). | `[boundary]` |
| K5.38 | `trace` con `--show-knowledge`, nodo con 3 knowledge items (1 supports, 1 contradicts, 1 contextualizes) | Los 3 listados con sus respectivos relation/status/confidence. | `[happy]` |
| K5.39 | `trace` con `--show-knowledge`, KN linked a LINK (edge) no a nodo. | Knowledge de edges no aparece en el array del nodo (solo KN linked directamente al nodo). | `[boundary]` |
| K5.40 | `trace` con `--show-knowledge` y `--depth 1` | Solo nodos en depth=1 tienen knowledge resuelto. | `[boundary]` |

### Node rm

| ID | Escenario | Resultado esperado | Cat |
|----|-----------|-------------------|-----|
| K5.41 | `node rm UDE-003` (2 KN apuntan a él) | Warning `KNOWLEDGE_ORPHANED` con IDs de los KN afectados. Nodo borrado. KN items intactos (sus links ahora dangling). | `[interaction]` |
| K5.42 | `node rm UDE-003` (0 KN apuntan a él) | Sin warning extra. Comportamiento normal. | `[happy]` |
| K5.43 | `node rm UDE-003,RC-001` (batch). KN-001 apunta a UDE-003, KN-002 apunta a RC-001. | Warning `KNOWLEDGE_ORPHANED` para ambos, listando KN-001 y KN-002. | `[interaction]` |
| K5.44 | `node rm UDE-003` → `undo` → `validate` | Nodo restaurado. No warning DANGLING (el link ya no cuelga). | `[interaction]` |

### Tree walk

| ID | Escenario | Resultado esperado | Cat |
|----|-----------|-------------------|-----|
| K5.45 | `tree walk T --show-knowledge` | Cada nodo incluye resumen de knowledge (count por relation). | `[happy]` |
| K5.46 | `tree walk T` (sin --show-knowledge) | Output normal, sin campo knowledge. | `[happy]` |
| K5.47 | `tree walk T --show-knowledge` con nodos sin knowledge | Campo presente con counts a 0. | `[boundary]` |

---

## Fase K6: Tests End-to-End (Workflows)

| ID | Workflow | Resultado esperado | Cat |
|----|---------|-------------------|-----|
| **Hypothesis-driven cycle** | | | |
| K6.1 | `knowledge add` (hypothesis) → `link` to RC → `node edit --epistemic hypothesis` → `knowledge edit --status verified` → `node edit --epistemic fact` → `validate` | Ciclo completo. Validate limpio. | `[happy]` |
| K6.2 | Mismo ciclo pero `validate` ANTES de promover a fact | Warning `EPISTEMIC_UNGROUNDED` (es fact sin support verified). | `[ordering]` |
| **Refutation cascade** | | | |
| K6.3 | `knowledge add` → `link supports` → `node edit --epistemic fact` → `knowledge edit --status refuted` → `validate` | UNGROUNDED: el fact ya no tiene support verified (solo refuted). | `[happy]` |
| K6.4 | KN supports chain de 3 nodos (todos fact). Refutar KN → validate | Solo el nodo directamente linked tiene warning. Knowledge no propaga (ADR-001: motor no juzga cascada). | `[interaction]` |
| **Inbox workflow** | | | |
| K6.5 | Add 5 KN sin links → `list --unlinked` (5) → link 3 → `list --unlinked` (2) → link 2 → `list --unlinked` (0) | Inbox se vacía correctamente con cada link. | `[happy]` |
| **Contradiction detection** | | | |
| K6.6 | KN-001 supports UDE-003 (fact), KN-002 contradicts UDE-003 (fact). Ambos verified. `validate` | Warning `EPISTEMIC_CONTRADICTED`. | `[happy]` |
| K6.7 | Misma situación pero UDE-003 es `hypothesis` | Sin warning CONTRADICTED (no aplica a hypothesis). | `[boundary]` |
| **Multi-target link** | | | |
| K6.8 | KN-001 supports UDE-003, KN-001 supports RC-001. `knowledge inspect KN-001` | Ambos links visibles con labels resueltos. | `[happy]` |
| K6.9 | KN-001 supports UDE-003, KN-001 contradicts UDE-003. `knowledge inspect KN-001` | Ambos links al mismo target con diferentes relations visibles. | `[boundary]` |
| **Undo roundtrip** | | | |
| K6.10 | `knowledge add` → `knowledge link` → `undo` → verify link gone → `undo` → verify KN gone | Undo granular: primero deshace link, luego deshace add. | `[interaction]` |
| K6.11 | `knowledge add` → `knowledge link` → `undo` → `redo` → verify link back | Redo restaura el link. | `[interaction]` |
| **Batch + undo** | | | |
| K6.12 | `begin-batch` → `knowledge add` + `knowledge link` + `node edit --epistemic` → `end-batch` → `undo` | Todo el batch se revierte. KN eliminado, epistemic restaurado. | `[interaction]` |
| **Node rm + dangling** | | | |
| K6.13 | Link KN to UDE → `node rm UDE` → `validate` | Warning `DANGLING_KNOWLEDGE_REF`. | `[interaction]` |
| K6.14 | Link KN to UDE → `node rm UDE` → `undo` (restaura UDE) → `validate` | Sin warnings. Ref ya no es dangling. | `[interaction]` |
| **Backwards compatibility** | | | |
| K6.15 | Workspace con nodos sin campo epistemic → `node list --epistemic hypothesis` | Nodos existentes se tratan como hypothesis. | `[happy]` |
| K6.16 | Workspace con nodos sin campo epistemic → `validate` | Sin warnings de EPISTEMIC (hypothesis no requiere grounding). | `[happy]` |
| **Knowledge + trace integration** | | | |
| K6.17 | Build chain RC→INT→UDE, link knowledge a cada nodo, `trace --show-knowledge` upstream desde UDE | Cada entry muestra su knowledge. | `[happy]` |
| K6.18 | Build chain RC→INT→UDE, link knowledge a edge LINK entre INT y UDE, `trace --show-knowledge` | Knowledge de edges: ¿aparece en link_to_next o separado? Decisión clara. | `[boundary]` |
| **Edge cases compuestos** | | | |
| K6.19 | `knowledge add` → `link to ASM-001` → `invalidate ASM-001` → `validate` | KN link aún existe (ASM sigue existiendo, solo status invalid). Sin DANGLING. | `[interaction]` |
| K6.20 | `path collapse --from A --to E` con KN linked a interior_node B → `validate` | Sin DANGLING (B sigue en pool). Sin warning nuevo. | `[interaction]` |
| K6.21 | `path explode` que crea nuevo nodo intermedio → `validate` | Nuevo nodo no tiene epistemic (default hypothesis). Sin warnings. | `[interaction]` |
| K6.22 | `node split UDE-003 --into "A" "B"` con KN linked a UDE-003 → `validate` | UDE-003 ya no existe → Warning `DANGLING_KNOWLEDGE_REF`. | `[interaction]` |
| K6.23 | `tree rm T` con nodos exclusivos de T que tienen KN linked → `validate` | Nodos siguen en pool (tree rm no borra nodos). Sin DANGLING para nodos. Edges de T desaparecen → DANGLING para KN linked a LINK-IDs del tree eliminado. | `[interaction]` |
| K6.24 | `link group` (N edges merge into 1 new edge) con KN linked a uno de los old edges → `validate` | Old edge ID desaparece → DANGLING_KNOWLEDGE_REF. | `[interaction]` |
| K6.25 | `link dissolve` (undo group) → `validate` | Old edge IDs restaurados. Sin DANGLING. | `[interaction]` |
| K6.26 | KN-001 linked a NBR edge. `nbr rm` elimina la NBR → `validate` | Edge desapareció → DANGLING_KNOWLEDGE_REF. | `[interaction]` |
| **Status coherence** | | | |
| K6.27 | 10 KN items, varía: add/rm/link/unlink. `status` after each | `knowledge_health` siempre refleja el estado actual (no cache stale). | `[ordering]` |
| K6.28 | `knowledge edit KN-001 --status superseded` → `status` | `by_status.superseded` incrementa. | `[ordering]` |

---

## Fase K7: MCP Server (Knowledge Tools)

| ID | Acción | Resultado esperado | Cat |
|----|--------|-------------------|-----|
| **Tool discovery** | | | |
| K7.1 | `tools/list` | Incluye 7 nuevos tools: `ltp/knowledge_add`, `ltp/knowledge_edit`, `ltp/knowledge_rm`, `ltp/knowledge_inspect`, `ltp/knowledge_list`, `ltp/knowledge_link`, `ltp/knowledge_unlink`. | `[happy]` |
| K7.2 | Cada tool en `tools/list` tiene `inputSchema` con todos los params documentados | Schemas completos con types, required, enums. | `[happy]` |
| **knowledge_add** | | | |
| K7.3 | `ltp/knowledge_add` con params válidos via JSON-RPC | Crea KN. Genera undo entry. Output contiene `created_knowledge_id`. | `[happy]` |
| K7.4 | `ltp/knowledge_add` sin `source_uri` ni `source_excerpt` | Error JSON-RPC con detalle `SOURCE_REQUIRED`. `isError: true`. | `[boundary]` |
| K7.5 | `ltp/knowledge_add` con `label: ""` | Error con detalle `LABEL_REQUIRED`. | `[boundary]` |
| K7.6 | `ltp/knowledge_add` con `type` inválido | Error JSON-RPC -32602 (invalid params). | `[boundary]` |
| **knowledge_edit** | | | |
| K7.7 | `ltp/knowledge_edit` actualiza status | Success. Undo entry generated. | `[happy]` |
| K7.8 | `ltp/knowledge_edit` con ID inexistente | `isError: true`, detalle `KNOWLEDGE_NOT_FOUND`. | `[boundary]` |
| **knowledge_rm** | | | |
| K7.9 | `ltp/knowledge_rm` elimina item | Success. | `[happy]` |
| K7.10 | `ltp/knowledge_rm` con ID inexistente | `isError: true`, `KNOWLEDGE_NOT_FOUND`. | `[boundary]` |
| **knowledge_link** | | | |
| K7.11 | `ltp/knowledge_link` con target válido | Link creado. | `[happy]` |
| K7.12 | `ltp/knowledge_link` con target inexistente | `isError: true`, `TARGET_NOT_FOUND`. | `[referential]` |
| K7.13 | `ltp/knowledge_link` duplicado | Warning `DUPLICATE_LINK`. Success (idempotente). | `[idempotent]` |
| **knowledge_unlink** | | | |
| K7.14 | `ltp/knowledge_unlink` elimina link | Success. | `[happy]` |
| K7.15 | `ltp/knowledge_unlink` link inexistente | `isError: true`, `LINK_NOT_FOUND`. | `[boundary]` |
| **knowledge_list** | | | |
| K7.16 | `ltp/knowledge_list` con filtro `unlinked: true` | Retorna items sin links. | `[happy]` |
| K7.17 | `ltp/knowledge_list` con filtro `target: "UDE-003"` | Retorna items linked a ese nodo. | `[happy]` |
| K7.18 | `ltp/knowledge_list` con `target` + `relation` | Filtro combinado funciona. | `[happy]` |
| K7.19 | `ltp/knowledge_list` sin filtros | Retorna todos. | `[happy]` |
| **knowledge_inspect** | | | |
| K7.20 | `ltp/knowledge_inspect` con ID válido | Detalle completo con links resueltos. | `[happy]` |
| K7.21 | `ltp/knowledge_inspect` con ID inexistente | `isError: true`. | `[boundary]` |
| **Extensiones a tools existentes** | | | |
| K7.22 | `ltp/node_add` con campo `epistemic` en params | Nodo creado con epistemic status. | `[happy]` |
| K7.23 | `ltp/node_add` sin campo `epistemic` | Nodo creado con default (hypothesis, omitido de JSON). | `[happy]` |
| K7.24 | `ltp/node_edit` con campo `epistemic` | Epistemic actualizado. | `[happy]` |
| K7.25 | `ltp/node_list` con filtro `epistemic` | Filtra correctamente. | `[happy]` |
| K7.26 | `ltp/trace` con `show_knowledge: true` | Incluye knowledge por nodo. | `[happy]` |
| K7.27 | `ltp/trace` sin `show_knowledge` (o false) | Output normal sin knowledge. | `[happy]` |
| K7.28 | `ltp/tree_walk` con `show_knowledge: true` | Incluye resumen knowledge por nodo. | `[happy]` |
| K7.29 | `ltp/status` | Incluye `knowledge_health`. | `[happy]` |
| K7.30 | `ltp/validate` | Reporta warnings de knowledge (DANGLING, EPISTEMIC_*). | `[happy]` |
| **Undo integration via MCP** | | | |
| K7.31 | `ltp/knowledge_add` → `ltp/undo` | KN eliminado. Undo funciona cross-tool. | `[interaction]` |
| K7.32 | `ltp/knowledge_link` → `ltp/undo` | Link eliminado. | `[interaction]` |
| **Concurrent/sequential** | | | |
| K7.33 | 3 `ltp/knowledge_add` llamadas secuenciales rápidas | IDs secuenciales sin colisión (lock garantiza). | `[ordering]` |

---

## Resumen de Conteos

| Fase | UATs | Happy | Boundary | Interaction | Corrupt | Idempotent | Ordering | Referential |
|------|------|-------|----------|-------------|---------|------------|----------|-------------|
| K1 | 16 | 4 | 6 | 2 | 2 | 1 | 2 | 0 |
| K2 | 47 | 12 | 14 | 8 | 1 | 4 | 4 | 0 |
| K3 | 37 | 6 | 6 | 8 | 0 | 1 | 0 | 9 |
| K4 | 16 | 5 | 4 | 2 | 2 | 1 | 0 | 0 |
| K5 | 47 | 8 | 22 | 4 | 0 | 0 | 0 | 5 |
| K6 | 28 | 7 | 3 | 15 | 0 | 0 | 3 | 0 |
| K7 | 33 | 16 | 6 | 3 | 0 | 1 | 1 | 1 |
| **TOTAL** | **224** | **58** | **61** | **42** | **5** | **8** | **10** | **15** |

---

## Decisiones de Diseño Abiertas (Reveladas por UATs)

Estas UATs revelan puntos donde la spec requiere decisión explícita:

1. **K3.8**: ¿Se permite mismo target con different relations? → **Sí** (caso legítimo).
2. **K3.11**: ¿Link a nodo huérfano (en pool pero no en tree)?  → **Sí** (validación de attachment es concern del tree).
3. **K3.25**: ¿`unlink --from X` elimina TODOS los links a X o solo uno? → **Todos** (unlink es por target, no por relation).
4. **K3.32**: ¿`list --target X` muestra KN una vez o una vez por link? → **Una vez** con array de relations.
5. **K5.13**: ¿`UNGROUNDED` requiere 0 supports totales o 0 supports verified? → **0 supports totales** (cualquier support, incluso unverified, es grounding).
6. **K5.27**: ¿`UPGRADEABLE` se emite si también hay contradicción? → **No** (contradecido anula sugerencia de upgrade).
7. **K5.32**: ¿Validate con tree filter reporta warnings de epistemic solo para nodos en ese tree? → **Sí** (consistente con validate existente que es per-tree).
8. **K6.22**: ¿`node split` deja dangling refs de knowledge? → **Sí** (el ID original desaparece).
