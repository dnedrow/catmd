//! Markdown AST to terminal text.

pub mod highlight;
pub mod inline;
pub mod layout;
pub mod table;

use anstyle::Style;
use comrak::nodes::{ListType, Node, NodeAlert, NodeCodeBlock, NodeList, NodeValue};
use comrak::{Arena, Options, parse_document};

use crate::Settings;
use crate::theme::{Glyphs, Theme};
use layout::{Span, Writer};

/// Indentation applied to every line of a code block.
const CODE_INDENT: &str = "    ";

/// Render a Markdown document to terminal text according to `settings`.
///
/// The result is empty for an empty document and otherwise ends with exactly
/// one newline.
pub fn render(markdown: &str, settings: &Settings) -> String {
    let arena = Arena::new();
    let options = options();
    let root = parse_document(&arena, markdown, &options);
    let mut renderer = Renderer {
        theme: &settings.theme,
        glyphs: settings.glyphs,
        writer: Writer::new(settings.width, settings.depth),
        footnotes: 0,
    };
    renderer.block(root);
    renderer.writer.finish()
}

/// comrak options: CommonMark plus the GitHub extensions, front matter split off.
pub fn options() -> Options<'static> {
    let mut options = Options::default();
    options.extension.table = true;
    options.extension.strikethrough = true;
    options.extension.tasklist = true;
    options.extension.autolink = true;
    options.extension.footnotes = true;
    options.extension.alerts = true;
    options.extension.front_matter_delimiter = Some("---".to_string());
    options
}

struct Renderer<'t> {
    theme: &'t Theme,
    glyphs: Glyphs,
    writer: Writer,
    /// Footnote definitions rendered so far; comrak appends them to the
    /// document in order of first reference, so this is also their number.
    footnotes: usize,
}

impl Renderer<'_> {
    /// Render every child block of `node`, separated by blank lines when `separate` is set.
    fn blocks(&mut self, node: Node<'_>, separate: bool) {
        for (index, child) in node.children().enumerate() {
            if separate && index > 0 {
                self.writer.blank_line();
            }
            self.block(child);
        }
    }

    fn block(&mut self, node: Node<'_>) {
        let data = node.data();
        match &data.value {
            NodeValue::Document => self.blocks(node, true),
            NodeValue::FrontMatter(_) => {}
            NodeValue::Paragraph => {
                let spans = inline::render_inlines(node, self.theme, Style::new());
                self.writer.wrap(&spans);
            }
            NodeValue::Heading(heading) => self.heading(node, heading.level),
            NodeValue::ThematicBreak => self.rule(),
            NodeValue::HtmlBlock(html) => self.html_block(&html.literal),
            NodeValue::List(list) => self.list(node, list),
            NodeValue::BlockQuote => self.quote(node),
            NodeValue::Alert(alert) => self.alert(node, alert),
            NodeValue::CodeBlock(code) => self.code_block(code),
            NodeValue::Table(table) => table::render_table(self, node, table),
            NodeValue::FootnoteDefinition(_) => self.footnote_definition(node),
            _ => self.blocks(node, true),
        }
    }

    fn footnote_definition(&mut self, node: Node<'_>) {
        if self.footnotes == 0 {
            self.rule();
            self.writer.blank_line();
        }
        self.footnotes += 1;
        let marker = format!("[{}] ", self.footnotes);
        let rest = " ".repeat(marker.len());
        self.writer.push_prefix(
            vec![Span::new(marker, self.theme.footnote_marker)],
            vec![Span::plain(rest)],
        );
        self.blocks(node, true);
        self.writer.pop_prefix();
    }

    fn gutter(&self) -> Vec<Span> {
        vec![Span::new(
            format!("{} ", self.glyphs.quote()),
            self.theme.quote_gutter,
        )]
    }

    fn quote(&mut self, node: Node<'_>) {
        self.writer.push_uniform_prefix(self.gutter());
        self.blocks(node, true);
        self.writer.pop_prefix();
    }

    fn alert(&mut self, node: Node<'_>, alert: &NodeAlert) {
        let title = alert
            .title
            .clone()
            .unwrap_or_else(|| alert.alert_type.default_title().to_string());
        self.writer.push_uniform_prefix(self.gutter());
        self.writer
            .line(&[Span::new(title, self.theme.alert(alert.alert_type))]);
        self.blocks(node, true);
        self.writer.pop_prefix();
    }

    fn code_block(&mut self, code: &NodeCodeBlock) {
        self.writer
            .push_uniform_prefix(vec![Span::plain(CODE_INDENT)]);
        // Only a fenced block names a language, and highlighting is pure
        // styling, so the grammar set is never loaded when styling is off.
        let language = if code.fenced && self.writer.color() {
            highlight::language(&code.info)
        } else {
            None
        };
        match language {
            Some(language) => {
                for spans in highlight::lines(language, &code.literal, self.theme) {
                    self.writer.raw_line(&spans);
                }
            }
            None => {
                for line in code.literal.lines() {
                    self.writer
                        .raw_line(&[Span::new(line, self.theme.code_block)]);
                }
            }
        }
        self.writer.pop_prefix();
    }

    fn list(&mut self, node: Node<'_>, list: &NodeList) {
        let ordered = list.list_type == ListType::Ordered;
        let item_count = node.children().count();
        let last_number = list.start + item_count.saturating_sub(1);
        let marker_width = format!("{last_number}.").len();

        for (index, item) in node.children().enumerate() {
            if index > 0 && !list.tight {
                self.writer.blank_line();
            }
            let (first, rest) = match &item.data().value {
                NodeValue::TaskItem(task) => {
                    let marker = if task.symbol.is_some() {
                        "[x] "
                    } else {
                        "[ ] "
                    };
                    (
                        Span::new(marker, self.theme.task_checkbox),
                        Span::plain("    "),
                    )
                }
                _ if ordered => {
                    let number = format!("{}.", list.start + index);
                    (
                        Span::new(format!("{number:>marker_width$} "), self.theme.list_number),
                        Span::plain(" ".repeat(marker_width + 1)),
                    )
                }
                _ => (
                    Span::new(format!("{} ", self.glyphs.bullet()), self.theme.bullet),
                    Span::plain("  "),
                ),
            };
            self.writer.push_prefix(vec![first], vec![rest]);
            if item.first_child().is_none() {
                self.writer.line(&[]);
            } else {
                self.blocks(item, !list.tight);
            }
            self.writer.pop_prefix();
        }
    }

    fn heading(&mut self, node: Node<'_>, level: u8) {
        let mut spans = vec![
            Span::new("#".repeat(usize::from(level)), self.theme.heading_marker),
            Span::plain(" "),
        ];
        spans.extend(inline::render_inlines(
            node,
            self.theme,
            self.theme.heading_level(level),
        ));
        self.writer.wrap(&spans);
    }

    fn rule(&mut self) {
        let width = self.writer.available_width();
        let rule = self.glyphs.rule().to_string().repeat(width);
        self.writer.line(&[Span::new(rule, self.theme.rule)]);
    }

    fn html_block(&mut self, literal: &str) {
        for line in literal.lines() {
            self.writer.line(&[Span::new(line, self.theme.html)]);
        }
    }
}
