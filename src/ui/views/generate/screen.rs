use crate::ui::app::{Action, GenerateAction, LaReviewApp};

use crate::ui::components::status::error_banner;
use crate::ui::spacing;
use crate::ui::theme;
use catppuccin_egui::MOCHA;
use eframe::egui;

impl LaReviewApp {
    pub fn ui_generate(&mut self, ui: &mut egui::Ui) {
        let mut trigger_generate = false;
        let mut trigger_reset = false;
        // New: Trigger for auto-fetching PRs
        let mut trigger_fetch_pr: Option<String> = None;

        let _action_text = if self.state.is_generating {
            format!("{} Generating...", egui_phosphor::regular::HOURGLASS_HIGH)
        } else {
            format!("{} Run", egui_phosphor::regular::PLAY)
        };

        let theme = theme::current_theme();
        let has_content =
            !self.state.diff_text.trim().is_empty() || self.state.generate_preview.is_some();
        let run_enabled =
            has_content && !self.state.is_generating && !self.state.is_preview_fetching;

        // --- Resizable Panes Setup ---
        let pane_width_id = ui.id().with("pane_width");
        let available_width = ui.available_width();
        let resize_handle_width = spacing::SPACING_XS;
        let content_width = (available_width - resize_handle_width).max(0.0);

        let saved_right_width = ui
            .memory(|mem| mem.data.get_temp::<f32>(pane_width_id))
            .unwrap_or(300.0);
        let right_width =
            crate::ui::layout::clamp_width(saved_right_width, 400.0, content_width * 0.5);

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
                .inner_margin(egui::Margin::same(spacing::SPACING_SM as i8))
                .show(&mut left_ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(spacing::SPACING_XS, 6.0);

                    // 1. DETERMINE CONTENT SOURCE
                    // We prioritize the fetched preview if it exists.
                    // If not, we use the raw text from the input box.
                    let (active_diff_text, is_from_github) =
                        if let Some(preview) = &self.state.generate_preview {
                            (preview.diff_text.clone(), true)
                        } else {
                            (self.state.diff_text.clone(), false)
                        };

                    let input_trimmed = active_diff_text.trim();

                    // Logic to decide if we show the Editor or the Input Box
                    let show_diff_viewer = !input_trimmed.is_empty()
                        && (is_from_github
                            || input_trimmed.starts_with("diff --git ")
                            || input_trimmed.starts_with("--- a/"));

                    // -- TOOLBAR --
                    ui.horizontal(|ui| {
                        ui.heading(egui::RichText::new("INPUT").size(16.0).color(MOCHA.text));

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .button(
                                    egui::RichText::new(format!(
                                        "{} New",
                                        egui_phosphor::regular::PLUS
                                    ))
                                    .color(MOCHA.green),
                                )
                                .clicked()
                            {
                                trigger_reset = true;
                            }
                        });
                    });

                    ui.add_space(spacing::SPACING_XS);

                    // -- UNIFIED CONTENT AREA --
                    egui::Frame::new()
                        .fill(MOCHA.crust)
                        .inner_margin(egui::Margin::same(spacing::SPACING_XS as i8))
                        .stroke(egui::Stroke::new(1.0, MOCHA.surface0))
                        .show(ui, |ui| {
                            // Loading Spinner Override
                            if self.state.is_preview_fetching && !is_from_github {
                                let available = ui.available_size();
                                let (rect, _) =
                                    ui.allocate_exact_size(available, egui::Sense::hover());
                                ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                                    ui.centered_and_justified(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.spinner();
                                            ui.label("Fetching PR preview...");
                                        });
                                    });
                                });
                                return;
                            }

                            if show_diff_viewer {
                                // === UNIFIED VIEW ===

                                // A. Render GitHub Metadata Card (If available)
                                if let Some(preview) = &self.state.generate_preview
                                    && let Some(gh) = &preview.github
                                {
                                    egui::Frame::group(ui.style())
                                        .fill(MOCHA.mantle)
                                        .stroke(egui::Stroke::NONE)
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
                                                            .color(MOCHA.subtext1)
                                                            .size(11.0),
                                                        );
                                                        ui.label(
                                                            egui::RichText::new(format!(
                                                                "#{}",
                                                                gh.pr.number
                                                            ))
                                                            .color(MOCHA.subtext0)
                                                            .size(11.0),
                                                        );
                                                    });
                                                    ui.label(
                                                        egui::RichText::new(&gh.meta.title)
                                                            .strong()
                                                            .color(MOCHA.text),
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
                                let mut output = self.state.diff_text.clone();

                                let available = ui.available_size();
                                let row_height = ui.text_style_height(&egui::TextStyle::Monospace);
                                let desired_rows = ((available.y / row_height) as usize).max(12);

                                let editor = egui::TextEdit::multiline(&mut output)
                                    .id_salt(ui.id().with("input_editor"))
                                    .hint_text(
                                        "Paste a unified diff OR a GitHub PR URL/owner/repo#123...",
                                    )
                                    .font(egui::TextStyle::Monospace)
                                    .desired_width(f32::INFINITY)
                                    .desired_rows(desired_rows)
                                    .lock_focus(true);

                                let response = ui.add_sized(available, editor);

                                if response.changed() {
                                    self.state.diff_text = output.clone();
                                    // Trigger auto-fetch if valid URL
                                    if crate::infra::github::parse_pr_ref(&output).is_some() {
                                        trigger_fetch_pr = Some(output);
                                    }
                                }
                            }
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
                crate::ui::layout::clamp_width(new_right_width, 250.0, content_width * 0.5);
            ui.memory_mut(|mem| {
                mem.data.insert_temp(pane_width_id, clamped_width);
            });
        }

        let line_color = if resize_response.hovered() || resize_response.dragged() {
            theme.accent
        } else {
            theme.bg_secondary
        };
        ui.painter().rect_filled(resize_rect, 2.0, line_color);
        if resize_response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }

        // --- RIGHT PANE (Agent & Timeline) ---
        let mut right_ui = ui.new_child(egui::UiBuilder::new().max_rect(right_rect));
        right_ui.set_clip_rect(right_rect);

        egui::Frame::default()
            .fill(right_ui.style().visuals.window_fill)
            .inner_margin(egui::Margin::symmetric(
                spacing::SPACING_SM as i8,
                spacing::SPACING_SM as i8,
            ))
            .show(&mut right_ui, |ui| {
                ui.spacing_mut().item_spacing =
                    egui::vec2(spacing::BUTTON_PADDING.0, spacing::BUTTON_PADDING.1); // 8.0, 4.0

                ui.horizontal(|ui| {
                    ui.heading(egui::RichText::new("AGENT").size(16.0).color(MOCHA.text));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Custom "Run" Button with Sci-Fi/Tron Effect
                        let button_size = egui::vec2(120.0, 28.0);
                        let (rect, response) =
                            ui.allocate_exact_size(button_size, egui::Sense::click());

                        if response.clicked() && run_enabled {
                            trigger_generate = true;
                        }

                        // Animation State
                        if self.state.is_generating {
                            ui.ctx().request_repaint(); // Continuous animation
                        }

                        let t = ui.input(|i| i.time);

                        if ui.is_rect_visible(rect) {
                            let painter = ui.painter();
                            let style = ui.style();
                            // visuals variable removed as unused

                            let mut fill_color = if self.state.is_generating {
                                // Deep "Void" background for Tron mode
                                MOCHA.crust
                            } else if run_enabled {
                                theme.brand
                            } else {
                                theme.bg_card
                            };

                            if response.hovered() && run_enabled {
                                // Manual lighten (blend with white)
                                let c = fill_color;
                                fill_color = egui::Color32::from_rgb(
                                    c.r().saturating_add(20),
                                    c.g().saturating_add(20),
                                    c.b().saturating_add(20),
                                );
                            }

                            // Base Button Shape
                            painter.rect(
                                rect,
                                style.visuals.widgets.noninteractive.corner_radius,
                                fill_color,
                                egui::Stroke::NONE,
                                egui::StrokeKind::Middle,
                            );

                            let border_color = if self.state.is_generating {
                                MOCHA.surface2
                            } else {
                                theme.border
                            };

                            painter.rect_stroke(
                                rect,
                                style.visuals.widgets.noninteractive.corner_radius,
                                egui::Stroke::new(1.0, border_color),
                                egui::StrokeKind::Middle,
                            );

                            // Tron/Sci-Fi Animation (Simplified)
                            if self.state.is_generating {
                                let scanner_color = MOCHA.sky;

                                // Pulsing Border Only
                                let pulse = (t * 2.0).sin() * 0.5 + 0.5; // 0.0 to 1.0
                                let border_alpha = (pulse * 200.0) as u8; // 0-200 alpha

                                painter.rect_stroke(
                                    rect,
                                    style.visuals.widgets.noninteractive.corner_radius,
                                    egui::Stroke::new(
                                        1.5,
                                        egui::Color32::from_rgba_unmultiplied(
                                            scanner_color.r(),
                                            scanner_color.g(),
                                            scanner_color.b(),
                                            border_alpha,
                                        ),
                                    ),
                                    egui::StrokeKind::Middle,
                                );
                            }

                            // Text & Icon
                            let text_color = if self.state.is_generating {
                                // Pulsing Text
                                let text_pulse = (t * 3.0).sin() * 0.3 + 0.7;
                                let alpha = (text_pulse * 255.0) as u8;
                                egui::Color32::from_rgba_unmultiplied(
                                    MOCHA.text.r(),
                                    MOCHA.text.g(),
                                    MOCHA.text.b(),
                                    alpha,
                                )
                            } else if run_enabled {
                                theme.text_inverse
                            } else {
                                theme.text_disabled
                            };

                            let display_text = if self.state.is_generating {
                                "GENERATING..."
                            } else {
                                "RUN AGENT"
                            };

                            let icon = if self.state.is_generating {
                                None
                            } else {
                                Some(egui_phosphor::regular::PLAY)
                            };

                            // Centered Layout
                            let font_id = egui::FontId::proportional(14.0);
                            let galley = ui.painter().layout_no_wrap(
                                format!(
                                    "{}{}",
                                    if let Some(ic) = icon {
                                        format!("{} ", ic)
                                    } else {
                                        "".to_string()
                                    },
                                    display_text
                                ),
                                font_id,
                                text_color,
                            );

                            let text_pos = rect.center() - galley.size() / 2.0;
                            ui.painter().galley(
                                text_pos,
                                galley,
                                egui::Color32::BLACK, /* unused background */
                            );
                        }
                    });
                });

                if let Some(err) = &self.state.generation_error {
                    ui.add_space(spacing::SPACING_XS);
                    error_banner(ui, err);
                }

                ui.add_space(spacing::SPACING_XS);

                // ... Agent Selection Chips (Existing Code) ...
                // Agent Selector Component
                let mut temp_agent = self.state.selected_agent.clone();
                crate::ui::components::agent_selector::agent_selector(ui, &mut temp_agent);
                if temp_agent != self.state.selected_agent {
                    self.dispatch(Action::Generate(GenerateAction::SelectAgent(temp_agent)));
                }

                ui.add_space(spacing::SPACING_SM);

                // Status Section
                egui::Frame::group(ui.style())
                    .inner_margin(egui::Margin::symmetric(
                        spacing::SPACING_MD as i8,
                        spacing::SPACING_SM as i8,
                    ))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("STATUS")
                                    .size(11.0)
                                    .color(MOCHA.subtext0),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.add_space(spacing::SPACING_XS);
                                    let status_text = if self.state.is_generating {
                                        "Agent is working..."
                                    } else if self.state.diff_text.trim().is_empty() {
                                        "Waiting for input..."
                                    } else {
                                        "Ready."
                                    };
                                    ui.label(
                                        egui::RichText::new(status_text)
                                            .color(MOCHA.subtext1)
                                            .size(12.0),
                                    );
                                },
                            );
                        });
                    });
                ui.add_space(8.0);

                // Plan & Timeline (Existing Code)
                if let Some(plan) = &self.state.latest_plan {
                    super::plan::render_plan_panel(ui, plan);
                    ui.add_space(spacing::SPACING_SM);
                }

                egui::Frame::group(ui.style())
                    .inner_margin(egui::Margin::symmetric(
                        spacing::SPACING_MD as i8,
                        spacing::SPACING_SM as i8,
                    ))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("ACTIVITY")
                                    .size(11.0)
                                    .color(MOCHA.subtext0),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if !self.state.agent_timeline.is_empty()
                                        && ui
                                            .small_button(
                                                egui::RichText::new(format!(
                                                    "{} Clear",
                                                    egui_phosphor::regular::TRASH
                                                ))
                                                .color(MOCHA.overlay2),
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

                        ui.add_space(spacing::SPACING_XS);
                        ui.separator();

                        egui::ScrollArea::vertical()
                            .id_salt(ui.id().with("agent_activity_scroll"))
                            .stick_to_bottom(true)
                            .show(ui, |ui| {
                                for item in &self.state.agent_timeline {
                                    super::timeline::render_timeline_item(ui, item);
                                }
                            });
                    });
            });

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
