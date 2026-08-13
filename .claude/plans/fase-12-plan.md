# Plan F12: MCP Server

## Resumen

Exponer todas las primitivas de `ltp-engine` como servidor MCP (Model Context Protocol) sobre stdin/stdout usando JSON-RPC 2.0. El servidor será un binario alternativo (`ltp-mcp`) que reutiliza toda la lógica de `src/` sin duplicar código.

---

## Decisiones de Diseño

### D1: Binario separado vs. subcomando

**Decisión**: Binario separado `src/bin/ltp_mcp.rs` + módulo `src/mcp/`.

**Justificación**: El servidor MCP tiene un ciclo de vida completamente distinto (loop infinito stdin/stdout) al CLI (comando → salida → exit). Separar binarios evita contaminar el parsing de clap y mantiene `main.rs` limpio. Cargo soporta múltiples binarios en `[[bin]]`.

### D2: Implementación JSON-RPC

**Decisión**: Implementación mínima propia (sin crate externo de MCP).

**Justificación**: 
- El protocolo MCP es un subset pequeño de JSON-RPC 2.0 (initialize, tools/list, tools/call + notifications).
- Evita dependencias pesadas (`rmcp`, `tower`, `tokio`) que añadirían async innecesario — el motor es síncrono y determinista.
- ~200 líneas de parsing/dispatch JSON-RPC son manejables y auditables.

### D3: Mapping CLI → MCP Tools

**Decisión**: Nombre de tool = `ltp/<namespace>_<action>` (ej. `ltp/node_add`, `ltp/link_connect`).

**Justificación**: Namespace `ltp/` previene colisiones. Snake_case es idiomático para MCP tools. El mapping 1:1 con CLI garantiza paridad funcional.

### D4: Workspace path

**Decisión**: El servidor recibe `--workspace <path>` como argumento al iniciar, o usa CWD.

**Justificación**: Un agente LLM configura el servidor MCP con un path fijo. No cambia durante la sesión.

### D5: History hooks en MCP

**Decisión**: Los tools de manipulación generan entries de undo/redo igual que CLI.

**Justificación**: UAT 12.5 requiere que las mutaciones generen undo entries. El MCP server usa exactamente la misma lógica `history_begin`/`history_commit` que `main.rs`.

---

## Arquitectura

```
src/
├── mcp/
│   ├── mod.rs          # Re-exports
│   ├── types.rs        # Structs JSON-RPC 2.0 + MCP (Request, Response, Tool, etc.)
│   ├── server.rs       # Loop stdin/stdout, dispatch
│   ├── tools.rs        # Registro de tools (nombre, descripción, inputSchema)
│   └── dispatch.rs     # Router: tool_name → execute_* function
├── bin/
│   └── ltp_mcp.rs      # Entry point del servidor MCP (minimal)
```

### Flujo de datos

```
stdin (JSON-RPC request)
  → parse Request
  → match method:
      "initialize"  → return capabilities + server info
      "tools/list"  → return tool definitions con inputSchema
      "tools/call"  → dispatch(tool_name, params) → execute_* → CommandOutput<T> → MCP content
  → serialize Response
  → stdout
```

---

## Tools a exponer (UAT 12.10)

Total: ~60 tools organizados por namespace:

| Namespace | Tools | Count |
|-----------|-------|-------|
| workspace | `init`, `status` | 2 |
| node | `add`, `edit`, `rm`, `inspect`, `list`, `search`, `split` | 7 |
| tree | `new`, `list`, `rm`, `attach`, `detach`, `clone`, `diff`, `walk` | 8 |
| link | `connect`, `disconnect`, `feedback`, `inspect`, `find`, `reverse`, `move`, `insert_between`, `group`, `dissolve`, `split`, `reoperator`, `add_cause`, `rm_cause` | 14 |
| assume | `add`, `edit`, `rm`, `list`, `move` | 5 |
| invalidate | `invalidate` | 1 |
| validate | `validate` | 1 |
| trace | `trace` | 1 |
| path | `collapse`, `explode`, `replace` | 3 |
| nbr | `add`, `rm`, `list`, `inspect` | 4 |
| history | `undo`, `redo`, `list`, `check`, `invalidate`, `begin_batch`, `end_batch`, `clear` | 8 |
| **TOTAL** | | **54** |

---

## JSON-RPC Error Codes

| Código | Significado |
|--------|-------------|
| -32700 | Parse error (JSON malformado) |
| -32600 | Invalid request (falta method/id) |
| -32601 | Method not found |
| -32602 | Invalid params |
| -32001 | Workspace not initialized |
| -32002 | Tool execution error (LtpError mapeado) |

---

## Formato de Tool Result

Cada tool retorna un `content` array con un bloque de tipo `text` conteniendo el JSON del `CommandOutput<T>`:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\"success\":true,\"action\":\"node_add\",...}"
      }
    ],
    "isError": false
  }
}
```

Cuando `success: false` en el CommandOutput, se retorna `isError: true` en el MCP result (NO un JSON-RPC error), para que el agente pueda leer los detalles estructurados.

---

## Implementación paso a paso

### Paso 1: Tipos MCP (types.rs)

- `JsonRpcRequest`, `JsonRpcResponse`, `JsonRpcError`
- `McpCapabilities`, `ServerInfo`, `Tool`, `ToolInputSchema`
- `ToolCallParams`, `ToolResult`, `ContentBlock`
- `InitializeResult`, `ToolsListResult`

### Paso 2: Registro de Tools (tools.rs)

- Función `all_tools() -> Vec<Tool>` que genera la lista completa con:
  - `name`: `ltp/node_add`, etc.
  - `description`: texto breve del comando
  - `inputSchema`: JSON Schema con `type: "object"`, `properties`, `required`

### Paso 3: Dispatcher (dispatch.rs)

- `dispatch_tool(name: &str, params: Value, storage: &FsStorage, dry_run: bool) -> ToolResult`
- Match sobre el nombre del tool
- Extrae params del JSON → llama a `execute_*` → serializa `CommandOutput` → `ToolResult`
- Maneja history hooks para tools mutantes

### Paso 4: Server loop (server.rs)

- Lee líneas de stdin (cada línea = un JSON-RPC message)
- Parsea → dispatcha → escribe response a stdout + flush
- Maneja `initialize`, `initialized` (notification, no response), `tools/list`, `tools/call`
- Graceful shutdown en EOF

### Paso 5: Binary entry point (bin/ltp_mcp.rs)

- Parsea args (`--workspace`)
- Crea `FsStorage`
- Llama a `mcp::server::run(storage)`

### Paso 6: Tests de integración

- Un test por UAT (12.1–12.10)
- Helper que spawea el proceso `ltp-mcp` y envía/recibe JSON-RPC vía stdin/stdout pipes

---

## Dependencias nuevas

Ninguna crate nueva necesaria. Solo se usa `serde_json` (ya en deps) para parsing JSON-RPC.

---

## Cargo.toml changes

```toml
[[bin]]
name = "ltp-mcp"
path = "src/bin/ltp_mcp.rs"
```

---

## UATs → Test mapping

| UAT | Escenario | Test |
|-----|-----------|------|
| 12.1 | `initialize` request → capabilities + tools | `test_initialize_returns_capabilities` |
| 12.2 | `ltp/node_add` vía JSON-RPC → mismo resultado que CLI | `test_node_add_via_mcp` |
| 12.3 | Request con params inválidos → error JSON-RPC | `test_invalid_params_error` |
| 12.4 | Tool de navegación (inspect, trace) → sin undo entry | `test_navigation_no_undo_entry` |
| 12.5 | Tool de manipulación → genera undo entry | `test_manipulation_generates_undo` |
| 12.6 | `ltp/validate` → errors + warnings en formato MCP | `test_validate_returns_errors_warnings` |
| 12.7 | `dry_run: true` en tool de manipulación → sin mutar | `test_dry_run_no_mutation` |
| 12.8 | Request sin workspace inicializado → code -32001 | `test_no_workspace_error` |
| 12.9 | `ltp/node_add` que genera warning CLR#2 → preservado | `test_clr2_warning_preserved` |
| 12.10 | `tools/list` → contiene TODOS los subcomandos (54) | `test_tools_list_complete` |
| 12.11 | JSON malformado → code -32700 "Parse error", sin crash | `test_malformed_json_parse_error` |
| 12.12 | Método desconocido `ltp/nonexistent` → code -32601 | `test_unknown_method_not_found` |
| 12.13 | `ltp/node_rm` con ID inexistente → `isError: true` | `test_failed_tool_is_error_flag` |
| 12.14 | EOF en stdin → server termina con exit 0 | `test_eof_graceful_shutdown` |
| 12.15 | Flujo secuencial: init→node_add→connect→trace | `test_sequential_workflow_state_persists` |
| 12.16 | `tools/list` como request separado de `initialize` | `test_tools_list_standalone` |

---

## Riesgos y Mitigaciones

| Riesgo | Mitigación |
|--------|------------|
| Tool count incorrecto en 12.10 | Generar lista programáticamente, no hardcodear |
| Params parsing frágil | JSON Schema validation + mensajes de error claros |
| Deadlock stdin/stdout | Single-threaded sync, no buffering issues |
| History hooks duplicados con main.rs | Extraer helper compartido (ya existe `history_begin`/`history_commit`) |
