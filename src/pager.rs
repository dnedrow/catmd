//! Paging decision and pager hand-off.
//!
//! Output is rendered to a buffer first. If standard output is a terminal and
//! the buffer is taller than the terminal, it is piped through the user's pager
//! (git style); otherwise it is written directly, like `cat`.

use std::io::{self, Write};
use std::process::{Command, Stdio};

use unicode_width::UnicodeWidthStr;

use crate::env::{Environment, Settings};
use crate::render::layout::strip_ansi;

/// Pager used when output overflows the terminal.
pub const DEFAULT_PAGER: &str = "less";
/// `LESS` flags applied when the user has not configured `less` themselves:
/// quit if one screen, pass ANSI colors through, do not clear the screen on exit.
pub const DEFAULT_LESS: &str = "FRX";

/// A fully resolved pager invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PagerCommand {
    pub program: String,
    pub args: Vec<String>,
    /// Extra environment variables for the pager process.
    pub env: Vec<(String, String)>,
}

/// Resolve the pager command from the environment, or `None` when paging is disabled.
///
/// Precedence: `--no-pager`, then `CATMD_PAGER`, then `PAGER`, then `less`. A pager of
/// `cat` disables paging. `LESS=FRX` is supplied when `LESS` is unset or empty.
pub fn resolve_pager(no_pager: bool, env: &Environment) -> Option<PagerCommand> {
    if no_pager {
        return None;
    }
    let command_line = [&env.catmd_pager, &env.pager]
        .into_iter()
        .flatten()
        .map(|value| value.trim())
        .find(|value| !value.is_empty())
        .unwrap_or(DEFAULT_PAGER);
    let words = shell_words::split(command_line).ok()?;
    let (program, args) = words.split_first()?;
    if program == "cat" {
        return None;
    }
    let mut extra_env = Vec::new();
    if env.less.as_deref().is_none_or(str::is_empty) {
        extra_env.push(("LESS".to_string(), DEFAULT_LESS.to_string()));
    }
    Some(PagerCommand {
        program: program.clone(),
        args: args.to_vec(),
        env: extra_env,
    })
}

impl PagerCommand {
    /// Build the process command: arguments, extra environment, and a piped stdin.
    pub fn to_command(&self) -> Command {
        let mut command = Command::new(&self.program);
        command
            .args(&self.args)
            .envs(self.env.iter().map(|(key, value)| (key, value)))
            .stdin(Stdio::piped());
        command
    }
}

/// Terminal rows `output` occupies at `width`: each line counts as at least
/// one row, and a line wider than the terminal counts as the rows it wraps to.
pub fn visual_rows(output: &str, width: usize) -> usize {
    let width = width.max(1);
    output
        .lines()
        .map(|line| strip_ansi(line).width().div_ceil(width).max(1))
        .sum()
}

/// Whether `output` should go through the pager: only when standard output is a
/// terminal, paging is enabled, the height is known, and the output is taller
/// than the terminal. Output carrying Kitty or SIXEL graphics is never paged,
/// since external pagers corrupt or drop those sequences.
pub fn should_page(settings: &Settings, output: &str) -> bool {
    let Some(height) = settings.height else {
        return false;
    };
    settings.stdout_is_terminal
        && !has_graphics(output)
        && settings.pager.is_some()
        && visual_rows(output, settings.width) > height
}

/// Whether `output` contains a Kitty graphics (`ESC _ G`) or SIXEL (`ESC P`) sequence.
fn has_graphics(output: &str) -> bool {
    output.contains("\x1b_G") || output.contains("\x1bP")
}

/// Write `output` to standard output.
pub fn write_stdout(output: &str) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    stdout.write_all(output.as_bytes())?;
    stdout.flush()
}

/// Deliver rendered output: through the pager when it would overflow the
/// terminal, otherwise straight to standard output. If the pager cannot be
/// started, the failure is reported on standard error and the output is
/// written directly instead.
pub fn deliver(settings: &Settings, output: &str) -> io::Result<()> {
    if !should_page(settings, output) {
        return write_stdout(output);
    }
    let pager = settings
        .pager
        .as_ref()
        .expect("should_page is only true with a pager");
    let mut child = match pager.to_command().spawn() {
        Ok(child) => child,
        Err(err) => {
            eprintln!("catmd: could not start pager `{}`: {err}", pager.program);
            return write_stdout(output);
        }
    };
    let written = match child.stdin.take() {
        Some(mut stdin) => stdin.write_all(output.as_bytes()),
        None => Ok(()),
    };
    // Closing stdin (dropped above) lets the pager see EOF; wait so the terminal
    // is handed back before we exit.
    child.wait()?;
    match written {
        // The user quit the pager before it read everything.
        Err(err) if err.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    fn s(v: &str) -> Option<String> {
        Some(v.to_string())
    }

    fn tty_settings(height: Option<usize>) -> Settings {
        Settings {
            stdout_is_terminal: true,
            height,
            width: 80,
            pager: Some(PagerCommand {
                program: "less".into(),
                args: vec![],
                env: vec![],
            }),
            ..Settings::default()
        }
    }

    fn lines(n: usize) -> String {
        (0..n).map(|i| format!("line {i}\n")).collect()
    }

    #[test]
    fn visual_rows_counts_lines_and_wrapped_overflow() {
        assert_eq!(visual_rows("", 80), 0);
        assert_eq!(visual_rows("a\nb\n", 80), 2);
        assert_eq!(visual_rows(&"x".repeat(100), 40), 3);
        assert_eq!(visual_rows("\x1b[1mbold\x1b[0m\n", 2), 2);
        assert_eq!(visual_rows("\n\n", 80), 2);
    }

    #[test]
    fn short_output_streams() {
        assert!(!should_page(&tty_settings(Some(40)), &lines(10)));
    }

    #[test]
    fn graphics_output_is_never_paged() {
        for escape in ["\x1b_Ga=T;AAAA\x1b\\", "\x1bPq#0;2;0;0;0\x1b\\"] {
            let output = format!("{escape}\n{}", lines(60));
            assert!(!should_page(&tty_settings(Some(24)), &output));
        }
    }

    #[test]
    fn output_exactly_the_height_streams() {
        assert!(!should_page(&tty_settings(Some(10)), &lines(10)));
    }

    #[test]
    fn tall_output_pages() {
        assert!(should_page(&tty_settings(Some(24)), &lines(60)));
    }

    #[test]
    fn height_override_lowers_the_threshold() {
        assert!(should_page(&tty_settings(Some(10)), &lines(20)));
    }

    #[test]
    fn overflowing_lines_count_as_multiple_rows() {
        let settings = Settings {
            width: 40,
            ..tty_settings(Some(5))
        };
        let output: String = (0..3).map(|_| format!("{}\n", "x".repeat(100))).collect();
        assert!(should_page(&settings, &output));
    }

    #[test]
    fn never_pages_without_a_terminal() {
        let settings = Settings {
            stdout_is_terminal: false,
            ..tty_settings(Some(5))
        };
        assert!(!should_page(&settings, &lines(60)));
    }

    #[test]
    fn never_pages_when_disabled() {
        let settings = Settings {
            pager: None,
            ..tty_settings(Some(5))
        };
        assert!(!should_page(&settings, &lines(60)));
    }

    #[test]
    fn never_pages_with_unknown_height() {
        assert!(!should_page(&tty_settings(None), &lines(60)));
    }

    #[test]
    fn catmd_pager_takes_precedence_over_pager() {
        let env = Environment {
            catmd_pager: s("more"),
            pager: s("less"),
            ..Environment::default()
        };
        assert_eq!(resolve_pager(false, &env).unwrap().program, "more");
    }

    #[test]
    fn pager_value_is_split_into_program_and_arguments() {
        let env = Environment {
            pager: s("less -S"),
            ..Environment::default()
        };
        let pager = resolve_pager(false, &env).unwrap();
        assert_eq!(pager.program, "less");
        assert_eq!(pager.args, vec!["-S".to_string()]);
        let quoted = Environment {
            pager: s("less -P 'my prompt'"),
            ..Environment::default()
        };
        assert_eq!(
            resolve_pager(false, &quoted).unwrap().args,
            vec!["-P".to_string(), "my prompt".to_string()]
        );
    }

    #[test]
    fn cat_disables_paging() {
        let env = Environment {
            pager: s("cat"),
            ..Environment::default()
        };
        assert_eq!(resolve_pager(false, &env), None);
        assert_eq!(resolve_pager(true, &Environment::default()), None);
    }

    #[test]
    fn less_defaults_to_frx_only_when_unset() {
        let unset = resolve_pager(false, &Environment::default()).unwrap();
        assert_eq!(unset.env, vec![("LESS".to_string(), "FRX".to_string())]);
        let explicit_less = Environment {
            pager: s("less"),
            ..Environment::default()
        };
        assert_eq!(
            resolve_pager(false, &explicit_less).unwrap().env,
            vec![("LESS".to_string(), "FRX".to_string())]
        );
        let empty = Environment {
            less: s(""),
            ..Environment::default()
        };
        assert_eq!(resolve_pager(false, &empty).unwrap().env.len(), 1);
        let set = Environment {
            less: s("-i"),
            ..Environment::default()
        };
        assert!(resolve_pager(false, &set).unwrap().env.is_empty());
    }

    #[test]
    fn command_carries_program_arguments_and_less_env() {
        let env = Environment {
            pager: s("less -S"),
            ..Environment::default()
        };
        let command = resolve_pager(false, &env).unwrap().to_command();
        assert_eq!(command.get_program(), OsStr::new("less"));
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            vec![OsStr::new("-S")]
        );
        let envs: Vec<_> = command.get_envs().collect();
        assert_eq!(envs, vec![(OsStr::new("LESS"), Some(OsStr::new("FRX")))]);
    }

    #[test]
    fn command_leaves_less_alone_when_set() {
        let env = Environment {
            less: s("-i"),
            ..Environment::default()
        };
        let command = resolve_pager(false, &env).unwrap().to_command();
        assert_eq!(command.get_envs().count(), 0);
    }
}
