//! SIXEL backend.

use ::image::RgbaImage;
use icy_sixel::SixelImage;

use super::{BackendKind, Encoded, ImageBackend, Placement};

/// Encodes the image as a SIXEL data stream (`DCS ... ST`).
pub struct Sixel;

impl ImageBackend for Sixel {
    fn kind(&self) -> BackendKind {
        BackendKind::Sixel
    }

    fn encode(&self, image: &RgbaImage, _placement: &Placement) -> Encoded {
        let pixels = SixelImage::from_rgba(
            image.as_raw().clone(),
            image.width() as usize,
            image.height() as usize,
        );
        // On an encoding failure the reserved rows are simply left blank.
        Encoded::Graphics(pixels.encode().unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::{CellSize, plan};

    #[test]
    fn emits_a_dcs_sixel_stream() {
        let image = RgbaImage::from_pixel(8, 12, ::image::Rgba([0, 128, 255, 255]));
        let placement = plan((8, 12), 10, CellSize::DEFAULT);
        let Encoded::Graphics(out) = Sixel.encode(&image, &placement) else {
            panic!("sixel emits graphics");
        };
        assert!(out.starts_with("\x1bP"), "{out:?}");
        assert!(out.contains('q'), "{out:?}");
        assert!(out.ends_with("\x1b\\"), "{out:?}");
    }
}
