# RFC-002 Apendice: Casos de Uso por Transicion

| Campo       | Valor                              |
|-------------|------------------------------------|
| Status      | **EXPLORING**                      |
| Author      | Javier Asensio                     |
| Created     | 2026-09-12                         |
| Parent      | RFC-002 (Meta-Grafo)               |
| Scope       | 108 casos de uso (3 por transicion) para las 36 transiciones LTP |

---

## GRUPO A: Transiciones desde el GT (Goal Tree)

### T01. GT → CRT — Gap Analysis

1. **Hospital**: GT define "Mortalidad quirurgica <1%". CRT revela UDE: "Mortalidad actual 3.2% en cirugia cardiaca" — la NC de "equipo de anestesia certificado" no se cumple.
2. **Startup SaaS**: GT pide "Uptime 99.9%". CRT identifica UDE: "47 incidentes de caida en Q2" — NC de "redundancia multi-zona" inexistente.
3. **Universidad**: GT exige "Empleabilidad de graduados >85% en 6 meses". CRT muestra UDE: "62% a los 12 meses" — NC de "practicas obligatorias en empresa" no implementada.

### T02. GT → EC — Validacion Directa de Conflicto

1. **Banco de inversion**: GT tiene CSF "Captacion agresiva de talento quant" y CSF "Cultura de compliance estricta". Al formular NCs, descubre que NC "Bonus discrecionales sin techo para traders top" (captacion) es incompatible con NC "Aprobacion previa de toda operacion por compliance" (cultura). El conflicto no es la tension generica rentabilidad/riesgo — es que el modelo de incentivos para atraer talento socava activamente el modelo de control que la regulacion post-2008 exige. EC directa sobre el GT.
2. **ONG**: GT tiene CSF "Maximizar impacto social" y CSF "Sostenibilidad financiera". Las NCs de cada una compiten por los mismos recursos limitados.
3. **Escuela publica**: GT tiene CSF "Atencion personalizada" y CSF "Ratio alumno/profesor eficiente". Las NCs se contradicen con el presupuesto fijo.

### T03. GT → FRT — Benchmark de Futuro

1. **Retail**: GT definido. El CEO propone "entrar en e-commerce". FRT de benchmark simula impacto de la tienda online contra CSFs del GT antes de diagnosticar problemas actuales.
2. **Fabrica**: GT operativo estable. Un proveedor ofrece robotizacion. FRT de benchmark evalua: "Si robots, entonces +40% throughput" — se compara contra CSFs para decidir si merece investigar mas.
3. **Consultora**: GT definido. Surge IA generativa. FRT de benchmark: "Si adoptamos IA para propuestas, entonces -60% tiempo de redaccion" — evalua contra CSF de calidad y diferenciacion.

### T04. GT → PRT/TT — Roadmap Estrategico

1. **Startup greenfield**: nueva linea de negocio de delivery de farmacia. No hay operacion actual. NCs del GT → obstaculos directos (no hay flota, no hay licencia farmaceutica, no hay app).
2. **Gobierno**: nuevo programa de digitalizacion rural. No hay infraestructura previa. NCs (conectividad, formacion, equipamiento) → PRT de construccion desde cero.
3. **Spin-off corporativo**: division nueva de una empresa. GT heredado del grupo pero sin equipo ni procesos. NCs → PRT de edificacion.

---

## GRUPO B: Transiciones desde el CRT (Current Reality Tree)

### T05. CRT → GT — Feedback a la Norma

1. **Fabrica**: CRT identifica UDE "Los operarios sufren lesiones musculoesqueleticas". Ningun CSF del GT cubre ergonomia. Se anade CSF "Seguridad y bienestar del operario".
2. **Software**: CRT revela UDE "Los deploys rompen funcionalidad existente". El GT solo tiene CSFs de velocidad de entrega. Falta CSF "Estabilidad de releases".
3. **Restaurante**: CRT muestra UDE "Rotacion de personal del 80% anual". El GT solo mide satisfaccion del cliente. Falta CSF "Retencion de talento".

### T06. CRT → EC — Core Conflict

1. **Hospital**: CRC = "Los medicos priorizan volumen de pacientes sobre tiempo por consulta". EC: A="Atencion medica de calidad", B="Sostenibilidad financiera" (→D: "Consultas de 10 min"), C="Diagnostico preciso" (→D': "Consultas de 30+ min").
2. **Desarrollo de software**: CRC = "Se prioriza features nuevas sobre deuda tecnica". EC: A="Producto competitivo", B="Time to market" (→D: "Solo features"), C="Mantenibilidad" (→D': "Refactoring continuo").
3. **Call center**: CRC = "Politica de AHT (Average Handle Time) minimo". EC: A="Servicio eficiente", B="Coste por llamada bajo" (→D: "Limitar a 3 min"), C="Resolucion en primera llamada" (→D': "El tiempo que haga falta").

### T07. CRT → FRT — Salto Directo (sin EC)

1. **Compliance**: CRC = "No cumplimos GDPR". La solucion es impuesta por ley — no hay dilema que evaporar. FRT directo: "Si implementamos consentimiento explicito + derecho al olvido, entonces cumplimiento".
2. **Infraestructura**: CRC = "El servidor tiene 12 anos y no hay repuestos". No hay conflicto: hay que migrar. FRT directo evalua impacto de la migracion.
3. **Transporte**: CRC = "Los vehiculos no cumplen norma de emisiones Euro 7". Regulacion impuesta. FRT simula renovacion de flota.

### T08. CRT → PRT/TT — Accion de Emergencia

1. **Ciberseguridad**: CRT revela UDE "Datos de clientes expuestos por vulnerabilidad critica". TT de emergencia: parchear, notificar, rotar credenciales. En paralelo, flujo completo para solucion estructural.
2. **Alimentacion**: CRT identifica UDE "Lote contaminado en distribucion". TT inmediato: recall del producto, notificacion a sanidad, comunicado publico. El analisis de causa raiz sigue en paralelo.
3. **Construccion**: CRT detecta UDE "Cimentacion con defecto estructural en edificio habitado". Evacuacion y refuerzo temporal mientras se analiza la causa raiz.

---

## GRUPO C: Transiciones desde la EC (Evaporating Cloud)

### T09. EC → GT — Redefinicion de la Meta

1. **Banco**: EC revela supuesto invalidado "Asumimos que rentabilidad = margen por operacion". El mercado ha pivotado a modelo de suscripcion. El Goal del GT necesita reformularse.
2. **Educacion**: EC invalida "Asumimos que empleabilidad = empleo por cuenta ajena". Muchos graduados emprenden. CSF del GT debe incluir "autoempleo viable".
3. **Energia**: EC invalida "Asumimos que eficiencia energetica = reducir consumo". La meta deberia ser "optimizar coste energetico total" (incluye generacion propia).

### T10. EC → CRT — Revision Causal

1. **Hospital**: EC sobre tiempos de consulta. Supuesto: "Los medicos no tienen tiempo porque hay demasiados pacientes". Al investigar, resulta que el 40% del tiempo se pierde en burocracia administrativa no capturada en el CRT.
2. **E-commerce**: EC sobre conversion. Supuesto: "El precio es el factor decisivo". Resulta que el CRT no capturaba que el 60% de abandonos es por experiencia de pago, no por precio.
3. **Manufactura**: EC sobre calidad vs velocidad. Al investigar supuestos, se descubre que la CRC no era la velocidad de linea sino la calibracion de maquinas — el CRT necesita extension.

### T11. EC → FRT — Injection

1. **Call center**: EC invalida "Asumimos que resolver rapido = limitar tiempo". INJ: "Sistema de categorizacion automatica que pre-diagnostica y enruta al especialista". FRT simula: "-40% AHT y +25% resolucion en primera llamada".
2. **Software**: EC invalida "Asumimos que hay que elegir entre features y refactoring". INJ: "Presupuesto tecnico del 20% integrado en cada sprint". FRT simula impacto en velocity y deuda tecnica.
3. **Hospital**: EC invalida "Asumimos que la burocracia es necesaria para auditoria". INJ: "Transcripcion automatica de consultas con IA". FRT simula: "+15 min por consulta liberados".

### T12. EC → PRT/TT — Implementacion Directa

1. **Oficina**: EC produce INJ secundaria "Eliminar la aprobacion del director para compras <100 euros". Cambio de politica trivial → TT directo de 3 pasos.
2. **Fabrica**: INJ menor "Cambiar turno de limpieza de manana a noche". Sin riesgo, sin conflicto. PRT/TT directo.
3. **Software**: EC sobre "velocidad de review vs calidad de review" invalida supuesto "Asumimos que todo PR necesita aprobacion senior". INJ menor: "PRs que solo tocan tests o docs se auto-aprueban con CI verde". Sin riesgo, sin efectos colaterales plausibles. TT directo: actualizar CODEOWNERS, comunicar al equipo, monitorizar 2 sprints.

---

## GRUPO D: Transiciones desde el FRT (Future Reality Tree)

### T13. FRT → GT — Revision de la Meta Post-Simulacion

1. **Retail**: FRT muestra que la INJ de e-commerce no solo cubre ventas sino que habilita un marketplace B2B no contemplado. GT se amplia con nuevo CSF.
2. **Hospital**: FRT revela que la IA de transcripcion tambien puede detectar patrones de diagnostico. GT evoluciona de "eficiencia administrativa" a "asistencia clinica inteligente".
3. **Fabrica**: FRT muestra que el maximo alcanzable con la INJ es 88% de eficiencia, no el 95% del GT. La NC debe ajustarse a la realidad.

### T14. FRT → CRT — Revision de la Realidad

1. **Software**: FRT de "CI/CD automatizado" revela: "Si deploys automaticos, y si no hay ownership de microservicios, entonces nadie sabe quien arregla un deploy roto". El CRT no capturaba la falta de ownership.
2. **Retail**: FRT de "tienda 24h automatizada" revela dependencia de mantenimiento nocturno no diagnosticada en el CRT.
3. **Banco**: FRT de "onboarding digital" descubre que el 30% de clientes no tiene smartphone compatible — realidad no capturada.

### T15. FRT → EC — Challenges (Nuevo Conflicto)

1. **Energia**: FRT de "solar en tejados" genera NBR catastrofico: dependencia total de clima. Trimming inviable. Nueva EC: "Generacion propia" vs "Estabilidad de suministro".
2. **Hospital**: FRT de "IA diagnostica" genera NBR: responsabilidad legal difusa. Se necesita nueva EC sobre el conflicto regulatorio.
3. **Software**: FRT de "migracion a microservicios" genera NBR de complejidad operacional insostenible. Nueva EC: monolito modular vs microservicios.

### T16. FRT → PRT/TT — Prerequisite/Tactical

1. **Hospital**: INJ validada "IA de transcripcion". PRT: OBS "No hay integracion con HIS" → IO "API de integracion". TT detalla pasos de contratacion, desarrollo y piloto.
2. **E-commerce**: INJ "Checkout en 1 click". PRT: OBS "Procesador de pagos no soporta tokenizacion". IO "Migrar a Stripe". TT secuencia la migracion.
3. **Fabrica**: INJ "Mantenimiento predictivo con IoT". PRT: OBS "No hay sensores instalados". IO "Proyecto de sensorializacion". TT planifica por linea de produccion.

---

## GRUPO E: Transiciones desde PRT/TT

### T17. PRT/TT → GT — Revision Estrategica Post-Implementacion

1. **Software**: al implementar CI/CD, se descubre que el proveedor cloud ofrece ML ops. El GT puede evolucionar para incluir CSF de "Inteligencia de producto basada en datos".
2. **Fabrica**: al implementar IoT, se descubre que los datos de sensores permiten predecir demanda. GT evoluciona de "eficiencia operativa" a "supply chain inteligente".
3. **Retail**: al implementar e-commerce, se accede a datos de comportamiento de cliente que cambian lo que es posible. GT se amplia.

### T18. PRT/TT → CRT — La Realidad Ha Cambiado

1. **Hospital**: tras implementar IA de transcripcion, CRT post muestra UDE eliminada (burocracia) pero UDE nueva: "Los medicos no revisan las transcripciones y se acumulan errores".
2. **Software**: tras CI/CD, UDEs de deploys lentos eliminadas pero UDE nueva: "Alert fatigue por exceso de notificaciones del pipeline".
3. **Banco**: tras onboarding digital, UDE de tramites eliminada pero UDE nueva: "Fraude de identidad en apertura remota de cuentas".

### T19. PRT/TT → EC — Nuevo Conflicto de Implementacion

1. **Hospital**: PRT tiene IO "Contratar proveedor de IA" pero IT exige "solucion on-premise por datos de salud" y finanzas exige "SaaS por coste". EC de implementacion.
2. **Fabrica**: TT requiere parar linea 3 semanas para instalar sensores, pero comercial tiene pedidos comprometidos. Conflicto de implementacion → EC.
3. **Gobierno**: PRT tiene IO "Formar a 500 funcionarios" pero sindicato exige "formacion en horario laboral" y presupuesto solo cubre "formacion fuera de horario". EC.

### T20. PRT/TT → FRT — Feedback al Futuro

1. **E-commerce**: PRT descubre que la pasarela de pago cobra 3.5% (no 1.5% asumido). El DE del FRT "margen +5%" se reduce a "+3%". Hay que recalcular viabilidad.
2. **Hospital**: TT revela que la integracion con HIS requiere 8 meses (no 3 asumidos). El timeline del FRT cambia y algun DE puede no materializarse a tiempo.
3. **Fabrica**: PRT descubre que los sensores necesitan cableado nuevo (no wireless como asumia el FRT). Coste sube y el DE de "ROI en 18 meses" pasa a 30 meses.

---

## GRUPO F: Transiciones desde/hacia el NBR (Negative Branch Reservation)

### T21. NBR → GT — Revision de la Meta por Riesgo (Estructural)

1. **Hospital**: toda INJ de "IA diagnostica" genera NBRs que violan NC "Relacion medico-paciente de confianza". La meta debe incorporar CSF de "Factor humano en diagnostico".
2. **Banco**: INJ de "automatizacion total de creditos" genera NBR que viola NC "Inclusion financiera" (algoritmo sesga contra perfiles atipicos). GT debe ampliarse.
3. **Educacion**: INJ de "clases 100% online" genera NBRs que violan NC "Socializacion del alumnado" y NC "Deteccion temprana de problemas emocionales". GT debe reformularse.

### T22. NBR → CRT — Descubrimiento de Realidad Oculta

1. **Software**: NBR de "microservicios" revela: "Si cada equipo elige su stack, Y SI no hay estandares, entonces babelizacion tecnologica". El CRT no capturaba que ya hay 4 lenguajes en produccion sin gobernanza.
2. **Fabrica**: NBR de "automatizacion" revela: "Si robots, Y SI los operarios sabotean por miedo al despido, entonces averias sospechosas". CRT no capturaba clima laboral.
3. **Banco**: NBR de "onboarding digital" revela dependencia oculta de un proveedor de verificacion de identidad con SLA dudoso. CRT no lo tenia.

### T23. NBR → EC — Nuevo Conflicto por Efecto Colateral

1. **Energia**: NBR de "planta solar" genera conflicto: A="Energia sostenible", B="Autonomia energetica" (→D: "Maxima capacidad solar"), C="Estabilidad de red" (→D': "Generacion base controlable"). EC hija con supuestos propios.
2. **Hospital**: NBR de "IA" genera conflicto: A="Diagnostico optimo", B="Eficiencia" (→D: "IA decide"), C="Responsabilidad legal" (→D': "Medico decide"). La trimming no resuelve el dilema legal.
3. **Gobierno**: NBR de "digitalizacion" genera conflicto: A="Servicio accesible", B="Eficiencia" (→D: "Solo digital"), C="Equidad" (→D': "Mantener presencial para brecha digital").

### T24. NBR → FRT — Trimming

1. **E-commerce**: NBR "Si envio gratuito, Y SI devoluciones masivas, entonces coste insostenible". Trimming: "Envio gratuito solo a partir de 30 euros + politica de devoluciones con coste parcial".
2. **Software**: NBR "Si microservicios, Y SI falla el service mesh, entonces cascada de errores". Trimming: "Circuit breakers + fallback a modo degradado por servicio".
3. **Hospital**: NBR "Si IA transcribe, Y SI error en medicacion, entonces evento adverso". Trimming: "Doble validacion obligatoria en prescripciones generadas por IA".

### T25. NBR → PRT/TT — Obstaculos Derivados de Riesgos

1. **Fabrica**: NBR menor "Si IoT, Y SI los operarios no calibran sensores, entonces lecturas erraticas los primeros meses". No catastrofico → OBS en PRT: "Formacion de calibracion". Paso en TT: "Semana 1-2: taller practico".
2. **Software**: NBR menor "Si CI/CD automatico, Y SI los developers no estan acostumbrados a trunk-based development, entonces PRs enormes que bloquean el pipeline durante horas". No catastrofico → OBS en PRT: "Equipo sin cultura de commits pequenos". IO: "Taller de trunk-based + regla de PR <200 lineas antes de activar CD".
3. **Retail**: NBR menor "Si tienda 24h, Y SI no hay vigilancia nocturna, entonces hurto incremental". → Paso en TT: "Instalar camaras con IA de deteccion antes de abrir horario nocturno".

### T26. GT → NBR — Evaluacion de Riesgos de la Norma

1. **Startup**: GT bien formado con Goal "Plataforma de telemedicina lider en Latam", CSFs incluyen escalabilidad, compliance y calidad clinica. NBR proactivo: "Si logramos ser lideres, Y SI cada pais tiene regulacion de telemedicina diferente, entonces un cambio regulatorio en un mercado clave puede invalidar el modelo en 3 meses". Revela que el GT necesita un CSF de "Resiliencia regulatoria multi-jurisdiccional" que no era obvio.
2. **Farmaceutica**: GT tiene Goal "Lanzar farmaco en 18 meses". NBR proactivo: "Si aceleramos aprobacion, Y SI efectos secundarios no detectados, entonces retiro de mercado + litigios".
3. **Ciudad**: GT tiene Goal "Zona peatonal en el centro". NBR proactivo: "Si cerramos calles, Y SI comerciantes pierden acceso de carga, entonces exodo comercial".

### T27. CRT → NBR — Anticipacion de Riesgos Pre-Solucion

1. **Hospital**: CRC = "Falta de personal de enfermeria". Cualquier solucion implicara redistribucion de carga. Pre-NBR: "Si redistribuimos (cualquier forma), Y SI los equipos pierden cohesion, entonces calidad asistencial cae".
2. **Software**: CRC = "Arquitectura monolitica". Cualquier intervencion toca codigo legacy. Pre-NBR: "Si tocamos el monolito (cualquier forma), Y SI no hay tests, entonces regresiones".
3. **Retail**: CRC = "Dependencia de un unico proveedor". Cualquier diversificacion implica negociacion. Pre-NBR: "Si diversificamos, Y SI el proveedor actual retalia, entonces desabastecimiento temporal".

### T28. EC → NBR — Filtro Pre-FRT

1. **Fabrica**: EC produce 3 INJs. NBR rapido: INJ-A "robotizar" (riesgo alto: despidos masivos), INJ-B "cobots" (riesgo bajo: convivencia humano-maquina), INJ-C "outsourcing" (riesgo alto: perdida de know-how). Se prioriza INJ-B para FRT.
2. **Software**: EC produce INJ-A "rewrite total", INJ-B "strangler fig pattern", INJ-C "big bang migration". NBR rapido descarta INJ-A y INJ-C por riesgo. FRT solo para INJ-B.
3. **Banco**: EC produce INJ-A "IA propia", INJ-B "IA de tercero", INJ-C "hibrido". NBR rapido: INJ-A riesgo regulatorio, INJ-B riesgo de dependencia, INJ-C riesgo moderado → FRT para INJ-C e INJ-B.

### T29. FRT → NBR — Verificacion de Ramas Negativas (Estandar)

1. **E-commerce**: FRT de "checkout en 1 click". NBR sistematico: "Y SI fraude con tarjetas robadas → compras no autorizadas masivas"; "Y SI error en cantidad → pedidos duplicados no detectados".
2. **Gobierno**: FRT de "tramites online". NBR: "Y SI suplantacion de identidad → acceso a datos sensibles"; "Y SI caida del sistema en plazo legal → ciudadanos sin poder tramitar".
3. **Educacion**: FRT de "evaluacion por IA". NBR: "Y SI sesgo en el modelo → discriminacion sistematica"; "Y SI estudiantes hackean el sistema → integridad academica comprometida".

### T30. PRT/TT → NBR — Riesgos de Implementacion

1. **Hospital**: TT paso "Migrar historiales al nuevo HIS". NBR de implementacion: "Si migramos en vivo, Y SI hay incompatibilidades de formato entre sistemas, entonces perdida de historiales clinicos criticos".
2. **Software**: TT paso "Cortar trafico al monolito". NBR: "Si cortamos de golpe, Y SI el nuevo servicio tiene bug no detectado, entonces downtime en produccion".
3. **Banco**: PRT IO "Migrar cuentas al nuevo core bancario". NBR: "Si migramos por lotes, Y SI un lote falla, entonces saldos inconsistentes durante horas".

---

## GRUPO G: Auto-Transiciones (Diagonal)

### T31. GT → GT — Revision Interna de la Norma

1. **Hospital**: NC "Sistema de citas electronico" es una solucion disfrazada. La NC real es "Acceso agil a citas" (el sistema es UNA forma).
2. **Startup**: CSF "Equipo de 50 ingenieros" confunde medio con fin. CSF real: "Capacidad de desarrollo para 3 lanzamientos/trimestre".
3. **Universidad**: Goal "Ser Top-50 en ranking QS" es una metrica, no una meta. Goal real: "Excelencia investigadora con impacto social".

### T32. CRT → CRT — Profundizacion del Diagnostico

1. **Software**: CRT tiene "Deploys lentos". Se profundiza: por que? → "Tests tardan 45 min" → por que? → "Tests end-to-end redundantes + sin paralelizacion" → RC real mas profunda.
2. **Retail**: CRT tiene "Clientes no vuelven". Se profundiza: → "Experiencia post-venta mala" → "No hay follow-up" → "CRM sin datos de contacto actualizados" → RC real.
3. **Hospital**: CRT tiene "Listas de espera largas". → "Quirofanos infrautilizados por las tardes" → "No hay anestesistas disponibles en turno de tarde" → RC real.

### T33. EC → EC — Reformulacion del Conflicto

1. **Software**: EC tiene D="Rewrite" vs D'="Mantener monolito". Supuestos triviales. Reformulacion: D="Autonomia total de equipos" vs D'="Coherencia arquitectonica global". Ahora los supuestos son ricos.
2. **Hospital**: EC tiene D="Mas consultas" vs D'="Consultas mas largas". Reformulacion: D="El medico como gestor de volumen" vs D'="El medico como investigador de cada caso".
3. **Educacion**: EC tiene D="Clases presenciales" vs D'="Clases online". Reformulacion: D="Aprendizaje guiado sincrono" vs D'="Aprendizaje autonomo asincrono". Supuestos mas invalidables.

### T34. FRT → FRT — Iteracion sobre el Futuro

1. **E-commerce**: FRT modela DE "Conversion +15%". Efecto de segundo orden: +15% conversion → +30% pedidos → almacen actual insuficiente → necesidad de expansion o 3PL. No visible en primera iteracion.
2. **Hospital**: FRT modela DE "Consultas +20 min". Segundo orden: medicos mas satisfechos → rotacion baja → equipo estable → mejor formacion de residentes → calidad sube aun mas.
3. **Software**: FRT modela DE "Deploys 10x/dia". Segundo orden: mas deploys → mas datos de A/B testing → decisiones basadas en datos → producto evoluciona mas rapido → ventaja competitiva.

### T35. NBR → NBR — Recursion de Trimming

1. **Banco**: NBR "IA sesga creditos". Trimming: "Auditoria algoritmica trimestral". NBR2: "Y SI el auditor no entiende el modelo" → Trimming2: "Explicabilidad obligatoria (SHAP/LIME)". NBR3: severidad baja. Converge.
2. **Fabrica**: NBR "Robot causa lesion". Trimming: "Zona de exclusion con sensores". NBR2: "Y SI sensor falla" → Trimming2: "Doble sensor + parada automatica". NBR3: residual. Converge.
3. **Hospital**: NBR "IA da diagnostico erroneo". Trimming: "Validacion medica obligatoria". NBR2: "Y SI medico confia ciegamente en IA (automation bias)" → Trimming2: "IA presenta 3 opciones sin ranking". NBR3: residual. Converge.

### T36. PRT/TT → PRT/TT — Revision del Plan

1. **Software**: TT tenia "migrar DB" y "migrar servicio" en paralelo. Al detallar, se descubre que el servicio depende del nuevo schema. Se reordena: DB primero, servicio despues.
2. **Hospital**: PRT tenia IO "Comprar sistema" e IO "Formar medicos" en paralelo. Al detallar TT, la formacion depende del sistema elegido. Se secuencia.
3. **Retail**: TT tenia "lanzar web" y "campana de marketing" simultaneos. Al ejecutar, la web necesita 2 semanas mas. Se replanifica la campana.


---


## Flujos Detallados por Transicion

Cada flujo muestra la secuencia de operaciones MCP que el usuario (o el agente LLM) ejecuta para materializar la transicion. Los comandos marcados `[RFC-002]` son propuestas que aun no existen en el motor.

#### T01 Flow — Hospital: Gap Analysis (GT → CRT)

**Estado inicial**: GT `tree-gt-cirugia` con GOAL-001 "Excelencia quirurgica", OBJ-001 "Seguridad del paciente", NC-001 "Equipo anestesia certificado", NC-002 "Ratio enfermera:paciente 1:2", NC-003 "Mortalidad quirurgica <1%".
**Trigger**: El analista quiere diagnosticar la realidad contra la norma.

**Pasos**:

1. Consultar NCs del GT:
   ```
   ltp/tree_walk --tree_id tree-gt-cirugia
   ```
   → Lista completa de GOAL, OBJ, NC con sus IDs

2. Crear CRT y UDEs por cada NC no cumplida:
   ```
   ltp/history_begin_batch --label "T01: gap analysis GT→CRT cirugia"
   ltp/tree_new --type crt --name "Diagnostico cirugia Q3"
   ```
   → tree-crt-cirugia-q3
   ```
   ltp/node_add --type UDE --label "Mortalidad quirurgica actual es 3.2%"
   ```
   → UDE-001
   ```
   ltp/tree_attach --tree tree-crt-cirugia-q3 --node UDE-001 --role leaf
   ltp/node_add --type UDE --label "Ratio enfermera:paciente actual es 1:5"
   ```
   → UDE-002
   ```
   ltp/tree_attach --tree tree-crt-cirugia-q3 --node UDE-002 --role leaf
   ```

3. Crear nodos intermedios y causas raiz:
   ```
   ltp/node_add --type INT --label "Anestesistas sin certificacion avanzada cubren turnos criticos"
   ```
   → INT-001
   ```
   ltp/tree_attach --tree tree-crt-cirugia-q3 --node INT-001 --role intermediate
   ltp/link_connect --tree tree-crt-cirugia-q3 --from ["INT-001"] --to ["UDE-001"]
   ltp/node_add --type RC --label "Politica de contratacion no exige certificacion avanzada"
   ```
   → RC-001
   ```
   ltp/tree_attach --tree tree-crt-cirugia-q3 --node RC-001 --role leaf
   ltp/link_connect --tree tree-crt-cirugia-q3 --from ["RC-001"] --to ["INT-001"]
   ```

4. Validar y cerrar:
   ```
   ltp/validate
   ltp/history_end_batch
   ```

**Resultado**: CRT con 2 UDEs, 1 INT, 1 RC. Cada UDE ancla en una NC del GT.
**Meta-grafo**: `tree_relation { type: "gap_analysis", from: "tree-gt-cirugia", to: "tree-crt-cirugia-q3", handoff_nodes: ["NC-001→UDE-001", "NC-002→UDE-002"] }` [RFC-002]
**Capacidades RFC-002**: campo `nc_ref` en UDEs para trazar mapping NC→UDE. `tree_relation` tipada.

---

#### T02 Flow — Banco de Inversion: Validacion Directa de Conflicto (GT → EC)

**Estado inicial**: GT `tree-gt-banco` con OBJ-001 "Captacion agresiva de talento quant", NC-001 "Bonus discrecionales sin techo para traders top"; OBJ-002 "Cultura de compliance estricta", NC-004 "Aprobacion previa de toda operacion por compliance".
**Trigger**: Al formular NCs, el analista detecta que NC-001 y NC-004 son estructuralmente incompatibles.

**Pasos**:

1. Inspeccionar las NCs en conflicto:
   ```
   ltp/node_inspect --id NC-001
   ltp/node_inspect --id NC-004
   ```

2. Crear EC directamente desde GT:
   ```
   ltp/history_begin_batch --label "T02: conflicto directo GT→EC"
   ltp/tree_new --type ec --name "Conflicto incentivos vs compliance"
   ```
   → tree-ec-incentivos
   ```
   ltp/node_add --type OBJ --label "Banco competitivo y regulado"
   ```
   → OBJ-003
   ```
   ltp/tree_attach --tree tree-ec-incentivos --node OBJ-003 --role root
   ```

3. Reutilizar nodos del GT como needs del EC:
   ```
   ltp/tree_attach --tree tree-ec-incentivos --node OBJ-001 --role intermediate  # need B
   ltp/tree_attach --tree tree-ec-incentivos --node OBJ-002 --role intermediate  # need C
   ```

4. Crear prereqs (D y D'):
   ```
   ltp/node_add --type PRE --label "Bonus discrecionales sin techo atraen talento"
   ```
   → PRE-001
   ```
   ltp/tree_attach --tree tree-ec-incentivos --node PRE-001 --role leaf  # prereq D
   ltp/node_add --type PRE --label "Aprobacion previa de compliance en toda operacion"
   ```
   → PRE-002
   ```
   ltp/tree_attach --tree tree-ec-incentivos --node PRE-002 --role leaf  # prereq D'
   ```

5. Conectar estructura y marcar conflicto:
   ```
   ltp/link_connect --tree tree-ec-incentivos --from ["OBJ-003"] --to ["OBJ-001"]
   ltp/link_connect --tree tree-ec-incentivos --from ["OBJ-003"] --to ["OBJ-002"]
   ltp/link_connect --tree tree-ec-incentivos --from ["OBJ-001"] --to ["PRE-001"]
   ltp/link_connect --tree tree-ec-incentivos --from ["OBJ-002"] --to ["PRE-002"]
   ltp/link_connect --tree tree-ec-incentivos --from ["PRE-001"] --to ["PRE-002"] --operator XOR
   ltp/validate
   ltp/history_end_batch
   ```

**Resultado**: EC con 5 nodos, conflicto D↔D' marcado. Nodos B y C reutilizados del GT.
**Meta-grafo**: `tree_relation { type: "core_conflict", from: "tree-gt-banco", to: "tree-ec-incentivos", handoff_nodes: ["OBJ-001", "OBJ-002"] }` [RFC-002]
**Capacidades RFC-002**: `tree_relation` tipada. Warning `CANONICAL_SKIP_CRT`.

---

#### T03 Flow — Fabrica: Benchmark de Futuro (GT → FRT)

**Estado inicial**: GT `tree-gt-fabrica` completo con OBJs de eficiencia, calidad y coste. Un proveedor ofrece robotizacion.
**Trigger**: Evaluar impacto de la propuesta contra la norma ANTES de diagnosticar problemas actuales.

**Pasos**:

1. Consultar OBJs del GT para definir criterios de evaluacion:
   ```
   ltp/tree_walk --tree_id tree-gt-fabrica
   ```
   → OBJ-001 "Throughput >8000 ud/dia", OBJ-002 "Defectos <0.5%", OBJ-003 "Coste unitario <2.50EUR"

2. Crear FRT de benchmark con INJ externa:
   ```
   ltp/history_begin_batch --label "T03: benchmark robotizacion"
   ltp/tree_new --type frt --name "Benchmark robotizacion linea A"
   ```
   → tree-frt-benchmark-robot
   ```
   ltp/node_add --type INJ --label "Instalar celula robotizada de picking en linea A"
   ```
   → INJ-001
   ```
   ltp/tree_attach --tree tree-frt-benchmark-robot --node INJ-001 --role root
   ```

3. Modelar DEs esperados:
   ```
   ltp/node_add --type DE --label "Throughput de linea A sube a 12000 ud/dia"
   ```
   → DE-001
   ```
   ltp/node_add --type DE --label "Tasa de defectos baja a 0.2%"
   ```
   → DE-002
   ```
   ltp/node_add --type DE --label "Coste unitario sube a 3.10EUR (amortizacion robot)"
   ```
   → DE-003
   ```
   ltp/tree_attach --tree tree-frt-benchmark-robot --node DE-001 --role leaf
   ltp/tree_attach --tree tree-frt-benchmark-robot --node DE-002 --role leaf
   ltp/tree_attach --tree tree-frt-benchmark-robot --node DE-003 --role leaf
   ltp/link_connect --tree tree-frt-benchmark-robot --from ["INJ-001"] --to ["DE-001"]
   ltp/link_connect --tree tree-frt-benchmark-robot --from ["INJ-001"] --to ["DE-002"]
   ltp/link_connect --tree tree-frt-benchmark-robot --from ["INJ-001"] --to ["DE-003"]
   ```

4. Evaluar contra GT y cerrar:
   ```
   ltp/validate
   ltp/history_end_batch
   ```
   → DE-001 supera OBJ-001 check, DE-002 supera OBJ-002 check, DE-003 NO cumple OBJ-003 gap

**Resultado**: FRT benchmark muestra 2/3 OBJs superados, 1/3 con gap de coste. Informa decision: merece investigar mas pero el coste es un bloqueante.
**Meta-grafo**: `tree_relation { type: "benchmark", from: "tree-gt-fabrica", to: "tree-frt-benchmark-robot", handoff_nodes: ["INJ-001"] }` [RFC-002]
**Capacidades RFC-002**: `relation_type: benchmark` (nuevo). Mapping `DE→OBJ` para cobertura. Warning `BENCHMARK_WITHOUT_DIAGNOSIS`.

---

#### T04 Flow — Gobierno: Roadmap Estrategico (GT → PRT/TT)

**Estado inicial**: GT `tree-gt-digital-rural` con GOAL "Digitalizacion rural completa en 24 meses", NCs definidas. No existe sistema previo (greenfield).
**Trigger**: No hay realidad que diagnosticar — las NCs se convierten directamente en objetivos del PRT.

**Pasos**:

1. Consultar NCs del GT:
   ```
   ltp/tree_walk --tree_id tree-gt-digital-rural
   ```
   → NC-001 "Conectividad >10 Mbps en 500 municipios", NC-002 "Formacion digital para 5000 funcionarios", NC-003 "Equipamiento en 200 oficinas rurales"

2. Crear PRT y convertir NCs en objetivos:
   ```
   ltp/history_begin_batch --label "T04: roadmap greenfield"
   ltp/tree_new --type prt --name "Prerequisitos digitalizacion rural"
   ```
   → tree-prt-digital-rural
   ```
   ltp/tree_attach --tree tree-prt-digital-rural --node NC-001 --role root  # objetivo del PRT
   ```

3. Identificar obstaculos por NC:
   ```
   ltp/node_add --type OBS --label "No existe infraestructura de fibra en zonas rurales"
   ```
   → OBS-001
   ```
   ltp/tree_attach --tree tree-prt-digital-rural --node OBS-001 --role intermediate
   ltp/link_connect --tree tree-prt-digital-rural --from ["OBS-001"] --to ["NC-001"]
   ltp/node_add --type IO --label "Contrato con operador para despliegue de fibra rural"
   ```
   → IO-001
   ```
   ltp/tree_attach --tree tree-prt-digital-rural --node IO-001 --role leaf
   ltp/link_connect --tree tree-prt-digital-rural --from ["IO-001"] --to ["OBS-001"]
   ```

4. Validar y cerrar:
   ```
   ltp/validate
   ltp/history_end_batch
   ```

**Resultado**: PRT con NCs del GT como objetivos, obstaculos de construccion y IOs. Listo para derivar TT.
**Meta-grafo**: `tree_relation { type: "strategic_roadmap", from: "tree-gt-digital-rural", to: "tree-prt-digital-rural", handoff_nodes: ["NC-001", "NC-002", "NC-003"] }` [RFC-002]
**Capacidades RFC-002**: `relation_type: strategic_roadmap` (nuevo). Warning si CRT existe en el mismo sistema.

---

#### T05 Flow — Fabrica: Feedback a la Norma (CRT → GT)

**Estado inicial**: CRT `tree-crt-fabrica` con UDE-009 "Operarios sufren lesiones musculoesqueleticas cronicas". GT `tree-gt-fabrica` solo cubre throughput, calidad y coste.
**Trigger**: UDE-009 no viola ninguna NC/OBJ del GT — la norma esta incompleta.

**Pasos**:

1. Buscar UDEs sin ancla en GT:
   ```
   ltp/tree_walk --tree_id tree-crt-fabrica
   ltp/tree_walk --tree_id tree-gt-fabrica
   ```
   → UDE-009 no mapea a ninguna NC. Gap detectado.

2. Actualizar GT con nueva dimension:
   ```
   ltp/history_begin_batch --label "T05: feedback CRT→GT ergonomia"
   ltp/node_add --type OBJ --label "Seguridad y bienestar del operario"
   ```
   → OBJ-004
   ```
   ltp/tree_attach --tree tree-gt-fabrica --node OBJ-004 --role intermediate
   ltp/node_add --type NC --label "Tasa de lesiones laborales menor a 2 por 1000 horas"
   ```
   → NC-010
   ```
   ltp/tree_attach --tree tree-gt-fabrica --node NC-010 --role leaf
   ltp/link_connect --tree tree-gt-fabrica --from ["OBJ-004"] --to ["NC-010"]
   ```

3. Vincular al GOAL existente y validar:
   ```
   ltp/link_connect --tree tree-gt-fabrica --from ["GOAL-001"] --to ["OBJ-004"]
   ltp/validate
   ltp/history_end_batch
   ```

**Resultado**: GT ampliado con OBJ-004 y NC-010. UDE-009 del CRT ahora tiene ancla en la norma.
**Meta-grafo**: `tree_relation { type: "revision", from: "tree-crt-fabrica", to: "tree-gt-fabrica", handoff_nodes: ["UDE-009→OBJ-004"] }` [RFC-002]
**Capacidades RFC-002**: deteccion automatica de UDEs huerfanas en `validate`. Campo `nc_ref` en UDEs.

---

#### T06 Flow — Hospital: Core Conflict (CRT → EC)

**Estado inicial**: CRT `tree-crt-consultas` completo. `trace` identifica RC-003 "Politica de 10 min por consulta" como CRC (conecta con 85% de UDEs).
**Trigger**: La CRC persiste porque hay un conflicto subyacente — necesitamos entender POR QUE se tolera.

**Pasos**:

1. Identificar CRC via trace:
   ```
   ltp/trace --tree tree-crt-consultas --node_id RC-003 --direction downstream
   ```
   → RC-003 conecta downstream con UDE-001, UDE-002, UDE-004, UDE-005, UDE-007 (5/6 = 83%)

2. Crear EC a partir del conflicto detras de la CRC:
   ```
   ltp/history_begin_batch --label "T06: core conflict consultas"
   ltp/tree_new --type ec --name "Conflicto tiempo de consulta"
   ```
   → tree-ec-consultas
   ```
   ltp/node_add --type OBJ --label "Atencion medica de calidad sostenible"
   ```
   → OBJ-002
   ```
   ltp/tree_attach --tree tree-ec-consultas --node OBJ-002 --role root
   ltp/node_add --type REQ --label "Sostenibilidad financiera del hospital"
   ```
   → REQ-001
   ```
   ltp/node_add --type REQ --label "Diagnostico preciso de cada paciente"
   ```
   → REQ-002
   ```
   ltp/tree_attach --tree tree-ec-consultas --node REQ-001 --role intermediate  # need B
   ltp/tree_attach --tree tree-ec-consultas --node REQ-002 --role intermediate  # need C
   ltp/node_add --type PRE --label "Consultas de maximo 10 minutos"
   ```
   → PRE-003
   ```
   ltp/node_add --type PRE --label "Consultas de 30+ minutos con historia detallada"
   ```
   → PRE-004
   ```
   ltp/tree_attach --tree tree-ec-consultas --node PRE-003 --role leaf  # prereq D
   ltp/tree_attach --tree tree-ec-consultas --node PRE-004 --role leaf  # prereq D'
   ```

3. Conectar estructura, marcar conflicto y validar:
   ```
   ltp/link_connect --tree tree-ec-consultas --from ["OBJ-002"] --to ["REQ-001"]
   ltp/link_connect --tree tree-ec-consultas --from ["OBJ-002"] --to ["REQ-002"]
   ltp/link_connect --tree tree-ec-consultas --from ["REQ-001"] --to ["PRE-003"]
   ltp/link_connect --tree tree-ec-consultas --from ["REQ-002"] --to ["PRE-004"]
   ltp/link_connect --tree tree-ec-consultas --from ["PRE-003"] --to ["PRE-004"] --operator XOR
   ltp/validate
   ltp/history_end_batch
   ```

**Resultado**: EC con conflicto D↔D' (10 min vs 30+ min). Listo para generar supuestos.
**Meta-grafo**: `tree_relation { type: "core_conflict", from: "tree-crt-consultas", to: "tree-ec-consultas", handoff_nodes: ["RC-003"] }` [RFC-002]
**Capacidades RFC-002**: CRC auto-detection en `trace`. `tree_relation` con `handoff_nodes`.

---

#### T07 Flow — Compliance: Salto Directo sin EC (CRT → FRT)

**Estado inicial**: CRT `tree-crt-datos` con CRC RC-001 "La empresa no implementa consentimiento explicito GDPR". Solucion impuesta por regulacion.
**Trigger**: No hay dilema que evaporar — la ley obliga a implementar GDPR. FRT directo.

**Pasos**:

1. Confirmar CRC y que la solucion es impuesta:
   ```
   ltp/trace --tree tree-crt-datos --node_id RC-001 --direction downstream
   ```
   → RC-001 conecta con UDE-001 "Riesgo de multa GDPR >4% facturacion", UDE-002 "Clientes no confian en manejo de datos"

2. Crear FRT con INJ impuesta (sin EC):
   ```
   ltp/history_begin_batch --label "T07: salto directo CRT→FRT GDPR"
   ltp/tree_new --type frt --name "Futuro con cumplimiento GDPR"
   ```
   → tree-frt-gdpr
   ```
   ltp/node_add --type INJ --label "Implementar plataforma de gestion de consentimiento GDPR"
   ```
   → INJ-002
   ```
   ltp/tree_attach --tree tree-frt-gdpr --node INJ-002 --role root
   ```

3. Modelar DEs opuestos a UDEs:
   ```
   ltp/node_add --type DE --label "Riesgo de multa GDPR eliminado"
   ```
   → DE-004
   ```
   ltp/node_add --type DE --label "Clientes perciben transparencia en manejo de datos"
   ```
   → DE-005
   ```
   ltp/tree_attach --tree tree-frt-gdpr --node DE-004 --role leaf
   ltp/tree_attach --tree tree-frt-gdpr --node DE-005 --role leaf
   ltp/link_connect --tree tree-frt-gdpr --from ["INJ-002"] --to ["DE-004"]
   ltp/link_connect --tree tree-frt-gdpr --from ["INJ-002"] --to ["DE-005"]
   ltp/validate
   ltp/history_end_batch
   ```

**Resultado**: FRT con INJ impuesta y 2 DEs. EC saltada justificadamente (regulacion).
**Meta-grafo**: `tree_relation { type: "injection", from: "tree-crt-datos", to: "tree-frt-gdpr", handoff_nodes: ["RC-001→INJ-002"], ec_skipped: true }` [RFC-002]
**Capacidades RFC-002**: campo `source_type: imposed` en INJ. Warning `CANONICAL_SKIP_EC`.

---

#### T08 Flow — Ciberseguridad: Accion de Emergencia (CRT → PRT/TT)

**Estado inicial**: CRT `tree-crt-seguridad` con UDE-001 "Datos de 50.000 clientes expuestos por vulnerabilidad en API publica" (severidad critica).
**Trigger**: Riesgo inmediato — no se puede esperar a EC/FRT. Accion de contencion urgente.

**Pasos**:

1. Confirmar severidad critica:
   ```
   ltp/node_inspect --id UDE-001
   ```
   → Severidad critica. Requiere accion inmediata.

2. Crear TT de emergencia (sin EC ni FRT):
   ```
   ltp/history_begin_batch --label "T08: emergencia CRT→TT ciberseguridad"
   ltp/tree_new --type tt --name "Contencion brecha de datos - emergencia"
   ```
   → tree-tt-emergencia-brecha

3. Crear pasos tacticos de contencion:
   ```
   ltp/node_add --type IO --label "Desactivar endpoint API vulnerable en produccion"
   ```
   → IO-001
   ```
   ltp/tree_attach --tree tree-tt-emergencia-brecha --node IO-001 --role root  # primer paso
   ltp/node_add --type IO --label "Rotar todas las credenciales y tokens de API"
   ```
   → IO-002
   ```
   ltp/tree_attach --tree tree-tt-emergencia-brecha --node IO-002 --role intermediate
   ltp/link_connect --tree tree-tt-emergencia-brecha --from ["IO-001"] --to ["IO-002"]
   ltp/node_add --type IO --label "Notificar a autoridad de proteccion de datos en 72h"
   ```
   → IO-003
   ```
   ltp/tree_attach --tree tree-tt-emergencia-brecha --node IO-003 --role leaf
   ltp/link_connect --tree tree-tt-emergencia-brecha --from ["IO-002"] --to ["IO-003"]
   ```

4. Validar y cerrar:
   ```
   ltp/validate
   ltp/history_end_batch
   ```

**Resultado**: TT de emergencia con 3 acciones secuenciadas. Flujo completo (GT→CRT→EC→FRT→PRT→TT) sigue en paralelo para solucion estructural.
**Meta-grafo**: `tree_relation { type: "emergency_action", from: "tree-crt-seguridad", to: "tree-tt-emergencia-brecha", handoff_nodes: ["UDE-001"], temporary: true }` [RFC-002]
**Capacidades RFC-002**: `relation_type: emergency_action` (nuevo). Warning `EMERGENCY_ACTION_OPEN` si no hay flujo completo en paralelo. Campo `severity` en UDEs.

### GRUPO C: Transiciones desde la EC

#### T09 Flow — Banco: Redefinicion de la Meta (EC → GT)

**Estado inicial**: GT "Maximizar rentabilidad bancaria" con OBJ "Margen por operacion >2%". EC construida sobre conflicto de margen vs captacion. Supuestos generados.
**Trigger**: Al invalidar supuestos, se descubre que "rentabilidad = margen por operacion" es un supuesto caduco — el mercado pivota a modelo de suscripcion.

**Pasos**:

1. Listar supuestos de la EC:
   ```
   ltp/assume_list --tree tree-ec-margen
   ```
   → ASM-003: "El margen por operacion es la unica via de rentabilidad"

2. Invalidar el supuesto:
   ```
   ltp/invalidate --tree tree-ec-margen --link LNK-003 --asm ASM-003
   ```

3. Consultar el GT para localizar la NC afectada:
   ```
   ltp/tree_walk --tree_id tree-gt-banco
   ```
   → NC-002: "Margen por operacion >2% en cada producto"

4. Modificar la NC en batch:
   ```
   ltp/history_begin_batch --label "T09: revision GT por supuesto EC"
   ltp/node_edit --id NC-002 --label "Revenue recurrente por cliente >500 EUR/ano (incluye suscripcion y transaccional)"
   ltp/history_end_batch
   ```

5. Validar coherencia del GT:
   ```
   ltp/validate
   ```

**Resultado**: GT actualizado con NC reformulada. La EC puede necesitar reformulacion (T33) si el nodo A ya no alinea con el Goal.
**Meta-grafo**: `tree_relation { type: revision, from: tree-ec-margen, to: tree-gt-banco, handoff_nodes: [ASM-003 → NC-002] }`
**Capacidades RFC-002**: `tree_relation` tipada con handoff_nodes. Campo `invalidated_by` en assumptions para trazar que supuesto desencadeno la revision.

---

#### T10 Flow — Hospital: Revision Causal (EC → CRT)

**Estado inicial**: CRT "Diagnostico cirugia" con CRC = RC-003 "Exceso de pacientes por medico". EC construida sobre conflicto volumen vs calidad.
**Trigger**: Al generar supuestos para flecha D→B en la EC, se descubre que el 40% del tiempo del medico se pierde en burocracia — causa no capturada en el CRT.

**Pasos**:

1. Listar supuestos de la EC:
   ```
   ltp/assume_list --tree tree-ec-consultas
   ```
   → ASM-005: "Los medicos no tienen tiempo porque hay demasiados pacientes"

2. Investigar el supuesto — el analista descubre que la burocracia es el verdadero consumidor de tiempo. Anadir al CRT:
   ```
   ltp/history_begin_batch --label "T10: extension CRT por insight EC"
   ltp/node_add --type INT --label "Los medicos dedican 40% de cada consulta a documentacion administrativa"
   ```
   → INT-015
   ```
   ltp/tree_attach --tree tree-crt-cirugia --node INT-015 --role intermediate
   ltp/node_add --type RC --label "El sistema HIS requiere entrada manual de 23 campos por consulta"
   ```
   → RC-008
   ```
   ltp/tree_attach --tree tree-crt-cirugia --node RC-008 --role leaf
   ltp/link_connect --tree tree-crt-cirugia --from ["RC-008"] --to ["INT-015"]
   ltp/link_connect --tree tree-crt-cirugia --from ["INT-015"] --to ["UDE-001"]
   ltp/history_end_batch
   ```

3. Re-evaluar la CRC:
   ```
   ltp/trace --tree tree-crt-cirugia --node_id UDE-001 --direction upstream
   ```
   → RC-008 ahora conecta con mas UDEs que RC-003. La CRC puede haber cambiado.

4. Validar:
   ```
   ltp/validate
   ```

**Resultado**: CRT extendido con nueva cadena causal. La CRC potencialmente cambia de "exceso de pacientes" a "burocracia administrativa". La EC puede necesitar reformulacion.
**Meta-grafo**: `tree_relation { type: revision, from: tree-ec-consultas, to: tree-crt-cirugia, handoff_nodes: [ASM-005 → INT-015, RC-008] }`
**Capacidades RFC-002**: `discovered_via: ec_assumption` como provenance en nodos nuevos del CRT.

---

#### T11 Flow — Call center: Injection (EC → FRT)

**Estado inicial**: EC "Conflicto AHT" con D="Limitar a 3 min" vs D'="El tiempo que haga falta". Supuestos generados e invalidados. INJ identificada.
**Trigger**: ASM-007 "Resolver rapido = limitar tiempo" invalidado. INJ: "Sistema de categorizacion automatica que pre-diagnostica y enruta al especialista".

**Pasos**:

1. Crear INJ como nodo:
   ```
   ltp/history_begin_batch --label "T11: EC→FRT injection"
   ltp/node_add --type INJ --label "Implementar categorizacion automatica que pre-diagnostica y enruta al especialista correcto"
   ```
   → INJ-001

2. Crear FRT y adjuntar la INJ:
   ```
   ltp/tree_new --type frt --name "Futuro con categorizacion automatica"
   ```
   → tree-frt-callcenter
   ```
   ltp/tree_attach --tree tree-frt-callcenter --node INJ-001 --role root
   ```

3. Crear DEs opuestos a las UDEs del CRT:
   ```
   ltp/node_add --type DE --label "AHT promedio baja a 2.5 min manteniendo resolucion en primera llamada >80%"
   ```
   → DE-001
   ```
   ltp/node_add --type DE --label "Satisfaccion del cliente sube de 3.2 a 4.1 sobre 5"
   ```
   → DE-002
   ```
   ltp/tree_attach --tree tree-frt-callcenter --node DE-001 --role leaf
   ltp/tree_attach --tree tree-frt-callcenter --node DE-002 --role leaf
   ```

4. Conectar cadena causal:
   ```
   ltp/link_connect --tree tree-frt-callcenter --from ["INJ-001"] --to ["DE-001"]
   ltp/link_connect --tree tree-frt-callcenter --from ["INJ-001"] --to ["DE-002"]
   ltp/history_end_batch
   ```

5. Validar:
   ```
   ltp/validate
   ```

**Resultado**: FRT con INJ como raiz y DEs mapeados a UDEs del CRT. Listo para NBR (T29).
**Meta-grafo**: `tree_relation { type: injection, from: tree-ec-callcenter, to: tree-frt-callcenter, handoff_nodes: [INJ-001], logic_transition: nec→suf }`
**Capacidades RFC-002**: campo `ude_ref` en cada DE para trazar mapping UDE→DE. Campo `source_assumption` en INJ para trazar a ASM-007.

---

#### T12 Flow — Oficina: Implementacion Directa (EC → PRT/TT)

**Estado inicial**: EC "Conflicto de autonomia vs control de gastos" ha producido INJ principal (que ira a FRT) e INJ secundaria menor.
**Trigger**: INJ secundaria "Eliminar aprobacion del director para compras <100 EUR" es trivial y sin riesgo. No merece FRT completo.

**Pasos**:

1. Crear la INJ menor:
   ```
   ltp/node_add --type INJ --label "Eliminar aprobacion del director para compras menores a 100 EUR"
   ```
   → INJ-002

2. Crear TT directamente (sin FRT):
   ```
   ltp/history_begin_batch --label "T12: implementacion directa INJ menor"
   ltp/tree_new --type tt --name "TT quick-win aprobaciones"
   ```
   → tree-tt-aprobaciones
   ```
   ltp/tree_attach --tree tree-tt-aprobaciones --node INJ-002 --role root
   ```

3. Crear pasos del TT:
   ```
   ltp/node_add --type OBS --label "La politica actual requiere firma del director para toda compra"
   ```
   → OBS-001
   ```
   ltp/node_add --type IO --label "Actualizar politica interna de compras con nuevo umbral"
   ```
   → IO-001
   ```
   ltp/tree_attach --tree tree-tt-aprobaciones --node OBS-001 --role leaf
   ltp/tree_attach --tree tree-tt-aprobaciones --node IO-001 --role intermediate
   ltp/link_connect --tree tree-tt-aprobaciones --from ["OBS-001"] --to ["IO-001"]
   ltp/history_end_batch
   ```

4. Validar:
   ```
   ltp/validate
   ```

**Resultado**: TT operativo para INJ menor. La INJ principal sigue su flujo canonico EC→FRT→NBR→PRT/TT.
**Meta-grafo**: `tree_relation { type: tactical, from: tree-ec-gastos, to: tree-tt-aprobaciones, handoff_nodes: [INJ-002], flags: { frt_skipped: true } }`
**Capacidades RFC-002**: flag `frt_skipped` en tree_relation. Warning `CANONICAL_SKIP_FRT` en validate. Anotacion `risk_level: low` en INJ.

---

### GRUPO D: Transiciones desde el FRT

#### T13 Flow — Retail: Revision de la Meta Post-Simulacion (FRT → GT)

**Estado inicial**: GT "Retail competitivo" con OBJs enfocados en venta presencial. FRT "Futuro con e-commerce" completado y validado.
**Trigger**: El FRT muestra un DE inesperado: la plataforma e-commerce habilita un marketplace B2B no contemplado en el GT.

**Pasos**:

1. Inspeccionar el DE emergente en el FRT:
   ```
   ltp/node_inspect --node DE-005
   ```
   → DE-005: "La plataforma e-commerce atrae distribuidores B2B que quieren usar el catalogo como marketplace"

2. Verificar que el GT no cubre esto:
   ```
   ltp/tree_walk --tree_id tree-gt-retail
   ```
   → Ningun OBJ contempla canal B2B ni marketplace.

3. Anadir OBJ al GT:
   ```
   ltp/history_begin_batch --label "T13: GT ampliado por DE emergente del FRT"
   ltp/node_add --type OBJ --label "Revenue adicional via marketplace B2B integrado en plataforma e-commerce"
   ```
   → OBJ-005
   ```
   ltp/tree_attach --tree tree-gt-retail --node OBJ-005 --role intermediate
   ltp/node_add --type NC --label "Onboarding de distribuidores con catalogo auto-gestionable"
   ```
   → NC-012
   ```
   ltp/tree_attach --tree tree-gt-retail --node NC-012 --role leaf
   ltp/link_connect --tree tree-gt-retail --from ["OBJ-005"] --to ["NC-012"]
   ltp/history_end_batch
   ```

4. Validar:
   ```
   ltp/validate
   ```

**Resultado**: GT ampliado con dimension B2B. El FRT puede necesitar extension (T34) para modelar DEs del nuevo OBJ.
**Meta-grafo**: `tree_relation { type: revision, from: tree-frt-ecommerce, to: tree-gt-retail, handoff_nodes: [DE-005 → OBJ-005] }`
**Capacidades RFC-002**: mapping `DE → OBJ` para detectar DEs sin OBJ ancla (oportunidades emergentes).

---

#### T14 Flow — Software: Revision de la Realidad (FRT → CRT)

**Estado inicial**: CRT "Diagnostico delivery" con CRC = RC-001 "Pipeline CI/CD manual". FRT "Futuro con CI/CD automatizado" construido.
**Trigger**: Al modelar el FRT, se descubre: "Si deploys automaticos, y si no hay ownership de microservicios, entonces nadie sabe quien arregla un deploy roto". El CRT no capturaba falta de ownership.

**Pasos**:

1. El FRT revela la condicion oculta. Ir al CRT y anadir:
   ```
   ltp/history_begin_batch --label "T14: CRT extendido por insight FRT"
   ltp/node_add --type UDE --label "Ningun equipo tiene ownership claro de los 12 microservicios en produccion"
   ```
   → UDE-008
   ```
   ltp/tree_attach --tree tree-crt-delivery --node UDE-008 --role root
   ltp/node_add --type INT --label "Los microservicios se crearon ad-hoc sin asignar responsable"
   ```
   → INT-020
   ```
   ltp/tree_attach --tree tree-crt-delivery --node INT-020 --role intermediate
   ltp/link_connect --tree tree-crt-delivery --from ["INT-020"] --to ["UDE-008"]
   ```

2. Conectar con cadenas existentes:
   ```
   ltp/link_connect --tree tree-crt-delivery --from ["RC-001"] --to ["INT-020"]
   ltp/history_end_batch
   ```

3. Verificar si la CRC cambia:
   ```
   ltp/trace --tree tree-crt-delivery --node_id UDE-008 --direction upstream
   ltp/trace --tree tree-crt-delivery --node_id RC-001 --direction downstream
   ```

4. Validar:
   ```
   ltp/validate
   ```

**Resultado**: CRT mas completo. La INJ del FRT puede necesitar ajuste si el ownership es factor critico no contemplado.
**Meta-grafo**: `tree_relation { type: revision, from: tree-frt-cicd, to: tree-crt-delivery, handoff_nodes: [UDE-008, INT-020] }`
**Capacidades RFC-002**: provenance `discovered_via: frt_simulation` en nodos nuevos del CRT.

---

#### T15 Flow — Energia: Challenges / Nuevo Conflicto (FRT → EC)

**Estado inicial**: FRT "Futuro con solar en tejados" construido. NBR ejecutado (T29).
**Trigger**: NBR catastrofico: "Si solar 100%, Y SI 5 dias nublados consecutivos, entonces apagon total — sin capacidad de generacion alternativa". Trimming inviable (no hay espacio para baterias a escala suficiente).

**Pasos**:

1. Inspeccionar el NBR critico:
   ```
   ltp/nbr_inspect --tree tree-frt-solar --nbr_id NBR-001
   ```
   → Severidad: critica. UDE colateral: "Apagon total por dependencia climatica". Trimming evaluada como inviable.

2. Crear EC hija para resolver el conflicto del NBR:
   ```
   ltp/history_begin_batch --label "T15: EC hija por NBR catastrofico"
   ltp/tree_new --type ec --name "Conflicto autonomia solar vs estabilidad"
   ```
   → tree-ec-solar-estabilidad

3. Construir los 5 nodos de la EC:
   ```
   ltp/node_add --type OBJ --label "Energia sostenible y fiable para el edificio"
   ```
   → OBJ-010 (nodo A)
   ```
   ltp/node_add --type REQ --label "Maximizar autonomia energetica renovable"
   ```
   → REQ-010 (nodo B)
   ```
   ltp/node_add --type REQ --label "Garantizar suministro continuo 24/7/365"
   ```
   → REQ-011 (nodo C)
   ```
   ltp/node_add --type PRE --label "100% generacion solar en tejado"
   ```
   → PRE-010 (nodo D)
   ```
   ltp/node_add --type PRE --label "Mantener conexion a red electrica como respaldo"
   ```
   → PRE-011 (nodo D')
   ```
   ltp/tree_attach --tree tree-ec-solar-estabilidad --node OBJ-010 --role root
   ltp/tree_attach --tree tree-ec-solar-estabilidad --node REQ-010 --role intermediate
   ltp/tree_attach --tree tree-ec-solar-estabilidad --node REQ-011 --role intermediate
   ltp/tree_attach --tree tree-ec-solar-estabilidad --node PRE-010 --role leaf
   ltp/tree_attach --tree tree-ec-solar-estabilidad --node PRE-011 --role leaf
   ```

4. Conectar y marcar conflicto:
   ```
   ltp/link_connect --tree tree-ec-solar-estabilidad --from ["OBJ-010"] --to ["REQ-010"]
   ltp/link_connect --tree tree-ec-solar-estabilidad --from ["OBJ-010"] --to ["REQ-011"]
   ltp/link_connect --tree tree-ec-solar-estabilidad --from ["REQ-010"] --to ["PRE-010"]
   ltp/link_connect --tree tree-ec-solar-estabilidad --from ["REQ-011"] --to ["PRE-011"]
   ltp/link_connect --tree tree-ec-solar-estabilidad --from ["PRE-010"] --to ["PRE-011"] --operator XOR
   ltp/history_end_batch
   ```

5. Validar:
   ```
   ltp/validate
   ```

**Resultado**: EC hija lista para generacion de supuestos. La INJ que emerja sera una trimming fundamentada para el FRT original.
**Meta-grafo**: `tree_relation { type: challenges, from: tree-frt-solar, to: tree-ec-solar-estabilidad, handoff_nodes: [NBR-001] }`
**Capacidades RFC-002**: `parent_scenario` y `depth` en la EC hija para trackear recursion. `nbr_ref` en tree_relation.

---

#### T16 Flow — Hospital: Prerequisite/Tactical (FRT → PRT/TT)

**Estado inicial**: FRT "Futuro con IA de transcripcion" completo. NBRs procesados: 2 trimmed (T24), 1 absorbido (T25). INJ validada e inmunizada.
**Trigger**: La INJ esta lista para implementacion. Hay que planificar el como.

**Pasos**:

1. Verificar completitud del FRT:
   ```
   ltp/validate
   ltp/nbr_list --tree tree-frt-transcripcion
   ```
   → Todos los NBRs tienen status: trimmed o absorbed. No hay NBRs pendientes.

2. Crear PRT:
   ```
   ltp/history_begin_batch --label "T16: FRT→PRT planificacion de implementacion"
   ltp/tree_new --type prt --name "Prerequisitos IA transcripcion"
   ```
   → tree-prt-transcripcion

3. Adjuntar la INJ como objetivo del PRT:
   ```
   ltp/tree_attach --tree tree-prt-transcripcion --node INJ-001 --role root
   ```

4. Identificar obstaculos y crear IOs:
   ```
   ltp/node_add --type OBS --label "El sistema HIS actual no tiene API para integracion externa"
   ```
   → OBS-001
   ```
   ltp/node_add --type IO --label "Implementar API REST en el HIS para envio/recepcion de transcripciones"
   ```
   → IO-001
   ```
   ltp/node_add --type OBS --label "Los medicos no estan formados en revision de transcripciones automaticas"
   ```
   → OBS-002
   ```
   ltp/node_add --type IO --label "Programa de formacion piloto con 10 medicos voluntarios"
   ```
   → IO-002
   ```
   ltp/tree_attach --tree tree-prt-transcripcion --node OBS-001 --role leaf
   ltp/tree_attach --tree tree-prt-transcripcion --node IO-001 --role intermediate
   ltp/tree_attach --tree tree-prt-transcripcion --node OBS-002 --role leaf
   ltp/tree_attach --tree tree-prt-transcripcion --node IO-002 --role intermediate
   ltp/link_connect --tree tree-prt-transcripcion --from ["OBS-001"] --to ["IO-001"]
   ltp/link_connect --tree tree-prt-transcripcion --from ["OBS-002"] --to ["IO-002"]
   ltp/link_connect --tree tree-prt-transcripcion --from ["IO-001"] --to ["INJ-001"]
   ltp/link_connect --tree tree-prt-transcripcion --from ["IO-002"] --to ["INJ-001"]
   ltp/history_end_batch
   ```

5. Validar:
   ```
   ltp/validate
   ```

**Resultado**: PRT con obstaculos e IOs mapeados a la INJ. Listo para detallar en TT (pasos tacticos por IO).
**Meta-grafo**: `tree_relation { type: prerequisite, from: tree-frt-transcripcion, to: tree-prt-transcripcion, handoff_nodes: [INJ-001], logic_transition: suf→nec }`
**Capacidades RFC-002**: check de completitud en validate — `CANONICAL_SKIP_NBR` si no hay NBRs procesados antes de T16.

#### T17 Flow — Software: Revision Estrategica Post-Implementacion (PRT/TT → GT)

**Estado inicial**: GT "Producto SaaS competitivo" con OBJs de velocidad de entrega y estabilidad. PRT/TT de CI/CD en ejecucion.
**Trigger**: Al implementar IO-003 "Contratar proveedor cloud", el proveedor ofrece ML Ops integrado — capacidad no contemplada en el GT.

**Pasos**:

1. Documentar el hallazgo como knowledge:
   ```
   ltp/knowledge_add --label "Proveedor cloud incluye ML Ops" --type observation
   ```
   → KN-001

2. Vincular knowledge al IO del PRT que lo descubrio:
   ```
   ltp/knowledge_link --id KN-001 --target IO-003 --relation contextualizes
   ```

3. Revisar GT actual:
   ```
   ltp/tree_walk --tree_id tree-gt-producto
   ```
   → OBJ-001 "Velocidad de entrega", OBJ-002 "Estabilidad", OBJ-003 "Escalabilidad". No hay nada sobre inteligencia de datos.

4. Anadir nuevo OBJ al GT:
   ```
   ltp/history_begin_batch --label "T17: PRT/TT revela oportunidad → revision GT"
   ltp/node_add --type OBJ --label "El producto usa datos de uso para mejorar automaticamente"
   ```
   → OBJ-004
   ```
   ltp/tree_attach --tree tree-gt-producto --node OBJ-004 --role intermediate
   ltp/link_connect --tree tree-gt-producto --from ["GOAL-001"] --to ["OBJ-004"]
   ```

5. Derivar NCs para el nuevo OBJ:
   ```
   ltp/node_add --type NC --label "Pipeline de ML operativo procesa datos de telemetria"
   ```
   → NC-009
   ```
   ltp/tree_attach --tree tree-gt-producto --node NC-009 --role leaf
   ltp/link_connect --tree tree-gt-producto --from ["OBJ-004"] --to ["NC-009"]
   ltp/validate
   ltp/history_end_batch
   ```

**Resultado**: GT ampliado con OBJ-004 y NC-009. La meta evoluciona de "entrega eficiente" a "producto inteligente".
**Meta-grafo**: `tree_relation { type: revision, from: tree-prt-cicd, to: tree-gt-producto, handoff_nodes: [IO-003 → OBJ-004] }` [RFC-002]
**Capacidades RFC-002**: tree_relation tipada; knowledge_link como puente de trazabilidad entre descubrimiento y revision.

---

#### T18 Flow — Hospital: La Realidad Ha Cambiado (PRT/TT → CRT)

**Estado inicial**: CRT "Diagnostico cirugia Q3" con UDEs originales. TT de implementacion de IA de transcripcion completado.
**Trigger**: Post-implementacion, se observa UDE nueva: "Los medicos no revisan las transcripciones y se acumulan errores".

**Pasos**:

1. Clonar CRT original para comparacion futura:
   ```
   ltp/tree_clone --source tree-crt-cirugia-q3 --name "CRT pre-implementacion (snapshot)"
   ```
   → tree-crt-cirugia-q3-pre

2. Crear UDE nueva en el CRT original:
   ```
   ltp/history_begin_batch --label "T18: TT revela UDE nueva → revision CRT"
   ltp/node_add --type UDE --label "Los medicos no revisan transcripciones generadas por IA"
   ```
   → UDE-012
   ```
   ltp/tree_attach --tree tree-crt-cirugia-q3 --node UDE-012 --role root
   ```

3. Modelar cadena causal de la UDE nueva:
   ```
   ltp/node_add --type INT --label "Los medicos asumen que la IA no comete errores"
   ```
   → INT-020
   ```
   ltp/tree_attach --tree tree-crt-cirugia-q3 --node INT-020 --role intermediate
   ltp/link_connect --tree tree-crt-cirugia-q3 --from ["INT-020"] --to ["UDE-012"]
   ltp/node_add --type RC --label "No existe protocolo de revision de transcripciones automaticas"
   ```
   → RC-008
   ```
   ltp/tree_attach --tree tree-crt-cirugia-q3 --node RC-008 --role leaf
   ltp/link_connect --tree tree-crt-cirugia-q3 --from ["RC-008"] --to ["INT-020"]
   ```

4. Marcar UDEs originales resueltas (si aplica):
   ```
   ltp/node_edit --id UDE-003 --label "El 28% de entregas llegan tarde [RESUELTA post-implementacion]"
   ```

5. Validar y cerrar:
   ```
   ltp/validate
   ltp/history_end_batch
   ```

6. Comparar CRT pre vs post:
   ```
   ltp/tree_diff --tree-a tree-crt-cirugia-q3-pre --tree-b tree-crt-cirugia-q3
   ```

**Resultado**: CRT actualizado con UDE-012 nueva y cadena causal. UDEs originales marcadas como resueltas. Diff disponible.
**Meta-grafo**: `tree_relation { type: revision, from: tree-tt-ia-transcripcion, to: tree-crt-cirugia-q3, handoff_nodes: [paso-TT-final → UDE-012] }` [RFC-002]
**Capacidades RFC-002**: tree_diff para comparar CRT pre/post; campo `status: resolved` en UDEs.

---

#### T19 Flow — Hospital: Conflicto de Implementacion (PRT/TT → EC)

**Estado inicial**: PRT con IO-001 "Contratar proveedor de IA para transcripcion". IT exige on-premise; finanzas exige SaaS.
**Trigger**: Dos IOs mutuamente excluyentes bloquean la implementacion.

**Pasos**:

1. Inspeccionar los IOs en conflicto:
   ```
   ltp/node_inspect --node IO-001
   ltp/node_inspect --node IO-004
   ```
   → IO-001 "Seleccionar proveedor on-premise (IT)" vs IO-004 "Seleccionar proveedor SaaS (finanzas)"

2. Crear EC de implementacion:
   ```
   ltp/history_begin_batch --label "T19: conflicto de implementacion → EC"
   ltp/tree_new --type ec --name "Conflicto on-premise vs SaaS"
   ```
   → tree-ec-onprem-saas

3. Construir los 5 nodos de la EC:
   ```
   ltp/node_add --type OBJ --label "Implementar IA de transcripcion exitosamente"
   ```
   → OBJ-002 (nodo A)
   ```
   ltp/node_add --type REQ --label "Proteccion de datos clinicos sensibles"
   ```
   → REQ-010 (nodo B)
   ```
   ltp/node_add --type REQ --label "Coste de implementacion sostenible"
   ```
   → REQ-011 (nodo C)
   ```
   ltp/node_add --type PRE --label "La infraestructura de IA esta en servidores propios del hospital"
   ```
   → PRE-015 (nodo D)
   ```
   ltp/node_add --type PRE --label "La infraestructura de IA esta en la nube del proveedor"
   ```
   → PRE-016 (nodo D')

4. Ensamblar EC con roles y conflicto:
   ```
   ltp/tree_attach --tree tree-ec-onprem-saas --node OBJ-002 --role root
   ltp/tree_attach --tree tree-ec-onprem-saas --node REQ-010 --role intermediate
   ltp/tree_attach --tree tree-ec-onprem-saas --node REQ-011 --role intermediate
   ltp/tree_attach --tree tree-ec-onprem-saas --node PRE-015 --role leaf
   ltp/tree_attach --tree tree-ec-onprem-saas --node PRE-016 --role leaf
   ltp/link_connect --tree tree-ec-onprem-saas --from ["OBJ-002"] --to ["REQ-010"]
   ltp/link_connect --tree tree-ec-onprem-saas --from ["OBJ-002"] --to ["REQ-011"]
   ltp/link_connect --tree tree-ec-onprem-saas --from ["REQ-010"] --to ["PRE-015"]
   ltp/link_connect --tree tree-ec-onprem-saas --from ["REQ-011"] --to ["PRE-016"]
   ltp/link_connect --tree tree-ec-onprem-saas --from ["PRE-015"] --to ["PRE-016"] --operator XOR
   ```

5. Generar supuestos para cada flecha:
   ```
   ltp/assume_add --tree tree-ec-onprem-saas --link LNK-REQ010-PRE015 --text "Asumimos que datos clinicos solo estan seguros en servidores propios"
   ltp/assume_add --tree tree-ec-onprem-saas --link LNK-REQ011-PRE016 --text "Asumimos que SaaS es siempre mas barato que on-premise"
   ltp/assume_add --tree tree-ec-onprem-saas --link LNK-PRE015-PRE016 --text "Asumimos que on-premise y cloud son mutuamente excluyentes"
   ```

6. Invalidar supuesto mas fragil:
   ```
   ltp/invalidate --tree tree-ec-onprem-saas --link LNK-PRE015-PRE016 --asm ASM-003
   ```
   → INJ: "Arquitectura hibrida: procesamiento en cloud con datos anonimizados + almacenamiento clinico on-premise"

7. Validar y cerrar:
   ```
   ltp/validate
   ltp/history_end_batch
   ```

**Resultado**: EC de implementacion resuelta con INJ hibrida. El PRT puede continuar con la nueva estrategia.
**Meta-grafo**: `tree_relation { type: challenges, from: tree-prt-ia, to: tree-ec-onprem-saas, handoff_nodes: [IO-001, IO-004 → OBJ-002] }` [RFC-002]
**Capacidades RFC-002**: campo `ec_type: implementation` para distinguir EC de diagnostico vs implementacion.

---

#### T20 Flow — E-commerce: Feedback al Futuro (PRT/TT → FRT)

**Estado inicial**: FRT "Futuro con checkout 1-click" con DE-005 "Margen por pedido +5%". PRT en ejecucion.
**Trigger**: PRT descubre que la pasarela cobra 3.5% (no 1.5% asumido en FRT). El DE de margen es incorrecto.

**Pasos**:

1. Documentar hallazgo:
   ```
   ltp/knowledge_add --label "Pasarela cobra 3.5% no 1.5%" --type observation
   ```
   → KN-005

2. Inspeccionar DE afectado en FRT:
   ```
   ltp/node_inspect --node DE-005
   ```
   → DE-005 "El margen por pedido sube un 5% gracias a checkout optimizado"

3. Actualizar DE en FRT:
   ```
   ltp/history_begin_batch --label "T20: PRT corrige supuesto de coste → revision FRT"
   ltp/node_edit --id DE-005 --label "El margen por pedido sube un 3% gracias a checkout optimizado (pasarela 3.5%)"
   ```

4. Revisar si el DE actualizado sigue justificando la INJ:
   ```
   ltp/trace --tree tree-frt-checkout --node_id DE-005 --direction upstream
   ```
   → Cadena: INJ-001 → INT-010 → DE-005. La cadena sigue siendo valida, pero el beneficio es menor.

5. Verificar cobertura UDE→DE:
   ```
   ltp/tree_walk --tree_id tree-frt-checkout
   ```
   → Revisar que todos los DEs siguen justificando la inversion.

6. Cerrar:
   ```
   ltp/validate
   ltp/history_end_batch
   ```

**Resultado**: FRT actualizado con DE-005 corregido. Analisis coste-beneficio necesita recalculo.
**Meta-grafo**: `tree_relation { type: revision, from: tree-prt-checkout, to: tree-frt-checkout, handoff_nodes: [IO-002 → DE-005] }` [RFC-002]
**Capacidades RFC-002**: campo `revision_reason` en tree_relation para explicar que motivo la revision.

---

#### T21 Flow — Hospital: Revision de la Meta por Riesgo Estructural (NBR → GT)

**Estado inicial**: FRT con INJ "IA diagnostica asiste a medicos". NBR ha generado UDEs colaterales. GT tiene NC-007 "Relacion medico-paciente basada en confianza".
**Trigger**: Toda UDE colateral del NBR viola NC-007 del GT. La conexion es estructural.

**Pasos**:

1. Listar UDEs colaterales del NBR:
   ```
   ltp/nbr_list --tree tree-frt-ia-diagnostica
   ```
   → NBR-001: UDE-020 "Pacientes perciben que el medico depende de una maquina"
   → NBR-002: UDE-021 "Medicos sienten que su juicio clinico es cuestionado por IA"
   → NBR-003: UDE-022 "Responsabilidad legal difusa entre medico e IA"

2. Verificar que todas violan la misma norma GT:
   ```
   ltp/node_inspect --node NC-007
   ```
   → NC-007 "La relacion medico-paciente esta basada en confianza y juicio clinico humano"

3. [RFC-002] Anotar la referencia estructural:
   ```
   [RFC-002] ltp/node_edit --id UDE-020 --add_tag gt_norm:NC-007
   [RFC-002] ltp/node_edit --id UDE-021 --add_tag gt_norm:NC-007
   [RFC-002] ltp/node_edit --id UDE-022 --add_tag gt_norm:NC-007
   ```

4. Evaluar si la meta resiste o necesita revision:
   ```
   ltp/tree_walk --tree_id tree-gt-hospital
   ```
   → Los NBRs no se resuelven con trimming — son inherentes a toda INJ de IA diagnostica.

5. Revisar GT para acomodar la realidad:
   ```
   ltp/history_begin_batch --label "T21: NBRs estructurales → revision GT"
   ltp/node_add --type OBJ --label "El factor humano permanece central en el diagnostico clinico"
   ```
   → OBJ-008
   ```
   ltp/tree_attach --tree tree-gt-hospital --node OBJ-008 --role intermediate
   ltp/link_connect --tree tree-gt-hospital --from ["GOAL-001"] --to ["OBJ-008"]
   ltp/node_add --type NC --label "La IA asiste pero no sustituye el juicio clinico del medico"
   ```
   → NC-012
   ```
   ltp/tree_attach --tree tree-gt-hospital --node NC-012 --role leaf
   ltp/link_connect --tree tree-gt-hospital --from ["OBJ-008"] --to ["NC-012"]
   ltp/validate
   ltp/history_end_batch
   ```

**Resultado**: GT ampliado con OBJ-008 y NC-012 que establecen el limite de la IA. El FRT debera ajustar la INJ para respetar esta nueva norma.
**Meta-grafo**: `tree_relation { type: revision, from: tree-nbr-ia-diagnostica, to: tree-gt-hospital, handoff_nodes: [UDE-020/021/022 → OBJ-008] }` [RFC-002]
**Capacidades RFC-002**: campo `gt_norm_ref` en UDEs colaterales (norma-bisagra ampliada); `severity` en NBRs para clasificar si escalar.

---

#### T22 Flow — Software: Descubrimiento de Realidad Oculta (NBR → CRT)

**Estado inicial**: FRT "Futuro con microservicios". NBR revela condicion actual no diagnosticada. CRT "Diagnostico plataforma" existente.
**Trigger**: NBR dice "Si cada equipo elige su stack, Y SI no hay estandares, entonces babelizacion tecnologica". Se descubre que ya hay 4 lenguajes en produccion sin gobernanza.

**Pasos**:

1. Inspeccionar el NBR que revelo la realidad oculta:
   ```
   ltp/nbr_inspect --nbr_id NBR-004 --tree tree-frt-microservicios
   ```
   → "Si microservicios con autonomia de equipo, Y SI no hay gobernanza tecnologica, entonces proliferacion de stacks incompatibles"

2. Verificar que el CRT actual no captura esta realidad:
   ```
   ltp/node_search --query "gobernanza" --tree tree-crt-plataforma
   ltp/node_search --query "lenguaje" --tree tree-crt-plataforma
   ```
   → Sin resultados. El CRT no tiene nada sobre gobernanza tecnologica.

3. Anadir la realidad descubierta al CRT:
   ```
   ltp/history_begin_batch --label "T22: NBR revela realidad oculta → extension CRT"
   ltp/node_add --type UDE --label "La plataforma usa 4 lenguajes distintos sin estandar comun"
   ```
   → UDE-015
   ```
   ltp/tree_attach --tree tree-crt-plataforma --node UDE-015 --role root
   ltp/node_add --type RC --label "No existe politica de gobernanza tecnologica"
   ```
   → RC-010
   ```
   ltp/tree_attach --tree tree-crt-plataforma --node RC-010 --role leaf
   ltp/link_connect --tree tree-crt-plataforma --from ["RC-010"] --to ["UDE-015"]
   ltp/node_add --type INT --label "Cada equipo elige herramientas sin coordinacion con otros equipos"
   ```
   → INT-025
   ```
   ltp/tree_attach --tree tree-crt-plataforma --node INT-025 --role intermediate
   ltp/link_connect --tree tree-crt-plataforma --from ["RC-010"] --to ["INT-025"]
   ltp/link_connect --tree tree-crt-plataforma --from ["INT-025"] --to ["UDE-015"]
   ```

4. Verificar si la nueva cadena cambia la CRC:
   ```
   ltp/trace --tree tree-crt-plataforma --node_id RC-010 --direction downstream
   ltp/validate
   ltp/history_end_batch
   ```

**Resultado**: CRT ampliado con UDE-015, INT-025, RC-010. Si RC-010 conecta con suficientes UDEs, puede convertirse en nueva CRC o co-CRC.
**Meta-grafo**: `tree_relation { type: revision, from: tree-nbr-microservicios, to: tree-crt-plataforma, handoff_nodes: [NBR-004 → UDE-015] }` [RFC-002]
**Capacidades RFC-002**: campo `discovered_via: nbr` en nodos para provenance.

---

#### T23 Flow — Energia: Nuevo Conflicto por Efecto Colateral (NBR → EC)

**Estado inicial**: FRT "Futuro con planta solar". NBR-007 con severidad critica: dependencia total del clima. Trimming inviable.
**Trigger**: El NBR es demasiado grave para trimming — genera un conflicto genuino que necesita su propia EC.

**Pasos**:

1. Inspeccionar el NBR critico:
   ```
   ltp/nbr_inspect --nbr_id NBR-007 --tree tree-frt-solar
   ```
   → "Si 100% solar, Y SI semana nublada prolongada, entonces corte total de suministro. Severidad: CRITICA. Trimming: baterias insuficientes para >3 dias."

2. Confirmar que trimming no es viable:
   ```
   ltp/node_search --query "trimming" --tree tree-frt-solar
   ```
   → Trimming INJ propuesta (baterias) solo cubre 72h. Insuficiente para patron climatico de 7+ dias.

3. Crear EC hija para el conflicto del NBR:
   ```
   ltp/history_begin_batch --label "T23: NBR critico escala a EC hija"
   ltp/tree_new --type ec --name "Conflicto autonomia solar vs estabilidad de red"
   ```
   → tree-ec-solar-estabilidad

4. Construir nodos de la EC:
   ```
   ltp/node_add --type OBJ --label "La empresa tiene energia sostenible y fiable"
   ```
   → OBJ-005 (A)
   ```
   ltp/node_add --type REQ --label "Autonomia energetica maxima"
   ```
   → REQ-020 (B)
   ```
   ltp/node_add --type REQ --label "Estabilidad de suministro garantizada"
   ```
   → REQ-021 (C)
   ```
   ltp/node_add --type PRE --label "La empresa genera toda su energia con paneles solares"
   ```
   → PRE-030 (D)
   ```
   ltp/node_add --type PRE --label "La empresa mantiene conexion a la red electrica convencional"
   ```
   → PRE-031 (D')

5. Ensamblar y marcar conflicto:
   ```
   ltp/tree_attach --tree tree-ec-solar-estabilidad --node OBJ-005 --role root
   ltp/tree_attach --tree tree-ec-solar-estabilidad --node REQ-020 --role intermediate
   ltp/tree_attach --tree tree-ec-solar-estabilidad --node REQ-021 --role intermediate
   ltp/tree_attach --tree tree-ec-solar-estabilidad --node PRE-030 --role leaf
   ltp/tree_attach --tree tree-ec-solar-estabilidad --node PRE-031 --role leaf
   ltp/link_connect --tree tree-ec-solar-estabilidad --from ["OBJ-005"] --to ["REQ-020"]
   ltp/link_connect --tree tree-ec-solar-estabilidad --from ["OBJ-005"] --to ["REQ-021"]
   ltp/link_connect --tree tree-ec-solar-estabilidad --from ["REQ-020"] --to ["PRE-030"]
   ltp/link_connect --tree tree-ec-solar-estabilidad --from ["REQ-021"] --to ["PRE-031"]
   ltp/link_connect --tree tree-ec-solar-estabilidad --from ["PRE-030"] --to ["PRE-031"] --operator XOR
   ```

6. Generar supuestos e invalidar:
   ```
   ltp/assume_add --tree tree-ec-solar-estabilidad --link LNK-PRE030-PRE031 --text "Asumimos que 100% solar y conexion a red son incompatibles"
   ltp/invalidate --tree tree-ec-solar-estabilidad --link LNK-PRE030-PRE031 --asm ASM-010
   ```
   → INJ: "Modelo hibrido: 80% solar + 20% red como backup, con contrato de potencia minima"

7. Cerrar:
   ```
   ltp/validate
   ltp/history_end_batch
   ```

**Resultado**: EC hija resuelta con INJ hibrida. El FRT se actualiza con la nueva INJ (via T24 o nueva iteracion T11).
**Meta-grafo**: `tree_relation { type: challenges, from: tree-nbr-solar, to: tree-ec-solar-estabilidad, handoff_nodes: [NBR-007 → OBJ-005] }` [RFC-002]
**Capacidades RFC-002**: `parent_scenario`, `depth` en EC hija para tracking de recursion; `severity` en NBR.

---

#### T24 Flow — E-commerce: Trimming (NBR → FRT)

**Estado inicial**: FRT "Futuro con envio gratuito". NBR-010: "Si envio gratuito, Y SI devoluciones masivas, entonces coste insostenible".
**Trigger**: El NBR es severo pero recortable con una trimming INJ.

**Pasos**:

1. Inspeccionar el NBR:
   ```
   ltp/nbr_inspect --nbr_id NBR-010 --tree tree-frt-envio
   ```
   → UDE colateral: "El coste de devoluciones supera el beneficio del envio gratuito". Severidad: ALTA. Trimming viable.

2. Disenar trimming INJ:
   ```
   ltp/history_begin_batch --label "T24: trimming de NBR-010"
   ltp/node_add --type INJ --label "Envio gratuito solo para pedidos superiores a 30 euros con politica de devolucion con coste parcial"
   ```
   → INJ-005 (trimming)

3. Insertar trimming INJ en el FRT para cortar la rama negativa:
   ```
   ltp/tree_attach --tree tree-frt-envio --node INJ-005 --role intermediate
   ltp/node_add --type DE --label "Las devoluciones frivoloas se reducen un 60% por coste parcial"
   ```
   → DE-010
   ```
   ltp/tree_attach --tree tree-frt-envio --node DE-010 --role root
   ltp/link_connect --tree tree-frt-envio --from ["INJ-005"] --to ["DE-010"]
   ```

4. Verificar que la rama negativa se corta:
   ```
   ltp/trace --tree tree-frt-envio --node_id INJ-005 --direction downstream
   ```
   → La cadena ahora muestra: INJ-005 → DE-010 (mitiga UDE colateral de devoluciones).

5. Verificar que la trimming no invalida la INJ principal:
   ```
   ltp/validate
   ltp/history_end_batch
   ```

6. Re-evaluar el NBR:
   ```
   ltp/nbr_inspect --nbr_id NBR-010 --tree tree-frt-envio
   ```
   → Con trimming aplicada, riesgo residual: BAJO.

**Resultado**: FRT' = FRT + trimming INJ-005. La rama negativa de devoluciones esta mitigada. NBR-010 pasa de ALTA a BAJA severidad.
**Meta-grafo**: `tree_relation { type: trimming, from: tree-nbr-envio, to: tree-frt-envio, handoff_nodes: [NBR-010 → INJ-005] }` [RFC-002]
**Capacidades RFC-002**: `relation_type: trimming`; rol `trimming_injection` en tree_attach; `severity` mutable en NBR.

---

#### T25 Flow — Fabrica: Obstaculos Derivados de Riesgos (NBR → PRT/TT)

**Estado inicial**: FRT "Futuro con IoT predictivo". NBR-012 menor: "Si sensores IoT, Y SI operarios no calibran, entonces lecturas erraticas primeros meses".
**Trigger**: El NBR no es catastrofico ni genera conflicto — es un riesgo operativo gestionable.

**Pasos**:

1. Inspeccionar NBR menor:
   ```
   ltp/nbr_inspect --nbr_id NBR-012 --tree tree-frt-iot
   ```
   → UDE colateral: "Lecturas erraticas de sensores durante periodo de adaptacion". Severidad: BAJA. No requiere trimming en FRT.

2. Convertir el riesgo en obstaculo del PRT:
   ```
   ltp/history_begin_batch --label "T25: NBR menor → OBS en PRT"
   ltp/node_add --type OBS --label "Los operarios de planta no saben calibrar sensores IoT"
   ```
   → OBS-008
   ```
   ltp/tree_attach --tree tree-prt-iot --node OBS-008 --role intermediate
   ```

3. Crear IO para superar el obstaculo:
   ```
   ltp/node_add --type IO --label "Programa de formacion de calibracion completado antes del go-live"
   ```
   → IO-010
   ```
   ltp/tree_attach --tree tree-prt-iot --node IO-010 --role intermediate
   ltp/link_connect --tree tree-prt-iot --from ["OBS-008"] --to ["IO-010"]
   ```

4. Detallar en TT si existe:
   ```
   ltp/node_add --type DE --label "Operarios calibran sensores correctamente tras taller practico de 2 dias"
   ```
   → DE-015
   ```
   ltp/tree_attach --tree tree-tt-iot --node DE-015 --role root
   ltp/link_connect --tree tree-tt-iot --from ["IO-010"] --to ["DE-015"]
   ```

5. Vincular al NBR de origen:
   ```
   ltp/knowledge_add --label "NBR-012 absorbido en PRT como OBS-008" --type observation
   ltp/knowledge_link --id KN-010 --target OBS-008 --relation contextualizes
   ltp/knowledge_link --id KN-010 --target NBR-012 --relation contextualizes
   ```

6. Cerrar:
   ```
   ltp/validate
   ltp/history_end_batch
   ```

**Resultado**: NBR menor convertido en OBS-008 del PRT con IO-010 y paso en TT. El riesgo se gestiona operativamente sin modificar FRT.
**Meta-grafo**: `tree_relation { type: prerequisite, from: tree-nbr-iot, to: tree-prt-iot, handoff_nodes: [NBR-012 → OBS-008] }` [RFC-002]
**Capacidades RFC-002**: campo `source: nbr` en OBS para distinguir obstaculos descubiertos por riesgo vs obstaculos de implementacion.

#### T26 Flow — Startup Telemedicina: Evaluacion de Riesgos de la Norma (GT → NBR)

**Estado inicial**: GT "Telemedicina lider en Latam" completo con GOAL, OBJs (escalabilidad, compliance, calidad clinica) y NCs.
**Trigger**: Antes de diagnosticar la realidad (CRT), el analista quiere validar que la meta no tiene riesgos inherentes no obvios.

**Pasos**:

1. Revisar GT completo:
   ```
   ltp/tree_walk --tree_id tree-gt-telemedicina
   ```
   → GOAL-001, OBJ-001..OBJ-004, NC-001..NC-012

2. **[RFC-002: se necesita `nbr_add_proactive`]** — `nbr_add` requiere un FRT existente y una INJ como `source_node`. Para un risk check proactivo sobre el GT (sin FRT ni INJ), se propone un comando dedicado. Workaround con el motor actual — crear FRT hipotetico:
   ```
   ltp/history_begin_batch --label "T26: risk proactivo GT→NBR"
   ltp/tree_new --type frt --name "FRT hipotetico para risk check de GT"
   ```
   → tree-frt-risk-gt
   ```
   ltp/node_add --type INJ --label "Alcanzar liderazgo en telemedicina Latam (proxy del GOAL para risk check)"
   ```
   → INJ-050
   ```
   ltp/tree_attach --tree tree-frt-risk-gt --node INJ-050 --role root
   ltp/nbr_add --tree tree-frt-risk-gt --source_node INJ-050
   ```
   → NBR-001

3. Crear UDEs colaterales hipoteticas (riesgos de lograr la meta):
   ```
   ltp/node_add --type UDE --label "Un cambio regulatorio en un mercado clave invalida el modelo en 3 meses"
   ```
   → UDE-015
   ```
   ltp/node_add --type UDE --label "Regulaciones de telemedicina divergen por pais sin armonizacion"
   ```
   → UDE-016
   ```
   ltp/tree_attach --tree tree-frt-risk-gt --node UDE-015 --role leaf
   ltp/tree_attach --tree tree-frt-risk-gt --node UDE-016 --role leaf
   ```

4. Registrar insight como knowledge:
   ```
   ltp/knowledge_add --label "El GT necesita OBJ de resiliencia regulatoria multi-jurisdiccional" --type observation
   ```
   → KN-001
   ```
   ltp/knowledge_link --id KN-001 --target OBJ-001 --relation contextualizes
   ```

5. Cerrar batch y validar:
   ```
   ltp/validate
   ltp/history_end_batch
   ```

**Resultado**: NBR proactivo con 2 UDEs hipoteticas que revelan un OBJ faltante en el GT.
**Meta-grafo**: `[RFC-002] tree_relation { type: risk_check, from: tree-gt-telemedicina, to: NBR-001, handoff_nodes: [GOAL-001] }`
**Capacidades RFC-002**: Comando `nbr_add_proactive` que no requiera FRT padre — acepta GT y GOAL como contexto directamente. Campo `severity` en UDEs colaterales.

---

#### T27 Flow — Software: Anticipacion de Riesgos Pre-Solucion (CRT → NBR)

**Estado inicial**: CRT "Diagnostico monolito" con CRC identificada: RC-003 "Arquitectura monolitica sin tests".
**Trigger**: El analista sabe que cualquier intervencion sobre el monolito tocara codigo legacy — quiere anticipar riesgos genericos antes de elegir INJ.

**Pasos**:

1. Identificar CRC en el CRT:
   ```
   ltp/trace --tree tree-crt-monolito --node_id RC-003 --direction downstream
   ```
   → RC-003 conecta con 7 de 9 UDEs (78%)

2. **[RFC-002: se necesita `nbr_add_anticipatory` sin INJ concreta]** — `nbr_add` requiere `source_node` (INJ). Para anticipar riesgos genericos de tocar la CRC sin INJ definida, se propone un comando dedicado. Workaround — crear INJ placeholder y FRT minimo:
   ```
   ltp/history_begin_batch --label "T27: anticipacion riesgo pre-solucion"
   ltp/node_add --type INJ --label "Intervencion generica sobre CRC del monolito (placeholder para risk check)"
   ```
   → INJ-051
   ```
   ltp/tree_new --type frt --name "FRT minimo para anticipar riesgos del monolito"
   ```
   → tree-frt-anticipatorio
   ```
   ltp/tree_attach --tree tree-frt-anticipatorio --node INJ-051 --role root
   ltp/nbr_add --tree tree-frt-anticipatorio --source_node INJ-051
   ```
   → NBR-005

3. Crear UDEs colaterales genericas:
   ```
   ltp/node_add --type UDE --label "Cualquier refactoring causa regresiones en modulos no relacionados"
   ```
   → UDE-020
   ```
   ltp/node_add --type UDE --label "El equipo no puede verificar cambios por ausencia de tests automatizados"
   ```
   → UDE-021
   ```
   ltp/tree_attach --tree tree-frt-anticipatorio --node UDE-020 --role leaf
   ltp/tree_attach --tree tree-frt-anticipatorio --node UDE-021 --role leaf
   ```

4. Cerrar batch:
   ```
   ltp/validate
   ltp/history_end_batch
   ```

**Resultado**: NBR anticipatorio que informa la futura EC — buscar INJs con baja probabilidad de regresion.
**Meta-grafo**: `[RFC-002] tree_relation { type: risk_check, from: tree-crt-monolito, to: NBR-005, handoff_nodes: [RC-003] }`
**Capacidades RFC-002**: `nbr_type: anticipatory`. Comando `nbr_add_anticipatory` que vincule NBR a CRC sin requerir INJ concreta.

---

#### T28 Flow — Fabrica: Filtro Pre-FRT (EC → NBR)

**Estado inicial**: EC "Conflicto automatizacion" con 3 INJs candidatas tras invalidar distintos supuestos.
**Trigger**: Antes de construir FRT completo para cada INJ, el analista filtra por riesgo rapido.

**Pasos**:

1. Listar supuestos invalidados y sus INJs:
   ```
   ltp/assume_list --tree tree-ec-automatizacion
   ```
   → ASM-003 (invalidado) → INJ-001 "Robotizar linea completa"
   → ASM-007 (invalidado) → INJ-002 "Cobots colaborativos por estacion"
   → ASM-011 (invalidado) → INJ-003 "Outsourcing de produccion"

2. Crear FRT esqueletico por INJ y ejecutar NBR rapido:
   ```
   ltp/history_begin_batch --label "T28: filtro pre-FRT 3 INJs"
   ```

   Para INJ-001:
   ```
   ltp/tree_new --type frt --name "FRT skeleton robotizacion"
   ```
   → tree-frt-robot-skeleton
   ```
   ltp/tree_attach --tree tree-frt-robot-skeleton --node INJ-001 --role root
   ltp/nbr_add --tree tree-frt-robot-skeleton --source_node INJ-001
   ```
   → NBR-001
   ```
   ltp/node_add --type UDE --label "Despido masivo genera conflicto sindical y reputacional"
   ```
   → UDE-030
   ```
   ltp/tree_attach --tree tree-frt-robot-skeleton --node UDE-030 --role leaf
   ```

   Para INJ-002:
   ```
   ltp/tree_new --type frt --name "FRT skeleton cobots"
   ```
   → tree-frt-cobots-skeleton
   ```
   ltp/tree_attach --tree tree-frt-cobots-skeleton --node INJ-002 --role root
   ltp/nbr_add --tree tree-frt-cobots-skeleton --source_node INJ-002
   ```
   → NBR-002
   ```
   ltp/node_add --type UDE --label "Curva de adaptacion reduce productividad 3 meses"
   ```
   → UDE-031
   ```
   ltp/tree_attach --tree tree-frt-cobots-skeleton --node UDE-031 --role leaf
   ```

   Para INJ-003:
   ```
   ltp/tree_new --type frt --name "FRT skeleton outsourcing"
   ```
   → tree-frt-outsourcing-skeleton
   ```
   ltp/tree_attach --tree tree-frt-outsourcing-skeleton --node INJ-003 --role root
   ltp/nbr_add --tree tree-frt-outsourcing-skeleton --source_node INJ-003
   ```
   → NBR-003
   ```
   ltp/node_add --type UDE --label "Perdida de know-how de produccion critico e irreversible"
   ```
   → UDE-032
   ```
   ltp/tree_attach --tree tree-frt-outsourcing-skeleton --node UDE-032 --role leaf
   ```

3. Comparar y decidir:
   ```
   ltp/nbr_inspect --nbr_id NBR-001 --tree tree-frt-robot-skeleton
   ltp/nbr_inspect --nbr_id NBR-002 --tree tree-frt-cobots-skeleton
   ltp/nbr_inspect --nbr_id NBR-003 --tree tree-frt-outsourcing-skeleton
   ltp/history_end_batch
   ```
   → INJ-001: riesgo alto (social). INJ-003: riesgo alto (estrategico). INJ-002: riesgo bajo (temporal).
   → Se prioriza INJ-002 para FRT completo. INJ-003 descartada. INJ-001 en reserva.

**Resultado**: 3 NBRs rapidos, 1 INJ priorizada para FRT, 1 descartada, 1 en reserva.
**Meta-grafo**: `[RFC-002] tree_relation { type: risk_check, from: tree-ec-automatizacion, to: [NBR-001, NBR-002, NBR-003] }` con `risk_score` por NBR.
**Capacidades RFC-002**: `risk_score` comparativo por NBR. Ranking de INJs por riesgo residual.

---

#### T29 Flow — E-commerce: Verificacion de Ramas Negativas (FRT → NBR)

**Estado inicial**: FRT "Checkout 1-click" con INJ-005 "Tokenizacion de tarjetas + compra sin confirmacion" y DEs modelados.
**Trigger**: FRT completo — antes de pasar a implementacion hay que estresar cada cadena causal.

**Pasos**:

1. Revisar cadenas del FRT:
   ```
   ltp/tree_walk --tree_id tree-frt-checkout
   ```
   → INJ-005 → DE-001 "Conversion sube 15%" → DE-002 "Revenue +8%"
   → INJ-005 → DE-003 "Experiencia de compra fluida"

2. Crear NBR sistematico:
   ```
   ltp/history_begin_batch --label "T29: NBR sistematico checkout 1-click"
   ltp/nbr_add --tree tree-frt-checkout --source_node INJ-005
   ```
   → NBR-010

3. Para cada cadena, aplicar "Y SI...":
   ```
   ltp/node_add --type UDE --label "Tarjetas robadas permiten compras no autorizadas masivas"
   ```
   → UDE-040
   ```
   ltp/node_add --type UDE --label "Error en cantidad genera pedidos duplicados no detectados por el usuario"
   ```
   → UDE-041
   ```
   ltp/tree_attach --tree tree-frt-checkout --node UDE-040 --role leaf
   ltp/tree_attach --tree tree-frt-checkout --node UDE-041 --role leaf
   ltp/link_connect --tree tree-frt-checkout --from ["INJ-005"] --to ["UDE-040"]
   ltp/link_connect --tree tree-frt-checkout --from ["INJ-005"] --to ["UDE-041"]
   ```

4. Clasificar por severidad y decidir tratamiento:
   ```
   ltp/nbr_inspect --nbr_id NBR-010 --tree tree-frt-checkout
   ltp/history_end_batch
   ```
   → UDE-040: severidad critica → trimming necesaria (T24)
   → UDE-041: severidad media → absorcion operativa (T25)

**Resultado**: NBR con 2 UDEs colaterales clasificadas. UDE-040 escala a trimming, UDE-041 a gestion operativa.
**Meta-grafo**: `tree_relation { type: risk_check, from: tree-frt-checkout, to: NBR-010 }`
**Capacidades RFC-002**: `severity` por UDE colateral. Decision de routing: T24 vs T25 vs T23.

---

#### T30 Flow — Hospital: Riesgos de Implementacion (PRT/TT → NBR)

**Estado inicial**: TT "Migracion HIS" en ejecucion. Paso 3: IO-003 "Migrar historiales clinicos al nuevo sistema".
**Trigger**: Al detallar el paso de migracion, se descubren riesgos no anticipados por el NBR original del FRT.

**Pasos**:

1. Inspeccionar el paso del TT:
   ```
   ltp/node_inspect --node IO-003
   ```
   → "Migrar 2.4M historiales clinicos del HIS legacy al nuevo HIS en produccion"

2. Crear NBR de implementacion:
   ```
   ltp/history_begin_batch --label "T30: NBR implementacion migracion HIS"
   ltp/nbr_add --tree tree-frt-transcripcion --source_node INJ-008
   ```
   → NBR-015

3. Modelar riesgos especificos de implementacion:
   ```
   ltp/node_add --type UDE --label "Incompatibilidades de formato entre sistemas corrompen historiales clinicos"
   ```
   → UDE-050
   ```
   ltp/node_add --type UDE --label "Migracion en horario laboral causa indisponibilidad de historiales durante consultas"
   ```
   → UDE-051
   ```
   ltp/tree_attach --tree tree-frt-transcripcion --node UDE-050 --role leaf
   ltp/tree_attach --tree tree-frt-transcripcion --node UDE-051 --role leaf
   ```

4. Decidir tratamiento:
   ```
   ltp/nbr_inspect --nbr_id NBR-015 --tree tree-frt-transcripcion
   ltp/history_end_batch
   ```
   → UDE-050: critica → trimming (T24): "Migracion en staging con validacion cruzada antes de produccion"
   → UDE-051: media → ajuste del TT (T36): mover migracion a horario nocturno

**Resultado**: NBR de implementacion con 2 riesgos. Uno escala a trimming (FRT), otro ajusta el plan (TT).
**Meta-grafo**: `[RFC-002] tree_relation { type: risk_check, from: tree-tt-migracion, to: NBR-015 }`
**Capacidades RFC-002**: `nbr_type: implementation`. Vinculo NBR→paso_TT (no solo a INJ).

---

#### T31 Flow — Hospital: Revision Interna de la Norma (GT → GT)

**Estado inicial**: GT "Excelencia en atencion primaria" con NC-003 "Sistema de citas electronico".
**Trigger**: Al revisar el GT, el analista nota que NC-003 es una solucion disfrazada de necesidad.

**Pasos**:

1. Inspeccionar la NC sospechosa:
   ```
   ltp/node_inspect --node NC-003
   ```
   → label: "Sistema de citas electronico", type: NC

2. Verificar si es necesidad o solucion — test: "¿Sin ESTO ESPECIFICO es imposible lograr el OBJ?"
   → Sin sistema electronico, ¿es imposible? No — un sistema telefonico eficiente tambien podria cumplir.
   → NC-003 es una solucion, no una necesidad.

3. Reformular:
   ```
   ltp/history_begin_batch --label "T31: GT self-revision — NC solucion disfrazada"
   ltp/node_edit --id NC-003 --label "Los pacientes acceden a citas en menos de 48h sin fricciones"
   ```

4. Registrar la razon del cambio:
   ```
   ltp/knowledge_add --label "NC-003 era solucion disfrazada: 'sistema electronico' reformulada a 'acceso agil'" --type derived
   ```
   → KN-005
   ```
   ltp/knowledge_link --id KN-005 --target NC-003 --relation contextualizes
   ```

5. Validar coherencia del GT:
   ```
   ltp/validate
   ltp/history_end_batch
   ```

**Resultado**: GT corregido. NC-003 ahora expresa la necesidad real, no una solucion predeterminada.
**Meta-grafo**: `[RFC-002] tree_relation { type: self_revision, from: tree-gt-primaria, to: tree-gt-primaria, revision_count: 1 }`
**Capacidades RFC-002**: `revision_count` por auto-transicion para detectar `SELF_REVISION_LOOP` (>3 sin convergencia).

---

#### T32 Flow — Software: Profundizacion del Diagnostico (CRT → CRT)

**Estado inicial**: CRT "Diagnostico pipeline" con UDE-001 "Los deploys tardan >45 min" y cadena superficial.
**Trigger**: Al revisar, el analista sospecha que la cadena causal no llega a la RC real.

**Pasos**:

1. Revisar cadena actual:
   ```
   ltp/trace --tree tree-crt-pipeline --node_id UDE-001 --direction upstream
   ```
   → UDE-001 ← INT-001 "Tests end-to-end tardan 40 min" ← RC-001 "Suite de tests no optimizada"

2. Profundizar: ¿por que la suite no esta optimizada?
   ```
   ltp/history_begin_batch --label "T32: profundizar CRT — RC real"
   ltp/node_add --type INT --label "Nadie ha revisado la suite de tests en 3 anos"
   ```
   → INT-005
   ```
   ltp/tree_attach --tree tree-crt-pipeline --node INT-005 --role intermediate
   ltp/link_connect --tree tree-crt-pipeline --from ["INT-005"] --to ["RC-001"]
   ```

3. Seguir profundizando:
   ```
   ltp/node_add --type INT --label "No existe proceso de gobernanza de tests"
   ```
   → INT-006
   ```
   ltp/node_add --type RC --label "El equipo no tiene ownership definido sobre la calidad del pipeline"
   ```
   → RC-005
   ```
   ltp/tree_attach --tree tree-crt-pipeline --node INT-006 --role intermediate
   ltp/tree_attach --tree tree-crt-pipeline --node RC-005 --role root
   ltp/link_connect --tree tree-crt-pipeline --from ["INT-006"] --to ["INT-005"]
   ltp/link_connect --tree tree-crt-pipeline --from ["RC-005"] --to ["INT-006"]
   ```

4. Verificar nueva CRC:
   ```
   ltp/trace --tree tree-crt-pipeline --node_id RC-005 --direction downstream
   ```
   → RC-005 conecta (via cadena) con 8 de 9 UDEs (89%) — mejor que RC-001 (56%)

5. Validar:
   ```
   ltp/validate
   ltp/history_end_batch
   ```

**Resultado**: CRT profundizado. CRC cambia de "suite no optimizada" (superficial) a "falta de ownership del pipeline" (estructural).
**Meta-grafo**: `[RFC-002] tree_relation { type: extends, from: tree-crt-pipeline, to: tree-crt-pipeline }`
**Capacidades RFC-002**: Deteccion automatica de CRC (RC con mayor fan-out). Alerta si CRC cambia >2 veces.

---

#### T33 Flow — Software: Reformulacion del Conflicto (EC → EC)

**Estado inicial**: EC "Conflicto monolito" con D="Rewrite completo" vs D'="Mantener monolito".
**Trigger**: Al generar supuestos, todos son triviales ("rewrite es caro", "monolito es rigido") — el conflicto esta mal formulado.

**Pasos**:

1. Revisar supuestos actuales:
   ```
   ltp/assume_list --tree tree-ec-monolito
   ```
   → ASM-001 "Rewrite permite independencia de equipos" (trivial, obvio)
   → ASM-002 "Monolito permite coherencia de datos" (trivial, obvio)

2. Reformular D y D' a nivel mas profundo:
   ```
   ltp/history_begin_batch --label "T33: reformulacion EC — de tecnico a organizacional"
   ltp/node_edit --id PRE-001 --label "Cada equipo elige su stack y despliega independientemente"
   ltp/node_edit --id PRE-002 --label "Un equipo central gobierna arquitectura y estandares para todos"
   ```
   (PRE-001 = D, PRE-002 = D' en roles EC)

3. Eliminar supuestos triviales y regenerar:
   ```
   ltp/assume_rm --id ASM-001
   ltp/assume_rm --id ASM-002
   ```

4. Generar nuevos supuestos (ahora ricos e invalidables):
   ```
   ltp/assume_add --tree tree-ec-monolito --link LNK-004 --text "El conocimiento local de cada equipo es superior a cualquier estandar centralizado"
   ```
   → ASM-010
   ```
   ltp/assume_add --tree tree-ec-monolito --link LNK-005 --text "La coherencia arquitectonica solo se logra con control centralizado"
   ```
   → ASM-011
   ```
   ltp/assume_add --tree tree-ec-monolito --link LNK-005 --text "Los equipos autonomos no pueden coordinarse sin gobernanza top-down"
   ```
   → ASM-012

5. Validar:
   ```
   ltp/validate
   ltp/history_end_batch
   ```

**Resultado**: EC reformulada. D/D' pasan de nivel tecnico (rewrite/monolito) a organizacional (autonomia/coherencia). Supuestos ahora invalidables.
**Meta-grafo**: `[RFC-002] tree_relation { type: self_revision, from: tree-ec-monolito, to: tree-ec-monolito }`
**Capacidades RFC-002**: Metrica de "riqueza de supuestos". Alerta si D/D' cambian >2 veces → considerar T10.

---

#### T34 Flow — E-commerce: Iteracion sobre el Futuro (FRT → FRT)

**Estado inicial**: FRT "Checkout optimizado" con DE-001 "Conversion sube 15%". Solo efectos de primer orden.
**Trigger**: El analista quiere modelar efectos de segundo orden de la INJ para completar el FRT.

**Pasos**:

1. Revisar FRT actual:
   ```
   ltp/tree_walk --tree_id tree-frt-checkout
   ```
   → INJ-005 → DE-001 "Conversion +15%" → DE-002 "Revenue +8%"

2. Modelar cascada de segundo orden:
   ```
   ltp/history_begin_batch --label "T34: FRT iteracion — efectos segundo orden"
   ltp/node_add --type DE --label "Volumen de pedidos sube 30% por mayor conversion"
   ```
   → DE-010
   ```
   ltp/tree_attach --tree tree-frt-checkout --node DE-010 --role intermediate
   ltp/link_connect --tree tree-frt-checkout --from ["DE-001"] --to ["DE-010"]
   ```

3. Efecto de tercer orden (consecuencia):
   ```
   ltp/node_add --type INT --label "Almacen actual no tiene capacidad para +30% de pedidos"
   ```
   → INT-020
   ```
   ltp/node_add --type UDE --label "Tiempo de preparacion de pedidos supera SLA de 24h"
   ```
   → UDE-060
   ```
   ltp/tree_attach --tree tree-frt-checkout --node INT-020 --role intermediate
   ltp/tree_attach --tree tree-frt-checkout --node UDE-060 --role leaf
   ltp/link_connect --tree tree-frt-checkout --from ["DE-010"] --to ["INT-020"]
   ltp/link_connect --tree tree-frt-checkout --from ["INT-020"] --to ["UDE-060"]
   ```

4. Esto revela necesidad de INJ complementaria:
   ```
   ltp/node_add --type INJ --label "Contratar 3PL para picos de demanda superiores a capacidad propia"
   ```
   → INJ-010
   ```
   ltp/tree_attach --tree tree-frt-checkout --node INJ-010 --role intermediate
   ltp/link_connect --tree tree-frt-checkout --from ["INJ-010"] --to ["INT-020"]
   ```

5. Validar:
   ```
   ltp/validate
   ltp/history_end_batch
   ```

**Resultado**: FRT enriquecido con efectos de segundo/tercer orden. Revela necesidad de INJ complementaria para almacen.
**Meta-grafo**: `[RFC-002] tree_relation { type: extends, from: tree-frt-checkout, to: tree-frt-checkout }`
**Capacidades RFC-002**: Deteccion de profundidad de efectos (primer orden vs cascada).

---

#### T35 Flow — Banco: Recursion de Trimming (NBR → NBR)

**Estado inicial**: FRT "IA de creditos" con NBR-020 que identifica UDE colateral "El algoritmo sesga contra perfiles atipicos".
**Trigger**: La trimming INJ del NBR-020 tiene sus propios riesgos — necesita evaluacion recursiva.

**Pasos**:

1. Revisar NBR original y su trimming:
   ```
   ltp/nbr_inspect --nbr_id NBR-020 --tree tree-frt-creditos
   ```
   → UDE-070 "Algoritmo sesga contra perfiles atipicos" — Trimming: INJ-020 "Auditoria algoritmica trimestral"

2. Crear NBR sobre la trimming INJ:
   ```
   ltp/history_begin_batch --label "T35: NBR recursion nivel 2"
   ltp/nbr_add --tree tree-frt-creditos --source_node INJ-020
   ```
   → NBR-021

3. Modelar riesgos de la trimming:
   ```
   ltp/node_add --type UDE --label "El auditor externo no comprende el modelo y emite dictamen superficial"
   ```
   → UDE-071
   ```
   ltp/tree_attach --tree tree-frt-creditos --node UDE-071 --role leaf
   ```

4. Trimming de nivel 2:
   ```
   ltp/node_add --type INJ --label "Explicabilidad obligatoria del modelo con SHAP/LIME antes de cada auditoria"
   ```
   → INJ-021
   ```
   ltp/link_connect --tree tree-frt-creditos --from ["INJ-021"] --to ["UDE-071"]
   ```

5. Evaluar si necesita nivel 3:
   ```
   ltp/nbr_add --tree tree-frt-creditos --source_node INJ-021
   ```
   → NBR-022
   ```
   ltp/node_add --type UDE --label "Generar explicaciones SHAP anade 2h al proceso de scoring"
   ```
   → UDE-072
   ```
   ltp/tree_attach --tree tree-frt-creditos --node UDE-072 --role leaf
   ```
   → Severidad: baja (operativa, gestionable). La cadena converge.

6. Cerrar:
   ```
   ltp/nbr_inspect --nbr_id NBR-022 --tree tree-frt-creditos
   ltp/history_end_batch
   ```

**Resultado**: Recursion de 3 niveles. Severidad: NBR-020 (critica) → NBR-021 (media) → NBR-022 (baja). Converge — se acepta riesgo residual.
**Meta-grafo**: `[RFC-002] tree_relation { type: extends, from: NBR-020, to: NBR-021, to: NBR-022, recursion_depth: 3 }`
**Capacidades RFC-002**: `recursion_depth` por cadena de NBR. `TRIMMING_RECURSION_DEPTH` alerta si depth >= 2 sin disminucion de severidad.

---

#### T36 Flow — Software: Revision del Plan (PRT/TT → PRT/TT)

**Estado inicial**: TT "Migracion a microservicios" con pasos paralelos: IO-001 "Migrar DB a nuevo schema" y IO-002 "Migrar servicio de usuarios".
**Trigger**: Al detallar el TT, se descubre que el servicio depende del nuevo schema — no pueden ser paralelos.

**Pasos**:

1. Inspeccionar pasos actuales:
   ```
   ltp/node_inspect --node IO-001
   ```
   → "Migrar schema de base de datos a PostgreSQL 16"
   ```
   ltp/node_inspect --node IO-002
   ```
   → "Migrar servicio de usuarios a contenedor independiente"

2. Descubrir dependencia:
   ```
   ltp/trace --tree tree-tt-microservicios --node_id IO-002 --direction upstream
   ```
   → IO-002 no tiene dependencia explicita de IO-001, pero el servicio usa tablas del nuevo schema.

3. Reordenar — anadir dependencia:
   ```
   ltp/history_begin_batch --label "T36: reordenar TT por dependencia descubierta"
   ltp/link_connect --tree tree-tt-microservicios --from ["IO-001"] --to ["IO-002"]
   ```

4. Registrar la razon:
   ```
   ltp/knowledge_add --label "IO-002 depende de IO-001: servicio de usuarios requiere nuevo schema PostgreSQL" --type derived
   ```
   → KN-010
   ```
   ltp/knowledge_link --id KN-010 --target IO-001 --relation contextualizes
   ltp/knowledge_link --id KN-010 --target IO-002 --relation contextualizes
   ```

5. Validar secuencia:
   ```
   ltp/validate
   ltp/tree_walk --tree_id tree-tt-microservicios
   ltp/history_end_batch
   ```
   → Ahora: IO-001 → IO-002 (secuencial, no paralelo)

**Resultado**: TT reordenado. DB migra primero, servicio despues. Dependencia explicita en el arbol.
**Meta-grafo**: `[RFC-002] tree_relation { type: self_revision, from: tree-tt-microservicios, to: tree-tt-microservicios }`
**Capacidades RFC-002**: Deteccion de dependencias implicitas entre pasos del TT.

---

## Changelog

| Fecha | Cambio |
|-------|--------|
| 2026-09-13 | Correccion masiva de ~90 parametros MCP en 36 flujos: `link_connect` (añadido `--tree`, `from`/`to` como arrays), `tree_walk` (`--tree_id`), `trace` (`--node_id` + `--tree`), `tree_attach` (roles: root/leaf/intermediate), `knowledge_add`/`knowledge_link`/`nbr_add`/`nbr_inspect`/`assume_add`/`invalidate`/`node_edit` con parametros verificados contra schemas MCP reales. CSF→OBJ, ACT→IO. T26/T27/T28 reescritos para workaround `nbr_add` sin FRT (marcados [RFC-002]). UDE-COL-xxx→UDE-xxx. |
| 2026-09-13 | Flujos detallados: 36 flujos con comandos MCP reales, uno por transicion, organizados por grupo (A-G). Incluyen estado inicial, trigger, pasos, resultado, meta-grafo y capacidades RFC-002 necesarias. |
| 2026-09-12 | Creacion: 108 casos de uso (3 por transicion, 36 transiciones). Corregidos T02.1 (tension generica → conflicto especifico), T12.3 (tarea operativa → INJ menor de EC), T25.2 (falsos positivos → PRs enormes), T26.1 (GT incompleto → riesgo no obvio de meta bien formada). |
