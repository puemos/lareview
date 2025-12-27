use std::path::{Path, PathBuf};

use crate::infra::shell::find_bin;

#[derive(Debug, Clone, Copy)]
enum EditorOpenStyle {
    FileColonLine,
    FlagFileColonLine(&'static str),
    FlagLineThenFile(&'static str),
}

struct EditorDefinition {
    id: &'static str,
    label: &'static str,
    command: &'static str,
    extra_args: &'static [&'static str],
    open_style: EditorOpenStyle,
}

#[derive(Debug, Clone)]
pub struct EditorCandidate {
    pub id: &'static str,
    pub label: &'static str,
    pub path: PathBuf,
}

const EDITOR_DEFINITIONS: &[EditorDefinition] = &[
    EditorDefinition {
        id: "vscode",
        label: "Visual Studio Code",
        command: "code",
        extra_args: &["-r"],
        open_style: EditorOpenStyle::FlagFileColonLine("-g"),
    },
    EditorDefinition {
        id: "vscode-insiders",
        label: "VS Code Insiders",
        command: "code-insiders",
        extra_args: &["-r"],
        open_style: EditorOpenStyle::FlagFileColonLine("-g"),
    },
    EditorDefinition {
        id: "cursor",
        label: "Cursor",
        command: "cursor",
        extra_args: &[],
        open_style: EditorOpenStyle::FlagFileColonLine("-g"),
    },
    EditorDefinition {
        id: "sublime",
        label: "Sublime Text",
        command: "subl",
        extra_args: &[],
        open_style: EditorOpenStyle::FileColonLine,
    },
    EditorDefinition {
        id: "zed",
        label: "Zed",
        command: "zed",
        extra_args: &[],
        open_style: EditorOpenStyle::FileColonLine,
    },
    EditorDefinition {
        id: "idea",
        label: "IntelliJ IDEA",
        command: "idea",
        extra_args: &[],
        open_style: EditorOpenStyle::FlagLineThenFile("--line"),
    },
    EditorDefinition {
        id: "pycharm",
        label: "PyCharm",
        command: "pycharm",
        extra_args: &[],
        open_style: EditorOpenStyle::FlagLineThenFile("--line"),
    },
    EditorDefinition {
        id: "webstorm",
        label: "WebStorm",
        command: "webstorm",
        extra_args: &[],
        open_style: EditorOpenStyle::FlagLineThenFile("--line"),
    },
    EditorDefinition {
        id: "goland",
        label: "GoLand",
        command: "goland",
        extra_args: &[],
        open_style: EditorOpenStyle::FlagLineThenFile("--line"),
    },
    EditorDefinition {
        id: "clion",
        label: "CLion",
        command: "clion",
        extra_args: &[],
        open_style: EditorOpenStyle::FlagLineThenFile("--line"),
    },
    EditorDefinition {
        id: "rider",
        label: "Rider",
        command: "rider",
        extra_args: &[],
        open_style: EditorOpenStyle::FlagLineThenFile("--line"),
    },
    EditorDefinition {
        id: "rubymine",
        label: "RubyMine",
        command: "rubymine",
        extra_args: &[],
        open_style: EditorOpenStyle::FlagLineThenFile("--line"),
    },
    EditorDefinition {
        id: "phpstorm",
        label: "PhpStorm",
        command: "phpstorm",
        extra_args: &[],
        open_style: EditorOpenStyle::FlagLineThenFile("--line"),
    },
];

pub fn list_available_editors() -> Vec<EditorCandidate> {
    let mut editors = Vec::new();
    for def in EDITOR_DEFINITIONS {
        if let Some(path) = find_bin(def.command) {
            editors.push(EditorCandidate {
                id: def.id,
                label: def.label,
                path,
            });
        }
    }
    editors
}

pub fn is_editor_available(editor_id: &str) -> bool {
    editor_definition(editor_id)
        .and_then(|def| find_bin(def.command))
        .is_some()
}

pub fn editor_label(editor_id: &str) -> Option<&'static str> {
    editor_definition(editor_id).map(|def| def.label)
}

pub fn editor_command_for_open(
    editor_id: &str,
    file_path: &Path,
    line_number: usize,
) -> Option<(PathBuf, Vec<String>)> {
    let def = editor_definition(editor_id)?;
    let command_path = find_bin(def.command)?;
    let mut args = Vec::new();
    let file_str = file_path.to_string_lossy().to_string();
    let line_str = line_number.to_string();

    for extra in def.extra_args {
        args.push((*extra).to_string());
    }

    match def.open_style {
        EditorOpenStyle::FileColonLine => {
            args.push(format!("{file_str}:{line_str}"));
        }
        EditorOpenStyle::FlagFileColonLine(flag) => {
            args.push(flag.to_string());
            args.push(format!("{file_str}:{line_str}"));
        }
        EditorOpenStyle::FlagLineThenFile(flag) => {
            args.push(flag.to_string());
            args.push(line_str);
            args.push(file_str);
        }
    }

    Some((command_path, args))
}

fn editor_definition(editor_id: &str) -> Option<&'static EditorDefinition> {
    EDITOR_DEFINITIONS.iter().find(|def| def.id == editor_id)
}
