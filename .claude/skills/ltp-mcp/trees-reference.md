# Trees Reference — Reglas por Tipo + Handoffs

Referencia detallada de cada tipo de arbol LTP y la matriz de transiciones cross-tree.

## Goal Tree (GT) — Necesidad

- **Estructura:** 1 GOAL (cuspide) -> 3-5 OBJ (CSFs) -> multiples REQ (NCs).
- **Sintaxis:** "Para lograr [Goal/CSF], debo obligatoriamente cumplir [NC]."
- **Redaccion:** Presente de rendimiento sostenido. Condiciones medibles.
- **Anti-patrones:** Multiples GOALs; acciones en vez de resultados; feel-good phrases; NCs fuera de esfera de influencia.

## Current Reality Tree (CRT) — Suficiencia

### 10 Reglas de UDEs

1. Hecho presente comprobable
2. Negativo per se
3. NUNCA "ausencia de solucion"
4. Oracion atomica S+V+O
5. Sin palabras causales embutidas
6. Sin culpas personales
7. Viola un CSF/NC del GT
8. Dentro de esfera de influencia
9. Presente indicativo
10. Verificable por consenso

### Construccion

- **CRC:** Causa raiz que conecta downstream con >70% de UDEs (usar `ltp/trace --direction downstream`).
- **Direccion:** Bottom-up. Preguntar "por que?" para descender a causas.
- **Anti-patrones:** Ausencia de solucion en la base; causa fuera de control; flechas largas sin intermedios.

## Evaporating Cloud (EC) — Necesidad

- **5 nodos:** [A] Objetivo comun (role=objective), [B,C] Necesidades (role=requirement), [D,D'] Prerrequisitos en conflicto (role=prerequisite) con XOR.
- **3 tipos:** Nube de UDE, Dilema, Fire-fighting.
- **Inyecciones:** Deben romper un supuesto, satisfacer B y C, y estar dentro de esfera de influencia.
- **Anti-patrones:** Espejo (B=C invertido); solucion en D; necesidades "away from" en vez de "toward".

Para protocolo de auditoria y supuestos completo: ver `ec-protocol.md`.

## Future Reality Tree (FRT) — Suficiencia

- **Tiempo verbal:** PRESENTE indicativo (realidad ya instaurada).
- **Sintaxis:** "Si [Inyeccion], entonces [Efecto Deseado]."
- **Matriz 1:1:** 100% de UDEs del CRT deben tener un DE opuesto en el FRT.
- **Feedback loops:** `ltp/link_feedback` (no rompe DAG).
- **Anti-patrones:** Confundir FRT con PRT/TT; ignorar NBRs; inyecciones magicas sin origen en EC.

## NBR (Negative Branch Reservations) — Suficiencia

- **Anatomia:** Rama causal aislada que modela riesgo colateral de una INJ.
- **Trimming (6 pasos):**
  1. Identificar NDE (efecto negativo)
  2. Trazar cadena desde INJ hasta NDE
  3. Crear Trimming INJ que corte la rama
  4. Verificar que no compromete DEs del tronco
  5. Adjuntar trim
  6. Validar
- **Tools:** `ltp/nbr_add`, `ltp/nbr_inspect`, `ltp/nbr_list`, `ltp/nbr_rm`.

## Prerequisite Tree (PRT) — Necesidad

- **Estructura:** Pares OBS (obstaculo) -> IO (objetivo intermedio que lo vence).
- **Ordenacion:** Cronologica estricta (que IO primero).
- **Cuspide:** La INJ validada del FRT (adjuntada con role=objective).
- **Anti-patrones:** Obstaculos sin IO; mezclar logica de suficiencia.

## Transition Tree (TT) — Suficiencia

- **5 elementos por paso:**
  1. Realidad actual presente
  2. Necesidad no satisfecha (rationale)
  3. Accion tactica concreta
  4. Nueva realidad esperada
  5. Necesidad siguiente (conector al proximo paso)
- **Granularidad:** Instrucciones ejecutables por una persona sin contexto adicional.
- **Anti-patrones:** Pasos abstractos; saltar la necesidad no satisfecha; ejecucion rigida sin feedback.

## Handoffs Cross-Tree

| # | Transicion | Trigger | Objeto transferido | Validacion |
|---|-----------|---------|-------------------|-----------|
| 1 | GT -> CRT | Evaluar estado actual vs norma | Violacion CSF/NC -> UDE | Cada UDE viola un CSF/NC del GT |
| 2 | CRT -> EC | CRC identificada (>70% UDEs) | Politica que genera la CRC | EC responde "por que se tolera esta causa?" |
| 3 | EC -> FRT | Supuesto invalidado | INJ aprobada | Matriz 1:1 UDE->DE completa |
| 4 | FRT -> NBR | NDE detectado | Rama causal negativa | Trimming INJ corta antes del NDE |
| 5 | FRT/EC -> PRT | INJ validada e inmunizada | INJ como objetivo superior | PRT en necesidad, IOs en secuencia |
| 6 | PRT -> TT | Red de IOs establecida | IO individual | TT en suficiencia, 5 elementos/paso |

**Mecanica MCP:** Siempre `ltp/tree_attach --tree <DEST> --node <ID> --role <ROL>`. Nunca `ltp/node_add` con texto duplicado.
