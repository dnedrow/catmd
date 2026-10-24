//! Image blocks: a paragraph holding nothing but an image becomes a layout
//! element that occupies a fixed number of rows of the document.

use std::path::Path;
use std::sync::{LazyLock, Mutex};

use anstyle::Style;
use comrak::nodes::{Node, NodeLink, NodeValue};

use super::Renderer;
use super::inline::render_inlines;
use super::layout::spans_text;
use crate::image::cache::ImageCache;
use crate::image::{BackendKind, Encoded, backend_for, load, placeholder};

/// Decoded and encoded images shared by every render in the process.
static CACHE: LazyLock<Mutex<ImageCache>> = LazyLock::new(|| Mutex::new(ImageCache::new()));

/// The image of a paragraph that contains only that image.
pub(super) fn sole_image<'a>(paragraph: Node<'a>) -> Option<Node<'a>> {
    let first = paragraph.first_child()?;
    let only = first.next_sibling().is_none();
    (only && matches!(first.data().value, NodeValue::Image(_))).then_some(first)
}

impl Renderer<'_> {
    /// Draw `node` as an image block. Returns `false` when images are off, so
    /// the caller keeps the plain label.
    pub(super) fn image_block(&mut self, node: Node<'_>, link: &NodeLink) -> bool {
        let Some(kind) = self.images.backend else {
            return false;
        };
        let alt = spans_text(&render_inlines(node, self.theme, Style::new()));
        let available = self.writer.available_width();
        let rendered = self.try_render(kind, &link.url, available);
        match rendered {
            Ok((encoded, cols, rows)) => match &*encoded {
                Encoded::Cells(lines) => {
                    for line in lines {
                        self.writer.raw_cells(line);
                    }
                }
                Encoded::Graphics(escape) => {
                    self.graphics(kind, escape, cols, rows);
                }
            },
            Err(dimensions) => {
                for line in placeholder::lines(&alt, &link.url, dimensions, available, self.glyphs)
                {
                    self.writer.raw_cells(&line);
                }
            }
        }
        true
    }

    /// Encode the image, or report the natural size when known and the image
    /// could not be shown.
    #[allow(clippy::type_complexity)]
    fn try_render(
        &self,
        kind: BackendKind,
        url: &str,
        available: usize,
    ) -> Result<(std::sync::Arc<Encoded>, usize, usize), Option<(u32, u32)>> {
        let path = load::resolve(url, &self.base).map_err(|_| None)?;
        let Some(backend) = backend_for(kind, self.depth) else {
            return Err(load::dimensions(&path).ok());
        };
        let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
        match cache.render(&path, backend.as_ref(), available, self.images.cell) {
            Ok(rendered) => Ok((
                rendered.encoded,
                rendered.placement.cols,
                rendered.placement.rows,
            )),
            Err(_) => Err(load::dimensions(Path::new(&path)).ok()),
        }
    }

    /// Emit a graphics image: the escape rides on the first row and the rest
    /// of the rows are blank so the image's height is part of document flow.
    fn graphics(&mut self, kind: BackendKind, escape: &str, _cols: usize, rows: usize) {
        let indent = self.writer.indent_width();
        let suffix = match kind {
            // SIXEL moves the cursor, so reserve the rows first, climb back
            // to the first row, and restore the cursor afterwards.
            BackendKind::Sixel => {
                let climb = rows - 1;
                let mut seq = "\n".repeat(climb);
                if climb > 0 {
                    seq.push_str(&format!("\x1b[{climb}A"));
                }
                seq.push_str(&format!("\x1b[{}G\x1b7{escape}\x1b8", indent + 1));
                seq
            }
            _ => escape.to_string(),
        };
        self.writer.raw_cells(&suffix);
        for _ in 1..rows {
            self.writer.raw_cells("");
        }
    }
}
