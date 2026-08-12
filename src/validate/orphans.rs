use std::collections::HashSet;

use crate::link::Edge;
use crate::output::OutputWarning;
use crate::tree::types::NodeRef;

/// Detect nodes that are attached to a tree but have no edges (orphans within the tree).
pub fn check_orphans(nodes: &[NodeRef], edges: &[Edge], tree_id: &str) -> Vec<OutputWarning> {
    let mut connected: HashSet<&str> = HashSet::new();

    for edge in edges {
        for from_id in &edge.from {
            connected.insert(from_id.as_str());
        }
        connected.insert(edge.to.as_str());
    }

    let mut warnings = Vec::new();

    for node_ref in nodes {
        if !connected.contains(node_ref.node_ref.as_str()) {
            warnings.push(
                OutputWarning::new(
                    "ORPHAN_NODE_IN_TREE",
                    format!(
                        "Node '{}' is attached to tree '{}' but has no edges",
                        node_ref.node_ref, tree_id
                    ),
                )
                .with_context(
                    "node_id",
                    serde_json::Value::String(node_ref.node_ref.clone()),
                )
                .with_context("tree_id", serde_json::Value::String(tree_id.to_string())),
            );
        }
    }

    warnings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::link::{Edge, EdgeStatus, Logic, Operator};
    use crate::tree::types::NodeRef;

    fn node_ref(id: &str) -> NodeRef {
        NodeRef {
            node_ref: id.to_string(),
            role: None,
        }
    }

    fn make_edge(id: &str, from: Vec<&str>, to: &str) -> Edge {
        Edge {
            id: id.to_string(),
            from: from.into_iter().map(String::from).collect(),
            to: to.to_string(),
            operator: Operator::Single,
            weight: None,
            status: EdgeStatus::Active,
            logic: Logic::Sufficiency,
            assumptions: vec![],
        }
    }

    #[test]
    fn connected_nodes_no_orphans() {
        let nodes = vec![node_ref("A"), node_ref("B")];
        let edges = vec![make_edge("L1", vec!["A"], "B")];
        let warnings = check_orphans(&nodes, &edges, "test-tree");
        assert!(warnings.is_empty());
    }

    #[test]
    fn orphan_detected() {
        let nodes = vec![node_ref("A"), node_ref("B"), node_ref("C")];
        let edges = vec![make_edge("L1", vec!["A"], "B")];
        let warnings = check_orphans(&nodes, &edges, "test-tree");
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].code, "ORPHAN_NODE_IN_TREE");
        assert!(warnings[0].detail.contains("C"));
    }
}
