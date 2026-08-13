# MCP Quickstart — ltp-engine con Claude Code

## 1. Compilar el servidor

```bash
cd /Users/jullivarri/Documents/claude/RUST/LTP_Rust
cargo build --release
```

El binario queda en `target/release/ltp-mcp`.

## 2. Configurar en Claude Code

Crea (o edita) un archivo **`.mcp.json`** en la raíz del proyecto:

```json
{
  "mcpServers": {
    "ltp": {
      "command": "/Users/jullivarri/Documents/claude/RUST/LTP_Rust/target/release/ltp-mcp",
      "args": ["--workspace", "/tmp/ltp-demo"]
    }
  }
}
```

> Cambia `/tmp/ltp-demo` por la ruta donde quieras que viva tu workspace LTP.
> 
> Alternativa: usa `claude mcp add ltp -- /ruta/a/ltp-mcp --workspace /tmp/ltp-demo` desde la terminal.

## 3. Reiniciar Claude Code

Tras guardar la config, reinicia la sesión de Claude Code para que detecte el servidor MCP. Verás los tools `ltp/*` disponibles.

## 4. Ejemplo de sesión completa

Una vez conectado, pídele a Claude algo como:

> "Inicializa un workspace LTP y construye un CRT (Current Reality Tree) para analizar por qué las entregas de producto llegan tarde. Incluye al menos 5 UDEs conectados causalmente hasta 2 root causes."

Claude usará los tools MCP automáticamente. El flujo interno será:

```
ltp/init                    → Crea estructura del workspace
ltp/node_add (×7)          → Crea UDEs y RCs
ltp/tree_new               → Crea tree CRT
ltp/tree_attach (×7)       → Vincula nodos al tree
ltp/link_connect (×6)      → Conecta cadena causal
ltp/validate               → Verifica integridad DAG
ltp/trace                  → Navega upstream/downstream
```

## 5. Verificación manual (sin Claude Code)

Para probar que el servidor funciona sin necesidad de un cliente MCP:

```bash
# Crear directorio de workspace
mkdir -p /tmp/ltp-demo

# Enviar requests al servidor
cat <<'REQUESTS' | target/release/ltp-mcp --workspace /tmp/ltp-demo
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"ltp/init","arguments":{"name":"demo"}}}
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"ltp/node_add","arguments":{"label":"Entregas llegan tarde","type":"UDE"}}}
{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"ltp/node_add","arguments":{"label":"No hay stock en almacen","type":"UDE"}}}
{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"ltp/node_add","arguments":{"label":"Proveedor incumple plazos","type":"RC"}}}
{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"ltp/tree_new","arguments":{"type":"CRT","name":"entregas"}}}
{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"ltp/tree_attach","arguments":{"tree":"tree-crt-entregas","node":"UDE-001"}}}
{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"ltp/tree_attach","arguments":{"tree":"tree-crt-entregas","node":"UDE-002"}}}
{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"ltp/tree_attach","arguments":{"tree":"tree-crt-entregas","node":"RC-001"}}}
{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"ltp/link_connect","arguments":{"tree":"tree-crt-entregas","from":["RC-001"],"to":["UDE-002"]}}}
{"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"ltp/link_connect","arguments":{"tree":"tree-crt-entregas","from":["UDE-002"],"to":["UDE-001"]}}}
{"jsonrpc":"2.0","id":12,"method":"tools/call","params":{"name":"ltp/validate","arguments":{"tree":"tree-crt-entregas"}}}
{"jsonrpc":"2.0","id":13,"method":"tools/call","params":{"name":"ltp/trace","arguments":{"node_id":"UDE-001","tree":"tree-crt-entregas","direction":"upstream","depth":10}}}
{"jsonrpc":"2.0","id":14,"method":"tools/call","params":{"name":"ltp/status","arguments":{}}}
REQUESTS
```

Cada línea produce una respuesta JSON-RPC en stdout. La última (`status`) reporta el estado global del workspace.

## 6. Tools disponibles (54)

| Grupo | Tools |
|-------|-------|
| Workspace | `init`, `status` |
| Node | `add`, `edit`, `rm`, `inspect`, `list`, `search`, `split` |
| Tree | `new`, `list`, `rm`, `attach`, `detach`, `clone`, `diff`, `walk` |
| Link | `connect`, `disconnect`, `feedback`, `inspect`, `find`, `reverse`, `move`, `insert_between`, `group`, `dissolve`, `split`, `reoperator`, `add_cause`, `rm_cause` |
| Assume | `add`, `edit`, `rm`, `list`, `move` |
| Logic | `invalidate`, `validate`, `trace` |
| Path | `collapse`, `explode`, `replace` |
| NBR | `add`, `rm`, `list`, `inspect` |
| History | `undo`, `redo`, `list`, `check`, `invalidate`, `begin_batch`, `end_batch`, `clear` |

Todos prefijados con `ltp/` (ej. `ltp/node_add`).
