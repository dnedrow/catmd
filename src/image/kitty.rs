//! Kitty Graphics Protocol backend.

use std::borrow::Cow;
use std::num::NonZeroU32;

use ::image::RgbaImage;
use kittage::action::Action;
use kittage::display::{CursorMovementPolicy, DisplayConfig};
use kittage::image::Image;
use kittage::medium::{ChunkSize, Medium};
use kittage::{ImageDimensions, NumberOrId, PixelFormat, Verbosity};

use super::{BackendKind, Encoded, ImageBackend, Placement};

/// Transmits RGBA pixels inline and displays them at the cursor at their
/// natural pixel size, leaving the cursor where it was. The terminal is told
/// to stay silent so no replies leak into the shell's input.
pub struct Kitty;

impl ImageBackend for Kitty {
    fn kind(&self) -> BackendKind {
        BackendKind::Kitty
    }

    fn encode(&self, image: &RgbaImage, _placement: &Placement) -> Encoded {
        let action = Action::TransmitAndDisplay {
            image: Image {
                num_or_id: NumberOrId::Number(NonZeroU32::MIN),
                format: PixelFormat::Rgba32(
                    ImageDimensions {
                        width: image.width(),
                        height: image.height(),
                    },
                    None,
                ),
                medium: Medium::Direct {
                    chunk_size: Some(ChunkSize::default()),
                    data: Cow::Borrowed(image.as_raw()),
                },
            },
            config: DisplayConfig {
                cursor_movement: CursorMovementPolicy::DontMove,
                ..DisplayConfig::default()
            },
            placement_id: None,
        };
        let mut out = Vec::new();
        match action.write_transmit_to(&mut out, Verbosity::Silent) {
            Ok(_) => {
                // kittage marks only the first chunk quiet, and the terminal
                // answers on the last one, so silence every continuation too.
                let text = String::from_utf8_lossy(&out).replace("\x1b_Gm=", "\x1b_Gq=2,m=");
                Encoded::Graphics(text)
            }
            Err(_) => Encoded::Graphics(String::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::{CellSize, plan};

    #[test]
    fn emits_a_silent_transmit_and_display_without_moving_the_cursor() {
        let image = RgbaImage::from_pixel(2, 2, ::image::Rgba([255, 0, 0, 255]));
        let placement = plan((2, 2), 10, CellSize::DEFAULT);
        let Encoded::Graphics(out) = Kitty.encode(&image, &placement) else {
            panic!("kitty emits graphics");
        };
        assert!(out.starts_with("\x1b_G"), "{out:?}");
        assert!(out.ends_with("\x1b\\"), "{out:?}");
        assert!(out.contains("C=1"), "{out:?}");
        assert!(out.contains("q=2"), "{out:?}");
        assert!(out.contains("f=32"), "{out:?}");
    }

    #[test]
    fn large_images_are_chunked() {
        let image = RgbaImage::new(200, 200);
        let placement = plan((200, 200), 40, CellSize::DEFAULT);
        let Encoded::Graphics(out) = Kitty.encode(&image, &placement) else {
            panic!("kitty emits graphics");
        };
        assert!(out.matches("\x1b_G").count() > 1);
        assert!(out.contains("m=1"));
        assert!(out.contains("\x1b_Gq=2,m=0;"), "{out:?}");
        assert!(!out.contains("\x1b_Gm="), "{out:?}");
    }
}
