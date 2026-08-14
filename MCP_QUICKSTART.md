# Instalar ltp-engine con Claude Code

## Requisitos

- Rust toolchain ([rustup.rs](https://rustup.rs))
- Claude Code CLI

## 1. Instalar binarios

```bash
cargo install --path .
```

Esto coloca `ltp` y `ltp-mcp` en `~/.cargo/bin/` (ya esta en PATH).

## 2. Registrar el servidor MCP globalmente

```bash
claude mcp add --scope user ltp -- ltp-mcp
```

Sin argumentos adicionales. El servidor detecta automaticamente la carpeta del proyecto via `CLAUDE_PROJECT_DIR` (inyectada por Claude Code).

## 3. Verificar

Abre Claude Code en cualquier carpeta y pide:

> "Inicializa un workspace LTP y muestra el status"

Veras las tools `ltp/init` y `ltp/status` ejecutarse correctamente.

## Como funciona la resolucion del workspace

Prioridad (gana la primera que exista):

1. `--workspace <path>` — override explicito (para testing o CI)
2. `CLAUDE_PROJECT_DIR` — inyectada por Claude Code automaticamente
3. Directorio actual (`cwd`) — fallback ultimo

## Actualizar

```bash
cd /ruta/al/repo/LTP_Rust
git pull
cargo install --path .
```

Claude Code usara el nuevo binario en la proxima sesion.

## Desinstalar

```bash
cargo uninstall ltp-engine
claude mcp remove --scope user ltp
```

## Verificacion manual (sin Claude Code)

Para probar que el servidor funciona directamente:

```bash
mkdir -p /tmp/ltp-test

cat <<'REQUESTS' | ltp-mcp --workspace /tmp/ltp-test
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"ltp/init","arguments":{"name":"test"}}}
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"ltp/status","arguments":{}}}
REQUESTS
```

## Tools disponibles (54)

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

Todos prefijados con `ltp/` (ej. `ltp/node_add`, `ltp/tree_new`).
