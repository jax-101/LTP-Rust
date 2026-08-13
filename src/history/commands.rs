use serde::Serialize;

use crate::output::{CommandOutput, GraphHealth, OutputError, OutputWarning};
use crate::storage::{LockOutcome, Storage};
use crate::workspace::FsStorage;

use super::manager::HistoryManager;

// --- Output data types ---

/// Data returned by `undo`.
#[derive(Debug, Serialize)]
pub struct UndoData {
    pub seq: u64,
    pub action_undone: String,
    pub command_undone: String,
    pub files_restored: usize,
    pub dry_run: bool,
}

/// Data returned by `redo`.
#[derive(Debug, Serialize)]
pub struct RedoData {
    pub seq: u64,
    pub action_redone: String,
    pub command_redone: String,
    pub files_restored: usize,
    pub dry_run: bool,
}

/// Single entry in history list output.
#[derive(Debug, Serialize)]
pub struct HistoryEntryOutput {
    pub seq: u64,
    pub timestamp: String,
    pub action: String,
    pub command: String,
    pub batch: Option<String>,
}

/// Data returned by `history` (list).
#[derive(Debug, Serialize)]
pub struct HistoryListData {
    pub entries: Vec<HistoryEntryOutput>,
    pub total_count: usize,
}

/// Single check result in output.
#[derive(Debug, Serialize)]
pub struct CheckEntryOutput {
    pub seq: u64,
    pub action: String,
    pub valid: bool,
    pub diverged_file: Option<String>,
}

/// Data returned by `history check`.
#[derive(Debug, Serialize)]
pub struct HistoryCheckData {
    pub entries: Vec<CheckEntryOutput>,
    pub all_valid: bool,
}

/// Data returned by `history invalidate`.
#[derive(Debug, Serialize)]
pub struct HistoryInvalidateData {
    pub entries_removed: usize,
    pub from_seq: u64,
}

/// Data returned by `history clear`.
#[derive(Debug, Serialize)]
pub struct HistoryClearData {
    pub cleared: bool,
}

/// Data returned by `history begin-batch`.
#[derive(Debug, Serialize)]
pub struct BatchBeginData {
    pub label: String,
}

/// Data returned by `history end-batch`.
#[derive(Debug, Serialize)]
pub struct BatchEndData {
    pub label: String,
}

// --- Helpers ---

fn stale_lock_warning(outcome: &LockOutcome) -> Option<OutputWarning> {
    match outcome {
        LockOutcome::StaleLockRemoved { pid } => Some(OutputWarning::new(
            "STALE_LOCK_REMOVED",
            format!("Stale lock from PID {} was removed", pid),
        )),
        LockOutcome::Acquired => None,
    }
}

fn make_history_manager(storage: &FsStorage) -> Option<HistoryManager> {
    let config = storage.load_config().ok()?;
    Some(HistoryManager::new(
        storage.root().to_path_buf(),
        config.history,
    ))
}

// --- Command implementations ---

/// Execute `ltp undo [--dry-run]`.
pub fn execute_undo(storage: &FsStorage, dry_run: bool) -> CommandOutput<UndoData> {
    let ws_name = storage.workspace_name().unwrap_or_default();
    let action = "undo";

    let manager = match make_history_manager(storage) {
        Some(m) => m,
        None => {
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: UndoData {
                    seq: 0,
                    action_undone: String::new(),
                    command_undone: String::new(),
                    files_restored: 0,
                    dry_run,
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new(
                    "WORKSPACE_NOT_INITIALIZED",
                    "No LTP workspace found",
                )],
                warnings: vec![],
            };
        }
    };

    if !manager.is_enabled() {
        return CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: UndoData {
                seq: 0,
                action_undone: String::new(),
                command_undone: String::new(),
                files_restored: 0,
                dry_run,
            },
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![OutputError::new(
                "HISTORY_DISABLED",
                "History is disabled in workspace config",
            )],
            warnings: vec![],
        };
    }

    // Acquire lock for non-dry-run
    let mut warnings = vec![];
    if !dry_run {
        let lock_outcome = match storage.acquire_lock("undo") {
            Ok(o) => o,
            Err(e) => {
                return CommandOutput {
                    success: false,
                    action: action.to_string(),
                    workspace: ws_name,
                    data: UndoData {
                        seq: 0,
                        action_undone: String::new(),
                        command_undone: String::new(),
                        files_restored: 0,
                        dry_run,
                    },
                    graph_health: GraphHealth {
                        valid_dag: true,
                        orphan_nodes_count: 0,
                    },
                    errors: vec![OutputError::new("LOCK_ERROR", e.to_string())],
                    warnings: vec![],
                };
            }
        };
        if let Some(w) = stale_lock_warning(&lock_outcome) {
            warnings.push(w);
        }
    }

    let result = manager.execute_undo(dry_run);

    if !dry_run {
        let _ = storage.release_lock();
    }

    match result {
        Ok(undo_result) => {
            let mut output = CommandOutput::ok(
                action,
                &ws_name,
                UndoData {
                    seq: undo_result.entry.seq,
                    action_undone: undo_result.entry.action,
                    command_undone: undo_result.entry.command,
                    files_restored: undo_result.files_restored,
                    dry_run,
                },
            );
            output.warnings = warnings;
            output
        }
        Err(e) => {
            let (code, detail) = match &e {
                crate::errors::LtpError::UndoStateDiverged { file_path } => (
                    "UNDO_STATE_DIVERGED",
                    format!(
                        "File '{}' has been modified externally. Run `ltp history check`.",
                        file_path
                    ),
                ),
                crate::errors::LtpError::Io(io_err)
                    if io_err.kind() == std::io::ErrorKind::NotFound =>
                {
                    ("UNDO_STACK_EMPTY", "Nothing to undo".to_string())
                }
                other => ("UNDO_ERROR", other.to_string()),
            };
            CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: UndoData {
                    seq: 0,
                    action_undone: String::new(),
                    command_undone: String::new(),
                    files_restored: 0,
                    dry_run,
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new(code, detail)],
                warnings,
            }
        }
    }
}

/// Execute `ltp redo [--dry-run]`.
pub fn execute_redo(storage: &FsStorage, dry_run: bool) -> CommandOutput<RedoData> {
    let ws_name = storage.workspace_name().unwrap_or_default();
    let action = "redo";

    let manager = match make_history_manager(storage) {
        Some(m) => m,
        None => {
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: RedoData {
                    seq: 0,
                    action_redone: String::new(),
                    command_redone: String::new(),
                    files_restored: 0,
                    dry_run,
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new(
                    "WORKSPACE_NOT_INITIALIZED",
                    "No LTP workspace found",
                )],
                warnings: vec![],
            };
        }
    };

    if !manager.is_enabled() {
        return CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: RedoData {
                seq: 0,
                action_redone: String::new(),
                command_redone: String::new(),
                files_restored: 0,
                dry_run,
            },
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![OutputError::new(
                "HISTORY_DISABLED",
                "History is disabled in workspace config",
            )],
            warnings: vec![],
        };
    }

    let mut warnings = vec![];
    if !dry_run {
        let lock_outcome = match storage.acquire_lock("redo") {
            Ok(o) => o,
            Err(e) => {
                return CommandOutput {
                    success: false,
                    action: action.to_string(),
                    workspace: ws_name,
                    data: RedoData {
                        seq: 0,
                        action_redone: String::new(),
                        command_redone: String::new(),
                        files_restored: 0,
                        dry_run,
                    },
                    graph_health: GraphHealth {
                        valid_dag: true,
                        orphan_nodes_count: 0,
                    },
                    errors: vec![OutputError::new("LOCK_ERROR", e.to_string())],
                    warnings: vec![],
                };
            }
        };
        if let Some(w) = stale_lock_warning(&lock_outcome) {
            warnings.push(w);
        }
    }

    let result = manager.execute_redo(dry_run);

    if !dry_run {
        let _ = storage.release_lock();
    }

    match result {
        Ok(redo_result) => {
            let mut output = CommandOutput::ok(
                action,
                &ws_name,
                RedoData {
                    seq: redo_result.entry.seq,
                    action_redone: redo_result.entry.action,
                    command_redone: redo_result.entry.command,
                    files_restored: redo_result.files_restored,
                    dry_run,
                },
            );
            output.warnings = warnings;
            output
        }
        Err(e) => {
            let (code, detail) = match &e {
                crate::errors::LtpError::RedoStateDiverged { file_path } => (
                    "REDO_STATE_DIVERGED",
                    format!(
                        "File '{}' has been modified externally since undo.",
                        file_path
                    ),
                ),
                crate::errors::LtpError::Io(io_err)
                    if io_err.kind() == std::io::ErrorKind::NotFound =>
                {
                    ("REDO_STACK_EMPTY", "Nothing to redo".to_string())
                }
                other => ("REDO_ERROR", other.to_string()),
            };
            CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: RedoData {
                    seq: 0,
                    action_redone: String::new(),
                    command_redone: String::new(),
                    files_restored: 0,
                    dry_run,
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new(code, detail)],
                warnings,
            }
        }
    }
}

/// Execute `ltp history [--last N]`.
pub fn execute_history_list(
    storage: &FsStorage,
    last: Option<usize>,
) -> CommandOutput<HistoryListData> {
    let ws_name = storage.workspace_name().unwrap_or_default();
    let action = "history_list";

    let manager = match make_history_manager(storage) {
        Some(m) => m,
        None => {
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: HistoryListData {
                    entries: vec![],
                    total_count: 0,
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new(
                    "WORKSPACE_NOT_INITIALIZED",
                    "No LTP workspace found",
                )],
                warnings: vec![],
            };
        }
    };

    match manager.list_history(last) {
        Ok(entries) => {
            let total_count = entries.len();
            let output_entries: Vec<HistoryEntryOutput> = entries
                .into_iter()
                .map(|e| HistoryEntryOutput {
                    seq: e.seq,
                    timestamp: e.timestamp,
                    action: e.action,
                    command: e.command,
                    batch: e.batch,
                })
                .collect();
            CommandOutput::ok(
                action,
                &ws_name,
                HistoryListData {
                    entries: output_entries,
                    total_count,
                },
            )
        }
        Err(e) => CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: HistoryListData {
                entries: vec![],
                total_count: 0,
            },
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![OutputError::new("HISTORY_ERROR", e.to_string())],
            warnings: vec![],
        },
    }
}

/// Execute `ltp history check`.
pub fn execute_history_check(storage: &FsStorage) -> CommandOutput<HistoryCheckData> {
    let ws_name = storage.workspace_name().unwrap_or_default();
    let action = "history_check";

    let manager = match make_history_manager(storage) {
        Some(m) => m,
        None => {
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: HistoryCheckData {
                    entries: vec![],
                    all_valid: false,
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new(
                    "WORKSPACE_NOT_INITIALIZED",
                    "No LTP workspace found",
                )],
                warnings: vec![],
            };
        }
    };

    match manager.check_integrity() {
        Ok(results) => {
            let all_valid = results.iter().all(|r| r.valid);
            let entries: Vec<CheckEntryOutput> = results
                .into_iter()
                .map(|r| CheckEntryOutput {
                    seq: r.seq,
                    action: r.action,
                    valid: r.valid,
                    diverged_file: r.diverged_file,
                })
                .collect();
            CommandOutput::ok(action, &ws_name, HistoryCheckData { entries, all_valid })
        }
        Err(e) => CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: HistoryCheckData {
                entries: vec![],
                all_valid: false,
            },
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![OutputError::new("HISTORY_ERROR", e.to_string())],
            warnings: vec![],
        },
    }
}

/// Execute `ltp history invalidate --from <seq>`.
pub fn execute_history_invalidate(
    storage: &FsStorage,
    from_seq: u64,
) -> CommandOutput<HistoryInvalidateData> {
    let ws_name = storage.workspace_name().unwrap_or_default();
    let action = "history_invalidate";

    let manager = match make_history_manager(storage) {
        Some(m) => m,
        None => {
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: HistoryInvalidateData {
                    entries_removed: 0,
                    from_seq,
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new(
                    "WORKSPACE_NOT_INITIALIZED",
                    "No LTP workspace found",
                )],
                warnings: vec![],
            };
        }
    };

    match manager.invalidate_from(from_seq) {
        Ok(removed) => CommandOutput::ok(
            action,
            &ws_name,
            HistoryInvalidateData {
                entries_removed: removed,
                from_seq,
            },
        ),
        Err(e) => CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: HistoryInvalidateData {
                entries_removed: 0,
                from_seq,
            },
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![OutputError::new("HISTORY_ERROR", e.to_string())],
            warnings: vec![],
        },
    }
}

/// Execute `ltp history clear`.
pub fn execute_history_clear(storage: &FsStorage) -> CommandOutput<HistoryClearData> {
    let ws_name = storage.workspace_name().unwrap_or_default();
    let action = "history_clear";

    let manager = match make_history_manager(storage) {
        Some(m) => m,
        None => {
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: HistoryClearData { cleared: false },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new(
                    "WORKSPACE_NOT_INITIALIZED",
                    "No LTP workspace found",
                )],
                warnings: vec![],
            };
        }
    };

    match manager.clear() {
        Ok(()) => CommandOutput::ok(action, &ws_name, HistoryClearData { cleared: true }),
        Err(e) => CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: HistoryClearData { cleared: false },
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![OutputError::new("HISTORY_ERROR", e.to_string())],
            warnings: vec![],
        },
    }
}

/// Execute `ltp history begin-batch --label <label>`.
pub fn execute_history_begin_batch(
    storage: &FsStorage,
    label: &str,
) -> CommandOutput<BatchBeginData> {
    let ws_name = storage.workspace_name().unwrap_or_default();
    let action = "history_begin_batch";

    let manager = match make_history_manager(storage) {
        Some(m) => m,
        None => {
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: BatchBeginData {
                    label: label.to_string(),
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new(
                    "WORKSPACE_NOT_INITIALIZED",
                    "No LTP workspace found",
                )],
                warnings: vec![],
            };
        }
    };

    if manager.is_batch_active() {
        return CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: BatchBeginData {
                label: label.to_string(),
            },
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![OutputError::new(
                "BATCH_ALREADY_IN_PROGRESS",
                "A batch is already in progress. End it first with `ltp history end-batch`.",
            )],
            warnings: vec![],
        };
    }

    match manager.begin_batch(label) {
        Ok(()) => CommandOutput::ok(
            action,
            &ws_name,
            BatchBeginData {
                label: label.to_string(),
            },
        ),
        Err(e) => CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: BatchBeginData {
                label: label.to_string(),
            },
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![OutputError::new("BATCH_ERROR", e.to_string())],
            warnings: vec![],
        },
    }
}

/// Execute `ltp history end-batch`.
pub fn execute_history_end_batch(storage: &FsStorage) -> CommandOutput<BatchEndData> {
    let ws_name = storage.workspace_name().unwrap_or_default();
    let action = "history_end_batch";

    let manager = match make_history_manager(storage) {
        Some(m) => m,
        None => {
            return CommandOutput {
                success: false,
                action: action.to_string(),
                workspace: ws_name,
                data: BatchEndData {
                    label: String::new(),
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![OutputError::new(
                    "WORKSPACE_NOT_INITIALIZED",
                    "No LTP workspace found",
                )],
                warnings: vec![],
            };
        }
    };

    if !manager.is_batch_active() {
        return CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: BatchEndData {
                label: String::new(),
            },
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![OutputError::new(
                "NO_BATCH_IN_PROGRESS",
                "No batch is currently in progress. Start one with `ltp history begin-batch`.",
            )],
            warnings: vec![],
        };
    }

    match manager.end_batch() {
        Ok(()) => CommandOutput::ok(
            action,
            &ws_name,
            BatchEndData {
                label: "completed".to_string(),
            },
        ),
        Err(e) => CommandOutput {
            success: false,
            action: action.to_string(),
            workspace: ws_name,
            data: BatchEndData {
                label: String::new(),
            },
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![OutputError::new("BATCH_ERROR", e.to_string())],
            warnings: vec![],
        },
    }
}
