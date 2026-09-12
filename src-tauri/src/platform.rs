//! Platform capabilities stay behind this boundary.

pub trait PlatformAdapter {
    fn platform_name(&self) -> &'static str;
}

#[cfg(target_os = "macos")]
pub struct MacOsAdapter;

#[cfg(target_os = "macos")]
impl PlatformAdapter for MacOsAdapter {
    fn platform_name(&self) -> &'static str {
        "macOS"
    }
}

#[cfg(target_os = "macos")]
pub fn current_platform_name() -> &'static str {
    MacOsAdapter.platform_name()
}

#[cfg(target_os = "windows")]
pub struct WindowsAdapter;

#[cfg(target_os = "windows")]
impl PlatformAdapter for WindowsAdapter {
    fn platform_name(&self) -> &'static str {
        "Windows"
    }
}

#[cfg(target_os = "windows")]
pub fn current_platform_name() -> &'static str {
    WindowsAdapter.platform_name()
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn current_platform_name() -> &'static str {
    "unsupported"
}

#[cfg(test)]
mod tests {
    use super::PlatformAdapter;

    struct TestAdapter;

    impl PlatformAdapter for TestAdapter {
        fn platform_name(&self) -> &'static str {
            "test"
        }
    }

    #[test]
    fn platform_contract_is_independent_of_the_os() {
        assert_eq!(TestAdapter.platform_name(), "test");
    }
}
