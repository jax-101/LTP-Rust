use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryConfig {
    #[serde(default = "default_max_size_mb")]
    pub max_size_mb: u32,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_max_size_mb() -> u32 {
    5
}

fn default_enabled() -> bool {
    true
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            max_size_mb: default_max_size_mb(),
            enabled: default_enabled(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub name: String,
    #[serde(default)]
    pub history: HistoryConfig,
}
