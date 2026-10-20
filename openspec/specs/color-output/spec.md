# color-output Specification

## Purpose
Decides whether `catmd` emits ANSI styling so output is styled on capable terminals and clean plain text everywhere else, honoring the user's explicit choice and common environment conventions.

## Requirements

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
When styling is on the system SHALL use only the bold, dim, italic, underline, and strikethrough attributes and foreground colors, and SHALL NOT emit a color sequence beyond the resolved color depth: no `38;5;` or `38;2;` sequences at 16 colors, and no `38;2;` sequences at 256 colors.

#### Scenario: No 256-color or truecolor sequences
- **WHEN** any document is rendered with `--color always` at a resolved depth of 16 colors
- **THEN** the output contains no `38;5;` (256-color) or `38;2;` (truecolor) foreground sequences

#### Scenario: No truecolor sequences at 256 colors
- **WHEN** any document is rendered with `--color always` at a resolved depth of 256 colors with a theme that uses hex colors
- **THEN** the output contains no `38;2;` sequences

#### Scenario: Built-in theme stays portable at every depth
- **WHEN** any document is rendered with `--color always` and no theme at a resolved depth of 16 million colors
- **THEN** the output contains no `38;5;` or `38;2;` sequences

#### Scenario: No background colors
- **WHEN** any document is rendered with `--color always` with any theme
- **THEN** the output contains no `48;` background sequences

### Requirement: Styles never span line boundaries
The system SHALL close every active style attribute before each line break and re-open it on the next line, so that every output line is self-contained.

#### Scenario: Wrapped bold text
- **WHEN** a paragraph of bold text wraps across two lines with `--color always`
- **THEN** each of the two lines contains its own bold start sequence and ends with attributes reset

### Requirement: Color depth is detected from the environment
When styling is on and `--color-depth` is `auto`, the system SHALL resolve the terminal's color depth from environment variables alone: 16 million colors when `COLORTERM` is `truecolor` or `24bit` (case-insensitive) or `TERM` contains `direct`; 256 colors when `TERM` contains `256color`; otherwise 16 colors.

#### Scenario: COLORTERM announces truecolor
- **WHEN** `COLORTERM=truecolor` and `TERM=xterm-256color`
- **THEN** the resolved depth is 16 million colors

#### Scenario: TERM announces 256 colors
- **WHEN** `COLORTERM` is unset and `TERM=screen-256color`
- **THEN** the resolved depth is 256 colors

#### Scenario: Plain TERM
- **WHEN** `COLORTERM` is unset and `TERM=xterm`
- **THEN** the resolved depth is 16 colors

#### Scenario: Depth applies to forced color in a pipe
- **WHEN** the user runs `catmd --color always doc.md | cat` with `TERM=xterm-256color` and a theme that uses a 256-color index
- **THEN** the output contains the corresponding `38;5;` sequence

### Requirement: Color depth can be overridden
The system SHALL accept `--color-depth <auto|16|256|truecolor>`, defaulting to `auto`, SHALL use the given depth instead of detection, and SHALL reject any other value with a usage error.

#### Scenario: Forcing 16 colors on a truecolor terminal
- **WHEN** `COLORTERM=truecolor` and the user passes `--color-depth 16` with a theme that uses a hex color
- **THEN** the output contains no `38;5;` or `38;2;` sequences

#### Scenario: Forcing truecolor in tmux
- **WHEN** `TERM=screen` with `COLORTERM` unset and the user passes `--color-depth truecolor` with a theme that uses a hex color
- **THEN** the output contains the corresponding `38;2;` sequence

#### Scenario: Invalid depth
- **WHEN** the user passes `--color-depth 88`
- **THEN** standard error contains a usage message and the exit status is 2

### Requirement: Theme colors are downgraded to the resolved depth
The system SHALL convert every theme color to the nearest color representable at the resolved depth before emitting it: a hex color becomes the nearest 256-color index at 256 colors and the nearest of the 16 ANSI colors at 16 colors; a 256-color index becomes the nearest ANSI color at 16 colors; ANSI color names are emitted unchanged at every depth.

#### Scenario: Hex color on a 256-color terminal
- **WHEN** a theme sets `code = "#ff0000"` and the resolved depth is 256 colors
- **THEN** code spans are emitted with a `38;5;` sequence (index 196, the nearest cube color) and no `38;2;` sequence appears

#### Scenario: Hex color on a 16-color terminal
- **WHEN** a theme sets `code = "#ff0000"` and the resolved depth is 16 colors
- **THEN** code spans are emitted with a standard red foreground sequence (`31` or `91`) and no `38;5;` or `38;2;` sequence appears

#### Scenario: Index on a 16-color terminal
- **WHEN** a theme sets `code = 196` and the resolved depth is 16 colors
- **THEN** code spans are emitted with a standard red foreground sequence and no `38;5;` sequence appears

#### Scenario: Named color is never converted
- **WHEN** a theme sets `code = "yellow"` and the resolved depth is 16 million colors
- **THEN** code spans are emitted with the `33` foreground sequence
