//! Theme - "Sharp & Stone" design system

use gpui::{hsla, Hsla};

/// Color palette for the Sharp & Stone theme
pub struct ThemeColors {
    // Backgrounds
    pub bg: Hsla,
    pub surface: Hsla,
    pub surface_alt: Hsla,
    pub surface_hover: Hsla,

    // Borders
    pub border: Hsla,
    pub border_strong: Hsla,

    // Text
    pub text: Hsla,
    pub text_strong: Hsla,
    pub text_muted: Hsla,
    pub text_subtle: Hsla,

    // Primary
    pub primary: Hsla,
    pub primary_hover: Hsla,
    pub primary_contrast: Hsla,

    // Semantic
    pub success: Hsla,
    pub warning: Hsla,
    pub danger: Hsla,

    // Heatmap
    pub heat_high: Hsla,
    pub heat_medium: Hsla,
    pub heat_low: Hsla,
}

impl Default for ThemeColors {
    fn default() -> Self {
        let hue_stone = 215.0 / 360.0;

        Self {
            // Backgrounds
            bg: hsla(hue_stone, 0.20, 0.96, 1.0),
            surface: hsla(0.0, 0.0, 1.0, 1.0),
            surface_alt: hsla(hue_stone, 0.20, 0.94, 1.0),
            surface_hover: hsla(hue_stone, 0.15, 0.92, 1.0),

            // Borders
            border: hsla(hue_stone, 0.15, 0.85, 1.0),
            border_strong: hsla(hue_stone, 0.15, 0.70, 1.0),

            // Text
            text: hsla(hue_stone, 0.30, 0.15, 1.0),
            text_strong: hsla(hue_stone, 0.40, 0.10, 1.0),
            text_muted: hsla(hue_stone, 0.15, 0.45, 1.0),
            text_subtle: hsla(hue_stone, 0.10, 0.65, 1.0),

            // Primary - Technical Blue
            primary: hsla(220.0 / 360.0, 0.90, 0.50, 1.0),
            primary_hover: hsla(220.0 / 360.0, 0.90, 0.45, 1.0),
            primary_contrast: hsla(0.0, 0.0, 1.0, 1.0),

            // Semantic
            success: hsla(150.0 / 360.0, 0.80, 0.35, 1.0),
            warning: hsla(40.0 / 360.0, 0.90, 0.40, 1.0),
            danger: hsla(350.0 / 360.0, 0.80, 0.50, 1.0),

            // Heatmap
            heat_high: hsla(350.0 / 360.0, 0.80, 0.60, 1.0),
            heat_medium: hsla(40.0 / 360.0, 0.90, 0.55, 1.0),
            heat_low: hsla(150.0 / 360.0, 0.70, 0.45, 1.0),
        }
    }
}

/// Typography settings
pub struct Typography {
    pub font_body: &'static str,
    pub font_mono: &'static str,
}

impl Default for Typography {
    fn default() -> Self {
        Self {
            font_body: "Inter",
            font_mono: "JetBrains Mono",
        }
    }
}

/// Spacing scale
pub struct Spacing {
    pub space_1: f32,
    pub space_2: f32,
    pub space_3: f32,
    pub space_4: f32,
    pub space_5: f32,
    pub space_6: f32,
    pub space_8: f32,
    pub space_10: f32,
}

impl Default for Spacing {
    fn default() -> Self {
        Self {
            space_1: 4.0,
            space_2: 8.0,
            space_3: 12.0,
            space_4: 16.0,
            space_5: 20.0,
            space_6: 24.0,
            space_8: 32.0,
            space_10: 40.0,
        }
    }
}

/// Complete theme
pub struct Theme {
    pub colors: ThemeColors,
    pub typography: Typography,
    pub spacing: Spacing,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            colors: ThemeColors::default(),
            typography: Typography::default(),
            spacing: Spacing::default(),
        }
    }
}

/// Global theme instance
pub fn theme() -> Theme {
    Theme::default()
}
