use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::errors::{LtpError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockFile {
    pub pid: u32,
    pub timestamp: String,
    pub command: String,
}

pub fn acquire_lock(ltp_dir: &Path, command: &str) -> Result<()> {
    let lock_path = ltp_dir.join("lock");

    if lock_path.exists() {
        let content = fs::read_to_string(&lock_path)?;
        let existing: LockFile = serde_json::from_str(&content)?;

        if is_pid_alive(existing.pid) {
            return Err(LtpError::WorkspaceLocked {
                pid: existing.pid,
                timestamp: existing.timestamp,
            });
        }
        // Stale lock — remove and continue
        fs::remove_file(&lock_path)?;
    }

    let lock = LockFile {
        pid: std::process::id(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        command: command.to_string(),
    };

    let json = serde_json::to_string_pretty(&lock)?;
    fs::write(&lock_path, json)?;
    Ok(())
}

pub fn release_lock(ltp_dir: &Path) -> Result<()> {
    let lock_path = ltp_dir.join("lock");
    if lock_path.exists() {
        fs::remove_file(&lock_path)?;
    }
    Ok(())
}

#[cfg(unix)]
fn is_pid_alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

#[cfg(not(unix))]
fn is_pid_alive(_pid: u32) -> bool {
    // Conservative: assume alive on non-unix
    true
}
