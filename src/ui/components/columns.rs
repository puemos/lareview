use eframe::egui;

/// Three-column layout component
#[allow(dead_code)]
pub struct ThreeColumnLayout<'a> {
    left_content: Box<dyn FnMut(&mut egui::Ui) + 'a>,
    center_content: Box<dyn FnMut(&mut egui::Ui) + 'a>,
    right_content: Box<dyn FnMut(&mut egui::Ui) + 'a>,
}

impl<'a> ThreeColumnLayout<'a> {
    #[allow(dead_code)]
    pub fn new(
        left_fn: impl FnMut(&mut egui::Ui) + 'a,
        center_fn: impl FnMut(&mut egui::Ui) + 'a,
        right_fn: impl FnMut(&mut egui::Ui) + 'a,
    ) -> Self {
        Self {
            left_content: Box::new(left_fn),
            center_content: Box::new(center_fn),
            right_content: Box::new(right_fn),
        }
    }

    #[allow(dead_code)]
    pub fn show(mut self, ui: &mut egui::Ui) {
        ui.columns(3, |columns| {
            // Left column
            columns[0].vertical(|ui| {
                (self.left_content)(ui);
            });

            // Center column
            columns[1].vertical(|ui| {
                (self.center_content)(ui);
            });

            // Right column
            columns[2].vertical(|ui| {
                (self.right_content)(ui);
            });
        });
    }
}

/// Two-column layout component
#[allow(dead_code)]
pub struct TwoColumnLayout<'a> {
    left_content: Box<dyn FnMut(&mut egui::Ui) + 'a>,
    right_content: Box<dyn FnMut(&mut egui::Ui) + 'a>,
}

impl<'a> TwoColumnLayout<'a> {
    #[allow(dead_code)]
    pub fn new(
        left_fn: impl FnMut(&mut egui::Ui) + 'a,
        right_fn: impl FnMut(&mut egui::Ui) + 'a,
    ) -> Self {
        Self {
            left_content: Box::new(left_fn),
            right_content: Box::new(right_fn),
        }
    }

    #[allow(dead_code)]
    pub fn show(mut self, ui: &mut egui::Ui) {
        ui.columns(2, |columns| {
            // Left column
            columns[0].vertical(|ui| {
                (self.left_content)(ui);
            });

            // Right column
            columns[1].vertical(|ui| {
                (self.right_content)(ui);
            });
        });
    }
}
