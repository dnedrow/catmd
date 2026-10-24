//! Library-level rendering tests: exact expected output per spec scenario.

use anstyle::{AnsiColor, Style};
use catmd::theme::color::ColorDepth;
use catmd::{Glyphs, Settings, Theme, render};

const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const ITALIC: &str = "\x1b[3m";
const UNDERLINE: &str = "\x1b[4m";
const STRIKE: &str = "\x1b[9m";
const RESET: &str = "\x1b[0m";

fn settings(width: usize, color: bool, glyphs: Glyphs) -> Settings {
    Settings {
        depth: if color {
            ColorDepth::Ansi16
        } else {
            ColorDepth::Off
        },
        width,
        glyphs,
        ..Settings::default()
    }
}

/// Render with styling off and ASCII glyphs at the given width.
fn plain_at(md: &str, width: usize) -> String {
    render(md, &settings(width, false, Glyphs::Ascii))
}

/// Render with styling off and ASCII glyphs at width 80.
fn plain(md: &str) -> String {
    plain_at(md, 80)
}

/// Render with styling on and ASCII glyphs at width 80.
fn styled(md: &str) -> String {
    render(md, &settings(80, true, Glyphs::Ascii))
}

/// Render with styling off and Unicode glyphs at the given width.
fn unicode_at(md: &str, width: usize) -> String {
    render(md, &settings(width, false, Glyphs::Unicode))
}

fn lines(out: &str) -> Vec<&str> {
    out.lines().collect()
}

/// The run of escape sequences immediately preceding the first occurrence of `text`.
fn style_prefix<'a>(out: &'a str, text: &str) -> &'a str {
    let end = out
        .find(text)
        .unwrap_or_else(|| panic!("{text:?} not in {out:?}"));
    let mut start = end;
    while let Some(esc) = out[..start].rfind('\x1b') {
        let seq = &out[esc..start];
        let is_sgr = seq.starts_with("\x1b[")
            && seq.ends_with('m')
            && seq[2..seq.len() - 1]
                .chars()
                .all(|c| c.is_ascii_digit() || c == ';');
        if is_sgr {
            start = esc;
        } else {
            break;
        }
    }
    &out[start..end]
}

fn assert_no_escapes(out: &str) {
    assert!(!out.contains('\x1b'), "unexpected escape in {out:?}");
}

// ---------------------------------------------------------------------------
// inline-rendering
// ---------------------------------------------------------------------------

#[test]
fn strong_renders_bold_for_both_delimiters() {
    let out = styled("**one** and __two__\n");
    assert!(style_prefix(&out, "one").contains(BOLD), "{out:?}");
    assert!(style_prefix(&out, "two").contains(BOLD), "{out:?}");
    assert!(!out.contains('*') && !out.contains('_'), "{out:?}");
}

#[test]
fn strong_in_plain_mode_drops_delimiters_without_escapes() {
    let out = plain("**one**\n");
    assert_eq!(out, "one\n");
    assert_no_escapes(&out);
}

#[test]
fn emphasis_renders_italic() {
    let out = styled("*soft*\n");
    assert!(style_prefix(&out, "soft").contains(ITALIC), "{out:?}");
    assert!(!out.contains('*'));
}

#[test]
fn strikethrough_renders_with_strike_attribute() {
    let out = styled("~~old~~\n");
    assert!(style_prefix(&out, "old").contains(STRIKE), "{out:?}");
    assert!(!out.contains('~'));
}

#[test]
fn nested_inline_styles_combine() {
    let out = styled("**bold *both* bold**\n");
    let both = style_prefix(&out, "both");
    assert!(both.contains(BOLD) && both.contains(ITALIC), "{out:?}");
    let first_bold = style_prefix(&out, "bold");
    assert!(
        first_bold.contains(BOLD) && !first_bold.contains(ITALIC),
        "{out:?}"
    );
}

#[test]
fn code_span_content_is_literal_and_not_italic() {
    let out = styled("`a*b*c`\n");
    assert!(out.contains("a*b*c"), "{out:?}");
    assert!(!out.contains(ITALIC), "{out:?}");
    assert_eq!(plain("`a*b*c`\n"), "a*b*c\n");
}

#[test]
fn code_span_backticks_are_removed() {
    assert_eq!(plain("run `cargo build` now\n"), "run cargo build now\n");
}

#[test]
fn link_shows_text_and_destination_and_ignores_title() {
    let out = plain("[docs](https://example.com/docs \"Title\")\n");
    assert_eq!(out, "docs (https://example.com/docs)\n");
}

#[test]
fn link_text_is_underlined_but_url_is_not() {
    let out = styled("[docs](https://example.com)\n");
    assert!(style_prefix(&out, "docs").contains(UNDERLINE), "{out:?}");
    let url_prefix = style_prefix(&out, "https://example.com");
    assert!(!url_prefix.contains(UNDERLINE), "{out:?}");
    assert!(url_prefix.contains(DIM), "{out:?}");
}

#[test]
fn angle_bracket_autolink_shows_url_once() {
    let out = plain("<https://example.com>\n");
    assert_eq!(out, "https://example.com\n");
}

#[test]
fn bare_url_shows_url_once() {
    assert_eq!(
        plain("see https://example.com today\n"),
        "see https://example.com today\n"
    );
}

#[test]
fn email_autolink_strips_mailto() {
    let out = plain("<user@example.com>\n");
    assert_eq!(out, "user@example.com\n");
}

#[test]
fn image_with_alt_text() {
    assert_eq!(plain("![Logo](logo.png)\n"), "[image: Logo] (logo.png)\n");
}

#[test]
fn image_without_alt_text() {
    assert_eq!(plain("![](logo.png)\n"), "[image] (logo.png)\n");
}

#[test]
fn soft_break_becomes_a_space() {
    assert_eq!(
        plain("first line\nsecond line\n"),
        "first line second line\n"
    );
}

#[test]
fn hard_break_starts_a_new_line() {
    assert_eq!(plain("roses\\\nviolets\n"), "roses\nviolets\n");
    assert_eq!(plain("roses  \nviolets\n"), "roses\nviolets\n");
}

#[test]
fn inline_html_is_literal_and_dimmed() {
    assert_eq!(plain("press <kbd>Ctrl</kbd>\n"), "press <kbd>Ctrl</kbd>\n");
    let out = styled("press <kbd>Ctrl</kbd>\n");
    assert!(style_prefix(&out, "<kbd>").contains(DIM), "{out:?}");
}

#[test]
fn escapes_and_entities_are_decoded() {
    let out = plain("\\*not italic\\* &amp; &copy;\n");
    assert_eq!(out, "*not italic* & ©\n");
    assert!(!styled("\\*not italic\\* &amp; &copy;\n").contains(ITALIC));
}

#[test]
fn footnote_references_are_numbered_by_first_reference() {
    let out = plain("alpha[^b] beta[^a]\n\n[^a]: A\n[^b]: B\n");
    assert!(out.starts_with("alpha[1] beta[2]\n"), "{out:?}");
}

#[test]
fn styled_output_stripped_equals_plain_output() {
    let md = "# Title\n\nSome **bold** and *italic* with `code` and a [link](https://x.y).\n\n> quote\n\n- item\n";
    let styled_out = render(md, &settings(40, true, Glyphs::Ascii));
    let plain_out = render(md, &settings(40, false, Glyphs::Ascii));
    assert_eq!(catmd::render::layout::strip_ansi(&styled_out), plain_out);
    assert!(!styled_out.contains("38;5;") && !styled_out.contains("38;2;"));
    let _ = RESET;
}

// ---------------------------------------------------------------------------
// block-rendering: headings, paragraphs, rules, front matter, HTML blocks
// ---------------------------------------------------------------------------

#[test]
fn heading_then_paragraph_is_exact() {
    assert_eq!(plain("# Title\n\nBody text.\n"), "# Title\n\nBody text.\n");
}

#[test]
fn atx_heading_keeps_markers_in_plain_mode() {
    assert!(lines(&plain("## Usage\n")).contains(&"## Usage"));
    assert!(lines(&plain("###### Deep\n")).contains(&"###### Deep"));
}

#[test]
fn setext_heading_is_normalized_to_atx_form() {
    let out = plain("Title\n=====\n\nSub\n---\n");
    assert_eq!(out, "# Title\n\n## Sub\n");
}

#[test]
fn level_one_heading_is_bold_underlined_with_dim_marker() {
    let out = styled("# Title\n");
    let title = style_prefix(&out, "Title");
    assert!(title.contains(BOLD) && title.contains(UNDERLINE), "{out:?}");
    assert!(style_prefix(&out, "#").contains(DIM), "{out:?}");
    let sub = styled("## Sub\n");
    let sub_prefix = style_prefix(&sub, "Sub");
    assert!(
        sub_prefix.contains(BOLD) && !sub_prefix.contains(UNDERLINE),
        "{sub:?}"
    );
}

#[test]
fn long_paragraph_wraps_at_width() {
    let text: Vec<String> = (0..40).map(|i| format!("word{i}")).collect();
    let out = plain_at(&format!("{}\n", text.join(" ")), 40);
    for line in lines(&out) {
        assert!(line.len() <= 40, "{line:?}");
        assert!(!line.ends_with(' '));
    }
    assert_eq!(lines(&out).join(" "), text.join(" "));
}

#[test]
fn short_paragraph_stays_on_one_line() {
    assert_eq!(plain("short text\n"), "short text\n");
}

#[test]
fn cjk_paragraph_wraps_by_display_columns() {
    let text: String = std::iter::repeat_n('漢', 30).collect();
    let out = plain_at(&format!("{text}\n"), 40);
    assert_eq!(lines(&out)[0].chars().count(), 20);
}

#[test]
fn long_url_is_not_broken() {
    let url = "https://example.com/a/very/long/path/that/exceeds/the/width";
    let out = plain_at(&format!("see {url}\n"), 30);
    assert_eq!(lines(&out), vec!["see", url]);
}

#[test]
fn thematic_break_spans_the_width() {
    assert_eq!(plain_at("---\n", 30), format!("{}\n", "-".repeat(30)));
    assert_eq!(unicode_at("***\n", 10), format!("{}\n", "─".repeat(10)));
}

#[test]
fn front_matter_is_hidden() {
    let out = plain("---\ntitle: Doc\n---\n\n# Body\n");
    assert_eq!(out, "# Body\n");
}

#[test]
fn html_block_is_literal() {
    let out = plain("<details>\n<summary>More</summary>\n</details>\n");
    assert_eq!(out, "<details>\n<summary>More</summary>\n</details>\n");
    let colored = styled("<details>\n</details>\n");
    assert!(
        style_prefix(&colored, "<details>").contains(DIM),
        "{colored:?}"
    );
}

#[test]
fn empty_and_whitespace_documents_render_nothing() {
    assert_eq!(plain(""), "");
    assert_eq!(plain("  \n\n\t\n"), "");
}

#[test]
fn output_has_no_leading_blank_and_single_trailing_newline() {
    let out = plain("\n\n\nfirst\n\n\n\nsecond\n\n\n");
    assert_eq!(out, "first\n\nsecond\n");
}

// ---------------------------------------------------------------------------
// block-rendering: lists
// ---------------------------------------------------------------------------

#[test]
fn bullet_item_wraps_with_hanging_indent() {
    let text: Vec<String> = (0..20).map(|i| format!("word{i}")).collect();
    let out = plain_at(&format!("- {}\n", text.join(" ")), 40);
    let lines = lines(&out);
    assert!(lines.len() > 1);
    assert!(lines[0].starts_with("* "), "{out:?}");
    for line in &lines[1..] {
        assert!(
            line.starts_with("  ") && !line.starts_with("   "),
            "{line:?}"
        );
        assert!(line.len() <= 40);
    }
}

#[test]
fn unicode_bullet() {
    assert_eq!(unicode_at("- apple\n", 80), "• apple\n");
}

#[test]
fn ordered_list_honors_start_number() {
    assert_eq!(plain("3. c\n4. d\n"), "3. c\n4. d\n");
}

#[test]
fn ordered_markers_are_right_aligned_to_the_widest() {
    let md: String = (1..=10).map(|i| format!("{i}. item{i}\n")).collect();
    let out = plain(&md);
    let lines = lines(&out);
    assert_eq!(lines[0], " 1. item1");
    assert_eq!(lines[8], " 9. item9");
    assert_eq!(lines[9], "10. item10");
}

#[test]
fn ordered_item_continuation_aligns_to_content() {
    let text: Vec<String> = (0..20).map(|i| format!("w{i}")).collect();
    let out = plain_at(&format!("1. {}\n", text.join(" ")), 30);
    let lines = lines(&out);
    assert!(lines[0].starts_with("1. "));
    assert!(
        lines[1].starts_with("   ") && !lines[1].starts_with("    "),
        "{out:?}"
    );
}

#[test]
fn nested_list_is_indented_under_parent() {
    assert_eq!(plain("- parent\n  - child\n"), "* parent\n  * child\n");
}

#[test]
fn tight_list_has_no_blank_lines() {
    assert_eq!(plain("- a\n- b\n"), "* a\n* b\n");
}

#[test]
fn loose_list_has_one_blank_line_between_items() {
    assert_eq!(plain("- a\n\n- b\n"), "* a\n\n* b\n");
}

#[test]
fn task_items_show_checkboxes() {
    assert_eq!(plain("- [x] done\n- [ ] todo\n"), "[x] done\n[ ] todo\n");
    assert_eq!(plain("- [X] done\n"), "[x] done\n");
}

#[test]
fn second_paragraph_in_item_aligns_to_content_column() {
    assert_eq!(plain("- first\n\n  second\n"), "* first\n\n  second\n");
}

#[test]
fn code_and_quote_inside_item_align_to_content_column() {
    let out = plain("- item\n\n  > quoted\n\n  ```\n  code\n  ```\n");
    assert_eq!(out, "* item\n\n  | quoted\n\n      code\n");
}

#[test]
fn list_followed_by_paragraph_is_separated() {
    assert_eq!(plain("- a\n- b\n\npara\n"), "* a\n* b\n\npara\n");
}

// ---------------------------------------------------------------------------
// block-rendering: quotes, alerts, code blocks
// ---------------------------------------------------------------------------

#[test]
fn block_quote_has_a_gutter() {
    assert_eq!(plain("> quoted\n"), "| quoted\n");
    assert_eq!(unicode_at("> quoted\n", 80), "│ quoted\n");
}

#[test]
fn nested_quotes_stack_gutters() {
    assert_eq!(plain("> > deep\n"), "| | deep\n");
}

#[test]
fn quoted_content_wraps_within_remaining_width() {
    let text: Vec<String> = (0..20).map(|i| format!("word{i}")).collect();
    let out = plain_at(&format!("> {}\n", text.join(" ")), 30);
    let lines = lines(&out);
    assert!(lines.len() > 1);
    for line in &lines {
        assert!(line.starts_with("| "), "{line:?}");
        assert!(line.len() <= 30, "{line:?}");
    }
}

#[test]
fn quote_with_two_paragraphs_keeps_the_gutter_on_the_blank_line() {
    assert_eq!(plain("> one\n>\n> two\n"), "| one\n|\n| two\n");
}

#[test]
fn warning_alert_in_plain_ascii_mode() {
    let out = plain("> [!WARNING]\n> Mind the gap.\n");
    assert_eq!(out, "| Warning\n| Mind the gap.\n");
}

#[test]
fn alert_titles_use_github_names_and_colors() {
    for (kind, title) in [
        ("NOTE", "Note"),
        ("TIP", "Tip"),
        ("IMPORTANT", "Important"),
        ("WARNING", "Warning"),
        ("CAUTION", "Caution"),
    ] {
        let out = plain(&format!("> [!{kind}]\n> body\n"));
        assert_eq!(out, format!("| {title}\n| body\n"));
        let colored = styled(&format!("> [!{kind}]\n> body\n"));
        assert!(style_prefix(&colored, title).contains(BOLD), "{colored:?}");
    }
}

#[test]
fn alert_custom_title_replaces_default() {
    let out = plain("> [!NOTE] Heads up\n> text\n");
    let first = lines(&out)[0];
    assert!(first.ends_with("Heads up"), "{out:?}");
    assert!(!first.contains("Note"), "{out:?}");
}

#[test]
fn fenced_code_block_is_indented_without_fence_or_info() {
    let out = plain("```rust\nfn main() {}\n```\n");
    assert_eq!(out, "    fn main() {}\n");
    assert!(!out.contains("rust") && !out.contains('`'));
}

#[test]
fn indented_code_block_renders_like_fenced() {
    assert_eq!(plain("    let x = 1;\n"), "    let x = 1;\n");
}

#[test]
fn long_code_line_is_not_wrapped() {
    let long = "x".repeat(120);
    let out = plain_at(&format!("```\n{long}\n```\n"), 40);
    assert_eq!(out, format!("    {long}\n"));
}

#[test]
fn blank_lines_inside_code_are_preserved_and_empty() {
    assert_eq!(plain("```\na\n\nb\n```\n"), "    a\n\n    b\n");
}

#[test]
fn code_block_keeps_internal_trailing_spaces_and_markup() {
    assert_eq!(plain("```\n*not* emph  \n```\n"), "    *not* emph  \n");
}

#[test]
fn code_block_lines_are_styled_when_color_is_on() {
    let out = styled("```\ncode\n```\n");
    assert!(out.ends_with("\x1b[0m\n"), "{out:?}");
    assert!(out.contains("    \x1b["), "{out:?}");
}

// ---------------------------------------------------------------------------
// syntax-highlighting
// ---------------------------------------------------------------------------

/// Render with styling on at the given depth, ASCII glyphs, width 80.
fn styled_at_depth(md: &str, depth: ColorDepth) -> String {
    let settings = Settings {
        depth,
        width: 80,
        glyphs: Glyphs::Ascii,
        ..Settings::default()
    };
    render(md, &settings)
}

const MAGENTA: &str = "\x1b[35m";
const BLUE: &str = "\x1b[34m";
const CYAN: &str = "\x1b[36m";

/// A single content line in the plain code-block style and nothing else.
fn uniform_code_line(text: &str) -> String {
    format!("    {CYAN}{text}{RESET}\n")
}

#[test]
fn recognized_language_styles_tokens_by_role() {
    let out = styled("```rust\nfn main() {}\n```\n");
    assert_eq!(
        out,
        format!("    {MAGENTA}fn{RESET}{CYAN} {RESET}{BLUE}main{RESET}{CYAN}() {{}}{RESET}\n")
    );
}

#[test]
fn language_is_the_first_word_of_the_info_string() {
    for info in ["rust,ignore", "Rust", "rs", "rust title=\"x\""] {
        let out = styled(&format!("```{info}\nfn main() {{}}\n```\n"));
        assert!(
            out.contains(&format!("{MAGENTA}fn{RESET}")),
            "{info}: {out:?}"
        );
    }
    let out = styled("```js title=\"demo.js\"\nconst x = 1;\n```\n");
    assert!(out.contains(&format!("{MAGENTA}const{RESET}")), "{out:?}");
}

#[test]
fn aliases_resolve_to_grammars() {
    let out = styled("```shell\necho \"hi\"\n```\n");
    assert!(out.contains("\x1b[32m\"hi\"\x1b[0m"), "{out:?}");
    let out = styled("```jsonc\n{\"a\": \"b\"}\n```\n");
    assert!(out.contains("\x1b[32m\"b\"\x1b[0m"), "{out:?}");
}

#[test]
fn unknown_language_renders_like_a_fence_without_one() {
    let unknown = styled("```foobarlang\nfn main() {}\n```\n");
    let none = styled("```\nfn main() {}\n```\n");
    assert_eq!(unknown, none);
    assert_eq!(none, uniform_code_line("fn main() {}"));
}

#[test]
fn indented_and_plain_text_blocks_are_not_highlighted() {
    assert_eq!(
        styled("    fn main() {}\n"),
        uniform_code_line("fn main() {}")
    );
    for marker in ["text", "plaintext", "plain", "txt", "nohighlight"] {
        assert_eq!(
            styled(&format!("```{marker}\nfn main() {{}}\n```\n")),
            uniform_code_line("fn main() {}"),
            "{marker}"
        );
    }
}

#[test]
fn highlighted_block_inside_a_list_item_keeps_its_prefixes() {
    let out = styled("- item\n\n  ```rust\n  fn main() {}\n  ```\n");
    let line = lines(&out)
        .into_iter()
        .find(|l| l.contains("main"))
        .expect("code line");
    assert!(
        line.starts_with(&format!("      {MAGENTA}fn{RESET}")),
        "{line:?}"
    );
    assert_eq!(
        catmd::render::layout::strip_ansi(line),
        "      fn main() {}"
    );
}

#[test]
fn long_highlighted_line_is_not_wrapped() {
    let long = format!("let s = \"{}\";", "x".repeat(110));
    assert!(long.len() >= 120);
    let md = format!("```rust\n{long}\n```\n");
    let out = render(
        &md,
        &Settings {
            depth: ColorDepth::Ansi16,
            width: 40,
            glyphs: Glyphs::Ascii,
            ..Settings::default()
        },
    );
    let text = catmd::render::layout::strip_ansi(&out);
    assert_eq!(text, format!("    {long}\n"));
}

#[test]
fn highlighting_changes_only_styles() {
    let md = "# T\n\n```rust\nfn main() {\n    let s = \"a\\n\"; // c\n}\n```\n\n```toml\n[a]\nb = 1\n```\n\n```bash\nfor f in *; do echo \"$f\"; done\n```\n\ntail\n";
    let styled_out = render(md, &settings(40, true, Glyphs::Ascii));
    let plain_out = render(md, &settings(40, false, Glyphs::Ascii));
    assert_eq!(catmd::render::layout::strip_ansi(&styled_out), plain_out);
    assert_eq!(plain("```rust\nfn main() {}\n```\n"), "    fn main() {}\n");
}

#[test]
fn default_comment_style_is_dim_without_a_color() {
    let out = styled("```rust\n// note\n```\n");
    assert_eq!(out, format!("    {DIM}// note{RESET}\n"));
}

#[test]
fn default_highlighting_stays_portable_at_truecolor() {
    let md = "```rust\nfn main() { let x = 1; }\n```\n\n```toml\n[a]\nb = \"c\"\n```\n\n```bash\necho \"$HOME\" # c\n```\n\n```html\n<p class=\"x\">hi</p>\n```\n";
    let out = styled_at_depth(md, ColorDepth::TrueColor);
    assert!(out.contains(MAGENTA), "{out:?}");
    assert!(!out.contains("38;5;"), "{out:?}");
    assert!(!out.contains("38;2;"), "{out:?}");
    assert!(!out.contains("48;"), "{out:?}");
}

#[test]
fn syntax_roles_are_read_from_the_theme() {
    let theme = Theme {
        syntax_keyword: Style::new().fg_color(Some(AnsiColor::Red.into())),
        code_block: Style::new().fg_color(Some(AnsiColor::White.into())),
        ..Theme::default()
    };
    let settings = Settings {
        depth: ColorDepth::Ansi16,
        width: 80,
        glyphs: Glyphs::Ascii,
        theme,
        ..Settings::default()
    };
    let out = render("```rust\nfn main() {}\n```\n", &settings);
    assert!(out.contains("\x1b[31mfn\x1b[0m"), "{out:?}");
    assert!(out.contains("\x1b[37m() {}\x1b[0m"), "{out:?}");
    assert!(out.contains(&format!("{BLUE}main{RESET}")), "{out:?}");
}

// ---------------------------------------------------------------------------
// block-rendering: tables
// ---------------------------------------------------------------------------

#[test]
fn simple_table_in_ascii_mode() {
    let out = plain("| Name | Qty |\n|------|-----|\n| Apple | 3 |\n");
    assert_eq!(out, "Name  | Qty\n------+----\nApple | 3\n");
}

#[test]
fn simple_table_in_unicode_mode() {
    let out = unicode_at("| Name | Qty |\n|------|-----|\n| Apple | 3 |\n", 80);
    assert_eq!(out, "Name  │ Qty\n──────┼────\nApple │ 3\n");
}

#[test]
fn right_aligned_column() {
    let out = plain("| Qty |\n|----:|\n| 3 |\n| 100 |\n");
    assert_eq!(out, "Qty\n---\n  3\n100\n");
}

#[test]
fn centered_column() {
    let out = plain("| Head |\n|:----:|\n| ab |\n| abcd |\n");
    assert_eq!(out, "Head\n----\n ab\nabcd\n");
}

#[test]
fn header_cells_are_bold_and_cells_keep_inline_styles() {
    let out = styled("| Name | Qty |\n|------|-----|\n| **Apple** | `3` |\n");
    assert!(style_prefix(&out, "Name").contains(BOLD), "{out:?}");
    assert!(style_prefix(&out, "Apple").contains(BOLD), "{out:?}");
    assert_eq!(
        catmd::render::layout::strip_ansi(&out),
        "Name  | Qty\n------+----\nApple | 3\n"
    );
}

#[test]
fn wide_table_is_rendered_at_natural_width() {
    // Two columns of 28 and 29 columns joined by " | " = 60 columns per line.
    let c1 = "a".repeat(28);
    let c2 = "b".repeat(29);
    let md = format!("| {c1} | {c2} |\n|---|---|\n| {c1} | {c2} |\n");
    let out = plain_at(&md, 40);
    for line in lines(&out) {
        assert_eq!(line.len(), 60, "{line:?}");
    }
    assert!(out.contains(&c1) && out.contains(&c2));
}

#[test]
fn table_followed_by_paragraph_is_separated() {
    let out = plain("| A |\n|---|\n| 1 |\n\ntext\n");
    assert_eq!(out, "A\n-\n1\n\ntext\n");
}

// ---------------------------------------------------------------------------
// block-rendering: footnotes
// ---------------------------------------------------------------------------

#[test]
fn footnote_definitions_follow_a_rule_at_the_end() {
    let out = plain_at("text[^a]\n\n[^a]: the note\n", 20);
    assert_eq!(
        out,
        format!("text[1]\n\n{}\n\n[1] the note\n", "-".repeat(20))
    );
}

#[test]
fn footnote_definitions_are_numbered_by_first_reference_and_hang_indented() {
    let out = plain_at(
        "alpha[^b] beta[^a]\n\n[^a]: note a\n[^b]: note b is long enough to wrap\n",
        24,
    );
    let lines = lines(&out);
    assert_eq!(lines[0], "alpha[1] beta[2]");
    assert_eq!(lines[2], "-".repeat(24));
    assert_eq!(lines[4], "[1] note b is long");
    assert_eq!(lines[5], "    enough to wrap");
    assert_eq!(lines[7], "[2] note a");
}

#[test]
fn unreferenced_footnote_definitions_are_dropped() {
    assert_eq!(plain("text\n\n[^x]: unused\n"), "text\n");
}

// ---------------------------------------------------------------------------
// theming: roles
// ---------------------------------------------------------------------------

const ROLES_DOC: &str = "- apple\n- pear\n\n1. one\n2. two\n\n- [x] done\n- [ ] todo\n\n| Name | Qty |\n|------|----:|\n| Apple | 3 |\n";

/// Captured from the binary before theme roles existed (`--color always --ascii --width 40`).
const ROLES_DOC_DEFAULT_STYLED: &str = "* apple\n* pear\n\n1. one\n2. two\n\n[x] done\n[ ] todo\n\n\x1b[1mName\x1b[0m  \x1b[2m|\x1b[0m \x1b[1mQty\x1b[0m\n\x1b[2m------+----\x1b[0m\nApple \x1b[2m|\x1b[0m   3\n";

#[test]
fn default_theme_output_is_byte_identical_to_before_roles_existed() {
    for depth in [ColorDepth::Ansi16, ColorDepth::TrueColor] {
        let s = Settings {
            depth,
            ..settings(40, true, Glyphs::Ascii)
        };
        assert_eq!(render(ROLES_DOC, &s), ROLES_DOC_DEFAULT_STYLED, "{depth:?}");
    }
}

#[test]
fn newly_exposed_roles_take_effect() {
    let green = Style::new().fg_color(Some(AnsiColor::Green.into()));
    let mut theme = Theme::default();
    for role in ["bullet", "list_number", "task_checkbox", "table_border"] {
        theme.set(role, green).unwrap();
    }
    let s = Settings {
        theme,
        ..settings(40, true, Glyphs::Ascii)
    };
    let out = render(ROLES_DOC, &s);
    let g = "\x1b[32m";
    assert!(out.contains(&format!("{g}* {RESET}apple")), "{out:?}");
    assert!(out.contains(&format!("{g}1. {RESET}one")), "{out:?}");
    assert!(out.contains(&format!("{g}[ ] {RESET}todo")), "{out:?}");
    assert!(out.contains(&format!("{g}|{RESET}")), "{out:?}");
    assert!(out.contains(&format!("{g}------+----{RESET}")), "{out:?}");
    assert!(!out.contains(DIM), "{out:?}");
}

mod images {
    use std::path::{Path, PathBuf};

    use catmd::image::{BackendKind, CellSize, ImageSettings};
    use catmd::render::render_in;

    use super::*;

    fn fixture(name: &str, width: u32, height: u32) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("catmd-images-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        ::image::RgbaImage::from_pixel(width, height, ::image::Rgba([255, 0, 0, 255]))
            .save(&path)
            .unwrap();
        path
    }

    fn with(backend: Option<BackendKind>, width: usize, color: bool) -> Settings {
        Settings {
            images: ImageSettings {
                backend,
                cell: CellSize::new(10, 20).unwrap(),
            },
            ..settings(width, color, Glyphs::Unicode)
        }
    }

    fn at(md: &str, settings: &Settings, base: &Path) -> String {
        render_in(md, settings, base)
    }

    #[test]
    fn placeholder_box_shows_alt_source_size_and_notice() {
        let path = fixture("box.png", 1920, 1080);
        let base = path.parent().unwrap();
        let out = at(
            "![Architecture Diagram](box.png)",
            &with(Some(BackendKind::Placeholder), 80, false),
            base,
        );
        assert_eq!(
            out,
            "┌─ Image ───────────────────┐\n\
             │ Architecture Diagram      │\n\
             │ box.png                   │\n\
             │ 1920×1080                 │\n\
             │ Image display unsupported │\n\
             └───────────────────────────┘\n"
        );
    }

    #[test]
    fn a_missing_file_degrades_to_the_placeholder() {
        let out = at(
            "![gone](nope.png)",
            &with(Some(BackendKind::Kitty), 80, false),
            Path::new("."),
        );
        assert!(out.contains("nope.png") && out.contains("Image display unsupported"));
        assert!(!out.contains('\x1b'));
    }

    #[test]
    fn no_backend_keeps_the_plain_label() {
        let out = at("![alt](x.png)", &with(None, 80, false), Path::new("."));
        assert_eq!(out, "[image: alt] (x.png)\n");
    }

    #[test]
    fn mixed_paragraphs_keep_the_inline_label() {
        let path = fixture("mixed.png", 10, 10);
        let out = at(
            "See ![alt](mixed.png) here",
            &with(Some(BackendKind::Placeholder), 80, false),
            path.parent().unwrap(),
        );
        assert_eq!(out, "See [image: alt] (mixed.png) here\n");
    }

    #[test]
    fn half_blocks_fit_the_width_and_keep_the_aspect_ratio() {
        let path = fixture("wide.png", 1000, 200);
        let out = at(
            "![](wide.png)",
            &with(Some(BackendKind::HalfBlock), 40, true),
            path.parent().unwrap(),
        );
        let lines: Vec<&str> = out.lines().collect();
        // 40 columns * 10px = 400px wide, 80px tall = 4 cell rows.
        assert_eq!(lines.len(), 4);
        for line in lines {
            assert_eq!(catmd::render::layout::strip_ansi(line).chars().count(), 40);
        }
    }

    #[test]
    fn small_images_keep_their_natural_size() {
        let path = fixture("small.png", 30, 40);
        let out = at(
            "![](small.png)",
            &with(Some(BackendKind::HalfBlock), 80, true),
            path.parent().unwrap(),
        );
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(
            catmd::render::layout::strip_ansi(lines[0]).chars().count(),
            3
        );
    }

    #[test]
    fn kitty_images_reserve_their_rows_and_stay_inside_a_quote() {
        let path = fixture("k.png", 100, 100);
        let out = at(
            "> ![](k.png)\n\nafter",
            &with(Some(BackendKind::Kitty), 40, false),
            path.parent().unwrap(),
        );
        let lines: Vec<&str> = out.lines().collect();
        // 100x100px at 10x20 cells is 10 columns by 5 rows.
        assert!(lines[0].starts_with("│ \x1b_G"), "{:?}", lines[0]);
        assert!(lines[0].contains("C=1"));
        assert_eq!(&lines[1..5], &["│"; 4]);
        assert_eq!(lines[5], "");
        assert_eq!(lines[6], "after");
    }

    #[test]
    fn sixel_images_restore_the_cursor_to_the_first_row() {
        let path = fixture("s.png", 100, 100);
        let out = at(
            "![](s.png)",
            &with(Some(BackendKind::Sixel), 40, false),
            path.parent().unwrap(),
        );
        assert!(
            out.starts_with("\n\n\n\n\x1b[4A\x1b[1G\x1b7\x1bP"),
            "{out:?}"
        );
        assert!(out.contains("\x1b\\\x1b8\n"));
        assert_eq!(out.matches('\n').count(), 4 + 4 + 1);
    }

    #[test]
    fn the_same_inputs_render_identically() {
        let path = fixture("d.png", 64, 64);
        let settings = with(Some(BackendKind::HalfBlock), 40, true);
        let base = path.parent().unwrap();
        assert_eq!(
            at("![](d.png)", &settings, base),
            at("![](d.png)", &settings, base)
        );
    }
}
