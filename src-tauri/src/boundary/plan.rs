use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::boundary::destructive::ActionMode;
use crate::boundary::error::CommandError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BuildPlanArgs {
    pub session_id: String,
    pub finding_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ReviewPlanHeader {
    pub plan_id: String,
    pub session_id: String,
    pub item_count: u64,
    pub total_bytes: u64,
    pub created_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FetchPlanArgs {
    pub plan_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PlanItemSummary {
    pub item_id: String,
    pub original_path: String,
    pub size_bytes: u64,
    pub class_name: String,
    pub action_name: String,
    pub recoverable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ReviewPlan {
    pub plan_id: String,
    pub session_id: String,
    pub items: Vec<PlanItemSummary>,
    pub default_action_mode: ActionMode,
    pub created_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RevalidatePlanArgs {
    pub plan_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RevalidationResult {
    pub plan_id: String,
    pub is_valid: bool,
    pub stale_item_ids: Vec<String>,
}

#[tauri::command]
pub fn build_plan(_args: BuildPlanArgs) -> Result<ReviewPlanHeader, CommandError> {
    // Review plan building is planned for subsequent implementation.
    Err(CommandError::Unsupported {
        feature: "build_plan".into(),
        reason: "Review plan construction is not implemented in this milestone".into(),
    })
}

#[tauri::command]
pub fn fetch_plan(_args: FetchPlanArgs) -> Result<ReviewPlan, CommandError> {
    // Review plan retrieval is planned for subsequent implementation.
    Err(CommandError::Unsupported {
        feature: "fetch_plan".into(),
        reason: "Review plan retrieval is not implemented in this milestone".into(),
    })
}

#[tauri::command]
pub fn revalidate_plan(_args: RevalidatePlanArgs) -> Result<RevalidationResult, CommandError> {
    // Review plan revalidation is planned for subsequent implementation.
    Err(CommandError::Unsupported {
        feature: "revalidate_plan".into(),
        reason: "Review plan revalidation is not implemented in this milestone".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_commands_return_typed_unsupported_error() {
        let err = CommandError::Unsupported {
            feature: "build_plan".into(),
            reason: "Review plan construction is not implemented in this milestone".into(),
        };
        assert_eq!(
            err,
            CommandError::Unsupported {
                feature: "build_plan".into(),
                reason: "Review plan construction is not implemented in this milestone".into(),
            }
        );
    }
}
