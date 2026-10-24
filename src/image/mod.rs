//! Inline images: a protocol-independent layer over terminal image protocols.
//!
//! Layout depends only on an image's natural size, the available columns, and
//! the terminal cell size ([`plan`]); each [`ImageBackend`] turns a planned
//! placement into protocol-specific output. Backends are chosen by
//! [`detect::select`] in a fixed order: Kitty, SIXEL, half-block, placeholder.

pub mod cache;
pub mod detect;
pub mod halfblock;
pub mod kitty;
pub mod load;
pub mod placeholder;
pub mod probe;
pub mod sixel;

use ::image::RgbaImage;

use crate::theme::color::ColorDepth;

/// Pixel size of one terminal cell, used to turn pixels into columns and rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CellSize {
    pub width: u32,
    pub height: u32,
}

impl CellSize {
    /// Assumed when the terminal does not report its cell size.
    pub const DEFAULT: CellSize = CellSize {
        width: 8,
        height: 16,
    };

    /// A cell size, or `None` when either dimension is zero.
    pub fn new(width: u32, height: u32) -> Option<Self> {
        (width > 0 && height > 0).then_some(Self { width, height })
    }
}

impl Default for CellSize {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The ways an image can be shown, in selection-preference order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BackendKind {
    Kitty,
    Sixel,
    HalfBlock,
    /// A text box describing the image; never draws pixels.
    Placeholder,
}

impl BackendKind {
    /// Whether the backend draws pixels with terminal graphics escapes.
    pub fn is_graphical(self) -> bool {
        matches!(self, BackendKind::Kitty | BackendKind::Sixel)
    }
}

/// Resolved image settings carried in [`crate::Settings`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ImageSettings {
    /// The active backend; `None` keeps the plain `[image: alt] (src)` label.
    pub backend: Option<BackendKind>,
    pub cell: CellSize,
}

/// Where and how large an image is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Placement {
    /// Whole columns occupied (never more than the available width).
    pub cols: usize,
    /// Whole rows occupied.
    pub rows: usize,
    /// Pixel size the image is resampled to before encoding.
    pub px_width: u32,
    pub px_height: u32,
}

/// Protocol-specific output for one placement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Encoded {
    /// Terminal graphics escapes, drawn at the cursor without moving it.
    Graphics(String),
    /// Ready-to-print text rows, each exactly `Placement::cols` cells wide.
    Cells(Vec<String>),
}

/// A way of drawing a decoded image in the terminal.
pub trait ImageBackend: Send + Sync {
    fn kind(&self) -> BackendKind;

    /// Fit an image of `natural` pixels into `avail_cols` columns.
    fn plan(&self, natural: (u32, u32), avail_cols: usize, cell: CellSize) -> Placement {
        plan(natural, avail_cols, cell)
    }

    /// Encode `image` (already resampled to the placement's pixel size).
    fn encode(&self, image: &RgbaImage, placement: &Placement) -> Encoded;
}

/// The backend for `kind`, or `None` for the placeholder, which draws no pixels.
pub fn backend_for(kind: BackendKind, depth: ColorDepth) -> Option<Box<dyn ImageBackend>> {
    match kind {
        BackendKind::Kitty => Some(Box::new(kitty::Kitty)),
        BackendKind::Sixel => Some(Box::new(sixel::Sixel)),
        BackendKind::HalfBlock => Some(Box::new(halfblock::HalfBlock { depth })),
        BackendKind::Placeholder => None,
    }
}

/// Fit `natural` pixels into `avail_cols` columns: scale down proportionally
/// when wider than the area, never scale up.
pub fn plan(natural: (u32, u32), avail_cols: usize, cell: CellSize) -> Placement {
    let (width, height) = (natural.0.max(1), natural.1.max(1));
    let avail_cols = avail_cols.max(1);
    let max_px = avail_cols as u64 * u64::from(cell.width);
    let (px_width, px_height) = if u64::from(width) <= max_px {
        (width, height)
    } else {
        let scaled = (u64::from(height) * max_px + u64::from(width) / 2) / u64::from(width);
        (max_px as u32, (scaled as u32).max(1))
    };
    let cols = (px_width.div_ceil(cell.width) as usize).clamp(1, avail_cols);
    let rows = px_height.div_ceil(cell.height).max(1) as usize;
    Placement {
        cols,
        rows,
        px_width,
        px_height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CELL: CellSize = CellSize::DEFAULT;

    struct Fake;

    impl ImageBackend for Fake {
        fn kind(&self) -> BackendKind {
            BackendKind::Kitty
        }

        fn encode(&self, image: &RgbaImage, placement: &Placement) -> Encoded {
            Encoded::Graphics(format!(
                "{}x{}@{}x{}",
                image.width(),
                image.height(),
                placement.cols,
                placement.rows
            ))
        }
    }

    #[test]
    fn a_backend_plans_and_encodes_through_the_trait() {
        let backend: Box<dyn ImageBackend> = Box::new(Fake);
        let placement = backend.plan((80, 32), 40, CELL);
        let image = RgbaImage::new(placement.px_width, placement.px_height);
        assert_eq!(
            backend.encode(&image, &placement),
            Encoded::Graphics("80x32@10x2".to_string())
        );
    }

    #[test]
    fn narrow_images_keep_their_natural_size() {
        let p = plan((100, 50), 80, CELL);
        assert_eq!((p.px_width, p.px_height), (100, 50));
        assert_eq!((p.cols, p.rows), (13, 4));
    }

    #[test]
    fn wide_images_scale_down_to_the_available_columns() {
        let p = plan((1920, 1080), 80, CELL);
        assert_eq!((p.px_width, p.px_height), (640, 360));
        assert_eq!((p.cols, p.rows), (80, 23));
    }

    #[test]
    fn scaling_preserves_aspect_ratio_for_extreme_shapes() {
        let tall = plan((100, 4000), 5, CELL);
        assert_eq!((tall.px_width, tall.px_height), (40, 1600));
        assert_eq!((tall.cols, tall.rows), (5, 100));
        let flat = plan((4000, 10), 10, CELL);
        assert_eq!((flat.px_width, flat.px_height), (80, 1));
        assert_eq!((flat.cols, flat.rows), (10, 1));
    }

    #[test]
    fn placement_never_exceeds_the_available_columns() {
        for avail in 1..40 {
            for width in [1, 7, 8, 9, 300, 5000] {
                let p = plan((width, 123), avail, CELL);
                assert!(p.cols <= avail, "{width} into {avail}: {p:?}");
            }
        }
    }
}
