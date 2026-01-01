use eframe::Frame;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::{ffi::c_void, sync::Once};

use crate::utils::os::apply_native_rounded_corners;

/// Applies native rounded corners to the window if supported on the current platform.
/// This should be called once after the window is created.
///
/// For the main window, use this function with the `Frame` from `eframe::App::update`.
pub fn apply_rounded_corners(frame: &Frame) {
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        if let Ok(window_handle) = frame.window_handle() {
            apply_rounded_corners_from_handle(window_handle);
        }
    });
}

/// Internal helper to apply rounded corners from a window handle
fn apply_rounded_corners_from_handle(window_handle: raw_window_handle::WindowHandle) {
    let handle: RawWindowHandle = window_handle.as_raw();

    let ptr: Option<*mut c_void> = match handle {
        RawWindowHandle::Win32(h) => Some(h.hwnd.get() as *mut _),
        RawWindowHandle::AppKit(h) => Some(h.ns_view.as_ptr() as *mut _),
        RawWindowHandle::Xlib(h) => Some(h.window as *mut _),
        RawWindowHandle::Wayland(h) => Some(h.surface.as_ptr() as *mut _),
        _ => {
            println!(
                "ℹ️ Platform: Native rounded corners not supported for this window handle type: {:?}",
                handle
            );
            None
        }
    };

    if let Some(native_ptr) = ptr {
        match apply_native_rounded_corners(native_ptr) {
            Ok(_) => (),
            Err(e) => eprintln!("⚠️ Failed to apply native rounded corners: {}", e),
        }
    }
}
