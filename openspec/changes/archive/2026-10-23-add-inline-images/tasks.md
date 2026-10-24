## 1. Dependencies and module skeleton

- [x] 1.1 Add `image` (png, jpeg, gif, webp only), `kittage` and `icy_sixel` to Cargo.toml and create `src/image/` with `mod.rs` exported from `src/lib.rs`; verify `cargo build` succeeds
- [x] 1.2 Define `ImageBackend` trait, `BackendKind`, `Placement` and `Encoded` types; verify with a unit test using a fake backend

## 2. Loading and cache

- [x] 2.1 Implement local-file loading with document-relative path resolution and graceful failure for missing, remote, `data:` and undecodable sources; verify unit tests for each case
- [x] 2.2 Implement the two-tier cache (decoded by path; encoded by path, size, cell geometry, backend); verify tests that a repeated image decodes once and a width change reuses the decoded data but re-encodes

## 3. Scaling and layout

- [x] 3.1 Implement `plan` scaling: never upscale, preserve aspect ratio, cap at available columns, round rows; verify unit tests for wider, narrower and extreme-aspect images
- [x] 3.2 Add `Block::Image` for paragraphs consisting solely of an image, reserving whole rows and honoring quote and list indentation; verify exact-output cases in `tests/render.rs`
- [x] 3.3 Keep the inline label for images mixed with text and when inline images are inactive; verify the existing `inline-rendering` image tests still pass

## 4. Backends

- [x] 4.1 Implement the placeholder box (alt, source, `W×H`, unsupported note, truncation to width, glyph-set aware); verify exact-output tests including missing alt and unknown size
- [x] 4.2 Implement the half-block backend with ANSI colors; verify a 1x2 pixel image test and a width-fit test
- [x] 4.3 Implement the Kitty backend via kittage; verify the emitted sequence in a unit test
- [x] 4.4 Implement the SIXEL backend via icy_sixel; verify the emitted sequence in a unit test

## 5. Detection and selection

- [x] 5.1 Capture image-related env hints (`KITTY_WINDOW_ID`, `TERM`, `TERM_PROGRAM`, `TMUX`, `CATMD_IMAGES`) and cell pixel size in `Environment`; verify unit tests in `src/env.rs`
- [x] 5.2 Add the `--images <auto|kitty|sixel|blocks|none>` option in `src/cli.rs` and resolve it into `Settings`; verify the CLI tests parse each value
- [x] 5.3 Implement deterministic selection (Kitty, SIXEL, half-block, placeholder) and the TTY/color gate; verify a table-driven test over environments
- [x] 5.4 Implement the optional terminal probe (Kitty query, DA1, `CSI 16 t`) with a short timeout, run only when stdin and stdout are TTYs; verify timeout behavior with a unit test over a fake reader

## 6. Output and paging

- [x] 6.1 Bypass the external pager when a Kitty or SIXEL backend is selected; verify `resolve_pager` tests
- [x] 6.2 Verify piped output through the binary has no graphics sequences and keeps the plain label in `tests/cli.rs`, with environment variables controlled

## 7. Docs and final checks

- [x] 7.1 Update `man/catmd.1` for `--images`, `CATMD_IMAGES`, paging behavior and fallbacks; verify with `man ./man/catmd.1`
- [x] 7.2 Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test`; verify all pass
