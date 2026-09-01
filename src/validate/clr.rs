use std::collections::{HashMap, HashSet};

use crate::link::{Edge, Operator};
use crate::node::{Node, NodeType};
use crate::output::OutputWarning;

/// CLR#5: MAG edges targeting the same node should have weights summing to ~1.0.
pub fn lint_clr5_mag_weights(edges: &[Edge]) -> Vec<OutputWarning> {
    let mut warnings = Vec::new();

    // Group MAG edges by destination node
    let mut mag_groups: HashMap<&str, Vec<&Edge>> = HashMap::new();
    for edge in edges {
        if edge.operator == Operator::Mag {
            mag_groups.entry(edge.to.as_str()).or_default().push(edge);
        }
    }

    for (to_node, group) in &mag_groups {
        // Single MAG edge with multiple from[] — weight is the whole group's contribution
        if group.len() == 1 {
            let edge = group[0];
            if edge.weight.is_none() {
                warnings.push(
                    OutputWarning::new(
                        "CLR5_MAG_WEIGHT_UNDEFINED",
                        format!(
                            "MAG edge '{}' targeting '{}' has no weight defined",
                            edge.id, to_node
                        ),
                    )
                    .with_context("edge_id", serde_json::Value::String(edge.id.clone()))
                    .with_context("to_node", serde_json::Value::String(to_node.to_string())),
                );
            }
            continue;
        }

        // Multiple MAG edges to the same destination — weights should sum to ~1.0
        let mut has_undefined = false;
        let mut weight_sum = 0.0_f64;
        let mut undefined_edges: Vec<String> = Vec::new();

        for edge in group {
            match edge.weight {
                Some(w) => weight_sum += w,
                None => {
                    has_undefined = true;
                    undefined_edges.push(edge.id.clone());
                }
            }
        }

        if has_undefined {
            warnings.push(
                OutputWarning::new(
                    "CLR5_MAG_WEIGHT_UNDEFINED",
                    format!(
                        "MAG group targeting '{}' has {} edge(s) without weight: {}",
                        to_node,
                        undefined_edges.len(),
                        undefined_edges.join(", ")
                    ),
                )
                .with_context("to_node", serde_json::Value::String(to_node.to_string()))
                .with_context(
                    "undefined_edges",
                    serde_json::Value::Array(
                        undefined_edges
                            .iter()
                            .map(|id| serde_json::Value::String(id.clone()))
                            .collect(),
                    ),
                ),
            );
        }

        if !has_undefined && (weight_sum - 1.0).abs() > 0.01 {
            warnings.push(
                OutputWarning::new(
                    "CLR5_MAG_WEIGHTS_NOT_NORMALIZED",
                    format!(
                        "MAG group targeting '{}' has weights summing to {:.3} (expected ~1.0)",
                        to_node, weight_sum
                    ),
                )
                .with_context("to_node", serde_json::Value::String(to_node.to_string()))
                .with_context(
                    "weight_sum",
                    serde_json::Value::Number(
                        serde_json::Number::from_f64(weight_sum)
                            .unwrap_or_else(|| serde_json::Number::from(0)),
                    ),
                )
                .with_context("edge_count", serde_json::Value::Number(group.len().into())),
            );
        }
    }

    warnings
}

/// CLR#2: Detect causal conjunctions in node labels.
pub fn lint_clr2(nodes: &[Node]) -> Vec<OutputWarning> {
    let conjunctions = &["porque", "in order to", "because", " para ", " y "];
    let mut warnings = Vec::new();

    for node in nodes {
        let lower = node.label.to_lowercase();
        for &conjunction in conjunctions {
            if lower.contains(conjunction) {
                warnings.push(
                    OutputWarning::new(
                        "CLR2_CONJUNCTION_DETECTED",
                        format!(
                            "Causal conjunction '{}' detected in node '{}'. Consider splitting.",
                            conjunction.trim(),
                            node.id
                        ),
                    )
                    .with_context("node_id", serde_json::Value::String(node.id.clone()))
                    .with_context(
                        "conjunction",
                        serde_json::Value::String(conjunction.trim().to_string()),
                    ),
                );
            }
        }
    }

    warnings
}

/// CLR#4: Nodes with only 1 incoming SINGLE edge are candidates for insufficiency.
pub fn lint_clr4_insufficiency(edges: &[Edge]) -> Vec<OutputWarning> {
    let mut incoming: HashMap<&str, Vec<&Edge>> = HashMap::new();

    for edge in edges {
        incoming.entry(edge.to.as_str()).or_default().push(edge);
    }

    let mut warnings = Vec::new();

    for (node_id, incoming_edges) in &incoming {
        if incoming_edges.len() == 1 && incoming_edges[0].operator == Operator::Single {
            warnings.push(
                OutputWarning::new(
                    "CLR4_INSUFFICIENT_CAUSE",
                    format!(
                        "Node '{}' has only 1 incoming SINGLE edge — candidate for insufficiency (CLR#4)",
                        node_id
                    ),
                )
                .with_context("node_id", serde_json::Value::String(node_id.to_string()))
                .with_context(
                    "edge_id",
                    serde_json::Value::String(incoming_edges[0].id.clone()),
                ),
            );
        }
    }

    warnings
}

/// CLR#4/#5: AND edges with >4 inputs may mix independent causes.
pub fn lint_clr4_5_excessive_and(edges: &[Edge]) -> Vec<OutputWarning> {
    let mut warnings = Vec::new();

    for edge in edges {
        if edge.operator == Operator::And && edge.from.len() > 4 {
            warnings.push(
                OutputWarning::new(
                    "CLR4_5_EXCESSIVE_AND_INPUTS",
                    format!(
                        "Edge '{}' has {} inputs with AND operator — possible mix of independent causes (CLR#4/#5)",
                        edge.id,
                        edge.from.len()
                    ),
                )
                .with_context("edge_id", serde_json::Value::String(edge.id.clone()))
                .with_context(
                    "input_count",
                    serde_json::Value::Number(edge.from.len().into()),
                ),
            );
        }
    }

    warnings
}

/// CLR#6: Type inversion — high-level nodes (UDE, DE) in `from` pointing to low-level (RC, INT).
pub fn lint_clr6_type_inversion(
    edges: &[Edge],
    node_map: &HashMap<String, Node>,
) -> Vec<OutputWarning> {
    let high_level: HashSet<NodeType> = [NodeType::Ude, NodeType::De].into();
    let low_level: HashSet<NodeType> = [NodeType::Rc, NodeType::Int].into();

    let mut warnings = Vec::new();

    for edge in edges {
        let to_node = match node_map.get(&edge.to) {
            Some(n) => n,
            None => continue,
        };

        if !low_level.contains(&to_node.node_type) {
            continue;
        }

        let all_from_high = edge.from.iter().all(|from_id| {
            node_map
                .get(from_id)
                .map(|n| high_level.contains(&n.node_type))
                .unwrap_or(false)
        });

        if all_from_high {
            warnings.push(
                OutputWarning::new(
                    "CLR6_TYPE_INVERSION",
                    format!(
                        "Edge '{}': high-level node(s) in 'from' pointing to low-level node '{}' — suspicious inversion (CLR#6)",
                        edge.id, edge.to
                    ),
                )
                .with_context("edge_id", serde_json::Value::String(edge.id.clone()))
                .with_context("to_node", serde_json::Value::String(edge.to.clone()))
                .with_context(
                    "to_type",
                    serde_json::Value::String(format!("{:?}", to_node.node_type)),
                ),
            );
        }
    }

    warnings
}

/// CLR#7: Intangible nodes (observable: false) with <2 outgoing edges lack predicted effect.
pub fn lint_clr7_intangible(
    edges: &[Edge],
    node_map: &HashMap<String, Node>,
) -> Vec<OutputWarning> {
    let mut outgoing_count: HashMap<&str, usize> = HashMap::new();

    for edge in edges {
        for from_id in &edge.from {
            *outgoing_count.entry(from_id.as_str()).or_default() += 1;
        }
    }

    let mut warnings = Vec::new();

    for (node_id, node) in node_map {
        if !node.observable {
            let count = outgoing_count.get(node_id.as_str()).copied().unwrap_or(0);
            if count < 2 {
                warnings.push(
                    OutputWarning::new(
                        "CLR7_INTANGIBLE_NO_PREDICTED",
                        format!(
                            "Intangible node '{}' has {} outgoing edge(s) — needs at least 2 for predicted effect verification (CLR#7)",
                            node_id, count
                        ),
                    )
                    .with_context("node_id", serde_json::Value::String(node_id.clone()))
                    .with_context(
                        "outgoing_edge_count",
                        serde_json::Value::Number(count.into()),
                    ),
                );
            }
        }
    }

    warnings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::link::{Edge, EdgeStatus, Logic, Operator};
    use crate::node::{EpistemicStatus, Node, NodeMetadata, NodeStatus, NodeType};
    use std::collections::BTreeMap;

    fn make_node(id: &str, node_type: NodeType, observable: bool) -> Node {
        Node {
            id: id.to_string(),
            node_type,
            label: "Test node".to_string(),
            tags: vec![],
            observable,
            epistemic: EpistemicStatus::default(),
            metadata: NodeMetadata {
                status: NodeStatus::Active,
                extra: BTreeMap::new(),
            },
        }
    }

    fn make_edge_op(id: &str, from: Vec<&str>, to: &str, op: Operator) -> Edge {
        Edge {
            id: id.to_string(),
            from: from.into_iter().map(String::from).collect(),
            to: to.to_string(),
            operator: op,
            weight: None,
            status: EdgeStatus::Active,
            logic: Logic::Sufficiency,
            assumptions: vec![],
        }
    }

    #[test]
    fn clr2_detects_conjunction() {
        let nodes = vec![Node {
            id: "UDE-001".to_string(),
            node_type: NodeType::Ude,
            label: "Vendemos poco porque no hay marketing".to_string(),
            tags: vec![],
            observable: true,
            epistemic: EpistemicStatus::default(),
            metadata: NodeMetadata {
                status: NodeStatus::Active,
                extra: BTreeMap::new(),
            },
        }];
        let warnings = lint_clr2(&nodes);
        assert!(!warnings.is_empty());
        assert_eq!(warnings[0].code, "CLR2_CONJUNCTION_DETECTED");
    }

    #[test]
    fn clr4_single_input() {
        let edges = vec![make_edge_op("L1", vec!["A"], "B", Operator::Single)];
        let warnings = lint_clr4_insufficiency(&edges);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].code, "CLR4_INSUFFICIENT_CAUSE");
    }

    #[test]
    fn clr4_multiple_inputs_no_warning() {
        let edges = vec![
            make_edge_op("L1", vec!["A"], "B", Operator::Single),
            make_edge_op("L2", vec!["C"], "B", Operator::Single),
        ];
        let warnings = lint_clr4_insufficiency(&edges);
        assert!(warnings.is_empty());
    }

    #[test]
    fn clr4_5_excessive_and() {
        let edges = vec![make_edge_op(
            "L1",
            vec!["A", "B", "C", "D", "E"],
            "F",
            Operator::And,
        )];
        let warnings = lint_clr4_5_excessive_and(&edges);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].code, "CLR4_5_EXCESSIVE_AND_INPUTS");
    }

    #[test]
    fn clr6_type_inversion_detected() {
        let mut node_map = HashMap::new();
        node_map.insert(
            "UDE-001".to_string(),
            make_node("UDE-001", NodeType::Ude, true),
        );
        node_map.insert(
            "RC-001".to_string(),
            make_node("RC-001", NodeType::Rc, true),
        );

        let edges = vec![make_edge_op(
            "L1",
            vec!["UDE-001"],
            "RC-001",
            Operator::Single,
        )];
        let warnings = lint_clr6_type_inversion(&edges, &node_map);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].code, "CLR6_TYPE_INVERSION");
    }

    #[test]
    fn clr7_intangible_no_predicted() {
        let mut node_map = HashMap::new();
        node_map.insert(
            "RC-001".to_string(),
            make_node("RC-001", NodeType::Rc, false),
        );

        let edges = vec![make_edge_op(
            "L1",
            vec!["RC-001"],
            "UDE-001",
            Operator::Single,
        )];
        let warnings = lint_clr7_intangible(&edges, &node_map);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].code, "CLR7_INTANGIBLE_NO_PREDICTED");
    }

    #[test]
    fn clr7_intangible_with_2_outgoing_no_warning() {
        let mut node_map = HashMap::new();
        node_map.insert(
            "RC-001".to_string(),
            make_node("RC-001", NodeType::Rc, false),
        );

        let edges = vec![
            make_edge_op("L1", vec!["RC-001"], "UDE-001", Operator::Single),
            make_edge_op("L2", vec!["RC-001"], "UDE-002", Operator::Single),
        ];
        let warnings = lint_clr7_intangible(&edges, &node_map);
        assert!(warnings.is_empty());
    }

    fn make_mag_edge(id: &str, from: Vec<&str>, to: &str, weight: Option<f64>) -> Edge {
        Edge {
            id: id.to_string(),
            from: from.into_iter().map(String::from).collect(),
            to: to.to_string(),
            operator: Operator::Mag,
            weight,
            status: EdgeStatus::Active,
            logic: Logic::Sufficiency,
            assumptions: vec![],
        }
    }

    #[test]
    fn clr5_mag_weights_normalized() {
        let edges = vec![
            make_mag_edge("L1", vec!["A"], "C", Some(0.6)),
            make_mag_edge("L2", vec!["B"], "C", Some(0.4)),
        ];
        let warnings = lint_clr5_mag_weights(&edges);
        assert!(warnings.is_empty());
    }

    #[test]
    fn clr5_mag_weights_not_normalized() {
        let edges = vec![
            make_mag_edge("L1", vec!["A"], "C", Some(0.8)),
            make_mag_edge("L2", vec!["B"], "C", Some(0.7)),
        ];
        let warnings = lint_clr5_mag_weights(&edges);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].code, "CLR5_MAG_WEIGHTS_NOT_NORMALIZED");
    }

    #[test]
    fn clr5_mag_weight_undefined_in_group() {
        let edges = vec![
            make_mag_edge("L1", vec!["A"], "C", Some(0.6)),
            make_mag_edge("L2", vec!["B"], "C", None),
        ];
        let warnings = lint_clr5_mag_weights(&edges);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].code, "CLR5_MAG_WEIGHT_UNDEFINED");
    }

    #[test]
    fn clr5_single_mag_no_weight() {
        let edges = vec![make_mag_edge("L1", vec!["A", "B"], "C", None)];
        let warnings = lint_clr5_mag_weights(&edges);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].code, "CLR5_MAG_WEIGHT_UNDEFINED");
    }

    #[test]
    fn clr5_non_mag_edges_ignored() {
        let edges = vec![
            make_edge_op("L1", vec!["A"], "C", Operator::Single),
            make_edge_op("L2", vec!["B"], "C", Operator::Or),
        ];
        let warnings = lint_clr5_mag_weights(&edges);
        assert!(warnings.is_empty());
    }
}
