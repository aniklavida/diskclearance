use serde::{Deserialize, Serialize};
use std::fmt;
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CommandError {
    PermissionDenied {
        path: Option<String>,
        reason: String,
    },
    PathVanished {
        path: String,
    },
    ChangedAfterReview {
        plan_id: String,
        details: String,
    },
    Unsupported {
        feature: String,
        reason: String,
    },
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PermissionDenied { path, reason } => match path {
                Some(p) => write!(f, "permission denied for '{p}': {reason}"),
                None => write!(f, "permission denied: {reason}"),
            },
            Self::PathVanished { path } => write!(f, "path vanished: {path}"),
            Self::ChangedAfterReview { plan_id, details } => {
                write!(f, "plan '{plan_id}' changed after review: {details}")
            }
            Self::Unsupported { feature, reason } => {
                write!(f, "feature '{feature}' is unsupported: {reason}")
            }
        }
    }
}

impl std::error::Error for CommandError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_error_discriminated_union_variants() {
        let err1 = CommandError::PermissionDenied {
            path: Some("/test/path".into()),
            reason: "Read denied".into(),
        };
        let err2 = CommandError::PathVanished {
            path: "/vanished/path".into(),
        };
        let err3 = CommandError::ChangedAfterReview {
            plan_id: "plan-123".into(),
            details: "Inode changed".into(),
        };
        let err4 = CommandError::Unsupported {
            feature: "scan".into(),
            reason: "Not yet active".into(),
        };

        // Assert JSON serialization uses kind discriminator and camelCase fields
        let json1 = serde_json::to_string(&err1).expect("serialize");
        assert!(json1.contains("\"kind\":\"permissionDenied\""));
        assert!(json1.contains("\"path\":\"/test/path\""));

        let json2 = serde_json::to_string(&err2).expect("serialize");
        assert!(json2.contains("\"kind\":\"pathVanished\""));

        let json3 = serde_json::to_string(&err3).expect("serialize");
        assert!(json3.contains("\"kind\":\"changedAfterReview\""));
        assert!(json3.contains("\"planId\":\"plan-123\""));

        let json4 = serde_json::to_string(&err4).expect("serialize");
        assert!(json4.contains("\"kind\":\"unsupported\""));

        // Assert TypeScript declaration contains the discriminated union with all 4 variants
        let cfg = ts_rs::Config::default();
        let decl = CommandError::decl(&cfg);
        assert!(decl.contains("\"permissionDenied\""));
        assert!(decl.contains("\"pathVanished\""));
        assert!(decl.contains("\"changedAfterReview\""));
        assert!(decl.contains("\"unsupported\""));
        assert!(decl.contains("planId"));
    }
}
