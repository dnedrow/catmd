//! Unicode half-block backend: two vertically stacked pixels per cell.

use ::image::RgbaImage;
use anstyle::{Color, RgbColor, Style};

use super::{BackendKind, CellSize, Encoded, ImageBackend, Placement, plan as fit};
use crate::theme::color::{ColorDepth, clamp};

const UPPER_HALF: char = '▀';
/// Shading ramp used when color output is off, darkest first.
const RAMP: [char; 5] = [' ', '░', '▒', '▓', '█'];

/// Draws each cell as an upper half block whose foreground is the top pixel
/// and background the bottom pixel, reduced to the active color depth.
pub struct HalfBlock {
    pub depth: ColorDepth,
}

impl ImageBackend for HalfBlock {
    fn kind(&self) -> BackendKind {
        BackendKind::HalfBlock
    }

    /// One pixel per column and two per row, matched to the cell's shape.
    fn plan(&self, natural: (u32, u32), avail_cols: usize, cell: CellSize) -> Placement {
        let fitted = fit(natural, avail_cols, cell);
        let cols = (((fitted.px_width + cell.width / 2) / cell.width) as usize)
            .clamp(1, avail_cols.max(1));
        let half = (cell.height / 2).max(1);
        let sub_rows = ((fitted.px_height + half / 2) / half).max(1);
        let rows = sub_rows.div_ceil(2) as usize;
        Placement {
            cols,
            rows,
            px_width: cols as u32,
            px_height: rows as u32 * 2,
        }
    }

    fn encode(&self, image: &RgbaImage, placement: &Placement) -> Encoded {
        let pixel = |x: usize, y: usize| -> [u8; 3] {
            let [r, g, b, a] = image.get_pixel(x as u32, y as u32).0;
            let over_black = |c: u8| (u16::from(c) * u16::from(a) / 255) as u8;
            [over_black(r), over_black(g), over_black(b)]
        };
        let rows = (0..placement.rows)
            .map(|row| {
                let mut line = String::new();
                for col in 0..placement.cols {
                    let top = pixel(col, row * 2);
                    let bottom = pixel(col, row * 2 + 1);
                    if self.depth.is_on() {
                        let style = Style::new()
                            .fg_color(Some(self.color(top)))
                            .bg_color(Some(self.color(bottom)));
                        line.push_str(&style.render().to_string());
                        line.push(UPPER_HALF);
                    } else {
                        let luma = |[r, g, b]: [u8; 3]| {
                            (299 * u32::from(r) + 587 * u32::from(g) + 114 * u32::from(b)) / 1000
                        };
                        let level = (luma(top) + luma(bottom)) / 2;
                        line.push(RAMP[(level * 4 / 255) as usize]);
                    }
                }
                if self.depth.is_on() {
                    line.push_str("\x1b[0m");
                }
                line
            })
            .collect();
        Encoded::Cells(rows)
    }
}

impl HalfBlock {
    fn color(&self, [r, g, b]: [u8; 3]) -> Color {
        clamp(Color::Rgb(RgbColor(r, g, b)), self.depth)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::image::Rgba;

    fn truecolor() -> HalfBlock {
        HalfBlock {
            depth: ColorDepth::TrueColor,
        }
    }

    #[test]
    fn two_vertical_pixels_become_one_cell() {
        let mut image = RgbaImage::new(1, 2);
        image.put_pixel(0, 0, Rgba([255, 0, 0, 255]));
        image.put_pixel(0, 1, Rgba([0, 0, 255, 255]));
        let backend = truecolor();
        let placement = backend.plan((1, 2), 10, CellSize::DEFAULT);
        assert_eq!((placement.cols, placement.rows), (1, 1));
        assert_eq!(
            backend.encode(&image, &placement),
            Encoded::Cells(vec![
                "\x1b[38;2;255;0;0m\x1b[48;2;0;0;255m▀\x1b[0m".to_string()
            ])
        );
    }

    #[test]
    fn wide_images_fit_the_columns_with_the_cell_aspect() {
        let backend = truecolor();
        let placement = backend.plan((1920, 1080), 80, CellSize::DEFAULT);
        assert_eq!(placement.cols, 80);
        assert_eq!(placement.rows, 23);
        assert_eq!(placement.px_height, 46);
    }

    #[test]
    fn rows_are_exactly_the_placement_width() {
        let image = RgbaImage::from_pixel(6, 4, Rgba([10, 20, 30, 255]));
        let backend = truecolor();
        let placement = backend.plan((48, 32), 6, CellSize::DEFAULT);
        let Encoded::Cells(rows) = backend.encode(&image, &placement) else {
            panic!("half-block emits cells");
        };
        for row in rows {
            let visible = crate::render::layout::strip_ansi(&row);
            assert_eq!(visible.chars().count(), placement.cols);
        }
    }

    #[test]
    fn colorless_output_uses_a_shading_ramp() {
        let mut image = RgbaImage::new(2, 2);
        image.put_pixel(0, 0, Rgba([255, 255, 255, 255]));
        image.put_pixel(0, 1, Rgba([255, 255, 255, 255]));
        image.put_pixel(1, 0, Rgba([0, 0, 0, 255]));
        image.put_pixel(1, 1, Rgba([0, 0, 0, 255]));
        let backend = HalfBlock {
            depth: ColorDepth::Off,
        };
        let placement = Placement {
            cols: 2,
            rows: 1,
            px_width: 2,
            px_height: 2,
        };
        assert_eq!(
            backend.encode(&image, &placement),
            Encoded::Cells(vec!["█ ".to_string()])
        );
    }
}
