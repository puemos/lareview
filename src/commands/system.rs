#[tauri::command]
pub fn copy_to_clipboard(text: String) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.set_text(text).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    use crate::infra::platform::{Platform, current_platform};

    match current_platform() {
        Platform::MacOS => {
            std::process::Command::new("open")
                .arg(&url)
                .spawn()
                .map_err(|e| e.to_string())?;
        }
        Platform::LinuxWsl => {
            // Try multiple methods in order of reliability
            // 1. wslview (from wslu package) - best option, handles URLs properly
            // 2. explorer.exe - built into Windows, reliable for URLs
            // 3. powershell.exe - fallback with proper URL handling

            let wslview_result = std::process::Command::new("wslview").arg(&url).spawn();

            if wslview_result.is_ok() {
                // wslview started successfully
            } else {
                // Try explorer.exe which handles URLs well
                let explorer_result = std::process::Command::new("explorer.exe").arg(&url).spawn();

                if explorer_result.is_err() {
                    // Final fallback: PowerShell with proper escaping
                    let ps_command = format!("Start-Process '{}'", url.replace("'", "''"));
                    std::process::Command::new("powershell.exe")
                        .args(["-NoProfile", "-Command", &ps_command])
                        .spawn()
                        .map_err(|e| {
                            format!(
                                "Failed to open URL in WSL: {}. \
                                 Install wslu package (sudo apt install wslu) for better support, \
                                 or ensure Windows interop is enabled.",
                                e
                            )
                        })?;
                }
            }
        }
        Platform::Linux => {
            std::process::Command::new("xdg-open")
                .arg(&url)
                .spawn()
                .map_err(|e| e.to_string())?;
        }
        Platform::Windows => {
            std::process::Command::new("cmd")
                .args(["/C", "start", &url])
                .spawn()
                .map_err(|e| e.to_string())?;
        }
        Platform::Unknown => {
            return Err("Unsupported platform for opening URLs".to_string());
        }
    }
    Ok(())
}
