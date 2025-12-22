use crate::ui::app::{Action, GenerateAction, LaReviewApp};

use crate::ui::components::cyber_button::cyber_button;
use crate::ui::components::status::error_banner;
use crate::ui::spacing;
use crate::ui::theme;
use crate::ui::theme::current_theme;
use eframe::egui;

impl LaReviewApp {
    pub fn ui_generate(&mut self, ui: &mut egui::Ui) {
        if ui.available_width() < 100.0 {
            return;
        }

        let mut trigger_generate = false;
        let mut trigger_reset = false;
        // New: Trigger for auto-fetching PRs
        let mut trigger_fetch_pr: Option<String> = None;

        let _action_text = if self.state.session.is_generating {
            format!("{} Generating...", egui_phosphor::regular::HOURGLASS_HIGH)
        } else {
            format!("{} Run", egui_phosphor::regular::PLAY)
        };

        let theme = theme::current_theme();
        let has_content = !self.state.session.diff_text.trim().is_empty()
            || self.state.session.generate_preview.is_some();
        let run_enabled = has_content
            && !self.state.session.is_generating
            && !self.state.session.is_preview_fetching;

        let side_margin = 0.0;
        {
            let content_rect = ui
                .available_rect_before_wrap()
                .shrink2(egui::vec2(side_margin, 0.0));
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
                // Add a small gap for the resize handle.
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
                    .inner_margin(0)
                    .show(&mut left_ui, |ui| {
                        egui::Frame::NONE
                            .inner_margin(spacing::SPACING_SM)
                            .show(ui, |ui| {
                                ui.spacing_mut().item_spacing = egui::vec2(spacing::SPACING_XS, 6.0);

                                // 1. DETERMINE CONTENT SOURCE
                                // We prioritize the fetched preview if it exists.
                                // If not, we use the raw text from the input box.
                                let (active_diff_text, is_from_github) =
                                    if let Some(preview) = &self.state.session.generate_preview {
                                        (preview.diff_text.clone(), true)
                                    } else {
                                        (self.state.session.diff_text.clone(), false)
                                    };

                                let input_trimmed = active_diff_text.trim();

                                // Logic to decide if we show the Editor or the Input Box
                                let show_diff_viewer = !input_trimmed.is_empty()
                                    && (is_from_github
                                        || input_trimmed.starts_with("diff --git ")
                                        || input_trimmed.starts_with("--- a/"));




                                // -- UNIFIED CONTENT AREA --
                                egui::Frame::new()
                                    .fill(current_theme().bg_primary)
                                    .inner_margin(egui::Margin::same(spacing::SPACING_XS as i8))
                                    .show(ui, |ui| {
                                        // Loading Spinner Override
                                        if self.state.session.is_preview_fetching && !is_from_github {
                                            let available = ui.available_size();
                                            let (rect, _) = ui.allocate_exact_size(
                                                available,
                                                egui::Sense::hover(),
                                            );
                                            let painter = ui.painter_at(rect);
                                            crate::ui::animations::cyber::paint_cyber_loader(
                                                &painter,
                                                rect.center(),
                                                "Fetching PR preview...",
                                                ui.input(|i| i.time),
                                                current_theme().brand,
                                                current_theme().text_muted,
                                            );
                                            ui.ctx().request_repaint();
                                            return;
                                        }

                                        if show_diff_viewer {
                                            // === UNIFIED VIEW ===

                                            // A. Render GitHub Metadata Card (If available)
                                            if let Some(preview) = &self.state.session.generate_preview
                                                && let Some(gh) = &preview.github
                                            {
                                                egui::Frame::group(ui.style())
                                                    .fill(current_theme().bg_secondary)
                                                    .stroke(egui::Stroke::NONE)
                                                    .corner_radius(crate::ui::spacing::RADIUS_MD)
                                                    .inner_margin(spacing::SPACING_SM as i8)
                                                    .show(ui, |ui| {
                                                        ui.set_min_width(ui.available_width());
                                                        ui.horizontal(|ui| {
                                                            ui.label(
                                                                egui::RichText::new(
                                                                    egui_phosphor::regular::GITHUB_LOGO,
                                                                )
                                                                .size(16.0),
                                                            );
                                                            ui.vertical(|ui| {
                                                                ui.horizontal(|ui| {
                                                                    ui.label(
                                                                        egui::RichText::new(format!(
                                                                            "{}/{}",
                                                                            gh.pr.owner, gh.pr.repo
                                                                        ))
                                                                        .color(current_theme().text_muted)
                                                                        .size(11.0),
                                                                    );
                                                                    ui.label(
                                                                        egui::RichText::new(format!(
                                                                            "#{}",
                                                                            gh.pr.number
                                                                        ))
                                                                        .color(current_theme().text_muted)
                                                                        .size(11.0),
                                                                    );
                                                                });
                                                                ui.label(
                                                                    egui::RichText::new(&gh.meta.title)
                                                                        .strong()
                                                                        .color(
                                                                            current_theme().text_primary,
                                                                        ),
                                                                );
                                                            });
                                                        });
                                                    });
                                                ui.separator();
                                            }

                                            // B. Render the Diff (Same component for both!)
                                            crate::ui::components::diff::render_diff_editor(
                                                ui,
                                                &active_diff_text,
                                                "unified_diff_viewer",
                                            );
                                        } else {
                                            // === INPUT MODE ===
                                            // Render the text area for pasting
                                            let mut output = self.state.session.diff_text.clone();

                                            let available = ui.available_size();
                                            let row_height =
                                                ui.text_style_height(&egui::TextStyle::Monospace);
                                            let desired_rows =
                                                ((available.y / row_height) as usize).max(12);

                                            let editor = egui::TextEdit::multiline(&mut output)
                                                .id_salt(ui.id().with("input_editor"))
                                                .frame(false)
                                                .hint_text(
                                                    "Paste a unified diff OR a GitHub PR URL/owner/repo#123...",
                                                )
                                                .font(egui::TextStyle::Monospace)
                                                .desired_width(f32::INFINITY)
                                                .desired_rows(desired_rows)
                                                .lock_focus(true);

                                            let response = ui.add_sized(available, editor);

                                            if response.changed() {
                                                self.state.session.diff_text = output.clone();
                                                // Trigger auto-fetch if valid URL
                                                if crate::infra::github::parse_pr_ref(&output)
                                                    .is_some()
                                                {
                                                    trigger_fetch_pr = Some(output);
                                                }
                                            }
                                        }
                                    });
                            });
                    });
            }

            // --- RESIZE HANDLE (Standard) ---
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
                    // 1. Top Content (Padded)

                    egui::Frame::NONE
                        .inner_margin(spacing::SPACING_SM)
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing =
                                egui::vec2(spacing::BUTTON_PADDING.0, spacing::BUTTON_PADDING.1);

                            if let Some(err) = &self.state.session.generation_error {
                                ui.add_space(spacing::SPACING_XS);

                                error_banner(ui, err);
                            }

                            ui.add_space(spacing::SPACING_XS);

                            // --- Integrated Control Panel ---

                            egui::Frame::new()
                                // .stroke(egui::Stroke::new(1.0, current_theme().border))
                                // .corner_radius(crate::ui::spacing::RADIUS_MD)
                                // .inner_margin(spacing::SPACING_SM as i8)
                                .show(ui, |ui| {
                                    ui.vertical(|ui| {
                                        // 1. Configuration Row: Agent & Repo side-by-side

                                        ui.horizontal(|ui| {
                                            let mut temp_agent =
                                                self.state.session.selected_agent.clone();

                                            crate::ui::components::agent_selector::agent_selector(
                                                ui,
                                                &mut temp_agent,
                                            );

                                            if temp_agent != self.state.session.selected_agent {
                                                self.dispatch(Action::Generate(
                                                    GenerateAction::SelectAgent(temp_agent),
                                                ));
                                            }

                                            ui.add_space(spacing::SPACING_SM);

                                            let mut temp_repo_id =
                                                self.state.ui.selected_repo_id.clone();

                                            crate::ui::components::repo_selector::repo_selector(
                                                ui,
                                                &mut temp_repo_id,
                                                &self.state.domain.linked_repos,
                                            );

                                            if temp_repo_id != self.state.ui.selected_repo_id {
                                                self.dispatch(Action::Generate(
                                                    GenerateAction::SelectRepo(temp_repo_id),
                                                ));
                                            }
                                        });

                                        ui.add_space(spacing::SPACING_SM);

                                        ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing.x = spacing::SPACING_SM;

                                            let reset_width = 80.0;
                                            let run_width = ui.available_width()
                                                - reset_width
                                                - ui.spacing().item_spacing.x;

                                            let btn = cyber_button(
                                                ui,
                                                "RUN AGENT",
                                                run_enabled,
                                                self.state.session.is_generating,
                                                None,
                                                Some(run_width),
                                            );

                                            if btn.clicked() && run_enabled {
                                                trigger_generate = true;
                                            }

                                            let reset_btn = cyber_button(
                                                ui,
                                                "RESET",
                                                true,
                                                false,
                                                Some(egui::Color32::from_rgb(200, 60, 60)),
                                                Some(reset_width),
                                            );

                                            if reset_btn.clicked() {
                                                trigger_reset = true;
                                            }
                                        });
                                    });
                                });

                            ui.add_space(spacing::SPACING_SM);

                            // Plan Section

                            if let Some(plan) = self.state.session.latest_plan.as_ref() {
                                ui.add_space(spacing::SPACING_SM);

                                super::plan::render_plan_panel(ui, plan);
                            }
                        });

                    ui.add_space(spacing::SPACING_SM);

                    // 2. ACTIVITY Section (Header)

                    egui::Frame::NONE
                        .inner_margin(egui::Margin::symmetric(spacing::SPACING_SM as i8, 0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.add_space(spacing::SPACING_SM);

                                ui.label(
                                    egui::RichText::new("ACTIVITY")
                                        .size(11.0)
                                        .color(current_theme().text_muted),
                                );

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.add_space(spacing::SPACING_SM);

                                        if !self.state.session.agent_timeline.is_empty()
                                            && ui
                                                .small_button(
                                                    egui::RichText::new(format!(
                                                        "{} Clear",
                                                        egui_phosphor::regular::TRASH
                                                    ))
                                                    .color(current_theme().text_muted),
                                                )
                                                .clicked()
                                        {
                                            self.dispatch(Action::Generate(
                                                GenerateAction::ClearTimeline,
                                            ));
                                        }
                                    },
                                );
                            });
                        });

                    ui.separator();

                    // 3. ACTIVITY Timeline (Scrollable Area)

                    egui::Frame::NONE
                        .inner_margin(egui::Margin::symmetric(spacing::SPACING_SM as i8, 0))
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .id_salt(ui.id().with("agent_activity_scroll"))
                                .stick_to_bottom(true)
                                .show(ui, |ui| {
                                    ui.add_space(spacing::SPACING_XS);

                                    for item in &self.state.session.agent_timeline {
                                        super::timeline::render_timeline_item(ui, item);
                                    }
                                });
                        });
                });
        }

        // --- ACTION DISPATCHERS ---

        if trigger_reset {
            self.reset_generation_state();
        }

        if trigger_generate {
            self.start_generation_async();
        }

        // Handle the auto-fetch detected from the text input
        if let Some(url) = trigger_fetch_pr {
            self.dispatch(Action::Generate(GenerateAction::FetchPrContext(url)));
        }
    }
}
