use crate::errors::LtpError;
use crate::output::{OutputError, OutputWarning};
use crate::storage::Storage;
use crate::tree::Tree;

/// Information about a resolved target entity in the graph.
#[derive(Debug, Clone)]
pub struct ResolvedTarget {
    pub id: String,
    pub label: Option<String>,
    pub target_type: String,
}

/// Why a target could not be resolved either way (PLAN_v060 D-7): part of the
/// graph it could live in was unreadable and it was not found in the rest.
#[derive(Debug)]
pub enum ResolveError {
    /// The trees directory could not be listed.
    TreesUnlisted(LtpError),
    /// A tree could not be read (the first one, in ID order).
    TreeUnreadable {
        /// ID of the unreadable tree.
        tree_id: String,
        /// Underlying load error.
        source: LtpError,
    },
    /// The target node exists on disk but could not be read.
    NodeUnreadable {
        /// ID of the unreadable node.
        node_id: String,
        /// Underlying load error.
        source: LtpError,
    },
}

impl ResolveError {
    /// Warning for read-only commands (D-K5: warn, never block).
    pub fn warning(&self) -> OutputWarning {
        match self {
            Self::TreesUnlisted(e) => OutputWarning::new("IO_ERROR", e.to_string()),
            Self::TreeUnreadable { tree_id, source } => OutputWarning::new(
                "TREE_LOAD_ERROR",
                format!("Failed to load {tree_id}: {source}"),
            )
            .with_context("tree_id", tree_id.as_str()),
            Self::NodeUnreadable { node_id, source } => OutputWarning::new(
                "NODE_UNREADABLE",
                format!("Node '{node_id}' cannot be loaded: {source}"),
            )
            .with_context("node_id", node_id.as_str()),
        }
    }

    /// `IO_ERROR` for commands that would write on an unverified target
    /// (fail-closed, as ADR-016 D-4).
    pub fn error(&self) -> OutputError {
        match self {
            Self::TreesUnlisted(e) => OutputError::new("IO_ERROR", e.to_string()),
            Self::TreeUnreadable { tree_id, source } => OutputError::new(
                "IO_ERROR",
                format!("Cannot verify the target: failed to load {tree_id}: {source}"),
            )
            .with_context("tree_id", tree_id.as_str()),
            Self::NodeUnreadable { node_id, source } => OutputError::new(
                "IO_ERROR",
                format!("Cannot verify the target: node '{node_id}' cannot be loaded: {source}"),
            )
            .with_context("node_id", node_id.as_str()),
        }
    }
}

/// Attempts to resolve a target ID against the workspace graph.
///
/// `Ok(None)` means it does not exist: everything was read and it was not
/// there. `Err` means it was not found in the readable part and some tree (or
/// the node) could not be read. A target found in a readable tree is
/// `Ok(Some)` even if another tree is unreadable.
///
/// Resolution order:
/// - MACRO-XXX: always NOT FOUND (macro_edges are not standalone entities)
/// - LINK-XXX: search all trees' edges (trunk + nbr_branches)
/// - FB-XXX: search all trees' feedback_edges
/// - ASM-XXX: search all trees' edges' assumptions (trunk + nbr_branches)
/// - Anything else: treated as a node ID, checked in node pool
pub fn resolve_target(
    storage: &dyn Storage,
    target: &str,
) -> Result<Option<ResolvedTarget>, ResolveError> {
    if target.starts_with("MACRO-") {
        return Ok(None);
    }

    if target.starts_with("LINK-") {
        return search_trees(storage, |tree| find_edge(tree, target));
    }

    if target.starts_with("FB-") {
        return search_trees(storage, |tree| find_feedback_edge(tree, target));
    }

    if target.starts_with("ASM-") {
        return search_trees(storage, |tree| find_assumption(tree, target));
    }

    resolve_node(storage, target)
}

fn resolve_node(storage: &dyn Storage, id: &str) -> Result<Option<ResolvedTarget>, ResolveError> {
    match storage.load_node(id) {
        Ok(node) => Ok(Some(ResolvedTarget {
            id: id.to_string(),
            label: Some(node.label),
            target_type: "node".to_string(),
        })),
        Err(LtpError::NodeNotFound(_)) => Ok(None),
        Err(source) => Err(ResolveError::NodeUnreadable {
            node_id: id.to_string(),
            source,
        }),
    }
}

/// Runs `find` over every tree, in ID order. A tree that vanished between
/// listing and loading is absent; any other load failure is remembered and
/// reported only if no readable tree holds the target.
fn search_trees(
    storage: &dyn Storage,
    find: impl Fn(&Tree) -> Option<ResolvedTarget>,
) -> Result<Option<ResolvedTarget>, ResolveError> {
    let tree_ids = storage
        .list_tree_ids()
        .map_err(ResolveError::TreesUnlisted)?;
    let mut unreadable = None;
    for tree_id in tree_ids {
        match storage.load_tree(&tree_id) {
            Ok(tree) => {
                if let Some(found) = find(&tree) {
                    return Ok(Some(found));
                }
            }
            Err(LtpError::TreeNotFound(_)) => {}
            Err(source) => {
                unreadable.get_or_insert(ResolveError::TreeUnreadable { tree_id, source });
            }
        }
    }
    match unreadable {
        Some(e) => Err(e),
        None => Ok(None),
    }
}

fn find_edge(tree: &Tree, id: &str) -> Option<ResolvedTarget> {
    tree.edges
        .iter()
        .chain(tree.nbr_branches.iter().flat_map(|b| &b.edges))
        .find(|edge| edge.id == id)
        .map(|edge| ResolvedTarget {
            id: id.to_string(),
            label: Some(format!("{} -> {}", edge.from.join("+"), edge.to)),
            target_type: "edge".to_string(),
        })
}

fn find_feedback_edge(tree: &Tree, id: &str) -> Option<ResolvedTarget> {
    tree.feedback_edges
        .iter()
        .find(|fb| fb.id == id)
        .map(|fb| ResolvedTarget {
            id: id.to_string(),
            label: Some(
                fb.label
                    .clone()
                    .unwrap_or_else(|| format!("{} -> {}", fb.from, fb.to)),
            ),
            target_type: "feedback_edge".to_string(),
        })
}

fn find_assumption(tree: &Tree, id: &str) -> Option<ResolvedTarget> {
    tree.edges
        .iter()
        .chain(tree.nbr_branches.iter().flat_map(|b| &b.edges))
        .flat_map(|edge| &edge.assumptions)
        .find(|asm| asm.id == id)
        .map(|asm| ResolvedTarget {
            id: id.to_string(),
            label: Some(asm.text.clone()),
            target_type: "assumption".to_string(),
        })
}

/// Checks whether a target ID exists in the graph without resolving label.
/// `Err` when it cannot be told (see [`resolve_target`]).
pub fn target_exists(storage: &dyn Storage, target: &str) -> Result<bool, ResolveError> {
    resolve_target(storage, target).map(|r| r.is_some())
}
