use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::boundary::error::CommandError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ActionMode {
    Trash,
    PermanentDelete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ExecutePlanArgs {
    pub plan_id: String,
    pub action_mode: ActionMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionSummary {
    pub plan_id: String,
    pub action_mode: ActionMode,
    pub succeeded_items: u64,
    pub failed_items: u64,
    pub bytes_freed: u64,
}

#[tauri::command]
pub fn execute_plan(_args: ExecutePlanArgs) -> Result<ExecutionSummary, CommandError> {
    // Destructive plan execution is planned for subsequent implementation.
    Err(CommandError::Unsupported {
        feature: "execute_plan".into(),
        reason: "Plan execution is not implemented in this milestone".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_destructive_execute_plan_signature_does_not_accept_path() {
        let args = ExecutePlanArgs {
            plan_id: "plan-xyz-789".into(),
            action_mode: ActionMode::Trash,
        };

        // Assert serialized representation only contains planId and actionMode
        let value = serde_json::to_value(&args).expect("to_value");
        let map = value.as_object().expect("object");
        assert!(map.contains_key("planId"));
        assert!(map.contains_key("actionMode"));
        assert_eq!(
            map.len(),
            2,
            "ExecutePlanArgs must strictly only contain planId and actionMode; paths are strictly prohibited"
        );

        // Verify TypeScript declaration does not contain 'path'
        let cfg = ts_rs::Config::default();
        let decl = ExecutePlanArgs::decl(&cfg);
        assert!(
            !decl.to_lowercase().contains("path"),
            "ExecutePlanArgs TypeScript signature must not contain path fields"
        );
        assert!(decl.contains("planId: string"));
        assert!(decl.contains("actionMode: ActionMode"));
    }
}
