use crate::domain::Feedback;
use crate::ui::components::{DiffAction, render_diff_editor_full_view};
use crate::ui::theme::Theme;
use crate::ui::views::review::format_timestamp;
use crate::ui::{spacing, typography};
use eframe::egui;

pub(crate) fn render_feedback_context(
    ui: &mut egui::Ui,
    feedback: Option<&Feedback>,
    file_path: Option<&String>,
    line_number: Option<u32>,
    diff_snippet: Option<String>,
    theme: &Theme,
) -> DiffAction {
    let mut action = DiffAction::None;
    if let (Some(file_path), Some(line_number)) = (file_path, line_number)
        && line_number > 0
    {
        let updated_label = feedback
            .map(|t| format_timestamp(&t.updated_at))
            .unwrap_or_default();

        ui.horizontal(|ui| {
            let display_path = file_path.split('/').next_back().unwrap_or(file_path);
            ui.label(
                typography::body(format!("{display_path}:{line_number}"))
                    .color(theme.text_muted)
                    .size(12.0),
            );
            if !updated_label.is_empty() {
                ui.label(
                    typography::body(format!("• Updated {}", updated_label))
                        .color(theme.text_muted)
                        .size(11.0),
                );
            }
        });

        if let Some(diff_snippet) = diff_snippet {
            ui.add_space(spacing::SPACING_MD);
            ui.label(
                typography::body("Diff context")
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
                            action = render_diff_editor_full_view(ui, &diff_snippet, "diff");
                        });
                });
        }
    }

    action
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    #[test]
    fn test_render_feedback_context() {
        let file_path = "src/main.rs".to_string();
        let mut harness = Harness::new_ui(|ui| {
            render_feedback_context(
                ui,
                None,
                Some(&file_path),
                Some(10),
                Some("diff snippet".into()),
                &Theme::mocha(),
            );
        });
        harness.run();
        harness.get_by_label("main.rs:10");
        harness.get_by_label("Diff context");
    }
}
