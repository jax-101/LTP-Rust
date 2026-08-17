use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::errors::Result;
use crate::output::OutputWarning;

/// All entity types tracked by the counter system.
const ENTITY_TYPES: &[&str] = &[
    "UDE", "RC", "INJ", "NC", "GOAL", "OBJ", "WANT", "OBS", "IO", "INT", "DE", "REQ", "PRE",
    "TREE", "LINK", "ASM", "NBR", "MACRO", "KN",
];

/// Sequential counter state for all entity types in the workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Counters {
    #[serde(flatten)]
    pub values: BTreeMap<String, u64>,
}

impl Counters {
    /// Create a zeroed counter set with all known entity types.
    pub fn new_zeroed() -> Self {
        let values = ENTITY_TYPES
            .iter()
            .map(|&t| (t.to_string(), 0u64))
            .collect();
        Self { values }
    }

    /// Load counters from disk. Falls back to `rebuild` if file is missing or corrupt.
    pub fn load(counters_path: &Path, root: &Path) -> (Self, Vec<OutputWarning>) {
        match fs::read_to_string(counters_path) {
            Ok(content) => match serde_json::from_str::<Counters>(&content) {
                Ok(c) => (c, vec![]),
                Err(_) => Self::rebuild(root),
            },
            Err(_) => Self::rebuild(root),
        }
    }

    /// Rebuild counters by scanning `nodes/` and `trees/` directories.
    pub fn rebuild(root: &Path) -> (Self, Vec<OutputWarning>) {
        let mut counters = Self::new_zeroed();
        let warnings = vec![OutputWarning::new(
            "COUNTERS_REBUILT",
            "Counter file was missing or corrupt; rebuilt from filesystem scan",
        )];

        Self::scan_directory(&root.join("nodes"), &mut counters);
        Self::scan_directory(&root.join("trees"), &mut counters);
        Self::scan_directory(&root.join("knowledge"), &mut counters);

        (counters, warnings)
    }

    /// Increment the counter for `entity_type` and return the formatted ID.
    pub fn next(&mut self, entity_type: &str) -> String {
        let upper = entity_type.to_uppercase();
        let counter = self.values.entry(upper.clone()).or_insert(0);
        *counter += 1;
        format!("{}-{:03}", upper, counter)
    }

    /// Save counters to disk.
    pub fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(&self)?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Path to the counters file within the .ltp directory.
    pub fn file_path(root: &Path) -> PathBuf {
        root.join(".ltp").join("counters.json")
    }

    fn scan_directory(dir: &Path, counters: &mut Counters) {
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if let Some(stem) = name_str.strip_suffix(".json") {
                if let Some((prefix, num_str)) = stem.rsplit_once('-') {
                    if let Ok(num) = num_str.parse::<u64>() {
                        let upper = prefix.to_uppercase();
                        let current = counters.values.entry(upper).or_insert(0);
                        if num > *current {
                            *current = num;
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_id_increments_correctly() {
        let mut counters = Counters::new_zeroed();
        assert_eq!(counters.next("UDE"), "UDE-001");
        assert_eq!(counters.next("UDE"), "UDE-002");
        assert_eq!(counters.next("RC"), "RC-001");
    }

    #[test]
    fn new_zeroed_has_all_types() {
        let counters = Counters::new_zeroed();
        for &t in ENTITY_TYPES {
            assert_eq!(counters.values.get(t), Some(&0));
        }
    }
}
