use crate::ui::app::{Action, FullDiffView, LaReviewApp, ReviewAction};
use crate::ui::components::DiffAction;
use crate::ui::spacing;
use eframe::egui;

impl LaReviewApp {
    pub(crate) fn render_changes_tab(
        &mut self,
        ui: &mut egui::Ui,
        task: &crate::domain::ReviewTask,
    ) {
        if ui.available_width() < 50.0 {
            return;
        }

        let unified_diff = match &self.state.ui.cached_unified_diff {
            Some((cached_diff_refs, diff_string)) if cached_diff_refs == &task.diff_refs => {
                diff_string.clone()
            }
            _ => {
                let new_diff = if !task.diff_refs.is_empty() {
                    let run = self.state.domain.runs.iter().find(|r| r.id == task.run_id);
                    match run {
                        Some(run) => match crate::infra::diff_index::DiffIndex::new(&run.diff_text)
                        {
                            Ok(diff_index) => match diff_index.render_unified_diff(&task.diff_refs)
                            {
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

                self.state.ui.cached_unified_diff =
                    Some((task.diff_refs.clone(), new_diff.clone()));
                new_diff
            }
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
                                ..
                            } => {
                                self.dispatch(Action::Review(ReviewAction::OpenThread {
                                    task_id: task.id.clone(),
                                    thread_id: None,
                                    file_path: Some(file_path),
                                    line_number: Some(line_number as u32),
                                }));
                            }
                            DiffAction::ViewNotes {
                                file_path,
                                line_number,
                            } => {
                                let thread_id = self
                                    .state
                                    .domain
                                    .threads
                                    .iter()
                                    .find(|thread| {
                                        thread.task_id.as_ref() == Some(&task.id)
                                            && thread
                                                .anchor
                                                .as_ref()
                                                .and_then(|a| a.file_path.as_ref())
                                                == Some(&file_path)
                                            && thread.anchor.as_ref().and_then(|a| a.line_number)
                                                == Some(line_number)
                                    })
                                    .map(|thread| thread.id.clone());
                                self.dispatch(Action::Review(ReviewAction::OpenThread {
                                    task_id: task.id.clone(),
                                    thread_id,
                                    file_path: Some(file_path),
                                    line_number: Some(line_number),
                                }));
                            }
                            _ => {}
                        }
                    });
                });
            });
    }
}
