use std::io::Write;
use std::process::{Command, Stdio};
use which::which;

pub fn d2_to_svg(d2_code: &str, is_dark_mode: bool) -> Result<String, String> {
    let d2_path = which("d2").map_err(|_| {
        "D2 executable not found. Please install D2 and ensure it is in your PATH.".to_string()
    })?;

    let mut command = Command::new(&d2_path);

    // Input from stdin, output to stdout
    command.arg("-");
    command.arg("-");

    // Set theme based on mode
    if is_dark_mode {
        command.arg("--theme").arg("200"); // Dark Mauve theme
    } else {
        command.arg("--theme").arg("0"); // Default light theme
    }

    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    let stdin = child.stdin.as_mut().ok_or("Failed to open stdin")?;
    stdin
        .write_all(d2_code.as_bytes())
        .map_err(|e| e.to_string())?;

    let output = child.wait_with_output().map_err(|e| e.to_string())?;

    if output.status.success() {
        let svg = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;
        Ok(svg)
    } else {
        let error_message = String::from_utf8(output.stderr).map_err(|e| e.to_string())?;
        Err(error_message)
    }
}

/// Validate D2 code by running `d2 fmt --check`.
/// Returns Ok(()) if valid or if d2 is not installed (skip validation).
/// Returns Err(msg) if d2 is installed and validation fails.
pub fn validate_d2(d2_code: &str) -> Result<(), String> {
    let d2_path = match which("d2") {
        Ok(path) => path,
        Err(_) => return Ok(()), // Skip validation if d2 is not installed
    };

    let mut command = Command::new(d2_path);
    command.arg("fmt");
    command.arg("-"); // Input from stdin

    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::null()) // We don't care about formatted output
        .stderr(Stdio::piped()) // We want errors
        .spawn()
        .map_err(|e| format!("Failed to spawn d2: {}", e))?;

    let stdin = child.stdin.as_mut().ok_or("Failed to open stdin")?;
    stdin
        .write_all(d2_code.as_bytes())
        .map_err(|e| format!("Failed to write to d2 stdin: {}", e))?;

    let output = child
        .wait_with_output()
        .map_err(|e| format!("Failed to wait for d2: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        // Capture stderr for the error message
        let error_message = String::from_utf8_lossy(&output.stderr).to_string();
        // If stderr is empty (sometimes happens with fmt check?), try to be helpful
        if error_message.trim().is_empty() {
            Err("D2 validation failed (invalid syntax)".to_string())
        } else {
            Err(format!("D2 validation failed: {}", error_message.trim()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_d2_success() {
        if which("d2").is_ok() {
            let valid_code = "x -> y";
            let result = validate_d2(valid_code);
            assert!(result.is_ok(), "Validation failed: {:?}", result.err());
        }
    }

    #[test]
    fn test_validate_d2_failure() {
        if which("d2").is_ok() {
            let invalid_code = "x -> {"; // Unclosed brace
            let result = validate_d2(invalid_code);
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("validation failed"));
        }
    }

    #[test]
    fn test_validate_d2_ok_if_missing() {
        if which("d2").is_err() {
            assert!(validate_d2("bad code").is_ok());
        }
    }
}
