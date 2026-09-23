# INTEGRATION.md

Puerta **única** de entrada si vas a construir un cliente (UI, agente u otro consumidor) sobre `ltp-engine`.

> **Regla de oro**: el binario en marcha es la **fuente de verdad**. Esta guía solo enseña a leerlo — no duplica el contrato (esquemas, lista de tools), que se lee del propio servidor.
>
> **Versión mínima de motor asumida por esta guía: `v0.2.0`.**

## 1. Obtener el binario

```bash
git pull origin main
cargo build --release
# → target/release/ltp-mcp   (servidor MCP — lo que consume la UI)
# → target/release/ltp       (CLI, el mismo motor)
```

Fija una versión exacta si quieres reproducibilidad: `git checkout v0.2.0` antes de compilar.

## 2. Conectar (config MCP)

```json
{
  "mcpServers": {
    "ltp": {
      "command": "/ruta/al/repo/target/release/ltp-mcp",
      "args": ["--workspace", "/ruta/a/tu/workspace-ltp"]
    }
  }
}
```

- El `--workspace` es un directorio de trabajo LTP; se crea con `ltp init` (CLI) o el tool `ltp/init` (MCP).
- El servidor habla JSON-RPC por stdio (protocolo MCP `2024-11-05`).

## 3. Verificar la conexión (handshake recomendado)

Al arrancar, tu cliente debería:

1. **`initialize`** → leer `result.serverInfo.version`, con formato `MAJOR.MINOR.PATCH+<sha>[.dirty]` (p. ej. `0.2.0+61ebe5fd`).
   - **Feature-gate**: parsea el *core* antes de `+` y **exige `>= 0.2.0`**. Si es menor, muestra "motor vX; la UI necesita ≥ 0.2.0" y no continúes.
   - El fragmento tras `+` es el commit exacto (`git checkout <sha>` reproduce código y docs). Ver [RELEASE_POLICY.md](RELEASE_POLICY.md).
2. **`tools/list`** → verifica por **nombre** que están los tools que tu UI usa (p. ej. `ltp/macro_add`). **No** hardcodees el número total: `tools/list` es la lista canónica y siempre actual.
3. **Degrada con elegancia** si falta un tool esperado o la versión no cuadra, en vez de romper en silencio.

## 4. El contrato (dónde mirar)

| Necesitas… | Mira… |
|-----------|-------|
| Comandos, parámetros y esquemas JSON de cada tool | [ENGINE_SPEC.md](ENGINE_SPEC.md) |
| El shape de **toda** respuesta: `{success, action, workspace, data, graph_health, errors, warnings}` | §"Contrato de Salida JSON" de [CLAUDE.md](CLAUDE.md) |
| Qué cambió en cada versión (para tu feature-gating) | [CHANGELOG.md](CHANGELOG.md) |
| Cómo interpretamos las versiones (SemVer anclado al contrato) | [RELEASE_POLICY.md](RELEASE_POLICY.md) |

## 5. Estable vs WIP (no construyas contra vaporware)

- ✅ **v0.2.0** — implementado y con contrato estable: núcleo determinista (grafo causal, DAG, integridad referencial), Knowledge Pool, historial undo/redo, y la flecha larga (Slice 1 `macro-assume` + Slice 2 `macro add/expand/promote`).
- 🚧 **RFC-002 meta-grafo** (escenarios, marcos temporales, multi-actor) — **RFC de diseño, NO implementado**. No hay tools para eso en el binario. No construyas UI contra ello hasta que se versione: aparecerá como MINOR en [CHANGELOG.md](CHANGELOG.md) y se anunciará aquí (§5 + versión mínima §3).

## Cómo se mantiene esta guía al día

La **versión mínima soportada** (§3) y el estado **estable/WIP** (§5) se revisan como paso del checklist de release ([RELEASE_POLICY.md](RELEASE_POLICY.md) §5). Todo lo demás (tools, esquemas) **no se duplica aquí**: se lee del binario (`tools/list`, `initialize`) y de `ENGINE_SPEC.md`. Así esta guía no puede quedar desactualizada respecto al motor.
