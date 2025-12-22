use crate::ui::app::{Action, LaReviewApp};
use crate::ui::spacing;
use crate::ui::theme::current_theme;
use eframe::egui;

use crate::ui::views::generate::agent_pane::AgentPaneContext;
use crate::ui::views::generate::{render_agent_pane, render_input_pane, render_timeline_pane};

impl LaReviewApp {
    pub fn ui_generate(&mut self, ui: &mut egui::Ui) {
        if ui.available_width() < 100.0 {
            return;
        }

        let theme = current_theme();

        let has_content = !self.state.session.diff_text.trim().is_empty()
            || self.state.session.generate_preview.is_some();
        let run_enabled = has_content
            && !self.state.session.is_generating
            && !self.state.session.is_preview_fetching;

        let content_rect = ui.available_rect_before_wrap();
        let mut content_ui = ui.new_child(egui::UiBuilder::new().max_rect(content_rect));
        content_ui.set_clip_rect(content_rect);
        let ui = &mut content_ui;

        // --- Resizable Panes Setup ---
        let pane_width_id = ui.id().with("pane_width");
        let available_width = ui.available_width();
        let resize_handle_width = spacing::RESIZE_HANDLE_WIDTH;
        let content_width = (available_width - resize_handle_width).max(0.0);

        let saved_right_width = ui
            .memory(|mem| mem.data.get_temp::<f32>(pane_width_id))
            .unwrap_or(300.0);
        let right_width =
            crate::ui::layout::clamp_width(saved_right_width, 450.0, content_width * 0.5);

        let left_width = content_width - right_width;

        let (left_rect, right_rect) = {
            let available = ui.available_rect_before_wrap();
            let left = egui::Rect::from_min_size(
                available.min,
                egui::vec2(left_width, available.height()),
            );
            let right = egui::Rect::from_min_size(
                egui::pos2(
                    available.min.x + left_width + resize_handle_width,
                    available.min.y,
                ),
                egui::vec2(right_width, available.height()),
            );
            (left, right)
        };

        // --- LEFT PANE (Smart Input) ---
        let mut left_ui = ui.new_child(egui::UiBuilder::new().max_rect(left_rect));
        left_ui.set_clip_rect(left_rect);
        {
            egui::Frame::default()
                .fill(left_ui.style().visuals.window_fill)
                .inner_margin(spacing::SPACING_SM as i8)
                .show(&mut left_ui, |ui| {
                    if let Some(action) = render_input_pane(
                        ui,
                        &self.state.session.diff_text,
                        self.state.session.generate_preview.as_ref(),
                        self.state.session.is_preview_fetching,
                        &theme,
                    ) {
                        self.dispatch(Action::Generate(action));
                    }
                });
        }

        // --- RESIZE HANDLE ---
        let resize_id = ui.id().with("resize");
        let resize_rect = egui::Rect::from_min_size(
            egui::pos2(left_rect.max.x, left_rect.min.y),
            egui::vec2(resize_handle_width, left_rect.height()),
        );
        let resize_response = ui.interact(resize_rect, resize_id, egui::Sense::drag());

        if resize_response.dragged()
            && let Some(pointer_pos) = ui.ctx().pointer_interact_pos()
        {
            let new_right_width = content_width - (pointer_pos.x - left_rect.min.x);
            let clamped_width =
                crate::ui::layout::clamp_width(new_right_width, 450.0, content_width * 0.5);
            ui.memory_mut(|mem| {
                mem.data.insert_temp(pane_width_id, clamped_width);
            });
        }

        let line_color = if resize_response.hovered() || resize_response.dragged() {
            theme.accent
        } else {
            theme.border
        };
        ui.painter().rect_filled(resize_rect, 1.0, line_color);
        if resize_response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }

        // --- RIGHT PANE (Agent & Timeline) ---
        let mut right_ui = ui.new_child(egui::UiBuilder::new().max_rect(right_rect));
        right_ui.set_clip_rect(right_rect);
        let right_fill = right_ui.style().visuals.window_fill;

        egui::Frame::default()
            .fill(right_fill)
            .inner_margin(0)
            .show(&mut right_ui, |ui| {
                let ctx = AgentPaneContext {
                    selected_agent: &self.state.session.selected_agent,
                    selected_repo_id: self.state.ui.selected_repo_id.as_ref(),
                    linked_repos: &self.state.domain.linked_repos,
                    is_generating: self.state.session.is_generating,
                    generation_error: self.state.session.generation_error.as_ref(),
                    latest_plan: self.state.session.latest_plan.as_ref(),
                    run_enabled,
                };

                if let Some(action) = render_agent_pane(ui, ctx, &theme) {
                    self.dispatch(Action::Generate(action));
                }

                ui.add_space(spacing::SPACING_SM);

                if let Some(action) =
                    render_timeline_pane(ui, &self.state.session.agent_timeline, &theme)
                {
                    self.dispatch(Action::Generate(action));
                }
            });
    }
}
