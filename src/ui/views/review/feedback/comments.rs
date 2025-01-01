use crate::domain::Comment;
use crate::ui::components::markdown::render_markdown;
use crate::ui::theme::Theme;
use crate::ui::typography;
use crate::ui::views::review::format_timestamp;
use eframe::egui;

pub(crate) fn render_comment_list(
    ui: &mut egui::Ui,
    comments: &[Comment],
    theme: &Theme,
) -> Option<crate::ui::app::ReviewAction> {
    let mut action_out = None;
    ui.vertical(|ui| {
        for comment in comments {
            if let Some(action) = render_comment_bubble(ui, comment, theme) {
                action_out = Some(action);
            }
            ui.add_space(crate::ui::spacing::SPACING_MD);
        }
    });
    action_out
}

fn render_comment_bubble(
    ui: &mut egui::Ui,
    comment: &Comment,
    theme: &Theme,
) -> Option<crate::ui::app::ReviewAction> {
    let timestamp = format_timestamp(&comment.created_at);
    let mut action_out = None;

    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.label(
                typography::bold(&comment.author)
                    .size(13.0)
                    .color(theme.text_primary),
            );
            ui.label(typography::tiny(format!("• {}", timestamp)).color(theme.text_muted));
            ui.add_space(crate::ui::spacing::SPACING_MD);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(
                        typography::label(crate::ui::icons::ACTION_DELETE).color(theme.destructive),
                    )
                    .on_hover_text("Delete Comment")
                    .clicked()
                {
                    action_out = Some(crate::ui::app::ReviewAction::DeleteComment {
                        feedback_id: comment.feedback_id.clone(),
                        comment_id: comment.id.clone(),
                    });
                }
            });
        });

        render_markdown(ui, &comment.body);
    });

    action_out
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
            feedback_id: "t1".into(),
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
