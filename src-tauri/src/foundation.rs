use serde::Serialize;

use crate::platform;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FoundationStatus {
    product: &'static str,
    status: &'static str,
    platform: &'static str,
    scanning_implemented: bool,
    deletion_implemented: bool,
}

impl FoundationStatus {
    pub fn current() -> Self {
        Self {
            product: "DiskClearance",
            status: "Pre-implementation foundation",
            platform: platform::current_platform_name(),
            scanning_implemented: false,
            deletion_implemented: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FoundationStatus;

    #[test]
    fn foundation_does_not_claim_unimplemented_operations() {
        let status = FoundationStatus::current();
        assert!(!status.scanning_implemented);
        assert!(!status.deletion_implemented);
    }
}
