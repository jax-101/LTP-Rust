//! Meta-graph primitives (RFC-002, ADR-015).
//!
//! Pure functions over loaded nodes and trees: no `Storage`, no I/O. They are
//! the seed of the future `ltp-core` crate.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::tree::Tree;

/// Where a node lives: a tree trunk (`nbr: None`) or an NBR branch of a tree.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct Endpoint {
    /// Tree ID.
    pub tree: String,
    /// NBR branch ID, or `None` for the trunk. Always serialized (`null`).
    pub nbr: Option<String>,
}

impl Endpoint {
    /// Trunk endpoint of `tree`.
    pub fn trunk(tree: &str) -> Self {
        Self {
            tree: tree.to_string(),
            nbr: None,
        }
    }

    /// Endpoint for NBR branch `nbr` of `tree`.
    pub fn branch(tree: &str, nbr: &str) -> Self {
        Self {
            tree: tree.to_string(),
            nbr: Some(nbr.to_string()),
        }
    }
}

/// Maps every node ID to the set of endpoints it belongs to.
///
/// Trunk membership comes from `tree.nodes`; branch membership from every
/// `from`/`to` of `nbr_branches[].edges` plus the branch `source_node`
/// (NBR nodes need not be attached to the trunk).
pub fn tree_memberships(trees: &[Tree]) -> BTreeMap<String, BTreeSet<Endpoint>> {
    let mut out: BTreeMap<String, BTreeSet<Endpoint>> = BTreeMap::new();
    for tree in trees {
        for node_ref in &tree.nodes {
            out.entry(node_ref.node_ref.clone())
                .or_default()
                .insert(Endpoint::trunk(&tree.id));
        }
        for branch in &tree.nbr_branches {
            let endpoint = Endpoint::branch(&tree.id, &branch.id);
            let members = std::iter::once(&branch.source_node).chain(
                branch
                    .edges
                    .iter()
                    .flat_map(|e| e.from.iter().chain(std::iter::once(&e.to))),
            );
            for node in members {
                out.entry(node.clone())
                    .or_default()
                    .insert(endpoint.clone());
            }
        }
    }
    out
}

/// True if `node` is present in `tree` (trunk or any NBR branch).
pub fn node_in_tree(tree: &Tree, node: &str) -> bool {
    tree.nodes.iter().any(|r| r.node_ref == node)
        || tree.nbr_branches.iter().any(|b| {
            b.source_node == node
                || b.edges
                    .iter()
                    .any(|e| e.to == node || e.from.iter().any(|f| f == node))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::link::{Edge, EdgeStatus, Logic, Operator};
    use crate::tree::{NbrBranch, NodeRef};

    fn tree_json(id: &str) -> Tree {
        let raw = serde_json::json!({
            "id": id, "name": id, "type": "frt", "logic": "sufficiency", "nodes": [], "edges": []
        });
        match serde_json::from_value(raw) {
            Ok(t) => t,
            Err(e) => panic!("fixture tree must parse: {e}"),
        }
    }

    fn edge(id: &str, from: &[&str], to: &str) -> Edge {
        Edge {
            id: id.to_string(),
            from: from.iter().map(|s| s.to_string()).collect(),
            to: to.to_string(),
            operator: Operator::Single,
            weight: None,
            status: EdgeStatus::Active,
            logic: Logic::Sufficiency,
            assumptions: vec![],
        }
    }

    #[test]
    fn memberships_cover_trunk_branch_edges_and_source() {
        let mut t = tree_json("tree-frt-a");
        t.nodes.push(NodeRef {
            node_ref: "INJ-001".into(),
            role: None,
        });
        t.nbr_branches.push(NbrBranch {
            id: "NBR-001".into(),
            source_node: "INJ-001".into(),
            edges: vec![edge("LINK-001", &["INJ-001"], "UDE-009")],
            trim_injection: None,
        });
        let m = tree_memberships(&[t]);
        let inj: Vec<_> = m["INJ-001"].iter().cloned().collect();
        assert_eq!(
            inj,
            vec![
                Endpoint::trunk("tree-frt-a"),
                Endpoint::branch("tree-frt-a", "NBR-001")
            ]
        );
        let ude: Vec<_> = m["UDE-009"].iter().cloned().collect();
        assert_eq!(ude, vec![Endpoint::branch("tree-frt-a", "NBR-001")]);
    }

    #[test]
    fn node_in_tree_sees_branch_only_nodes() {
        let mut t = tree_json("tree-frt-a");
        t.nbr_branches.push(NbrBranch {
            id: "NBR-001".into(),
            source_node: "INJ-001".into(),
            edges: vec![edge("LINK-001", &["INJ-001"], "UDE-009")],
            trim_injection: None,
        });
        assert!(node_in_tree(&t, "UDE-009"));
        assert!(node_in_tree(&t, "INJ-001"));
        assert!(!node_in_tree(&t, "UDE-010"));
    }
}
