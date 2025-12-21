use super::format_timestamp;
use crate::domain::{Comment, ThreadImpact, ThreadStatus};
use crate::ui::app::{Action, LaReviewApp, ReviewAction};
use crate::ui::components::{PopupOption, popup_selector, render_diff_editor_full_view};
use crate::ui::spacing;
use crate::ui::theme::current_theme;
use eframe::egui;
use egui::Color32;
use egui::epaint::MarginF32;
use egui_phosphor::regular as icons;
use unidiff::PatchSet;

#[allow(dead_code)]
pub struct ThreadDetailView {
    pub task_id: String,
    pub thread_id: Option<String>,
    pub file_path: Option<String>,
    pub line_number: Option<u32>,
}

impl LaReviewApp {
    #[allow(dead_code)]
    pub(crate) fn render_thread_detail(&mut self, ui: &mut egui::Ui, view: &ThreadDetailView) {
        let theme = current_theme();

        let mut thread = view
            .thread_id
            .as_ref()
            .and_then(|id| self.state.threads.iter().find(|t| &t.id == id));

        if thread.is_none() {
            thread = self.state.threads.iter().find(|t| {
                t.task_id.as_ref() == Some(&view.task_id)
                    && t.anchor.as_ref().and_then(|a| a.file_path.as_ref())
                        == view.file_path.as_ref()
                    && t.anchor.as_ref().and_then(|a| a.line_number) == view.line_number
            });
        }

        let thread = thread.cloned();
        let thread_id = thread.as_ref().map(|t| t.id.clone());
        let comments = thread_id
            .as_ref()
            .and_then(|id| self.state.thread_comments.get(id))
            .cloned()
            .unwrap_or_default();

        // 1. Breadcrumbs / Header
        let status_choices = status_options(&theme);
        let impact_choices = impact_options(&theme);

        egui::Frame::NONE
            .inner_margin(MarginF32 {
                left: spacing::SPACING_XL,
                right: spacing::SPACING_XL,
                top: spacing::SPACING_LG,
                bottom: 0.0,
            })
            .show(ui, |ui| {
                // 2. Title and Status/Impact
                let existing_title = thread
                    .as_ref()
                    .map(|t| t.title.clone())
                    .unwrap_or_else(|| "".to_string());
                let can_edit_thread = thread_id.is_some();

                ui.horizontal(|ui| {
                    let status_width = 120.0;
                    let impact_width = 150.0;
                    let selector_gap = spacing::SPACING_MD;
                    let selector_total_width = status_width + impact_width + selector_gap;
                    let title_width = (ui.available_width() - selector_total_width).max(120.0);

                    // Edit Title - use centralized draft state
                    let mut edit_text = self.state.thread_title_draft.clone();

                    let response = ui
                        .scope(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut edit_text)
                                    .hint_text("Discussion Title")
                                    .desired_width(title_width)
                                    .text_color(Color32::WHITE)
                                    .text_color_opt(Some(theme.text_muted))
                                    .font(egui::FontId::proportional(16.0))
                                    .frame(false)
                                    .margin(egui::vec2(0.0, 0.0)),
                            )
                        })
                        .inner;

                    if response.changed() {
                        self.dispatch(Action::Review(ReviewAction::SetThreadTitleDraft {
                            text: edit_text.clone(),
                        }));
                    }

                    if response.lost_focus()
                        && edit_text != existing_title
                        && let Some(thread_id) = thread_id.clone()
                    {
                        self.dispatch(Action::Review(ReviewAction::UpdateThreadTitle {
                            thread_id,
                            title: edit_text.clone(),
                        }));
                    }

                    // Disable automatic item spacing for precise control
                    let old_spacing = ui.spacing().item_spacing.x;
                    ui.spacing_mut().item_spacing.x = 0.0;

                    // Status selector (left)
                    let status = thread
                        .as_ref()
                        .map(|t| t.status)
                        .unwrap_or(ThreadStatus::Todo);
                    if let Some(next_status) = popup_selector(
                        ui,
                        ui.make_persistent_id(("thread_status_popup", &view.task_id, &thread_id)),
                        status,
                        &status_choices,
                        status_width,
                        can_edit_thread,
                    ) && let Some(thread_id) = thread_id.clone()
                    {
                        self.dispatch(Action::Review(ReviewAction::UpdateThreadStatus {
                            thread_id,
                            status: next_status,
                        }));
                    }

                    // Manual spacing between selectors
                    ui.add_space(selector_gap);

                    // Impact selector (right)
                    let impact = thread
                        .as_ref()
                        .map(|t| t.impact)
                        .unwrap_or(ThreadImpact::Nitpick);
                    if let Some(next_impact) = popup_selector(
                        ui,
                        ui.make_persistent_id(("thread_impact_popup", &view.task_id, &thread_id)),
                        impact,
                        &impact_choices,
                        impact_width,
                        can_edit_thread,
                    ) && let Some(thread_id) = thread_id.clone()
                    {
                        self.dispatch(Action::Review(ReviewAction::UpdateThreadImpact {
                            thread_id,
                            impact: next_impact,
                        }));
                    }

                    // Restore spacing
                    ui.spacing_mut().item_spacing.x = old_spacing;
                });

                // Context
                if let (Some(file_path), Some(line_number)) =
                    (view.file_path.as_ref(), view.line_number)
                    && line_number > 0
                {
                    let updated_label = thread
                        .as_ref()
                        .map(|t| format_timestamp(&t.updated_at))
                        .unwrap_or_else(|| "".to_string());

                    ui.horizontal(|ui| {
                        let display_path = file_path.split('/').next_back().unwrap_or(file_path);
                        ui.label(
                            egui::RichText::new(format!("{display_path}:{line_number}"))
                                .color(theme.text_muted)
                                .size(12.0),
                        );
                        if !updated_label.is_empty() {
                            ui.label(
                                egui::RichText::new(format!("• Updated {}", updated_label))
                                    .color(theme.text_muted)
                                    .size(11.0),
                            );
                        }
                    });

                    if let Some(diff_snippet) =
                        self.thread_diff_snippet(&view.task_id, file_path, line_number)
                    {
                        ui.add_space(spacing::SPACING_MD);
                        ui.label(
                            egui::RichText::new("Diff context")
                                .size(12.0)
                                .color(theme.text_muted),
                        );
                        ui.add_space(spacing::SPACING_XS);

                        egui::Frame::NONE
                            .fill(theme.bg_tertiary)
                            .stroke(egui::Stroke::new(1.0, theme.border_secondary))
                            .corner_radius(crate::ui::spacing::RADIUS_MD)
                            .inner_margin(egui::Margin::same(spacing::SPACING_SM as i8))
                            .show(ui, |ui| {
                                egui::ScrollArea::vertical()
                                    .max_height(220.0)
                                    .show(ui, |ui| {
                                        render_diff_editor_full_view(ui, &diff_snippet, "diff");
                                    });
                            });
                    }
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
                ui.vertical(|ui| {
                    for comment in &comments {
                        self.render_comment_bubble(ui, comment);
                        ui.add_space(spacing::SPACING_MD);
                    }

                    // 4. Input Area
                    ui.add_space(spacing::SPACING_MD);
                    ui.vertical(|ui| {
                        let mut text = self.state.thread_reply_draft.clone();

                        let response = ui.add(
                            egui::TextEdit::multiline(&mut text)
                                .hint_text("Reply...")
                                .font(egui::TextStyle::Body)
                                .frame(false)
                                .desired_rows(3)
                                .desired_width(f32::INFINITY)
                                .margin(egui::vec2(0.0, 0.0))
                                .lock_focus(true),
                        );

                        if response.changed() {
                            self.dispatch(Action::Review(ReviewAction::SetThreadReplyDraft {
                                text: text.clone(),
                            }));
                        }

                        ui.add_space(spacing::SPACING_SM);
                        ui.horizontal(|ui| {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let can_send = !text.trim().is_empty();
                                    let old_padding = ui.spacing().button_padding;
                                    ui.spacing_mut().button_padding = egui::vec2(14.0, 8.0);
                                    if ui
                                        .add_enabled(
                                            can_send,
                                            egui::Button::new(format!(
                                                "{} Send Reply",
                                                icons::PAPER_PLANE_RIGHT
                                            )),
                                        )
                                        .clicked()
                                    {
                                        // Dispatch save
                                        let title = self.state.thread_title_draft.clone();
                                        let title = if title.trim().is_empty() {
                                            None
                                        } else {
                                            Some(title)
                                        };
                                        self.dispatch(Action::Review(
                                            ReviewAction::CreateThreadComment {
                                                task_id: view.task_id.clone(),
                                                thread_id: thread_id.clone(),
                                                file_path: view.file_path.clone(),
                                                line_number: view.line_number,
                                                title,
                                                body: text.trim().to_string(),
                                            },
                                        ));
                                        self.dispatch(Action::Review(
                                            ReviewAction::ClearThreadReplyDraft,
                                        ));
                                    }
                                    ui.spacing_mut().button_padding = old_padding;
                                },
                            );
                        });
                    });
                });
            });
    }

    fn thread_diff_snippet(
        &self,
        task_id: &str,
        file_path: &str,
        line_number: u32,
    ) -> Option<String> {
        let tasks = self.state.tasks();
        let task = tasks.iter().find(|t| t.id == task_id)?;
        let run = self.state.runs.iter().find(|r| r.id == task.run_id)?;
        diff_snippet_for_anchor(&run.diff_text, file_path, line_number)
    }

    fn render_comment_bubble(&self, ui: &mut egui::Ui, comment: &Comment) {
        let theme = current_theme();
        let timestamp = format_timestamp(&comment.created_at);

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(&comment.author)
                        .strong()
                        .size(13.0)
                        .color(theme.text_primary),
                );
                ui.label(
                    egui::RichText::new(format!("• {}", timestamp))
                        .size(10.0)
                        .color(theme.text_muted),
                );
            });

            ui.label(
                egui::RichText::new(&comment.body)
                    .color(theme.text_secondary)
                    .line_height(Some(22.0)),
            );
        });
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

fn status_options(theme: &crate::ui::theme::Theme) -> [PopupOption<ThreadStatus>; 4] {
    [
        ThreadStatus::Todo,
        ThreadStatus::Wip,
        ThreadStatus::Done,
        ThreadStatus::Reject,
    ]
    .map(|status| {
        let v = super::visuals::status_visuals(status, theme);
        PopupOption {
            label: v.label,
            value: status,
            fg: v.color,
            icon: Some(v.icon),
        }
    })
}

fn impact_options(theme: &crate::ui::theme::Theme) -> [PopupOption<ThreadImpact>; 3] {
    [
        ThreadImpact::Blocking,
        ThreadImpact::NiceToHave,
        ThreadImpact::Nitpick,
    ]
    .map(|impact| {
        let v = super::visuals::impact_visuals(impact, theme);
        PopupOption {
            label: v.label,
            value: impact,
            fg: v.color,
            icon: Some(v.icon),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::diff_snippet_for_anchor;

    #[test]
    fn diff_snippet_does_not_fall_through_to_other_files() {
        let diff = r#"diff --git a/pnpm-lock.yaml b/pnpm-lock.yaml
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
"#;

        let snippet = diff_snippet_for_anchor(diff, "pnpm-lock.yaml", 3);
        assert!(snippet.is_none());
    }

    #[test]
    fn diff_snippet_limits_context_window() {
        let diff = r#"diff --git a/src/foo.rs b/src/foo.rs
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
"#;

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
}
