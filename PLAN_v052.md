# PLAN v0.5.2 (PATCH) — Un ID nuevo nunca pisa uno existente

## Contexto

Al investigar la Parte B de `PLAN_post-v051.md` (2026-10-09, binario de `46a69a9`) apareció **pérdida de datos silenciosa**. Decisión del usuario: sale ya como **v0.5.2 PATCH**, junto con el refactor de la Parte A. Lo MINOR sigue en `PLAN_v060.md`.

Este plan es la **revisión 2**, tras pasar los Six Hats el mismo día. La primera versión solo arreglaba la reconstrucción de `counters.json`. La revisión midió que el problema es más amplio: **el motor se fía de `counters.json` aunque esté por debajo de lo que hay en disco**, y el caso más frecuente no necesita permisos raros, solo git.

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
- **D-3 Lo que no se puede leer depende de si hay referencia**:
  - **Sin referencia** (`counters.json` ausente o corrupto): fail-closed → `ID_GENERATION_ERROR` (C3, C4). Es D-K5: no hay ninguna base para no pisar un ID.
  - **Con referencia** (`counters.json` válido): se usa el máximo entre lo guardado y lo escaneado sin el fichero ilegible. Solo puede subir el contador. El riesgo que queda exige **dos fallos a la vez**: `counters.json` desactualizado y, además, que el ID que falta esté justo en el fichero ilegible. Se documenta y v0.6.0 lo hará visible (warning).
  - Se descarta el fail-closed estricto. Bloquearía todo minteo por un solo fichero ilegible ajeno al comando: un árbol con `chmod 000` impediría `node add`. Se descarta también no reconciliar cuando hay referencia, porque deja C1 sin arreglar.
- **D-4 Un clon se puede usar (C6)**: `acquire_lock` crea `.ltp/` si falta (`create_dir_all`). El primer minteo reconstruye con D-1 a D-3. Sin código nuevo: hoy es un `LOCK_ERROR`, después es un éxito.
- **D-5 El detalle dice qué fichero**: variante interna nueva `LtpError::CounterScan { path, source }` ("cannot read {path} to compute the next ID: {source}"). El código que sale sigue siendo `ID_GENERATION_ERROR`. El `detail` no es contrato, pero hoy sería "Is a directory (os error 21)", sin pista para reparar.

### Contrato

PATCH (RELEASE_POLICY §1). No hay códigos, campos ni flags nuevos:
- Un fallo del escaneo sin referencia sale como `ID_GENERATION_ERROR`, que ya emiten los **31** sitios que llaman a `next_id` (los 31 comprobados; los otros 2 resultados de `rg` eran comentarios de documentación).
- C6 pasa de `LOCK_ERROR` a éxito.
- En un workspace sano solo cambia la latencia de los comandos que mintean.

Se queda fuera, porque sería MINOR (va a `PLAN_v060.md` D-5): avisar de que el contador estaba desactualizado o de que hubo que reconstruir (`COUNTERS_REBUILT`, que hoy se descarta en `fs_storage.rs:231`).

---

## Tareas

| T | Qué | Verificación |
|---|---|---|
| T0 | **Tests primero** (`tests/v052_counters.rs`), en rojo antes de T1. **R1** C1 con dos clones en `tempdir` (necesita `git`; se salta si no está): `UDE-002` de A intacto y B recibe `UDE-003`. **R1b**, el mismo caso sin git: `counters.json` con `UDE` por debajo del disco. **R2** C2: marcas de conflicto → `LINK >= 2` en el nuevo edge. **R3** (determinista) C3 con el árbol sustituido por un **directorio** (EISDIR) y sin `counters.json` → `ID_GENERATION_ERROR` con la ruta en `detail` y workspace intacto byte a byte (helper de `v051_no_expect`). **R4** (`cfg(unix)`, se salta si `uid == 0`) C4 → `ID_GENERATION_ERROR` y `UDE-001` intacto. **R5** C6: clon sin `.ltp/` → `node add` funciona y respeta lo que hay en disco. **R6** el mismo EISDIR **con** `counters.json` válido → el minteo sigue funcionando (D-3 con referencia). **R7** `knowledge/` ausente → todo funciona (ausente ≠ ilegible). **R8** `LINK` desactualizado con un árbol traído por pull → no hay `LINK` duplicado entre árboles. | En rojo: R1, R1b, R2, R3, R4, R5, R8. En verde ya: R6, R7 |
| T1 | `counters.rs`: `fn scan(root) -> Result<ScanReport>` con `ScanReport { observed: Counters, unreadable: Vec<PathBuf> }`. Reglas: `read_dir` con `NotFound` → vacío; otro error, una entrada en error o `read_to_string` fallido → a `unreadable`; JSON no parseable → `observe_text`. `Counters::load` distingue `Missing`, `Corrupt` y `Valid(Counters)`. `next_id`: con referencia, `max(guardado, observed)`; sin referencia, si `unreadable` no está vacío → `Err(CounterScan)`. Después `+1` y `save`. `acquire_lock` → `create_dir_all(ltp_dir)` (D-4). `load_counters` (pública y sin usos) se elimina o se alinea. | Sin `unwrap`/`expect`/`clone` nuevos. U1–U4 siguen en verde |
| T2 | Mutaciones, revertidas a mano sobre el árbol de trabajo (sin `git checkout`): (M1) no reconciliar con referencia → mueren R1/R1b/R8; (M2) `unreadable` ignorado sin referencia → mueren R3/R4; (M3) quitar `observe_text` → muere R2; (M4) fail-closed también con referencia → muere R6; (M5) `NotFound` como ilegible → muere R7; (M6) quitar `create_dir_all` → muere R5. | 6/6 detectadas |
| T3 | Rendimiento: repetir la medición de arriba con el binario nuevo. Umbral aceptable: menos de 100 ms por `node add` en el workspace de 4 MB. Si `link connect` con N destinos lo supera (escanea N veces), se mintea en bloque **dentro de `FsStorage::next_id`**, sin caché entre llamadas. | Número en PROGRESS |
| T4 | Las 4 verificaciones | Verde |
| T5 | Docs: adenda a **ADR-009** (D-1 a D-3: `counters.json` como memoria de monotonía, el disco como seguridad, el doble fallo residual). ENGINE_SPEC §3.1: reconstrucción y reconciliación de contadores (hoy solo en PLAN.md:94) y que un clon funciona sin `.ltp/`. INTEGRATION: gate **`>= 0.5.2` para cualquier flujo con varios clones**, y nota de que dos clones que crean entidades **a la vez** siguen chocando en el ID. Git lo muestra como conflicto add/add, nunca como sobrescritura silenciosa. CHANGELOG `[0.5.2]`: `Fixed` (C1–C6) y `Changed` (Parte A). PROGRESS. | — |
| T6 | Release (RELEASE_POLICY §5): `0.5.2`, `chore(release): v0.5.2`, tag anotado, `git push --follow-tags` | `ltp --version` = `0.5.2+<sha>` sin `dirty` |

Commits: `test(v0.5.2)` (T0), `fix(v0.5.2)` (T1), `docs(v0.5.2)` (T2 + T3 + T5), `chore(release)` (T6).

## Fuera de alcance (registrado)

- Avisos `COUNTERS_REBUILT` y de contador desactualizado o reconciliado, y hacer visible el doble fallo de D-3 → MINOR, `PLAN_v060.md` D-5.
- **Dos clones que crean a la vez** el mismo ID (los dos mintean `UDE-002` sin haber hecho pull): es inherente a los IDs secuenciales (invariante 1). Git lo muestra como conflicto. Solo se documenta.
- `create_new` (O_EXCL) en `save_node`/`save_knowledge` como defensa en profundidad: con D-1 no hace falta, y `atomic_write` (rename) no lo admite sin rediseño.
- "Ilegible ≠ ausente" en los lectores → `PLAN_v060.md`.
