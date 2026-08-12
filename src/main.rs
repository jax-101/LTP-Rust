use std::process;

use clap::{Parser, Subcommand};
use serde::Serialize;

use ltp_engine::errors::LtpError;
use ltp_engine::output::{error_output, CommandOutput, GraphHealth, OutputError};
use ltp_engine::storage::Storage;
use ltp_engine::tree::Tree;
use ltp_engine::validate::check_dag;
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
        tree: String,
        #[arg(long, value_delimiter = ',')]
        r#type: Option<Vec<String>>,
        #[arg(long, value_delimiter = ',')]
        status: Option<Vec<String>>,
    },
    Search {
        #[arg(long)]
        tree: String,
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

// --- Command implementations ---

#[derive(Debug, Serialize)]
struct InitData {
    directories_created: Vec<String>,
    config_file: String,
    counters_file: String,
}

#[derive(Debug, Serialize)]
struct StatusData {
    node_count: usize,
    tree_count: usize,
    trees: Vec<TreeHealth>,
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

    let mut output = CommandOutput::ok(
        "status",
        &ws_name,
        StatusData {
            node_count: node_ids.len(),
            tree_count: tree_ids.len(),
            trees: trees_health,
        },
    );

    output.graph_health = GraphHealth {
        valid_dag: all_valid_dag,
        orphan_nodes_count: orphan_count,
    };

    let _ = total_feedback_count;

    output
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

fn main() {
    let cli = Cli::parse();

    let cwd = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Cannot determine current directory: {}", e);
            process::exit(1);
        }
    };

    let storage = FsStorage::new(cwd.clone());

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
        _ => {
            let output = error_output(
                "unknown",
                "",
                vec![OutputError::new(
                    "NOT_IMPLEMENTED",
                    "Command not yet implemented",
                )],
            );
            render_output(&output, cli.human);
            process::exit(1);
        }
    }
}
