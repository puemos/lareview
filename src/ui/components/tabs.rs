use catppuccin_egui::MOCHA;
use eframe::egui;

/// Common tab bar component for generic enums
pub struct TabBar<T> {
    tabs: Vec<(&'static str, T)>,
    current_tab: *mut T,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> TabBar<T>
where
    T: PartialEq + Copy,
{
    pub fn new(current_tab: &mut T) -> Self {
        Self {
            tabs: Vec::new(),
            current_tab: current_tab as *mut T,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn add(mut self, title: &'static str, tab_value: T) -> Self {
        self.tabs.push((title, tab_value));
        self
    }

    pub fn show(self, ui: &mut egui::Ui) {
        let current_tab = unsafe { &mut *self.current_tab };

        ui.horizontal(|ui| {
            for (title, tab_value) in self.tabs {
                let is_active = *current_tab == tab_value;
                let text_color = if is_active {
                    MOCHA.mauve
                } else {
                    MOCHA.subtext0
                };

                let response = ui.add(
                    egui::Button::new(egui::RichText::new(title).size(14.0).color(text_color))
                        .frame(false),
                );

                if is_active {
                    let rect = response.rect;
                    ui.painter().hline(
                        rect.x_range(),
                        rect.bottom(),
                        egui::Stroke::new(2.0, MOCHA.mauve),
                    );
                }

                if response.clicked() {
                    *current_tab = tab_value;
                }
                ui.add_space(10.0);
            }
        });
    }
}
