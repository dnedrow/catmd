//! Inline nodes to styled spans.

use anstyle::Style;
use comrak::nodes::{Node, NodeLink, NodeValue};

use super::layout::{HARD_BREAK, Span, spans_text};
use crate::theme::Theme;

/// Combine `base` with `overlay`: effects accumulate and the overlay's
/// foreground color, when it has one, replaces the base color.
pub fn merge(base: Style, overlay: Style) -> Style {
    let mut style = base | overlay.get_effects();
    if let Some(fg) = overlay.get_fg_color() {
        style = style.fg_color(Some(fg));
    }
    style
}

/// Render the inline children of `node` into spans, starting from `base`.
pub fn render_inlines(node: Node<'_>, theme: &Theme, base: Style) -> Vec<Span> {
    let mut spans = Vec::new();
    render_children(node, theme, base, &mut spans);
    spans
}

fn render_children(node: Node<'_>, theme: &Theme, base: Style, out: &mut Vec<Span>) {
    for child in node.children() {
        render_inline(child, theme, base, out);
    }
}

fn render_inline(node: Node<'_>, theme: &Theme, base: Style, out: &mut Vec<Span>) {
    let data = node.data();
    match &data.value {
        NodeValue::Text(text) => out.push(Span::new(text.to_string(), base)),
        NodeValue::Code(code) => out.push(Span::new(code.literal.clone(), merge(base, theme.code))),
        NodeValue::Emph => render_children(node, theme, merge(base, theme.emph), out),
        NodeValue::Strong => render_children(node, theme, merge(base, theme.strong), out),
        NodeValue::Strikethrough => {
            render_children(node, theme, merge(base, theme.strikethrough), out);
        }
        NodeValue::SoftBreak => out.push(Span::new(" ", base)),
        NodeValue::LineBreak => out.push(Span::new(HARD_BREAK, base)),
        NodeValue::HtmlInline(html) => out.push(Span::new(html.clone(), merge(base, theme.html))),
        NodeValue::Raw(raw) => out.push(Span::new(raw.clone(), base)),
        NodeValue::Math(math) => out.push(Span::new(math.literal.clone(), base)),
        NodeValue::EscapedTag(tag) => out.push(Span::new(tag.to_string(), base)),
        NodeValue::FootnoteReference(reference) => out.push(Span::new(
            format!("[{}]", reference.ix),
            merge(base, theme.footnote_marker),
        )),
        NodeValue::Link(link) => render_link(node, link, theme, base, out),
        NodeValue::Image(image) => render_image(node, image, theme, base, out),
        // Escaped characters and any inline container from an extension that
        // is not enabled: render the children unstyled.
        _ => render_children(node, theme, base, out),
    }
}

fn render_link(node: Node<'_>, link: &NodeLink, theme: &Theme, base: Style, out: &mut Vec<Span>) {
    let text_style = merge(base, theme.link_text);
    let text_spans = render_inlines(node, theme, text_style);
    let text = spans_text(&text_spans);
    let url = link.url.as_str();
    let bare_url = url.strip_prefix("mailto:").unwrap_or(url);
    if text == url || text == bare_url {
        // Autolinks and bare URLs: the destination once, underlined.
        out.extend(text_spans);
    } else {
        out.extend(text_spans);
        out.push(Span::new(" (", base));
        out.push(Span::new(url.to_string(), merge(base, theme.link_url)));
        out.push(Span::new(")", base));
    }
}

fn render_image(node: Node<'_>, image: &NodeLink, theme: &Theme, base: Style, out: &mut Vec<Span>) {
    let alt = spans_text(&render_inlines(node, theme, base));
    let label = if alt.is_empty() {
        "[image]".to_string()
    } else {
        format!("[image: {alt}]")
    };
    out.push(Span::new(label, merge(base, theme.image)));
    out.push(Span::new(" (", base));
    out.push(Span::new(image.url.clone(), merge(base, theme.link_url)));
    out.push(Span::new(")", base));
}

#[cfg(test)]
mod tests {
    use super::*;
    use anstyle::AnsiColor;

    #[test]
    fn merge_accumulates_effects_and_overrides_foreground() {
        let base = Style::new().bold().fg_color(Some(AnsiColor::Red.into()));
        let overlay = Style::new().italic().fg_color(Some(AnsiColor::Blue.into()));
        let merged = merge(base, overlay);
        assert!(merged.get_effects().contains(anstyle::Effects::BOLD));
        assert!(merged.get_effects().contains(anstyle::Effects::ITALIC));
        assert_eq!(merged.get_fg_color(), Some(AnsiColor::Blue.into()));
    }

    #[test]
    fn merge_keeps_base_foreground_when_overlay_has_none() {
        let base = Style::new().fg_color(Some(AnsiColor::Red.into()));
        let merged = merge(base, Style::new().underline());
        assert_eq!(merged.get_fg_color(), Some(AnsiColor::Red.into()));
        assert!(merged.get_effects().contains(anstyle::Effects::UNDERLINE));
    }
}
