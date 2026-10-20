# Proposal

## Why

Fenced code is the part of a README people read most carefully, and `catmd` paints every line of it one flat color, so a Rust function, a TOML table, and a shell session all look the same and keywords, strings, and comments give the eye nothing to hold on to. The first render change deliberately left highlighting out to keep the dependency footprint at zero; now that themes exist, the renderer already emits per-span styles and clamps every color to the terminal's depth, so highlighting is a matter of producing more spans per line — the plumbing is in place and the gap is the most visible one left.

## What Changes

- Highlight fenced code blocks whose info string names a language the bundled grammar set knows. Tokens are classified by Sublime Text scope (keyword, string, comment, constant, type, function, tag) and styled through new theme roles; everything else in the block keeps the existing `code_block` style. Content is never altered, wrapped, or re-indented — highlighting only changes which style each run of text carries.
- Resolve the language from the first word of the info string (so ```` ```rust,ignore ```` and ```` ```js title="x" ```` work), through a small alias table for common README-isms (`console`/`shell` → Bash, `jsonc` → JSON, `tf`/`hcl` → Terraform, `text`/`plaintext`/`txt` → none), then by grammar name or file extension, case-insensitively. A fenced block with no language, an unknown language, or an indented code block renders exactly as it does today.
- Bundle the grammar set curated by the `bat` project via the `two-face` crate (pure-Rust `fancy-regex` build), which adds TOML, TypeScript/TSX, Swift, Dockerfile, Kotlin, Terraform, Nix, Zig, Protobuf, Fish, and about 150 others to syntect's default set. The pure-Rust build omits six grammars (PowerShell, Sass, LiveScript, ARM assembly, Babel JavaScript, Salt); those languages fall back to the unhighlighted rendering.
- Add seven theme roles — `syntax_keyword`, `syntax_string`, `syntax_comment`, `syntax_constant`, `syntax_type`, `syntax_function`, `syntax_tag` — with 16-color defaults (magenta, green, dim, red, yellow, blue, bright blue). Like every other role they accept a bare color or a `{ fg, modifiers }` table, and a theme that omits them keeps the defaults; existing theme files load unchanged.
- Keep the default output portable: the built-in palette for highlighting is ANSI-16 only, so no `38;5;`/`38;2;` sequence appears without a theme at any depth and no background color is ever emitted. Theme colors for the new roles go through the same depth clamp as every other role.
- Load the grammar set lazily, only when a document actually contains a fenced block with a recognized language, so documents without code pay nothing at startup.
- Reverse the "no syntect in `cargo tree`" stance of `render-markdown-to-terminal` (task 9.2). The cost is a larger binary (estimate: 2.2 MB → roughly 5 MB) and a one-time grammar load on first use (tens of milliseconds); the benefit is highlighting that matches what `bat` shows for the same file.

Out of scope (follow-up candidates): selecting whole tmTheme palettes (Nord, Dracula, …) for truecolor terminals, a `--highlight` opt-out flag (styling is already disabled by `--color never`, and a theme can set every `syntax_*` role to the `code_block` color), shebang-based detection for fences without a language, highlighting inline code spans, and surfacing the language label in the output.

## Capabilities

### New Capabilities

- `syntax-highlighting`: How a fenced code block's language is resolved from its info string (first word, alias table, grammar lookup), how tokens are classified into the syntax roles, what falls back to the `code_block` style, and that unrecognized languages and indented blocks are rendered as before.

### Modified Capabilities

- `block-rendering`: The requirement "Code blocks are verbatim and indented" changes from "in a distinct color when styling is on" to "in the `code_block` style, with tokens of a recognized language styled by their syntax role when styling is on"; the verbatim, no-wrap, four-space-indent, and no-fence guarantees are unchanged.
- `theming`: The requirement "Every visible element has a role" grows from 29 roles to 36 with the seven `syntax_*` roles, and the "omitted roles keep the built-in style" guarantee covers them.

## Impact

- **`Cargo.toml`**: new runtime dependency `two-face` (`default-features = false, features = ["syntect-fancy"]`), which brings `syntect` 5.3 with `parsing`, `dump-load`, and `regex-fancy` only — no `onig`/C build, no HTML module, no bundled tmThemes. Pure Rust, so `cargo install --path .` keeps working with just a Rust toolchain.
- **`src/render/highlight.rs`** (new): language resolution and alias table, lazy `SyntaxSet`, per-line tokenization, and the scope-to-role mapping that turns a code line into `Vec<Span>`.
- **`src/render/mod.rs`**: `code_block` receives the info string alongside the literal and emits the spans the highlighter returns; the `Writer` is untouched — it already handles multi-span lines, per-line style reset, and depth clamping.
- **`src/theme/mod.rs`**: `Theme` gains seven fields, `ROLES` grows to 36, and `slot`/`slot_ref`/`all_styles` cover them; `src/theme/file.rs` needs no change since it validates against `ROLES`.
- **Tests**: `tests/render.rs` gains highlighted-output tests (recognized language styles a keyword, unknown language is unchanged, styled-minus-escapes equals plain, no `38;5;`/`38;2;`/`48;` with the default theme); the theme role-count test moves from 29 to 36; `tests/cli.rs` exercises a theme that recolors a `syntax_*` role.
- **`README.md`**: roles table (`code_block` row and seven new rows), the block-rendering table's code row, and removal of the "No syntax highlighting (yet)" limitation; a short list of recognized language names.
- **`man/catmd.1`**: the "Code blocks are not wrapped or syntax-highlighted" sentence and the roles list under THEMES.
- **`examples/themes/solarized-dark.toml`**: entries for the seven new roles so the shipped example exercises them.
- **Licensing**: the embedded grammars carry their own licenses; `two-face` exposes them through its `acknowledgement` module. The change needs to decide between a committed `NOTICE`-style file and a flag that prints them (as `bat --acknowledgements` does).
- **Compatibility**: plain output (`--color never`, pipes, `NO_COLOR`) is byte-identical to today for every document; styled output is identical for documents without a recognized fenced language; existing theme files load unchanged; flags, environment variables, and exit codes are unchanged.
