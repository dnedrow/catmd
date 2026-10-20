# Repository guidance

## Project structure

- `src/lib.rs` exposes the library modules; keep process I/O in `src/main.rs`,
  which should remain a thin CLI wrapper.
- `src/cli.rs` defines arguments. `src/env.rs` captures and resolves terminal
  settings. `src/input.rs` handles input sources. `src/pager.rs` delivers output.
- `src/render/` converts Markdown into terminal text; `src/theme/` defines
  styles, colors, and theme-file loading.
- `tests/render.rs` checks exact renderer output. `tests/cli.rs` exercises the
  compiled binary through pipes with controlled environment variables.
- `openspec/specs/` contains the current behavioral specifications. Consult
  the relevant spec when changing rendering, input, terminal geometry, paging,
  color, or theming.

## Development

Run these from the repository root:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Use `cargo run -- [OPTIONS] [FILE]...` for manual CLI checks. For a manual-page
preview, run `man ./man/catmd.1`.

## Change and test conventions

- Preserve the separation between library logic and process I/O. Use the
  `Environment` snapshot and settings resolution for environment-dependent
  behavior rather than reading process state throughout the renderer.
- Keep rendering deterministic for a fixed width, color mode, and glyph set.
  Add or update exact-output cases in `tests/render.rs` for rendering changes.
- Add CLI behavior coverage in `tests/cli.rs`; tests run the binary through
  pipes, so explicitly control environment variables when they affect results.
- Keep `man/catmd.1` in sync with the app's capabilities. Update it whenever
  CLI options, environment variables, paging, themes, rendering behavior, or
  exit statuses change.
- Prefer small, focused changes and follow the surrounding Rust style. Do not
  add dependencies without a clear need.
