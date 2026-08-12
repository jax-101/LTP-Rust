use std::fs;
use std::path::{Path, PathBuf};

use crate::errors::{LtpError, Result};
use crate::node::Node;
use crate::output::OutputWarning;
use crate::storage::{LockOutcome, Storage};
use crate::tree::Tree;
use crate::workspace::config::WorkspaceConfig;
use crate::workspace::counters::Counters;
use crate::workspace::lock::LockFile;

/// Filesystem-backed implementation of the `Storage` trait.
///
/// All I/O uses atomic writes (tmp → rename) to prevent corruption.
pub struct FsStorage {
    root: PathBuf,
}

impl FsStorage {
    /// Create a new `FsStorage` rooted at the given directory.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Return any warnings generated during counter operations.
    pub fn load_counters(&self) -> (Counters, Vec<OutputWarning>) {
        let path = Counters::file_path(&self.root);
        Counters::load(&path, &self.root)
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
        let tree: Tree = serde_json::from_str(&content)?;
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

        if lock_path.exists() {
            let content = fs::read_to_string(&lock_path)?;
            let existing: LockFile = serde_json::from_str(&content)?;

            if is_pid_alive(existing.pid) {
                return Err(LtpError::WorkspaceLocked {
                    pid: existing.pid,
                    timestamp: existing.timestamp,
                });
            }
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

        match stale_pid {
            Some(pid) => Ok(LockOutcome::StaleLockRemoved { pid }),
            None => Ok(LockOutcome::Acquired),
        }
    }

    fn release_lock(&self) -> Result<()> {
        let lock_path = self.lock_path();
        if lock_path.exists() {
            fs::remove_file(&lock_path)?;
        }
        Ok(())
    }

    fn next_id(&self, entity_type: &str) -> Result<String> {
        let counters_path = Counters::file_path(&self.root);
        let (mut counters, _) = Counters::load(&counters_path, &self.root);
        let id = counters.next(entity_type);
        counters.save(&counters_path)?;
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
}

#[cfg(unix)]
fn is_pid_alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

#[cfg(not(unix))]
fn is_pid_alive(_pid: u32) -> bool {
    true
}
