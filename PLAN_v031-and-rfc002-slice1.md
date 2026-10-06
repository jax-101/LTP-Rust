# PLAN — v0.3.1 (PATCH) + RFC-002 Slice 1 (refs + meta-grafo inferido) → v0.4.0

> Al ejecutar: copiar este plan a la raíz como `PLAN_v031-and-rfc002-slice1.md` (regla: docs en la raíz del proyecto).
> Restricción vigente: **no stagear, commitear ni editar** `RFC-002_meta-graph.md`, `RFC-002_appendix_20-transitions.md` ni `RFC-002_appendix_use-cases.md`. Las correcciones que necesitan se listan en §6 y no se aplican.

## Contexto

Tras v0.3.0, la siguiente fase es RFC-002 (meta-grafo) sobre JSON, con extracción oportunista de `ltp-core` (decisión de dos velocidades confirmada el 2026-10-06). La investigación de solo lectura (`.superpowers/research/rfc002-backlog.md` y `code-map.md`, más 2 exploraciones graph-first) encontró:

- **2 bugs latentes**:
  - `Counters::rebuild` reemite IDs que viven dentro del JSON de los árboles.
  - El schema MCP anuncia unos roles de EC que el validador rechaza.
- **4 huecos de integridad** que afectan a S1:
  - `node rm` y `node split` ignoran las ramas NBR.
  - `node split` pierde la metadata.
  - `validate` salta en silencio los nodos ilegibles.
- **10 decisiones** (D-1..D-10). Se evalúan abajo con los Six Hats. D-4 ya está decidida por el usuario: **sin `relation_type` en S1**.

**Resultado esperado**:
- **v0.3.1**: PATCH sin cambio de contrato.
- **v0.4.0**: MINOR con:
  - `refs` en los nodos;
  - tool nº 72 `tree_relation_list` (relaciones inferidas, estructurales y sin tipo);
  - warnings de norma;
  - integridad NBR.

Cada cambio lleva UATs automáticas adversariales: bordes, datos corruptos, transiciones ilegales, paridad CLI/MCP y mutation checks.

## 0. codebase-memory (antes de cada tarea)

- Estado al planificar: índice fresco (indexed_at 2026-10-06T09:31:38Z). Cobertura `metadata_match` en los ficheros de node/, tree/, validate/, mcp/, workspace/, history/ y main.rs.
- Antes de cada tarea:
  - ejecutar `detect_changes`; si hay deriva, `index_repository` (incremental);
  - explorar graph-first (`search_graph`, `trace_path`, `get_code_snippet`); usar grep solo para literales y docs.
- Tras cada commit: reindexar para que la tarea siguiente vea los símbolos nuevos (`CrossRef`, `infer_relations`, `tree_memberships`).

## 1. Six Thinking Hats sobre D-1..D-10

Blanco = hechos verificados · Negro = riesgos (obligatorio) · Amarillo/Verde = beneficio y alternativa · Rojo = ergonomía · Azul = encaje con los ADRs.

| D | Blanco | Negro | Amarillo / Verde / Rojo | Azul → **Veredicto** |
|---|---|---|---|---|
| **D-1** Forma de `refs` | `NodeMetadata.extra` usa flatten; un binario viejo conserva claves desconocidas. Pasar de string a objeto más tarde sería un cambio de shape. | (1) Sin `tree`, un destino adjunto a N árboles produce N relaciones (fan-out). (2) `tree` puede quedar obsoleto si el destino se desadjunta. (3) Una clave futura dentro del objeto (`kind`) la descartaría el binario v0.4.0. | Los objetos permiten añadir `assumption` más adelante (F05/F13) de forma aditiva. CLI `--ref NODE[@TREE]`, MCP `refs:[{node,tree?}]`. | **Aceptar (b)**: `CrossRef {node, tree: Option}`. Es un set ordenado por `(node, tree)`; un duplicado es un no-op idempotente. Se valida al escribir (§3). El fan-out sin `tree` es intencionado y está documentado. |
| **D-2** Dónde viven | Los nodos son globales en el pool. El snapshot de undo cubre `nodes/`, así que es deshacible gratis. Hay 8 literales `NodeMetadata{..}`. | Un nodo adjunto a CRT y FRT a la vez genera relaciones desde ambos. `split` hoy pierde la metadata, así que **perdería los refs**. | Cero cambios en `Storage`, undo o snapshot. Un `NodeMetadata::new(status)` elimina los 8 literales. | **Aceptar (a)**, en `node.metadata.refs`, campo tipado `#[serde(default, skip_serializing_if="Vec::is_empty")]`. Obliga a arreglar `split` y `rm` (T-S1.1). |
| **D-3** Solo inferido en S1 | ADR-004 dice que las vistas se calculan al vuelo. No existe `relations/`. | Una relación no persistida no tiene ID, y S3 necesitará identidad para "fijarla". | La clave natural `(referencing, referenced)` es determinista y no exige migración. | **Aceptar**: S1 solo calcula; S3 fija (persiste) con la clave natural. Sin IDs `TREL-` en S1. |
| **D-4** Tipo por par | Un mismo ref (UDE→NC) encaja con T01 (GT→CRT `gap_analysis`) y con T05 (CRT→GT feedback). Lo mismo T21/T26 y FRT-DE→CRT-UDE (T07). | Inferir el tipo **afirma la intención** del analista, lo que viola ADR-001, y la tabla de 36 celdas es impugnable. `from/to` también afirma una dirección. | Un hecho estructural neutro es suficiente para la UI ("ver conexiones"). Añadir `relation_type` en S3 es MINOR. | **Decidido (usuario)**: sin `relation_type`; extremos neutros `referencing` y `referenced`; `logic` por extremo, derivado de `tree_type.logic()` (ADR-014). La memoria "Opción 2" se actualiza. |
| **D-5** Alcance de `NORM_REF_MISSING` | Validate tiene el patrón de entrada sintética `_knowledge_pool`. El golden `validate.json` no tiene GT. | (1) Ruido en workspaces sin GT (se mitiga con la condición ≥1 GT). (2) ¿Una ref a una NC que no está en ningún GT cuenta? Si cuenta, la relación no aparece y el usuario no se entera. (3) Una UDE en 2 CRT da 2 warnings. | Un warning por nodo con `trees:[...]` en el contexto. | **Aceptar (b)**: UDE adjunta a un CRT o en los edges de una rama NBR, solo si hay ≥1 GT y siempre como warning. Se satisface **solo** con una ref a una norma adjunta a ≥1 GT. Ni las refs colgantes ni las de tipo erróneo satisfacen. |
| **D-6** Tipos norma | v0.3.0 entrega GOAL ← CSF ← NC. Los GT legacy usan OBJ. | "OBJ legacy" necesita contexto: OBJ es también un tipo válido en EC y PRT. | Predicado puro `is_norm(node_type, attached_to_gt)`. | **Aceptar**: norma = NC o CSF, u OBJ **solo si está adjunto a un GT**. GOAL queda fuera: las UDE se miden contra NC/CSF (Dettmer). |
| **D-7** Endpoint NBR | Los nodos NBR viven en `nbr_branches[].edges` y **no** tienen por qué estar en `tree.nodes` (link/commands.rs:163). | Un nodo en el tronco y en una rama a la vez da 2 extremos. Un nodo en 2 ramas da 2 extremos. | Endpoint `{tree, nbr}` con `nbr: null` siempre presente (convención de INTEGRATION: campo de valor = null). | **Aceptar (a)**, con la pertenencia calculada por un helper único `tree_memberships(&[Tree])` que incluye las ramas NBR. Se emiten todos los extremos, deduplicados y ordenados. |
| **D-8** Roles EC | ec.rs exige objective/requirement/prerequisite y PRE→REQ. tools.rs:157 dice "root, leaf, intermediate". `--role` no tiene help. Ningún golden toca esto. | Validar roles en `attach` rompería entradas aceptadas hasta ahora (no es PATCH). Una descripción copiada a mano puede volver a divergir. ec.rs **no** valida la XOR que ES:371 exige (hueco aparte). | Una constante única `EC_ROLES` de la que sale la descripción hace imposible la deriva (test). | **Aceptar (a)** como PATCH: descripción MCP + help CLI + ES:124 + USAGE_GUIDE §3.3 generados desde `EC_ROLES`. La falta de validación XOR queda **registrada** como hueco, sin arreglar (sería MINOR o MAJOR). |
| **D-9** Nombre | `KnowledgeRelation` existe (knowledge/types.rs:37). La CLI anida subcomandos (`tree walk`). | Ninguno relevante. | MCP `ltp/tree_relation_list` ↔ CLI `tree relation list`. | **Aceptar (b)**. |
| **D-10** Severity | Los flujos usan 4 niveles. Es de S2. | Fijar ahora un enum sin usarlo es deuda especulativa. | Necesitará `Ord` para W09 ("baja la severidad"). | **Diferir la implementación a S2.** Se fija solo en el ADR: `critical/high/medium/low`, `snake_case`, `Ord` en el orden declarado. |

## 2. v0.3.1 — PATCH (sin cambio de contrato)

### T-A1 `Counters::rebuild` recupera los IDs embebidos — `src/workspace/counters.rs`
- Añadir `"FB"` a `ENTITY_TYPES`. `TREE` se mantiene (el contador está muerto, pero se conserva por compatibilidad del fichero).
- `scan_directory`: aceptar el prefijo **solo si es ASCII mayúsculas** y no aplicar `to_uppercase`. Esto corrige la clave basura `TREE-CRT: 2024` que genera `tree-crt-2024.json`.
- Nuevo `scan_tree_contents(dir)`:
  - para cada `trees/*.json`, parsear con `serde_json::Value` (no `load_tree`, para no depender de storage ni romperse con un schema legacy);
  - recorrer de forma recursiva todo valor string bajo la clave `"id"` que case con `^[A-Z]+-\d+$` y quedarse con el máximo por prefijo;
  - si un fichero no se puede leer o parsear, saltarlo y seguir con el resto.
- Sin regex crate: un parser manual (`split_once('-')`, prefijo `is_ascii_uppercase`, sufijo `parse::<u64>`) compartido por los dos escaneos.
- Commit `fix(F3): counters rebuild recupera IDs embebidos en árboles (LINK/ASM/NBR/MACRO/MASM/FB)`.

### T-A2 Roles de EC desde una fuente única
- En `src/validate/ec.rs`: `pub const EC_ROLES: [&str; 3] = ["objective","requirement","prerequisite"];`, usada por el validador.
- En `src/mcp/tools.rs:157`: la descripción se genera con `format!` desde `EC_ROLES` ("Required in EC trees: objective, requirement, prerequisite; free-form in other trees"). Corregir también `with_capacity(61)` → `72` (anticipa S1).
- En `src/main.rs:287`: doc `///` en `role`, con el mismo texto.
- `ENGINE_SPEC.md:124` y `USAGE_GUIDE.md` §3.3: literales de rol y dirección PRE→REQ→OBJ.
- Commit `fix(F3): vocabulario de roles EC alineado con el validador (MCP, CLI, docs)`.

### UATs v0.3.1 — `tests/counters_rebuild.rs` + `tests/ec_roles.rs`
| # | Caso (adversarial) | Aserción |
|---|---|---|
| U1 | Un workspace real, construido por CLI, tiene LINK, ASM, NBR, MACRO, MASM y FB con un máximo ≥2 cada uno. Se borra `counters.json` y se crea uno nuevo de cada tipo. | Cada ID nuevo es máximo+1. El helper `all_ids(workspace)` (recorre nodes/, trees/ y knowledge/) **no tiene duplicados**. |
| U2 | `counters.json` corrupto: vacío, `garbage`, `[]`, `{"UDE":"x"}`. | Misma recuperación que U1 y sin panic. |
| U3 | `trees/tree-x.json` corrupto y además falta `counters.json`. | El comando no hace panic; los prefijos de los árboles sanos y de los nodos se recuperan. |
| U4 | Frontera de 3 dígitos: un edge `LINK-999` (fixture escrito a mano) y luego `link connect`. | El ID nuevo es `LINK-1000`; tras otro rebuild, el siguiente es `LINK-1001`. |
| U5 | Árbol `tree-crt-2024` y rebuild. | `counters.json` no tiene la clave `TREE-CRT`. Los ficheros en minúscula o sin número en nodes/ se ignoran. |
| U6 | Rebuild dos veces sobre el mismo disco. | `counters.json` es idéntico byte a byte (determinismo). |
| U7 | `"id": "LINK-050"` anidado en un sitio inesperado (assumption de un macro edge dentro de una rama NBR). | Se recupera (recorrido recursivo). |
| U8 | Unit: el escaneo **no** cuenta referencias (`from`, `interior_links`, `projection_refs`) como IDs nuevos, salvo bajo la clave `id`. | Un fixture con `from: ["UDE-900"]` sin fichero no sube UDE a 900. |
| U9 | **Mutation check**: comentar la llamada a `scan_tree_contents`. | U1 y U7 fallan; luego se restaura. |
| U10 | `tools/list`: la descripción del `role` de `ltp/tree_attach`. | Contiene cada valor de `EC_ROLES` y no contiene "root" ni "leaf". |
| U11 | EC construida con `root/intermediate/leaf` (la trampa de los flujos RFC). | `validate` da success=false con `EC_VALIDATION`, `sub_rule=missing_objective`. La misma EC con los roles correctos da success=true. |
| U12 | Roles casi correctos: `Objective`, `" objective"`, `requirements`. | Falla (fija que es case-sensitive y exacto). |
| U13 | `ltp tree attach --help`. | Lista los 3 roles. |

**Release v0.3.1**: gate de 4 comandos → `Cargo.toml` 0.3.1 → CHANGELOG `[0.3.1]` (Fixed) → PROGRESS → `chore(release): v0.3.1` → tag anotado → `git push --follow-tags`. INTEGRATION no cambia (el contrato no cambia).

## 3. v0.4.0 — RFC-002 Slice 1 (MINOR)

### T-S1.0 ADR-015 (doc primero) — `ADR.md`
Registra D-1..D-10 tal como quedan en §1, con la razón de D-4 (ADR-001). Precisa que S1 **no** elimina `edge.logic` (K-20). Commit `docs(RFC2): ADR-015 refs y meta-grafo inferido`.

### T-S1.1 Tipos e integridad previa (type-first)
- `src/node/types.rs`:
  - `pub struct CrossRef { pub node: String, #[serde(default)] pub tree: Option<String> }` (Ord, en el orden `node`, `tree`);
  - `NodeMetadata.refs: Vec<CrossRef>` con `serde(default, skip_serializing_if = "Vec::is_empty")`;
  - `NodeMetadata::new(status)`, que sustituye los 8 literales (node/commands.rs ×3, assume, path, macro_edge, validate/clr ×2).
- `src/meta/mod.rs` (módulo **puro**, sin `Storage`, candidato a `ltp-core`):
  - `tree_memberships(&[Tree]) -> BTreeMap<String, BTreeSet<Endpoint>>`, que incluye tronco y `nbr_branches[].edges` (from/to) y `source_node`;
  - `Endpoint { tree, nbr: Option<String> }`.
- `node rm`, que hoy deja las NBR colgando:
  - quitar los edges de rama NBR que tocan el nodo;
  - si `source_node` era el nodo, eliminar la rama, con warning `NBR_BRANCH_REMOVED {tree_id, nbr_id}`;
  - quitar las **refs entrantes** de otros nodos, con warning `REFS_STRIPPED {node_id, referencing}` (mismo patrón que `KNOWLEDGE_ORPHANED`, pero limpiando).
- `node split`:
  - copiar las `refs` salientes a ambos hijos;
  - reescribir las refs entrantes al nodo partido para que apunten a **ambos** hijos (set ordenado);
  - redirigir también los edges NBR.
- Commit `feat(RFC2): CrossRef, membresía de árbol con NBR e integridad en node rm/split`.

### T-S1.2 Escribir refs: `node add` / `node edit` (CLI + MCP)
- CLI: `node add --ref NODE[@TREE]` (repetible) y `node edit --add-ref/--rm-ref NODE[@TREE]` (repetibles).
- MCP:
  - `node_add.refs: [{node, tree?}]` y `node_edit.add_refs/rm_refs`;
  - parser **estricto** nuevo `get_ref_array` junto a `get_str_array` (dispatch.rs:170): un no-objeto, una falta de `node` o un tipo erróneo dan error de params inválidos (código existente del dispatch).
- Validación bloqueante (es integridad referencial; Invariante 2):
  - destino inexistente → `NODE_NOT_FOUND`;
  - `tree` inexistente → `TREE_NOT_FOUND`;
  - destino no presente en ese árbol (incluyendo ramas NBR) → `NODE_NOT_IN_TREE`;
  - auto-ref → **`SELF_REF`** (código nuevo, MINOR);
  - `rm_ref` de una ref inexistente → no-op con warning `REF_NOT_PRESENT`;
  - un duplicado → no-op idempotente.
- `NodeAddData`, `NodeEditData` y `node inspect` exponen `refs` y `referenced_by` (aditivo).
- Commit `feat(RFC2): refs en node add/edit/inspect (CLI+MCP)`.

### T-S1.3 `infer_relations` + tool `tree_relation_list`
- Función pura `meta::infer_relations(&[Node], &[Tree]) -> Vec<InferredRelation>`:
  - para cada nodo N y cada ref r: extremos(N) × extremos(r.node), filtrados por `r.tree` si viene;
  - descartar el par cuando ambos extremos son el mismo `Endpoint`;
  - agrupar por `(referencing, referenced)` con `basis` ordenado;
  - salida ordenada (BTreeMap).
- Shape:
  ```json
  { "relations": [ { "referencing": {"tree":"tree-crt-x","nbr":null}, "referenced": {"tree":"tree-gt-y","nbr":null},
      "logic": {"referencing":"sufficiency","referenced":"necessity"},
      "basis": [{"node":"UDE-001","ref":"NC-003"}], "inferred": true } ], "count": 1 }
  ```
- `logic` sale de `tree_type.logic()`.
- Param opcional `tree`: filtra las relaciones que tocan ese árbol; si no existe, `TREE_NOT_FOUND`.
- Cableado en 4 sitios: `TreeAction::Relation{List}` + brazo en main, `tools.push` (nº 72), brazo en `dispatch_tool`, `dispatch_tree_relation_list`. Solo lectura, sin history.
- Actualizar `assert_eq!(tools.len(), 71→72)` (fase_12.rs:336, :523).
- Golden `contract/tree_relation_list.json` con un fixture GT+CRT con ref.
- Commit `feat(RFC2): tree_relation_list — meta-grafo inferido desde refs (tool 72)`.

### T-S1.4 Warnings de validate (entrada sintética `_meta_graph`)
Función pura `meta::check_refs(nodes, trees) -> Vec<OutputWarning>`, llamada tras `_knowledge_pool` (validate/mod.rs:252). Con `--tree`, se filtra a los nodos de ese árbol. Warnings:
- `DANGLING_NODE_REF {node_id, ref, reason: node_missing|tree_missing|not_in_tree}`, cuando un dato manual o un binario viejo dejan refs rotas.
- `NORM_REF_MISSING {node_id, trees}`, según D-5/D-6.
- `NODE_UNREADABLE {node_id}`, para un nodo listado pero que no carga (hoy se salta en silencio: mod.rs:174).

`contract/validate.json` no debe cambiar porque el fixture no tiene GT (se aserta). Commit `feat(RFC2): validate — DANGLING_NODE_REF, NORM_REF_MISSING, NODE_UNREADABLE`.

### UATs S1 — `tests/rfc002_s1.rs` (helpers de `tests/tree_logic.rs`: `mcp_call`, `run_ltp`, `attach`…)
| # | Caso adversarial | Aserción |
|---|---|---|
| R1 | Happy path: GT(NC-001) y CRT(UDE con `--ref NC-001`). | 1 relación; `logic` nec/suf; `basis` exacto; `inferred:true`; sin `relation_type`. |
| R2 | Ref a un nodo inexistente / árbol inexistente / nodo fuera de ese árbol. | `NODE_NOT_FOUND` / `TREE_NOT_FOUND` / `NODE_NOT_IN_TREE`. El nodo en disco queda **intacto** (bytes). |
| R3 | Auto-ref en add y en edit. | `SELF_REF`; nada escrito. |
| R4 | Ref duplicada (`--ref NC-001 --ref NC-001`) y `add_ref` repetido. | Una sola ref; el orden en disco es canónico al margen del orden de entrada. |
| R5 | MCP con `refs` malformado: `"NC-001"` (string), `[{}]`, `[{"node":5}]`, `[{"tree":"x"}]`. | Error de params; nada escrito. |
| R6 | Multi-attach: NC en 2 GT y ref sin `tree`. | 2 relaciones. Con `@tree-gt-a`, solo 1. |
| R7 | UDE colateral solo en edges de una rama NBR (no en `tree.nodes`). | Extremo `{tree:FRT, nbr:"NBR-001"}`; la ref se acepta. Un nodo en tronco y rama da 2 extremos. |
| R8 | Ref dentro del mismo árbol (UDE→RC en el mismo CRT). | No hay relación, pero la ref se guarda. |
| R9 | `node rm NC-001` referenciada por 2 UDE. | Las refs se quitan de ambas; un `REFS_STRIPPED` por cada una; `tree_relation_list` vacío; **undo** restaura los bytes de los 3 nodos. |
| R10 | `node rm` del `source_node` de un NBR y de un nodo interior de la rama. | La rama se elimina (`NBR_BRANCH_REMOVED`) o pierde el edge. `validate` sin `DANGLING`. Undo exacto. |
| R11 | `node split` de una NC referenciada y de una UDE con refs. | Las refs entrantes apuntan a ambos hijos; los hijos heredan las salientes; los edges NBR se redirigen. |
| R12 | Refs escritas a mano: destino borrado, `tree` borrado, destino desadjuntado. | `DANGLING_NODE_REF` con el `reason` correcto en cada caso; `validate` sigue success=true (warning). |
| R13 | `refs: "garbage"` en el JSON del nodo. | `NODE_UNREADABLE` (antes, silencio); los demás nodos se validan. |
| R14 | `NORM_REF_MISSING`: workspace sin GT. | No hay warning. Con GT, la UDE sin ref → warning; ref a NC que no está en ningún GT → warning; ref a RC → warning; ref a CSF de GT → ok; ref a OBJ de GT → ok; ref a OBJ de una EC → warning. |
| R15 | `NORM_REF_MISSING` sobre UDE de una rama NBR, y UDE adjunta a 2 CRT. | Un warning por nodo con `trees` ordenado. |
| R16 | `validate --tree` filtrado. | Solo los warnings de nodos de ese árbol. |
| R17 | Round-trip con un binario viejo: un struct de test que imita v0.3.0 (`{status, #[flatten] extra}`) lee y reescribe el nodo. | El binario nuevo vuelve a ver `refs` idénticas. Un nodo **sin** refs se reescribe byte-idéntico a v0.3.0. |
| R18 | Determinismo: el mismo workspace construido en 2 órdenes de comandos distintos. | `tree_relation_list` igual byte a byte. |
| R19 | Paridad CLI ↔ MCP en R1, R2, R5 (vía CLI `--ref` inválido como `@`, `NC-001@`) y R9. | Mismo `data`, `errors` y `warnings`. |
| R20 | `tree_relation_list --tree` inexistente; workspace vacío. | `TREE_NOT_FOUND`; `{relations:[],count:0}` con success. |
| R21 | `tools/list`. | Hay 72 tools; existe `ltp/tree_relation_list`; `node_add` tiene `refs`. |
| R22 | Unit puro de `infer_relations` y `check_refs` (~15): permutaciones de entrada → misma salida; fan-out N×M; un endpoint idéntico se descarta. | Sin I/O. |
| R23 | **Mutation checks**: (a) quitar el filtro de `r.tree`, (b) quitar la condición ≥1 GT, (c) quitar la limpieza de refs en `node rm`. | Fallan R6, R14 y R9 respectivamente; luego se restaura. |

> `--dry-run` hoy solo aplica a init/undo/redo (main.rs:1202), así que no hay UAT de dry-run en las mutaciones. Queda anotado como hueco, fuera de alcance.

### T-S1.5 Docs + release v0.4.0
- `ENGINE_SPEC.md`: `refs`, `tree relation list`, los códigos nuevos (`SELF_REF`, `REF_NOT_PRESENT`, `REFS_STRIPPED`, `NBR_BRANCH_REMOVED`, `DANGLING_NODE_REF`, `NORM_REF_MISSING`, `NODE_UNREADABLE`).
- `USAGE_GUIDE.md`: recorrido "NC del GT → UDE con ref → ver relación".
- `INTEGRATION.md`:
  - §4: RFC-002 S1 pasa a estable;
  - versión mínima a 0.4.0.
- `RELEASE_POLICY.md`: añadir la línea de feature-gate `>= 0.4.0`.
- `CHANGELOG.md`: `[0.4.0]`.
- `PROGRESS.md`: recuento real de tests y factor de escala.
- Release: tag `v0.4.0` y push (fin de fase).

## 4. Ficheros críticos y reutilización
- **Reutilizar**:
  - `OutputWarning::new().with_context()` (src/output.rs);
  - el patrón `validate_knowledge` + `_knowledge_pool` (validate/mod.rs:252);
  - `execute_nbr_list` como plantilla de solo lectura (nbr/mod.rs:423);
  - `nodes_involved` de NBR (nbr/mod.rs:44);
  - `get_str_array` (dispatch.rs:170);
  - el patrón repetible de `KnowledgeAction::Edit` (main.rs:685);
  - `check_or_update_golden` (tests/contract.rs:204);
  - los helpers de tests/tree_logic.rs.
- **Modificar**:
  - src/workspace/counters.rs, src/validate/ec.rs, src/mcp/tools.rs, src/mcp/dispatch.rs, src/main.rs;
  - src/node/types.rs, src/node/commands.rs;
  - src/validate/mod.rs;
  - **nuevo** src/meta/mod.rs;
  - tests/fase_12.rs, tests/contract.rs;
  - docs del §3.

## 5. Verificación (en cada commit con código)
1. `cargo check --all-targets --all-features`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. `cargo test --workspace`
4. `cargo fmt --all -- --check`

Además:
- mutation checks U9 y R23, documentados en el commit;
- e2e manual con el MCP `ltp` en un workspace temporal: GT+CRT, ref, `tree_relation_list`, `node rm`, undo;
- reindexar codebase-memory tras cada commit.

**Commits**: uno por tarea (T-A1, T-A2, release v0.3.1; T-S1.0..T-S1.5, release v0.4.0). Push en cada release. Sin `.unwrap()`/`.expect()` en producción y con `///` en todo lo `pub`.

## 6. Fuera de alcance, registrado
- **Correcciones pendientes en los docs RFC-002** (los edita el usuario; aquí no se tocan):
  - roles EC root/leaf → objective/requirement/prerequisite en UC:347-367 y :567-587;
  - OBJ→CSF en los flujos GT (K-6);
  - D-4 sin tipo en S1;
  - NBR como endpoint `{tree,nbr}` (K-1).
- **Huecos sin arreglar en este plan**:
  - falta la validación de XOR en EC (ES:371);
  - `COUNTERS_REBUILT` se descarta (fs_storage.rs:231); exponerlo sería MINOR;
  - `--dry-run` en las mutaciones.
- **Memoria**:
  - actualizar `project_rfc-002-meta-graph` (D-4 revisa la Opción 2);
  - MyBrain: aprendizaje "inferir relaciones desde refs: estructurales, no tipadas (ADR-001)".
