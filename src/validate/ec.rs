use crate::link::Edge;
use crate::output::OutputError;
use crate::tree::types::NodeRef;

/// Validate Evaporating Cloud (EC) specific rules.
///
/// Rules:
/// - Exactly 1 node with role "objective"
/// - At least 2 nodes with role "requirement"
/// - Each requirement must have at least 1 prerequisite connected to it
pub fn check_ec_rules(nodes: &[NodeRef], edges: &[Edge], tree_id: &str) -> Vec<OutputError> {
    let mut errors = Vec::new();

    let objectives: Vec<&NodeRef> = nodes
        .iter()
        .filter(|n| n.role.as_deref() == Some("objective"))
        .collect();

    if objectives.len() != 1 {
        errors.push(
            OutputError::new(
                "EC_VALIDATION",
                format!(
                    "EC requires exactly 1 node with role 'objective', found {}",
                    objectives.len()
                ),
            )
            .with_context("tree_id", serde_json::Value::String(tree_id.to_string()))
            .with_context(
                "sub_rule",
                serde_json::Value::String("missing_objective".to_string()),
            ),
        );
    }

    let requirements: Vec<&NodeRef> = nodes
        .iter()
        .filter(|n| n.role.as_deref() == Some("requirement"))
        .collect();

    if requirements.len() < 2 {
        errors.push(
            OutputError::new(
                "EC_VALIDATION",
                format!(
                    "EC requires at least 2 nodes with role 'requirement', found {}",
                    requirements.len()
                ),
            )
            .with_context("tree_id", serde_json::Value::String(tree_id.to_string()))
            .with_context(
                "sub_rule",
                serde_json::Value::String("minimum_2_requirements".to_string()),
            ),
        );
    }

    let prerequisites: Vec<&NodeRef> = nodes
        .iter()
        .filter(|n| n.role.as_deref() == Some("prerequisite"))
        .collect();

    for req in &requirements {
        let has_prerequisite = prerequisites.iter().any(|pre| {
            edges
                .iter()
                .any(|edge| edge.from.contains(&pre.node_ref) && edge.to == req.node_ref)
        });

        if !has_prerequisite {
            errors.push(
                OutputError::new(
                    "EC_VALIDATION",
                    format!(
                        "Requirement '{}' has no prerequisite connected to it",
                        req.node_ref
                    ),
                )
                .with_context("tree_id", serde_json::Value::String(tree_id.to_string()))
                .with_context(
                    "sub_rule",
                    serde_json::Value::String("requirement_without_prerequisite".to_string()),
                )
                .with_context("node_id", serde_json::Value::String(req.node_ref.clone())),
            );
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::link::{Edge, EdgeStatus, Logic, Operator};
    use crate::tree::types::NodeRef;

    fn node_ref(id: &str, role: Option<&str>) -> NodeRef {
        NodeRef {
            node_ref: id.to_string(),
            role: role.map(String::from),
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
            logic: Logic::Necessity,
            assumptions: vec![],
        }
    }

    #[test]
    fn valid_ec_no_errors() {
        let nodes = vec![
            node_ref("OBJ-001", Some("objective")),
            node_ref("REQ-001", Some("requirement")),
            node_ref("REQ-002", Some("requirement")),
            node_ref("PRE-001", Some("prerequisite")),
            node_ref("PRE-002", Some("prerequisite")),
        ];
        let edges = vec![
            make_edge("L1", vec!["PRE-001"], "REQ-001"),
            make_edge("L2", vec!["PRE-002"], "REQ-002"),
        ];
        let errors = check_ec_rules(&nodes, &edges, "test-ec");
        assert!(errors.is_empty());
    }

    #[test]
    fn missing_objective() {
        let nodes = vec![
            node_ref("REQ-001", Some("requirement")),
            node_ref("REQ-002", Some("requirement")),
        ];
        let errors = check_ec_rules(&nodes, &[], "test-ec");
        assert!(errors.iter().any(|e| e.detail.contains("objective")));
    }

    #[test]
    fn insufficient_requirements() {
        let nodes = vec![
            node_ref("OBJ-001", Some("objective")),
            node_ref("REQ-001", Some("requirement")),
        ];
        let errors = check_ec_rules(&nodes, &[], "test-ec");
        assert!(errors.iter().any(|e| e.detail.contains("at least 2")));
    }

    #[test]
    fn requirement_without_prerequisite() {
        let nodes = vec![
            node_ref("OBJ-001", Some("objective")),
            node_ref("REQ-001", Some("requirement")),
            node_ref("REQ-002", Some("requirement")),
            node_ref("PRE-001", Some("prerequisite")),
        ];
        let edges = vec![make_edge("L1", vec!["PRE-001"], "REQ-001")];
        let errors = check_ec_rules(&nodes, &edges, "test-ec");
        assert!(errors.iter().any(|e| e.detail.contains("REQ-002")));
    }
}
