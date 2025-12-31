use crate::ui::app::{Action, LaReviewApp, ReviewAction, SendToPrOverlayData};
use crate::ui::theme::current_theme;
use crate::ui::{icons, spacing, typography};
use eframe::egui;
use egui::Margin;

pub fn render_push_feedback_confirm(ctx: &egui::Context, app: &mut LaReviewApp, feedback_id: &str) {
    let Some(feedback) = app
        .state
        .domain
        .feedbacks
        .iter()
        .find(|f| f.id == feedback_id)
        .cloned()
    else {
        app.dispatch(Action::Review(ReviewAction::CancelSendFeedbackConfirm));
        return;
    };

    let is_pending = app
        .state
        .ui
        .push_feedback_pending
        .as_deref()
        .map(|id| id == feedback_id)
        .unwrap_or(false);
    let error = app.state.ui.push_feedback_error.clone();

    let theme = current_theme();
    let mut open = true;
    let path_line = feedback
        .anchor
        .as_ref()
        .and_then(|a| a.file_path.clone())
        .unwrap_or_else(|| "<no file>".to_string());
    let line_num = feedback
        .anchor
        .as_ref()
        .and_then(|a| a.line_number)
        .map(|n| n.to_string())
        .unwrap_or_else(|| "?".to_string());

    egui::Window::new("Send feedback to GitHub?")
        .id(egui::Id::new("push_feedback_to_pr_overlay"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .frame(
            egui::Frame::window(&ctx.style())
                .inner_margin(egui::Margin::same(spacing::SPACING_MD as i8)),
        )
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.label(typography::h2("Send this feedback to the linked PR?"));
                ui.add_space(spacing::SPACING_SM);
                ui.label(
                    typography::body(format!(
                        "Feedback: {} • Status: {} • Impact: {}",
                        feedback.title, feedback.status, feedback.impact
                    ))
                    .color(theme.text_primary),
                );
                ui.label(
                    typography::body(format!("Location: {path_line}:{line_num}"))
                        .color(theme.text_muted),
                );

                if let Some(err) = error {
                    ui.add_space(spacing::SPACING_SM);
                    ui.label(typography::body(err).color(theme.destructive));
                }

                ui.add_space(spacing::SPACING_MD);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            !is_pending,
                            egui::Button::new(typography::label(format!(
                                "{} Send",
                                crate::ui::icons::ICON_GITHUB
                            ))),
                        )
                        .clicked()
                    {
                        app.dispatch(Action::Review(ReviewAction::SendFeedbackToPr {
                            feedback_id: feedback.id.clone(),
                        }));
                    }

                    if ui
                        .add_enabled(!is_pending, egui::Button::new(typography::label("Cancel")))
                        .clicked()
                    {
                        app.dispatch(Action::Review(ReviewAction::CancelSendFeedbackConfirm));
                    }
                });
            });
        });

    if !open {
        app.dispatch(Action::Review(ReviewAction::CancelSendFeedbackConfirm));
    }
}

pub fn render_send_to_pr_overlay(
    ctx: &egui::Context,
    app: &mut LaReviewApp,
    data: &SendToPrOverlayData,
) {
    let Some(review_id) = app.state.ui.selected_review_id.clone() else {
        app.dispatch(Action::Review(ReviewAction::CloseSendToPrModal));
        return;
    };

    let Some(review) = app
        .state
        .domain
        .reviews
        .iter()
        .find(|r| r.id == review_id)
        .cloned()
    else {
        app.dispatch(Action::Review(ReviewAction::CloseSendToPrModal));
        return;
    };

    let theme = current_theme();
    let mut open = true;

    let mut feedbacks: Vec<crate::domain::Feedback> = app
        .state
        .domain
        .feedbacks
        .iter()
        .filter(|f| f.review_id == review_id)
        .cloned()
        .collect();

    feedbacks.sort_by(|a, b| {
        let rank_a = a.status.rank();
        let rank_b = b.status.rank();
        if rank_a != rank_b {
            rank_a.cmp(&rank_b)
        } else {
            b.updated_at.cmp(&a.updated_at)
        }
    });

    egui::Window::new("Send to GitHub PR")
        .id(egui::Id::new("send_to_pr_overlay"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .frame(
            egui::Frame::window(&ctx.style())
                .inner_margin(egui::Margin::same(spacing::SPACING_LG as i8)),
        )
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.set_width(ui.available_width().max(520.0));
                ui.label(typography::h2("Send review to GitHub PR"));

                let is_github =
                    matches!(review.source, crate::domain::ReviewSource::GitHubPr { .. });
                if !is_github {
                    ui.label(
                        typography::body("Selected review is not linked to a GitHub PR.")
                            .color(theme.destructive),
                    );
                    return;
                }

                ui.add_space(spacing::SPACING_MD);
                let mut include_summary = data.include_summary;
                if ui
                    .checkbox(
                        &mut include_summary,
                        typography::body("Include summary for all tasks"),
                    )
                    .changed()
                {
                    app.dispatch(Action::Review(ReviewAction::ToggleSendToPrSummary {
                        include: include_summary,
                    }));
                }

                ui.add_space(spacing::SPACING_MD);
                ui.label(typography::body("Select feedback to send:").color(theme.text_muted));
                ui.add_space(spacing::SPACING_SM);

                let feedback_to_toggle = std::cell::RefCell::new(None);

                egui::ScrollArea::vertical()
                    .max_height(360.0)
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                        for feedback in &feedbacks {
                            use crate::ui::components::list_item::ListItem;
                            use crate::ui::views::review::visuals;

                            let eligible = feedback
                                .anchor
                                .as_ref()
                                .map(|a| {
                                    a.file_path.is_some()
                                        && a.line_number.is_some()
                                        && a.side.is_some()
                                })
                                .unwrap_or(false);

                            let mut checked = data.selection.contains(&feedback.id);

                            let impact_v = visuals::impact_visuals(feedback.impact, &theme);
                            let title_text = if eligible {
                                typography::bold(&feedback.title).color(theme.text_primary)
                            } else {
                                typography::bold(&feedback.title).color(theme.text_muted)
                            };

                            let mut metadata_job = egui::text::LayoutJob::default();
                            metadata_job.append(
                                impact_v.icon,
                                0.0,
                                egui::TextFormat {
                                    font_id: egui::FontId::proportional(12.0),
                                    color: impact_v.color,
                                    ..Default::default()
                                },
                            );
                            metadata_job.append(
                                &format!(" {} ", impact_v.label),
                                0.0,
                                egui::TextFormat {
                                    font_id: egui::FontId::proportional(12.0),
                                    color: impact_v.color,
                                    ..Default::default()
                                },
                            );

                            metadata_job.append(
                                "· ",
                                0.0,
                                egui::TextFormat {
                                    font_id: egui::FontId::proportional(12.0),
                                    color: theme.text_disabled,
                                    ..Default::default()
                                },
                            );

                            if let Some(anchor) = &feedback.anchor {
                                if let (Some(path), Some(line)) =
                                    (anchor.file_path.as_ref(), anchor.line_number)
                                {
                                    let loc_text = format!(
                                        "{}:{} ({:?})",
                                        path,
                                        line,
                                        anchor.side.unwrap_or(crate::domain::FeedbackSide::New)
                                    );
                                    metadata_job.append(
                                        &loc_text,
                                        0.0,
                                        egui::TextFormat {
                                            font_id: egui::FontId::proportional(12.0),
                                            color: theme.text_muted,
                                            ..Default::default()
                                        },
                                    );
                                } else {
                                    metadata_job.append(
                                        "No location",
                                        0.0,
                                        egui::TextFormat {
                                            font_id: egui::FontId::proportional(12.0),
                                            color: theme.text_muted,
                                            ..Default::default()
                                        },
                                    );
                                }
                            }

                            if !eligible {
                                metadata_job.append(
                                    " (Missing context)",
                                    0.0,
                                    egui::TextFormat {
                                        font_id: egui::FontId::proportional(12.0),
                                        color: theme.destructive,
                                        ..Default::default()
                                    },
                                );
                            }

                            let feedback_id = feedback.id.clone();
                            let item = ListItem::new(title_text)
                                .metadata(egui::WidgetText::from(metadata_job))
                                .inner_margin(Margin::symmetric(
                                    spacing::SPACING_SM as i8,
                                    spacing::SPACING_XS as i8,
                                ))
                                .checkbox(&mut checked)
                                .action(|| {
                                    *feedback_to_toggle.borrow_mut() = Some(feedback_id);
                                });

                            ui.add_enabled_ui(eligible, |ui| {
                                item.show_with_bg(ui, &theme);
                            });
                        }
                    });

                if let Some(id) = feedback_to_toggle.into_inner() {
                    app.dispatch(Action::Review(ReviewAction::ToggleSendToPrFeedback {
                        feedback_id: id,
                    }));
                }

                if let Some(err) = &data.error {
                    ui.add_space(spacing::SPACING_SM);
                    ui.label(typography::body(err).color(theme.destructive));
                }

                ui.add_space(spacing::SPACING_MD);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            !data.pending,
                            egui::Button::new(typography::label(format!(
                                "{} Send to PR",
                                icons::ICON_GITHUB
                            ))),
                        )
                        .clicked()
                    {
                        app.dispatch(Action::Review(ReviewAction::ConfirmSendToPr));
                    }

                    if ui
                        .add_enabled(
                            !data.pending,
                            egui::Button::new(typography::label("Cancel")),
                        )
                        .clicked()
                    {
                        app.dispatch(Action::Review(ReviewAction::CloseSendToPrModal));
                    }

                    if data.pending {
                        ui.add_space(spacing::SPACING_SM);
                        crate::ui::animations::cyber::cyber_spinner(
                            ui,
                            theme.brand,
                            Some(crate::ui::animations::cyber::CyberSpinnerSize::Sm),
                        );
                    }
                });
            });
        });

    if !open {
        app.dispatch(Action::Review(ReviewAction::CloseSendToPrModal));
    }
}
