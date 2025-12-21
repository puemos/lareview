use super::super::doc::DiffDoc;
use super::super::model::DiffViewState;
use super::utils::{DIFF_FONT_SIZE, HEADER_FONT_SIZE, middle_truncate, strip_git_prefix};
use crate::ui::theme;
use eframe::egui;

pub fn render_file_header(
    ui: &mut egui::Ui,
    file_idx: usize,
    view_state: &mut DiffViewState,
    state_id: egui::Id,
    doc: &DiffDoc,
) {
    let theme = theme::current_theme();
    let file = &doc.files[file_idx];

    let is_open = if file_idx < view_state.collapsed.len() {
        !view_state.collapsed[file_idx]
    } else {
        false
    };
    let icon_closed = egui_phosphor::regular::PLUS;
    let icon_open = egui_phosphor::regular::MINUS;
    let icon = if is_open { icon_open } else { icon_closed };

    let clicked = ui
        .scope(|ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
            ui.horizontal(|ui| {
                let mut clicked_local = false;

                let display_path = if file.new_path != "/dev/null" {
                    strip_git_prefix(&file.new_path)
                } else {
                    strip_git_prefix(&file.old_path)
                };

                if ui
                    .button(
                        egui::RichText::new(icon.to_string())
                            .size(DIFF_FONT_SIZE)
                            .color(theme.text_primary),
                    )
                    .clicked()
                {
                    clicked_local = true;
                }

                let mut stats_reserved_width = 0.0;
                if file.additions > 0 {
                    stats_reserved_width += 60.0;
                }
                if file.deletions > 0 {
                    stats_reserved_width += 60.0;
                }

                let available_width = ui.available_width();
                let path_area_width = (available_width - stats_reserved_width).max(50.0);

                let char_capacity = (path_area_width / 7.0) as usize;
                let max_len = char_capacity.saturating_sub(3).max(15);

                let truncated_path = middle_truncate(&display_path, max_len);

                ui.label(
                    egui::RichText::new(truncated_path)
                        .strong()
                        .color(theme.text_primary)
                        .size(HEADER_FONT_SIZE),
                )
                .on_hover_text(display_path);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if file.additions > 0 {
                        ui.label(
                            egui::RichText::new(format!("+{}", file.additions))
                                .color(theme.success)
                                .size(DIFF_FONT_SIZE),
                        );
                    }
                    if file.deletions > 0 {
                        ui.label(
                            egui::RichText::new(format!("-{}", file.deletions))
                                .color(theme.destructive)
                                .size(DIFF_FONT_SIZE),
                        );
                    }
                });

                clicked_local
            })
            .inner
        })
        .inner;

    ui.add_space(2.0);
    ui.separator();

    if clicked {
        if file_idx < view_state.collapsed.len() {
            view_state.collapsed[file_idx] = !view_state.collapsed[file_idx];
        } else {
            while view_state.collapsed.len() <= file_idx {
                view_state.collapsed.push(false);
            }
            view_state.collapsed[file_idx] = true;
        }

        let scroll_id = ui.id().with("diff_scroll");
        ui.ctx().memory_mut(|mem| {
            mem.data.remove::<egui::scroll_area::State>(scroll_id);
        });

        ui.ctx()
            .memory_mut(|mem| mem.data.insert_persisted(state_id, view_state.clone()));
    }
}
