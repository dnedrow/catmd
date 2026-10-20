//! Color depth, color parsing, and palette downgrading.

use std::collections::BTreeMap;
use std::fmt;

use anstyle::{Ansi256Color, AnsiColor, Color, Effects, RgbColor};

/// How many colors the output may use. `Off` means no styling at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ColorDepth {
    Off,
    Ansi16,
    Ansi256,
    TrueColor,
}

impl ColorDepth {
    /// Resolve the depth a styled terminal supports from `COLORTERM` and `TERM`
    /// alone: `COLORTERM=truecolor|24bit` or a `TERM` containing `direct` means
    /// 16 million colors, a `TERM` containing `256color` means 256, else 16.
    pub fn detect(colorterm: Option<&str>, term: Option<&str>) -> ColorDepth {
        let colorterm = colorterm.unwrap_or("").trim().to_ascii_lowercase();
        let term = term.unwrap_or("").trim().to_ascii_lowercase();
        if colorterm == "truecolor" || colorterm == "24bit" || term.contains("direct") {
            ColorDepth::TrueColor
        } else if term.contains("256color") {
            ColorDepth::Ansi256
        } else {
            ColorDepth::Ansi16
        }
    }

    /// Whether any styling is emitted.
    pub fn is_on(self) -> bool {
        self != ColorDepth::Off
    }
}

/// Reduce `color` to the nearest color representable at `depth`. ANSI colors
/// are never changed, so the built-in theme renders identically everywhere.
pub fn clamp(color: Color, depth: ColorDepth) -> Color {
    match (depth, color) {
        (ColorDepth::TrueColor, _) | (_, Color::Ansi(_)) => color,
        (ColorDepth::Ansi256, Color::Rgb(rgb)) => Color::Ansi256(anstyle_lossy::rgb_to_xterm(rgb)),
        (ColorDepth::Ansi256, other) => other,
        (ColorDepth::Ansi16 | ColorDepth::Off, other) => Color::Ansi(anstyle_lossy::color_to_ansi(
            other,
            anstyle_lossy::palette::VGA,
        )),
    }
}

/// Named colors declared in a theme's `[palette]` table.
pub type Palette = BTreeMap<String, Color>;

/// Why a theme color value was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColorError {
    /// An integer outside `0..=255`.
    OutOfRange(i64),
    /// A `#` value that is not exactly six hex digits.
    BadHex(String),
    /// A string that is neither a palette entry nor an ANSI color name.
    UnknownName(String),
    /// A TOML value of a type that cannot be a color.
    WrongType(&'static str),
}

impl fmt::Display for ColorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ColorError::OutOfRange(n) => write!(f, "color index {n} is out of range 0-255"),
            ColorError::BadHex(s) => write!(f, "invalid hex color `{s}` (expected #rrggbb)"),
            ColorError::UnknownName(s) => write!(
                f,
                "unknown color `{s}` (expected an ANSI name, 0-255, #rrggbb, or a palette name)"
            ),
            ColorError::WrongType(t) => write!(f, "expected a color, found a {t}"),
        }
    }
}

impl std::error::Error for ColorError {}

/// A modifier name outside `bold`, `dim`, `italic`, `underline`, `strikethrough`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownModifier(pub String);

impl fmt::Display for UnknownModifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown modifier `{}` (expected bold, dim, italic, underline, or strikethrough)",
            self.0
        )
    }
}

impl std::error::Error for UnknownModifier {}

/// Parse a theme color: an integer `0..=255`, a `#rrggbb` string, a palette
/// name (checked before ANSI names), or one of the sixteen ANSI names.
pub fn parse_color(value: &toml::Value, palette: &Palette) -> Result<Color, ColorError> {
    match value {
        toml::Value::Integer(n) => u8::try_from(*n)
            .map(index)
            .map_err(|_| ColorError::OutOfRange(*n)),
        toml::Value::String(s) => parse_color_name(s, palette),
        toml::Value::Float(_) => Err(ColorError::WrongType("float")),
        toml::Value::Boolean(_) => Err(ColorError::WrongType("boolean")),
        toml::Value::Datetime(_) => Err(ColorError::WrongType("datetime")),
        toml::Value::Array(_) => Err(ColorError::WrongType("array")),
        toml::Value::Table(_) => Err(ColorError::WrongType("table")),
    }
}

/// Parse the string form of a color (see [`parse_color`]).
pub fn parse_color_name(name: &str, palette: &Palette) -> Result<Color, ColorError> {
    if let Some(color) = palette.get(name) {
        return Ok(*color);
    }
    if let Some(hex) = name.strip_prefix('#') {
        return parse_hex(hex).ok_or_else(|| ColorError::BadHex(name.to_string()));
    }
    ansi_by_name(name)
        .map(Color::Ansi)
        .ok_or_else(|| ColorError::UnknownName(name.to_string()))
}

fn parse_hex(hex: &str) -> Option<Color> {
    if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let channel = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    Some(rgb(channel(0)?, channel(2)?, channel(4)?))
}

fn ansi_by_name(name: &str) -> Option<AnsiColor> {
    Some(match name {
        "black" => AnsiColor::Black,
        "red" => AnsiColor::Red,
        "green" => AnsiColor::Green,
        "yellow" => AnsiColor::Yellow,
        "blue" => AnsiColor::Blue,
        "magenta" => AnsiColor::Magenta,
        "cyan" => AnsiColor::Cyan,
        "white" => AnsiColor::White,
        "bright_black" => AnsiColor::BrightBlack,
        "bright_red" => AnsiColor::BrightRed,
        "bright_green" => AnsiColor::BrightGreen,
        "bright_yellow" => AnsiColor::BrightYellow,
        "bright_blue" => AnsiColor::BrightBlue,
        "bright_magenta" => AnsiColor::BrightMagenta,
        "bright_cyan" => AnsiColor::BrightCyan,
        "bright_white" => AnsiColor::BrightWhite,
        _ => return None,
    })
}

/// Parse a modifier name into its text effect.
pub fn parse_modifier(name: &str) -> Result<Effects, UnknownModifier> {
    Ok(match name {
        "bold" => Effects::BOLD,
        "dim" => Effects::DIMMED,
        "italic" => Effects::ITALIC,
        "underline" => Effects::UNDERLINE,
        "strikethrough" => Effects::STRIKETHROUGH,
        _ => return Err(UnknownModifier(name.to_string())),
    })
}

/// Convenience constructors used by tests and parsers.
pub fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb(RgbColor(r, g, b))
}

pub fn index(i: u8) -> Color {
    Color::Ansi256(Ansi256Color(i))
}

pub fn ansi(c: AnsiColor) -> Color {
    Color::Ansi(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colorterm_announces_truecolor_in_any_case() {
        for value in ["truecolor", "24bit", "TrueColor", " 24BIT "] {
            assert_eq!(
                ColorDepth::detect(Some(value), Some("xterm-256color")),
                ColorDepth::TrueColor,
                "COLORTERM={value:?}"
            );
        }
    }

    #[test]
    fn term_direct_means_truecolor() {
        assert_eq!(
            ColorDepth::detect(None, Some("xterm-direct")),
            ColorDepth::TrueColor
        );
    }

    #[test]
    fn term_256color_means_256() {
        assert_eq!(
            ColorDepth::detect(None, Some("screen-256color")),
            ColorDepth::Ansi256
        );
        assert_eq!(
            ColorDepth::detect(Some(""), Some("xterm-256color")),
            ColorDepth::Ansi256
        );
    }

    #[test]
    fn plain_or_missing_term_means_16() {
        assert_eq!(ColorDepth::detect(None, Some("xterm")), ColorDepth::Ansi16);
        assert_eq!(ColorDepth::detect(None, None), ColorDepth::Ansi16);
        assert_eq!(
            ColorDepth::detect(Some("yes"), Some("screen")),
            ColorDepth::Ansi16
        );
    }

    #[test]
    fn hex_red_becomes_index_196_at_256_colors() {
        assert_eq!(clamp(rgb(255, 0, 0), ColorDepth::Ansi256), index(196));
    }

    #[test]
    fn hex_and_index_red_become_ansi_red_at_16_colors() {
        for color in [rgb(255, 0, 0), index(196)] {
            let clamped = clamp(color, ColorDepth::Ansi16);
            assert!(
                matches!(
                    clamped,
                    Color::Ansi(AnsiColor::Red) | Color::Ansi(AnsiColor::BrightRed)
                ),
                "{color:?} -> {clamped:?}"
            );
        }
    }

    #[test]
    fn ansi_colors_are_never_changed() {
        for depth in [
            ColorDepth::Ansi16,
            ColorDepth::Ansi256,
            ColorDepth::TrueColor,
        ] {
            assert_eq!(
                clamp(ansi(AnsiColor::Yellow), depth),
                ansi(AnsiColor::Yellow),
                "{depth:?}"
            );
            assert_eq!(
                clamp(ansi(AnsiColor::BrightMagenta), depth),
                ansi(AnsiColor::BrightMagenta)
            );
        }
    }

    #[test]
    fn truecolor_leaves_everything_unchanged() {
        assert_eq!(clamp(rgb(1, 2, 3), ColorDepth::TrueColor), rgb(1, 2, 3));
        assert_eq!(clamp(index(208), ColorDepth::TrueColor), index(208));
    }

    #[test]
    fn index_is_kept_at_256_colors() {
        assert_eq!(clamp(index(208), ColorDepth::Ansi256), index(208));
    }

    fn val(toml: &str) -> toml::Value {
        let table: toml::Table = format!("v = {toml}").parse().unwrap();
        table["v"].clone()
    }

    #[test]
    fn every_color_form_is_accepted() {
        let empty = Palette::new();
        assert_eq!(
            parse_color(&val("\"bright_yellow\""), &empty),
            Ok(ansi(AnsiColor::BrightYellow))
        );
        assert_eq!(
            parse_color(&val("\"black\""), &empty),
            Ok(ansi(AnsiColor::Black))
        );
        assert_eq!(parse_color(&val("208"), &empty), Ok(index(208)));
        assert_eq!(parse_color(&val("0"), &empty), Ok(index(0)));
        assert_eq!(parse_color(&val("255"), &empty), Ok(index(255)));
        assert_eq!(
            parse_color(&val("\"#FF8800\""), &empty),
            Ok(rgb(255, 136, 0))
        );
        assert_eq!(
            parse_color(&val("\"#00aaff\""), &empty),
            Ok(rgb(0, 170, 255))
        );
        let mut palette = Palette::new();
        palette.insert("accent".into(), rgb(0, 170, 255));
        assert_eq!(
            parse_color(&val("\"accent\""), &palette),
            Ok(rgb(0, 170, 255))
        );
    }

    #[test]
    fn palette_names_shadow_ansi_names() {
        let mut palette = Palette::new();
        palette.insert("red".into(), index(196));
        assert_eq!(parse_color(&val("\"red\""), &palette), Ok(index(196)));
        assert_eq!(
            parse_color(&val("\"red\""), &Palette::new()),
            Ok(ansi(AnsiColor::Red))
        );
    }

    #[test]
    fn invalid_colors_are_rejected_with_the_offending_value() {
        let empty = Palette::new();
        assert_eq!(
            parse_color(&val("\"#f80\""), &empty),
            Err(ColorError::BadHex("#f80".into()))
        );
        assert_eq!(
            parse_color(&val("\"#gg0000\""), &empty),
            Err(ColorError::BadHex("#gg0000".into()))
        );
        assert_eq!(
            parse_color(&val("256"), &empty),
            Err(ColorError::OutOfRange(256))
        );
        assert_eq!(
            parse_color(&val("-1"), &empty),
            Err(ColorError::OutOfRange(-1))
        );
        assert_eq!(
            parse_color(&val("\"RED\""), &empty),
            Err(ColorError::UnknownName("RED".into()))
        );
        assert_eq!(
            parse_color(&val("\"purple\""), &empty),
            Err(ColorError::UnknownName("purple".into()))
        );
        assert_eq!(
            parse_color(&val("true"), &empty),
            Err(ColorError::WrongType("boolean"))
        );
        assert_eq!(
            parse_color(&val("1.5"), &empty),
            Err(ColorError::WrongType("float"))
        );
        assert!(ColorError::OutOfRange(256).to_string().contains("256"));
        assert!(
            ColorError::BadHex("#f80".into())
                .to_string()
                .contains("#f80")
        );
    }

    #[test]
    fn modifiers_parse_and_unknown_ones_are_rejected() {
        assert_eq!(parse_modifier("bold"), Ok(Effects::BOLD));
        assert_eq!(parse_modifier("dim"), Ok(Effects::DIMMED));
        assert_eq!(parse_modifier("italic"), Ok(Effects::ITALIC));
        assert_eq!(parse_modifier("underline"), Ok(Effects::UNDERLINE));
        assert_eq!(parse_modifier("strikethrough"), Ok(Effects::STRIKETHROUGH));
        let err = parse_modifier("blink").unwrap_err();
        assert_eq!(err, UnknownModifier("blink".into()));
        assert!(err.to_string().contains("blink"));
        assert!(parse_modifier("Bold").is_err());
    }

    #[test]
    fn depth_ordering_and_is_on() {
        assert!(ColorDepth::Off < ColorDepth::Ansi16);
        assert!(ColorDepth::Ansi256 < ColorDepth::TrueColor);
        assert!(!ColorDepth::Off.is_on());
        assert!(ColorDepth::Ansi16.is_on());
    }
}
