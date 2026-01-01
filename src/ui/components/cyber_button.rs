use crate::ui::theme::current_theme;
use egui::{Color32, FontId, Response, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

pub fn cyber_button(
    ui: &mut Ui,
    text: &str,
    enabled: bool,
    is_generating: bool,
    color: Option<Color32>,
    fixed_width: Option<f32>,
) -> Response {
    let theme = current_theme();
    let time = ui.input(|i| i.time);

    let height = 28.0;
    let width = fixed_width.unwrap_or_else(|| ui.available_width());
    let (rect, response) = ui.allocate_exact_size(
        vec2(width, height),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );

    // Repaint triggers for animations
    if is_generating || (response.hovered() && enabled) {
        ui.ctx().request_repaint();
        if enabled {
            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
        }
    }

    let hover_ratio = ui
        .ctx()
        .animate_bool(response.id, response.hovered() && enabled);

    if !ui.is_rect_visible(rect) {
        return response;
    }

    let painter = ui.painter();
    let accent_color = color.unwrap_or(theme.brand);

    let bg_color = if is_generating {
        theme.bg_secondary.gamma_multiply(0.4)
    } else if enabled {
        theme
            .bg_card
            .lerp_to_gamma(accent_color, hover_ratio * 0.08)
    } else {
        theme.bg_muted.gamma_multiply(0.3)
    };

    painter.rect_filled(rect, egui::CornerRadius::same(5), bg_color);

    let border_color = if enabled {
        theme.border.lerp_to_gamma(accent_color, hover_ratio)
    } else {
        theme.border.gamma_multiply(0.2)
    };
    painter.rect_stroke(
        rect,
        egui::CornerRadius::same(5),
        Stroke::new(1.0, border_color),
        StrokeKind::Inside,
    );

    let content_color = if is_generating {
        theme.text_accent
    } else if enabled {
        theme.text_primary.lerp_to_gamma(
            if color.is_some() {
                accent_color
            } else {
                theme.text_accent
            },
            hover_ratio,
        )
    } else {
        theme.text_disabled
    };

    let display_text = if is_generating { "RUNNING..." } else { text }.to_uppercase();
    let galley = painter.layout_no_wrap(display_text, FontId::monospace(11.0), content_color);

    let gap = 10.0;
    let icon_size = 8.0;
    let total_w = icon_size + gap + galley.size().x;

    let start_x = rect.center().x - (total_w / 2.0);
    let icon_center = pos2(start_x + (icon_size / 2.0), rect.center().y);

    // DRAW GEOMETRIC ICON
    if is_generating {
        // Rotating Reticle (Running)
        crate::ui::animations::cyber::rotating_reticle(
            painter,
            crate::ui::animations::cyber::ReticleParams {
                center: icon_center,
                radius_min: 2.0,
                radius_max: 5.0,
                time,
                color: accent_color,
                n_arms: 4,
                stroke_width: 1.2,
            },
        );
    } else {
        // Dynamic Diamond icon with interactive animations.
        let is_custom = color.is_some();

        // Calculate a pulsating scale factor using the hover animation ratio.
        // A sine function over the normalized ratio creates a smooth pulse effect.
        let pulse = 1.0 + (hover_ratio * std::f32::consts::PI).sin() * 0.2;

        let angle = if enabled {
            if is_custom {
                // Secondary actions undergo a full 180-degree flip.
                hover_ratio * std::f32::consts::PI
            } else {
                // Primary actions perform a professional 45-degree rotation.
                hover_ratio * std::f32::consts::PI / 4.0
            }
        } else {
            0.0
        };

        let cos_a = angle.cos();
        let sin_a = angle.sin();
        let r = 4.0 * pulse;

        let corners = [vec2(0.0, -r), vec2(r, 0.0), vec2(0.0, r), vec2(-r, 0.0)].map(|v| {
            let rx = v.x * cos_a - v.y * sin_a;
            let ry = v.x * sin_a + v.y * cos_a;
            icon_center + vec2(rx, ry)
        });

        painter.add(egui::Shape::convex_polygon(
            corners.to_vec(),
            Color32::TRANSPARENT,
            Stroke::new(1.2, content_color.gamma_multiply(0.8)),
        ));

        // Center dot also pulses
        if hover_ratio > 0.01 {
            painter.circle_filled(icon_center, 1.0 * hover_ratio * pulse, accent_color);
        }
    }

    // DRAW TEXT
    let text_pos = pos2(
        start_x + icon_size + gap,
        rect.center().y - galley.size().y / 2.0,
    );
    painter.galley(text_pos, galley, Color32::TRANSPARENT);

    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, text));

    response
}
