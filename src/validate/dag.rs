use std::collections::{HashMap, HashSet};

use crate::errors::{LtpError, Result};
use crate::link::Edge;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Color {
    White,
    Gray,
    Black,
}

pub fn check_dag(edges: &[Edge], tree_id: &str) -> Result<()> {
    let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut all_nodes: HashSet<&str> = HashSet::new();

    for edge in edges {
        for from_node in &edge.from {
            adjacency
                .entry(from_node.as_str())
                .or_default()
                .push(edge.to.as_str());
            all_nodes.insert(from_node.as_str());
        }
        all_nodes.insert(edge.to.as_str());
    }

    let mut colors: HashMap<&str, Color> = all_nodes.iter().map(|&n| (n, Color::White)).collect();

    for &node in &all_nodes {
        if colors[node] == Color::White && has_cycle(node, &adjacency, &mut colors) {
            return Err(LtpError::CircularDependencyDetected {
                tree_id: tree_id.to_string(),
            });
        }
    }

    Ok(())
}

fn has_cycle<'a>(
    node: &'a str,
    adjacency: &HashMap<&'a str, Vec<&'a str>>,
    colors: &mut HashMap<&'a str, Color>,
) -> bool {
    colors.insert(node, Color::Gray);

    if let Some(neighbors) = adjacency.get(node) {
        for &neighbor in neighbors {
            let color = colors.get(neighbor).copied().unwrap_or(Color::Black);
            if color == Color::Gray {
                return true;
            }
            if color == Color::White && has_cycle(neighbor, adjacency, colors) {
                return true;
            }
        }
    }

    colors.insert(node, Color::Black);
    false
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
    fn valid_dag_passes() {
        let edges = vec![
            make_edge("L1", vec!["A"], "B"),
            make_edge("L2", vec!["B"], "C"),
        ];
        assert!(check_dag(&edges, "test-tree").is_ok());
    }

    #[test]
    fn cycle_detected() {
        let edges = vec![
            make_edge("L1", vec!["A"], "B"),
            make_edge("L2", vec!["B"], "C"),
            make_edge("L3", vec!["C"], "A"),
        ];
        assert!(matches!(
            check_dag(&edges, "test-tree"),
            Err(LtpError::CircularDependencyDetected { .. })
        ));
    }

    #[test]
    fn diamond_dag_passes() {
        let edges = vec![
            make_edge("L1", vec!["A"], "B"),
            make_edge("L2", vec!["A"], "C"),
            make_edge("L3", vec!["B", "C"], "D"),
        ];
        assert!(check_dag(&edges, "test-tree").is_ok());
    }
}
