# Spec Delta

## MODIFIED Requirements

### Requirement: Every visible element has a role
The system SHALL recognize exactly these roles: `heading_marker`, `heading1` … `heading6`, `strong`, `emph`, `strikethrough`, `code`, `code_block`, `link_text`, `link_url`, `image`, `quote_gutter`, `alert_note`, `alert_tip`, `alert_important`, `alert_warning`, `alert_caution`, `table_header`, `table_border`, `rule`, `html`, `footnote_marker`, `bullet`, `list_number`, `task_checkbox`, `syntax_keyword`, `syntax_string`, `syntax_comment`, `syntax_constant`, `syntax_type`, `syntax_function`, `syntax_tag`.

#### Scenario: Theme setting every role loads
- **WHEN** a theme assigns a color to each of the 36 roles above
- **THEN** the theme loads without error

#### Scenario: Newly exposed roles take effect
- **WHEN** a theme sets `bullet = "green"`, `list_number = "green"`, `task_checkbox = "green"`, and `table_border = "green"` and a document contains a bullet item, an ordered item, a task item, and a table rendered with `--color always`
- **THEN** the bullet glyph, the ordered marker, the `[ ]` checkbox, and the table's separators and header rule are each emitted with a green foreground

#### Scenario: Syntax roles take effect
- **WHEN** a theme sets `syntax_keyword = "red"` and `syntax_comment = { modifiers = ["italic"] }` and a document contains "```rust\nfn main() {} // x\n```" rendered with `--color always` at 16 colors
- **THEN** `fn` is emitted with a red foreground sequence (`31`) and `// x` is emitted with the italic attribute and no foreground color sequence

#### Scenario: Syntax roles are independent of the code block role
- **WHEN** a theme sets only `code_block = "white"` and a document contains "```rust\nfn main() {}\n```" rendered with `--color always` at 16 colors
- **THEN** `{}` is emitted with a white foreground sequence (`37`) and `fn` keeps its built-in magenta foreground sequence (`35`)

#### Scenario: Theme from before the syntax roles still loads
- **WHEN** a theme assigns a color to each of the 29 roles that predate the `syntax_*` roles and none of the `syntax_*` roles
- **THEN** the theme loads without error and highlighted tokens keep their built-in styles
