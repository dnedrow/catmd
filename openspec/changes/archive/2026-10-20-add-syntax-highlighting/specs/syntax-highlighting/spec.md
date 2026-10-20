# Spec Delta

## Purpose

Colors the tokens of fenced code blocks by their syntactic role — keywords, strings, comments, constants, types, functions, and tags — according to the language named on the fence, so code reads in the terminal the way it does in an editor while the text itself stays untouched.

## ADDED Requirements

### Requirement: The language is the first word of the info string
The system SHALL determine a fenced code block's language from the first whitespace-delimited word of its info string, cut at the first `,` and stripped of surrounding `{` and `}`, and SHALL match that word case-insensitively against grammar names and file extensions.

#### Scenario: Attribute after the language
- **WHEN** a document contains "```rust,ignore\nfn main() {}\n```" rendered with `--color always`
- **THEN** `fn` is emitted in the `syntax_keyword` style

#### Scenario: Title after the language
- **WHEN** a document contains "```js title=\"demo.js\"\nconst x = 1;\n```" rendered with `--color always`
- **THEN** `const` is emitted in the `syntax_keyword` style

#### Scenario: Mixed-case language name
- **WHEN** a document contains "```Rust\nfn main() {}\n```" rendered with `--color always`
- **THEN** `fn` is emitted in the `syntax_keyword` style

#### Scenario: File extension as the language
- **WHEN** a document contains "```rs\nfn main() {}\n```" rendered with `--color always`
- **THEN** `fn` is emitted in the `syntax_keyword` style

### Requirement: Common languages are recognized
The system SHALL recognize at least the languages commonly fenced in project documentation, including those absent from basic grammar sets such as TOML, TypeScript, Swift, Dockerfile, and Kotlin.

#### Scenario: Language tokens that resolve
- **WHEN** a fenced block names any of `bash`, `sh`, `c`, `cpp`, `csharp`, `css`, `dockerfile`, `go`, `html`, `java`, `javascript`, `js`, `json`, `kotlin`, `markdown`, `python`, `py`, `ruby`, `rust`, `sql`, `swift`, `toml`, `typescript`, `ts`, `tsx`, `xml`, or `yaml`
- **THEN** the block is highlighted

### Requirement: Common language aliases are recognized
The system SHALL treat at least these info-string words as aliases: `shell` and `zsh` for Bash, `jsonc` and `json5` for JSON, and `tf` and `hcl` for Terraform.

#### Scenario: Shell alias
- **WHEN** a document contains "```shell\necho \"hi\"\n```" rendered with `--color always`
- **THEN** `"hi"` is emitted in the `syntax_string` style

#### Scenario: JSON with comments alias
- **WHEN** a document contains "```jsonc\n{\"a\": \"b\"}\n```" rendered with `--color always`
- **THEN** `"b"` is emitted in the `syntax_string` style

### Requirement: Plain-text markers disable highlighting for a block
The system SHALL NOT highlight a fenced block whose language word is `text`, `plaintext`, `plain`, `txt`, or `nohighlight`, even if the content looks like code.

#### Scenario: Code marked as text
- **WHEN** a document contains "```text\nfn main() {}\n```" rendered with `--color always`
- **THEN** the content line is emitted entirely in the `code_block` style with no other style

### Requirement: Blocks without a recognized language render as before
The system SHALL render a fenced block whose language is absent or unrecognized, and every indented code block, with each content line entirely in the `code_block` style, byte-identical to the rendering of the same content with no info string.

#### Scenario: Unknown language
- **WHEN** a document contains "```foobarlang\nfn main() {}\n```" rendered with `--color always`
- **THEN** the output is identical to rendering "```\nfn main() {}\n```" with `--color always`

#### Scenario: Fence without an info string
- **WHEN** a document contains "```\nfn main() {}\n```" rendered with `--color always`
- **THEN** the content line is emitted entirely in the `code_block` style with no other style

#### Scenario: Indented code block
- **WHEN** a document contains `    fn main() {}` as an indented code block rendered with `--color always`
- **THEN** the content line is emitted entirely in the `code_block` style with no other style

### Requirement: Tokens of a recognized language are styled by syntax role
When styling is on, the system SHALL emit each token of a recognized language in the style of its role — `syntax_keyword` for keywords and declaration words, `syntax_string` for string literals including their delimiters, `syntax_comment` for comments including their markers, `syntax_constant` for numeric and language constants, `syntax_type` for type names, `syntax_function` for function names, `syntax_tag` for markup tag names — and all other text in the `code_block` style.

#### Scenario: Keyword, function name, and punctuation
- **WHEN** a document contains "```rust\nfn main() {}\n```" rendered with `--color always`
- **THEN** `fn` is emitted in the `syntax_keyword` style, `main` in the `syntax_function` style, and `()` and `{}` in the `code_block` style

#### Scenario: Constant and comment
- **WHEN** a document contains "```rust\nlet x = 1; // one\n```" rendered with `--color always`
- **THEN** `1` is emitted in the `syntax_constant` style, `// one` in the `syntax_comment` style, and `=` in the `code_block` style

#### Scenario: String with delimiters
- **WHEN** a document contains "```python\nprint(\"hi\")\n```" rendered with `--color always`
- **THEN** `"hi"` including both quotes is emitted in the `syntax_string` style

#### Scenario: Markup tag
- **WHEN** a document contains "```html\n<p>x</p>\n```" rendered with `--color always`
- **THEN** both occurrences of `p` are emitted in the `syntax_tag` style

#### Scenario: Highlighted block inside a list item
- **WHEN** a document contains "- item\n\n  ```rust\n  fn main() {}\n  ```" rendered with `--color always` in ASCII mode
- **THEN** the output contains a line consisting of two spaces, four spaces, then `fn main() {}` with `fn` in the `syntax_keyword` style

### Requirement: Highlighting changes only styles
The system SHALL leave the text, line breaks, indentation, and absence of wrapping of a highlighted code block exactly as they are without highlighting, so that stripping escape sequences from the styled rendering yields the plain rendering.

#### Scenario: Styled and plain renderings agree
- **WHEN** a document containing a Rust, a TOML, and a Bash fenced block is rendered once with `--color always` and once with `--color never` at the same width
- **THEN** stripping all escape sequences from the styled output yields output identical to the plain output

#### Scenario: Long highlighted line is not wrapped
- **WHEN** a Rust fenced block contains a 120-character line and the width is 40 with `--color always`
- **THEN** the text of that line appears on one output line

#### Scenario: Styling off
- **WHEN** a document contains "```rust\nfn main() {}\n```" rendered with `--color never`
- **THEN** the output is exactly `    fn main() {}\n`

### Requirement: Syntax roles have portable built-in styles
With no theme, the system SHALL style `syntax_keyword` magenta, `syntax_string` green, `syntax_comment` dim, `syntax_constant` red, `syntax_type` yellow, `syntax_function` blue, and `syntax_tag` bright blue, using only the 16 ANSI colors and text attributes.

#### Scenario: Default keyword color
- **WHEN** a document contains "```rust\nfn main() {}\n```" rendered with `--color always` and no theme at 16 colors
- **THEN** `fn` is emitted with a magenta foreground sequence (`35`)

#### Scenario: Default comment style
- **WHEN** a document contains "```rust\n// note\n```" rendered with `--color always` and no theme
- **THEN** `// note` is emitted with the dim attribute (`2`) and no foreground color sequence

#### Scenario: Defaults stay portable at full depth
- **WHEN** a document containing Rust, TOML, Bash, and HTML fenced blocks is rendered with `--color always` and no theme at a resolved depth of 16 million colors
- **THEN** the output contains no `38;5;`, `38;2;`, or `48;` sequences

### Requirement: Analysis failures degrade to the plain code style
If a line of a recognized language cannot be analyzed, the system SHALL emit that line and the rest of the block in the `code_block` style, with the text unchanged, and SHALL still render the remainder of the document and exit successfully.

#### Scenario: Grammar gives up mid-block
- **WHEN** analysis of a code block fails on its second line
- **THEN** the first line is highlighted, the second and later lines are emitted in the `code_block` style, every line's text is unchanged, and the blocks after it are rendered normally
