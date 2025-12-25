use crate::ui::app::LaReviewApp;
use egui::FontFamily;
use egui_kittest::Harness;
use std::sync::{Arc, Mutex};

pub fn setup_harness(app: Arc<Mutex<LaReviewApp>>) -> Harness<'static> {
    let app_clone = app.clone();
    Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .build(move |ctx: &egui::Context| {
            LaReviewApp::setup_fonts(ctx);
            let ready = ctx.fonts(|f| f.families().contains(&FontFamily::Name("GeistBold".into())));
            if ready {
                app_clone.lock().unwrap().render(ctx);
            }
        })
}
