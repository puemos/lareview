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
