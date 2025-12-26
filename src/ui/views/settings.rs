use crate::ui::app::{Action, LaReviewApp, SettingsAction};
use crate::ui::spacing::TOP_HEADER_HEIGHT;
use crate::ui::{icons, spacing, typography};
use eframe::egui;
use once_cell::sync::Lazy;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::ui::theme;

static LOGO_BYTES_CACHE: Lazy<Mutex<HashMap<String, Arc<[u8]>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn load_logo_bytes(path: &str) -> Option<Arc<[u8]>> {
    if let Ok(mut cache) = LOGO_BYTES_CACHE.lock() {
        if let Some(bytes) = cache.get(path) {
            return Some(bytes.clone());
        }

        let bytes: Arc<[u8]> = crate::assets::get_content(path)?.into();
        cache.insert(path.to_owned(), bytes.clone());
        Some(bytes)
    } else {
        crate::assets::get_content(path).map(Into::into)
    }
}

impl LaReviewApp {
    pub fn ui_settings(&mut self, ui: &mut egui::Ui) {
        if ui.available_width() < 100.0 {
            return;
        }

        let theme = theme::current_theme();

        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(spacing::SPACING_XL as i8, 0))
            .show(ui, |ui| {
                ui.set_min_height(TOP_HEADER_HEIGHT);
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), TOP_HEADER_HEIGHT),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        // A. Left Side: Context Selectors
                        ui.horizontal(|ui| ui.label(typography::h2("Settings")));
                    },
                );
            });

        ui.separator();

        // --- GitHub Section ---
        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(
                spacing::SPACING_XL as i8,
                spacing::SPACING_MD as i8,
            ))
            .show(ui, |ui| {
                // Title Block
                ui.horizontal(|ui| {
                    ui.label(typography::label("GitHub CLI Integration").color(theme.text_primary));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.state.session.is_gh_status_checking {
                            ui.label(typography::body("Checking...").color(theme.warning));
                        } else if self.state.session.gh_status.is_some()
                            && self.state.session.gh_status_error.is_none()
                        {
                            ui.label(
                                typography::label(format!("{} Ready", icons::ICON_CHECK))
                                    .color(theme.success),
                            );
                        } else {
                            ui.label(
                                typography::label(format!(
                                    "{} Error/Unknown",
                                    icons::STATUS_REJECTED
                                ))
                                .color(theme.destructive),
                            );
                        }
                    });
                });

                ui.add_space(spacing::SPACING_SM);

                // Content Block
                egui::Grid::new("gh_settings_grid")
                    .num_columns(2)
                    .spacing([spacing::SPACING_LG, spacing::SPACING_MD])
                    .show(ui, |ui| {
                        ui.label(typography::label("Connection:"));
                        ui.horizontal(|ui| {
                            if let Some(err) = &self.state.session.gh_status_error {
                                ui.label(
                                    typography::label("Disconnected").color(theme.destructive),
                                );
                                ui.label(typography::weak(format!("(Error: {})", err)));
                            } else if let Some(status) = &self.state.session.gh_status {
                                ui.label(typography::label("Connected").color(theme.success));
                                if let Some(login) = &status.login {
                                    ui.label(
                                        typography::bold(format!("(@{})", login))
                                            .color(theme.text_disabled),
                                    );
                                }
                            } else {
                                ui.label(typography::label("Unknown").color(theme.warning));
                            }
                        });
                        ui.end_row();

                        if let Some(status) = &self.state.session.gh_status {
                            ui.label(typography::label("Executable Path:"));
                            ui.label(typography::mono(&status.gh_path));
                            ui.end_row();
                        }
                    });

                ui.add_space(spacing::SPACING_MD);

                ui.horizontal(|ui| {
                    let btn_label = if self.state.session.is_gh_status_checking {
                        typography::label("Checking...")
                    } else {
                        typography::label("Refresh Status")
                    };
                    if ui
                        .add_enabled(
                            !self.state.session.is_gh_status_checking,
                            egui::Button::new(btn_label),
                        )
                        .clicked()
                    {
                        self.dispatch(Action::Settings(SettingsAction::CheckGitHubStatus));
                    }
                });

                // Troubleshooting
                if self.state.session.gh_status.is_none()
                    || self.state.session.gh_status_error.is_some()
                {
                    ui.add_space(spacing::SPACING_LG);
                    egui::CollapsingHeader::new(
                        typography::bold("Setup Instructions").color(theme.text_secondary),
                    )
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.add_space(spacing::SPACING_SM);
                        egui::Frame::NONE
                            .fill(theme.bg_secondary)
                            .inner_margin(spacing::SPACING_MD)
                            .corner_radius(crate::ui::spacing::RADIUS_MD)
                            .show(ui, |ui| {
                                ui.vertical(|ui| {
                                    self.ui_copyable_command(
                                        ui,
                                        "1. Install via Homebrew",
                                        "brew install gh",
                                    );
                                    ui.add_space(spacing::SPACING_MD);
                                    self.ui_copyable_command(
                                        ui,
                                        "2. Authenticate",
                                        "gh auth login",
                                    );
                                });
                            });
                    });
                }
            });

        ui.separator();

        // --- D2 Section ---
        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(
                spacing::SPACING_XL as i8,
                spacing::SPACING_MD as i8,
            ))
            .show(ui, |ui| {
                let d2_installed = crate::infra::brew::find_bin("d2").is_some();

                ui.horizontal(|ui| {
                    ui.label(typography::label("D2 Diagram Engine").color(theme.text_primary));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.state.ui.is_d2_installing {
                            ui.label(typography::body("Installing...").color(theme.warning));
                        } else if d2_installed {
                            ui.label(
                                typography::label(format!("{} Installed", icons::ICON_CHECK))
                                    .color(theme.success),
                            );
                        } else {
                            ui.label(typography::label("Not Installed").color(theme.text_disabled));
                        }
                    });
                });

                ui.add_space(spacing::SPACING_SM);

                let install_cmd = "curl -fsSL https://d2lang.com/install.sh | sh -s --";
                let uninstall_cmd =
                    "curl -fsSL https://d2lang.com/install.sh | sh -s -- --uninstall";

                if d2_installed {
                    ui.label(typography::body("D2 is ready to render diagrams."));
                    ui.add_space(spacing::SPACING_MD);

                    ui.collapsing(typography::label("Uninstall Options"), |ui| {
                        ui.add_space(spacing::SPACING_SM);
                        self.ui_copyable_command(ui, "Manual Uninstall", uninstall_cmd);

                        ui.add_space(spacing::SPACING_SM);
                        let btn = egui::Button::new(typography::label("Run Uninstall Script"))
                            .fill(theme.bg_card);
                        if ui.add_enabled(!self.state.ui.is_d2_installing, btn).clicked() {
                            self.dispatch(Action::Settings(SettingsAction::RequestD2Uninstall));
                        }
                    });
                } else {
                    egui::Frame::NONE
                        .fill(theme.bg_surface)
                        .inner_margin(spacing::SPACING_SM)
                        .stroke(egui::Stroke::new(1.0, theme.warning))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    typography::body(icons::ICON_WARNING)
                                        .color(theme.warning)
                                        .size(16.0),
                                );
                                ui.vertical(|ui| {
                                    ui.label(typography::bold("Remote Script Warning"));
                                    ui.label(typography::body("Installation requires running a remote shell script. You can run it manually or allow LaReview to run it."));
                                });
                            });
                        });

                    ui.add_space(spacing::SPACING_MD);

                    let mut allow = self.state.ui.allow_d2_install;
                    if ui
                        .checkbox(
                            &mut allow,
                            typography::label("I understand and want to proceed"),
                        )
                        .changed()
                    {
                        self.dispatch(Action::Settings(SettingsAction::SetAllowD2Install(allow)));
                    }

                    ui.add_space(12.0);

                    ui.horizontal(|ui| {
                        let can_install =
                            self.state.ui.allow_d2_install && !self.state.ui.is_d2_installing;
                        if ui
                            .add_enabled(
                                can_install,
                                egui::Button::new(typography::label("Install Automatically")),
                            )
                            .clicked()
                    {
                        self.dispatch(Action::Settings(SettingsAction::RequestD2Install));
                    }
                    });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    self.ui_copyable_command(ui, "Manual Install Command", install_cmd);
                }

                if self.state.ui.is_d2_installing {
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        crate::ui::animations::cyber::cyber_spinner(
                            ui,
                            theme.brand,
                            Some(crate::ui::animations::cyber::CyberSpinnerSize::Sm),
                        );
                        ui.label(typography::body("Processing..."));
                    });
                }

                if !self.state.ui.d2_install_output.is_empty() {
                    ui.add_space(12.0);
                    egui::CollapsingHeader::new(typography::bold("Script Output Log"))
                        .default_open(true)
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .max_height(150.0)
                                .show(ui, |ui| {
                                    ui.add(
                                        egui::TextEdit::multiline(
                                            &mut self.state.ui.d2_install_output.as_str(),
                                        )
                                        .font(typography::mono_font(13.0))
                                        .desired_width(f32::INFINITY)
                                        .lock_focus(true),
                                    );
                                });
                        });
                }
            });

        ui.separator();

        // --- Agents Section ---
        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(
                spacing::SPACING_XL as i8,
                spacing::SPACING_MD as i8,
            ))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(typography::label("ACP Agents").color(theme.text_primary));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(typography::label(format!(
                                "{} Add Custom Agent",
                                icons::ICON_PLUS
                            )))
                            .clicked()
                        {
                            self.dispatch(Action::Settings(SettingsAction::OpenAddCustomAgent));
                        }
                    });
                });

                ui.add_space(spacing::SPACING_MD);

                let candidates = crate::infra::acp::list_agent_candidates();

                ui.vertical(|ui| {
                    let card_width = 240.0;
                    let spacing_x = spacing::SPACING_MD;
                    let available_width = ui.available_width();
                    let columns = (available_width / (card_width + spacing_x))
                        .floor()
                        .max(1.0) as usize;

                    egui::Grid::new("agents_grid_cards")
                        .spacing([spacing_x, spacing::SPACING_MD])
                        .min_col_width(card_width)
                        .show(ui, |ui| {
                            for (i, candidate) in candidates.iter().enumerate() {
                                self.ui_agent_card(ui, candidate);
                                if (i + 1) % columns == 0 {
                                    ui.end_row();
                                }
                            }
                        });
                });
            });

        // --- Modals ---
        self.ui_agent_settings_modal(ui.ctx());
        self.ui_add_custom_agent_modal(ui.ctx());
    }

    fn ui_agent_card(&mut self, ui: &mut egui::Ui, candidate: &crate::infra::acp::AgentCandidate) {
        let theme = theme::current_theme();

        let card_width = 240.0;
        let card_height = 120.0;

        egui::Frame::NONE
            .fill(theme.bg_card)
            .stroke(egui::Stroke::new(1.0, theme.border))
            .inner_margin(spacing::SPACING_MD)
            .corner_radius(spacing::RADIUS_MD)
            .show(ui, |ui| {
                ui.set_min_size(egui::vec2(card_width, card_height));
                ui.set_max_size(egui::vec2(card_width, card_height));

                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        // Logo
                        if let Some(logo_path) = &candidate.logo
                            && let Some(bytes) = load_logo_bytes(logo_path)
                        {
                            let uri = format!("bytes://{}", logo_path);
                            let image = egui::Image::from_bytes(uri, bytes)
                                .fit_to_exact_size(egui::vec2(16.0, 16.0))
                                .corner_radius(2.0);
                            ui.add(image);
                        } else {
                            // Placeholder icon
                            ui.label(typography::body(icons::ICON_EMPTY).size(20.0));
                        }

                        ui.add_space(spacing::SPACING_XS);

                        ui.vertical(|ui| {
                            ui.label(typography::bold(&candidate.label));
                            ui.label(typography::weak(&candidate.id).size(11.0));
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                            if candidate.available {
                                ui.label(
                                    typography::label(icons::STATUS_DONE).color(theme.success),
                                );
                            } else {
                                ui.label(
                                    typography::label(icons::STATUS_REJECTED)
                                        .color(theme.destructive),
                                );
                            }
                        });
                    });

                    ui.add_space(spacing::SPACING_SM);

                    // Path/Command info
                    let cmd = candidate.command.as_deref().unwrap_or("Not found");
                    ui.add(egui::Label::new(typography::weak(cmd).size(10.0)).truncate());

                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                        ui.add_space(spacing::SPACING_XS);
                        if ui
                            .add_sized(
                                [ui.available_width(), 28.0],
                                egui::Button::new(typography::label(format!(
                                    "{} Settings",
                                    icons::VIEW_SETTINGS
                                ))),
                            )
                            .clicked()
                        {
                            self.dispatch(Action::Settings(SettingsAction::OpenAgentSettings(
                                candidate.id.clone(),
                            )));
                        }
                    });
                });
            });
    }

    fn ui_agent_settings_modal(&mut self, ctx: &egui::Context) {
        let Some(agent_id) = self.state.ui.editing_agent_id.clone() else {
            return;
        };

        let theme = theme::current_theme();
        let current_path = self
            .state
            .ui
            .agent_path_overrides
            .get(&agent_id)
            .and_then(|path| {
                if path.is_empty() {
                    None
                } else {
                    Some(path.clone())
                }
            });
        let current_envs = self.state.ui.agent_envs.get(&agent_id).and_then(|envs| {
            if envs.is_empty() {
                None
            } else {
                Some(envs.clone())
            }
        });
        let is_agent_dirty = match self.state.ui.agent_settings_snapshot.as_ref() {
            Some(snapshot) => {
                snapshot.agent_id == agent_id
                    && (snapshot.path_override != current_path || snapshot.envs != current_envs)
            }
            None => false,
        };
        let mut open = true;
        egui::Window::new(typography::bold(format!("Agent Settings: {}", agent_id)))
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.set_width(500.0);

                egui::Frame::NONE
                    .inner_margin(spacing::SPACING_LG)
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.label(typography::bold("Executable Path Override"));
                            ui.add_space(spacing::SPACING_XS);

                            let mut path = self
                                .state
                                .ui
                                .agent_path_overrides
                                .get(&agent_id)
                                .cloned()
                                .unwrap_or_default();
                            if ui
                                .add(
                                    egui::TextEdit::singleline(&mut path)
                                        .desired_width(f32::INFINITY),
                                )
                                .changed()
                            {
                                self.dispatch(Action::Settings(SettingsAction::UpdateAgentPath(
                                    agent_id.clone(),
                                    path,
                                )));
                            }
                            ui.label(typography::weak(
                                "Leave empty to use default auto-discovery.",
                            ));

                            ui.add_space(spacing::SPACING_XL);
                            ui.separator();
                            ui.add_space(spacing::SPACING_LG);

                            ui.label(typography::bold("Environment Variables"));
                            ui.add_space(spacing::SPACING_MD);

                            let key_width = 140.0;
                            let val_width = 200.0;
                            let btn_width = 90.0;
                            let row_height = 24.0;
                            let cell_stroke = egui::Stroke::new(1.0, theme.border);
                            let cell_margin = egui::Margin::symmetric(6, 2);

                            egui::ScrollArea::vertical()
                                .max_height(220.0)
                                .auto_shrink([false, true])
                                .show(ui, |ui| {
                                    ui.spacing_mut().item_spacing.y = spacing::SPACING_MD;
                                    let mut to_remove = None;

                                    egui::Grid::new("agent_env_grid")
                                        .spacing(egui::vec2(0.0, 0.0))
                                        .show(ui, |ui| {
                                            egui::Frame::NONE
                                                .stroke(cell_stroke)
                                                .inner_margin(cell_margin)
                                                .show(ui, |ui| {
                                                    ui.set_min_size(egui::vec2(
                                                        key_width, row_height,
                                                    ));
                                                    ui.with_layout(
                                                        egui::Layout::left_to_right(
                                                            egui::Align::Center,
                                                        ),
                                                        |ui| {
                                                            ui.label(typography::weak(
                                                                "Variable Name",
                                                            ));
                                                        },
                                                    );
                                                });
                                            egui::Frame::NONE
                                                .stroke(cell_stroke)
                                                .inner_margin(cell_margin)
                                                .show(ui, |ui| {
                                                    ui.set_min_size(egui::vec2(
                                                        val_width, row_height,
                                                    ));
                                                    ui.with_layout(
                                                        egui::Layout::left_to_right(
                                                            egui::Align::Center,
                                                        ),
                                                        |ui| {
                                                            ui.label(typography::weak("Value"));
                                                        },
                                                    );
                                                });
                                            egui::Frame::NONE
                                                .stroke(cell_stroke)
                                                .inner_margin(cell_margin)
                                                .show(ui, |ui| {
                                                    ui.set_min_size(egui::vec2(
                                                        btn_width, row_height,
                                                    ));
                                                });
                                            ui.end_row();

                                            if let Some(envs) =
                                                self.state.ui.agent_envs.get(&agent_id)
                                            {
                                                for (key, value) in envs {
                                                    egui::Frame::NONE
                                                        .stroke(cell_stroke)
                                                        .inner_margin(cell_margin)
                                                        .show(ui, |ui| {
                                                            ui.set_min_size(egui::vec2(
                                                                key_width, row_height,
                                                            ));
                                                            ui.with_layout(
                                                                egui::Layout::left_to_right(
                                                                    egui::Align::Center,
                                                                ),
                                                                |ui| {
                                                                    ui.add(
                                                                        egui::Label::new(
                                                                            typography::mono(key),
                                                                        )
                                                                        .truncate(),
                                                                    );
                                                                },
                                                            );
                                                        });
                                                    let display_val = if value.len() > 10 {
                                                        "*******"
                                                    } else {
                                                        value
                                                    };
                                                    egui::Frame::NONE
                                                        .stroke(cell_stroke)
                                                        .inner_margin(cell_margin)
                                                        .show(ui, |ui| {
                                                            ui.set_min_size(egui::vec2(
                                                                val_width, row_height,
                                                            ));
                                                            ui.with_layout(
                                                                egui::Layout::left_to_right(
                                                                    egui::Align::Center,
                                                                ),
                                                                |ui| {
                                                                    ui.label(typography::label(
                                                                        display_val,
                                                                    ));
                                                                },
                                                            );
                                                        });

                                                    egui::Frame::NONE
                                                        .stroke(cell_stroke)
                                                        .inner_margin(cell_margin)
                                                        .show(ui, |ui| {
                                                            ui.set_min_size(egui::vec2(
                                                                btn_width, row_height,
                                                            ));
                                                            ui.with_layout(
                                                                egui::Layout::left_to_right(
                                                                    egui::Align::Center,
                                                                ),
                                                                |ui| {
                                                                    if ui
                                                                        .button(typography::label(
                                                                            icons::ACTION_TRASH,
                                                                        ))
                                                                        .clicked()
                                                                    {
                                                                        to_remove =
                                                                            Some(key.clone());
                                                                    }
                                                                },
                                                            );
                                                        });
                                                    ui.end_row();
                                                }
                                            }

                                            egui::Frame::NONE
                                                .stroke(cell_stroke)
                                                .inner_margin(cell_margin)
                                                .show(ui, |ui| {
                                                    ui.set_min_size(egui::vec2(
                                                        key_width, row_height,
                                                    ));
                                                    ui.add(
                                                        egui::TextEdit::singleline(
                                                            &mut self.state.ui.agent_env_draft_key,
                                                        )
                                                        .hint_text(typography::weak("KEY"))
                                                        .desired_width(key_width),
                                                    );
                                                });
                                            egui::Frame::NONE
                                                .stroke(cell_stroke)
                                                .inner_margin(cell_margin)
                                                .show(ui, |ui| {
                                                    ui.set_min_size(egui::vec2(
                                                        val_width, row_height,
                                                    ));
                                                    ui.add(
                                                        egui::TextEdit::singleline(
                                                            &mut self
                                                                .state
                                                                .ui
                                                                .agent_env_draft_value,
                                                        )
                                                        .hint_text(typography::weak("VALUE"))
                                                        .password(true)
                                                        .desired_width(val_width),
                                                    );
                                                });
                                            egui::Frame::NONE
                                                .stroke(cell_stroke)
                                                .inner_margin(cell_margin)
                                                .show(ui, |ui| {
                                                    ui.set_min_size(egui::vec2(
                                                        btn_width, row_height,
                                                    ));
                                                    ui.with_layout(
                                                        egui::Layout::left_to_right(
                                                            egui::Align::Center,
                                                        ),
                                                        |ui| {
                                                            if ui
                                                                .button(typography::label(
                                                                    "Add more",
                                                                ))
                                                                .clicked()
                                                                && !self
                                                                    .state
                                                                    .ui
                                                                    .agent_env_draft_key
                                                                    .is_empty()
                                                            {
                                                                self.dispatch(Action::Settings(
                                                                    SettingsAction::UpdateAgentEnv(
                                                                        agent_id.clone(),
                                                                        self.state
                                                                            .ui
                                                                            .agent_env_draft_key
                                                                            .clone(),
                                                                        self.state
                                                                            .ui
                                                                            .agent_env_draft_value
                                                                            .clone(),
                                                                    ),
                                                                ));
                                                                self.state
                                                                    .ui
                                                                    .agent_env_draft_key
                                                                    .clear();
                                                                self.state
                                                                    .ui
                                                                    .agent_env_draft_value
                                                                    .clear();
                                                            }
                                                        },
                                                    );
                                                });
                                            ui.end_row();
                                        });

                                    if let Some(key) = to_remove {
                                        self.dispatch(Action::Settings(
                                            SettingsAction::RemoveAgentEnv(agent_id.clone(), key),
                                        ));
                                    }
                                });

                            ui.add_space(spacing::SPACING_XL);
                            ui.horizontal(|ui| {
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui.button(typography::label("Done")).clicked() {
                                            self.dispatch(Action::Settings(
                                                SettingsAction::CloseAgentSettings,
                                            ));
                                        }
                                        if is_agent_dirty
                                            && ui
                                                .button(
                                                    typography::bold(format!(
                                                        "{} Save Changes",
                                                        icons::ACTION_SAVE
                                                    ))
                                                    .color(theme.brand),
                                                )
                                                .clicked()
                                        {
                                            self.dispatch(Action::Settings(
                                                SettingsAction::SaveAgentSettings,
                                            ));
                                        }
                                    },
                                );
                            });
                        });
                    });
            });

        if !open {
            self.dispatch(Action::Settings(SettingsAction::CloseAgentSettings));
        }
    }

    fn ui_add_custom_agent_modal(&mut self, ctx: &egui::Context) {
        if !self.state.ui.show_add_custom_agent_modal {
            return;
        }

        let mut open = true;
        egui::Window::new(typography::bold("Add Custom ACP Agent"))
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.set_width(450.0);

                egui::Frame::NONE
                    .inner_margin(spacing::SPACING_LG)
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            egui::Grid::new("add_custom_agent_grid")
                                .num_columns(2)
                                .spacing([spacing::SPACING_LG, spacing::SPACING_LG])
                                .show(ui, |ui| {
                                    ui.label(typography::bold("Unique ID:"));
                                    ui.text_edit_singleline(
                                        &mut self.state.ui.custom_agent_draft.id,
                                    );
                                    ui.end_row();

                                    ui.label(typography::bold("Display Label:"));
                                    ui.text_edit_singleline(
                                        &mut self.state.ui.custom_agent_draft.label,
                                    );
                                    ui.end_row();

                                    ui.label(typography::bold("Command/Binary:"));
                                    ui.text_edit_singleline(
                                        &mut self.state.ui.custom_agent_draft.command,
                                    );
                                    ui.end_row();
                                });

                            ui.add_space(spacing::SPACING_XL);
                            ui.horizontal(|ui| {
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui.button(typography::label("Add Agent")).clicked()
                                            && !self.state.ui.custom_agent_draft.id.is_empty()
                                        {
                                            self.dispatch(Action::Settings(
                                                SettingsAction::AddCustomAgent(
                                                    self.state.ui.custom_agent_draft.clone(),
                                                ),
                                            ));
                                            self.dispatch(Action::Settings(
                                                SettingsAction::CloseAddCustomAgent,
                                            ));
                                        }

                                        ui.add_space(spacing::SPACING_MD);

                                        if ui.button(typography::label("Cancel")).clicked() {
                                            self.dispatch(Action::Settings(
                                                SettingsAction::CloseAddCustomAgent,
                                            ));
                                        }
                                    },
                                );
                            });
                        });
                    });
            });

        if !open {
            self.dispatch(Action::Settings(SettingsAction::CloseAddCustomAgent));
        }
    }

    /// Helper UI component for commands
    fn ui_copyable_command(&self, ui: &mut egui::Ui, label: &str, cmd: &str) {
        let theme = theme::current_theme();
        ui.label(typography::label(label));
        ui.horizontal(|ui| {
            // Command text in a box
            egui::Frame::NONE
                .fill(theme.bg_surface)
                .inner_margin(spacing::SPACING_SM) // Using SPACING_SM (8.0) as closest to 6.0
                .corner_radius(4.0)
                .show(ui, |ui| {
                    ui.label(typography::mono(cmd));
                });

            // Copy button
            if ui
                .button(typography::label(format!("{} Copy", icons::ACTION_COPY)))
                .clicked()
            {
                ui.ctx().copy_text(cmd.to_string());
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    #[test]
    fn test_ui_copyable_command() {
        let app = LaReviewApp::new_for_test();
        let mut harness = Harness::new_ui(|ui| {
            app.ui_copyable_command(ui, "Label", "ls -la");
        });
        harness.run();
        harness.get_by_label("Label");
        harness.get_by_label("ls -la");
        harness
            .get_by_label(&format!("{} Copy", icons::ACTION_COPY))
            .click();
    }

    #[test]
    fn test_ui_settings_rendering() {
        let mut app = LaReviewApp::new_for_test();
        let mut harness = Harness::new(|ctx| {
            crate::ui::app::LaReviewApp::setup_fonts(ctx);
            egui::CentralPanel::default().show(ctx, |ui| {
                app.ui_settings(ui);
            });
        });
        harness.run();
        harness.get_by_label("Settings");
        harness.get_by_label("GitHub CLI Integration");
        harness.get_by_label("D2 Diagram Engine");
    }

    #[test]
    fn test_ui_settings_gh_troubleshoot() {
        let mut app = LaReviewApp::new_for_test();
        app.state.session.gh_status = None;
        app.state.session.gh_status_error = Some("Mock Error".into());

        let mut harness = Harness::new(|ctx| {
            crate::ui::app::LaReviewApp::setup_fonts(ctx);
            egui::CentralPanel::default().show(ctx, |ui| {
                app.ui_settings(ui);
            });
        });
        harness.run();
        harness.get_by_label("Setup Instructions");
        harness.get_by_label("brew install gh");
    }
}
