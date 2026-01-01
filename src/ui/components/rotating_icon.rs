use eframe::egui;

pub fn rotating_icon(ui: &mut egui::Ui, icon: &str, color: egui::Color32, size: f32) {
    let font_id = crate::ui::typography::body_font(size);
    let galley = ui
        .painter()
        .layout_no_wrap(icon.to_string(), font_id, color);
    let (rect, _response) = ui.allocate_exact_size(galley.size(), egui::Sense::hover());

    if ui.is_rect_visible(rect) {
        let angle = (ui.input(|i| i.time) * 3.0) as f32;
        let center = rect.center();

        let rot = egui::emath::Rot2::from_angle(angle);
        let pos = center - rot * galley.rect.center().to_vec2();

        ui.painter().add(egui::Shape::Text(egui::epaint::TextShape {
            pos,
            galley,
            underline: egui::Stroke::NONE,
            override_text_color: Some(color),
            angle,
            fallback_color: color,
            opacity_factor: 1.0,
        }));

        ui.ctx().request_repaint();
    }
}
