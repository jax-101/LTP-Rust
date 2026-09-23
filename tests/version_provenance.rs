//! UATs for the release versioning policy (RELEASE_POLICY.md).
//!
//! The whole point of the policy is: from a running binary there must be zero
//! doubt about which commit/docs it corresponds to. These tests assert that
//! both binaries report a `MAJOR.MINOR.PATCH+<provenance>` string, that the
//! SemVer core matches the compiled package version, and — critically — that
//! the CLI and the MCP server report the *identical* string.

use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::{json, Value};

fn ltp_bin() -> String {
    env!("CARGO_BIN_EXE_ltp").to_string()
}

fn mcp_bin() -> String {
    env!("CARGO_BIN_EXE_ltp-mcp").to_string()
}

/// Read `ltp --version` and return the version token (the part after the name).
fn cli_version() -> String {
    let output = Command::new(ltp_bin())
        .arg("--version")
        .output()
        .expect("failed to run ltp --version");
    assert!(output.status.success(), "ltp --version exited non-zero");
    let stdout = String::from_utf8(output.stdout).expect("non-utf8 --version output");
    // clap prints e.g. "ltp 0.2.0+a1b2c3d4"; take the last whitespace token.
    stdout
        .trim()
        .rsplit(' ')
        .next()
        .unwrap_or_default()
        .to_string()
}

/// Read the MCP server's `initialize` → `serverInfo.version`.
fn mcp_version() -> String {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut child = Command::new(mcp_bin())
        .arg("--workspace")
        .arg(dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn ltp-mcp");

    let req = json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}});
    {
        let mut stdin = child.stdin.take().expect("stdin");
        writeln!(
            stdin,
            "{}",
            serde_json::to_string(&req).expect("serialize req")
        )
        .expect("write stdin");
    }
    let output = child.wait_with_output().expect("wait ltp-mcp");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .find(|l| !l.trim().is_empty())
        .expect("no MCP response");
    let resp: Value = serde_json::from_str(line).expect("parse MCP response");
    resp["result"]["serverInfo"]["version"]
        .as_str()
        .expect("serverInfo.version missing")
        .to_string()
}

/// Assert a version string has shape `MAJOR.MINOR.PATCH+<provenance>` with a
/// numeric SemVer core equal to the compiled package version.
fn assert_versioned(version: &str) {
    let Some((core, provenance)) = version.split_once('+') else {
        panic!("version '{version}' lacks '+<provenance>' build metadata");
    };

    let parts: Vec<&str> = core.split('.').collect();
    assert_eq!(
        parts.len(),
        3,
        "SemVer core must be MAJOR.MINOR.PATCH, got '{core}'"
    );
    for p in &parts {
        assert!(
            !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()),
            "SemVer component '{p}' not numeric in '{core}'"
        );
    }
    assert!(
        !provenance.is_empty(),
        "provenance after '+' is empty in '{version}'"
    );
    assert_eq!(
        core,
        env!("CARGO_PKG_VERSION"),
        "reported core '{core}' != compiled package version '{}'",
        env!("CARGO_PKG_VERSION")
    );
}

// --- UAT V.1: CLI reports a provenanced SemVer version ---
#[test]
fn cli_version_is_provenanced() {
    assert_versioned(&cli_version());
}

// --- UAT V.2: MCP serverInfo reports a provenanced SemVer version ---
#[test]
fn mcp_version_is_provenanced() {
    assert_versioned(&mcp_version());
}

// --- UAT V.3 (anti-ambiguity core): CLI and MCP report the identical string ---
#[test]
fn cli_and_mcp_report_identical_version() {
    let cli = cli_version();
    let mcp = mcp_version();
    assert_eq!(
        cli, mcp,
        "CLI ('{cli}') and MCP ('{mcp}') must report the identical version"
    );
}

// --- UAT V.4: version carries git provenance, not a bare SemVer ---
#[test]
fn version_is_not_bare_semver() {
    let v = cli_version();
    assert!(
        v.contains('+'),
        "version '{v}' must carry '+<sha>' provenance, not a bare SemVer"
    );
    // The provenance segment is either a hex-ish short sha (optionally .dirty)
    // or the explicit 'unknown' fallback — never empty.
    let provenance = v.split_once('+').map(|(_, p)| p).unwrap_or_default();
    assert!(!provenance.is_empty(), "provenance segment empty in '{v}'");
}
