use serde::Serialize;
use std::collections::BTreeMap;

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

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("serialization should not fail")
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
