use crate::domain::ReviewTask;
use crate::ui::app::{DomainState, ReviewAction, UiState};
use crate::ui::icons;
use crate::ui::spacing;
use crate::ui::theme::Theme;
use crate::ui::typography;
use eframe::egui;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub(crate) enum ReviewTab {
    Description,
    Diagram,
    Changes,
    Discussion,
}

pub(crate) fn render_task_tabs(
    ui: &mut egui::Ui,
    task: &ReviewTask,
    ui_state: &UiState,
    domain_state: &DomainState,
    theme: &Theme,
) -> (ReviewTab, Option<ReviewAction>) {
    let mut active_tab = ui
        .ctx()
        .data(|d| d.get_temp::<ReviewTab>(egui::Id::new(("active_tab", &task.id))))
        .unwrap_or(ReviewTab::Description);

    let mut action_out = None;

    // Force Discussion tab if thread is active
    if ui_state.active_thread.is_some() {
        active_tab = ReviewTab::Discussion;
    }

    let note_count = domain_state
        .threads
        .iter()
        .filter(|thread| thread.task_id.as_ref() == Some(&task.id))
        .count();
    let discussion_label = if note_count > 0 {
        format!("Discussion ({})", note_count)
    } else {
        "Discussion".to_string()
    };

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = spacing::SPACING_MD;

        let mut tab_button = |ui: &mut egui::Ui, tab: ReviewTab, label: &str, icon: &str| {
            let is_selected = active_tab == tab;
            let text = format!("{} {}", icon, label);

            let mut text = typography::body(text).size(13.0);
            if is_selected {
                text = typography::bold(text.text()).size(13.0).color(theme.brand);
            } else {
                text = text.color(theme.text_muted);
            };

            let resp = ui.add(
                egui::Button::new(text)
                    .fill(if is_selected {
                        theme.brand.gamma_multiply(0.1)
                    } else {
                        egui::Color32::TRANSPARENT
                    })
                    .stroke(egui::Stroke::NONE)
                    .corner_radius(spacing::RADIUS_MD),
            );
            let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);

            if resp.clicked() {
                if ui_state.active_thread.is_some() {
                    action_out = Some(ReviewAction::CloseThread);
                }
                active_tab = tab;
                ui.ctx()
                    .data_mut(|d| d.insert_temp(egui::Id::new(("active_tab", &task.id)), tab));
            }
        };

        tab_button(
            ui,
            ReviewTab::Description,
            "Description",
            icons::TAB_DESCRIPTION,
        );
        if task.diagram.as_ref().is_some_and(|d| !d.is_empty()) {
            tab_button(ui, ReviewTab::Diagram, "Diagram", icons::TAB_DIAGRAM);
        }
        if !task.diff_refs.is_empty() {
            tab_button(ui, ReviewTab::Changes, "Changes", icons::TAB_CHANGES);
        }

        tab_button(
            ui,
            ReviewTab::Discussion,
            &discussion_label,
            icons::TAB_DISCUSSION,
        );
    });

    (active_tab, action_out)
}
