# Proposal

## Why

`catmd` renders with a single hard-coded 16-color theme and forbids anything richer, so users on 256-color and truecolor terminals cannot get colors that match their terminal palette, and there is no way to adjust a color that clashes with a background. Externalizing the theme and letting it use the terminal's real color depth turns appearance into user data instead of a code change.

## What Changes

- Add a theme file format (TOML) that assigns a foreground color and text modifiers to each rendering role — headings by level, emphasis, code, links, quote gutters, alert titles, table borders, bullets, and the rest. Colors may be written as one of the 16 ANSI names, a 256-color index (`0`–`255`), a hex `#rrggbb` value, or a name from an optional `[palette]` table. Roles a theme omits keep the built-in default.
- Add `--theme <path|name>`: an argument containing a path separator or ending in `.toml` is read as a file; otherwise it names `<name>.toml` under `$XDG_CONFIG_HOME/catmd/themes/` (`~/.config/catmd/themes/` when unset). `CATMD_THEME` supplies the same value when the flag is absent; the flag wins.
- Fail fast on a bad theme: an unreadable file, invalid TOML, an unknown role, an unknown modifier, or an unparseable color is reported on standard error naming the theme and exits with status 2 rather than silently falling back.
- Detect the terminal's color depth — `COLORTERM=truecolor|24bit` → 16 million colors, `TERM` containing `256color` → 256 colors, `TERM` containing `direct` → 16 million, otherwise 16 — and add `--color-depth <auto|16|256|truecolor>` to override it. Depth is read from the environment only, so it also applies to `--color always` through a pipe.
- Downgrade every emitted color to the resolved depth: hex colors become the nearest 256-color index on a 256-color terminal and the nearest of the 16 ANSI colors on a basic terminal; 256-color indices become the nearest ANSI color on a basic terminal. A theme author writes a color once and every reader's terminal gets a valid sequence. **BREAKING** for the existing spec guarantee only: output may now contain `38;5;` and `38;2;` sequences, but only when the terminal (or `--color-depth`) says it supports them; a 16-color terminal still never receives them.
- Keep the built-in default theme exactly as it is today (16-color). Richer colors appear only when a theme asks for them.
- Expose four roles the renderer styles implicitly today so themes can address everything visible: `bullet` (currently unstyled), `list_number` (currently unstyled), `task_checkbox` (currently unstyled), and `table_border` (currently borrows `rule`). Their defaults reproduce today's appearance.

Out of scope (follow-up candidates): glyph overrides in theme files, background colors, theme inheritance between files (`inherits = ...`), a richer second built-in theme, and a general configuration file.

## Capabilities

### New Capabilities

- `theming`: How a user supplies an external theme — file format, color syntax, the role vocabulary and its defaults, partial-theme inheritance, `--theme`/`CATMD_THEME` lookup and precedence, and error handling for invalid themes.

### Modified Capabilities

- `color-output`: The requirement "Styling uses portable terminal attributes" changes from "16-color palette only" to "never exceed the terminal's resolved color depth"; new requirements cover color-depth detection from `COLORTERM`/`TERM`, the `--color-depth` override, and lossy downgrading of theme colors to the resolved depth.

## Impact

- **`Cargo.toml`**: new runtime dependencies `toml` (untyped table parsing with `default-features = false, features = ["parse"]`; no direct `serde` dependency) for theme files, and `anstyle-lossy` for palette downgrading (depends only on `anstyle`, already present).
- **`src/theme.rs`**: `Theme` gains the four new roles and becomes loadable from a parsed theme file; a `Default` that reproduces today's appearance stays as the fallback. Likely split into `theme/` with the file format parser and color/modifier parsing as separate modules.
- **`src/cli.rs` / `src/env.rs`**: `--theme` and `--color-depth` flags; `Environment` captures `COLORTERM`, `CATMD_THEME`, and `XDG_CONFIG_HOME`/`HOME`; `Settings.color: bool` becomes a color-depth value; the resolved `Theme` travels with `Settings`.
- **`src/render/layout.rs`**: the emitter downgrades each span's color to the resolved depth before rendering escape sequences — the only place depth is applied.
- **`src/render/mod.rs`, `table.rs`**: bullets, list numbers, task checkboxes, and table borders read their style from the theme instead of being plain or sharing `rule`.
- **`src/main.rs`**: theme loading (file I/O) happens here before settings resolution; a theme error exits 2.
- **Tests**: the two tests asserting "no `38;5;`/`38;2;` ever" become depth-conditional; new unit tests for color parsing, downgrading, depth detection, and theme loading; CLI tests with fixture theme files and `--color-depth`.
- **`README.md`** and **`examples/themes/solarized-dark.toml`**: document theme files, `--theme`, `CATMD_THEME`, `--color-depth`, and `COLORTERM`, with a shipped example theme that exercises every color form.
- **Compatibility**: default output is byte-identical to today for users without a theme; existing invocations and exit codes are unchanged.
