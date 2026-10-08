//! v0.5.1 (ADR-017) — `--dry-run` runs the real command on a disposable copy
//! of the workspace: the output is byte-identical to the real run and the
//! workspace (including `.ltp/`) does not change by a single byte.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::Value;
use sha2::{Digest, Sha256};

// --- harness -----------------------------------------------------------------

/// stdout, stderr and exit code of one `ltp` invocation.
#[derive(Debug, PartialEq, Eq)]
struct Run {
    stdout: String,
    stderr: String,
    code: i32,
}

fn run_env(dir: &Path, args: &[&str], env: &[(&str, &OsStr)]) -> Run {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_ltp"));
    cmd.args(args)
        .current_dir(dir)
        .env_remove("LTP_DRY_RUN_CHILD");
    for (k, v) in env {
        cmd.env(k, v);
    }
    let output = cmd.output().expect("failed to execute ltp binary");
    Run {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        code: output.status.code().unwrap_or(-1),
    }
}

fn run(dir: &Path, args: &[&str]) -> Run {
    run_env(dir, args, &[])
}

fn json(run: &Run) -> Value {
    serde_json::from_str(&run.stdout)
        .unwrap_or_else(|_| panic!("Failed to parse JSON.\nstdout: {}", run.stdout))
}

fn run_ok(dir: &Path, args: &[&str]) -> Value {
    let r = run(dir, args);
    assert_eq!(r.code, 0, "ltp {args:?} failed: {}", r.stdout);
    json(&r)
}

/// `args` with `--dry-run` appended (or prepended when `first`).
fn dry(args: &[&str], first: bool) -> Vec<String> {
    let mut v: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    if first {
        v.insert(0, "--dry-run".to_string());
    } else {
        v.push("--dry-run".to_string());
    }
    v
}

fn as_strs(v: &[String]) -> Vec<&str> {
    v.iter().map(String::as_str).collect()
}

/// SHA-256 of every entry under `dir` (hidden ones included, symlinks not
/// followed), keyed by relative path. Directories are recorded too, so an
/// empty directory created by the run is detected.
fn fingerprint(dir: &Path) -> BTreeMap<String, String> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>) {
        let mut entries: Vec<_> = fs::read_dir(dir).unwrap().flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let path = entry.path();
            let rel = path.strip_prefix(root).unwrap().display().to_string();
            let ft = entry.file_type().unwrap();
            if ft.is_dir() {
                out.insert(format!("{rel}/"), String::new());
                walk(root, &path, out);
            } else if ft.is_symlink() {
                let target = fs::read_link(&path).unwrap();
                out.insert(rel, format!("-> {}", target.display()));
            } else {
                let hash = Sha256::digest(fs::read(&path).unwrap());
                out.insert(rel, format!("{hash:x}"));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

fn copy_dir(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap().flatten() {
        let to = dst.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &to);
        } else {
            fs::copy(entry.path(), &to).unwrap();
        }
    }
}

/// Names under `dir` that look like a dry-run copy.
fn dry_run_leftovers(dir: &Path) -> Vec<String> {
    fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("ltp-dry-run-"))
        .collect()
}

fn warning_codes(json: &Value) -> Vec<String> {
    json["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["code"].as_str().unwrap().to_string())
        .collect()
}

fn error_codes(json: &Value) -> Vec<String> {
    json["errors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["code"].as_str().unwrap().to_string())
        .collect()
}

const MAIN: &str = "tree-crt-main";

/// Rich fixture (IDs are fixed by construction):
/// - nodes RC-001..004, INT-001..002, UDE-001..003 (UDE-003 attached, unlinked), INJ-001 (pool only);
/// - `tree-crt-main`: LINK-001 RC-001→INT-001 (ASM-001), LINK-002 INT-001→UDE-001,
///   LINK-003 {RC-002, RC-003} AND→UDE-002 (ASM-002), LINK-004 INT-002→UDE-002,
///   LINK-005 RC-004→UDE-002, FB-001 UDE-001→RC-001, MACRO-001 RC-002⇢UDE-001
///   (reservation), MACRO-002 RC-001⇢UDE-001 (overlay over LINK-001/002, MASM-001),
///   NBR-001 from RC-001;
/// - `tree-frt-other` with UDE-001;
/// - KN-001 supports UDE-001, KN-002 unlinked;
/// - history with captured mutations.
fn rich_workspace(dir: &Path) {
    run_ok(dir, &["init", "--name", "DR"]);
    for (ty, label) in [
        ("RC", "Cause A"),
        ("RC", "Cause B"),
        ("INT", "Middle"),
        ("UDE", "Effect"),
        ("UDE", "Effect two"),
        ("UDE", "Spare"),
        ("INJ", "Inject"),
        ("RC", "Cause C"),
        ("INT", "Middle two"),
        ("RC", "Cause D"),
    ] {
        run_ok(dir, &["node", "add", label, "--type", ty]);
    }
    run_ok(dir, &["tree", "new", "crt", "main"]);
    for node in [
        "RC-001", "RC-002", "INT-001", "UDE-001", "UDE-002", "UDE-003", "RC-003", "INT-002",
        "RC-004",
    ] {
        run_ok(dir, &["tree", "attach", "--tree", MAIN, "--node", node]);
    }
    for (from, to) in [
        ("RC-001", "INT-001"),
        ("INT-001", "UDE-001"),
        ("RC-002", "UDE-002"),
    ] {
        run_ok(
            dir,
            &[
                "link", "connect", "--tree", MAIN, "--from", from, "--to", to,
            ],
        );
    }
    run_ok(
        dir,
        &[
            "link",
            "add-cause",
            "--tree",
            MAIN,
            "--link",
            "LINK-003",
            "--node",
            "RC-003",
            "--promote-to",
            "AND",
        ],
    );
    for (from, to) in [("INT-002", "UDE-002"), ("RC-004", "UDE-002")] {
        run_ok(
            dir,
            &[
                "link", "connect", "--tree", MAIN, "--from", from, "--to", to,
            ],
        );
    }
    for (link, text) in [("LINK-001", "Premise one"), ("LINK-003", "Premise two")] {
        run_ok(
            dir,
            &[
                "assume", "add", "--tree", MAIN, "--link", link, "--text", text,
            ],
        );
    }
    run_ok(
        dir,
        &[
            "link", "feedback", "--tree", MAIN, "--from", "UDE-001", "--to", "RC-001", "--type",
            "positive",
        ],
    );
    run_ok(
        dir,
        &[
            "macro", "add", "--tree", MAIN, "--from", "RC-002", "--to", "UDE-001", "--label",
            "Jump",
        ],
    );
    run_ok(
        dir,
        &[
            "path", "collapse", "--tree", MAIN, "--from", "RC-001", "--to", "UDE-001", "--label",
            "Summary",
        ],
    );
    run_ok(
        dir,
        &[
            "macro-assume",
            "add",
            "--tree",
            MAIN,
            "--macro-link",
            "MACRO-002",
            "--text",
            "Macro premise",
        ],
    );
    run_ok(dir, &["tree", "new", "frt", "other"]);
    run_ok(
        dir,
        &[
            "tree",
            "attach",
            "--tree",
            "tree-frt-other",
            "--node",
            "UDE-001",
        ],
    );
    run_ok(
        dir,
        &["nbr", "add", "--tree", MAIN, "--source-node", "RC-001"],
    );
    for label in ["Obs", "Obs2"] {
        run_ok(
            dir,
            &[
                "knowledge",
                "add",
                label,
                "--type",
                "observation",
                "--source-excerpt",
                "s",
            ],
        );
    }
    run_ok(
        dir,
        &[
            "knowledge",
            "link",
            "KN-001",
            "--to",
            "UDE-001",
            "--relation",
            "supports",
        ],
    );
}

/// One row per CLI mutation (DR1).
const MUTATIONS: &[&[&str]] = &[
    &["node", "add", "New", "--type", "UDE"],
    &["node", "edit", "UDE-002", "--label", "Renamed"],
    &["node", "rm", "UDE-003"],
    &["node", "rm", "UDE-001", "--force"],
    &[
        "node", "split", "--tree", MAIN, "INT-001", "--into", "PartA", "PartB",
    ],
    &["tree", "new", "ec", "third"],
    &["tree", "rename", "--name", "renamed", "tree-frt-other"],
    &["tree", "rm", "tree-frt-other"],
    &[
        "tree",
        "attach",
        "--tree",
        "tree-frt-other",
        "--node",
        "RC-002",
    ],
    &["tree", "detach", "--tree", MAIN, "--node", "UDE-003"],
    &["tree", "clone", "--name", "copy", MAIN],
    &[
        "link", "connect", "--tree", MAIN, "--from", "RC-002", "--to", "INT-001",
    ],
    &["link", "disconnect", "--tree", MAIN, "--links", "LINK-004"],
    &[
        "link", "feedback", "--tree", MAIN, "--from", "UDE-002", "--to", "RC-002", "--type",
        "negative",
    ],
    &[
        "link",
        "feedback-rm",
        "--tree",
        MAIN,
        "--feedback",
        "FB-001",
    ],
    &[
        "link", "reverse", "--tree", MAIN, "--link", "LINK-001", "--force",
    ],
    &[
        "link", "move", "--tree", MAIN, "--link", "LINK-004", "--new-to", "UDE-003",
    ],
    &[
        "link",
        "insert-between",
        "--tree",
        MAIN,
        "--link",
        "LINK-004",
        "--node",
        "UDE-003",
    ],
    &[
        "link",
        "group",
        "--tree",
        MAIN,
        "--links",
        "LINK-004,LINK-005",
        "--operator",
        "AND",
    ],
    &["link", "dissolve", "--tree", MAIN, "--link", "LINK-003"],
    &[
        "link",
        "split",
        "--tree",
        MAIN,
        "--link",
        "LINK-003",
        "--extract",
        "RC-003",
    ],
    &[
        "link",
        "reoperator",
        "--tree",
        MAIN,
        "--link",
        "LINK-003",
        "--operator",
        "OR",
    ],
    &[
        "link",
        "add-cause",
        "--tree",
        MAIN,
        "--link",
        "LINK-004",
        "--node",
        "UDE-003",
        "--promote-to",
        "AND",
    ],
    &[
        "link", "rm-cause", "--tree", MAIN, "--link", "LINK-003", "--node", "RC-003",
    ],
    &[
        "assume", "add", "--tree", MAIN, "--link", "LINK-002", "--text", "Extra",
    ],
    &[
        "assume", "edit", "--tree", MAIN, "--asm", "ASM-001", "--text", "Changed",
    ],
    &["assume", "rm", "--tree", MAIN, "--asm", "ASM-002"],
    &[
        "assume",
        "move",
        "--tree",
        MAIN,
        "--asm",
        "ASM-002",
        "--to-link",
        "LINK-002",
    ],
    &[
        "macro-assume",
        "add",
        "--tree",
        MAIN,
        "--macro-link",
        "MACRO-002",
        "--text",
        "More",
    ],
    &[
        "macro-assume",
        "rm",
        "--tree",
        MAIN,
        "--macro-link",
        "MACRO-002",
        "--asm",
        "MASM-001",
    ],
    &[
        "macro-assume",
        "gather",
        "--tree",
        MAIN,
        "--macro-link",
        "MACRO-002",
    ],
    &[
        "macro", "add", "--tree", MAIN, "--from", "RC-002", "--to", "UDE-003", "--label", "J2",
    ],
    &[
        "macro",
        "expand",
        "--tree",
        MAIN,
        "--macro-link",
        "MACRO-001",
        "--steps",
        "S1,S2",
    ],
    &[
        "macro",
        "promote",
        "--tree",
        MAIN,
        "--macro-link",
        "MACRO-001",
    ],
    &[
        "path", "collapse", "--tree", MAIN, "--from", "RC-002", "--to", "UDE-002", "--label", "C2",
    ],
    &[
        "path", "explode", "--tree", MAIN, "--link", "LINK-001", "--asm", "ASM-001", "--label",
        "Exploded",
    ],
    &[
        "path",
        "replace",
        "--tree",
        MAIN,
        "--macro-link",
        "MACRO-002",
        "--by-node",
        "UDE-003",
    ],
    &[
        "invalidate",
        "--tree",
        MAIN,
        "--link",
        "LINK-001",
        "--asm",
        "ASM-001",
        "--injection",
        "Fix",
    ],
    &["nbr", "add", "--tree", MAIN, "--source-node", "RC-002"],
    &["nbr", "rm", "--tree", MAIN, "--nbr", "NBR-001"],
    &[
        "knowledge",
        "add",
        "K3",
        "--type",
        "observation",
        "--source-excerpt",
        "s",
    ],
    &["knowledge", "edit", "KN-002", "--status", "verified"],
    &["knowledge", "rm", "KN-002"],
    &[
        "knowledge",
        "link",
        "KN-002",
        "--to",
        "RC-001",
        "--relation",
        "supports",
    ],
    &["knowledge", "unlink", "KN-001", "--from", "UDE-001"],
    &["history", "clear"],
];

/// Runs `args` with `--dry-run` on a copy of `template` and for real on a twin
/// copy; asserts byte-identical output and an untouched dry-run workspace.
/// Returns the real run.
fn assert_dry_matches_real(template: &Path, args: &[&str], first: bool) -> Run {
    let (simulated, actual) = dry_and_real(template, args, first);
    assert_eq!(simulated, actual, "dry-run output differs for {args:?}");
    actual
}

/// Runs `args` with `--dry-run` on one copy of `template` and for real on a
/// twin copy; asserts the dry-run copy is untouched. Returns (simulated, real).
fn dry_and_real(template: &Path, args: &[&str], first: bool) -> (Run, Run) {
    let sim = tempfile::tempdir().unwrap();
    let real = tempfile::tempdir().unwrap();
    copy_dir(template, sim.path());
    copy_dir(template, real.path());
    let before = fingerprint(sim.path());

    let dry_args = dry(args, first);
    let simulated = run(sim.path(), &as_strs(&dry_args));
    let actual = run(real.path(), args);
    assert_eq!(
        fingerprint(sim.path()),
        before,
        "--dry-run wrote to disk: {dry_args:?}"
    );
    (simulated, actual)
}

// --- DR1: every mutation --------------------------------------------------------

#[test]
fn dr1_every_mutation_simulates_byte_identical_without_writing() {
    let template = tempfile::tempdir().unwrap();
    rich_workspace(template.path());
    let pristine = fingerprint(template.path());

    for (i, args) in MUTATIONS.iter().enumerate() {
        let actual = assert_dry_matches_real(template.path(), args, i % 2 == 0);
        // Non-vacuous: every row is a real, successful mutation.
        assert_eq!(
            actual.code, 0,
            "row {args:?} must succeed: {}",
            actual.stdout
        );
    }
    assert_eq!(fingerprint(template.path()), pristine);
}

#[test]
fn dr1b_human_output_identical() {
    let template = tempfile::tempdir().unwrap();
    rich_workspace(template.path());
    for args in [
        &["node", "rm", "UDE-001", "--force", "--human"][..],
        &["--human", "node", "add", "X", "--type", "UDE"][..],
    ] {
        let actual = assert_dry_matches_real(template.path(), args, false);
        assert!(actual.stdout.starts_with("[OK]"), "{}", actual.stdout);
    }
}

// --- DR2: destructive previews ---------------------------------------------------

#[test]
fn dr2_destructive_preview_shows_real_warnings() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    rich_workspace(dir);
    let before = fingerprint(dir);

    let rm = json(&run(
        dir,
        &["node", "rm", "UDE-001", "--force", "--dry-run"],
    ));
    assert_eq!(rm["success"], true);
    let codes = warning_codes(&rm);
    assert_eq!(
        codes.iter().filter(|c| *c == "MACRO_EDGE_REMOVED").count(),
        2,
        "{rm}"
    );
    assert_eq!(
        rm["data"]["affected_trees"],
        serde_json::json!([MAIN, "tree-frt-other"])
    );

    let split = json(&run(
        dir,
        &[
            "--dry-run",
            "node",
            "split",
            "--tree",
            MAIN,
            "UDE-001",
            "--into",
            "A",
            "B",
        ],
    ));
    assert_eq!(split["success"], true);
    assert_eq!(
        split["data"]["affected_trees"],
        serde_json::json!([MAIN, "tree-frt-other"])
    );
    assert_eq!(fingerprint(dir), before);
    assert!(dir.join("nodes/UDE-001.json").exists());
}

// --- DR3: failing simulations ------------------------------------------------------

#[test]
fn dr3_failing_simulation_same_error_no_write() {
    let template = tempfile::tempdir().unwrap();
    rich_workspace(template.path());

    let cycle = assert_dry_matches_real(
        template.path(),
        &[
            "link", "connect", "--tree", MAIN, "--from", "UDE-001", "--to", "RC-001",
        ],
        false,
    );
    assert_eq!(cycle.code, 1);
    let cycle = json(&cycle);
    assert_eq!(error_codes(&cycle), vec!["CIRCULAR_DEPENDENCY_DETECTED"]);
    // T1b: the DFS starts at the smallest node ID, so the rotation is fixed.
    assert_eq!(
        cycle["errors"][0]["cycle_path"],
        serde_json::json!(["INT-001", "UDE-001", "RC-001", "INT-001"])
    );

    let missing = assert_dry_matches_real(template.path(), &["node", "rm", "NOPE-999"], true);
    assert_eq!(missing.code, 1);
    assert_eq!(error_codes(&json(&missing)), vec!["NODE_NOT_FOUND"]);

    // An unreadable tree makes `node split` fail closed (ADR-016).
    fs::write(template.path().join("trees/tree-frt-other.json"), "{broken").unwrap();
    let split = assert_dry_matches_real(
        template.path(),
        &[
            "node", "split", "--tree", MAIN, "UDE-001", "--into", "A", "B",
        ],
        false,
    );
    assert_eq!(split.code, 1, "{}", split.stdout);
    assert_eq!(json(&split)["success"], false);
}

// --- DR4: counters ---------------------------------------------------------------------

#[test]
fn dr4_simulation_does_not_consume_counters() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    run_ok(dir, &["init", "--name", "DR4"]);
    for _ in 0..3 {
        let sim = json(&run(
            dir,
            &["node", "add", "Draft", "--type", "UDE", "--dry-run"],
        ));
        assert_eq!(sim["data"]["id"], "UDE-001", "{sim}");
    }
    assert!(!dir.join("nodes/UDE-001.json").exists());
    let real = run_ok(dir, &["node", "add", "Real", "--type", "UDE"]);
    assert_eq!(real["data"]["id"], "UDE-001");
}

// --- DR5: history ------------------------------------------------------------------------

#[test]
fn dr5_no_history_entry_and_undo_targets_last_real() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    rich_workspace(dir);
    let history_before = run(dir, &["history", "list"]);

    let sim = run(dir, &["node", "add", "Ghost", "--type", "UDE", "--dry-run"]);
    assert_eq!(sim.code, 0);
    assert_eq!(run(dir, &["history", "list"]), history_before);

    // The last real mutation is `knowledge link KN-001 → UDE-001`.
    let undo = run_ok(dir, &["undo"]);
    assert_eq!(undo["data"]["action_undone"], "knowledge_link", "{undo}");
    let kn: Value =
        serde_json::from_str(&fs::read_to_string(dir.join("knowledge/KN-001.json")).unwrap())
            .unwrap();
    // `links` is omitted when empty.
    assert!(
        kn["links"].as_array().is_none_or(Vec::is_empty),
        "KN-001 must have lost its link: {kn}"
    );
    assert!(!dir.join("nodes/UDE-004.json").exists());
}

// --- DR6 / DR7: lock ------------------------------------------------------------------------

fn write_lock(dir: &Path, pid: u32) -> Vec<u8> {
    let lock = serde_json::json!({
        "pid": pid,
        "timestamp": "2026-10-08T00:00:00+00:00",
        "command": "node add",
    });
    let bytes = serde_json::to_vec_pretty(&lock).unwrap();
    fs::write(dir.join(".ltp/lock"), &bytes).unwrap();
    bytes
}

#[test]
fn dr6_live_lock_same_error_and_lock_untouched() {
    let template = tempfile::tempdir().unwrap();
    rich_workspace(template.path());
    let lock = write_lock(template.path(), std::process::id());

    let actual = assert_dry_matches_real(
        template.path(),
        &["node", "add", "Blocked", "--type", "UDE"],
        false,
    );
    assert_eq!(actual.code, 1);
    assert_eq!(error_codes(&json(&actual)), vec!["WORKSPACE_LOCKED"]);

    // And directly on the template: the lock stays byte-identical.
    run(template.path(), &["--dry-run", "node", "rm", "UDE-003"]);
    assert_eq!(fs::read(template.path().join(".ltp/lock")).unwrap(), lock);
}

fn dead_pid() -> u32 {
    let mut child = Command::new("true").spawn().unwrap();
    let pid = child.id();
    child.wait().unwrap();
    pid
}

#[test]
fn dr7_stale_lock_reported_but_not_repaired() {
    let template = tempfile::tempdir().unwrap();
    rich_workspace(template.path());
    let lock = write_lock(template.path(), dead_pid());

    let actual = assert_dry_matches_real(
        template.path(),
        &["node", "add", "After", "--type", "UDE"],
        true,
    );
    assert_eq!(actual.code, 0, "{}", actual.stdout);
    assert!(
        warning_codes(&json(&actual)).contains(&"STALE_LOCK_REMOVED".to_string()),
        "{}",
        actual.stdout
    );

    let sim = run(
        template.path(),
        &["node", "add", "After", "--type", "UDE", "--dry-run"],
    );
    assert!(warning_codes(&json(&sim)).contains(&"STALE_LOCK_REMOVED".to_string()));
    assert_eq!(fs::read(template.path().join(".ltp/lock")).unwrap(), lock);
}

// --- DR8: batch ---------------------------------------------------------------------------

#[test]
fn dr8_inside_batch_only_real_mutations_grouped() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    rich_workspace(dir);
    let pre_batch = fingerprint(&dir.join("nodes"));

    run_ok(dir, &["history", "begin-batch", "--label", "batch"]);
    let in_batch = fingerprint(dir);
    let sim = run(dir, &["--dry-run", "node", "add", "Ghost", "--type", "UDE"]);
    assert_eq!(sim.code, 0, "{}", sim.stdout);
    assert_eq!(fingerprint(dir), in_batch, "dry-run wrote inside a batch");

    let real = run_ok(dir, &["node", "add", "Real", "--type", "UDE"]);
    assert_eq!(real["data"]["id"], "UDE-004");
    run_ok(dir, &["history", "end-batch"]);

    // Undoing the batch restores exactly the pre-batch node pool.
    run_ok(dir, &["undo"]);
    assert_eq!(fingerprint(&dir.join("nodes")), pre_batch);
}

// --- DR9 / DR10: cleanup and foreign files ------------------------------------------------

#[test]
fn dr9_no_copy_left_after_success_error_or_human() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    rich_workspace(dir);
    let tmpdir = tempfile::tempdir().unwrap();
    let env = [("TMPDIR", tmpdir.path().as_os_str())];

    for args in [
        &["node", "add", "Ok", "--type", "UDE", "--dry-run"][..],
        &["node", "rm", "NOPE-999", "--dry-run"][..],
        &["--human", "--dry-run", "node", "rm", "UDE-001", "--force"][..],
        &["--dry-run", "status"][..],
    ] {
        let r = run_env(dir, args, &env);
        assert!(!r.stdout.is_empty(), "{args:?} produced no output");
        assert_eq!(
            dry_run_leftovers(tmpdir.path()),
            Vec::<String>::new(),
            "{args:?}"
        );
    }
}

#[test]
fn dr10_foreign_files_untouched_and_not_left_in_tmp() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    rich_workspace(dir);
    let big: Vec<u8> = (0..5 * 1024 * 1024).map(|i| (i % 251) as u8).collect();
    fs::write(dir.join("notes.bin"), &big).unwrap();
    fs::create_dir(dir.join("src")).unwrap();
    fs::write(dir.join("src/lib.rs"), "fn main() {}").unwrap();
    let before = fingerprint(dir);
    let tmpdir = tempfile::tempdir().unwrap();

    let r = run_env(
        dir,
        &["node", "rm", "UDE-003", "--dry-run"],
        &[("TMPDIR", tmpdir.path().as_os_str())],
    );
    assert_eq!(r.code, 0, "{}", r.stdout);
    assert_eq!(fingerprint(dir), before);
    assert!(fs::read_dir(tmpdir.path()).unwrap().next().is_none());
}

// --- DR11: native exceptions and read-only commands ----------------------------------------

#[test]
fn dr11_init_undo_redo_keep_native_dry_run() {
    // With an unusable TMPDIR, any command routed through the copy fails with
    // IO_ERROR: the native ones must not be.
    let missing_tmp = tempfile::tempdir().unwrap().path().join("gone");
    let env = [("TMPDIR", missing_tmp.as_os_str())];

    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let init = run_env(dir, &["init", "--name", "N", "--dry-run"], &env);
    assert_eq!(init.code, 0, "{}", init.stdout);
    assert!(
        fs::read_dir(dir).unwrap().next().is_none(),
        "init --dry-run wrote"
    );

    rich_workspace(dir);
    let before = fingerprint(dir);
    for args in [&["undo", "--dry-run"][..], &["--dry-run", "undo"][..]] {
        let r = run_env(dir, args, &env);
        assert_eq!(r.code, 0, "{args:?}: {}", r.stdout);
        assert_eq!(json(&r)["data"]["dry_run"], true, "{}", r.stdout);
        assert_eq!(fingerprint(dir), before);
    }

    run_ok(dir, &["undo"]);
    let before = fingerprint(dir);
    let redo = run_env(dir, &["redo", "--dry-run"], &env);
    assert_eq!(redo.code, 0, "{}", redo.stdout);
    assert_eq!(json(&redo)["data"]["dry_run"], true);
    assert_eq!(fingerprint(dir), before);
}

#[test]
fn dr11b_read_only_commands_identical() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    rich_workspace(dir);
    let before = fingerprint(dir);
    for args in [
        &["tree", "list"][..],
        &["status"][..],
        &["tree", "walk", MAIN, "--show-knowledge"][..],
    ] {
        assert_eq!(
            run(dir, &as_strs(&dry(args, false))),
            run(dir, args),
            "{args:?}"
        );
    }
    assert_eq!(
        run(dir, &["validate", "--dry-run"]),
        run(dir, &["validate"])
    );
    assert_eq!(fingerprint(dir), before);
}

// --- DR12: no workspace ---------------------------------------------------------------------

#[test]
fn dr12_outside_workspace_same_error_nothing_created() {
    let sim = tempfile::tempdir().unwrap();
    let real = tempfile::tempdir().unwrap();
    for args in [
        &["node", "add", "X", "--type", "UDE"][..],
        &["tree", "list"][..],
    ] {
        let simulated = run(sim.path(), &as_strs(&dry(args, false)));
        assert_eq!(simulated, run(real.path(), args), "{args:?}");
        if args[0] == "node" {
            assert_eq!(simulated.code, 1, "{}", simulated.stdout);
        }
        assert!(
            fs::read_dir(sim.path()).unwrap().next().is_none(),
            "dry-run created files outside a workspace"
        );
    }
}

// --- D-6 / D-7: mechanism failures fail closed ------------------------------------------------

#[test]
fn d6_unusable_tmpdir_is_io_error_without_real_run() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    rich_workspace(dir);
    let before = fingerprint(dir);
    let missing_tmp = tempfile::tempdir().unwrap().path().join("gone");

    let r = run_env(
        dir,
        &["node", "add", "X", "--type", "UDE", "--dry-run"],
        &[("TMPDIR", missing_tmp.as_os_str())],
    );
    assert_eq!(r.code, 1);
    let out = json(&r);
    assert_eq!(out["success"], false);
    assert_eq!(error_codes(&out), vec!["IO_ERROR"]);
    assert_eq!(out["workspace"], "DR");
    assert_eq!(fingerprint(dir), before, "fell back to a real run");
}

#[test]
fn d7_dry_run_inside_child_is_io_error() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    rich_workspace(dir);
    let before = fingerprint(dir);

    let r = run_env(
        dir,
        &["node", "add", "X", "--type", "UDE", "--dry-run"],
        &[("LTP_DRY_RUN_CHILD", OsStr::new("1"))],
    );
    assert_eq!(r.code, 1, "{}", r.stdout);
    assert_eq!(error_codes(&json(&r)), vec!["IO_ERROR"]);
    assert_eq!(fingerprint(dir), before);
}

// --- T1b: deterministic output across processes ------------------------------------------------

/// Invariant 1: every process gets a fresh hash seed, so an output built by
/// iterating a HashMap/HashSet changes between runs. Twenty runs must agree.
#[test]
fn t1b_validate_and_cycle_outputs_stable_across_processes() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    rich_workspace(dir);
    // CLR#7 needs intangible nodes; CLR#5 a MAG group.
    for id in ["INT-001", "INT-002", "RC-004"] {
        run_ok(dir, &["node", "edit", id, "--observable", "false"]);
    }
    let cycle = [
        "link", "connect", "--tree", MAIN, "--from", "UDE-001", "--to", "RC-001",
    ];
    let first_validate = run(dir, &["validate"]);
    let first_cycle = run(dir, &cycle);
    assert_eq!(first_cycle.code, 1);
    let codes: Vec<String> = json(&first_validate)["data"]["details"][0]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["code"].as_str().unwrap().to_string())
        .collect();
    assert!(
        codes
            .iter()
            .filter(|c| *c == "CLR4_INSUFFICIENT_CAUSE")
            .count()
            >= 2
            && codes
                .iter()
                .filter(|c| *c == "CLR7_INTANGIBLE_NO_PREDICTED")
                .count()
                >= 2,
        "fixture must exercise several warnings of each lint: {codes:?}"
    );
    for _ in 0..20 {
        assert_eq!(run(dir, &["validate"]), first_validate);
        assert_eq!(run(dir, &cycle), first_cycle);
    }
}
