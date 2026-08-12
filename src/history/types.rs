use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSnapshot {
    pub before: Option<String>,
    pub after_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoEntry {
    pub seq: u64,
    pub action: String,
    pub command: String,
    pub timestamp: String,
    pub batch: Option<String>,
    pub affected_files: BTreeMap<String, FileSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedoFileSnapshot {
    pub before_hash: Option<String>,
    pub after: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedoEntry {
    pub seq: u64,
    pub action: String,
    pub command: String,
    pub timestamp: String,
    pub affected_files: BTreeMap<String, RedoFileSnapshot>,
}
