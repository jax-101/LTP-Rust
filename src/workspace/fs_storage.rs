use std::cell::Cell;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use tracing::{debug, warn};

use crate::errors::{LtpError, Result};
use crate::knowledge::KnowledgeItem;
use crate::node::Node;
use crate::storage::{CounterNotice, LockOutcome, MintedId, Storage};
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
    /// State of the current lock session: scopes already reconciled with the
    /// disk (PLAN_v052 D-7) and whether a counter notice was already returned
    /// (PLAN_v060 D-5). The scopes are pure memoization: with the lock held
    /// nothing that respects it adds IDs, and after the first reconciliation
    /// the stored counters are above the disk. Cleared on both `acquire_lock`
    /// and `release_lock`, and only consulted while locked, so a command that
    /// forgets to release can never leak it into the next one (MCP reuses a
    /// single `FsStorage` for the whole server lifetime).
    session: Cell<LockSession>,
    /// Disk scans performed by `next_id`, per scope (tests only).
    #[cfg(test)]
    scans: Cell<ScanCounts>,
}

/// Per-lock state of `next_id` (PLAN_v060 D-5).
#[derive(Debug, Clone, Copy, Default)]
struct LockSession {
    /// Scopes already reconciled with the disk.
    scopes: ReconciledScopes,
    /// A counter notice was already returned: at most one per session, so a
    /// command minting in two scopes never reports the second scope's
    /// counters (saved at 0 by the first rebuild) as a false `stale`.
    notice_emitted: bool,
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
            session: Cell::new(LockSession::default()),
            #[cfg(test)]
            scans: Cell::new(ScanCounts::default()),
        }
    }

    /// Start a fresh lock session and record whether the lock is held.
    fn reset_session(&self, locked: bool) {
        self.session.set(LockSession::default());
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

    /// Read `path`, turning a genuine absence into `not_found()` and any other
    /// failure into `LtpError::Io` (PLAN_v060 D-3).
    fn read_entity(path: &Path, not_found: impl FnOnce() -> LtpError) -> Result<String> {
        fs::read_to_string(path).map_err(|e| absent_or_io(path, e, not_found))
    }

    /// Delete `path` with the same absent-versus-broken rule as [`Self::read_entity`].
    fn remove_entity(path: &Path, not_found: impl FnOnce() -> LtpError) -> Result<()> {
        fs::remove_file(path).map_err(|e| absent_or_io(path, e, not_found))
    }

    /// Sorted IDs of the `*.json` entries of `dir`. An absent directory is an
    /// empty list; one that cannot be read is an error (PLAN_v060 D-3).
    fn list_json_ids(dir: &Path) -> Result<Vec<String>> {
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(e) if is_absent(dir, &e) => return Ok(vec![]),
            Err(e) => return Err(e.into()),
        };
        let mut ids = Vec::new();
        for entry in entries {
            let name = entry?.file_name();
            if let Some(id) = name.to_string_lossy().strip_suffix(".json") {
                ids.push(id.to_string());
            }
        }
        ids.sort();
        Ok(ids)
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
        let content = Self::read_entity(&path, || LtpError::NodeNotFound(id.to_string()))?;
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
        Self::remove_entity(&path, || LtpError::NodeNotFound(id.to_string()))
    }

    fn list_node_ids(&self) -> Result<Vec<String>> {
        Self::list_json_ids(&self.nodes_dir())
    }

    fn load_tree(&self, id: &str) -> Result<Tree> {
        let path = self.trees_dir().join(format!("{}.json", id));
        let content = Self::read_entity(&path, || LtpError::TreeNotFound(id.to_string()))?;
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
        Self::remove_entity(&path, || LtpError::TreeNotFound(id.to_string()))
    }

    fn list_tree_ids(&self) -> Result<Vec<String>> {
        Self::list_json_ids(&self.trees_dir())
    }

    fn acquire_lock(&self, command: &str) -> Result<LockOutcome> {
        let lock_path = self.lock_path();
        let mut stale_pid = None;

        debug!(command, "acquiring lock");
        self.reset_session(false);

        // A fresh git clone has no `.ltp/` (it is gitignored): create it so the
        // clone is usable (PLAN_v052 D-4).
        fs::create_dir_all(self.ltp_dir())?;

        let current = match fs::read_to_string(&lock_path) {
            Ok(content) => Some(content),
            Err(e) if is_absent(&lock_path, &e) => None,
            Err(e) => return Err(e.into()),
        };
        if let Some(content) = current {
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

        self.reset_session(true);
        match stale_pid {
            Some(pid) => Ok(LockOutcome::StaleLockRemoved { pid }),
            None => Ok(LockOutcome::Acquired),
        }
    }

    fn release_lock(&self) -> Result<()> {
        self.reset_session(false);
        let lock_path = self.lock_path();
        match fs::remove_file(&lock_path) {
            Err(e) if !is_absent(&lock_path, &e) => Err(e.into()),
            _ => Ok(()),
        }
    }

    fn next_id(&self, entity_type: &str) -> Result<MintedId> {
        // PLAN_v052 D-1/D-3: the stored counters are only a monotonicity memory;
        // reconcile them with the disk, scanning only where the prefix lives.
        let counters_path = Counters::file_path(&self.root);
        let (mut counters, rebuilt) = Counters::load_stored(&counters_path)?.into_counters();
        let prefix = entity_type.to_uppercase();
        let scope = scope_of(&prefix);
        // D-7: one reconciliation per scope while the lock is held.
        let locked = self.locked.get();
        let mut session = self.session.get();
        let memoized = locked && session.scopes.contains(scope);
        let mut raised = Vec::new();
        if !memoized {
            #[cfg(test)]
            self.count_scan(scope);
            let observed = Counters::observe_scope(&self.root, scope)?;
            raised = counters.reconcile(&observed, scope);
        }
        let id = counters.next(&prefix);
        counters.save(&counters_path)?;
        // PLAN_v060 D-5: the first reason wins. Missing/Corrupt come from the
        // file itself; `stale` names the minted prefix if it was raised.
        let notice = match rebuilt {
            Some(reason) => Some(CounterNotice::Rebuilt { reason }),
            None => raised
                .into_iter()
                .min_by_key(|(p, _, _)| *p != prefix)
                .map(|(prefix, from, to)| CounterNotice::Reconciled { prefix, from, to }),
        };
        let notice = if locked && session.notice_emitted {
            None
        } else {
            notice
        };
        // Mark only once the reconciled counters are on disk; otherwise the
        // next mint would start from a stale file without rescanning.
        if locked {
            if !memoized {
                session.scopes = session.scopes.with(scope);
            }
            session.notice_emitted |= notice.is_some();
            self.session.set(session);
        }
        Ok(MintedId { id, notice })
    }

    fn workspace_exists(&self) -> Result<bool> {
        let path = self.config_path();
        match fs::metadata(&path) {
            Ok(_) => Ok(true),
            Err(e) if is_absent(&path, &e) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    fn workspace_name(&self) -> Result<String> {
        let config = self.load_config()?;
        Ok(config.name)
    }

    fn init_workspace(&self, name: &str) -> Result<()> {
        if self.workspace_exists()? {
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

        // Never overwrite a user's .gitignore; `create_new` checks and creates
        // in one step.
        let gitignore = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.root.join(".gitignore"));
        match gitignore {
            Ok(mut file) => io::Write::write_all(&mut file, b".ltp/\n")?,
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e.into()),
        }

        Ok(())
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn load_knowledge(&self, id: &str) -> Result<KnowledgeItem> {
        let path = self.knowledge_dir().join(format!("{}.json", id));
        let content = Self::read_entity(&path, || LtpError::KnowledgeNotFound(id.to_string()))?;
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
        Self::remove_entity(&path, || LtpError::KnowledgeNotFound(id.to_string()))
    }

    fn list_knowledge_ids(&self) -> Result<Vec<String>> {
        Self::list_json_ids(&self.knowledge_dir())
    }

    fn ensure_knowledge_dir(&self) -> Result<bool> {
        match fs::create_dir(self.knowledge_dir()) {
            Ok(()) => {
                debug!("created knowledge/ directory on demand");
                Ok(true)
            }
            // Something already sits there; if it is not a directory, the
            // save that follows reports it as an I/O error.
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Ok(false),
            Err(e) => Err(e.into()),
        }
    }
}

/// Whether `error`, raised while opening `path`, means the entry is really
/// absent. A `NotFound` on an entry that `symlink_metadata` still sees is a
/// dangling symlink: the listing shows it, so it is broken, not absent
/// (PLAN_v060 D-3).
fn is_absent(path: &Path, error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::NotFound
        && matches!(fs::symlink_metadata(path), Err(e) if e.kind() == io::ErrorKind::NotFound)
}

/// `not_found()` when `path` is really absent, `LtpError::Io(error)` otherwise.
fn absent_or_io(path: &Path, error: io::Error, not_found: impl FnOnce() -> LtpError) -> LtpError {
    if is_absent(path, &error) {
        not_found()
    } else {
        LtpError::Io(error)
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
    use crate::storage::RebuildReason;

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

    fn rm_counters(storage: &FsStorage) {
        fs::remove_file(Counters::file_path(&storage.root)).unwrap();
    }

    // D-5 — a missing file warns once per lock session, even across scopes:
    // the second scope finds the file saved at 0 and must not report `stale`.
    #[test]
    fn d5_one_notice_per_session_across_scopes() {
        let (_dir, storage) = workspace();
        write_tree(&storage, "tree-a", &["LINK-003"]);
        rm_counters(&storage);
        storage.acquire_lock("test").unwrap();
        let first = storage.next_id("INT").unwrap();
        assert_eq!(
            first.notice,
            Some(CounterNotice::Rebuilt {
                reason: RebuildReason::Missing
            })
        );
        let second = storage.next_id("LINK").unwrap();
        assert_eq!(second.id, "LINK-004");
        assert_eq!(second.notice, None, "no false stale from the second scope");
        storage.release_lock().unwrap();
    }

    // D-5 — the session resets on acquire: a new repair warns again.
    #[test]
    fn d5_notice_again_in_the_next_session() {
        let (_dir, storage) = workspace();
        rm_counters(&storage);
        storage.acquire_lock("first").unwrap();
        assert!(storage.next_id("UDE").unwrap().notice.is_some());
        storage.release_lock().unwrap();
        storage.acquire_lock("second").unwrap();
        assert_eq!(storage.next_id("UDE").unwrap().notice, None);
        storage.release_lock().unwrap();
        rm_counters(&storage);
        storage.acquire_lock("third").unwrap();
        assert!(storage.next_id("UDE").unwrap().notice.is_some());
        storage.release_lock().unwrap();
    }

    // D-5 — like R14 for the notice: a lock never released does not silence
    // the repair of the next command; acquire resets `notice_emitted` too.
    #[test]
    fn d5_acquire_forgets_a_notice_left_by_a_missing_release() {
        let (_dir, storage) = workspace();
        rm_counters(&storage);
        storage.acquire_lock("first").unwrap();
        assert!(storage.next_id("UDE").unwrap().notice.is_some());
        // No release_lock; the lock file goes away and the counters are lost.
        fs::remove_file(storage.lock_path()).unwrap();
        rm_counters(&storage);

        storage.acquire_lock("second").unwrap();
        assert_eq!(
            storage.next_id("UDE").unwrap().notice,
            Some(CounterNotice::Rebuilt {
                reason: RebuildReason::Missing
            })
        );
        storage.release_lock().unwrap();
    }

    // D-5 — a legitimate `stale` in the second scope still warns, and names
    // the minted prefix when several were raised.
    #[test]
    fn d5_stale_in_second_scope_names_the_minted_prefix() {
        let (_dir, storage) = workspace();
        write_tree(&storage, "tree-a", &["ASM-002", "LINK-007"]);
        storage.acquire_lock("test").unwrap();
        assert_eq!(storage.next_id("UDE").unwrap().notice, None);
        let link = storage.next_id("LINK").unwrap();
        assert_eq!(link.id, "LINK-008");
        assert_eq!(
            link.notice,
            Some(CounterNotice::Reconciled {
                prefix: "LINK".into(),
                from: 0,
                to: 7
            })
        );
        storage.release_lock().unwrap();
    }

    // D-5 — a corrupt file is reported as such.
    #[test]
    fn d5_corrupt_counters_notice() {
        let (_dir, storage) = workspace();
        fs::write(Counters::file_path(&storage.root), "{").unwrap();
        storage.acquire_lock("test").unwrap();
        assert_eq!(
            storage.next_id("UDE").unwrap().notice,
            Some(CounterNotice::Rebuilt {
                reason: RebuildReason::Corrupt
            })
        );
        storage.release_lock().unwrap();
    }

    // R13 — with the lock held, many mints across tree prefixes scan once, and
    // that single scan raised every tree prefix (not only the first one asked).
    #[test]
    fn r13_one_scan_per_scope_per_command() {
        let (_dir, storage) = workspace();
        write_tree(&storage, "tree-a", &["LINK-003", "ASM-009", "FB-004"]);
        storage.acquire_lock("test").unwrap();

        assert_eq!(storage.next_id("LINK").unwrap().id, "LINK-004");
        assert_eq!(storage.next_id("ASM").unwrap().id, "ASM-010");
        assert_eq!(storage.next_id("FB").unwrap().id, "FB-005");
        for i in 0..97 {
            let prefix = ["LINK", "ASM", "FB"][i % 3];
            let _ = storage.next_id(prefix).unwrap();
        }
        assert_eq!(storage.next_id("UDE").unwrap().id, "UDE-001");
        assert_eq!(storage.next_id("UDE").unwrap().id, "UDE-002");

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
        assert_eq!(storage.next_id("LINK").unwrap().id, "LINK-001");
        // No release_lock. Meanwhile a pull brings a tree, and the lock file
        // goes away (e.g. removed by hand).
        write_tree(&storage, "tree-pulled", &["LINK-050"]);
        fs::remove_file(storage.lock_path()).unwrap();

        storage.acquire_lock("second").unwrap();
        assert_eq!(storage.next_id("LINK").unwrap().id, "LINK-051");
        assert_eq!(storage.scans.get().trees, 2);
        storage.release_lock().unwrap();
    }

    // R15 — without the lock every mint reconciles.
    #[test]
    fn r15_without_lock_every_mint_scans() {
        let (_dir, storage) = workspace();
        let _ = storage.next_id("LINK").unwrap();
        write_tree(&storage, "tree-pulled", &["LINK-020"]);
        assert_eq!(storage.next_id("LINK").unwrap().id, "LINK-021");
        let _ = storage.next_id("LINK").unwrap();
        assert_eq!(storage.scans.get().trees, 3);

        storage.acquire_lock("cmd").unwrap();
        let _ = storage.next_id("LINK").unwrap();
        storage.release_lock().unwrap();
        let _ = storage.next_id("LINK").unwrap();
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
        assert_eq!(storage.next_id("LINK").unwrap().id, "LINK-008");
        assert_eq!(storage.scans.get().trees, 2);
        storage.release_lock().unwrap();
    }

    // PLAN_v060 D-3 — absent is `*NotFound`; anything else is `Io`.
    #[test]
    fn d3_absent_entity_is_not_found() {
        let (_dir, storage) = workspace();
        assert!(matches!(
            storage.load_node("UDE-404"),
            Err(LtpError::NodeNotFound(_))
        ));
        assert!(matches!(
            storage.load_tree("tree-x"),
            Err(LtpError::TreeNotFound(_))
        ));
        assert!(matches!(
            storage.load_knowledge("KN-404"),
            Err(LtpError::KnowledgeNotFound(_))
        ));
        assert!(matches!(
            storage.delete_node("UDE-404"),
            Err(LtpError::NodeNotFound(_))
        ));
        assert!(matches!(
            storage.delete_tree("tree-x"),
            Err(LtpError::TreeNotFound(_))
        ));
        assert!(matches!(
            storage.delete_knowledge("KN-404"),
            Err(LtpError::KnowledgeNotFound(_))
        ));
    }

    #[test]
    fn d3_entity_that_is_a_directory_is_io() {
        let (_dir, storage) = workspace();
        fs::create_dir(storage.trees_dir().join("tree-x.json")).unwrap();
        fs::create_dir(storage.nodes_dir().join("UDE-001.json")).unwrap();
        assert!(matches!(storage.load_tree("tree-x"), Err(LtpError::Io(_))));
        assert!(matches!(storage.load_node("UDE-001"), Err(LtpError::Io(_))));
        assert!(matches!(
            storage.delete_tree("tree-x"),
            Err(LtpError::Io(_))
        ));
    }

    #[test]
    fn d3_pool_dir_that_is_a_file_is_io() {
        let (_dir, storage) = workspace();
        fs::remove_dir(storage.nodes_dir()).unwrap();
        fs::write(storage.nodes_dir(), "").unwrap();
        assert!(matches!(storage.load_node("UDE-001"), Err(LtpError::Io(_))));
        assert!(matches!(storage.list_node_ids(), Err(LtpError::Io(_))));
    }

    #[test]
    fn d3_absent_pool_dir_lists_empty() {
        let (_dir, storage) = workspace();
        fs::remove_dir(storage.knowledge_dir()).unwrap();
        assert_eq!(storage.list_knowledge_ids().unwrap(), Vec::<String>::new());
        assert!(storage.ensure_knowledge_dir().unwrap());
        assert!(!storage.ensure_knowledge_dir().unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn d3_broken_symlink_is_io_not_not_found() {
        let (_dir, storage) = workspace();
        let link = storage.trees_dir().join("tree-x.json");
        std::os::unix::fs::symlink(storage.root.join("nowhere.json"), &link).unwrap();
        assert_eq!(storage.list_tree_ids().unwrap(), vec!["tree-x"]);
        assert!(matches!(storage.load_tree("tree-x"), Err(LtpError::Io(_))));
    }

    #[cfg(unix)]
    #[test]
    fn d3_workspace_exists_distinguishes_absent_from_broken() {
        let dir = tempfile::tempdir().unwrap();
        let storage = FsStorage::new(dir.path().to_path_buf());
        assert!(!storage.workspace_exists().unwrap());
        std::os::unix::fs::symlink(dir.path().join("nowhere"), storage.config_path()).unwrap();
        assert!(storage.workspace_exists().is_err());
        assert!(storage.init_workspace("X").is_err(), "never init over it");
    }

    #[test]
    fn d3_lock_absent_is_not_an_error_and_gitignore_is_kept() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".gitignore"), "mine\n").unwrap();
        let storage = FsStorage::new(dir.path().to_path_buf());
        storage.init_workspace("X").unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join(".gitignore")).unwrap(),
            "mine\n"
        );
        storage.release_lock().unwrap();
        assert!(matches!(
            storage.acquire_lock("a"),
            Ok(LockOutcome::Acquired)
        ));
        storage.release_lock().unwrap();
    }
}
