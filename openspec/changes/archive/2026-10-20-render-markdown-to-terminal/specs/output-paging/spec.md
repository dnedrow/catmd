# Spec Delta

## Purpose

Hands long output to the user's pager when it would scroll off the screen, while keeping short output streaming to the terminal exactly like `cat`.

## ADDED Requirements

### Requirement: Output is paged only when it would overflow a terminal
The system SHALL send rendered output through a pager only when standard output is a terminal, paging is not disabled, the height is known, and the output would occupy more terminal rows than the height (lines wider than the width count as the rows they wrap to); otherwise it SHALL write directly to standard output.

#### Scenario: Short document streams directly
- **WHEN** standard output is a terminal 40 rows tall and the rendered output is 10 lines
- **THEN** no pager process is started and the output is written to standard output

#### Scenario: Tall document is paged
- **WHEN** standard output is a terminal 24 rows tall and the rendered output is 60 lines
- **THEN** a pager process is started and receives the rendered output

#### Scenario: Output to a pipe is never paged
- **WHEN** the user runs `catmd --height 5 long.md | cat` and the rendered output is 60 lines
- **THEN** no pager process is started

#### Scenario: Height override drives the threshold
- **WHEN** standard output is a terminal 50 rows tall, the user passes `--height 10`, and the rendered output is 20 lines
- **THEN** a pager process is started

### Requirement: Paging can be disabled
The system SHALL accept `--no-pager` and SHALL never start a pager when it is given.

#### Scenario: Flag given on a small terminal
- **WHEN** standard output is a terminal 24 rows tall, the rendered output is 60 lines, and the user passes `--no-pager`
- **THEN** no pager process is started and the output is written to standard output

### Requirement: Pager command is chosen from the environment
The system SHALL use the first non-empty value of `CATMD_PAGER`, then `PAGER`, as a shell command line for the pager, falling back to `less` when neither is set; a value of `cat` SHALL be treated as paging disabled.

#### Scenario: CATMD_PAGER takes precedence
- **WHEN** `CATMD_PAGER=more` and `PAGER=less` are set and paging is triggered
- **THEN** the pager process started is `more`

#### Scenario: Pager value with arguments
- **WHEN** `PAGER="less -S"` is set and paging is triggered
- **THEN** the pager process started is `less` with the argument `-S`

#### Scenario: PAGER set to cat
- **WHEN** `PAGER=cat` is set and the output would otherwise be paged
- **THEN** no pager process is started and the output is written directly to standard output

### Requirement: Default less behavior preserves colors and screen contents
When the `LESS` environment variable is unset, the system SHALL run the pager with `LESS=FRX` in its environment so that `less` interprets escape sequences, exits immediately on a single screen, and leaves the output visible after exiting; other pagers ignore the variable.

#### Scenario: LESS unset
- **WHEN** `PAGER` and `LESS` are both unset and paging is triggered
- **THEN** the `less` process is started with `LESS=FRX` in its environment

#### Scenario: Explicit PAGER=less without LESS
- **WHEN** `PAGER=less` is set, `LESS` is unset, and paging is triggered
- **THEN** the `less` process is started with `LESS=FRX` in its environment

#### Scenario: LESS already set
- **WHEN** `LESS=-i` is set and paging is triggered
- **THEN** the pager process is started with `LESS=-i` unchanged

### Requirement: Color decision survives the pager
The system SHALL decide whether to style output based on the original standard output terminal before starting the pager, and SHALL send that styled output to the pager unchanged.

#### Scenario: Styled output through the pager
- **WHEN** standard output is a color-capable terminal, color mode is `auto`, and paging is triggered for a document containing `**bold**`
- **THEN** the data written to the pager contains the bold escape sequence around `bold`

### Requirement: Pager failure falls back to direct output
If the pager process cannot be started, the system SHALL report the failure on standard error and write the rendered output directly to standard output.

#### Scenario: Pager command not found
- **WHEN** `PAGER=definitely-not-a-real-pager` is set and paging is triggered
- **THEN** standard error contains a message naming the pager, the rendered output appears on standard output, and the exit status reflects only input read errors

### Requirement: The process waits for the pager
The system SHALL wait for the pager process to exit before exiting, and its exit status SHALL reflect input read errors rather than the pager's exit status.

#### Scenario: User quits the pager
- **WHEN** paging is triggered for readable inputs and the user quits the pager
- **THEN** `catmd` exits after the pager with status zero
