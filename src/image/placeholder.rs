//! The text box shown when an image cannot be drawn.

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::theme::Glyphs;

/// Smallest total box width; narrower areas get plain lines instead.
const MIN_BOX: usize = 11;

/// Lines (no styling, none wider than `width`) describing an image: its alt
/// text when present, its source, its pixel size when known, and a note that
/// image display is unsupported.
pub fn lines(
    alt: &str,
    src: &str,
    dimensions: Option<(u32, u32)>,
    width: usize,
    glyphs: Glyphs,
) -> Vec<String> {
    let (corner_tl, corner_tr, corner_bl, corner_br, horizontal, vertical, times, ellipsis) =
        match glyphs {
            Glyphs::Unicode => ('┌', '┐', '└', '┘', '─', '│', '×', "…"),
            Glyphs::Ascii => ('+', '+', '+', '+', '-', '|', 'x', "..."),
        };
    let mut content = Vec::new();
    if !alt.is_empty() {
        content.push(alt.to_string());
    }
    content.push(src.to_string());
    if let Some((w, h)) = dimensions {
        content.push(format!("{w}{times}{h}"));
    }
    content.push("Image display unsupported".to_string());

    if width < MIN_BOX {
        return content
            .iter()
            .map(|line| truncate(line, width, ellipsis))
            .collect();
    }
    let inner = content
        .iter()
        .map(|line| line.width())
        .max()
        .unwrap_or(0)
        .clamp(MIN_BOX - 4, width - 4);
    let rule = |count: usize| horizontal.to_string().repeat(count);

    let mut out = Vec::with_capacity(content.len() + 2);
    // "<h> Image " is 8 columns; the rest of the top edge is rule.
    out.push(format!(
        "{corner_tl}{horizontal} Image {}{corner_tr}",
        rule(inner + 2 - 8)
    ));
    for line in &content {
        let text = truncate(line, inner, ellipsis);
        let pad = inner - text.width();
        out.push(format!("{vertical} {text}{} {vertical}", " ".repeat(pad)));
    }
    out.push(format!("{corner_bl}{}{corner_br}", rule(inner + 2)));
    out
}

/// Cut `text` to at most `max` columns, ending in `ellipsis` when shortened.
fn truncate(text: &str, max: usize, ellipsis: &str) -> String {
    if text.width() <= max {
        return text.to_string();
    }
    let budget = max.saturating_sub(ellipsis.width());
    let mut out = String::new();
    let mut used = 0;
    for ch in text.chars() {
        let w = ch.width().unwrap_or(0);
        if used + w > budget {
            break;
        }
        used += w;
        out.push(ch);
    }
    if max >= ellipsis.width() {
        out.push_str(ellipsis);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_metadata_box() {
        let out = lines(
            "Architecture Diagram",
            "diagrams/rendering.png",
            Some((1920, 1080)),
            80,
            Glyphs::Unicode,
        );
        assert_eq!(
            out,
            vec![
                "┌─ Image ───────────────────┐",
                "│ Architecture Diagram      │",
                "│ diagrams/rendering.png    │",
                "│ 1920×1080                 │",
                "│ Image display unsupported │",
                "└───────────────────────────┘",
            ]
        );
    }

    #[test]
    fn omits_missing_alt_and_unknown_dimensions() {
        let out = lines("", "a.png", None, 80, Glyphs::Ascii);
        assert_eq!(
            out,
            vec![
                "+- Image -------------------+",
                "| a.png                     |",
                "| Image display unsupported |",
                "+---------------------------+",
            ]
        );
    }

    #[test]
    fn long_content_is_truncated_to_the_width() {
        let out = lines(
            "A very long alternative text that cannot fit",
            "some/very/long/path/to/an/image/file.png",
            Some((10, 10)),
            30,
            Glyphs::Unicode,
        );
        for line in &out {
            assert!(line.width() <= 30, "{line:?}");
        }
        assert!(out.iter().any(|l| l.contains('…')));
    }

    #[test]
    fn tiny_widths_fall_back_to_plain_lines() {
        let out = lines("alt", "image.png", None, 6, Glyphs::Unicode);
        assert!(out.iter().all(|l| l.width() <= 6));
        assert_eq!(out.len(), 3);
    }
}
