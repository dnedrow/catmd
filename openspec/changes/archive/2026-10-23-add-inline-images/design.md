## Context

Today `render()` returns a deterministic `String` that is piped to an external pager (`less -FRX`) when taller than the terminal (`src/pager.rs`). Scrolling, search, and resize belong to `less`, which cannot carry Kitty or SIXEL sequences or redraw images on scroll. Images are inline spans (`src/render/inline.rs`), so they cannot own rows. `Environment` is a pure env snapshot; there is no terminal query. See proposal.md for motivation and scope; the interactive viewport is deferred to `add-builtin-pager`.

## Goals / Non-Goals

**Goals:**
- Protocol-independent backend trait; adding a backend touches only `src/image/` and the selection list.
- Images as block-level layout elements with exact row/column reservation.
- Output remains deterministic for a fixed width, color mode, glyph set, backend, and cell size.
- Seam for the follow-up pager: layout and encoding are pure and cacheable.

**Non-Goals:**
- Interactive scrolling, search, resize monitoring, or redraw/erase lifecycle (`add-builtin-pager`).
- Remote fetching, animation, user scaling config.

## Decisions

**Block-level image element.** A paragraph whose only content is an image becomes a `Block::Image`; images mixed with text stay as the inline label. Rationale: graphics cannot share a row with text portably. Alternative: lift all images out of paragraphs, rejected as it reorders content.

**Backend trait.** `ImageBackend` exposes `kind()`, `plan(natural, avail_cols, cell) -> Placement` (cols, rows, pixel size), and `encode(decoded, placement) -> Encoded` (escape bytes or text rows). Rationale: layout depends only on `plan`; protocol bytes stay in `encode`. A later `upload/place/erase` split for Kitty ids belongs to the pager change; the trait is shaped so that can be added without altering callers. Backends: Kitty (kittage), SIXEL (icy_sixel), HalfBlock, Placeholder.

**Output model for graphics.** The block reserves N blank-bearing rows; the encoded sequence is written on the first row and the cursor is advanced past the reserved rows. Alternative: placing the image after text rendering via row offsets, rejected as fragile.

**External pager bypass.** When a graphical backend is selected and output would page, write directly (no `less`), since `less` corrupts graphics. Half-block and placeholder output are plain text and page normally. Rationale: correctness over paging until the built-in pager exists.

**Selection and detection.** Order: Kitty, SIXEL, HalfBlock, Placeholder. Detection inputs, in order: explicit override (`--images <auto|kitty|sixel|blocks|none>`, `CATMD_IMAGES`), env hints (`KITTY_WINDOW_ID`, `TERM`, `TERM_PROGRAM`), then an optional active probe (Kitty query plus DA1 for SIXEL, short timeout) only when stdin and stdout are TTYs. Probe results enter `Environment` so the renderer stays pure. Rationale: env hints are cheap and testable; the probe handles tmux/ssh cases. Alternative: probe only, rejected for latency and test hostility.

**Cell geometry.** Prefer pixel size from `TIOCGWINSZ`, else `CSI 16 t` in the probe, else assume 8x16. Rendered rows = round(height_px / cell_h), columns = ceil(width_px / cell_w), both capped by available width.

**Scaling.** scale = min(1, avail_cols * cell_w / natural_w), aspect preserved, never upscale. Resample with `image`'s Lanczos3 only when the target differs from natural size.

**Cache.** Tier 1: decoded `DynamicImage` plus natural size, keyed by canonical path. Tier 2: `Encoded`, keyed by (path, cols, rows, cell size, backend). Resize or backend change drops only tier 2. Process-lifetime in-memory maps for now.

**Loading.** Local files only; relative paths resolve against the input file's directory (cwd for stdin). `http(s)` and `data:` sources render the placeholder. Decoder features limited to png, jpeg, gif (first frame), webp.

**Placeholder box.** Plain Unicode or ASCII glyph set from `Glyphs`; lines truncated to fit; lines: alt, source, `WxH`, "Image display unsupported".

## Risks / Trade-offs

- [Probe hangs or garbles on odd terminals] -> short timeout, only on TTY, `--images` override.
- [Wrong cell pixel size distorts height] -> default ratio, documented override; rows rounded not floored.
- [Direct write skips paging for long documents with images] -> accepted until `add-builtin-pager`; documented in the man page.
- [tmux needs passthrough wrapping] -> detect `TMUX` and wrap or fall back to half-block.
- [Binary size and compile time from `image`] -> minimal features.

## Open Questions

- Exact flag/env names (`--images`, `CATMD_IMAGES`) can be finalized at implementation.
- Whether a size limit on decoded images is needed.
