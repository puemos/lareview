use std::collections::HashSet;

pub fn normalize_task_path(path: &str) -> String {
    path.trim()
        .trim_start_matches("./")
        .trim_start_matches("a/")
        .trim_start_matches("b/")
        .to_string()
}

pub fn extract_changed_files(diff_text: &str) -> HashSet<String> {
    let mut files = HashSet::new();
    for line in diff_text.lines() {
        if let Some(rest) = line.strip_prefix("diff --git ") {
            let mut parts = rest.split_whitespace();
            let a_path = parts.next().unwrap_or("");
            let b_path = parts.next().unwrap_or("");
            if b_path.is_empty() {
                continue;
            }

            let b_clean = normalize_task_path(b_path);
            if b_clean == "dev/null" || b_clean == "/dev/null" {
                let a_clean = normalize_task_path(a_path);
                if !a_clean.is_empty() && a_clean != "dev/null" && a_clean != "/dev/null" {
                    files.insert(a_clean);
                }
            } else if !b_clean.is_empty() {
                files.insert(b_clean);
            }
        }
    }
    files
}
