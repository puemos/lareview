use crate::ui::app::LaReviewApp;
use catppuccin_egui::MOCHA;
use eframe::egui;

impl LaReviewApp {
    pub fn ui_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("Settings");

        ui.add_space(16.0);

        egui::Frame::group(ui.style())
            .fill(MOCHA.surface0)
            .show(ui, |ui| {
                ui.label("D2 Installation");
                ui.add_space(8.0);

                let d2_installed = which::which("d2").is_ok();

                if d2_installed {
                    if ui.button("Uninstall D2").clicked() {
                        self.state.is_d2_installing = true;
                        self.state.d2_install_output.clear();

                        let d2_install_tx = self.d2_install_tx.clone();
                        let command_str =
                            "curl -fsSL https://d2lang.com/install.sh | sh -s -- --uninstall"
                                .to_string();

                        crate::RUNTIME.get().unwrap().spawn(async move {
                            let mut child = tokio::process::Command::new("sh")
                                .arg("-c")
                                .arg(command_str)
                                .stdout(std::process::Stdio::piped())
                                .stderr(std::process::Stdio::piped())
                                .spawn()
                                .expect("Failed to spawn D2 uninstallation process");

                            let stdout = child.stdout.take().unwrap();
                            let stderr = child.stderr.take().unwrap();

                            use tokio::io::AsyncBufReadExt;
                            let mut stdout_reader = tokio::io::BufReader::new(stdout).lines();
                            let mut stderr_reader = tokio::io::BufReader::new(stderr).lines();

                            loop {
                                tokio::select! {
                                    line = stdout_reader.next_line() => {
                                        if let Ok(Some(line)) = line {
                                            let _ = d2_install_tx.send(line).await;
                                        } else {
                                            break;
                                        }
                                    },
                                    line = stderr_reader.next_line() => {
                                        if let Ok(Some(line)) = line {
                                            let _ = d2_install_tx.send(line).await;
                                        } else {
                                            break;
                                        }
                                    }
                                }
                            }
                            let _ = d2_install_tx
                                .send("___INSTALL_COMPLETE___".to_string())
                                .await;
                        });
                    }
                } else if ui.button("Install D2").clicked() {
                    self.state.is_d2_installing = true;
                    self.state.d2_install_output.clear();

                    let d2_install_tx = self.d2_install_tx.clone();
                    let command_str =
                        "curl -fsSL https://d2lang.com/install.sh | sh -s --".to_string();

                    crate::RUNTIME.get().unwrap().spawn(async move {
                        let mut child = tokio::process::Command::new("sh")
                            .arg("-c")
                            .arg(command_str)
                            .stdout(std::process::Stdio::piped())
                            .stderr(std::process::Stdio::piped())
                            .spawn()
                            .expect("Failed to spawn D2 installation process");

                        let stdout = child.stdout.take().unwrap();
                        let stderr = child.stderr.take().unwrap();

                        use tokio::io::AsyncBufReadExt;
                        let mut stdout_reader = tokio::io::BufReader::new(stdout).lines();
                        let mut stderr_reader = tokio::io::BufReader::new(stderr).lines();

                        loop {
                            tokio::select! {
                                line = stdout_reader.next_line() => {
                                    if let Ok(Some(line)) = line {
                                        let _ = d2_install_tx.send(line).await;
                                    } else {
                                        break;
                                    }
                                },
                                line = stderr_reader.next_line() => {
                                    if let Ok(Some(line)) = line {
                                        let _ = d2_install_tx.send(line).await;
                                    } else {
                                        break;
                                    }
                                }
                            }
                        }
                        let _ = d2_install_tx
                            .send("___INSTALL_COMPLETE___".to_string())
                            .await;
                    });
                }

                if self.state.is_d2_installing {
                    ui.label("Processing...");
                    ui.spinner();
                }

                egui::CollapsingHeader::new("Installation Log")
                    .default_open(false)
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .auto_shrink([false; 2])
                            .max_height(200.0) // Limit height of scrollable area
                            .show(ui, |ui| {
                                ui.code(&self.state.d2_install_output);
                            });
                    });
            });
    }
}
