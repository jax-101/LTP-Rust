use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::errors::{LtpError, Result};

/// All entity types tracked by the counter system.
const ENTITY_TYPES: &[&str] = &[
    "UDE", "RC", "INJ", "NC", "GOAL", "OBJ", "WANT", "OBS", "IO", "INT", "DE", "REQ", "PRE", "CSF",
    "TREE", "LINK", "ASM", "NBR", "MACRO", "KN", "MASM", "FB",
];

/// Where the IDs of a prefix live on disk, and therefore what a mint has to
/// scan to avoid reissuing one (PLAN_v052 D-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanScope {
    /// Node IDs: the file names in `nodes/`.
    Nodes,
    /// Knowledge IDs: the file names in `knowledge/`.
    Knowledge,
    /// IDs that only exist inside tree files (edges, assumptions, feedback,
    /// NBR branches, macro edges and their assumptions).
    Trees,
    /// Unknown prefix: scan everything.
    All,
}

/// Return the scope where IDs with `prefix` (uppercase) live.
pub fn scope_of(prefix: &str) -> ScanScope {
    match prefix {
        "UDE" | "RC" | "INJ" | "NC" | "GOAL" | "OBJ" | "WANT" | "OBS" | "IO" | "INT" | "DE"
        | "REQ" | "PRE" | "CSF" => ScanScope::Nodes,
        "KN" => ScanScope::Knowledge,
        "LINK" | "ASM" | "FB" | "NBR" | "MACRO" | "MASM" => ScanScope::Trees,
        _ => ScanScope::All,
    }
}

/// What `.ltp/counters.json` holds.
#[derive(Debug)]
pub enum StoredCounters {
    /// The file does not exist (e.g. a fresh git clone: `.ltp/` is ignored).
    Missing,
    /// The file exists but is not a valid counter map.
    Corrupt,
    /// The file parsed.
    Valid(Counters),
}

impl StoredCounters {
    /// The stored counters, or a zeroed set when there is no usable file.
    pub fn into_counters(self) -> Counters {
        match self {
            Self::Valid(c) => c,
            Self::Missing | Self::Corrupt => Counters::new_zeroed(),
        }
    }
}

/// Sequential counter state for all entity types in the workspace.
///
/// The stored file is only a monotonicity memory (IDs of deleted entities are
/// never reissued); safety comes from reconciling it with the disk before
/// every mint (PLAN_v052 D-1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Counters {
    /// Highest issued number per prefix.
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

    /// Read the stored counters. A missing or unparsable file is not an error
    /// (the disk scan covers it); any other read failure is.
    pub fn load_stored(counters_path: &Path) -> Result<StoredCounters> {
        match fs::read_to_string(counters_path) {
            Ok(content) => Ok(match serde_json::from_str::<Counters>(&content) {
                Ok(c) => StoredCounters::Valid(c),
                Err(_) => StoredCounters::Corrupt,
            }),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(StoredCounters::Missing),
            Err(e) => Err(scan_error(counters_path, e)),
        }
    }

    /// Highest ID observed on disk for every prefix, looking only where
    /// `scope` lives. Anything in that scope that cannot be read is an error:
    /// skipping it could reissue an ID it holds.
    pub fn observe_scope(root: &Path, scope: ScanScope) -> Result<Self> {
        let mut observed = Self {
            values: BTreeMap::new(),
        };
        let (nodes, knowledge, trees) = match scope {
            ScanScope::Nodes => (true, false, false),
            ScanScope::Knowledge => (false, true, false),
            ScanScope::Trees => (false, false, true),
            ScanScope::All => (true, true, true),
        };
        if nodes {
            observed.observe_file_names(&root.join("nodes"))?;
        }
        if knowledge {
            observed.observe_file_names(&root.join("knowledge"))?;
        }
        if trees {
            observed.observe_tree_contents(&root.join("trees"))?;
        }
        Ok(observed)
    }

    /// Raise every counter of `scope` to at least what was observed on disk.
    /// Counters never go down.
    pub fn reconcile(&mut self, observed: &Self, scope: ScanScope) {
        for (prefix, &num) in &observed.values {
            if scope_of(prefix) != scope {
                continue;
            }
            let current = self.values.entry(prefix.clone()).or_insert(0);
            if num > *current {
                *current = num;
            }
        }
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

    /// `*.json` entries of `dir`. An absent directory has none; an unlistable
    /// one is an error.
    fn json_entries(dir: &Path) -> Result<Vec<(String, PathBuf)>> {
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(vec![]),
            Err(e) => return Err(scan_error(dir, e)),
        };
        let mut out = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| scan_error(dir, e))?;
            let name = entry.file_name();
            if let Some(stem) = name.to_string_lossy().strip_suffix(".json") {
                out.push((stem.to_string(), entry.path()));
            }
        }
        Ok(out)
    }

    fn observe_file_names(&mut self, dir: &Path) -> Result<()> {
        for (stem, _) in Self::json_entries(dir)? {
            self.observe(&stem);
        }
        Ok(())
    }

    /// Record every ID stored inside tree files. A file that cannot be read is
    /// an error; one that reads but does not parse (e.g. merge-conflict
    /// markers) is scanned as text, which can only overestimate.
    fn observe_tree_contents(&mut self, dir: &Path) -> Result<()> {
        for (_, path) in Self::json_entries(dir)? {
            let bytes = fs::read(&path).map_err(|e| scan_error(&path, e))?;
            match serde_json::from_slice::<serde_json::Value>(&bytes) {
                Ok(value) => self.observe_ids_in(&value),
                Err(_) => self.observe_text(&String::from_utf8_lossy(&bytes)),
            }
        }
        Ok(())
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

    /// Record every `[A-Z]+-[0-9]+` token in `text`. Used when a file cannot
    /// be parsed; references are counted too, which is safe (overestimate).
    fn observe_text(&mut self, text: &str) {
        let bytes = text.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if !bytes[i].is_ascii_uppercase() {
                i += 1;
                continue;
            }
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_uppercase() {
                i += 1;
            }
            if i < bytes.len() && bytes[i] == b'-' {
                let digits_start = i + 1;
                let mut j = digits_start;
                while j < bytes.len() && bytes[j].is_ascii_digit() {
                    j += 1;
                }
                if j > digits_start {
                    // Both slices are ASCII, so `text` boundaries are valid.
                    self.observe(&text[start..j]);
                }
                i = j.max(i + 1);
            }
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

fn scan_error(path: &Path, source: io::Error) -> LtpError {
    LtpError::CounterScan {
        path: path.to_path_buf(),
        source,
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

    // R9 — every mintable prefix maps to the scope where it lives.
    #[test]
    fn r9_every_tracked_prefix_has_a_concrete_scope() {
        for &t in ENTITY_TYPES {
            if t == "TREE" {
                // Tree IDs are slugs; TREE is never minted.
                continue;
            }
            assert_ne!(scope_of(t), ScanScope::All, "{t} has no scope");
        }
        assert_eq!(scope_of("UDE"), ScanScope::Nodes);
        assert_eq!(scope_of("INT"), ScanScope::Nodes);
        assert_eq!(scope_of("KN"), ScanScope::Knowledge);
        assert_eq!(scope_of("MASM"), ScanScope::Trees);
        assert_eq!(scope_of("ZZZ"), ScanScope::All);
    }

    #[test]
    fn observe_text_counts_every_token() {
        let mut counters = Counters::new_zeroed();
        counters.observe_text(
            "<<<<<<< HEAD\n{\"id\": \"LINK-007\", \"ref\": \"UDE-020\"}\n=======\nNBR-003 xLINK-009 LINK- -12 ASM-\n>>>>>>> b",
        );
        let get = |k: &str| counters.values.get(k).copied();
        assert_eq!(get("LINK"), Some(9));
        assert_eq!(get("UDE"), Some(20));
        assert_eq!(get("NBR"), Some(3));
        assert_eq!(get("ASM"), Some(0));
        assert!(!counters.values.contains_key("HEAD"));
    }

    #[test]
    fn reconcile_only_raises_counters_of_the_scope() {
        let mut stored = Counters::new_zeroed();
        stored.values.insert("LINK".into(), 10);
        let mut observed = Counters::new_zeroed();
        observed.values.insert("LINK".into(), 4);
        observed.values.insert("ASM".into(), 7);
        observed.values.insert("UDE".into(), 9);
        stored.reconcile(&observed, ScanScope::Trees);
        assert_eq!(stored.values["LINK"], 10, "never goes down");
        assert_eq!(stored.values["ASM"], 7);
        assert_eq!(stored.values["UDE"], 0, "out of scope");
    }

    #[test]
    fn new_zeroed_has_all_types() {
        let counters = Counters::new_zeroed();
        for &t in ENTITY_TYPES {
            assert_eq!(counters.values.get(t), Some(&0));
        }
    }
}
