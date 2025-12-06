//! Main application state with Entity pattern for interactive UI

use gpui::{Context, Entity, Window, div, prelude::*, px};

use crate::data::db::Database;
use crate::data::repository::{NoteRepository, PullRequestRepository, TaskRepository};
use crate::domain::ReviewTask;

use super::theme::theme;
use super::views::{generate_view::GenerateView, review_view::ReviewView};
use std::sync::Arc;

/// Current view in the application
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppView {
    Generate,
    Review,
}

/// Selected agent for task generation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectedAgent {
    Codex,
    Gemini,
}

/// Shared application state
pub struct AppState {
    pub current_view: AppView,
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
    pub agent_messages: Vec<String>,
    pub agent_thoughts: Vec<String>,
    pub agent_logs: Vec<String>,
    pub current_note: Option<String>,
    pub review_error: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            current_view: AppView::Generate,
            tasks: Vec::new(),
            is_generating: false,
            generation_error: None,
            selected_agent: SelectedAgent::Codex,
            diff_text: String::new(),
            pr_id: "local-pr".to_string(),
            pr_title: "Local Review".to_string(),
            pr_repo: "local/repo".to_string(),
            pr_author: "me".to_string(),
            pr_branch: "main".to_string(),
            selected_task_id: None,
            agent_messages: Vec::new(),
            agent_thoughts: Vec::new(),
            agent_logs: Vec::new(),
            current_note: None,
            review_error: None,
        }
    }
}

/// Main application - holds entity reference to shared state
pub struct LaReviewApp {
    state: Entity<AppState>,
    generate_view: Entity<GenerateView>,
    review_view: Entity<ReviewView>,
    _db: Database, // Keep db alive
}

impl LaReviewApp {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let db = Database::open().expect("Failed to open database");
        // Make DB path discoverable for ACP worker threads
        let _ = std::env::var("LAREVIEW_DB_PATH").map_err(|_| unsafe {
            std::env::set_var("LAREVIEW_DB_PATH", db.path().to_string_lossy().to_string())
        });
        let conn = db.connection();

        let task_repo = Arc::new(TaskRepository::new(conn.clone()));
        let pr_repo = Arc::new(PullRequestRepository::new(conn.clone()));
        let note_repo = Arc::new(NoteRepository::new(conn.clone()));

        // Load existing tasks for the default PR (local-pr)
        // In a real app we'd have a PR selection screen
        let initial_tasks = task_repo
            .find_by_pr(&"local-pr".to_string())
            .unwrap_or_default();

        let state = cx.new(|_| AppState {
            tasks: initial_tasks,
            ..AppState::default()
        });

        let generate_view =
            cx.new(|cx| GenerateView::new(state.clone(), task_repo.clone(), pr_repo, cx));
        let review_view =
            cx.new(|cx| ReviewView::new(state.clone(), note_repo, task_repo.clone(), cx));

        Self {
            state,
            generate_view,
            review_view,
            _db: db,
        }
    }

    pub fn switch_to_review(&mut self, cx: &mut Context<Self>) {
        self.state.update(cx, |state, _| {
            state.current_view = AppView::Review;
        });
        let review_view = self.review_view.clone();
        cx.update_entity(&review_view, |view, cx| {
            view.sync_from_db(cx);
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

    fn render_content(&self, current_view: AppView, _cx: &mut Context<Self>) -> impl IntoElement {
        let spacing = theme().spacing;

        div()
            .flex_1()
            .p(px(spacing.space_8))
            .id("content-scroll")
            .overflow_scroll()
            .child(match current_view {
                AppView::Generate => self.generate_view.clone().into_any_element(),
                AppView::Review => self.review_view.clone().into_any_element(),
            })
    }
}
