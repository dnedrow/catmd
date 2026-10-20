# terminal-geometry Specification

## Purpose
Determines the column width used to lay out output and the row height used for paging decisions, from the attached terminal or from explicit user overrides.

## Requirements

### Requirement: Width is detected from the terminal
When standard output is a terminal and `--width` is not given, the system SHALL use the terminal's current column count as the rendering width.

#### Scenario: Rule spans the detected width
- **WHEN** standard output is a terminal 100 columns wide and the document contains a thematic break
- **THEN** the rendered rule is exactly 100 columns wide

### Requirement: Width falls back when the terminal cannot be queried
When the width cannot be read from the terminal and `--width` is not given, the system SHALL use the `COLUMNS` environment variable if it holds a positive integer, and otherwise SHALL use 80.

#### Scenario: Piped output without COLUMNS
- **WHEN** the user runs `catmd doc.md | cat` with `COLUMNS` unset and the document contains a thematic break
- **THEN** the rendered rule is exactly 80 columns wide

#### Scenario: Piped output with COLUMNS
- **WHEN** the user runs `COLUMNS=60 catmd doc.md | cat` and the document contains a thematic break
- **THEN** the rendered rule is exactly 60 columns wide

### Requirement: Width can be overridden
The system SHALL accept `--width <N>` where N is a positive integer and SHALL use it as the rendering width regardless of the terminal or environment.

#### Scenario: Override on a wide terminal
- **WHEN** standard output is a terminal 120 columns wide and the user passes `--width 40`
- **THEN** paragraphs wrap so that no wrapped line exceeds 40 columns

#### Scenario: Non-numeric width
- **WHEN** the user passes `--width wide`
- **THEN** standard error contains a usage message and the exit status is non-zero

### Requirement: Effective width has a floor
The system SHALL raise any detected, environment-provided, or user-provided width below 20 columns to 20 so that layout remains well-formed.

#### Scenario: Tiny override is clamped
- **WHEN** the user passes `--width 5` and the document contains a thematic break
- **THEN** the rendered rule is 20 columns wide

### Requirement: Height is detected from the terminal
When standard output is a terminal and `--height` is not given, the system SHALL use the terminal's current row count as the height for paging decisions.

#### Scenario: Document taller than the terminal
- **WHEN** standard output is a terminal 24 rows tall and the rendered document is 30 lines
- **THEN** the paging threshold used is 24 rows

### Requirement: Height falls back when the terminal cannot be queried
When the height cannot be read from the terminal and `--height` is not given, the system SHALL use the `LINES` environment variable if it holds a positive integer, and otherwise SHALL treat the height as unknown.

#### Scenario: Piped output without LINES
- **WHEN** the user runs `catmd doc.md | cat` with `LINES` unset
- **THEN** the height is unknown and no paging threshold applies

### Requirement: Height can be overridden
The system SHALL accept `--height <N>` where N is a positive integer and SHALL use it as the height for paging decisions regardless of the terminal or environment.

#### Scenario: Override lowers the threshold
- **WHEN** standard output is a terminal 50 rows tall and the user passes `--height 10`
- **THEN** the paging threshold used is 10 rows

#### Scenario: Zero height
- **WHEN** the user passes `--height 0`
- **THEN** standard error contains a usage message and the exit status is non-zero
