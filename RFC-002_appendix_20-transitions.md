# RFC-002 Apendice: Auditoria Sistematica de las 20 Transiciones entre Arboles LTP

| Campo       | Valor                              |
|-------------|------------------------------------|
| Status      | **EXPLORING**                      |
| Author      | Javier Asensio                     |
| Created     | 2026-09-11                         |
| Parent      | RFC-002 (Meta-Grafo)               |
| Scope       | Grafo dirigido completo 5×4 = 20 transiciones entre GT, CRT, EC, FRT, PRT/TT |

---

## 1. Contexto

El LTP de Dettmer/Fedurko se ensena como secuencia lineal: GT → CRT → EC → FRT → PRT → TT. Pero en la practica avanzada los arboles funcionan como un **ecosistema modular**. Hay retroalimentaciones, saltos, iteraciones. Este apendice audita exhaustivamente las 20 transiciones directas posibles (5 tipos de arbol × 4 destinos cada uno) para servir como guia tecnica de referencia y como input para los `relation_type` del meta-grafo (RFC-002 seccion 3.2).

### Logica de cada arbol

| Arbol | Logica | Lectura | Direccion |
|-------|--------|---------|-----------|
| GT | Necesidad | "Para lograr X, debo tener Y" | Top-down |
| CRT | Suficiencia | "Si A, entonces B" | Bottom-up |
| EC | Necesidad | "Para A, necesito B; para B, debo D" + XOR | Top-down |
| FRT | Suficiencia | "Si INJ, entonces DE" | Bottom-up |
| PRT/TT | Necesidad (PRT) / Suficiencia (TT) | PRT: "Para INJ, debo superar OBS via IO" / TT: "Si accion, entonces nueva realidad" | PRT top-down, TT bottom-up |

> **Nota sobre PRT/TT**: se tratan como unidad por su acoplamiento funcional (IOs del PRT alimentan directamente el TT). Cuando la distincion importa se explicita.

---

## 2. Matriz Resumen 5×5

Clasificacion de cada transicion: **(a)** Estandar/Secuencial, **(b)** Retroalimentacion/Ajuste, **(c)** Patron Avanzado/No Convencional.

| Origen ↓ \ Destino → | GT | CRT | EC | FRT | PRT/TT |
|---|---|---|---|---|---|
| **GT** | — | **(a)** gap_analysis | **(c)** validacion directa | **(c)** benchmark futuro | **(c)** roadmap estrategico |
| **CRT** | **(b)** feedback norma | — | **(a)** core_conflict | **(c)** salto directo | **(c)** accion de emergencia |
| **EC** | **(b)** redefinicion meta | **(b)** revision causal | — | **(a)** injection | **(c)** implementacion directa |
| **FRT** | **(c)** revision meta | **(b)** revision | **(b)** challenges | — | **(a)** prerequisite/tactical |
| **PRT/TT** | **(b)** revision meta | **(b)** realidad cambiada | **(c)** nuevo conflicto | **(b)** feedback futuro | — |

### Leyenda de colores logicos

| Transicion logica | Pares |
|-------------------|-------|
| nec → suf | GT→CRT, EC→FRT, PRT→TT |
| suf → nec | CRT→EC, FRT→PRT, CRT→GT, FRT→EC, TT→PRT |
| nec → nec | GT→EC, EC→GT, GT→PRT/TT, EC→PRT/TT, PRT→GT |
| suf → suf | CRT→FRT, FRT→CRT, CRT→PRT/TT*, TT→CRT, TT→FRT |

*PRT es necesidad; TT es suficiencia. Se indica en cada transicion.

---

## 3. Las 20 Transiciones — Analisis Detallado

---

### GRUPO A: Transiciones desde el GT (Goal Tree)

---

#### T01. GT → CRT — Gap Analysis

| Dimension | Detalle |
|-----------|---------|
| **Viabilidad** | **(a) Estandar/Secuencial** — Primera transicion del flujo canonico |
| **Logic transition** | nec → suf |
| **relation_type** | `gap_analysis` |

**Por que (Desencadenante):** El GT define la norma ideal (Goal, CSFs, NCs). El analista compara cada NC contra la realidad. Las NCs no cumplidas se convierten en UDEs del CRT. El trigger es: "tengo la norma completa, ahora necesito medir la brecha con la realidad".

**Para que (Proposito):**
1. **Fundamentar el diagnostico en la norma**: sin GT, el CRT es una lista de quejas sin referencia. Con GT, cada UDE es una violacion medible de una NC especifica.
2. **Priorizar UDEs por impacto**: las UDEs que violan CSFs de alto nivel son mas criticas que las que violan NCs perifericas.
3. **Garantizar cobertura**: el GT como checklist asegura que no hay areas del sistema sin diagnosticar.

**Escenario de negocio:** Un Director de Operaciones logisticas define su GT: Goal = "Logistica de clase mundial", CSFs = "Entregas on-time", "Coste por envio competitivo", "Visibilidad completa del pedido". Al comparar con la realidad, identifica que "Entregas on-time" tiene una tasa del 72% (NC viola la norma del 95%). Esa brecha se convierte en UDE-001: "El 28% de las entregas llegan fuera de plazo".

---

#### T02. GT → EC — Validacion Directa de Conflicto

| Dimension | Detalle |
|-----------|---------|
| **Viabilidad** | **(c) Patron Avanzado** — Salta CRT. Solo justificado cuando el conflicto es evidente |
| **Logic transition** | nec → nec |
| **relation_type** | `core_conflict` (atipico) |

**Por que (Desencadenante):** Al construir el GT, el analista descubre que dos CSFs o dos NCs son **inherentemente incompatibles**: satisfacer una impide o dificulta satisfacer la otra. No hace falta un CRT para diagnosticarlo — el conflicto emerge directamente del diseno del sistema ideal.

**Para que (Proposito):**
1. **Resolver conflictos estructurales antes de diagnosticar**: si el propio GT tiene un conflicto interno, construir un CRT sobre esa base produce un diagnostico sobre cimientos rotos.
2. **Validar que la meta es coherente**: la EC fuerza a explicitar los supuestos detras de las NCs en tension.
3. **Acelerar el ciclo en sistemas donde el problema es de diseno**: si el sistema nunca funciono bien, el problema no esta en la realidad sino en la meta misma.

**Escenario de negocio:** Una startup de SaaS define su GT con CSF-1: "Adquisicion agresiva de clientes" y CSF-2: "Calidad de soporte excepcional". Al formular NCs, descubre que NC-1 ("Equipo de ventas de 20+ personas") y NC-5 ("Ratio de soporte 1:50 clientes") son incompatibles con el presupuesto actual. El conflicto no esta en la realidad operativa — esta en el diseno del propio objetivo. Se construye una EC directamente sobre los CSFs en tension.

**Advertencia metodologica:** Este salto es peligroso si se usa como atajo. Solo es legitimo cuando el conflicto es *estructural al GT*, no cuando el analista "cree saber" cual es el conflicto sin haber diagnosticado la realidad.

---

#### T03. GT → FRT — Benchmark de Futuro

| Dimension | Detalle |
|-----------|---------|
| **Viabilidad** | **(c) Patron Avanzado** — Salta CRT y EC. Uso muy especifico |
| **Logic transition** | nec → suf |
| **relation_type** | `benchmark` (nuevo, o `extends`) |

**Por que (Desencadenante):** El GT existe como norma estrategica. Se propone una transformacion radical (nueva tecnologia, adquisicion, reestructuracion) cuyas consecuencias se quieren simular *antes* de diagnosticar problemas actuales. El trigger es: "tenemos una iniciativa estrategica — modelemos su impacto futuro contra la norma".

**Para que (Proposito):**
1. **Evaluacion proactiva de oportunidades**: no todo LTP empieza por un problema. A veces empieza por una oportunidad que se quiere validar.
2. **Analisis what-if estrategico**: "si implementamos IA en logistica, que DEs conseguimos contra nuestros CSFs?"
3. **Complemento del flujo tradicional**: el FRT de benchmark puede luego compararse con el FRT post-EC para medir si la solucion derivada del analisis causal supera a la iniciativa proactiva.

**Escenario de negocio:** El Director de IT tiene su GT definido. El CEO propone migrar todo a la nube. En vez de diagnosticar primero que esta mal (CRT), el Director construye un FRT de benchmark: "Si migramos a cloud, entonces [efecto 1, 2, 3...]" y mapea los DEs contra los CSFs del GT. Esto le permite evaluar la propuesta antes de invertir meses en diagnostico.

**Advertencia metodologica:** Este patron NO sustituye al flujo completo. Es una evaluacion rapida. Si el FRT de benchmark muestra gaps, el flujo completo (GT→CRT→EC→FRT) sigue siendo necesario para entender POR QUE los gaps existen.

---

#### T04. GT → PRT/TT — Roadmap Estrategico

| Dimension | Detalle |
|-----------|---------|
| **Viabilidad** | **(c) Patron Avanzado** — Salta todo el diagnostico. Solo para greenfield |
| **Logic transition** | nec → nec (GT→PRT) o nec → suf (GT→TT) |
| **relation_type** | `strategic_roadmap` (nuevo, o `tactical`) |

**Por que (Desencadenante):** En un sistema greenfield (que aun no existe), no hay realidad que diagnosticar (no hay CRT posible), ni conflicto que resolver (no hay EC). Los NCs del GT se convierten directamente en objetivos intermedios del PRT: "para cumplir esta NC, necesito superar estos obstaculos". El trigger es: "estamos creando algo desde cero".

**Para que (Proposito):**
1. **Planificacion de implementacion greenfield**: convertir la norma en plan de accion sin pasar por diagnostico de problemas inexistentes.
2. **Identificar obstaculos de construccion**: el PRT fuerza a pensar "que nos impide lograr esta NC" incluso cuando no hay sistema actual.
3. **Secuenciar la construccion**: el TT ordena las acciones tacticas para edificar el sistema desde cero.

**Escenario de negocio:** Una empresa lanza una nueva linea de negocio. El GT define la meta y requisitos. No hay CRT (no hay operacion actual). Los NCs del GT se convierten en objetivos del PRT: "Para NC-003 (Sistema de tracking GPS), obstaculos: no hay proveedor seleccionado, no hay integracion IT, no hay formacion de conductores". Cada obstaculo genera un IO y luego acciones en el TT.

**Advertencia metodologica:** Solo legitimo en greenfield real. Si el sistema existe (aunque sea embrionario), el CRT es necesario para no repetir errores no diagnosticados.

---

### GRUPO B: Transiciones desde el CRT (Current Reality Tree)

---

#### T05. CRT → GT — Feedback a la Norma

| Dimension | Detalle |
|-----------|---------|
| **Viabilidad** | **(b) Retroalimentacion/Ajuste** — Documentado por Dettmer como loop correctivo |
| **Logic transition** | suf → nec |
| **relation_type** | `revision` |

**Por que (Desencadenante):** Al construir el CRT, se descubre que ciertas UDEs no violan ningun CSF/NC del GT. Esto significa que el GT esta incompleto: hay dimensiones del sistema que no se capturaron como norma. Alternativamente, una CRC revela que una NC del GT es *parte del problema* — la norma misma es contraproducente.

**Para que (Proposito):**
1. **Completar el GT**: anadir NCs/CSFs que faltan para que todas las UDEs tengan ancla en la norma.
2. **Corregir NCs contraproducentes**: si una NC causa UDEs (la norma es parte del problema), revisarla antes de continuar.
3. **Asegurar coherencia GT↔CRT**: la regla "cada UDE viola un CSF/NC del GT" debe cumplirse bidireccionalmente.

**Escenario de negocio:** Al construir el CRT de logistica, el analista identifica UDE-009: "Los conductores reportan fatiga cronica". Ninguna NC del GT cubre condiciones laborales de conductores — el GT solo mira entregas, costes y visibilidad. Se vuelve al GT para agregar CSF: "Bienestar operativo del equipo de campo" con NCs asociadas. Sin esta revision, la solucion final podria ignorar un factor humano critico.

---

#### T06. CRT → EC — Core Conflict

| Dimension | Detalle |
|-----------|---------|
| **Viabilidad** | **(a) Estandar/Secuencial** — Segunda transicion del flujo canonico |
| **Logic transition** | suf → nec |
| **relation_type** | `core_conflict` |

**Por que (Desencadenante):** El CRT ha identificado la CRC (Causa Raiz Central): la causa que conecta downstream con >70% de las UDEs. La CRC persiste porque hay un conflicto subyacente — una politica, practica o creencia que la sostiene. El trigger es: "sabemos CUAL es la causa raiz; ahora necesitamos entender POR QUE se tolera".

**Para que (Proposito):**
1. **Aislar el conflicto**: la EC estructura el dilema detras de la CRC como conflicto de necesidades incompatibles.
2. **Generar inyecciones fundamentadas**: las INJs nacen de supuestos invalidados, no de intuiciones.
3. **Evitar soluciones sintomaticas**: sin EC, las soluciones atacan efectos intermedios del CRT en vez de la raiz logica.

**Escenario de negocio:** El CRT de logistica identifica CRC: "La politica de entregas prioriza volumen sobre puntualidad" (conecta con el 85% de UDEs). La EC aisla el conflicto: A="Logistica competitiva", B="Minimizar coste por envio" (gerencia financiera), C="Maximizar puntualidad" (cliente), D="Agrupar envios para eficiencia" vs D'="Rutas dedicadas por urgencia". El conflicto es: eficiencia vs servicio.

---

#### T07. CRT → FRT — Salto Directo (sin EC)

| Dimension | Detalle |
|-----------|---------|
| **Viabilidad** | **(c) Patron Avanzado** — Salta la EC. Riesgoso pero a veces justificado |
| **Logic transition** | suf → suf |
| **relation_type** | `injection` (atipico) |

**Por que (Desencadenante):** La CRC del CRT tiene una solucion obvia o ya conocida (por experiencia, regulacion o imposicion externa). El conflicto no necesita aislarse porque no hay dilema: la causa raiz tiene una unica via de resolucion. El trigger es: "sabemos exactamente que hacer; necesitamos validar que funciona".

**Para que (Proposito):**
1. **Acelerar ciclo en problemas bien entendidos**: si la solucion es regulatoria ("la ley cambia, debemos cumplir"), no hay conflicto que evaporar.
2. **Validar soluciones impuestas**: el FRT simula el impacto de una solucion que no se eligio sino que se recibio.
3. **Analisis de impacto rapido**: util en gestion de crisis donde el tiempo para construir una EC completa no existe.

**Escenario de negocio:** El CRT revela que la CRC es "no hay sistema de trazabilidad de envios". La regulacion de transporte obliga a implementar tracking GPS antes del Q2. No hay dilema: la solucion es impuesta. Se salta la EC y se construye el FRT directamente: "Si implementamos GPS tracking, entonces [DEs esperados]". Los NBRs del FRT validan riesgos de la implementacion obligatoria.

**Advertencia metodologica:** El SKILL.md de ltp-mcp lista este salto como "Error comun: saltar directamente a FRT sin EC — inyecciones sin fundamento logico". Solo es legitimo cuando genuinamente no hay dilema. Si hay la minima duda, construir la EC.

---

#### T08. CRT → PRT/TT — Accion de Emergencia

| Dimension | Detalle |
|-----------|---------|
| **Viabilidad** | **(c) Patron Avanzado** — Salta EC y FRT. Solo para emergencias |
| **Logic transition** | suf → nec (CRT→PRT) o suf → suf (CRT→TT) |
| **relation_type** | `emergency_action` (nuevo, o `tactical`) |

**Por que (Desencadenante):** El CRT revela una situacion critica que requiere accion inmediata antes de completar el analisis: un riesgo de seguridad, una perdida financiera acelerada, o un incumplimiento regulatorio inminente. El trigger es: "no podemos esperar a resolver el conflicto raiz — necesitamos actuar AHORA sobre un efecto intermedio critico".

**Para que (Proposito):**
1. **Contener dano inmediato**: estabilizar el sistema antes de buscar la solucion raiz.
2. **Comprar tiempo para el analisis completo**: las acciones de emergencia no resuelven la CRC, pero reducen la urgencia para poder pensar con claridad.
3. **Documentar la deuda**: el PRT/TT de emergencia queda registrado como accion tactica que NO resuelve la raiz, evitando la ilusion de que "ya se arreglo".

**Escenario de negocio:** El CRT de una planta quimica identifica UDE-001: "Las valvulas de presion fallan sin previo aviso" (CRC: politica de mantenimiento basada en calendario, no en condicion). No se puede esperar a la EC/FRT — hay riesgo de seguridad. Se salta directamente a un TT de emergencia: inspeccion inmediata de todas las valvulas, protocolo de cierre preventivo, y contratacion temporal de inspectores. Paralelamente, el flujo completo GT→CRT→EC→FRT→PRT→TT sigue su curso para la solucion estructural.

**Advertencia metodologica:** Las acciones de emergencia son parches. SIEMPRE deben ir acompanadas del flujo completo en paralelo. Marcar la tree_relation como `emergency_action` para distinguirla de las transiciones normales.

---

### GRUPO C: Transiciones desde la EC (Evaporating Cloud)

---

#### T09. EC → GT — Redefinicion de la Meta

| Dimension | Detalle |
|-----------|---------|
| **Viabilidad** | **(b) Retroalimentacion/Ajuste** — Loop correctivo poco frecuente pero valioso |
| **Logic transition** | nec → nec |
| **relation_type** | `revision` |

**Por que (Desencadenante):** Al construir la EC, se descubre que el nodo A (objetivo comun) o las necesidades B/C revelan que el GT esta mal formulado. El objetivo de la EC no se puede reconciliar con el Goal del GT: hay una inconsistencia fundamental. Alternativamente, los supuestos invalidados de la EC revelan que una NC del GT era un supuesto no examinado.

**Para que (Proposito):**
1. **Corregir el GT antes de generar inyecciones**: si la meta es incoherente, las INJs resolveran el conflicto equivocado.
2. **Incorporar insight de los supuestos**: los supuestos invalidados de la EC a menudo revelan creencias implicitas que permeaban el GT sin cuestionarse.
3. **Evitar optimizar para un objetivo equivocado**: resolver un conflicto cuyo objetivo A no alinea con el Goal del GT produce inyecciones irrelevantes.

**Escenario de negocio:** La EC de logistica tiene A="Logistica competitiva". Al generar supuestos, se invalida: "Asumimos que competitiva significa barata". El equipo descubre que el mercado ha cambiado — competitiva ahora significa rapida y predecible, no barata. El Goal del GT ("Minimizar coste logistico total") necesita revision: deberia ser "Maximizar fiabilidad logistica dentro de margenes sostenibles". Sin esta revision, todas las inyecciones optimizarian coste cuando el mercado pide velocidad.

---

#### T10. EC → CRT — Revision Causal

| Dimension | Detalle |
|-----------|---------|
| **Viabilidad** | **(b) Retroalimentacion/Ajuste** — Loop correctivo comun |
| **Logic transition** | nec → suf |
| **relation_type** | `revision` |

**Por que (Desencadenante):** Al construir la EC, el analista descubre que la CRC elegida no es la verdadera raiz del conflicto: el nodo D o D' no conecta logicamente con la CRC del CRT, o los supuestos revelan que hay otra causa mas profunda no capturada. El trigger es: "la EC no cuadra con lo que el CRT dice — necesitamos re-diagnosticar".

**Para que (Proposito):**
1. **Corregir la CRC**: si la causa raiz estaba mal identificada, el CRT necesita extension o reestructuracion.
2. **Descubrir causas ocultas**: los supuestos de la EC a menudo apuntan a causas que el CRT no capturo porque estaban fuera del marco inicial.
3. **Ampliar el scope del CRT**: la EC puede revelar que el sistema tiene mas actores o mas interacciones de las que el CRT modelaba.

**Escenario de negocio:** La EC de logistica identifica D="Agrupar envios" vs D'="Rutas dedicadas". Al generar supuestos para la flecha D→B, se descubre: "Asumimos que agrupar envios reduce costes". Al investigar, resulta que el coste real no viene de las rutas sino de las penalizaciones por retraso que el CRT no capturo. Se vuelve al CRT para agregar la cadena causal: "Penalizaciones contractuales → INT-008 → UDE-003" que cambia la CRC.

---

#### T11. EC → FRT — Injection

| Dimension | Detalle |
|-----------|---------|
| **Viabilidad** | **(a) Estandar/Secuencial** — Tercera transicion del flujo canonico |
| **Logic transition** | nec → suf |
| **relation_type** | `injection` |

**Por que (Desencadenante):** La EC ha producido una o varias INJs al invalidar supuestos. Cada INJ necesita ser validada: simular su efecto en la realidad futura para verificar que (a) elimina las UDEs del CRT, (b) no introduce efectos negativos imprevistos. El trigger es: "tenemos la inyeccion — ahora hay que probar que funciona".

**Para que (Proposito):**
1. **Validar la INJ contra toda la realidad**: el FRT no solo verifica que la INJ resuelve la CRC, sino que mapea su impacto en TODAS las cadenas del sistema.
2. **Detectar efectos colaterales (NBRs)**: ramas causales negativas que la EC no podia anticipar porque opera en abstraccion.
3. **Generar la matriz UDE→DE**: cada UDE del CRT debe tener un DE (Desired Effect) opuesto en el FRT. Si no, la INJ es insuficiente.

**Escenario de negocio:** La EC invalida el supuesto "Solo podemos entregar a tiempo con rutas dedicadas". INJ: "Implementar sistema de optimizacion de rutas dinamicas que agrupe envios compatibles por ventana de tiempo y zona". El FRT simula: "Si optimizacion dinamica, entonces entregas on-time >95% Y coste por envio -15%". Se detecta NBR: "Si optimizacion dinamica, entonces dependencia critica de un unico proveedor de software". Trimming: contrato multi-vendor.

---

#### T12. EC → PRT/TT — Implementacion Directa

| Dimension | Detalle |
|-----------|---------|
| **Viabilidad** | **(c) Patron Avanzado** — Salta FRT. Solo para inyecciones triviales |
| **Logic transition** | nec → nec (EC→PRT) o nec → suf (EC→TT) |
| **relation_type** | `tactical` (atipico) |

**Por que (Desencadenante):** La INJ generada por la EC es tan sencilla y de bajo riesgo que no necesita simulacion en un FRT completo. Tipicamente es un cambio de politica, una decision administrativa, o una accion que no tiene efectos colaterales plausibles. El trigger es: "la inyeccion es obvia y sin riesgo — pasemos a implementarla".

**Para que (Proposito):**
1. **Evitar ceremonia innecesaria**: un FRT para "cambiar la politica de aprobacion de gastos de 3 firmas a 1 firma" es overengineering.
2. **Implementar quick wins**: mientras el flujo completo continua para la solucion estructural, INJs menores se implementan inmediatamente.
3. **Mantener momentum**: las victorias rapidas generan confianza en el proceso.

**Escenario de negocio:** La EC de logistica produce una INJ secundaria (ademas de la principal): "Eliminar la aprobacion manual del jefe de almacen para envios menores a 5kg". Es un cambio de politica sin efectos colaterales. Se salta el FRT y se crea un TT directo: paso 1 actualizar politica, paso 2 comunicar al equipo, paso 3 ajustar sistema IT, paso 4 monitorizar 2 semanas.

**Advertencia metodologica:** Solo para INJs menores y de bajo riesgo. La INJ principal SIEMPRE pasa por FRT con NBRs.

---

### GRUPO D: Transiciones desde el FRT (Future Reality Tree)

---

#### T13. FRT → GT — Revision de la Meta Post-Simulacion

| Dimension | Detalle |
|-----------|---------|
| **Viabilidad** | **(c) Patron Avanzado** — Raro pero transformador |
| **Logic transition** | suf → nec |
| **relation_type** | `revision` |

**Por que (Desencadenante):** El FRT revela que la solucion propuesta *supera* los CSFs del GT original, abriendo posibilidades no contempladas. O al contrario: el FRT muestra que incluso con la mejor INJ, ciertos CSFs son inalcanzables, lo que obliga a redefinir la meta. El trigger es: "la simulacion del futuro ha cambiado nuestra comprension de lo que es posible".

**Para que (Proposito):**
1. **Elevar la ambicion**: si el FRT muestra que la INJ logra 95% on-time cuando el GT pedia 90%, el GT puede (y debe) ajustarse al alza.
2. **Aceptar limitaciones realistas**: si el FRT muestra que el maximo alcanzable es 85% on-time, el GT necesita ajustar la NC en vez de perseguir un imposible.
3. **Capturar oportunidades emergentes**: el FRT puede revelar DEs no anticipados que merecen convertirse en CSFs del GT.

**Escenario de negocio:** El FRT de la optimizacion de rutas muestra un DE inesperado: "La visibilidad en tiempo real permite ofrecer servicio premium a clientes B2B con tracking live". Esto no era parte del GT. El Director revisa el GT para agregar CSF: "Revenue adicional por servicios de visibilidad premium". La meta evoluciona de "logistica eficiente" a "logistica como ventaja competitiva".

---

#### T14. FRT → CRT — Revision de la Realidad

| Dimension | Detalle |
|-----------|---------|
| **Viabilidad** | **(b) Retroalimentacion/Ajuste** — Loop comun documentado por Dettmer |
| **Logic transition** | suf → suf |
| **relation_type** | `revision` |

**Por que (Desencadenante):** Al simular el futuro, el FRT revela UDEs que el CRT no capturo. La cadena causal del FRT, al modelar la interaccion entre la INJ y la realidad, descubre efectos intermedios que no eran visibles en el diagnostico inicial. El trigger es: "la simulacion del futuro ha revelado un problema del presente que no habiamos visto".

**Para que (Proposito):**
1. **Completar el diagnostico**: el CRT gana cadenas causales que solo se hacen visibles cuando se intenta cambiar el sistema.
2. **Evitar soluciones que empeoran lo existente**: si el FRT revela una UDE nueva, el CRT necesita incorporarla para que la solucion la contemple.
3. **Iterar con precision**: el ciclo FRT→CRT→EC→FRT es el loop de mejora incremental del LTP.

**Escenario de negocio:** El FRT de "optimizacion de rutas dinamicas" revela: "Si optimizacion dinamica, y si los conductores no confian en el algoritmo, entonces los conductores ignoran las rutas sugeridas". El CRT no capturaba la resistencia cultural de los conductores. Se vuelve al CRT para agregar: UDE-012: "Los conductores desconfian de sistemas automatizados". Esto puede cambiar la CRC o generar una cadena causal nueva que modifique la EC.

---

#### T15. FRT → EC — Challenges (Nuevo Conflicto)

| Dimension | Detalle |
|-----------|---------|
| **Viabilidad** | **(b) Retroalimentacion/Ajuste** — Tipicamente via NBR |
| **Logic transition** | suf → nec |
| **relation_type** | `challenges` |

**Por que (Desencadenante):** Un NBR del FRT revela un efecto negativo grave que no se puede recortar con una trimming INJ. La INJ actual genera un problema tan serio que invalida la solucion: se necesita una INJ diferente, lo que obliga a volver a la EC para buscar otro supuesto que invalidar. El trigger es: "la solucion es peor que el problema — necesitamos una inyeccion diferente".

**Para que (Proposito):**
1. **Buscar INJ alternativa**: volver a los supuestos de la EC y buscar otro que invalidar.
2. **Reexaminar el conflicto**: el NBR puede revelar que el conflicto de la EC estaba mal formulado (D y D' no eran las verdaderas posiciones en tension).
3. **Escenarios alternativos**: cada INJ alternativa genera un escenario nuevo (FRT-A vs FRT-B), enriqueciendo el meta-grafo con relaciones `alternative`.

**Escenario de negocio:** El FRT de "optimizacion de rutas" tiene un NBR critico: "Si dependemos de un algoritmo centralizado, y si el sistema cae durante la temporada alta, entonces todas las entregas se paralizan simultaneamente (riesgo catastrofico)". La trimming INJ (redundancia de servidores) es prohibitivamente cara. Se vuelve a la EC para buscar otra INJ: en vez de invalidar "solo podemos agrupar envios estáticamente", se invalida "la optimizacion debe ser centralizada" — nueva INJ: "Optimizacion descentralizada con heuristicas locales por zona".

---

#### T16. FRT → PRT/TT — Prerequisite/Tactical

| Dimension | Detalle |
|-----------|---------|
| **Viabilidad** | **(a) Estandar/Secuencial** — Transiciones 5 y 6 del flujo canonico |
| **Logic transition** | suf → nec (FRT→PRT), suf → suf (implicito FRT→PRT→TT) |
| **relation_type** | `prerequisite` (FRT→PRT), `tactical` (PRT→TT) |

**Por que (Desencadenante):** El FRT esta completo: todas las UDEs tienen DEs opuestos, los NBRs estan recortados, la INJ esta validada e inmunizada. Ahora hay que implementar. El trigger es: "sabemos QUE hacer y que FUNCIONA — ahora necesitamos un plan de COMO hacerlo".

**Para que (Proposito):**
1. **Identificar obstaculos de implementacion**: el PRT fuerza a pensar "que nos impide implementar esta INJ" — recursos, permisos, dependencias, resistencias.
2. **Secuenciar la ejecucion**: los IOs del PRT se ordenan cronologicamente: que primero, que despues.
3. **Granularizar en acciones**: el TT convierte cada IO en pasos ejecutables por una persona concreta con los 5 elementos por paso.

**Escenario de negocio:** La INJ validada "Implementar optimizacion de rutas descentralizada" entra en el PRT. Obstaculos identificados: OBS-1 "No hay proveedor de software seleccionado" → IO-1 "Evaluar y contratar proveedor"; OBS-2 "Los conductores no estan formados" → IO-2 "Programa de formacion". El TT detalla IO-1: Paso 1: "Realidad actual: no hay shortlist de proveedores. Necesidad: base para evaluacion. Accion: solicitar propuestas a 3 proveedores preseleccionados. Nueva realidad: shortlist evaluable. Siguiente necesidad: criterios de seleccion definidos."

---

### GRUPO E: Transiciones desde el PRT/TT

---

#### T17. PRT/TT → GT — Revision Estrategica Post-Implementacion

| Dimension | Detalle |
|-----------|---------|
| **Viabilidad** | **(b) Retroalimentacion/Ajuste** — Loop de cierre del ciclo completo |
| **Logic transition** | nec → nec (PRT→GT) o suf → nec (TT→GT) |
| **relation_type** | `revision` |

**Por que (Desencadenante):** Durante la implementacion (PRT/TT), se descubre que la meta original del GT no es alcanzable con los recursos disponibles, o que la implementacion ha revelado oportunidades que requieren redefinir la norma. El trigger es: "la ejecucion nos ha ensenado algo nuevo sobre lo que es posible/necesario".

**Para que (Proposito):**
1. **Feedback estrategico**: la implementacion es la prueba de fuego de la estrategia. Si algo no funciona, la meta debe adaptarse.
2. **Evolucion continua del GT**: el GT no es estatico — cada ciclo de implementacion lo refina.
3. **Cerrar el loop LTP**: GT → ... → TT → GT completa el ciclo y prepara la siguiente iteracion.

**Escenario de negocio:** Durante la implementacion del TT, el IO-3 "Negociar con proveedor logistico" revela que el proveedor ofrece un servicio de analitica predictiva no contemplado. Esto cambia lo que es posible: el GT puede ahora incluir un CSF de "Prediccion de demanda logistica" que antes era imposible. La meta evoluciona.

---

#### T18. PRT/TT → CRT — La Realidad Ha Cambiado

| Dimension | Detalle |
|-----------|---------|
| **Viabilidad** | **(b) Retroalimentacion/Ajuste** — Loop de monitorizacion post-implementacion |
| **Logic transition** | suf → suf (TT→CRT) o nec → suf (PRT→CRT) |
| **relation_type** | `revision` |

**Por que (Desencadenante):** La implementacion del TT ha cambiado la realidad. El CRT original ya no refleja el estado actual del sistema. Ademas, la implementacion puede haber generado UDEs nuevas que no existian antes. El trigger es: "hemos cambiado el sistema — necesitamos rediagnosticar".

**Para que (Proposito):**
1. **Medir impacto real**: construir un CRT post-implementacion y compararlo con el original para verificar que las UDEs se eliminaron.
2. **Detectar UDEs nuevas**: toda intervencion en un sistema genera efectos no previstos. El CRT post es la red de seguridad.
3. **Alimentar el siguiente ciclo**: si hay UDEs nuevas, el flujo completo se reinicia sobre la nueva realidad.

**Escenario de negocio:** Tras implementar la optimizacion de rutas, se construye un CRT post-implementacion. UDEs originales eliminadas: entregas tardias (-80%), coste excesivo (-15%). UDE nueva: "El equipo de soporte recibe quejas de conductores que no entienden las rutas asignadas". Esta UDE nueva inicia un mini-ciclo: CRT post → EC (conflicto: eficiencia algoritmica vs comprension humana) → FRT (nueva INJ: dashboard de explicabilidad para conductores).

---

#### T19. PRT/TT → EC — Nuevo Conflicto de Implementacion

| Dimension | Detalle |
|-----------|---------|
| **Viabilidad** | **(c) Patron Avanzado** — Raro pero legitimamente necesario |
| **Logic transition** | nec → nec (PRT→EC) o suf → nec (TT→EC) |
| **relation_type** | `challenges` |

**Por que (Desencadenante):** Un obstaculo del PRT revela un conflicto genuino: dos IOs son mutuamente excluyentes, o la ejecucion de una accion del TT requiere algo que contradice otra accion. El conflicto no es de diagnostico (CRT) ni de solucion (FRT) — es de implementacion. El trigger es: "no podemos implementar porque hay un dilema tactico".

**Para que (Proposito):**
1. **Resolver conflictos de implementacion con rigor**: en vez de "negociar" o "decidir intuitivamente", la EC estructura el dilema con supuestos y genera una INJ de implementacion.
2. **Evitar compromisos que debilitan la solucion**: sin EC, el conflicto de implementacion se resuelve con un compromiso que puede invalidar la INJ original.
3. **Documentar la decision**: la EC de implementacion queda como registro de por que se eligio un camino sobre otro.

**Escenario de negocio:** El PRT tiene OBS-1: "El proveedor de software exige exclusividad (contrato de 3 anos)" y OBS-4: "IT exige multi-vendor para evitar lock-in". Estos obstaculos son contradictorios. Se construye una EC de implementacion: A="Implementar optimizacion de rutas", B="Coste predecible" (→ D: "Firmar exclusividad"), C="Flexibilidad tecnologica" (→ D': "Mantener multi-vendor"). Los supuestos revelan: "Asumimos que exclusividad = lock-in". INJ: "Contrato con clausula de portabilidad de datos + API abierta". Se resuelve el dilema tactico y el PRT continua.

---

#### T20. PRT/TT → FRT — Feedback al Futuro

| Dimension | Detalle |
|-----------|---------|
| **Viabilidad** | **(b) Retroalimentacion/Ajuste** — Loop de refinamiento |
| **Logic transition** | nec → suf (PRT→FRT) o suf → suf (TT→FRT) |
| **relation_type** | `revision` |

**Por que (Desencadenante):** Durante la planificacion de prerequisitos o la ejecucion tactica, se descubre que el FRT no contemplaba ciertas realidades de implementacion. Un obstaculo del PRT revela que una cadena causal del FRT no es viable, o una accion del TT muestra que un DE asumido no se materializa como se esperaba. El trigger es: "la implementacion muestra que nuestra simulacion del futuro era incompleta".

**Para que (Proposito):**
1. **Actualizar el modelo del futuro**: el FRT necesita incorporar restricciones de implementacion que no eran visibles en la simulacion abstracta.
2. **Verificar que la INJ sigue siendo valida**: si las restricciones de implementacion cambian las cadenas causales, la INJ puede necesitar ajuste.
3. **Recalibrar NBRs**: los obstaculos del PRT pueden revelar NBRs no detectados que requieren trimming INJs adicionales.

**Escenario de negocio:** El PRT identifica OBS: "El sistema actual no tiene API para integracion con el optimizador de rutas". Este obstaculo implica un DE del FRT que decia "Los datos de pedido fluyen automaticamente al optimizador" es falso sin una migracion de sistemas. Se vuelve al FRT para agregar la cadena: "Si migramos a sistema con API (INJ adicional), entonces los datos fluyen". Esto puede cambiar el analisis coste-beneficio completo.

---

## 4. Sintesis: Patrones Emergentes

### 4.1. Clusters de transiciones por funcion

| Cluster | Transiciones | Funcion |
|---------|-------------|---------|
| **Flujo canonico** | T01, T06, T11, T16 | Progresion lineal GT→CRT→EC→FRT→PRT/TT |
| **Loops correctivos** | T05, T09, T10, T14 | Retroalimentacion que mejora arboles anteriores |
| **Challenges** | T15, T19 | Conflictos que obligan a buscar alternativas |
| **Revision post-implementacion** | T17, T18, T20 | La implementacion cambia la comprension |
| **Saltos acelerados** | T07, T08, T12 | Atajos para situaciones con poco conflicto/riesgo |
| **Patrones estrategicos** | T02, T03, T04, T13 | Uso avanzado en greenfield, benchmark o evolucion de meta |

### 4.2. Las 20 transiciones por utilidad practica

| Tier | Transiciones | Descripcion |
|------|-------------|-------------|
| **Tier 1 — Esenciales** | T01, T06, T11, T16 | Flujo canonico. Todo analisis LTP las usa. |
| **Tier 2 — Frecuentes** | T05, T10, T14, T15, T18, T20 | Loops correctivos y challenges. Analisis serios las usan regularmente. |
| **Tier 3 — Situacionales** | T07, T09, T12, T13, T17, T19 | Patrones que se activan en contextos especificos. No en todo analisis. |
| **Tier 4 — Excepcionales** | T02, T03, T04, T08 | Patrones avanzados para casos atipicos (greenfield, emergencias, conflictos estructurales). |

### 4.3. Alternancia de logica en las transiciones

| Patron | Cantidad | Transiciones |
|--------|----------|-------------|
| suf → nec | 6 | T06, T13, T15, T05*, T17*, T19* |
| nec → suf | 4 | T01, T10, T11, T16* |
| suf → suf | 5 | T07, T14, T18*, T20*, T08* |
| nec → nec | 5 | T02, T03*, T04*, T09, T12*, T17* |

*Depende de si el destino es PRT (nec) o TT (suf).

**Hallazgo**: las transiciones canonicas (Tier 1) alternan sistematicamente entre logicas. Los loops correctivos tienden a ser same-logic (suf→suf para CRT/FRT, nec→nec para GT/EC/PRT). Esto tiene sentido: la retroalimentacion se queda "en el mismo modo de pensar", mientras que la progresion fuerza el cambio de perspectiva.

### 4.4. El meta-grafo como registro de navegacion

Las 20 transiciones demuestran que un analisis LTP real no es lineal — es un **grafo de navegacion** donde el analista va y viene entre arboles segun lo que descubre. El meta-grafo (RFC-002) captura esta navegacion como entidad persistente:

- Cada transicion es una **tree_relation** con `relation_type`, `handoff_nodes`, y `logic_transition` calculado.
- Los loops correctivos crean multiples tree_relations entre el mismo par de arboles (CRT↔FRT puede tener una relacion `injection` y dos `revision`).
- Los escenarios emergen naturalmente de las transiciones T15 (challenges que generan INJs alternativas) y T03/T13 (benchmarks que comparan futuros).

---

## 5. Mapping a los `relation_type` del RFC-002

### Cobertura actual

| relation_type (RFC-002 S3.2) | Transiciones cubiertas | Tier |
|------------------------------|----------------------|------|
| `gap_analysis` | T01 (GT→CRT) | 1 |
| `core_conflict` | T06 (CRT→EC), T02 (GT→EC, atipico) | 1, 4 |
| `injection` | T11 (EC→FRT), T07 (CRT→FRT, atipico) | 1, 3 |
| `risk_check` | (FRT→NBR, no es transicion inter-arbol principal) | 1 |
| `prerequisite` | T16 (FRT→PRT) | 1 |
| `tactical` | T16 (PRT→TT), T12 (EC→PRT/TT), T08 (CRT→PRT/TT) | 1, 3, 4 |
| `alternative` | (derivado de T15 cuando hay INJ alternativa) | 2 |
| `revision` | T05, T09, T10, T13, T14, T17, T18, T20 | 2, 3 |
| `challenges` | T15 (FRT→EC), T19 (PRT/TT→EC) | 2, 3 |
| `extends` | (sub-arboles del mismo tipo) | — |

### Transiciones sin `relation_type` explicito

| Transicion | relation_type propuesto | Notas |
|-----------|------------------------|-------|
| T03 (GT→FRT) | `benchmark` | Nuevo. Evaluacion proactiva sin pasar por diagnostico. |
| T04 (GT→PRT/TT) | `strategic_roadmap` | Nuevo. Greenfield donde NCs→obstaculos directamente. |
| T08 (CRT→PRT/TT) | `emergency_action` | Nuevo. Accion de contencion urgente. Marcar como temporal. |

### Recomendacion

Agregar 3 `relation_type` nuevos al RFC-002:

```
| benchmark         | Evaluacion proactiva de una iniciativa contra la norma  | GT → FRT           |
| strategic_roadmap | Planificacion greenfield desde la norma                  | GT → PRT/TT        |
| emergency_action  | Contencion urgente antes de completar el analisis        | CRT → PRT/TT       |
```

Estos 3 tipos son Tier 4 (excepcionales) pero necesitan `relation_type` propio para distinguirlos de las transiciones estandar.

---

## 6. Implicaciones para ltp-engine

### Validacion

`ltp validate` puede emitir warnings cuando detecte transiciones inusuales:

| Warning | Trigger | Severidad |
|---------|---------|-----------|
| `CANONICAL_SKIP_EC` | Existe relacion CRT→FRT sin EC intermedia (T07) | MEDIA |
| `CANONICAL_SKIP_FRT` | Existe relacion EC→PRT/TT sin FRT intermedio (T12) | BAJA |
| `EMERGENCY_ACTION_OPEN` | Relacion `emergency_action` sin flujo completo paralelo | MEDIA |
| `BENCHMARK_WITHOUT_DIAGNOSIS` | Relacion `benchmark` GT→FRT sin CRT existente | INFO |
| `MULTIPLE_REVISIONS` | >2 relaciones `revision` entre el mismo par de arboles | INFO |

### Narrativa

`trace --narrative` puede usar esta taxonomia para generar narrativas enriquecidas:

```
[FLUJO CANONICO] GT "Logistica competitiva" → CRT "Diagnostico Q3" (gap_analysis)
  Transicion logica: necesidad → suficiencia
  Handoff: 5 NCs no cumplidas → 5 UDEs

[LOOP CORRECTIVO] FRT "Futuro con optimizacion" → CRT "Diagnostico Q3" (revision)
  Transicion logica: suficiencia → suficiencia (same_suf)
  Trigger: FRT revelo UDE-012 no detectada

[CHALLENGE] FRT "Futuro con optimizacion" → EC "Conflicto entregas" (challenges)
  Transicion logica: suficiencia → necesidad
  Trigger: NBR catastrofico no recortable
```

---

## Changelog

| Fecha | Cambio |
|-------|--------|
| 2026-09-11 | Creacion: auditoria completa de 20 transiciones, matriz 5×5, mapping a relation_types, implicaciones para el motor |
