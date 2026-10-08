//! `--dry-run` (ADR-017): run the real command on a disposable copy of the
//! workspace and discard it.
//!
//! The parent copies only what LTP manages (D-2) to a temporary directory,
//! re-executes its own binary there without the `--dry-run` token, forwards
//! stdout/stderr byte for byte and exits with the child's code. It never takes
//! the real lock nor writes in the workspace (D-3), and any failure of the
//! mechanism is an `IO_ERROR`, never a fallback to a real run (D-6).

use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ltp_engine::output::{error_output, OutputError};
use ltp_engine::storage::Storage;
use ltp_engine::workspace::FsStorage;

use crate::{Cli, Commands};

/// Environment variable set on the simulation child (D-7 recursion guard).
const CHILD_ENV: &str = "LTP_DRY_RUN_CHILD";

/// The global flag token removed from the child's arguments.
const DRY_RUN_FLAG: &str = "--dry-run";

/// Prefix of the temporary copies under `temp_dir()`.
const COPY_PREFIX: &str = "ltp-dry-run-";

/// Directories managed by LTP, copied recursively (D-2).
const MANAGED_DIRS: [&str; 4] = ["nodes", "trees", "knowledge", ".ltp"];

/// Attempts to find a free `ltp-dry-run-<pid>-<n>` name before giving up.
const MAX_NAME_ATTEMPTS: u32 = 100;

/// Temporary copy of the workspace for `--dry-run`; deleted when dropped
/// (the parent never calls `process::exit` while it is alive).
struct DryRunCopy {
    root: PathBuf,
}

impl DryRunCopy {
    /// Copies what LTP manages (D-2) from `workspace` to a fresh
    /// `base/ltp-dry-run-<pid>-<n>`. On error the partial copy is removed.
    fn create(workspace: &Path, base: &Path) -> io::Result<Self> {
        let copy = Self::reserve(base)?;
        let config = workspace.join("ltp.config.json");
        if fs::symlink_metadata(&config).is_ok_and(|m| m.is_file()) {
            fs::copy(&config, copy.root.join("ltp.config.json"))?;
        }
        for dir in MANAGED_DIRS {
            let src = workspace.join(dir);
            match fs::symlink_metadata(&src) {
                Ok(meta) if meta.is_dir() => {
                    // `.ltp/tmp/` holds in-flight atomic writes: never copied.
                    let skip = (dir == ".ltp").then_some(OsStr::new("tmp"));
                    copy_tree(&src, &copy.root.join(dir), skip)?;
                }
                Ok(_) => {}
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(e),
            }
        }
        Ok(copy)
    }

    /// Creates the (empty) copy directory. The guard exists from the moment
    /// the directory does, so every later failure cleans it up.
    fn reserve(base: &Path) -> io::Result<Self> {
        let pid = std::process::id();
        for n in 0..MAX_NAME_ATTEMPTS {
            let root = base.join(format!("{COPY_PREFIX}{pid}-{n}"));
            match fs::create_dir(&root) {
                Ok(()) => return Ok(Self { root }),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "no free name for the dry-run copy",
        ))
    }
}

impl Drop for DryRunCopy {
    fn drop(&mut self) {
        // Only ever a directory this process created with `create_dir`.
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// Recursively copies regular files and directories from `src` to a new `dst`,
/// skipping the top-level entry named `skip`. Symlinks and special files are
/// not followed (D-2).
fn copy_tree(src: &Path, dst: &Path, skip: Option<&OsStr>) -> io::Result<()> {
    fs::create_dir(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let name = entry.file_name();
        if skip == Some(name.as_os_str()) {
            continue;
        }
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            copy_tree(&entry.path(), &dst.join(&name), None)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), dst.join(&name))?;
        }
    }
    Ok(())
}

/// Child arguments: the originals without the first `--dry-run` token found
/// before a `--` separator. Works on `OsString` so non-UTF-8 arguments survive.
fn child_args(args: &[OsString]) -> Vec<OsString> {
    let flag = args
        .iter()
        .take_while(|a| a.as_os_str() != "--")
        .position(|a| a.as_os_str() == DRY_RUN_FLAG);
    args.iter()
        .enumerate()
        .filter(|(i, _)| Some(*i) != flag)
        .map(|(_, a)| a.clone())
        .collect()
}

/// D-7: a `--dry-run` that reaches the simulation child is an internal error.
fn ensure_not_child(child_env: Option<&OsStr>) -> io::Result<()> {
    match child_env {
        Some(_) => Err(io::Error::other(
            "--dry-run reached the simulation child (recursion guard)",
        )),
        None => Ok(()),
    }
}

/// Runs the current binary on the copy and returns its `Output`.
fn run_child(copy: &DryRunCopy, args: &[OsString]) -> io::Result<Output> {
    Command::new(std::env::current_exe()?)
        .args(args)
        .current_dir(&copy.root)
        .env(CHILD_ENV, "1")
        .output()
}

/// Copies `workspace` under `base`, runs the child there and deletes the copy.
fn simulate(workspace: &Path, base: &Path, args: &[OsString]) -> io::Result<Output> {
    let copy = DryRunCopy::create(workspace, base)?;
    run_child(&copy, &child_args(args))
}

/// Entry point from `main` before dispatching: `Some(exit_code)` if the
/// command was simulated, `None` if it must run normally (no `--dry-run`, or
/// `init`/`undo`/`redo`, which keep their native dry-run — D-5).
pub fn intercept(cli: &Cli, cwd: &Path) -> Option<i32> {
    if !cli.dry_run
        || matches!(
            cli.command,
            Commands::Init { .. } | Commands::Undo { .. } | Commands::Redo { .. }
        )
    {
        return None;
    }
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let result = ensure_not_child(std::env::var_os(CHILD_ENV).as_deref())
        .and_then(|()| simulate(cwd, &std::env::temp_dir(), &args));
    Some(match result {
        Ok(output) => forward(&output),
        Err(e) => {
            let workspace = FsStorage::new(cwd.to_path_buf())
                .workspace_name()
                .unwrap_or_default();
            let output = error_output(
                "dry_run",
                workspace,
                vec![OutputError::new(
                    "IO_ERROR",
                    format!("dry-run simulation failed: {e}"),
                )],
            );
            crate::render_output(&output, cli.human);
            1
        }
    })
}

/// Forwards the child's stdout and stderr byte for byte and returns its exit
/// code (1 if it was killed by a signal).
fn forward(output: &Output) -> i32 {
    // A closed stdout/stderr leaves nothing to report to; the code still counts.
    let _ = io::stdout().write_all(&output.stdout);
    let _ = io::stdout().flush();
    let _ = io::stderr().write_all(&output.stderr);
    output.status.code().unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn os(args: &[&str]) -> Vec<OsString> {
        args.iter().map(OsString::from).collect()
    }

    /// Relative paths of every entry under `dir`, sorted.
    fn listing(dir: &Path) -> Vec<String> {
        fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
            for entry in fs::read_dir(dir).unwrap().flatten() {
                let path = entry.path();
                out.push(path.strip_prefix(root).unwrap().display().to_string());
                if entry.file_type().unwrap().is_dir() {
                    walk(root, &path, out);
                }
            }
        }
        let mut out = Vec::new();
        walk(dir, dir, &mut out);
        out.sort();
        out
    }

    /// A workspace with managed files, `.ltp/` state, foreign files and symlinks.
    fn workspace() -> tempfile::TempDir {
        let ws = tempfile::tempdir().unwrap();
        let d = ws.path();
        fs::write(d.join("ltp.config.json"), "{}").unwrap();
        for dir in [
            "nodes",
            "trees",
            "knowledge",
            ".ltp/undo/sub",
            ".ltp/redo",
            ".ltp/tmp",
        ] {
            fs::create_dir_all(d.join(dir)).unwrap();
        }
        fs::write(d.join("nodes/UDE-001.json"), "n").unwrap();
        fs::write(d.join("trees/tree-crt-a.json"), "t").unwrap();
        fs::write(d.join("knowledge/KN-001.json"), "k").unwrap();
        fs::write(d.join(".ltp/counters.json"), "c").unwrap();
        fs::write(d.join(".ltp/lock"), "l").unwrap();
        fs::write(d.join(".ltp/undo/001.json"), "u").unwrap();
        fs::write(d.join(".ltp/undo/sub/blob"), "b").unwrap();
        fs::write(d.join(".ltp/tmp/inflight.json"), "x").unwrap();
        // Foreign content (D-2: never copied).
        fs::write(d.join("notes.bin"), vec![0u8; 1024]).unwrap();
        fs::create_dir(d.join("src")).unwrap();
        fs::write(d.join("src/lib.rs"), "fn x() {}").unwrap();
        ws
    }

    #[test]
    fn child_args_removes_flag_anywhere() {
        for (input, expected) in [
            (
                &["--dry-run", "node", "add", "X"][..],
                &["node", "add", "X"][..],
            ),
            (&["node", "--dry-run", "add", "X"], &["node", "add", "X"]),
            (&["node", "add", "X", "--dry-run"], &["node", "add", "X"]),
            (&["node", "add", "X"], &["node", "add", "X"]),
        ] {
            assert_eq!(child_args(&os(input)), os(expected), "{input:?}");
        }
    }

    #[test]
    fn child_args_removes_only_first_and_stops_at_separator() {
        assert_eq!(
            child_args(&os(&["--dry-run", "a", "--dry-run"])),
            os(&["a", "--dry-run"])
        );
        assert_eq!(
            child_args(&os(&["node", "add", "--", "--dry-run"])),
            os(&["node", "add", "--", "--dry-run"])
        );
        assert_eq!(
            child_args(&os(&["--label=--dry-run", "x"])),
            os(&["--label=--dry-run", "x"])
        );
    }

    #[cfg(unix)]
    #[test]
    fn child_args_keeps_non_utf8() {
        use std::os::unix::ffi::OsStringExt;
        let raw = OsString::from_vec(vec![b'n', 0xff, b'x']);
        let args = vec![raw.clone(), OsString::from("--dry-run")];
        assert_eq!(child_args(&args), vec![raw]);
    }

    #[test]
    fn copy_contains_only_managed_files() {
        let ws = workspace();
        let base = tempfile::tempdir().unwrap();
        let copy = DryRunCopy::create(ws.path(), base.path()).unwrap();
        assert!(copy.root.starts_with(base.path()));
        assert!(copy
            .root
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with(&format!("{COPY_PREFIX}{}-", std::process::id())));
        assert_eq!(
            listing(&copy.root),
            [
                ".ltp",
                ".ltp/counters.json",
                ".ltp/lock",
                ".ltp/redo",
                ".ltp/undo",
                ".ltp/undo/001.json",
                ".ltp/undo/sub",
                ".ltp/undo/sub/blob",
                "knowledge",
                "knowledge/KN-001.json",
                "ltp.config.json",
                "nodes",
                "nodes/UDE-001.json",
                "trees",
                "trees/tree-crt-a.json",
            ]
        );
        assert_eq!(fs::read(copy.root.join(".ltp/lock")).unwrap(), b"l");
        // The source is copied, not moved.
        assert!(ws.path().join("nodes/UDE-001.json").exists());
    }

    #[cfg(unix)]
    #[test]
    fn copy_does_not_follow_symlinks() {
        use std::os::unix::fs::symlink;
        let ws = workspace();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("secret.json"), "s").unwrap();
        symlink(
            outside.path().join("secret.json"),
            ws.path().join("nodes/LINKED.json"),
        )
        .unwrap();
        symlink(outside.path(), ws.path().join("trees/linked-dir")).unwrap();
        // A managed directory that is itself a symlink is not copied.
        fs::remove_dir_all(ws.path().join("knowledge")).unwrap();
        symlink(outside.path(), ws.path().join("knowledge")).unwrap();

        let base = tempfile::tempdir().unwrap();
        let copy = DryRunCopy::create(ws.path(), base.path()).unwrap();
        let files = listing(&copy.root);
        assert!(!files.iter().any(|f| f.contains("LINKED")), "{files:?}");
        assert!(!files.iter().any(|f| f.contains("linked-dir")), "{files:?}");
        assert!(
            !files.iter().any(|f| f.starts_with("knowledge")),
            "{files:?}"
        );
        assert!(files.contains(&"nodes/UDE-001.json".to_string()));
    }

    #[test]
    fn copy_is_deleted_on_drop() {
        let ws = workspace();
        let base = tempfile::tempdir().unwrap();
        let copy = DryRunCopy::create(ws.path(), base.path()).unwrap();
        let root = copy.root.clone();
        assert!(root.exists());
        drop(copy);
        assert!(!root.exists());
        assert!(fs::read_dir(base.path()).unwrap().next().is_none());
    }

    #[test]
    fn two_live_copies_get_distinct_names() {
        let ws = workspace();
        let base = tempfile::tempdir().unwrap();
        let a = DryRunCopy::create(ws.path(), base.path()).unwrap();
        let b = DryRunCopy::create(ws.path(), base.path()).unwrap();
        assert_ne!(a.root, b.root);
    }

    #[test]
    fn missing_workspace_dirs_are_skipped() {
        let ws = tempfile::tempdir().unwrap();
        let base = tempfile::tempdir().unwrap();
        let copy = DryRunCopy::create(ws.path(), base.path()).unwrap();
        assert!(listing(&copy.root).is_empty());
    }

    #[test]
    fn d6_unusable_base_fails_without_running() {
        let ws = workspace();
        let before = listing(ws.path());
        let missing = tempfile::tempdir().unwrap().path().join("gone");
        let err = simulate(ws.path(), &missing, &os(&["node", "add", "X", "--dry-run"]));
        assert!(err.is_err());
        assert_eq!(listing(ws.path()), before);
    }

    #[cfg(unix)]
    #[test]
    fn d6_failed_copy_leaves_nothing_behind() {
        use std::os::unix::fs::PermissionsExt;
        let ws = workspace();
        let unreadable = ws.path().join("nodes/UDE-001.json");
        fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o000)).unwrap();
        if fs::read(&unreadable).is_ok() {
            return; // running as root: permissions are not enforced
        }
        let base = tempfile::tempdir().unwrap();
        assert!(DryRunCopy::create(ws.path(), base.path()).is_err());
        assert!(fs::read_dir(base.path()).unwrap().next().is_none());
        fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o644)).unwrap();
    }

    #[test]
    fn d7_recursion_guard() {
        assert!(ensure_not_child(None).is_ok());
        assert!(ensure_not_child(Some(OsStr::new("1"))).is_err());
        assert!(ensure_not_child(Some(OsStr::new(""))).is_err());
    }
}
