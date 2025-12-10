use catppuccin_egui::MOCHA;
use eframe::egui;

/// Selection chips component for any enum type
pub fn selection_chips<T>(
    ui: &mut egui::Ui,
    current_item: &mut T,
    items: &[T],
    labels: &[&str],
    label_prefix: &str,
) where
    T: PartialEq + Clone,
{
    ui.horizontal(|ui| {
        if !label_prefix.is_empty() {
            ui.label(label_prefix);
        }

        for (i, item) in items.iter().enumerate() {
            if i < labels.len() {
                let selected = current_item == item;
                let label = labels[i];

                let text = egui::RichText::new(label).color(if selected {
                    MOCHA.crust
                } else {
                    MOCHA.subtext0
                });

                let chip = egui::Button::new(text)
                    .fill(if selected { MOCHA.sky } else { MOCHA.surface0 })
                    .stroke(egui::Stroke::NONE);

                if ui.add(chip).clicked() {
                    *current_item = item.clone();
                }
            }
        }
    });
}
