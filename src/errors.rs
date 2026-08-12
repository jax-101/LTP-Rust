use thiserror::Error;

#[derive(Debug, Error)]
pub enum LtpError {
    #[error("node not found: {0}")]
    NodeNotFound(String),

    #[error("tree not found: {0}")]
    TreeNotFound(String),

    #[error("link not found: {0}")]
    LinkNotFound(String),

    #[error("assumption not found: {0}")]
    AssumptionNotFound(String),

    #[error("circular dependency detected in tree {tree_id}")]
    CircularDependencyDetected { tree_id: String },

    #[error("referential integrity violation: node {node_id} referenced in edge but not in pool")]
    ReferentialIntegrityViolation { node_id: String },

    #[error("workspace locked by PID {pid} since {timestamp}")]
    WorkspaceLocked { pid: u32, timestamp: String },

    #[error("stale lock removed (PID {pid} not alive)")]
    StaleLockRemoved { pid: u32 },

    #[error("undo state diverged: file {file_path} has been modified externally")]
    UndoStateDiverged { file_path: String },

    #[error("redo state diverged: file {file_path} has been modified externally")]
    RedoStateDiverged { file_path: String },

    #[error("workspace not initialized (run `ltp init` first)")]
    WorkspaceNotInitialized,

    #[error("workspace already exists at {path}")]
    WorkspaceAlreadyExists { path: String },

    #[error("invalid operator transition: {from} -> {to}")]
    InvalidOperatorTransition { from: String, to: String },

    #[error("node {node_id} is not attached to tree {tree_id}")]
    NodeNotInTree { node_id: String, tree_id: String },

    #[error("duplicate node ID: {0}")]
    DuplicateNodeId(String),

    #[error("EC validation: {0}")]
    EcValidation(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, LtpError>;
