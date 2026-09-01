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
    let mut path: Vec<&str> = Vec::new();

    for &node in &all_nodes {
        if colors[node] == Color::White {
            if let Some(cycle) = find_cycle(node, &adjacency, &mut colors, &mut path) {
                return Err(LtpError::CircularDependencyDetected {
                    tree_id: tree_id.to_string(),
                    cycle_path: cycle,
                });
            }
        }
    }

    Ok(())
}

/// DFS that returns the cycle path when a back-edge is found.
fn find_cycle<'a>(
    node: &'a str,
    adjacency: &HashMap<&'a str, Vec<&'a str>>,
    colors: &mut HashMap<&'a str, Color>,
    path: &mut Vec<&'a str>,
) -> Option<Vec<String>> {
    colors.insert(node, Color::Gray);
    path.push(node);

    if let Some(neighbors) = adjacency.get(node) {
        for &neighbor in neighbors {
            let color = colors.get(neighbor).copied().unwrap_or(Color::Black);
            if color == Color::Gray {
                let cycle_start = path.iter().position(|&n| n == neighbor).unwrap_or(0);
                let mut cycle: Vec<String> =
                    path[cycle_start..].iter().map(|s| s.to_string()).collect();
                cycle.push(neighbor.to_string());
                return Some(cycle);
            }
            if color == Color::White {
                if let Some(cycle) = find_cycle(neighbor, adjacency, colors, path) {
                    return Some(cycle);
                }
            }
        }
    }

    path.pop();
    colors.insert(node, Color::Black);
    None
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
    fn cycle_detected_with_path() {
        let edges = vec![
            make_edge("L1", vec!["A"], "B"),
            make_edge("L2", vec!["B"], "C"),
            make_edge("L3", vec!["C"], "A"),
        ];
        let err = check_dag(&edges, "test-tree").unwrap_err();
        match err {
            LtpError::CircularDependencyDetected {
                tree_id,
                cycle_path,
            } => {
                assert_eq!(tree_id, "test-tree");
                assert!(cycle_path.len() >= 3);
                assert_eq!(cycle_path.first(), cycle_path.last());
            }
            _ => panic!("Expected CircularDependencyDetected"),
        }
    }

    #[test]
    fn cycle_in_subgraph_reports_correct_nodes() {
        let edges = vec![
            make_edge("L1", vec!["X"], "A"),
            make_edge("L2", vec!["A"], "B"),
            make_edge("L3", vec!["B"], "C"),
            make_edge("L4", vec!["C"], "B"),
        ];
        let err = check_dag(&edges, "test-tree").unwrap_err();
        match err {
            LtpError::CircularDependencyDetected { cycle_path, .. } => {
                assert!(cycle_path.contains(&"B".to_string()));
                assert!(cycle_path.contains(&"C".to_string()));
                assert!(!cycle_path.contains(&"X".to_string()));
            }
            _ => panic!("Expected CircularDependencyDetected"),
        }
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
