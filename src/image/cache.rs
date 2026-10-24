//! Two-tier image cache.
//!
//! Tier one holds decoded pixels per source path and survives any change in
//! layout. Tier two holds backend output keyed by source, placement, cell size
//! and backend, so a new width, cell size, or backend misses only this tier.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use ::image::RgbaImage;
use ::image::imageops::{FilterType, resize};

use super::load::{self, LoadError};
use super::{BackendKind, CellSize, Encoded, ImageBackend, Placement};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct EncodeKey {
    path: PathBuf,
    placement: Placement,
    cell: CellSize,
    backend: BackendKind,
}

/// A fitted, encoded image ready to emit.
#[derive(Debug, Clone)]
pub struct Rendered {
    pub natural: (u32, u32),
    pub placement: Placement,
    pub encoded: Arc<Encoded>,
}

#[derive(Default)]
pub struct ImageCache {
    decoded: HashMap<PathBuf, Arc<RgbaImage>>,
    encoded: HashMap<EncodeKey, Arc<Encoded>>,
    /// Files decoded so far.
    pub decodes: usize,
    /// Placements encoded so far.
    pub encodes: usize,
}

impl ImageCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Decoded pixels for `path`, decoding on first use.
    pub fn decoded(&mut self, path: &Path) -> Result<Arc<RgbaImage>, LoadError> {
        if let Some(image) = self.decoded.get(path) {
            return Ok(Arc::clone(image));
        }
        let image = Arc::new(load::decode(path)?);
        self.decodes += 1;
        self.decoded.insert(path.to_path_buf(), Arc::clone(&image));
        Ok(image)
    }

    /// Fit the image at `path` into `avail_cols` and encode it with `backend`.
    pub fn render(
        &mut self,
        path: &Path,
        backend: &dyn ImageBackend,
        avail_cols: usize,
        cell: CellSize,
    ) -> Result<Rendered, LoadError> {
        let image = self.decoded(path)?;
        let natural = (image.width(), image.height());
        let placement = backend.plan(natural, avail_cols, cell);
        let key = EncodeKey {
            path: path.to_path_buf(),
            placement,
            cell,
            backend: backend.kind(),
        };
        let encoded = match self.encoded.get(&key) {
            Some(encoded) => Arc::clone(encoded),
            None => {
                let fitted = ((placement.px_width, placement.px_height) != natural).then(|| {
                    resize(
                        &*image,
                        placement.px_width,
                        placement.px_height,
                        FilterType::Lanczos3,
                    )
                });
                let source: &RgbaImage = fitted.as_ref().unwrap_or(&image);
                let encoded = Arc::new(backend.encode(source, &placement));
                self.encodes += 1;
                self.encoded.insert(key, Arc::clone(&encoded));
                encoded
            }
        };
        Ok(Rendered {
            natural,
            placement,
            encoded,
        })
    }

    /// Drop every encoded entry, keeping decoded pixels (e.g. after the
    /// terminal geometry changed).
    pub fn invalidate_encoded(&mut self) {
        self.encoded.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::plan;

    struct Counting(BackendKind);

    impl ImageBackend for Counting {
        fn kind(&self) -> BackendKind {
            self.0
        }

        fn encode(&self, image: &RgbaImage, placement: &Placement) -> Encoded {
            Encoded::Graphics(format!(
                "{}x{}/{}",
                image.width(),
                image.height(),
                placement.cols
            ))
        }
    }

    fn fixture(name: &str, width: u32, height: u32) -> PathBuf {
        let path = std::env::temp_dir().join(format!("catmd-{name}-{}.png", std::process::id()));
        RgbaImage::new(width, height).save(&path).unwrap();
        path
    }

    const CELL: CellSize = CellSize::DEFAULT;

    #[test]
    fn repeated_images_decode_and_encode_once() {
        let path = fixture("repeat", 200, 100);
        let mut cache = ImageCache::new();
        let backend = Counting(BackendKind::Kitty);
        for _ in 0..3 {
            cache.render(&path, &backend, 40, CELL).unwrap();
        }
        std::fs::remove_file(&path).unwrap();
        assert_eq!((cache.decodes, cache.encodes), (1, 1));
    }

    #[test]
    fn a_new_width_reencodes_but_reuses_decoded_pixels() {
        let path = fixture("resize", 400, 200);
        let mut cache = ImageCache::new();
        let backend = Counting(BackendKind::Kitty);
        let wide = cache.render(&path, &backend, 80, CELL).unwrap();
        let narrow = cache.render(&path, &backend, 20, CELL).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!((cache.decodes, cache.encodes), (1, 2));
        assert_eq!(wide.placement, plan((400, 200), 80, CELL));
        assert_eq!(*narrow.encoded, Encoded::Graphics("160x80/20".to_string()));
    }

    #[test]
    fn a_new_backend_or_cell_size_misses_only_the_encoded_tier() {
        let path = fixture("backend", 64, 64);
        let mut cache = ImageCache::new();
        cache
            .render(&path, &Counting(BackendKind::Kitty), 40, CELL)
            .unwrap();
        cache
            .render(&path, &Counting(BackendKind::Sixel), 40, CELL)
            .unwrap();
        let big_cell = CellSize::new(10, 20).unwrap();
        cache
            .render(&path, &Counting(BackendKind::Sixel), 40, big_cell)
            .unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!((cache.decodes, cache.encodes), (1, 3));
    }

    #[test]
    fn invalidating_drops_only_encoded_output() {
        let path = fixture("invalidate", 64, 64);
        let mut cache = ImageCache::new();
        let backend = Counting(BackendKind::Kitty);
        cache.render(&path, &backend, 40, CELL).unwrap();
        cache.invalidate_encoded();
        cache.render(&path, &backend, 40, CELL).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!((cache.decodes, cache.encodes), (1, 2));
    }

    #[test]
    fn failures_are_not_cached_as_success() {
        let mut cache = ImageCache::new();
        let backend = Counting(BackendKind::Kitty);
        let missing = Path::new("/nonexistent/x.png");
        assert!(cache.render(missing, &backend, 40, CELL).is_err());
        assert_eq!(cache.decodes, 0);
    }
}
