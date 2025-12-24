use crate::ui::spacing;
use crate::ui::{theme, typography};
use eframe::egui;

/// Common status label component
#[allow(dead_code)]
pub fn status_label(ui: &mut egui::Ui, text: &str, color: egui::Color32) {
    ui.label(typography::body(text).color(color).size(12.0));
}

/// Common error banner component
pub fn error_banner(ui: &mut egui::Ui, error_message: &str) {
    if ui.available_width() < 50.0 {
        return;
    }
    let theme = theme::current_theme();
    egui::Frame::new()
        .fill(theme.destructive.gamma_multiply(0.2))
        .inner_margin(egui::Margin::symmetric(
            spacing::SPACING_MD as i8,
            spacing::SPACING_SM as i8,
        ))
        .show(ui, |ui| {
            let mut job = egui::text::LayoutJob::default();
            let font_id = egui::TextStyle::Body.resolve(ui.style());

            job.append(
                "error: ",
                0.0,
                egui::TextFormat {
                    font_id: font_id.clone(),
                    color: theme.destructive,
                    ..Default::default()
                },
            );
            job.append(
                error_message,
                0.0,
                egui::TextFormat {
                    font_id,
                    color: theme.text_primary,
                    ..Default::default()
                },
            );

            ui.label(job);
        });
}
