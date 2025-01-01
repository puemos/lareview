use crate::ui::app::{Action, FullDiffView, LaReviewApp, ReviewAction};
use crate::ui::components::DiffAction;
use crate::ui::spacing;
use eframe::egui;
use std::sync::Arc;

impl LaReviewApp {
    pub(crate) fn render_changes_tab(
        &mut self,
        ui: &mut egui::Ui,
        task: &crate::domain::ReviewTask,
    ) {
        if ui.available_width() < 50.0 {
            return;
        }

        let unified_diff = if let Some(cached) =
            crate::ui::app::ui_memory::get_ui_memory(ui.ctx()).get_cached_diff(&task.id)
        {
            cached
        } else {
            let new_diff = if !task.diff_refs.is_empty() {
                let run = self.state.domain.runs.iter().find(|r| r.id == task.run_id);

                match run {
                    Some(run) => match crate::infra::diff_index::DiffIndex::new(&run.diff_text) {
                        Ok(diff_index) => match diff_index.render_unified_diff(&task.diff_refs) {
                            Ok((diff_text, _ordered_files)) => diff_text,
                            Err(_) => String::new(),
                        },
                        Err(_) => String::new(),
                    },
                    None => String::new(),
                }
            } else {
                String::new()
            };

            let new_diff_arc: Arc<str> = Arc::from(new_diff);
            let arc_copy = new_diff_arc.clone();
            crate::ui::app::ui_memory::with_ui_memory_mut(ui.ctx(), |mem| {
                mem.cache_diff(task.id.clone(), new_diff_arc);
            });
            arc_copy
        };

        egui::Frame::NONE
            .inner_margin(spacing::SPACING_XL)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.set_min_height(300.0);

                    ui.push_id(("unified_diff", &task.id), |ui| {
                        let action =
                            crate::ui::components::render_diff_editor_with_comment_callback(
                                ui,
                                &unified_diff,
                                "diff",
                                true,
                                None,
                                None,
                            );

                        match action {
                            DiffAction::OpenFullWindow => {
                                self.dispatch(Action::Review(ReviewAction::OpenFullDiff(
                                    FullDiffView {
                                        title: format!("Task diff - {}", task.title),
                                        text: unified_diff.clone(),
                                    },
                                )));
                            }
                            DiffAction::AddNote {
                                line_number,
                                file_path,
                                side,
                                ..
                            } => {
                                self.dispatch(Action::Review(ReviewAction::OpenFeedback {
                                    task_id: task.id.clone(),
                                    feedback_id: None,
                                    file_path: Some(file_path),
                                    line_number: Some(line_number as u32),
                                    side: Some(side),
                                }));
                            }
                            DiffAction::ViewNotes {
                                file_path,
                                line_number,
                            } => {
                                let feedback_id = self
                                    .state
                                    .domain
                                    .feedbacks
                                    .iter()
                                    .find(|feedback| {
                                        feedback.task_id.as_ref() == Some(&task.id)
                                            && feedback
                                                .anchor
                                                .as_ref()
                                                .and_then(|a| a.file_path.as_ref())
                                                == Some(&file_path)
                                            && feedback.anchor.as_ref().and_then(|a| a.line_number)
                                                == Some(line_number)
                                    })
                                    .map(|feedback| feedback.id.clone());
                                let side = self
                                    .state
                                    .domain
                                    .feedbacks
                                    .iter()
                                    .find(|feedback| Some(&feedback.id) == feedback_id.as_ref())
                                    .and_then(|f| f.anchor.as_ref().and_then(|a| a.side));
                                self.dispatch(Action::Review(ReviewAction::OpenFeedback {
                                    task_id: task.id.clone(),
                                    feedback_id,
                                    file_path: Some(file_path),
                                    line_number: Some(line_number),
                                    side,
                                }));
                            }
                            DiffAction::OpenInEditor {
                                file_path,
                                line_number,
                            } => {
                                self.dispatch(Action::Review(ReviewAction::OpenInEditor {
                                    file_path,
                                    line_number,
                                }));
                            }
                            _ => {}
                        }
                    });
                });
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::Harness;

    #[test]
    fn test_render_changes_tab_empty() {
        let mut app = LaReviewApp::new_for_test();
        let task = crate::domain::ReviewTask {
            id: "task1".into(),
            ..Default::default()
        };
        let mut harness = Harness::new(|ctx| {
            crate::ui::app::LaReviewApp::setup_fonts(ctx);
            egui::CentralPanel::default().show(ctx, |ui| {
                app.render_changes_tab(ui, &task);
            });
        });
        harness.run();
    }
}
