use crate::ui::app::{Action, LaReviewApp, ReviewAction};
use crate::ui::components::list_item::ListItem;
use crate::ui::theme::current_theme;
use crate::ui::views::review::format_timestamp;
use crate::ui::{spacing, typography};
use eframe::egui;
use egui_phosphor::regular as icons;

impl LaReviewApp {
    pub(crate) fn render_discussion_tab(
        &mut self,
        ui: &mut egui::Ui,
        task: &crate::domain::ReviewTask,
    ) {
        if ui.available_width() < 50.0 {
            return;
        }

        if let Some(thread_ctx) = &self.state.ui.active_thread {
            let view = crate::ui::views::review::thread_detail::ThreadDetailView {
                task_id: task.id.clone(),
                thread_id: thread_ctx.thread_id.clone(),
                file_path: thread_ctx.file_path.clone(),
                line_number: thread_ctx.line_number,
            };
            self.render_thread_detail(ui, &view);
            return;
        }

        let theme = current_theme();
        let mut task_threads: Vec<crate::domain::Thread> = self
            .state
            .domain
            .threads
            .iter()
            .filter(|thread| thread.task_id.as_ref() == Some(&task.id))
            .cloned()
            .collect();

        if task_threads.is_empty() {
            egui::Frame::NONE
                .inner_margin(spacing::SPACING_XL)
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(40.0);
                        ui.label(
                            typography::body(icons::CHAT_CIRCLE)
                                .size(44.0)
                                .color(theme.text_disabled),
                        );
                        ui.add_space(spacing::SPACING_MD);
                        ui.label(typography::h1("No discussions yet"));
                        ui.label(typography::weak(
                            "Add comments in the 'Changes' tab or start a general thread.",
                        ));
                    });
                });
            return;
        }

        task_threads.sort_by(|a, b| {
            a.status
                .rank()
                .cmp(&b.status.rank())
                .then_with(|| b.updated_at.cmp(&a.updated_at))
                .then_with(|| b.created_at.cmp(&a.created_at))
        });

        for (index, thread) in task_threads.iter().enumerate() {
            let (path, line) = thread
                .anchor
                .as_ref()
                .map(|a| {
                    (
                        a.file_path.clone().unwrap_or_default(),
                        a.line_number.unwrap_or(0),
                    )
                })
                .unwrap_or_default();

            let title = if thread.title.is_empty() {
                "Untitled thread".to_string()
            } else {
                thread.title.clone()
            };

            let status_v = crate::ui::views::review::visuals::status_visuals(thread.status, &theme);
            let impact_v = crate::ui::views::review::visuals::impact_visuals(thread.impact, &theme);

            let comments = self.state.domain.thread_comments.get(&thread.id);
            let reply_count = comments
                .map(|items: &Vec<crate::domain::Comment>| items.len())
                .unwrap_or(0);
            let _updated_label = format_timestamp(&thread.updated_at);

            // Metadata: status icon/label + impact icon/label + comments count
            let mut metadata_job = egui::text::LayoutJob::default();

            // Status
            metadata_job.append(
                status_v.icon,
                0.0,
                egui::TextFormat {
                    font_id: egui::FontId::proportional(12.0),
                    color: status_v.color,
                    ..Default::default()
                },
            );
            metadata_job.append(
                &format!(" {} · ", status_v.label),
                0.0,
                egui::TextFormat {
                    font_id: egui::FontId::proportional(12.0),
                    color: theme.text_secondary,
                    ..Default::default()
                },
            );

            // Impact
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
                &format!(" {} · ", impact_v.label),
                0.0,
                egui::TextFormat {
                    font_id: egui::FontId::proportional(12.0),
                    color: theme.text_secondary,
                    ..Default::default()
                },
            );

            // Count + Path
            let count_label = if path.is_empty() {
                format!("{} comments", reply_count)
            } else {
                format!("{} comments • {}", reply_count, path)
            };
            metadata_job.append(
                &count_label,
                0.0,
                egui::TextFormat {
                    font_id: egui::FontId::proportional(12.0),
                    color: theme.text_secondary,
                    ..Default::default()
                },
            );

            let mut action_out = None;
            ListItem::new(typography::bold(&title).color(theme.text_primary))
                .metadata(egui::WidgetText::from(metadata_job))
                .action(|| {
                    action_out = Some(ReviewAction::OpenThread {
                        task_id: task.id.clone(),
                        thread_id: Some(thread.id.clone()),
                        file_path: if path.is_empty() {
                            None
                        } else {
                            Some(path.clone())
                        },
                        line_number: if line == 0 { None } else { Some(line) },
                    });
                })
                .show_with_bg(ui, &theme);

            if let Some(action) = action_out {
                self.dispatch(Action::Review(action));
            }

            if index + 1 < task_threads.len() {
                ui.separator();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    #[test]
    fn test_render_discussion_tab_with_threads() {
        let mut app = LaReviewApp::new_for_test();
        let task = crate::domain::ReviewTask {
            id: "task1".into(),
            ..Default::default()
        };
        let thread = crate::domain::Thread {
            id: "thread1".into(),
            review_id: "rev1".into(),
            task_id: Some("task1".into()),
            title: "Discussion Title".into(),
            status: crate::domain::ReviewStatus::Todo,
            impact: crate::domain::ThreadImpact::Nitpick,
            anchor: None,
            author: "User".into(),
            created_at: "now".into(),
            updated_at: "now".into(),
        };
        app.state.domain.threads.push(thread);

        let mut harness = Harness::new(|ctx| {
            crate::ui::app::LaReviewApp::setup_fonts(ctx);
            egui::CentralPanel::default().show(ctx, |ui| {
                app.render_discussion_tab(ui, &task);
            });
        });
        harness.run();
        harness.get_by_role(egui::accesskit::Role::Button);
        harness
            .get_all_by_role(egui::accesskit::Role::Label)
            .into_iter()
            .find(|n| n.value().as_deref() == Some("Discussion Title"))
            .expect("Discussion Title label not found");
    }
}
