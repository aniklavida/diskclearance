use serde::Serialize;

use crate::platform::{self, PlatformAdapter};

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
        Self::from_platform(platform::current_platform_adapter().as_ref())
    }

    pub fn from_platform(adapter: &dyn PlatformAdapter) -> Self {
        Self {
            product: "DiskClearance",
            status: "Pre-implementation foundation",
            platform: adapter.platform_name(),
            scanning_implemented: false,
            deletion_implemented: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FoundationStatus;
    use crate::platform::tests::TestAdapter;

    #[test]
    fn foundation_does_not_claim_unimplemented_operations() {
        let status = FoundationStatus::current();
        assert!(!status.scanning_implemented);
        assert!(!status.deletion_implemented);
    }

    #[test]
    fn foundation_status_from_adapter() {
        let adapter = TestAdapter::default();
        let status = FoundationStatus::from_platform(&adapter);
        assert_eq!(status.platform, "test");
        assert!(!status.scanning_implemented);
        assert!(!status.deletion_implemented);
    }
}
