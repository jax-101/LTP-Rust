# Guia Completa: Analisis LTP con ltp-engine + Claude Code

Guia practica para ejecutar un ciclo completo de Logical Thinking Process usando `ltp-engine` como motor determinista y Claude Code como agente de razonamiento.

---

## 1. Instalacion y Configuracion (desde cero)

### 1.1. Prerrequisitos

- **Rust toolchain**: si no lo tienes, instala con `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **Claude Code CLI**: instalado y funcional

### 1.2. Compilar e instalar ltp-engine

```bash
cd /Users/jullivarri/Documents/claude/RUST/LTP_Rust
git pull                    # asegurar ultima version
cargo install --path .      # instala `ltp` y `ltp-mcp` en ~/.cargo/bin/
```

Verifica:
```bash
ltp --version
ltp-mcp --help
```

### 1.3. Registrar el servidor MCP en Claude Code (una sola vez)

```bash
claude mcp add --scope user ltp -- ltp-mcp
```

Esto registra `ltp-mcp` como servidor MCP global. Funcionara en cualquier carpeta donde abras Claude Code.

### 1.4. Actualizar en el futuro

```bash
cd /Users/jullivarri/Documents/claude/RUST/LTP_Rust
git pull
cargo install --path .
```

La proxima sesion de Claude Code usara el binario nuevo automaticamente.

---

## 2. Preparar un Proyecto de Analisis

### 2.1. Crear la carpeta del proyecto

```bash
mkdir ~/Documents/mi-analisis-ltp
cd ~/Documents/mi-analisis-ltp
```

### 2.2. Abrir Claude Code ahi

```bash
claude
```

### 2.3. Inicializar el workspace LTP

Desde Claude Code:
> "Inicializa un workspace LTP llamado 'Mi Problema'"

Claude ejecutara `ltp/init` con el nombre que le des. Se crearan:
- `nodes/` — pool global de nodos
- `trees/` — vistas topologicas
- `knowledge/` — pool de conocimiento
- `ltp.config.json` — configuracion
- `.ltp/` — estado interno (undo, redo, lock, counters)

### 2.4. (Opcional) Crear un CLAUDE.md para el proyecto

Puedes crear un `CLAUDE.md` en la raiz del proyecto con instrucciones especificas para Claude:

```markdown
# CLAUDE.md

## Proyecto
Analisis LTP de [tu problema]. Usar las tools MCP de ltp-engine.

## Contexto del dominio
[Describe brevemente el sistema bajo analisis para que Claude tenga contexto]

## Instrucciones
- Habla en espanol
- Al hacer preguntas sobre hipotesis, lista las que estan pendientes de validar
- Usa el Knowledge Pool para registrar toda evidencia
```

---

## 3. Flujo de Trabajo Completo: GT → CRT → EC → FRT → PRT

**La logica de cada arbol la fija el motor segun su tipo** (ADR-014); no hay que pedirla ni se puede cambiar:

| Arbol | Logica | Se lee como | `walk` por defecto |
|-------|--------|-------------|--------------------|
| GT, EC, PRT | Necesidad | "Para tener X, necesito Y" | `reverse` (desde el objetivo) |
| CRT, FRT, TT | Suficiencia | "Si A, entonces B" | `topological` (desde las causas) |

Cada edge hereda la logica de su arbol al crearse (`connect`, `insert_between`, `group`, `path_replace`, `macro_expand/promote`). Los edges de una rama NBR son siempre de suficiencia. Los GT creados con versiones anteriores (guardados como suficiencia) se leen ya corregidos y se reescriben bien en la siguiente modificacion; el `undo` sigue funcionando.

### 3.1. Goal Tree (GT) — Que queremos lograr

El GT define el estado deseado con logica de necesidad. Jerarquia de Dettmer: **`GOAL ← CSF ← NC`**.

**Pasos:**
1. Define el GOAL (objetivo superior) — tipo de nodo `GOAL`
2. Define los Critical Success Factors como condiciones necesarias del objetivo — tipo `CSF` (IDs `CSF-001`…)
3. Define las Necessary Conditions de cada CSF — tipo `NC`

**Comandos tipicos:**

```
"Crea un Goal Tree llamado 'GT Sistema Logistico'"
"Anade el objetivo: 'El sistema logistico entrega en menos de 10 dias al 95% de los pedidos'"
"Anade los CSF: 'Capacidad de transporte suficiente', 'Procesos de picking optimizados'"
"Anade las NC del primer CSF: 'Flota dimensionada para el pico estacional', 'Proveedores entregan materiales a tiempo'"
"Conecta las NC a su CSF y los CSF al objetivo"
"Haz un walk del GT"   → sale del GOAL hacia los CSF y las NC
```

> `validate` no avisa de CLR#4 (insuficiencia) en el GT: cada condicion necesaria es insuficiente por si sola, por construccion.

**Interaccion con Knowledge Pool:**
```
"Registra como medicion que el KPI actual de entrega es 18.3 dias (fuente: dashboard Salesforce Q2)"
"Vincula esa evidencia al goal"
```

### 3.2. Current Reality Tree (CRT) — Que esta mal hoy

El CRT modela la realidad actual usando logica de suficiencia (Si A, entonces B). El objetivo es encontrar las pocas causas raiz (RC) que generan multiples efectos indeseables (UDE).

**Pasos:**
1. Lista los UDEs (efectos indeseables observables)
2. Busca las causas intermedias (INT) y las causas raiz (RC)
3. Conecta con relaciones causa-efecto
4. Valida la logica (CLR)

**Comandos tipicos:**

```
"Crea un CRT llamado 'CRT Logistica'"
"Anade estos UDEs: [lista]"
"Anade estas causas raiz: [lista]"
"Conecta RC-001 y RC-002 como AND hacia INT-001"
"Valida el arbol"
```

**Tecnicas para construirlo:**
- Empieza por los UDEs (lo que duele) y pregunta "por que?"
- Busca causas comunes: si RC-001 aparece debajo de 3+ UDEs, es candidata a core problem
- Usa `ltp/validate` frecuentemente — te dira si hay insuficiencia, huerfanos, inversiones

**Preguntas clave a hacerle a Claude:**
```
"Que hipotesis estamos asumiendo en este CRT?"
"Que supuestos ocultos hay en la conexion entre INT-003 y UDE-002?"
"Hay evidencia que respalde que RC-001 existe realmente?"
"Que causas raiz tienen mayor impacto downstream?"
```

### 3.3. Evaporating Cloud (EC) — Resolver el conflicto

La EC expone el dilema que impide actuar. Usa logica de necesidad (Para X, necesito Y).

**Estructura:**
- 1 Objective (lo que ambos lados quieren)
- 2+ Requirements (necesidades contrapuestas)
- Prerequisites que entran en conflicto (XOR)

**Roles exactos al adjuntar** (`tree attach --role`, case-sensitive; `validate` falla con `EC_VALIDATION` si no):

| Rol | Cuántos | Conexión |
|-----|---------|----------|
| `objective` | exactamente 1 | recibe de los `requirement` |
| `requirement` | al menos 2 | `requirement → objective` |
| `prerequisite` | al menos 1 por requirement | `prerequisite → requirement` |

No uses `root`, `leaf` ni `intermediate` en un EC: no son roles válidos.

**Comandos tipicos:**

```
"Crea una Evaporating Cloud llamada 'EC Velocidad vs Costo'"
"El objetivo comun es: 'Sistema logistico rentable y rapido'"
"Requirement 1: 'Minimizar costo de transporte' → Prerequisito: 'Consolidar envios (batch)'"
"Requirement 2: 'Maximizar velocidad de entrega' → Prerequisito: 'Enviar inmediatamente (flow)'"
"Los prerequisitos estan en conflicto (XOR)"
```

**Trabajar supuestos:**
```
"Anade supuestos a cada flecha de la EC"
"Que supuesto podemos invalidar para romper el conflicto?"
"Invalida el supuesto ASM-003 y genera una inyeccion"
```

### 3.4. Future Reality Tree (FRT) — La solucion funciona?

El FRT verifica que las inyecciones (soluciones propuestas) eliminan los UDEs sin crear problemas nuevos.

**Pasos:**
1. Inserta las inyecciones en una copia del CRT
2. Verifica que los UDEs desaparecen
3. Busca Negative Branch Reservations (efectos negativos colaterales)
4. Agrega inyecciones de trimming para neutralizarlos

**Comandos tipicos:**

```
"Crea un FRT llamado 'FRT Solucion Hibrida'"
"Anade la inyeccion: 'Implementar modelo hibrido batch+express con umbral de 48h'"
"Conecta la inyeccion al grafo y muestra que UDE-001 ya no se produce"
"Hay alguna Negative Branch? Que efectos negativos podria causar esta inyeccion?"
"Anade una NBR desde la inyeccion"
"Anade una inyeccion de trimming para neutralizar el efecto negativo"
```

### 3.5. Prerequisite Tree (PRT) — Que obstaculos hay para implementar

El PRT identifica obstaculos para la implementacion y los ordena.

**Comandos tipicos:**

```
"Crea un PRT llamado 'PRT Implementacion Hibrida'"
"Cuales son los obstaculos para implementar INJ-001?"
"Ordena los obstaculos: cual necesita resolverse primero?"
```

### 3.6. Transition Tree (TT) — Plan de accion detallado

El TT es el plan paso a paso con logica de suficiencia.

```
"Crea un TT llamado 'TT Plan Piloto'"
"Define las acciones en secuencia con sus efectos esperados"
```

---

## 4. Knowledge Pool — Gestion de Evidencia e Hipotesis

### 4.1. Concepto

El Knowledge Pool es tu "base de evidencia". Aqui vive todo lo que sabes (o crees saber) sobre el sistema, separado del grafo causal pero vinculado a el.

**Tipos de knowledge:**
| Tipo | Cuando usar |
|------|-------------|
| `measurement` | Datos cuantitativos (KPIs, metricas, conteos) |
| `testimony` | Lo que alguien dijo (entrevista, mensaje, reunion) |
| `hypothesis` | Suposicion por validar |
| `document` | Referencia a doc formal (reporte, contrato, politica) |
| `observation` | Algo que observaste directamente |
| `derived` | Conclusion derivada de otros items |

**Status:**
| Status | Significado |
|--------|-------------|
| `unverified` | Capturado pero no confirmado (default) |
| `verified` | Confirmado con evidencia |
| `refuted` | Demostrado como falso |
| `superseded` | Reemplazado por info mas reciente |

### 4.2. Capturar conocimiento desde el inicio

Antes de construir el CRT, registra todo lo que sabes:

```
"Registra como medicion: 'Tiempo medio de entrega 18.3 dias en Q2' con fuente del dashboard de Salesforce. Confianza alta."
"Registra como testimonio: 'El director de logistica dice que el cuello de botella esta en el picking' (reunion del lunes). Confianza media."
"Registra como hipotesis: 'Los retrasos se concentran en la ultima milla'. Sin verificar."
"Registra como observacion: 'Los pedidos del viernes tardan 3 dias mas que los del lunes'. Confianza alta."
```

### 4.3. Vincular evidencia al grafo

Una vez tengas nodos y edges, vincula el conocimiento:

```
"Vincula la medicion de 18.3 dias (KN-001) como soporte de UDE-001"
"Vincula la hipotesis de ultima milla (KN-003) como soporte de RC-002"
"Vincula el testimonio del director como contexto de INT-005"
```

**Relaciones disponibles:**
- `supports` — esta evidencia respalda que el target es real/verdadero
- `contradicts` — esta evidencia debilita o contradice el target
- `contextualizes` — info relevante de fondo, no directamente probatoria

### 4.4. Gestion de hipotesis (el ciclo epistemico)

Este es el flujo mas poderoso para un analisis riguroso:

```
CAPTURAR → VINCULAR → CONSTRUIR → BUSCAR EVIDENCIA → PROMOVER o REFUTAR
```

**Preguntas clave para hacerle a Claude:**

```
"Que hipotesis tenemos actualmente sin verificar?"
"Que nodos del CRT estan marcados como hipotesis y no tienen evidencia que los respalde?"
"Hay contradicciones entre la evidencia y lo que declaramos como hechos?"
"Que hipotesis deberia validar primero para avanzar con mas confianza?"
"Muestra el status epistemico del analisis: cuantos hechos vs hipotesis tenemos?"
```

**Cuando encuentres evidencia nueva:**

```
"Encontre datos que confirman que RC-002 es real. Registra esta medicion: [...]. Promueve RC-002 a hecho."
"Los datos del ERP contradicen la hipotesis de que el picking es el cuello de botella. Registra como refutacion."
```

### 4.5. Inbox: conocimiento huerfano

Los knowledge items sin vincular funcionan como "inbox" de informacion pendiente de procesar:

```
"Muestra el knowledge sin vincular"
"Tengo estas notas de la entrevista de hoy: [...]. Registralas y luego veremos donde encajan"
```

---

## 5. Validacion y Auditoria Continua

### 5.1. Usar `validate` frecuentemente

```
"Valida el CRT"
"Valida todo el workspace"
```

**Errores bloqueantes** (hay que corregir):
- Ciclos en el grafo (tautologia)
- Integridad referencial rota. Desde v0.5.0 se revisa todo el arbol (nodos, edges, feedback, ramas NBR y flechas largas), asi que un workspace danado por versiones anteriores puede empezar a fallar aqui: estaba roto, ahora se ve. Cada error indica `location` y `field` para saber que reparar
- EC sin estructura valida

**Warnings** (oportunidades de mejora):
- CLR#2: nodo con conjuncion causal ("porque", "para") → dividir en dos
- CLR#4: causa unica (insuficiencia), OR implicito, AND con demasiadas entradas → buscar causas adicionales. Solo en arboles de suficiencia (CRT/FRT/TT); en GT/EC/PRT no aplica
- CLR#6: inversion causa-efecto sospechosa
- CLR#7: causa intangible sin efecto predicho
- Nodos huerfanos
- Knowledge dangling (apunta a target eliminado)
- Hechos sin soporte epistemico
- Hechos contradichos
- Hipotesis promocionables (2+ soportes verificados)

### 5.2. Trazar cadenas causales

```
"Traza upstream desde UDE-001 — quiero ver todas las causas raiz"
"Traza downstream desde RC-003 — que UDEs produce?"
"Traza upstream desde UDE-001 mostrando el knowledge asociado"
```

### 5.3. Comparar escenarios (what-if)

```
"Clona el CRT como 'CRT What-If Inyeccion A'"
"En el clon, aplica la inyeccion y muestra las diferencias con el original"
"Haz un diff entre el CRT original y el what-if"
```

---

## 6. Patrones Avanzados

### 6.1. Colapso para vision ejecutiva

Cuando el CRT es muy detallado y necesitas presentar un resumen:

```
"Colapsa la cadena entre RC-001 y UDE-005 con el label 'Cadena logistica ineficiente'"
"Muestra el walk del arbol con los macro edges visibles"
```

### 6.2. Explotar supuestos en nodos

Si un supuesto es tan importante que merece ser un nodo explicito en la cadena:

```
"Explota el supuesto ASM-003 del link LINK-007 como nodo intermedio 'La capacidad no se ajusta estacionalmente'"
```

### 6.3. Negative Branch Reservations completas

```
"Crea una NBR desde INJ-001 en el FRT"
"La inyeccion podria causar: 'Aumento de costos de transporte express' → 'Margen reducido en pedidos pequenos'"
"Anade una inyeccion de trimming: 'Establecer pedido minimo de 50 EUR para express'"
```

### 6.4. Batch operations con undo atomico

Cuando vas a hacer muchas operaciones relacionadas:

```
"Inicia un batch llamado 'Construccion CRT principal'"
[... muchas operaciones ...]
"Cierra el batch"
```

Si algo sale mal: `"Deshaz todo"` — revierte el batch completo de una vez.

### 6.5. Feedback loops

Para modelar ciclos de retroalimentacion (que refuerzan o estabilizan):

```
"Crea un feedback loop positivo: UDE-003 refuerza RC-001 ('La perdida de clientes reduce ingresos, lo que agrava la falta de inversion')"
```

### 6.6. Conectar arboles: refs y meta-grafo inferido (v0.4.0)

Un nodo puede **referirse** a otro nodo de otro arbol (ADR-015). El caso tipico: cada UDE del CRT se mide contra una NC (o CSF) del Goal Tree.

```
"Haz que UDE-001 referencie NC-002 del Goal Tree"
"Que relaciones hay entre arboles?"
"Valida el workspace: que UDEs no tienen norma?"
```

Por CLI:

```bash
ltp node edit UDE-001 --add-ref NC-002@tree-gt-meta   # el pin @TREE es opcional
ltp tree relation list                                # meta-grafo inferido
ltp tree relation list --tree tree-crt-realidad       # solo relaciones que tocan ese arbol
ltp validate                                          # NORM_REF_MISSING / DANGLING_NODE_REF en _meta_graph
```

Claves:
- Las relaciones son **inferidas al vuelo y sin tipo**: el motor dice que el CRT *referencia* al GT y con que `logic` va cada extremo, pero no interpreta la intencion (eso lo hace Claude o la UI).
- Sin pin, una ref a un nodo adjunto a 2 arboles produce 2 relaciones; usa `@TREE` para fijar una.
- Refs dentro del mismo arbol se guardan pero no generan relacion.
- `NORM_REF_MISSING` solo aparece si existe al menos un Goal Tree.
- `node rm` limpia las refs entrantes (`REFS_STRIPPED`) y `node split` las redirige a ambos hijos. Todo es deshacible.

---

## 7. Preguntas Poderosas para Claude durante el Analisis

### Exploracion inicial
- "Que informacion tengo disponible? Resume el knowledge pool"
- "Que patrones ves en los UDEs que he registrado?"
- "Sugiere posibles causas raiz comunes para estos UDEs"

### Construccion del CRT
- "Revisa la logica de este CRT. Hay insuficiencias?"
- "Que supuestos ocultos hay entre INT-002 y UDE-001?"
- "Donde hay flechas largas que necesitan nodos intermedios?"
- "Cual es la causa raiz con mayor impacto (mas UDEs downstream)?"

### Gestion epistemica
- "Estado epistemico: que proporcion de nodos son hechos vs hipotesis?"
- "Que hipotesis son las mas riesgosas si resultan falsas?"
- "Que deberia validar primero?"
- "Hay conocimiento en el inbox sin vincular?"
- "Que contradicciones tiene el analisis actual?"

### Evaporating Cloud
- "Que supuestos sostienen cada flecha de la EC?"
- "Cual es el supuesto mas debil? Donde podemos romper?"
- "Sugiere inyecciones que invaliden ASM-003"

### FRT
- "Con esta inyeccion, que UDEs se eliminan y cuales persisten?"
- "Que efectos negativos podria causar esta inyeccion?"
- "Las NBR estan neutralizadas?"

### Calidad general
- "Haz un walk completo del CRT" (orden por defecto segun la logica; pide "en orden reverse" o "en orden topologico" para forzarlo)
- "Hay nodos huerfanos o desconectados?"
- "El grafo es un DAG valido?"
- "Dame un resumen ejecutivo del analisis"

---

## 8. Ejemplo Completo: Sesion Tipica

```
Yo: "Inicializa un workspace LTP llamado 'Retrasos Logistica 2026'"

Yo: "Registra estas mediciones como knowledge:
     - Tiempo medio de entrega: 18.3 dias (fuente: ERP, Q2-2026, confianza alta)
     - 23% de pedidos llegan despues de 20 dias (fuente: dashboard CX, confianza alta)
     - El picking tarda 4.2 dias de media (fuente: WMS, confianza alta)"

Yo: "Registra estos testimonios:
     - 'El problema es que consolidamos demasiados pedidos antes de enviar' (Director Ops, reunion 12-ago)
     - 'Los transportistas no recogen hasta tener carga completa' (Jefe Almacen)"

Yo: "Crea un CRT. Los UDEs son:
     1. El tiempo de entrega supera 15 dias
     2. Los clientes se quejan activamente del servicio
     3. Estamos perdiendo contratos con clientes clave
     4. El equipo de ventas no puede cerrar renovaciones"

Yo: "Sugiere causas raiz para estos UDEs basandote en la evidencia del knowledge pool"

[Claude sugiere, yo valido y ajusto]

Yo: "Conecta la cadena causal. Valida el arbol."

Yo: "Que hipotesis estamos asumiendo? Registralas en el knowledge pool"

Yo: "Cual es la causa raiz con mayor impacto?"

[Itero: anade evidencia, refina el CRT, busca el core problem]

Yo: "Crea una EC para el conflicto central"

Yo: "Que supuestos podemos invalidar? Genera inyecciones"

Yo: "Crea un FRT con las inyecciones. Busca NBRs"

Yo: "Crea el PRT — que obstaculos hay para implementar?"
```

---

## 9. Tips y Buenas Practicas

1. **Registra TODO como knowledge primero** — incluso antes de construir el grafo. El inbox es tu memoria externa.

2. **Valida frecuentemente** — cada 3-5 operaciones. Corregir a tiempo es mas barato. Y siempre despues de `node rm` o `node split` con `affected_trees` no vacio: valida esos arboles. `split` es global (reescribe todos los arboles que usan el nodo), y `rm` avisa con `NBR_BRANCH_REMOVED` / `MACRO_EDGE_REMOVED` de las ramas y flechas largas que destruye. Lo que queda inconsistente (p. ej. un resumen de flecha larga obsoleto) solo lo detecta `validate`.

3. **Marca el status epistemico** — un CRT lleno de hipotesis no verificadas es un castillo de naipes. Haz visible lo que sabes vs. lo que supones.

4. **Usa batches** para construcciones grandes — un solo undo si algo sale mal.

5. **Clona antes de experimentar** — `tree clone` te da un sandbox sin riesgo.

6. **Pregunta por contradicciones** — la herramienta detecta cuando evidencia verificada contradice hechos declarados.

7. **No busques perfeccion al inicio** — empieza con UDEs y causas basicas, refina iterativamente.

8. **Traza para entender impacto** — antes de proponer inyecciones, traza downstream desde la RC candidata para ver cuantos UDEs resuelve.

9. **Supuestos explicitos** — cada flecha tiene supuestos ocultos. Hacerlos explicitos es el paso previo a romperlos.

10. **El motor no juzga causalidad** — eso lo haces tu con ayuda de Claude. El motor garantiza coherencia topologica y te avisa de patrones sospechosos.

---

## 10. Tools MCP Disponibles

| Grupo | Tools | Proposito |
|-------|-------|-----------|
| Workspace | `init`, `status` | Crear y diagnosticar workspace |
| Node | `add`, `edit`, `rm`, `inspect`, `list`, `search`, `split` | Gestionar entidades del pool |
| Tree | `new`, `list`, `rename`, `rm`, `attach`, `detach`, `clone`, `diff`, `walk`, `relation_list` | Gestionar vistas topologicas y el meta-grafo inferido |
| Link | `connect`, `disconnect`, `feedback`, `inspect`, `find`, `reverse`, `move`, `insert_between`, `group`, `dissolve`, `split`, `reoperator`, `add_cause`, `rm_cause` | Relaciones causa-efecto |
| Assume | `add`, `edit`, `rm`, `list`, `move` | Supuestos en edges |
| Logic | `invalidate`, `validate`, `trace` | Romper supuestos, validar, explorar |
| Path | `collapse`, `explode`, `replace` | Abstraccion y mutacion de sub-grafos |
| Macro-assume | `gather`, `add`, `rm`, `list` | Supuestos-resumen de la flecha larga (Slice 1) |
| Macro (flecha larga) | `add`, `expand`, `promote` | Ciclo de vida de la flecha larga (Slice 2) |
| NBR | `add`, `rm`, `list`, `inspect` | Negative Branch Reservations |
| History | `undo`, `redo`, `list`, `check`, `invalidate`, `begin_batch`, `end_batch`, `clear` | Deshacer/rehacer |
| Knowledge | `add`, `edit`, `rm`, `inspect`, `list`, `link`, `unlink` | Gestion de evidencia |

Todos prefijados con `ltp/` (ej. `ltp/node_add`, `ltp/knowledge_add`, `ltp/tree_walk`). Esta tabla es un resumen orientativo; **la lista canónica y siempre actual es `tools/list` del servidor MCP** — no dependas del recuento aquí.

---

## 11. Resolucion de Problemas

| Problema | Solucion |
|----------|----------|
| "Tool ltp/init not found" | `claude mcp add --scope user ltp -- ltp-mcp` y reinicia sesion |
| "Workspace not initialized" | Ejecuta `ltp/init` primero |
| "WORKSPACE_LOCKED" | Otro proceso tiene el lock. Si es stale, se auto-libera |
| "UNDO_STATE_DIVERGED" | Editaste archivos manualmente. Usa `ltp/history_check` y luego `ltp/history_invalidate` |
| Un `node rm` borro una flecha larga (`MACRO_EDGE_REMOVED`) | Se borro un extremo o el overlay se quedo sin interior. El warning lista los `assumption_ids` perdidos; `undo` lo revierte |
| `macro expand`/`path replace` devuelve `NODE_NOT_FOUND` o `NODE_NOT_IN_TREE` sobre un extremo | La flecha larga apunta a un nodo borrado o desadjuntado (dano de versiones < 0.5.0 o un `tree detach`). Ejecuta `validate` y repara el extremo |
| "INVALID_ORDER" en `tree_walk` | `order` solo admite `topological` o `reverse` (en minusculas). Omitelo para usar el orden de la logica del arbol |
| El walk de un GT sale "al reves" respecto a antes | Desde ADR-014 el GT recorre en `reverse` (desde el objetivo). Pide `order: topological` para el orden anterior |
| Tools no aparecen en la sesion | Verifica con `claude mcp list` que `ltp` esta registrado |
| Version desactualizada | `cd LTP_Rust && git pull && cargo install --path .` |
