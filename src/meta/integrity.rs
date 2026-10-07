//! Global integrity of node mutations (ADR-016, v0.5.0).
//!
//! Pure functions over a single [`Tree`]: no `Storage`, no I/O. `node split` uses
//! [`redirect_split`], `node rm` uses [`prune_removed`] and `validate` uses
//! [`check_tree_integrity`]. Seed of the future `ltp-core` crate.

use std::collections::HashSet;

use serde::Serialize;

use crate::link::Edge;
use crate::output::OutputError;
use crate::tree::{MacroEdge, MacroEdgeStatus, NodeRef, Tree};

/// Why `node rm` removed a long arrow (ADR-016 D-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacroRemovalReason {
    /// The `from` or `to` endpoint of the macro was deleted.
    EndpointRemoved,
    /// An `Overlay` lost every interior link.
    InteriorEmptied,
}

impl MacroRemovalReason {
    /// Wire name (`snake_case`). Single source of truth: `Serialize` delegates here.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EndpointRemoved => "endpoint_removed",
            Self::InteriorEmptied => "interior_emptied",
        }
    }
}

impl Serialize for MacroRemovalReason {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// A long arrow removed by [`prune_removed`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovedMacro {
    /// Macro ID (`MACRO-xxx`).
    pub id: String,
    /// Why it was removed.
    pub reason: MacroRemovalReason,
    /// Lifecycle state it had: losing a `Reservation` loses a top-down intent that is
    /// nowhere else in the graph; losing an `Overlay` leaves its interior chain in place.
    pub status: MacroEdgeStatus,
    /// `from` endpoint of the removed macro.
    pub from: String,
    /// `to` endpoint of the removed macro.
    pub to: String,
    /// IDs of the `MacroAssumption`s destroyed with it, in stored (creation) order.
    pub assumption_ids: Vec<String>,
}

/// Result of pruning one tree after `node rm`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PruneReport {
    /// Edges removed: trunk + feedback + NBR (same meaning as `removed_edges_count`).
    pub edges_removed: usize,
    /// NBR branches removed because their source node was deleted, in tree order.
    pub removed_branches: Vec<String>,
    /// Long arrows removed, in tree order.
    pub removed_macros: Vec<RemovedMacro>,
    /// Whether the tree changed at all, so it must be saved.
    pub changed: bool,
}

/// Redirects `original` to (`first`, `second`) in every structure of the tree (ADR-016 D-2).
///
/// Inbound references go to `first` (edge `to`, feedback `to`, macro `to`, NBR
/// `source_node`). Outbound references go to `second` (edge `from[]`, feedback `from`,
/// macro `from`). In `nodes[]` and `macro.interior_nodes`, `original` is replaced in place
/// by `[first, second]`, without duplicating a child that is already listed. Returns
/// whether anything changed.
pub fn redirect_split(tree: &mut Tree, original: &str, first: &str, second: &str) -> bool {
    let children = [first, second];
    let mut changed = splice_children(
        &mut tree.nodes,
        |n| n.node_ref.as_str(),
        original,
        children,
        |c| NodeRef {
            node_ref: c.to_string(),
            role: None,
        },
    );
    for edge in &mut tree.edges {
        changed |= redirect_edge(edge, original, first, second);
    }
    for fb in &mut tree.feedback_edges {
        changed |= redirect_ref(&mut fb.from, original, second);
        changed |= redirect_ref(&mut fb.to, original, first);
    }
    for branch in &mut tree.nbr_branches {
        changed |= redirect_ref(&mut branch.source_node, original, first);
        for edge in &mut branch.edges {
            changed |= redirect_edge(edge, original, first, second);
        }
    }
    for m in &mut tree.macro_edges {
        changed |= redirect_ref(&mut m.from, original, second);
        changed |= redirect_ref(&mut m.to, original, first);
        changed |= splice_children(
            &mut m.interior_nodes,
            String::as_str,
            original,
            children,
            str::to_string,
        );
    }
    changed
}

/// Replaces `slot` with `target` if it holds `original`.
fn redirect_ref(slot: &mut String, original: &str, target: &str) -> bool {
    if slot == original {
        *slot = target.to_string();
        true
    } else {
        false
    }
}

/// Inbound (`to`) goes to `first`, outbound (`from[]`) goes to `second`.
fn redirect_edge(edge: &mut Edge, original: &str, first: &str, second: &str) -> bool {
    let mut changed = redirect_ref(&mut edge.to, original, first);
    for from in &mut edge.from {
        changed |= redirect_ref(from, original, second);
    }
    changed
}

/// Replaces `original` in place by the children that are not listed yet (ADR-016 D-2).
///
/// Entries already in the list are never moved or rebuilt, so an existing child keeps its
/// slot and its data (e.g. `role`). The missing children take the slot of the first
/// occurrence of `original`, in order. Every occurrence of `original` is dropped.
fn splice_children<T>(
    items: &mut Vec<T>,
    key: impl Fn(&T) -> &str,
    original: &str,
    children: [&str; 2],
    make: impl Fn(&str) -> T,
) -> bool {
    let Some(pos) = items.iter().position(|it| key(it) == original) else {
        return false;
    };
    let missing: Vec<T> = children
        .into_iter()
        .filter(|c| !items.iter().any(|it| key(it) == *c))
        .map(make)
        .collect();
    // `pos` is the first occurrence, so nothing before it is removed and it stays valid.
    items.retain(|it| key(it) != original);
    items.splice(pos..pos, missing);
    true
}

/// Removes every reference to `ids` from the tree: trunk, feedback, NBR and macros (ADR-016 D-3).
///
/// A macro whose endpoint is removed goes away (`EndpointRemoved`, which takes priority).
/// Otherwise the removed nodes leave `interior_nodes` and the removed trunk edges leave
/// `interior_links`. An `Overlay` this call touched (trimmed) that has no live interior link
/// left (one still in `tree.edges`) goes away (`InteriorEmptied`). Untouched macros are never
/// modified, and ghost link IDs left by other commands are not cleaned.
pub fn prune_removed(tree: &mut Tree, ids: &HashSet<&str>) -> PruneReport {
    let mut report = PruneReport::default();

    let before_nodes = tree.nodes.len();
    tree.nodes.retain(|n| !ids.contains(n.node_ref.as_str()));
    report.changed = tree.nodes.len() != before_nodes;

    let (gone, kept): (Vec<Edge>, Vec<Edge>) = std::mem::take(&mut tree.edges)
        .into_iter()
        .partition(|e| touches(e, ids));
    tree.edges = kept;
    report.edges_removed += gone.len();
    let removed_links: HashSet<String> = gone.into_iter().map(|e| e.id).collect();

    let before_fb = tree.feedback_edges.len();
    tree.feedback_edges
        .retain(|fb| !ids.contains(fb.from.as_str()) && !ids.contains(fb.to.as_str()));
    report.edges_removed += before_fb - tree.feedback_edges.len();

    // An NBR branch whose source is removed goes away entirely; otherwise only the branch
    // edges touching a removed node are dropped.
    for branch in std::mem::take(&mut tree.nbr_branches) {
        if ids.contains(branch.source_node.as_str()) {
            report.edges_removed += branch.edges.len();
            report.removed_branches.push(branch.id);
        } else {
            tree.nbr_branches.push(branch);
        }
    }
    for branch in &mut tree.nbr_branches {
        let before = branch.edges.len();
        branch.edges.retain(|e| !touches(e, ids));
        report.edges_removed += before - branch.edges.len();
    }

    // `link` commands do not prune `interior_links`, so it may hold ghost IDs: "emptied"
    // means no live interior link, not an empty list.
    let live: HashSet<&str> = tree.edges.iter().map(|e| e.id.as_str()).collect();
    for mut m in std::mem::take(&mut tree.macro_edges) {
        if ids.contains(m.from.as_str()) || ids.contains(m.to.as_str()) {
            report
                .removed_macros
                .push(RemovedMacro::new(m, MacroRemovalReason::EndpointRemoved));
            continue;
        }
        let before = (m.interior_nodes.len(), m.interior_links.len());
        m.interior_nodes.retain(|n| !ids.contains(n.as_str()));
        m.interior_links.retain(|l| !removed_links.contains(l));
        if before != (m.interior_nodes.len(), m.interior_links.len()) {
            report.changed = true;
            let alive = m.interior_links.iter().any(|l| live.contains(l.as_str()));
            if m.status == MacroEdgeStatus::Overlay && !alive {
                report
                    .removed_macros
                    .push(RemovedMacro::new(m, MacroRemovalReason::InteriorEmptied));
                continue;
            }
        }
        tree.macro_edges.push(m);
    }

    report.changed |= report.edges_removed > 0
        || !report.removed_branches.is_empty()
        || !report.removed_macros.is_empty();
    report
}

/// Whether the edge references any removed node (`to` or any `from`).
fn touches(edge: &Edge, ids: &HashSet<&str>) -> bool {
    ids.contains(edge.to.as_str()) || edge.from.iter().any(|f| ids.contains(f.as_str()))
}

impl RemovedMacro {
    fn new(m: MacroEdge, reason: MacroRemovalReason) -> Self {
        // Stored order is creation order (sequential IDs, appended), hence numeric; a text
        // sort would misplace IDs past 999.
        let assumption_ids = m.assumptions.into_iter().map(|a| a.id).collect();
        Self {
            id: m.id,
            reason,
            status: m.status,
            from: m.from,
            to: m.to,
            assumption_ids,
        }
    }
}

/// Referential integrity violations in every structure of the tree (ADR-016 D-5).
///
/// Reports one `REFERENTIAL_INTEGRITY_VIOLATION` per node reference missing from `pool`, in
/// a fixed order: `nodes` → `edges` → `feedback_edges` → `nbr_branches` → `macro_edges`.
/// Each error carries `tree_id`, `node_id`, `location` and the container ID (`edge_id`,
/// `feedback_id`, `nbr_id` or `macro_link`).
pub fn check_tree_integrity(tree: &Tree, pool: &HashSet<String>) -> Vec<OutputError> {
    let mut errors = Vec::new();
    let mut check = |node: &str, location: &str, container: &[(&str, &str)], detail: String| {
        if pool.contains(node) {
            return;
        }
        let mut e = OutputError::new("REFERENTIAL_INTEGRITY_VIOLATION", detail)
            .with_context("tree_id", tree.id.as_str())
            .with_context("node_id", node)
            .with_context("location", location);
        for (k, v) in container {
            e = e.with_context(*k, *v);
        }
        errors.push(e);
    };

    for n in &tree.nodes {
        let id = n.node_ref.as_str();
        check(
            id,
            "nodes",
            &[],
            format!(
                "Node '{id}' attached to tree '{}' does not exist in pool",
                tree.id
            ),
        );
    }
    for edge in &tree.edges {
        for id in edge.from.iter().chain(std::iter::once(&edge.to)) {
            check(
                id,
                "edges",
                &[("edge_id", &edge.id)],
                format!(
                    "Node '{id}' referenced in edge '{}' does not exist in pool",
                    edge.id
                ),
            );
        }
    }
    for fb in &tree.feedback_edges {
        for id in [&fb.from, &fb.to] {
            check(
                id,
                "feedback_edges",
                &[("feedback_id", &fb.id)],
                format!(
                    "Node '{id}' referenced in feedback edge '{}' does not exist in pool",
                    fb.id
                ),
            );
        }
    }
    for b in &tree.nbr_branches {
        let id = b.source_node.as_str();
        check(
            id,
            "nbr_branches",
            &[("nbr_id", &b.id)],
            format!(
                "Node '{id}' is the source of NBR branch '{}' but does not exist in pool",
                b.id
            ),
        );
        for edge in &b.edges {
            for id in edge.from.iter().chain(std::iter::once(&edge.to)) {
                check(
                    id,
                    "nbr_branches",
                    &[("nbr_id", &b.id), ("edge_id", &edge.id)],
                    format!(
                        "Node '{id}' referenced in edge '{}' of NBR branch '{}' does not exist in pool",
                        edge.id, b.id
                    ),
                );
            }
        }
    }
    for m in &tree.macro_edges {
        let interior = m.interior_nodes.iter();
        for id in [&m.from, &m.to].into_iter().chain(interior) {
            check(
                id,
                "macro_edges",
                &[("macro_link", &m.id)],
                format!(
                    "Node '{id}' referenced in macro edge '{}' does not exist in pool",
                    m.id
                ),
            );
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::link::{
        Assumption, AssumptionStatus, Edge, EdgeStatus, FeedbackEdge, FeedbackLoopType, Logic,
        Operator,
    };
    use crate::tree::{MacroAssumption, MacroEdge, MacroEdgeStatus, NbrBranch};

    fn tree(nodes: &[&str]) -> Tree {
        let refs: Vec<_> = nodes
            .iter()
            .map(|n| serde_json::json!({"ref": n, "role": null}))
            .collect();
        let raw = serde_json::json!({
            "id": "tree-crt-a", "name": "a", "type": "crt", "logic": "sufficiency", "nodes": refs
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
            operator: if from.len() > 1 {
                Operator::And
            } else {
                Operator::Single
            },
            weight: None,
            status: EdgeStatus::Active,
            logic: Logic::Sufficiency,
            assumptions: vec![],
        }
    }

    fn feedback(id: &str, from: &str, to: &str) -> FeedbackEdge {
        FeedbackEdge {
            id: id.to_string(),
            from: from.to_string(),
            to: to.to_string(),
            loop_type: FeedbackLoopType::Positive,
            label: None,
        }
    }

    fn overlay(id: &str, from: &str, to: &str, interior: &[&str], links: &[&str]) -> MacroEdge {
        MacroEdge {
            id: id.to_string(),
            from: from.to_string(),
            to: to.to_string(),
            label: "long arrow".to_string(),
            interior_nodes: interior.iter().map(|s| s.to_string()).collect(),
            interior_links: links.iter().map(|s| s.to_string()).collect(),
            status: MacroEdgeStatus::Overlay,
            assumptions: vec![],
        }
    }

    fn masm(id: &str) -> MacroAssumption {
        MacroAssumption {
            id: id.to_string(),
            status: AssumptionStatus::Valid,
            text: "summary".to_string(),
            projection_refs: vec![],
        }
    }

    fn node_ids(t: &Tree) -> Vec<&str> {
        t.nodes.iter().map(|n| n.node_ref.as_str()).collect()
    }

    fn json(t: &Tree) -> String {
        match serde_json::to_string(t) {
            Ok(s) => s,
            Err(e) => panic!("tree must serialize: {e}"),
        }
    }

    fn ids<'a>(list: &[&'a str]) -> HashSet<&'a str> {
        list.iter().copied().collect()
    }

    fn pool(list: &[&str]) -> HashSet<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    fn ctx<'a>(e: &'a OutputError, key: &str) -> Option<&'a str> {
        e.context.get(key).and_then(|v| v.as_str())
    }

    // ---- redirect_split ----------------------------------------------------------

    // U1: empty tree, or tree without the node => false, identical tree.
    #[test]
    fn u1_redirect_without_node_is_noop() {
        let mut empty = tree(&[]);
        let before = json(&empty);
        assert!(!redirect_split(&mut empty, "X", "X1", "X2"));
        assert_eq!(json(&empty), before);

        let mut t = tree(&["A", "B"]);
        t.edges.push(edge("LINK-001", &["A"], "B"));
        t.feedback_edges.push(feedback("FB-001", "B", "A"));
        t.macro_edges
            .push(overlay("MACRO-001", "A", "B", &[], &["LINK-001"]));
        let before = json(&t);
        assert!(!redirect_split(&mut t, "X", "X1", "X2"));
        assert_eq!(json(&t), before);
    }

    // U2: in-place replacement in nodes[] and interior_nodes.
    #[test]
    fn u2_children_replace_original_in_position() {
        let mut t = tree(&["A", "X", "B"]);
        t.edges.push(edge("LINK-001", &["A"], "X"));
        t.edges.push(edge("LINK-002", &["X"], "B"));
        t.macro_edges.push(overlay(
            "MACRO-001",
            "A",
            "B",
            &["X"],
            &["LINK-001", "LINK-002"],
        ));
        assert!(redirect_split(&mut t, "X", "X1", "X2"));
        assert_eq!(node_ids(&t), vec!["A", "X1", "X2", "B"]);
        assert!(t.nodes.iter().all(|n| n.role.is_none()));
        assert_eq!(t.macro_edges[0].interior_nodes, vec!["X1", "X2"]);
        assert_eq!(t.edges[0].to, "X1");
        assert_eq!(t.edges[1].from, vec!["X2"]);
    }

    // U2b: a child already listed is not duplicated and keeps its own slot, before or after
    // the original; only the missing children take the original's slot (D2, option C).
    #[test]
    fn u2b_children_not_duplicated() {
        let mut t = tree(&["X1", "X", "B"]);
        t.macro_edges
            .push(overlay("MACRO-001", "A", "B", &["X", "X1"], &[]));
        assert!(redirect_split(&mut t, "X", "X1", "X2"));
        assert_eq!(node_ids(&t), vec!["X1", "X2", "B"]);
        assert_eq!(t.macro_edges[0].interior_nodes, vec!["X2", "X1"]);
    }

    // U2c: an existing child listed after the original keeps its role (no data loss).
    #[test]
    fn u2c_existing_child_after_original_keeps_role() {
        let mut t = tree(&["X", "X1", "B"]);
        t.nodes[1].role = Some("objective".into());
        assert!(redirect_split(&mut t, "X", "X1", "X2"));
        assert_eq!(node_ids(&t), vec!["X2", "X1", "B"]);
        assert_eq!(t.nodes[1].role.as_deref(), Some("objective"));
    }

    // U2d: both children already listed => only the original leaves; repeated originals too.
    #[test]
    fn u2d_both_children_present_and_repeated_original() {
        let mut t = tree(&["X1", "X", "X2", "X"]);
        assert!(redirect_split(&mut t, "X", "X1", "X2"));
        assert_eq!(node_ids(&t), vec!["X1", "X2"]);
    }

    // U3: AND edge keeps the other cause, operator and assumptions.
    #[test]
    fn u3_and_edge_keeps_operator_and_assumptions() {
        let mut t = tree(&["X", "Y", "Z"]);
        let mut e = edge("LINK-001", &["X", "Y"], "Z");
        e.assumptions.push(Assumption {
            id: "ASM-001".into(),
            status: AssumptionStatus::Valid,
            text: "because".into(),
        });
        t.edges.push(e);
        assert!(redirect_split(&mut t, "X", "X1", "X2"));
        let e = &t.edges[0];
        assert_eq!(e.from, vec!["X2", "Y"]);
        assert_eq!(e.to, "Z");
        assert_eq!(e.operator, Operator::And);
        assert_eq!(e.assumptions.len(), 1);
        assert_eq!(e.assumptions[0].id, "ASM-001");
    }

    // D-2 in every structure: feedback, NBR (source + edges), macro endpoints.
    #[test]
    fn redirect_covers_feedback_nbr_and_macro_endpoints() {
        let mut t = tree(&["X", "Y"]);
        t.feedback_edges.push(feedback("FB-001", "X", "Y"));
        t.feedback_edges.push(feedback("FB-002", "Y", "X"));
        t.nbr_branches.push(NbrBranch {
            id: "NBR-001".into(),
            source_node: "X".into(),
            edges: vec![
                edge("LINK-010", &["X"], "N1"),
                edge("LINK-011", &["N1"], "X"),
            ],
            trim_injection: None,
        });
        t.macro_edges.push(overlay("MACRO-001", "X", "Y", &[], &[]));
        t.macro_edges.push(overlay("MACRO-002", "Y", "X", &[], &[]));
        assert!(redirect_split(&mut t, "X", "X1", "X2"));

        assert_eq!(
            (
                t.feedback_edges[0].from.as_str(),
                t.feedback_edges[0].to.as_str()
            ),
            ("X2", "Y")
        );
        assert_eq!(
            (
                t.feedback_edges[1].from.as_str(),
                t.feedback_edges[1].to.as_str()
            ),
            ("Y", "X1")
        );
        let b = &t.nbr_branches[0];
        assert_eq!(b.source_node, "X1");
        assert_eq!(b.edges[0].from, vec!["X2"]);
        assert_eq!(b.edges[1].to, "X1");
        assert_eq!(t.macro_edges[0].from, "X2");
        assert_eq!(t.macro_edges[1].to, "X1");
        assert!(!json(&t).contains("\"X\""));
    }

    // A node that only lives in an NBR branch (not in nodes[]) is still redirected.
    #[test]
    fn redirect_branch_only_node() {
        let mut t = tree(&["A"]);
        t.nbr_branches.push(NbrBranch {
            id: "NBR-001".into(),
            source_node: "A".into(),
            edges: vec![edge("LINK-010", &["A"], "X"), edge("LINK-011", &["X"], "B")],
            trim_injection: None,
        });
        assert!(redirect_split(&mut t, "X", "X1", "X2"));
        assert_eq!(node_ids(&t), vec!["A"]);
        assert_eq!(t.nbr_branches[0].edges[0].to, "X1");
        assert_eq!(t.nbr_branches[0].edges[1].from, vec!["X2"]);
    }

    // ---- prune_removed -----------------------------------------------------------

    fn diamond() -> Tree {
        // A→B→E, A→C→E collapsed into MACRO-001 (A ⇒ E).
        let mut t = tree(&["A", "B", "C", "E"]);
        t.edges.push(edge("LINK-001", &["A"], "B"));
        t.edges.push(edge("LINK-002", &["B"], "E"));
        t.edges.push(edge("LINK-003", &["A"], "C"));
        t.edges.push(edge("LINK-004", &["C"], "E"));
        t.macro_edges.push(overlay(
            "MACRO-001",
            "A",
            "E",
            &["B", "C"],
            &["LINK-001", "LINK-002", "LINK-003", "LINK-004"],
        ));
        t
    }

    #[test]
    fn prune_without_matches_is_noop() {
        let mut t = diamond();
        let before = json(&t);
        let r = prune_removed(&mut t, &ids(&["ZZZ"]));
        assert_eq!(r, PruneReport::default());
        assert_eq!(json(&t), before);
    }

    // U4: endpoint and interior removed in the same batch => one RemovedMacro, endpoint wins.
    #[test]
    fn u4_endpoint_takes_priority_and_is_reported_once() {
        let mut t = diamond();
        // Creation order across the 999 boundary: stored order must be kept, not re-sorted
        // as text (which would put MASM-1000 before MASM-998).
        t.macro_edges[0].assumptions = vec![masm("MASM-998"), masm("MASM-1000")];
        let r = prune_removed(&mut t, &ids(&["A", "B", "E"]));
        assert_eq!(
            r.removed_macros,
            vec![RemovedMacro {
                id: "MACRO-001".into(),
                reason: MacroRemovalReason::EndpointRemoved,
                status: MacroEdgeStatus::Overlay,
                from: "A".into(),
                to: "E".into(),
                assumption_ids: vec!["MASM-998".into(), "MASM-1000".into()],
            }]
        );
        assert!(t.macro_edges.is_empty());
        assert!(r.changed);
    }

    // U5: removing one diamond branch trims the macro, which survives.
    #[test]
    fn u5_interior_trim_keeps_macro() {
        let mut t = diamond();
        let r = prune_removed(&mut t, &ids(&["B"]));
        assert!(r.changed);
        assert!(r.removed_macros.is_empty());
        assert_eq!(r.edges_removed, 2);
        let m = &t.macro_edges[0];
        assert_eq!(m.interior_nodes, vec!["C"]);
        assert_eq!(m.interior_links, vec!["LINK-003", "LINK-004"]);
        assert_eq!(node_ids(&t), vec!["A", "C", "E"]);
    }

    // M4 (unit): linear overlay A→B→E, removing B empties the interior.
    #[test]
    fn overlay_with_emptied_interior_is_removed() {
        let mut t = tree(&["A", "B", "E"]);
        t.edges.push(edge("LINK-001", &["A"], "B"));
        t.edges.push(edge("LINK-002", &["B"], "E"));
        let mut m = overlay("MACRO-001", "A", "E", &["B"], &["LINK-001", "LINK-002"]);
        m.assumptions.push(masm("MASM-001"));
        t.macro_edges.push(m);
        let r = prune_removed(&mut t, &ids(&["B"]));
        assert_eq!(r.removed_macros.len(), 1);
        assert_eq!(
            r.removed_macros[0].reason,
            MacroRemovalReason::InteriorEmptied
        );
        assert_eq!(r.removed_macros[0].assumption_ids, vec!["MASM-001"]);
        assert!(t.macro_edges.is_empty());
    }

    // D3/F3: `link` commands leave ghost IDs in `interior_links`. A macro this rm touched
    // with no live interior link left is removed, ghosts notwithstanding.
    #[test]
    fn d3_touched_overlay_with_only_ghost_links_is_removed() {
        let mut t = tree(&["A", "B", "E"]);
        // LINK-001 (A→B) was disconnected earlier: gone from edges, still in the macro.
        t.edges.push(edge("LINK-002", &["B"], "E"));
        t.macro_edges.push(overlay(
            "MACRO-001",
            "A",
            "E",
            &["B"],
            &["LINK-001", "LINK-002"],
        ));
        let r = prune_removed(&mut t, &ids(&["B"]));
        assert_eq!(r.removed_macros.len(), 1);
        assert_eq!(
            r.removed_macros[0].reason,
            MacroRemovalReason::InteriorEmptied
        );
        assert!(t.macro_edges.is_empty());
    }

    // D3: a touched overlay that keeps one live link survives; its ghosts are not cleaned.
    #[test]
    fn d3_touched_overlay_with_a_live_link_survives_and_keeps_ghosts() {
        let mut t = diamond();
        // LINK-001 (A→B) disconnected earlier: ghost in the macro.
        t.edges.retain(|e| e.id != "LINK-001");
        let r = prune_removed(&mut t, &ids(&["C"]));
        assert!(r.removed_macros.is_empty());
        assert_eq!(
            t.macro_edges[0].interior_links,
            vec!["LINK-001", "LINK-002"]
        );
    }

    // D3: an overlay that was already empty (legacy/hand-made) is not touched by an
    // unrelated rm.
    #[test]
    fn d3_untouched_empty_overlay_is_left_alone() {
        let mut t = tree(&["A", "E", "Z"]);
        t.macro_edges.push(overlay("MACRO-001", "A", "E", &[], &[]));
        // Untouched overlay made only of ghosts: also left alone.
        t.macro_edges
            .push(overlay("MACRO-002", "A", "E", &[], &["LINK-404"]));
        let before: Vec<String> = t
            .macro_edges
            .iter()
            .map(|m| serde_json::to_string(m).unwrap_or_default())
            .collect();
        let r = prune_removed(&mut t, &ids(&["Z"]));
        assert!(r.removed_macros.is_empty());
        let after: Vec<String> = t
            .macro_edges
            .iter()
            .map(|m| serde_json::to_string(m).unwrap_or_default())
            .collect();
        assert_eq!(after, before);
    }

    // D3/F2: interior node removed while `interior_links` was already empty: the rm touched
    // it and nothing live remains, so it goes (rule and code say the same thing).
    #[test]
    fn d3_touched_overlay_without_links_is_removed() {
        let mut t = tree(&["A", "B", "E"]);
        t.macro_edges
            .push(overlay("MACRO-001", "A", "E", &["B"], &[]));
        let r = prune_removed(&mut t, &ids(&["B"]));
        assert_eq!(r.removed_macros.len(), 1);
        assert_eq!(
            r.removed_macros[0].reason,
            MacroRemovalReason::InteriorEmptied
        );
    }

    // A Reservation (empty interior by construction) only goes away by its endpoint.
    #[test]
    fn reservation_only_removed_by_endpoint() {
        let mut t = tree(&["A", "B", "E"]);
        let mut r1 = overlay("MACRO-001", "A", "E", &[], &[]);
        r1.status = MacroEdgeStatus::Reservation;
        t.macro_edges.push(r1);
        let r = prune_removed(&mut t, &ids(&["B"]));
        assert!(r.removed_macros.is_empty());
        assert_eq!(t.macro_edges.len(), 1);

        let r = prune_removed(&mut t, &ids(&["E"]));
        assert_eq!(r.removed_macros.len(), 1);
        assert_eq!(
            r.removed_macros[0].reason,
            MacroRemovalReason::EndpointRemoved
        );
        assert_eq!(r.removed_macros[0].status, MacroEdgeStatus::Reservation);
        assert!(t.macro_edges.is_empty());
    }

    // Same semantics as v0.4.0 for trunk, feedback and NBR; unrelated macros untouched.
    #[test]
    fn prune_trunk_feedback_nbr_and_counts() {
        let mut t = tree(&["A", "B", "C", "D"]);
        t.edges.push(edge("LINK-001", &["A", "B"], "C"));
        t.edges.push(edge("LINK-002", &["C"], "D"));
        t.feedback_edges.push(feedback("FB-001", "D", "B"));
        t.feedback_edges.push(feedback("FB-002", "D", "A"));
        t.nbr_branches.push(NbrBranch {
            id: "NBR-001".into(),
            source_node: "B".into(),
            edges: vec![
                edge("LINK-010", &["B"], "N1"),
                edge("LINK-011", &["N1"], "N2"),
            ],
            trim_injection: None,
        });
        t.nbr_branches.push(NbrBranch {
            id: "NBR-002".into(),
            source_node: "C".into(),
            edges: vec![
                edge("LINK-020", &["C"], "B"),
                edge("LINK-021", &["C"], "N3"),
            ],
            trim_injection: None,
        });
        t.macro_edges
            .push(overlay("MACRO-009", "C", "D", &[], &["LINK-002"]));
        let r = prune_removed(&mut t, &ids(&["B"]));

        // trunk LINK-001 + FB-001 + NBR-001's 2 edges + LINK-020
        assert_eq!(r.edges_removed, 5);
        assert_eq!(r.removed_branches, vec!["NBR-001"]);
        assert!(r.removed_macros.is_empty());
        assert!(r.changed);
        assert_eq!(node_ids(&t), vec!["A", "C", "D"]);
        assert_eq!(t.edges.len(), 1);
        assert_eq!(t.feedback_edges.len(), 1);
        assert_eq!(t.nbr_branches.len(), 1);
        assert_eq!(t.nbr_branches[0].edges.len(), 1);
        assert_eq!(t.macro_edges[0].interior_links, vec!["LINK-002"]);
    }

    // ---- check_tree_integrity ----------------------------------------------------

    // U6: one failure per structure, fixed order, correct location and container.
    #[test]
    fn u6_one_violation_per_structure_in_fixed_order() {
        let mut t = tree(&["A", "G1"]);
        t.edges.push(edge("LINK-001", &["A"], "G2"));
        t.feedback_edges.push(feedback("FB-001", "G3", "A"));
        t.nbr_branches.push(NbrBranch {
            id: "NBR-001".into(),
            source_node: "G4".into(),
            edges: vec![edge("LINK-010", &["A"], "G5")],
            trim_injection: None,
        });
        t.macro_edges
            .push(overlay("MACRO-001", "G6", "G7", &["G8"], &[]));

        let errs = check_tree_integrity(&t, &pool(&["A"]));
        let got: Vec<_> = errs
            .iter()
            .map(|e| {
                (
                    e.code.as_str(),
                    ctx(e, "node_id"),
                    ctx(e, "location"),
                    ctx(e, "tree_id"),
                )
            })
            .collect();
        let v = "REFERENTIAL_INTEGRITY_VIOLATION";
        let tid = Some("tree-crt-a");
        assert_eq!(
            got,
            vec![
                (v, Some("G1"), Some("nodes"), tid),
                (v, Some("G2"), Some("edges"), tid),
                (v, Some("G3"), Some("feedback_edges"), tid),
                (v, Some("G4"), Some("nbr_branches"), tid),
                (v, Some("G5"), Some("nbr_branches"), tid),
                (v, Some("G6"), Some("macro_edges"), tid),
                (v, Some("G7"), Some("macro_edges"), tid),
                (v, Some("G8"), Some("macro_edges"), tid),
            ]
        );
        assert_eq!(ctx(&errs[1], "edge_id"), Some("LINK-001"));
        assert_eq!(ctx(&errs[2], "feedback_id"), Some("FB-001"));
        assert_eq!(ctx(&errs[3], "nbr_id"), Some("NBR-001"));
        assert_eq!(ctx(&errs[4], "nbr_id"), Some("NBR-001"));
        assert_eq!(ctx(&errs[4], "edge_id"), Some("LINK-010"));
        for e in &errs[5..] {
            assert_eq!(ctx(e, "macro_link"), Some("MACRO-001"));
        }

        let all = pool(&["A", "G1", "G2", "G3", "G4", "G5", "G6", "G7", "G8"]);
        assert!(check_tree_integrity(&t, &all).is_empty());
    }

    // Trunk edges keep the v0.4.0 detail text (contract stability).
    #[test]
    fn trunk_edge_detail_is_unchanged() {
        let mut t = tree(&["A"]);
        t.edges.push(edge("LINK-001", &["A"], "MISSING"));
        let errs = check_tree_integrity(&t, &pool(&["A"]));
        assert_eq!(errs.len(), 1);
        assert_eq!(
            errs[0].detail,
            "Node 'MISSING' referenced in edge 'LINK-001' does not exist in pool"
        );
    }

    // Wire names are contract (`MACRO_EDGE_REMOVED.reason`); serde delegates to `as_str`.
    #[test]
    fn reason_wire_names_are_pinned() {
        for (r, wire) in [
            (MacroRemovalReason::EndpointRemoved, "endpoint_removed"),
            (MacroRemovalReason::InteriorEmptied, "interior_emptied"),
        ] {
            assert_eq!(r.as_str(), wire);
            assert_eq!(
                serde_json::to_value(r).ok(),
                Some(serde_json::Value::String(wire.to_string()))
            );
        }
    }
}
