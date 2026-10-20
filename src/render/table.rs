//! Table layout: columns padded to their widest cell, aligned per the delimiter
//! row, joined by a column separator, with a rule after the header row. Tables
//! are rendered at their natural width and never wrapped.

use anstyle::Style;
use comrak::nodes::{Node, NodeTable, NodeValue, TableAlignment};

use super::Renderer;
use super::inline::render_inlines;
use super::layout::{Span, spans_width};

struct Cell {
    spans: Vec<Span>,
    width: usize,
}

pub(super) fn render_table(r: &mut Renderer<'_>, node: Node<'_>, table: &NodeTable) {
    let rows: Vec<(bool, Vec<Cell>)> = node
        .children()
        .map(|row| {
            let is_header = matches!(row.data().value, NodeValue::TableRow(true));
            let base = if is_header {
                r.theme.table_header
            } else {
                Style::new()
            };
            let cells = row
                .children()
                .map(|cell| {
                    let spans = single_line(render_inlines(cell, r.theme, base));
                    Cell {
                        width: spans_width(&spans),
                        spans,
                    }
                })
                .collect();
            (is_header, cells)
        })
        .collect();

    let num_columns = table
        .num_columns
        .max(rows.iter().map(|(_, cells)| cells.len()).max().unwrap_or(0));
    let mut widths = vec![0; num_columns];
    for (_, cells) in &rows {
        for (column, cell) in cells.iter().enumerate() {
            widths[column] = widths[column].max(cell.width);
        }
    }

    let border = r.theme.table_border;
    let separator = [
        Span::plain(" "),
        Span::new(r.glyphs.vertical().to_string(), border),
        Span::plain(" "),
    ];
    let rule_glyph = r.glyphs.rule().to_string();
    let rule_joint = format!("{rule_glyph}{}{rule_glyph}", r.glyphs.cross());

    for (is_header, cells) in &rows {
        let mut line = Vec::new();
        for (column, &width) in widths.iter().enumerate() {
            if column > 0 {
                line.extend(separator.iter().cloned());
            }
            let alignment = table
                .alignments
                .get(column)
                .copied()
                .unwrap_or(TableAlignment::None);
            let (spans, cell_width) = match cells.get(column) {
                Some(cell) => (cell.spans.as_slice(), cell.width),
                None => (&[][..], 0),
            };
            let padding = width.saturating_sub(cell_width);
            let (left, right) = match alignment {
                TableAlignment::None | TableAlignment::Left => (0, padding),
                TableAlignment::Right => (padding, 0),
                TableAlignment::Center => (padding / 2, padding - padding / 2),
            };
            if left > 0 {
                line.push(Span::plain(" ".repeat(left)));
            }
            line.extend(spans.iter().cloned());
            if right > 0 {
                line.push(Span::plain(" ".repeat(right)));
            }
        }
        r.writer.line(&line);

        if *is_header {
            let mut rule = Vec::new();
            for (column, &width) in widths.iter().enumerate() {
                if column > 0 {
                    rule.push(Span::new(rule_joint.clone(), border));
                }
                rule.push(Span::new(rule_glyph.repeat(width), border));
            }
            r.writer.line(&rule);
        }
    }
}

/// Table cells are a single line: any line break inside becomes a space.
fn single_line(mut spans: Vec<Span>) -> Vec<Span> {
    for span in &mut spans {
        if span.text.contains('\n') {
            span.text = span.text.replace('\n', " ");
        }
    }
    spans
}
