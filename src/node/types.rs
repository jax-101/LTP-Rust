use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;

/// Epistemic classification of a node within the Logical Thinking Process.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EpistemicStatus {
    /// Grounded in verified evidence.
    Fact,
    /// Proposed causal explanation, not yet verified.
    #[default]
    Hypothesis,
    /// Taken as given without direct evidence.
    Assumption,
    /// Logically inferred from other nodes.
    Derived,
}

fn default_epistemic() -> EpistemicStatus {
    EpistemicStatus::Hypothesis
}

fn is_hypothesis(status: &EpistemicStatus) -> bool {
    *status == EpistemicStatus::Hypothesis
}

/// Deserializes `EpistemicStatus`, treating `null` as `Hypothesis`.
fn deserialize_epistemic_nullable<'de, D>(
    deserializer: D,
) -> std::result::Result<EpistemicStatus, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<EpistemicStatus>::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum NodeType {
    Ude,
    Rc,
    Inj,
    Nc,
    Goal,
    Obj,
    Want,
    Obs,
    Io,
    Int,
    De,
    Req,
    Pre,
    /// Critical Success Factor: nivel intermedio del Goal Tree (`GOAL ← CSF ← NC`).
    Csf,
}

impl NodeType {
    /// Returns the ID prefix string used for sequential ID generation.
    pub fn prefix(&self) -> &'static str {
        match self {
            Self::Ude => "UDE",
            Self::Rc => "RC",
            Self::Inj => "INJ",
            Self::Nc => "NC",
            Self::Goal => "GOAL",
            Self::Obj => "OBJ",
            Self::Want => "WANT",
            Self::Obs => "OBS",
            Self::Io => "IO",
            Self::Int => "INT",
            Self::De => "DE",
            Self::Req => "REQ",
            Self::Pre => "PRE",
            Self::Csf => "CSF",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeStatus {
    Active,
    Draft,
    Invalidated,
    Superseded,
}

/// Cross-tree reference from one node to another (ADR-015).
///
/// `tree` pins the reference to a single tree the target is attached to;
/// `None` means "every tree the target belongs to" (intentional fan-out).
/// Ordered by `(node, tree)` so a node's refs form a canonical sorted set.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CrossRef {
    /// Target node ID.
    pub node: String,
    /// Optional tree the reference is scoped to; serialized as `null` when unset.
    #[serde(default)]
    pub tree: Option<String>,
}

impl CrossRef {
    /// Parses the CLI form `NODE` or `NODE@TREE`. Both parts must be non-empty
    /// and contain no whitespace; at most one `@` is allowed.
    pub fn parse(s: &str) -> std::result::Result<Self, String> {
        let valid = |part: &str| !part.is_empty() && !part.chars().any(char::is_whitespace);
        let (node, tree) = match s.split_once('@') {
            Some((node, tree)) => (node, Some(tree)),
            None => (s, None),
        };
        if !valid(node) || tree.is_some_and(|t| !valid(t) || t.contains('@')) {
            return Err(format!("Invalid ref '{s}': expected NODE or NODE@TREE"));
        }
        Ok(Self {
            node: node.to_string(),
            tree: tree.map(str::to_string),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetadata {
    pub status: NodeStatus,
    /// Outbound cross-tree references, kept sorted and deduplicated.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refs: Vec<CrossRef>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

impl NodeMetadata {
    /// Creates metadata with the given status, no refs and no extra keys.
    pub fn new(status: NodeStatus) -> Self {
        Self {
            status,
            refs: Vec::new(),
            extra: BTreeMap::new(),
        }
    }

    /// Inserts `r` keeping `refs` sorted and unique. Returns `false` if already present.
    pub fn add_ref(&mut self, r: CrossRef) -> bool {
        match self.refs.binary_search(&r) {
            Ok(_) => false,
            Err(pos) => {
                self.refs.insert(pos, r);
                true
            }
        }
    }

    /// Removes `r`. Returns `false` if it was not present.
    pub fn remove_ref(&mut self, r: &CrossRef) -> bool {
        let before = self.refs.len();
        self.refs.retain(|x| x != r);
        self.refs.len() != before
    }

    /// Sorts and deduplicates `refs` (normalizes hand-edited or legacy data).
    pub fn normalize_refs(&mut self) {
        self.refs.sort();
        self.refs.dedup();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: NodeType,
    pub label: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default = "default_observable")]
    pub observable: bool,
    #[serde(
        default = "default_epistemic",
        skip_serializing_if = "is_hypothesis",
        deserialize_with = "deserialize_epistemic_nullable"
    )]
    pub epistemic: EpistemicStatus,
    pub metadata: NodeMetadata,
}

fn default_observable() -> bool {
    true
}

#[cfg(test)]
mod ref_tests {
    use super::*;

    fn parse(raw: serde_json::Value) -> NodeMetadata {
        match serde_json::from_value(raw) {
            Ok(m) => m,
            Err(e) => panic!("metadata must parse: {e}"),
        }
    }

    #[test]
    fn metadata_without_refs_serializes_like_v030() {
        let raw = serde_json::json!({"status": "active", "owner": "ana"});
        let meta = parse(raw.clone());
        assert!(meta.refs.is_empty());
        assert_eq!(serde_json::to_value(&meta).ok(), Some(raw));
    }

    #[test]
    fn refs_are_typed_not_swallowed_by_extra() {
        let meta = parse(serde_json::json!({
            "status": "active",
            "refs": [{"node": "NC-001"}, {"node": "CSF-001", "tree": "tree-gt-a"}]
        }));
        assert!(!meta.extra.contains_key("refs"));
        assert_eq!(meta.refs.len(), 2);
        assert_eq!(meta.refs[0].tree, None);
        let out = serde_json::to_value(&meta).ok();
        assert_eq!(
            out.as_ref().and_then(|v| v["refs"][0].get("tree")).cloned(),
            Some(serde_json::Value::Null),
            "unset tree serializes as null"
        );
    }

    #[test]
    fn add_ref_keeps_sorted_unique_set() {
        let mut meta = NodeMetadata::new(NodeStatus::Active);
        let r = |n: &str, t: Option<&str>| CrossRef {
            node: n.into(),
            tree: t.map(Into::into),
        };
        assert!(meta.add_ref(r("NC-002", None)));
        assert!(meta.add_ref(r("NC-001", Some("tree-gt-b"))));
        assert!(meta.add_ref(r("NC-001", None)));
        assert!(!meta.add_ref(r("NC-002", None)));
        assert_eq!(
            meta.refs,
            vec![
                r("NC-001", None),
                r("NC-001", Some("tree-gt-b")),
                r("NC-002", None)
            ]
        );
        assert!(meta.remove_ref(&r("NC-001", None)));
        assert!(!meta.remove_ref(&r("NC-001", None)));
        assert_eq!(meta.refs.len(), 2);
    }

    #[test]
    fn cli_ref_parse_accepts_node_and_node_at_tree_only() {
        let ok = |s: &str| CrossRef::parse(s).ok();
        assert_eq!(
            ok("NC-001"),
            Some(CrossRef {
                node: "NC-001".into(),
                tree: None
            })
        );
        assert_eq!(
            ok("NC-001@tree-gt-a"),
            Some(CrossRef {
                node: "NC-001".into(),
                tree: Some("tree-gt-a".into())
            })
        );
        for bad in [
            "",
            "@",
            "@tree-gt-a",
            "NC-001@",
            "a@b@c",
            "NC 001",
            " NC-001",
        ] {
            assert!(CrossRef::parse(bad).is_err(), "{bad:?} must be rejected");
        }
    }

    #[test]
    fn malformed_refs_fail_to_parse() {
        for bad in [
            serde_json::json!("garbage"),
            serde_json::json!(["NC-001"]),
            serde_json::json!([{"tree": "x"}]),
            serde_json::json!([{"node": 5}]),
        ] {
            let raw = serde_json::json!({"status": "active", "refs": bad});
            assert!(serde_json::from_value::<NodeMetadata>(raw).is_err());
        }
    }
}
