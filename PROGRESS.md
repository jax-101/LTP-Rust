# Progreso del Proyecto — `ltp-engine`

## Dashboard

| Métrica | Valor |
|---------|-------|
| **Avance global** | 93% |
| **Fase actual** | F12 — MCP Server |
| **Última fase completada** | E2E Tests |
| **Factor de escala (velocity)** | 1.0x |
| **Paquetes replanificados** | 3 (UATs expandidas) |
| **UATs totales** | 185 (era 179, +6 E2E Six Hats) |

---

## Estimación por Paquetes de Trabajo

Cada paquete tiene un peso relativo (% del total = 100%). Tras completar un paquete, se registra el esfuerzo real y se calcula el factor de escala para reestimar los restantes.

| Fase | Paquete | Peso Est. | Peso Ajust. | Estado | UATs | Notas |
|------|---------|-----------|-------------|--------|------|-------|
| F1 | Fundación (workspace, traits, IDs, pipeline) | 10% | 10% | ✅ Completado | 6/6 | |
| F2a | Nodos standalone (add/edit/list/search) | 4% | 4% | ✅ Completado | 9/9 | |
| F3 | Vistas (trees) | 8% | 8% | ✅ Completado | 11/11 | |
| F4 | Enlaces básicos (connect/disconnect/feedback) | 9% | 9% | ✅ Completado | 11/11 | |
| F2b | Nodos cross-tree (rm/split/inspect) | 5% | 5% | ✅ Completado | 7/7 | |
| F5 | Validación completa | 8% | 8% | ✅ Completado | 14/14 | |
| F6 | Enlaces avanzados | 14% | 14% | ✅ Completado | 17/17 | |
| F7 | Supuestos (assumptions) | 6% | 6% | ✅ Completado | 15/15 | +7 UATs error paths + idempotencia |
| F8 | Navegación (trace) | 6% | 6% | ✅ Completado | 15/15 | +7 UATs broken links + errors |
| F9 | Abstracción (path) | 8% | 8% | ✅ Completado | 12/12 | +8 UATs sub-grafo + errors |
| F10 | NBR | 5% | 5% | ✅ Completado | 17/17 | +5 UATs Six Hats (ciclo, inspect err, trim err, multi-source, trace) |
| F11 | Historial (undo/redo) | 6% | 6% | ✅ Completado | 22/22 | +4 UATs redo-dry-run + divergence + creation + rotation |
| E2E | Tests end-to-end | 4% | 4% | ✅ Completado | 19/19 | +6 UATs Six Hats (agent sim, undo cross-tree, EC incremental, path replace+undo, history divergence, multi-warning fix) |
| F12 | MCP Server | 7% | 7% | ⬜ Pendiente | 0/10 | +3 UATs error workspace + tools list |
| | **TOTAL** | **100%** | **100%** | | **156/179** | |

---

## Historial de Avance

### [F1] — Fundación (workspace, traits, IDs, pipeline)
**Fecha**: 2026-08-12
**Avance fase**: 6/6 UATs ✅
**Avance global**: 0% → 10%
**Esfuerzo estimado**: 10% | **Esfuerzo real (percibido)**: 10%
**Factor de escala acumulado**: 1.0x

#### Entregables
- Trait `Storage` + impl `FsStorage` con escritura atómica (tmp → rename)
- Trait `SnapshotHook` + `NoOpHook`
- Módulo `Counters` con auto-reconstrucción desde filesystem
- `ltp init` funcional con output JSON canónico
- `ltp status` con conteo de nodos/trees y validación DAG
- Lock file con detección de stale PID
- 6 tests de integración CLI (UATs 1.1–1.6)

#### Siguiente
- F2a: Nodos standalone (add/edit/list/search)

---

### [F2a] — Nodos standalone (add/edit/list/search)
**Fecha**: 2026-08-12
**Avance fase**: 9/9 UATs ✅
**Avance global**: 10% → 14%
**Esfuerzo estimado**: 4% | **Esfuerzo real (percibido)**: 4%
**Factor de escala acumulado**: 1.0x

#### Entregables
- `ltp node add` con generación de ID secuencial y linter CLR#2
- `ltp node edit` (label, add-tag, rm-tag, observable)
- `ltp node list` sobre pool global con filtros --type/--status
- `ltp node search` por substring case-insensitive
- Linter CLR#2: detección de conjunciones causales (warning no-bloqueante)
- 9 tests de integración CLI (UATs 2a.1–2a.9)

#### Siguiente
- F3: Vistas (trees)

---

### [F3] — Vistas (trees)
**Fecha**: 2026-08-12
**Avance fase**: 11/11 UATs ✅
**Avance global**: 14% → 22%
**Esfuerzo estimado**: 8% | **Esfuerzo real (percibido)**: 8%
**Factor de escala acumulado**: 1.0x

#### Entregables
- `ltp tree new` con ID slug-based y lógica por tipo (sufficiency/necessity)
- `ltp tree list/rm/attach/detach`
- `ltp tree clone` con edges independientes y nodos compartidos
- `ltp tree diff` entre dos trees (nodes/edges added/removed)
- `ltp tree walk` con Kahn's algorithm (topological/reverse)
- 11 tests de integración CLI (UATs 3.1–3.11)

#### Siguiente
- F4: Enlaces básicos (connect/disconnect/feedback)

---

### [F4] — Enlaces básicos (connect/disconnect/feedback)
**Fecha**: 2026-08-12
**Avance fase**: 11/11 UATs ✅
**Avance global**: 22% → 31%
**Esfuerzo estimado**: 9% | **Esfuerzo real (percibido)**: 9%
**Factor de escala acumulado**: 1.0x

#### Entregables
- `ltp link connect` con operadores SINGLE/AND/OR/MAG/XOR y weight opcional
- Multi-destination: `--to A,B` genera un edge SINGLE por destino
- Multi-source: `--from A,B` con operador inferido (AND) o explícito
- Validación de integridad referencial (nodo existe en pool + attached al tree)
- Validación DAG (ciclo detectado = error bloqueante, no se persiste)
- Warning `MAG_WEIGHT_MISSING` cuando operador MAG sin weight
- `ltp link disconnect` elimina edges por ID
- `ltp link feedback` crea feedback loops (positive/negative) en `feedback_edges[]`
- Feedback edges excluidos de validación DAG
- `ltp status` reporta `feedback_edge_count` por tree
- 11 tests de integración CLI (UATs 4.1–4.11)

#### Siguiente
- F2b: Nodos cross-tree (rm/split/inspect)

---

### [F2b] — Nodos cross-tree (rm/split/inspect)
**Fecha**: 2026-08-13
**Avance fase**: 7/7 UATs ✅
**Avance global**: 31% → 36%
**Esfuerzo estimado**: 5% | **Esfuerzo real (percibido)**: 5%
**Factor de escala acumulado**: 1.0x

#### Entregables
- `ltp node rm` con limpieza cross-tree (edges, feedback_edges, node refs)
- `ltp node rm` batch (múltiples IDs separados por coma)
- `ltp node inspect` muestra participación en trees, roles, edges inbound/outbound
- `ltp node split` divide nodo en dos, hereda edges entrantes→primero, salientes→segundo
- `ltp node list --tree` filtra por membership en tree (antes se ignoraba el parámetro)
- `NodeType::prefix()` helper para generación de IDs desde el enum
- 7 tests de integración CLI (UATs 2b.1–2b.7)

#### Siguiente
- F5: Validación completa

---

### [F5] — Validación completa
**Fecha**: 2026-08-13
**Avance fase**: 14/14 UATs ✅
**Avance global**: 36% → 44%
**Esfuerzo estimado**: 8% | **Esfuerzo real (percibido)**: 8%
**Factor de escala acumulado**: 1.0x

#### Entregables
- `ltp validate [--tree]` con orquestación completa de reglas
- Infraestructura de tracing (`tracing` + `tracing-subscriber`, activable con `LTP_LOG`)
- DAG check sobre edges del tree y cada `nbr_branches[].edges`
- Integridad referencial: nodos en edges deben existir en pool global
- EC validation: objective (=1), requirements (>=2), prerequisite por requirement
- CLR#2 lint: conjunciones causales en labels de nodos del tree
- CLR#4: nodo con 1 sola entrada SINGLE (candidato a insuficiencia)
- CLR#4/#5: AND con >4 entradas (mezcla de causas independientes)
- CLR#6: inversión de tipos (UDE/DE → RC/INT)
- CLR#7: nodo intangible con <2 salientes (falta efecto predicho)
- Huérfanos: nodos attached sin edges en el tree
- `NodeType` ahora deriva `Hash`
- 14 tests de integración CLI (UATs 5.1–5.14)

#### Siguiente
- F6: Enlaces avanzados / F7: Supuestos / F8: Navegación (parallelizables)

---

### [F6] — Enlaces avanzados
**Fecha**: 2026-08-13
**Avance fase**: 17/17 UATs ✅
**Avance global**: 44% → 58%
**Esfuerzo estimado**: 14% | **Esfuerzo real (percibido)**: 14%
**Factor de escala acumulado**: 1.0x

#### Entregables
- `ltp link reverse` con gate de --force para assumptions, marca needs_review
- `ltp link move` redirige from/to con validación de integridad
- `ltp link insert-between` con 3 variantes (SINGLE, AND+after-cause, AND+before-effect)
- `ltp link group` agrupa edges SINGLE bajo operador AND/OR/MAG/XOR
- `ltp link dissolve` deshace grupo, hereda assumptions con needs_review
- `ltp link split` extrae causas de un grupo, auto-downgrade a SINGLE
- `ltp link reoperator` cambia operador con reglas de cardinalidad y weight
- `ltp link add-cause` expande from[] con promote-to para SINGLE
- `ltp link rm-cause` reduce from[], auto-downgrade a SINGLE
- Fix: serde rename `NeedsReview` → `needs_review` (snake_case)
- 17 tests de integración CLI (UATs 6.1–6.17)

#### Siguiente
- F7: Supuestos / F8: Navegación (parallelizables)

---

### [F7] — Supuestos (assumptions)
**Fecha**: 2026-08-13
**Avance fase**: 15/15 UATs ✅
**Avance global**: 58% → 64%
**Esfuerzo estimado**: 6% | **Esfuerzo real (percibido)**: 6%
**Factor de escala acumulado**: 1.0x

#### Entregables
- `ltp assume add` crea assumption con ID secuencial (ASM-XXX) en un edge
- `ltp assume edit` actualiza texto de assumption (scan lineal por ID)
- `ltp assume list [--status]` lista assumptions con filtro opcional por status
- `ltp assume move` mueve assumption entre edges (rollback si target no existe)
- `ltp assume rm` elimina assumption de su edge
- `ltp invalidate` marca ASM invalid + edge broken + crea INJ opcional
- Idempotencia (ADR-010): re-invalidate retorna success + changed:false + warning
- Auto-reparación de estados inconsistentes (ASM invalid/edge active o viceversa)
- Error codes: ASSUMPTION_NOT_FOUND, ASSUMPTION_NOT_IN_LINK, LINK_NOT_FOUND, TREE_NOT_FOUND
- 15 tests de integración CLI (UATs 7.1–7.15)

#### Siguiente
- F8: Navegación (trace)

---

### [F8] — Navegación (trace)
**Fecha**: 2026-08-13
**Avance fase**: 15/15 UATs ✅
**Avance global**: 64% → 70%
**Esfuerzo estimado**: 6% | **Esfuerzo real (percibido)**: 6%
**Factor de escala acumulado**: 1.0x

#### Entregables
- `ltp trace` BFS upstream/downstream con chain_health (broken_links, superseded_links)
- Soporte feedback_edges incluidos por defecto, excluidos con `--no-feedback`
- Soporte NBR edges con flag `--nbr` (incluye nbr_branches[].edges en traversal)
- `--depth N` limita profundidad de BFS
- ADR-010: trace no se detiene ante broken/superseded/needs_review links
- `ltp link inspect` detalle completo: from con labels, to, operator, weight, status, logic, assumptions
- `ltp link find --from A --to B` encuentra edges entre dos nodos (array vacío si no hay)
- Error codes: TREE_NOT_FOUND, NODE_NOT_FOUND, NODE_NOT_IN_TREE, LINK_NOT_FOUND
- 15 tests de integración CLI (UATs 8.1–8.15)

#### Siguiente
- F9: Abstracción (path collapse/explode/replace)

---

### [F9] — Abstracción (path)
**Fecha**: 2026-08-13
**Avance fase**: 12/12 UATs ✅
**Avance global**: 70% → 78%
**Esfuerzo estimado**: 8% | **Esfuerzo real (percibido)**: 8%
**Factor de escala acumulado**: 1.0x

#### Entregables
- `ltp path collapse` colapsa sub-grafo completo (from→to) en macro_edge (ADR-010 Decisión 1)
- BFS bidireccional (forward+backward) para calcular interior_nodes/interior_links del DAG
- Soporte para diamonds: A→B→D→E, A→C→D→E → interior_nodes: [B,C,D]
- Caso degenerado: single edge directo → interior_nodes: [], interior_links: [link]
- Validación: `NESTED_MACRO_NOT_ALLOWED` si sub-grafo ya contiene macro_edge
- `ltp path explode` convierte assumption en nodo INT intermedio, split edge en 2
- Edges nuevos heredan `logic` del original, status: active, operator: SINGLE
- `ltp path replace` marca sub-grafo táctico como superseded, conecta nodo inyección
- Error codes: NO_DIRECTED_PATH, NESTED_MACRO_NOT_ALLOWED, ASSUMPTION_NOT_IN_LINK, MACRO_EDGE_NOT_FOUND, NODE_NOT_FOUND
- 12 tests de integración CLI (UATs 9.1–9.12)

#### Siguiente
- F10: NBR (Negative Branch Reservations)

---

### [F10] — NBR (Negative Branch Reservations)
**Fecha**: 2026-08-13
**Avance fase**: 17/17 UATs ✅
**Avance global**: 78% → 83%
**Esfuerzo estimado**: 5% | **Esfuerzo real (percibido)**: 5%
**Factor de escala acumulado**: 1.0x

#### Entregables
- `ltp nbr add` crea NBR vacía con source_node + optional trim_injection
- Validación: source_node existe en pool, attached al tree, trim_injection existe si se proporciona
- `ltp nbr rm` elimina NBR branch (ADR-010 Decisión 2: nodos permanecen en pool)
- `ltp nbr list` muestra resumen: id, source_node, edge_count, has_trim
- `ltp nbr inspect` detalle completo: edges, nodes_involved, trim_injection
- `ltp link connect --nbr NBR-XXX` crea edges dentro de la NBR (no en trunk)
- Validación DAG independiente por NBR branch (ciclo en NBR = error bloqueante)
- Nodos en NBR edges solo requieren existencia en pool (no attached al tree)
- Soporte para múltiples NBRs con mismo source_node (una inyección genera N ramas)
- Recursión por referencia: NBR-002.source_node = NBR-001.trim_injection
- Error codes: NODE_NOT_FOUND, NODE_NOT_IN_TREE, NBR_NOT_FOUND, REFERENTIAL_INTEGRITY_VIOLATION, CIRCULAR_DEPENDENCY_DETECTED
- 17 tests de integración CLI (UATs 10.1–10.17, expandidos con Six Hats)

#### Siguiente
- F11: Historial (undo/redo)

---

### [F11] — Historial (Undo/Redo)
**Fecha**: 2026-08-13
**Avance fase**: 22/22 UATs ✅
**Avance global**: 83% → 89%
**Esfuerzo estimado**: 6% | **Esfuerzo real (percibido)**: 6%
**Factor de escala acumulado**: 1.0x

#### Entregables
- `HistoryManager` como componente central con snapshot-based undo/redo (ADR-009)
- SHA-256 checksums para detección de divergencias externas (edición manual, Git)
- `ltp undo [--dry-run]` con restauración atómica cross-file (write-then-rename)
- `ltp redo [--dry-run]` con verificación de before_hash
- `ltp history [--last N]` lista el stack con seq, timestamp, action, command
- `ltp history check` valida integridad de cada entry contra disco
- `ltp history invalidate --from <seq>` descarta entries desde punto de divergencia
- `ltp history clear` limpia ambos stacks
- `ltp history begin-batch --label` / `end-batch` colapsa N operaciones en una sola entry
- Rotación FIFO por `max_size_mb` (configurable en ltp.config.json)
- Captura integrada en todos los comandos mutantes (30+ commands) sin modificar el trait Storage
- Error codes: UNDO_STACK_EMPTY, REDO_STACK_EMPTY, UNDO_STATE_DIVERGED, REDO_STATE_DIVERGED, BATCH_ALREADY_IN_PROGRESS, NO_BATCH_IN_PROGRESS, HISTORY_DISABLED
- Paths relativos en entries (portabilidad entre máquinas)
- 22 tests de integración CLI (UATs 11.1–11.22, expandidos con Six Hats)

#### Siguiente
- E2E Tests: Workflows completos

---

### [E2E] — Tests End-to-End (Workflows Completos)
**Fecha**: 2026-08-13
**Avance fase**: 19/19 UATs ✅
**Avance global**: 89% → 93%
**Esfuerzo estimado**: 4% | **Esfuerzo real (percibido)**: 4%
**Factor de escala acumulado**: 1.0x

#### Entregables
- 19 tests E2E en `tests/e2e.rs` (13 originales + 6 Six Hats)
- E2E.1: CRT completo (10 nodos, cadena causal, validate + status)
- E2E.2: Insuficiencia → corrección (CLR#4 cycle)
- E2E.3: Invalidación completa (assume → invalidate → undo roundtrip)
- E2E.4: EC validation (roles, requirements, prerequisites, XOR)
- E2E.5: CRT→EC→FRT cycle (multi-tree, nodos compartidos, NBR con trim)
- E2E.6: Batch undo (10 ops atómicas)
- E2E.7: Clone + diff (edges independientes, link find en clone)
- E2E.8: Trace depth (8 niveles, depth-limited vs full)
- E2E.9: Nodo compartido multi-tree (edit visible en ambos trees)
- E2E.10: Counters recovery (auto-rebuild tras borrado)
- E2E.11: Invalidate + trace lifecycle (broken links en cadena, undo restaura)
- E2E.12: Path collapse + validate (macro_edges sin falsos positivos)
- E2E.13: NBR + invalidate + undo (aislamiento trunk/NBR)
- E2E.14: Agent simulation (navegación intercalada con mutaciones + undo/redo)
- E2E.15: Undo cross-tree cascade (node rm multi-tree → undo restaura ambos)
- E2E.16: EC incremental construction (error → fix iterativo)
- E2E.17: Path replace + undo roundtrip (superseded → active)
- E2E.18: History divergence recovery (edición externa → check → invalidate)
- E2E.19: Multi-warning iterative fix (CLR#4, CLR#6, CLR#7 → fix secuencial)

#### Siguiente
- F12: MCP Server

---

### Plantilla de reporte (se copia tras cada paquete completado)

```
### [Fase X] — [Nombre del paquete]
**Fecha**: YYYY-MM-DD
**Avance fase**: X/Y UATs ✅
**Avance global**: NN% → MM%
**Esfuerzo estimado**: N% | **Esfuerzo real (percibido)**: M%
**Factor de escala acumulado**: X.Xx

#### Descubrimientos
- (nuevos paquetes, complejidad no prevista, simplificaciones)

#### En curso
- (qué se está haciendo ahora)

#### Siguiente
- (próximo paquete según el plan)
```

---

## Reglas de Reestimación

1. **Tras completar un paquete**: si el esfuerzo real difiere significativamente del estimado (>30%), se recalcula el factor de escala:
   ```
   velocity = peso_estimado / peso_real
   ```
   Los paquetes restantes se escalan por `1/velocity` para ajustar la predicción.

2. **Descubrimiento de nuevo trabajo**: se añade como nueva fila con peso estimado. Se rebalancea el % total a 100% redistribuyendo proporcionalmente entre todos los paquetes no completados. Los completados mantienen su % real.

3. **Eliminación de trabajo**: si un paquete se simplifica o se descarta, su peso se redistribuye entre los restantes.

4. **El avance global** se calcula como:
   ```
   avance = Σ (peso_ajustado × progreso_fase)
   donde progreso_fase = UATs_pasando / UATs_total de esa fase
   ```

---

## Histórico de Reestimaciones

| Fecha | Motivo | Cambio | Impacto en total |
|-------|--------|--------|-----------------|
| — | Plan inicial | 14 paquetes, 128 UATs | 100% baseline |
| 2026-08-13 | Expansión de UATs (ADR-010) | +42 UATs en F7–F12+E2E (error paths, edge cases, nbr rm, trace broken, invalidate idempotente) | 128 → 170 UATs. Avance global sigue 58% (pesos por fase sin cambio; fases completadas mantienen 100% de su peso). |

