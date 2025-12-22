use crate::ui::app::{GenerateAction, TimelineItem};
use crate::ui::spacing;
use crate::ui::theme::Theme;
use eframe::egui;
use egui_phosphor::regular as icons;

pub(crate) fn render_timeline_pane(
    ui: &mut egui::Ui,
    agent_timeline: &[TimelineItem],
    theme: &Theme,
) -> Option<GenerateAction> {
    let mut action_out = None;

    ui.vertical(|ui| {
        // 1. ACTIVITY Section Header
        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(spacing::SPACING_SM as i8, 0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(spacing::SPACING_SM);
                    ui.label(
                        egui::RichText::new("ACTIVITY")
                            .size(11.0)
                            .color(theme.text_muted),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(spacing::SPACING_SM);
                        if !agent_timeline.is_empty()
                            && ui
                                .small_button(
                                    egui::RichText::new(format!("{} Clear", icons::TRASH))
                                        .color(theme.text_muted),
                                )
                                .clicked()
                        {
                            action_out = Some(GenerateAction::ClearTimeline);
                        }
                    });
                });
            });

        ui.separator();

        // 2. ACTIVITY Timeline (Scrollable Area)
        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(spacing::SPACING_SM as i8, 0))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("agent_activity_scroll")
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        ui.add_space(spacing::SPACING_XS);
                        for item in agent_timeline {
                            crate::ui::views::generate::timeline::render_timeline_item(ui, item);
                        }
                        ui.add_space(spacing::SPACING_XS);
                    });
            });
    });

    action_out
}
