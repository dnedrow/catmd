# Spec Delta

## MODIFIED Requirements

### Requirement: Code blocks are verbatim and indented
The system SHALL render fenced and indented code blocks with each content line preceded by four spaces, preserving content exactly without wrapping, and SHALL NOT render the fence or info string. When styling is on, content is emitted in the `code_block` style, except that tokens of a fenced block whose info string names a recognized language are emitted in their syntax role's style instead.

#### Scenario: Fenced block with language
- **WHEN** a document contains "```rust\nfn main() {}\n```" and styling is off
- **THEN** the output contains the line `    fn main() {}` and does not contain `rust` or backticks

#### Scenario: Long code line is not wrapped
- **WHEN** a code block contains a 120-character line and the width is 40
- **THEN** that line appears in the output unbroken

#### Scenario: Blank lines inside code are preserved
- **WHEN** a code block contains `a\n\nb`
- **THEN** the output contains `    a`, an empty line, and `    b` consecutively

#### Scenario: Fenced block without a language is styled uniformly
- **WHEN** a document contains "```\ncode\n```" and styling is on
- **THEN** the content line is emitted in the `code_block` style and carries no other style

#### Scenario: Fenced block with a recognized language is styled by token
- **WHEN** a document contains "```rust\nfn main() {}\n```" and styling is on
- **THEN** the output contains a line whose text is `    fn main() {}`, in which `fn` carries the `syntax_keyword` style and `{}` carries the `code_block` style
