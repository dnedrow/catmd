//! Styles and glyph sets.
//!
//! The built-in theme uses only text effects and the 16-color ANSI palette
//! ([`AnsiColor`]) so it renders identically on every terminal. Theme files may
//! use richer colors; the emitter clamps them to the terminal's depth.

pub mod color;
pub mod file;

use anstyle::{AnsiColor, Style};
use comrak::nodes::AlertType;

/// Terminal styles per semantic role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    /// The `#` markers in front of heading text.
    pub heading_marker: Style,
    /// Heading text by level; index 0 is level 1.
    pub heading: [Style; 6],
    pub strong: Style,
    pub emph: Style,
    pub strikethrough: Style,
    /// Inline code spans.
    pub code: Style,
    pub link_text: Style,
    pub link_url: Style,
    /// The `[image: alt]` placeholder.
    pub image: Style,
    /// Block quote and alert gutters.
    pub quote_gutter: Style,
    pub alert_note: Style,
    pub alert_tip: Style,
    pub alert_important: Style,
    pub alert_warning: Style,
    pub alert_caution: Style,
    /// Fenced and indented code blocks.
    pub code_block: Style,
    pub table_header: Style,
    /// Thematic breaks and table borders.
    pub rule: Style,
    /// Literal inline and block HTML.
    pub html: Style,
    /// `[n]` footnote references and definition markers.
    pub footnote_marker: Style,
    /// Bullet glyph of unordered list items.
    pub bullet: Style,
    /// Number marker of ordered list items.
    pub list_number: Style,
    /// `[x]` / `[ ]` of task items.
    pub task_checkbox: Style,
    /// Table column separators and the header rule.
    pub table_border: Style,
    /// Keywords and declaration words in highlighted code.
    pub syntax_keyword: Style,
    /// String literals, including their delimiters, in highlighted code.
    pub syntax_string: Style,
    /// Comments, including their markers, in highlighted code.
    pub syntax_comment: Style,
    /// Numeric and language constants in highlighted code.
    pub syntax_constant: Style,
    /// Type names in highlighted code.
    pub syntax_type: Style,
    /// Function names in highlighted code.
    pub syntax_function: Style,
    /// Markup tag names in highlighted code.
    pub syntax_tag: Style,
}

/// A role name that is not in [`Theme::ROLES`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownRole(pub String);

impl std::fmt::Display for UnknownRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown role `{}`", self.0)
    }
}

impl std::error::Error for UnknownRole {}

impl Default for Theme {
    fn default() -> Self {
        let fg = |color: AnsiColor| Style::new().fg_color(Some(color.into()));
        Self {
            heading_marker: Style::new().dimmed(),
            heading: [
                fg(AnsiColor::BrightMagenta).bold().underline(),
                fg(AnsiColor::Magenta).bold(),
                fg(AnsiColor::BrightBlue).bold(),
                Style::new().bold(),
                Style::new().bold(),
                Style::new().bold(),
            ],
            strong: Style::new().bold(),
            emph: Style::new().italic(),
            strikethrough: Style::new().strikethrough(),
            code: fg(AnsiColor::Yellow),
            link_text: fg(AnsiColor::Blue).underline(),
            link_url: Style::new().dimmed(),
            image: fg(AnsiColor::Magenta).dimmed(),
            quote_gutter: fg(AnsiColor::Cyan).dimmed(),
            alert_note: fg(AnsiColor::Blue).bold(),
            alert_tip: fg(AnsiColor::Green).bold(),
            alert_important: fg(AnsiColor::Magenta).bold(),
            alert_warning: fg(AnsiColor::Yellow).bold(),
            alert_caution: fg(AnsiColor::Red).bold(),
            code_block: fg(AnsiColor::Cyan),
            table_header: Style::new().bold(),
            rule: Style::new().dimmed(),
            html: Style::new().dimmed(),
            footnote_marker: fg(AnsiColor::Cyan),
            bullet: Style::new(),
            list_number: Style::new(),
            task_checkbox: Style::new(),
            table_border: Style::new().dimmed(),
            syntax_keyword: fg(AnsiColor::Magenta),
            syntax_string: fg(AnsiColor::Green),
            syntax_comment: Style::new().dimmed(),
            syntax_constant: fg(AnsiColor::Red),
            syntax_type: fg(AnsiColor::Yellow),
            syntax_function: fg(AnsiColor::Blue),
            syntax_tag: fg(AnsiColor::BrightBlue),
        }
    }
}

impl Theme {
    /// Every role a theme file may address, in documentation order.
    pub const ROLES: [&'static str; 36] = [
        "heading_marker",
        "heading1",
        "heading2",
        "heading3",
        "heading4",
        "heading5",
        "heading6",
        "strong",
        "emph",
        "strikethrough",
        "code",
        "code_block",
        "link_text",
        "link_url",
        "image",
        "quote_gutter",
        "alert_note",
        "alert_tip",
        "alert_important",
        "alert_warning",
        "alert_caution",
        "table_header",
        "table_border",
        "rule",
        "html",
        "footnote_marker",
        "bullet",
        "list_number",
        "task_checkbox",
        "syntax_keyword",
        "syntax_string",
        "syntax_comment",
        "syntax_constant",
        "syntax_type",
        "syntax_function",
        "syntax_tag",
    ];

    fn slot(&mut self, role: &str) -> Option<&mut Style> {
        Some(match role {
            "heading_marker" => &mut self.heading_marker,
            "heading1" => &mut self.heading[0],
            "heading2" => &mut self.heading[1],
            "heading3" => &mut self.heading[2],
            "heading4" => &mut self.heading[3],
            "heading5" => &mut self.heading[4],
            "heading6" => &mut self.heading[5],
            "strong" => &mut self.strong,
            "emph" => &mut self.emph,
            "strikethrough" => &mut self.strikethrough,
            "code" => &mut self.code,
            "code_block" => &mut self.code_block,
            "link_text" => &mut self.link_text,
            "link_url" => &mut self.link_url,
            "image" => &mut self.image,
            "quote_gutter" => &mut self.quote_gutter,
            "alert_note" => &mut self.alert_note,
            "alert_tip" => &mut self.alert_tip,
            "alert_important" => &mut self.alert_important,
            "alert_warning" => &mut self.alert_warning,
            "alert_caution" => &mut self.alert_caution,
            "table_header" => &mut self.table_header,
            "table_border" => &mut self.table_border,
            "rule" => &mut self.rule,
            "html" => &mut self.html,
            "footnote_marker" => &mut self.footnote_marker,
            "bullet" => &mut self.bullet,
            "list_number" => &mut self.list_number,
            "task_checkbox" => &mut self.task_checkbox,
            "syntax_keyword" => &mut self.syntax_keyword,
            "syntax_string" => &mut self.syntax_string,
            "syntax_comment" => &mut self.syntax_comment,
            "syntax_constant" => &mut self.syntax_constant,
            "syntax_type" => &mut self.syntax_type,
            "syntax_function" => &mut self.syntax_function,
            "syntax_tag" => &mut self.syntax_tag,
            _ => return None,
        })
    }

    fn slot_ref(&self, role: &str) -> Option<&Style> {
        Some(match role {
            "heading_marker" => &self.heading_marker,
            "heading1" => &self.heading[0],
            "heading2" => &self.heading[1],
            "heading3" => &self.heading[2],
            "heading4" => &self.heading[3],
            "heading5" => &self.heading[4],
            "heading6" => &self.heading[5],
            "strong" => &self.strong,
            "emph" => &self.emph,
            "strikethrough" => &self.strikethrough,
            "code" => &self.code,
            "code_block" => &self.code_block,
            "link_text" => &self.link_text,
            "link_url" => &self.link_url,
            "image" => &self.image,
            "quote_gutter" => &self.quote_gutter,
            "alert_note" => &self.alert_note,
            "alert_tip" => &self.alert_tip,
            "alert_important" => &self.alert_important,
            "alert_warning" => &self.alert_warning,
            "alert_caution" => &self.alert_caution,
            "table_header" => &self.table_header,
            "table_border" => &self.table_border,
            "rule" => &self.rule,
            "html" => &self.html,
            "footnote_marker" => &self.footnote_marker,
            "bullet" => &self.bullet,
            "list_number" => &self.list_number,
            "task_checkbox" => &self.task_checkbox,
            "syntax_keyword" => &self.syntax_keyword,
            "syntax_string" => &self.syntax_string,
            "syntax_comment" => &self.syntax_comment,
            "syntax_constant" => &self.syntax_constant,
            "syntax_type" => &self.syntax_type,
            "syntax_function" => &self.syntax_function,
            "syntax_tag" => &self.syntax_tag,
            _ => return None,
        })
    }

    /// Replace the style of `role`; fails for a name outside [`Theme::ROLES`].
    pub fn set(&mut self, role: &str, style: Style) -> Result<(), UnknownRole> {
        match self.slot(role) {
            Some(slot) => {
                *slot = style;
                Ok(())
            }
            None => Err(UnknownRole(role.to_string())),
        }
    }

    /// The style of `role`, or `None` for a name outside [`Theme::ROLES`].
    pub fn get(&self, role: &str) -> Option<Style> {
        self.slot_ref(role).copied()
    }

    /// Heading text style for a 1-based level; levels beyond 6 use the level-6 style.
    pub fn heading_level(&self, level: u8) -> Style {
        let index = usize::from(level.clamp(1, 6)) - 1;
        self.heading[index]
    }

    /// Title style for a GitHub alert type.
    pub fn alert(&self, alert_type: AlertType) -> Style {
        match alert_type {
            AlertType::Note => self.alert_note,
            AlertType::Tip => self.alert_tip,
            AlertType::Important => self.alert_important,
            AlertType::Warning => self.alert_warning,
            AlertType::Caution => self.alert_caution,
        }
    }

    /// Every style in the theme, for exhaustive checks.
    pub fn all_styles(&self) -> Vec<Style> {
        let mut styles = vec![
            self.heading_marker,
            self.strong,
            self.emph,
            self.strikethrough,
            self.code,
            self.link_text,
            self.link_url,
            self.image,
            self.quote_gutter,
            self.alert_note,
            self.alert_tip,
            self.alert_important,
            self.alert_warning,
            self.alert_caution,
            self.code_block,
            self.table_header,
            self.rule,
            self.html,
            self.footnote_marker,
            self.bullet,
            self.list_number,
            self.task_checkbox,
            self.table_border,
            self.syntax_keyword,
            self.syntax_string,
            self.syntax_comment,
            self.syntax_constant,
            self.syntax_type,
            self.syntax_function,
            self.syntax_tag,
        ];
        styles.extend(self.heading);
        styles
    }
}

/// Glyph set used for bullets, rules, and borders.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Glyphs {
    #[default]
    Unicode,
    Ascii,
}

impl Glyphs {
    /// List bullet.
    pub fn bullet(self) -> char {
        match self {
            Glyphs::Unicode => '•',
            Glyphs::Ascii => '*',
        }
    }

    /// Block quote gutter.
    pub fn quote(self) -> char {
        match self {
            Glyphs::Unicode => '│',
            Glyphs::Ascii => '|',
        }
    }

    /// Thematic break and table rule.
    pub fn rule(self) -> char {
        match self {
            Glyphs::Unicode => '─',
            Glyphs::Ascii => '-',
        }
    }

    /// Table column separator.
    pub fn vertical(self) -> char {
        match self {
            Glyphs::Unicode => '│',
            Glyphs::Ascii => '|',
        }
    }

    /// Intersection of a table rule and a column separator.
    pub fn cross(self) -> char {
        match self {
            Glyphs::Unicode => '┼',
            Glyphs::Ascii => '+',
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_theme_uses_only_the_16_color_palette() {
        // The built-in theme must render identically at every depth, which
        // holds because ANSI colors are never converted.
        for style in Theme::default().all_styles() {
            let rendered = style.render().to_string();
            assert!(
                !rendered.contains("38;5;") && !rendered.contains("38;2;"),
                "style {style:?} renders as {rendered:?}"
            );
            assert!(
                !rendered.contains("48;"),
                "style {style:?} sets a background color"
            );
        }
    }

    #[test]
    fn roles_are_unique_and_round_trip_through_set_and_get() {
        let unique: std::collections::BTreeSet<&str> = Theme::ROLES.iter().copied().collect();
        assert_eq!(unique.len(), 36);
        let mut theme = Theme::default();
        let marker = Style::new()
            .fg_color(Some(AnsiColor::Green.into()))
            .italic();
        for role in Theme::ROLES {
            theme.set(role, marker).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(theme.get(role), Some(marker), "{role}");
        }
        assert_eq!(theme.all_styles().len(), 36);
        assert!(theme.all_styles().iter().all(|s| *s == marker));
    }

    #[test]
    fn syntax_roles_default_to_the_portable_palette() {
        let fg = |color: AnsiColor| Style::new().fg_color(Some(color.into()));
        let theme = Theme::default();
        assert_eq!(theme.syntax_keyword, fg(AnsiColor::Magenta));
        assert_eq!(theme.syntax_string, fg(AnsiColor::Green));
        assert_eq!(theme.syntax_comment, Style::new().dimmed());
        assert_eq!(theme.syntax_constant, fg(AnsiColor::Red));
        assert_eq!(theme.syntax_type, fg(AnsiColor::Yellow));
        assert_eq!(theme.syntax_function, fg(AnsiColor::Blue));
        assert_eq!(theme.syntax_tag, fg(AnsiColor::BrightBlue));
    }

    #[test]
    fn heading_roles_map_onto_levels() {
        let mut theme = Theme::default();
        theme.set("heading3", Style::new().bold().italic()).unwrap();
        assert_eq!(theme.heading[2], Style::new().bold().italic());
        assert_eq!(theme.get("heading1"), Some(theme.heading[0]));
        assert_eq!(theme.get("heading6"), Some(theme.heading[5]));
    }

    #[test]
    fn unknown_roles_are_rejected() {
        let mut theme = Theme::default();
        let err = theme.set("heading7", Style::new()).unwrap_err();
        assert_eq!(err, UnknownRole("heading7".to_string()));
        assert_eq!(err.to_string(), "unknown role `heading7`");
        assert_eq!(theme.get("Heading1"), None);
        assert_eq!(theme, Theme::default());
    }

    #[test]
    fn new_roles_default_to_todays_appearance() {
        let theme = Theme::default();
        assert_eq!(theme.bullet, Style::new());
        assert_eq!(theme.list_number, Style::new());
        assert_eq!(theme.task_checkbox, Style::new());
        assert_eq!(theme.table_border, Style::new().dimmed());
    }

    #[test]
    fn heading_levels_clamp_into_range() {
        let theme = Theme::default();
        assert_eq!(theme.heading_level(1), theme.heading[0]);
        assert_eq!(theme.heading_level(6), theme.heading[5]);
        assert_eq!(theme.heading_level(9), theme.heading[5]);
        assert_eq!(theme.heading_level(0), theme.heading[0]);
    }

    #[test]
    fn glyph_sets_are_single_column() {
        use unicode_width::UnicodeWidthChar;
        for glyphs in [Glyphs::Unicode, Glyphs::Ascii] {
            for glyph in [
                glyphs.bullet(),
                glyphs.quote(),
                glyphs.rule(),
                glyphs.vertical(),
                glyphs.cross(),
            ] {
                assert_eq!(glyph.width(), Some(1), "{glyph:?} in {glyphs:?}");
            }
        }
        assert_eq!(Glyphs::Ascii.bullet(), '*');
        assert_eq!(Glyphs::Unicode.bullet(), '•');
    }
}
