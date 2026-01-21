//! Platform detection module with WSL support.
//!
//! Provides runtime detection of the current platform, including
//! distinguishing between native Linux and Linux running under WSL.

use std::sync::OnceLock;

/// Represents the detected platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    MacOS,
    Linux,
    LinuxWsl,
    Windows,
    Unknown,
}

/// Cached platform detection result.
static PLATFORM: OnceLock<Platform> = OnceLock::new();

/// Returns true if running inside WSL (Windows Subsystem for Linux).
pub fn is_wsl() -> bool {
    matches!(current_platform(), Platform::LinuxWsl)
}

/// Returns the current platform with WSL detection.
pub fn current_platform() -> Platform {
    *PLATFORM.get_or_init(detect_platform)
}

fn detect_platform() -> Platform {
    #[cfg(target_os = "macos")]
    {
        Platform::MacOS
    }
    #[cfg(target_os = "windows")]
    {
        Platform::Windows
    }
    #[cfg(target_os = "linux")]
    {
        if detect_wsl() {
            Platform::LinuxWsl
        } else {
            Platform::Linux
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Platform::Unknown
    }
}

/// Detects if running under WSL using multiple indicators.
#[cfg(target_os = "linux")]
fn detect_wsl() -> bool {
    // Method 1: Check WSL_DISTRO_NAME environment variable (most reliable)
    if std::env::var("WSL_DISTRO_NAME").is_ok() {
        return true;
    }

    // Method 2: Check for WSLInterop file
    if std::path::Path::new("/proc/sys/fs/binfmt_misc/WSLInterop").exists() {
        return true;
    }

    // Method 3: Check /proc/version for Microsoft/WSL indicators
    if let Ok(version) = std::fs::read_to_string("/proc/version") {
        let version_lower = version.to_lowercase();
        if version_lower.contains("microsoft") || version_lower.contains("wsl") {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_platform_returns_consistent_value() {
        let p1 = current_platform();
        let p2 = current_platform();
        assert_eq!(p1, p2);
    }

    #[test]
    fn test_is_wsl_matches_platform() {
        let platform = current_platform();
        assert_eq!(is_wsl(), platform == Platform::LinuxWsl);
    }
}
