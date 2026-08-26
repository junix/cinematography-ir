//! SVG encoder: the reference rendering of a `Canvas`. PNG and HTML build on
//! it, so its output must be byte-stable (fixed-precision numbers, no
//! timestamps, sorted nothing).

use std::fmt::Write;

use crate::palette::Rgb;
use crate::view::canvas::{
    Anchor, ArrowKind, Canvas, CanvasItem, GlyphShape, LineStyle, Rect, Vec2,
};
use crate::view::layout::guide_to_lines;
use crate::view::panel::{severity_color, silhouette_parts};

pub const FONT_FAMILY: &str = "Liberation Sans, DejaVu Sans, Arial, Helvetica, sans-serif";

fn n(v: f32) -> String {
    let s = format!("{v:.2}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-0" {
        "0".to_owned()
    } else {
        s.to_owned()
    }
}

fn esc(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn dash(style: LineStyle) -> &'static str {
    match style {
        LineStyle::Solid => "",
        LineStyle::Dashed => r##" stroke-dasharray="6 4""##,
        LineStyle::Dotted => r##" stroke-dasharray="1.5 3""##,
    }
}

fn arrow_head(out: &mut String, from: Vec2, to: Vec2, color: Rgb, size: f32) {
    let dir = to.sub(from);
    let length = dir.length();
    if length < 1e-3 {
        return;
    }
    let unit = dir.scale(1.0 / length);
    let normal = Vec2::new(-unit.y, unit.x);
    let base = to.sub(unit.scale(size));
    let left = base.add(normal.scale(size * 0.5));
    let right = base.sub(normal.scale(size * 0.5));
    let _ = write!(
        out,
        r##"<polygon points="{},{} {},{} {},{}" fill="{}"/>"##,
        n(to.x),
        n(to.y),
        n(left.x),
        n(left.y),
        n(right.x),
        n(right.y),
        color.hex()
    );
}

fn text(
    out: &mut String,
    at: Vec2,
    content: &str,
    anchor: Anchor,
    size: f32,
    color: Rgb,
    bold: bool,
) {
    let anchor = match anchor {
        Anchor::Start => "start",
        Anchor::Middle => "middle",
        Anchor::End => "end",
    };
    let weight = if bold { r##" font-weight="bold""## } else { "" };
    let _ = write!(
        out,
        r##"<text x="{}" y="{}" font-family="{FONT_FAMILY}" font-size="{}" text-anchor="{anchor}" fill="{}"{weight}>{}</text>"##,
        n(at.x),
        n(at.y),
        n(size),
        color.hex(),
        esc(content)
    );
}

fn rect(
    out: &mut String,
    r: Rect,
    stroke: Option<Rgb>,
    fill: Option<Rgb>,
    width: f32,
    style: LineStyle,
    rx: f32,
) {
    let fill = fill.map_or("none".to_owned(), |c| c.hex());
    let stroke_attr = stroke.map_or(String::new(), |c| {
        format!(
            r##" stroke="{}" stroke-width="{}"{}"##,
            c.hex(),
            n(width),
            dash(style)
        )
    });
    let rx_attr = if rx > 0.0 {
        format!(r##" rx="{}""##, n(rx))
    } else {
        String::new()
    };
    let _ = write!(
        out,
        r##"<rect x="{}" y="{}" width="{}" height="{}" fill="{fill}"{stroke_attr}{rx_attr}/>"##,
        n(r.min.x),
        n(r.min.y),
        n(r.width()),
        n(r.height())
    );
}

fn item(out: &mut String, entry: &CanvasItem, canvas: &Canvas) {
    match entry {
        CanvasItem::Background { color } => {
            let _ = write!(
                out,
                r##"<rect x="0" y="0" width="{}" height="{}" fill="{}"/>"##,
                n(canvas.width),
                n(canvas.height),
                color.hex()
            );
        }
        CanvasItem::Glyph {
            shape,
            at,
            facing_deg,
            size,
            color,
            label,
        } => {
            let w = size.x.max(6.0);
            let d = size.y.max(6.0);
            let fill = color.hex();
            let _ = write!(
                out,
                r##"<g transform="translate({} {}) rotate({})">"##,
                n(at.x),
                n(at.y),
                n(*facing_deg)
            );
            // In the rotated group +x is the facing direction.
            match shape {
                GlyphShape::Character => {
                    let r = w.max(d) / 2.0;
                    let _ = write!(
                        out,
                        r##"<circle cx="0" cy="0" r="{}" fill="{fill}" stroke="#141418" stroke-width="1"/><polygon points="{},{} {},{} {},{}" fill="#141418"/>"##,
                        n(r),
                        n(r * 0.6),
                        n(-r * 0.45),
                        n(r * 1.35),
                        n(0.0),
                        n(r * 0.6),
                        n(r * 0.45)
                    );
                }
                GlyphShape::Vehicle => {
                    let _ = write!(
                        out,
                        r##"<rect x="{}" y="{}" width="{}" height="{}" rx="{}" fill="{fill}" stroke="#141418" stroke-width="1"/><polygon points="{},{} {},{} {},{}" fill="#141418"/>"##,
                        n(-d / 2.0),
                        n(-w / 2.0),
                        n(d),
                        n(w),
                        n(w * 0.2),
                        n(d * 0.5),
                        n(-w * 0.3),
                        n(d * 0.5 + w * 0.3),
                        n(0.0),
                        n(d * 0.5),
                        n(w * 0.3)
                    );
                }
                GlyphShape::Screen => {
                    let _ = write!(
                        out,
                        r##"<rect x="{}" y="{}" width="{}" height="{}" fill="{fill}" stroke="#141418" stroke-width="1"/>"##,
                        n(-d.max(3.0) / 2.0),
                        n(-w / 2.0),
                        n(d.max(3.0)),
                        n(w)
                    );
                }
                GlyphShape::Environment => {
                    let _ = write!(
                        out,
                        r##"<rect x="{}" y="{}" width="{}" height="{}" fill="none" stroke="{fill}" stroke-width="1.5" stroke-dasharray="4 3"/>"##,
                        n(-d / 2.0),
                        n(-w / 2.0),
                        n(d),
                        n(w)
                    );
                }
                GlyphShape::Abstract => {
                    let r = w.max(d) / 2.0;
                    let _ = write!(
                        out,
                        r##"<polygon points="{},0 0,{} {},0 0,{}" fill="{fill}" stroke="#141418" stroke-width="1"/>"##,
                        n(r),
                        n(r),
                        n(-r),
                        n(-r)
                    );
                }
                GlyphShape::Prop => {
                    let _ = write!(
                        out,
                        r##"<rect x="{}" y="{}" width="{}" height="{}" fill="{fill}" stroke="#141418" stroke-width="1"/>"##,
                        n(-d / 2.0),
                        n(-w / 2.0),
                        n(d),
                        n(w)
                    );
                }
            }
            out.push_str("</g>");
            if let Some(label) = label {
                text(
                    out,
                    Vec2::new(at.x, at.y + w.max(d) / 2.0 + 12.0),
                    label,
                    Anchor::Middle,
                    11.0,
                    crate::view::canvas::theme::INK,
                    false,
                );
            }
        }
        CanvasItem::Silhouette {
            shape,
            rect: r,
            color,
            label,
            label_at,
        } => {
            let (head, body) = silhouette_parts(*shape, *r);
            if let Some((c, radius)) = head {
                let _ = write!(
                    out,
                    r##"<circle cx="{}" cy="{}" r="{}" fill="{}"/>"##,
                    n(c.x),
                    n(c.y),
                    n(radius),
                    color.hex()
                );
            }
            let rx = if *shape == GlyphShape::Character {
                body.width() * 0.25
            } else {
                0.0
            };
            if *shape == GlyphShape::Environment {
                // An environment box would hide everything; outline it instead.
                rect(out, body, Some(*color), None, 1.5, LineStyle::Dashed, 0.0);
            } else {
                rect(out, body, None, Some(*color), 0.0, LineStyle::Solid, rx);
            }
            if let Some(label) = label {
                text(
                    out,
                    *label_at,
                    label,
                    Anchor::Middle,
                    10.0,
                    crate::view::canvas::theme::INK,
                    false,
                );
            }
        }
        CanvasItem::CameraWedge {
            at,
            facing_deg,
            hfov_deg,
            length,
            color,
            label,
        } => {
            let half = hfov_deg / 2.0;
            let a = at.add(Vec2::from_angle(facing_deg - half).scale(*length));
            let b = at.add(Vec2::from_angle(facing_deg + half).scale(*length));
            let _ = write!(
                out,
                r##"<polygon points="{},{} {},{} {},{}" fill="{}" fill-opacity="0.18" stroke="{}" stroke-width="1.2"/><circle cx="{}" cy="{}" r="4" fill="{}"/>"##,
                n(at.x),
                n(at.y),
                n(a.x),
                n(a.y),
                n(b.x),
                n(b.y),
                color.hex(),
                color.hex(),
                n(at.x),
                n(at.y),
                color.hex()
            );
            if let Some(label) = label {
                text(
                    out,
                    Vec2::new(at.x + 8.0, at.y + 14.0),
                    label,
                    Anchor::Start,
                    10.0,
                    *color,
                    true,
                );
            }
        }
        CanvasItem::Polyline {
            points,
            color,
            width,
            style,
            arrow_head: head,
            kind,
        } => {
            if points.len() < 2 {
                return;
            }
            let path: Vec<String> = points
                .iter()
                .map(|p| format!("{},{}", n(p.x), n(p.y)))
                .collect();
            let opacity = if *kind == ArrowKind::MotionCue && *width >= 3.0 {
                r##" stroke-opacity="0.85""##
            } else {
                ""
            };
            let _ = write!(
                out,
                r##"<polyline points="{}" fill="none" stroke="{}" stroke-width="{}" stroke-linecap="round" stroke-linejoin="round"{}{}/>"##,
                path.join(" "),
                color.hex(),
                n(*width),
                dash(*style),
                opacity
            );
            if *head {
                let from = points[points.len() - 2];
                let to = points[points.len() - 1];
                arrow_head(out, from, to, *color, (width * 3.0).max(6.0));
            }
        }
        CanvasItem::AxisLine {
            from,
            to,
            color,
            label,
        } => {
            let _ = write!(
                out,
                r##"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1.5" stroke-dasharray="8 4"/>"##,
                n(from.x),
                n(from.y),
                n(to.x),
                n(to.y),
                color.hex()
            );
            if let Some(label) = label {
                let mid = Vec2::new((from.x + to.x) / 2.0, (from.y + to.y) / 2.0 - 5.0);
                text(out, mid, label, Anchor::Middle, 9.0, *color, false);
            }
        }
        CanvasItem::Guide { kind, color } => {
            for line in guide_to_lines(*kind, canvas.bounds(), *color) {
                item(out, &line, canvas);
            }
        }
        CanvasItem::Rect {
            rect: r,
            stroke,
            fill,
            width,
            style,
        } => rect(out, *r, *stroke, *fill, *width, *style, 0.0),
        CanvasItem::Marker { at, color, label } => {
            let _ = write!(
                out,
                r##"<path d="M{} {}l4 -4l4 4l-4 4z" fill="none" stroke="{}" stroke-width="1"/>"##,
                n(at.x - 4.0),
                n(at.y),
                color.hex()
            );
            if let Some(label) = label {
                text(
                    out,
                    Vec2::new(at.x + 7.0, at.y + 3.0),
                    label,
                    Anchor::Start,
                    9.0,
                    *color,
                    false,
                );
            }
        }
        CanvasItem::Text {
            at,
            content,
            anchor,
            size,
            color,
            bold,
        } => text(out, *at, content, *anchor, *size, *color, *bold),
        CanvasItem::Diagnostic { at, code, severity } => {
            let color = severity_color(*severity);
            let _ = write!(
                out,
                r##"<polygon points="{},{} {},{} {},{}" fill="{}"/>"##,
                n(at.x),
                n(at.y - 8.0),
                n(at.x - 5.0),
                n(at.y),
                n(at.x + 5.0),
                n(at.y),
                color.hex()
            );
            text(
                out,
                Vec2::new(at.x + 8.0, at.y),
                code,
                Anchor::Start,
                9.0,
                color,
                true,
            );
        }
    }
}

/// Serialises a canvas as a standalone SVG document.
pub fn canvas_to_svg(canvas: &Canvas) -> String {
    let mut out = String::with_capacity(4096);
    let _ = write!(
        out,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">"##,
        n(canvas.width),
        n(canvas.height),
        n(canvas.width),
        n(canvas.height)
    );
    for i in &canvas.items {
        item(&mut out, i, canvas);
    }
    out.push_str("</svg>\n");
    out
}

/// The SVG element without the XML root attributes width/height, for inline
/// embedding in HTML where CSS controls the size.
pub fn canvas_to_inline_svg(canvas: &Canvas) -> String {
    let mut out = String::with_capacity(4096);
    let _ = write!(
        out,
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}" preserveAspectRatio="xMidYMid meet">"##,
        n(canvas.width),
        n(canvas.height)
    );
    for i in &canvas.items {
        item(&mut out, i, canvas);
    }
    out.push_str("</svg>");
    out
}
