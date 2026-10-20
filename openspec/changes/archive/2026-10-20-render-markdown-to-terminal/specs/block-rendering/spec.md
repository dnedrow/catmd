# Spec Delta

## Purpose

Lays out block-level Markdown constructs within the available width so documents read naturally in a terminal, with glyphs that match what the terminal can display.

## ADDED Requirements

### Requirement: Headings keep their level markers and are emphasized
The system SHALL render a heading as its `#` markers (one per level) followed by a space and the heading text in bold, with the markers dimmed when styling is on, and SHALL render level-1 heading text underlined when styling is on.

#### Scenario: ATX heading in plain mode
- **WHEN** a document contains `## Usage` and styling is off
- **THEN** the output contains the line `## Usage`

#### Scenario: Setext heading is normalized
- **WHEN** a document contains `Title\n=====`
- **THEN** the output contains the line `# Title` and does not contain `=====`

#### Scenario: Level-1 heading styled
- **WHEN** a document contains `# Title` and styling is on
- **THEN** `Title` carries the bold and underline attributes and `#` carries the dim attribute

### Requirement: Paragraphs are re-flowed to the width
The system SHALL join a paragraph's source lines and wrap the text at word boundaries so no line exceeds the effective width.

#### Scenario: Long paragraph wraps
- **WHEN** a paragraph of 200 characters of ordinary words is rendered at width 40
- **THEN** every output line is at most 40 columns and no word is split across lines

#### Scenario: Short paragraph is not wrapped
- **WHEN** a paragraph `short text` is rendered at width 80
- **THEN** the output contains `short text` on a single line

### Requirement: Wrapping measures display columns
The system SHALL measure line length in terminal display columns, counting East Asian wide characters as two columns and combining characters as zero.

#### Scenario: Wide characters
- **WHEN** a paragraph of 30 CJK ideographs (60 columns) is rendered at width 40
- **THEN** the first output line contains at most 20 ideographs

### Requirement: Words are never broken mid-word
The system SHALL NOT split a space-delimited word to fit the width, except that a break is permitted on either side of an East Asian wide character; a word longer than the width SHALL be placed on its own line and allowed to exceed the width.

#### Scenario: Long URL
- **WHEN** a paragraph contains `see https://example.com/a/very/long/path/that/exceeds/the/width` rendered at width 30
- **THEN** the URL appears unbroken on one line by itself

### Requirement: Blocks are separated by single blank lines
The system SHALL separate consecutive top-level blocks with exactly one blank line, SHALL NOT emit a leading blank line, and SHALL end the output with exactly one newline.

#### Scenario: Heading followed by paragraph
- **WHEN** a document is `# Title\n\nBody text.\n`
- **THEN** the output is exactly `# Title\n\nBody text.\n` when styling is off

### Requirement: Lines have no trailing whitespace
Outside code blocks, the system SHALL NOT emit lines that end with space characters.

#### Scenario: Wrapped paragraph
- **WHEN** any paragraph is wrapped
- **THEN** no output line of the paragraph ends with a space

### Requirement: Bullet lists use a bullet glyph with hanging indent
The system SHALL render each bullet item as the bullet glyph, a space, and the item content, with continuation lines indented to the content column.

#### Scenario: Wrapped bullet item in ASCII mode
- **WHEN** a bullet item with 100 characters of text is rendered at width 40 in ASCII mode
- **THEN** the first line starts with `* ` and every continuation line starts with two spaces

#### Scenario: Unicode bullet
- **WHEN** a bullet item `- apple` is rendered in Unicode mode
- **THEN** the output contains the line `• apple`

### Requirement: Ordered lists show aligned numbers
The system SHALL render ordered items as their number, a period, and a space, starting from the list's declared start number, with markers right-aligned to the widest marker in the list and continuation lines aligned to the content column.

#### Scenario: Start number honored
- **WHEN** a list is written as `3. c\n4. d`
- **THEN** the output contains the lines `3. c` and `4. d`

#### Scenario: Markers aligned across a width change
- **WHEN** an ordered list has ten items
- **THEN** item nine is rendered as ` 9. ` followed by its text and item ten as `10. ` followed by its text

### Requirement: Nested lists are indented under their parent item
The system SHALL indent a nested list to the content column of the item that contains it.

#### Scenario: Two-level bullet list
- **WHEN** a document contains `- parent\n  - child` rendered in ASCII mode
- **THEN** the output contains the line `* parent` followed by the line `  * child`

### Requirement: Loose and tight list spacing is preserved
The system SHALL render tight lists with no blank lines between items and loose lists with one blank line between items.

#### Scenario: Tight list
- **WHEN** a list is written as `- a\n- b`
- **THEN** the lines for `a` and `b` are consecutive with no blank line between them

#### Scenario: Loose list
- **WHEN** a list is written as `- a\n\n- b`
- **THEN** exactly one blank line separates the lines for `a` and `b`

### Requirement: Task items show a checkbox in place of the bullet
The system SHALL render a task item as `[x]` when checked or `[ ]` when unchecked, followed by a space and the item content.

#### Scenario: Mixed task list
- **WHEN** a document contains `- [x] done\n- [ ] todo`
- **THEN** the output contains the lines `[x] done` and `[ ] todo`

### Requirement: Block content inside list items aligns to the content column
The system SHALL indent paragraphs, code blocks, and quotes nested inside a list item to that item's content column.

#### Scenario: Second paragraph in an item
- **WHEN** a document contains `- first\n\n  second` rendered in ASCII mode
- **THEN** the output contains the line `* first`, a blank line, and the line `  second`

### Requirement: Block quotes have a gutter
The system SHALL prefix every line of a block quote with the quote glyph and a space, wrap the quoted content within the remaining width, and stack gutters for nested quotes.

#### Scenario: Quote in ASCII mode
- **WHEN** a document contains `> quoted` rendered in ASCII mode
- **THEN** the output contains the line `| quoted`

#### Scenario: Nested quote
- **WHEN** a document contains `> > deep` rendered in ASCII mode
- **THEN** the output contains the line `| | deep`

### Requirement: Alerts render as titled quotes
The system SHALL render a GitHub alert (`> [!NOTE]`, `[!TIP]`, `[!IMPORTANT]`, `[!WARNING]`, `[!CAUTION]`) as a block quote whose first line is the alert title (`Note`, `Tip`, `Important`, `Warning`, `Caution`, or the custom title if given) in bold, colored by type when styling is on.

#### Scenario: Warning alert in plain ASCII mode
- **WHEN** a document contains `> [!WARNING]\n> Mind the gap.` rendered in ASCII mode with styling off
- **THEN** the output contains the line `| Warning` followed by the line `| Mind the gap.`

#### Scenario: Custom title
- **WHEN** a document contains `> [!NOTE] Heads up\n> text`
- **THEN** the first line of the alert ends with `Heads up` and does not contain `Note`

### Requirement: Code blocks are verbatim and indented
The system SHALL render fenced and indented code blocks with each content line preceded by four spaces, preserving content exactly without wrapping, in a distinct color when styling is on, and SHALL NOT render the fence or info string.

#### Scenario: Fenced block with language
- **WHEN** a document contains "```rust\nfn main() {}\n```" and styling is off
- **THEN** the output contains the line `    fn main() {}` and does not contain `rust` or backticks

#### Scenario: Long code line is not wrapped
- **WHEN** a code block contains a 120-character line and the width is 40
- **THEN** that line appears in the output unbroken

#### Scenario: Blank lines inside code are preserved
- **WHEN** a code block contains `a\n\nb`
- **THEN** the output contains `    a`, an empty line, and `    b` consecutively

### Requirement: Tables render with aligned columns and a header rule
The system SHALL render a table with each column padded to its widest cell (omitting trailing padding on the last column), cells joined by ` <vertical> ` (space, glyph, space), a rule row of horizontal glyphs joined by `<horizontal><cross><horizontal>` after the header, header cells in bold when styling is on, and no outer border.

#### Scenario: Simple table in ASCII mode
- **WHEN** a document contains `| Name | Qty |\n|------|-----|\n| Apple | 3 |` rendered in ASCII mode with styling off
- **THEN** the output contains the lines `Name  | Qty`, `------+----`, and `Apple | 3`

### Requirement: Table cell alignment is honored
The system SHALL left-align, center, or right-align cell text within its column according to the table's delimiter row.

#### Scenario: Right-aligned column
- **WHEN** a document contains `| Qty |\n|----:|\n| 3 |\n| 100 |` rendered with styling off
- **THEN** the output contains the lines `Qty`, `---`, `  3`, and `100`

### Requirement: Wide tables are not squeezed
The system SHALL render a table whose natural width exceeds the effective width at its natural width without truncating or wrapping cells.

#### Scenario: Table wider than width
- **WHEN** a table's natural width is 60 columns and the width is 40
- **THEN** every table line is 60 columns wide and all cell text is present

### Requirement: Thematic breaks span the width
The system SHALL render a thematic break as a line of the rule glyph exactly as wide as the effective width.

#### Scenario: Rule in ASCII mode
- **WHEN** a document contains `---` rendered at width 30 in ASCII mode
- **THEN** the output contains a line of exactly 30 `-` characters

### Requirement: Footnote definitions are listed at the end
The system SHALL render footnote definitions after all other content, preceded by a thematic-break rule, each as `[n]`, a space, and the definition text, numbered in order of first reference.

#### Scenario: One footnote
- **WHEN** a document contains `text[^a]\n\n[^a]: the note` rendered at width 20 in ASCII mode with styling off
- **THEN** the output ends with a line of 20 `-` characters, a blank line, and the line `[1] the note`

### Requirement: HTML blocks are shown literally
The system SHALL render an HTML block as its literal source lines, dimmed when styling is on.

#### Scenario: Details block
- **WHEN** a document contains `<details>\n<summary>More</summary>\n</details>` and styling is off
- **THEN** the output contains the lines `<details>`, `<summary>More</summary>`, and `</details>`

### Requirement: Front matter is not rendered
The system SHALL NOT render YAML front matter delimited by `---` lines at the start of a document.

#### Scenario: Document with front matter
- **WHEN** a document is `---\ntitle: Doc\n---\n\n# Body`
- **THEN** the output does not contain `title: Doc` and contains the line `# Body`

### Requirement: Glyph set follows the locale with an ASCII override
The system SHALL use Unicode glyphs (`•`, `│`, `─`, `┼`) when `LC_ALL`, `LC_CTYPE`, or `LANG` (first one set) names a UTF-8 encoding or when none of them is set, SHALL use ASCII glyphs (`*`, `|`, `-`, `+`) otherwise, and SHALL use ASCII glyphs whenever `--ascii` is given.

#### Scenario: UTF-8 locale
- **WHEN** `LANG=en_US.UTF-8` and a document contains `---` rendered at width 10
- **THEN** the output contains a line of ten `─` characters

#### Scenario: Non-UTF-8 locale
- **WHEN** `LC_ALL=C` and a document contains `---` rendered at width 10
- **THEN** the output contains a line of ten `-` characters

#### Scenario: ASCII flag overrides locale
- **WHEN** `LANG=en_US.UTF-8` and the user passes `--ascii` and a document contains `- item`
- **THEN** the output contains the line `* item`
