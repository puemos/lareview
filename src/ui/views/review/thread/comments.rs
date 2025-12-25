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

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    #[test]
    fn test_render_comment_list() {
        let comments = vec![Comment {
            id: "c1".into(),
            thread_id: "t1".into(),
            author: "User A".into(),
            body: "Hello world".into(),
            parent_id: None,
            created_at: "2024-01-01T00:00:00Z".into(),
            updated_at: "2024-01-01T00:00:00Z".into(),
        }];
        let mut harness = Harness::new_ui(|ui| {
            render_comment_list(ui, &comments, &Theme::mocha());
        });
        harness.run();
        harness.get_by_label("User A");
        harness.get_by_label("Hello world");
    }
}
