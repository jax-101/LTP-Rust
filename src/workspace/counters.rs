use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::errors::Result;
use crate::output::OutputWarning;

/// All entity types tracked by the counter system.
const ENTITY_TYPES: &[&str] = &[
    "UDE", "RC", "INJ", "NC", "GOAL", "OBJ", "WANT", "OBS", "IO", "INT", "DE", "REQ", "PRE", "CSF",
    "TREE", "LINK", "ASM", "NBR", "MACRO", "KN", "MASM", "FB",
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

    /// Rebuild counters by scanning `nodes/`, `trees/` and `knowledge/`.
    ///
    /// Besides file names, tree files are parsed so that IDs embedded in the
    /// tree JSON (`LINK`, `ASM`, `NBR`, `MACRO`, `MASM`, `FB`) are recovered.
    pub fn rebuild(root: &Path) -> (Self, Vec<OutputWarning>) {
        let mut counters = Self::new_zeroed();
        let warnings = vec![OutputWarning::new(
            "COUNTERS_REBUILT",
            "Counter file was missing or corrupt; rebuilt from filesystem scan",
        )];

        Self::scan_directory(&root.join("nodes"), &mut counters);
        Self::scan_directory(&root.join("trees"), &mut counters);
        Self::scan_directory(&root.join("knowledge"), &mut counters);
        Self::scan_tree_contents(&root.join("trees"), &mut counters);

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
                counters.observe(stem);
            }
        }
    }

    /// Parse every tree file and record each string value stored under an
    /// `"id"` key. Unreadable or unparsable files are skipped.
    fn scan_tree_contents(dir: &Path, counters: &mut Counters) {
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let Ok(content) = fs::read_to_string(&path) else {
                continue;
            };
            let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) else {
                continue;
            };
            counters.observe_ids_in(&value);
        }
    }

    fn observe_ids_in(&mut self, value: &serde_json::Value) {
        match value {
            serde_json::Value::Object(map) => {
                for (key, child) in map {
                    if let (true, serde_json::Value::String(id)) = (key == "id", child) {
                        self.observe(id);
                    } else {
                        self.observe_ids_in(child);
                    }
                }
            }
            serde_json::Value::Array(items) => {
                for item in items {
                    self.observe_ids_in(item);
                }
            }
            _ => {}
        }
    }

    /// Raise the counter for `id` if it has the shape `PREFIX-NUMBER` with an
    /// uppercase ASCII prefix. Anything else (e.g. `tree-crt-2024`) is ignored.
    fn observe(&mut self, id: &str) {
        if let Some((prefix, num)) = parse_sequential_id(id) {
            let current = self.values.entry(prefix.to_string()).or_insert(0);
            if num > *current {
                *current = num;
            }
        }
    }
}

/// Split a sequential ID (`UDE-001`) into prefix and number.
fn parse_sequential_id(id: &str) -> Option<(&str, u64)> {
    let (prefix, num_str) = id.split_once('-')?;
    if prefix.is_empty() || !prefix.bytes().all(|b| b.is_ascii_uppercase()) {
        return None;
    }
    if num_str.is_empty() || !num_str.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    num_str.parse::<u64>().ok().map(|n| (prefix, n))
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
    fn parse_sequential_id_accepts_only_canonical_shape() {
        assert_eq!(parse_sequential_id("UDE-001"), Some(("UDE", 1)));
        assert_eq!(parse_sequential_id("LINK-1000"), Some(("LINK", 1000)));
        assert_eq!(parse_sequential_id("tree-crt-2024"), None);
        assert_eq!(parse_sequential_id("ude-001"), None);
        assert_eq!(parse_sequential_id("UDE-"), None);
        assert_eq!(parse_sequential_id("UDE-01a"), None);
        assert_eq!(parse_sequential_id("UDE-+1"), None);
        assert_eq!(parse_sequential_id("-001"), None);
        assert_eq!(parse_sequential_id("notes"), None);
    }

    #[test]
    fn tree_scan_counts_only_id_keys() {
        let mut counters = Counters::new_zeroed();
        let tree = serde_json::json!({
            "id": "tree-crt-x",
            "nodes": [{"ref": "UDE-900", "role": null}],
            "edges": [{"id": "LINK-004", "from": ["RC-777"], "to": "UDE-900",
                       "assumptions": [{"id": "ASM-002"}]}],
            "macro_edges": [{"id": "MACRO-003", "interior_links": ["LINK-500"],
                             "assumptions": [{"id": "MASM-006", "projection_refs": ["ASM-900"]}]}],
            "feedback_edges": [{"id": "FB-002"}],
            "nbr_branches": [{"id": "NBR-005", "source_node": "INJ-800",
                              "edges": [{"id": "LINK-050", "assumptions": [{"id": "ASM-010"}]}]}]
        });
        counters.observe_ids_in(&tree);
        let get = |k: &str| counters.values.get(k).copied();
        assert_eq!(get("LINK"), Some(50));
        assert_eq!(get("ASM"), Some(10));
        assert_eq!(get("MACRO"), Some(3));
        assert_eq!(get("MASM"), Some(6));
        assert_eq!(get("FB"), Some(2));
        assert_eq!(get("NBR"), Some(5));
        // References are never counted as issued IDs.
        assert_eq!(get("UDE"), Some(0));
        assert_eq!(get("RC"), Some(0));
        assert_eq!(get("INJ"), Some(0));
        assert!(!counters.values.contains_key("TREE-CRT"));
    }

    #[test]
    fn new_zeroed_has_all_types() {
        let counters = Counters::new_zeroed();
        for &t in ENTITY_TYPES {
            assert_eq!(counters.values.get(t), Some(&0));
        }
    }
}
