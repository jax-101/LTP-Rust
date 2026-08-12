use serde::{Deserialize, Serialize};

use crate::link::{Edge, FeedbackEdge};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TreeType {
    Gt,
    Crt,
    Ec,
    Frt,
    Prt,
    Tt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TreeLogic {
    Sufficiency,
    Necessity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRef {
    #[serde(rename = "ref")]
    pub node_ref: String,
    pub role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroEdge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub label: String,
    pub interior_nodes: Vec<String>,
    pub interior_links: Vec<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NbrBranch {
    pub id: String,
    pub source_node: String,
    pub edges: Vec<Edge>,
    pub trim_injection: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tree {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub tree_type: TreeType,
    pub logic: TreeLogic,
    #[serde(default)]
    pub nodes: Vec<NodeRef>,
    #[serde(default)]
    pub edges: Vec<Edge>,
    #[serde(default)]
    pub macro_edges: Vec<MacroEdge>,
    #[serde(default)]
    pub feedback_edges: Vec<FeedbackEdge>,
    #[serde(default)]
    pub nbr_branches: Vec<NbrBranch>,
}
