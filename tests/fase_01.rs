use std::process::Command;

use serde_json::Value;

fn ltp_bin() -> String {
    env!("CARGO_BIN_EXE_ltp-engine").to_string()
}

fn run_ltp(dir: &std::path::Path, args: &[&str]) -> (Value, i32) {
    let output = Command::new(ltp_bin())
        .args(args)
        .current_dir(dir)
        .output()
        .expect("failed to execute ltp binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let code = output.status.code().unwrap_or(-1);

    let json: Value = serde_json::from_str(&stdout).unwrap_or_else(|_| {
        panic!(
            "Failed to parse JSON output.\nstdout: {}\nstderr: {}",
            stdout,
            String::from_utf8_lossy(&output.stderr)
        )
    });

    (json, code)
}

/// UAT 1.1: `ltp init --name "Test"` creates workspace structure correctly.
#[test]
fn uat_1_1_init_creates_workspace() {
    let tmp = tempfile::tempdir().expect("failed to create tempdir");
    let dir = tmp.path();

    let (json, code) = run_ltp(dir, &["init", "--name", "Test"]);

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["action"], "init");
    assert_eq!(json["workspace"], "Test");

    // Verify directories created
    assert!(dir.join("nodes").is_dir());
    assert!(dir.join("trees").is_dir());
    assert!(dir.join(".ltp").is_dir());
    assert!(dir.join(".ltp/undo").is_dir());
    assert!(dir.join(".ltp/redo").is_dir());
    assert!(dir.join(".ltp/tmp").is_dir());

    // Verify config file
    assert!(dir.join("ltp.config.json").is_file());
    let config: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("ltp.config.json")).unwrap())
            .unwrap();
    assert_eq!(config["name"], "Test");

    // Verify counters
    assert!(dir.join(".ltp/counters.json").is_file());

    // Verify .gitignore
    assert!(dir.join(".gitignore").is_file());
    let gitignore = std::fs::read_to_string(dir.join(".gitignore")).unwrap();
    assert!(gitignore.contains(".ltp/"));
}

/// UAT 1.2: `ltp init` repeated returns error WORKSPACE_ALREADY_EXISTS.
#[test]
fn uat_1_2_init_repeated_errors() {
    let tmp = tempfile::tempdir().expect("failed to create tempdir");
    let dir = tmp.path();

    // First init succeeds
    let (json, code) = run_ltp(dir, &["init", "--name", "Test"]);
    assert_eq!(code, 0);
    assert_eq!(json["success"], true);

    // Second init fails
    let (json, code) = run_ltp(dir, &["init", "--name", "Test"]);
    assert_ne!(code, 0);
    assert_eq!(json["success"], false);
    assert_eq!(json["errors"][0]["code"], "WORKSPACE_ALREADY_EXISTS");
}

/// UAT 1.3: `ltp status` in empty workspace shows 0 nodes, 0 trees, valid_dag true.
#[test]
fn uat_1_3_status_empty_workspace() {
    let tmp = tempfile::tempdir().expect("failed to create tempdir");
    let dir = tmp.path();

    // Init first
    run_ltp(dir, &["init", "--name", "StatusTest"]);

    let (json, code) = run_ltp(dir, &["status"]);

    assert_eq!(code, 0);
    assert_eq!(json["success"], true);
    assert_eq!(json["action"], "status");
    assert_eq!(json["workspace"], "StatusTest");
    assert_eq!(json["data"]["node_count"], 0);
    assert_eq!(json["data"]["tree_count"], 0);
    assert_eq!(json["graph_health"]["valid_dag"], true);
    assert_eq!(json["graph_health"]["orphan_nodes_count"], 0);
}

/// UAT 1.4: Concurrent lock acquisition fails with WORKSPACE_LOCKED.
#[test]
fn uat_1_4_lock_concurrent() {
    let tmp = tempfile::tempdir().expect("failed to create tempdir");
    let dir = tmp.path();

    // Init workspace
    run_ltp(dir, &["init", "--name", "LockTest"]);

    // Manually write a lock file with the current PID (which is alive)
    let lock_content = serde_json::json!({
        "pid": std::process::id(),
        "timestamp": "2024-01-01T00:00:00Z",
        "command": "test-command"
    });
    std::fs::write(
        dir.join(".ltp/lock"),
        serde_json::to_string_pretty(&lock_content).unwrap(),
    )
    .unwrap();

    // Now try to run status (which should try to acquire the lock)
    // Status doesn't acquire a lock, so we need a command that does.
    // Actually, for Phase 1 the lock is tested via the Storage trait directly.
    // Let's test it programmatically using the library.
    use ltp_engine::storage::Storage;
    use ltp_engine::workspace::FsStorage;

    let storage = FsStorage::new(dir.to_path_buf());
    let result = storage.acquire_lock("another-command");
    assert!(result.is_err());

    let err = result.unwrap_err();
    match err {
        ltp_engine::errors::LtpError::WorkspaceLocked { pid, .. } => {
            assert_eq!(pid, std::process::id());
        }
        other => panic!("Expected WorkspaceLocked, got: {:?}", other),
    }
}

/// UAT 1.5: Stale lock (dead PID) is auto-removed with warning STALE_LOCK_REMOVED.
#[test]
fn uat_1_5_stale_lock_removed() {
    let tmp = tempfile::tempdir().expect("failed to create tempdir");
    let dir = tmp.path();

    // Init workspace
    run_ltp(dir, &["init", "--name", "StaleLockTest"]);

    // Write a lock with a definitely-dead PID
    let dead_pid = 99999u32;
    let lock_content = serde_json::json!({
        "pid": dead_pid,
        "timestamp": "2024-01-01T00:00:00Z",
        "command": "dead-command"
    });
    std::fs::write(
        dir.join(".ltp/lock"),
        serde_json::to_string_pretty(&lock_content).unwrap(),
    )
    .unwrap();

    use ltp_engine::storage::{LockOutcome, Storage};
    use ltp_engine::workspace::FsStorage;

    let storage = FsStorage::new(dir.to_path_buf());
    let result = storage.acquire_lock("new-command");
    assert!(result.is_ok());

    match result.unwrap() {
        LockOutcome::StaleLockRemoved { pid } => {
            assert_eq!(pid, dead_pid);
        }
        LockOutcome::Acquired => {
            panic!("Expected StaleLockRemoved, got Acquired");
        }
    }

    // Clean up
    storage.release_lock().unwrap();
}

/// UAT 1.6: counters.json exists after init with all types at 0.
#[test]
fn uat_1_6_counters_initialized() {
    let tmp = tempfile::tempdir().expect("failed to create tempdir");
    let dir = tmp.path();

    run_ltp(dir, &["init", "--name", "CounterTest"]);

    let counters_path = dir.join(".ltp/counters.json");
    assert!(counters_path.is_file());

    let content = std::fs::read_to_string(&counters_path).unwrap();
    let counters: Value = serde_json::from_str(&content).unwrap();

    let expected_types = [
        "UDE", "RC", "INJ", "NC", "GOAL", "OBJ", "WANT", "OBS", "IO", "INT", "DE", "REQ", "PRE",
        "TREE", "LINK", "ASM", "NBR", "MACRO",
    ];

    for entity_type in expected_types {
        assert_eq!(
            counters[entity_type], 0,
            "Counter for {} should be 0",
            entity_type
        );
    }
}
