use serde_json::json;

use crate::mcp::types::ToolDefinition;

/// Generate the complete list of MCP tool definitions with JSON Schema inputs.
pub fn all_tools() -> Vec<ToolDefinition> {
    let mut tools = Vec::with_capacity(54);

    // --- Workspace ---
    tools.push(tool(
        "ltp/init",
        "Initialize a new LTP workspace",
        json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "description": "Workspace name" }
            }
        }),
    ));
    tools.push(tool(
        "ltp/status",
        "Show workspace health status",
        json!({ "type": "object", "properties": {} }),
    ));

    // --- Node ---
    tools.push(tool(
        "ltp/node_add",
        "Add a node to the global pool",
        json!({
            "type": "object",
            "properties": {
                "label": { "type": "string", "description": "Node label text" },
                "type": { "type": "string", "description": "Node type (UDE, RC, INJ, NC, GOAL, OBJ, WANT, OBS, IO, INT, DE, REQ, PRE)" },
                "tags": { "type": "array", "items": { "type": "string" }, "description": "Optional tags" },
                "observable": { "type": "boolean", "description": "Whether the node is observable" },
                "epistemic": { "type": "string", "enum": ["fact", "hypothesis", "assumption", "derived"], "description": "Epistemic status (default: hypothesis)" }
            },
            "required": ["label", "type"]
        }),
    ));
    tools.push(tool(
        "ltp/node_edit",
        "Edit an existing node",
        json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "Node ID" },
                "label": { "type": "string", "description": "New label" },
                "add_tag": { "type": "string", "description": "Tag to add" },
                "rm_tag": { "type": "string", "description": "Tag to remove" },
                "observable": { "type": "boolean", "description": "Observable flag" },
                "epistemic": { "type": "string", "enum": ["fact", "hypothesis", "assumption", "derived"], "description": "Epistemic status" }
            },
            "required": ["id"]
        }),
    ));
    tools.push(tool(
        "ltp/node_rm",
        "Remove nodes from the pool",
        json!({
            "type": "object",
            "properties": {
                "ids": { "type": "array", "items": { "type": "string" }, "description": "Node IDs to remove" },
                "force": { "type": "boolean", "description": "Force removal even if referenced" }
            },
            "required": ["ids"]
        }),
    ));
    tools.push(tool(
        "ltp/node_inspect",
        "Inspect a node's full details",
        json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "Node ID" }
            },
            "required": ["id"]
        }),
    ));
    tools.push(tool(
        "ltp/node_list",
        "List nodes, optionally filtered by tree/type/status/epistemic",
        json!({
            "type": "object",
            "properties": {
                "tree": { "type": "string", "description": "Filter by tree ID" },
                "type": { "type": "array", "items": { "type": "string" }, "description": "Filter by node types" },
                "status": { "type": "array", "items": { "type": "string" }, "description": "Filter by status" },
                "epistemic": { "type": "string", "enum": ["fact", "hypothesis", "assumption", "derived"], "description": "Filter by epistemic status" }
            }
        }),
    ));
    tools.push(tool(
        "ltp/node_search",
        "Search nodes by text query",
        json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Search query" },
                "tree": { "type": "string", "description": "Scope search to tree" }
            },
            "required": ["query"]
        }),
    ));
    tools.push(tool(
        "ltp/node_split",
        "Split a node into two",
        json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "Node ID to split" },
                "into": { "type": "array", "items": { "type": "string" }, "minItems": 2, "maxItems": 2, "description": "Two labels for the resulting nodes" },
                "tree": { "type": "string", "description": "Tree context" }
            },
            "required": ["id", "into", "tree"]
        }),
    ));

    // --- Tree ---
    tools.push(tool(
        "ltp/tree_new",
        "Create a new topological view (tree)",
        json!({
            "type": "object",
            "properties": {
                "type": { "type": "string", "description": "Tree type (CRT, FRT, EC, TT, PRT)" },
                "name": { "type": "string", "description": "Tree name" }
            },
            "required": ["type", "name"]
        }),
    ));
    tools.push(tool(
        "ltp/tree_list",
        "List all trees in the workspace",
        json!({ "type": "object", "properties": {} }),
    ));
    tools.push(tool(
        "ltp/tree_rm",
        "Remove a tree",
        json!({
            "type": "object",
            "properties": {
                "tree_id": { "type": "string", "description": "Tree ID to remove" }
            },
            "required": ["tree_id"]
        }),
    ));
    tools.push(tool(
        "ltp/tree_attach",
        "Attach a node to a tree",
        json!({
            "type": "object",
            "properties": {
                "tree": { "type": "string", "description": "Tree ID" },
                "node": { "type": "string", "description": "Node ID" },
                "role": { "type": "string", "description": "Role in tree (root, leaf, intermediate)" }
            },
            "required": ["tree", "node"]
        }),
    ));
    tools.push(tool(
        "ltp/tree_detach",
        "Detach a node from a tree",
        json!({
            "type": "object",
            "properties": {
                "tree": { "type": "string", "description": "Tree ID" },
                "node": { "type": "string", "description": "Node ID" }
            },
            "required": ["tree", "node"]
        }),
    ));
    tools.push(tool(
        "ltp/tree_clone",
        "Clone a tree with a new name",
        json!({
            "type": "object",
            "properties": {
                "tree_id": { "type": "string", "description": "Source tree ID" },
                "name": { "type": "string", "description": "Name for the clone" }
            },
            "required": ["tree_id", "name"]
        }),
    ));
    tools.push(tool(
        "ltp/tree_diff",
        "Compare two trees",
        json!({
            "type": "object",
            "properties": {
                "tree_a": { "type": "string", "description": "First tree ID" },
                "tree_b": { "type": "string", "description": "Second tree ID" }
            },
            "required": ["tree_a", "tree_b"]
        }),
    ));
    tools.push(tool(
        "ltp/tree_walk",
        "Walk a tree in topological or reverse order",
        json!({
            "type": "object",
            "properties": {
                "tree_id": { "type": "string", "description": "Tree ID" },
                "order": { "type": "string", "description": "Walk order: topological or reverse" },
                "show_origin": { "type": "boolean", "description": "Show node origin info" },
                "expand_nbr": { "type": "boolean", "description": "Expand NBR branches" }
            },
            "required": ["tree_id"]
        }),
    ));

    // --- Link ---
    tools.push(tool(
        "ltp/link_connect",
        "Create a cause-effect link between nodes",
        json!({
            "type": "object",
            "properties": {
                "tree": { "type": "string", "description": "Tree ID" },
                "from": { "type": "array", "items": { "type": "string" }, "description": "Cause node IDs" },
                "to": { "type": "array", "items": { "type": "string" }, "description": "Effect node IDs" },
                "operator": { "type": "string", "description": "Logical operator (AND, OR, MAG)" },
                "weight": { "type": "number", "description": "Optional weight for MAG" },
                "nbr": { "type": "string", "description": "Target NBR branch ID" }
            },
            "required": ["tree", "from", "to"]
        }),
    ));
    tools.push(tool(
        "ltp/link_disconnect",
        "Remove links from a tree",
        json!({
            "type": "object",
            "properties": {
                "tree": { "type": "string", "description": "Tree ID" },
                "links": { "type": "array", "items": { "type": "string" }, "description": "Link IDs to remove" }
            },
            "required": ["tree", "links"]
        }),
    ));
    tools.push(tool(
        "ltp/link_feedback",
        "Create a feedback loop edge",
        json!({
            "type": "object",
            "properties": {
                "tree": { "type": "string", "description": "Tree ID" },
                "from": { "type": "string", "description": "Source node ID" },
                "to": { "type": "string", "description": "Target node ID" },
                "type": { "type": "string", "description": "Feedback type: positive or negative" },
                "label": { "type": "string", "description": "Optional label" }
            },
            "required": ["tree", "from", "to", "type"]
        }),
    ));
    tools.push(tool(
        "ltp/link_inspect",
        "Inspect a link's full details",
        json!({
            "type": "object",
            "properties": {
                "link_id": { "type": "string", "description": "Link ID" },
                "tree": { "type": "string", "description": "Tree ID" }
            },
            "required": ["link_id", "tree"]
        }),
    ));
    tools.push(tool(
        "ltp/link_find",
        "Find links between two nodes",
        json!({
            "type": "object",
            "properties": {
                "tree": { "type": "string", "description": "Tree ID" },
                "from": { "type": "string", "description": "Source node ID" },
                "to": { "type": "string", "description": "Target node ID" }
            },
            "required": ["tree", "from", "to"]
        }),
    ));
    tools.push(tool(
        "ltp/link_reverse",
        "Reverse the direction of a link",
        json!({
            "type": "object",
            "properties": {
                "tree": { "type": "string", "description": "Tree ID" },
                "link": { "type": "string", "description": "Link ID" },
                "force": { "type": "boolean", "description": "Force even if creates cycle" }
            },
            "required": ["tree", "link"]
        }),
    ));
    tools.push(tool(
        "ltp/link_move",
        "Move a link's endpoint(s)",
        json!({
            "type": "object",
            "properties": {
                "tree": { "type": "string", "description": "Tree ID" },
                "link": { "type": "string", "description": "Link ID" },
                "new_from": { "type": "string", "description": "New source node ID" },
                "new_to": { "type": "string", "description": "New target node ID" }
            },
            "required": ["tree", "link"]
        }),
    ));
    tools.push(tool(
        "ltp/link_insert_between",
        "Insert a node in the middle of an existing link",
        json!({
            "type": "object",
            "properties": {
                "tree": { "type": "string", "description": "Tree ID" },
                "link": { "type": "string", "description": "Link ID" },
                "node": { "type": "string", "description": "Node ID to insert" },
                "insert_after_cause": { "type": "string", "description": "Specific cause to insert after" },
                "insert_before_effect": { "type": "boolean", "description": "Insert before the effect" }
            },
            "required": ["tree", "link", "node"]
        }),
    ));
    tools.push(tool(
        "ltp/link_group",
        "Group multiple links under a logical operator",
        json!({
            "type": "object",
            "properties": {
                "tree": { "type": "string", "description": "Tree ID" },
                "links": { "type": "array", "items": { "type": "string" }, "description": "Link IDs to group" },
                "operator": { "type": "string", "description": "Operator (AND, OR, MAG)" }
            },
            "required": ["tree", "links", "operator"]
        }),
    ));
    tools.push(tool(
        "ltp/link_dissolve",
        "Dissolve a grouped link back to individual links",
        json!({
            "type": "object",
            "properties": {
                "tree": { "type": "string", "description": "Tree ID" },
                "link": { "type": "string", "description": "Link ID to dissolve" }
            },
            "required": ["tree", "link"]
        }),
    ));
    tools.push(tool(
        "ltp/link_split",
        "Split causes out of a grouped link",
        json!({
            "type": "object",
            "properties": {
                "tree": { "type": "string", "description": "Tree ID" },
                "link": { "type": "string", "description": "Link ID" },
                "extract": { "type": "array", "items": { "type": "string" }, "description": "Cause node IDs to extract" }
            },
            "required": ["tree", "link", "extract"]
        }),
    ));
    tools.push(tool(
        "ltp/link_reoperator",
        "Change the operator of a grouped link",
        json!({
            "type": "object",
            "properties": {
                "tree": { "type": "string", "description": "Tree ID" },
                "link": { "type": "string", "description": "Link ID" },
                "operator": { "type": "string", "description": "New operator (AND, OR, MAG)" }
            },
            "required": ["tree", "link", "operator"]
        }),
    ));
    tools.push(tool(
        "ltp/link_add_cause",
        "Add a cause to an existing grouped link",
        json!({
            "type": "object",
            "properties": {
                "tree": { "type": "string", "description": "Tree ID" },
                "link": { "type": "string", "description": "Link ID" },
                "node": { "type": "string", "description": "Node ID to add as cause" },
                "weight": { "type": "number", "description": "Weight for MAG" },
                "promote_to": { "type": "string", "description": "Promote single link to group operator" }
            },
            "required": ["tree", "link", "node"]
        }),
    ));
    tools.push(tool(
        "ltp/link_rm_cause",
        "Remove a cause from a grouped link",
        json!({
            "type": "object",
            "properties": {
                "tree": { "type": "string", "description": "Tree ID" },
                "link": { "type": "string", "description": "Link ID" },
                "node": { "type": "string", "description": "Node ID to remove" }
            },
            "required": ["tree", "link", "node"]
        }),
    ));

    // --- Assume ---
    tools.push(tool(
        "ltp/assume_add",
        "Add an assumption to a link",
        json!({
            "type": "object",
            "properties": {
                "tree": { "type": "string", "description": "Tree ID" },
                "link": { "type": "string", "description": "Link ID" },
                "text": { "type": "string", "description": "Assumption text" }
            },
            "required": ["tree", "link", "text"]
        }),
    ));
    tools.push(tool(
        "ltp/assume_edit",
        "Edit an existing assumption",
        json!({
            "type": "object",
            "properties": {
                "tree": { "type": "string", "description": "Tree ID" },
                "asm": { "type": "string", "description": "Assumption ID" },
                "text": { "type": "string", "description": "New text" }
            },
            "required": ["tree", "asm", "text"]
        }),
    ));
    tools.push(tool(
        "ltp/assume_rm",
        "Remove an assumption",
        json!({
            "type": "object",
            "properties": {
                "tree": { "type": "string", "description": "Tree ID" },
                "asm": { "type": "string", "description": "Assumption ID" }
            },
            "required": ["tree", "asm"]
        }),
    ));
    tools.push(tool(
        "ltp/assume_list",
        "List assumptions in a tree",
        json!({
            "type": "object",
            "properties": {
                "tree": { "type": "string", "description": "Tree ID" },
                "status": { "type": "string", "description": "Filter by status (valid, invalidated)" }
            },
            "required": ["tree"]
        }),
    ));
    tools.push(tool(
        "ltp/assume_move",
        "Move an assumption to another link",
        json!({
            "type": "object",
            "properties": {
                "tree": { "type": "string", "description": "Tree ID" },
                "asm": { "type": "string", "description": "Assumption ID" },
                "to_link": { "type": "string", "description": "Target link ID" }
            },
            "required": ["tree", "asm", "to_link"]
        }),
    ));

    // --- Invalidate ---
    tools.push(tool(
        "ltp/invalidate",
        "Invalidate a link via an assumption, optionally creating an injection",
        json!({
            "type": "object",
            "properties": {
                "tree": { "type": "string", "description": "Tree ID" },
                "link": { "type": "string", "description": "Link ID" },
                "asm": { "type": "string", "description": "Assumption ID to invalidate" },
                "injection": { "type": "string", "description": "Label for new injection node" }
            },
            "required": ["tree", "link", "asm"]
        }),
    ));

    // --- Validate ---
    tools.push(tool(
        "ltp/validate",
        "Validate workspace or specific tree (DAG, integrity, CLR linting)",
        json!({
            "type": "object",
            "properties": {
                "tree": { "type": "string", "description": "Specific tree to validate (all if omitted)" }
            }
        }),
    ));

    // --- Trace ---
    tools.push(tool(
        "ltp/trace",
        "Trace upstream or downstream from a node",
        json!({
            "type": "object",
            "properties": {
                "node_id": { "type": "string", "description": "Node ID to trace from" },
                "tree": { "type": "string", "description": "Tree to trace within" },
                "direction": { "type": "string", "description": "Direction: upstream or downstream" },
                "depth": { "type": "integer", "description": "Max traversal depth" },
                "no_feedback": { "type": "boolean", "description": "Exclude feedback edges" },
                "nbr": { "type": "boolean", "description": "Include NBR edges" }
            },
            "required": ["node_id", "tree", "direction"]
        }),
    ));

    // --- Path ---
    tools.push(tool(
        "ltp/path_collapse",
        "Collapse a path between two nodes into a macro-link",
        json!({
            "type": "object",
            "properties": {
                "tree": { "type": "string", "description": "Tree ID" },
                "from": { "type": "string", "description": "Start node ID" },
                "to": { "type": "string", "description": "End node ID" },
                "label": { "type": "string", "description": "Label for the macro-link" }
            },
            "required": ["tree", "from", "to", "label"]
        }),
    ));
    tools.push(tool(
        "ltp/path_explode",
        "Explode a macro-link back to its original sub-graph",
        json!({
            "type": "object",
            "properties": {
                "tree": { "type": "string", "description": "Tree ID" },
                "link": { "type": "string", "description": "Macro-link ID" },
                "asm": { "type": "string", "description": "Assumption ID" },
                "label": { "type": "string", "description": "Label" }
            },
            "required": ["tree", "link", "asm", "label"]
        }),
    ));
    tools.push(tool(
        "ltp/path_replace",
        "Replace a macro-link with a new intermediary node",
        json!({
            "type": "object",
            "properties": {
                "tree": { "type": "string", "description": "Tree ID" },
                "macro_link": { "type": "string", "description": "Macro-link ID" },
                "by_node": { "type": "string", "description": "Node ID to insert" }
            },
            "required": ["tree", "macro_link", "by_node"]
        }),
    ));

    // --- NBR ---
    tools.push(tool(
        "ltp/nbr_add",
        "Add a Negative Branch Reservation to a tree",
        json!({
            "type": "object",
            "properties": {
                "tree": { "type": "string", "description": "Tree ID" },
                "source_node": { "type": "string", "description": "Source injection node" },
                "trim": { "type": "string", "description": "Trim injection node ID" }
            },
            "required": ["tree", "source_node"]
        }),
    ));
    tools.push(tool(
        "ltp/nbr_rm",
        "Remove a Negative Branch Reservation",
        json!({
            "type": "object",
            "properties": {
                "tree": { "type": "string", "description": "Tree ID" },
                "nbr": { "type": "string", "description": "NBR ID" }
            },
            "required": ["tree", "nbr"]
        }),
    ));
    tools.push(tool(
        "ltp/nbr_list",
        "List all NBRs in a tree",
        json!({
            "type": "object",
            "properties": {
                "tree": { "type": "string", "description": "Tree ID" }
            },
            "required": ["tree"]
        }),
    ));
    tools.push(tool(
        "ltp/nbr_inspect",
        "Inspect a specific NBR",
        json!({
            "type": "object",
            "properties": {
                "nbr_id": { "type": "string", "description": "NBR ID" },
                "tree": { "type": "string", "description": "Tree ID" }
            },
            "required": ["nbr_id", "tree"]
        }),
    ));

    // --- History ---
    tools.push(tool(
        "ltp/undo",
        "Undo the last operation",
        json!({
            "type": "object",
            "properties": {
                "dry_run": { "type": "boolean", "description": "Preview without executing" }
            }
        }),
    ));
    tools.push(tool(
        "ltp/redo",
        "Redo the last undone operation",
        json!({
            "type": "object",
            "properties": {
                "dry_run": { "type": "boolean", "description": "Preview without executing" }
            }
        }),
    ));
    tools.push(tool(
        "ltp/history_list",
        "List operation history",
        json!({
            "type": "object",
            "properties": {
                "last": { "type": "integer", "description": "Show last N entries" }
            }
        }),
    ));
    tools.push(tool(
        "ltp/history_check",
        "Check history integrity (checksums)",
        json!({ "type": "object", "properties": {} }),
    ));
    tools.push(tool(
        "ltp/history_invalidate",
        "Invalidate history entries from a sequence number",
        json!({
            "type": "object",
            "properties": {
                "from": { "type": "integer", "description": "Sequence number to invalidate from" }
            }
        }),
    ));
    tools.push(tool(
        "ltp/history_begin_batch",
        "Begin a batch of operations (collapsed into one undo entry)",
        json!({
            "type": "object",
            "properties": {
                "label": { "type": "string", "description": "Batch label" }
            },
            "required": ["label"]
        }),
    ));
    tools.push(tool(
        "ltp/history_end_batch",
        "End the current batch",
        json!({ "type": "object", "properties": {} }),
    ));
    tools.push(tool(
        "ltp/history_clear",
        "Clear all history",
        json!({ "type": "object", "properties": {} }),
    ));

    tools
}

fn tool(name: &str, description: &str, input_schema: serde_json::Value) -> ToolDefinition {
    ToolDefinition {
        name: name.to_string(),
        description: description.to_string(),
        input_schema,
    }
}
