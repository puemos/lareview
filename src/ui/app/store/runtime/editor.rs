use std::path::PathBuf;

pub fn open_in_editor(editor_id: String, file_path: PathBuf, line_number: usize) {
    let Some((command, args)) =
        crate::infra::editor::editor_command_for_open(&editor_id, &file_path, line_number)
    else {
        eprintln!(
            "[editor] Could not resolve editor command for '{}'",
            editor_id
        );
        return;
    };

    crate::RUNTIME.get().unwrap().spawn(async move {
        let mut cmd = tokio::process::Command::new(command);
        cmd.args(args);

        match cmd.spawn() {
            Ok(mut child) => {
                let _ = child.wait().await;
            }
            Err(err) => {
                eprintln!("[editor] Failed to open file in editor: {err}");
            }
        }
    });
}
