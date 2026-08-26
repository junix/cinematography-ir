//! Encoders (ADR-1115 D4 stage 5): one `Canvas`, several byte formats.

pub mod ascii;
pub mod html;
pub mod png;
pub mod svg;

pub use ascii::canvas_to_ascii;
pub use html::{animation_page, still_page};
pub use png::canvas_to_png;
pub use svg::{canvas_to_inline_svg, canvas_to_svg};
