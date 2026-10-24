//! `catmd` — render Markdown as styled text for the terminal.
//!
//! The library holds everything that does not touch the process environment:
//! argument definitions, settings resolution from an [`Environment`] snapshot,
//! the renderer, and the paging decision. The binary in `main.rs` is a thin
//! wrapper that performs I/O.

pub mod cli;
pub mod env;
pub mod image;
pub mod input;
pub mod licenses;
pub mod pager;
pub mod render;
pub mod theme;

pub use env::{Environment, Settings, resolve};
pub use render::render;
pub use theme::{Glyphs, Theme};
