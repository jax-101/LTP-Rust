# PLAN v0.5.2 (PATCH) — Un ID nuevo nunca pisa uno existente

## Contexto

Al investigar la Parte B de `PLAN_post-v051.md` (2026-10-09, binario de `46a69a9`) apareció **pérdida de datos silenciosa**. Decisión del usuario: sale ya como **v0.5.2 PATCH**, junto con el refactor de la Parte A. Lo MINOR sigue en `PLAN_v060.md`.

Este plan va por la **revisión 3**, tras dos pasadas de Six Hats el mismo día.
- La rev 1 solo arreglaba la reconstrucción de `counters.json`.
- La rev 2 midió que el problema es más amplio: **el motor se fía de `counters.json` aunque esté por debajo de lo que hay en disco**, y el caso más frecuente no necesita permisos raros, solo git.
- La rev 3 (decisiones del usuario del 2026-10-09) sustituye "tolerante con referencia" por **escaneo acotado al prefijo y estricto** (D-3), y añade **IDs copiados por `tree clone` y `link dissolve`** (D-6). Tras una pasada de Six Hats sobre su coste, añade también **una reconciliación por ámbito y por comando** (D-7).

### Datos (medidos)

| # | Caso | Resultado |
|---|---|---|
| C1 | **Dos clones por git.** B clona (y reconstruye: `UDE: 1`), A crea `UDE-002` y hace push, B hace pull y luego `node add` | `success: true`, `id: UDE-002`, 0 warnings: **sobrescribe el nodo de A**. `git status` solo muestra ` M nodes/UDE-002.json` |
| C2 | `counters.json` ausente + un árbol con **marcas de conflicto de merge** (`<<<<<<<`), que contiene `LINK-001` y `NBR-001`. `link connect` en otro árbol | `success: true`, **`LINK-001` duplicado**. U3 (`tests/counters_rebuild.rs:318`) da hoy este salto por correcto |
| C3 | `counters.json` ausente + árbol en `chmod 000` | `LINK-001` duplicado, 0 warnings |
| C4 | `counters.json` ausente + `nodes/` en `0300` (`-wx`) | `node add` **sobrescribe `UDE-001`**, 0 warnings |
| C5 | `counters.json` en `chmod 000` | `ID_GENERATION_ERROR` (fail-closed por accidente: `save` falla) |
| C6 | **Clon recién hecho** (`.ltp/` está en `.gitignore`, así que no existe) | toda mutación da `LOCK_ERROR` ("No such file or directory"). `init` da `WORKSPACE_ALREADY_EXISTS`. **El clon no se puede usar** hasta hacer `mkdir .ltp` a mano |

C1 y C6 afectan al flujo que ADR-002 promueve ("git-diffable, branchable"). C6 tapa a C1: hoy nadie llega a C1 sin saltarse antes C6 a mano. Si se arreglara solo C6, C1 pasaría a ser el camino normal. Por eso **van juntos**.

Coste medido del escaneo completo (binario release `2a3a4fe`, 500 nodos + 50 árboles con 5.000 aristas y 5.000 supuestos, 4 MB): `node add` tarda unos **53 ms con `counters.json` y unos 90 ms reconstruyendo**, es decir, **unos 37 ms más por minteo** en un workspace grande.

### Principio

**Sobrestimar un contador es inocuo; subestimarlo destruye.** ADR-009 acepta huecos en los IDs (no retroceden). Por tanto:
- **D-1 Reconciliar en cada minteo**: `next_id` = máximo entre lo guardado en `counters.json` y lo observado en disco, más uno. `counters.json` deja de ser la fuente de verdad: queda como **memoria de monotonía**, para que no se reutilicen IDs de entidades borradas (el escaneo solo no lo garantiza). El escaneo da la seguridad.
- **D-2 Lo que no se puede parsear se escanea como texto**: un JSON corrupto (por ejemplo, con marcas de conflicto) → escáner de bytes sin regex que observa cada token `[A-Z]+-[0-9]+`. Puede pasarse por arriba, y eso es seguro (C2). En U3 se sigue sin pánico, pero ahora no puede quedarse corto.
- **D-3 Escaneo acotado al prefijo y estricto dentro de su ámbito** (rev 3, Six Hats del 2026-10-09; sustituye a "tolerante con referencia"). Cada prefijo vive en un único sitio y el escaneo solo mira ese sitio:

  | Ámbito | Prefijos | Qué se lee |
  |---|---|---|
  | `Nodes` | `UDE RC INJ NC GOAL OBJ WANT OBS IO INT DE REQ PRE CSF` | solo los **nombres** de `nodes/` (`read_dir`, sin abrir ficheros) |
  | `Knowledge` | `KN` | solo los nombres de `knowledge/` |
  | `Trees` | `LINK ASM FB NBR MACRO MASM` | el **contenido** de cada `trees/*.json` (D-2 si no parsea) |
  | todo | prefijo desconocido | los tres, estricto |

  Dentro del ámbito, **cualquier cosa ilegible da `ID_GENERATION_ERROR`, haya o no `counters.json`**: un `read_dir` que no sea `NotFound`, una entrada en error o un `read_to_string` fallido. Sin excepción, así que no queda ningún riesgo residual, ni callado ni avisado. Fuera del ámbito no se lee nada, y por eso un árbol con `chmod 000` **no** bloquea `node add` ni `knowledge add`.
  - Un árbol ilegible **sí** bloquea `link connect` en *otro* árbol. Es lo justificado: `LINK`, `ASM`, `FB` y `MACRO` son únicos en todo el workspace porque `knowledge/resolve.rs` los busca en todos los árboles y se queda con la primera coincidencia. Ya hay precedente en ADR-016 D-4 (`rm` y `split`: un árbol ilegible da fail-closed).
  - El mapeo sale de `fn scope_of(prefix: &str) -> ScanScope` con `enum ScanScope { Nodes, Knowledge, Trees, All }`. Un `match` sobre literales no avisa si falta un prefijo, así que un test recorre `ENTITY_TYPES` y comprueba que todo prefijo minteable (menos `TREE`, que nunca se mintea porque los IDs de árbol son slugs) cae en un ámbito distinto de `All`.
  - No se escanean los `node_ref` de los árboles para los prefijos de nodo: eso haría que un árbol ilegible volviera a bloquear `node add`, y las referencias colgantes ya las detecta `validate`.
  - Descartadas:
    - **T, tolerante con referencia** (rev 2): deja un doble fallo silencioso (`nodes/` sin listar y contador desactualizado → sobrescritura, C4 con C1), cosa que contradice D-K5.
    - **S, estricto en todo**: un árbol ajeno bloquearía todo minteo.
    - **Sondeo** (`UDE-00N.json` existe → N+1, sin listar): reutiliza IDs borrados en otro clon (si existe 003 pero no 002, devuelve 002) y viola la monotonía de ADR-009.
- **D-4 Un clon se puede usar (C6)**: `acquire_lock` crea `.ltp/` si falta (`create_dir_all`). El primer minteo reconstruye con D-1 a D-3. Sin código nuevo: hoy es un `LOCK_ERROR`, después es un éxito.
- **D-5 El detalle dice qué fichero**: variante interna nueva `LtpError::CounterScan { path, source }` ("cannot read {path} to compute the next ID: {source}"). El código que sale sigue siendo `ID_GENERATION_ERROR`. El `detail` no es contrato, pero hoy sería "Is a directory (os error 21)", sin pista para reparar.

- **D-6 Copiar una entidad copia también sus IDs** (hallazgo de la rev 3, mismo invariante: un ID identifica una sola entidad). Medido con el binario `0.5.1+2a3a4fe`:
  - `tree clone` vuelve a mintear `LINK`, pero copia **`ASM-xxx` y `FB-xxx` tal cual** (`tree/commands.rs:780` y `:794`). Un `knowledge link KN-001 --to ASM-001` queda ambiguo y, tras `tree rm` del original, **pasa en silencio a la copia**: `success: true`, 0 warnings y sin ningún aviso (`KNOWLEDGE_ORPHANED` solo lo emite `node rm`).
  - `link dissolve` reparte los mismos `ASM` en las N aristas nuevas (`link/advanced.rs:1252`): `ASM-001` queda en `LINK-002` y en `LINK-003` **del mismo árbol**. Luego `assume rm --asm ASM-001` responde `success: true`, pero la otra copia sigue viva. ADR-005 ("direccionables para invalidación") se rompe, y `validate` no lo detecta.
  - `link insert-between` **no** está afectado: mueve los `ASM` a una sola arista y la original desaparece. Tampoco `macro expand`, que ya mintea `ASM` nuevos.
  - **Arreglo**: en `tree clone`, cada `ASM` y cada `FB` copiados reciben un ID nuevo (`next_id`), mientras el original conserva el suyo y sus knowledge links. En `link dissolve`, la **primera** arista nueva conserva los `ASM` originales (así sus knowledge links siguen apuntando a un único destino) y las demás reciben copias con IDs nuevos. `dissolve` no emite hoy ningún aviso sobre supuestos y no se añade ninguno (sería MINOR).
  - Es PATCH: los IDs son datos, no contrato, y el comando sigue devolviendo la misma forma. `edges_cloned` y `created_links` no cambian.

- **D-7 Una reconciliación por ámbito y por comando** (Six Hats del 2026-10-09, aprobada por el usuario). Con D-1 tal cual, cada minteo vuelve a escanear su ámbito. Los comandos que mintean en serie (`tree clone` con un `LINK` por arista más, por D-6, un `ASM` por supuesto y un `FB` por feedback; `link dissolve`, `path replace`/`explode`, `macro expand`, `link connect` con N destinos) escanearían miles de veces. Estimación **sin medir**: 5.000 aristas y 5.000 supuestos ≈ 10.000 × 37 ms ≈ 6 min.
  - **Regla**: mientras el comando tiene el lock, el primer minteo de un ámbito lo escanea y lleva a `counters.json` el máximo de **todos** los prefijos de ese ámbito. Los minteos siguientes del mismo ámbito usan solo `counters.json`. Es seguro porque, con el lock tomado, nada que respete el protocolo añade IDs, y tras la primera reconciliación `counters.json` ya está por encima del disco.
  - **Estado**: `FsStorage` lleva `reconciled: Cell<ReconciledScopes>`, con `struct ReconciledScopes { nodes: bool, knowledge: bool, trees: bool }` (`Copy`, sin asignaciones) y un `locked` equivalente. La memoria **se borra en `acquire_lock` y en `release_lock`** y solo se consulta mientras hay un lock tomado.
    - Motivo: en MCP hay un único `FsStorage` durante toda la vida del servidor (`bin/ltp_mcp.rs:20`) y hay 266 `release_lock` manuales. Si se borrara solo al soltar, un camino que se olvide de soltar arrastraría la memoria al comando siguiente, y un `git pull` entre medias devolvería C1.
    - Un minteo sin lock reconcilia siempre: más lento, pero seguro.
  - Es estado interno, a diferencia del buzón de avisos descartado en `PLAN_v060.md` D-5, pero **no cambia ningún resultado**: es memoización de rendimiento, atada a la vida del lock. Se documenta con `///` en el campo.
  - Un error de escaneo no marca el ámbito como reconciliado: el siguiente minteo vuelve a intentarlo y vuelve a fallar.
  - Descartadas:
    - **`next_ids(prefix, n)`**: obliga a contar de antemano en comandos que mezclan prefijos, y si cuenta de menos vuelve al problema.
    - **Reconciliar todo en `acquire_lock`**: leería los árboles en cada `node add` y reintroduciría la opción S.
    - **Memoria por prefijo**: leería los árboles una vez por `LINK`, otra por `ASM` y otra por `FB`.

### Contrato

PATCH (RELEASE_POLICY §1). No hay códigos, campos ni flags nuevos:
- Un fallo del escaneo dentro del ámbito del prefijo sale como `ID_GENERATION_ERROR`, que ya emiten los **31** sitios que llaman a `next_id` (los 31 comprobados; los otros 2 resultados de `rg` eran comentarios de documentación).
- C6 pasa de `LOCK_ERROR` a éxito.
- En un workspace sano solo cambia la latencia de los comandos que mintean, y los IDs que `tree clone` y `link dissolve` asignan a las copias (D-6).

Se queda fuera, porque sería MINOR (va a `PLAN_v060.md` D-5): avisar de que el contador estaba desactualizado o de que hubo que reconstruir (`COUNTERS_REBUILT`, que hoy se descarta en `fs_storage.rs:231`), y detectar en `validate` los IDs duplicados que ya existan en workspaces creados antes de v0.5.2.

---

## Tareas

| T | Qué | Verificación |
|---|---|---|
| T0 | **Tests primero** (`tests/v052_counters.rs`), en rojo antes de T1. **R1** C1 con dos clones en `tempdir` (necesita `git`; se salta si no está): `UDE-002` de A intacto y B recibe `UDE-003`. **R1b**, el mismo caso sin git: `counters.json` con `UDE` por debajo del disco. **R2** C2: marcas de conflicto → `LINK >= 2` en el nuevo edge. **R3** (determinista) C3 con el árbol sustituido por un **directorio** (EISDIR) y sin `counters.json` → `ID_GENERATION_ERROR` con la ruta en `detail` y workspace intacto byte a byte (helper de `v051_no_expect`). **R4** (`cfg(unix)`, se salta si `uid == 0`) C4 → `ID_GENERATION_ERROR` y `UDE-001` intacto. **R5** C6: clon sin `.ltp/` → `node add` funciona y respeta lo que hay en disco. **R6** el mismo EISDIR **con** `counters.json` válido → `link connect` da `ID_GENERATION_ERROR` (estricto con o sin referencia) y `node add` y `knowledge add` funcionan (fuera de ámbito). **R6b** (`cfg(unix)`, se salta si `uid == 0`) `nodes/` en `0300` **con** `counters.json` válido → `ID_GENERATION_ERROR` (el doble fallo que T dejaba pasar). **R7** `knowledge/` ausente → todo funciona (ausente ≠ ilegible). **R8** `LINK` desactualizado con un árbol traído por pull → no hay `LINK` duplicado entre árboles. **R9** test de tabla: todo prefijo de `ENTITY_TYPES` salvo `TREE` tiene un ámbito distinto de `All`. **R10** `tree clone` con `ASM` y `FB` → ningún `ASM`/`FB` repetido entre árboles, y el original conserva los suyos. **R11** `knowledge link` a `ASM-001` + `tree clone` + `tree rm` del original → `knowledge inspect` da `target_type: "unknown"` (es lo que sale hoy con un destino borrado) y ya no `"assumption"` resuelto contra la copia. Ningún comando avisa de un knowledge link colgante: ni `tree rm` ni `assume rm` emiten `KNOWLEDGE_ORPHANED`, que solo existe en `node rm`. Eso queda fuera de alcance. **R12** `link dissolve` de un AND con `ASM` → IDs de `ASM` únicos en el árbol y `assume rm` del original deja 0 copias. | En rojo: R1, R1b, R2, R3, R4, R5, R6 (parte `link connect`), R6b, R8, R10, R11, R12. En verde ya: R7. R9 nace con T1 |
| T1 | `counters.rs`: `enum ScanScope { Nodes, Knowledge, Trees, All }`, `fn scope_of(prefix) -> ScanScope` y `fn observe_scope(root, scope) -> Result<Counters>` (el máximo observado de todos los prefijos del ámbito). Reglas: `read_dir` con `NotFound` → 0; cualquier otro error, una entrada en error o un `read_to_string` fallido → `Err(CounterScan { path, .. })`, en el primero; JSON no parseable → `observe_text`. `Counters::load` distingue `Missing`, `Corrupt` y `Valid(Counters)` (`Missing` y `Corrupt` arrancan de cero). `next_id`: si el ámbito no está reconciliado (D-7), aplica `max(guardado o 0, observado)` a todos sus prefijos y lo marca; después hace `+1` y `save`. `rebuild` y `scan_directory` / `scan_tree_contents`, que se tragan los errores, se eliminan. `acquire_lock` → `create_dir_all(ltp_dir)` (D-4). `load_counters` (pública y sin usos) se elimina o se alinea. **T1b (D-6)**: `tree clone` vuelve a mintear `ASM` y `FB`; `link dissolve` vuelve a mintear los `ASM` de la 2.ª arista en adelante. Todo antes de `save_tree`. Un fallo de minteo da `ID_GENERATION_ERROR` sin escribir (contadores quemados, ADR-009). **T1c (D-7)**: `Cell<ReconciledScopes>` y `locked` en `FsStorage`, borrados en `acquire_lock` y en `release_lock`, más un contador de escaneos `#[cfg(test)]`. Tests unitarios en `fs_storage.rs`: **R13**, con el lock tomado, 100 minteos que mezclan `LINK`, `ASM` y `FB` hacen **1** escaneo de `Trees`, y uno de `UDE` hace 1 de `Nodes`; **R14**, un `acquire` sin `release` previo, con un árbol añadido a mano entre medias, vuelve a escanear y respeta ese árbol; **R15**, sin lock, cada minteo escanea; **R16**, un escaneo que falla no marca el ámbito. | Sin `unwrap`/`expect`/`clone` nuevos. U1–U4 siguen en verde (U3 se ajusta: ahora sí ve los IDs del árbol en conflicto) |
| T2 | Mutaciones, revertidas a mano sobre el árbol de trabajo (sin `git checkout`): (M1) no reconciliar con referencia → mueren R1/R1b/R8; (M2) saltarse lo ilegible (como hoy) → mueren R3/R4; (M3) quitar `observe_text` → muere R2; (M4) tolerar lo ilegible cuando hay referencia (la regla T) → mueren R6/R6b; (M5) `NotFound` como ilegible → muere R7; (M6) quitar `create_dir_all` → muere R5; (M7) `scope_of` siempre `All` → muere R6 (parte `node add`); (M8) `tree clone` vuelve a copiar `ASM` → mueren R10/R11; (M9) `dissolve` vuelve a repartir los mismos `ASM` → muere R12; (M10) reconciliar siempre (sin memoria) → muere R13; (M11) borrar la memoria solo en `release_lock` → muere R14; (M12) marcar el ámbito antes de escanear → muere R16; (M13) actualizar solo el prefijo pedido y marcar todo el ámbito → muere R13, que debe comprobar también que `ASM` respeta el disco tras mintear primero `LINK`. | 13/13 detectadas |
| T3 | Rendimiento: repetir la medición de arriba con el binario nuevo. Umbral aceptable: menos de 100 ms por minteo en el workspace de 4 MB. Con D-3, `node add` solo lista `nodes/` y debería quedar cerca de los 53 ms de hoy; el coste del escaneo se concentra en los prefijos de `Trees`. Medir también **`tree clone`** de un árbol de 5.000 aristas y 5.000 supuestos (con D-7 debería costar una sola lectura de los árboles) y `link connect` con N destinos. Si `tree clone` supera 1 s, hay que revisar D-7 antes de la release. | Números en PROGRESS |
| T4 | Las 4 verificaciones | Verde |
| T5 | Docs: adenda a **ADR-009** (D-1 a D-3 y D-7, una reconciliación por ámbito y por comando atada al lock: `counters.json` como memoria de monotonía, el disco como seguridad, la tabla de ámbitos por prefijo, estricto dentro del ámbito) y a **ADR-005** (D-6: copiar una arista o un árbol mintea IDs de supuesto nuevos). ENGINE_SPEC §3.1: reconstrucción y reconciliación de contadores (hoy solo en PLAN.md:94) y que un clon funciona sin `.ltp/`. INTEGRATION: gate **`>= 0.5.2` para cualquier flujo con varios clones**, y nota de que dos clones que crean entidades **a la vez** siguen chocando en el ID. Git lo muestra como conflicto add/add, nunca como sobrescritura silenciosa. CHANGELOG `[0.5.2]`: `Fixed` (C1–C6 y D-6) y `Changed` (Parte A). PROGRESS. | — |
| T6 | Release (RELEASE_POLICY §5): `0.5.2`, `chore(release): v0.5.2`, tag anotado, `git push --follow-tags` | `ltp --version` = `0.5.2+<sha>` sin `dirty` |

Commits: `test(v0.5.2)` (T0), `fix(v0.5.2)` (T1, T1b y T1c, uno por cada uno), `docs(v0.5.2)` (T2 + T3 + T5), `chore(release)` (T6).

## Fuera de alcance (registrado)

- Avisos `COUNTERS_REBUILT` y de contador desactualizado o reconciliado → MINOR, `PLAN_v060.md` D-5. (Con la D-3 de la rev 3 ya no hay doble fallo que hacer visible.)
- **Duplicados que ya existen** (workspaces con `tree clone` o `link dissolve` anteriores a v0.5.2): detectarlos en `validate` necesita un código de warning nuevo → MINOR, para v0.6.0 o después. v0.5.2 solo deja de crearlos.
- **Knowledge links colgantes** tras `tree rm`, `assume rm`, `link disconnect` o `link feedback-rm`: hoy son silenciosos (`target_type: "unknown"`). Avisar sería un warning nuevo o ampliar `KNOWLEDGE_ORPHANED` → MINOR. Reparar con `assume rm` + `assume add` no sirve hoy, porque `assume rm` borra solo la primera copia.
- **Dos clones que crean a la vez** el mismo ID (los dos mintean `UDE-002` sin haber hecho pull): es inherente a los IDs secuenciales (invariante 1). Git lo muestra como conflicto. Solo se documenta.
- `create_new` (O_EXCL) en `save_node`/`save_knowledge` como defensa en profundidad: con D-1 no hace falta, y `atomic_write` (rename) no lo admite sin rediseño.
- "Ilegible ≠ ausente" en los lectores → `PLAN_v060.md`.
