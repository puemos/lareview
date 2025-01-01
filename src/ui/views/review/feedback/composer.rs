use crate::ui::app::ReviewAction;
use crate::ui::icons;
use crate::ui::spacing;
use eframe::egui;

pub(crate) fn render_reply_composer(
    ui: &mut egui::Ui,
    task_id: &str,
    feedback_id: Option<String>,
    file_path: Option<String>,
    line_number: Option<u32>,
    side: Option<crate::domain::FeedbackSide>,
    draft_key: &str,
) -> Option<ReviewAction> {
    let ctx = ui.ctx().clone();
    let (reply_draft, title_draft) = crate::ui::app::ui_memory::get_ui_memory(&ctx)
        .feedback_drafts
        .get(draft_key)
        .map(|d| (d.reply.clone(), d.title.clone()))
        .unwrap_or_default();
    let mut action_out = None;

    ui.vertical(|ui| {
        let mut text = reply_draft.to_string();

        let response = ui.add(
            egui::TextEdit::multiline(&mut text)
                .hint_text("Reply...")
                .font(egui::TextStyle::Body)
                .frame(false)
                .desired_rows(3)
                .desired_width(f32::INFINITY)
                .margin(egui::vec2(0.0, 0.0))
                .lock_focus(true),
        );

        if response.changed() {
            crate::ui::app::ui_memory::with_ui_memory_mut(&ctx, |mem| {
                mem.feedback_drafts
                    .entry(draft_key.to_string())
                    .or_default()
                    .reply = text.clone();
            });
        }

        ui.add_space(spacing::SPACING_SM);
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let can_send = !text.trim().is_empty();
                let old_padding = ui.spacing().button_padding;
                ui.spacing_mut().button_padding = egui::vec2(14.0, 8.0);
                if ui
                    .add_enabled(
                        can_send,
                        egui::Button::new(format!("{} Send Reply", icons::ACTION_REPLY)),
                    )
                    .clicked()
                {
                    let title = if title_draft.trim().is_empty() {
                        None
                    } else {
                        Some(title_draft.to_string())
                    };
                    action_out = Some(ReviewAction::CreateFeedbackComment {
                        task_id: task_id.to_string(),
                        feedback_id: feedback_id.clone(),
                        file_path: file_path.clone(),
                        line_number,
                        side,
                        title,
                        body: text.trim().to_string(),
                    });
                }
                ui.spacing_mut().button_padding = old_padding;
            });
        });
    });

    action_out
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    #[test]
    fn test_render_reply_composer() {
        let mut harness = Harness::new_ui(|ui| {
            render_reply_composer(ui, "task1", None, None, None, None, "test_draft");
        });
        harness.run();
        harness.get_by_role(egui::accesskit::Role::MultilineTextInput);
        harness
            .get_all_by_role(egui::accesskit::Role::Button)
            .into_iter()
            .find(|n| format!("{:?}", n).contains("Send Reply"))
            .expect("Send Reply button not found");
    }
}
