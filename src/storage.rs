use std::path::Path;

use crate::errors::Result;
use crate::knowledge::KnowledgeItem;
use crate::node::Node;
use crate::tree::Tree;
use crate::workspace::WorkspaceConfig;

/// Abstraction over workspace persistence.
///
/// Implementations handle reading/writing nodes, trees, config,
/// lock management, and ID generation for a workspace root.
pub trait Storage {
    /// Load the workspace configuration file.
    fn load_config(&self) -> Result<WorkspaceConfig>;

    /// Persist the workspace configuration file atomically.
    fn save_config(&self, config: &WorkspaceConfig) -> Result<()>;

    /// Load a node by its ID from the node pool.
    fn load_node(&self, id: &str) -> Result<Node>;

    /// Persist a node atomically to the node pool.
    fn save_node(&self, node: &Node) -> Result<()>;

    /// Delete a node from the pool.
    fn delete_node(&self, id: &str) -> Result<()>;

    /// List all node IDs present in the pool.
    fn list_node_ids(&self) -> Result<Vec<String>>;

    /// Load a tree by its ID.
    fn load_tree(&self, id: &str) -> Result<Tree>;

    /// Persist a tree atomically.
    fn save_tree(&self, tree: &Tree) -> Result<()>;

    /// Delete a tree file.
    fn delete_tree(&self, id: &str) -> Result<()>;

    /// List all tree IDs present in the workspace.
    fn list_tree_ids(&self) -> Result<Vec<String>>;

    /// Acquire an exclusive lock for the given command.
    fn acquire_lock(&self, command: &str) -> Result<LockOutcome>;

    /// Release the workspace lock.
    fn release_lock(&self) -> Result<()>;

    /// Generate the next sequential ID for the given entity type.
    fn next_id(&self, entity_type: &str) -> Result<String>;

    /// Check whether the workspace has been initialized.
    fn workspace_exists(&self) -> bool;

    /// Return the workspace name from config.
    fn workspace_name(&self) -> Result<String>;

    /// Initialize the workspace directory structure and config.
    fn init_workspace(&self, name: &str) -> Result<()>;

    /// Return the workspace root path.
    fn root(&self) -> &Path;

    /// Load a knowledge item by its ID from the knowledge pool.
    fn load_knowledge(&self, id: &str) -> Result<KnowledgeItem>;

    /// Persist a knowledge item atomically to the knowledge pool.
    fn save_knowledge(&self, item: &KnowledgeItem) -> Result<()>;

    /// Delete a knowledge item from the pool.
    fn delete_knowledge(&self, id: &str) -> Result<()>;

    /// List all knowledge item IDs present in the pool.
    fn list_knowledge_ids(&self) -> Result<Vec<String>>;

    /// Ensure the knowledge directory exists, creating it if needed.
    /// Returns true if the directory was newly created.
    fn ensure_knowledge_dir(&self) -> Result<bool>;
}

/// Outcome of a lock acquisition attempt that may involve stale-lock cleanup.
#[derive(Debug, Clone)]
pub enum LockOutcome {
    /// Lock acquired cleanly.
    Acquired,
    /// A stale lock was removed before acquiring; contains the dead PID.
    StaleLockRemoved { pid: u32 },
}
