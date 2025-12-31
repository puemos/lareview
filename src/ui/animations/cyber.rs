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

/// A pulsing circle.
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

/// Combined loader widget.
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

#[derive(Debug, Clone, Copy)]
pub enum Easing {
    Linear,
    Smooth,
    EaseInOut,
    Bounce,
    Elastic,
    Pulse,
}

/// Interpolate between two colors with a smooth wave pattern
pub fn color_wave(c1: Color32, c2: Color32, time: f64) -> Color32 {
    let t = smooth_wave(time, 1.0);
    lerp_color(c1, c2, t)
}

/// Advanced color wave with customizable speed and easing
pub fn color_wave_advanced(
    c1: Color32,
    c2: Color32,
    time: f64,
    speed: f32,
    easing: Easing,
) -> Color32 {
    let t = match easing {
        Easing::Linear => linear_wave(time, speed),
        Easing::Smooth => smooth_wave(time, speed),
        Easing::EaseInOut => ease_in_out_wave(time, speed),
        Easing::Bounce => bounce_wave(time, speed),
        Easing::Elastic => elastic_wave(time, speed),
        Easing::Pulse => pulse_wave(time, speed),
    };
    lerp_color(c1, c2, t)
}

// === Core interpolation ===

/// Lerp between two colors in linear RGB space
fn lerp_color(c1: Color32, c2: Color32, t: f32) -> Color32 {
    let rgba1 = egui::Rgba::from(c1);
    let rgba2 = egui::Rgba::from(c2);

    // Use egui's built-in lerp for smooth blending
    Color32::from(egui::Rgba::from_rgba_premultiplied(
        rgba1.r() + (rgba2.r() - rgba1.r()) * t,
        rgba1.g() + (rgba2.g() - rgba1.g()) * t,
        rgba1.b() + (rgba2.b() - rgba1.b()) * t,
        rgba1.a() + (rgba2.a() - rgba1.a()) * t,
    ))
}

// === Wave functions ===

fn linear_wave(time: f64, speed: f32) -> f32 {
    let t = ((time * speed as f64 * 2.0).cos() + 1.0) / 2.0;
    t as f32
}

fn smooth_wave(time: f64, speed: f32) -> f32 {
    let t = ((time * speed as f64 * 2.0).cos() + 1.0) / 2.0;
    // Smoothstep for extra smoothness
    let t = t * t * (3.0 - 2.0 * t);
    t as f32
}

fn pulse_wave(time: f64, speed: f32) -> f32 {
    let t = ((time * speed as f64 * 2.0).cos() + 1.0) / 2.0;
    (1.0 - (1.0 - t).powi(12)) as f32
}

fn ease_in_out_wave(time: f64, speed: f32) -> f32 {
    let t = ((time * speed as f64 * 2.0).cos() + 1.0) / 2.0;
    // Cubic easing
    let t = if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    };
    t as f32
}

fn bounce_wave(time: f64, speed: f32) -> f32 {
    let t = ((time * speed as f64 * 2.0).cos() + 1.0) / 2.0;
    let t = if t < 0.5 {
        8.0 * t * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(4) / 2.0
    };
    t as f32
}

fn elastic_wave(time: f64, speed: f32) -> f32 {
    let angle = time * speed as f64 * 2.0;
    let t = (angle.cos() + 1.0) / 2.0;
    // Add a subtle elastic effect
    let elastic = (angle * 4.0).sin() * 0.1 * (1.0 - t).abs();
    (t + elastic).clamp(0.0, 1.0) as f32
}

// === Multi-color gradients ===

/// Animate through multiple colors
pub fn color_wave_multi(colors: &[Color32], time: f64, speed: f32) -> Color32 {
    if colors.is_empty() {
        return Color32::BLACK;
    }
    if colors.len() == 1 {
        return colors[0];
    }

    let cycle = (time * speed as f64).rem_euclid(colors.len() as f64);
    let idx = cycle.floor() as usize;
    let next_idx = (idx + 1) % colors.len();
    let t = (cycle - idx as f64) as f32;

    // Smoothstep for smooth transitions
    let t = t * t * (3.0 - 2.0 * t);
    lerp_color(colors[idx], colors[next_idx], t)
}

// === Pulse effects ===

pub fn color_pulse(color: Color32, time: f64, speed: f32) -> Color32 {
    let t = smooth_wave(time, speed);
    let rgba = egui::Rgba::from(color);
    let new_a = rgba.a() * t;
    Color32::from(egui::Rgba::from_rgba_premultiplied(
        rgba.r(),
        rgba.g(),
        rgba.b(),
        new_a,
    ))
}

/// Pulse brightness
pub fn brightness_pulse(color: Color32, time: f64, speed: f32, intensity: f32) -> Color32 {
    let t = smooth_wave(time, speed);
    let factor = 1.0 + (t - 0.5) * 2.0 * intensity;

    let rgba = egui::Rgba::from(color);
    Color32::from(egui::Rgba::from_rgba_premultiplied(
        (rgba.r() * factor).clamp(0.0, 1.0),
        (rgba.g() * factor).clamp(0.0, 1.0),
        (rgba.b() * factor).clamp(0.0, 1.0),
        rgba.a(),
    ))
}

/// Renders text with a directional wave effect (light travels across characters)
pub fn render_wave_text(
    ui: &mut egui::Ui,
    text: &str,
    font_id: egui::FontId,
    c1: Color32,
    c2: Color32,
    time: f64,
    speed: f32,
) {
    let mut job = egui::text::LayoutJob::default();

    for (i, c) in text.chars().enumerate() {
        // Delay each character by a fraction of the cycle
        // 0.1 is a good spacing for most words
        let offset = i as f64 * 0.1;

        let char_color = color_wave_advanced(c1, c2, time - offset, speed, Easing::Pulse);

        job.append(
            &c.to_string(),
            0.0,
            egui::TextFormat {
                font_id: font_id.clone(),
                color: char_color,
                ..Default::default()
            },
        );
    }

    ui.label(job);
}
