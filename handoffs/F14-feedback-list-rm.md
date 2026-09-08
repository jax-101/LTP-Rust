# F14: link feedback-list + link feedback-rm

**Fecha**: 2026-09-08
**Autor**: ltp-engine (backend)
**Consumidor**: UI / otro chat

---

## Resumen

Dos comandos nuevos para gestionar feedback edges. Completan el CRUD que empezo en F4 (`link feedback` solo creaba).

| Comando | Tipo | Historial | MCP tool |
|---------|------|-----------|----------|
| `link feedback-list` | Lectura | No | `ltp/link_feedback_list` |
| `link feedback-rm` | Mutacion | Si (undo restaura) | `ltp/link_feedback_rm` |

---

## CLI

```bash
ltp link feedback-list --tree <TREE_ID>
ltp link feedback-rm --tree <TREE_ID> --feedback <FB_ID>
```

---

## MCP tools

### `ltp/link_feedback_list`

| Param | Tipo | Requerido | Descripcion |
|-------|------|-----------|-------------|
| `tree` | string | si | Tree ID |

### `ltp/link_feedback_rm`

| Param | Tipo | Requerido | Descripcion |
|-------|------|-----------|-------------|
| `tree` | string | si | Tree ID |
| `feedback` | string | si | Feedback edge ID (ej. `FB-001`) |

---

## Output JSON

### feedback-list (success)

```json
{
  "success": true,
  "action": "link_feedback_list",
  "workspace": "Demo",
  "data": {
    "tree_id": "tree-crt-salesanalysis",
    "feedback_edges": [
      {
        "id": "FB-001",
        "from": "UDE-001",
        "to": "RC-001",
        "loop_type": "positive",
        "label": "Reinforcing loop"
      },
      {
        "id": "FB-002",
        "from": "RC-001",
        "to": "UDE-001",
        "loop_type": "negative",
        "label": null
      }
    ]
  },
  "graph_health": {
    "valid_dag": true,
    "orphan_nodes_count": 0
  },
  "errors": [],
  "warnings": []
}
```

### feedback-list (tree vacio, sin feedback edges)

```json
{
  "success": true,
  "action": "link_feedback_list",
  "data": {
    "tree_id": "tree-crt-example",
    "feedback_edges": []
  },
  "errors": [],
  "warnings": []
}
```

### feedback-rm (success)

```json
{
  "success": true,
  "action": "link_feedback_rm",
  "workspace": "Demo",
  "data": {
    "removed_id": "FB-001",
    "tree_id": "tree-crt-salesanalysis"
  },
  "graph_health": {
    "valid_dag": true,
    "orphan_nodes_count": 0
  },
  "errors": [],
  "warnings": []
}
```

---

## Errores

| Codigo | Cuando | Aplica a |
|--------|--------|----------|
| `TREE_NOT_FOUND` | El tree ID no existe | Ambos |
| `FEEDBACK_EDGE_NOT_FOUND` | El feedback ID no existe en ese tree | `feedback-rm` |

### feedback-rm error example

```json
{
  "success": false,
  "action": "link_feedback_rm",
  "data": {
    "removed_id": "",
    "tree_id": "tree-crt-salesanalysis"
  },
  "errors": [
    {
      "code": "FEEDBACK_EDGE_NOT_FOUND",
      "detail": "Feedback edge 'FB-999' not found in tree 'tree-crt-salesanalysis'"
    }
  ],
  "warnings": []
}
```

### feedback-list tree not found example

```json
{
  "success": false,
  "action": "link_feedback_list",
  "data": {
    "tree_id": "tree-ghost",
    "feedback_edges": []
  },
  "errors": [
    {
      "code": "TREE_NOT_FOUND",
      "detail": "tree not found: tree-ghost"
    }
  ],
  "warnings": []
}
```

---

## Modelo de datos: FeedbackEdge

```typescript
interface FeedbackEdge {
  id: string;         // "FB-001", "FB-002", ...
  from: string;       // Node ID (single, not array)
  to: string;         // Node ID (single, not array)
  loop_type: "positive" | "negative";
  label: string | null;  // null cuando no se proporciono label
}
```

**Nota**: `label` se serializa como `null`, no se omite del JSON. Si la UI necesita distinguir "sin label" de otro estado, comparar contra `null`.

---

## Contexto para la UI

- Los IDs de feedback edge son monotonos (`FB-001`, `FB-002`, ...). Nunca se reciclan tras borrar.
- `feedback-rm` soporta undo: tras `ltp undo`, la feedback edge reaparece intacta.
- No hay batch rm. Para borrar N feedback edges atomicamente: `ltp history begin-batch` + N llamadas a `feedback-rm` + `ltp history end-batch`. Un `undo` revierte todo el batch.
- `feedback-list` es solo lectura — no necesita confirmacion ni tiene side effects.
- Las feedback edges viven dentro del tree (no son entidades globales como los nodos). Si el tree se borra, las feedback edges desaparecen con el.
- `ltp status` reporta `feedback_edge_count` por tree (solo el conteo, no los detalles).
- `ltp trace` incluye feedback edges por defecto (en `feedback_loops[]`), excluibles con `--no-feedback`.

---

## Tool count MCP

61 -> 63 (2 tools nuevos). Total actual: **63 tools**.
