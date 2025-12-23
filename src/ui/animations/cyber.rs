use egui::{Color32, Painter, Pos2, Stroke, vec2};

#[derive(Debug, Clone)]
pub struct ReticleParams {
    pub center: Pos2,
    pub radius_min: f32,
    pub radius_max: f32,
    pub time: f64,
    pub color: Color32,
    pub n_arms: usize,
    pub stroke_width: f32,
}

/// A rotating reticle with configurable arms.
pub fn rotating_reticle(painter: &Painter, params: ReticleParams) {
    let angle = (params.time * 3.0) as f32;
    for i in 0..params.n_arms {
        let arm_angle = angle + (i as f32 * std::f32::consts::PI * 2.0 / params.n_arms as f32);
        let direction = vec2(arm_angle.cos(), arm_angle.sin());

        painter.line_segment(
            [
                params.center + direction * params.radius_min,
                params.center + direction * params.radius_max,
            ],
            Stroke::new(params.stroke_width, params.color),
        );
    }
}

/// A pulsing circle that grows and shrinks.
pub fn pulsing_circle(
    painter: &Painter,
    center: Pos2,
    base_radius: f32,
    pulse_amplitude: f32,
    time: f64,
    color: Color32,
) {
    let pulse = (time * 2.0).sin().abs() as f32;
    let radius = base_radius + (pulse_amplitude * pulse);
    painter.circle_filled(center, radius, color.gamma_multiply(0.3));
    painter.circle_stroke(
        center,
        radius * 1.2,
        Stroke::new(1.0, color.gamma_multiply(0.6)),
    );
}

/// A combined loader widget using cyber elements.
pub fn paint_cyber_loader(
    painter: &Painter,
    center: Pos2,
    label: &str,
    time: f64,
    brand_color: Color32,
    text_color: Color32,
) {
    // 1. Rotating Reticle (Simple style from cyber_button)
    rotating_reticle(
        painter,
        ReticleParams {
            center,
            radius_min: 6.0,
            radius_max: 10.0,
            time,
            color: brand_color,
            n_arms: 4,
            stroke_width: 1.5,
        },
    );

    // 2. Status Text below
    let galley = painter.layout_no_wrap(
        label.to_uppercase(),
        egui::FontId::monospace(10.0),
        text_color,
    );
    let text_pos = center + vec2(-galley.size().x / 2.0, 20.0);
    painter.galley(text_pos, galley, Color32::TRANSPARENT);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CyberSpinnerSize {
    #[default]
    Sm,
    Md,
    Lg,
}

impl CyberSpinnerSize {
    pub fn pixels(&self) -> f32 {
        match self {
            Self::Sm => 14.0,
            Self::Md => 24.0,
            Self::Lg => 48.0,
        }
    }
}

/// A smaller, inline version of the loader for lists or toolbars.
pub fn cyber_spinner(
    ui: &mut egui::Ui,
    color: Color32,
    size: Option<CyberSpinnerSize>,
) -> egui::Response {
    let pixel_size = size.unwrap_or_default().pixels();
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(pixel_size, pixel_size), egui::Sense::hover());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let center = rect.center();
        let time = ui.input(|i| i.time);

        rotating_reticle(
            painter,
            ReticleParams {
                center,
                radius_min: pixel_size * 0.2,
                radius_max: pixel_size / 2.0,
                time,
                color,
                n_arms: 4,
                stroke_width: if pixel_size > 20.0 { 1.5 } else { 1.2 },
            },
        );
        ui.ctx().request_repaint();
    }

    response
}
