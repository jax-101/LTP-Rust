---
name: ltp-thinking-process
description: Guides Claude to build rigorous Logical Thinking Process (Dettmer) analyses using the `ltp` CLI tool. Covers methodology, tree types, CLR validation, and command patterns.
user_invocable: true
---

# Logical Thinking Process — Skill para Claude Code

Eres un facilitador experto del Logical Thinking Process (LTP) de H. William Dettmer. Usas el CLI `ltp` (disponible en PATH) para construir y validar modelos causa-efecto rigurosos.

## Marco Conceptual

El LTP es un sistema de 5 árboles lógicos que se construyen en secuencia:

```
Goal Tree (GT) → Current Reality Tree (CRT) → Evaporating Cloud (EC) → Future Reality Tree (FRT) → Prerequisite Tree (PRT) → Transition Tree (TT)
```

| Árbol | Propósito | Lógica | Tipo en ltp |
|-------|-----------|--------|-------------|
| **GT** | Define el objetivo del sistema y condiciones necesarias | Necesidad | `GT` |
| **CRT** | Mapea la realidad actual: UDEs conectadas a Root Causes | Suficiencia | `CRT` |
| **EC** | Expone el conflicto raíz que perpetúa las causas | Necesidad | `EC` |
| **FRT** | Diseña la realidad futura: inyecciones que eliminan causas | Suficiencia | `FRT` |
| **PRT** | Identifica obstáculos y objetivos intermedios para cada inyección | Necesidad | `PRT` |
| **TT** | Plan de acción paso a paso con acciones secuenciales | Suficiencia | `TT` |

## Tipos de Nodo

| Tipo | Significado | Uso típico |
|------|-------------|------------|
| `UDE` | Efecto Indeseable (observable) | CRT: síntomas del problema |
| `DE` | Efecto Deseable | FRT: resultados esperados |
| `RC` | Root Cause (causa raíz) | CRT: fondo de la cadena causal |
| `INJ` | Inyección (acción nueva) | FRT: solución propuesta |
| `INT` | Entidad Intermedia | Pasos intermedios en cadenas |
| `NC` | Condición Necesaria | EC, GT |
| `GOAL` | Objetivo del sistema | GT |
| `OBJ` | Objetivo intermedio | PRT |
| `WANT` | Deseo / requisito | EC |
| `OBS` | Obstáculo | PRT |
| `IO` | Objetivo Intermedio | PRT |
| `REQ` | Requisito | EC |
| `PRE` | Prerrequisito | EC |

## Operadores de Enlace

| Operador | Significado |
|----------|-------------|
| `SINGLE` | Causa única suficiente |
| `AND` | Causas dependientes (todas necesarias, elipse) |
| `OR` | Causas adicionales independientes (cada una suficiente) |
| `MAG` | Magnitudinario (causas aditivas, cada una aporta %) |
| `XOR` | Mutuamente excluyentes |

## Flujo de Trabajo con el CLI

### 1. Inicializar workspace

```bash
ltp init --name "Nombre del análisis"
```

### 2. Crear nodos (pool global)

```bash
ltp node add "El cliente recibe pedidos con retraso" --type UDE
ltp node add "No existe stock de seguridad" --type RC
ltp node add "Implementar buffer de inventario" --type INJ
```

### 3. Crear árbol y vincular nodos

```bash
ltp tree new --type CRT --name "cadena-suministro"
ltp tree attach --tree tree-crt-cadena-suministro --node UDE-001
ltp tree attach --tree tree-crt-cadena-suministro --node RC-001
```

### 4. Conectar causalmente

```bash
# RC-001 causa UDE-001 (suficiencia simple)
ltp link connect --tree tree-crt-cadena-suministro --from RC-001 --to UDE-001

# Causas dependientes (AND): ambas necesarias para el efecto
ltp link connect --tree tree-crt-cadena-suministro --from RC-001,RC-002 --to UDE-003 --operator AND

# Causas magnitudinarias: cada una aporta un % al efecto
ltp link connect --tree tree-crt-cadena-suministro --from RC-001,RC-002 --to UDE-004 --operator MAG
```

### 5. Validar

```bash
ltp validate --tree tree-crt-cadena-suministro
```

### 6. Trazar cadenas causales

```bash
ltp trace --node UDE-001 --tree tree-crt-cadena-suministro --direction upstream --depth 10
```

### 7. Supuestos e invalidación

```bash
ltp assume add --link LINK-001 --text "Asumimos que el proveedor mantiene su lead time"
ltp invalidate --assumption ASM-001 --tree tree-crt-cadena-suministro --injection "Negociar contrato con penalización"
```

### 8. NBR (Negative Branch Reservations)

Cuando una inyección podría causar efectos negativos:

```bash
ltp nbr add --tree tree-frt-solucion --source INJ-001
ltp node add "Podría generar sobrestock" --type UDE
ltp link connect --tree tree-frt-solucion --nbr NBR-001 --from INJ-001 --to UDE-005
ltp node add "Establecer nivel máximo de inventario" --type INJ
# El trim corta la rama negativa
```

### 9. Path abstraction (vistas ejecutivas)

```bash
# Colapsar cadena detallada en un macro-enlace para vista ejecutiva
ltp path collapse --tree tree-crt-cadena-suministro --from RC-001 --to UDE-001

# Explotar supuesto en nodo intermedio
ltp path explode --tree tree-crt-cadena-suministro --assumption ASM-001

# Reemplazar sub-grafo táctico por inyección
ltp path replace --tree tree-crt-cadena-suministro --from RC-001 --to UDE-001 --injection INJ-001
```

### 10. Undo/Redo

```bash
ltp undo              # Deshacer última operación
ltp redo              # Rehacer
ltp history --last 5  # Ver últimas 5 operaciones
ltp history begin-batch --label "Construir CRT completo"
# ... múltiples operaciones ...
ltp history end-batch  # Todo se deshace como una unidad
```

## Las 8 Categorías de Legitimación Lógica (CLR)

Aplica estas pruebas a cada entidad y conexión:

1. **Claridad**: ¿Se entiende sin explicación adicional? ¿Hay saltos lógicos (flecha larga)?
2. **Existencia de Entidad**: ¿Oración completa (sujeto+verbo)? ¿Una sola idea? ¿Sin "porque"/"para"? ¿Evidencia real?
3. **Existencia de Causalidad**: ¿"Si A, entonces B" es directo e inevitable?
4. **Insuficiencia de Causa**: ¿Falta algún factor dependiente? → Usar operador AND
5. **Causa Adicional**: ¿Hay otra causa independiente que produce el mismo efecto? → Conectar por separado
6. **Inversión Causa-Efecto**: ¿La flecha apunta en la dirección correcta? ¿No se confunde síntoma con causa?
7. **Efecto Predicho**: Si la causa es intangible, ¿hay un segundo efecto observable que la confirme?
8. **Tautología**: ¿Se justifica la causa citando el mismo efecto?

El motor `ltp validate` detecta automáticamente:
- CLR#2: conjunciones prohibidas en labels (warning)
- CLR#4: nodo con 1 sola entrada SINGLE (posible insuficiencia)
- CLR#4/#5: AND con >4 entradas (mezcla de causas independientes)
- CLR#6: inversión de tipos (UDE→RC)
- CLR#7: causa intangible sin efecto predicho
- CLR#8: ciclos (DFS 3 colores, bloqueante)
- Huérfanos: nodos sin conexiones

## Principios de Construcción

1. **Empezar por los UDEs observables**: lo que el usuario ve/sufre/mide.
2. **Preguntar "¿por qué?"** para descender a causas intermedias y root causes.
3. **Un nodo = una idea atómica**. Si tiene "y" o "porque", dividir.
4. **Validar frecuentemente**: `ltp validate` tras cada grupo de conexiones.
5. **Los nodos son globales**: un mismo RC puede aparecer en múltiples árboles (compartido).
6. **El motor NO juzga semántica**: tú (Claude) evalúas causalidad, claridad y suficiencia aplicando las CLR.
7. **Usar `--dry-run`** antes de operaciones destructivas para previsualizar el resultado.
8. **Batch undo** para secuencias largas: `history begin-batch` / `end-batch`.

## Output del CLI

Todos los comandos retornan JSON por defecto:

```json
{
  "success": true/false,
  "action": "nombre_comando",
  "workspace": "nombre",
  "data": { ... },
  "graph_health": { "valid_dag": true, "orphan_nodes_count": 0 },
  "errors": [],
  "warnings": []
}
```

Usa `--human` para salida legible cuando el usuario necesite leer directamente.

## Estructura del Workspace en Disco

```
mi-analisis/
├── ltp.config.json       # Config global
├── nodes/                # UDE-001.json, RC-001.json, ...
├── trees/                # tree-crt-nombre.json, ...
└── .ltp/                 # Estado interno (undo, redo, counters)
```
