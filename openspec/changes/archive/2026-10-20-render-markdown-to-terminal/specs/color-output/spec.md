# Spec Delta

## Purpose

Decides whether `catmd` emits ANSI styling so output is styled on capable terminals and clean plain text everywhere else, honoring the user's explicit choice and common environment conventions.

## ADDED Requirements

### Requirement: Color mode is selectable with a flag
The system SHALL accept `--color <auto|always|never>`, defaulting to `auto`, and SHALL reject any other value with a usage error.

#### Scenario: Never disables styling on a terminal
- **WHEN** the user runs `catmd --color never doc.md` with standard output attached to a color-capable terminal
- **THEN** the output contains no escape sequences

#### Scenario: Always forces styling into a pipe
- **WHEN** the user runs `catmd --color always doc.md | cat` where `doc.md` contains `**bold**`
- **THEN** the output contains the bold escape sequence around `bold`

#### Scenario: Invalid value
- **WHEN** the user runs `catmd --color sometimes doc.md`
- **THEN** standard error contains a usage message and the exit status is non-zero

### Requirement: Auto mode styles only color-capable terminals
In `auto` mode the system SHALL emit styling only when standard output is a terminal and the terminal is not identified as `dumb` via the `TERM` variable.

#### Scenario: Interactive terminal
- **WHEN** standard output is a terminal with `TERM=xterm-256color` and no overriding environment variables are set
- **THEN** the output contains styling escape sequences

#### Scenario: Output piped to another program
- **WHEN** the user runs `catmd doc.md | cat`
- **THEN** the output contains no escape sequences

#### Scenario: Dumb terminal
- **WHEN** standard output is a terminal and `TERM=dumb`
- **THEN** the output contains no escape sequences

### Requirement: NO_COLOR disables styling in auto mode
The system SHALL emit no styling in `auto` mode when the `NO_COLOR` environment variable is set to a non-empty value, and SHALL still honor an explicit `--color always`.

#### Scenario: NO_COLOR on a terminal
- **WHEN** `NO_COLOR=1` is set and standard output is a color-capable terminal
- **THEN** the output contains no escape sequences

#### Scenario: Explicit always overrides NO_COLOR
- **WHEN** `NO_COLOR=1` is set and the user runs `catmd --color always doc.md`
- **THEN** the output contains styling escape sequences

### Requirement: CLICOLOR conventions are honored in auto mode
In `auto` mode the system SHALL force styling on when `CLICOLOR_FORCE` is set to a non-empty value other than `0`, and SHALL disable styling when `CLICOLOR=0`.

#### Scenario: CLICOLOR_FORCE into a pipe
- **WHEN** `CLICOLOR_FORCE=1` is set and the user runs `catmd doc.md | cat`
- **THEN** the output contains styling escape sequences

#### Scenario: CLICOLOR=0 on a terminal
- **WHEN** `CLICOLOR=0` is set and standard output is a color-capable terminal
- **THEN** the output contains no escape sequences

### Requirement: Plain output preserves all text
When styling is off the system SHALL write output that contains no ESC (0x1B) bytes and that contains exactly the same text content, in the same order, as the styled output.

#### Scenario: Styled and plain renderings agree on text
- **WHEN** the same document is rendered once with `--color always` and once with `--color never` at the same width
- **THEN** stripping all escape sequences from the styled output yields output identical to the plain output

### Requirement: Styling uses portable terminal attributes
When styling is on the system SHALL use only the bold, dim, italic, underline, and strikethrough attributes and foreground colors from the standard 16-color ANSI palette, so output renders on basic terminals.

#### Scenario: No 256-color or truecolor sequences
- **WHEN** any document is rendered with `--color always`
- **THEN** the output contains no `38;5;` (256-color) or `38;2;` (truecolor) foreground sequences

### Requirement: Styles never span line boundaries
The system SHALL close every active style attribute before each line break and re-open it on the next line, so that every output line is self-contained.

#### Scenario: Wrapped bold text
- **WHEN** a paragraph of bold text wraps across two lines with `--color always`
- **THEN** each of the two lines contains its own bold start sequence and ends with attributes reset
