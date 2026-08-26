//! PNG encoder: rasterises the SVG rendering with `resvg` using only the
//! embedded font, so output is byte-identical across machines.

use std::sync::Arc;

use crate::view::canvas::Canvas;
use crate::view::encode::svg::canvas_to_svg;
use crate::view::ViewError;

/// Liberation Sans (SIL OFL 1.1), see `assets/fonts/LICENSE-LiberationSans.txt`.
const FONT: &[u8] = include_bytes!("../../../assets/fonts/LiberationSans-Regular.ttf");

pub fn canvas_to_png(canvas: &Canvas, scale: f32) -> Result<Vec<u8>, ViewError> {
    let svg = canvas_to_svg(canvas);
    let mut options = usvg::Options::default();
    let mut fontdb = usvg::fontdb::Database::new();
    fontdb.load_font_data(FONT.to_vec());
    options.fontdb = Arc::new(fontdb);
    options.font_family = "Liberation Sans".to_owned();
    let tree = usvg::Tree::from_str(&svg, &options)
        .map_err(|e| ViewError::Encode(format!("svg parse failed: {e}")))?;
    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };
    let width = (canvas.width * scale).round().max(1.0) as u32;
    let height = (canvas.height * scale).round().max(1.0) as u32;
    let mut pixmap = tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| ViewError::Encode(format!("cannot allocate {width}x{height} pixmap")))?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap
        .encode_png()
        .map_err(|e| ViewError::Encode(format!("png encode failed: {e}")))
}
