//! Environment snapshot and settings resolution.
//!
//! Everything that depends on the process environment is captured once into an
//! [`Environment`] and turned into [`Settings`] by the pure [`resolve`] function,
//! so every decision can be unit-tested with fabricated snapshots.

use std::io::IsTerminal;
use std::num::NonZeroUsize;

use crate::cli::{Args, ColorMode, DepthArg};
use crate::pager::PagerCommand;
use crate::theme::color::ColorDepth;
use crate::theme::{Glyphs, Theme};

/// Width used when neither the terminal nor `COLUMNS` provides one.
pub const DEFAULT_WIDTH: usize = 80;
/// Smallest width layout is ever asked to work with.
pub const MIN_WIDTH: usize = 20;

/// A snapshot of everything about the process environment that influences output.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Environment {
    /// Whether standard output is attached to a terminal.
    pub stdout_is_terminal: bool,
    /// `(columns, rows)` of the terminal attached to standard output, if it could be read.
    pub terminal_size: Option<(u16, u16)>,
    pub term: Option<String>,
    pub colorterm: Option<String>,
    pub no_color: Option<String>,
    pub clicolor: Option<String>,
    pub clicolor_force: Option<String>,
    pub columns: Option<String>,
    pub lines: Option<String>,
    pub lc_all: Option<String>,
    pub lc_ctype: Option<String>,
    pub lang: Option<String>,
    pub catmd_pager: Option<String>,
    pub pager: Option<String>,
    pub less: Option<String>,
    pub catmd_theme: Option<String>,
    pub xdg_config_home: Option<String>,
    pub home: Option<String>,
}

impl Environment {
    /// Capture the current process environment.
    pub fn capture() -> Self {
        let var = |key: &str| std::env::var(key).ok();
        let stdout = std::io::stdout();
        Self {
            stdout_is_terminal: stdout.is_terminal(),
            terminal_size: terminal_size::terminal_size_of(&stdout).map(|(w, h)| (w.0, h.0)),
            term: var("TERM"),
            colorterm: var("COLORTERM"),
            no_color: var("NO_COLOR"),
            clicolor: var("CLICOLOR"),
            clicolor_force: var("CLICOLOR_FORCE"),
            columns: var("COLUMNS"),
            lines: var("LINES"),
            lc_all: var("LC_ALL"),
            lc_ctype: var("LC_CTYPE"),
            lang: var("LANG"),
            catmd_pager: var("CATMD_PAGER"),
            pager: var("PAGER"),
            less: var("LESS"),
            catmd_theme: var("CATMD_THEME"),
            xdg_config_home: var("XDG_CONFIG_HOME"),
            home: var("HOME"),
        }
    }
}

/// Fully resolved output settings; nothing downstream re-detects the environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// Whether styling is emitted and, if so, how many colors may be used.
    pub depth: ColorDepth,
    /// Rendering width in columns, never below [`MIN_WIDTH`].
    pub width: usize,
    /// Terminal height in rows for paging decisions; `None` means unknown (never page).
    pub height: Option<usize>,
    /// Glyph set for bullets, rules, and borders.
    pub glyphs: Glyphs,
    /// Pager to use when output overflows; `None` disables paging.
    pub pager: Option<PagerCommand>,
    /// Whether standard output was a terminal when settings were resolved.
    pub stdout_is_terminal: bool,
    /// Styles for every rendering role.
    pub theme: Theme,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            depth: ColorDepth::Off,
            width: DEFAULT_WIDTH,
            height: None,
            glyphs: Glyphs::Unicode,
            pager: None,
            stdout_is_terminal: false,
            theme: Theme::default(),
        }
    }
}

/// Resolve [`Settings`] from arguments and an [`Environment`] snapshot.
pub fn resolve(args: &Args, env: &Environment) -> Settings {
    Settings {
        depth: resolve_depth(args, env),
        width: resolve_width(args.width, env),
        height: resolve_height(args.height, env),
        glyphs: resolve_glyphs(args.ascii, env),
        pager: crate::pager::resolve_pager(args.no_pager, env),
        stdout_is_terminal: env.stdout_is_terminal,
        theme: Theme::default(),
    }
}

/// Prepare the terminal for the resolved settings. On Windows this enables
/// virtual-terminal processing so ANSI sequences are interpreted.
pub fn prepare_terminal(settings: &Settings) {
    #[cfg(windows)]
    if settings.depth.is_on() {
        let _ = anstyle_query::windows::enable_ansi_colors();
    }
    #[cfg(not(windows))]
    let _ = settings;
}

fn resolve_depth(args: &Args, env: &Environment) -> ColorDepth {
    if !resolve_color(args.color, env) {
        return ColorDepth::Off;
    }
    match args.color_depth {
        DepthArg::Auto => ColorDepth::detect(env.colorterm.as_deref(), env.term.as_deref()),
        DepthArg::Ansi16 => ColorDepth::Ansi16,
        DepthArg::Ansi256 => ColorDepth::Ansi256,
        DepthArg::TrueColor => ColorDepth::TrueColor,
    }
}

fn resolve_color(mode: ColorMode, env: &Environment) -> bool {
    match mode {
        ColorMode::Always => true,
        ColorMode::Never => false,
        ColorMode::Auto => {
            if is_set(&env.no_color) {
                return false;
            }
            if env
                .clicolor_force
                .as_deref()
                .is_some_and(|v| !v.is_empty() && v != "0")
            {
                return true;
            }
            if env.clicolor.as_deref() == Some("0") {
                return false;
            }
            env.stdout_is_terminal && env.term.as_deref() != Some("dumb")
        }
    }
}

fn resolve_width(flag: Option<NonZeroUsize>, env: &Environment) -> usize {
    flag.map(NonZeroUsize::get)
        .or_else(|| terminal_dimension(env, |(cols, _)| cols))
        .or_else(|| positive_int(&env.columns))
        .unwrap_or(DEFAULT_WIDTH)
        .max(MIN_WIDTH)
}

fn resolve_height(flag: Option<NonZeroUsize>, env: &Environment) -> Option<usize> {
    flag.map(NonZeroUsize::get)
        .or_else(|| terminal_dimension(env, |(_, rows)| rows))
        .or_else(|| positive_int(&env.lines))
}

fn terminal_dimension(env: &Environment, pick: fn((u16, u16)) -> u16) -> Option<usize> {
    if !env.stdout_is_terminal {
        return None;
    }
    env.terminal_size
        .map(pick)
        .map(usize::from)
        .filter(|n| *n > 0)
}

fn resolve_glyphs(ascii: bool, env: &Environment) -> Glyphs {
    if ascii {
        return Glyphs::Ascii;
    }
    let locale = [&env.lc_all, &env.lc_ctype, &env.lang]
        .into_iter()
        .flatten()
        .find(|value| !value.is_empty());
    match locale {
        None => Glyphs::Unicode,
        Some(locale) => {
            let lower = locale.to_ascii_lowercase();
            if lower.contains("utf-8") || lower.contains("utf8") {
                Glyphs::Unicode
            } else {
                Glyphs::Ascii
            }
        }
    }
}

fn is_set(value: &Option<String>) -> bool {
    value.as_deref().is_some_and(|v| !v.is_empty())
}

fn positive_int(value: &Option<String>) -> Option<usize> {
    value
        .as_deref()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .filter(|n| *n > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &str) -> Option<String> {
        Some(v.to_string())
    }

    fn tty() -> Environment {
        Environment {
            stdout_is_terminal: true,
            terminal_size: Some((120, 40)),
            term: s("xterm-256color"),
            ..Environment::default()
        }
    }

    fn piped() -> Environment {
        Environment {
            stdout_is_terminal: false,
            terminal_size: None,
            term: s("xterm-256color"),
            ..Environment::default()
        }
    }

    fn args() -> Args {
        Args::default()
    }

    fn with_color(mode: ColorMode) -> Args {
        Args {
            color: mode,
            ..Args::default()
        }
    }

    #[test]
    fn auto_styles_a_capable_terminal() {
        assert!(resolve(&args(), &tty()).depth.is_on());
    }

    #[test]
    fn auto_does_not_style_a_pipe() {
        assert!(!resolve(&args(), &piped()).depth.is_on());
    }

    #[test]
    fn auto_does_not_style_a_dumb_terminal() {
        let env = Environment {
            term: s("dumb"),
            ..tty()
        };
        assert!(!resolve(&args(), &env).depth.is_on());
    }

    #[test]
    fn auto_styles_a_terminal_without_term_variable() {
        let env = Environment {
            term: None,
            ..tty()
        };
        assert!(resolve(&args(), &env).depth.is_on());
    }

    #[test]
    fn no_color_disables_auto_styling() {
        let env = Environment {
            no_color: s("1"),
            ..tty()
        };
        assert!(!resolve(&args(), &env).depth.is_on());
    }

    #[test]
    fn empty_no_color_is_ignored() {
        let env = Environment {
            no_color: s(""),
            ..tty()
        };
        assert!(resolve(&args(), &env).depth.is_on());
    }

    #[test]
    fn always_overrides_no_color_and_pipes() {
        let env = Environment {
            no_color: s("1"),
            ..piped()
        };
        assert!(resolve(&with_color(ColorMode::Always), &env).depth.is_on());
    }

    #[test]
    fn never_overrides_a_capable_terminal() {
        assert!(!resolve(&with_color(ColorMode::Never), &tty()).depth.is_on());
    }

    #[test]
    fn clicolor_force_styles_a_pipe() {
        let env = Environment {
            clicolor_force: s("1"),
            ..piped()
        };
        assert!(resolve(&args(), &env).depth.is_on());
    }

    #[test]
    fn clicolor_force_zero_or_empty_does_not_force() {
        for value in ["0", ""] {
            let env = Environment {
                clicolor_force: s(value),
                ..piped()
            };
            assert!(
                !resolve(&args(), &env).depth.is_on(),
                "CLICOLOR_FORCE={value:?}"
            );
        }
    }

    #[test]
    fn no_color_beats_clicolor_force() {
        let env = Environment {
            no_color: s("1"),
            clicolor_force: s("1"),
            ..tty()
        };
        assert!(!resolve(&args(), &env).depth.is_on());
    }

    #[test]
    fn clicolor_zero_disables_styling_on_a_terminal() {
        let env = Environment {
            clicolor: s("0"),
            ..tty()
        };
        assert!(!resolve(&args(), &env).depth.is_on());
    }

    #[test]
    fn clicolor_one_does_not_change_auto_behavior() {
        let env = Environment {
            clicolor: s("1"),
            ..tty()
        };
        assert!(resolve(&args(), &env).depth.is_on());
        let env = Environment {
            clicolor: s("1"),
            ..piped()
        };
        assert!(!resolve(&args(), &env).depth.is_on());
    }

    #[test]
    fn depth_is_off_when_styling_is_off_regardless_of_colorterm() {
        let env = Environment {
            colorterm: s("truecolor"),
            ..piped()
        };
        assert_eq!(resolve(&args(), &env).depth, ColorDepth::Off);
        let a = Args {
            color_depth: DepthArg::TrueColor,
            ..with_color(ColorMode::Never)
        };
        assert_eq!(resolve(&a, &tty()).depth, ColorDepth::Off);
    }

    #[test]
    fn depth_is_detected_in_auto_mode() {
        assert_eq!(resolve(&args(), &tty()).depth, ColorDepth::Ansi256);
        let env = Environment {
            colorterm: s("truecolor"),
            ..tty()
        };
        assert_eq!(resolve(&args(), &env).depth, ColorDepth::TrueColor);
        let env = Environment {
            term: s("xterm"),
            ..tty()
        };
        assert_eq!(resolve(&args(), &env).depth, ColorDepth::Ansi16);
    }

    #[test]
    fn depth_detection_ignores_the_tty_flag_when_color_is_forced() {
        let env = Environment {
            term: s("xterm-256color"),
            ..piped()
        };
        assert_eq!(
            resolve(&with_color(ColorMode::Always), &env).depth,
            ColorDepth::Ansi256
        );
    }

    #[test]
    fn depth_override_replaces_detection() {
        let env = Environment {
            colorterm: s("truecolor"),
            ..tty()
        };
        for (flag, expected) in [
            (DepthArg::Ansi16, ColorDepth::Ansi16),
            (DepthArg::Ansi256, ColorDepth::Ansi256),
            (DepthArg::TrueColor, ColorDepth::TrueColor),
        ] {
            let a = Args {
                color_depth: flag,
                ..args()
            };
            assert_eq!(resolve(&a, &env).depth, expected, "{flag:?}");
        }
        let a = Args {
            color_depth: DepthArg::TrueColor,
            ..args()
        };
        let plain = Environment {
            term: s("screen"),
            ..tty()
        };
        assert_eq!(resolve(&a, &plain).depth, ColorDepth::TrueColor);
    }

    #[test]
    fn width_comes_from_the_terminal() {
        assert_eq!(resolve(&args(), &tty()).width, 120);
    }

    #[test]
    fn width_falls_back_to_columns_then_default() {
        let env = Environment {
            columns: s("60"),
            ..piped()
        };
        assert_eq!(resolve(&args(), &env).width, 60);
        assert_eq!(resolve(&args(), &piped()).width, DEFAULT_WIDTH);
    }

    #[test]
    fn invalid_columns_values_fall_back_to_default() {
        for value in ["", "wide", "0", "-5"] {
            let env = Environment {
                columns: s(value),
                ..piped()
            };
            assert_eq!(
                resolve(&args(), &env).width,
                DEFAULT_WIDTH,
                "COLUMNS={value:?}"
            );
        }
    }

    #[test]
    fn terminal_size_is_ignored_when_stdout_is_not_a_terminal() {
        let env = Environment {
            terminal_size: Some((200, 50)),
            ..piped()
        };
        let settings = resolve(&args(), &env);
        assert_eq!(settings.width, DEFAULT_WIDTH);
        assert_eq!(settings.height, None);
    }

    #[test]
    fn width_flag_overrides_everything() {
        let a = Args {
            width: NonZeroUsize::new(40),
            ..args()
        };
        let env = Environment {
            columns: s("60"),
            ..tty()
        };
        assert_eq!(resolve(&a, &env).width, 40);
    }

    #[test]
    fn width_is_clamped_to_the_minimum() {
        let a = Args {
            width: NonZeroUsize::new(5),
            ..args()
        };
        assert_eq!(resolve(&a, &tty()).width, MIN_WIDTH);
        let env = Environment {
            terminal_size: Some((10, 40)),
            ..tty()
        };
        assert_eq!(resolve(&args(), &env).width, MIN_WIDTH);
        let env = Environment {
            columns: s("3"),
            ..piped()
        };
        assert_eq!(resolve(&args(), &env).width, MIN_WIDTH);
    }

    #[test]
    fn height_comes_from_the_terminal() {
        assert_eq!(resolve(&args(), &tty()).height, Some(40));
    }

    #[test]
    fn height_falls_back_to_lines_then_unknown() {
        let env = Environment {
            lines: s("30"),
            ..piped()
        };
        assert_eq!(resolve(&args(), &env).height, Some(30));
        assert_eq!(resolve(&args(), &piped()).height, None);
    }

    #[test]
    fn height_flag_overrides_the_terminal() {
        let a = Args {
            height: NonZeroUsize::new(10),
            ..args()
        };
        assert_eq!(resolve(&a, &tty()).height, Some(10));
    }

    #[test]
    fn glyphs_follow_the_locale() {
        let cases = [
            (None, None, None, Glyphs::Unicode),
            (None, None, s("en_US.UTF-8"), Glyphs::Unicode),
            (None, None, s("en_US.utf8"), Glyphs::Unicode),
            (s("C"), None, s("en_US.UTF-8"), Glyphs::Ascii),
            (None, s("POSIX"), s("en_US.UTF-8"), Glyphs::Ascii),
            (s(""), None, s("en_US.UTF-8"), Glyphs::Unicode),
            (None, None, s("de_DE.ISO-8859-1"), Glyphs::Ascii),
        ];
        for (lc_all, lc_ctype, lang, expected) in cases {
            let env = Environment {
                lc_all: lc_all.clone(),
                lc_ctype: lc_ctype.clone(),
                lang: lang.clone(),
                ..tty()
            };
            assert_eq!(
                resolve(&args(), &env).glyphs,
                expected,
                "LC_ALL={lc_all:?} LC_CTYPE={lc_ctype:?} LANG={lang:?}"
            );
        }
    }

    #[test]
    fn ascii_flag_overrides_the_locale() {
        let a = Args {
            ascii: true,
            ..args()
        };
        let env = Environment {
            lang: s("en_US.UTF-8"),
            ..tty()
        };
        assert_eq!(resolve(&a, &env).glyphs, Glyphs::Ascii);
    }

    #[test]
    fn pager_precedence_is_catmd_pager_then_pager_then_less() {
        let env = Environment {
            catmd_pager: s("more"),
            pager: s("less"),
            ..tty()
        };
        assert_eq!(resolve(&args(), &env).pager.unwrap().program, "more");
        let env = Environment {
            pager: s("most"),
            ..tty()
        };
        assert_eq!(resolve(&args(), &env).pager.unwrap().program, "most");
        assert_eq!(resolve(&args(), &tty()).pager.unwrap().program, "less");
    }

    #[test]
    fn empty_pager_values_are_skipped() {
        let env = Environment {
            catmd_pager: s(""),
            pager: s("  "),
            ..tty()
        };
        assert_eq!(resolve(&args(), &env).pager.unwrap().program, "less");
    }

    #[test]
    fn cat_and_no_pager_disable_paging() {
        let env = Environment {
            pager: s("cat"),
            ..tty()
        };
        assert_eq!(resolve(&args(), &env).pager, None);
        let a = Args {
            no_pager: true,
            ..args()
        };
        assert_eq!(resolve(&a, &tty()).pager, None);
    }

    #[test]
    fn stdout_terminal_flag_is_carried_into_settings() {
        assert!(resolve(&args(), &tty()).stdout_is_terminal);
        assert!(!resolve(&args(), &piped()).stdout_is_terminal);
    }
}
