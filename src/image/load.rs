//! Locating and decoding image sources.

use std::fmt;
use std::path::{Path, PathBuf};

use ::image::{ImageReader, RgbaImage};

/// Why an image source could not be turned into pixels.
#[derive(Debug)]
pub enum LoadError {
    /// `http(s)`, `data:` and other non-file sources are not fetched.
    Unsupported,
    Io(std::io::Error),
    Decode(::image::ImageError),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Unsupported => f.write_str("not a local file"),
            LoadError::Io(err) => err.fmt(f),
            LoadError::Decode(err) => err.fmt(f),
        }
    }
}

/// Resolve an image `url` from a document to a local path. Relative paths are
/// relative to `base`, the directory of the Markdown file.
pub fn resolve(url: &str, base: &Path) -> Result<PathBuf, LoadError> {
    let url = url.trim();
    if url.is_empty() || url.starts_with("data:") || is_remote(url) {
        return Err(LoadError::Unsupported);
    }
    let path = Path::new(url);
    Ok(if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    })
}

/// A URL with a scheme (`https://`, `file://`, ...).
fn is_remote(url: &str) -> bool {
    url.split_once("://").is_some_and(|(scheme, _)| {
        !scheme.is_empty()
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
    })
}

/// Decode the file at `path` to RGBA pixels.
pub fn decode(path: &Path) -> Result<RgbaImage, LoadError> {
    let reader = ImageReader::open(path)
        .map_err(LoadError::Io)?
        .with_guessed_format()
        .map_err(LoadError::Io)?;
    Ok(reader.decode().map_err(LoadError::Decode)?.to_rgba8())
}

/// Read only the pixel dimensions of the file at `path`.
pub fn dimensions(path: &Path) -> Result<(u32, u32), LoadError> {
    let reader = ImageReader::open(path)
        .map_err(LoadError::Io)?
        .with_guessed_format()
        .map_err(LoadError::Io)?;
    reader.into_dimensions().map_err(LoadError::Decode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_paths_resolve_against_the_base() {
        assert_eq!(
            resolve("img/x.png", Path::new("docs")).unwrap(),
            PathBuf::from("docs/img/x.png")
        );
    }

    #[test]
    fn absolute_paths_are_kept() {
        let abs = std::env::temp_dir().join("x.png");
        assert_eq!(
            resolve(abs.to_str().unwrap(), Path::new("docs")).unwrap(),
            abs
        );
    }

    #[test]
    fn remote_and_data_sources_are_unsupported() {
        for url in [
            "https://example.com/a.png",
            "http://x/y.png",
            "data:image/png;base64,AAAA",
            "file:///tmp/a.png",
            "",
        ] {
            assert!(
                matches!(resolve(url, Path::new(".")), Err(LoadError::Unsupported)),
                "{url}"
            );
        }
    }

    #[test]
    fn missing_files_fail_with_io() {
        let missing = Path::new("/nonexistent/catmd/none.png");
        assert!(matches!(decode(missing), Err(LoadError::Io(_))));
        assert!(dimensions(missing).is_err());
    }

    #[test]
    fn undecodable_files_fail_with_decode() {
        let path = std::env::temp_dir().join(format!("catmd-bad-{}.png", std::process::id()));
        std::fs::write(&path, b"not an image").unwrap();
        let result = decode(&path);
        std::fs::remove_file(&path).unwrap();
        assert!(matches!(result, Err(LoadError::Decode(_))));
    }
}
