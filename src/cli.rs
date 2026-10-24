//! Command-line argument definitions.

use std::num::NonZeroUsize;
use std::path::PathBuf;

use clap::{Parser, ValueEnum};

/// `cat` for Markdown: render Markdown files as styled text in the terminal.
#[derive(Parser, Debug, Clone)]
#[command(name = "catmd", version, about, long_about = None)]
pub struct Args {
    /// Markdown files to render, in order. Use `-` for standard input.
    /// Reads standard input when no files are given.
    #[arg(value_name = "FILE")]
    pub files: Vec<PathBuf>,

    /// When to emit ANSI styling.
    #[arg(long, value_enum, default_value_t = ColorMode::Auto, value_name = "WHEN")]
    pub color: ColorMode,

    /// How many colors to use (default: detected from COLORTERM and TERM).
    #[arg(long, value_enum, default_value_t = DepthArg::Auto, value_name = "DEPTH")]
    pub color_depth: DepthArg,

    /// Rendering width in columns (default: terminal width, COLUMNS, or 80).
    #[arg(long, value_name = "N")]
    pub width: Option<NonZeroUsize>,

    /// Terminal height in rows used to decide when to page (default: terminal height or LINES).
    #[arg(long, value_name = "N")]
    pub height: Option<NonZeroUsize>,

    /// Theme file path, or a theme name under $XDG_CONFIG_HOME/catmd/themes/
    /// (default: $CATMD_THEME, else the built-in theme).
    #[arg(long, value_name = "THEME")]
    pub theme: Option<String>,

    /// Use ASCII glyphs for bullets, rules, and table borders.
    #[arg(long)]
    pub ascii: bool,

    /// Never hand output to a pager.
    #[arg(long)]
    pub no_pager: bool,

    /// How to show images (default: $CATMD_IMAGES, else detected from the terminal).
    #[arg(long, value_enum, value_name = "MODE")]
    pub images: Option<ImagesArg>,
}

/// The `--images` mode.
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImagesArg {
    /// Pick the best backend the terminal supports: kitty, sixel, then blocks.
    #[default]
    Auto,
    /// Kitty Graphics Protocol.
    Kitty,
    /// SIXEL graphics.
    Sixel,
    /// Unicode half-block cells.
    Blocks,
    /// A text box describing each image.
    Placeholder,
    /// The plain `[image: alt] (src)` label.
    None,
}

/// The `--color` mode.
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorMode {
    /// Style output only when standard output is a color-capable terminal.
    #[default]
    Auto,
    /// Always emit styling, even into a pipe.
    Always,
    /// Never emit styling.
    Never,
}

/// The `--color-depth` override.
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DepthArg {
    /// Detect from `COLORTERM` and `TERM`.
    #[default]
    Auto,
    /// The 16 standard ANSI colors.
    #[value(name = "16")]
    Ansi16,
    /// The 256-color xterm palette.
    #[value(name = "256")]
    Ansi256,
    /// 24-bit RGB colors.
    #[value(name = "truecolor")]
    TrueColor,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            files: Vec::new(),
            color: ColorMode::Auto,
            color_depth: DepthArg::Auto,
            theme: None,
            width: None,
            height: None,
            ascii: false,
            no_pager: false,
            images: None,
        }
    }
}
