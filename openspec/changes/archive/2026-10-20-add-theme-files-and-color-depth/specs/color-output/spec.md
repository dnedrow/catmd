# Spec Delta

## ADDED Requirements

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

## MODIFIED Requirements

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
