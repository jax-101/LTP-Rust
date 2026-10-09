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
    ///
    /// Implementations must return the tree normalized via `Tree::normalize_logic`
    /// (ADR-014), without writing back to storage. Every backend must pass the shared
    /// contract in `tests/storage_contract.rs`.
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
    ///
    /// The returned [`MintedId`] carries a counter notice when the stored
    /// counters had to be rebuilt or reconciled; at most one per lock session
    /// (PLAN_v060 D-5).
    fn next_id(&self, entity_type: &str) -> Result<MintedId>;

    /// Check whether the workspace has been initialized.
    ///
    /// `Err` when the config cannot be inspected: "cannot tell" is never
    /// reported as "no workspace" (PLAN_v060 D-3).
    fn workspace_exists(&self) -> Result<bool>;

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

/// A freshly minted ID, plus the counter repair it required (PLAN_v060 D-5).
#[derive(Debug)]
#[must_use = "the counter notice must reach the command output"]
pub struct MintedId {
    /// The new sequential ID (e.g. `UDE-003`).
    pub id: String,
    /// Set only on the first repair of the current lock session.
    pub notice: Option<CounterNotice>,
}

impl MintedId {
    /// Return the ID, keeping the first notice of the command in `notice`.
    pub fn into_id(self, notice: &mut Option<CounterNotice>) -> String {
        if notice.is_none() {
            *notice = self.notice;
        }
        self.id
    }
}

/// Why `.ltp/counters.json` could not be used as stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebuildReason {
    /// The file did not exist (e.g. a fresh git clone).
    Missing,
    /// The file did not parse as a counter map.
    Corrupt,
}

/// A repair of the stored counters done while minting (PLAN_v060 D-5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CounterNotice {
    /// The counters were rebuilt from the disk.
    Rebuilt {
        /// What was wrong with the stored file.
        reason: RebuildReason,
    },
    /// A valid stored counter was below the highest ID on disk and was raised.
    Reconciled {
        /// Prefix of the raised counter.
        prefix: String,
        /// Stored value.
        from: u64,
        /// Highest number observed on disk.
        to: u64,
    },
}
