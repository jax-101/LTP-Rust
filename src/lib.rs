//! `ltp-engine`: deterministic engine for Dettmer's Logical Thinking Process.
//!
//! Manages the global causal graph on disk (nodes, trees, knowledge) through
//! headless commands; the CLI (`ltp`) and the MCP server (`ltp-mcp`) are thin
//! front-ends over these modules. See `ENGINE_SPEC.md`.

// Documentation debt is frozen, not paid (v0.5.1, D-8b): every module marked
// `#[allow(missing_docs)]` below predates the lint; new modules must be fully
// documented, and removing an `allow` is the way to pay a module's debt.
#![warn(missing_docs)]

#[allow(missing_docs)]
pub mod assume;
#[allow(missing_docs)]
pub mod errors;
#[allow(missing_docs)]
pub mod history;
#[allow(missing_docs)]
pub mod knowledge;
#[allow(missing_docs)]
pub mod link;
pub mod macro_assume;
pub mod macro_edge;
#[allow(missing_docs)]
pub mod mcp;
pub mod meta;
#[allow(missing_docs)]
pub mod nbr;
#[allow(missing_docs)]
pub mod node;
#[allow(missing_docs)]
pub mod output;
#[allow(missing_docs)]
pub mod path;
#[allow(missing_docs)]
pub mod storage;
#[allow(missing_docs)]
pub mod trace;
#[allow(missing_docs)]
pub mod tree;
#[allow(missing_docs)]
pub mod validate;
#[allow(missing_docs)]
pub mod workspace;

/// Semantic version of the engine, taken from `Cargo.toml` (e.g. `0.2.0`).
///
/// Human-facing release identity; use it for feature-gating a consumer
/// (`>= 0.2.0` ⇒ long-arrow macro lifecycle available). See `RELEASE_POLICY.md`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Git provenance embedded at build time: short commit SHA, suffixed `.dirty`
/// when the working tree had uncommitted changes, or `unknown` if git was
/// unavailable. Populated by `build.rs`.
pub const GIT_SHA: &str = env!("LTP_GIT_SHA");

/// Full version string: SemVer core plus git provenance as SemVer build
/// metadata, e.g. `0.2.0+a1b2c3d4` or `0.2.0+a1b2c3d4.dirty`.
///
/// Reported identically by the CLI (`ltp --version`) and the MCP server
/// (`initialize` → `serverInfo.version`) so there is never doubt about which
/// binary is running or which commit/docs it corresponds to.
pub const FULL_VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "+", env!("LTP_GIT_SHA"));
