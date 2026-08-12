use serde::Serialize;

use crate::link::types::{AssumptionStatus, Edge, EdgeStatus, Operator};
use crate::output::{CommandOutput, GraphHealth, OutputError, OutputWarning};
use crate::storage::{LockOutcome, Storage};
use crate::validate::check_dag;

fn parse_operator(s: &str) -> Option<Operator> {
    match s.to_uppercase().as_str() {
        "SINGLE" => Some(Operator::Single),
        "AND" => Some(Operator::And),
        "OR" => Some(Operator::Or),
        "MAG" => Some(Operator::Mag),
        "XOR" => Some(Operator::Xor),
        _ => None,
    }
}

fn stale_lock_warning(outcome: &LockOutcome) -> Option<OutputWarning> {
    match outcome {
        LockOutcome::StaleLockRemoved { pid } => Some(OutputWarning::new(
            "STALE_LOCK_REMOVED",
            format!("Stale lock from PID {} was removed", pid),
        )),
        LockOutcome::Acquired => None,
    }
}

/// Data returned by `link reverse`.
#[derive(Debug, Serialize)]
pub struct LinkReverseData {
    pub link_id: String,
    pub tree_id: String,
    pub new_from: Vec<String>,
    pub new_to: String,
}

/// Data returned by `link move`.
#[derive(Debug, Serialize)]
pub struct LinkMoveData {
    pub link_id: String,
    pub tree_id: String,
}

/// Data returned by `link insert-between`.
#[derive(Debug, Serialize)]
pub struct LinkInsertBetweenData {
    pub removed_link: String,
    pub created_links: Vec<String>,
    pub tree_id: String,
}

/// Data returned by `link group`.
#[derive(Debug, Serialize)]
pub struct LinkGroupData {
    pub created_link: String,
    pub removed_links: Vec<String>,
    pub tree_id: String,
}

/// Data returned by `link dissolve`.
#[derive(Debug, Serialize)]
pub struct LinkDissolveData {
    pub created_links: Vec<String>,
    pub removed_link: String,
    pub tree_id: String,
}

/// Data returned by `link split`.
#[derive(Debug, Serialize)]
pub struct LinkSplitData {
    pub extracted_link: String,
    pub original_link: String,
    pub tree_id: String,
}

/// Data returned by `link reoperator`.
#[derive(Debug, Serialize)]
pub struct LinkReoperatorData {
    pub link_id: String,
    pub old_operator: Operator,
    pub new_operator: Operator,
    pub tree_id: String,
}

/// Data returned by `link add-cause`.
#[derive(Debug, Serialize)]
pub struct LinkAddCauseData {
    pub link_id: String,
    pub added_node: String,
    pub tree_id: String,
}

/// Data returned by `link rm-cause`.
#[derive(Debug, Serialize)]
pub struct LinkRmCauseData {
    pub link_id: String,
    pub removed_node: String,
    pub new_operator: Operator,
    pub tree_id: String,
}
