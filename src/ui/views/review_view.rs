//! Review view - main interface for reviewing tasks

use gpui::{Context, Entity, InteractiveElement, SharedString, Window, div, prelude::*, px};

use crate::domain::ReviewTask;
use crate::ui::app::AppState;
use crate::ui::theme::theme;

/// Review view for browsing and completing tasks
pub struct ReviewView {
    state: Entity<AppState>,
}

impl ReviewView {
    pub fn new(state: Entity<AppState>, _cx: &mut Context<impl Render>) -> Self {
        Self { state }
    }

    fn select_task(&self, task_id: String, cx: &mut Context<Self>) {
        self.state.update(cx, |state, _| {
            state.selected_task_id = Some(task_id);
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
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_sm()
                            .text_color(colors.text_muted)
                            .child(format!("TASKS ({})", tasks.len())),
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
                        let risk = task.stats.risk.clone();

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
                                            .text_sm()
                                            .font_weight(if is_selected {
                                                gpui::FontWeight::SEMIBOLD
                                            } else {
                                                gpui::FontWeight::NORMAL
                                            })
                                            .text_color(colors.text)
                                            .child(task_title),
                                    )
                                    .child(
                                        div().flex().items_center().gap(px(spacing.space_2)).child(
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

    fn render_task_detail(&self, task: &ReviewTask, _cx: &mut Context<Self>) -> impl IntoElement {
        let colors = theme().colors;
        let spacing = theme().spacing;

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
                                    )),
                            ),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .p(px(spacing.space_6))
                    .id("patches-panel")
                    .overflow_scroll()
                    .child(div().flex().flex_col().gap(px(spacing.space_6)).children(
                        task.patches.iter().map(|patch| {
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
                        }),
                    )),
            )
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

        let selected_task = state
            .selected_task_id
            .as_ref()
            .and_then(|id| state.tasks.iter().find(|t| &t.id == id).cloned());

        div()
            .flex()
            .size_full()
            .bg(colors.bg)
            .child(self.render_sidebar(cx))
            .child(if let Some(task) = selected_task {
                self.render_task_detail(&task, cx).into_any_element()
            } else {
                self.render_empty_state().into_any_element()
            })
    }
}
