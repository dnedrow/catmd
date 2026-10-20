# Proposal

## Why

catmd documents its usage in the README and command-line help, but lacks a manual page that users can read through the standard Unix manual interface. Shipping a self-contained manual in the repository makes that reference available offline without adding build dependencies.

## What Changes

- Add a handwritten section-1 manual at `man/catmd.1` using portable roff man macros.
- Document the existing synopsis, input handling, options and defaults, environment-variable precedence, paging, theme lookup and configuration, exit statuses, and practical examples.
- Add README instructions to preview the repository copy and manually install it into a manual search path.
- Explicitly explain that `cargo install --path .` installs the executable, not the manual page.
- Keep generation, install/uninstall helpers, release packaging, new dependencies, and runtime changes out of scope.

## Capabilities

### New Capabilities

None. This change documents existing behavior rather than introducing system behavior.

### Modified Capabilities

None. The existing input-sources, color-output, terminal-geometry, output-paging, and theming requirements remain unchanged. This documentation-only change declares `skip_specs: true`.

## Impact

- Implementation will affect only the new `man/catmd.1` and the existing `README.md`.
- CLI definitions in `src/cli.rs`, behavior in the input, environment, pager, and theme modules, and existing specs serve as reference material; they are not changed.
- No Cargo manifest, dependency, API, workflow, or release process changes are needed.
- Validation will compare the manual against existing behavior and inspect its rendered output with available system manual tools.
