# Progreso del Proyecto — `ltp-engine`

## Dashboard

| Métrica | Valor |
|---------|-------|
| **Avance global** | 58% |
| **Fase actual** | F7 / F8 (paralelizables) |
| **Última fase completada** | F6 — Enlaces avanzados |
| **Factor de escala (velocity)** | 1.0x |
| **Paquetes replanificados** | 0 |

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
| F7 | Supuestos (assumptions) | 6% | 6% | ⬜ Pendiente | 0/8 | |
| F8 | Navegación (trace) | 6% | 6% | ⬜ Pendiente | 0/8 | |
| F9 | Abstracción (path) | 8% | 8% | ⬜ Pendiente | 0/4 | |
| F10 | NBR | 5% | 5% | ⬜ Pendiente | 0/6 | |
| F11 | Historial (undo/redo) | 6% | 6% | ⬜ Pendiente | 0/12 | |
| E2E | Tests end-to-end | 4% | 4% | ⬜ Pendiente | 0/8 | |
| F12 | MCP Server | 7% | 7% | ⬜ Pendiente | 0/7 | |
| | **TOTAL** | **100%** | **100%** | | **75/128** | |

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

