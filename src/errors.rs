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

    #[error("circular dependency detected in tree {tree_id}: {}", cycle_path.join(" -> "))]
    CircularDependencyDetected {
        tree_id: String,
        cycle_path: Vec<String>,
    },

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

    #[error("knowledge item not found: {0}")]
    KnowledgeNotFound(String),

    #[error("knowledge source required: at least uri or excerpt must be provided")]
    KnowledgeSourceRequired,

    #[error("label required: label cannot be empty")]
    LabelRequired,

    #[error("EC validation: {0}")]
    EcValidation(String),

    /// Something in the scope of a prefix could not be read while computing
    /// the next ID (PLAN_v052 D-3/D-5). Skipping it could reissue an ID.
    #[error("cannot read {} to compute the next ID: {source}", path.display())]
    CounterScan {
        /// The file or directory that could not be read.
        path: std::path::PathBuf,
        /// The underlying I/O error.
        source: std::io::Error,
    },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, LtpError>;
