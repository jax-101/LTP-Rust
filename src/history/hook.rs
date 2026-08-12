use std::path::Path;

use crate::errors::Result;

/// Hook invoked before and after mutations for snapshot-based undo/redo.
///
/// Implementations capture file state for the history stack.
/// The `NoOpHook` skips snapshots when history is disabled.
pub trait SnapshotHook {
    /// Called before writing mutated files to disk.
    fn before_mutation(&self, affected_files: &[&Path]) -> Result<()>;

    /// Called after mutated files have been written to disk.
    fn after_mutation(&self, affected_files: &[&Path]) -> Result<()>;
}

/// No-op implementation that skips snapshot capture.
pub struct NoOpHook;

impl SnapshotHook for NoOpHook {
    fn before_mutation(&self, _affected_files: &[&Path]) -> Result<()> {
        Ok(())
    }

    fn after_mutation(&self, _affected_files: &[&Path]) -> Result<()> {
        Ok(())
    }
}
