use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Serialize;
use serde_json::Value;

use crate::assume::{
    execute_assume_add, execute_assume_edit, execute_assume_list, execute_assume_move,
    execute_assume_rm, execute_invalidate,
};
use crate::history::{
    execute_history_begin_batch, execute_history_check, execute_history_clear,
    execute_history_end_batch, execute_history_invalidate, execute_history_list, execute_redo,
    execute_undo, CaptureContext, HistoryManager,
};
use crate::link::advanced::{
    execute_link_add_cause, execute_link_dissolve, execute_link_group, execute_link_insert_between,
    execute_link_move, execute_link_reoperator, execute_link_reverse, execute_link_rm_cause,
    execute_link_split,
};
use crate::link::commands::{execute_link_connect, execute_link_disconnect, execute_link_feedback};
use crate::mcp::types::{JsonRpcError, ToolCallResult};
use crate::nbr::{execute_nbr_add, execute_nbr_inspect, execute_nbr_list, execute_nbr_rm};
use crate::node::commands::{
    execute_node_add, execute_node_edit, execute_node_inspect, execute_node_list, execute_node_rm,
    execute_node_search, execute_node_split,
};
use crate::output::CommandOutput;
use crate::path::{execute_path_collapse, execute_path_explode, execute_path_replace};
use crate::storage::Storage;
use crate::trace::{execute_link_find, execute_link_inspect, execute_trace};
use crate::tree::commands::{
    execute_tree_attach, execute_tree_clone, execute_tree_detach, execute_tree_diff,
    execute_tree_list, execute_tree_new, execute_tree_rm, execute_tree_walk,
};
use crate::validate::execute_validate;
use crate::workspace::FsStorage;

/// Dispatch a tool call to the appropriate engine function.
///
/// Returns `Ok(ToolCallResult)` on successful dispatch (even if the command itself fails),
/// or `Err(JsonRpcError)` for protocol-level errors (invalid params, workspace not initialized).
pub fn dispatch_tool(
    name: &str,
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    // Tools that don't require workspace to exist
    if name == "ltp/init" {
        return dispatch_init(args, storage);
    }

    // All other tools require workspace
    if !storage.workspace_exists() {
        return Err(JsonRpcError::workspace_not_initialized());
    }

    match name {
        "ltp/status" => dispatch_status(storage),
        "ltp/node_add" => dispatch_node_add(args, storage),
        "ltp/node_edit" => dispatch_node_edit(args, storage),
        "ltp/node_rm" => dispatch_node_rm(args, storage),
        "ltp/node_inspect" => dispatch_node_inspect(args, storage),
        "ltp/node_list" => dispatch_node_list(args, storage),
        "ltp/node_search" => dispatch_node_search(args, storage),
        "ltp/node_split" => dispatch_node_split(args, storage),
        "ltp/tree_new" => dispatch_tree_new(args, storage),
        "ltp/tree_list" => dispatch_tree_list(storage),
        "ltp/tree_rm" => dispatch_tree_rm(args, storage),
        "ltp/tree_attach" => dispatch_tree_attach(args, storage),
        "ltp/tree_detach" => dispatch_tree_detach(args, storage),
        "ltp/tree_clone" => dispatch_tree_clone(args, storage),
        "ltp/tree_diff" => dispatch_tree_diff(args, storage),
        "ltp/tree_walk" => dispatch_tree_walk(args, storage),
        "ltp/link_connect" => dispatch_link_connect(args, storage),
        "ltp/link_disconnect" => dispatch_link_disconnect(args, storage),
        "ltp/link_feedback" => dispatch_link_feedback(args, storage),
        "ltp/link_inspect" => dispatch_link_inspect(args, storage),
        "ltp/link_find" => dispatch_link_find(args, storage),
        "ltp/link_reverse" => dispatch_link_reverse(args, storage),
        "ltp/link_move" => dispatch_link_move(args, storage),
        "ltp/link_insert_between" => dispatch_link_insert_between(args, storage),
        "ltp/link_group" => dispatch_link_group(args, storage),
        "ltp/link_dissolve" => dispatch_link_dissolve(args, storage),
        "ltp/link_split" => dispatch_link_split(args, storage),
        "ltp/link_reoperator" => dispatch_link_reoperator(args, storage),
        "ltp/link_add_cause" => dispatch_link_add_cause(args, storage),
        "ltp/link_rm_cause" => dispatch_link_rm_cause(args, storage),
        "ltp/assume_add" => dispatch_assume_add(args, storage),
        "ltp/assume_edit" => dispatch_assume_edit(args, storage),
        "ltp/assume_rm" => dispatch_assume_rm(args, storage),
        "ltp/assume_list" => dispatch_assume_list(args, storage),
        "ltp/assume_move" => dispatch_assume_move(args, storage),
        "ltp/invalidate" => dispatch_invalidate(args, storage),
        "ltp/validate" => dispatch_validate(args, storage),
        "ltp/trace" => dispatch_trace(args, storage),
        "ltp/path_collapse" => dispatch_path_collapse(args, storage),
        "ltp/path_explode" => dispatch_path_explode(args, storage),
        "ltp/path_replace" => dispatch_path_replace(args, storage),
        "ltp/nbr_add" => dispatch_nbr_add(args, storage),
        "ltp/nbr_rm" => dispatch_nbr_rm(args, storage),
        "ltp/nbr_list" => dispatch_nbr_list(args, storage),
        "ltp/nbr_inspect" => dispatch_nbr_inspect(args, storage),
        "ltp/undo" => dispatch_undo(args, storage),
        "ltp/redo" => dispatch_redo(args, storage),
        "ltp/history_list" => dispatch_history_list(args, storage),
        "ltp/history_check" => dispatch_history_check(storage),
        "ltp/history_invalidate" => dispatch_history_invalidate(args, storage),
        "ltp/history_begin_batch" => dispatch_history_begin_batch(args, storage),
        "ltp/history_end_batch" => dispatch_history_end_batch(storage),
        "ltp/history_clear" => dispatch_history_clear(storage),
        _ => Err(JsonRpcError::method_not_found(name)),
    }
}

// --- Helpers ---

fn get_str<'a>(args: &'a BTreeMap<String, Value>, key: &str) -> Result<&'a str, JsonRpcError> {
    args.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| JsonRpcError::invalid_params(&format!("missing required field: {key}")))
}

fn get_str_opt<'a>(args: &'a BTreeMap<String, Value>, key: &str) -> Option<&'a str> {
    args.get(key).and_then(|v| v.as_str())
}

fn get_bool(args: &BTreeMap<String, Value>, key: &str) -> bool {
    args.get(key).and_then(|v| v.as_bool()).unwrap_or(false)
}

fn get_f64_opt(args: &BTreeMap<String, Value>, key: &str) -> Option<f64> {
    args.get(key).and_then(|v| v.as_f64())
}

fn get_u64_opt(args: &BTreeMap<String, Value>, key: &str) -> Option<u64> {
    args.get(key).and_then(|v| v.as_u64())
}

fn get_str_array(args: &BTreeMap<String, Value>, key: &str) -> Result<Vec<String>, JsonRpcError> {
    match args.get(key) {
        Some(Value::Array(arr)) => {
            let mut result = Vec::with_capacity(arr.len());
            for item in arr {
                match item.as_str() {
                    Some(s) => result.push(s.to_string()),
                    None => {
                        return Err(JsonRpcError::invalid_params(&format!(
                            "field '{key}' must be an array of strings"
                        )))
                    }
                }
            }
            Ok(result)
        }
        Some(_) => Err(JsonRpcError::invalid_params(&format!(
            "field '{key}' must be an array"
        ))),
        None => Err(JsonRpcError::invalid_params(&format!(
            "missing required field: {key}"
        ))),
    }
}

fn get_str_array_opt(args: &BTreeMap<String, Value>, key: &str) -> Option<Vec<String>> {
    args.get(key).and_then(|v| {
        v.as_array().map(|arr| {
            arr.iter()
                .filter_map(|item| item.as_str().map(|s| s.to_string()))
                .collect()
        })
    })
}

fn to_result<T: Serialize>(output: &CommandOutput<T>) -> Result<ToolCallResult, JsonRpcError> {
    let value = serde_json::to_value(output)
        .map_err(|e| JsonRpcError::invalid_params(&format!("serialization error: {e}")))?;
    Ok(ToolCallResult::from_json(&value, !output.success))
}

/// Begin history capture for a mutating operation.
fn history_begin(storage: &FsStorage) -> Option<(HistoryManager, CaptureContext)> {
    let config = storage.load_config().ok()?;
    if !config.history.enabled {
        return None;
    }
    let manager = HistoryManager::new(storage.root().to_path_buf(), config.history);
    if manager.is_batch_active() {
        return None;
    }
    let paths = snapshot_workspace_paths(storage.root());
    let ctx = manager.begin_capture(&paths).ok()?;
    Some((manager, ctx))
}

fn history_commit(capture: Option<(HistoryManager, CaptureContext)>, action: &str, command: &str) {
    if let Some((manager, ctx)) = capture {
        let _ = manager.commit_capture(ctx, action, command);
    }
}

fn snapshot_workspace_paths(root: &std::path::Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let nodes_dir = root.join("nodes");
    let trees_dir = root.join("trees");

    if let Ok(entries) = std::fs::read_dir(&nodes_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                paths.push(path);
            }
        }
    }
    if let Ok(entries) = std::fs::read_dir(&trees_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                paths.push(path);
            }
        }
    }
    paths
}

// --- Workspace ---

fn dispatch_init(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let name = get_str_opt(args, "name").unwrap_or("ltp-workspace");

    if storage.workspace_exists() {
        let output = crate::output::error_output(
            "init",
            name,
            vec![crate::output::OutputError::new(
                "WORKSPACE_ALREADY_EXISTS",
                format!(
                    "Workspace already initialized at {}",
                    storage.root().display()
                ),
            )],
        );
        return to_result(&output);
    }

    if let Err(e) = storage.init_workspace(name) {
        let output = crate::output::error_output(
            "init",
            name,
            vec![crate::output::OutputError::new("INIT_ERROR", e.to_string())],
        );
        return to_result(&output);
    }

    let output = CommandOutput::ok(
        "init",
        name,
        serde_json::json!({
            "directories_created": ["nodes/", "trees/", "knowledge/", ".ltp/", ".ltp/undo/", ".ltp/redo/", ".ltp/tmp/"],
            "config_file": "ltp.config.json",
            "counters_file": ".ltp/counters.json"
        }),
    );
    to_result(&output)
}

fn dispatch_status(storage: &FsStorage) -> Result<ToolCallResult, JsonRpcError> {
    use crate::tree::Tree;
    use crate::validate::check_dag;

    let ws_name = storage.workspace_name().unwrap_or_default();
    let node_ids = storage
        .list_node_ids()
        .map_err(|e| JsonRpcError::invalid_params(&e.to_string()))?;
    let tree_ids = storage
        .list_tree_ids()
        .map_err(|e| JsonRpcError::invalid_params(&e.to_string()))?;

    let mut trees_health = Vec::new();
    let mut all_valid_dag = true;
    let mut referenced_nodes: std::collections::HashSet<String> = std::collections::HashSet::new();

    for tree_id in &tree_ids {
        let tree: Tree = match storage.load_tree(tree_id) {
            Ok(t) => t,
            Err(_) => continue,
        };

        let dag_valid = check_dag(&tree.edges, &tree.id).is_ok();
        if !dag_valid {
            all_valid_dag = false;
        }

        for node_ref in &tree.nodes {
            referenced_nodes.insert(node_ref.node_ref.clone());
        }

        trees_health.push(serde_json::json!({
            "id": tree.id,
            "name": tree.name,
            "edge_count": tree.edges.len(),
            "feedback_edge_count": tree.feedback_edges.len(),
            "valid_dag": dag_valid,
        }));
    }

    let orphan_count = node_ids
        .iter()
        .filter(|id| !referenced_nodes.contains(*id))
        .count();

    let mut output = CommandOutput::ok(
        "status",
        &ws_name,
        serde_json::json!({
            "node_count": node_ids.len(),
            "tree_count": tree_ids.len(),
            "trees": trees_health,
        }),
    );
    output.graph_health = crate::output::GraphHealth {
        valid_dag: all_valid_dag,
        orphan_nodes_count: orphan_count,
    };
    to_result(&output)
}

// --- Node ---

fn dispatch_node_add(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let label = get_str(args, "label")?;
    let node_type = get_str(args, "type")?;
    let tags = get_str_array_opt(args, "tags");
    let observable = args.get("observable").and_then(|v| v.as_bool());
    let epistemic = get_str_opt(args, "epistemic");

    let capture = history_begin(storage);
    let output = execute_node_add(storage, label, node_type, tags, observable, epistemic);
    if output.success {
        history_commit(capture, "node_add", &format!("mcp:ltp/node_add {label}"));
    }
    to_result(&output)
}

fn dispatch_node_edit(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let id = get_str(args, "id")?;
    let label = get_str_opt(args, "label");
    let add_tag = get_str_opt(args, "add_tag");
    let rm_tag = get_str_opt(args, "rm_tag");
    let observable = args.get("observable").and_then(|v| v.as_bool());
    let epistemic = get_str_opt(args, "epistemic");

    let capture = history_begin(storage);
    let output = execute_node_edit(storage, id, label, add_tag, rm_tag, observable, epistemic);
    if output.success {
        history_commit(capture, "node_edit", &format!("mcp:ltp/node_edit {id}"));
    }
    to_result(&output)
}

fn dispatch_node_rm(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let ids = get_str_array(args, "ids")?;
    let force = get_bool(args, "force");

    let capture = history_begin(storage);
    let output = execute_node_rm(storage, &ids, force);
    if output.success {
        history_commit(capture, "node_rm", "mcp:ltp/node_rm");
    }
    to_result(&output)
}

fn dispatch_node_inspect(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let id = get_str(args, "id")?;
    let output = execute_node_inspect(storage, id);
    to_result(&output)
}

fn dispatch_node_list(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let tree = get_str_opt(args, "tree");
    let type_filter = get_str_array_opt(args, "type");
    let status_filter = get_str_array_opt(args, "status");
    let epistemic = get_str_opt(args, "epistemic");
    let output = execute_node_list(
        storage,
        tree,
        type_filter.as_deref(),
        status_filter.as_deref(),
        epistemic,
    );
    to_result(&output)
}

fn dispatch_node_search(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let query = get_str(args, "query")?;
    let output = execute_node_search(storage, query);
    to_result(&output)
}

fn dispatch_node_split(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let id = get_str(args, "id")?;
    let into = get_str_array(args, "into")?;
    let tree = get_str(args, "tree")?;

    let capture = history_begin(storage);
    let output = execute_node_split(storage, id, &into, tree);
    if output.success {
        history_commit(capture, "node_split", &format!("mcp:ltp/node_split {id}"));
    }
    to_result(&output)
}

// --- Tree ---

fn dispatch_tree_new(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let tree_type = get_str(args, "type")?;
    let name = get_str(args, "name")?;

    let capture = history_begin(storage);
    let output = execute_tree_new(storage, tree_type, name);
    if output.success {
        history_commit(capture, "tree_new", &format!("mcp:ltp/tree_new {name}"));
    }
    to_result(&output)
}

fn dispatch_tree_list(storage: &FsStorage) -> Result<ToolCallResult, JsonRpcError> {
    let output = execute_tree_list(storage);
    to_result(&output)
}

fn dispatch_tree_rm(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let tree_id = get_str(args, "tree_id")?;

    let capture = history_begin(storage);
    let output = execute_tree_rm(storage, tree_id);
    if output.success {
        history_commit(capture, "tree_rm", &format!("mcp:ltp/tree_rm {tree_id}"));
    }
    to_result(&output)
}

fn dispatch_tree_attach(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let tree = get_str(args, "tree")?;
    let node = get_str(args, "node")?;
    let role = get_str_opt(args, "role");

    let capture = history_begin(storage);
    let output = execute_tree_attach(storage, tree, node, role);
    if output.success {
        history_commit(capture, "tree_attach", "mcp:ltp/tree_attach");
    }
    to_result(&output)
}

fn dispatch_tree_detach(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let tree = get_str(args, "tree")?;
    let node = get_str(args, "node")?;

    let capture = history_begin(storage);
    let output = execute_tree_detach(storage, tree, node);
    if output.success {
        history_commit(capture, "tree_detach", "mcp:ltp/tree_detach");
    }
    to_result(&output)
}

fn dispatch_tree_clone(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let tree_id = get_str(args, "tree_id")?;
    let name = get_str(args, "name")?;

    let capture = history_begin(storage);
    let output = execute_tree_clone(storage, tree_id, name);
    if output.success {
        history_commit(capture, "tree_clone", "mcp:ltp/tree_clone");
    }
    to_result(&output)
}

fn dispatch_tree_diff(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let tree_a = get_str(args, "tree_a")?;
    let tree_b = get_str(args, "tree_b")?;
    let output = execute_tree_diff(storage, tree_a, tree_b);
    to_result(&output)
}

fn dispatch_tree_walk(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let tree_id = get_str(args, "tree_id")?;
    let order = get_str_opt(args, "order").unwrap_or("topological");
    let show_knowledge = get_bool(args, "show_knowledge");
    let output = execute_tree_walk(storage, tree_id, order, show_knowledge);
    to_result(&output)
}

// --- Link ---

fn dispatch_link_connect(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let tree = get_str(args, "tree")?;
    let from = get_str_array(args, "from")?;
    let to = get_str_array(args, "to")?;
    let operator = get_str_opt(args, "operator");
    let weight = get_f64_opt(args, "weight");
    let nbr = get_str_opt(args, "nbr");

    let capture = history_begin(storage);
    let output = execute_link_connect(storage, tree, &from, &to, operator, weight, nbr);
    if output.success {
        history_commit(capture, "link_connect", "mcp:ltp/link_connect");
    }
    to_result(&output)
}

fn dispatch_link_disconnect(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let tree = get_str(args, "tree")?;
    let links = get_str_array(args, "links")?;

    let capture = history_begin(storage);
    let output = execute_link_disconnect(storage, tree, &links);
    if output.success {
        history_commit(capture, "link_disconnect", "mcp:ltp/link_disconnect");
    }
    to_result(&output)
}

fn dispatch_link_feedback(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let tree = get_str(args, "tree")?;
    let from = get_str(args, "from")?;
    let to = get_str(args, "to")?;
    let fb_type = get_str(args, "type")?;
    let label = get_str_opt(args, "label");

    let capture = history_begin(storage);
    let output = execute_link_feedback(storage, tree, from, to, fb_type, label);
    if output.success {
        history_commit(capture, "link_feedback", "mcp:ltp/link_feedback");
    }
    to_result(&output)
}

fn dispatch_link_inspect(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let link_id = get_str(args, "link_id")?;
    let tree = get_str(args, "tree")?;
    let output = execute_link_inspect(storage, link_id, tree);
    to_result(&output)
}

fn dispatch_link_find(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let tree = get_str(args, "tree")?;
    let from = get_str(args, "from")?;
    let to = get_str(args, "to")?;
    let output = execute_link_find(storage, tree, from, to);
    to_result(&output)
}

fn dispatch_link_reverse(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let tree = get_str(args, "tree")?;
    let link = get_str(args, "link")?;
    let force = get_bool(args, "force");

    let capture = history_begin(storage);
    let output = execute_link_reverse(storage, tree, link, force);
    if output.success {
        history_commit(capture, "link_reverse", "mcp:ltp/link_reverse");
    }
    to_result(&output)
}

fn dispatch_link_move(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let tree = get_str(args, "tree")?;
    let link = get_str(args, "link")?;
    let new_from = get_str_opt(args, "new_from");
    let new_to = get_str_opt(args, "new_to");

    let capture = history_begin(storage);
    let output = execute_link_move(storage, tree, link, new_from, new_to);
    if output.success {
        history_commit(capture, "link_move", "mcp:ltp/link_move");
    }
    to_result(&output)
}

fn dispatch_link_insert_between(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let tree = get_str(args, "tree")?;
    let link = get_str(args, "link")?;
    let node = get_str(args, "node")?;
    let insert_after_cause = get_str_opt(args, "insert_after_cause");
    let insert_before_effect = get_bool(args, "insert_before_effect");

    let capture = history_begin(storage);
    let output = execute_link_insert_between(
        storage,
        tree,
        link,
        node,
        insert_after_cause,
        insert_before_effect,
    );
    if output.success {
        history_commit(
            capture,
            "link_insert_between",
            "mcp:ltp/link_insert_between",
        );
    }
    to_result(&output)
}

fn dispatch_link_group(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let tree = get_str(args, "tree")?;
    let links = get_str_array(args, "links")?;
    let operator = get_str(args, "operator")?;

    let capture = history_begin(storage);
    let output = execute_link_group(storage, tree, &links, operator);
    if output.success {
        history_commit(capture, "link_group", "mcp:ltp/link_group");
    }
    to_result(&output)
}

fn dispatch_link_dissolve(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let tree = get_str(args, "tree")?;
    let link = get_str(args, "link")?;

    let capture = history_begin(storage);
    let output = execute_link_dissolve(storage, tree, link);
    if output.success {
        history_commit(capture, "link_dissolve", "mcp:ltp/link_dissolve");
    }
    to_result(&output)
}

fn dispatch_link_split(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let tree = get_str(args, "tree")?;
    let link = get_str(args, "link")?;
    let extract = get_str_array(args, "extract")?;

    let capture = history_begin(storage);
    let output = execute_link_split(storage, tree, link, &extract);
    if output.success {
        history_commit(capture, "link_split", "mcp:ltp/link_split");
    }
    to_result(&output)
}

fn dispatch_link_reoperator(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let tree = get_str(args, "tree")?;
    let link = get_str(args, "link")?;
    let operator = get_str(args, "operator")?;

    let capture = history_begin(storage);
    let output = execute_link_reoperator(storage, tree, link, operator);
    if output.success {
        history_commit(capture, "link_reoperator", "mcp:ltp/link_reoperator");
    }
    to_result(&output)
}

fn dispatch_link_add_cause(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let tree = get_str(args, "tree")?;
    let link = get_str(args, "link")?;
    let node = get_str(args, "node")?;
    let weight = get_f64_opt(args, "weight");
    let promote_to = get_str_opt(args, "promote_to");

    let capture = history_begin(storage);
    let output = execute_link_add_cause(storage, tree, link, node, weight, promote_to);
    if output.success {
        history_commit(capture, "link_add_cause", "mcp:ltp/link_add_cause");
    }
    to_result(&output)
}

fn dispatch_link_rm_cause(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let tree = get_str(args, "tree")?;
    let link = get_str(args, "link")?;
    let node = get_str(args, "node")?;

    let capture = history_begin(storage);
    let output = execute_link_rm_cause(storage, tree, link, node);
    if output.success {
        history_commit(capture, "link_rm_cause", "mcp:ltp/link_rm_cause");
    }
    to_result(&output)
}

// --- Assume ---

fn dispatch_assume_add(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let tree = get_str(args, "tree")?;
    let link = get_str(args, "link")?;
    let text = get_str(args, "text")?;

    let capture = history_begin(storage);
    let output = execute_assume_add(storage, tree, link, text);
    if output.success {
        history_commit(capture, "assume_add", "mcp:ltp/assume_add");
    }
    to_result(&output)
}

fn dispatch_assume_edit(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let tree = get_str(args, "tree")?;
    let asm = get_str(args, "asm")?;
    let text = get_str(args, "text")?;

    let capture = history_begin(storage);
    let output = execute_assume_edit(storage, tree, asm, text);
    if output.success {
        history_commit(capture, "assume_edit", "mcp:ltp/assume_edit");
    }
    to_result(&output)
}

fn dispatch_assume_rm(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let tree = get_str(args, "tree")?;
    let asm = get_str(args, "asm")?;

    let capture = history_begin(storage);
    let output = execute_assume_rm(storage, tree, asm);
    if output.success {
        history_commit(capture, "assume_rm", "mcp:ltp/assume_rm");
    }
    to_result(&output)
}

fn dispatch_assume_list(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let tree = get_str(args, "tree")?;
    let status = get_str_opt(args, "status");
    let output = execute_assume_list(storage, tree, status);
    to_result(&output)
}

fn dispatch_assume_move(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let tree = get_str(args, "tree")?;
    let asm = get_str(args, "asm")?;
    let to_link = get_str(args, "to_link")?;

    let capture = history_begin(storage);
    let output = execute_assume_move(storage, tree, asm, to_link);
    if output.success {
        history_commit(capture, "assume_move", "mcp:ltp/assume_move");
    }
    to_result(&output)
}

// --- Invalidate ---

fn dispatch_invalidate(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let tree = get_str(args, "tree")?;
    let link = get_str(args, "link")?;
    let asm = get_str(args, "asm")?;
    let injection = get_str_opt(args, "injection");

    let capture = history_begin(storage);
    let output = execute_invalidate(storage, tree, link, asm, injection);
    if output.success {
        history_commit(capture, "invalidate", "mcp:ltp/invalidate");
    }
    to_result(&output)
}

// --- Validate ---

fn dispatch_validate(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let tree = get_str_opt(args, "tree");
    let output = execute_validate(storage, tree);
    to_result(&output)
}

// --- Trace ---

fn dispatch_trace(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let node_id = get_str(args, "node_id")?;
    let tree = get_str(args, "tree")?;
    let direction = get_str(args, "direction")?;
    let depth = get_u64_opt(args, "depth").map(|d| d as usize);
    let no_feedback = get_bool(args, "no_feedback");
    let nbr = get_bool(args, "nbr");
    let show_knowledge = get_bool(args, "show_knowledge");

    let output = execute_trace(
        storage,
        node_id,
        tree,
        direction,
        depth,
        no_feedback,
        nbr,
        show_knowledge,
    );
    to_result(&output)
}

// --- Path ---

fn dispatch_path_collapse(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let tree = get_str(args, "tree")?;
    let from = get_str(args, "from")?;
    let to = get_str(args, "to")?;
    let label = get_str(args, "label")?;

    let capture = history_begin(storage);
    let output = execute_path_collapse(storage, tree, from, to, label);
    if output.success {
        history_commit(capture, "path_collapse", "mcp:ltp/path_collapse");
    }
    to_result(&output)
}

fn dispatch_path_explode(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let tree = get_str(args, "tree")?;
    let link = get_str(args, "link")?;
    let asm = get_str(args, "asm")?;
    let label = get_str(args, "label")?;

    let capture = history_begin(storage);
    let output = execute_path_explode(storage, tree, link, asm, label);
    if output.success {
        history_commit(capture, "path_explode", "mcp:ltp/path_explode");
    }
    to_result(&output)
}

fn dispatch_path_replace(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let tree = get_str(args, "tree")?;
    let macro_link = get_str(args, "macro_link")?;
    let by_node = get_str(args, "by_node")?;

    let capture = history_begin(storage);
    let output = execute_path_replace(storage, tree, macro_link, by_node);
    if output.success {
        history_commit(capture, "path_replace", "mcp:ltp/path_replace");
    }
    to_result(&output)
}

// --- NBR ---

fn dispatch_nbr_add(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let tree = get_str(args, "tree")?;
    let source_node = get_str(args, "source_node")?;
    let trim = get_str_opt(args, "trim");

    let capture = history_begin(storage);
    let output = execute_nbr_add(storage, tree, source_node, trim);
    if output.success {
        history_commit(capture, "nbr_add", "mcp:ltp/nbr_add");
    }
    to_result(&output)
}

fn dispatch_nbr_rm(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let tree = get_str(args, "tree")?;
    let nbr = get_str(args, "nbr")?;

    let capture = history_begin(storage);
    let output = execute_nbr_rm(storage, tree, nbr);
    if output.success {
        history_commit(capture, "nbr_rm", "mcp:ltp/nbr_rm");
    }
    to_result(&output)
}

fn dispatch_nbr_list(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let tree = get_str(args, "tree")?;
    let output = execute_nbr_list(storage, tree);
    to_result(&output)
}

fn dispatch_nbr_inspect(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let tree = get_str(args, "tree")?;
    let nbr_id = get_str(args, "nbr_id")?;
    let output = execute_nbr_inspect(storage, tree, nbr_id);
    to_result(&output)
}

// --- History ---

fn dispatch_undo(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let dry_run = get_bool(args, "dry_run");
    let output = execute_undo(storage, dry_run);
    to_result(&output)
}

fn dispatch_redo(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let dry_run = get_bool(args, "dry_run");
    let output = execute_redo(storage, dry_run);
    to_result(&output)
}

fn dispatch_history_list(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let last = get_u64_opt(args, "last").map(|n| n as usize);
    let output = execute_history_list(storage, last);
    to_result(&output)
}

fn dispatch_history_check(storage: &FsStorage) -> Result<ToolCallResult, JsonRpcError> {
    let output = execute_history_check(storage);
    to_result(&output)
}

fn dispatch_history_invalidate(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let from = get_u64_opt(args, "from").unwrap_or(1);
    let output = execute_history_invalidate(storage, from);
    to_result(&output)
}

fn dispatch_history_begin_batch(
    args: &BTreeMap<String, Value>,
    storage: &FsStorage,
) -> Result<ToolCallResult, JsonRpcError> {
    let label = get_str(args, "label")?;
    let output = execute_history_begin_batch(storage, label);
    to_result(&output)
}

fn dispatch_history_end_batch(storage: &FsStorage) -> Result<ToolCallResult, JsonRpcError> {
    let output = execute_history_end_batch(storage);
    to_result(&output)
}

fn dispatch_history_clear(storage: &FsStorage) -> Result<ToolCallResult, JsonRpcError> {
    let output = execute_history_clear(storage);
    to_result(&output)
}
