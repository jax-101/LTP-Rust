use serde::Serialize;
use std::collections::BTreeMap;

use crate::errors::LtpError;
use crate::storage::{CounterNotice, LockOutcome, RebuildReason};

#[derive(Debug, Clone, Serialize)]
pub struct GraphHealth {
    pub valid_dag: bool,
    pub orphan_nodes_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct OutputError {
    pub code: String,
    pub detail: String,
    #[serde(flatten)]
    pub context: BTreeMap<String, serde_json::Value>,
}

impl OutputError {
    pub fn new(code: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            detail: detail.into(),
            context: BTreeMap::new(),
        }
    }

    pub fn with_context(
        mut self,
        key: impl Into<String>,
        value: impl Into<serde_json::Value>,
    ) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }

    /// Error for a failed load or delete (PLAN_v060 D-4): `not_found` with
    /// `detail` when the entity is genuinely absent, `IO_ERROR` with the
    /// cause otherwise.
    pub fn load_failed(e: &LtpError, not_found: &'static str, detail: impl Into<String>) -> Self {
        match load_error_code(e, not_found) {
            "IO_ERROR" => Self::new("IO_ERROR", e.to_string()),
            code => Self::new(code, detail),
        }
    }
}

/// Error code for a failed load or delete (PLAN_v060 D-4, the rule of
/// ADR-016 D-4 extended to the whole engine). Only a `*NotFound` keeps the
/// site's `not_found` code; anything else (unreadable, corrupt, broken
/// symlink) is `IO_ERROR`, never a false "does not exist".
pub fn load_error_code(e: &LtpError, not_found: &'static str) -> &'static str {
    match e {
        LtpError::NodeNotFound(_)
        | LtpError::TreeNotFound(_)
        | LtpError::KnowledgeNotFound(_)
        | LtpError::LinkNotFound(_)
        | LtpError::AssumptionNotFound(_) => not_found,
        _ => "IO_ERROR",
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct OutputWarning {
    pub code: String,
    pub detail: String,
    #[serde(flatten)]
    pub context: BTreeMap<String, serde_json::Value>,
}

impl OutputWarning {
    pub fn new(code: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            detail: detail.into(),
            context: BTreeMap::new(),
        }
    }

    pub fn with_context(
        mut self,
        key: impl Into<String>,
        value: impl Into<serde_json::Value>,
    ) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }
}

/// Warning for a counter repair done while minting (PLAN_v060 D-5).
pub fn counter_notice_warning(notice: &CounterNotice) -> OutputWarning {
    match notice {
        CounterNotice::Rebuilt { reason } => {
            let (reason, why) = match reason {
                RebuildReason::Missing => ("missing", "was missing"),
                RebuildReason::Corrupt => ("corrupt", "could not be parsed"),
            };
            OutputWarning::new(
                "COUNTERS_REBUILT",
                format!(".ltp/counters.json {why}; counters were rebuilt from the workspace"),
            )
            .with_context("reason", reason)
        }
        CounterNotice::Reconciled { prefix, from, to } => OutputWarning::new(
            "COUNTERS_REBUILT",
            format!(
                "Counter {prefix} was {from} but the workspace already holds {prefix}-{to:03}; raised to {to}"
            ),
        )
        .with_context("reason", "stale")
        .with_context("prefix", prefix.as_str())
        .with_context("from", *from)
        .with_context("to", *to),
    }
}

/// Put the lock-session warnings ahead of `warnings`, in contract order:
/// `STALE_LOCK_REMOVED` first, then `COUNTERS_REBUILT` (PLAN_v060 D-5).
pub fn prepend_session_warnings(
    warnings: &mut Vec<OutputWarning>,
    lock: &LockOutcome,
    notice: Option<&CounterNotice>,
) {
    let stale = match lock {
        LockOutcome::StaleLockRemoved { pid } => Some(OutputWarning::new(
            "STALE_LOCK_REMOVED",
            format!("Stale lock from PID {pid} was removed"),
        )),
        LockOutcome::Acquired => None,
    };
    let leading: Vec<OutputWarning> = stale
        .into_iter()
        .chain(notice.map(counter_notice_warning))
        .collect();
    warnings.splice(0..0, leading);
}

#[derive(Debug, Serialize)]
pub struct CommandOutput<T: Serialize> {
    pub success: bool,
    pub action: String,
    pub workspace: String,
    pub data: T,
    pub graph_health: GraphHealth,
    pub errors: Vec<OutputError>,
    pub warnings: Vec<OutputWarning>,
}

impl<T: Serialize> CommandOutput<T> {
    pub fn ok(action: impl Into<String>, workspace: impl Into<String>, data: T) -> Self {
        Self {
            success: true,
            action: action.into(),
            workspace: workspace.into(),
            data,
            graph_health: GraphHealth {
                valid_dag: true,
                orphan_nodes_count: 0,
            },
            errors: vec![],
            warnings: vec![],
        }
    }

    pub fn with_health(mut self, health: GraphHealth) -> Self {
        self.graph_health = health;
        self
    }

    pub fn with_warnings(mut self, warnings: Vec<OutputWarning>) -> Self {
        self.warnings = warnings;
        self
    }

    /// Canonical pretty JSON. If `data` cannot be serialized, returns the
    /// error contract (`INTERNAL_ERROR`) instead of panicking.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|e| {
            let fallback = serde_json::json!({
                "success": false,
                "action": self.action,
                "workspace": self.workspace,
                "data": null,
                "graph_health": { "valid_dag": true, "orphan_nodes_count": 0 },
                "errors": [{
                    "code": "INTERNAL_ERROR",
                    "detail": format!("output serialization failed: {e}"),
                }],
                "warnings": [],
            });
            // `Display` of a `Value` cannot fail; `{:#}` is the 2-space pretty form.
            format!("{fallback:#}")
        })
    }
}

pub fn error_output(
    action: impl Into<String>,
    workspace: impl Into<String>,
    errors: Vec<OutputError>,
) -> CommandOutput<()> {
    CommandOutput {
        success: false,
        action: action.into(),
        workspace: workspace.into(),
        data: (),
        graph_health: GraphHealth {
            valid_dag: true,
            orphan_nodes_count: 0,
        },
        errors,
        warnings: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A payload whose serialization always fails.
    struct Unserializable;

    impl Serialize for Unserializable {
        fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("boom"))
        }
    }

    #[test]
    fn to_json_falls_back_to_error_contract_when_serialization_fails() {
        let json = CommandOutput::ok("node_add", "ws \"q\"", Unserializable).to_json();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["success"], false);
        assert_eq!(value["action"], "node_add");
        assert_eq!(value["workspace"], "ws \"q\"");
        assert_eq!(value["data"], serde_json::Value::Null);
        assert_eq!(value["graph_health"]["valid_dag"], true);
        assert_eq!(value["graph_health"]["orphan_nodes_count"], 0);
        assert_eq!(value["errors"][0]["code"], "INTERNAL_ERROR");
        assert!(value["errors"][0]["detail"]
            .as_str()
            .unwrap()
            .contains("boom"));
        assert_eq!(value["warnings"], serde_json::json!([]));
        // Canonical key order, 2-space indent, like the regular output.
        let keys: Vec<&str> = value
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            keys,
            [
                "action",
                "data",
                "errors",
                "graph_health",
                "success",
                "warnings",
                "workspace"
            ]
        );
        assert!(json.starts_with("{\n  \""), "{json}");
    }

    #[test]
    fn to_json_regular_output_unchanged() {
        let json = CommandOutput::ok("status", "ws", 7).to_json();
        assert_eq!(
            json,
            serde_json::to_string_pretty(&CommandOutput::ok("status", "ws", 7)).unwrap()
        );
    }
}
