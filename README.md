# catmd

`cat` for Markdown. `catmd` reads Markdown files (or standard input) and writes
them to the terminal with headings, emphasis, lists, quotes, code, and tables
rendered as styled text instead of raw syntax.

```
$ catmd README.md
```

- Detects whether your terminal supports color, how wide it is, and how tall it
  is — and lets you override all three.
- Streams like `cat`; only when the output is taller than the terminal does it
  hand off to your pager, the way `git` does.
- Parses CommonMark plus the GitHub extensions (tables, strikethrough, task
  lists, autolinks, footnotes, alerts) with [comrak](https://github.com/kivikakk/comrak).
- Highlights fenced code by the language on the fence — Rust, TOML,
  TypeScript, Swift, Dockerfile, shell, and some two hundred more — using the
  grammars curated by [`bat`](https://github.com/sharkdp/bat).
- Themeable: point `--theme` at a small TOML file to recolor anything, in
  16, 256, or 16 million colors — downgraded automatically to what the
  terminal can show.

## Installation

Requires a Rust toolchain.

```
cargo install --path .
```

### Manual page

The handwritten [catmd(1) manual](man/catmd.1) covers options, environment
variables, paging, themes, and exit statuses. Preview it from the repository root:

```sh
man ./man/catmd.1
```

`cargo install --path .` installs the executable only, not the manual page.
To install the manual separately without administrator privileges:

```sh
prefix="$HOME/.local"
mkdir -p "$prefix/share/man/man1"
cp man/catmd.1 "$prefix/share/man/man1/catmd.1"
```

If that directory is not already on your manual search path, add it for the
current shell, then open the page:

```sh
export MANPATH="$HOME/.local/share/man:${MANPATH:-}:"
man catmd
```

The trailing empty `MANPATH` component preserves the system's default manual
search paths. To keep the setting, add the export to your shell configuration.
You can choose another `prefix`; use its `share/man` directory in `MANPATH`.
For a system-wide installation, use an administrator-writable prefix such as
`/usr/local` and the appropriate privileges for the directory creation and copy.

## Usage

```
catmd [OPTIONS] [FILE]...
```

Each `FILE` is rendered in order as its own document, separated by a blank line.
With no `FILE`, or where a `FILE` is `-`, Markdown is read from standard input.

```
catmd README.md                 # one file
catmd intro.md chapter-*.md     # several files, in order
curl -s https://example.com/README.md | catmd
catmd header.md - footer.md < body.md
```

A file that cannot be read is reported on standard error, the remaining files
are still rendered, and the exit status is `1`. Usage errors exit with `2`.

### Options

| Option | Effect |
|--------|--------|
| `--color <auto\|always\|never>` | When to emit ANSI styling. `auto` (default) styles output only when standard output is a color-capable terminal. |
| `--color-depth <auto\|16\|256\|truecolor>` | How many colors to use. `auto` (default) reads `COLORTERM` and `TERM`. |
| `--theme <THEME>` | A theme file path, or a theme name looked up in `$XDG_CONFIG_HOME/catmd/themes/`. Defaults to `$CATMD_THEME`, else the built-in theme. See [Themes](#themes). |
| `--width <N>` | Wrap and draw rules to `N` columns instead of the detected width. Values below 20 are raised to 20. |
| `--height <N>` | Treat the terminal as `N` rows tall when deciding whether to page. |
| `--ascii` | Use `*`, `\|`, `-`, `+` instead of `•`, `│`, `─`, `┼` for bullets, gutters, rules, and table borders. |
| `--no-pager` | Never hand output to a pager. |
| `-h`, `--help`, `-V`, `--version` | The usual. |

### How the terminal is detected

**Color.** With `--color auto`, styling is on when standard output is a terminal
and `TERM` is not `dumb`. The usual environment conventions are honored:
`NO_COLOR` (any non-empty value) turns styling off, `CLICOLOR_FORCE` (non-empty,
not `0`) turns it on even into a pipe, and `CLICOLOR=0` turns it off.
`--color always` and `--color never` override everything. When styling is off
the output contains no escape sequences at all, and the text is identical to the
styled rendering with the escapes removed.

**Color depth.** With `--color-depth auto`, `COLORTERM=truecolor` or `24bit`
(or a `TERM` containing `direct`) means 16 million colors, a `TERM` containing
`256color` means 256, and anything else means the 16 standard ANSI colors. The
detection reads only environment variables, so it also applies to
`--color always | less -R`. Terminal multiplexers and SSH sometimes hide the
real depth; `--color-depth 16|256|truecolor` overrides it. Every color is
downgraded to the nearest one the resolved depth can show, and output never
contains a sequence beyond that depth. The built-in theme uses only the 16 ANSI
colors, so without a theme catmd looks the same everywhere.

**Width.** `--width`, else the terminal's column count, else `COLUMNS`, else 80.
Piped output therefore wraps at 80 columns unless you set `COLUMNS` or `--width`.

**Height.** `--height`, else the terminal's row count, else `LINES`, else
unknown. Height is used only to decide whether to page.

**Glyphs.** Unicode bullets and box-drawing glyphs are used when `LC_ALL`,
`LC_CTYPE`, or `LANG` (the first one set) names a UTF-8 encoding, or when none
of them is set. Any other locale, or `--ascii`, selects the ASCII set.

### Paging

When standard output is a terminal and the rendered output has more rows than
the terminal, `catmd` pipes it through a pager; otherwise it writes directly.
Paging never happens when output is piped or redirected.

The pager is the first non-empty of `CATMD_PAGER`, then `PAGER`, then `less`.
The value is a command line (`less -S` works). `PAGER=cat` or `--no-pager`
disables paging. If `LESS` is unset, the pager runs with `LESS=FRX` so `less`
shows colors, quits immediately when everything fits on one screen, and leaves
the text on screen when you quit. If the pager cannot be started, `catmd` says
so on standard error and prints the output directly.

Color is decided against your terminal *before* the pager starts, so colors
survive the trip through `less`.

## Themes

A theme is a TOML file that assigns a style to each rendering *role*. Pass it
with `--theme`, or set `CATMD_THEME` once in your shell.

```
catmd --theme examples/themes/solarized-dark.toml README.md
catmd --theme ~/my-theme.toml README.md           # any path works
catmd --theme solarized                           # ~/.config/catmd/themes/solarized.toml
CATMD_THEME=solarized catmd README.md             # default theme for every run
```

A value containing a `/` or ending in `.toml` is a file path; anything else is
a name resolved to `$XDG_CONFIG_HOME/catmd/themes/<name>.toml`
(`~/.config/catmd/themes/` when `XDG_CONFIG_HOME` is unset). `--theme` wins
over `CATMD_THEME`.

### Writing a theme

```toml
# Roles you leave out keep the built-in style.
code     = "yellow"                                   # a bare string is a foreground color
link_url = { modifiers = ["dim"] }                    # modifiers only, no color
heading1 = { fg = "accent", modifiers = ["bold", "underline"] }
strong   = "#ff8800"                                  # replaces bold with an orange foreground

[palette]                                             # optional named colors; keep this
accent = "#00aaff"                                    # table LAST: a TOML header applies
muted  = 245                                          # to every key after it
```

Each role takes either a **bare color** (`"red"`, `208`, `"#rrggbb"`, or a
palette name) or a **table** with an optional `fg` color and an optional
`modifiers` list drawn from `bold`, `dim`, `italic`, `underline`, and
`strikethrough`. An entry *replaces* the built-in style for that role entirely,
so `strong = "red"` makes strong text red and not bold; write
`strong = { fg = "red", modifiers = ["bold"] }` to keep the bold.

Colors can be:

| Form | Example | Notes |
|------|---------|-------|
| ANSI name | `"red"`, `"bright_blue"` | `black red green yellow blue magenta cyan white` and their `bright_` variants; never converted, so they follow the terminal's own palette |
| 256-color index | `208` | a TOML integer from 0 to 255 |
| Hex | `"#cb4b16"` | exactly six hex digits; downgraded on 256- and 16-color terminals |
| Palette name | `"accent"` | a key from `[palette]`; checked before ANSI names, so a palette may redefine `red` |

Roles, with the built-in defaults:

| Role | Styles | Default |
|------|--------|---------|
| `heading_marker` | the `#` markers | dim |
| `heading1` … `heading6` | heading text by level | bold; `heading1` bright magenta + underline, `heading2` magenta, `heading3` bright blue |
| `strong`, `emph`, `strikethrough` | inline emphasis | bold / italic / strikethrough |
| `code` | inline code spans | yellow |
| `code_block` | code block text that no syntax role claims; all of an unhighlighted block | cyan |
| `syntax_keyword` | keywords and declaration words (`fn`, `struct`, `if`) | magenta |
| `syntax_string` | string literals with their quotes | green |
| `syntax_comment` | comments with their markers | dim |
| `syntax_constant` | numbers, `true`/`false`/`null` | red |
| `syntax_type` | type names | yellow |
| `syntax_function` | function names | blue |
| `syntax_tag` | HTML/XML tag names, TOML/YAML/JSON keys | bright blue |
| `link_text`, `link_url` | link text / the `(url)` | blue + underline / dim |
| `image` | the `[image: alt]` placeholder | magenta + dim |
| `quote_gutter` | the `│` gutter of quotes and alerts | cyan + dim |
| `alert_note`, `alert_tip`, `alert_important`, `alert_warning`, `alert_caution` | alert title lines | bold blue / green / magenta / yellow / red |
| `table_header`, `table_border` | header cells / separators and header rule | bold / dim |
| `rule` | thematic breaks | dim |
| `html` | literal HTML | dim |
| `footnote_marker` | `[n]` references and definitions | cyan |
| `bullet`, `list_number`, `task_checkbox` | `•`, `1.`, `[x]` list markers | unstyled |

A theme that cannot be used — missing file, invalid TOML, an unknown role,
modifier, or color — stops catmd before any output with a message naming the
file and the problem, and exit status 2. Themes have no effect when styling is
off (`--color never`, piped output in `auto` mode, `NO_COLOR`).

`examples/themes/solarized-dark.toml` is a complete example.

## Syntax highlighting

A fenced block whose info string names a language is highlighted; everything
else in it keeps the `code_block` color.

````markdown
```rust
fn main() {}          // `fn` keyword, `main` function, the rest code_block
```

```rust,ignore        <- the language is the first word, cut at a comma
```

```text              <- text, plaintext, plain, txt, nohighlight: never highlighted
```
````

The language word is matched case-insensitively against grammar names and file
extensions (`rust`, `Rust`, `rs` all work), after a few aliases: `shell` and
`zsh` → Bash, `jsonc`/`json5` → JSON, `tf`/`hcl` → Terraform, `csharp` → C#,
`fsharp` → F#, `golang` → Go, `docker` → Dockerfile. Unknown languages,
indented code blocks, and `console` shell transcripts render in the plain
`code_block` style. (The Bash grammar would color a transcript's program output
as commands and lose the rest of the block to the first apostrophe, so
`console` is deliberately left alone.)

The grammars are the set curated by [`bat`](https://github.com/sharkdp/bat),
embedded through [`two-face`](https://codeberg.org/CosmicHarper/two-face):
roughly two hundred languages, including TOML, TypeScript/TSX, Swift,
Dockerfile, Kotlin, Terraform, Nix, Zig, Protobuf, and Fish on top of the
usual C-family, scripting, and markup languages. The pure-Rust build omits six
grammars that need Oniguruma-only regex features — PowerShell, Sass,
LiveScript, ARM assembly, Babel JavaScript, and Salt State — so those blocks
render unhighlighted. Their licenses are listed in
[`THIRD_PARTY_LICENSES.md`](THIRD_PARTY_LICENSES.md).

Highlighting is styling, so it is off whenever styling is off (`--color never`,
piped output in `auto` mode, `NO_COLOR`). To keep color but flatten code blocks,
set the seven `syntax_*` roles to the `code_block` color in a theme. The
grammar set is loaded only when a document actually contains a highlightable
block; on the development machine that first block costs about 14 ms, and the
grammars account for about 3.3 MB of the 5.2 MB release binary.

## What it renders

| Markdown | Terminal |
|----------|----------|
| `# Heading` | `#` markers kept (dimmed), text bold; level 1 also underlined |
| `**strong**`, `*emphasis*`, `~~strike~~` | bold, italic, strikethrough |
| `` `code` `` | colored, literal |
| `[text](url)` | `text (url)` with the text underlined; bare URLs shown once |
| `![alt](src)` | `[image: alt] (src)` |
| paragraphs | re-flowed and wrapped at word boundaries; words are never split |
| `- item`, `1. item`, `- [x] task` | bullets, right-aligned numbers, `[x]`/`[ ]` checkboxes, nested indentation |
| `> quote` | gutter on every line, nested gutters stack |
| `> [!NOTE]` … | GitHub alerts as quotes with a colored title line |
| fenced / indented code | indented four spaces, verbatim, never wrapped; fenced blocks highlighted by the language on the fence |
| tables | columns padded to their widest cell, alignment honored, rule under the header |
| `---` | a rule across the full width |
| `[^1]` footnotes | `[1]` markers; definitions listed after a rule at the end |
| `<html>` | shown literally, dimmed |
| YAML front matter | hidden |

## Limitations

- Tables wider than the terminal are not squeezed; they overflow.
- Italic and strikethrough depend on the terminal; some terminals ignore them.
- Themes set foreground colors and text attributes only — no background
  colors, no glyph changes (use `--ascii`), and no inheritance between theme
  files.
- Links are shown as text plus URL, not as clickable OSC-8 hyperlinks.
- Images are shown as a placeholder, never rendered.

## Development

```
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

Rendering is deterministic for a given width, color mode, and glyph set, so the
tests in `tests/render.rs` compare exact output; `tests/cli.rs` exercises the
binary through pipes.
