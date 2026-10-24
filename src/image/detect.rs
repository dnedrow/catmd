//! Capability detection and deterministic backend selection.

use clap::ValueEnum;

use super::{BackendKind, CellSize, ImageSettings};
use crate::cli::{Args, ImagesArg};
use crate::env::Environment;
use crate::theme::Glyphs;

/// Which graphical protocols the terminal is believed to support.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Capabilities {
    pub kitty: bool,
    pub sixel: bool,
}

/// Terminals known to speak the Kitty protocol, by lowercase `TERM_PROGRAM`.
const KITTY_PROGRAMS: &[&str] = &["ghostty", "wezterm"];
/// Terminals known to speak SIXEL, by lowercase `TERM_PROGRAM`.
const SIXEL_PROGRAMS: &[&str] = &["iterm.app", "mlterm", "contour"];

fn in_tmux(env: &Environment) -> bool {
    env.tmux.as_deref().is_some_and(|value| !value.is_empty())
}

/// Combine environment hints with the probe result. Inside tmux nothing is
/// assumed, since it swallows graphics sequences.
pub fn capabilities(env: &Environment) -> Capabilities {
    if in_tmux(env) {
        return Capabilities::default();
    }
    let term = env.term.as_deref().unwrap_or("").to_ascii_lowercase();
    let program = env
        .term_program
        .as_deref()
        .unwrap_or("")
        .to_ascii_lowercase();
    let probe = env.probe.unwrap_or_default();
    Capabilities {
        kitty: env
            .kitty_window_id
            .as_deref()
            .is_some_and(|id| !id.is_empty())
            || term.contains("kitty")
            || term.contains("ghostty")
            || KITTY_PROGRAMS.contains(&program.as_str()),
        sixel: probe.sixel
            || term.contains("sixel")
            || term.starts_with("foot")
            || term.starts_with("mlterm")
            || SIXEL_PROGRAMS.contains(&program.as_str()),
    }
}

/// The cell size the terminal reported, or the default.
pub fn cell_size(env: &Environment) -> CellSize {
    [env.cell_pixels, env.probe.and_then(|probe| probe.cell)]
        .into_iter()
        .flatten()
        .find_map(|(width, height)| CellSize::new(width, height))
        .unwrap_or_default()
}

/// The requested mode: the flag, else `CATMD_IMAGES`, else automatic.
pub fn mode(args: &Args, env: &Environment) -> ImagesArg {
    args.images
        .or_else(|| {
            env.catmd_images
                .as_deref()
                .and_then(|value| ImagesArg::from_str(value.trim(), true).ok())
        })
        .unwrap_or_default()
}

/// Choose the backend, or `None` to keep the plain image label.
///
/// An explicit mode is honored as given. Automatic selection needs a terminal,
/// then prefers Kitty, SIXEL, half-block, and finally the placeholder box when
/// color or Unicode cells are unavailable.
pub fn select(
    mode: ImagesArg,
    caps: Capabilities,
    stdout_is_terminal: bool,
    color_on: bool,
    glyphs: Glyphs,
) -> Option<BackendKind> {
    match mode {
        ImagesArg::None => None,
        ImagesArg::Kitty => Some(BackendKind::Kitty),
        ImagesArg::Sixel => Some(BackendKind::Sixel),
        ImagesArg::Blocks => Some(BackendKind::HalfBlock),
        ImagesArg::Placeholder => Some(BackendKind::Placeholder),
        ImagesArg::Auto => {
            if !stdout_is_terminal {
                None
            } else if !color_on {
                Some(BackendKind::Placeholder)
            } else if caps.kitty {
                Some(BackendKind::Kitty)
            } else if caps.sixel {
                Some(BackendKind::Sixel)
            } else if glyphs == Glyphs::Unicode {
                Some(BackendKind::HalfBlock)
            } else {
                Some(BackendKind::Placeholder)
            }
        }
    }
}

/// Resolve the image settings.
pub fn resolve(args: &Args, env: &Environment, color_on: bool, glyphs: Glyphs) -> ImageSettings {
    ImageSettings {
        backend: select(
            mode(args, env),
            capabilities(env),
            env.stdout_is_terminal,
            color_on,
            glyphs,
        ),
        cell: cell_size(env),
    }
}

/// Whether to query the terminal: only in automatic mode, on an interactive
/// terminal, when the hints have not already settled the question.
pub fn should_probe(args: &Args, env: &Environment) -> bool {
    mode(args, env) == ImagesArg::Auto
        && env.stdout_is_terminal
        && env.stdin_is_terminal
        && env.term.as_deref() != Some("dumb")
        && !in_tmux(env)
        && !capabilities(env).kitty
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::probe::ProbeResult;

    fn tty(term: &str) -> Environment {
        Environment {
            stdout_is_terminal: true,
            stdin_is_terminal: true,
            term: Some(term.to_string()),
            ..Environment::default()
        }
    }

    fn auto(env: &Environment, color: bool, glyphs: Glyphs) -> Option<BackendKind> {
        select(
            ImagesArg::Auto,
            capabilities(env),
            env.stdout_is_terminal,
            color,
            glyphs,
        )
    }

    #[test]
    fn kitty_wins_even_when_sixel_is_also_available() {
        let env = Environment {
            kitty_window_id: Some("1".into()),
            probe: Some(ProbeResult {
                sixel: true,
                ..ProbeResult::default()
            }),
            ..tty("xterm-256color")
        };
        assert_eq!(auto(&env, true, Glyphs::Unicode), Some(BackendKind::Kitty));
    }

    #[test]
    fn sixel_is_used_without_kitty() {
        let env = tty("foot");
        assert_eq!(auto(&env, true, Glyphs::Unicode), Some(BackendKind::Sixel));
        let probed = Environment {
            probe: Some(ProbeResult {
                sixel: true,
                ..ProbeResult::default()
            }),
            ..tty("xterm-256color")
        };
        assert_eq!(
            auto(&probed, true, Glyphs::Unicode),
            Some(BackendKind::Sixel)
        );
    }

    #[test]
    fn plain_terminals_get_half_blocks_then_the_placeholder() {
        let env = tty("xterm-256color");
        assert_eq!(
            auto(&env, true, Glyphs::Unicode),
            Some(BackendKind::HalfBlock)
        );
        assert_eq!(
            auto(&env, true, Glyphs::Ascii),
            Some(BackendKind::Placeholder)
        );
        assert_eq!(
            auto(&env, false, Glyphs::Unicode),
            Some(BackendKind::Placeholder)
        );
    }

    #[test]
    fn pipes_keep_the_plain_label() {
        let env = Environment {
            stdout_is_terminal: false,
            kitty_window_id: Some("1".into()),
            ..Environment::default()
        };
        assert_eq!(auto(&env, true, Glyphs::Unicode), None);
    }

    #[test]
    fn explicit_modes_override_detection() {
        let caps = Capabilities::default();
        let pick = |mode| select(mode, caps, false, false, Glyphs::Ascii);
        assert_eq!(pick(ImagesArg::Kitty), Some(BackendKind::Kitty));
        assert_eq!(pick(ImagesArg::Sixel), Some(BackendKind::Sixel));
        assert_eq!(pick(ImagesArg::Blocks), Some(BackendKind::HalfBlock));
        assert_eq!(pick(ImagesArg::Placeholder), Some(BackendKind::Placeholder));
        assert_eq!(pick(ImagesArg::None), None);
    }

    #[test]
    fn tmux_disables_detection() {
        let env = Environment {
            tmux: Some("/tmp/tmux-1/default,1,0".into()),
            kitty_window_id: Some("1".into()),
            ..tty("tmux-256color")
        };
        assert_eq!(capabilities(&env), Capabilities::default());
        assert!(!should_probe(&Args::default(), &env));
    }

    #[test]
    fn the_flag_beats_the_environment_variable() {
        let env = Environment {
            catmd_images: Some("SIXEL".into()),
            ..Environment::default()
        };
        assert_eq!(mode(&Args::default(), &env), ImagesArg::Sixel);
        let args = Args {
            images: Some(ImagesArg::None),
            ..Args::default()
        };
        assert_eq!(mode(&args, &env), ImagesArg::None);
        let bad = Environment {
            catmd_images: Some("bogus".into()),
            ..Environment::default()
        };
        assert_eq!(mode(&Args::default(), &bad), ImagesArg::Auto);
    }

    #[test]
    fn cell_size_prefers_the_ioctl_then_the_probe_then_the_default() {
        let mut env = Environment::default();
        assert_eq!(cell_size(&env), CellSize::DEFAULT);
        env.probe = Some(ProbeResult {
            cell: Some((9, 18)),
            ..ProbeResult::default()
        });
        assert_eq!(cell_size(&env), CellSize::new(9, 18).unwrap());
        env.cell_pixels = Some((10, 20));
        assert_eq!(cell_size(&env), CellSize::new(10, 20).unwrap());
        env.cell_pixels = Some((0, 0));
        assert_eq!(cell_size(&env), CellSize::new(9, 18).unwrap());
    }

    #[test]
    fn probing_only_happens_in_auto_mode_on_an_interactive_terminal() {
        let env = tty("xterm-256color");
        assert!(should_probe(&Args::default(), &env));
        let forced = Args {
            images: Some(ImagesArg::Blocks),
            ..Args::default()
        };
        assert!(!should_probe(&forced, &env));
        let piped_in = Environment {
            stdin_is_terminal: false,
            ..env.clone()
        };
        assert!(!should_probe(&Args::default(), &piped_in));
        assert!(!should_probe(&Args::default(), &tty("dumb")));
    }
}
