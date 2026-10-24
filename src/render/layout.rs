//! Prefix stack, wrapping, and line emission.
//!
//! The [`Writer`] is the only place that knows about the output width and
//! whether color is on. Blocks hand it styled [`Span`]s; it applies the active
//! prefixes (list markers, quote gutters, code indentation), wraps where asked,
//! trims trailing whitespace, and emits each span with its style opened and
//! closed on the same line so attributes never bleed across line breaks.

use anstyle::Style;
use unicode_width::UnicodeWidthStr;

use crate::theme::color::{ColorDepth, clamp};

/// A run of text with one style.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    pub style: Style,
}

impl Span {
    pub fn new(text: impl Into<String>, style: Style) -> Self {
        Self {
            text: text.into(),
            style,
        }
    }

    pub fn plain(text: impl Into<String>) -> Self {
        Self::new(text, Style::new())
    }

    /// Display width in terminal columns.
    pub fn width(&self) -> usize {
        self.text.width()
    }
}

/// Text placed in a [`Span`] to force a line break inside wrapped content.
pub const HARD_BREAK: &str = "\n";

/// Sum of the display widths of `spans`.
pub fn spans_width(spans: &[Span]) -> usize {
    spans.iter().map(Span::width).sum()
}

/// Concatenated text of `spans`, ignoring styles.
pub fn spans_text(spans: &[Span]) -> String {
    spans.iter().map(|s| s.text.as_str()).collect()
}

struct Prefix {
    first: Vec<Span>,
    rest: Vec<Span>,
    used: bool,
}

/// Accumulates output lines under a stack of prefixes.
pub struct Writer {
    out: String,
    width: usize,
    depth: ColorDepth,
    prefixes: Vec<Prefix>,
    /// A rendered blank line (prefixes only) to emit before the next line, if any.
    pending_blank: Option<String>,
}

impl Writer {
    pub fn new(width: usize, depth: ColorDepth) -> Self {
        Self {
            out: String::new(),
            width: width.max(1),
            depth,
            prefixes: Vec::new(),
            pending_blank: None,
        }
    }

    /// Total rendering width.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Whether styling is emitted.
    pub fn color(&self) -> bool {
        self.depth.is_on()
    }

    /// Columns left for content after the active prefixes.
    pub fn available_width(&self) -> usize {
        self.width.saturating_sub(self.prefix_width()).max(1)
    }

    /// Push a prefix whose `first` form is used for the next emitted line and
    /// whose `rest` form is used for every line after that.
    pub fn push_prefix(&mut self, first: Vec<Span>, rest: Vec<Span>) {
        self.prefixes.push(Prefix {
            first,
            rest,
            used: false,
        });
    }

    /// Push a prefix that is the same on every line.
    pub fn push_uniform_prefix(&mut self, spans: Vec<Span>) {
        self.push_prefix(spans.clone(), spans);
    }

    pub fn pop_prefix(&mut self) {
        self.prefixes.pop();
    }

    /// Request a blank line before the next emitted line. Suppressed at the very
    /// start of the output; repeated requests collapse into one blank line; a
    /// request that nothing follows is dropped.
    pub fn blank_line(&mut self) {
        if self.out.is_empty() {
            return;
        }
        let spans: Vec<Span> = self.prefixes.iter().flat_map(|p| p.rest.clone()).collect();
        self.pending_blank = Some(self.render_line(spans, true));
    }

    /// Emit one line of content with trailing spaces trimmed.
    pub fn line(&mut self, content: &[Span]) {
        self.emit(content, true);
    }

    /// Emit one line of verbatim content (code). A whitespace-only line is
    /// emitted as an empty line so indentation never leaves trailing spaces.
    pub fn raw_line(&mut self, content: &[Span]) {
        let blank = content.iter().all(|s| s.text.trim().is_empty());
        self.emit(content, blank);
    }

    /// Emit one line whose text after the prefixes is `cells`, which may hold
    /// escape sequences and is neither measured nor trimmed.
    pub fn raw_cells(&mut self, cells: &str) {
        self.emit(&[Span::plain(cells)], cells.is_empty());
    }

    /// Columns taken by the prefix of the next emitted line.
    pub fn indent_width(&self) -> usize {
        self.prefix_width()
    }

    /// Word-wrap `content` to the available width and emit the resulting lines.
    pub fn wrap(&mut self, content: &[Span]) {
        let available = self.available_width();
        for line in wrap_spans(content, available) {
            self.line(&line);
        }
    }

    /// Finish and return the output, which is either empty or ends with exactly one newline.
    pub fn finish(self) -> String {
        self.out
    }

    fn prefix_width(&self) -> usize {
        self.prefixes
            .iter()
            .map(|p| spans_width(if p.used { &p.rest } else { &p.first }))
            .sum()
    }

    fn emit(&mut self, content: &[Span], trim: bool) {
        if let Some(blank) = self.pending_blank.take() {
            self.out.push_str(&blank);
            self.out.push('\n');
        }
        let mut spans = Vec::with_capacity(self.prefixes.len() * 2 + content.len());
        for prefix in &mut self.prefixes {
            let active = if prefix.used {
                &prefix.rest
            } else {
                &prefix.first
            };
            spans.extend(active.iter().cloned());
            prefix.used = true;
        }
        spans.extend_from_slice(content);
        let rendered = self.render_line(spans, trim);
        self.out.push_str(&rendered);
        self.out.push('\n');
    }

    fn render_line(&self, mut spans: Vec<Span>, trim: bool) -> String {
        if trim {
            trim_trailing_spaces(&mut spans);
        }
        let spans = coalesce(spans);
        let mut line = String::new();
        for span in &spans {
            if self.depth.is_on() && span.style != Style::new() {
                let style = match span.style.get_fg_color() {
                    Some(fg) => span.style.fg_color(Some(clamp(fg, self.depth))),
                    None => span.style,
                };
                line.push_str(&style.render().to_string());
                line.push_str(&span.text);
                line.push_str(&style.render_reset().to_string());
            } else {
                line.push_str(&span.text);
            }
        }
        line
    }
}

fn trim_trailing_spaces(spans: &mut Vec<Span>) {
    while let Some(last) = spans.last_mut() {
        let trimmed = last.text.trim_end_matches([' ', '\t']);
        if trimmed.is_empty() {
            spans.pop();
        } else {
            if trimmed.len() != last.text.len() {
                last.text.truncate(trimmed.len());
            }
            break;
        }
    }
}

/// Merge adjacent spans that share a style and drop empty ones.
pub(crate) fn coalesce(spans: Vec<Span>) -> Vec<Span> {
    let mut merged: Vec<Span> = Vec::with_capacity(spans.len());
    for span in spans {
        if span.text.is_empty() {
            continue;
        }
        match merged.last_mut() {
            Some(last) if last.style == span.style => last.text.push_str(&span.text),
            _ => merged.push(span),
        }
    }
    merged
}

enum Token {
    Word {
        fragments: Vec<Span>,
        width: usize,
    },
    /// A breakable gap one column wide, carrying the style of the space it came from.
    Space(Style),
    /// A breakable gap of zero width (around East Asian wide characters).
    Opportunity,
    Break,
}

fn tokenize(content: &[Span]) -> Vec<Token> {
    use unicode_width::UnicodeWidthChar;

    let mut tokens = Vec::new();
    let mut fragments: Vec<Span> = Vec::new();
    let flush_word = |fragments: &mut Vec<Span>, tokens: &mut Vec<Token>| {
        if !fragments.is_empty() {
            let width = spans_width(fragments);
            tokens.push(Token::Word {
                fragments: std::mem::take(fragments),
                width,
            });
        }
    };
    for span in content {
        let mut buffer = String::new();
        let flush_buffer = |buffer: &mut String, fragments: &mut Vec<Span>| {
            if !buffer.is_empty() {
                fragments.push(Span::new(std::mem::take(buffer), span.style));
            }
        };
        for ch in span.text.chars() {
            match ch {
                ' ' | '\t' => {
                    flush_buffer(&mut buffer, &mut fragments);
                    flush_word(&mut fragments, &mut tokens);
                    tokens.push(Token::Space(span.style));
                }
                '\n' => {
                    flush_buffer(&mut buffer, &mut fragments);
                    flush_word(&mut fragments, &mut tokens);
                    tokens.push(Token::Break);
                }
                wide if wide.width() == Some(2) => {
                    flush_buffer(&mut buffer, &mut fragments);
                    flush_word(&mut fragments, &mut tokens);
                    tokens.push(Token::Opportunity);
                    tokens.push(Token::Word {
                        fragments: vec![Span::new(wide.to_string(), span.style)],
                        width: 2,
                    });
                    tokens.push(Token::Opportunity);
                }
                _ => buffer.push(ch),
            }
        }
        flush_buffer(&mut buffer, &mut fragments);
    }
    flush_word(&mut fragments, &mut tokens);
    tokens
}

/// A pending breakable gap between the current line and the next word.
#[derive(Clone, Copy)]
enum Gap {
    Zero,
    Space(Style),
}

impl Gap {
    fn width(self) -> usize {
        match self {
            Gap::Zero => 0,
            Gap::Space(_) => 1,
        }
    }
}

/// Greedily fill lines of at most `available` columns with the words in
/// `content`. Words are never split; a word wider than `available` is placed on
/// its own line and overflows. East Asian wide characters may be broken between,
/// and a [`HARD_BREAK`] forces a new line.
pub fn wrap_spans(content: &[Span], available: usize) -> Vec<Vec<Span>> {
    let mut lines: Vec<Vec<Span>> = Vec::new();
    let mut current: Vec<Span> = Vec::new();
    let mut current_width = 0;
    let mut pending: Option<Gap> = None;

    for token in tokenize(content) {
        match token {
            Token::Space(style) => {
                if !current.is_empty() && !matches!(pending, Some(Gap::Space(_))) {
                    pending = Some(Gap::Space(style));
                }
            }
            Token::Opportunity => {
                if !current.is_empty() && pending.is_none() {
                    pending = Some(Gap::Zero);
                }
            }
            Token::Break => {
                lines.push(std::mem::take(&mut current));
                current_width = 0;
                pending = None;
            }
            Token::Word { fragments, width } => {
                if current.is_empty() {
                    current = fragments;
                    current_width = width;
                } else {
                    let gap = pending.unwrap_or(Gap::Zero);
                    if current_width + gap.width() + width <= available {
                        if let Gap::Space(style) = gap {
                            current.push(Span::new(" ", style));
                        }
                        current.extend(fragments);
                        current_width += gap.width() + width;
                    } else {
                        lines.push(std::mem::take(&mut current));
                        current = fragments;
                        current_width = width;
                    }
                }
                pending = None;
            }
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// Remove ANSI CSI escape sequences (the kind `anstyle` emits) from `text`.
pub fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' && chars.peek() == Some(&'[') {
            chars.next();
            for c in chars.by_ref() {
                if ('\x40'..='\x7e').contains(&c) {
                    break;
                }
            }
        } else {
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bold() -> Style {
        Style::new().bold()
    }

    fn plain_writer(width: usize) -> Writer {
        Writer::new(width, ColorDepth::Off)
    }

    #[test]
    fn blocks_are_separated_by_one_blank_line_and_output_ends_with_newline() {
        let mut w = plain_writer(80);
        w.blank_line();
        w.line(&[Span::plain("# Title")]);
        w.blank_line();
        w.blank_line();
        w.line(&[Span::plain("Body text.")]);
        w.blank_line();
        assert_eq!(w.finish(), "# Title\n\nBody text.\n");
    }

    #[test]
    fn empty_writer_produces_empty_output() {
        let mut w = plain_writer(80);
        w.blank_line();
        assert_eq!(w.finish(), "");
    }

    #[test]
    fn prefix_switches_from_first_to_rest_after_the_first_line() {
        let mut w = plain_writer(12);
        w.push_prefix(vec![Span::plain("* ")], vec![Span::plain("  ")]);
        w.wrap(&[Span::plain("alpha beta gamma delta")]);
        w.pop_prefix();
        assert_eq!(w.finish(), "* alpha beta\n  gamma\n  delta\n");
    }

    #[test]
    fn blank_lines_inside_a_prefix_carry_the_rest_prefix_trimmed() {
        let mut w = plain_writer(40);
        w.push_uniform_prefix(vec![Span::plain("| ")]);
        w.line(&[Span::plain("one")]);
        w.blank_line();
        w.line(&[Span::plain("two")]);
        w.pop_prefix();
        assert_eq!(w.finish(), "| one\n|\n| two\n");
    }

    #[test]
    fn blank_line_requested_before_a_prefix_is_pushed_has_no_prefix() {
        let mut w = plain_writer(40);
        w.line(&[Span::plain("para")]);
        w.blank_line();
        w.push_prefix(vec![Span::plain("* ")], vec![Span::plain("  ")]);
        w.line(&[Span::plain("item")]);
        w.pop_prefix();
        assert_eq!(w.finish(), "para\n\n* item\n");
    }

    #[test]
    fn trailing_spaces_are_trimmed_on_normal_lines() {
        let mut w = plain_writer(40);
        w.line(&[Span::plain("text   "), Span::new("  ", bold())]);
        w.push_uniform_prefix(vec![Span::plain("    ")]);
        w.raw_line(&[Span::plain("   ")]);
        w.raw_line(&[Span::plain("code  ")]);
        w.pop_prefix();
        assert_eq!(w.finish(), "text\n\n    code  \n");
    }

    #[test]
    fn color_output_stripped_of_escapes_equals_plain_output() {
        let content = [
            Span::new("Bold", bold()),
            Span::plain(" and "),
            Span::new("under", Style::new().underline()),
            Span::plain(" text that wraps"),
        ];
        let mut plain = plain_writer(16);
        plain.wrap(&content);
        let mut color = Writer::new(16, ColorDepth::Ansi16);
        color.wrap(&content);
        let colored = color.finish();
        assert!(colored.contains("\x1b[1m"), "{colored:?}");
        assert_eq!(strip_ansi(&colored), plain.finish());
    }

    #[test]
    fn unstyled_spans_emit_no_escapes_even_in_color_mode() {
        let mut w = Writer::new(40, ColorDepth::Ansi16);
        w.line(&[Span::plain("plain")]);
        assert_eq!(w.finish(), "plain\n");
    }

    #[test]
    fn long_paragraph_wraps_without_exceeding_width_or_splitting_words() {
        let words: Vec<String> = (0..40).map(|i| format!("word{i}")).collect();
        let text = words.join(" ");
        assert!(text.len() >= 200);
        let mut w = plain_writer(40);
        w.wrap(&[Span::plain(text.clone())]);
        let out = w.finish();
        let lines: Vec<&str> = out.lines().collect();
        assert!(lines.len() > 1);
        for line in &lines {
            assert!(line.width() <= 40, "{line:?} is too wide");
            assert!(!line.ends_with(' '));
        }
        let rejoined = lines.join(" ");
        assert_eq!(rejoined, text);
    }

    #[test]
    fn wide_characters_count_as_two_columns() {
        let text: String = std::iter::repeat_n('漢', 30).collect();
        let mut w = plain_writer(40);
        w.wrap(&[Span::plain(text)]);
        let out = w.finish();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 2, "{out:?}");
        assert_eq!(lines[0].chars().count(), 20);
        assert_eq!(lines[0].width(), 40);
        assert_eq!(lines[1].chars().count(), 10);
    }

    #[test]
    fn wide_characters_mixed_with_words_keep_their_spaces() {
        let mut w = plain_writer(80);
        w.wrap(&[Span::plain("日本語 text 漢字")]);
        assert_eq!(w.finish(), "日本語 text 漢字\n");
    }

    #[test]
    fn overlong_word_is_placed_on_its_own_line_unbroken() {
        let url = "https://example.com/a/very/long/path/that/exceeds/the/width";
        let mut w = plain_writer(30);
        w.wrap(&[Span::plain(format!("see {url} now"))]);
        let out = w.finish();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines, vec!["see", url, "now"]);
    }

    #[test]
    fn hard_break_forces_a_new_line() {
        let mut w = plain_writer(80);
        w.wrap(&[
            Span::plain("roses"),
            Span::plain(HARD_BREAK),
            Span::plain("violets"),
        ]);
        assert_eq!(w.finish(), "roses\nviolets\n");
    }

    #[test]
    fn styles_are_opened_and_closed_on_every_wrapped_line() {
        let mut w = Writer::new(20, ColorDepth::Ansi16);
        w.wrap(&[Span::new("bold words that wrap onto another line", bold())]);
        let out = w.finish();
        let lines: Vec<&str> = out.lines().collect();
        assert!(lines.len() >= 2, "{out:?}");
        for line in lines {
            assert!(line.starts_with("\x1b[1m"), "{line:?}");
            assert!(line.ends_with("\x1b[0m"), "{line:?}");
        }
    }

    #[test]
    fn spaces_inside_a_styled_span_keep_the_style_and_words_keep_fragments() {
        let mut w = Writer::new(80, ColorDepth::Ansi16);
        w.wrap(&[
            Span::new("bo", bold()),
            Span::plain("ld link"),
            Span::new("under lined", Style::new().underline()),
        ]);
        let out = w.finish();
        assert_eq!(out, "\x1b[1mbo\x1b[0mld link\x1b[4munder lined\x1b[0m\n");
    }

    #[test]
    fn repeated_and_edge_spaces_collapse() {
        let mut w = plain_writer(80);
        w.wrap(&[Span::plain("  a   b  "), Span::plain("\tc ")]);
        assert_eq!(w.finish(), "a b c\n");
    }

    #[test]
    fn available_width_accounts_for_prefixes() {
        let mut w = plain_writer(20);
        assert_eq!(w.available_width(), 20);
        w.push_uniform_prefix(vec![Span::plain("| ")]);
        w.push_prefix(vec![Span::plain("10. ")], vec![Span::plain("    ")]);
        assert_eq!(w.available_width(), 14);
    }

    #[test]
    fn emitter_clamps_colors_to_the_depth() {
        let hex = Style::new().fg_color(Some(anstyle::RgbColor(255, 0, 0).into()));
        let mut w = Writer::new(80, ColorDepth::TrueColor);
        w.line(&[Span::new("x", hex)]);
        assert_eq!(w.finish(), "\x1b[38;2;255;0;0mx\x1b[0m\n");
        let mut w = Writer::new(80, ColorDepth::Ansi256);
        w.line(&[Span::new("x", hex)]);
        assert_eq!(w.finish(), "\x1b[38;5;196mx\x1b[0m\n");
        let mut w = Writer::new(80, ColorDepth::Ansi16);
        w.line(&[Span::new("x", hex)]);
        let out = w.finish();
        assert!(
            out == "\x1b[31mx\x1b[0m\n" || out == "\x1b[91mx\x1b[0m\n",
            "{out:?}"
        );
    }

    #[test]
    fn strip_ansi_removes_csi_sequences_only() {
        assert_eq!(strip_ansi("\x1b[1;4mhi\x1b[0m there"), "hi there");
        assert_eq!(strip_ansi("no escapes"), "no escapes");
    }
}
