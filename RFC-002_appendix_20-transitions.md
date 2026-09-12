# RFC-002 Apendice: Auditoria Sistematica de las 36 Transiciones entre Arboles LTP

| Campo       | Valor                              |
|-------------|------------------------------------|
| Status      | **EXPLORING**                      |
| Author      | Javier Asensio                     |
| Created     | 2026-09-11                         |
| Updated     | 2026-09-12                         |
| Parent      | RFC-002 (Meta-Grafo)               |
| Scope       | Grafo dirigido completo 6×6 = 36 transiciones entre GT, CRT, EC, FRT, NBR, PRT/TT (20 inter-arbol + 10 con NBR + 6 auto-transiciones) |

---

## 1. Contexto

El LTP de Dettmer/Fedurko se ensena como secuencia lineal: GT → CRT → EC → FRT → PRT → TT. Pero en la practica avanzada los arboles funcionan como un **ecosistema modular**. Hay retroalimentaciones, saltos, iteraciones. Este apendice audita exhaustivamente las 36 transiciones posibles en una matriz 6×6 (GT, CRT, EC, FRT, NBR, PRT/TT): 20 transiciones inter-arbol originales + 10 transiciones con NBR como 6.ª herramienta + 6 auto-transiciones (diagonal). Sirve como guia tecnica de referencia y como input para los `relation_type` del meta-grafo (RFC-002 seccion 3.2).

> **Nota sobre NBR**: el NBR (Negative Branch Reservation) es un proceso analitico distinto del FRT. Aunque se aplica *sobre* el FRT, genera sus propios artefactos (UDEs colaterales, trimming INJs) y retroalimenta multiples arboles. Tratarlo como herramienta independiente captura transiciones que de otro modo quedan implicitas.

### Logica de cada arbol

| Arbol | Logica | Lectura | Direccion |
|-------|--------|---------|-----------|
| GT | Necesidad | "Para lograr X, debo tener Y" | Top-down |
| CRT | Suficiencia | "Si A, entonces B" | Bottom-up |
| EC | Necesidad | "Para A, necesito B; para B, debo D" + XOR | Top-down |
| FRT | Suficiencia | "Si INJ, entonces DE" | Bottom-up |
| NBR | Suficiencia | "Si INJ, Y SI [condicion], entonces [efecto negativo]" | Bottom-up |
| PRT/TT | Necesidad (PRT) / Suficiencia (TT) | PRT: "Para INJ, debo superar OBS via IO" / TT: "Si accion, entonces nueva realidad" | PRT top-down, TT bottom-up |

> **Nota sobre PRT/TT**: se tratan como unidad por su acoplamiento funcional (IOs del PRT alimentan directamente el TT). Cuando la distincion importa se explicita.

> **Nota sobre NBR**: el NBR opera con logica de suficiencia — extiende cadenas causales del FRT anadiendo condiciones negativas ("Y SI...") para explorar ramas de riesgo. Los efectos negativos que identifica son **UDEs colaterales**: misma tipologia que las UDEs del CRT, distinto origen (nacen de la INJ, no de la realidad actual).

---

## 2. Matriz Resumen 6×6

Clasificacion de cada transicion: **(a)** Estandar/Secuencial, **(b)** Retroalimentacion/Ajuste, **(c)** Patron Avanzado/No Convencional. La diagonal (auto-transiciones) se marca con **(d)**.

| Origen ↓ \ Destino → | GT | CRT | EC | FRT | NBR | PRT/TT |
|---|---|---|---|---|---|---|
| **GT** | **(d)** T31 self_revision | **(a)** T01 gap_analysis | **(c)** T02 validacion directa | **(c)** T03 benchmark futuro | **(c)** T26 risk proactivo | **(c)** T04 roadmap estrategico |
| **CRT** | **(b)** T05 feedback norma | **(d)** T32 extends | **(a)** T06 core_conflict | **(c)** T07 salto directo | **(c)** T27 anticipacion riesgo | **(c)** T08 accion emergencia |
| **EC** | **(b)** T09 redefinicion meta | **(b)** T10 revision causal | **(d)** T33 self_revision | **(a)** T11 injection | **(c)** T28 risk pre-FRT | **(c)** T12 implementacion directa |
| **FRT** | **(c)** T13 revision meta | **(b)** T14 revision | **(b)** T15 challenges | **(d)** T34 extends | **(a)** T29 risk_check | **(a)** T16 prerequisite/tactical |
| **NBR** | **(c)** T21 revision meta riesgo | **(b)** T22 realidad oculta | **(b)** T23 nuevo conflicto | **(a)** T24 trimming | **(d)** T35 extends | **(b)** T25 obstaculos riesgo |
| **PRT/TT** | **(b)** T17 revision meta | **(b)** T18 realidad cambiada | **(c)** T19 nuevo conflicto | **(b)** T20 feedback futuro | **(b)** T30 riesgos impl. | **(d)** T36 self_revision |

### Leyenda de colores logicos

| Transicion logica | Pares |
|-------------------|-------|
| nec → suf | GT→CRT, EC→FRT, PRT→TT, GT→NBR, EC→NBR |
| suf → nec | CRT→EC, FRT→PRT, CRT→GT, FRT→EC, TT→PRT, NBR→GT, NBR→EC |
| nec → nec | GT→EC, EC→GT, GT→PRT/TT, EC→PRT/TT, PRT→GT, GT→GT, EC→EC, PRT→PRT |
| suf → suf | CRT→FRT, FRT→CRT, CRT→PRT/TT*, TT→CRT, TT→FRT, FRT→NBR, NBR→FRT, NBR→CRT, NBR→PRT/TT, CRT→NBR, CRT→CRT, FRT→FRT, NBR→NBR, TT→NBR |

*PRT es necesidad; TT es suficiencia. Se indica en cada transicion.

---

## 3. Las 36 Transiciones — Analisis Detallado

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

### GRUPO F: Transiciones desde/hacia el NBR (Negative Branch Reservation)

> **Premisa**: el NBR es el 6.º proceso analitico del LTP. Opera con logica de suficiencia extendiendo cadenas causales del FRT con condiciones adversas ("Y SI..."). Sus hallazgos — UDEs colaterales — retroalimentan multiples arboles. Tratar el NBR como herramienta independiente captura 10 transiciones que de otro modo quedan implicitas dentro de "el FRT tiene NBRs".

---

#### T21. NBR → GT — Revision de la Meta por Riesgo Sistemico

| Dimension | Detalle |
|-----------|---------|
| **Viabilidad** | **(c) Patron Avanzado** — Raro. Solo cuando los riesgos cuestionan la meta misma |
| **Logic transition** | suf → nec |
| **relation_type** | `revision` |

**Por que (Desencadenante):** El NBR revela que incluso con la mejor INJ disponible, los efectos colaterales son tan graves y sistémicos que la meta misma se vuelve cuestionable. No es un problema de la INJ — es que *cualquier* intervencion hacia esa meta genera riesgos inaceptables. El trigger es: "la meta produce riesgos inherentes que ninguna inyeccion puede mitigar".

**Para que (Proposito):**
1. **Cuestionar la meta antes de optimizar la solucion**: si el destino es toxico, mejorar la ruta no ayuda.
2. **Incorporar dimensiones de riesgo al GT**: anadir CSFs de resiliencia, sostenibilidad o seguridad que la meta original no contemplaba.
3. **Evitar la trampa de la escalada**: sin esta transicion, el analista busca INJ tras INJ para una meta que deberia reformularse.

**Escenario de negocio:** El GT de una empresa logistica tiene Goal="Automatizacion total del almacen". La INJ elegida ("sistema robotizado de picking") genera un NBR: "Si automatizacion total, Y SI hay fallo de software, entonces paralizacion completa del almacen (0 capacidad manual residual)". Otro NBR: "Si automatizacion total, entonces despido masivo de personal local (riesgo reputacional y regulatorio)". Estos NBRs no se resuelven con trimming — son inherentes a la meta. Se vuelve al GT para reformular: "Automatizacion selectiva con capacidad manual residual del 30%".

---

#### T22. NBR → CRT — Descubrimiento de Realidad Oculta

| Dimension | Detalle |
|-----------|---------|
| **Viabilidad** | **(b) Retroalimentacion/Ajuste** — Relativamente frecuente |
| **Logic transition** | suf → suf |
| **relation_type** | `revision` |

**Por que (Desencadenante):** Los NBRs del FRT revelan cadenas causales de la realidad actual que el CRT no capturo. Al modelar "que pasaria si..." con la INJ, emergen condiciones del sistema actual que no eran visibles en el diagnostico original. El trigger es: "el analisis de riesgo revelo aspectos de la realidad que no habiamos diagnosticado".

**Para que (Proposito):**
1. **Completar el diagnostico con informacion emergente**: el CRT gana cadenas causales que solo se manifiestan cuando se intenta perturbar el sistema.
2. **Entender por que los NBRs son posibles**: si el NBR dice "los conductores ignoraran las rutas", eso revela una realidad (cultura de autonomia) que el CRT deberia capturar.
3. **Informar futuras iteraciones**: un CRT mas completo produce ECs mas precisas y FRTs mas robustos.

**Escenario de negocio:** El FRT propone INJ "Optimizacion dinamica de rutas". El NBR identifica: "Si optimizacion dinamica, Y SI los conductores no confian en el algoritmo, entonces los conductores ignoran las rutas sugeridas y la eficiencia cae". Esto revela que el CRT no capturaba UDE-012: "Existe una cultura de autonomia entre los conductores que rechaza sistemas automatizados de decision". Se vuelve al CRT para incorporar esta cadena causal, lo que puede cambiar la CRC o ampliar el scope del conflicto.

---

#### T23. NBR → EC — Nuevo Conflicto por Efecto Colateral

| Dimension | Detalle |
|-----------|---------|
| **Viabilidad** | **(b) Retroalimentacion/Ajuste** — El caso recursivo documentado en RFC-002 S14 |
| **Logic transition** | suf → nec |
| **relation_type** | `challenges` |

**Por que (Desencadenante):** Un NBR grave que no se puede recortar con una trimming INJ genera un conflicto genuino. La UDE colateral es tan severa que requiere su propia EC para resolverla: hay dos necesidades incompatibles — la necesidad de implementar la INJ original y la necesidad de evitar el efecto colateral. El trigger es: "la solucion genera un problema que es un conflicto en si mismo".

**Para que (Proposito):**
1. **Resolver el efecto colateral con rigor**: una EC formal estructura el conflicto del NBR con supuestos invalidables, no con parches ad hoc.
2. **Activar la recursion LTP controlada**: EC hija → INJ de trimming fundamentada → FRT actualizado. Cada nivel de recursion es visible en el meta-grafo.
3. **Evitar la escalada de trimming**: sin EC, las trimming INJs se acumulan sin fundamento logico. Con EC, cada trimming tiene un supuesto invalidado que la justifica.

**Escenario de negocio:** El FRT de "Optimizacion de rutas" tiene un NBR critico: "Si dependemos de un unico proveedor de software, Y SI el proveedor quiebra o sube precios, entonces lock-in tecnologico catastrofico". La trimming obvia ("tener plan B") es vaga. Se construye una EC: A="Implementar optimizacion de rutas exitosamente", B="Coste predecible a largo plazo" (→ D: "Contrato exclusivo con proveedor"), C="Flexibilidad tecnologica" (→ D': "Arquitectura multi-vendor"). Los supuestos revelan: "Asumimos que optimizacion avanzada requiere solucion propietaria". INJ de trimming fundamentada: "Implementar capa de abstraccion con API estandar (OSRM) que permita cambiar de proveedor sin reescritura".

**Nota RFC-002 S14:** Esta transicion genera una EC hija que es **especifica del escenario** que produjo el NBR. No es compartida con otros escenarios. El meta-grafo debe capturar esta relacion de parentesco.

---

#### T24. NBR → FRT — Trimming (Recorte de Ramas Negativas)

| Dimension | Detalle |
|-----------|---------|
| **Viabilidad** | **(a) Estandar/Secuencial** — El paso estandar post-NBR |
| **Logic transition** | suf → suf |
| **relation_type** | `trimming` (nuevo) |

**Por que (Desencadenante):** El NBR ha identificado una UDE colateral que *puede* mitigarse con una trimming INJ — una intervencion adicional que elimina o reduce la rama negativa sin invalidar la INJ principal. El trigger es: "el riesgo es manejable con una intervencion adicional".

**Para que (Proposito):**
1. **Inmunizar el FRT**: cada trimming INJ anula una rama negativa, convirtiendo un FRT vulnerable en uno robusto.
2. **Enriquecer la solucion**: las trimming INJs son parte integral de la solucion final — no son parches sino refinamientos.
3. **Documentar la mitigacion**: el FRT actualizado muestra exactamente como cada riesgo se mitigo, con trazabilidad al NBR que lo origino.

**Escenario de negocio:** El NBR del FRT de logistica identifica: "Si optimizacion dinamica, Y SI el algoritmo falla durante temporada alta, entonces todas las entregas se paralizan". Trimming INJ: "Implementar modo de fallback con rutas estaticas pre-calculadas que se activan automaticamente si el optimizador no responde en 30 segundos". El FRT se actualiza: la cadena "algoritmo falla → paralizacion" se corta con un nuevo DE: "Si fallo detectado, entonces fallback automatico → entregas continuan con eficiencia reducida (-15%) pero sin paralizacion".

**Relacion con T23:** Si el NBR no se puede recortar con trimming (el riesgo es demasiado grave o la trimming es inviable), se escala a T23 (NBR→EC). La decision entre T24 y T23 es un juicio del analista sobre la severidad.

---

#### T25. NBR → PRT/TT — Obstaculos Derivados de Riesgos

| Dimension | Detalle |
|-----------|---------|
| **Viabilidad** | **(b) Retroalimentacion/Ajuste** — Para NBRs menores que son riesgos operativos |
| **Logic transition** | suf → nec (NBR→PRT) o suf → suf (NBR→TT) |
| **relation_type** | `prerequisite` (atipico) |

**Por que (Desencadenante):** El NBR identifica UDEs colaterales menores que no requieren trimming INJ (no son lo suficientemente graves para cambiar el FRT) pero si requieren gestion operativa durante la implementacion. Se convierten en obstaculos del PRT o en pasos preventivos del TT. El trigger es: "el riesgo es menor pero hay que gestionarlo durante la implementacion".

**Para que (Proposito):**
1. **No perder riesgos menores**: sin esta transicion, los NBRs que no escalan a trimming se olvidan. Con ella, se convierten en tareas explicitas del plan.
2. **Enriquecer el PRT con mitigaciones**: los obstaculos derivados de NBRs son cualitativamente diferentes de los obstaculos de implementacion normales — son riesgos anticipados, no barreras presentes.
3. **Priorizar la implementacion por riesgo**: los pasos del TT que mitigan NBRs pueden requerir secuenciacion especifica (mitigar antes de arriesgar).

**Escenario de negocio:** El NBR del FRT identifica un riesgo menor: "Si optimizacion dinamica, Y SI los conductores no reciben formacion, entonces curva de adaptacion de 3 meses con eficiencia reducida". No es catastrofico (T24) ni genera conflicto (T23), pero requiere gestion. Se convierte en OBS del PRT: "Los conductores no estan formados en el nuevo sistema". IO: "Programa de formacion pre-lanzamiento". Paso del TT: "Semana -4: taller de 8h con conductores piloto; Semana -2: periodo de sombra con sistema en paralelo".

---

#### T26. GT → NBR — Evaluacion de Riesgos de la Norma

| Dimension | Detalle |
|-----------|---------|
| **Viabilidad** | **(c) Patron Avanzado** — Muy raro. Evaluacion proactiva |
| **Logic transition** | nec → suf |
| **relation_type** | `risk_check` (atipico) |

**Por que (Desencadenante):** Antes de diagnosticar la realidad actual (CRT), el analista quiere evaluar que riesgos inherentes tiene la propia norma del GT. Es un analisis proactivo: "si lograramos la meta tal como esta definida, que podria salir mal?" El trigger es: "queremos validar que la meta no tiene efectos secundarios antes de invertir en diagnostico y solucion".

**Para que (Proposito):**
1. **Deteccion temprana de metas toxicas**: si la meta misma genera riesgos inaceptables, mejor saberlo antes de meses de analisis.
2. **Informar el diseno de la meta**: los NBRs proactivos sobre el GT pueden revelar CSFs que faltan (seguridad, sostenibilidad, etica).
3. **Ahorro de esfuerzo**: si el GT necesita reformulacion por riesgos inherentes, evita un ciclo completo CRT→EC→FRT→NBR para descubrirlo.

**Escenario de negocio:** Una empresa define su GT con Goal="Maximizar throughput del almacen a 10.000 pedidos/dia". Antes de diagnosticar por que hoy solo hacen 3.000, un NBR proactivo evalua: "Si 10.000 pedidos/dia, Y SI la infraestructura electrica actual es insuficiente, entonces riesgo de sobrecarga"; "Si 10.000 pedidos/dia, Y SI la normativa de seguridad limita el trafico interno de carretillas, entonces incumplimiento legal". Estos NBRs revelan que el Goal necesita CSFs adicionales: "Dentro de limites de infraestructura" y "Cumplimiento normativo de seguridad".

**Advertencia metodologica:** Este patron es util como sanity check rapido, no como sustituto del flujo completo. Los NBRs sobre el GT son necesariamente abstractos (no hay INJ concreta que evaluar). Su valor es de filtro, no de diseno.

---

#### T27. CRT → NBR — Anticipacion de Riesgos Pre-Solucion

| Dimension | Detalle |
|-----------|---------|
| **Viabilidad** | **(c) Patron Avanzado** — Raro. Evaluacion hipotetica |
| **Logic transition** | suf → suf |
| **relation_type** | `risk_check` (atipico) |

**Por que (Desencadenante):** Al analizar la CRC del CRT, el analista anticipa que *cualquier* intervencion sobre esa causa raiz generara cierto tipo de riesgos, independientemente de la INJ especifica que se elija. Es un "pre-NBR" que evalua la zona de intervencion antes de tener una solucion concreta. El trigger es: "sabemos DONDE vamos a intervenir — evaluemos los riesgos de tocar esa zona".

**Para que (Proposito):**
1. **Guiar la EC**: saber que riesgos genera intervenir en la CRC ayuda a elegir supuestos que, al invalidarse, minimicen esos riesgos.
2. **Filtrar INJs de alto riesgo**: si un tipo de intervencion esta pre-descartado por riesgos, la EC puede enfocarse en supuestos que generen INJs de otro tipo.
3. **Preparar al equipo**: los stakeholders saben de antemano que tipo de riesgos acompanaran la solucion, sea cual sea.

**Escenario de negocio:** El CRT identifica CRC: "El sistema de gestion de inventario es manual". El analista sabe que cualquier solucion implicara digitalizacion. Un NBR anticipatorio evalua: "Si digitalizamos el inventario (cualquier forma), Y SI los operarios no son digitalmente competentes, entonces periodo de caos operativo". Este riesgo es independiente de la INJ especifica (software A vs B). Saberlo de antemano guia la EC: buscar supuestos cuya invalidacion produzca INJs con curva de adopcion baja.

**Advertencia metodologica:** Sin INJ concreta, los NBRs son genericos y de baja precision. Util como orientacion, no como evaluacion definitiva.

---

#### T28. EC → NBR — Evaluacion Rapida de Riesgo Pre-FRT

| Dimension | Detalle |
|-----------|---------|
| **Viabilidad** | **(c) Patron Avanzado** — Para filtrar INJs antes de invertir en FRTs completos |
| **Logic transition** | nec → suf |
| **relation_type** | `risk_check` |

**Por que (Desencadenante):** La EC ha producido multiples INJs (de invalidar supuestos distintos). Antes de construir un FRT completo para cada una, el analista aplica un NBR rapido a cada INJ para descartar las de riesgo inaceptable. El trigger es: "tenemos 3-5 INJs candidatas — filtremos por riesgo antes de invertir en simulacion completa".

**Para que (Proposito):**
1. **Eficiencia del proceso**: un FRT completo requiere esfuerzo significativo. Un NBR rapido por INJ es una evaluacion de 30 minutos vs dias de trabajo.
2. **Descarte temprano**: si una INJ tiene un NBR catastrofico obvio (seguridad, legalidad), no merece FRT.
3. **Priorizacion informada**: las INJs que sobrevivan al filtro de NBR rapido se priorizan por menor riesgo residual.

**Escenario de negocio:** La EC de logistica produce 3 INJs: INJ-A "Optimizacion dinamica centralizada", INJ-B "Heuristicas locales por zona", INJ-C "Subcontratar toda la logistica". Un NBR rapido evalua: INJ-A tiene riesgo de single point of failure (medio), INJ-B tiene riesgo de sub-optimizacion global (bajo), INJ-C tiene riesgo de perdida de core competency (alto). Se descarta INJ-C, se priorizan INJ-B y INJ-A para FRT completo.

**Relacion con escenarios (RFC-002 S14):** Esta transicion es el mecanismo natural del proceso scout en el modelo de escenarios: evaluar alternativas rapidamente antes de comprometerse.

---

#### T29. FRT → NBR — Verificacion de Ramas Negativas (Estandar)

| Dimension | Detalle |
|-----------|---------|
| **Viabilidad** | **(a) Estandar/Secuencial** — Paso canonico post-FRT |
| **Logic transition** | suf → suf |
| **relation_type** | `risk_check` |

**Por que (Desencadenante):** El FRT esta construido con la INJ principal y sus DEs. Antes de pasar a implementacion, hay que verificar sistematicamente si alguna cadena causal del FRT tiene ramas negativas: efectos no deseados que emergen de las mismas condiciones que producen los DEs. El trigger es: "tenemos el futuro deseado — ahora verifiquemos que no estamos creando problemas nuevos".

**Para que (Proposito):**
1. **Inmunizar la solucion**: el NBR es el stress test de la INJ. Sin el, la solucion tiene puntos ciegos.
2. **Identificar UDEs colaterales**: cada "Y SI..." que produce un efecto negativo es una UDE colateral que necesita gestion (trimming T24, escalada T23, o absorcion operativa T25).
3. **Completar el modelo del futuro**: un FRT sin NBR es un FRT optimista. Con NBR, el modelo del futuro es realista.

**Escenario de negocio:** El FRT de "Optimizacion dinamica de rutas" muestra DE-001: "Entregas on-time >95%", DE-002: "Coste por envio -15%", DE-003: "Visibilidad en tiempo real". El NBR sistematico evalua cada cadena: "Si optimizacion dinamica, Y SI el volumen de datos excede la capacidad del servidor, entonces latencia > 10s → conductores ven rutas desactualizadas → accidentes de trafico". Tambien: "Si optimizacion dinamica, Y SI hay zona sin cobertura movil, entonces conductores sin ruta asignada → entregas improvisadas". Cada NBR se clasifica por severidad y se decide su tratamiento (T24, T23 o T25).

---

#### T30. PRT/TT → NBR — Riesgos de Implementacion

| Dimension | Detalle |
|-----------|---------|
| **Viabilidad** | **(b) Retroalimentacion/Ajuste** — La implementacion revela riesgos no anticipados |
| **Logic transition** | nec → suf (PRT→NBR) o suf → suf (TT→NBR) |
| **relation_type** | `risk_check` |

**Por que (Desencadenante):** Durante la planificacion de prerequisitos (PRT) o la ejecucion tactica (TT), se descubren riesgos que el NBR original no anticipó porque solo son visibles con el detalle de implementacion. La granularidad del PRT/TT revela condiciones que el FRT abstracto no modelaba. El trigger es: "al planificar/ejecutar, hemos descubierto riesgos que no vimos en el analisis".

**Para que (Proposito):**
1. **Capturar riesgos emergentes de implementacion**: los riesgos de "como hacerlo" son diferentes de los riesgos de "que pasa si funciona".
2. **Retroalimentar el FRT si es necesario**: si el NBR de implementacion revela un riesgo grave, puede escalar a T24 (trimming) o T23 (nueva EC).
3. **Ajustar el plan**: los riesgos de implementacion pueden cambiar la secuencia del TT o anadir pasos de mitigacion.

**Escenario de negocio:** El TT tiene paso 3: "Migrar datos historicos de pedidos al nuevo sistema de optimizacion". Un NBR de implementacion evalua: "Si migramos en produccion, Y SI hay incompatibilidades de formato entre sistemas, entonces perdida de datos historicos → imposibilidad de entrenar el algoritmo con datos reales". Este riesgo no era visible en el FRT (que solo decia "los datos fluyen al optimizador"). Se necesita una trimming operativa: "Migracion en entorno de staging primero, con validacion cruzada antes de produccion".

---

### GRUPO G: Auto-Transiciones (Diagonal)

> **Premisa**: las auto-transiciones capturan procesos de refinamiento interno donde un arbol se revisa, extiende o reestructura *sin input de otro arbol*. No son transiciones triviales — representan momentos de reflexion critica donde el analista mejora la calidad de un arbol por meritos propios (consistencia interna, completitud, precision).

---

#### T31. GT → GT — Revision Interna de la Norma

| Dimension | Detalle |
|-----------|---------|
| **Viabilidad** | **(d) Auto-transicion** — Frecuente en las fases tempranas |
| **Logic transition** | nec → nec |
| **relation_type** | `self_revision` (nuevo) |

**Por que (Desencadenante):** El analista revisa el GT y detecta problemas de estructura interna: NCs que son realmente CSFs disfrazadas, CSFs redundantes, relaciones de necesidad mal formuladas, o el Goal mismo es ambiguo. No hay input de otro arbol — es una revision critica del propio GT contra las CLRs de necesidad. El trigger es: "revisando el GT, algo no encaja internamente".

**Para que (Proposito):**
1. **Garantizar solidez de la norma antes de usarla**: un GT con NCs mal categorizadas produce un CRT con UDEs mal ancladas.
2. **Aplicar CLRs de necesidad rigurosamente**: cada relacion "para X necesito Y" debe pasar las pruebas de necesidad (CLR #1: claridad, CLR #4: suficiencia grupal).
3. **Iteracion natural del pensamiento**: el primer GT rara vez es definitivo. La auto-revision es parte del proceso normal.

**Escenario de negocio:** Al revisar el GT de logistica, el analista nota que NC-003 "Sistema de tracking GPS" no es una NC — es una *solucion* disfrazada de necesidad. La verdadera NC es "Visibilidad en tiempo real de la ubicacion del envio". GPS es una forma de conseguirlo, pero no la unica. Se reestructura el GT sin input de CRT ni EC — es correccion interna de calidad.

---

#### T32. CRT → CRT — Profundizacion del Diagnostico

| Dimension | Detalle |
|-----------|---------|
| **Viabilidad** | **(d) Auto-transicion** — Comun durante la construccion iterativa |
| **Logic transition** | suf → suf |
| **relation_type** | `extends` |

**Por que (Desencadenante):** El CRT se extiende por descubrimiento interno: al revisar las cadenas causales, el analista identifica UDEs adicionales que habia pasado por alto, INTs intermedios que faltan, o cadenas que se pueden profundizar hasta causas mas fundamentales. No es feedback de otro arbol — es rigor analitico sobre el propio diagnostico. El trigger es: "revisando el CRT, veo huecos en las cadenas causales".

**Para que (Proposito):**
1. **Completar cadenas causales**: un CRT con saltos logicos produce ECs sobre causas superficiales.
2. **Descubrir UDEs ocultas**: al profundizar cadenas, emergen efectos que el analista no habia verbalizado pero que el sistema sufre.
3. **Identificar la verdadera CRC**: profundizar puede revelar que la CRC aparente es en realidad un efecto intermedio de una causa mas profunda.

**Escenario de negocio:** El CRT de logistica tiene la cadena: "Rutas no optimizadas → Entregas tardias". Al profundizar, el analista pregunta: "Por que las rutas no estan optimizadas?" y descubre INT-015: "El planificador de rutas usa un mapa estatico de 2019" → INT-016: "Nadie actualiza el mapa porque no hay proceso definido" → RC-007: "Falta de gobernanza de datos maestros". La CRC real no era "rutas no optimizadas" sino "falta de gobernanza de datos". Este descubrimiento es interno al CRT.

---

#### T33. EC → EC — Reformulacion del Conflicto

| Dimension | Detalle |
|-----------|---------|
| **Viabilidad** | **(d) Auto-transicion** — Frecuente durante la generacion de supuestos |
| **Logic transition** | nec → nec |
| **relation_type** | `self_revision` |

**Por que (Desencadenante):** Al generar supuestos para los edges de la EC, el analista se da cuenta de que los nodos (A, B, C, D, D') no capturan bien el conflicto real. Las posiciones en tension (D, D') son sintomaticas, no fundamentales. O el objetivo comun (A) esta mal definido. No hay nuevo input del CRT ni del GT — es una reformulacion del propio conflicto por reflexion critica. El trigger es: "los supuestos no son interesantes porque el conflicto esta mal formulado".

**Para que (Proposito):**
1. **Conseguir supuestos invalidables**: una EC bien formulada produce supuestos fragiles. Si los supuestos son todos triviales o todos irrompibles, el conflicto necesita reformulacion.
2. **Capturar el conflicto real**: D y D' deben representar los verdaderos estados mutuamente excluyentes, no proxy de estados mas profundos.
3. **Iterar hasta precision**: Fedurko recomienda multiples iteraciones de la EC hasta que "se siente" que el conflicto esta bien capturado (Sombrero Rojo).

**Escenario de negocio:** La EC tiene D="Agrupar envios" vs D'="Rutas dedicadas". Al generar supuestos para D→B, el analista nota que todos son variaciones de "agrupar es mas barato" — triviales e irrompibles. El conflicto real no es *como* se hacen las rutas, sino *quien decide*: D="Planificacion centralizada por algoritmo" vs D'="Decision descentralizada por conductor experto". Al reformular, los supuestos se vuelven ricos e invalidables: "Asumimos que el conocimiento local del conductor es inferior al modelo estadistico".

---

#### T34. FRT → FRT — Iteracion sobre el Futuro

| Dimension | Detalle |
|-----------|---------|
| **Viabilidad** | **(d) Auto-transicion** — Comun durante el refinamiento |
| **Logic transition** | suf → suf |
| **relation_type** | `extends` |

**Por que (Desencadenante):** El FRT se refina internamente: DEs se ajustan para ser mas precisos, cadenas causales se extienden para capturar efectos de segundo orden de la INJ, o se incorporan INJs secundarias que complementan la principal. No es feedback de NBR, PRT ni CRT — es profundizacion del propio modelo del futuro. El trigger es: "el FRT es correcto pero incompleto — hay efectos de la INJ que no hemos modelado".

**Para que (Proposito):**
1. **Modelar efectos de segundo orden**: la INJ no solo elimina UDEs directamente — sus efectos se propagan. Un FRT maduro captura estas cascadas.
2. **Incorporar DEs no obvios**: a veces el efecto mas valioso de una INJ no es el directo sino uno emergente que no era el objetivo.
3. **Preparar para NBR mas preciso**: un FRT mas detallado produce un NBR mas completo porque hay mas cadenas que estresar.

**Escenario de negocio:** El FRT de "Optimizacion dinamica de rutas" tiene DE-001: "Entregas on-time >95%". Al iterar, el analista modela efectos de segundo orden: "Si entregas on-time >95%, entonces satisfaccion de cliente sube → recompra aumenta 20% → volumen de pedidos sube 15% → necesidad de mas conductores O mas eficiencia del optimizador". Este efecto cascada no era visible en la primera version del FRT y puede cambiar el analisis de viabilidad.

---

#### T35. NBR → NBR — Recursion de Trimming

| Dimension | Detalle |
|-----------|---------|
| **Viabilidad** | **(d) Auto-transicion** — Ocurre cuando la trimming INJ tiene sus propios riesgos |
| **Logic transition** | suf → suf |
| **relation_type** | `extends` |

**Por que (Desencadenante):** Una trimming INJ anadida al FRT para recortar un NBR genera ella misma una nueva rama negativa. La mitigacion tiene sus propios riesgos. El trigger es: "la solucion al riesgo genera un nuevo riesgo".

**Para que (Proposito):**
1. **Evaluar la mitigacion misma**: toda intervencion (incluidas las de mitigacion) puede tener efectos no deseados.
2. **Detectar cadenas de riesgo**: si cada trimming genera un nuevo NBR que necesita otra trimming, hay un problema estructural — la INJ principal es demasiado fragil.
3. **Criterio de parada**: si la severidad de los NBR sucesivos decrece en cada iteracion, la solucion converge. Si no decrece, escalar a T23 (NBR→EC) o T21 (NBR→GT).

**Escenario de negocio:** NBR-001: "Si optimizacion centralizada, entonces single point of failure". Trimming INJ: "Servidor redundante en otro datacenter". NBR-002 (sobre la trimming): "Si servidor redundante, Y SI la sincronizacion entre datacenters falla, entonces rutas divergentes por zona → conflictos de asignacion". Trimming INJ-2: "Protocolo de consenso con fallback a ultimo estado sincronizado valido". NBR-003 (sobre trimming-2): "Si consenso, Y SI latencia de red > 5s, entonces fallback demasiado frecuente → eficiencia reducida". La severidad decrece: NBR-001 (catastrofico) → NBR-002 (alto) → NBR-003 (bajo). La cadena converge — se acepta el riesgo residual de NBR-003.

**Regla practica:** Si despues de 2 niveles de recursion (NBR → trimming → NBR → trimming) la severidad no ha bajado al menos un nivel, tratar como señal de que la INJ principal o incluso la meta necesitan revision (T23 o T21).

---

#### T36. PRT/TT → PRT/TT — Revision del Plan

| Dimension | Detalle |
|-----------|---------|
| **Viabilidad** | **(d) Auto-transicion** — Frecuente durante la ejecucion |
| **Logic transition** | nec → nec (PRT→PRT) o suf → suf (TT→TT) |
| **relation_type** | `self_revision` |

**Por que (Desencadenante):** El PRT/TT se revisa internamente: obstaculos se reordenan al descubrir dependencias no previstas, IOs se refinan por mejor comprension de las soluciones intermedias, o acciones del TT se replanifican porque la secuencia original era suboptima. No es feedback de otro arbol — es aprendizaje durante la propia planificacion/ejecucion. El trigger es: "el plan necesita ajuste por lo que aprendemos al planificar/ejecutar".

**Para que (Proposito):**
1. **Optimizar la secuencia**: el primer PRT rara vez tiene la secuencia optima. Al detallar el TT, se descubren dependencias que obligan a reordenar.
2. **Incorporar aprendizaje tactico**: la ejecucion de los primeros pasos del TT genera informacion que mejora los pasos siguientes.
3. **Mantener la coherencia interna**: si un IO cambia, los pasos del TT que dependen de el deben ajustarse.

**Escenario de negocio:** El PRT tiene OBS-1→IO-1: "Evaluar proveedores" y OBS-2→IO-2: "Formar conductores". Originalmente eran paralelos. Al detallar el TT, se descubre que la formacion de conductores depende del proveedor elegido (cada sistema tiene interfaz diferente). Se reordena: IO-1 debe completarse antes de IO-2. El TT se reestructura: pasos 1-3 son evaluacion de proveedores, paso 4 es decision, pasos 5-8 son formacion con el sistema elegido. Este ajuste es interno al PRT/TT.

---

## 4. Sintesis: Patrones Emergentes

### 4.1. Clusters de transiciones por funcion

| Cluster | Transiciones | Funcion |
|---------|-------------|---------|
| **Flujo canonico** | T01, T06, T11, T29, T24, T16 | Progresion lineal GT→CRT→EC→FRT→NBR→trimming→PRT/TT |
| **Loops correctivos** | T05, T09, T10, T14, T22 | Retroalimentacion que mejora arboles anteriores |
| **Challenges** | T15, T19, T23 | Conflictos que obligan a buscar alternativas (incluyendo recursion NBR→EC) |
| **Revision post-implementacion** | T17, T18, T20, T30 | La implementacion cambia la comprension (incluyendo riesgos emergentes) |
| **Saltos acelerados** | T07, T08, T12, T28 | Atajos para situaciones con poco conflicto/riesgo (incluyendo filtro pre-FRT) |
| **Patrones estrategicos** | T02, T03, T04, T13, T21, T26, T27 | Uso avanzado: greenfield, benchmark, evolucion de meta, risk proactivo |
| **Absorcion operativa** | T25 | NBRs menores que se convierten en obstaculos de implementacion |
| **Auto-revision** | T31, T32, T33, T34, T35, T36 | Refinamiento interno sin cambio de herramienta |

### 4.2. Las 36 transiciones por utilidad practica

| Tier | Transiciones | Descripcion |
|------|-------------|-------------|
| **Tier 1 — Esenciales** | T01, T06, T11, T16, T29, T24 | Flujo canonico incluyendo NBR + trimming. Todo analisis LTP las usa. |
| **Tier 2 — Frecuentes** | T05, T10, T14, T15, T18, T20, T22, T23, T25, T31-T36 | Loops correctivos, challenges, auto-revision. Analisis serios las usan regularmente. |
| **Tier 3 — Situacionales** | T07, T09, T12, T13, T17, T19, T28, T30 | Patrones que se activan en contextos especificos. No en todo analisis. |
| **Tier 4 — Excepcionales** | T02, T03, T04, T08, T21, T26, T27 | Patrones avanzados: greenfield, emergencias, conflictos estructurales, risk proactivo sobre norma. |

### 4.3. Alternancia de logica en las transiciones

| Patron | Cantidad | Transiciones |
|--------|----------|-------------|
| suf → nec | 9 | T06, T13, T15, T21, T23, T05*, T17*, T19*, T25* |
| nec → suf | 6 | T01, T10, T11, T26, T28, T16* |
| suf → suf | 12 | T07, T14, T22, T24, T27, T29, T30*, T32, T34, T35, T18*, T20*, T08* |
| nec → nec | 9 | T02, T09, T31, T33, T36*, T03*, T04*, T12*, T17* |

*Depende de si el destino es PRT (nec) o TT (suf).

**Hallazgo**: las transiciones canonicas (Tier 1) alternan sistematicamente entre logicas. Los loops correctivos tienden a ser same-logic (suf→suf para CRT/FRT/NBR, nec→nec para GT/EC/PRT). Las auto-transiciones siempre preservan la logica (nec→nec o suf→suf), lo cual es logico: el refinamiento interno no cambia la perspectiva. Las transiciones NBR tienden a ser suf→suf (por su naturaleza de suficiencia), excepto cuando escalan a GT o EC (cambio a necesidad).

### 4.4. El meta-grafo como registro de navegacion

Las 36 transiciones demuestran que un analisis LTP real no es lineal — es un **grafo de navegacion** donde el analista va y viene entre arboles segun lo que descubre. El meta-grafo (RFC-002) captura esta navegacion como entidad persistente:

- Cada transicion es una **tree_relation** con `relation_type`, `handoff_nodes`, y `logic_transition` calculado.
- Los loops correctivos crean multiples tree_relations entre el mismo par de arboles (CRT↔FRT puede tener una relacion `injection` y dos `revision`).
- Los escenarios emergen naturalmente de las transiciones T15/T23 (challenges que generan INJs alternativas) y T03/T13 (benchmarks que comparan futuros).
- El NBR como herramienta independiente anade 10 transiciones que hacen explicito un flujo que antes estaba subsumido en "el FRT tiene NBRs".
- Las auto-transiciones (T31-T36) capturan el proceso iterativo de calidad que todo analista practica pero que rara vez se formaliza.

### 4.5. El NBR como amplificador de transiciones

El NBR es la herramienta con mayor densidad de retroalimentacion:

| Desde/hacia NBR | Tipo | Frecuencia |
|-----------------|------|------------|
| FRT → NBR (T29) | Estandar | Siempre. Todo FRT tiene NBR. |
| NBR → FRT (T24) | Trimming | Muy frecuente. La mayoria de NBRs se recortan. |
| NBR → EC (T23) | Challenge | Frecuente en analisis serios (~20% de NBRs escalan). |
| NBR → CRT (T22) | Revision | Moderado. Algunos NBRs revelan realidad oculta. |
| NBR → NBR (T35) | Recursion | Moderado. Trimmings con sus propios riesgos. |
| NBR → PRT/TT (T25) | Absorcion | Frecuente. NBRs menores se gestionan operativamente. |
| NBR → GT (T21) | Revision meta | Raro. Solo cuando los riesgos cuestionan la meta. |
| EC → NBR (T28) | Filtro | Situacional. Util con multiples INJs candidatas. |
| CRT → NBR (T27) | Anticipacion | Raro. Pre-evaluacion hipotetica. |
| GT → NBR (T26) | Proactivo | Muy raro. Sanity check de la norma. |
| PRT/TT → NBR (T30) | Emergente | Moderado. La implementacion revela riesgos nuevos. |

**Hallazgo**: el NBR actua como **amplificador de feedback**. Un FRT que parece completo revela, tras NBR, multiples lineas de retroalimentacion a CRT, EC, GT y PRT. Sin modelo explicito de NBR como herramienta, estas retroalimentaciones quedan implicitas y no trazables.

### 4.6. Auto-transiciones como indicador de madurez

| Auto-transicion | Señal de | Cuando preocuparse |
|-----------------|----------|-------------------|
| GT → GT (T31) | Rigor en la definicion de la norma | Si ocurre >3 veces sin convergencia: la meta puede ser inestable |
| CRT → CRT (T32) | Profundidad del diagnostico | Si la CRC cambia >2 veces: el scope puede estar mal definido |
| EC → EC (T33) | Precision del conflicto | Si D/D' cambian >2 veces: volver al CRT (T10) |
| FRT → FRT (T34) | Completitud del modelo futuro | Normal. Multiples iteraciones son senal de rigor |
| NBR → NBR (T35) | Robustez de la mitigacion | Si la severidad no decrece: escalar a T23 o T21 |
| PRT/TT → PRT/TT (T36) | Aprendizaje durante la ejecucion | Normal. El plan siempre se refina al ejecutar |

**Hallazgo**: las auto-transiciones son saludables si convergen (cada iteracion produce cambios menores). Si no convergen, son señal de un problema mas profundo que requiere cambiar de herramienta (transicion inter-arbol).

---

## 5. Mapping a los `relation_type` del RFC-002

### Cobertura actual (actualizada con 36 transiciones)

| relation_type (RFC-002 S3.2) | Transiciones cubiertas | Tier |
|------------------------------|----------------------|------|
| `gap_analysis` | T01 (GT→CRT) | 1 |
| `core_conflict` | T06 (CRT→EC), T02 (GT→EC, atipico) | 1, 4 |
| `injection` | T11 (EC→FRT), T07 (CRT→FRT, atipico) | 1, 3 |
| `risk_check` | T29 (FRT→NBR), T26 (GT→NBR), T27 (CRT→NBR), T28 (EC→NBR), T30 (PRT/TT→NBR) | 1, 3, 4 |
| `prerequisite` | T16 (FRT→PRT), T25 (NBR→PRT/TT, atipico) | 1, 2 |
| `tactical` | T16 (PRT→TT), T12 (EC→PRT/TT), T08 (CRT→PRT/TT) | 1, 3, 4 |
| `alternative` | (derivado de T15/T23 cuando hay INJ alternativa) | 2 |
| `revision` | T05, T09, T10, T13, T14, T17, T18, T20, T21, T22 | 2, 3 |
| `challenges` | T15 (FRT→EC), T19 (PRT/TT→EC), T23 (NBR→EC) | 2, 3 |
| `extends` | T32 (CRT→CRT), T34 (FRT→FRT), T35 (NBR→NBR) | 2 |

### Transiciones que requieren `relation_type` nuevos

| Transicion | relation_type propuesto | Notas |
|-----------|------------------------|-------|
| T03 (GT→FRT) | `benchmark` | Evaluacion proactiva sin pasar por diagnostico. |
| T04 (GT→PRT/TT) | `strategic_roadmap` | Greenfield donde NCs→obstaculos directamente. |
| T08 (CRT→PRT/TT) | `emergency_action` | Accion de contencion urgente. Marcar como temporal. |
| T24 (NBR→FRT) | `trimming` | Recorte de rama negativa. Distinto de `revision` porque no cambia la estructura del FRT sino que anade una mitigacion puntual. |
| T31, T33, T36 | `self_revision` | Reestructuracion interna de un arbol (GT, EC, PRT/TT). Distinto de `extends` (que anade) — `self_revision` reorganiza o reformula. |

### Recomendacion

Agregar 5 `relation_type` nuevos al RFC-002:

```
| relation_type     | Descripcion                                              | Transiciones       |
|-------------------|----------------------------------------------------------|--------------------|
| benchmark         | Evaluacion proactiva de una iniciativa contra la norma   | GT → FRT           |
| strategic_roadmap | Planificacion greenfield desde la norma                  | GT → PRT/TT        |
| emergency_action  | Contencion urgente antes de completar el analisis        | CRT → PRT/TT       |
| trimming          | Recorte de rama negativa del FRT via INJ de mitigacion   | NBR → FRT          |
| self_revision     | Reestructuracion interna sin cambio de herramienta       | X → X (diagonal)   |
```

`benchmark`, `strategic_roadmap` y `emergency_action` son Tier 4 (excepcionales). `trimming` es Tier 1 (esencial — todo analisis con NBR lo usa). `self_revision` es Tier 2 (frecuente en analisis serios).

---

## 6. Implicaciones para ltp-engine

### Validacion

`ltp validate` puede emitir warnings cuando detecte transiciones inusuales:

| Warning | Trigger | Severidad |
|---------|---------|-----------|
| `CANONICAL_SKIP_EC` | Existe relacion CRT→FRT sin EC intermedia (T07) | MEDIA |
| `CANONICAL_SKIP_FRT` | Existe relacion EC→PRT/TT sin FRT intermedio (T12) | BAJA |
| `CANONICAL_SKIP_NBR` | Existe relacion FRT→PRT/TT sin NBR intermedio (T29 ausente) | MEDIA |
| `EMERGENCY_ACTION_OPEN` | Relacion `emergency_action` sin flujo completo paralelo | MEDIA |
| `BENCHMARK_WITHOUT_DIAGNOSIS` | Relacion `benchmark` GT→FRT sin CRT existente | INFO |
| `MULTIPLE_REVISIONS` | >2 relaciones `revision` entre el mismo par de arboles | INFO |
| `TRIMMING_RECURSION_DEPTH` | >2 niveles de NBR→NBR (T35) sin disminucion de severidad | MEDIA |
| `SELF_REVISION_LOOP` | >3 auto-transiciones del mismo arbol sin convergencia | INFO |
| `NBR_ESCALATION_MISSING` | NBR con severidad "critica" sin transicion a EC (T23) ni a GT (T21) | MEDIA |

### Narrativa

`trace --narrative` puede usar esta taxonomia para generar narrativas enriquecidas:

```
[FLUJO CANONICO] GT "Logistica competitiva" → CRT "Diagnostico Q3" (gap_analysis)
  Transicion logica: necesidad → suficiencia
  Handoff: 5 NCs no cumplidas → 5 UDEs

[FLUJO CANONICO] FRT "Futuro con optimizacion" → NBR "Riesgos de optimizacion" (risk_check)
  Transicion logica: suficiencia → suficiencia
  Hallazgos: 3 UDEs colaterales (1 critica, 2 menores)

[TRIMMING] NBR "Riesgos de optimizacion" → FRT "Futuro con optimizacion" (trimming)
  Transicion logica: suficiencia → suficiencia
  Trimming INJ: "Modo fallback con rutas estaticas"

[CHALLENGE] NBR "Riesgos de optimizacion" → EC "Conflicto lock-in" (challenges)
  Transicion logica: suficiencia → necesidad
  Trigger: UDE colateral critica no recortable — escalada a EC hija

[LOOP CORRECTIVO] NBR "Riesgos de optimizacion" → CRT "Diagnostico Q3" (revision)
  Transicion logica: suficiencia → suficiencia
  Trigger: NBR revelo cultura de autonomia no diagnosticada

[AUTO-REVISION] EC "Conflicto entregas" → EC "Conflicto entregas" (self_revision)
  Transicion logica: necesidad → necesidad (same_nec)
  Trigger: supuestos triviales — conflicto reformulado
```

---

## 7. Diagrama de Flujo Canonico Expandido

```
GT ──(T01)──→ CRT ──(T06)──→ EC ──(T11)──→ FRT ──(T29)──→ NBR ──(T24)──→ FRT' ──(T16)──→ PRT ──→ TT
 ↑               ↑               ↑              ↑              │              │               │
 │               │               │              │              ├──(T23)──→ EC₂ (recursion)    │
 │               │               │              │              ├──(T22)──→ CRT (revision)     │
 │               │               │              │              ├──(T25)──→ PRT (absorcion)    │
 │               │               │              │              └──(T21)──→ GT  (raro)         │
 │               │               │              │                                             │
 │               │               │              ├──────────(T15)──→ EC  (challenge)           │
 │               │               │              └──────────(T14)──→ CRT (revision)            │
 │               │               │                                                            │
 │               ├──(T10)──← EC  │                                                            │
 ├──(T05)──← CRT │              │                                                            │
 ├──(T09)──← EC  │              │                                                            │
 ├──(T13)──← FRT │              │                                                            │
 ├──(T17)──← PRT/TT             │                                                            │
 │                               ├──(T18)──← PRT/TT                                          │
 │                               ├──(T19)──← PRT/TT → EC (challenge)                         │
 │                               └──(T20)──← PRT/TT → FRT (revision)                         │
 │
 └──── Auto-transiciones (T31-T36): cada arbol puede revisarse a si mismo ────┘
```

> El flujo canonico expandido incluye NBR como paso obligatorio entre FRT y PRT/TT. Las flechas secundarias muestran las retroalimentaciones mas frecuentes.

---

## Changelog

| Fecha | Cambio |
|-------|--------|
| 2026-09-12 | Expansion a 36 transiciones: GRUPO F (T21-T30, NBR como 6.ª herramienta), GRUPO G (T31-T36, auto-transiciones). Matriz actualizada de 5×5 a 6×6. Sintesis ampliada con secciones 4.5 (NBR como amplificador) y 4.6 (auto-transiciones como indicador de madurez). Mapping actualizado con 5 nuevos relation_types (benchmark, strategic_roadmap, emergency_action, trimming, self_revision). Validacion ampliada con 4 nuevos warnings. Seccion 7 con diagrama de flujo canonico expandido. |
| 2026-09-11 | Creacion: auditoria completa de 20 transiciones, matriz 5×5, mapping a relation_types, implicaciones para el motor |
