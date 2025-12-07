//! Dark theme configuration for the app
use eframe::egui;

pub struct AppTheme {
    pub diff_added_bg: egui::Color32,
    pub diff_added_text: egui::Color32,
    pub diff_removed_bg: egui::Color32,
    pub diff_removed_text: egui::Color32,
    #[allow(dead_code)]
    pub diff_header: egui::Color32,
    pub diff_line_num: egui::Color32,
    pub text_primary: egui::Color32,
    pub text_secondary: egui::Color32,
    pub accent: egui::Color32,

    pub diff_equal_bg: egui::Color32,
    pub code_bg: egui::Color32,
    pub code_border: egui::Color32,
    pub inline_added_bg: egui::Color32,
    pub inline_removed_bg: egui::Color32,
    pub gutter_bg: egui::Color32,
    pub gutter_separator: egui::Color32,
}

impl Default for AppTheme {
    fn default() -> Self {
        Self {
            diff_added_bg: egui::Color32::from_rgb(34, 55, 34),
            diff_added_text: egui::Color32::from_rgb(87, 171, 90),
            diff_removed_bg: egui::Color32::from_rgb(64, 31, 31),
            diff_removed_text: egui::Color32::from_rgb(248, 81, 73),
            diff_header: egui::Color32::from_rgb(88, 166, 255),
            diff_line_num: egui::Color32::from_rgb(100, 100, 110),
            text_primary: egui::Color32::from_rgb(200, 200, 210),
            text_secondary: egui::Color32::from_rgb(150, 150, 160),
            accent: egui::Color32::from_rgb(88, 166, 255),
            diff_equal_bg: egui::Color32::TRANSPARENT,
            code_bg: egui::Color32::from_rgb(25, 25, 30),
            code_border: egui::Color32::from_rgb(60, 60, 70),
            inline_added_bg: egui::Color32::from_rgba_unmultiplied(34, 55, 34, 180),
            inline_removed_bg: egui::Color32::from_rgba_unmultiplied(64, 31, 31, 180),
            gutter_bg: egui::Color32::from_rgb(20, 20, 25),
            gutter_separator: egui::Color32::from_rgb(60, 60, 70),
        }
    }
}
