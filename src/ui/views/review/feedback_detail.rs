use crate::ui::app::ui_memory::with_ui_memory_mut;
use crate::ui::app::{Action, LaReviewApp, ReviewAction};
use crate::ui::components::DiffAction;
use crate::ui::spacing;
use crate::ui::theme::current_theme;

use eframe::egui;
use egui::epaint::MarginF32;
use unidiff::PatchSet;

use crate::ui::views::review::feedback::{
    render_comment_list, render_feedback_context, render_feedback_header, render_reply_composer,
};

#[allow(dead_code)]
pub struct FeedbackDetailView {
    pub task_id: String,
    pub feedback_id: Option<String>,
    pub file_path: Option<String>,
    pub line_number: Option<u32>,
    pub side: Option<crate::domain::FeedbackSide>,
}

impl LaReviewApp {
    #[allow(dead_code)]
    pub(crate) fn render_feedback_detail(&mut self, ui: &mut egui::Ui, view: &FeedbackDetailView) {
        if ui.available_width() < 50.0 {
            return;
        }

        let theme = current_theme();

        let feedback = view
            .feedback_id
            .as_ref()
            .and_then(|id| self.state.domain.feedbacks.iter().find(|t| &t.id == id))
            .cloned();

        let feedback = if feedback.is_none() {
            self.state
                .domain
                .feedbacks
                .iter()
                .find(|t| {
                    t.task_id.as_ref() == Some(&view.task_id)
                        && t.anchor.as_ref().and_then(|a| a.file_path.as_ref())
                            == view.file_path.as_ref()
                        && t.anchor.as_ref().and_then(|a| a.line_number) == view.line_number
                })
                .cloned()
        } else {
            feedback
        };

        let feedback_id = feedback.as_ref().map(|t| t.id.clone());
        let side = view.side.or_else(|| {
            feedback
                .as_ref()
                .and_then(|f| f.anchor.as_ref().and_then(|a| a.side))
        });
        let comments = feedback_id
            .as_ref()
            .and_then(|id| self.state.domain.feedback_comments.get(id))
            .cloned()
            .unwrap_or_default();

        egui::Frame::NONE
            .inner_margin(MarginF32 {
                left: spacing::SPACING_XL,
                right: spacing::SPACING_XL,
                top: spacing::SPACING_LG,
                bottom: 0.0,
            })
            .show(ui, |ui| {
                // Get draft key - use feedback_id if exists, otherwise synthesize from context
                let draft_key = crate::ui::app::ui_memory::UiMemory::feedback_draft_key(
                    feedback_id.as_deref(),
                    &view.task_id,
                    view.file_path.as_deref(),
                    view.line_number,
                );

                let sent_url = feedback_id
                    .as_ref()
                    .and_then(|id| self.state.domain.feedback_links.get(id))
                    .map(|l| l.provider_root_comment_id.clone());

                if let Some(action) = render_feedback_header(
                    ui,
                    feedback.as_ref(),
                    sent_url,
                    &theme,
                    &view.task_id,
                    &draft_key,
                ) {
                    self.dispatch(Action::Review(action));
                }

                let is_github_review = self
                    .state
                    .ui
                    .selected_review_id
                    .as_ref()
                    .and_then(|id| self.state.domain.reviews.iter().find(|r| &r.id == id))
                    .map(|r| matches!(r.source, crate::domain::ReviewSource::GitHubPr { .. }))
                    .unwrap_or(false);

                if is_github_review {
                    // status/link handled in header actions row
                }

                let diff_snippet =
                    if let (Some(path), Some(line)) = (view.file_path.as_ref(), view.line_number) {
                        self.feedback_diff_snippet(&view.task_id, path, line)
                    } else {
                        None
                    };

                let action = render_feedback_context(
                    ui,
                    feedback.as_ref(),
                    view.file_path.as_ref(),
                    view.line_number,
                    diff_snippet,
                    &theme,
                );
                if let DiffAction::OpenInEditor {
                    file_path,
                    line_number,
                } = action
                {
                    self.dispatch(Action::Review(ReviewAction::OpenInEditor {
                        file_path,
                        line_number,
                    }));
                }
            });

        ui.add_space(spacing::SPACING_MD);
        ui.separator();
        ui.add_space(spacing::SPACING_XL);

        // 3. Timeline & Input
        egui::Frame::NONE
            .inner_margin(egui::Margin {
                left: spacing::SPACING_XL as i8,
                right: spacing::SPACING_XL as i8,
                top: 0,
                bottom: spacing::SPACING_XL as i8,
            })
            .show(ui, |ui| {
                let draft_key = crate::ui::app::ui_memory::UiMemory::feedback_draft_key(
                    feedback_id.as_deref(),
                    &view.task_id,
                    view.file_path.as_deref(),
                    view.line_number,
                );

                ui.vertical(|ui| {
                    if let Some(action) = render_comment_list(ui, &comments, &theme) {
                        self.dispatch(Action::Review(action));
                    }

                    // 4. Input Area
                    ui.add_space(spacing::SPACING_MD);
                    if let Some(action) = render_reply_composer(
                        ui,
                        &view.task_id,
                        feedback_id.clone(),
                        view.file_path.clone(),
                        view.line_number,
                        side,
                        &draft_key,
                    ) {
                        // Clear drafts after sending
                        if matches!(action, ReviewAction::CreateFeedbackComment { .. }) {
                            with_ui_memory_mut(ui.ctx(), |mem| {
                                mem.feedback_drafts.remove(&draft_key);
                            });
                        }
                        self.dispatch(Action::Review(action));
                    }
                });
            });
    }

    fn feedback_diff_snippet(
        &self,
        task_id: &str,
        file_path: &str,
        line_number: u32,
    ) -> Option<String> {
        let tasks = self.state.tasks();
        let task = tasks.iter().find(|t| t.id == task_id)?;
        let run = self
            .state
            .domain
            .runs
            .iter()
            .find(|r| r.id == task.run_id)?;
        diff_snippet_for_anchor(&run.diff_text, file_path, line_number)
    }
}

fn diff_snippet_for_anchor(diff_text: &str, file_path: &str, line_number: u32) -> Option<String> {
    let diff_text = diff_text.trim();
    if diff_text.is_empty() {
        return None;
    }

    let mut patch_set = PatchSet::new();
    if patch_set.parse(diff_text).is_err() {
        return None;
    }

    // Normalize path: strip a/ or b/ prefixes
    let normalize_path = |path: &str| -> String {
        path.strip_prefix("b/")
            .or_else(|| path.strip_prefix("a/"))
            .unwrap_or(path)
            .to_string()
    };

    // Extract basename for fallback matching
    let basename = |path: &str| -> String { path.rsplit('/').next().unwrap_or(path).to_string() };

    let normalized_file_path = normalize_path(file_path);
    let file_basename = basename(&normalized_file_path);

    // Check if two paths match (exact, suffix, or basename)
    let paths_match = |diff_path: &str, anchor_path: &str, anchor_basename: &str| -> bool {
        // Exact match
        if diff_path == anchor_path {
            return true;
        }
        // Anchor is a suffix of diff path (e.g., "package.json" matches "libs/package.json")
        if diff_path.ends_with(&format!("/{}", anchor_path)) {
            return true;
        }
        // Diff path is a suffix of anchor (e.g., "libs/package.json" matches "package.json" in diff)
        if anchor_path.ends_with(&format!("/{}", diff_path)) {
            return true;
        }
        // Basename match as last resort
        if basename(diff_path) == anchor_basename {
            return true;
        }
        false
    };

    for file in patch_set.files() {
        let new_path = normalize_path(&file.target_file);
        let old_path = normalize_path(&file.source_file);

        let file_matches = paths_match(&new_path, &normalized_file_path, &file_basename)
            || paths_match(&old_path, &normalized_file_path, &file_basename);

        if !file_matches {
            continue;
        }

        let display_path = if file.target_file == "/dev/null" {
            old_path.as_str()
        } else {
            new_path.as_str()
        };

        for hunk in file.hunks() {
            let lines = hunk.lines();
            let target_idx = lines.iter().position(|line| {
                line.source_line_no == Some(line_number as usize)
                    || line.target_line_no == Some(line_number as usize)
            });
            let Some(target_idx) = target_idx else {
                continue;
            };

            let start_idx = target_idx.saturating_sub(4);
            let end_idx = (target_idx + 4).min(lines.len().saturating_sub(1));
            let slice = &lines[start_idx..=end_idx];

            let mut old_start = None;
            let mut new_start = None;
            let mut old_count = 0usize;
            let mut new_count = 0usize;

            for line in slice {
                if line.source_line_no.is_some() {
                    old_start = old_start.or(line.source_line_no);
                    old_count += 1;
                }
                if line.target_line_no.is_some() {
                    new_start = new_start.or(line.target_line_no);
                    new_count += 1;
                }
            }

            let old_start = old_start.unwrap_or(hunk.source_start);
            let new_start = new_start.unwrap_or(hunk.target_start);
            let section_header = if hunk.section_header.is_empty() {
                String::new()
            } else {
                format!(" {}", hunk.section_header)
            };

            let mut snippet = format!(
                "diff --git a/{path} b/{path}\n--- a/{path}\n+++ b/{path}\n",
                path = display_path
            );
            snippet.push_str(&format!(
                "@@ -{},{} +{},{} @@{}\n",
                old_start, old_count, new_start, new_count, section_header
            ));
            for line in slice {
                snippet.push_str(&format!("{}\n", line));
            }
            return Some(snippet);
        }

        // File matched but line not found in any hunk - don't fall through to other files
        return None;
    }

    None
}

#[cfg(test)]
mod tests {
    use super::diff_snippet_for_anchor;

    #[test]
    fn diff_snippet_does_not_fall_through_to_other_files() {
        let diff = r###"diff --git a/pnpm-lock.yaml b/pnpm-lock.yaml
--- a/pnpm-lock.yaml
+++ b/pnpm-lock.yaml
@@ -100,3 +100,3 @@
-lockfileVersion: 6
+lockfileVersion: 7
diff --git a/src/App.tsx b/src/App.tsx
--- a/src/App.tsx
+++ b/src/App.tsx
@@ -1,3 +1,6 @@
+import React from "react";
 const App = () => null;
"###;

        let snippet = diff_snippet_for_anchor(diff, "pnpm-lock.yaml", 3);
        assert!(snippet.is_none());
    }

    #[test]
    fn diff_snippet_limits_context_window() {
        let diff = r###"diff --git a/src/foo.rs b/src/foo.rs
--- a/src/foo.rs
+++ b/src/foo.rs
@@ -1,12 +1,12 @@
 line1
 line2
 line3
 line4
 line5
 line6
 line7
 line8
 line9
 line10
 line11
 line12
"###;

        let snippet =
            diff_snippet_for_anchor(diff, "src/foo.rs", 10).expect("snippet should exist");
        assert!(snippet.contains("\n line6\n"));
        assert!(snippet.contains("\n line12\n"));
        assert!(!snippet.contains("\n line5\n"));

        let lines: Vec<&str> = snippet.lines().collect();
        let hunk_idx = lines
            .iter()
            .position(|line| line.starts_with("@@"))
            .expect("hunk header missing");
        let body = &lines[hunk_idx + 1..];
        assert_eq!(body.len(), 7);
    }

    #[test]
    fn test_diff_snippet_path_matching() {
        let diff = "diff --git a/libs/ui/src/button.rs b/libs/ui/src/button.rs\n--- a/libs/ui/src/button.rs\n+++ b/libs/ui/src/button.rs\n@@ -1,1 +1,1 @@\n-old\n+new";

        // Exact match
        assert!(diff_snippet_for_anchor(diff, "libs/ui/src/button.rs", 1).is_some());
        // Suffix match
        assert!(diff_snippet_for_anchor(diff, "button.rs", 1).is_some());
        // Basename match
        assert!(diff_snippet_for_anchor(diff, "button", 1).is_none()); // Basename match only works for full component
    }
}
