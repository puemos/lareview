//! Main application state with Entity pattern for interactive UI

use gpui::{div, prelude::*, px, Context, Entity, SharedString, VisualContext, Window};
use gpui::window::WindowContext;
use std::ops::DerefMut;

use crate::domain::{PullRequest, ReviewTask};

use super::theme::theme;
use super::views::{generate_view::GenerateView, review_view::ReviewView};

/// Current view in the application
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppView {
    Generate,
    Review,
}

/// Selected agent for task generation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectedAgent {
    Stub,
    Codex,
    Gemini,
}

impl Default for SelectedAgent {
    fn default() -> Self {
        Self::Stub
    }
}

/// Shared application state
pub struct AppState {
    pub current_view: AppView,
    pub pr: Option<PullRequest>,
    pub tasks: Vec<ReviewTask>,
    pub is_generating: bool,
    pub generation_error: Option<String>,
    pub selected_agent: SelectedAgent,
    pub diff_text: String,
    pub pr_id: String,
    pub pr_title: String,
    pub pr_repo: String,
    pub pr_author: String,
    pub pr_branch: String,
    pub selected_task_id: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            current_view: AppView::Generate,
            pr: None,
            tasks: Vec::new(),
            is_generating: false,
            generation_error: None,
            selected_agent: SelectedAgent::default(),
            diff_text: String::new(),
            pr_id: "local-pr".to_string(),
            pr_title: "Local Review".to_string(),
            pr_repo: "local/repo".to_string(),
            pr_author: "me".to_string(),
            pr_branch: "main".to_string(),
            selected_task_id: None,
        }
    }
}

/// Main application - holds entity reference to shared state
pub struct LaReviewApp {
    state: Entity<AppState>,
}

impl LaReviewApp {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let state = cx.new(|_| AppState::default());
        Self { state }
    }

    pub fn switch_to_review(&mut self, cx: &mut Context<Self>) {
        self.state.update(cx, |state, _| {
            state.current_view = AppView::Review;
        });
    }

    pub fn switch_to_generate(&mut self, cx: &mut Context<Self>) {
        self.state.update(cx, |state, _| {
            state.current_view = AppView::Generate;
        });
    }
}

impl Render for LaReviewApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = theme().colors;
        let state = self.state.read(cx);
        let current_view = state.current_view;

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(colors.bg)
            .text_color(colors.text)
            .font_family("Inter")
            .child(self.render_header(current_view, cx))
            .child(self.render_content(current_view, cx))
    }
}

impl LaReviewApp {
    fn render_header(&self, current_view: AppView, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = theme().colors;
        let spacing = theme().spacing;

        div()
            .flex()
            .items_center()
            .justify_between()
            .px(px(spacing.space_8))
            .py(px(spacing.space_4))
            .bg(colors.surface)
            .border_b_1()
            .border_color(colors.border_strong)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(spacing.space_4))
                    .child(
                        div()
                            .text_xl()
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(colors.text_strong)
                            .child("LaReview"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(colors.text_muted)
                            .child("Intent-first PR review"),
                    ),
            )
            .child(self.render_nav(current_view, cx))
    }

    fn render_nav(&self, current_view: AppView, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = theme().colors;
        let spacing = theme().spacing;

        div()
            .flex()
            .gap(px(spacing.space_4))
            .child(
                div()
                    .id("nav-generate")
                    .px(px(spacing.space_4))
                    .py(px(spacing.space_2))
                    .bg(if current_view == AppView::Generate {
                        colors.primary
                    } else {
                        colors.surface_alt
                    })
                    .text_color(if current_view == AppView::Generate {
                        colors.primary_contrast
                    } else {
                        colors.text
                    })
                    .border_1()
                    .border_color(colors.border_strong)
                    .text_sm()
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.switch_to_generate(cx);
                    }))
                    .child("Generate"),
            )
            .child(
                div()
                    .id("nav-review")
                    .px(px(spacing.space_4))
                    .py(px(spacing.space_2))
                    .bg(if current_view == AppView::Review {
                        colors.primary
                    } else {
                        colors.surface_alt
                    })
                    .text_color(if current_view == AppView::Review {
                        colors.primary_contrast
                    } else {
                        colors.text
                    })
                    .border_1()
                    .border_color(colors.border_strong)
                    .text_sm()
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.switch_to_review(cx);
                    }))
                    .child("Review"),
            )
    }

    fn render_content(&self, current_view: AppView, cx: &mut Context<Self>) -> impl IntoElement {
        let spacing = theme().spacing;
        let state_entity = self.state.clone();

        div()
            .flex_1()
            .p(px(spacing.space_8))
            .overflow_hidden()
            .child(match current_view {
                AppView::Generate => {
                    let cx: &mut WindowContext = cx.deref_mut();
                    cx.new_view(|cx| GenerateView::new(state_entity.clone(), cx)).into_any_element()
                }
                AppView::Review => {
                    let cx: &mut WindowContext = cx.deref_mut();
                    cx.new_view(|cx| ReviewView::new(state_entity.clone(), cx)).into_any_element()
                }
            })
    }
}
