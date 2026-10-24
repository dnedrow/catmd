//! Input sources: files and standard input.

use std::fmt;
use std::io::{self, Read};
use std::path::PathBuf;

use crate::Settings;

/// One Markdown input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    Stdin,
    File(PathBuf),
}

impl Source {
    /// Resolve positional arguments: no arguments means standard input, and a
    /// `-` argument means standard input at that position.
    pub fn from_args(files: &[PathBuf]) -> Vec<Source> {
        if files.is_empty() {
            return vec![Source::Stdin];
        }
        files
            .iter()
            .map(|path| {
                if path.as_os_str() == "-" {
                    Source::Stdin
                } else {
                    Source::File(path.clone())
                }
            })
            .collect()
    }

    /// Read the whole source, decoding it as UTF-8 and replacing invalid
    /// sequences with U+FFFD.
    pub fn read(&self) -> io::Result<String> {
        let bytes = match self {
            Source::Stdin => {
                let mut buffer = Vec::new();
                io::stdin().lock().read_to_end(&mut buffer)?;
                buffer
            }
            Source::File(path) => std::fs::read(path)?,
        };
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Source::Stdin => f.write_str("-"),
            Source::File(path) => write!(f, "{}", path.display()),
        }
    }
}

/// Read and render every source in order as an independent document.
///
/// Non-empty renderings are joined by a single blank line. A source that
/// cannot be read is passed to `report` and skipped; the remaining sources are
/// still rendered. Returns the combined output and whether any source failed.
pub fn render_all(
    sources: &[Source],
    settings: &Settings,
    report: &mut dyn FnMut(&Source, &io::Error),
) -> (String, bool) {
    let mut output = String::new();
    let mut failed = false;
    for source in sources {
        let markdown = match source.read() {
            Ok(markdown) => markdown,
            Err(err) => {
                report(source, &err);
                failed = true;
                continue;
            }
        };
        let base = match source {
            Source::File(path) => path
                .parent()
                .map_or_else(|| PathBuf::from("."), PathBuf::from),
            Source::Stdin => PathBuf::from("."),
        };
        let rendered = crate::render::render_in(&markdown, settings, &base);
        if rendered.is_empty() {
            continue;
        }
        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str(&rendered);
    }
    (output, failed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_arguments_means_stdin() {
        assert_eq!(Source::from_args(&[]), vec![Source::Stdin]);
    }

    #[test]
    fn dash_maps_to_stdin_in_place() {
        let args = [
            PathBuf::from("a.md"),
            PathBuf::from("-"),
            PathBuf::from("b.md"),
        ];
        assert_eq!(
            Source::from_args(&args),
            vec![
                Source::File("a.md".into()),
                Source::Stdin,
                Source::File("b.md".into()),
            ]
        );
    }

    #[test]
    fn display_names_the_path_or_dash() {
        assert_eq!(Source::Stdin.to_string(), "-");
        assert_eq!(Source::File("docs/x.md".into()).to_string(), "docs/x.md");
    }
}
