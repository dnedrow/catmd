# Design

## Context

See `proposal.md` — Why. Relevant current state (from `render-markdown-to-terminal`):

- `Theme` (`src/theme.rs`) is a struct of 25 `anstyle::Style` values (19 roles + `heading: [Style; 6]`) with a `Default` that is the only theme. It is constructed once, at `src/render/mod.rs:26`, and consumed through 21 `theme.<role>` reads in `render/mod.rs`, `render/inline.rs`, and `render/table.rs`. Bullets, ordered markers, and task checkboxes are emitted with `Span::plain`; table separators and the header rule use `theme.rule`.
- `Settings.color: bool` is the only color knob. `env::resolve` decides on/off from `--color`, `NO_COLOR`, `CLICOLOR*`, TTY, and `TERM=dumb`; `Environment` does not capture `COLORTERM`.
- The emitter is a single function, `Writer::render_line` (`src/render/layout.rs`), which calls `style.render()` per span when color is on. Nothing else produces escape sequences.
- The `color-output` main spec forbids `38;5;`/`38;2;`, and two tests enforce it (`theme::tests::theme_uses_only_the_16_color_palette`, `tests/cli.rs::styled_output_uses_only_the_16_color_palette`).
- `anstyle::Color` already models `Ansi`, `Ansi256`, and `Rgb`; `anstyle-lossy` converts between them and depends only on `anstyle`. `toml` 1.x parses to an untyped `toml::Table` with `default-features = false, features = ["parse"]`.
- Settings resolution is pure over an `Environment` snapshot (design D3 of the previous change) and tests rely on fabricated snapshots; theme loading is file I/O and must not break that.

## Goals / Non-Goals

**Goals:**

- Appearance becomes data: every visible element has a stable role name a theme file can address, and the renderer never learns where a style came from.
- Depth is applied in exactly one place (the emitter), so a theme author writes a color once and every terminal receives a valid sequence.
- Byte-identical default output for users without a theme, including after the new roles are introduced.
- Theme errors are loud, specific (role/modifier/color named), and exit before any output.
- Keep settings resolution pure and unit-testable; keep the dependency footprint small.

**Non-Goals:**

- Background colors, glyph overrides, `inherits` between theme files, named built-in themes beyond the default, or a general config file (all listed as follow-ups in the proposal).
- Detecting color depth by querying the terminal (e.g. `OSC 4`/`DA` replies). Environment variables only, matching how every other CLI tool does it.
- Perceptual color matching. Nearest-color downgrading uses `anstyle-lossy`'s RGB distance; good enough for UI accents.

## Decisions

### D1. `ColorDepth` replaces `Settings.color`

```
enum ColorDepth { Off, Ansi16, Ansi256, TrueColor }
```

`Off` is what `color: false` meant; the three others are "styling on, at this depth". `env::resolve` first makes today's on/off decision unchanged, then, when on, resolves depth: `--color-depth` if not `auto`; else `COLORTERM ∈ {truecolor, 24bit}` (case-insensitive) or `TERM` contains `direct` → `TrueColor`; else `TERM` contains `256color` → `Ansi256`; else `Ansi16`. `Environment` gains `colorterm`. Depth never consults the TTY flag, so `--color always | less -R` keeps the user's real depth.

- *Alternative*: keep `color: bool` and add a separate `depth` field. Rejected — two fields with an invalid combination (`color: false, depth: TrueColor`) versus one enum where `Off` is just the zero depth. Tests constructing `Settings` are ours to update.

### D2. Downgrade in the emitter, nowhere else

`Writer` holds the `ColorDepth`. In `render_line`, before `style.render()`, the span's foreground is passed through:

```
fn clamp(color: anstyle::Color, depth: ColorDepth) -> anstyle::Color
  TrueColor -> unchanged
  Ansi256   -> Rgb(c)      => Ansi256(anstyle_lossy::rgb_to_xterm(c)); others unchanged
  Ansi16    -> Rgb|Ansi256 => Ansi(anstyle_lossy::color_to_ansi(c, palette::VGA)); Ansi unchanged
```

`Ansi` colors pass through untouched at every depth, which is what makes the default theme byte-identical. Because this is the only escape-producing site, the "never exceed the resolved depth" requirement is structural: a theme cannot smuggle a `38;2;` past it.

- *Alternative*: downgrade once at theme-load time. Rejected — depth is resolved from the environment and the theme from a file; coupling them would mean re-loading the theme to change depth, and the layout code would have to know about depth anyway for tests.

### D3. `Theme` grows four roles and a name-based setter

Add `bullet`, `list_number`, `task_checkbox`, and `table_border` to `Theme`. Defaults: the first three are `Style::new()` (what `Span::plain` produced), `table_border` is `Style::new().dimmed()` (what `rule` is). `render/mod.rs` and `render/table.rs` switch those emission sites from `Span::plain`/`theme.rule` to the new roles — a rename, not a behavior change, which the "default appearance is unchanged" scenario pins.

`Theme` gains:

```
pub const ROLES: [&str; 29]                      // the public vocabulary, in documentation order
pub fn set(&mut self, role: &str, style: Style) -> Result<(), UnknownRole>
pub fn get(&self, role: &str) -> Option<Style>   // for tests and --help text
```

`heading1`…`heading6` map onto `heading[0..6]`. The renderer keeps reading fields directly; only the file loader goes through `set`.

- *Alternative*: `HashMap<String, Style>` keyed by role. Rejected — the renderer would pay a lookup per span and lose compile-time checking of role names; the struct stays the source of truth and the name mapping is one `match`.

### D4. Theme files parse to an untyped `toml::Table`

`toml = { version = "1.1", default-features = false, features = ["std", "parse", "serde"] }` — `toml::Table` is gated behind toml's `serde` feature, which pulls in `serde_core` (tiny) but not `serde_derive`; `display` (serialization) is left off. The loader walks the top-level table:

```
for (key, value) in table:
  key == "palette"      -> value must be a table; each entry parsed as a Color (any form)
  key in Theme::ROLES   -> value is String | Integer -> fg only
                                    | Table  -> optional fg, optional modifiers array
  otherwise             -> ThemeError::UnknownRole(key)
```

Palette entries are parsed first so a role can reference them. A role entry *replaces* the built-in style for that role (table form with neither key yields `Style::new()`); this is simpler to reason about than merging and is what the spec's "bare string replaces modifiers too" scenario pins.

- *Alternative*: `serde` derive with a 29-field struct and `deny_unknown_fields`. Rejected — the polymorphic value (`"red"` | `196` | `{ fg, modifiers }`) needs a custom deserializer anyway, error messages for unknown fields are less controllable, and the typed approach adds `serde_derive` to the build for no gain.

### D5. Color and modifier syntax lives in `theme::color`

```
parse_color(value: &toml::Value, palette: &Palette) -> Result<anstyle::Color, ColorError>
  Integer 0..=255          -> Ansi256(n)           (out of range -> error)
  String in palette        -> the palette color     (checked before ANSI names, per spec)
  String "#rrggbb"         -> Rgb (exactly 6 hex digits, case-insensitive; "#f80" -> error)
  String ANSI name         -> Ansi                  (16 lowercase names, bright_ prefix)
  anything else            -> error naming the value
parse_modifier(&str) -> Result<Effects, ModifierError>   // bold dim italic underline strikethrough
```

Palette values themselves are parsed with an empty palette, so palette entries cannot reference each other (keeps resolution single-pass and acyclic).

### D6. Theme selection is pure; loading is I/O in `main`

```
theme::select(flag: Option<&str>, env: &Environment) -> Option<ThemeSource>
  value = flag, else CATMD_THEME when non-empty, else None
  contains '/' (or '\\' on Windows) or ends with ".toml" -> ThemeSource::Path(value)
  otherwise -> ThemeSource::Named { name, path: <config_dir>/catmd/themes/<name>.toml }
  config_dir = XDG_CONFIG_HOME when non-empty, else HOME/.config

theme::load(source: &ThemeSource) -> Result<Theme, ThemeError>   // fs::read_to_string + parse
```

`Environment` gains `catmd_theme`, `xdg_config_home`, and `home`. `main` calls `select` then `load`, prints `catmd: theme <path>: <error>` and exits 2 on failure *before* reading any input, and stores the result in `Settings.theme`. `Settings::default()` keeps `Theme::default()`, so `render()` tests are unaffected. `env::resolve` stays pure and does not touch the file system.

`ThemeError` variants: `Read(path, io::Error)`, `Parse(path, toml::de::Error)` (message includes line/column), `UnknownRole(path, key)`, `UnknownModifier(path, name)`, `InvalidColor(path, role, value)`, `BadShape(path, key, what)` (e.g. `palette` not a table, `modifiers` not an array). All carry the resolved path so the "missing named theme" message shows where catmd looked.

### D7. CLI surface

`--theme <THEME>` (free string, no default; the env default is handled in `select`, not clap, so precedence stays visible in one place) and `--color-depth <auto|16|256|truecolor>` (clap `ValueEnum` with explicit value names; default `auto`; exit 2 on anything else via clap).

### D8. Module layout

```
src/theme/
  mod.rs      Theme (+4 roles, ROLES, set/get), Default, Glyphs (moved verbatim from theme.rs)
  color.rs    ColorDepth, detect(), clamp(), parse_color(), parse_modifier()
  file.rs     ThemeSource, select(), load(), ThemeError
```

`render/layout.rs` takes `ColorDepth` in `Writer::new`; `render/mod.rs`, `render/table.rs` adopt the new roles; `env.rs` adds the four `Environment` fields and depth resolution; `cli.rs` adds two flags; `main.rs` adds theme selection/loading and the exit-2 path.

### D9. Testing

- **Unit** (`theme::color`): every color form, rejection cases, modifier names, `clamp` at each depth (`#ff0000` → `Ansi256(196)` → `Ansi(Red|BrightRed)`; named colors untouched), depth detection matrix in `env` tests.
- **Unit** (`theme::file`): `select` precedence and path-vs-name rules with fabricated `Environment`s; `load` against inline TOML strings written to temp files for each error variant and each value shape.
- **Library** (`tests/render.rs`): render with a `Settings` carrying a custom `Theme` and `ColorDepth`, asserting exact sequences (`38;5;196`, `38;2;255;0;0`, `31`/`91`), the new roles taking effect, and a byte-for-byte comparison of the default theme at `TrueColor` versus the previous expected output for a list+table document.
- **CLI** (`tests/cli.rs`): fixture theme files in a temp dir; `--theme` path/name/XDG/HOME/`CATMD_THEME`/precedence; every fail-fast case asserting exit 2, empty stdout, and the named token on stderr; `--color-depth` with `TERM`/`COLORTERM` combinations through `--color always`. The test helper's env-scrub list gains `COLORTERM`, `CATMD_THEME`, `XDG_CONFIG_HOME`, and `HOME` (setting `HOME` to a temp dir where needed).
- The two existing "no `38;5;`" tests become explicit about depth: 16-color for any theme, and the default theme at truecolor.

## Risks / Trade-offs

- [Depth detection lies under tmux/screen/ssh] → `--color-depth` is the documented escape hatch; `TERM` containing `256color` is the conventional, widely-set signal.
- [`anstyle-lossy`'s nearest-ANSI mapping may pick `red` vs `bright_red` differently than a user expects] → Spec scenarios accept either; theme authors who care can name ANSI colors directly, which are never converted.
- [Role names are now public API] → They are frozen by the `theming` spec; adding roles later is additive, renaming is a spec change.
- [Entry-replaces-style semantics surprise users who write `strong = "red"` and lose bold] → Documented in README with the table form shown first; the alternative (merge) makes "remove bold from strong" impossible to express.
- [`toml` with `default-features = false` may not expose table parsing without `serde`] → Falls back to default features; cost is `serde_core`, which is tiny.
- [Reading `HOME` for the XDG fallback on Windows] → Best-effort as before; `--theme <path>` always works.

## Migration Plan

No data or persisted state. Users without `--theme`/`CATMD_THEME` see identical output. The two tests that encoded the 16-color-only guarantee are updated in the same change. Rollback is reverting the change.

## Open Questions

- Whether to list the role vocabulary in `--help` or only in the README. Either satisfies the spec; the README table is required regardless.
