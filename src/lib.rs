pub mod assume;
pub mod errors;
pub mod history;
pub mod knowledge;
pub mod link;
pub mod macro_assume;
pub mod macro_edge;
pub mod mcp;
pub mod nbr;
pub mod node;
pub mod output;
pub mod path;
pub mod storage;
pub mod trace;
pub mod tree;
pub mod validate;
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
