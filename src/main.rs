use std::process;

use clap::{Parser, Subcommand};
use serde::Serialize;
use tracing_subscriber::EnvFilter;

use ltp_engine::assume::{
    execute_assume_add, execute_assume_edit, execute_assume_list, execute_assume_move,
    execute_assume_rm, execute_invalidate,
};
use ltp_engine::errors::LtpError;
use ltp_engine::history::{
    execute_history_begin_batch, execute_history_check, execute_history_clear,
    execute_history_end_batch, execute_history_invalidate, execute_history_list, execute_redo,
    execute_undo, HistoryManager,
};
use ltp_engine::knowledge::commands::{
    execute_knowledge_add, execute_knowledge_edit, execute_knowledge_inspect,
    execute_knowledge_link, execute_knowledge_list, execute_knowledge_rm, execute_knowledge_unlink,
};
use ltp_engine::knowledge::{Confidence, KnowledgeRelation, KnowledgeStatus, KnowledgeType};
use ltp_engine::link::advanced::{
    execute_link_add_cause, execute_link_dissolve, execute_link_group, execute_link_insert_between,
    execute_link_move, execute_link_reoperator, execute_link_reverse, execute_link_rm_cause,
    execute_link_split,
};
use ltp_engine::link::commands::{
    execute_link_connect, execute_link_disconnect, execute_link_feedback,
};
use ltp_engine::nbr::{execute_nbr_add, execute_nbr_inspect, execute_nbr_list, execute_nbr_rm};
use ltp_engine::node::commands::{
    execute_node_add, execute_node_edit, execute_node_inspect, execute_node_list, execute_node_rm,
    execute_node_search, execute_node_split,
};
use ltp_engine::output::{CommandOutput, GraphHealth, OutputError};
use ltp_engine::path::{execute_path_collapse, execute_path_explode, execute_path_replace};
use ltp_engine::storage::Storage;
use ltp_engine::trace::{execute_link_find, execute_link_inspect, execute_trace};
use ltp_engine::tree::commands::{
    execute_tree_attach, execute_tree_clone, execute_tree_detach, execute_tree_diff,
    execute_tree_list, execute_tree_new, execute_tree_rm, execute_tree_walk,
};
use ltp_engine::tree::Tree;
use ltp_engine::validate::{check_dag, execute_validate};
use ltp_engine::workspace::FsStorage;

#[derive(Parser)]
#[command(name = "ltp")]
#[command(about = "Deterministic engine for Dettmer's Logical Thinking Process")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Human-readable output (default is JSON)
    #[arg(long, global = true)]
    human: bool,

    /// Simulate the operation without writing to disk
    #[arg(long, global = true)]
    dry_run: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new LTP workspace
    Init {
        /// Workspace name
        #[arg(long)]
        name: Option<String>,
    },

    /// Show workspace health status
    Status,

    /// Manage nodes in the global pool
    Node {
        #[command(subcommand)]
        action: NodeAction,
    },

    /// Manage topological views (trees)
    Tree {
        #[command(subcommand)]
        action: TreeAction,
    },

    /// Manage cause-effect links
    Link {
        #[command(subcommand)]
        action: LinkAction,
    },

    /// Manage assumptions on edges
    Assume {
        #[command(subcommand)]
        action: AssumeAction,
    },

    /// Trace upstream/downstream from a node
    Trace {
        /// Node ID to trace from
        node_id: String,

        /// Tree to trace within
        #[arg(long)]
        tree: String,

        /// Direction: upstream or downstream
        #[arg(long)]
        direction: String,

        /// Max depth
        #[arg(long)]
        depth: Option<usize>,

        /// Exclude feedback edges
        #[arg(long)]
        no_feedback: bool,

        /// Include NBR edges
        #[arg(long)]
        nbr: bool,

        /// Include knowledge items linked to each node
        #[arg(long)]
        show_knowledge: bool,
    },

    /// Validate workspace or specific tree
    Validate {
        /// Specific tree to validate (validates all if omitted)
        #[arg(long)]
        tree: Option<String>,
    },

    /// Path abstraction operations
    Path {
        #[command(subcommand)]
        action: PathAction,
    },

    /// Negative Branch Reservations
    Nbr {
        #[command(subcommand)]
        action: NbrAction,
    },

    /// Undo last operation
    Undo {
        #[arg(long)]
        dry_run: bool,
    },

    /// Redo last undone operation
    Redo {
        #[arg(long)]
        dry_run: bool,
    },

    /// View and manage operation history
    History {
        #[command(subcommand)]
        action: Option<HistoryAction>,

        /// Show last N entries
        #[arg(long)]
        last: Option<usize>,
    },

    /// Invalidate a link via assumption
    Invalidate {
        #[arg(long)]
        tree: String,

        #[arg(long)]
        link: String,

        #[arg(long)]
        asm: String,

        /// Label for a new injection node
        #[arg(long)]
        injection: Option<String>,
    },

    /// Manage knowledge items in the epistemic pool
    Knowledge {
        #[command(subcommand)]
        action: KnowledgeAction,
    },
}

#[derive(Subcommand)]
enum NodeAction {
    Add {
        label: String,
        #[arg(long)]
        r#type: String,
        #[arg(long, value_delimiter = ',')]
        tags: Option<Vec<String>>,
        #[arg(long)]
        observable: Option<bool>,
        #[arg(long)]
        epistemic: Option<String>,
    },
    Edit {
        id: String,
        #[arg(long)]
        label: Option<String>,
        #[arg(long)]
        add_tag: Option<String>,
        #[arg(long)]
        rm_tag: Option<String>,
        #[arg(long)]
        observable: Option<bool>,
        #[arg(long)]
        epistemic: Option<String>,
    },
    Rm {
        #[arg(value_delimiter = ',')]
        ids: Vec<String>,
        #[arg(long)]
        force: bool,
    },
    Inspect {
        id: String,
    },
    List {
        #[arg(long)]
        tree: Option<String>,
        #[arg(long, value_delimiter = ',')]
        r#type: Option<Vec<String>>,
        #[arg(long, value_delimiter = ',')]
        status: Option<Vec<String>>,
        #[arg(long)]
        epistemic: Option<String>,
    },
    Search {
        #[arg(long)]
        tree: Option<String>,
        #[arg(long)]
        query: String,
    },
    Split {
        id: String,
        #[arg(long, num_args = 2)]
        into: Vec<String>,
        #[arg(long)]
        tree: String,
    },
}

#[derive(Subcommand)]
enum TreeAction {
    New {
        r#type: String,
        name: String,
    },
    List,
    Rm {
        tree_id: String,
    },
    Attach {
        #[arg(long)]
        tree: String,
        #[arg(long)]
        node: String,
        #[arg(long)]
        role: Option<String>,
    },
    Detach {
        #[arg(long)]
        tree: String,
        #[arg(long)]
        node: String,
    },
    Clone {
        tree_id: String,
        #[arg(long)]
        name: String,
    },
    Diff {
        tree_a: String,
        tree_b: String,
    },
    Walk {
        tree_id: String,
        #[arg(long, default_value = "topological")]
        order: String,
        #[arg(long)]
        show_origin: bool,
        #[arg(long)]
        expand_nbr: bool,
        /// Include knowledge counts per node
        #[arg(long)]
        show_knowledge: bool,
    },
}

#[derive(Subcommand)]
enum LinkAction {
    Connect {
        #[arg(long)]
        tree: String,
        #[arg(long, value_delimiter = ',')]
        from: Vec<String>,
        #[arg(long, value_delimiter = ',')]
        to: Vec<String>,
        #[arg(long)]
        operator: Option<String>,
        #[arg(long)]
        weight: Option<f64>,
        /// Target NBR branch for the edge (creates edge inside NBR, not trunk)
        #[arg(long)]
        nbr: Option<String>,
    },
    Disconnect {
        #[arg(long)]
        tree: String,
        #[arg(long, value_delimiter = ',')]
        links: Vec<String>,
    },
    Feedback {
        #[arg(long)]
        tree: String,
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        #[arg(long)]
        r#type: String,
        #[arg(long)]
        label: Option<String>,
    },
    Inspect {
        link_id: String,
        #[arg(long)]
        tree: String,
    },
    Find {
        #[arg(long)]
        tree: String,
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
    },
    Reverse {
        #[arg(long)]
        tree: String,
        #[arg(long)]
        link: String,
        #[arg(long)]
        force: bool,
    },
    Move {
        #[arg(long)]
        tree: String,
        #[arg(long)]
        link: String,
        #[arg(long)]
        new_from: Option<String>,
        #[arg(long)]
        new_to: Option<String>,
    },
    InsertBetween {
        #[arg(long)]
        tree: String,
        #[arg(long)]
        link: String,
        #[arg(long)]
        node: String,
        #[arg(long)]
        insert_after_cause: Option<String>,
        #[arg(long)]
        insert_before_effect: bool,
    },
    Group {
        #[arg(long)]
        tree: String,
        #[arg(long, value_delimiter = ',')]
        links: Vec<String>,
        #[arg(long)]
        operator: String,
    },
    Dissolve {
        #[arg(long)]
        tree: String,
        #[arg(long)]
        link: String,
    },
    Split {
        #[arg(long)]
        tree: String,
        #[arg(long)]
        link: String,
        #[arg(long, value_delimiter = ',')]
        extract: Vec<String>,
    },
    Reoperator {
        #[arg(long)]
        tree: String,
        #[arg(long)]
        link: String,
        #[arg(long)]
        operator: String,
    },
    AddCause {
        #[arg(long)]
        tree: String,
        #[arg(long)]
        link: String,
        #[arg(long)]
        node: String,
        #[arg(long)]
        weight: Option<f64>,
        #[arg(long)]
        promote_to: Option<String>,
    },
    RmCause {
        #[arg(long)]
        tree: String,
        #[arg(long)]
        link: String,
        #[arg(long)]
        node: String,
    },
}

#[derive(Subcommand)]
enum AssumeAction {
    Add {
        #[arg(long)]
        tree: String,
        #[arg(long)]
        link: String,
        #[arg(long)]
        text: String,
    },
    Edit {
        #[arg(long)]
        tree: String,
        #[arg(long)]
        asm: String,
        #[arg(long)]
        text: String,
    },
    Rm {
        #[arg(long)]
        tree: String,
        #[arg(long)]
        asm: String,
    },
    List {
        #[arg(long)]
        tree: String,
        #[arg(long)]
        status: Option<String>,
    },
    Move {
        #[arg(long)]
        tree: String,
        #[arg(long)]
        asm: String,
        #[arg(long)]
        to_link: String,
    },
}

#[derive(Subcommand)]
enum PathAction {
    Collapse {
        #[arg(long)]
        tree: String,
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        #[arg(long)]
        label: String,
    },
    Explode {
        #[arg(long)]
        tree: String,
        #[arg(long)]
        link: String,
        #[arg(long)]
        asm: String,
        #[arg(long)]
        label: String,
    },
    Replace {
        #[arg(long)]
        tree: String,
        #[arg(long)]
        macro_link: String,
        #[arg(long)]
        by_node: String,
    },
}

#[derive(Subcommand)]
enum NbrAction {
    Add {
        #[arg(long)]
        tree: String,
        #[arg(long)]
        source_node: String,
        #[arg(long)]
        trim: Option<String>,
    },
    Rm {
        #[arg(long)]
        tree: String,
        #[arg(long)]
        nbr: String,
    },
    List {
        #[arg(long)]
        tree: String,
    },
    Inspect {
        nbr_id: String,
        #[arg(long)]
        tree: String,
    },
}

#[derive(Subcommand)]
enum HistoryAction {
    Check,
    Invalidate {
        #[arg(long)]
        from: Option<u64>,
    },
    BeginBatch {
        #[arg(long)]
        label: String,
    },
    EndBatch,
    Clear,
}

#[derive(Subcommand)]
enum KnowledgeAction {
    Add {
        label: String,
        #[arg(long)]
        r#type: String,
        #[arg(long)]
        source_uri: Option<String>,
        #[arg(long)]
        source_excerpt: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        confidence: Option<String>,
        #[arg(long, value_delimiter = ',')]
        tags: Option<Vec<String>>,
    },
    Edit {
        id: String,
        #[arg(long)]
        label: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        confidence: Option<String>,
        #[arg(long)]
        source_uri: Option<String>,
        #[arg(long)]
        source_excerpt: Option<String>,
        #[arg(long, value_delimiter = ',')]
        add_tag: Option<Vec<String>>,
        #[arg(long, value_delimiter = ',')]
        rm_tag: Option<Vec<String>>,
    },
    Rm {
        #[arg(value_delimiter = ',')]
        ids: Vec<String>,
    },
    Inspect {
        id: String,
    },
    List {
        #[arg(long)]
        r#type: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        confidence: Option<String>,
        #[arg(long)]
        unlinked: bool,
        #[arg(long)]
        tag: Option<String>,
        #[arg(long)]
        target: Option<String>,
        #[arg(long)]
        relation: Option<String>,
    },
    /// Link a knowledge item to a graph entity
    Link {
        id: String,
        #[arg(long)]
        to: String,
        #[arg(long)]
        relation: String,
    },
    /// Unlink a knowledge item from a graph entity
    Unlink {
        id: String,
        #[arg(long)]
        from: String,
    },
}

// --- Command implementations ---

#[derive(Debug, Serialize)]
struct InitData {
    directories_created: Vec<String>,
    config_file: String,
    counters_file: String,
}

#[derive(Debug, Serialize)]
struct KnowledgeHealthData {
    total: usize,
    unlinked_items: usize,
    contradictions: usize,
    by_status: KnowledgeByStatus,
    epistemic_coverage: EpistemicCoverage,
}

#[derive(Debug, Serialize)]
struct KnowledgeByStatus {
    unverified: usize,
    verified: usize,
    refuted: usize,
    superseded: usize,
}

#[derive(Debug, Serialize)]
struct EpistemicCoverage {
    fact: usize,
    hypothesis: usize,
    assumption: usize,
    derived: usize,
}

#[derive(Debug, Serialize)]
struct StatusData {
    node_count: usize,
    tree_count: usize,
    trees: Vec<TreeHealth>,
    #[serde(skip_serializing_if = "Option::is_none")]
    knowledge_health: Option<KnowledgeHealthData>,
}

#[derive(Debug, Serialize)]
struct TreeHealth {
    id: String,
    name: String,
    edge_count: usize,
    feedback_edge_count: usize,
    valid_dag: bool,
}

fn execute_init(storage: &FsStorage, name: &str, dry_run: bool) -> CommandOutput<InitData> {
    if storage.workspace_exists() {
        let err = OutputError::new(
            "WORKSPACE_ALREADY_EXISTS",
            format!(
                "Workspace already initialized at {}",
                storage.root().display()
            ),
        );
        return CommandOutput {
            success: false,
            action: "init".to_string(),
            workspace: name.to_string(),
            data: InitData {
                directories_created: vec![],
                config_file: String::new(),
                counters_file: String::new(),
            },
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![err],
            warnings: vec![],
        };
    }

    if dry_run {
        return CommandOutput::ok(
            "init",
            name,
            InitData {
                directories_created: vec![
                    "nodes/".to_string(),
                    "trees/".to_string(),
                    "knowledge/".to_string(),
                    ".ltp/".to_string(),
                    ".ltp/undo/".to_string(),
                    ".ltp/redo/".to_string(),
                    ".ltp/tmp/".to_string(),
                ],
                config_file: "ltp.config.json".to_string(),
                counters_file: ".ltp/counters.json".to_string(),
            },
        );
    }

    if let Err(e) = storage.init_workspace(name) {
        let err = match e {
            LtpError::WorkspaceAlreadyExists { path } => OutputError::new(
                "WORKSPACE_ALREADY_EXISTS",
                format!("Workspace already initialized at {}", path),
            ),
            LtpError::Io(ref io_err) => OutputError::new("IO_ERROR", io_err.to_string()),
            other => OutputError::new("INTERNAL_ERROR", other.to_string()),
        };
        return CommandOutput {
            success: false,
            action: "init".to_string(),
            workspace: name.to_string(),
            data: InitData {
                directories_created: vec![],
                config_file: String::new(),
                counters_file: String::new(),
            },
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![err],
            warnings: vec![],
        };
    }

    CommandOutput::ok(
        "init",
        name,
        InitData {
            directories_created: vec![
                "nodes/".to_string(),
                "trees/".to_string(),
                "knowledge/".to_string(),
                ".ltp/".to_string(),
                ".ltp/undo/".to_string(),
                ".ltp/redo/".to_string(),
                ".ltp/tmp/".to_string(),
            ],
            config_file: "ltp.config.json".to_string(),
            counters_file: ".ltp/counters.json".to_string(),
        },
    )
}

fn execute_status(storage: &FsStorage) -> CommandOutput<StatusData> {
    if !storage.workspace_exists() {
        let err = OutputError::new(
            "WORKSPACE_NOT_INITIALIZED",
            "No LTP workspace found (run `ltp init` first)",
        );
        return CommandOutput {
            success: false,
            action: "status".to_string(),
            workspace: String::new(),
            data: StatusData {
                node_count: 0,
                tree_count: 0,
                trees: vec![],
                knowledge_health: None,
            },
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![err],
            warnings: vec![],
        };
    }

    let ws_name = storage.workspace_name().unwrap_or_default();

    let node_ids = match storage.list_node_ids() {
        Ok(ids) => ids,
        Err(e) => {
            let err = OutputError::new("IO_ERROR", e.to_string());
            return CommandOutput {
                success: false,
                action: "status".to_string(),
                workspace: ws_name,
                data: StatusData {
                    node_count: 0,
                    tree_count: 0,
                    trees: vec![],
                    knowledge_health: None,
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![err],
                warnings: vec![],
            };
        }
    };

    let tree_ids = match storage.list_tree_ids() {
        Ok(ids) => ids,
        Err(e) => {
            let err = OutputError::new("IO_ERROR", e.to_string());
            return CommandOutput {
                success: false,
                action: "status".to_string(),
                workspace: ws_name,
                data: StatusData {
                    node_count: 0,
                    tree_count: 0,
                    trees: vec![],
                    knowledge_health: None,
                },
                graph_health: GraphHealth {
                    valid_dag: true,
                    orphan_nodes_count: 0,
                },
                errors: vec![err],
                warnings: vec![],
            };
        }
    };

    let mut trees_health = Vec::new();
    let mut all_valid_dag = true;
    let mut referenced_nodes: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut total_feedback_count = 0usize;

    for tree_id in &tree_ids {
        let tree: Tree = match storage.load_tree(tree_id) {
            Ok(t) => t,
            Err(_) => continue,
        };

        let dag_valid = check_dag(&tree.edges, &tree.id).is_ok();
        if !dag_valid {
            all_valid_dag = false;
        }

        for node_ref in &tree.nodes {
            referenced_nodes.insert(node_ref.node_ref.clone());
        }

        let feedback_count = tree.feedback_edges.len();
        total_feedback_count += feedback_count;

        trees_health.push(TreeHealth {
            id: tree.id.clone(),
            name: tree.name.clone(),
            edge_count: tree.edges.len(),
            feedback_edge_count: feedback_count,
            valid_dag: dag_valid,
        });
    }

    let orphan_count = node_ids
        .iter()
        .filter(|id| !referenced_nodes.contains(*id))
        .count();

    // Compute knowledge health
    let knowledge_health = compute_knowledge_health(storage, &node_ids);

    let mut output = CommandOutput::ok(
        "status",
        &ws_name,
        StatusData {
            node_count: node_ids.len(),
            tree_count: tree_ids.len(),
            trees: trees_health,
            knowledge_health: Some(knowledge_health),
        },
    );

    output.graph_health = GraphHealth {
        valid_dag: all_valid_dag,
        orphan_nodes_count: orphan_count,
    };

    let _ = total_feedback_count;

    output
}

fn compute_knowledge_health(storage: &FsStorage, node_ids: &[String]) -> KnowledgeHealthData {
    let kn_ids = storage.list_knowledge_ids().unwrap_or_default();
    let items: Vec<ltp_engine::knowledge::KnowledgeItem> = kn_ids
        .iter()
        .filter_map(|id| storage.load_knowledge(id).ok())
        .collect();

    let total = items.len();
    let unlinked_items = items.iter().filter(|i| i.links.is_empty()).count();

    // Count contradictions: KN with status=verified and relation=contradicts to a fact node
    let mut contradictions = 0usize;
    for item in &items {
        if item.status == ltp_engine::knowledge::KnowledgeStatus::Verified {
            for link in &item.links {
                if link.relation == ltp_engine::knowledge::KnowledgeRelation::Contradicts {
                    if let Ok(node) = storage.load_node(&link.target) {
                        if node.epistemic == ltp_engine::node::types::EpistemicStatus::Fact {
                            contradictions += 1;
                        }
                    }
                }
            }
        }
    }

    let by_status = KnowledgeByStatus {
        unverified: items
            .iter()
            .filter(|i| i.status == ltp_engine::knowledge::KnowledgeStatus::Unverified)
            .count(),
        verified: items
            .iter()
            .filter(|i| i.status == ltp_engine::knowledge::KnowledgeStatus::Verified)
            .count(),
        refuted: items
            .iter()
            .filter(|i| i.status == ltp_engine::knowledge::KnowledgeStatus::Refuted)
            .count(),
        superseded: items
            .iter()
            .filter(|i| i.status == ltp_engine::knowledge::KnowledgeStatus::Superseded)
            .count(),
    };

    // Epistemic coverage: count nodes by epistemic status
    let mut fact_count = 0usize;
    let mut hypothesis_count = 0usize;
    let mut assumption_count = 0usize;
    let mut derived_count = 0usize;

    for node_id in node_ids {
        if let Ok(node) = storage.load_node(node_id) {
            match node.epistemic {
                ltp_engine::node::types::EpistemicStatus::Fact => fact_count += 1,
                ltp_engine::node::types::EpistemicStatus::Hypothesis => hypothesis_count += 1,
                ltp_engine::node::types::EpistemicStatus::Assumption => assumption_count += 1,
                ltp_engine::node::types::EpistemicStatus::Derived => derived_count += 1,
            }
        }
    }

    KnowledgeHealthData {
        total,
        unlinked_items,
        contradictions,
        by_status,
        epistemic_coverage: EpistemicCoverage {
            fact: fact_count,
            hypothesis: hypothesis_count,
            assumption: assumption_count,
            derived: derived_count,
        },
    }
}

fn render_output<T: Serialize>(output: &CommandOutput<T>, human: bool) {
    if human {
        render_human(output);
    } else {
        println!("{}", output.to_json());
    }
}

fn render_human<T: Serialize>(output: &CommandOutput<T>) {
    if output.success {
        println!("[OK] {}", output.action);
    } else {
        println!("[FAIL] {}", output.action);
    }

    if !output.workspace.is_empty() {
        println!("  workspace: {}", output.workspace);
    }

    for err in &output.errors {
        println!("  ERROR [{}]: {}", err.code, err.detail);
    }

    for warn in &output.warnings {
        println!("  WARN  [{}]: {}", warn.code, warn.detail);
    }

    let data_value = serde_json::to_value(&output.data).unwrap_or_default();
    if let Some(obj) = data_value.as_object() {
        for (k, v) in obj {
            println!("  {}: {}", k, v);
        }
    }

    println!(
        "  graph_health: valid_dag={}, orphans={}",
        output.graph_health.valid_dag, output.graph_health.orphan_nodes_count
    );
}

/// Snapshot all mutable workspace files (nodes/ + trees/ + knowledge/ + config) for history capture.
fn snapshot_workspace_paths(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();
    let dirs = [
        root.join("nodes"),
        root.join("trees"),
        root.join("knowledge"),
    ];

    for dir in &dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("json") {
                    paths.push(path);
                }
            }
        }
    }
    paths
}

/// Record a mutation in history. Called before/after a mutating command.
/// Returns a CaptureContext if history is enabled; caller must commit after mutation.
fn history_begin(
    storage: &FsStorage,
) -> Option<(HistoryManager, ltp_engine::history::CaptureContext)> {
    let config = storage.load_config().ok()?;
    if !config.history.enabled {
        return None;
    }
    let manager = HistoryManager::new(storage.root().to_path_buf(), config.history);
    if manager.is_batch_active() {
        return None;
    }
    let paths = snapshot_workspace_paths(storage.root());
    let ctx = manager.begin_capture(&paths).ok()?;
    Some((manager, ctx))
}

/// Commit the capture after a successful mutation.
fn history_commit(
    capture: Option<(HistoryManager, ltp_engine::history::CaptureContext)>,
    action: &str,
    command: &str,
) {
    if let Some((manager, ctx)) = capture {
        let _ = manager.commit_capture(ctx, action, command);
    }
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_env("LTP_LOG"))
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    let cwd = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Cannot determine current directory: {}", e);
            process::exit(1);
        }
    };

    let storage = FsStorage::new(cwd.clone());
    let full_command: String = std::env::args().collect::<Vec<_>>().join(" ");

    match cli.command {
        Commands::Init { name } => {
            let ws_name = name.unwrap_or_else(|| {
                cwd.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "ltp-workspace".to_string())
            });
            let output = execute_init(&storage, &ws_name, cli.dry_run);
            render_output(&output, cli.human);
            if !output.success {
                process::exit(1);
            }
        }
        Commands::Status => {
            let output = execute_status(&storage);
            render_output(&output, cli.human);
            if !output.success {
                process::exit(1);
            }
        }
        Commands::Node { action } => match action {
            NodeAction::Add {
                label,
                r#type,
                tags,
                observable,
                epistemic,
            } => {
                let capture = history_begin(&storage);
                let output = execute_node_add(
                    &storage,
                    &label,
                    &r#type,
                    tags,
                    observable,
                    epistemic.as_deref(),
                );
                if output.success {
                    history_commit(capture, "node_add", &full_command);
                }
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            NodeAction::Edit {
                id,
                label,
                add_tag,
                rm_tag,
                observable,
                epistemic,
            } => {
                let capture = history_begin(&storage);
                let output = execute_node_edit(
                    &storage,
                    &id,
                    label.as_deref(),
                    add_tag.as_deref(),
                    rm_tag.as_deref(),
                    observable,
                    epistemic.as_deref(),
                );
                if output.success {
                    history_commit(capture, "node_edit", &full_command);
                }
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            NodeAction::List {
                tree,
                r#type,
                status,
                epistemic,
            } => {
                let output = execute_node_list(
                    &storage,
                    tree.as_deref(),
                    r#type.as_deref(),
                    status.as_deref(),
                    epistemic.as_deref(),
                );
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            NodeAction::Search { tree: _tree, query } => {
                let output = execute_node_search(&storage, &query);
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            NodeAction::Rm { ids, force } => {
                let capture = history_begin(&storage);
                let output = execute_node_rm(&storage, &ids, force);
                if output.success {
                    history_commit(capture, "node_rm", &full_command);
                }
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            NodeAction::Inspect { id } => {
                let output = execute_node_inspect(&storage, &id);
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            NodeAction::Split { id, into, tree } => {
                let capture = history_begin(&storage);
                let output = execute_node_split(&storage, &id, &into, &tree);
                if output.success {
                    history_commit(capture, "node_split", &full_command);
                }
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
        },
        Commands::Tree { action } => match action {
            TreeAction::New { r#type, name } => {
                let capture = history_begin(&storage);
                let output = execute_tree_new(&storage, &r#type, &name);
                if output.success {
                    history_commit(capture, "tree_new", &full_command);
                }
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            TreeAction::List => {
                let output = execute_tree_list(&storage);
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            TreeAction::Rm { tree_id } => {
                let capture = history_begin(&storage);
                let output = execute_tree_rm(&storage, &tree_id);
                if output.success {
                    history_commit(capture, "tree_rm", &full_command);
                }
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            TreeAction::Attach { tree, node, role } => {
                let capture = history_begin(&storage);
                let output = execute_tree_attach(&storage, &tree, &node, role.as_deref());
                if output.success {
                    history_commit(capture, "tree_attach", &full_command);
                }
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            TreeAction::Detach { tree, node } => {
                let capture = history_begin(&storage);
                let output = execute_tree_detach(&storage, &tree, &node);
                if output.success {
                    history_commit(capture, "tree_detach", &full_command);
                }
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            TreeAction::Clone { tree_id, name } => {
                let capture = history_begin(&storage);
                let output = execute_tree_clone(&storage, &tree_id, &name);
                if output.success {
                    history_commit(capture, "tree_clone", &full_command);
                }
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            TreeAction::Diff { tree_a, tree_b } => {
                let output = execute_tree_diff(&storage, &tree_a, &tree_b);
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            TreeAction::Walk {
                tree_id,
                order,
                show_origin: _,
                expand_nbr: _,
                show_knowledge,
            } => {
                let output = execute_tree_walk(&storage, &tree_id, &order, show_knowledge);
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
        },
        Commands::Link { action } => match action {
            LinkAction::Connect {
                tree,
                from,
                to,
                operator,
                weight,
                nbr,
            } => {
                let capture = history_begin(&storage);
                let output = execute_link_connect(
                    &storage,
                    &tree,
                    &from,
                    &to,
                    operator.as_deref(),
                    weight,
                    nbr.as_deref(),
                );
                if output.success {
                    history_commit(capture, "link_connect", &full_command);
                }
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            LinkAction::Disconnect { tree, links } => {
                let capture = history_begin(&storage);
                let output = execute_link_disconnect(&storage, &tree, &links);
                if output.success {
                    history_commit(capture, "link_disconnect", &full_command);
                }
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            LinkAction::Feedback {
                tree,
                from,
                to,
                r#type,
                label,
            } => {
                let capture = history_begin(&storage);
                let output =
                    execute_link_feedback(&storage, &tree, &from, &to, &r#type, label.as_deref());
                if output.success {
                    history_commit(capture, "link_feedback", &full_command);
                }
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            LinkAction::Reverse { tree, link, force } => {
                let capture = history_begin(&storage);
                let output = execute_link_reverse(&storage, &tree, &link, force);
                if output.success {
                    history_commit(capture, "link_reverse", &full_command);
                }
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            LinkAction::Move {
                tree,
                link,
                new_from,
                new_to,
            } => {
                let capture = history_begin(&storage);
                let output = execute_link_move(
                    &storage,
                    &tree,
                    &link,
                    new_from.as_deref(),
                    new_to.as_deref(),
                );
                if output.success {
                    history_commit(capture, "link_move", &full_command);
                }
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            LinkAction::InsertBetween {
                tree,
                link,
                node,
                insert_after_cause,
                insert_before_effect,
            } => {
                let capture = history_begin(&storage);
                let output = execute_link_insert_between(
                    &storage,
                    &tree,
                    &link,
                    &node,
                    insert_after_cause.as_deref(),
                    insert_before_effect,
                );
                if output.success {
                    history_commit(capture, "link_insert_between", &full_command);
                }
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            LinkAction::Group {
                tree,
                links,
                operator,
            } => {
                let capture = history_begin(&storage);
                let output = execute_link_group(&storage, &tree, &links, &operator);
                if output.success {
                    history_commit(capture, "link_group", &full_command);
                }
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            LinkAction::Dissolve { tree, link } => {
                let capture = history_begin(&storage);
                let output = execute_link_dissolve(&storage, &tree, &link);
                if output.success {
                    history_commit(capture, "link_dissolve", &full_command);
                }
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            LinkAction::Split {
                tree,
                link,
                extract,
            } => {
                let capture = history_begin(&storage);
                let output = execute_link_split(&storage, &tree, &link, &extract);
                if output.success {
                    history_commit(capture, "link_split", &full_command);
                }
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            LinkAction::Reoperator {
                tree,
                link,
                operator,
            } => {
                let capture = history_begin(&storage);
                let output = execute_link_reoperator(&storage, &tree, &link, &operator);
                if output.success {
                    history_commit(capture, "link_reoperator", &full_command);
                }
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            LinkAction::AddCause {
                tree,
                link,
                node,
                weight,
                promote_to,
            } => {
                let capture = history_begin(&storage);
                let output = execute_link_add_cause(
                    &storage,
                    &tree,
                    &link,
                    &node,
                    weight,
                    promote_to.as_deref(),
                );
                if output.success {
                    history_commit(capture, "link_add_cause", &full_command);
                }
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            LinkAction::RmCause { tree, link, node } => {
                let capture = history_begin(&storage);
                let output = execute_link_rm_cause(&storage, &tree, &link, &node);
                if output.success {
                    history_commit(capture, "link_rm_cause", &full_command);
                }
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            LinkAction::Inspect { link_id, tree } => {
                let output = execute_link_inspect(&storage, &link_id, &tree);
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            LinkAction::Find { tree, from, to } => {
                let output = execute_link_find(&storage, &tree, &from, &to);
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
        },
        Commands::Validate { tree } => {
            let output = execute_validate(&storage, tree.as_deref());
            render_output(&output, cli.human);
            if !output.success {
                process::exit(1);
            }
        }
        Commands::Assume { action } => match action {
            AssumeAction::Add { tree, link, text } => {
                let capture = history_begin(&storage);
                let output = execute_assume_add(&storage, &tree, &link, &text);
                if output.success {
                    history_commit(capture, "assume_add", &full_command);
                }
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            AssumeAction::Edit { tree, asm, text } => {
                let capture = history_begin(&storage);
                let output = execute_assume_edit(&storage, &tree, &asm, &text);
                if output.success {
                    history_commit(capture, "assume_edit", &full_command);
                }
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            AssumeAction::Rm { tree, asm } => {
                let capture = history_begin(&storage);
                let output = execute_assume_rm(&storage, &tree, &asm);
                if output.success {
                    history_commit(capture, "assume_rm", &full_command);
                }
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            AssumeAction::List { tree, status } => {
                let output = execute_assume_list(&storage, &tree, status.as_deref());
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            AssumeAction::Move { tree, asm, to_link } => {
                let capture = history_begin(&storage);
                let output = execute_assume_move(&storage, &tree, &asm, &to_link);
                if output.success {
                    history_commit(capture, "assume_move", &full_command);
                }
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
        },
        Commands::Invalidate {
            tree,
            link,
            asm,
            injection,
        } => {
            let capture = history_begin(&storage);
            let output = execute_invalidate(&storage, &tree, &link, &asm, injection.as_deref());
            if output.success {
                history_commit(capture, "invalidate", &full_command);
            }
            render_output(&output, cli.human);
            if !output.success {
                process::exit(1);
            }
        }
        Commands::Trace {
            node_id,
            tree,
            direction,
            depth,
            no_feedback,
            nbr,
            show_knowledge,
        } => {
            let output = execute_trace(
                &storage,
                &node_id,
                &tree,
                &direction,
                depth,
                no_feedback,
                nbr,
                show_knowledge,
            );
            render_output(&output, cli.human);
            if !output.success {
                process::exit(1);
            }
        }
        Commands::Path { action } => match action {
            PathAction::Collapse {
                tree,
                from,
                to,
                label,
            } => {
                let capture = history_begin(&storage);
                let output = execute_path_collapse(&storage, &tree, &from, &to, &label);
                if output.success {
                    history_commit(capture, "path_collapse", &full_command);
                }
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            PathAction::Explode {
                tree,
                link,
                asm,
                label,
            } => {
                let capture = history_begin(&storage);
                let output = execute_path_explode(&storage, &tree, &link, &asm, &label);
                if output.success {
                    history_commit(capture, "path_explode", &full_command);
                }
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            PathAction::Replace {
                tree,
                macro_link,
                by_node,
            } => {
                let capture = history_begin(&storage);
                let output = execute_path_replace(&storage, &tree, &macro_link, &by_node);
                if output.success {
                    history_commit(capture, "path_replace", &full_command);
                }
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
        },
        Commands::Nbr { action } => match action {
            NbrAction::Add {
                tree,
                source_node,
                trim,
            } => {
                let capture = history_begin(&storage);
                let output = execute_nbr_add(&storage, &tree, &source_node, trim.as_deref());
                if output.success {
                    history_commit(capture, "nbr_add", &full_command);
                }
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            NbrAction::Rm { tree, nbr } => {
                let capture = history_begin(&storage);
                let output = execute_nbr_rm(&storage, &tree, &nbr);
                if output.success {
                    history_commit(capture, "nbr_rm", &full_command);
                }
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            NbrAction::List { tree } => {
                let output = execute_nbr_list(&storage, &tree);
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            NbrAction::Inspect { nbr_id, tree } => {
                let output = execute_nbr_inspect(&storage, &tree, &nbr_id);
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
        },
        Commands::Undo { dry_run: undo_dry } => {
            let effective_dry_run = cli.dry_run || undo_dry;
            let output = execute_undo(&storage, effective_dry_run);
            render_output(&output, cli.human);
            if !output.success {
                process::exit(1);
            }
        }
        Commands::Redo { dry_run: redo_dry } => {
            let effective_dry_run = cli.dry_run || redo_dry;
            let output = execute_redo(&storage, effective_dry_run);
            render_output(&output, cli.human);
            if !output.success {
                process::exit(1);
            }
        }
        Commands::History { action, last } => match action {
            Some(HistoryAction::Check) => {
                let output = execute_history_check(&storage);
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            Some(HistoryAction::Invalidate { from }) => {
                let from_seq = from.unwrap_or(1);
                let output = execute_history_invalidate(&storage, from_seq);
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            Some(HistoryAction::BeginBatch { label }) => {
                let output = execute_history_begin_batch(&storage, &label);
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            Some(HistoryAction::EndBatch) => {
                let output = execute_history_end_batch(&storage);
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            Some(HistoryAction::Clear) => {
                let output = execute_history_clear(&storage);
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            None => {
                let output = execute_history_list(&storage, last);
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
        },
        Commands::Knowledge { action } => match action {
            KnowledgeAction::Add {
                label,
                r#type,
                source_uri,
                source_excerpt,
                status,
                confidence,
                tags,
            } => {
                let ktype = match parse_knowledge_type(&r#type) {
                    Ok(t) => t,
                    Err(msg) => {
                        let output = ltp_engine::output::error_output(
                            "knowledge_add",
                            storage.workspace_name().unwrap_or_default(),
                            vec![OutputError::new("INVALID_KNOWLEDGE_TYPE", msg)],
                        );
                        render_output(&output, cli.human);
                        process::exit(1);
                    }
                };
                let kstatus = match status.as_deref().map(parse_knowledge_status) {
                    Some(Ok(s)) => Some(s),
                    Some(Err(msg)) => {
                        let output = ltp_engine::output::error_output(
                            "knowledge_add",
                            storage.workspace_name().unwrap_or_default(),
                            vec![OutputError::new("INVALID_STATUS", msg)],
                        );
                        render_output(&output, cli.human);
                        process::exit(1);
                    }
                    None => None,
                };
                let kconfidence = match confidence.as_deref().map(parse_confidence) {
                    Some(Ok(c)) => Some(c),
                    Some(Err(msg)) => {
                        let output = ltp_engine::output::error_output(
                            "knowledge_add",
                            storage.workspace_name().unwrap_or_default(),
                            vec![OutputError::new("INVALID_CONFIDENCE", msg)],
                        );
                        render_output(&output, cli.human);
                        process::exit(1);
                    }
                    None => None,
                };

                let capture = history_begin(&storage);
                let output = execute_knowledge_add(
                    &storage,
                    &label,
                    ktype,
                    source_uri.as_deref(),
                    source_excerpt.as_deref(),
                    kstatus,
                    kconfidence,
                    tags,
                );
                if output.success {
                    history_commit(capture, "knowledge_add", &full_command);
                }
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            KnowledgeAction::Edit {
                id,
                label,
                status,
                confidence,
                source_uri,
                source_excerpt,
                add_tag,
                rm_tag,
            } => {
                let kstatus = match status.as_deref().map(parse_knowledge_status) {
                    Some(Ok(s)) => Some(s),
                    Some(Err(msg)) => {
                        let output = ltp_engine::output::error_output(
                            "knowledge_edit",
                            storage.workspace_name().unwrap_or_default(),
                            vec![OutputError::new("INVALID_STATUS", msg)],
                        );
                        render_output(&output, cli.human);
                        process::exit(1);
                    }
                    None => None,
                };
                let kconfidence = match confidence.as_deref().map(parse_confidence) {
                    Some(Ok(c)) => Some(c),
                    Some(Err(msg)) => {
                        let output = ltp_engine::output::error_output(
                            "knowledge_edit",
                            storage.workspace_name().unwrap_or_default(),
                            vec![OutputError::new("INVALID_CONFIDENCE", msg)],
                        );
                        render_output(&output, cli.human);
                        process::exit(1);
                    }
                    None => None,
                };

                let capture = history_begin(&storage);
                let output = execute_knowledge_edit(
                    &storage,
                    &id,
                    label.as_deref(),
                    kstatus,
                    kconfidence,
                    source_uri.as_deref(),
                    source_excerpt.as_deref(),
                    add_tag,
                    rm_tag,
                );
                if output.success {
                    history_commit(capture, "knowledge_edit", &full_command);
                }
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            KnowledgeAction::Rm { ids } => {
                let capture = history_begin(&storage);
                let output = execute_knowledge_rm(&storage, &ids);
                if output.success {
                    history_commit(capture, "knowledge_rm", &full_command);
                }
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            KnowledgeAction::Inspect { id } => {
                let output = execute_knowledge_inspect(&storage, &id);
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            KnowledgeAction::List {
                r#type,
                status,
                confidence,
                unlinked,
                tag,
                target,
                relation,
            } => {
                let ktype = match r#type.as_deref().map(parse_knowledge_type) {
                    Some(Ok(t)) => Some(t),
                    Some(Err(msg)) => {
                        let output = ltp_engine::output::error_output(
                            "knowledge_list",
                            storage.workspace_name().unwrap_or_default(),
                            vec![OutputError::new("INVALID_KNOWLEDGE_TYPE", msg)],
                        );
                        render_output(&output, cli.human);
                        process::exit(1);
                    }
                    None => None,
                };
                let kstatus = match status.as_deref().map(parse_knowledge_status) {
                    Some(Ok(s)) => Some(s),
                    Some(Err(msg)) => {
                        let output = ltp_engine::output::error_output(
                            "knowledge_list",
                            storage.workspace_name().unwrap_or_default(),
                            vec![OutputError::new("INVALID_STATUS", msg)],
                        );
                        render_output(&output, cli.human);
                        process::exit(1);
                    }
                    None => None,
                };
                let kconfidence = match confidence.as_deref().map(parse_confidence) {
                    Some(Ok(c)) => Some(c),
                    Some(Err(msg)) => {
                        let output = ltp_engine::output::error_output(
                            "knowledge_list",
                            storage.workspace_name().unwrap_or_default(),
                            vec![OutputError::new("INVALID_CONFIDENCE", msg)],
                        );
                        render_output(&output, cli.human);
                        process::exit(1);
                    }
                    None => None,
                };
                let krelation = match relation.as_deref().map(parse_knowledge_relation) {
                    Some(Ok(r)) => Some(r),
                    Some(Err(msg)) => {
                        let output = ltp_engine::output::error_output(
                            "knowledge_list",
                            storage.workspace_name().unwrap_or_default(),
                            vec![OutputError::new("INVALID_RELATION", msg)],
                        );
                        render_output(&output, cli.human);
                        process::exit(1);
                    }
                    None => None,
                };

                let output = execute_knowledge_list(
                    &storage,
                    ktype,
                    kstatus,
                    kconfidence,
                    unlinked,
                    tag.as_deref(),
                    target.as_deref(),
                    krelation,
                );
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            KnowledgeAction::Link { id, to, relation } => {
                let krelation = match parse_knowledge_relation(&relation) {
                    Ok(r) => r,
                    Err(msg) => {
                        let output = ltp_engine::output::error_output(
                            "knowledge_link",
                            storage.workspace_name().unwrap_or_default(),
                            vec![OutputError::new("INVALID_RELATION", msg)],
                        );
                        render_output(&output, cli.human);
                        process::exit(1);
                    }
                };

                let capture = history_begin(&storage);
                let output = execute_knowledge_link(&storage, &id, &to, krelation);
                if output.success {
                    history_commit(capture, "knowledge_link", &full_command);
                }
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
            KnowledgeAction::Unlink { id, from } => {
                let capture = history_begin(&storage);
                let output = execute_knowledge_unlink(&storage, &id, &from);
                if output.success {
                    history_commit(capture, "knowledge_unlink", &full_command);
                }
                render_output(&output, cli.human);
                if !output.success {
                    process::exit(1);
                }
            }
        },
    }
}

fn parse_knowledge_type(s: &str) -> std::result::Result<KnowledgeType, String> {
    match s.to_lowercase().as_str() {
        "measurement" => Ok(KnowledgeType::Measurement),
        "testimony" => Ok(KnowledgeType::Testimony),
        "hypothesis" => Ok(KnowledgeType::Hypothesis),
        "document" => Ok(KnowledgeType::Document),
        "observation" => Ok(KnowledgeType::Observation),
        "derived" => Ok(KnowledgeType::Derived),
        other => Err(format!("Unknown knowledge type: {}", other)),
    }
}

fn parse_knowledge_status(s: &str) -> std::result::Result<KnowledgeStatus, String> {
    match s.to_lowercase().as_str() {
        "unverified" => Ok(KnowledgeStatus::Unverified),
        "verified" => Ok(KnowledgeStatus::Verified),
        "refuted" => Ok(KnowledgeStatus::Refuted),
        "superseded" => Ok(KnowledgeStatus::Superseded),
        other => Err(format!("Unknown knowledge status: {}", other)),
    }
}

fn parse_confidence(s: &str) -> std::result::Result<Confidence, String> {
    match s.to_lowercase().as_str() {
        "high" => Ok(Confidence::High),
        "medium" => Ok(Confidence::Medium),
        "low" => Ok(Confidence::Low),
        other => Err(format!("Unknown confidence level: {}", other)),
    }
}

fn parse_knowledge_relation(s: &str) -> std::result::Result<KnowledgeRelation, String> {
    match s.to_lowercase().as_str() {
        "supports" => Ok(KnowledgeRelation::Supports),
        "contradicts" => Ok(KnowledgeRelation::Contradicts),
        "contextualizes" => Ok(KnowledgeRelation::Contextualizes),
        other => Err(format!("Unknown knowledge relation: {}", other)),
    }
}
