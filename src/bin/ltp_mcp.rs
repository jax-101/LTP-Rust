use std::env;
use std::path::PathBuf;
use std::process;

use ltp_engine::mcp::server::run;
use ltp_engine::workspace::FsStorage;

fn main() {
    let args: Vec<String> = env::args().collect();

    let workspace_path = parse_workspace_arg(&args)
        .or_else(|| env::var("CLAUDE_PROJECT_DIR").ok().map(PathBuf::from))
        .unwrap_or_else(|| {
            env::current_dir().unwrap_or_else(|e| {
                eprintln!("Cannot determine current directory: {e}");
                process::exit(1);
            })
        });

    let storage = FsStorage::new(workspace_path);
    run(storage);
}

/// Parse `--workspace <path>` from command line arguments.
fn parse_workspace_arg(args: &[String]) -> Option<PathBuf> {
    let mut iter = args.iter().skip(1);
    while let Some(arg) = iter.next() {
        if arg == "--workspace" {
            return iter.next().map(PathBuf::from);
        }
        if let Some(path) = arg.strip_prefix("--workspace=") {
            return Some(PathBuf::from(path));
        }
    }
    None
}
