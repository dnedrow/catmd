//! Theme file selection and loading.
//!
//! Selection ([`select`]) is pure and works from an [`Environment`] snapshot;
//! loading ([`load`]) is the only part that touches the file system.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use anstyle::Style;

use super::Theme;
use super::color::{ColorError, Palette, parse_color, parse_modifier};
use crate::env::Environment;

/// Where a theme comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThemeSource {
    /// An explicit file path given by the user.
    Path(PathBuf),
    /// A theme name resolved to a file inside the themes directory.
    Named { name: String, path: PathBuf },
}

impl ThemeSource {
    /// The file that will be read.
    pub fn path(&self) -> &Path {
        match self {
            ThemeSource::Path(path) | ThemeSource::Named { path, .. } => path,
        }
    }
}

/// Decide which theme file to load: `--theme` beats `CATMD_THEME`; empty values
/// are ignored; a value containing a path separator or ending in `.toml` is a
/// path, anything else names `<value>.toml` in the themes directory.
pub fn select(flag: Option<&str>, env: &Environment) -> Option<ThemeSource> {
    let value = non_empty(flag).or_else(|| non_empty(env.catmd_theme.as_deref()))?;
    if is_path(value) {
        return Some(ThemeSource::Path(PathBuf::from(value)));
    }
    let path = config_dir(env)
        .join("catmd")
        .join("themes")
        .join(format!("{value}.toml"));
    Some(ThemeSource::Named {
        name: value.to_string(),
        path,
    })
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|v| !v.is_empty())
}

fn is_path(value: &str) -> bool {
    value.contains('/') || (cfg!(windows) && value.contains('\\')) || value.ends_with(".toml")
}

/// `$XDG_CONFIG_HOME`, else `$HOME/.config`.
fn config_dir(env: &Environment) -> PathBuf {
    match non_empty(env.xdg_config_home.as_deref()) {
        Some(xdg) => PathBuf::from(xdg),
        None => PathBuf::from(non_empty(env.home.as_deref()).unwrap_or("~")).join(".config"),
    }
}

/// Why a theme could not be loaded. Every variant names the file involved.
#[derive(Debug)]
pub enum ThemeError {
    Read {
        path: PathBuf,
        source: io::Error,
    },
    Parse {
        path: PathBuf,
        message: String,
    },
    UnknownRole {
        path: PathBuf,
        key: String,
    },
    UnknownModifier {
        path: PathBuf,
        role: String,
        name: String,
    },
    InvalidColor {
        path: PathBuf,
        key: String,
        error: ColorError,
    },
    BadShape {
        path: PathBuf,
        key: String,
        expected: &'static str,
    },
}

impl ThemeError {
    /// The theme file the error refers to.
    pub fn path(&self) -> &Path {
        match self {
            ThemeError::Read { path, .. }
            | ThemeError::Parse { path, .. }
            | ThemeError::UnknownRole { path, .. }
            | ThemeError::UnknownModifier { path, .. }
            | ThemeError::InvalidColor { path, .. }
            | ThemeError::BadShape { path, .. } => path,
        }
    }
}

impl fmt::Display for ThemeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let path = self.path().display();
        match self {
            ThemeError::Read { source, .. } => write!(f, "{path}: {source}"),
            ThemeError::Parse { message, .. } => {
                write!(f, "{path}: invalid TOML: {}", message.trim_end())
            }
            ThemeError::UnknownRole { key, .. } => write!(f, "{path}: unknown role `{key}`"),
            ThemeError::UnknownModifier { role, name, .. } => write!(
                f,
                "{path}: in `{role}`: unknown modifier `{name}` (expected bold, dim, italic, underline, or strikethrough)"
            ),
            ThemeError::InvalidColor { key, error, .. } => {
                write!(f, "{path}: in `{key}`: {error}")
            }
            ThemeError::BadShape { key, expected, .. } => {
                write!(f, "{path}: `{key}` must be {expected}")
            }
        }
    }
}

impl std::error::Error for ThemeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ThemeError::Read { source, .. } => Some(source),
            ThemeError::InvalidColor { error, .. } => Some(error),
            _ => None,
        }
    }
}

/// Read and parse the theme file `source` points at.
pub fn load(source: &ThemeSource) -> Result<Theme, ThemeError> {
    let path = source.path();
    let text = std::fs::read_to_string(path).map_err(|source| ThemeError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    parse(&text, path)
}

/// Parse theme TOML. Unmentioned roles keep the built-in default; a mentioned
/// role is replaced entirely by the entry.
pub fn parse(text: &str, path: &Path) -> Result<Theme, ThemeError> {
    let table: toml::Table = text
        .parse()
        .map_err(|err: toml::de::Error| ThemeError::Parse {
            path: path.to_path_buf(),
            message: err.to_string(),
        })?;

    let mut palette = Palette::new();
    if let Some(value) = table.get("palette") {
        let entries = value.as_table().ok_or_else(|| ThemeError::BadShape {
            path: path.to_path_buf(),
            key: "palette".to_string(),
            expected: "a table of named colors",
        })?;
        for (name, value) in entries {
            // Palette entries cannot reference each other: parse against an empty palette.
            let color =
                parse_color(value, &Palette::new()).map_err(|error| ThemeError::InvalidColor {
                    path: path.to_path_buf(),
                    key: format!("palette.{name}"),
                    error,
                })?;
            palette.insert(name.clone(), color);
        }
    }

    let mut theme = Theme::default();
    for (key, value) in &table {
        if key == "palette" {
            continue;
        }
        if !Theme::ROLES.contains(&key.as_str()) {
            return Err(ThemeError::UnknownRole {
                path: path.to_path_buf(),
                key: key.clone(),
            });
        }
        let style = parse_style(key, value, &palette, path)?;
        theme
            .set(key, style)
            .expect("role name was checked against Theme::ROLES");
    }
    Ok(theme)
}

fn parse_style(
    role: &str,
    value: &toml::Value,
    palette: &Palette,
    path: &Path,
) -> Result<Style, ThemeError> {
    let invalid_color = |error| ThemeError::InvalidColor {
        path: path.to_path_buf(),
        key: role.to_string(),
        error,
    };
    match value {
        toml::Value::String(_) | toml::Value::Integer(_) => {
            let color = parse_color(value, palette).map_err(invalid_color)?;
            Ok(Style::new().fg_color(Some(color)))
        }
        toml::Value::Table(entry) => {
            let mut style = Style::new();
            for (key, value) in entry {
                match key.as_str() {
                    "fg" => {
                        let color = parse_color(value, palette).map_err(invalid_color)?;
                        style = style.fg_color(Some(color));
                    }
                    "modifiers" => {
                        let names = value.as_array().ok_or_else(|| ThemeError::BadShape {
                            path: path.to_path_buf(),
                            key: format!("{role}.modifiers"),
                            expected: "an array of modifier names",
                        })?;
                        for name in names {
                            let name = name.as_str().ok_or_else(|| ThemeError::BadShape {
                                path: path.to_path_buf(),
                                key: format!("{role}.modifiers"),
                                expected: "an array of modifier names",
                            })?;
                            let effect =
                                parse_modifier(name).map_err(|e| ThemeError::UnknownModifier {
                                    path: path.to_path_buf(),
                                    role: role.to_string(),
                                    name: e.0,
                                })?;
                            style |= effect;
                        }
                    }
                    other => {
                        return Err(ThemeError::BadShape {
                            path: path.to_path_buf(),
                            key: format!("{role}.{other}"),
                            expected: "`fg` or `modifiers`",
                        });
                    }
                }
            }
            Ok(style)
        }
        _ => Err(ThemeError::BadShape {
            path: path.to_path_buf(),
            key: role.to_string(),
            expected: "a color or a table with `fg` and/or `modifiers`",
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::color::{ansi, index, rgb};
    use anstyle::AnsiColor;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    struct TempTheme(PathBuf);

    impl TempTheme {
        fn new(contents: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "catmd-theme-test-{}-{}.toml",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::SeqCst)
            ));
            std::fs::write(&path, contents).unwrap();
            TempTheme(path)
        }

        fn load(&self) -> Result<Theme, ThemeError> {
            load(&ThemeSource::Path(self.0.clone()))
        }
    }

    impl Drop for TempTheme {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    fn s(v: &str) -> Option<String> {
        Some(v.to_string())
    }

    fn fg(color: anstyle::Color) -> Style {
        Style::new().fg_color(Some(color))
    }

    // ----- select -----

    #[test]
    fn flag_beats_environment() {
        let env = Environment {
            catmd_theme: s("./a.toml"),
            ..Environment::default()
        };
        assert_eq!(
            select(Some("./b.toml"), &env),
            Some(ThemeSource::Path("./b.toml".into()))
        );
        assert_eq!(
            select(None, &env),
            Some(ThemeSource::Path("./a.toml".into()))
        );
    }

    #[test]
    fn empty_values_are_ignored() {
        let env = Environment {
            catmd_theme: s("  "),
            ..Environment::default()
        };
        assert_eq!(select(None, &env), None);
        assert_eq!(select(Some(""), &env), None);
        assert_eq!(select(None, &Environment::default()), None);
    }

    #[test]
    fn separators_and_toml_suffix_mean_a_path() {
        let env = Environment::default();
        for value in ["./t.toml", "dark.toml", "themes/dark", "/abs/x"] {
            assert_eq!(
                select(Some(value), &env),
                Some(ThemeSource::Path(PathBuf::from(value))),
                "{value}"
            );
        }
    }

    #[test]
    fn names_resolve_through_xdg_config_home_then_home() {
        let env = Environment {
            xdg_config_home: s("/tmp/cfg"),
            home: s("/tmp/home"),
            ..Environment::default()
        };
        assert_eq!(
            select(Some("dark"), &env),
            Some(ThemeSource::Named {
                name: "dark".into(),
                path: "/tmp/cfg/catmd/themes/dark.toml".into()
            })
        );
        for xdg in [None, s("")] {
            let env = Environment {
                xdg_config_home: xdg,
                home: s("/tmp/home"),
                ..Environment::default()
            };
            assert_eq!(
                select(Some("dark"), &env).unwrap().path(),
                Path::new("/tmp/home/.config/catmd/themes/dark.toml")
            );
        }
    }

    // ----- load / parse -----

    #[test]
    fn bare_string_and_integer_entries_set_only_a_foreground() {
        let theme = TempTheme::new("code = \"red\"\nemph = 196\n")
            .load()
            .unwrap();
        assert_eq!(theme.code, fg(ansi(AnsiColor::Red)));
        assert_eq!(theme.emph, fg(index(196)));
    }

    #[test]
    fn bare_string_replaces_the_whole_style() {
        let theme = TempTheme::new("strong = \"red\"\n").load().unwrap();
        assert_eq!(theme.strong, fg(ansi(AnsiColor::Red)));
        assert!(!theme.strong.get_effects().contains(anstyle::Effects::BOLD));
    }

    #[test]
    fn table_entries_combine_fg_and_modifiers() {
        let theme = TempTheme::new(
            "heading1 = { fg = \"cyan\", modifiers = [\"bold\", \"underline\"] }\nlink_url = { modifiers = [\"dim\"] }\nrule = {}\n",
        )
        .load()
        .unwrap();
        assert_eq!(
            theme.heading[0],
            fg(ansi(AnsiColor::Cyan)).bold().underline()
        );
        assert_eq!(theme.link_url, Style::new().dimmed());
        assert_eq!(theme.link_url.get_fg_color(), None);
        assert_eq!(theme.rule, Style::new());
    }

    #[test]
    fn palette_is_resolved_first_and_may_use_any_form() {
        let theme = TempTheme::new(
            "[palette]\naccent = \"#00aaff\"\nred = 196\nplain = \"bright_white\"\n\n[dummy_never_read]\n",
        )
        .load();
        assert!(
            matches!(theme, Err(ThemeError::UnknownRole { ref key, .. }) if key == "dummy_never_read")
        );
        let theme = TempTheme::new(
            "code = \"red\"\nlink_text = \"accent\"\nhtml = \"plain\"\n[palette]\naccent = \"#00aaff\"\nred = 196\nplain = \"bright_white\"\n",
        )
        .load()
        .unwrap();
        assert_eq!(theme.code, fg(index(196)));
        assert_eq!(theme.link_text, fg(rgb(0, 170, 255)));
        assert_eq!(theme.html, fg(ansi(AnsiColor::BrightWhite)));
    }

    #[test]
    fn partial_theme_keeps_defaults_for_other_roles() {
        let theme = TempTheme::new("code = \"red\"\n").load().unwrap();
        let default = Theme::default();
        for role in Theme::ROLES {
            if role != "code" {
                assert_eq!(theme.get(role), default.get(role), "{role}");
            }
        }
    }

    #[test]
    fn missing_file_is_a_read_error_naming_the_path() {
        let path = std::env::temp_dir().join("catmd-definitely-missing-theme.toml");
        let err = load(&ThemeSource::Path(path.clone())).unwrap_err();
        assert!(matches!(err, ThemeError::Read { .. }));
        assert!(
            err.to_string()
                .contains("catmd-definitely-missing-theme.toml")
        );
    }

    #[test]
    fn invalid_toml_reports_a_parse_error_with_position() {
        let t = TempTheme::new("code = \"red\n");
        let err = t.load().unwrap_err();
        assert!(matches!(err, ThemeError::Parse { .. }));
        let msg = err.to_string();
        assert!(msg.contains("invalid TOML"), "{msg}");
        assert!(msg.contains("line 1"), "{msg}");
    }

    #[test]
    fn unknown_role_is_rejected() {
        let err = TempTheme::new("heading7 = \"red\"\n").load().unwrap_err();
        assert!(matches!(err, ThemeError::UnknownRole { ref key, .. } if key == "heading7"));
        assert!(err.to_string().contains("heading7"));
    }

    #[test]
    fn unknown_modifier_is_rejected() {
        let err = TempTheme::new("strong = { modifiers = [\"blink\"] }\n")
            .load()
            .unwrap_err();
        assert!(matches!(err, ThemeError::UnknownModifier { ref name, .. } if name == "blink"));
        assert!(err.to_string().contains("blink"));
    }

    #[test]
    fn invalid_colors_are_rejected_in_roles_and_palette() {
        let err = TempTheme::new("code = \"#f80\"\n").load().unwrap_err();
        assert!(matches!(err, ThemeError::InvalidColor { ref key, .. } if key == "code"));
        assert!(err.to_string().contains("#f80"));
        let err = TempTheme::new("code = 256\n").load().unwrap_err();
        assert!(err.to_string().contains("256"));
        let err = TempTheme::new("code = \"x\"\n[palette]\nx = \"nope\"\n")
            .load()
            .unwrap_err();
        assert!(matches!(err, ThemeError::InvalidColor { ref key, .. } if key == "palette.x"));
    }

    #[test]
    fn wrong_shapes_are_rejected() {
        let cases = [
            ("palette = 3\n", "palette"),
            ("strong = { modifiers = \"bold\" }\n", "strong.modifiers"),
            ("strong = { modifiers = [1] }\n", "strong.modifiers"),
            ("strong = { bg = \"red\" }\n", "strong.bg"),
            ("strong = true\n", "strong"),
            ("strong = [\"red\"]\n", "strong"),
        ];
        for (toml, key) in cases {
            let err = TempTheme::new(toml).load().unwrap_err();
            assert!(
                matches!(err, ThemeError::BadShape { key: ref k, .. } if k == key),
                "{toml:?} -> {err}"
            );
        }
    }

    #[test]
    fn every_role_can_be_set_from_a_file() {
        let toml: String = Theme::ROLES
            .iter()
            .map(|role| format!("{role} = \"green\"\n"))
            .collect();
        let theme = TempTheme::new(&toml).load().unwrap();
        assert!(
            theme
                .all_styles()
                .iter()
                .all(|s| *s == fg(ansi(AnsiColor::Green)))
        );
    }
}
