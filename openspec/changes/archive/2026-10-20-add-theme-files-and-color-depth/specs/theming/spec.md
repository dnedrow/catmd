# Spec Delta

## Purpose

Lets users replace the built-in colors and text attributes with a theme file of their own, selected per run or by default, so `catmd` can match any terminal palette without a code change.

## ADDED Requirements

### Requirement: Themes are TOML files keyed by role
The system SHALL read a theme as a TOML document whose top-level keys are rendering role names (plus an optional `[palette]` table), each assigning the style used for that role.

#### Scenario: Minimal theme restyles one role
- **WHEN** the user passes `--theme t.toml` where `t.toml` contains `code = "red"` and renders `` `x` `` with `--color always` at 16 colors
- **THEN** `x` is emitted with a red foreground sequence (`31`)

#### Scenario: Styling off ignores the theme
- **WHEN** the user passes `--theme t.toml --color never`
- **THEN** the output is identical to rendering without `--theme`

### Requirement: A role entry is a color string or a style table
The system SHALL accept a role value either as a bare string naming a foreground color, or as a table with optional `fg` (a color) and optional `modifiers` (a list drawn from `bold`, `dim`, `italic`, `underline`, `strikethrough`). An entry replaces the role's built-in style entirely.

#### Scenario: Table form with modifiers
- **WHEN** a theme sets `heading1 = { fg = "cyan", modifiers = ["bold", "underline"] }` and a document contains `# T` rendered with `--color always`
- **THEN** `T` carries the bold and underline attributes and a cyan foreground, and no magenta sequence appears

#### Scenario: Bare string replaces modifiers too
- **WHEN** a theme sets `strong = "red"` and a paragraph contains `**b**` rendered with `--color always`
- **THEN** `b` is emitted with a red foreground and without the bold attribute

#### Scenario: Modifiers without a color
- **WHEN** a theme sets `link_url = { modifiers = ["dim"] }`
- **THEN** link URLs are emitted dimmed with no foreground color sequence

### Requirement: Colors are ANSI names, 256-color indices, hex values, or palette names
The system SHALL accept a color as one of the sixteen ANSI names (`black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `white`, and their `bright_` variants), a TOML integer from 0 to 255, a string of the form `#rrggbb`, or a key of the `[palette]` table.

#### Scenario: Each color form is accepted
- **WHEN** a theme sets `code = "bright_yellow"`, `emph = 208`, `strong = "#FF8800"`, and `link_text = "accent"` with `[palette] accent = "#00aaff"`
- **THEN** the theme loads and each role receives the corresponding color

#### Scenario: Palette entries may use any color form and are resolved first
- **WHEN** a theme defines `[palette] red = 196` and sets `code = "red"`
- **THEN** code spans use 256-color index 196, not the ANSI color named `red`

#### Scenario: Short hex is rejected
- **WHEN** a theme sets `code = "#f80"`
- **THEN** standard error names the theme and reports the invalid color and the exit status is 2

#### Scenario: Index out of range is rejected
- **WHEN** a theme sets `code = 256`
- **THEN** standard error names the theme and reports the invalid color and the exit status is 2

### Requirement: Every visible element has a role
The system SHALL recognize exactly these roles: `heading_marker`, `heading1` … `heading6`, `strong`, `emph`, `strikethrough`, `code`, `code_block`, `link_text`, `link_url`, `image`, `quote_gutter`, `alert_note`, `alert_tip`, `alert_important`, `alert_warning`, `alert_caution`, `table_header`, `table_border`, `rule`, `html`, `footnote_marker`, `bullet`, `list_number`, `task_checkbox`.

#### Scenario: Theme setting every role loads
- **WHEN** a theme assigns a color to each of the 29 roles above
- **THEN** the theme loads without error

#### Scenario: Newly exposed roles take effect
- **WHEN** a theme sets `bullet = "green"`, `list_number = "green"`, `task_checkbox = "green"`, and `table_border = "green"` and a document contains a bullet item, an ordered item, a task item, and a table rendered with `--color always`
- **THEN** the bullet glyph, the ordered marker, the `[ ]` checkbox, and the table's separators and header rule are each emitted with a green foreground

### Requirement: Omitted roles keep the built-in style
The system SHALL apply the built-in default style to every role a theme does not mention.

#### Scenario: Partial theme
- **WHEN** a theme contains only `code = "red"` and a document contains `# T` rendered with `--color always`
- **THEN** `T` is rendered exactly as it is without a theme (bold, underline, bright magenta)

#### Scenario: Default appearance is unchanged by this capability
- **WHEN** a document containing a bullet list, an ordered list, a task item, and a table is rendered with `--color always` and no theme
- **THEN** the bullet, ordered marker, and checkbox carry no escape sequences and the table separators and header rule are dimmed, exactly as before themes existed

### Requirement: The theme is selected by path or name
The system SHALL accept `--theme <value>`: a value containing a path separator or ending in `.toml` is read as a file path; any other value names the file `<value>.toml` inside `$XDG_CONFIG_HOME/catmd/themes/`, or `~/.config/catmd/themes/` when `XDG_CONFIG_HOME` is unset or empty.

#### Scenario: Explicit relative path
- **WHEN** the user passes `--theme ./themes/dark.toml`
- **THEN** that file is loaded

#### Scenario: Name resolved through XDG_CONFIG_HOME
- **WHEN** `XDG_CONFIG_HOME=/tmp/cfg` and the user passes `--theme dark` and `/tmp/cfg/catmd/themes/dark.toml` exists
- **THEN** that file is loaded

#### Scenario: Name resolved through HOME
- **WHEN** `XDG_CONFIG_HOME` is unset, `HOME=/tmp/home`, and the user passes `--theme dark`
- **THEN** `/tmp/home/.config/catmd/themes/dark.toml` is loaded

#### Scenario: Bare filename with extension is a path
- **WHEN** the user passes `--theme dark.toml` in a directory containing `dark.toml`
- **THEN** `./dark.toml` is loaded, not a file in the themes directory

### Requirement: CATMD_THEME supplies a default theme
The system SHALL use the value of `CATMD_THEME`, resolved exactly like a `--theme` argument, when `--theme` is not given and the variable is non-empty; `--theme` SHALL take precedence over it.

#### Scenario: Environment default
- **WHEN** `CATMD_THEME=./t.toml` is set, `--theme` is absent, and `t.toml` sets `code = "red"`
- **THEN** code spans are emitted red

#### Scenario: Flag wins
- **WHEN** `CATMD_THEME=./a.toml` is set and the user passes `--theme ./b.toml`
- **THEN** `b.toml` is loaded and `a.toml` is not read

### Requirement: Invalid themes fail fast
The system SHALL exit with status 2 and write a message to standard error naming the theme source and the problem when the theme file cannot be read, is not valid TOML, uses an unknown role or top-level key, lists an unknown modifier, or contains an unparseable color; nothing SHALL be written to standard output.

#### Scenario: Missing file
- **WHEN** the user passes `--theme ./nope.toml` and the file does not exist
- **THEN** standard error contains `nope.toml` and standard output is empty and the exit status is 2

#### Scenario: Missing named theme
- **WHEN** `XDG_CONFIG_HOME=/tmp/cfg` and the user passes `--theme nope` and no such file exists
- **THEN** standard error contains `/tmp/cfg/catmd/themes/nope.toml` and the exit status is 2

#### Scenario: Unknown role
- **WHEN** a theme contains `heading7 = "red"`
- **THEN** standard error contains `heading7` and the exit status is 2

#### Scenario: Unknown modifier
- **WHEN** a theme contains `strong = { modifiers = ["blink"] }`
- **THEN** standard error contains `blink` and the exit status is 2

#### Scenario: Invalid TOML
- **WHEN** a theme contains `code = "red`
- **THEN** standard error names the theme and reports a parse error and the exit status is 2
