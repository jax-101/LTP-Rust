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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetadata {
    pub status: NodeStatus,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
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
