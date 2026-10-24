## Why

Markdown images currently render only as an `[image: alt] (src)` text label, so diagrams and screenshots are invisible even in terminals that can display them. Adding inline image rendering with graceful fallback makes catmd output faithful to the document on capable terminals without breaking plain terminals or pipes.

## What Changes

- Add a protocol-independent image abstraction with backends for the Kitty Graphics Protocol (kittage) and SIXEL (icy_sixel), selected at runtime in a fixed order: Kitty, then SIXEL, then half-block cells, then a text placeholder.
- Represent images as first-class layout blocks that reserve rows and columns in the document flow.
- Scale images down proportionally so they never exceed the available content width; never scale up.
- Add a boxed placeholder (alt text, source, original dimensions, "unsupported" note) and a Unicode half-block fallback for terminals without a graphical protocol.
- Cache decoded images and per-backend encoded output; define invalidation when width, cell pixel size, or backend changes.
- Make rendering reflow-ready: layout is a pure function of width and cell geometry, so a later viewport can re-layout on resize without reloading images.
- Output stays plain text when stdout is not a terminal, preserving the existing `[image: alt] (src)` behavior.
- Out of scope (follow-up change `add-builtin-pager`): a native interactive pager with scrolling, search navigation, resize monitoring, and redraw/erase of images. Output through external `less` carries no graphical images.
- Out of scope: remote `http(s)` image fetching, animated images, user scaling configuration.

## Capabilities

### New Capabilities
- `inline-images`: image loading, backend selection and fallback, width-fit scaling, placeholder and half-block rendering, caching.

### Modified Capabilities
- `inline-rendering`: the "Images render as a labeled placeholder" requirement applies only when image rendering is disabled or output is not a terminal.

## Impact

- Code: new `src/image/` module (backend trait, kitty, sixel, halfblock, placeholder, cache); `src/render/` layout gains an image block; `src/env.rs` gains terminal capability detection and cell pixel size; `src/pager.rs` bypasses the external pager when graphical output is selected.
- Dependencies: `kittage`, `icy_sixel`, and an image decoder (`image`, limited features).
- Tests: fake backend for exact-output cases in `tests/render.rs`; piped CLI cases in `tests/cli.rs`.
- Docs: `man/catmd.1` for new options and environment variables.
