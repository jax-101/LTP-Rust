# INTEGRATION.md

Puerta **única** de entrada si vas a construir un consumidor sobre `ltp-engine`.

Hay **dos superficies de integración**, ambas sobre el **mismo motor** y el **mismo contrato de salida** (`CommandOutput` JSON):

| Superficie | Para quién | Binario | Modelo |
|-----------|-----------|---------|--------|
| **CLI** | Apps / UI que spawnean el binario (p. ej. una UI de escritorio Tauri) | `ltp` | Un proceso por comando; sin servidor persistente |
| **MCP** | Una IA headless que consume el motor por JSON-RPC | `ltp-mcp` | Servidor persistente por stdio |

> **Regla de oro**: el binario en marcha es la **fuente de verdad**. Esta guía enseña a invocarlo y a leer su versión; **no** duplica el catálogo de comandos (vive en `ENGINE_SPEC.md` y en el propio binario).
>
> **Versión mínima de motor asumida por esta guía: `v0.2.0`.**

## 1. Obtener el binario

```bash
git pull origin main          # o: git checkout v0.2.0  (para fijar una versión)
cargo build --release
# → target/release/ltp        (CLI)
# → target/release/ltp-mcp    (servidor MCP)
```

Para reproducibilidad, compila desde un tag: `git checkout v0.2.0 && cargo build --release`.

## 2A. Integración por CLI (apps / UI)

Modelo: **spawnea `ltp` una vez por comando**. No hay estado en memoria entre llamadas; el estado vive en disco (el workspace).

- **Invocación**: `ltp <subcomando> [args]` (p. ej. `ltp tree walk <treeId>`, `ltp link connect --tree ...`).
- **Workspace = directorio de trabajo (`cwd`)**. ⚠️ El CLI **no** tiene flag `--workspace`: el motor opera sobre el `cwd` del proceso. Tu consumidor debe fijar el *current directory* al workspace al spawnear (p. ej. `Command::new(bin).current_dir(workspace)`).
- **Salida**: JSON canónico (`CommandOutput`) por **stdout**. `--human` da texto legible (no lo parsees); `--dry-run` simula la operación sin escribir a disco.
- **Errores de proceso**: si `stdout` viene vacío, lee `stderr` (fallo antes de producir salida JSON).
- **Handshake / feature-gate**: `ltp --version` → `X.Y.Z+<sha>[.dirty]`. Parsea el *core* antes de `+` y exige `>= 0.2.0`. El fragmento tras `+` es el commit exacto (`git checkout <sha>` reproduce código y docs).

```bash
cd /ruta/al/workspace       # el cwd ES el workspace
ltp tree walk <treeId>      # → CommandOutput JSON en stdout
```

### Parseo del output (contrato transversal — aplica a todo comando)

Detalles del contrato que un consumidor CLI debe conocer para no trabajar de memoria:

- **`errors[]` y `warnings[]` son objetos, no strings**: `{ "code", "detail", ...contexto }`. El contexto va **aplanado al nivel raíz del objeto** (no anidado bajo `context`). Ej.: `{ "code": "NODE_NOT_IN_TREE", "detail": "...", "node_id": "UDE-001", "tree_id": "tree-crt-x" }`. Las claves de contexto varían según el `code`.
- **Enums en minúscula**: p. ej. `tree_type` → `crt|ec|frt|prt` (el campo es `tree_type`, **no** `type`); `logic` → `sufficiency|necessity`.
- **Campos opcionales: dos convenciones distintas, no las confundas.** Un campo *gated por flag* se **omite** por completo cuando el flag no está (p. ej. `knowledge` en `tree walk` solo aparece con `--show-knowledge`) → compruébalo con presencia de clave. Un campo *de valor* sin asignar sí sale, como `null` (p. ej. `role: null` en un nodo sin rol) → compruébalo contra `null`, no contra ausencia. Los goldens en [`contract/`](contract/) fijan ambos casos por máquina.
- Los **shapes de `data` por comando** (p. ej. la forma de cada nodo en `tree walk`, o de `tree list`) viven en [ENGINE_SPEC.md](ENGINE_SPEC.md) — no se duplican aquí. Y están **fijados por máquina** como golden fixtures en [`contract/`](contract/): ejemplos reales del `CommandOutput` que un snapshot test regenera y compara, de modo que cualquier deriva de shape rompe el build antes de llegarte (RELEASE_POLICY §1). Léelos como muestras siempre-verdaderas de lo que parseas.

## 2B. Integración por MCP (IA headless)

Modelo: **servidor persistente** por stdio (JSON-RPC, protocolo MCP `2024-11-05`).

```json
{
  "mcpServers": {
    "ltp": {
      "command": "/ruta/target/release/ltp-mcp",
      "args": ["--workspace", "/ruta/al/workspace"]
    }
  }
}
```

⚠️ Asimetría deliberada: aquí el workspace **sí** va como flag `--workspace` (no como `cwd`).

- **Handshake**: `initialize` → leer `result.serverInfo.version` (mismo formato `X.Y.Z+sha`); `tools/list` → verificar los tools por **nombre** (p. ej. `ltp/macro_add`), sin hardcodear el recuento.

## 3. El contrato (compartido por ambas superficies)

Toda invocación —CLI o MCP— devuelve el mismo shape:

```json
{ "success": bool, "action": "...", "workspace": "...", "data": { }, "graph_health": { }, "errors": [ ], "warnings": [ ] }
```

Los nombres se corresponden 1:1: subcomando CLI `tree walk` ↔ tool MCP `ltp/tree_walk`.

| Necesitas… | Mira… |
|-----------|-------|
| Comandos, args y esquemas JSON de cada operación | [ENGINE_SPEC.md](ENGINE_SPEC.md) |
| El shape completo de **toda** respuesta | §"Contrato de Salida JSON" de [CLAUDE.md](CLAUDE.md) |
| Qué cambió en cada versión (para tu feature-gating) | [CHANGELOG.md](CHANGELOG.md) |
| Cómo interpretamos las versiones (SemVer anclado al contrato) | [RELEASE_POLICY.md](RELEASE_POLICY.md) |

## 4. Estable vs WIP (no construyas contra vaporware)

- ✅ **v0.2.0** — implementado y con contrato estable: núcleo determinista (grafo causal, DAG, integridad referencial), Knowledge Pool, historial undo/redo, y la flecha larga (Slice 1 `macro-assume` + Slice 2 `macro add/expand/promote`).
- 🚧 **RFC-002 meta-grafo** (escenarios, marcos temporales, multi-actor) — **RFC de diseño, NO implementado**. No hay comandos ni tools para eso en el binario. No construyas contra ello hasta que se versione: aparecerá como MINOR en [CHANGELOG.md](CHANGELOG.md) y se anunciará aquí (§4 + versión mínima en cabecera).

## Cómo se mantiene esta guía al día

La **versión mínima soportada** (cabecera) y el estado **estable/WIP** (§4) se revisan como paso del checklist de release ([RELEASE_POLICY.md](RELEASE_POLICY.md) §5). El catálogo de comandos **no se duplica aquí**: se lee del binario (`ltp --help` / `tools/list`) y de `ENGINE_SPEC.md`. Así esta guía no puede quedar desactualizada respecto al motor.
