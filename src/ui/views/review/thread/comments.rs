use crate::domain::Comment;
use crate::ui::theme::Theme;
use crate::ui::typography;
use crate::ui::views::review::format_timestamp;
use eframe::egui;

pub(crate) fn render_comment_list(ui: &mut egui::Ui, comments: &[Comment], theme: &Theme) {
    ui.vertical(|ui| {
        for comment in comments {
            render_comment_bubble(ui, comment, theme);
            ui.add_space(crate::ui::spacing::SPACING_MD);
        }
    });
}

fn render_comment_bubble(ui: &mut egui::Ui, comment: &Comment, theme: &Theme) {
    let timestamp = format_timestamp(&comment.created_at);

    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.label(
                typography::bold(&comment.author)
                    .size(13.0)
                    .color(theme.text_primary),
            );
            ui.label(typography::tiny(format!("• {}", timestamp)).color(theme.text_muted));
        });

        ui.label(
            typography::body(&comment.body)
                .color(theme.text_secondary)
                .line_height(Some(26.0)),
        );
    });
}
