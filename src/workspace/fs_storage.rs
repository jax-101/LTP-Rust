use std::cell::Cell;
use std::fs;
use std::path::{Path, PathBuf};

use tracing::{debug, warn};

use crate::errors::{LtpError, Result};
use crate::knowledge::KnowledgeItem;
use crate::node::Node;
use crate::storage::{LockOutcome, Storage};
use crate::tree::Tree;
use crate::workspace::config::WorkspaceConfig;
use crate::workspace::counters::{scope_of, Counters, ScanScope};
use crate::workspace::lock::LockFile;

/// Filesystem-backed implementation of the `Storage` trait.
///
/// All I/O uses atomic writes (tmp → rename) to prevent corruption.
pub struct FsStorage {
    root: PathBuf,
    /// Whether this instance currently holds the workspace lock.
    locked: Cell<bool>,
    /// Scopes already reconciled with the disk under the current lock
    /// (PLAN_v052 D-7). Pure memoization: with the lock held nothing that
    /// respects it adds IDs, and after the first reconciliation the stored
    /// counters are above the disk. Cleared on both `acquire_lock` and
    /// `release_lock`, and only consulted while locked, so a command that
    /// forgets to release can never leak it into the next one (MCP reuses a
    /// single `FsStorage` for the whole server lifetime).
    reconciled: Cell<ReconciledScopes>,
    /// Disk scans performed by `next_id`, per scope (tests only).
    #[cfg(test)]
    scans: Cell<ScanCounts>,
}

/// Scopes already reconciled with the disk while the current lock is held.
#[derive(Debug, Clone, Copy, Default)]
struct ReconciledScopes {
    nodes: bool,
    knowledge: bool,
    trees: bool,
}

impl ReconciledScopes {
    /// `All` (unknown prefix) is never memoized.
    fn contains(self, scope: ScanScope) -> bool {
        match scope {
            ScanScope::Nodes => self.nodes,
            ScanScope::Knowledge => self.knowledge,
            ScanScope::Trees => self.trees,
            ScanScope::All => false,
        }
    }

    fn with(mut self, scope: ScanScope) -> Self {
        match scope {
            ScanScope::Nodes => self.nodes = true,
            ScanScope::Knowledge => self.knowledge = true,
            ScanScope::Trees => self.trees = true,
            ScanScope::All => {}
        }
        self
    }
}

/// Number of disk scans per scope (tests only).
#[cfg(test)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ScanCounts {
    nodes: u32,
    knowledge: u32,
    trees: u32,
    all: u32,
}

impl FsStorage {
    /// Create a new `FsStorage` rooted at the given directory.
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            locked: Cell::new(false),
            reconciled: Cell::new(ReconciledScopes::default()),
            #[cfg(test)]
            scans: Cell::new(ScanCounts::default()),
        }
    }

    /// Forget every reconciled scope and record whether the lock is held.
    fn reset_reconciled(&self, locked: bool) {
        self.reconciled.set(ReconciledScopes::default());
        self.locked.set(locked);
    }

    #[cfg(test)]
    fn count_scan(&self, scope: ScanScope) {
        let mut c = self.scans.get();
        match scope {
            ScanScope::Nodes => c.nodes += 1,
            ScanScope::Knowledge => c.knowledge += 1,
            ScanScope::Trees => c.trees += 1,
            ScanScope::All => c.all += 1,
        }
        self.scans.set(c);
    }

    fn ltp_dir(&self) -> PathBuf {
        self.root.join(".ltp")
    }

    fn tmp_dir(&self) -> PathBuf {
        self.ltp_dir().join("tmp")
    }

    fn nodes_dir(&self) -> PathBuf {
        self.root.join("nodes")
    }

    fn trees_dir(&self) -> PathBuf {
        self.root.join("trees")
    }

    fn knowledge_dir(&self) -> PathBuf {
        self.root.join("knowledge")
    }

    fn config_path(&self) -> PathBuf {
        self.root.join("ltp.config.json")
    }

    fn lock_path(&self) -> PathBuf {
        self.ltp_dir().join("lock")
    }

    /// Write JSON content atomically: write to tmp, then rename.
    fn atomic_write(&self, target: &Path, content: &str) -> Result<()> {
        let tmp_dir = self.tmp_dir();
        fs::create_dir_all(&tmp_dir)?;

        let file_name = target.file_name().unwrap_or_default().to_string_lossy();
        let tmp_path = tmp_dir.join(format!("{}.tmp", file_name));

        debug!(target = %target.display(), "atomic write");
        fs::write(&tmp_path, content)?;
        fs::rename(&tmp_path, target)?;
        Ok(())
    }

    /// Serialize a value to canonical JSON (2-space indent, sorted keys via BTreeMap).
    fn to_canonical_json<T: serde::Serialize>(value: &T) -> Result<String> {
        let json = serde_json::to_string_pretty(value)?;
        Ok(json)
    }
}

impl Storage for FsStorage {
    fn load_config(&self) -> Result<WorkspaceConfig> {
        let content = fs::read_to_string(self.config_path())?;
        let config: WorkspaceConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    fn save_config(&self, config: &WorkspaceConfig) -> Result<()> {
        let json = Self::to_canonical_json(config)?;
        self.atomic_write(&self.config_path(), &json)
    }

    fn load_node(&self, id: &str) -> Result<Node> {
        let path = self.nodes_dir().join(format!("{}.json", id));
        if !path.exists() {
            return Err(LtpError::NodeNotFound(id.to_string()));
        }
        let content = fs::read_to_string(&path)?;
        let node: Node = serde_json::from_str(&content)?;
        Ok(node)
    }

    fn save_node(&self, node: &Node) -> Result<()> {
        let path = self.nodes_dir().join(format!("{}.json", node.id));
        let json = Self::to_canonical_json(node)?;
        self.atomic_write(&path, &json)
    }

    fn delete_node(&self, id: &str) -> Result<()> {
        let path = self.nodes_dir().join(format!("{}.json", id));
        if !path.exists() {
            return Err(LtpError::NodeNotFound(id.to_string()));
        }
        fs::remove_file(&path)?;
        Ok(())
    }

    fn list_node_ids(&self) -> Result<Vec<String>> {
        let dir = self.nodes_dir();
        if !dir.exists() {
            return Ok(vec![]);
        }
        let mut ids = Vec::new();
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if let Some(id) = name_str.strip_suffix(".json") {
                ids.push(id.to_string());
            }
        }
        ids.sort();
        Ok(ids)
    }

    fn load_tree(&self, id: &str) -> Result<Tree> {
        let path = self.trees_dir().join(format!("{}.json", id));
        if !path.exists() {
            return Err(LtpError::TreeNotFound(id.to_string()));
        }
        let content = fs::read_to_string(&path)?;
        let mut tree: Tree = serde_json::from_str(&content)?;
        // ADR-014: logic is derived from the tree type. Legacy files are fixed in memory
        // only; the corrected tree reaches disk on the next mutation (never on a read).
        tree.normalize_logic();
        Ok(tree)
    }

    fn save_tree(&self, tree: &Tree) -> Result<()> {
        let path = self.trees_dir().join(format!("{}.json", tree.id));
        let json = Self::to_canonical_json(tree)?;
        self.atomic_write(&path, &json)
    }

    fn delete_tree(&self, id: &str) -> Result<()> {
        let path = self.trees_dir().join(format!("{}.json", id));
        if !path.exists() {
            return Err(LtpError::TreeNotFound(id.to_string()));
        }
        fs::remove_file(&path)?;
        Ok(())
    }

    fn list_tree_ids(&self) -> Result<Vec<String>> {
        let dir = self.trees_dir();
        if !dir.exists() {
            return Ok(vec![]);
        }
        let mut ids = Vec::new();
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if let Some(id) = name_str.strip_suffix(".json") {
                ids.push(id.to_string());
            }
        }
        ids.sort();
        Ok(ids)
    }

    fn acquire_lock(&self, command: &str) -> Result<LockOutcome> {
        let lock_path = self.lock_path();
        let mut stale_pid = None;

        debug!(command, "acquiring lock");
        self.reset_reconciled(false);

        // A fresh git clone has no `.ltp/` (it is gitignored): create it so the
        // clone is usable (PLAN_v052 D-4).
        fs::create_dir_all(self.ltp_dir())?;

        if lock_path.exists() {
            let content = fs::read_to_string(&lock_path)?;
            let existing: LockFile = serde_json::from_str(&content)?;

            if is_pid_alive(existing.pid) {
                return Err(LtpError::WorkspaceLocked {
                    pid: existing.pid,
                    timestamp: existing.timestamp,
                });
            }
            warn!(pid = existing.pid, "removing stale lock");
            stale_pid = Some(existing.pid);
            fs::remove_file(&lock_path)?;
        }

        let lock = LockFile {
            pid: std::process::id(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            command: command.to_string(),
        };

        let json = serde_json::to_string_pretty(&lock)?;
        fs::write(&lock_path, json)?;

        self.reset_reconciled(true);
        match stale_pid {
            Some(pid) => Ok(LockOutcome::StaleLockRemoved { pid }),
            None => Ok(LockOutcome::Acquired),
        }
    }

    fn release_lock(&self) -> Result<()> {
        self.reset_reconciled(false);
        let lock_path = self.lock_path();
        if lock_path.exists() {
            fs::remove_file(&lock_path)?;
        }
        Ok(())
    }

    fn next_id(&self, entity_type: &str) -> Result<String> {
        // PLAN_v052 D-1/D-3: the stored counters are only a monotonicity memory;
        // reconcile them with the disk, scanning only where the prefix lives.
        let counters_path = Counters::file_path(&self.root);
        let mut counters = Counters::load_stored(&counters_path)?.into_counters();
        let prefix = entity_type.to_uppercase();
        let scope = scope_of(&prefix);
        // D-7: one reconciliation per scope while the lock is held.
        let locked = self.locked.get();
        let memoized = locked && self.reconciled.get().contains(scope);
        if !memoized {
            #[cfg(test)]
            self.count_scan(scope);
            let observed = Counters::observe_scope(&self.root, scope)?;
            counters.reconcile(&observed, scope);
        }
        let id = counters.next(&prefix);
        counters.save(&counters_path)?;
        // Mark only once the reconciled counters are on disk; otherwise the
        // next mint would start from a stale file without rescanning.
        if locked && !memoized {
            self.reconciled.set(self.reconciled.get().with(scope));
        }
        Ok(id)
    }

    fn workspace_exists(&self) -> bool {
        self.config_path().exists()
    }

    fn workspace_name(&self) -> Result<String> {
        let config = self.load_config()?;
        Ok(config.name)
    }

    fn init_workspace(&self, name: &str) -> Result<()> {
        if self.workspace_exists() {
            return Err(LtpError::WorkspaceAlreadyExists {
                path: self.root.display().to_string(),
            });
        }

        fs::create_dir_all(self.nodes_dir())?;
        fs::create_dir_all(self.trees_dir())?;
        fs::create_dir_all(self.knowledge_dir())?;
        fs::create_dir_all(self.ltp_dir().join("undo"))?;
        fs::create_dir_all(self.ltp_dir().join("redo"))?;
        fs::create_dir_all(self.tmp_dir())?;

        let config = WorkspaceConfig {
            name: name.to_string(),
            history: Default::default(),
        };
        let config_json = Self::to_canonical_json(&config)?;
        fs::write(self.config_path(), config_json)?;

        let counters = Counters::new_zeroed();
        counters.save(&Counters::file_path(&self.root))?;

        let gitignore_path = self.root.join(".gitignore");
        if !gitignore_path.exists() {
            fs::write(&gitignore_path, ".ltp/\n")?;
        }

        Ok(())
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn load_knowledge(&self, id: &str) -> Result<KnowledgeItem> {
        let path = self.knowledge_dir().join(format!("{}.json", id));
        if !path.exists() {
            return Err(LtpError::KnowledgeNotFound(id.to_string()));
        }
        let content = fs::read_to_string(&path)?;
        let item: KnowledgeItem = serde_json::from_str(&content)?;
        Ok(item)
    }

    fn save_knowledge(&self, item: &KnowledgeItem) -> Result<()> {
        self.ensure_knowledge_dir()?;
        let path = self.knowledge_dir().join(format!("{}.json", item.id));
        let json = Self::to_canonical_json(item)?;
        self.atomic_write(&path, &json)
    }

    fn delete_knowledge(&self, id: &str) -> Result<()> {
        let path = self.knowledge_dir().join(format!("{}.json", id));
        if !path.exists() {
            return Err(LtpError::KnowledgeNotFound(id.to_string()));
        }
        fs::remove_file(&path)?;
        Ok(())
    }

    fn list_knowledge_ids(&self) -> Result<Vec<String>> {
        let dir = self.knowledge_dir();
        if !dir.exists() {
            return Ok(vec![]);
        }
        let mut ids = Vec::new();
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if let Some(id) = name_str.strip_suffix(".json") {
                ids.push(id.to_string());
            }
        }
        ids.sort();
        Ok(ids)
    }

    fn ensure_knowledge_dir(&self) -> Result<bool> {
        let dir = self.knowledge_dir();
        if dir.exists() {
            return Ok(false);
        }
        fs::create_dir_all(&dir)?;
        debug!("created knowledge/ directory on demand");
        Ok(true)
    }
}

#[cfg(unix)]
fn is_pid_alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

#[cfg(not(unix))]
fn is_pid_alive(_pid: u32) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace() -> (tempfile::TempDir, FsStorage) {
        let dir = tempfile::tempdir().unwrap();
        let storage = FsStorage::new(dir.path().to_path_buf());
        storage.init_workspace("D7").unwrap();
        (dir, storage)
    }

    /// Writes a tree file holding the given IDs (only `"id"` keys are counted).
    fn write_tree(storage: &FsStorage, name: &str, ids: &[&str]) {
        let edges: Vec<serde_json::Value> = ids
            .iter()
            .map(|id| serde_json::json!({ "id": id }))
            .collect();
        let tree = serde_json::json!({ "id": name, "edges": edges });
        fs::write(
            storage.trees_dir().join(format!("{name}.json")),
            tree.to_string(),
        )
        .unwrap();
    }

    // R13 — with the lock held, many mints across tree prefixes scan once, and
    // that single scan raised every tree prefix (not only the first one asked).
    #[test]
    fn r13_one_scan_per_scope_per_command() {
        let (_dir, storage) = workspace();
        write_tree(&storage, "tree-a", &["LINK-003", "ASM-009", "FB-004"]);
        storage.acquire_lock("test").unwrap();

        assert_eq!(storage.next_id("LINK").unwrap(), "LINK-004");
        assert_eq!(storage.next_id("ASM").unwrap(), "ASM-010");
        assert_eq!(storage.next_id("FB").unwrap(), "FB-005");
        for i in 0..97 {
            let prefix = ["LINK", "ASM", "FB"][i % 3];
            storage.next_id(prefix).unwrap();
        }
        assert_eq!(storage.next_id("UDE").unwrap(), "UDE-001");
        assert_eq!(storage.next_id("UDE").unwrap(), "UDE-002");

        let scans = storage.scans.get();
        assert_eq!(scans.trees, 1, "{scans:?}");
        assert_eq!(scans.nodes, 1, "{scans:?}");
        assert_eq!(scans.knowledge, 0, "{scans:?}");
        storage.release_lock().unwrap();
    }

    // R14 — a lock that was never released does not carry the memo into the
    // next command: the next acquire rescans and sees what changed meanwhile.
    #[test]
    fn r14_acquire_forgets_a_memo_left_by_a_missing_release() {
        let (_dir, storage) = workspace();
        storage.acquire_lock("first").unwrap();
        assert_eq!(storage.next_id("LINK").unwrap(), "LINK-001");
        // No release_lock. Meanwhile a pull brings a tree, and the lock file
        // goes away (e.g. removed by hand).
        write_tree(&storage, "tree-pulled", &["LINK-050"]);
        fs::remove_file(storage.lock_path()).unwrap();

        storage.acquire_lock("second").unwrap();
        assert_eq!(storage.next_id("LINK").unwrap(), "LINK-051");
        assert_eq!(storage.scans.get().trees, 2);
        storage.release_lock().unwrap();
    }

    // R15 — without the lock every mint reconciles.
    #[test]
    fn r15_without_lock_every_mint_scans() {
        let (_dir, storage) = workspace();
        storage.next_id("LINK").unwrap();
        write_tree(&storage, "tree-pulled", &["LINK-020"]);
        assert_eq!(storage.next_id("LINK").unwrap(), "LINK-021");
        storage.next_id("LINK").unwrap();
        assert_eq!(storage.scans.get().trees, 3);

        storage.acquire_lock("cmd").unwrap();
        storage.next_id("LINK").unwrap();
        storage.release_lock().unwrap();
        storage.next_id("LINK").unwrap();
        assert_eq!(storage.scans.get().trees, 5, "release ends the memo");
    }

    // R16 — a failed scan (or a failed save) never marks the scope.
    #[test]
    fn r16_failed_scan_does_not_mark_the_scope() {
        let (_dir, storage) = workspace();
        let broken = storage.trees_dir().join("tree-x.json");
        fs::create_dir(&broken).unwrap();
        storage.acquire_lock("cmd").unwrap();

        let err = storage.next_id("LINK").unwrap_err();
        assert!(matches!(err, LtpError::CounterScan { .. }), "{err}");
        assert!(err.to_string().contains("tree-x.json"), "{err}");

        fs::remove_dir(&broken).unwrap();
        write_tree(&storage, "tree-x", &["LINK-007"]);
        assert_eq!(storage.next_id("LINK").unwrap(), "LINK-008");
        assert_eq!(storage.scans.get().trees, 2);
        storage.release_lock().unwrap();
    }
}
