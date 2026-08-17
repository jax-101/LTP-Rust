# EC Protocol — Auditoria + Supuestos (Fedurko)

Referencia completa para Evaporating Clouds. Leer cuando se construye, audita, o genera supuestos para una EC.

## Estructura de la EC

**5 nodos:** [A] Objetivo comun (role=objective), [B,C] Necesidades (role=requirement), [D,D'] Prerrequisitos en conflicto (role=prerequisite) conectados con XOR.

**3 tipos:** Nube de UDE, Dilema, Fire-fighting.

## Protocolo de Auditoria (5 Checks)

Ejecutar en orden estricto. Si un check falla, DETENERSE e informar.

### Check 0 — Pre-check Estructural (motor)

- `ltp/validate --tree <ec-tree>`
- Verifica: objective=1, requirements>=2, prerequisite por requirement, DAG valido.
- Si falla: `ltp/node_add` + `ltp/tree_attach --role <rol>`.

### Check 1 — Rigurosidad de Redaccion

| Regla | Aplica a | Violacion |
|-------|----------|-----------|
| Sin palabras causales ("porque", "por lo tanto", "debido a", "para que", "ya que") | Todos (A,B,C,D,D') | "Reducir costes *porque* el margen cae" |
| Fraseo positivo (sin negaciones) | A, B, C (no D,D') | "No perder clientes" -> "Retener clientes" |
| Oracion atomica (S+V+O) | Todos | Compuestas con "y/o" -> splitear |

D y D' pueden contener negaciones (son acciones/wants).

Accion: `ltp/node_edit --label "<correccion>"` o `ltp/node_split`.

### Check 2 — Logica Vertical ("Lectura en Voz Alta")

Leer en esta secuencia exacta:

1. "Para lograr **[A]**, necesito obligatoriamente **[B]**."
2. "Para satisfacer **[B]**, debo hacer **[D]**."
3. "Para lograr **[A]**, necesito obligatoriamente **[C]**."
4. "Para satisfacer **[C]**, debo hacer **[D']**."

Criterio de rechazo: si suena forzada, condicional ("a veces") o evitable ("podria lograrse de otra forma obvia").

Accion: `ltp/node_edit` o `ltp/link_insert_between`.

### Check 3 — Conflicto Directo (D vs D')

Verificar que D y D' son incompatibles **en el contexto real del sistema**.

Prueba: "En el contexto real, es viable ejecutar D y D' sin que uno anule al otro?" Si SI -> conflicto no es real.

**Tipos validos:**
- Exclusion fisica (imposible simultaneamente)
- Exclusion de recurso (mismo presupuesto/equipo/tiempo finito)
- Exclusion logica (uno contradice la premisa del otro)

**Causa comun de fallo:** D y D' son preferencias graduales ("invertir mas" vs "invertir menos") en vez de acciones discretas incompatibles.

Accion: `ltp/node_edit` sobre D y/o D'.

### Check 4 — Cross-Check de Amenaza (Endangerment — el mas critico)

Verificar relaciones diagonales:
- "Hacer **[D]** pone en peligro **[C]**?" -> debe ser SI
- "Hacer **[D']** pone en peligro **[B]**?" -> debe ser SI

**Adversarial guard (obligatorio):** Tras afirmar amenaza, intentar REFUTAR:
- "Existe escenario realista donde D NO amenace C?"
- Solo confirmar si no hay contra-argumento solido.
- Refutacion facil -> la necesidad probablemente es falsa.

Si una diagonal NO se cumple:
1. La necesidad (B o C) es una "excusa noble" — suena razonable pero no es la fuerza motriz real.
2. Preguntar: "Por que realmente hago [D/D']? Que pierdo si no lo hago?"
3. Reemplazar con la necesidad genuina.

Accion: `ltp/node_edit --node <B|C> --label "<necesidad real>"`.

### Resultado de la Auditoria

- **Todos pasan** -> "EC validada", proceder a supuestos.
- **Fallo** -> mostrar: (1) que check, (2) propuesta, (3) pedir confirmacion.
- **Override usuario** -> registrar advertencia, continuar.

## Secuencia Completa de Construccion

```
1. Construir:  tree_new(ec) -> node_add x 5 -> tree_attach(roles) -> link_connect(XOR entre D,D')
2. Auditar:    Check 0-4 en orden
3. Supuestos:  Generar segun protocolo abajo
4. Rankear:    Identificar supuesto mas fragil
5. Romper:     invalidate el supuesto mas fragil -> genera INJ
6. Validar:    validate + verificar INJ satisface B y C sin conflicto
```

## Protocolo de Generacion de Supuestos

### Mapeo de Flechas a Edges

| Flecha | Lectura | Edge MCP |
|--------|---------|----------|
| B->A | "Para lograr A, necesito B" | Edge de B hacia A (SINGLE) |
| C->A | "Para lograr A, necesito C" | Edge de C hacia A (SINGLE) |
| D->B | "Para satisfacer B, debo hacer D" | Edge de D hacia B (SINGLE) |
| D'->C | "Para satisfacer C, debo hacer D'" | Edge de D' hacia C (SINGLE) |
| D<->D' | "D y D' son incompatibles porque..." | Edge XOR entre D y D' |

### Reglas para Flechas Rectas (B->A, C->A, D->B, D'->C)

**Regla 1 — Palabras Referentes:** El supuesto DEBE contener palabras que refieran a ambos nodos E introducir un **tercer elemento** (agente, concepto, factor) que los vincule.
- ✅ "El departamento legal [3er] exige certificacion [B] para aprobar operaciones [A]"
- ❌ "Necesitamos B para lograr A" (repeticion sin tercer elemento)

**Regla 2 — Explicar vinculo, no entidad:** Explica POR QUE cola es medio necesario para punta.
- ❌ "B es importante porque..."
- ✅ "Dado que [3er elemento], B es la unica via hacia A"

**Regla 3 — Mecanismo Prevalente:** "Cual es el mecanismo actual por el cual logramos [punta] a traves de [cola]?"
Categorias: politica organizacional, limitacion tecnologica, creencia arraigada, dependencia temporal.

**Regla 4 — Test "Si... y si... entonces...":**
> "Si queremos **[punta]**, Y SI **[supuesto]**, entonces debemos **[cola]**."
- Debe ser logicamente inevitable.
- Lo de "y si" debe ser genuinamente diferente de lo de "si".

### Reglas para Flecha de Conflicto (D<->D')

**Regla C1 — Sintaxis Bilateral:** SIEMPRE usar:
> "**[D]** Y **[D']** estan en conflicto porque... [supuesto]"
- ❌ PROHIBIDO: "D esta en conflicto con D' porque..." (sesga)

**Regla C2 — "Que nos falta?":** Preguntar que NO tenemos que impide eliminar el conflicto.
Categorias: falta de conocimiento/metodo, reglas/procedimiento, confianza, disposicion a cooperar, recurso.

Los supuestos D<->D' se adjuntan al edge XOR.

### 7 Errores Fatales (gate de descarte)

Verificar ANTES de `ltp/assume_add`. Si incurre en alguno, descartar y regenerar:

| # | Error | Deteccion rapida |
|---|-------|-----------------|
| 1 | Repeticion inversa | Solo invierte la flecha? -> DESCARTAR |
| 2 | "Es la unica forma" | Anade entendimiento nuevo? Si no -> DESCARTAR |
| 3 | Explicar solo un nodo | Menciona ambos + tercer elemento? Si no -> DESCARTAR |
| 4 | "Para lograr que?" | Establece objetivo en vez de explicar vinculo? -> DESCARTAR |
| 5 | Supuesto irrelevante | Pasa Regla 4? Si no -> DESCARTAR |
| 6 | Sintaxis sesgada D<->D' | Usa forma bilateral (C1)? Si no -> REFORMULAR |
| 7 | Cop-out "mismo tiempo" | Explica CAUSA del conflicto o solo lo define? Si define -> DESCARTAR |

**Error #7 vs Check 3:** Check 3 VERIFICA que D,D' sean excluyentes (necesario). Error #7 prohibe USAR esa exclusividad como supuesto (no explica POR QUE).

### Protocolo Operativo

```
Por cada flecha:
1. Generar candidatos (Reglas 1+2+3, o C1+C2 para D<->D')
2. Validar con Regla 4
3. Filtrar contra 7 Errores Fatales
4. Repetir hasta loop-until-dry (sin supuestos nuevos que pasen filtros)
5. Rankear por fragilidad
6. ltp/assume_add (mas fragil primero)
7. Comunicar candidato a invalidacion y por que
```
