use std::sync::Arc;

use eframe::egui;
use eframe::egui::FontDefinitions;
use egui::{FontData, FontFamily};
use tokio::sync::mpsc;

use crate::infra::db::{
    CommentRepository, Database, ReviewRepository, ReviewRunRepository, TaskRepository,
    ThreadRepository,
};

use super::LaReviewApp;
use super::state::{AppState, AppView, SelectedAgent};

impl LaReviewApp {
    pub fn new_egui(cc: &eframe::CreationContext<'_>) -> Self {
        let mut fonts = FontDefinitions::default();

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
            .extend([
                "Geist".to_owned(),
                "GeistBold".to_owned(),
                "GeistItalic".to_owned(),
            ]);

        fonts
            .families
            .insert(FontFamily::Name("Geist".into()), vec!["Geist".to_owned()]);
        fonts.families.insert(
            FontFamily::Name("GeistBold".into()),
            vec!["GeistBold".to_owned()],
        );
        fonts.families.insert(
            FontFamily::Name("GeistItalic".into()),
            vec!["GeistItalic".to_owned()],
        );
        fonts
            .families
            .entry(FontFamily::Monospace)
            .or_default()
            .insert(0, "GeistMono".to_owned());

        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

        cc.egui_ctx.set_fonts(fonts);
        egui_extras::install_image_loaders(&cc.egui_ctx);

        let db = Database::open().expect("db open");

        let conn = db.connection();
        let task_repo = Arc::new(TaskRepository::new(conn.clone()));
        let thread_repo = Arc::new(ThreadRepository::new(conn.clone()));
        let comment_repo = Arc::new(CommentRepository::new(conn.clone()));
        let review_repo = Arc::new(ReviewRepository::new(conn.clone()));
        let run_repo = Arc::new(ReviewRunRepository::new(conn.clone()));
        let repo_repo = Arc::new(crate::infra::db::repository::RepoRepository::new(
            conn.clone(),
        ));

        let config = crate::infra::app_config::load_config();

        let mut state = AppState {
            current_view: AppView::Generate,
            selected_agent: SelectedAgent::new("codex"),
            diff_text: String::new(),
            ..Default::default()
        };

        state.extra_path = config.extra_path.clone().unwrap_or_default();
        state.has_seen_requirements = config.has_seen_requirements;
        state.show_requirements_modal = !config.has_seen_requirements;

        if !state.extra_path.trim().is_empty() {
            // set_var is currently unsafe on nightly; this is limited to process-local config.
            unsafe {
                std::env::set_var("LAREVIEW_EXTRA_PATH", state.extra_path.clone());
            }
        }

        if let Ok(repos) = repo_repo.find_all() {
            state.linked_repos = repos;
        }

        if let Ok(reviews) = review_repo.list_all() {
            state.reviews = reviews;
            if let Some(first) = state.reviews.first() {
                state.selected_review_id = Some(first.id.clone());
                state.selected_run_id = first.active_run_id.clone();
            }
        } else {
            state.review_error = Some("Failed to load reviews".to_string());
        }

        let (gen_tx, gen_rx) = mpsc::channel(32);
        let (gh_tx, gh_rx) = mpsc::channel(8);
        let (d2_install_tx, d2_install_rx) = mpsc::channel(32);
        let (action_tx, action_rx) = mpsc::channel(32);

        let mut app = Self {
            state,
            task_repo,
            thread_repo,
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
        };

        if let Some(image_bytes) = crate::assets::get_content("assets/icons/icon-512.png")
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

        app.sync_review_from_db();
        app
    }
}
