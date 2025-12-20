use crate::domain::{Note, NoteSeverity};
use crate::ui::app::{Action, LaReviewApp, ReviewAction};
use crate::ui::components::badge::badge;
use crate::ui::spacing;
use crate::ui::theme::current_theme;
use eframe::egui;
use egui_phosphor::regular as icons;

#[allow(dead_code)]
pub struct ThreadDetailView {
    pub file_path: String,
    pub line_number: u32,
    pub task_id: String,
}

impl LaReviewApp {
    #[allow(dead_code)]
    pub(crate) fn render_thread_detail(&mut self, ui: &mut egui::Ui, view: &ThreadDetailView) {
        let theme = current_theme();

        // 1. Breadcrumbs / Header
        ui.horizontal(|ui| {
            if ui
                .button(
                    egui::RichText::new(format!("{} Back to Discussion", icons::CARET_LEFT))
                        .size(14.0),
                )
                .clicked()
            {
                self.dispatch(Action::Review(ReviewAction::CloseThread));
            }
        });

        ui.add_space(spacing::SPACING_MD);

        // 2. Title and Severity
        // We need to find the root note or "thread metadata"
        // For now, we'll assume the first note or a dedicated thread object holds this.
        // Since we attached title/severity to Note, we look for the root note of this location.

        let notes: Vec<Note> = self
            .state
            .task_line_notes
            .iter()
            .filter(|n| {
                n.file_path.as_ref() == Some(&view.file_path)
                    && n.line_number == Some(view.line_number)
            })
            .cloned()
            .collect();

        let root_note = notes.first().cloned(); // Simplified finding root

        ui.horizontal(|ui| {
            // Edit Title
            let title = root_note
                .as_ref()
                .and_then(|n| n.title.clone())
                .unwrap_or_else(|| "".to_string());

            let id = ui.id().with("title_edit");
            let mut edit_text = ui
                .ctx()
                .data(|d| d.get_temp::<String>(id))
                .unwrap_or(title.clone());
            if edit_text.is_empty() && !title.is_empty() {
                edit_text = title.clone();
            }

            let response = ui.add(
                egui::TextEdit::singleline(&mut edit_text)
                    .hint_text("Discussion Title")
                    .desired_width(ui.available_width() - 150.0) // Leave space for badge
                    .font(egui::FontId::proportional(20.0))
                    .frame(false),
            );

            if response.changed() {
                ui.ctx().data_mut(|d| d.insert_temp(id, edit_text.clone()));
            }

            if response.lost_focus()
                && edit_text != title
                && let Some(note) = &root_note
            {
                self.dispatch(Action::Review(ReviewAction::UpdateNote {
                    note_id: note.id.clone(),
                    title: Some(edit_text.clone()),
                    severity: None,
                }));
            }

            ui.ctx().data_mut(|d| d.insert_temp(id, edit_text));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Severity Toggle
                let severity = root_note
                    .as_ref()
                    .and_then(|n| n.severity)
                    .unwrap_or(NoteSeverity::NonBlocking);

                let (text, color, bg) = match severity {
                    NoteSeverity::Blocking => (
                        "Blocking",
                        theme.destructive,
                        theme.destructive.gamma_multiply(0.1),
                    ),
                    NoteSeverity::NonBlocking => (
                        "Non-Blocking",
                        theme.success,
                        theme.success.gamma_multiply(0.1),
                    ),
                };

                // Make badge clickable
                let badge_resp = badge(ui, text, bg, color).interact(egui::Sense::click());
                if badge_resp.clicked()
                    && let Some(note) = &root_note
                {
                    let new_severity = match severity {
                        NoteSeverity::Blocking => NoteSeverity::NonBlocking,
                        NoteSeverity::NonBlocking => NoteSeverity::Blocking,
                    };
                    self.dispatch(Action::Review(ReviewAction::UpdateNote {
                        note_id: note.id.clone(),
                        title: None, // Preserve existing
                        severity: Some(new_severity),
                    }));
                }
                if badge_resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
            });
        });

        // Context
        ui.horizontal(|ui| {
            let display_path = view
                .file_path
                .split('/')
                .next_back()
                .unwrap_or(&view.file_path);
            ui.label(
                egui::RichText::new(format!("{}:{}", display_path, view.line_number))
                    .color(theme.text_accent) // Blueish
                    .underline(),
            );
        });

        ui.add_space(spacing::SPACING_LG);
        ui.separator();
        ui.add_space(spacing::SPACING_LG);

        // 3. Timeline
        egui::ScrollArea::vertical()
            .id_salt("thread_timeline")
            .max_height(ui.available_height() - 100.0) // Leave space for input
            .show(ui, |ui| {
                for note in &notes {
                    self.render_note_bubble(ui, note); // We'll need a bubble render 
                    ui.add_space(spacing::SPACING_MD);
                }
            });

        // 4. Input Area
        ui.add_space(spacing::SPACING_MD);
        ui.vertical(|ui| {
            let input_id = egui::Id::new("thread_reply_input");
            let mut text = ui
                .ctx()
                .data(|d| d.get_temp::<String>(input_id))
                .unwrap_or_default();

            ui.add(
                egui::TextEdit::multiline(&mut text)
                    .hint_text("Reply...")
                    .desired_rows(3)
                    .desired_width(f32::INFINITY),
            );

            ui.ctx().data_mut(|d| d.insert_temp(input_id, text.clone()));

            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(format!("{} Send Reply", icons::PAPER_PLANE_RIGHT))
                        .clicked()
                    {
                        // Dispatch save
                        if !text.trim().is_empty() {
                            self.dispatch(Action::Review(ReviewAction::SaveLineNote {
                                task_id: view.task_id.clone(),
                                file_path: view.file_path.clone(),
                                line_number: view.line_number,
                                body: text,
                            }));
                            ui.ctx()
                                .data_mut(|d| d.insert_temp(input_id, String::new()));
                        }
                    }
                });
            });
        });
    }

    fn render_note_bubble(&self, ui: &mut egui::Ui, note: &Note) {
        let theme = current_theme();

        ui.horizontal_top(|ui| {
            // Avatar (smaller, no background?)
            let initials = note
                .author
                .chars()
                .next()
                .unwrap_or('?')
                .to_uppercase()
                .to_string();

            let avatar_size = 28.0;
            ui.allocate_ui(egui::vec2(avatar_size, avatar_size), |ui| {
                let rect = ui.max_rect();
                ui.painter()
                    .circle_filled(rect.center(), avatar_size / 2.0, theme.bg_secondary);
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    initials,
                    egui::FontId::proportional(12.0),
                    theme.text_primary,
                );
            });

            ui.add_space(spacing::SPACING_SM);

            ui.vertical(|ui| {
                // Header line
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(&note.author)
                            .strong()
                            .size(13.0)
                            .color(theme.text_primary),
                    );
                    ui.label(
                        egui::RichText::new(format!("• {}", note.created_at))
                            .size(10.0)
                            .color(theme.text_muted),
                    );
                });

                // Body
                ui.label(
                    egui::RichText::new(&note.body)
                        .color(theme.text_secondary)
                        .line_height(Some(22.0)),
                );
            });
        });
    }
}
