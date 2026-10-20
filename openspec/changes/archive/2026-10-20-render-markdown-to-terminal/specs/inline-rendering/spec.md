# Spec Delta

## Purpose

Maps inline Markdown constructs to terminal text attributes so emphasis, code, and links are visually distinguishable while the underlying text is preserved exactly.

## ADDED Requirements

### Requirement: Strong emphasis renders as bold
The system SHALL render strong emphasis (`**text**` or `__text__`) as its text in bold, without the delimiters.

#### Scenario: Asterisk and underscore forms
- **WHEN** a paragraph contains `**one** and __two__` and styling is on
- **THEN** `one` and `two` are each wrapped in the bold attribute and no `*` or `_` delimiters appear in the output

#### Scenario: Plain mode
- **WHEN** a paragraph contains `**one**` and styling is off
- **THEN** the output contains `one` with no delimiters and no escape sequences

### Requirement: Emphasis renders as italic
The system SHALL render emphasis (`*text*` or `_text_`) as its text in italic, without the delimiters.

#### Scenario: Italic attribute
- **WHEN** a paragraph contains `*soft*` and styling is on
- **THEN** `soft` is wrapped in the italic attribute

### Requirement: Strikethrough renders with the strikethrough attribute
The system SHALL render `~~text~~` as its text with the strikethrough attribute, without the delimiters.

#### Scenario: Struck text
- **WHEN** a paragraph contains `~~old~~` and styling is on
- **THEN** `old` is wrapped in the strikethrough attribute

### Requirement: Nested inline styles combine
The system SHALL apply all enclosing inline styles to nested text simultaneously.

#### Scenario: Bold italic
- **WHEN** a paragraph contains `**bold *both* bold**` and styling is on
- **THEN** `both` carries both the bold and italic attributes and the surrounding `bold` text carries only bold

### Requirement: Code spans render literally in a distinct color
The system SHALL render a code span as its exact content in a distinct foreground color, without the backticks, and SHALL NOT interpret Markdown inside it.

#### Scenario: Markup inside code is literal
- **WHEN** a paragraph contains `` `a*b*c` ``
- **THEN** the output contains the text `a*b*c` with no italic attribute applied to `b`

#### Scenario: Backticks removed
- **WHEN** a paragraph contains `` run `cargo build` now `` and styling is off
- **THEN** the output contains `run cargo build now`

### Requirement: Links show their text and destination
The system SHALL render a link as its text, underlined, followed by a space and the destination URL in parentheses, and SHALL ignore any link title.

#### Scenario: Link with distinct text
- **WHEN** a paragraph contains `[docs](https://example.com/docs "Title")` and styling is off
- **THEN** the output contains `docs (https://example.com/docs)` and does not contain `Title`

#### Scenario: Underline attribute on text only
- **WHEN** a paragraph contains `[docs](https://example.com)` and styling is on
- **THEN** `docs` is wrapped in the underline attribute and the parenthesized URL is not underlined

### Requirement: Links whose text is their destination show the URL once
The system SHALL render a link as only its destination, underlined, when the link text equals the destination, including autolinks and bare URLs.

#### Scenario: Angle-bracket autolink
- **WHEN** a paragraph contains `<https://example.com>` and styling is off
- **THEN** the output contains `https://example.com` exactly once and contains no parentheses around it

#### Scenario: Bare URL
- **WHEN** a paragraph contains `see https://example.com today` and styling is off
- **THEN** the output contains `see https://example.com today`

#### Scenario: Email autolink
- **WHEN** a paragraph contains `<user@example.com>` and styling is off
- **THEN** the output contains `user@example.com` and does not contain `mailto:`

### Requirement: Images render as a labeled placeholder
The system SHALL render an image as `[image: <alt>]` followed by a space and the source in parentheses, or `[image]` when the alt text is empty.

#### Scenario: Image with alt text
- **WHEN** a paragraph contains `![Logo](logo.png)` and styling is off
- **THEN** the output contains `[image: Logo] (logo.png)`

#### Scenario: Image without alt text
- **WHEN** a paragraph contains `![](logo.png)` and styling is off
- **THEN** the output contains `[image] (logo.png)`

### Requirement: Soft line breaks become spaces
The system SHALL treat a single newline inside a paragraph as a space so the paragraph can be re-flowed.

#### Scenario: Two source lines become one sentence
- **WHEN** a paragraph is written as `first line\nsecond line` and the width is 80
- **THEN** the output contains `first line second line` on one line

### Requirement: Hard line breaks start a new line
The system SHALL render a hard line break (two trailing spaces or a trailing backslash) as a line break within the paragraph.

#### Scenario: Backslash break
- **WHEN** a paragraph is written as `roses\\\nviolets`
- **THEN** `roses` and `violets` appear on consecutive output lines

### Requirement: Inline HTML is shown literally
The system SHALL render inline HTML as its literal source text, dimmed when styling is on.

#### Scenario: Keyboard tag
- **WHEN** a paragraph contains `press <kbd>Ctrl</kbd>` and styling is off
- **THEN** the output contains `press <kbd>Ctrl</kbd>`

### Requirement: Escapes and entities are decoded
The system SHALL render backslash escapes and HTML entities as the characters they denote.

#### Scenario: Escaped asterisk and named entity
- **WHEN** a paragraph contains `\*not italic\* &amp; &copy;`
- **THEN** the output contains `*not italic* & ©` with no italic attribute

### Requirement: Footnote references render as numbered markers
The system SHALL render a footnote reference as `[n]`, where n is the footnote's number in order of first reference.

#### Scenario: Two references
- **WHEN** a document contains `alpha[^b] beta[^a]` with definitions for both
- **THEN** the output contains `alpha[1] beta[2]`
