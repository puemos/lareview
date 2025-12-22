use crate::ui::spacing;
use crate::ui::theme::{Theme, current_theme};
use eframe::egui;
use egui_phosphor::regular as icons;
use once_cell::sync::Lazy;
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use std::collections::{HashMap, VecDeque};
use std::ops::Add;
use std::sync::{Arc, Mutex};
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);
static THEME_SET: Lazy<ThemeSet> = Lazy::new(ThemeSet::load_defaults);

const CODE_HIGHLIGHT_CACHE_CAPACITY: usize = 128;

#[derive(Default, Clone)]
struct CodeHighlightCache {
    inner: Arc<Mutex<LruCodeHighlightCache>>,
}

#[derive(Default)]
struct LruCodeHighlightCache {
    map: HashMap<u64, Arc<egui::text::LayoutJob>>,
    order: VecDeque<u64>,
}

impl CodeHighlightCache {
    fn get_or_insert_with(
        &self,
        key: u64,
        compute: impl FnOnce() -> Arc<egui::text::LayoutJob>,
    ) -> Arc<egui::text::LayoutJob> {
        let mut inner = self.inner.lock().unwrap();

        if let Some(job) = inner.map.get(&key).cloned() {
            inner.touch(key);
            return job;
        }

        let job = compute();
        inner.insert(key, job.clone());
        job
    }
}

impl LruCodeHighlightCache {
    fn touch(&mut self, key: u64) {
        if let Some(pos) = self.order.iter().position(|&k| k == key) {
            self.order.remove(pos);
        }
        self.order.push_back(key);
    }

    fn insert(&mut self, key: u64, job: Arc<egui::text::LayoutJob>) {
        self.map.insert(key, job);
        self.touch(key);

        while self.order.len() > CODE_HIGHLIGHT_CACHE_CAPACITY {
            if let Some(old) = self.order.pop_front() {
                self.map.remove(&old);
            } else {
                break;
            }
        }
    }
}

fn code_cache_id() -> egui::Id {
    egui::Id::new("markdown_code_highlight_cache")
}

const FNV_OFFSET_BASIS: u64 = 14695981039346656037;
const FNV_PRIME: u64 = 1099511628211;

fn fnv1a_update(mut hash: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn mix_u64(a: u64, b: u64) -> u64 {
    a ^ b.wrapping_mul(0x9E3779B97F4A7C15).rotate_left(7)
}

fn code_cache_key(content_hash: u64, lang: &str, wrap_px: u32, theme_hash: u64) -> u64 {
    let mut key = content_hash;
    key = mix_u64(key, egui::util::hash(lang));
    key = mix_u64(key, wrap_px as u64);
    key = mix_u64(key, theme_hash);
    key
}

pub fn render_markdown(ui: &mut egui::Ui, text: &str) {
    let theme = current_theme();

    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);

    let parser = Parser::new_ext(text, options);

    ui.spacing_mut().item_spacing.y = 6.0;

    let mut state = MarkdownState::new(theme);

    for event in parser {
        match event {
            Event::Start(tag) => state.start_tag(ui, tag),
            Event::End(tag) => state.end_tag(ui, tag),
            Event::Text(content) => state.text(&content),
            Event::Code(content) => state.inline_code(&content),
            Event::SoftBreak => state.soft_break(),
            Event::HardBreak => state.hard_break(ui),
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
    code_block_hash: u64,
    code_block_counter: usize, // Added to track unique code blocks
    pending_bullet: Option<String>,
    pending_check: Option<(bool, egui::Color32)>,
    in_blockquote: bool,
    in_table: bool,
    table_rows: Vec<Vec<Arc<egui::text::LayoutJob>>>,
    current_table_row: Vec<Arc<egui::text::LayoutJob>>,
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
            code_block_hash: FNV_OFFSET_BASIS,
            code_block_counter: 0, // Initialize counter
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
                1 => 24.0,
                2 => 21.0,
                3 => 18.0,
                4 => 15.0,
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

        egui::TextFormat {
            font_id: egui::FontId::new(size, egui::FontFamily::Name(font_name.into())),
            color,
            italics: self.is_italic,
            strikethrough: if self.is_strikethrough {
                egui::Stroke::new(1.0, color)
            } else {
                egui::Stroke::NONE
            },
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
                self.code_block_hash = FNV_OFFSET_BASIS;
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
                self.current_table_row
                    .push(Arc::new(std::mem::take(&mut self.job)));
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
            self.code_block_hash = fnv1a_update(self.code_block_hash, content.as_bytes());
        } else {
            self.job.append(content, 0.0, self.current_format());
        }
    }

    fn inline_code(&mut self, content: &str) {
        let mut format = self.current_format();
        format.font_id = egui::FontId::monospace(13.0);
        format.background = self.theme.bg_tertiary;
        format.color = self.theme.brand;

        self.job.append(content, 0.0, format);
    }

    fn soft_break(&mut self) {
        if self.in_code_block {
            self.code_block_content.push('\n');
            self.code_block_hash = fnv1a_update(self.code_block_hash, b"\n");
        } else {
            self.job.append(" ", 0.0, self.current_format());
        }
    }

    fn hard_break(&mut self, ui: &mut egui::Ui) {
        if self.in_code_block {
            self.code_block_content.push('\n');
            self.code_block_hash = fnv1a_update(self.code_block_hash, b"\n");
        } else {
            self.flush(ui);
        }
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

        let job = std::mem::take(&mut self.job);
        let link = self.job_has_link.take();

        let render_job = |ui: &mut egui::Ui, job: egui::text::LayoutJob, link: Option<String>| {
            let available_width = ui.available_width();
            let mut job = job;
            job.wrap.max_width = available_width;

            let mut label = egui::Label::new(job).wrap_mode(egui::TextWrapMode::Wrap);

            if link.is_some() {
                label = label.sense(egui::Sense::click());
            }

            let response = ui.add(label);

            if let Some(url) = link {
                let response = response.on_hover_text(&url);

                if response.clicked() {
                    ui.ctx().open_url(egui::OpenUrl { url, new_tab: true });
                }
            }
        };

        if self.in_table {
            self.job = egui::text::LayoutJob::default();
            return;
        }

        if let Some(bullet) = self.pending_bullet.take() {
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.label(
                    egui::RichText::new(bullet)
                        .color(self.theme.text_secondary)
                        .size(15.0),
                );
                render_job(ui, job, link);
            });
        } else if let Some((checked, color)) = self.pending_check.take() {
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                let icon = if checked {
                    icons::CHECK_SQUARE
                } else {
                    icons::SQUARE
                };
                ui.label(egui::RichText::new(icon).color(color).size(16.0));
                render_job(ui, job, link);
            });
        } else if self.in_blockquote {
            let available_width = ui.available_width();
            let text_max_w = (available_width - 32.0).max(32.0);

            let mut job = job;
            job.wrap.max_width = text_max_w;

            egui::Frame::NONE
                .fill(self.theme.bg_secondary.gamma_multiply(0.3))
                .stroke(egui::Stroke::new(1.0, self.theme.border_secondary))
                .corner_radius(crate::ui::spacing::RADIUS_MD)
                .inner_margin(egui::Margin::same(spacing::SPACING_SM as i8))
                .show(ui, |ui| {
                    let rect = ui.available_rect_before_wrap();
                    ui.painter().line_segment(
                        [
                            rect.left_top().add(egui::vec2(-12.0, 0.0)),
                            rect.left_bottom().add(egui::vec2(-12.0, 0.0)),
                        ],
                        egui::Stroke::new(2.0, self.theme.brand),
                    );
                    render_job(ui, job, link);
                });
        } else {
            render_job(ui, job, link);
        }

        ui.add_space(2.0);
    }

    fn render_code_block(&mut self, ui: &mut egui::Ui) {
        let content = self.code_block_content.as_str();
        if content.trim().is_empty() {
            return;
        }

        self.code_block_counter += 1;

        // This affects both the highlight cache key and the layout.
        let wrap_px = (ui.available_width() - (spacing::SPACING_MD * 2.0))
            .max(32.0)
            .round() as u32;

        let theme_hash = egui::util::hash((
            self.theme.bg_tertiary,
            self.theme.text_primary,
            self.theme.text_secondary,
            self.theme.brand,
        ));

        let cache_key = code_cache_key(
            self.code_block_hash,
            &self.code_block_lang,
            wrap_px,
            theme_hash,
        );

        let job: Arc<egui::text::LayoutJob> = ui.ctx().memory_mut(|mem| {
            let cache = mem
                .data
                .get_temp_mut_or_default::<CodeHighlightCache>(code_cache_id());

            cache.get_or_insert_with(cache_key, || {
                let syntax = SYNTAX_SET
                    .find_syntax_by_token(&self.code_block_lang)
                    .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());

                let mut h = HighlightLines::new(syntax, &THEME_SET.themes["base16-ocean.dark"]);

                let mono = egui::FontId::monospace(13.0);
                let mut job = egui::text::LayoutJob::default();
                job.wrap.max_width = wrap_px as f32;

                for line in LinesWithEndings::from(content) {
                    let Ok(ranges) = h.highlight_line(line, &SYNTAX_SET) else {
                        continue;
                    };

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
                                font_id: mono.clone(),
                                color,
                                ..Default::default()
                            },
                        );
                    }
                }

                Arc::new(job)
            })
        });

        egui::Frame::NONE
            .fill(self.theme.bg_tertiary)
            .corner_radius(crate::ui::spacing::RADIUS_MD)
            .inner_margin(egui::Margin::same(spacing::SPACING_MD as i8))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt(ui.id().with("code_scroll").with(self.code_block_counter))
                    .max_height(260.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add(egui::Label::new(job.clone()).wrap_mode(egui::TextWrapMode::Wrap));
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
