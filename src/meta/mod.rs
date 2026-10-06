//! Meta-graph primitives (RFC-002, ADR-015).
//!
//! Pure functions over loaded nodes and trees: no `Storage`, no I/O. They are
//! the seed of the future `ltp-core` crate.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::node::Node;
use crate::tree::{Tree, TreeLogic};

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

/// Logic of each side of an inferred relation (ADR-014: derived from tree type).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RelationLogic {
    /// Logic of the referencing endpoint's tree.
    pub referencing: TreeLogic,
    /// Logic of the referenced endpoint's tree.
    pub referenced: TreeLogic,
}

/// One node ref supporting an inferred relation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct RelationBasis {
    /// Referencing node ID.
    pub node: String,
    /// Referenced node ID.
    #[serde(rename = "ref")]
    pub target: String,
}

/// Structural, untyped relation between two endpoints, inferred from node refs
/// (ADR-015, D-4: no `relation_type` and no asserted direction of intent).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InferredRelation {
    /// Endpoint holding the referencing nodes.
    pub referencing: Endpoint,
    /// Endpoint holding the referenced nodes.
    pub referenced: Endpoint,
    /// Tree logic of each endpoint.
    pub logic: RelationLogic,
    /// Supporting refs, sorted.
    pub basis: Vec<RelationBasis>,
    /// Always `true` in Slice 1 (computed on the fly, never persisted).
    pub inferred: bool,
}

/// Infers the meta-graph from node refs.
///
/// For every node N and ref r: endpoints(N) × endpoints(r.node), keeping only
/// endpoints in `r.tree` when the ref is pinned. Pairs with identical endpoints
/// are dropped (intra-tree refs are not relations). Refs to nodes or trees that
/// are absent contribute nothing. Output is sorted by `(referencing, referenced)`
/// and independent of input order.
pub fn infer_relations(nodes: &[Node], trees: &[Tree]) -> Vec<InferredRelation> {
    let memberships = tree_memberships(trees);
    let logic_of: BTreeMap<&str, TreeLogic> = trees
        .iter()
        .map(|t| (t.id.as_str(), t.tree_type.logic()))
        .collect();
    let mut grouped: BTreeMap<(&Endpoint, &Endpoint), BTreeSet<RelationBasis>> = BTreeMap::new();
    for node in nodes {
        let Some(from_ends) = memberships.get(&node.id) else {
            continue;
        };
        for r in &node.metadata.refs {
            let Some(to_ends) = memberships.get(&r.node) else {
                continue;
            };
            for to in to_ends
                .iter()
                .filter(|e| r.tree.as_ref().is_none_or(|t| *t == e.tree))
            {
                for from in from_ends.iter().filter(|from| *from != to) {
                    grouped
                        .entry((from, to))
                        .or_default()
                        .insert(RelationBasis {
                            node: node.id.clone(),
                            target: r.node.clone(),
                        });
                }
            }
        }
    }
    grouped
        .into_iter()
        .filter_map(|((from, to), basis)| {
            Some(InferredRelation {
                logic: RelationLogic {
                    referencing: *logic_of.get(from.tree.as_str())?,
                    referenced: *logic_of.get(to.tree.as_str())?,
                },
                referencing: from.clone(),
                referenced: to.clone(),
                basis: basis.into_iter().collect(),
                inferred: true,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::link::{Edge, EdgeStatus, Logic, Operator};
    use crate::node::{CrossRef, EpistemicStatus, NodeMetadata, NodeStatus, NodeType};
    use crate::tree::{NbrBranch, NodeRef};

    fn tree_json(id: &str) -> Tree {
        typed_tree(id, "frt", &[])
    }

    fn typed_tree(id: &str, kind: &str, nodes: &[&str]) -> Tree {
        let refs: Vec<_> = nodes
            .iter()
            .map(|n| serde_json::json!({"ref": n, "role": null}))
            .collect();
        let raw = serde_json::json!({
            "id": id, "name": id, "type": kind, "logic": "sufficiency", "nodes": refs, "edges": []
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

    fn node(id: &str, refs: &[(&str, Option<&str>)]) -> Node {
        let mut metadata = NodeMetadata::new(NodeStatus::Active);
        for (target, tree) in refs {
            metadata.add_ref(CrossRef {
                node: target.to_string(),
                tree: tree.map(str::to_string),
            });
        }
        Node {
            id: id.to_string(),
            node_type: NodeType::Ude,
            label: id.to_string(),
            tags: vec![],
            observable: true,
            epistemic: EpistemicStatus::default(),
            metadata,
        }
    }

    fn pairs(rels: &[InferredRelation]) -> Vec<(String, String)> {
        rels.iter()
            .map(|r| {
                let show = |e: &Endpoint| match &e.nbr {
                    Some(n) => format!("{}/{n}", e.tree),
                    None => e.tree.clone(),
                };
                (show(&r.referencing), show(&r.referenced))
            })
            .collect()
    }

    #[test]
    fn single_ref_yields_one_relation_with_logic_per_endpoint() {
        let trees = [
            typed_tree("tree-crt-x", "crt", &["UDE-001"]),
            typed_tree("tree-gt-y", "gt", &["NC-001"]),
        ];
        let nodes = [node("UDE-001", &[("NC-001", None)]), node("NC-001", &[])];
        let rels = infer_relations(&nodes, &trees);
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].referencing, Endpoint::trunk("tree-crt-x"));
        assert_eq!(rels[0].referenced, Endpoint::trunk("tree-gt-y"));
        assert_eq!(rels[0].logic.referencing, TreeLogic::Sufficiency);
        assert_eq!(rels[0].logic.referenced, TreeLogic::Necessity);
        assert_eq!(
            rels[0].basis,
            vec![RelationBasis {
                node: "UDE-001".into(),
                target: "NC-001".into()
            }]
        );
        assert!(rels[0].inferred);
    }

    #[test]
    fn fan_out_is_n_by_m_and_pin_filters_target_side() {
        let trees = [
            typed_tree("tree-crt-a", "crt", &["UDE-001"]),
            typed_tree("tree-crt-b", "crt", &["UDE-001"]),
            typed_tree("tree-gt-a", "gt", &["NC-001"]),
            typed_tree("tree-gt-b", "gt", &["NC-001"]),
        ];
        let free = [node("UDE-001", &[("NC-001", None)])];
        assert_eq!(infer_relations(&free, &trees).len(), 4);
        let pinned = [node("UDE-001", &[("NC-001", Some("tree-gt-b"))])];
        assert_eq!(
            pairs(&infer_relations(&pinned, &trees)),
            vec![
                ("tree-crt-a".into(), "tree-gt-b".into()),
                ("tree-crt-b".into(), "tree-gt-b".into())
            ]
        );
    }

    #[test]
    fn identical_endpoints_are_dropped_but_trunk_to_branch_is_kept() {
        let mut frt = typed_tree("tree-frt-a", "frt", &["INJ-001", "DE-001"]);
        frt.nbr_branches.push(NbrBranch {
            id: "NBR-001".into(),
            source_node: "INJ-001".into(),
            edges: vec![edge("LINK-009", &["INJ-001"], "UDE-009")],
            trim_injection: None,
        });
        let trees = [frt];
        let same = [node("DE-001", &[("INJ-001", Some("tree-frt-a"))])];
        // INJ-001 lives in trunk and branch: trunk→trunk dropped, trunk→branch kept.
        assert_eq!(
            pairs(&infer_relations(&same, &trees)),
            vec![("tree-frt-a".into(), "tree-frt-a/NBR-001".into())]
        );
        let intra = [node("DE-001", &[])];
        assert!(infer_relations(&intra, &trees).is_empty());
    }

    #[test]
    fn absent_targets_and_unattached_nodes_contribute_nothing() {
        let trees = [typed_tree("tree-crt-x", "crt", &["UDE-001"])];
        let nodes = [
            node(
                "UDE-001",
                &[("NC-404", None), ("NC-001", Some("tree-gt-gone"))],
            ),
            node("UDE-777", &[("UDE-001", None)]),
        ];
        assert!(infer_relations(&nodes, &trees).is_empty());
    }

    #[test]
    fn basis_is_grouped_sorted_and_input_order_independent() {
        let trees = vec![
            typed_tree("tree-crt-x", "crt", &["UDE-001", "UDE-002"]),
            typed_tree("tree-gt-y", "gt", &["NC-001", "NC-002"]),
        ];
        let nodes = vec![
            node("UDE-002", &[("NC-001", None), ("NC-002", None)]),
            node(
                "UDE-001",
                &[("NC-002", None), ("NC-002", Some("tree-gt-y"))],
            ),
        ];
        let a = infer_relations(&nodes, &trees);
        let mut rn = nodes;
        rn.reverse();
        let mut rt = trees;
        rt.reverse();
        let b = infer_relations(&rn, &rt);
        assert_eq!(a, b);
        assert_eq!(a.len(), 1);
        let basis: Vec<(&str, &str)> = a[0]
            .basis
            .iter()
            .map(|x| (x.node.as_str(), x.target.as_str()))
            .collect();
        assert_eq!(
            basis,
            vec![
                ("UDE-001", "NC-002"),
                ("UDE-002", "NC-001"),
                ("UDE-002", "NC-002")
            ]
        );
    }
}
