# Plan de Desarrollo — `ltp-engine` (Rust)

## Dependencias entre Fases

```
F1 ──→ F2a ──→ F3 ──→ F4 ──→ F2b ──→ F5
                                       │
                             ┌─────────┼─────────┐
                             ▼         ▼         ▼
                            F6        F7        F8
                             │         │
                             ▼         ▼
                            F9       F10
                             │         │
                             └────┬────┘
                                  ▼
                                 F11
                                  │
                                  ▼
                              E2E Tests
                                  │
                                  ▼
                                 F12
```

- **F1** incluye diseño de traits (Storage, Engine) + estrategia de IDs + pipeline de escritura con hook de snapshot (no-op inicial).
- **F2a** (node add/edit/list/search) es standalone — no necesita trees ni links.
- **F2b** (node rm/split/inspect cross-tree) se mueve después de F4 porque depende de trees y edges.
- **F6, F7, F8** pueden avanzar en paralelo una vez F5 está lista.
- **F9** requiere F6. **F10** requiere F4 mínimo.
- **F11** activa el hook de snapshot diseñado en F1 — no es un retrofit.
- **E2E Tests** validan workflows completos tras F11.
- **F12** envuelve todo como servidor MCP.

---

## Fase 1: Fundación (Workspace, Storage Layer & IDs)

**Scope**: `ltp init`, `ltp status`, traits `Storage`/`Engine`, pipeline de escritura, lock file, config, estrategia de IDs.

**Archivos**: `src/workspace/`, `src/storage.rs`, `src/engine.rs`

**Estado**: [ ] Pendiente

### Diseño de Interfaces (Traits)

Antes de implementar init/status, se definen los contratos:

```rust
/// Capa de persistencia. Toda escritura pasa por aquí.
trait Storage {
    fn load_config(&self) -> Result<WorkspaceConfig>;
    fn save_config(&self, config: &WorkspaceConfig) -> Result<()>;
    fn load_node(&self, id: &str) -> Result<Node>;
    fn save_node(&self, node: &Node) -> Result<()>;
    fn delete_node(&self, id: &str) -> Result<()>;
    fn list_node_ids(&self) -> Result<Vec<String>>;
    fn load_tree(&self, id: &str) -> Result<Tree>;
    fn save_tree(&self, tree: &Tree) -> Result<()>;
    fn delete_tree(&self, id: &str) -> Result<()>;
    fn list_tree_ids(&self) -> Result<Vec<String>>;
    fn acquire_lock(&self, command: &str) -> Result<()>;
    fn release_lock(&self) -> Result<()>;
    fn next_id(&self, prefix: &str, node_type: &str) -> Result<String>;
}

/// Hook de snapshot — no-op en F1, se activa en F11.
trait SnapshotHook {
    fn before_mutation(&self, affected_files: &[&str]) -> Result<()>;
    fn after_mutation(&self, affected_files: &[&str]) -> Result<()>;
}

/// Motor en memoria — carga workspace, ejecuta operaciones.
trait Engine {
    fn init(&mut self, name: &str) -> Result<()>;
    fn status(&self) -> Result<WorkspaceStatus>;
    fn validate(&self, tree_id: Option<&str>) -> Result<ValidationResult>;
}
```

### Estrategia de IDs

- Contador secuencial por tipo, almacenado en `.ltp/counters.json`:
  ```json
  {"UDE": 3, "RC": 1, "INT": 2, "LINK": 5, "ASM": 2, "NBR": 1, "MACRO": 1, "FB": 0}
  ```
- Formato nodos: `<TYPE>-<SEQ_PADDED>` (ej. `UDE-001`, `RC-003`, `INJ-012`). Sin prefijo de tree.
- Formato links: `LINK-<SEQ>` (ej. `LINK-001`). Contador global, no por tree.
- Formato assumptions: `ASM-<SEQ>` (ej. `ASM-001`). Contador global.
- Formato feedback: `FB-<SEQ>` (ej. `FB-001`). Contador global.
- Formato NBR: `NBR-<SEQ>` (ej. `NBR-001`). Contador global.
- Formato macro edges: `MACRO-<SEQ>` (ej. `MACRO-001`). Contador global.
- El lock file protege de colisiones: solo un proceso muta a la vez.
- Si `counters.json` falta o está corrupto: auto-reconstrucción escaneando `nodes/` y `trees/`, emitiendo warning `COUNTERS_REBUILT`.

### Pipeline de Escritura

Toda mutación sigue este flujo:
1. `acquire_lock(command)`
2. `snapshot_hook.before_mutation(files)` — (no-op hasta F11)
3. Escribir a `.ltp/tmp/<file>` (staging)
4. `rename` atómico de tmp → destino (para cada fichero)
5. `snapshot_hook.after_mutation(files)` — (no-op hasta F11)
6. `release_lock()`

### UATs

| ID | Comando | Resultado esperado |
|----|---------|-------------------|
| 1.1 | `ltp init --name "Logistica-2026"` | Crea `nodes/`, `trees/`, `ltp.config.json`, `.ltp/` con `undo/`, `redo/`, `tmp/`, `counters.json`. Retorna JSON con `success: true`. |
| 1.2 | `ltp init` (repetido en mismo dir) | Error `WORKSPACE_ALREADY_EXISTS`. |
| 1.3 | `ltp status --json` | Retorna conteo 0 nodos, 0 trees, `valid_dag: true`, `orphan_nodes_count: 0`. |
| 1.4 | Dos procesos lanzan comando mutante concurrentemente | Uno adquiere lock, el otro falla con `WORKSPACE_LOCKED`. |
| 1.5 | Matar proceso con lock → ejecutar otro comando | Auto-break del stale lock con warning `STALE_LOCK_REMOVED`. |
| 1.6 | `ltp init --name "Test"` + verificar `counters.json` existe | Fichero creado con todos los contadores a 0. |

---

## Fase 2a: Entidades — Operaciones Standalone

**Scope**: `node add/edit/list/search` (no requieren trees ni links).

**Archivos**: `src/node/`

**Estado**: [ ] Pendiente

### UATs

| ID | Comando | Resultado esperado |
|----|---------|-------------------|
| 2a.1 | `ltp node add "El tiempo de entrega supera 15 días" --type UDE --tags logistica,critico` | Crea `nodes/UDE-001.json`. Output JSON con `success: true`, `data.id: "UDE-001"`. Counter UDE incrementa a 1. |
| 2a.2 | `ltp node add "Vendemos poco porque no hay marketing" --type UDE` | Warning CLR#2: "conjunción causal detectada: *porque*". Nodo se crea igual (no bloqueante). |
| 2a.3 | `ltp node add "Falta de capacidad" --type RC` | Crea `nodes/RC-001.json`. Counter RC incrementa. |
| 2a.4 | `ltp node edit UDE-001 --label "Nuevo texto"` | Actualiza label en disco. |
| 2a.5 | `ltp node edit UDE-001 --add-tag urgente` | Añade tag. |
| 2a.6 | `ltp node edit UDE-001 --rm-tag critico` | Quita tag. |
| 2a.7 | `ltp node edit UDE-001 --observable false` | Marca como no observable. |
| 2a.8 | `ltp node list` (sin tree, todos los nodos del pool) | Lista todos los nodos del pool global. |
| 2a.9 | `ltp node search --query "entrega"` | Encuentra nodos por substring en label. |

---

## Fase 3: Vistas (Trees)

**Scope**: `tree new/list/rm/attach/detach/clone/diff/walk`

**Archivos**: `src/tree/`

**Estado**: [ ] Pendiente

### UATs

| ID | Comando | Resultado esperado |
|----|---------|-------------------|
| 3.1 | `ltp tree new crt "CRT Logística"` | Crea `trees/tree-crt-logistica.json` con `logic: sufficiency`. |
| 3.2 | `ltp tree new ec "Evaporating Cloud"` | Crea tree con `logic: necessity`. |
| 3.3 | `ltp tree list` | Lista trees con id, name, type, node count. |
| 3.4 | `ltp tree attach --tree tree-crt-logistica --node UDE-001` | Nodo aparece en `nodes[]` del tree. `status` lo reporta como huérfano (sin edges). |
| 3.5 | `ltp tree attach --tree tree-crt-logistica --node UDE-001 --role core_problem` | Nodo con role asignado. |
| 3.6 | `ltp tree detach --tree tree-crt-logistica --node UDE-001` | Nodo sale del tree pero sigue en pool global. |
| 3.7 | `ltp tree clone tree-crt-logistica --name "CRT What-If"` | Nuevo tree con edges independientes, nodos compartidos (mismo ref). |
| 3.8 | Modificar clone → `ltp tree diff tree-crt-logistica tree-crt-what-if` | Reporta nodos y edges añadidos/quitados/modificados. |
| 3.9 | `ltp tree walk tree-crt-logistica --order topological` | Recorrido bottom-up completo con contexto por nodo. |
| 3.10 | `ltp tree walk tree-crt-logistica --order reverse` | Recorrido top-down. |
| 3.11 | `ltp tree rm tree-crt-logistica` | Elimina tree, nodos quedan en pool. |

---

## Fase 4: Enlaces — Conexión Básica

**Scope**: `link connect/disconnect/feedback`, validación de integridad referencial, generación de IDs de links.

**Archivos**: `src/link/`

**Estado**: [ ] Pendiente

### UATs

| ID | Comando | Resultado esperado |
|----|---------|-------------------|
| 4.1 | `ltp link connect --tree T --from A --to B` | Crea edge SINGLE con ID `LINK-001`. |
| 4.2 | `ltp link connect --tree T --from A,B --to C --operator AND` | Crea edge AND con `from: ["A","B"]`. |
| 4.3 | `ltp link connect --tree T --from A --to B,C` | Crea 2 edges SINGLE (A→B, A→C). |
| 4.4 | `ltp link connect --tree T --from A --to B --operator MAG` | Warning: "weight omitido con MAG". |
| 4.5 | `ltp link connect --tree T --from A --to B --operator MAG --weight 0.6` | Crea edge MAG con weight. |
| 4.6 | `ltp link connect --tree T --from A --to B` (B no existe en pool) | Error `REFERENTIAL_INTEGRITY_VIOLATION`. |
| 4.7 | `ltp link connect --tree T --from A --to B` (B no attached al tree) | Error `NODE_NOT_IN_TREE`. |
| 4.8 | Crear ciclo A→B→C→A | Error `CIRCULAR_DEPENDENCY_DETECTED`. |
| 4.9 | `ltp link disconnect --tree T --links L1,L2` | Elimina ambos edges. |
| 4.10 | `ltp link feedback --tree T --from UDE-003 --to RC-001 --type positive --label "..."` | Crea entrada en `feedback_edges[]`. No dispara error de ciclo. |
| 4.11 | `ltp status` tras 4.10 | Reporta "1 feedback loop (positive)". |

---

## Fase 2b: Entidades — Operaciones con Dependencia Cross-Tree

**Scope**: `node rm/split/inspect` (requieren conocimiento de trees y edges).

**Archivos**: `src/node/`

**Estado**: [ ] Pendiente

**Prerequisito**: F3 + F4 completadas.

### UATs

| ID | Comando | Resultado esperado |
|----|---------|-------------------|
| 2b.1 | `ltp node rm UDE-001` (nodo attached a un tree con edges) | Elimina nodo del pool Y todos sus edges en todos los trees. |
| 2b.2 | `ltp node rm UDE-001` (no existe) | Error `NODE_NOT_FOUND`. |
| 2b.3 | `ltp node rm UDE-001,RC-001` (batch) | Elimina ambos con todos sus edges. |
| 2b.4 | `ltp node inspect UDE-001` | Muestra en qué trees participa, con qué rol, y sus conexiones (edges entrantes/salientes). |
| 2b.5 | `ltp node split INT-001 --into "Idea A" "Idea B" --tree tree-crt-logistica` | Crea 2 nodos, hereda edges entrantes al primero y salientes al segundo, elimina original. |
| 2b.6 | `ltp node list --tree tree-crt-logistica --type UDE` | Lista filtrada por tree y tipo. |
| 2b.7 | `ltp node list --tree tree-crt-logistica --status active` | Lista filtrada por status. |

---

## Fase 5: Validación Completa

**Scope**: `ltp validate` con todas las reglas (DAG, integridad, EC, linter CLR#2, warnings CLR#4/#6/#7).

**Archivos**: `src/validate/`

**Estado**: [ ] Pendiente

### UATs

| ID | Comando | Resultado esperado |
|----|---------|-------------------|
| 5.1 | `ltp validate` (workspace sano) | `errors: [], warnings: []`. |
| 5.2 | Tree con ciclo inyectado manualmente en JSON | Error: `CIRCULAR_DEPENDENCY_DETECTED`. |
| 5.3 | Edge referencia nodo inexistente en pool | Error: `REFERENTIAL_INTEGRITY_VIOLATION`. |
| 5.4 | EC sin nodo con role `objective` | Error: `EC_VALIDATION: missing objective`. |
| 5.5 | EC con 1 solo requirement | Error: `EC_VALIDATION: minimum 2 requirements`. |
| 5.6 | EC sin prerequisite para un requirement | Error: `EC_VALIDATION: requirement without prerequisite`. |
| 5.7 | Nodo con label "Vendemos poco porque..." | Warning CLR#2: conjunción causal. |
| 5.8 | Nodo con 1 sola entrada SINGLE | Warning CLR#4: candidato a insuficiencia. |
| 5.9 | AND con >4 entradas | Warning CLR#4/#5: posible mezcla de causas independientes. |
| 5.10 | Nodo `observable: false` con <2 salientes | Warning CLR#7: causa intangible sin efecto predicho. |
| 5.11 | UDE en posición `from` apuntando a RC | Warning CLR#6: inversión sospechosa. |
| 5.12 | Nodo attached sin edges | Warning: nodo huérfano. |
| 5.13 | `ltp validate --tree tree-crt-logistica` | Valida solo ese tree. |
| 5.14 | NBR con ciclo interno | Error: `CIRCULAR_DEPENDENCY_DETECTED` (dentro de NBR). |

---

## Fase 6: Enlaces — Manipulación Avanzada

**Scope**: `link reverse/move/insert-between/group/dissolve/split/reoperator/add-cause/rm-cause`

**Archivos**: `src/link/`

**Estado**: [ ] Pendiente

### UATs

| ID | Comando | Resultado esperado |
|----|---------|-------------------|
| 6.1 | `ltp link reverse --tree T --link L1` (sin assumptions) | Invierte `from↔to`. |
| 6.2 | `ltp link reverse --tree T --link L1` (con assumptions, sin --force) | Error: requiere `--force`. |
| 6.3 | `ltp link reverse --tree T --link L1 --force` | Invierte + marca assumptions `needs_review`. |
| 6.4 | `ltp link move --tree T --link L1 --new-to D` | Redirige destino. |
| 6.5 | `ltp link insert-between --tree T --link L1 --node X` (SINGLE) | A→B se convierte en A→X→B (2 edges nuevos, original eliminado). |
| 6.6 | `ltp link insert-between` con AND + `--insert-after-cause A` | Extrae A, crea A→X, X entra en grupo en su lugar. |
| 6.7 | `ltp link insert-between` con `--insert-before-effect` | [A,B]→C se convierte en [A,B]→X + X→C. |
| 6.8 | `ltp link group --tree T --links L1,L2 --operator AND` | Agrupa en 1 edge con `from[]` múltiple. |
| 6.9 | `ltp link group` con edges que NO comparten mismo `to` | Error. |
| 6.10 | `ltp link dissolve --tree T --link L1` | Cada causa vuelve a SINGLE. Assumptions heredadas con `needs_review`. |
| 6.11 | `ltp link split --tree T --link L1 --extract A` | Extrae A del grupo, grupo reducido. |
| 6.12 | Grupo queda con 1 causa tras split | Se convierte automáticamente en SINGLE. |
| 6.13 | `ltp link reoperator --tree T --link L1 --operator MAG` | Cambia operador. Warning si no hay weights. |
| 6.14 | `ltp link reoperator` MAG→AND | Descarta weights silenciosamente. |
| 6.15 | `ltp link add-cause --tree T --link L1 --node X` (edge AND) | Expande `from[]`. |
| 6.16 | `ltp link add-cause` en edge SINGLE sin `--promote-to` | Error: requiere `--promote-to`. |
| 6.17 | `ltp link rm-cause --tree T --link L1 --node X` (queda 1) | Reduce y convierte a SINGLE. |

---

## Fase 7: Supuestos (Assumptions)

**Scope**: `assume add/edit/rm/list/move`, `invalidate`

**Archivos**: `src/assume/`

**Estado**: [ ] Pendiente

### UATs

| ID | Comando | Resultado esperado |
|----|---------|-------------------|
| 7.1 | `ltp assume add --tree T --link L1 --text "Capacidad no aumenta"` | Crea ASM-001 dentro del edge. |
| 7.2 | `ltp assume edit --tree T --asm ASM-001 --text "Nuevo texto"` | Actualiza texto. |
| 7.3 | `ltp assume list --tree T` | Lista todos con status. |
| 7.4 | `ltp assume list --tree T --status valid` | Filtra por status. |
| 7.5 | `ltp assume move --tree T --asm ASM-001 --to-link L2` | Mueve a otro edge. |
| 7.6 | `ltp assume rm --tree T --asm ASM-001` | Elimina assumption. |
| 7.7 | `ltp invalidate --tree T --link L1 --asm ASM-001` | Marca ASM como `invalid`, link como `broken`. |
| 7.8 | `ltp invalidate ... --injection "Aumentar flota"` | Además crea nodo INJ borrador en pool. |

---

## Fase 8: Navegación (Trace & Inspección)

**Scope**: `trace`, `link inspect/find`.

**Archivos**: `src/trace/`

**Estado**: [ ] Pendiente

### UATs

| ID | Comando | Resultado esperado |
|----|---------|-------------------|
| 8.1 | `ltp trace RC-001 --tree T --direction downstream` | Cadena completa hasta UDEs/DEs. |
| 8.2 | `ltp trace UDE-005 --tree T --direction upstream` | Llega a causas raíz. |
| 8.3 | `ltp trace UDE-005 --tree T --direction upstream --depth 2` | Solo 2 niveles. |
| 8.4 | `ltp trace` con feedback loops presentes | Los incluye por defecto en sección separada. |
| 8.5 | `ltp trace ... --no-feedback` | Excluye feedback edges. |
| 8.6 | `ltp trace ... --nbr` | Incluye edges de NBR branches. |
| 8.7 | `ltp link inspect L1 --tree T` | Detalle completo: from (con labels), to, operator, weight, status, assumptions. |
| 8.8 | `ltp link find --tree T --from A --to B` | Encuentra edge(s) entre dos nodos. |

---

## Fase 9: Abstracción (Path)

**Scope**: `path collapse/explode/replace`

**Archivos**: `src/path/`

**Estado**: [ ] Pendiente

### UATs

| ID | Comando | Resultado esperado |
|----|---------|-------------------|
| 9.1 | `ltp path collapse --tree T --from A --to E --label "Cadena logística"` | Crea `macro_edge` con `interior_nodes` e `interior_links`. Nodos tácticos intactos en disco. |
| 9.2 | `ltp tree walk T` tras collapse | Macro edge visible; nodos interiores siguen en walk táctico. |
| 9.3 | `ltp path explode --tree T --link L1 --asm ASM-001 --label "Nodo intermedio"` | Crea nodo INT, parte el edge en 2, elimina el assumption del edge original. |
| 9.4 | `ltp path replace --tree T --macro-link MACRO-001 --by-node INJ-001` | Sub-grafo táctico marcado `superseded`. INJ conectada en su lugar. |

---

## Fase 10: NBR (Negative Branch Reservations)

**Scope**: `nbr add/list/inspect`, edges dentro de NBR.

**Archivos**: `src/nbr/`

**Estado**: [ ] Pendiente

### UATs

| ID | Comando | Resultado esperado |
|----|---------|-------------------|
| 10.1 | `ltp nbr add --tree T --source-node INJ-001` | Crea NBR vacía en `nbr_branches[]`. |
| 10.2 | `ltp link connect --tree T --nbr NBR-001 --from INJ-001 --to NDE-001` | Crea edge DENTRO de la NBR (no en `edges` del trunk). |
| 10.3 | `ltp nbr list --tree T` | Muestra NBRs con source, edge count, trim status. |
| 10.4 | `ltp nbr inspect NBR-001 --tree T` | Cadena causal completa de la NBR. |
| 10.5 | `ltp nbr add --tree T --source-node INJ-001 --trim INJ-003` | NBR con trim_injection asignada. |
| 10.6 | `ltp validate` sobre tree con NBR | Valida DAG de cada NBR como sub-grafo independiente. |

---

## Fase 11: Historial (Undo/Redo)

**Scope**: Activar el `SnapshotHook`, `undo`, `redo`, `history`, batch, checksums SHA-256.

**Archivos**: `src/history/`

**Estado**: [ ] Pendiente

**Nota**: El pipeline de escritura (F1) ya pasa por `SnapshotHook`. Esta fase lo activa: implementa la captura de `before` + cálculo de `after_hash` + escritura en `.ltp/undo/`.

### UATs

| ID | Comando | Resultado esperado |
|----|---------|-------------------|
| 11.1 | `ltp node add ...` → `ltp undo` | Nodo desaparece del pool. Entry movida a redo stack. |
| 11.2 | `ltp redo` | Nodo reaparece. |
| 11.3 | `ltp undo --dry-run` | Muestra qué se deshará sin ejecutar. |
| 11.4 | Editar fichero manualmente → `ltp undo` | Error `UNDO_STATE_DIVERGED` con detalle del fichero divergente. |
| 11.5 | `ltp history --last 5` | Muestra últimas 5 operaciones: seq, action, command, timestamp. |
| 11.6 | `ltp history check` | Valida integridad de cada entry contra estado actual en disco. |
| 11.7 | `ltp history begin-batch --label "Construcción CRT"` → N ops → `ltp history end-batch` | Un solo `ltp undo` deshace todo el batch. |
| 11.8 | `ltp undo` de `invalidate` | Resucita assumption a `valid`, link a `active`, elimina INJ borrador si fue creada. |
| 11.9 | Operación mutante nueva tras undo | Redo stack se vacía completamente. |
| 11.10 | Stack supera `max_size_mb` (5MB) | Rotación FIFO: entradas más antiguas descartadas. |
| 11.11 | `ltp history invalidate --from 3` | Descarta entries desde seq 3 en adelante. |
| 11.12 | `ltp history clear` | Limpia undo + redo stacks. |

---

## Tests End-to-End (Workflows Completos)

**Scope**: Validar la cohesión del sistema con flujos reales del LTP.

**Archivos**: `tests/e2e/`

**Estado**: [ ] Pendiente

**Prerequisito**: F11 completada.

### UATs

| ID | Workflow | Resultado esperado |
|----|---------|-------------------|
| E2E.1 | **CRT completo**: init → add 5 UDEs + 3 RCs + 2 INTs → new CRT → attach todos → connect cadena → validate → status | Workspace sano, 0 errores, graph_health ok. |
| E2E.2 | **Insuficiencia → corrección**: crear edge SINGLE insuficiente → validate (warning CLR#4) → add-cause → validate (warning desaparece) | Ciclo de corrección funciona. |
| E2E.3 | **Invalidación completa**: assume add → invalidate con injection → verify nodo INJ creado + link broken → undo → verify ASM resucita + INJ eliminada + link active | Undo de invalidate es correcto. |
| E2E.4 | **EC validation**: new EC → attach nodes con roles → validate reglas EC (objective, requirements, prerequisites, XOR) | Las 4 reglas EC se aplican correctamente. |
| E2E.5 | **CRT→EC→FRT cycle**: construir CRT → clonar nodo core problem a EC → construir EC → invalidar → crear FRT con inyección → add NBR → trim | Flujo multi-tree funciona con nodos compartidos. |
| E2E.6 | **Batch undo**: begin-batch → 10 operaciones de construcción → end-batch → undo → verificar que TODOS los cambios se revierten | Batch es atómico. |
| E2E.7 | **Clone + diff**: clone tree → modificar clone (add edge, rm edge, reoperator) → diff → verify report correcto | Diff detecta todas las diferencias. |
| E2E.8 | **Trace depth**: construir cadena de 8 niveles → trace upstream depth 3 → verificar que solo muestra 3 niveles → trace sin depth → muestra todos | Depth limita correctamente. |

---

## Fase 12: MCP Server

**Scope**: Protocolo JSON-RPC sobre stdin/stdout, exposición de todas las primitivas como tools.

**Archivos**: `src/mcp/` (nuevo módulo)

**Estado**: [ ] Pendiente

### UATs

| ID | Comando | Resultado esperado |
|----|---------|-------------------|
| 12.1 | Enviar `initialize` request | Responde con capabilities y lista completa de tools. |
| 12.2 | Invocar `ltp/node_add` vía JSON-RPC | Mismo resultado que CLI, devuelto como tool result. |
| 12.3 | Request con params inválidos | Error JSON-RPC con code y message estructurado. |
| 12.4 | Invocar tool de navegación (inspect, trace) | Retorna data sin mutar estado, sin generar undo entry. |
| 12.5 | Invocar tool de manipulación | Muta estado + genera undo entry. |
| 12.6 | Invocar `ltp/validate` | Retorna errors + warnings en formato MCP. |
| 12.7 | `--dry-run` via MCP | Tool de manipulación con dry_run retorna preview sin mutar. |

---

## Estimación de Complejidad

| Fase | Archivos principales | Complejidad |
|------|---------------------|-------------|
| F1 | workspace/, storage.rs, engine.rs | Media |
| F2a | node/ (standalone) | Baja |
| F3 | tree/ | Media |
| F4 | link/ (connect/disconnect) | Media |
| F2b | node/ (cross-tree) | Media |
| F5 | validate/ | Media |
| F6 | link/ (avanzado) | Alta |
| F7 | assume/ | Media |
| F8 | trace/ | Media |
| F9 | path/ | Alta |
| F10 | nbr/ | Media |
| F11 | history/ | Media (hook ya diseñado) |
| E2E | tests/e2e/ | Media |
| F12 | mcp/ | Media-Alta |

---

## Decisiones Técnicas Clave

1. **Storage como trait** — permite inyectar implementación in-memory para unit tests y filesystem para producción.
2. **SnapshotHook no-op desde F1** — toda mutación ya pasa por el hook. F11 solo activa la implementación real.
3. **IDs en `.ltp/counters.json`** — protegidos por lock, secuenciales, deterministas.
4. **Atomicidad multi-file** — write-then-rename por fichero. Si crash entre ficheros, `ltp validate` + `ltp history check` detectan inconsistencia.
5. **`--dry-run` global** — todo comando mutante acepta `--dry-run` que ejecuta la lógica completa (incluyendo validación DAG) pero no escribe a disco. Retorna el output JSON con los cambios que SE HARÍAN.
6. **Node IDs = solo tipo** — formato `<TYPE>-<SEQ>` (ej. `UDE-001`, `RC-003`). Sin prefijo de tree. El nodo es global, su ID no depende de ninguna vista.
7. **Errors como objetos tipados** — cada entry en `errors[]` es `{"code": "...", "detail": "...", ...campos contextuales}`. Permite branching programático por el agente LLM.
8. **Tree IDs autogenerados** — formato `tree-<type>-<slug(name)>`. Si colisión, sufijo numérico (`-2`, `-3`).
9. **JSON como output por defecto** — el consumidor principal es un agente LLM. Flag `--human` para formato legible por humanos.
10. **UATs vía CLI end-to-end** — los tests invocan el binario real como proceso hijo en un tempdir. Verifican stdout JSON + estado en disco. Unit tests de lib por separado para desarrollo rápido.
11. **Auto-reconstrucción de counters** — si `counters.json` falta o está corrupto, el motor escanea `nodes/` y `trees/` para recalcular el máximo de cada tipo. Emite warning `COUNTERS_REBUILT`.
