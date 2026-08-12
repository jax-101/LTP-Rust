use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
    pub metadata: NodeMetadata,
}

fn default_observable() -> bool {
    true
}
