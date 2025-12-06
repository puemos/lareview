//! Review view - main interface for reviewing tasks

use gpui::{
    Context, Entity, InteractiveElement, MouseButton, SharedString, StatefulInteractiveElement,
    Window, div, prelude::*, px,
};

use crate::data::repository::{NoteRepository, TaskRepository};
use crate::domain::{Note, ReviewTask, TaskStatus};
use crate::ui::app::AppState;
use crate::ui::theme::theme;
use std::collections::HashMap;
use std::sync::Arc;

/// Review view for browsing and completing tasks
pub struct ReviewView {
    state: Entity<AppState>,
    note_repo: Arc<NoteRepository>,
    task_repo: Arc<TaskRepository>,
}

impl ReviewView {
    pub fn sync_from_db(&self, cx: &mut Context<Self>) {
        let (pr_id, selected_task_id) = {
            let state = self.state.read(cx);
            (state.pr_id.clone(), state.selected_task_id.clone())
        };

        let tasks = match self.task_repo.find_by_pr(&pr_id) {
            Ok(tasks) => tasks,
            Err(err) => {
                self.state.update(cx, |state, _| {
                    state.review_error = Some(format!("Failed to load tasks: {}", err));
                });
                return;
            }
        };

        let task_ids: Vec<_> = tasks.iter().map(|t| t.id.clone()).collect();
        let notes = match self.note_repo.find_by_tasks(&task_ids) {
            Ok(n) => n,
            Err(err) => {
                self.state.update(cx, |state, _| {
                    state.review_error = Some(format!("Failed to load notes: {}", err));
                });
                return;
            }
        };
        let note_map: HashMap<_, _> = notes.into_iter().map(|n| (n.task_id.clone(), n)).collect();

        self.state.update(cx, |state, _| {
            state.tasks = tasks.clone();
            state.review_error = None;

            let next_selected = selected_task_id
                .clone()
                .filter(|id| tasks.iter().any(|t| &t.id == id))
                .or_else(|| tasks.first().map(|t| t.id.clone()));

            state.selected_task_id = next_selected.clone();
            state.current_note =
                next_selected.and_then(|id| note_map.get(&id).map(|n| n.body.clone()));
        });
    }

    pub fn new(
        state: Entity<AppState>,
        note_repo: Arc<NoteRepository>,
        task_repo: Arc<TaskRepository>,
        _cx: &mut Context<impl Render>,
    ) -> Self {
        Self {
            state,
            note_repo,
            task_repo,
        }
    }

    fn select_task(&self, task_id: String, cx: &mut Context<Self>) {
        let note_repo = self.note_repo.clone();
        let note_result = note_repo.find_by_task(&task_id);

        self.state.update(cx, |state, _| {
            state.selected_task_id = Some(task_id.clone());
            match note_result {
                Ok(Some(n)) => {
                    state.current_note = Some(n.body);
                    state.review_error = None;
                }
                Ok(None) => {
                    state.current_note = None;
                    state.review_error = None;
                }
                Err(err) => {
                    state.current_note = None;
                    state.review_error = Some(format!("Failed to load note: {}", err));
                }
            }
        });
    }

    fn save_note(&self, task_id: &str, body: String, cx: &mut Context<Self>) {
        let note_repo = self.note_repo.clone();
        let task_id = task_id.to_string();
        let timestamp = chrono::Utc::now().to_rfc3339();
        let result = note_repo.save(&Note {
            task_id: task_id.clone(),
            body: body.clone(),
            updated_at: timestamp,
        });

        self.state.update(cx, |state, _| match result {
            Ok(_) => {
                if state.selected_task_id.as_deref() == Some(task_id.as_str()) {
                    state.current_note = if body.is_empty() {
                        None
                    } else {
                        Some(body.clone())
                    };
                }
                state.review_error = None;
            }
            Err(err) => {
                state.review_error = Some(format!("Failed to save note: {}", err));
            }
        });
    }

    fn clear_note(&self, task_id: String, cx: &mut Context<Self>) {
        let result = self.note_repo.delete_by_task(&task_id);
        self.state.update(cx, |state, _| {
            if let Err(err) = result {
                state.review_error = Some(format!("Failed to clear note: {}", err));
            } else if state.selected_task_id.as_deref() == Some(task_id.as_str()) {
                state.current_note = None;
                state.review_error = None;
            }
        });
    }

    fn paste_note_from_clipboard(&self, task_id: String, cx: &mut Context<Self>) {
        let clipboard = cx
            .read_from_clipboard()
            .and_then(|item| item.text())
            .map(|text| text.trim().to_string())
            .filter(|text| !text.is_empty());

        if let Some(text) = clipboard {
            self.save_note(&task_id, text, cx);
        } else {
            self.state.update(cx, |state, _| {
                state.review_error =
                    Some("Clipboard is empty or does not contain text to save as a note.".into());
            });
        }
    }

    fn set_task_status(&self, task_id: String, status: TaskStatus, cx: &mut Context<Self>) {
        let result = self.task_repo.update_status(&task_id, status);
        let task_id_for_state = task_id.clone();
        self.state.update(cx, |state, _| {
            if let Some(task) = state.tasks.iter_mut().find(|t| t.id == task_id_for_state) {
                task.status = status;
            }
            if let Err(err) = result {
                state.review_error = Some(format!("Failed to update status: {}", err));
            } else {
                state.review_error = None;
            }
        });
    }

    fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = theme().colors;
        let spacing = theme().spacing;
        let state = self.state.read(cx);
        let tasks = &state.tasks;
        let selected_id = state.selected_task_id.clone();

        div()
            .w(px(300.0))
            .h_full()
            .flex()
            .flex_col()
            .border_r_1()
            .border_color(colors.border_strong)
            .bg(colors.surface)
            .child(
                div()
                    .p(px(spacing.space_4))
                    .border_b_1()
                    .border_color(colors.border)
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_sm()
                            .text_color(colors.text_muted)
                            .child(format!("TASKS ({})", tasks.len())),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(colors.text)
                            .bg(colors.surface_alt)
                            .border_1()
                            .border_color(colors.border)
                            .rounded_md()
                            .px(px(spacing.space_2))
                            .py(px(spacing.space_1))
                            .cursor_pointer()
                            .on_mouse_up(
                                MouseButton::Left,
                                cx.listener(|this, _event, _window, cx| {
                                    this.sync_from_db(cx);
                                }),
                            )
                            .child("Sync"),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .id("task-list")
                    .overflow_scroll()
                    .flex()
                    .flex_col()
                    .children(tasks.iter().map(|task| {
                        let is_selected = Some(task.id.clone()) == selected_id;
                        let task_id = task.id.clone();
                        let task_title = task.title.clone();
                        let risk = task.stats.risk;
                        let status = task.status;

                        div()
                            .id(SharedString::from(format!("task-{}", task_id)))
                            .on_click(cx.listener(move |this, _event, _window, cx| {
                                this.select_task(task_id.clone(), cx);
                            }))
                            .p(px(spacing.space_3))
                            .cursor_pointer()
                            .bg(if is_selected {
                                colors.surface_alt
                            } else {
                                colors.surface
                            })
                            .border_l_4()
                            .border_color(if is_selected {
                                colors.primary
                            } else {
                                gpui::Hsla::default() // Transparent
                            })
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(spacing.space_1))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .font_weight(if is_selected {
                                                        gpui::FontWeight::SEMIBOLD
                                                    } else {
                                                        gpui::FontWeight::NORMAL
                                                    })
                                                    .text_color(if status == TaskStatus::Reviewed {
                                                        colors.text_muted
                                                    } else {
                                                        colors.text
                                                    })
                                                    .child(task_title),
                                            )
                                            .when(status == TaskStatus::Reviewed, |this| {
                                                this.child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(colors.success)
                                                        .child("✓"),
                                                )
                                            }),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(spacing.space_2))
                                            .child(self.status_chip(status))
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(colors.text_muted)
                                                    .child(format!("{:?}", risk)),
                                            ),
                                    ),
                            )
                    })),
            )
    }

    fn render_task_detail(
        &self,
        task: &ReviewTask,
        current_note: Option<String>,
        review_error: Option<String>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let colors = theme().colors;
        let spacing = theme().spacing;
        let task_id = task.id.clone();

        div()
            .flex_1()
            .flex()
            .flex_col()
            .h_full()
            .id("task-detail")
            .overflow_scroll()
            .child(
                div()
                    .p(px(spacing.space_6))
                    .border_b_1()
                    .border_color(colors.border_strong)
                    .bg(colors.surface)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(spacing.space_4))
                            .child(
                                div()
                                    .text_2xl()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(colors.text_strong)
                                    .child(task.title.clone()),
                            )
                            .child(
                                div()
                                    .text_color(colors.text)
                                    .child(task.description.clone()),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap(px(spacing.space_4))
                                    .child(self.stat_badge(
                                        "Additions",
                                        &format!("+{}", task.stats.additions),
                                        colors.success,
                                    ))
                                    .child(self.stat_badge(
                                        "Deletions",
                                        &format!("-{}", task.stats.deletions),
                                        colors.danger,
                                    ))
                                    .child(self.stat_badge(
                                        "Files",
                                        &task.files.len().to_string(),
                                        colors.primary,
                                    ))
                                    .child(
                                        div()
                                            .flex_1()
                                            .flex()
                                            .justify_end()
                                            .gap(px(spacing.space_2))
                                            .child(self.render_status_button(
                                                "Pending",
                                                TaskStatus::Pending,
                                                task.status,
                                                task_id.clone(),
                                                cx,
                                            ))
                                            .child(self.render_status_button(
                                                "Reviewed",
                                                TaskStatus::Reviewed,
                                                task.status,
                                                task_id.clone(),
                                                cx,
                                            ))
                                            .child(self.render_status_button(
                                                "Ignore",
                                                TaskStatus::Ignored,
                                                task.status,
                                                task_id.clone(),
                                                cx,
                                            )),
                                    ),
                            ),
                    ),
            )
            .when_some(review_error.clone(), |this, err| {
                this.child(
                    div()
                        .bg(colors.danger)
                        .text_color(colors.surface)
                        .p(px(spacing.space_3))
                        .border_b_1()
                        .border_color(colors.border_strong)
                        .child(err),
                )
            })
            .child(
                div()
                    .flex_1()
                    .p(px(spacing.space_6))
                    .id("patches-panel")
                    .overflow_scroll()
                    .child(if task.patches.is_empty() {
                        div()
                            .text_sm()
                            .text_color(colors.text_muted)
                            .child("No patch hunks available for this task.")
                            .into_any_element()
                    } else {
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(spacing.space_6))
                            .children(task.patches.iter().map(|patch| {
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(spacing.space_2))
                                    .child(
                                        div()
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .text_sm()
                                            .text_color(colors.text_muted)
                                            .child(patch.file.clone()),
                                    )
                                    .child(
                                        div()
                                            .p(px(spacing.space_4))
                                            .bg(colors.surface_alt)
                                            .border_1()
                                            .border_color(colors.border)
                                            .font_family("JetBrains Mono")
                                            .text_sm()
                                            .child(patch.hunk.clone()),
                                    )
                            }))
                            .into_any_element()
                    }),
            )
            .when_some(task.insight.clone(), |this, insight| {
                this.child(
                    div()
                        .p(px(spacing.space_6))
                        .border_t_1()
                        .border_color(colors.border)
                        .bg(colors.surface_alt)
                        .child(
                            div()
                                .font_weight(gpui::FontWeight::BOLD)
                                .text_sm()
                                .text_color(colors.text_muted)
                                .mb(px(spacing.space_2))
                                .child("INSIGHT"),
                        )
                        .child(div().text_sm().text_color(colors.text).child(insight)),
                )
            })
            .when_some(task.diagram.clone(), |this, diagram| {
                this.child(
                    div()
                        .p(px(spacing.space_6))
                        .border_t_1()
                        .border_color(colors.border)
                        .bg(colors.surface)
                        .child(
                            div()
                                .font_weight(gpui::FontWeight::BOLD)
                                .text_sm()
                                .text_color(colors.text_muted)
                                .mb(px(spacing.space_2))
                                .child("DIAGRAM"),
                        )
                        .child(
                            div()
                                .p(px(spacing.space_4))
                                .bg(colors.surface_alt)
                                .border_1()
                                .border_color(colors.border)
                                .font_family("JetBrains Mono")
                                .text_sm()
                                .child(diagram),
                        ),
                )
            })
            .child(
                div()
                    .p(px(spacing.space_6))
                    .border_t_1()
                    .border_color(colors.border)
                    .bg(colors.surface_alt)
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_sm()
                            .text_color(colors.text_muted)
                            .mb(px(spacing.space_2))
                            .child("NOTES"),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(spacing.space_3))
                            .child(
                                div()
                                    .p(px(spacing.space_3))
                                    .bg(colors.surface)
                                    .border_1()
                                    .border_color(colors.border)
                                    .rounded_md()
                                    .text_sm()
                                    .text_color(if current_note.is_some() {
                                        colors.text
                                    } else {
                                        colors.text_muted
                                    })
                                    .child(
                                        current_note.unwrap_or_else(|| "No notes yet.".to_string()),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_wrap()
                                    .gap(px(spacing.space_2))
                                    .child(
                                        div()
                                            .px(px(spacing.space_3))
                                            .py(px(spacing.space_2))
                                            .id("note-paste-btn")
                                            .bg(colors.primary)
                                            .text_color(colors.primary_contrast)
                                            .rounded_md()
                                            .text_sm()
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .cursor_pointer()
                                            .on_click(cx.listener({
                                                let task_id = task_id.clone();
                                                move |this, _event, _window, cx| {
                                                    this.paste_note_from_clipboard(
                                                        task_id.clone(),
                                                        cx,
                                                    );
                                                }
                                            }))
                                            .child("Paste note from clipboard"),
                                    )
                                    .child(
                                        div()
                                            .px(px(spacing.space_3))
                                            .py(px(spacing.space_2))
                                            .id("note-sample-btn")
                                            .bg(colors.surface)
                                            .text_color(colors.text)
                                            .border_1()
                                            .border_color(colors.border)
                                            .rounded_md()
                                            .text_sm()
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .cursor_pointer()
                                            .on_click(cx.listener({
                                                let task_id = task_id.clone();
                                                let title = task.title.clone();
                                                move |this, _event, _window, cx| {
                                                    let sample = format!(
                                                        "Reviewed {} — ready for follow-up.",
                                                        title
                                                    );
                                                    this.save_note(&task_id, sample, cx);
                                                }
                                            }))
                                            .child("Save sample note"),
                                    )
                                    .child(
                                        div()
                                            .px(px(spacing.space_3))
                                            .py(px(spacing.space_2))
                                            .id("note-clear-btn")
                                            .bg(colors.surface)
                                            .text_color(colors.text_muted)
                                            .border_1()
                                            .border_color(colors.border)
                                            .rounded_md()
                                            .text_sm()
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .cursor_pointer()
                                            .on_click(cx.listener({
                                                let task_id = task_id.clone();
                                                move |this, _event, _window, cx| {
                                                    this.clear_note(task_id.clone(), cx);
                                                }
                                            }))
                                            .child("Clear note"),
                                    ),
                            ),
                    ),
            )
    }

    fn render_status_button(
        &self,
        label: &str,
        target: TaskStatus,
        current: TaskStatus,
        task_id: String,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let colors = theme().colors;
        let spacing = theme().spacing;
        let is_active = target == current;
        let (bg, text) = match target {
            TaskStatus::Reviewed => (colors.success, colors.surface),
            TaskStatus::Ignored => (colors.warning, colors.surface),
            TaskStatus::Pending => (colors.surface_alt, colors.text),
        };

        div()
            .px(px(spacing.space_3))
            .py(px(spacing.space_1))
            .bg(if is_active { bg } else { colors.surface_alt })
            .text_color(if is_active { text } else { colors.text })
            .border_1()
            .border_color(colors.border)
            .rounded_md()
            .text_sm()
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .cursor_pointer()
            .on_mouse_up(
                MouseButton::Left,
                cx.listener({
                    let task_id = task_id.clone();
                    move |this, _event, _window, cx| {
                        this.set_task_status(task_id.clone(), target, cx);
                    }
                }),
            )
            .child(label.to_string())
    }

    fn status_chip(&self, status: TaskStatus) -> impl IntoElement {
        let colors = theme().colors;
        let spacing = theme().spacing;
        let (label, color) = match status {
            TaskStatus::Pending => ("Pending", colors.text_muted),
            TaskStatus::Reviewed => ("Reviewed", colors.success),
            TaskStatus::Ignored => ("Ignored", colors.warning),
        };

        div()
            .px(px(spacing.space_2))
            .py(px(spacing.space_1))
            .bg(colors.surface_alt)
            .border_1()
            .border_color(colors.border)
            .rounded_md()
            .text_xs()
            .font_weight(gpui::FontWeight::BOLD)
            .text_color(color)
            .child(label.to_string())
    }

    fn stat_badge(&self, label: &str, value: &str, color: gpui::Hsla) -> impl IntoElement {
        let colors = theme().colors;
        let spacing = theme().spacing;

        div()
            .flex()
            .items_center()
            .gap(px(spacing.space_2))
            .child(
                div()
                    .text_xs()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(colors.text_muted)
                    .child(label.to_string()),
            )
            .child(
                div()
                    .px(px(spacing.space_2))
                    .py(px(spacing.space_1))
                    .bg(colors.surface_alt)
                    .border_1()
                    .border_color(colors.border)
                    .rounded_md()
                    .text_xs()
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(color)
                    .child(value.to_string()),
            )
    }

    fn render_empty_state(&self) -> impl IntoElement {
        let colors = theme().colors;
        let spacing = theme().spacing;

        div().flex_1().flex().items_center().justify_center().child(
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(spacing.space_4))
                .child(
                    div()
                        .text_xl()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(colors.text_muted)
                        .child("Select a task to view details"),
                ),
        )
    }
}

impl Render for ReviewView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = theme().colors;
        let state = self.state.read(cx);
        let selected_task_id = state.selected_task_id.clone();
        let current_note = state.current_note.clone();
        let review_error = state.review_error.clone();
        let selected_task = if let Some(id) = selected_task_id {
            state.tasks.iter().find(|t| t.id == id).cloned() // Assuming ReviewTask is Clone
        } else {
            None
        };

        div()
            .flex()
            .size_full()
            .bg(colors.bg)
            .child(self.render_sidebar(cx))
            .child(if let Some(task) = selected_task {
                self.render_task_detail(&task, current_note, review_error, cx)
                    .into_any_element()
            } else {
                self.render_empty_state().into_any_element()
            })
    }
}
