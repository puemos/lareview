use crate::ui::app::{Action, LaReviewApp, SettingsAction};
use crate::ui::theme::current_theme;
use crate::ui::{spacing, typography};
use eframe::egui;

pub fn render_requirements_overlay(ctx: &egui::Context, app: &mut LaReviewApp) {
    let theme = current_theme();
    let mut open = true;

    let gh_path = crate::infra::shell::find_bin("gh");
    let d2_path = crate::infra::shell::find_bin("d2");
    let agents = crate::infra::acp::list_agent_candidates();

    egui::Window::new("Setup Checklist")
        .id(egui::Id::new("setup_checklist_overlay"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .frame(
            egui::Frame::window(&ctx.style())
                .inner_margin(egui::Margin::same(spacing::SPACING_MD as i8)),
        )
        .show(ctx, |ui| {
            ui.label("Ensure these tools are installed and discoverable:");
            ui.add_space(spacing::SPACING_SM);

            egui::Grid::new(ui.make_persistent_id("requirements_grid"))
                .num_columns(3)
                .spacing([spacing::SPACING_LG, spacing::SPACING_SM])
                .show(ui, |ui| {
                    ui.label(typography::bold("Tool"));
                    ui.label(typography::bold("Status"));
                    ui.label(typography::bold("Path"));
                    ui.end_row();

                    render_requirement_row(ui, "GitHub CLI (gh)", &gh_path, &theme);
                    render_requirement_row(ui, "D2", &d2_path, &theme);

                    for agent in &agents {
                        let label = format!("Agent: {}", agent.label);
                        let path = agent.command.as_ref().map(std::path::PathBuf::from);
                        render_requirement_row(ui, &label, &path, &theme);
                    }
                });

            ui.add_space(spacing::SPACING_MD);
            ui.horizontal(|ui| {
                if ui.button("Open Settings").clicked() {
                    app.switch_to_settings();
                    app.dispatch(Action::Settings(SettingsAction::DismissRequirements));
                }
                if ui.button("Dismiss").clicked() {
                    app.dispatch(Action::Settings(SettingsAction::DismissRequirements));
                }
            });
        });

    if !open {
        app.dispatch(Action::Settings(SettingsAction::DismissRequirements));
    }
}

pub fn render_editor_picker_overlay(ctx: &egui::Context, app: &mut LaReviewApp) {
    let theme = current_theme();
    let mut open = true;
    let editors = crate::infra::editor::list_available_editors();

    egui::Window::new("Choose Editor")
        .id(egui::Id::new("choose_editor_overlay"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .frame(
            egui::Frame::window(&ctx.style())
                .inner_margin(egui::Margin::same(spacing::SPACING_MD as i8)),
        )
        .show(ctx, |ui| {
            ui.label("Select a text editor to open files:");
            ui.add_space(spacing::SPACING_SM);

            if let Some(err) = &app.state.ui.editor_picker_error {
                ui.label(typography::body(err).color(theme.destructive));
                ui.add_space(spacing::SPACING_SM);
            }

            if editors.is_empty() {
                ui.label(typography::body("No supported editors found on PATH."));
                ui.label(typography::weak(
                    "Install one (VS Code, Cursor, Sublime, JetBrains) or add it to PATH.",
                ));
            } else {
                for editor in editors {
                    let label = format!("{} ({})", editor.label, editor.path.display());
                    if ui
                        .add_sized(
                            [ui.available_width(), 28.0],
                            egui::Button::new(typography::label(label)),
                        )
                        .clicked()
                    {
                        app.dispatch(Action::Settings(SettingsAction::SetPreferredEditor(
                            editor.id.to_string(),
                        )));
                    }
                }
            }

            ui.add_space(spacing::SPACING_MD);
            if ui.button("Cancel").clicked() {
                app.dispatch(Action::Settings(SettingsAction::ClearPreferredEditor));
            }
        });

    if !open {
        app.dispatch(Action::Settings(SettingsAction::ClearPreferredEditor));
    }
}

fn render_requirement_row(
    ui: &mut egui::Ui,
    label: &str,
    path: &Option<std::path::PathBuf>,
    theme: &crate::ui::theme::Theme,
) {
    ui.label(label);
    if let Some(p) = path {
        ui.label(egui::RichText::new("✓").color(theme.brand));
        ui.label(typography::weak(p.to_string_lossy()));
    } else {
        ui.label(egui::RichText::new("✗").color(theme.destructive));
        ui.label(typography::weak("Not found"));
    }
    ui.end_row();
}
