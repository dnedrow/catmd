# Design

## Context

See `proposal.md` — Why. The repository is a bare `cargo new` (edition 2024, `rustc 1.99`) with a single dependency, `comrak = "0.56"`, pulled in with its default features. Nothing else exists yet, so this design establishes the crate structure as well as the rendering approach.

Constraints that shape the approach:

- `comrak`'s default features (`cli`, `syntect-onig`, `bon`) drag in `clap`, `syntect`, and the `onig_sys` C build. None are needed for AST access; `Options` derives `Default` without the `bon` builder.
- `comrak` already does the hard parsing work the specs rely on: GFM tables/strikethrough/tasklists/autolinks/footnotes/alerts are plain `Options` toggles, front matter can be split off with `front_matter_delimiter`, and footnote definitions are moved to the end of the document in first-reference order with `ix` stamped on each reference.
- The color-convention decision tree (`NO_COLOR`, `CLICOLOR`, `CLICOLOR_FORCE`, `TERM=dumb`, CI, Windows VT enablement) is implemented by `anstyle-query`; `terminal_size` reads columns/rows. Both are small and already in the dependency graph via `clap`.
- Several spec scenarios depend on stdout being a terminal (width/height detection, paging), which ordinary `cargo test` processes cannot provide. The architecture must keep those decisions testable without a pty.
- Requirements are written as exact plain-text output (`openspec/changes/render-markdown-to-terminal/specs/**`), so the renderer must be deterministic and width-aware down to the column.

## Goals / Non-Goals

**Goals:**

- A renderer that is exact and deterministic for a given (markdown, width, color, glyph set) tuple, so spec scenarios translate directly into tests.
- Every environment-dependent decision (color, width, height, glyphs, pager) is resolved once, up front, from a snapshot of the environment, and the rest of the program is pure.
- Styles never cross line boundaries; plain output is byte-identical to styled output with escapes removed.
- Small dependency footprint and fast builds (no C compilation).
- Room to add syntax highlighting, OSC-8 links, and table fitting later without restructuring.

**Non-Goals:**

- Streaming/incremental rendering. Documents are rendered fully into memory before output (needed to count lines for paging; Markdown files are small).
- Reacting to terminal resize during output or while the pager is running.
- Pixel/image rendering, a configuration file, or user-defined themes.
- A built-in interactive pager.

## Decisions

### D1. Trim comrak to `default-features = false`

Use `comrak = { version = "0.56", default-features = false }`. Enable extensions through `Options` fields: `table`, `strikethrough`, `tasklist`, `autolink`, `footnotes`, `alerts`, `front_matter_delimiter = Some("---")`. Leave `underline`, `superscript`, `subscript`, `spoiler`, `math_dollars`, `shortcodes`, `description_lists`, `multiline_block_quotes`, `wikilinks` off so parsing matches GitHub.

- *Alternative*: keep defaults for a one-line Cargo change. Rejected — doubles compile time, adds a C toolchain requirement, and ships a second CLI parser.
- *Alternative*: `pulldown-cmark` (mdcat's parser). Rejected — comrak's full AST (not an event stream) makes tables, nested lists, and footnote renumbering simpler, and the user specifically chose comrak.

### D2. Library + binary crate

`src/lib.rs` exposes the pure core — `Settings`, `Glyphs`, `Theme`, `render(markdown: &str, settings: &Settings) -> String`, and the pager decision — and `src/main.rs` is a thin binary that parses arguments, snapshots the environment, resolves `Settings`, reads inputs, and performs I/O.

Why: TTY-dependent scenarios (detected width, paging threshold) are tested by calling the library with explicit `Settings` and a fake `Environment`, while non-TTY CLI behavior is tested end-to-end with `assert_cmd`. No pty is needed.

Proposed module layout:

```
src/
  lib.rs            re-exports; render() entry point
  cli.rs            clap derive: Args { files, color, width, height, ascii, no_pager }
  env.rs            Environment snapshot (is_tty, size, env vars) + resolve(Args, Environment) -> Settings
  input.rs          Source enum (File(PathBuf) | Stdin); read_all() with UTF-8 lossy decoding; error collection
  theme.rs          Theme (anstyle::Style per role) and Glyphs (unicode | ascii)
  render/
    mod.rs          Renderer: walks the comrak AST, dispatches blocks
    inline.rs       inline nodes -> Vec<Span { text, style }>
    layout.rs       prefix stack, width-aware greedy wrapping, line emission
    table.rs        column measurement, alignment, header rule
  pager.rs          should_page(settings, rendered) and spawn/stream fallback
  main.rs           wiring, exit codes, BrokenPipe handling
```

### D3. Resolve all environment decisions into one `Settings` value, up front

`env::Environment` is a plain struct captured once in `main`: `stdout_is_terminal: bool`, `terminal_size: Option<(u16, u16)>` (columns, rows — read from the terminal attached to stdout), and the relevant environment variables (`TERM`, `NO_COLOR`, `CLICOLOR`, `CLICOLOR_FORCE`, `COLUMNS`, `LINES`, `LC_ALL`, `LC_CTYPE`, `LANG`, `CATMD_PAGER`, `PAGER`, `LESS`). `env::resolve(&Args, &Environment) -> Settings` is a pure function producing:

```
Settings {
  color:              bool,                 // final answer; nothing downstream re-detects
  width:              usize,                // max(resolved, 20)
  height:             Option<usize>,        // None = unknown -> never page
  glyphs:             Glyphs,               // Unicode | Ascii
  pager:              Option<PagerCommand>, // None = paging disabled
  stdout_is_terminal: bool,                 // carried for the paging decision
}

PagerCommand { program: String, args: Vec<String>, env: Vec<(String, String)> }
```

`PagerCommand` is fully resolved: the `CATMD_PAGER`/`PAGER`/`less` command line is already split with `shell-words`, and `env` already holds `LESS=FRX` when `LESS` is unset or empty, so the pager module only has to spawn it (see D8).

Color precedence (auto mode): `NO_COLOR` non-empty → off; `CLICOLOR_FORCE` non-empty and not `0` → on; `CLICOLOR=0` → off; otherwise on only when stdout is a terminal and `TERM` is not `dumb` — an unset `TERM` counts as capable, matching the spec's "not identified as dumb". `--color always|never` short-circuits. The decision is made against the *original* stdout, which is what the paging spec requires. These conventions are evaluated against the `Environment` snapshot rather than through `anstyle-query`'s lookup functions, which read the live process environment and would defeat tests with fabricated snapshots. On Windows, `env::prepare_terminal(&Settings)` — called from `main` right after `resolve`, keeping `resolve` pure — calls `anstyle_query::windows::enable_ansi_colors()` once when color resolved on.

Width precedence: `--width` → terminal columns (only when stdout is a terminal) → `COLUMNS` → 80, then clamp to ≥ 20. Height: `--height` → terminal rows → `LINES` → `None`.

- *Alternative*: write through `anstream::AutoStream`, which strips ANSI automatically. Rejected — it decides per stream, so output piped to the pager would be stripped, and the renderer also needs the boolean to choose heading/alert styling. The conventions are small enough to mirror directly against the snapshot.

### D4. Rendering pipeline: AST → styled spans → layout → emit

```
  comrak AST
      |
      v  render/inline.rs
  Vec<Span { text: String, style: anstyle::Style }>     per paragraph/cell/heading
      |
      v  render/layout.rs
  greedy word-wrap measured with unicode-width; prefix stack applied per line
      |
      v  emit
  if settings.color { style.render() + text + style.render_reset() } else { text }
```

- Inline styles compose by merging `anstyle::Style` values down the tree (effects are OR-ed; a nested foreground color wins). The theme is a struct of `Style` constants restricted to effects and `AnsiColor` (16-color) values — this makes the "no `38;5;`/`38;2;`" requirement structural rather than policed.
- Each `Span` fragment is opened and closed individually on the line where it lands, so styles never span a line break. Adjacent fragments with identical styles are merged within a line to reduce escape noise.
- Wrapping splits spans on spaces into words while keeping each word's style, then fills lines greedily against `width - prefix_width`. A word wider than the remaining width is placed on its own line and allowed to overflow. East Asian wide characters (CJK ideographs, emoji) are individual zero-width break opportunities, following UAX #14's treatment of the ideographic class, since such text has no spaces to break at. `LineBreak` nodes force a new line; `SoftBreak` becomes a space.
- *Alternative*: `textwrap`. Rejected as the primary mechanism — it wraps plain strings and would need the styles re-applied afterwards; a greedy fill over styled words is ~50 lines and keeps the invariant obvious. `unicode-width` is used directly for measurement.

### D5. Nesting via a prefix stack

`layout::Writer` holds a stack of `Prefix { first: Vec<Span>, rest: Vec<Span> }`. Emitting a line concatenates every prefix (using `first` until a prefix has emitted its first line, then `rest`) before the content. This single mechanism produces:

- list items: `first = "• "`, `rest = "  "` (ordered: marker right-aligned to the widest marker in the list, `rest` = spaces of the same width);
- task items: `first = "[x] "`, `rest = "    "`;
- block quotes / alerts: `first = rest = "│ "` (styled gutter), stacked for nesting;
- code blocks: a fixed `"    "` prefix with the whole line in the code style and no wrapping;
- footnote definitions: `first = "[n] "`, `rest = "    "`.

Block spacing is handled by the writer: a block requests a blank line *before* itself, and the writer suppresses it at the very start of output and inside tight lists. Output always ends with exactly one `\n`, and the writer trims trailing spaces on every non-code line.

### D6. Headings keep their `#` markers

Render `#`×level (dim) + space + text (bold; level 1 also underlined). Setext headings are normalized to the same form. Rationale: with `--color never` or piped output there are no attributes, and marker-less headings would be indistinguishable from paragraphs; the markers also keep H3–H6 hierarchy visible. This supersedes the "markers removed" default floated during exploration.

### D7. Tables at natural width

Each cell's inlines are rendered to spans (no wrapping), measured with `unicode-width`, and columns are padded to their widest cell; the last column is not right-padded. Alignment comes from `NodeTable.alignments` (header cells follow their column's alignment, matching GFM). No column fitting in this version — a table wider than the terminal simply overflows, which the spec permits.

### D8. Paging: render to a buffer, then decide

`render()` returns a `String`. `pager::should_page(&Settings, &str)` is true only when `stdout_is_tty && pager != Disabled && height.is_some() && visual_rows(output, width) > height`, where `visual_rows` counts each line as `max(1, ceil(display_width / width))` so overflowing code/table lines are accounted for.

Pager command resolution: first non-empty of `CATMD_PAGER`, `PAGER`, else `less`; `cat` (or `--no-pager`) → `Disabled`. The command string is split with `shell-words` and executed directly via `std::process::Command` with piped stdin; `LESS=FRX` is added to the child environment when `LESS` is unset. On spawn failure, report to stderr and write the buffer to stdout. The parent waits for the pager; the exit code reflects input errors only.

- *Alternative*: `sh -c "$PAGER"`. Rejected — not portable to Windows and brings shell quoting surprises; `shell-words` handles the common `less -S` case the spec requires.
- *Alternative*: always spawn `less -F` and let it decide. Rejected — makes catmd depend on `less` semantics for the common short-document case and breaks `PAGER=more`.

### D9. Input and exit codes

Each `Source` is read fully as bytes and decoded with `String::from_utf8_lossy`; `-` maps to stdin. Each source is parsed and rendered independently and the renderings are joined with a single blank line (empty renderings contribute nothing). Read errors are printed as `catmd: <path>: <io error>` to stderr and the run continues; exit code is `1` if any source failed, `0` otherwise, `2` for usage errors (clap's default). A `BrokenPipe` on stdout (e.g. `catmd x.md | head`) exits silently with the input-read status (`0`, or `1` if a source could not be read), as `cat` effectively does.

### D10. Testing strategy

- **Unit tests** (in-module): `env::resolve` across the color/width/height/glyph matrices with fabricated `Environment` values; `layout` wrapping with ASCII, CJK, and overlong words; `pager::should_page` and command resolution.
- **Library tests** (`tests/render.rs`): call `render()` with explicit `Settings` and compare against exact expected strings taken from the spec scenarios (plain mode), plus targeted checks for attribute placement in color mode (e.g. bold open/close on each wrapped line, no `38;5;`).
- **CLI tests** (`tests/cli.rs`, `assert_cmd` + `predicates`): files/stdin/`-` ordering, missing-file stderr and exit code, `--color always` through a pipe, `COLUMNS`/`LANG`/`--ascii`/`--width` effects, `--color bogus` and `--height 0` usage errors, and that piped output is never paged even with `--height 5`.
- Pager spawning with a real terminal is verified manually; the decision and command resolution are covered by unit tests. No snapshot framework — expected output is inline so the tests double as readable examples.

## Risks / Trade-offs

- [Italic/strikethrough are not rendered by every terminal] → These are standard SGR attributes; nothing to do in-tool. Documented as a terminal limitation.
- [Visual row estimate is approximate for ambiguous-width emoji] → Worst case a slightly-too-tall document streams instead of paging, or vice-versa; `LESS=F` exits immediately if it fits anyway.
- [`less` missing in minimal containers] → Spawn failure falls back to direct output with a stderr note; exit code unaffected.
- [Whole-document buffering] → Fine for Markdown-sized inputs; a multi-MB file still renders in well under a second. Streaming can be added later behind `--no-pager` if ever needed.
- [Locale heuristic for glyphs may choose Unicode on a non-UTF-8 terminal when no locale variables are set] → `--ascii` is the explicit override; the default favors modern terminals (including Windows Terminal, where `LANG` is usually unset).
- [comrak minor versions change the AST] → Pin to `0.56`; the renderer touches a small, stable subset of `NodeValue`.
- [Wide tables and long code lines overflow] → Accepted by spec for this version; column fitting is the natural follow-up and D7 keeps measurement centralized for it.

## Migration Plan

Greenfield: `src/main.rs` is replaced and new modules added; no data, config, or persisted state exists. Rollback is reverting the change. The first commit of the repository will contain this work.

## Open Questions

- Exact palette per role (which of the 16 colors for H1 vs. H2, inline code, link URLs, alert types). Deferred to implementation; the specs constrain only attributes and the 16-color palette, so any choice is conforming.
- Whether a future version should surface the fenced-block info string (language label) once highlighting exists. Not needed now; the AST carries it.
