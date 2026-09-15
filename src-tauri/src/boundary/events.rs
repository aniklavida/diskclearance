use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const EVENT_SCAN_PROGRESS: &str = "scan:progress";
pub const EVENT_VERIFIED_COUNT: &str = "scan:verified-count";
pub const EVENT_COVERAGE_WARNING: &str = "scan:warning";
pub const EVENT_EXECUTION_OUTCOME: &str = "execute:item-outcome";
pub const EVENT_TERMINAL_COMPLETION: &str = "scan:terminal";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ScanPhase {
    Discovering,
    Scanning,
    Classifying,
    Coalescing,
    Complete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgressPayload {
    pub session_id: String,
    pub phase: ScanPhase,
    pub current_scope: String,
    pub items_visited: u64,
    pub bytes_visited: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedCountPayload {
    pub session_id: String,
    pub category: String,
    pub verified_count: u64,
    pub verified_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CoverageWarningPayload {
    pub session_id: String,
    pub path: String,
    pub warning_code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionOutcomePayload {
    pub plan_id: String,
    pub item_id: String,
    pub success: bool,
    pub error: Option<String>,
    pub bytes_freed: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum TerminalStatus {
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TerminalCompletionPayload {
    pub session_id: String,
    pub status: TerminalStatus,
    pub total_items: u64,
    pub total_bytes: u64,
    pub summary_message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_payload_serialization() {
        let progress = ScanProgressPayload {
            session_id: "scan-1".into(),
            phase: ScanPhase::Scanning,
            current_scope: "Library/Caches".into(),
            items_visited: 42,
            bytes_visited: 1048576,
        };
        let json = serde_json::to_string(&progress).expect("serialize");
        assert!(json.contains("\"sessionId\":\"scan-1\""));
        assert!(json.contains("\"phase\":\"scanning\""));
        assert!(json.contains("\"currentScope\":\"Library/Caches\""));
        assert!(json.contains("\"itemsVisited\":42"));
        assert!(json.contains("\"bytesVisited\":1048576"));
    }
}
