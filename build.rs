//! Build script: embeds git provenance (short SHA + dirty flag) into the binary
//! so the running engine can report exactly which commit it was built from.
//!
//! Emits `LTP_GIT_SHA` as a compile-time env var, consumed via `env!` in
//! `src/lib.rs`. The value is the short commit hash, suffixed with `.dirty` when
//! the working tree has uncommitted changes at build time (best-effort; see
//! RELEASE_POLICY.md). Falls back to `unknown` when git is unavailable (e.g.
//! building from a source tarball).

use std::process::Command;

fn main() {
    let sha = git(&["rev-parse", "--short=8", "HEAD"]).unwrap_or_else(|| "unknown".to_string());

    let dirty = match git(&["status", "--porcelain"]) {
        Some(out) => !out.is_empty(),
        None => false,
    };

    let provenance = if dirty { format!("{sha}.dirty") } else { sha };

    println!("cargo:rustc-env=LTP_GIT_SHA={provenance}");

    // Re-run when HEAD moves or the index changes so the SHA/dirty flag stay
    // fresh across commits (see the `dirty` caveat in RELEASE_POLICY.md).
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/index");
}

/// Run a git subcommand, returning its trimmed stdout on success, or `None` when
/// git is missing or the command fails.
fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    Some(text.trim().to_string())
}
