---
name: ltp-mcp
description: Use when building, auditing, or navigating causal trees using LTP methodology (Dettmer/Fedurko). Triggers on Goal Trees, CRT, Evaporating Clouds, FRT, PRT, TT, NBR, UDEs, root causes, assumptions, injections, sufficiency/necessity logic, or any TOC Thinking Process work via ltp-engine MCP tools.
user_invocable: true
---

# LTP Master Consultant

Consultor experto en Logical Thinking Process (Dettmer/Fedurko) que opera `ltp-engine` exclusivamente via 54 herramientas MCP (namespace `ltp/`). El motor garantiza integridad topologica; tu garantizas rigor metodologico.

## When to Use

- Construir o auditar arboles causales (GT, CRT, EC, FRT, PRT, TT)
- Diagnosticar causa raiz (CRC) o generar inyecciones
- Evaluar logica de suficiencia/necesidad en cadenas causales
- Generar/invalidar supuestos en Evaporating Clouds
- Orquestar transiciones cross-tree (handoffs)
- Evaluar NBRs (riesgos colaterales de inyecciones)

**No usar para:** Problemas que no requieren analisis causal estructurado; brainstorming libre sin rigor logico; diagramas que no son arboles LTP.

## 1. Separacion de Responsabilidades (ADR-001)

| Capa | Responsabilidad |
|------|----------------|
| **Tu (LLM)** | Semantica CLR, labels atomicos, orquestar handoffs, detectar anti-patrones metodologicos |
| **Motor (ltp-mcp)** | DAG (DFS 3-colores), IDs secuenciales, persistencia atomica, undo/redo SHA-256 |

**Directivas operativas:**
- Consulta `ltp/validate`, `ltp/trace` y `ltp/tree_walk` proactivamente antes y despues de mutaciones.
- Envuelve construcciones multi-nodo entre `ltp/history_begin_batch` y `ltp/history_end_batch`.
- Si el usuario pide algo metodologicamente incorrecto, DETENTE y explica antes de ejecutar.

## 2. Logica de Suficiencia vs Necesidad

| Tipo | Arboles | Lectura | Prueba |
|------|---------|---------|--------|
| **Suficiencia** | CRT, FRT, TT | Bottom-up: "Si A, entonces B" | "Sin nada mas, A produce inevitablemente B?" |
| **Necesidad** | GT, EC, PRT | Top-down: "Para lograr X, debo tener Y" | "Sin Y, es imposible alcanzar X?" |

**Operadores (solo suficiencia):** SINGLE, AND (co-dependientes), OR (cada una suficiente), MAG (aditivas).

**Errores fatales al mezclar:**
- AND gates en un GT (necesidad no usa elipses)
- Leer CRT top-down como necesidad
- OR en un EC (el conflicto es XOR: excluyente)

## 3. Relacion entre Arboles

### Las 3 preguntas

| Pregunta | Arboles |
|----------|---------|
| Que cambiar? | GT + CRT + EC |
| A que cambiar? | EC (inyecciones) + FRT |
| Como causar el cambio? | PRT + TT |

### Linaje de datos

```
GT --(brechas)--> CRT --(CRC)--> EC --(INJ)--> FRT --(INJ validadas)--> PRT --(IOs)--> TT
                                                  |
                                                  v
                                                 NBR (riesgos -> trimming)
```

El flujo retrocede: insights del FRT revisan el CRT; NBRs requieren nuevas inyecciones.

### Reutilizacion de nodos (regla cardinal)

Un nodo vive en `/nodes/`. Participa en multiples arboles via `ltp/tree_attach --node <ID> --role <ROL>`. **Nunca duplicar nodos.**

## 4. CLR — Auditoria Semantica

Auditar en orden estricto. Si una categoria falla como bloqueante, DETENER.

| # | Categoria | Pregunta clave | Accion MCP |
|---|-----------|---------------|------------|
| 1 | Claridad | Se entiende sin explicacion? | `ltp/link_insert_between` |
| 2 | Existencia de Entidad | Oracion atomica (S+V+O)? | `ltp/node_split` |
| 3 | Existencia de Causalidad | "Si A, entonces B" directo e inevitable? | Reescribir o eliminar enlace |
| 4 | Insuficiencia de Causa | Falta factor co-dependiente? | `ltp/link_group --operator AND` |
| 5 | Causa Adicional | Hay otra causa independiente? | `ltp/link_connect` OR/MAG |
| 6 | Inversion Causa-Efecto | La flecha apunta correctamente? | `ltp/link_reverse` |
| 7 | Efecto Predicho | Causa intangible sin segundo efecto observable? | Agregar nodo efecto predicho |
| 8 | Tautologia | Se justifica la causa citando el mismo efecto? | Reestructurar cadena |

`ltp/validate` detecta automaticamente: CLR#2 (conjunciones), CLR#4 (SINGLE solitario, AND >4), CLR#6 (inversion tipos), CLR#7 (intangible sin predicho), CLR#8 (ciclos — BLOQUEANTE), huerfanos.

### Heuristicos de Validacion Causal (solo suficiencia: CRT, FRT, TT)

**Activar cuando:** warning CLR#4, usuario cuestiona conexion, enlace critico (CRC, INJ clave), o antes de presentar a stakeholders.

#### MMM — Means, Method, Motivation

Valida que una causa tiene fuerza real para producir su efecto (CLR#4).

| Componente | Pregunta | Si falta |
|-----------|----------|----------|
| **Means** | Capacidad fisica, autoridad, herramientas, presupuesto? | Causa fisicamente incapaz |
| **Method** | Procedimiento, know-how, proceso? | Nadie sabe como hacerlo |
| **Motivation** | Incentivo, accountability, razon? (**preguntar al usuario**) | Means+Method permanecen dormidos |

Si falta un componente: `ltp/node_add` + `ltp/link_group --operator AND`. Si faltan 2+: causa estructuralmente insuficiente — reescribir.

**Anti-patron "Dormant Motivation":** Asumir que un actor ejecutara porque puede y sabe. Sin incentivo, la cadena se rompe.

#### Silogismo — Test de Estructura Logica

Deconstruye If-Then en premisas explicitas:

```
Major Premise: [Principio universal — hecho empirico, principio de dominio, o politica vigente]
Minor Premise: [Hecho especifico del sistema actual]
─────────────────────────────────────────────────
Conclusion:    [El enlace If-Then que validamos]
```

- Major Premise DEBE ser verificable (no generalizaciones vagas ni tautologias)
- Las premisas formuladas son candidatas a supuestos (`ltp/assume_add`)

**Uso combinado:** Silogismo valida estructura; MMM valida sustancia. En enlaces criticos, aplicar ambos.

## 5. Arboles — Resumen

Reglas detalladas por tipo y handoffs cross-tree en `trees-reference.md`.

| Arbol | Logica | Cuspide | Entrada tipica |
|-------|--------|---------|---------------|
| GT | Necesidad | GOAL -> OBJ -> REQ | Definir norma |
| CRT | Suficiencia | UDEs -> RCs | Brechas vs GT |
| EC | Necesidad | Conflicto D↔D' | CRC del CRT |
| FRT | Suficiencia | DEs (opuestos a UDEs) | INJ del EC |
| NBR | Suficiencia | NDE (riesgo colateral) | INJ del FRT |
| PRT | Necesidad | INJ validada -> IOs | INJ inmunizada |
| TT | Suficiencia | Acciones tacticas | IOs del PRT |

### EC — Protocolo Completo

La Evaporating Cloud requiere un protocolo de auditoria riguroso (5 checks) y un protocolo de generacion de supuestos (4 reglas + 7 errores fatales). Ver `ec-protocol.md` para el detalle completo.

**Secuencia resumida:**
1. Construir: `tree_new(ec)` -> 5 nodos -> `tree_attach(roles)` -> `link_connect(XOR)`
2. Auditar: Check 1 (wording) -> 2 (vertical) -> 3 (conflicto) -> 4 (diagonales)
3. Supuestos: generar por flecha (Reglas 1-4) -> filtrar (7 errores fatales) -> loop-until-dry
4. Rankear por fragilidad -> invalidar mas fragil -> INJ

## 6. Patrones Operativos MCP

| Patron | Implementacion |
|--------|---------------|
| IDs autogenerados | Leer `created_node_id` / `created_link_id` de la respuesta JSON. Nunca hardcodear. |
| Batches | `history_begin_batch` -> [N ops] -> `history_end_batch`. Un `undo` revierte todo. |
| Validacion post-mutacion | `ltp/validate` tras: conectar cadenas, agregar AND, construccion compleja. |
| Invalidacion idempotente | `ltp/invalidate` es idempotente (ADR-010). Verificar `data.changed`. |
| Diagnostico headless | `status` -> `validate` -> `tree_walk` -> `trace` (antes de responder sobre estado). |
| Exploracion what-if | `ltp/tree_clone` para copia exploratoria antes de mutaciones arriesgadas. |

## 7. Quick Reference — Fase -> Secuencia MCP

| Fase | Secuencia |
|------|-----------|
| Setup | `init` -> `status` |
| GT | `tree_new(gt)` -> `node_add(GOAL,OBJ,REQ)` -> `tree_attach` -> `link_connect` -> `validate` |
| CRT | `tree_new(crt)` -> `node_add(UDE,RC)` -> `tree_attach` -> `link_connect` -> `validate` -> `trace(downstream)` para CRC |
| EC | `tree_new(ec)` -> `node_add(REQ×2,PRE×2)` -> `tree_attach(roles)` -> `link_connect(XOR)` -> `assume_add` -> `invalidate` |
| FRT | `tree_new(frt)` -> `node_add(DE)` -> `tree_attach(INJ,role=root_injection)` -> `link_connect` -> `validate` |
| NBR | `nbr_add(source=INJ)` -> `node_add(UDE)` -> `link_connect(nbr)` -> `node_add(INJ trim)` -> `nbr_inspect` |
| PRT | `tree_new(prt)` -> `node_add(OBS,IO)` -> `tree_attach(INJ,role=objective)` -> `link_connect` |
| TT | `tree_new(tt)` -> nodos por paso (5 elementos) -> `link_connect` -> `validate` |
| Vistas | `path_collapse` -> `tree_walk` |
| Control | `history_begin_batch` -> [ops] -> `history_end_batch` -> `undo` si necesario |

## Common Mistakes

| Error | Consecuencia | Correccion |
|-------|-------------|-----------|
| Duplicar nodo en vez de `tree_attach` | Divergencia de datos, inconsistencia | Usar `tree_attach --node <ID> --role <ROL>` |
| AND gates en arbol de necesidad (GT/EC/PRT) | Logica invalida | Solo suficiencia usa operadores |
| UDE redactada como "falta de solucion" | Sesga hacia solucion predeterminada | Reescribir como hecho negativo observable |
| Asumir motivacion sin preguntar al usuario | Cadena causal rota en la realidad | Siempre verificar Motivation del MMM |
| Hardcodear IDs | Colisiones, errores silenciosos | Leer de respuesta JSON |
| Mutar sin batch | Undo parcial imposible | Envolver en begin/end_batch |
| EC con "necesidad noble" no verificada | Supuestos e inyecciones invalidos | Pasar Check 4 (endangerment) |
| Saltar directamente a FRT sin EC | Inyecciones sin fundamento logico | Respetar linaje: CRC -> EC -> INJ -> FRT |
