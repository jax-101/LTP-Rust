Registro de Decisiones de Arquitectura (ADR) - ltp-engineEste documento registra el contexto, las alternativas evaluadas, los trade-offs y las decisiones técnicas fundamentales tomadas durante el diseño del motor ltp-engine.ADR-001: Separación Estricta de Responsabilidades (LLM Semántico vs. Motor Determinista)ContextoLos Modelos de Lenguaje (LLMs) destacan analizando texto desordenado, infiriendo relaciones causa-efecto semánticas e identificando inconsistencias del mundo real. Sin embargo, son probabilísticos y propensos a alucinar estados cuando deben garantizar la integridad de grafos, prevenir ciclos dirigidos o gestionar identificadores únicos en conversaciones extensas.DecisiónAdoptar la arquitectura Motor Determinista CLI + Agente Semántico LLM.ConsecuenciasPositivas: 0% alucinaciones de estado o topología. El motor valida matemáticamente la coherencia de los árboles.Negativas: Exige definir una API de CLI rica, expresiva y con salidas JSON altamente estructuradas.ADR-002: Persistencia Modular en JSON Canónico vs. Base de Datos SQLiteContextoEl motor necesita almacenar la estructura de nodos y enlaces de forma persistente. Se evaluó el uso de SQLite (por sus consultas de grafos relacionales) frente a texto plano en el sistema de archivos.DecisiónAlmacenar el proyecto en archivos JSON modulares e indentados de forma canónica (claves ordenadas alfabéticamente).JustificaciónSQLite es un binario que destruye la capacidad de Git para realizar git diff línea por línea.Con JSONs modulares, un Agente LLM o un usuario puede crear una rama de Git (git checkout -b hipotesis-inyeccion-b), modificar un par de archivos y comparar el razonamiento de forma nativa mediante Git.Un árbol LTP típico rara vez supera los 500 nodos. Parsear los archivos en memoria toma menos de 5ms, haciendo innecesaria la velocidad de indexación de SQLite.ADR-003: Inclusión de MAG como Operador de Causas Aditivas (Revisado)ContextoEl LTP de Dettmer reconoce explícitamente causas independientes pero aditivas (magnitudinarias): cada una aporta un porcentaje al efecto graduado. Eliminando una causa el efecto se reduce pero no desaparece. Esto se sitúa entre AND (dependencia total) y OR (independencia total al 100%). Se evaluó inicialmente excluir MAG argumentando que destruye la capacidad del linter para auditar CLR #4 (Insuficiencia). Este argumento se revisó: CLR #4 no aplica a MAG por definición (es una distinción semántica legítima). MAG es topológicamente idéntico a OR. No rompe el DAG ni complica validate. Dettmer lo documenta como variación de Causa Adicional (CLR #5).DecisiónIncluir MAG como operador válido: ltp link connect --operator MAG. El motor lo trata topológicamente como OR a efectos de validación (no aplica regla de insuficiencia CLR #4).ConsecuenciasPositivas: Modelado fiel de sistemas donde las causas son aditivas proporcionales. Permite estrategias Pareto (priorizar por mayor contribución). trace y status reportan causas aditivas. El agente LLM puede razonar sobre contribución relativa.Negativas: El operador por sí solo no captura el % de contribución — esa estimación recae en metadata opcional del edge (campo weight). Decisión de incluir weight se difiere a implementación.ADR-004: Modelo "Ground Truth" Unidireccional con Mutaciones por IntenciónContextoEn el LTP es indispensable trabajar a nivel táctico/detallado y a nivel ejecutivo/resumido. Permitir la edición bidireccional directa sobre una vista ejecutiva genera el View Update Problem (ambigüedad al reestructurar los nodos intermedios ocultos).DecisiónAdoptar Unidireccionalidad Estricta de Datos con Comandos de Mutación por Intención.Los archivos en disco siempre guardan el grafo táctico de máxima resolución (Bottom-Up).La vista ejecutiva es una proyección calculada al vuelo en datos (Render).Si se desea modificar la estructura desde el nivel ejecutivo, se invocan comandos con intención explícita (replace, bypass, invalidate), los cuales ejecutan un algoritmo determinista para actualizar el sub-grafo táctico.ADR-005: Supuestos (Assumptions) como Entidades de Primer NivelContextoEn herramientas tradicionales, los supuestos son comentarios o texto secundario pegado a una línea.DecisiónTratar los supuestos como entidades con identidad propia que viven dentro de la definición del enlace (edge).JustificaciónEn el LTP, la resolución de conflictos (Evaporating Cloud) y la prevención de Ramas Negativas se basan en invalidar supuestos ocultos. Hacerlos direccionables por ID permite al LLM ejecutar comandos como ltp link invalidate --assumption-id ASM-01, provocando la ruptura controlada de la flecha y exigiendo la creación de una Inyección.ADR-006: Omisión Temporal de Representación Visual / Enfoque Headless PuroContextoSe evaluó incluir motores de renderizado gráfico (Mermaid.js, Graphviz DOT, mapas en código ASCII) en las primeras fases del desarrollo del motor.DecisiónPostergación de capas visuales. El motor se centra exclusivamente en la manipulación headless de grafos causales, contrato JSON y protocolo MCP.JustificaciónLos agentes LLM procesan y razonan sobre estructuras lógicas mucho más eficientemente mediante JSON estructurado / ASTs que parseando o generando sintaxis de diagramación visual.Evita acoplar el motor determinista a librerías de renderizado o layouts de coordenadas en fases tempranas.El enfoque headless puro garantiza que ltp-engine sea 100% ligero, rápido e integrable en cualquier pipeline backend de agentes AI.

ADR-007: Feedback Loops como Pool Separado de Aristas (`feedback_edges`)

Contexto

El LTP de Dettmer reconoce bucles de retroalimentación (positivos/reinforcing y negativos/balancing) tanto en CRT como en FRT. Estos constituyen ciclos dirigidos legítimos en el modelo causal. Sin embargo, el motor implementa validación DAG estricta (DFS 3 colores) que bloquea cualquier ciclo en `edges`.

Se evaluaron tres opciones:
1. Permitir ciclos anotados en `edges` con exención del DFS → rompe el invariante DAG universal y complica la distinción entre ciclo válido (feedback) y ciclo inválido (tautología CLR #8).
2. Nodo proxy `LOOP_MARKER` que rompe el ciclo topológicamente → introduce entidades artificiales ajenas al dominio de Dettmer.
3. Pool separado `feedback_edges` en el tree → preserva DAG puro en `edges`, sin entidades ficticias.

Decisión

Adoptar un pool separado `feedback_edges[]` dentro de cada vista topológica. Las aristas de retroalimentación no participan en la validación DAG.

Consecuencias
- Positivas: `edges` permanece como DAG puro; `validate`, `trace` y `collapse` funcionan sin modificación sobre `edges`. La distinción "ciclo legítimo vs. tautología" se resuelve por diseño (está en `feedback_edges` = intencional; ciclo en `edges` = error bloqueante). `ltp status` reporta loops sin ejecutar detección de ciclos.
- `trace` incluye feedback edges por defecto; se excluyen con `--no-feedback`.
- Negativas: Dos pools de aristas que el consumidor debe considerar. Mitigación: `trace` los incluye por defecto para que no se ignoren accidentalmente.

ADR-008: Negative Branch Reservations como Sub-Trees Embebidos (`nbr_branches`)

Contexto

En el FRT (y ocasionalmente TT), al proponer inyecciones surgen Negative Branch Reservations (NBR): cadenas causales de efectos no deseados que requieren inyecciones de trimming para neutralizarlas. Una NBR puede tener 5-10 nodos propios y ser recursiva (la inyección de trim genera otra NBR).

Se evaluaron dos opciones:
1. NBR como ramas anotadas mezcladas en los `edges` del FRT → ruido en la lectura del trunk, dificultad para distinguir cadena "feliz" de cadena NBR en `validate`, complejidad con anidación recursiva.
2. NBR como sub-tree embebido (`nbr_branches[]`) dentro del tree principal → filtrable, encapsulado, el trunk del FRT queda limpio.

Decisión

Adoptar `nbr_branches[]` como estructura embebida en los trees que lo requieran (FRT, TT).

Estructura:
```json
{
  "nbr_branches": [
    {
      "id": "NBR-001",
      "source_node": "FRT-INJ-001",
      "edges": [ ... ],
      "trim_injection": "FRT-INJ-003"
    }
  ]
}
```

Consecuencias
- Positivas: El FRT trunk permanece limpio. Cada NBR es autocontenida y filtrable (`ltp tree inspect --nbr NBR-001`). `validate` aplica reglas específicas a NBRs (la cadena termina en efecto negativo). Soporta recursión natural (una NBR puede tener su propia `nbr_branches` anidada).
- Negativas: Los nodos de una NBR siguen viviendo en el pool global `/nodes/`, pero solo participan en el sub-tree embebido. El motor debe buscar nodos referenciados tanto en `edges` como en `nbr_branches[].edges`.

ADR-009: Undo/Redo por Snapshot con Validación de Checksums

Contexto

El motor necesita capacidad de deshacer/rehacer operaciones. Se evaluaron tres enfoques:
1. Git-native (micro-commits automáticos) → mezcla commits del motor con commits del usuario, genera historial ruidoso, obliga Git como dependencia dura.
2. Operation journal + inverse commands → calcular inversas de operaciones complejas (node split, link group) es frágil. Si journal y disco divergen, el undo puede corromper estado.
3. Snapshot de ficheros afectados (delta stack) → garantía absoluta de correctitud, no hay inversas que calcular, trivial de implementar.

Se eligió la opción 3 refinada con checksums de validación tras análisis con Six Hats.

Decisión

Adoptar un stack lineal (LIFO) de snapshots en `.ltp/undo/` con checksums SHA-256 para detectar divergencias externas.

Diseño:
- Cada operación mutante captura el estado previo (`before`) de los ficheros afectados + el hash del estado posterior (`after_hash`).
- `ltp undo` verifica que el estado actual coincida con `after_hash` antes de restaurar. Si diverge (Git, edición manual), rechaza con `UNDO_STATE_DIVERGED`.
- Redo valida con `before_hash` antes de reaplicar.
- Escritura atómica: write-then-rename vía `.ltp/tmp/` para prevenir corrupción en caso de crash a medias.
- Lock file con PID + timestamp para concurrencia. Auto-break de locks stale (PID muerto).
- `begin-batch` / `end-batch` para colapsar N operaciones en una sola entrada de undo.
- Rotación por tamaño (`max_size_mb`), no por conteo de entradas.

Consecuencias
- Positivas: Correctitud garantizada (snapshot = restore infalible). Independiente de Git. Detección universal de divergencias (Git, ediciones manuales, bugs). Stack lineal previene corrupción por deshacer fuera de orden. Metadata legible (action, command, timestamp).
- Negativas: Storage local (mitigado: negligible con workspaces ≤500 nodos, ~1MB máximo con rotación). Undo post-merge Git probablemente invalidado (mitigado: `ltp history check` detecta, usuario decide). Undo de `invalidate` resucita assumptions (comportamiento correcto pero documentar para evitar confusión).