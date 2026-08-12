use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Operator {
    Single,
    And,
    Or,
    Mag,
    Xor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Logic {
    Sufficiency,
    Necessity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeStatus {
    Active,
    Broken,
    Superseded,
    NeedsReview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssumptionStatus {
    Valid,
    Invalid,
    NeedsReview,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assumption {
    pub id: String,
    pub status: AssumptionStatus,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: String,
    pub from: Vec<String>,
    pub to: String,
    pub operator: Operator,
    pub weight: Option<f64>,
    pub status: EdgeStatus,
    pub logic: Logic,
    #[serde(default)]
    pub assumptions: Vec<Assumption>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FeedbackLoopType {
    Positive,
    Negative,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackEdge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub loop_type: FeedbackLoopType,
    #[serde(default)]
    pub label: Option<String>,
}
