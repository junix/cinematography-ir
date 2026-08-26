//! Generates the self-contained `bpy` script. The Python side is deliberately
//! convention-free: every location and basis arrives already in Blender
//! world space, so the script only builds objects, keyframes, view layers,
//! and the compositor.

use crate::execute::blender::convert::BlenderPayload;

const TEMPLATE: &str = include_str!("template.py");

/// Escapes JSON for embedding inside a single-quoted Python string literal.
fn python_single_quoted(json: &str) -> String {
    json.replace('\\', "\\\\").replace('\'', "\\'")
}

pub fn render_script(payload: &BlenderPayload) -> String {
    let json = serde_json::to_string(payload).expect("payload serializes");
    TEMPLATE.replace("__PAYLOAD_JSON__", &python_single_quoted(&json))
}
