use std::sync::Arc;

use eframe::egui;
use eframe::egui::{FontData, FontDefinitions, FontFamily};
use tokio::sync::mpsc;

use crate::infra::db::{Database, NoteRepository, PullRequestRepository, TaskRepository};

use super::LaReviewApp;
use super::state::{AppState, AppView, SelectedAgent};

impl LaReviewApp {
    pub fn new_egui(cc: &eframe::CreationContext<'_>) -> Self {
        let mut fonts = FontDefinitions::default();
        fonts.font_data.insert(
            "SpaceMono".to_owned(),
            FontData::from_static(include_bytes!(
                "../../../assets/fonts/SpaceMono-Regular.ttf"
            ))
            .into(),
        );
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "SpaceMono".to_owned());
        fonts
            .families
            .entry(FontFamily::Monospace)
            .or_default()
            .insert(0, "SpaceMono".to_owned());

        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

        cc.egui_ctx.set_fonts(fonts);
        egui_extras::install_image_loaders(&cc.egui_ctx);

        let db = Database::open().expect("db open");

        let conn = db.connection();
        let task_repo = Arc::new(TaskRepository::new(conn.clone()));
        let note_repo = Arc::new(NoteRepository::new(conn.clone()));
        let pr_repo = Arc::new(PullRequestRepository::new(conn.clone()));

        let mut state = AppState {
            current_view: AppView::Generate,
            selected_agent: SelectedAgent::new("codex"),
            diff_text: String::new(),
            pr_id: "local-pr".to_string(),
            pr_title: "Local Review".to_string(),
            pr_repo: "local/repo".to_string(),
            pr_author: "me".to_string(),
            pr_branch: "main".to_string(),
            ..Default::default()
        };

        if let Ok(prs) = pr_repo.list_all() {
            state.prs = prs;
            if let Some(first_pr) = state.prs.first() {
                state.selected_pr_id = Some(first_pr.id.clone());
                state.pr_id = first_pr.id.clone();
                state.pr_title = first_pr.title.clone();
                state.pr_repo = first_pr.repo.clone();
                state.pr_author = first_pr.author.clone();
                state.pr_branch = first_pr.branch.clone();
            }
        } else {
            state.review_error = Some("Failed to load pull requests".to_string());
        }

        let (gen_tx, gen_rx) = mpsc::channel(32);
        let (d2_install_tx, d2_install_rx) = mpsc::channel(32);

        let mut app = Self {
            state,
            task_repo,
            note_repo,
            pr_repo,
            _db: db,
            gen_tx,
            gen_rx,
            d2_install_tx,
            d2_install_rx,
        };

        if let Ok(image_bytes) = std::fs::read("assets/icons/icon-512.png")
            && let Ok(image) = image::load_from_memory(&image_bytes)
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
