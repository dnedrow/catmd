# input-sources Specification

## Purpose
Defines which Markdown sources `catmd` reads and in what order, so the tool accepts files and standard input the way `cat` does.

## Requirements

### Requirement: Read Markdown from positional file paths
The system SHALL treat each positional argument as the path of a Markdown file to read, and SHALL render the inputs in the order they were given.

#### Scenario: Single file
- **WHEN** the user runs `catmd README.md`
- **THEN** the rendered contents of `README.md` are written to standard output

#### Scenario: Multiple files keep argument order
- **WHEN** the user runs `catmd a.md b.md` where `a.md` contains `# Alpha` and `b.md` contains `# Beta`
- **THEN** the output contains the rendering of `Alpha` before the rendering of `Beta`

### Requirement: Read standard input when no file is given
The system SHALL read Markdown from standard input when no positional arguments are supplied.

#### Scenario: Piped input
- **WHEN** the user runs `printf '# Hi\n' | catmd`
- **THEN** the output contains the rendering of the heading `Hi`

### Requirement: A dash argument denotes standard input
The system SHALL treat the positional argument `-` as standard input, in the position where it appears among the other inputs.

#### Scenario: Dash between files
- **WHEN** the user runs `printf 'middle' | catmd a.md - b.md`
- **THEN** the output contains the rendering of `a.md`, then `middle`, then `b.md`, in that order

### Requirement: Each input is rendered as an independent document
The system SHALL parse and render each input as its own Markdown document, and SHALL separate the output of consecutive inputs with a single blank line.

#### Scenario: Paragraphs do not merge across files
- **WHEN** `a.md` ends with the text `end of a` and no trailing newline, and `b.md` starts with `start of b`
- **THEN** `end of a` and `start of b` appear in separate paragraphs in the output

#### Scenario: Footnotes and reference links do not leak across inputs
- **WHEN** `a.md` defines a reference link `[ref]: https://example.com` and `b.md` uses `[text][ref]` without defining it
- **THEN** the usage in `b.md` is rendered as literal text, not as a link to `https://example.com`

### Requirement: Unreadable inputs are reported without aborting
The system SHALL report an input that cannot be read on standard error, naming the path and the reason, SHALL continue with the remaining inputs, and SHALL exit with a non-zero status once all inputs have been processed.

#### Scenario: Missing file among valid files
- **WHEN** the user runs `catmd a.md missing.md b.md` and `missing.md` does not exist
- **THEN** the renderings of `a.md` and `b.md` are both written to standard output, standard error contains a message naming `missing.md`, and the exit status is non-zero

#### Scenario: Directory given as input
- **WHEN** the user runs `catmd some-directory/`
- **THEN** standard error contains a message naming `some-directory/` as unreadable and the exit status is non-zero

### Requirement: Successful runs exit with status zero
The system SHALL exit with status zero when every input was read and rendered.

#### Scenario: All inputs readable
- **WHEN** the user runs `catmd a.md b.md` and both files are readable
- **THEN** the exit status is zero

### Requirement: Input is decoded as UTF-8 with replacement
The system SHALL decode input as UTF-8 and SHALL replace invalid byte sequences with U+FFFD rather than failing.

#### Scenario: Invalid UTF-8 bytes
- **WHEN** an input file contains the bytes `hello \xff world`
- **THEN** the output contains `hello � world` and the exit status is zero

### Requirement: Empty input produces empty output
The system SHALL write nothing to standard output for an input that is empty or contains only whitespace.

#### Scenario: Empty file
- **WHEN** the user runs `catmd empty.md` where `empty.md` is zero bytes
- **THEN** standard output is empty and the exit status is zero
