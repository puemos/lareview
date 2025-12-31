use crate::ui::app::{Action, LaReviewApp, SettingsAction};
use crate::ui::{icons, spacing, typography};
use crate::ui::theme;
use eframe::egui;
use once_cell::sync::Lazy;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

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
    pub fn ui_settings_agents(&mut self, ui: &mut egui::Ui) {
        let theme = theme::current_theme();
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
                            crate::ui::app::ui_memory::with_ui_memory_mut(ui.ctx(), |mem| {
                                mem.settings.show_add_custom_agent_modal = true;
                                mem.settings.custom_agent_draft = Default::default();
                            });
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
                            let is_custom = self
                                .state
                                .ui
                                .custom_agents
                                .iter()
                                .any(|a| a.id == candidate.id);
                            if is_custom
                                && ui
                                    .button(
                                        typography::label(icons::ACTION_DELETE)
                                            .color(theme.destructive),
                                    )
                                    .on_hover_text("Delete custom agent")
                                    .clicked()
                            {
                                self.dispatch(Action::Settings(SettingsAction::DeleteCustomAgent(
                                    candidate.id.clone(),
                                )));
                            }

                            if candidate.available {
                                ui.label(
                                    typography::label(icons::STATUS_DONE).color(theme.success),
                                );
                            } else {
                                ui.label(
                                    typography::label(icons::STATUS_IGNORED)
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
                            crate::ui::app::ui_memory::with_ui_memory_mut(ui.ctx(), |mem| {
                                mem.settings.editing_agent_id = Some(candidate.id.clone());
                                // Take snapshot for dirty check
                                let path_override = self
                                    .state
                                    .ui
                                    .agent_path_overrides
                                    .get(&candidate.id)
                                    .cloned();
                                let envs = self.state.ui.agent_envs.get(&candidate.id).cloned();
                                mem.settings.agent_settings_snapshot =
                                    Some(crate::ui::app::state::AgentSettingsSnapshot {
                                        agent_id: candidate.id.clone(),
                                        path_override,
                                        envs,
                                    });
                            });
                        }
                    });
                });
            });
    }

    fn ui_agent_settings_modal(&mut self, ctx: &egui::Context) {
        let ui_memory = crate::ui::app::ui_memory::get_ui_memory(ctx);
        let Some(agent_id) = ui_memory.settings.editing_agent_id.clone() else {
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
        let is_agent_dirty = match ui_memory.settings.agent_settings_snapshot.as_ref() {
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
                                                    let mut key = crate::ui::app::ui_memory::get_ui_memory(ui.ctx()).settings.agent_env_draft_key;
                                                    if ui
                                                        .add(
                                                            egui::TextEdit::singleline(
                                                                &mut key,
                                                            )
                                                            .hint_text(typography::weak("KEY"))
                                                            .desired_width(key_width),
                                                        )
                                                        .changed()
                                                    {
                                                        crate::ui::app::ui_memory::with_ui_memory_mut(ui.ctx(), |mem| {
                                                            mem.settings.agent_env_draft_key = key;
                                                        });
                                                    }
                                                });
                                            egui::Frame::NONE
                                                .stroke(cell_stroke)
                                                .inner_margin(cell_margin)
                                                .show(ui, |ui| {
                                                    ui.set_min_size(egui::vec2(
                                                        val_width, row_height,
                                                    ));
                                                    let mut val = crate::ui::app::ui_memory::get_ui_memory(ui.ctx()).settings.agent_env_draft_value;
                                                    if ui
                                                        .add(
                                                            egui::TextEdit::singleline(
                                                                &mut val,
                                                            )
                                                            .hint_text(typography::weak("VALUE"))
                                                            .desired_width(val_width),
                                                        )
                                                        .changed()
                                                    {
                                                        crate::ui::app::ui_memory::with_ui_memory_mut(ui.ctx(), |mem| {
                                                            mem.settings.agent_env_draft_value = val;
                                                        });
                                                    }
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
                                                            let key = crate::ui::app::ui_memory::get_ui_memory(ui.ctx()).settings.agent_env_draft_key;
                                                            let val = crate::ui::app::ui_memory::get_ui_memory(ui.ctx()).settings.agent_env_draft_value;
                                                            if ui
                                                                .add_enabled(
                                                                    !key.is_empty()
                                                                        && !val.is_empty(),
                                                                    egui::Button::new(
                                                                        typography::label(
                                                                            icons::ICON_PLUS,
                                                                        ),
                                                                    ),
                                                                )
                                                                .clicked()
                                                            {
                                                                self.dispatch(Action::Settings(
                                                                    SettingsAction::UpdateAgentEnv(
                                                                        agent_id.clone(),
                                                                        key,
                                                                        val,
                                                                    ),
                                                                ));
                                                                crate::ui::app::ui_memory::with_ui_memory_mut(ui.ctx(), |mem| {
                                                                    mem.settings.agent_env_draft_key = String::new();
                                                                    mem.settings.agent_env_draft_value = String::new();
                                                                });
                                                            }
                                                        },
                                                    );
                                                });
                                            ui.end_row();
                                        });

                                    if let Some(key) = to_remove {
                                        self.dispatch(Action::Settings(
                                            SettingsAction::RemoveAgentEnv(
                                                agent_id.clone(),
                                                key,
                                            ),
                                        ));
                                    }
                                });

                            ui.add_space(spacing::SPACING_XL);

                            ui.horizontal(|ui| {
                                if ui
                                    .add_enabled(
                                        is_agent_dirty,
                                        egui::Button::new(
                                            typography::label("Save Changes").color(theme.brand),
                                        ),
                                    )
                                    .clicked()
                                {
                                    self.dispatch(Action::Settings(
                                        SettingsAction::SaveAgentSettings,
                                    ));
                                    crate::ui::app::ui_memory::with_ui_memory_mut(
                                        ctx,
                                        |mem| {
                                            mem.settings.agent_settings_snapshot = Some(
                                                crate::ui::app::state::AgentSettingsSnapshot {
                                                    agent_id: agent_id.clone(),
                                                    path_override: current_path,
                                                    envs: current_envs,
                                                },
                                            );
                                        },
                                    );
                                }

                                if ui.button(typography::label("Close")).clicked() {
                                    crate::ui::app::ui_memory::with_ui_memory_mut(
                                        ctx,
                                        |mem| {
                                            mem.settings.editing_agent_id = None;
                                            mem.settings.agent_settings_snapshot = None;
                                        },
                                    );
                                }
                            });
                        });
                    });
            });

        if !open {
            crate::ui::app::ui_memory::with_ui_memory_mut(ctx, |mem| {
                mem.settings.editing_agent_id = None;
                mem.settings.agent_settings_snapshot = None;
            });
        }
    }

    fn ui_add_custom_agent_modal(&mut self, ctx: &egui::Context) {
        let show = crate::ui::app::ui_memory::get_ui_memory(ctx)
            .settings
            .show_add_custom_agent_modal;
        if !show {
            return;
        }

        let mut open = true;
        egui::Window::new(typography::bold("Add Custom Agent"))
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.set_width(400.0);

                egui::Frame::NONE
                    .inner_margin(spacing::SPACING_LG)
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.label(typography::label("Agent ID (unique)"));
                            let mut id = crate::ui::app::ui_memory::get_ui_memory(ui.ctx()).settings.custom_agent_draft.id.clone();
                            if ui
                                .add(
                                    egui::TextEdit::singleline(&mut id)
                                        .desired_width(f32::INFINITY),
                                )
                                .changed()
                            {
                                crate::ui::app::ui_memory::with_ui_memory_mut(ui.ctx(), |mem| {
                                    mem.settings.custom_agent_draft.id = id;
                                });
                            }

                            ui.add_space(spacing::SPACING_MD);

                            ui.label(typography::label("Display Name"));
                            let mut name = crate::ui::app::ui_memory::get_ui_memory(ui.ctx()).settings.custom_agent_draft.label.clone();
                            if ui
                                .add(
                                    egui::TextEdit::singleline(&mut name)
                                        .desired_width(f32::INFINITY),
                                )
                                .changed()
                            {
                                crate::ui::app::ui_memory::with_ui_memory_mut(ui.ctx(), |mem| {
                                    mem.settings.custom_agent_draft.label = name;
                                });
                            }

                            ui.add_space(spacing::SPACING_MD);

                            ui.label(typography::label("Execution Command"));
                            let mut cmd = crate::ui::app::ui_memory::get_ui_memory(ui.ctx()).settings.custom_agent_draft.command.clone();
                            if ui
                                .add(
                                    egui::TextEdit::singleline(&mut cmd)
                                        .desired_width(f32::INFINITY),
                                )
                                .changed()
                            {
                                crate::ui::app::ui_memory::with_ui_memory_mut(ui.ctx(), |mem| {
                                    mem.settings.custom_agent_draft.command = cmd;
                                });
                            }
                            ui.label(typography::weak("e.g. /usr/local/bin/my-agent or my-agent"));

                            ui.add_space(spacing::SPACING_XL);

                            ui.horizontal(|ui| {
                                let draft = crate::ui::app::ui_memory::get_ui_memory(ui.ctx()).settings.custom_agent_draft.clone();
                                let is_valid = !draft.id.is_empty()
                                    && !draft.label.is_empty()
                                    && !draft.command.is_empty();

                                if ui
                                    .add_enabled(is_valid, egui::Button::new(typography::label("Add Agent")))
                                    .clicked()
                                {
                                    self.dispatch(Action::Settings(SettingsAction::AddCustomAgent(
                                        draft,
                                    )));
                                    crate::ui::app::ui_memory::with_ui_memory_mut(ctx, |mem| {
                                        mem.settings.show_add_custom_agent_modal = false;
                                    });
                                }

                                if ui.button(typography::label("Cancel")).clicked() {
                                    crate::ui::app::ui_memory::with_ui_memory_mut(ctx, |mem| {
                                        mem.settings.show_add_custom_agent_modal = false;
                                    });
                                }
                            });
                        });
                    });
            });

        if !open {
            crate::ui::app::ui_memory::with_ui_memory_mut(ctx, |mem| {
                mem.settings.show_add_custom_agent_modal = false;
            });
        }
    }
}
