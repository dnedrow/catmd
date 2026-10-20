# Tasks

This documentation-only change follows `proposal.md`. Specs are explicitly skipped via `skip_specs: true`; the conditional design artifact is deliberately omitted because there are no unresolved technical decisions or architectural changes.

## 1. Handwritten manual

- [x] 1.1 Add `man/catmd.1` as a self-contained section-1 manual using portable roff man macros, with NAME, SYNOPSIS, DESCRIPTION, OPTIONS, ENVIRONMENT, FILES, THEMES, EXIT STATUS, EXAMPLES, and SEE ALSO sections; verify that a system formatter (`mandoc` or `groff -man -Tascii`, as available) renders it without errors and that headings and examples remain readable.
- [x] 1.2 Document every positional argument and option in `src/cli.rs`, including help/version, accepted enum values, defaults, positive width/height requirements, the width floor of 20, standard input via no files or `-`, ordered independent documents, and continued processing after unreadable inputs; verify coverage against the argument definitions, `src/input.rs`, `src/env.rs`, `src/main.rs`, and the existing CLI tests.
- [x] 1.3 Document existing environment and paging behavior, including `CATMD_THEME`, `XDG_CONFIG_HOME`, `HOME`, `CATMD_PAGER`, `PAGER`, `LESS`, `NO_COLOR`, `CLICOLOR_FORCE`, `CLICOLOR`, `COLORTERM`, `TERM`, `COLUMNS`, `LINES`, `LC_ALL`, `LC_CTYPE`, and `LANG`; verify precedence, empty-value handling, terminal-only paging, and pager-failure fallback against `src/env.rs`, `src/pager.rs`, and `src/theme/file.rs` rather than assuming that all variables treat empty values alike.
- [x] 1.4 Include theme path/name resolution, the TOML role/style/color/palette syntax, supported roles and modifiers, replacement versus omitted-role behavior, color-depth downgrade behavior, a small valid theme example, exit statuses 0/1/2, and practical file/stdin/color/pager/theme examples; verify the descriptions against the theme modules and `src/main.rs`, and check that the manual can be understood without opening the README.

## 2. Preview and manual installation documentation

- [x] 2.1 Extend the README installation section with `man ./man/catmd.1`, a link to the checked-in page, and an explicit statement that `cargo install --path .` does not install manuals; verify the preview command from the repository root with a noninteractive pager.
- [x] 2.2 Document manual copying into a chosen prefix's `share/man/man1` directory, including directory creation and how to make a user-local prefix discoverable via the manual search path without hiding platform defaults; verify installation and `man catmd` lookup using a temporary prefix and temporary environment settings, without modifying system directories or persistent shell configuration.

## 3. Documentation integration

- [x] 3.1 Review the rendered manual and README together for consistent option names, examples, paths, defaults, and installation caveats; verify lookup of the installed temporary copy, inspect formatter diagnostics, and remove only temporary files created for these checks.
- [x] 3.2 Inspect the implementation diff to confirm that changes are limited to `man/catmd.1`, `README.md`, and this change's planning artifacts; verify that no generator, dependency, installer helper, release packaging, workflow, or runtime changes were introduced.
