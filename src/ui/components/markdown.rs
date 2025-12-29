use crate::ui::spacing;
use crate::ui::theme::{Theme, current_theme};
use crate::ui::typography;
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
const MARKDOWN_CACHE_CAPACITY: usize = 50;

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

fn code_cache_key(content_hash: u64, lang: &str, theme_hash: u64) -> u64 {
    let mut key = content_hash;
    key = mix_u64(key, egui::util::hash(lang));
    key = mix_u64(key, theme_hash);
    key
}

/// Quantize a width to the nearest step (default 40px) to reduce text re-layout frequency.
const WRAP_WIDTH_STEP: f32 = 40.0;
// Size limit for code block syntax highlighting (50KB)
// Large blocks can cause runaway highlighting time, so we fall back to plain text
// This prevents rendering hangs on extremely large code blocks
const CODE_BLOCK_SIZE_LIMIT: usize = 50000;

fn quantize_width(width: f32) -> f32 {
    (width / WRAP_WIDTH_STEP).floor() * WRAP_WIDTH_STEP
}

// --- Markdown Caching Types ---

#[derive(Clone)]
enum ItemDecoration {
    None,
    Bullet(String),
    Check(bool, egui::Color32),
    BlockQuote,
}

#[derive(Clone)]
enum MarkdownItem {
    Text {
        job: Arc<egui::text::LayoutJob>,
        link: Option<String>,
        decoration: ItemDecoration,
    },
    CodeBlock {
        content: String,
        lang: String,
        hash: u64,
    },
    Table {
        rows: Vec<Vec<Arc<egui::text::LayoutJob>>>,
    },
    Space(f32),
}

#[derive(Default, Clone)]
struct MarkdownCache {
    inner: Arc<Mutex<LruMarkdownCache>>,
}

#[derive(Default)]
struct LruMarkdownCache {
    map: HashMap<u64, Arc<Vec<MarkdownItem>>>,
    order: VecDeque<u64>,
}

impl MarkdownCache {
    fn get_or_insert_with(
        &self,
        key: u64,
        compute: impl FnOnce() -> Arc<Vec<MarkdownItem>>,
    ) -> Arc<Vec<MarkdownItem>> {
        let mut inner = self.inner.lock().unwrap();

        if let Some(items) = inner.map.get(&key).cloned() {
            inner.touch(key);
            return items;
        }

        let items = compute();
        inner.insert(key, items.clone());
        items
    }
}

impl LruMarkdownCache {
    fn touch(&mut self, key: u64) {
        if let Some(pos) = self.order.iter().position(|&k| k == key) {
            self.order.remove(pos);
        }
        self.order.push_back(key);
    }

    fn insert(&mut self, key: u64, items: Arc<Vec<MarkdownItem>>) {
        self.map.insert(key, items);
        self.touch(key);

        while self.order.len() > MARKDOWN_CACHE_CAPACITY {
            if let Some(old) = self.order.pop_front() {
                self.map.remove(&old);
            } else {
                break;
            }
        }
    }
}

fn markdown_cache_id() -> egui::Id {
    egui::Id::new("markdown_parse_cache")
}

fn get_cached_theme_hash(ui: &egui::Ui, theme: &Theme) -> u64 {
    let cache_key = egui::Id::new("theme_hash");
    let current_hash = egui::util::hash((
        theme.bg_tertiary,
        theme.text_primary,
        theme.text_secondary,
        theme.brand,
        theme.success,
        theme.text_muted,
        theme.bg_secondary,
        theme.border_secondary,
    ));

    ui.ctx().memory_mut(|mem| {
        let cached = mem.data.get_temp_mut_or_default::<u64>(cache_key);
        if *cached != current_hash {
            *cached = current_hash;
        }
        *cached
    })
}

pub fn render_markdown(ui: &mut egui::Ui, text: &str) {
    let theme = current_theme();

    ui.spacing_mut().item_spacing.y = 6.0;

    let theme_hash = get_cached_theme_hash(ui, &theme);
    let content_hash = egui::util::hash(text);
    let cache_key = mix_u64(content_hash, theme_hash);

    let items: Arc<Vec<MarkdownItem>> = ui.ctx().memory_mut(|mem| {
        let cache = mem
            .data
            .get_temp_mut_or_default::<MarkdownCache>(markdown_cache_id());

        cache.get_or_insert_with(cache_key, || Arc::new(parse_markdown(text, theme)))
    });

    let mut code_block_counter = 0;
    let mut table_counter = 0;

    // Use the smaller of available width or 900px for better readability
    let max_width = ui.available_width().min(900.0);
    ui.set_max_width(max_width);

    for item in items.iter() {
        match item {
            MarkdownItem::Text {
                job,
                link,
                decoration,
            } => {
                render_text_item(ui, job, link.clone(), decoration, theme);
            }
            MarkdownItem::CodeBlock {
                content,
                lang,
                hash,
            } => {
                code_block_counter += 1;
                render_code_block(ui, content, lang, *hash, code_block_counter, theme);
            }
            MarkdownItem::Table { rows } => {
                table_counter += 1;
                render_table(ui, rows, table_counter, theme);
            }
            MarkdownItem::Space(height) => {
                ui.add_space(*height);
            }
        }
    }
}

fn render_text_item(
    ui: &mut egui::Ui,
    job: &Arc<egui::text::LayoutJob>,
    link: Option<String>,
    decoration: &ItemDecoration,
    theme: Theme,
) {
    let render_job = |ui: &mut egui::Ui, job: &egui::text::LayoutJob, link: Option<String>| {
        let available_width = quantize_width(ui.available_width()).max(32.0);
        let mut job = job.clone();
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

    match decoration {
        ItemDecoration::None => {
            render_job(ui, job, link);
        }
        ItemDecoration::Bullet(bullet) => {
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.label(
                    typography::body(bullet)
                        .color(theme.text_secondary)
                        .size(15.0),
                );
                render_job(ui, job, link);
            });
        }
        ItemDecoration::Check(checked, color) => {
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                let icon = if *checked {
                    icons::CHECK_SQUARE
                } else {
                    icons::SQUARE
                };
                ui.label(typography::body(icon).color(*color).size(16.0));
                render_job(ui, job, link);
            });
        }
        ItemDecoration::BlockQuote => {
            let available_width = ui.available_width();
            let text_max_w = quantize_width((available_width - 32.0).max(32.0)).max(32.0);

            let mut job = job.as_ref().clone();
            job.wrap.max_width = text_max_w;

            egui::Frame::NONE
                .fill(theme.bg_secondary.gamma_multiply(0.3))
                .stroke(egui::Stroke::new(1.0, theme.border_secondary))
                .corner_radius(crate::ui::spacing::RADIUS_MD)
                .inner_margin(egui::Margin::same(spacing::SPACING_SM as i8))
                .show(ui, |ui| {
                    let rect = ui.available_rect_before_wrap();
                    ui.painter().line_segment(
                        [
                            rect.left_top().add(egui::vec2(-12.0, 0.0)),
                            rect.left_bottom().add(egui::vec2(-12.0, 0.0)),
                        ],
                        egui::Stroke::new(2.0, theme.brand),
                    );
                    render_job(ui, &job, link);
                });
        }
    }

    ui.add_space(2.0);
}

fn render_code_block(
    ui: &mut egui::Ui,
    content: &str,
    lang: &str,
    content_hash: u64,
    counter: usize,
    theme: Theme,
) {
    if content.trim().is_empty() {
        return;
    }

    let theme_hash = egui::util::hash((
        theme.bg_tertiary,
        theme.text_primary,
        theme.text_secondary,
        theme.brand,
    ));

    let cache_key = code_cache_key(content_hash, lang, theme_hash);

    let job: Arc<egui::text::LayoutJob> = ui.ctx().memory_mut(|mem| {
        let cache = mem
            .data
            .get_temp_mut_or_default::<CodeHighlightCache>(code_cache_id());

        cache.get_or_insert_with(cache_key, || {
            // Only highlight if content is reasonably sized to avoid runaway highlighting
            // Very large blocks (>50KB) fall back to plain text to prevent rendering hangs
            // Note: This trades off syntax highlighting for stability on massive code blocks
            if content.len() > CODE_BLOCK_SIZE_LIMIT {
                let mono = egui::FontId::monospace(13.0);
                let mut job = egui::text::LayoutJob::default();
                job.wrap.max_width = f32::INFINITY;
                for line in content.lines() {
                    job.append(
                        line,
                        0.0,
                        egui::TextFormat {
                            font_id: mono.clone(),
                            color: theme.text_primary,
                            ..Default::default()
                        },
                    );
                }
                return Arc::new(job);
            }

            let syntax = SYNTAX_SET
                .find_syntax_by_token(lang)
                .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());

            let mut h = HighlightLines::new(syntax, &THEME_SET.themes["base16-ocean.dark"]);

            let mono = egui::FontId::monospace(13.0);
            let mut job = egui::text::LayoutJob::default();
            job.wrap.max_width = f32::INFINITY;

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
        .fill(theme.bg_tertiary)
        .corner_radius(crate::ui::spacing::RADIUS_MD)
        .inner_margin(egui::Margin::same(spacing::SPACING_MD as i8))
        .show(ui, |ui| {
            // Use Extend mode for horizontal scroll (no wrapping)
            // This preserves layout for wide code blocks and diagrams
            egui::ScrollArea::horizontal()
                .id_salt(ui.id().with("code_h_scroll").with(counter))
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.add(egui::Label::new(job.clone()).wrap_mode(egui::TextWrapMode::Extend));
                });
        });

    ui.add_space(spacing::SPACING_SM);
}

fn render_table(
    ui: &mut egui::Ui,
    rows: &[Vec<Arc<egui::text::LayoutJob>>],
    counter: usize,
    theme: Theme,
) {
    if rows.is_empty() {
        return;
    }

    egui::Frame::NONE
        .fill(theme.bg_tertiary.gamma_multiply(0.3))
        .stroke(egui::Stroke::new(1.0, theme.border_secondary))
        .corner_radius(crate::ui::spacing::RADIUS_MD)
        .inner_margin(egui::Margin::same(spacing::SPACING_SM as i8))
        .show(ui, |ui| {
            let num_cols = rows[0].len();
            egui::Grid::new(ui.id().with("table_grid").with(counter))
                .num_columns(num_cols)
                .spacing([12.0, 8.0])
                .striped(true)
                .show(ui, |ui| {
                    for row in rows {
                        for cell in row {
                            ui.label((*cell).clone());
                        }
                        ui.end_row();
                    }
                });
        });
    ui.add_space(spacing::SPACING_SM);
}

fn parse_markdown(text: &str, theme: Theme) -> Vec<MarkdownItem> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(text, options);

    let mut state = MarkdownState::new(theme);

    for event in parser {
        match event {
            Event::Start(tag) => state.start_tag(tag),
            Event::End(tag) => state.end_tag(tag),
            Event::Text(content) => state.text(&content),
            Event::Code(content) => state.inline_code(&content),
            Event::SoftBreak => state.soft_break(),
            Event::HardBreak => state.hard_break(),
            Event::TaskListMarker(checked) => state.task_list_marker(checked),
            _ => {}
        }
    }

    state.complete()
}

struct MarkdownState {
    theme: Theme,
    items: Vec<MarkdownItem>,
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
            items: Vec::new(),
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

    fn complete(mut self) -> Vec<MarkdownItem> {
        self.flush();
        self.items
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

        let family = if self.is_bold {
            crate::ui::typography::geist_bold()
        } else if self.is_italic {
            crate::ui::typography::geist_italic()
        } else {
            egui::FontFamily::Proportional
        };

        egui::TextFormat {
            font_id: egui::FontId::new(size, family),
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

    fn start_tag(&mut self, tag: Tag) {
        match tag {
            Tag::Paragraph => {
                self.in_paragraph = true;
                self.job = egui::text::LayoutJob::default();
            }
            Tag::Heading { level, .. } => {
                self.flush();
                // We handle spacing in render now
                self.items.push(MarkdownItem::Space(spacing::SPACING_SM));
                self.in_heading = Some(level as u32);
                self.job = egui::text::LayoutJob::default();
            }
            Tag::Strong => self.is_bold = true,
            Tag::Emphasis => self.is_italic = true,
            Tag::Strikethrough => self.is_strikethrough = true,
            Tag::List(first) => {
                self.flush();
                self.list_stack.push(first);
            }
            Tag::Item => {
                self.flush();
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
                self.flush();
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
                self.flush();
                self.in_blockquote = true;
            }
            Tag::Table(_) => {
                self.flush();
                self.in_table = true;
                self.table_rows.clear();
            }
            Tag::TableHead | Tag::TableRow => {
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
                self.flush();
                self.job = egui::text::LayoutJob::default();
                self.job
                    .append(&format!("{}: ", label), 0.0, self.current_format());
            }
            _ => {}
        }
    }

    fn end_tag(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph => {
                self.flush();
                self.in_paragraph = false;
            }
            TagEnd::Heading(_) => {
                self.flush();
                self.in_heading = None;
                self.items.push(MarkdownItem::Space(spacing::SPACING_XS));
            }
            TagEnd::Strong => self.is_bold = false,
            TagEnd::Emphasis => self.is_italic = false,
            TagEnd::Strikethrough => self.is_strikethrough = false,
            TagEnd::List(_) => {
                self.list_stack.pop();
            }
            TagEnd::Item => {
                self.flush();
                self.in_paragraph = false;
            }
            TagEnd::CodeBlock => {
                self.in_code_block = false;
                self.items.push(MarkdownItem::CodeBlock {
                    content: self.code_block_content.clone(),
                    lang: self.code_block_lang.clone(),
                    hash: self.code_block_hash,
                });
            }
            TagEnd::BlockQuote(_) => {
                self.flush();
                self.in_blockquote = false;
            }
            TagEnd::Table => {
                self.in_table = false;
                self.items.push(MarkdownItem::Table {
                    rows: std::mem::take(&mut self.table_rows),
                });
            }
            TagEnd::TableHead | TagEnd::TableRow => {
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
                self.flush();
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

    fn hard_break(&mut self) {
        if self.in_code_block {
            self.code_block_content.push('\n');
            self.code_block_hash = fnv1a_update(self.code_block_hash, b"\n");
        } else {
            self.flush();
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

    fn flush(&mut self) {
        if self.job.sections.is_empty() {
            return;
        }

        let job = Arc::new(std::mem::take(&mut self.job));
        let link = self.job_has_link.take();

        if self.in_table {
            // Table cells are handled in end_tag
            self.job = egui::text::LayoutJob::default();
            // If we were building up a job for a cell, we need to put it back or handle it differently
            // Actually, in `start_tag(TableCell)`, we reset job. In `end_tag(TableCell)`, we push it.
            // But if flush is called *during* a cell (e.g. hard break), we might split it?
            // Markdown tables usually don't support hard breaks or complex blocks inside cells.
            // For now, let's assume flush inside table is only happening at end of inline content
            // which should be fine if we are accumulating into one job per cell.
            // Wait, standard `flush` logic pushes to `self.items`. We don't want that for tables.

            // Revert the take if we are in table, because `end_tag` will handle it.
            self.job = Arc::try_unwrap(job).unwrap_or_else(|arc| (*arc).clone());
            return;
        }

        let decoration = if let Some(bullet) = self.pending_bullet.take() {
            ItemDecoration::Bullet(bullet)
        } else if let Some((checked, color)) = self.pending_check.take() {
            ItemDecoration::Check(checked, color)
        } else if self.in_blockquote {
            ItemDecoration::BlockQuote
        } else {
            ItemDecoration::None
        };

        self.items.push(MarkdownItem::Text {
            job,
            link,
            decoration,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    #[test]
    fn test_markdown_parsing_basics() {
        let theme = current_theme();
        let items = parse_markdown("# Heading\n\n**Bold** and *italic* and `code`.", theme);

        // Space, Text(Heading), Space, Text(Paragraph)
        assert!(items.len() >= 4);
    }

    #[test]
    fn test_markdown_rendering() {
        let mut harness = Harness::new(|ctx| {
            crate::ui::app::LaReviewApp::setup_fonts(ctx);
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.style_mut().override_font_id = Some(egui::FontId::proportional(12.0));
                render_markdown(ui, "# Title\n\n**Bold** and *italic* and ~~strikethrough~~ and [link](https://example.com).\n\n> Blockquote\n\n- Bullet 1\n- [ ] Task 1\n\n```rust\nfn main() {}\n```");
            });
        });
        harness.run_steps(5);

        harness.get_by_label("Title");
        harness
            .get_all_by_role(egui::accesskit::Role::Label)
            .into_iter()
            .find(|n| format!("{:?}", n).contains("Bold and italic"))
            .expect("Paragraph not found");
        harness.get_by_label("Blockquote");
        assert!(harness.get_all_by_label("•").count() >= 1);
        harness.get_by_label("Task 1");
        harness
            .get_all_by_role(egui::accesskit::Role::Label)
            .into_iter()
            .find(|n| format!("{:?}", n).contains("fn main() {}"))
            .expect("Code block not found");
    }

    #[test]
    fn test_markdown_table_rendering() {
        let mut harness = Harness::new(|ctx| {
            crate::ui::app::LaReviewApp::setup_fonts(ctx);
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.style_mut().override_font_id = Some(egui::FontId::proportional(12.0));
                render_markdown(ui, "| A | B |\n|---|---|\n| 1 | 2 |");
            });
        });
        harness.run_steps(5);
        harness.get_by_label("A");
        harness.get_by_label("1");
    }
}
