use crate::ui::app::{GenerateAction, GeneratePreview};
use crate::ui::theme::Theme;
use crate::ui::{spacing, typography};
use eframe::egui;
use std::sync::Arc;

pub(crate) fn render_input_pane(
    ui: &mut egui::Ui,
    diff_text: &str,
    generate_preview: Option<&GeneratePreview>,
    is_preview_fetching: bool,
    theme: &Theme,
) -> Option<GenerateAction> {
    let mut action_out = None;

    // 1. DETERMINE CONTENT SOURCE

    let (active_diff_text, is_from_github): (Arc<str>, bool) =
        if let Some(preview) = generate_preview {
            (preview.diff_text.clone(), true)
        } else {
            (Arc::from(diff_text), false)
        };

    let input_trimmed = active_diff_text.trim();

    let show_diff_viewer = !input_trimmed.is_empty()
        && (is_from_github
            || input_trimmed.starts_with("diff --git ")
            || input_trimmed.starts_with("--- a/"));

    egui::Frame::new()
        .fill(theme.bg_primary)
        .inner_margin(egui::Margin::same(spacing::SPACING_XS as i8))
        .show(ui, |ui| {
            if is_preview_fetching && !is_from_github {
                let available = ui.available_size();
                let (rect, _) = ui.allocate_exact_size(available, egui::Sense::hover());
                let painter = ui.painter_at(rect);
                crate::ui::animations::cyber::paint_cyber_loader(
                    &painter,
                    rect.center(),
                    "Fetching PR preview...",
                    ui.input(|i| i.time),
                    theme.brand,
                    theme.text_muted,
                );
                ui.ctx().request_repaint();
                return;
            }

            if show_diff_viewer {
                if let Some(preview) = generate_preview
                    && let Some(gh) = &preview.github
                {
                    egui::Frame::group(ui.style())
                        .fill(theme.bg_secondary)
                        .stroke(egui::Stroke::NONE)
                        .corner_radius(spacing::RADIUS_MD)
                        .inner_margin(spacing::SPACING_SM as i8)
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.label(
                                    typography::body(egui_phosphor::regular::GITHUB_LOGO)
                                        .size(16.0),
                                );
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            typography::body(format!(
                                                "{}/{}",
                                                gh.pr.owner, gh.pr.repo
                                            ))
                                            .color(theme.text_muted)
                                            .size(11.0),
                                        );
                                        ui.label(
                                            typography::body(format!("#{}", gh.pr.number))
                                                .color(theme.text_muted)
                                                .size(11.0),
                                        );
                                    });
                                    ui.label(
                                        typography::bold(&gh.meta.title).color(theme.text_primary),
                                    );
                                });
                            });
                        });
                    ui.separator();
                }

                crate::ui::components::diff::render_diff_editor(
                    ui,
                    &active_diff_text,
                    "unified_diff_viewer",
                );
            } else {
                let mut output = diff_text.to_string();
                let available = ui.available_size();
                let row_height = ui.text_style_height(&egui::TextStyle::Monospace);
                let desired_rows = ((available.y / row_height) as usize).max(12);

                let editor = egui::TextEdit::multiline(&mut output)
                    .id_salt(ui.id().with("input_editor"))
                    .frame(false)
                    .hint_text("Paste a unified diff OR a GitHub PR URL/owner/repo#123...")
                    .font(egui::TextStyle::Monospace)
                    .desired_width(f32::INFINITY)
                    .desired_rows(desired_rows)
                    .lock_focus(true);

                let response = ui.add_sized(available, editor);

                if response.changed() {
                    action_out = Some(GenerateAction::UpdateDiffText(output));
                }
            }
        });

    action_out
}
