# Proposal

## Why

`cat`-ing a Markdown file dumps raw syntax (`**`, `#`, `|---|`) that is tedious to read in a terminal. The `catmd` project exists to be a drop-in `cat` for Markdown — stream a file to the terminal with headings, emphasis, lists, code, quotes, and tables rendered as styled text — but today it is only a `cargo new` scaffold with a `Hello, world!` and a `comrak` dependency. This change defines and delivers the first usable version.

## What Changes

- Add a `catmd` command-line tool that reads one or more Markdown files (or stdin), parses them with `comrak` (CommonMark + GFM extensions), and writes a terminal-formatted rendering to stdout.
- Render inline constructs as terminal text attributes: strong → bold, emphasis → italic, strikethrough, inline code highlighted, links underlined with their URL shown. Parsing stays CommonMark/GitHub-compatible (`__text__` is bold; the comrak `underline` extension is **not** enabled).
- Render block constructs with layout: headings (bold, `#` level markers kept but dimmed so hierarchy survives plain-text output), paragraphs wrapped to the terminal width, bullet/ordered/nested lists, task items, block quotes and GitHub alerts with a gutter, fenced/indented code blocks (never re-wrapped, no syntax highlighting in this version), tables with borders, thematic breaks spanning the width.
- Automatically detect whether stdout supports color (TTY check, `TERM`, `NO_COLOR`, `CLICOLOR`/`CLICOLOR_FORCE`) and the terminal's width and height; expose `--color <auto|always|never>`, `--width <N>`, and `--height <N>` overrides. Output is plain text (no escape sequences) when color is off or stdout is not a terminal.
- Use the detected/provided height for git-style auto-paging only: when stdout is a terminal and the rendered output is taller than the terminal, pipe it through `$PAGER` (default `less`, run with `LESS=FRX` when `LESS` is unset); otherwise stream directly. `--no-pager` disables this. catmd never implements its own interactive pager.
- Use Unicode glyphs (bullets, rules, box borders) when the locale is UTF-8, with an `--ascii` flag to force plain ASCII equivalents.
- Trim the `comrak` dependency to `default-features = false` (dropping its bundled CLI, `syntect`, and the `onig` C build) and add the small set of crates needed for argument parsing, terminal probing, and width-aware wrapping.

Out of scope for this change (candidates for follow-ups): syntax highlighting inside code blocks, OSC-8 clickable hyperlinks, shrinking/wrapping table columns to fit narrow terminals, rendering images, a configuration file or theming, and any interactive/built-in pager.

## Capabilities

### New Capabilities

- `input-sources`: Which inputs `catmd` reads — positional file paths, `-`/no-argument stdin, concatenation order, and behavior when a file cannot be read.
- `color-output`: How the tool decides whether to emit ANSI styling — the `--color` mode flag, `NO_COLOR`/`CLICOLOR` conventions, TTY detection, and the guarantee that plain output contains no escape sequences.
- `terminal-geometry`: How the rendering width and the paging height are determined — terminal size detection, `--width`/`--height` overrides, fallbacks when not attached to a terminal, and minimum-width clamping.
- `inline-rendering`: How inline Markdown constructs (strong, emphasis, strikethrough, code spans, links, images, autolinks, line breaks, inline HTML) are mapped to terminal text and attributes.
- `block-rendering`: How block constructs (headings, paragraphs, lists, task items, block quotes, alerts, code blocks, tables, thematic breaks, footnotes, HTML blocks, front matter) are laid out within the available width, including Unicode vs. ASCII glyph selection.
- `output-paging`: When and how rendered output is handed to an external pager based on terminal height, and how color is preserved through the pager.

### Modified Capabilities

_None — this project has no existing specs._

## Impact

- **`Cargo.toml`**: `comrak` switched to `default-features = false` with only the needed extension features; new runtime dependencies for CLI parsing (`clap`), terminal size (`terminal_size`), color-convention detection and styling (`anstyle`, `anstyle-query`), and display-width measurement (`unicode-width`); test-only dependencies for CLI integration tests.
- **`src/`**: `main.rs` replaced by a real entry point; new modules for CLI arguments, terminal probing, the AST-to-styled-text renderer, layout/wrapping, output emission, and pager hand-off (structure detailed in `design.md`).
- **Build footprint**: dropping comrak's default features removes `syntect`, `onig_sys` (C compilation), `xdg`, `plist`, and comrak's internal `clap` usage from the dependency graph; a `clap` dependency is re-added directly for catmd's own CLI.
- **Platforms**: standard output behavior on macOS and Linux; Windows is supported via the same ANSI/VT path (`anstyle-query` handles VT enablement) and is a best-effort target in this version.
- **No external services, APIs, or data formats** are introduced. Exit status follows `cat` conventions (non-zero if any input could not be read).
