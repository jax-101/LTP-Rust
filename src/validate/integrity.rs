use std::collections::HashSet;

use crate::link::Edge;
use crate::output::OutputError;

/// Check referential integrity: every node referenced in edges must exist in the global pool.
pub fn check_integrity(
    edges: &[Edge],
    node_pool: &HashSet<String>,
    tree_id: &str,
) -> Vec<OutputError> {
    let mut errors = Vec::new();

    for edge in edges {
        for from_node in &edge.from {
            if !node_pool.contains(from_node.as_str()) {
                errors.push(
                    OutputError::new(
                        "REFERENTIAL_INTEGRITY_VIOLATION",
                        format!(
                            "Node '{}' referenced in edge '{}' does not exist in pool",
                            from_node, edge.id
                        ),
                    )
                    .with_context("node_id", serde_json::Value::String(from_node.clone()))
                    .with_context("edge_id", serde_json::Value::String(edge.id.clone()))
                    .with_context("tree_id", serde_json::Value::String(tree_id.to_string())),
                );
            }
        }

        if !node_pool.contains(edge.to.as_str()) {
            errors.push(
                OutputError::new(
                    "REFERENTIAL_INTEGRITY_VIOLATION",
                    format!(
                        "Node '{}' referenced in edge '{}' does not exist in pool",
                        edge.to, edge.id
                    ),
                )
                .with_context("node_id", serde_json::Value::String(edge.to.clone()))
                .with_context("edge_id", serde_json::Value::String(edge.id.clone()))
                .with_context("tree_id", serde_json::Value::String(tree_id.to_string())),
            );
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::link::{Edge, EdgeStatus, Logic, Operator};

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
    fn valid_references_no_errors() {
        let pool: HashSet<String> = ["A", "B", "C"].iter().map(|s| s.to_string()).collect();
        let edges = vec![
            make_edge("L1", vec!["A"], "B"),
            make_edge("L2", vec!["B"], "C"),
        ];
        let errors = check_integrity(&edges, &pool, "test-tree");
        assert!(errors.is_empty());
    }

    #[test]
    fn missing_from_node_detected() {
        let pool: HashSet<String> = ["A", "B"].iter().map(|s| s.to_string()).collect();
        let edges = vec![make_edge("L1", vec!["MISSING"], "B")];
        let errors = check_integrity(&edges, &pool, "test-tree");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "REFERENTIAL_INTEGRITY_VIOLATION");
    }

    #[test]
    fn missing_to_node_detected() {
        let pool: HashSet<String> = ["A"].iter().map(|s| s.to_string()).collect();
        let edges = vec![make_edge("L1", vec!["A"], "MISSING")];
        let errors = check_integrity(&edges, &pool, "test-tree");
        assert_eq!(errors.len(), 1);
    }
}
