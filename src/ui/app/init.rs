use std::sync::Arc;

use eframe::egui;
use eframe::egui::FontDefinitions;

use crate::ui::app::{Action, SettingsAction};
use egui::{FontData, FontFamily};
use tokio::sync::mpsc;

use crate::infra::db::Database;

use super::LaReviewApp;
use super::state::{AppState, AppView, SelectedAgent};

impl LaReviewApp {
    pub fn setup_fonts(ctx: &egui::Context) {
        let mut fonts = FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

        let phosphor_font = fonts
            .font_data
            .keys()
            .find(|name| name.to_lowercase().contains("phosphor"))
            .cloned()
            .unwrap_or_else(|| "phosphor-regular".to_owned());

        // Load Geist for proportional text
        fonts.font_data.insert(
            "Geist".to_owned(),
            FontData::from_static(
                crate::assets::get_content("assets/fonts/Geist.ttf").expect("Geist font missing"),
            )
            .into(),
        );

        fonts.font_data.insert(
            "GeistBold".to_owned(),
            FontData::from_static(
                crate::assets::get_content("assets/fonts/Geist-Bold.ttf")
                    .expect("Geist-Bold font missing"),
            )
            .into(),
        );

        fonts.font_data.insert(
            "GeistItalic".to_owned(),
            FontData::from_static(
                crate::assets::get_content("assets/fonts/Geist-Italic.ttf")
                    .expect("Geist-Italic font missing"),
            )
            .into(),
        );

        // Load Geist Mono for monospace text
        fonts.font_data.insert(
            "GeistMono".to_owned(),
            FontData::from_static(
                crate::assets::get_content("assets/fonts/GeistMono.ttf")
                    .expect("GeistMono font missing"),
            )
            .into(),
        );

        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "Geist".to_owned());

        fonts
            .families
            .entry(FontFamily::Monospace)
            .or_default()
            .insert(0, "GeistMono".to_owned());

        // Pre-bind custom names to standard families to avoid panics if they are not yet fully loaded
        fonts.families.insert(
            FontFamily::Name("GeistBold".into()),
            vec![FontFamily::Proportional.to_string(), phosphor_font.clone()],
        );
        fonts.families.insert(
            FontFamily::Name("GeistItalic".into()),
            vec![FontFamily::Proportional.to_string(), phosphor_font.clone()],
        );
        fonts.families.insert(
            FontFamily::Name("Geist".into()),
            vec![FontFamily::Proportional.to_string(), phosphor_font.clone()],
        );

        fonts.families.insert(
            FontFamily::Name("Geist".into()),
            vec!["Geist".to_owned(), phosphor_font.clone()],
        );
        fonts.families.insert(
            FontFamily::Name("GeistBold".into()),
            vec!["GeistBold".to_owned(), phosphor_font.clone()],
        );
        fonts.families.insert(
            FontFamily::Name("GeistItalic".into()),
            vec!["GeistItalic".to_owned(), phosphor_font.clone()],
        );
        fonts.families.insert(
            FontFamily::Name("GeistMono".into()),
            vec!["GeistMono".to_owned(), phosphor_font.clone()],
        );

        ctx.set_fonts(fonts);
    }

    pub fn new_egui(cc: &eframe::CreationContext<'_>) -> Self {
        Self::setup_fonts(&cc.egui_ctx);
        egui_extras::install_image_loaders(&cc.egui_ctx);

        let db = Database::open().expect("db open");

        let task_repo = Arc::new(db.task_repo());
        let feedback_repo = Arc::new(db.feedback_repo());
        let comment_repo = Arc::new(db.comment_repo());
        let review_repo = Arc::new(db.review_repo());
        let run_repo = Arc::new(db.run_repo());
        let repo_repo = Arc::new(db.repo_repo());

        let config = crate::infra::app_config::load_config();

        let mut state = AppState {
            session: crate::ui::app::state::SessionState {
                selected_agent: SelectedAgent::new("codex"),
                ..Default::default()
            },
            ui: crate::ui::app::state::UiState {
                current_view: AppView::Generate,
                ..Default::default()
            },
            ..Default::default()
        };

        state.ui.has_seen_requirements = config.has_seen_requirements;
        state.ui.show_requirements_modal = !config.has_seen_requirements;
        state.ui.agent_path_overrides = config.agent_path_overrides;
        state.ui.custom_agents = config.custom_agents;
        state.ui.agent_envs = config.agent_envs;
        state.ui.preferred_editor_id = config.preferred_editor_id;

        if let Ok(repos) = repo_repo.find_all() {
            state.domain.linked_repos = repos;
        }

        if let Ok(reviews) = review_repo.list_all() {
            state.domain.reviews = reviews;
            if let Some(first) = state.domain.reviews.first() {
                state.ui.selected_review_id = Some(first.id.clone());
                state.ui.selected_run_id = first.active_run_id.clone();
            }
        } else {
            state.ui.review_error = Some("Failed to load reviews".to_string());
        }

        let (gen_tx, gen_rx) = mpsc::channel(32);
        let (gh_tx, gh_rx) = mpsc::channel(8);
        let (d2_install_tx, d2_install_rx) = mpsc::channel(32);
        let (action_tx, action_rx) = mpsc::channel(32);

        let mut app = Self {
            state,
            task_repo,
            feedback_repo,
            comment_repo,
            review_repo,
            run_repo,
            repo_repo,
            _db: db,
            gen_tx,
            gen_rx,
            gh_tx,
            gh_rx,
            d2_install_tx,
            d2_install_rx,
            action_tx,
            action_rx,
            agent_task: None,
            agent_cancel_token: None,
            skip_runtime: false,
        };

        if let Some(image_bytes) = crate::assets::get_content("assets/logo/512-mac.png")
            && let Ok(image) = image::load_from_memory(image_bytes)
        {
            let size = [image.width() as usize, image.height() as usize];
            let rgba = image.to_rgba8();
            let pixels = rgba.as_raw();

            let _logo_handle = cc.egui_ctx.load_texture(
                "app_logo",
                egui::ColorImage::from_rgba_unmultiplied(size, pixels),
                egui::TextureOptions::LINEAR,
            );
        }

        if app.state.session.gh_status.is_none()
            && app.state.session.gh_status_error.is_none()
            && !app.state.session.is_gh_status_checking
        {
            app.dispatch(Action::Settings(SettingsAction::CheckGitHubStatus));
        }

        app.sync_review_from_db();
        app
    }

    pub fn new_for_test() -> Self {
        let db = Database::open_in_memory().expect("db open");

        let task_repo = Arc::new(db.task_repo());
        let feedback_repo = Arc::new(db.feedback_repo());
        let comment_repo = Arc::new(db.comment_repo());
        let review_repo = Arc::new(db.review_repo());
        let run_repo = Arc::new(db.run_repo());
        let repo_repo = Arc::new(db.repo_repo());

        let state = AppState {
            session: crate::ui::app::state::SessionState {
                selected_agent: SelectedAgent::new("codex"),
                ..Default::default()
            },
            ui: crate::ui::app::state::UiState {
                current_view: AppView::Generate,
                ..Default::default()
            },
            ..Default::default()
        };

        let (gen_tx, gen_rx) = mpsc::channel(32);
        let (gh_tx, gh_rx) = mpsc::channel(8);
        let (d2_install_tx, d2_install_rx) = mpsc::channel(32);
        let (action_tx, action_rx) = mpsc::channel(32);

        Self {
            state,
            task_repo,
            feedback_repo,
            comment_repo,
            review_repo,
            run_repo,
            repo_repo,
            _db: db,
            gen_tx,
            gen_rx,
            gh_tx,
            gh_rx,
            d2_install_tx,
            d2_install_rx,
            action_tx,
            action_rx,
            agent_task: None,
            agent_cancel_token: None,
            skip_runtime: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_setup_fonts() {
        let ctx = egui::Context::default();
        LaReviewApp::setup_fonts(&ctx);
    }
}
