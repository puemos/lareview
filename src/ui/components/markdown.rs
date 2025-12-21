use crate::ui::spacing;
use crate::ui::theme::{Theme, current_theme};
use eframe::egui;
use egui_phosphor::regular as icons;
use once_cell::sync::Lazy;
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use std::ops::Add;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);
static THEME_SET: Lazy<ThemeSet> = Lazy::new(ThemeSet::load_defaults);

pub fn render_markdown(ui: &mut egui::Ui, text: &str) {
    let theme = current_theme();
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(text, options);
    let mut state = MarkdownState::new(theme);

    for event in parser {
        match event {
            Event::Start(tag) => state.start_tag(ui, tag),
            Event::End(tag) => state.end_tag(ui, tag),
            Event::Text(text) => state.text(text.as_ref()),
            Event::Code(code) => state.inline_code(code.as_ref()),
            Event::SoftBreak => state.soft_break(),
            Event::HardBreak => state.hard_break(ui),
            Event::Rule => {
                ui.add_space(spacing::SPACING_SM);
                ui.separator();
                ui.add_space(spacing::SPACING_SM);
            }
            Event::TaskListMarker(checked) => state.task_list_marker(checked),
            _ => {}
        }
    }

    state.flush(ui);
}

struct MarkdownState {
    theme: Theme,
    job: egui::text::LayoutJob,
    is_bold: bool,
    is_italic: bool,
    is_strikethrough: bool,
    list_stack: Vec<Option<u64>>,
    in_paragraph: bool,
    in_heading: Option<u32>,
    in_code_block: bool,
    code_block_lang: String,
    code_block_content: String,
    pending_bullet: Option<String>,
    pending_check: Option<(bool, egui::Color32)>,
    in_blockquote: bool,
    in_table: bool,
    table_rows: Vec<Vec<egui::text::LayoutJob>>,
    current_table_row: Vec<egui::text::LayoutJob>,
    active_link: Option<String>,
    job_has_link: Option<String>,
}

impl MarkdownState {
    fn new(theme: Theme) -> Self {
        Self {
            theme,
            job: egui::text::LayoutJob::default(),
            is_bold: false,
            is_italic: false,
            is_strikethrough: false,
            list_stack: Vec::new(),
            in_paragraph: false,
            in_heading: None,
            in_code_block: false,
            code_block_lang: String::new(),
            code_block_content: String::new(),
            pending_bullet: None,
            pending_check: None,
            in_blockquote: false,
            in_table: false,
            table_rows: Vec::new(),
            current_table_row: Vec::new(),
            active_link: None,
            job_has_link: None,
        }
    }

    fn current_format(&self) -> egui::TextFormat {
        let size = if let Some(level) = self.in_heading {
            match level {
                1 => 22.0,
                2 => 18.0,
                3 => 16.0,
                _ => 15.0,
            }
        } else {
            15.0
        };

        let color = if self.active_link.is_some() {
            self.theme.brand
        } else if self.in_heading.is_some() || self.is_bold {
            self.theme.text_primary
        } else {
            self.theme.text_secondary
        };

        let font_name = if self.is_bold {
            "GeistBold"
        } else if self.is_italic {
            "GeistItalic"
        } else {
            "Geist"
        };

        let font_id = egui::FontId::new(size, egui::FontFamily::Name(font_name.into()));

        egui::TextFormat {
            font_id,
            color,
            line_height: Some(if self.in_heading.is_some() {
                size * 1.4
            } else {
                26.0
            }),
            strikethrough: if self.is_strikethrough {
                egui::Stroke::new(1.2, color)
            } else {
                egui::Stroke::NONE
            },
            underline: if self.active_link.is_some() {
                egui::Stroke::new(1.0, color)
            } else {
                egui::Stroke::NONE
            },
            valign: egui::Align::Center,
            ..Default::default()
        }
    }

    fn start_tag(&mut self, ui: &mut egui::Ui, tag: Tag) {
        match tag {
            Tag::Paragraph => {
                self.in_paragraph = true;
                self.job = egui::text::LayoutJob::default();
            }
            Tag::Heading { level, .. } => {
                self.flush(ui);
                ui.add_space(spacing::SPACING_SM);
                self.in_heading = Some(level as u32);
                self.job = egui::text::LayoutJob::default();
            }
            Tag::Strong => self.is_bold = true,
            Tag::Emphasis => self.is_italic = true,
            Tag::Strikethrough => self.is_strikethrough = true,
            Tag::List(first) => {
                self.flush(ui);
                self.list_stack.push(first);
            }
            Tag::Item => {
                self.flush(ui);
                let bullet = if let Some(Some(n)) = self.list_stack.last_mut() {
                    let b = format!("{}.", n);
                    *n += 1;
                    b
                } else {
                    "•".to_string()
                };
                self.pending_bullet = Some(bullet);
                self.in_paragraph = true;
                self.job = egui::text::LayoutJob::default();
            }
            Tag::CodeBlock(kind) => {
                self.flush(ui);
                self.in_code_block = true;
                self.code_block_content.clear();
                if let pulldown_cmark::CodeBlockKind::Fenced(lang) = kind {
                    self.code_block_lang = lang.to_string();
                } else {
                    self.code_block_lang.clear();
                }
            }
            Tag::BlockQuote(_) => {
                self.flush(ui);
                self.in_blockquote = true;
            }
            Tag::Table(_) => {
                self.flush(ui);
                self.in_table = true;
                self.table_rows.clear();
            }
            Tag::TableRow => {
                self.current_table_row.clear();
            }
            Tag::TableCell => {
                self.job = egui::text::LayoutJob::default();
            }
            Tag::Link { dest_url, .. } => {
                self.active_link = Some(dest_url.to_string());
                self.job_has_link = Some(dest_url.to_string());
            }
            Tag::FootnoteDefinition(label) => {
                self.flush(ui);
                self.job = egui::text::LayoutJob::default();
                self.job
                    .append(&format!("{}: ", label), 0.0, self.current_format());
            }
            _ => {}
        }
    }

    fn end_tag(&mut self, ui: &mut egui::Ui, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph => {
                self.flush(ui);
                self.in_paragraph = false;
            }
            TagEnd::Heading(_) => {
                self.flush(ui);
                self.in_heading = None;
                ui.add_space(spacing::SPACING_XS);
            }
            TagEnd::Strong => self.is_bold = false,
            TagEnd::Emphasis => self.is_italic = false,
            TagEnd::Strikethrough => self.is_strikethrough = false,
            TagEnd::List(_) => {
                self.list_stack.pop();
            }
            TagEnd::Item => {
                self.flush(ui);
                self.in_paragraph = false;
            }
            TagEnd::CodeBlock => {
                self.in_code_block = false;
                self.render_code_block(ui);
            }
            TagEnd::BlockQuote(_) => {
                self.flush(ui);
                self.in_blockquote = false;
            }
            TagEnd::Table => {
                self.render_table(ui);
                self.in_table = false;
            }
            TagEnd::TableRow => {
                self.table_rows
                    .push(std::mem::take(&mut self.current_table_row));
            }
            TagEnd::TableCell => {
                self.current_table_row.push(std::mem::take(&mut self.job));
            }
            TagEnd::Link => {
                self.active_link = None;
            }
            TagEnd::FootnoteDefinition => {
                self.flush(ui);
            }
            _ => {}
        }
    }

    fn text(&mut self, content: &str) {
        if self.in_code_block {
            self.code_block_content.push_str(content);
        } else {
            self.job.append(content, 0.0, self.current_format());
        }
    }

    fn inline_code(&mut self, content: &str) {
        let mut format = self.current_format();
        format.font_id = egui::FontId::monospace(13.5);
        format.background = self.theme.bg_tertiary.gamma_multiply(0.8);
        format.color = self.theme.text_primary;
        format.valign = egui::Align::Center;

        self.job.append(content, 0.0, format);
    }

    fn soft_break(&mut self) {
        if self.in_code_block {
            self.code_block_content.push('\n');
        } else {
            self.job.append(" ", 0.0, self.current_format());
        }
    }

    fn hard_break(&mut self, ui: &mut egui::Ui) {
        self.flush(ui);
    }

    fn task_list_marker(&mut self, checked: bool) {
        let color = if checked {
            self.theme.success
        } else {
            self.theme.text_muted
        };

        self.pending_bullet = None;
        self.pending_check = Some((checked, color));
    }

    fn flush(&mut self, ui: &mut egui::Ui) {
        if self.job.sections.is_empty() {
            return;
        }
        if self.in_table {
            return;
        }

        let job = std::mem::take(&mut self.job);
        let link_url = self.job_has_link.take();
        let available_width = ui.available_width();

        let render_job = |ui: &mut egui::Ui,
                          mut job: egui::text::LayoutJob,
                          url: Option<String>,
                          max_w: f32|
         -> egui::Response {
            job.wrap.max_width = max_w;
            let resp = ui.label(job);
            if let Some(url) = url {
                let resp = resp.interact(egui::Sense::click());
                if resp.clicked() {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                }
                if resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
            }
            resp
        };

        if self.in_blockquote {
            ui.horizontal_top(|ui| {
                ui.add_space(8.0);
                let text_max_w = available_width - 32.0;
                let bg_color = self.theme.text_disabled.gamma_multiply(0.5);

                ui.vertical(|ui| {
                    let resp = render_job(ui, job, link_url, text_max_w);
                    let rect = resp.rect;
                    ui.painter().line_segment(
                        [
                            rect.left_top().add(egui::vec2(-12.0, 0.0)),
                            rect.left_bottom().add(egui::vec2(-12.0, 0.0)),
                        ],
                        egui::Stroke::new(2.5, bg_color),
                    );
                });
            });
        } else if let Some((checked, color)) = self.pending_check.take() {
            ui.horizontal_top(|ui| {
                let indent = (self.list_stack.len().saturating_sub(1)) as f32 * 24.0;
                ui.add_space(indent);

                let icon = if checked {
                    icons::CHECK_SQUARE
                } else {
                    icons::SQUARE
                };

                ui.allocate_ui_with_layout(
                    egui::vec2(28.0, 20.0),
                    egui::Layout::left_to_right(egui::Align::Min),
                    |ui| {
                        ui.add_space(2.0);
                        ui.label(egui::RichText::new(icon).color(color).size(16.0));
                    },
                );

                let text_max_w = available_width - indent - 28.0;
                ui.vertical(|ui| {
                    render_job(ui, job, link_url, text_max_w);
                });
            });
        } else if let Some(bullet) = self.pending_bullet.take() {
            ui.horizontal_top(|ui| {
                let indent = (self.list_stack.len().saturating_sub(1)) as f32 * 24.0;
                ui.add_space(indent);

                ui.allocate_ui_with_layout(
                    egui::vec2(28.0, 20.0),
                    egui::Layout::left_to_right(egui::Align::Min),
                    |ui| {
                        ui.label(
                            egui::RichText::new(bullet)
                                .color(self.theme.text_muted)
                                .size(14.0),
                        );
                    },
                );

                let text_max_w = available_width - indent - 28.0;
                ui.vertical(|ui| {
                    render_job(ui, job, link_url, text_max_w);
                });
            });
        } else if !self.list_stack.is_empty() {
            ui.horizontal_top(|ui| {
                let indent = (self.list_stack.len().saturating_sub(1)) as f32 * 24.0 + 28.0;
                ui.add_space(indent);
                let text_max_w = available_width - indent;
                ui.vertical(|ui| {
                    render_job(ui, job, link_url, text_max_w);
                });
            });
        } else {
            render_job(ui, job, link_url, available_width);
        }
    }

    fn render_code_block(&mut self, ui: &mut egui::Ui) {
        let content = self.code_block_content.trim().to_string();
        if content.is_empty() {
            return;
        }

        let syntax = SYNTAX_SET
            .find_syntax_by_token(&self.code_block_lang)
            .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());

        let mut h = HighlightLines::new(syntax, &THEME_SET.themes["base16-ocean.dark"]);
        let mut job = egui::text::LayoutJob::default();

        for line in LinesWithEndings::from(&content) {
            let ranges: Vec<(syntect::highlighting::Style, &str)> =
                h.highlight_line(line, &SYNTAX_SET).unwrap();

            for (style, text) in ranges {
                let color = egui::Color32::from_rgb(
                    style.foreground.r,
                    style.foreground.g,
                    style.foreground.b,
                );
                job.append(
                    text,
                    0.0,
                    egui::TextFormat {
                        font_id: egui::FontId::monospace(13.0),
                        color,
                        ..Default::default()
                    },
                );
            }
        }

        egui::Frame::NONE
            .fill(self.theme.bg_tertiary)
            .inner_margin(egui::Margin::same(spacing::SPACING_MD as i8))
            .corner_radius(crate::ui::spacing::RADIUS_MD)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                egui::ScrollArea::horizontal()
                    .id_salt(ui.id().with("code_scroll"))
                    .show(ui, |ui| {
                        ui.add(egui::Label::new(job).wrap_mode(egui::TextWrapMode::Extend));
                    });
            });
        ui.add_space(spacing::SPACING_SM);
    }

    fn render_table(&mut self, ui: &mut egui::Ui) {
        if self.table_rows.is_empty() {
            return;
        }

        egui::Frame::NONE
            .fill(self.theme.bg_tertiary.gamma_multiply(0.3))
            .stroke(egui::Stroke::new(1.0, self.theme.border_secondary))
            .corner_radius(crate::ui::spacing::RADIUS_MD)
            .inner_margin(egui::Margin::same(spacing::SPACING_SM as i8))
            .show(ui, |ui| {
                let num_cols = self.table_rows[0].len();
                egui::Grid::new(ui.id().with("table_grid"))
                    .num_columns(num_cols)
                    .spacing([12.0, 8.0])
                    .striped(true)
                    .show(ui, |ui| {
                        for row in &self.table_rows {
                            for cell in row {
                                ui.label(cell.clone());
                            }
                            ui.end_row();
                        }
                    });
            });
        ui.add_space(spacing::SPACING_SM);
    }
}
