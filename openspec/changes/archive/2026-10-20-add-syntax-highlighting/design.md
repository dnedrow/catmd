# Design

## Context

See `proposal.md` — Why. Relevant current state:

- `Renderer::code_block(&mut self, literal: &str)` (`src/render/mod.rs`) pushes a uniform four-space prefix and emits one `Span` per line with `theme.code_block` through `Writer::raw_line`. comrak's `NodeCodeBlock` carries `info` (the fence's info string) and `fenced`, and the renderer passes only `literal`.
- `Writer::render_line` (`src/render/layout.rs`) is the single escape-producing site. It drops empty spans, coalesces adjacent spans with equal styles, clamps each foreground to the resolved `ColorDepth`, and opens and resets the style around every span, so a line of many differently styled tokens is already handled correctly and no style ever crosses a line break.
- `Theme` (`src/theme/mod.rs`) is a struct of 29 roles; `Theme::ROLES`, `slot`, `slot_ref`, and `all_styles` enumerate them, and `theme/file.rs` validates a theme file's keys against `ROLES`, so adding a role is a change to `theme/mod.rs` alone. Theme semantics are "an entry replaces the role's built-in style entirely" — roles do not inherit from one another.
- `render(markdown, &Settings)` is a pure function; `Settings` holds `depth`, `theme`, `width`, and `glyphs`. Nothing in `Settings` needs to change for highlighting.
- The current dependency tree has 49 crates and contains no regex engine, no `serde` (only `serde_core`), and no compression crate. `syntect` 5.3 is in the local registry cache because comrak declares it as an optional dependency; it is not compiled today.
- The `color-output` spec requires that the built-in theme emits no `38;5;`/`38;2;` at any depth and that no background color is ever emitted; `block-rendering` requires code blocks to be verbatim, unwrapped, and indented four spaces.
- `syntect` exposes what the mapping needs without its theme machinery: `SyntaxSet::find_syntax_by_token` (extension, then case-insensitive name), `ParseState::parse_line` producing `(offset, ScopeStackOp)` pairs, `easy::ScopeRegionIterator` turning those into `(text, op)` regions, `ScopeStack::apply`, and `highlighting::ScopeSelectors::does_match(&[Scope]) -> Option<MatchPower>` with tmTheme selector syntax including `-` exclusions. With `regex-fancy`, a regex error (for example the backtracking limit) is treated as a non-match rather than a panic or a hang.
- `two-face` 0.5 (`bat` 0.26 assets) re-exports `syntect`, provides `syntax::extra_newlines() -> SyntaxSet`, and `acknowledgement::listing().for_syntaxes() -> &[License]` for the grammars' licenses. Its `syntect-fancy` feature enables only `syntect/regex-fancy` on top of `parsing` and `dump-load`.

## Goals / Non-Goals

**Goals:**

- Highlighting is a pure function from `(language, line)` to `Vec<Span>`; the `Writer` and every layout rule stay untouched, so the verbatim/indent/no-wrap guarantees are structural rather than re-tested.
- Default output stays ANSI-16 and background-free; richer colors appear only through a theme, exactly as for every other role.
- Zero cost for documents without a recognized fenced language: no grammar deserialization, no regex compilation.
- A highlighting failure (unknown language, grammar error, pathological line) degrades to today's rendering, never to an error or missing text.
- The dependency build stays pure Rust so `cargo install --path .` needs only a Rust toolchain.

**Non-Goals:**

- Pixel-parity with `bat`. We reuse its grammars, not its themes; the 7-role ANSI palette is deliberately coarser than a tmTheme.
- Highlighting inline code spans, indented code blocks, or HTML blocks.
- Detecting a language from content (shebangs, mode lines) when the fence has no info string.
- Exposing the grammar set to users (adding `.sublime-syntax` files from a config directory). Follow-up material.

## Decisions

### D1. Grammars come from `two-face` with the `fancy-regex` build of `syntect`

```toml
two-face = { version = "0.5", default-features = false, features = ["syntect-fancy"] }
```

`syntect` is used through `two_face::re_exports::syntect` rather than as a direct dependency, so the two cannot drift apart. The `syntect-fancy` feature enables `parsing`, `dump-load`, and `regex-fancy` only — no `onig_sys` C build, no `html`, no `plist-load`/`yaml-load`, no bundled tmThemes.

- *Alternative: `syntect` alone with its default grammar set.* Rejected — lacks TOML, TypeScript, Swift, Dockerfile, Kotlin, Terraform, Nix, Zig; the user's documents need the first three. two-face adds them for ~0.6 MiB.
- *Alternative: `syntect-onig`.* Rejected — pulls a C library build into `cargo install`, which the README promises needs only a Rust toolchain. The cost of `fancy` is six grammars (PowerShell, Sass, LiveScript, ARM assembly, Babel JavaScript, Salt State), which fall back to the unhighlighted rendering.
- *Alternative: tree-sitter.* Rejected — one C parser crate per language and tens of megabytes for comparable coverage; accuracy beyond what a `cat` replacement needs.
- *Alternative: hand-written lexers.* Rejected — each language is a maintenance item and quality degrades on exactly the constructs that matter (heredocs, raw strings, nested templates).
- *Alternative: shell out to `bat`/`pygmentize` when installed.* Rejected — a process spawn per block, output that differs between machines, and no way to honor catmd's depth clamp or the "plain equals styled minus escapes" invariant.

### D2. Scopes map to catmd roles; tmThemes are not used

The highlighter never constructs a `syntect::highlighting::Theme`. It walks each line's scope regions and classifies the scope stack against a fixed ordered table of `(ScopeSelectors, Role)`; the entry with the highest `MatchPower` wins, and no match means the `code_block` role. The role is then read from the active `Theme` like any other role, so TOML theme files recolor highlighting with no new syntax, and `Writer::render_line` clamps the color to depth as it does for everything else.

- *Alternative: use `HighlightLines` with two-face's embedded `Ansi` theme (which encodes ANSI indices in the alpha channel, a `bat` convention).* Rejected for now — it would introduce a second theming vocabulary (tmTheme scopes and colors) beside the role one, its palette is `bat`'s rather than catmd's defaults, and it would still need the decoding hack. Keeping roles as the only model leaves the door open to a later `syntax_theme = "nord"` key whose fallback is exactly this design.
- *Alternative: a bundled RGB tmTheme as the default.* Rejected — violates the "built-in theme is portable at every depth" requirement and ignores light/dark terminal palettes.

### D3. Role vocabulary and initial selector table

Seven roles, chosen to cover what ANSI color schemes conventionally distinguish and nothing finer:

| Role              | Selectors                                                                                                   | Default         |
|-------------------|-------------------------------------------------------------------------------------------------------------|-----------------|
| `syntax_keyword`  | `keyword - keyword.operator`, `storage`                                                                     | magenta         |
| `syntax_string`   | `string`                                                                                                    | green           |
| `syntax_comment`  | `comment`                                                                                                   | dim             |
| `syntax_constant` | `constant`, `keyword.constant`                                                                              | red             |
| `syntax_type`     | `entity.name.type`, `entity.name.class`, `entity.name.struct`, `entity.name.enum`, `entity.name.trait`, `entity.name.interface`, `entity.name.impl`, `entity.other.inherited-class`, `support.type`, `support.class` | yellow |
| `syntax_function` | `entity.name.function`, `entity.type.function`, `support.function`, `variable.function`                     | blue            |
| `syntax_tag`      | `entity.name.tag`, `entity.name.table`, `meta.mapping.key string`                                           | bright blue     |

Consequences of "deepest match wins": string delimiters (`punctuation.definition.string` under `string`) take the string color, comment markers take the comment style, and escape sequences inside strings (`constant.character.escape`) take the constant color — the conventional look. Operators are excluded from `keyword` so `=`, `+`, and `->` stay in the `code_block` style. Punctuation, variables, operators, attributes, and anything else unmatched keep `code_block`, so a themer still has one knob for "the text of a code block". The table above is the tuned result of running the grammars over Rust, TOML, TypeScript, Swift, Bash, YAML, JSON, Go, Python, HTML, and Dockerfile snippets: `storage.type` stays a keyword because every grammar uses it for declaration words and scopes type names as `support.type`/`entity.name.*`; `keyword.constant` (Swift `true`) is a constant; `entity.type.function` is a Swift grammar quirk for function names; `entity.name.table` colors TOML `[section]` headers like keys; and `meta.mapping.key string` makes JSON keys differ from JSON string values.

- *Alternative: unmatched tokens in a highlighted block use the terminal default color instead of `code_block`.* Rejected — it would make `code_block` mean two different things depending on whether a language was recognized, and the user chose the single-knob behavior. Anyone who prefers plain identifiers sets `code_block = { }` in a theme.
- *Alternative: more roles (`syntax_attribute`, `syntax_variable`, `syntax_punctuation`, `syntax_operator`).* Deferred — roles are additive, and seven already exceeds what most 16-color schemes distinguish.

### D4. Language resolution from the info string

```
info string ──> first whitespace-delimited word ──> trim `{`/`}` ──> ASCII-lowercase
   ──> alias table ──> find_syntax_by_token ──> Some(syntax) | None (render as today)
```

- The first word rule follows CommonMark's description of the info string and covers rustdoc's ```` ```rust,ignore ```` (the word is split on `,` as well) and attribute-style ```` ```js title="x" ````.
- Alias table (small, in code): `shell`, `zsh` → `bash`; `jsonc`, `json5` → `json`; `tf`, `hcl` → `terraform`; `csharp`, `c#` → `cs`; `fsharp`, `f#` → `fs`; `golang` → `go`; `docker` → `dockerfile`; `yml` is already an extension; `text`, `plaintext`, `plain`, `txt`, `nohighlight` → explicitly none, so a document can opt a block out. `console` and `shell-session` are deliberately not aliased: running the Bash grammar over a session transcript colors program output as commands and an apostrophe in the output (`Don't panic`) opens a string that swallows the rest of the block, so those blocks render in the plain code style.
- `find_syntax_by_token` then matches by file extension (`rs`, `ts`, `tsx`, `toml`, `swift`, `kt`, `py`, `rb`) or case-insensitive grammar name (`Rust`, `Dockerfile`, `TypeScriptReact`).
- Indented code blocks (`fenced == false`) and fences with an empty info string are never highlighted.

- *Alternative: `find_syntax_by_first_line` for bare fences.* Deferred to a follow-up — it is three lines, but it makes highlighting depend on content rather than markup and deserves its own scenario.

### D5. Highlighting pipeline and emission

```
  code_block(info, literal)
      |
      v
  highlight::language(info) ---------- None ----------> one Span per line, code_block style (today)
      | Some(lang)
      v
  SYNTAXES.get_or_init(two_face::syntax::extra_newlines)
      |
      v
  ParseState::new(syntax)
  for line in literal.split_inclusive('\n'):
      ops  = state.parse_line(line, set)        -- Err => stop highlighting, rest as today
      for (text, op) in ScopeRegionIterator::new(&ops, line):
          stack.apply(op)                        -- Err => same fallback
          role = classify(stack.as_slice())      -- D3 table, else code_block
          push Span::new(text, theme.role)
      strip the trailing "\n" (and "\r") from the last span
      writer.raw_line(&spans)
```

- The "newlines" grammar set is used, so each line is fed with its `\n`; the newline is removed from the final span before emission because `Writer` adds line breaks itself. `raw_line` keeps treating whitespace-only lines as empty, so indentation never leaves trailing spaces.
- Highlighting lives in `src/render/highlight.rs` and exposes two functions: `language(info: &str) -> Option<Language>` and `lines(lang, literal, theme) -> Option<Vec<Vec<Span>>>` (or an iterator); `render/mod.rs` falls back to the existing loop when either returns `None`. The `Writer` remains unaware of highlighting.
- `coalesce` in `render_line` merges adjacent tokens that resolved to the same style, so an identifier followed by punctuation (both `code_block`) costs one escape pair, not two.
- Text is passed through unchanged, so the `color-output` requirement that stripping escapes from styled output yields the plain output holds by construction; a test still pins it on a highlighted document.

### D6. The grammar set is a lazily initialized process-wide value

`static SYNTAXES: OnceLock<SyntaxSet>` in `highlight.rs`, initialized on the first block that resolves to a language. Deserializing two-face's dump costs tens of milliseconds once; documents without a highlightable block never pay it, and `render()` stays a pure function of its arguments because the set is immutable data, not configuration.

- *Alternative: load eagerly in `main` or store the set in `Settings`.* Rejected — every `catmd README.md` of a README without code would pay the load, and `Settings` would carry a multi-megabyte value through code that never uses it.

### D7. `Theme` gains the seven roles, nothing else changes in theming

`Theme` gets seven `Style` fields with the defaults from D3; `ROLES` goes to 36 and `slot`/`slot_ref`/`all_styles` are extended. `theme/file.rs` validates against `ROLES`, so theme files gain the new keys automatically, and the existing "omitted roles keep the built-in style" rule covers old theme files without a migration. The `syntax_*` roles do not inherit from `code_block`; a theme that recolors `code_block` and wants matching highlights sets the syntax roles too (the shipped `solarized-dark.toml` shows this).

### D8. No new flag or environment variable

Highlighting is a styling concern, so `--color never`, `NO_COLOR`, `CLICOLOR=0`, pipes, and `TERM=dumb` already disable it. A user who wants code blocks flat on a color terminal sets the seven roles to the `code_block` color in a theme. A `--highlight <auto|never>` flag is cheap to add later if that proves too clumsy.

### D9. Grammar licenses live in a committed `THIRD_PARTY_LICENSES.md`

Generated from `two_face::acknowledgement::listing().for_syntaxes()` and committed at the repository root. A unit test regenerates the text and compares it with the committed file, so a `two-face` bump that changes the grammar set fails the build until the file is refreshed (the test's failure message says how). No CLI surface, no spec, no man page entry.

- *Alternative: `--acknowledgements` at runtime (as `bat`).* Rejected by the user for now — catmd is installed from source, so the repository is where a reader looks; adding a flag would touch the CLI spec and the manual for a rarely used path.

## Risks / Trade-offs

- [Binary grows from 2.2 MB to an estimated ~5 MB, dependency tree from 49 to roughly 65–75 crates] → Accepted knowingly; measured in the first implementation task so the README can state the real number. The `syntect-fancy` feature keeps out `html`, `plist`, `yaml`, and tmThemes.
- [First highlighted block pays a grammar-set deserialization of tens of milliseconds] → Lazy `OnceLock` (D6); measured in the first task; if it exceeds ~50 ms on the development machine, evaluate trimming the set with `SyntaxSetBuilder` in a follow-up rather than in this change.
- [Pathological lines (minified JavaScript, giant JSON) can be slow under a backtracking regex engine] → `fancy-regex`'s backtracking limit turns runaway matches into non-matches, bounding the cost; the `ParsingError` fallback (D5) renders the rest of the block unhighlighted rather than failing. No line-length cutoff initially; add one if a fixture shows the need.
- [The selector table misclassifies tokens in some grammar (for example a grammar that scopes type names as `storage.type`)] → Table is tuned against fixture snippets for the languages the user named; misclassification degrades to a wrong color, never to wrong text. Roles are additive, so a later change can split a role without breaking themes.
- [Default `code_block` is cyan, so a highlighted block mixes cyan identifiers with the new colors] → Chosen deliberately to keep one knob and unchanged rendering for unknown languages; a theme can set `code_block = { }`. If the default looks busy in practice, changing the `code_block` default is a one-line follow-up with its own scenario.
- [Six grammars excluded by `fancy-regex`, PowerShell being the visible one] → Documented in the README's language note; those blocks render as today.
- [`two-face` tracks `bat` releases; a bump may add or remove grammars] → `THIRD_PARTY_LICENSES.md` sync test (D9) and the fixture tests catch changes; the alias table and selector table are ours and unaffected.

## Migration Plan

No data or configuration migrates. Deployment is a normal release:

1. Land the change; plain output is byte-identical for every document and styled output is identical for documents without a recognized fenced language, so no existing invocation changes meaning.
2. Existing theme files load unchanged; theme authors add `syntax_*` keys at their leisure.
3. Rollback is reverting the change — there is no persisted state, no new flag, and no new environment variable to retire.

## Open Questions

None remaining. The `console` alias and the `storage.type` placement were both settled against fixture output during implementation (see D3 and D4).
