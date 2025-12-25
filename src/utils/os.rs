use std::error::Error;
use std::ffi::c_void;

#[cfg(target_os = "windows")]
mod platform {
    use super::*;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Dwm::{DWMWINDOWATTRIBUTE, DwmSetWindowAttribute};

    const DWMWA_WINDOW_CORNER_PREFERENCE: DWMWINDOWATTRIBUTE = DWMWINDOWATTRIBUTE(33);
    const DWMWCP_ROUND: u32 = 2;

    pub fn apply_native_rounded_corners(ptr: *mut c_void) -> Result<(), Box<dyn Error>> {
        if ptr.is_null() {
            return Err("Null HWND pointer".into());
        }

        let hwnd = HWND(ptr as _);

        unsafe {
            let hr = DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                &DWMWCP_ROUND as *const _ as *const _,
                size_of::<u32>() as u32,
            );

            if hr.is_ok() {
                Ok(())
            } else {
                Err(format!(
                    "DwmSetWindowAttribute failed: {:?}. Possibly not Windows 11+.",
                    hr
                )
                .into())
            }
        }
    }
}

#[cfg(target_os = "macos")]
#[allow(deprecated, unexpected_cfgs)]
mod platform {
    use super::*;
    use cocoa::base::{YES, id, nil};
    use objc::{msg_send, sel, sel_impl};

    #[allow(deprecated, unexpected_cfgs)]
    pub fn apply_native_rounded_corners(ptr: *mut c_void) -> Result<(), Box<dyn Error>> {
        if ptr.is_null() {
            return Err("Null NSView pointer".into());
        }

        unsafe {
            let ns_view: id = ptr as id;
            if ns_view == nil {
                return Err("Invalid NSView (nil)".into());
            }

            // Get NSWindow from NSView
            let ns_window: id = msg_send![ns_view, window];
            if ns_window == nil {
                return Err("Failed to obtain NSWindow from NSView".into());
            }

            // Transparent titlebar
            let _: () = msg_send![ns_window, setTitlebarAppearsTransparent: YES];

            // Hide title
            let _: () = msg_send![ns_window, setTitleVisibility: 1u64]; // NSWindowTitleHidden = 1

            // Rounded contentView layer
            let content_view: id = msg_send![ns_window, contentView];
            if content_view != nil {
                let _: () = msg_send![content_view, setWantsLayer: YES];
                let layer: id = msg_send![content_view, layer];
                if layer != nil {
                    let _: () = msg_send![layer, setCornerRadius: 12.0f64];
                    let _: () = msg_send![layer, setMasksToBounds: YES];
                }
            }

            Ok(())
        }
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use super::*;

    pub fn apply_native_rounded_corners(_ptr: *mut c_void) -> Result<(), Box<dyn Error>> {
        // Rounded corners are not supported on Linux for now.
        println!("ℹ️ Linux: Rounded corners are currently not supported.");
        Ok(())
    }
}

/// Try to apply native rounded corners to the OS window represented by `ptr`.
///
/// The pointer type and semantics depend on the platform (e.g. HWND on Windows,
/// NSView on macOS). Returns `Ok(())` on success or an error with details.
pub fn apply_native_rounded_corners(ptr: *mut c_void) -> Result<(), Box<dyn Error>> {
    #[cfg(target_os = "windows")]
    {
        platform::apply_native_rounded_corners(ptr)
    }
    #[cfg(target_os = "macos")]
    {
        platform::apply_native_rounded_corners(ptr)
    }
    #[cfg(target_os = "linux")]
    {
        platform::apply_native_rounded_corners(ptr)
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = ptr;
        Err("Native rounded corners not supported on this platform".into())
    }
}

/// Returns true if we have a native strategy for rounded corners on this platform.
pub fn supports_native_rounded_corners() -> bool {
    #[cfg(target_os = "windows")]
    {
        true
    }
    #[cfg(target_os = "macos")]
    {
        true
    }
    #[cfg(target_os = "linux")]
    {
        false
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supports_native_rounded_corners() {
        // Just verify it doesn't crash and returns a consistent value for the platform
        let _ = supports_native_rounded_corners();
    }

    #[test]
    fn test_apply_native_rounded_corners_null() {
        let res = apply_native_rounded_corners(std::ptr::null_mut());
        assert!(res.is_err());
    }
}
