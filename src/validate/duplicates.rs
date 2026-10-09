//! Inherited duplicate entity IDs (PLAN_v060 D-8).
//!
//! Before v0.5.2 `tree clone` copied assumption and feedback IDs, and
//! `link dissolve` spread one assumption over several edges. Those duplicates
//! are reported as warnings, never errors: the clone ones have no repair
//! command, and an error would keep such a workspace red forever.

use std::collections::BTreeMap;

use serde_json::{json, Value};

use crate::link::types::Edge;
use crate::output::OutputWarning;
use crate::tree::Tree;

/// Where an ID appears: tree and typed path inside it (e.g. `edges[0].assumptions[1]`).
#[derive(Debug)]
struct Occurrence<'a> {
    tree_id: &'a str,
    location: String,
}

/// One `DUPLICATE_ENTITY_ID` warning per ID that appears more than once in
/// `trees`, ordered by ID; occurrences in tree order, then traversal order.
///
/// `trees` must be in tree-ID order and hold only readable trees (an
/// unreadable one is reported by the caller, never treated as empty). With
/// `scope`, only IDs present in that tree are reported, but their occurrences
/// span the whole workspace.
pub fn check_duplicates(trees: &[Tree], scope: Option<&str>) -> Vec<OutputWarning> {
    let mut seen: BTreeMap<&str, Vec<Occurrence<'_>>> = BTreeMap::new();
    for tree in trees {
        visit_tree(tree, &mut |id, location| {
            seen.entry(id).or_default().push(Occurrence {
                tree_id: &tree.id,
                location,
            });
        });
    }
    seen.into_iter()
        .filter(|(_, occ)| occ.len() > 1)
        .filter(|(_, occ)| scope.is_none_or(|t| occ.iter().any(|o| o.tree_id == t)))
        .map(|(id, occ)| duplicate_warning(id, &occ))
        .collect()
}

/// Calls `f(id, location)` for every entity with its own ID inside `tree`.
fn visit_tree<'t>(tree: &'t Tree, f: &mut impl FnMut(&'t str, String)) {
    visit_edges(&tree.edges, "edges", f);
    for (i, fb) in tree.feedback_edges.iter().enumerate() {
        f(&fb.id, format!("feedback_edges[{i}]"));
    }
    for (i, nbr) in tree.nbr_branches.iter().enumerate() {
        let base = format!("nbr_branches[{i}]");
        visit_edges(&nbr.edges, &format!("{base}.edges"), f);
        f(&nbr.id, base);
    }
    for (i, macro_edge) in tree.macro_edges.iter().enumerate() {
        let base = format!("macro_edges[{i}]");
        for (j, asm) in macro_edge.assumptions.iter().enumerate() {
            f(&asm.id, format!("{base}.assumptions[{j}]"));
        }
        f(&macro_edge.id, base);
    }
}

fn visit_edges<'t>(edges: &'t [Edge], base: &str, f: &mut impl FnMut(&'t str, String)) {
    for (i, edge) in edges.iter().enumerate() {
        f(&edge.id, format!("{base}[{i}]"));
        for (j, asm) in edge.assumptions.iter().enumerate() {
            f(&asm.id, format!("{base}[{i}].assumptions[{j}]"));
        }
    }
}

fn duplicate_warning(id: &str, occurrences: &[Occurrence<'_>]) -> OutputWarning {
    let first_tree = occurrences.first().map(|o| o.tree_id);
    let cross_tree = occurrences.iter().any(|o| Some(o.tree_id) != first_tree);
    let within_tree = occurrences
        .windows(2)
        .any(|pair| pair[0].tree_id == pair[1].tree_id);
    let mut repairs = Vec::new();
    if within_tree && id.starts_with("ASM-") {
        repairs.push(format!(
            "within a tree, `assume rm --asm {id}` removes one copy"
        ));
    }
    if cross_tree {
        repairs.push(
            "across trees, `tree rm` the clone and clone it again (edits to the clone are lost)"
                .to_string(),
        );
    }
    repairs.push("or edit the tree JSON".to_string());
    let list: Vec<Value> = occurrences
        .iter()
        .map(|o| json!({ "tree_id": o.tree_id, "location": o.location }))
        .collect();
    OutputWarning::new(
        "DUPLICATE_ENTITY_ID",
        format!(
            "ID '{id}' appears {} times (created by `tree clone`/`link dissolve` before v0.5.2); repair: {}",
            occurrences.len(),
            repairs.join("; ")
        ),
    )
    .with_context("id", id)
    .with_context("occurrences", Value::Array(list))
}
