use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::errors::{LtpError, Result};
use crate::workspace::config::HistoryConfig;

use super::types::{FileSnapshot, RedoEntry, RedoFileSnapshot, UndoEntry};

/// Manages the undo/redo stacks for workspace history.
///
/// Operates directly on the filesystem using paths relative to workspace root.
/// All writes use atomic tmp-then-rename to prevent corruption.
pub struct HistoryManager {
    root: PathBuf,
    undo_dir: PathBuf,
    redo_dir: PathBuf,
    tmp_dir: PathBuf,
    config: HistoryConfig,
}

/// Context captured before a mutation, holding file states for undo.
pub struct CaptureContext {
    pub affected_paths: Vec<PathBuf>,
    pub before_contents: BTreeMap<String, Option<String>>,
}

/// Result of an undo operation.
pub struct UndoResult {
    pub entry: UndoEntry,
    pub files_restored: usize,
}

/// Result of a redo operation.
pub struct RedoResult {
    pub entry: RedoEntry,
    pub files_restored: usize,
}

/// Result of a single entry integrity check.
#[derive(Debug)]
pub struct CheckResult {
    pub seq: u64,
    pub action: String,
    pub valid: bool,
    pub diverged_file: Option<String>,
}

/// Summary of a history entry for listing.
#[derive(Debug)]
pub struct HistoryEntry {
    pub seq: u64,
    pub timestamp: String,
    pub action: String,
    pub command: String,
    pub batch: Option<String>,
}

impl HistoryManager {
    /// Create a new HistoryManager for the given workspace root and config.
    pub fn new(root: PathBuf, config: HistoryConfig) -> Self {
        let ltp_dir = root.join(".ltp");
        Self {
            undo_dir: ltp_dir.join("undo"),
            redo_dir: ltp_dir.join("redo"),
            tmp_dir: ltp_dir.join("tmp"),
            root,
            config,
        }
    }

    /// Whether history capture is enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Compute SHA-256 hash of content, returning `sha256:<hex>` string.
    pub fn sha256_hash(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let result = hasher.finalize();
        format!("sha256:{:x}", result)
    }

    /// Convert an absolute path to a workspace-relative string.
    fn relative_path(&self, abs_path: &Path) -> String {
        abs_path
            .strip_prefix(&self.root)
            .unwrap_or(abs_path)
            .to_string_lossy()
            .to_string()
    }

    /// Get the next sequence number by scanning the undo directory.
    fn next_seq(&self) -> Result<u64> {
        let entries = self.list_undo_seqs()?;
        Ok(entries.last().copied().unwrap_or(0) + 1)
    }

    /// List all sequence numbers in the undo stack, sorted ascending.
    fn list_undo_seqs(&self) -> Result<Vec<u64>> {
        self.list_seqs_in_dir(&self.undo_dir)
    }

    /// List all sequence numbers in the redo stack, sorted ascending.
    fn list_redo_seqs(&self) -> Result<Vec<u64>> {
        self.list_seqs_in_dir(&self.redo_dir)
    }

    fn list_seqs_in_dir(&self, dir: &Path) -> Result<Vec<u64>> {
        if !dir.exists() {
            return Ok(vec![]);
        }
        let mut seqs = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if let Some(num_str) = name_str.strip_suffix(".json") {
                if let Ok(seq) = num_str.parse::<u64>() {
                    seqs.push(seq);
                }
            }
        }
        seqs.sort();
        Ok(seqs)
    }

    /// Path to a specific undo entry file.
    fn undo_entry_path(&self, seq: u64) -> PathBuf {
        self.undo_dir.join(format!("{:03}.json", seq))
    }

    /// Path to a specific redo entry file.
    fn redo_entry_path(&self, seq: u64) -> PathBuf {
        self.redo_dir.join(format!("{:03}.json", seq))
    }

    /// Load an undo entry by sequence number.
    fn load_undo_entry(&self, seq: u64) -> Result<UndoEntry> {
        let path = self.undo_entry_path(seq);
        let content = fs::read_to_string(&path)?;
        let entry: UndoEntry = serde_json::from_str(&content)?;
        Ok(entry)
    }

    /// Load a redo entry by sequence number.
    fn load_redo_entry(&self, seq: u64) -> Result<RedoEntry> {
        let path = self.redo_entry_path(seq);
        let content = fs::read_to_string(&path)?;
        let entry: RedoEntry = serde_json::from_str(&content)?;
        Ok(entry)
    }

    /// Write atomically via tmp directory.
    fn atomic_write(&self, target: &Path, content: &str) -> Result<()> {
        fs::create_dir_all(&self.tmp_dir)?;
        let file_name = target.file_name().unwrap_or_default().to_string_lossy();
        let tmp_path = self.tmp_dir.join(format!("{}.tmp", file_name));
        fs::write(&tmp_path, content)?;
        fs::rename(&tmp_path, target)?;
        Ok(())
    }

    /// Total size of all files in a directory (non-recursive).
    fn dir_size(dir: &Path) -> Result<u64> {
        if !dir.exists() {
            return Ok(0);
        }
        let mut total = 0u64;
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let meta = entry.metadata()?;
            total += meta.len();
        }
        Ok(total)
    }

    /// Remove oldest entries from undo stack until total size is below limit.
    fn rotate_if_needed(&self) -> Result<()> {
        let max_bytes = u64::from(self.config.max_size_mb) * 1024 * 1024;
        if max_bytes == 0 {
            return Ok(());
        }

        loop {
            let current_size = Self::dir_size(&self.undo_dir)?;
            if current_size <= max_bytes {
                break;
            }
            let seqs = self.list_undo_seqs()?;
            if seqs.is_empty() {
                break;
            }
            let oldest = seqs[0];
            let path = self.undo_entry_path(oldest);
            fs::remove_file(&path)?;
        }
        Ok(())
    }

    /// Clear the entire redo stack.
    fn clear_redo_stack(&self) -> Result<()> {
        let seqs = self.list_redo_seqs()?;
        for seq in seqs {
            let path = self.redo_entry_path(seq);
            if path.exists() {
                fs::remove_file(&path)?;
            }
        }
        Ok(())
    }

    // --- Batch state ---

    fn batch_file_path(&self) -> PathBuf {
        self.root.join(".ltp").join("batch_active.json")
    }

    /// Check if a batch is currently active.
    pub fn is_batch_active(&self) -> bool {
        self.batch_file_path().exists()
    }

    /// Load the batch state (snapshot of workspace files at begin-batch time).
    fn load_batch_state(&self) -> Result<BatchState> {
        let content = fs::read_to_string(self.batch_file_path())?;
        let state: BatchState = serde_json::from_str(&content)?;
        Ok(state)
    }

    // --- Public API ---

    /// Begin capture: read current state of affected files before mutation.
    pub fn begin_capture(&self, affected_paths: &[PathBuf]) -> Result<CaptureContext> {
        let mut before_contents = BTreeMap::new();

        for path in affected_paths {
            let rel = self.relative_path(path);
            if path.exists() {
                let content = fs::read_to_string(path)?;
                before_contents.insert(rel, Some(content));
            } else {
                before_contents.insert(rel, None);
            }
        }

        Ok(CaptureContext {
            affected_paths: affected_paths.to_vec(),
            before_contents,
        })
    }

    /// Commit capture: compute after-hashes and write undo entry.
    /// Called after mutation has been written to disk.
    pub fn commit_capture(&self, ctx: CaptureContext, action: &str, command: &str) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        if self.is_batch_active() {
            return Ok(());
        }

        let seq = self.next_seq()?;
        let timestamp = chrono::Utc::now().to_rfc3339();

        let mut affected_files = BTreeMap::new();

        // Record files that existed before (and may have changed or been deleted)
        for (rel, before) in &ctx.before_contents {
            let abs_path = self.root.join(rel);
            let after_hash = if abs_path.exists() {
                let content = fs::read_to_string(&abs_path)?;
                Some(Self::sha256_hash(&content))
            } else {
                None
            };

            let before_hash = before.as_ref().map(|c| Self::sha256_hash(c));
            if after_hash != before_hash {
                affected_files.insert(
                    rel.clone(),
                    FileSnapshot {
                        before: before.clone(),
                        after_hash,
                    },
                );
            }
        }

        // Detect newly created files (not in before_contents)
        let nodes_dir = self.root.join("nodes");
        let trees_dir = self.root.join("trees");
        let knowledge_dir = self.root.join("knowledge");
        for dir in [&nodes_dir, &trees_dir, &knowledge_dir] {
            if !dir.exists() {
                continue;
            }
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) != Some("json") {
                        continue;
                    }
                    let rel = self.relative_path(&path);
                    if !ctx.before_contents.contains_key(&rel) {
                        let content = fs::read_to_string(&path)?;
                        let after_hash = Some(Self::sha256_hash(&content));
                        affected_files.insert(
                            rel,
                            FileSnapshot {
                                before: None,
                                after_hash,
                            },
                        );
                    }
                }
            }
        }

        if affected_files.is_empty() {
            return Ok(());
        }

        let entry = UndoEntry {
            seq,
            action: action.to_string(),
            command: command.to_string(),
            timestamp,
            batch: None,
            affected_files,
        };

        fs::create_dir_all(&self.undo_dir)?;
        let json = serde_json::to_string_pretty(&entry)?;
        self.atomic_write(&self.undo_entry_path(seq), &json)?;

        self.clear_redo_stack()?;
        self.rotate_if_needed()?;

        Ok(())
    }

    /// Execute undo: verify checksums and restore previous state.
    pub fn execute_undo(&self, dry_run: bool) -> Result<UndoResult> {
        let seqs = self.list_undo_seqs()?;
        let last_seq = seqs.last().copied().ok_or_else(|| {
            LtpError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "undo stack empty",
            ))
        })?;

        let entry = self.load_undo_entry(last_seq)?;

        // Verify checksums
        for (rel_path, snapshot) in &entry.affected_files {
            let abs_path = self.root.join(rel_path);
            let current_hash = if abs_path.exists() {
                let content = fs::read_to_string(&abs_path)?;
                Some(Self::sha256_hash(&content))
            } else {
                None
            };

            if current_hash != snapshot.after_hash {
                return Err(LtpError::UndoStateDiverged {
                    file_path: rel_path.clone(),
                });
            }
        }

        if dry_run {
            let files_restored = entry.affected_files.len();
            return Ok(UndoResult {
                entry,
                files_restored,
            });
        }

        // Build redo entry before restoring
        // redo.before_hash = hash of state AFTER undo completes (= hash of undo.before)
        // redo.after = current content (what redo will restore to)
        let mut redo_affected = BTreeMap::new();
        for (rel_path, snapshot) in &entry.affected_files {
            let abs_path = self.root.join(rel_path);
            let current_content = if abs_path.exists() {
                Some(fs::read_to_string(&abs_path)?)
            } else {
                None
            };
            let before_hash = snapshot.before.as_ref().map(|c| Self::sha256_hash(c));

            redo_affected.insert(
                rel_path.clone(),
                RedoFileSnapshot {
                    before_hash,
                    after: current_content,
                },
            );
        }

        // Restore files atomically via tmp
        fs::create_dir_all(&self.tmp_dir)?;
        let mut tmp_files: Vec<(PathBuf, Option<String>)> = Vec::new();

        for (rel_path, snapshot) in &entry.affected_files {
            let abs_path = self.root.join(rel_path);
            tmp_files.push((abs_path, snapshot.before.clone()));
        }

        // Write all to tmp first
        let mut staged: Vec<(PathBuf, PathBuf)> = Vec::new();
        let mut to_delete: Vec<PathBuf> = Vec::new();

        for (abs_path, before) in &tmp_files {
            match before {
                Some(content) => {
                    let file_name = abs_path.file_name().unwrap_or_default().to_string_lossy();
                    let tmp_path = self.tmp_dir.join(format!("restore_{}", file_name));
                    fs::write(&tmp_path, content)?;
                    staged.push((tmp_path, abs_path.clone()));
                }
                None => {
                    to_delete.push(abs_path.clone());
                }
            }
        }

        // Rename all staged files to final destinations
        for (tmp_path, final_path) in &staged {
            if let Some(parent) = final_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::rename(tmp_path, final_path)?;
        }

        // Delete files that were created by the original operation
        for path in &to_delete {
            if path.exists() {
                fs::remove_file(path)?;
            }
        }

        // Write redo entry
        let redo_entry = RedoEntry {
            seq: entry.seq,
            action: entry.action.clone(),
            command: entry.command.clone(),
            timestamp: entry.timestamp.clone(),
            affected_files: redo_affected,
        };

        fs::create_dir_all(&self.redo_dir)?;
        let redo_json = serde_json::to_string_pretty(&redo_entry)?;
        self.atomic_write(&self.redo_entry_path(entry.seq), &redo_json)?;

        // Remove undo entry
        let undo_path = self.undo_entry_path(last_seq);
        if undo_path.exists() {
            fs::remove_file(&undo_path)?;
        }

        let files_restored = entry.affected_files.len();
        Ok(UndoResult {
            entry,
            files_restored,
        })
    }

    /// Execute redo: verify checksums and re-apply operation.
    pub fn execute_redo(&self, dry_run: bool) -> Result<RedoResult> {
        let seqs = self.list_redo_seqs()?;
        let last_seq = seqs.last().copied().ok_or_else(|| {
            LtpError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "redo stack empty",
            ))
        })?;

        let entry = self.load_redo_entry(last_seq)?;

        // Verify before_hash for each file
        for (rel_path, snapshot) in &entry.affected_files {
            let abs_path = self.root.join(rel_path);
            let current_hash = if abs_path.exists() {
                let content = fs::read_to_string(&abs_path)?;
                Some(Self::sha256_hash(&content))
            } else {
                None
            };

            if current_hash != snapshot.before_hash {
                return Err(LtpError::RedoStateDiverged {
                    file_path: rel_path.clone(),
                });
            }
        }

        if dry_run {
            let files_restored = entry.affected_files.len();
            return Ok(RedoResult {
                entry,
                files_restored,
            });
        }

        // Rebuild the undo entry from redo data
        let mut undo_affected = BTreeMap::new();
        for (rel_path, snapshot) in &entry.affected_files {
            let abs_path = self.root.join(rel_path);
            let before = if abs_path.exists() {
                Some(fs::read_to_string(&abs_path)?)
            } else {
                None
            };
            undo_affected.insert(
                rel_path.clone(),
                FileSnapshot {
                    before,
                    after_hash: snapshot.after.as_ref().map(|c| Self::sha256_hash(c)),
                },
            );
        }

        // Restore files atomically
        fs::create_dir_all(&self.tmp_dir)?;
        let mut staged: Vec<(PathBuf, PathBuf)> = Vec::new();
        let mut to_delete: Vec<PathBuf> = Vec::new();

        for (rel_path, snapshot) in &entry.affected_files {
            let abs_path = self.root.join(rel_path);
            match &snapshot.after {
                Some(content) => {
                    let file_name = abs_path.file_name().unwrap_or_default().to_string_lossy();
                    let tmp_path = self.tmp_dir.join(format!("redo_{}", file_name));
                    fs::write(&tmp_path, content)?;
                    staged.push((tmp_path, abs_path));
                }
                None => {
                    to_delete.push(abs_path);
                }
            }
        }

        for (tmp_path, final_path) in &staged {
            if let Some(parent) = final_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::rename(tmp_path, final_path)?;
        }

        for path in &to_delete {
            if path.exists() {
                fs::remove_file(path)?;
            }
        }

        // Write undo entry back
        let undo_entry = UndoEntry {
            seq: entry.seq,
            action: entry.action.clone(),
            command: entry.command.clone(),
            timestamp: entry.timestamp.clone(),
            batch: None,
            affected_files: undo_affected,
        };

        fs::create_dir_all(&self.undo_dir)?;
        let undo_json = serde_json::to_string_pretty(&undo_entry)?;
        self.atomic_write(&self.undo_entry_path(entry.seq), &undo_json)?;

        // Remove redo entry
        let redo_path = self.redo_entry_path(last_seq);
        if redo_path.exists() {
            fs::remove_file(&redo_path)?;
        }

        let files_restored = entry.affected_files.len();
        Ok(RedoResult {
            entry,
            files_restored,
        })
    }

    /// List history entries from the undo stack.
    pub fn list_history(&self, last: Option<usize>) -> Result<Vec<HistoryEntry>> {
        let seqs = self.list_undo_seqs()?;
        let iter: Box<dyn Iterator<Item = &u64>> = match last {
            Some(n) => Box::new(seqs.iter().rev().take(n)),
            None => Box::new(seqs.iter().rev()),
        };

        let mut entries = Vec::new();
        for &seq in iter {
            if let Ok(undo) = self.load_undo_entry(seq) {
                entries.push(HistoryEntry {
                    seq: undo.seq,
                    timestamp: undo.timestamp,
                    action: undo.action,
                    command: undo.command,
                    batch: undo.batch,
                });
            }
        }
        entries.reverse();
        Ok(entries)
    }

    /// Check integrity of each undo entry against current disk state.
    pub fn check_integrity(&self) -> Result<Vec<CheckResult>> {
        let seqs = self.list_undo_seqs()?;
        let mut results = Vec::new();

        for seq in seqs {
            let entry = self.load_undo_entry(seq)?;
            let mut valid = true;
            let mut diverged_file = None;

            for (rel_path, snapshot) in &entry.affected_files {
                let abs_path = self.root.join(rel_path);
                let current_hash = if abs_path.exists() {
                    let content = fs::read_to_string(&abs_path)?;
                    Some(Self::sha256_hash(&content))
                } else {
                    None
                };

                if current_hash != snapshot.after_hash {
                    valid = false;
                    diverged_file = Some(rel_path.clone());
                    break;
                }
            }

            results.push(CheckResult {
                seq,
                action: entry.action,
                valid,
                diverged_file,
            });
        }

        Ok(results)
    }

    /// Invalidate (discard) entries from a given sequence number onward.
    pub fn invalidate_from(&self, from_seq: u64) -> Result<usize> {
        let seqs = self.list_undo_seqs()?;
        let mut removed = 0;
        for seq in seqs {
            if seq >= from_seq {
                let path = self.undo_entry_path(seq);
                if path.exists() {
                    fs::remove_file(&path)?;
                    removed += 1;
                }
            }
        }
        Ok(removed)
    }

    /// Clear all history (both undo and redo stacks).
    pub fn clear(&self) -> Result<()> {
        let undo_seqs = self.list_undo_seqs()?;
        for seq in undo_seqs {
            let path = self.undo_entry_path(seq);
            if path.exists() {
                fs::remove_file(&path)?;
            }
        }
        self.clear_redo_stack()?;

        // Also remove batch file if present
        let batch_path = self.batch_file_path();
        if batch_path.exists() {
            fs::remove_file(&batch_path)?;
        }

        Ok(())
    }

    /// Begin a batch: snapshot the workspace state and suppress individual entries.
    pub fn begin_batch(&self, label: &str) -> Result<()> {
        if self.is_batch_active() {
            return Err(LtpError::Io(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "batch already in progress",
            )));
        }

        // Snapshot all mutable workspace files
        let mut files_snapshot = BTreeMap::new();
        self.snapshot_dir(&self.root.join("nodes"), &mut files_snapshot)?;
        self.snapshot_dir(&self.root.join("trees"), &mut files_snapshot)?;
        self.snapshot_dir(&self.root.join("knowledge"), &mut files_snapshot)?;
        let config_path = self.root.join("ltp.config.json");
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let rel = self.relative_path(&config_path);
            files_snapshot.insert(rel, Some(content));
        }

        let state = BatchState {
            label: label.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            files_snapshot,
        };

        let json = serde_json::to_string_pretty(&state)?;
        self.atomic_write(&self.batch_file_path(), &json)?;
        Ok(())
    }

    /// End a batch: create a single undo entry with the begin-batch snapshot.
    pub fn end_batch(&self) -> Result<()> {
        if !self.is_batch_active() {
            return Err(LtpError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no batch in progress",
            )));
        }

        let state = self.load_batch_state()?;

        // Build affected_files by comparing begin-batch snapshot with current state
        let mut affected_files = BTreeMap::new();

        // Check files that existed at begin-batch time
        for (rel_path, before_content) in &state.files_snapshot {
            let abs_path = self.root.join(rel_path);
            let after_hash = if abs_path.exists() {
                let current = fs::read_to_string(&abs_path)?;
                Some(Self::sha256_hash(&current))
            } else {
                None
            };

            // Only include if changed
            let before_hash = before_content.as_ref().map(|c| Self::sha256_hash(c));
            if after_hash != before_hash {
                affected_files.insert(
                    rel_path.clone(),
                    FileSnapshot {
                        before: before_content.clone(),
                        after_hash,
                    },
                );
            }
        }

        // Check for newly created files (not in begin-batch snapshot)
        let mut current_files = BTreeMap::new();
        self.snapshot_dir(&self.root.join("nodes"), &mut current_files)?;
        self.snapshot_dir(&self.root.join("trees"), &mut current_files)?;
        self.snapshot_dir(&self.root.join("knowledge"), &mut current_files)?;

        for rel_path in current_files.keys() {
            if !state.files_snapshot.contains_key(rel_path) {
                let abs_path = self.root.join(rel_path);
                let after_hash = if abs_path.exists() {
                    let content = fs::read_to_string(&abs_path)?;
                    Some(Self::sha256_hash(&content))
                } else {
                    None
                };
                affected_files.insert(
                    rel_path.clone(),
                    FileSnapshot {
                        before: None,
                        after_hash,
                    },
                );
            }
        }

        // Only write entry if something changed
        if !affected_files.is_empty() {
            let seq = self.next_seq()?;
            let entry = UndoEntry {
                seq,
                action: "batch".to_string(),
                command: format!("batch: {}", state.label),
                timestamp: state.timestamp,
                batch: Some(state.label),
                affected_files,
            };

            fs::create_dir_all(&self.undo_dir)?;
            let json = serde_json::to_string_pretty(&entry)?;
            self.atomic_write(&self.undo_entry_path(seq), &json)?;
            self.clear_redo_stack()?;
            self.rotate_if_needed()?;
        }

        // Remove batch file
        fs::remove_file(self.batch_file_path())?;
        Ok(())
    }

    /// Snapshot all JSON files in a directory into the map (relative paths).
    fn snapshot_dir(&self, dir: &Path, map: &mut BTreeMap<String, Option<String>>) -> Result<()> {
        if !dir.exists() {
            return Ok(());
        }
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                let content = fs::read_to_string(&path)?;
                let rel = self.relative_path(&path);
                map.insert(rel, Some(content));
            }
        }
        Ok(())
    }
}

/// Internal batch state persisted to `.ltp/batch_active.json`.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct BatchState {
    label: String,
    timestamp: String,
    files_snapshot: BTreeMap<String, Option<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_produces_consistent_hash() {
        let hash = HistoryManager::sha256_hash("hello world");
        assert!(hash.starts_with("sha256:"));
        let hash2 = HistoryManager::sha256_hash("hello world");
        assert_eq!(hash, hash2);
    }

    #[test]
    fn sha256_different_content_different_hash() {
        let h1 = HistoryManager::sha256_hash("aaa");
        let h2 = HistoryManager::sha256_hash("bbb");
        assert_ne!(h1, h2);
    }
}
