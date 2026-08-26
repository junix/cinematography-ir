//! ASCII encoder: rasterises a `Canvas` onto a character grid. Terminal-safe
//! (plain ASCII), intended for humans and agents reading a plan at a glance.
//!
//! Drawing happens in three passes — lines, then symbols, then labels — so
//! captions and subject initials are never overwritten by geometry.

use crate::view::canvas::{
    Anchor, ArrowKind, Canvas, CanvasItem, GlyphShape, LineStyle, Rect, Vec2,
};
use crate::view::layout::guide_to_lines;
use crate::view::panel::silhouette_parts;

/// Terminal cells are roughly twice as tall as wide.
const CELL_ASPECT: f32 = 2.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Pass {
    Lines,
    Symbols,
    Labels,
}

struct Grid {
    cols: usize,
    rows: usize,
    width: f32,
    height: f32,
    cell_w: f32,
    cell_h: f32,
    cells: Vec<char>,
}

impl Grid {
    fn new(canvas: &Canvas, columns: usize) -> Grid {
        let cols = columns.max(8);
        let cell_w = canvas.width / cols as f32;
        let cell_h = cell_w * CELL_ASPECT;
        let rows = (canvas.height / cell_h).ceil().max(1.0) as usize;
        Grid {
            cols,
            rows,
            width: canvas.width,
            height: canvas.height,
            cell_w,
            cell_h,
            cells: vec![' '; cols * rows],
        }
    }

    /// Cell containing `p`; points on the right/bottom edge map to the last
    /// column/row so edge-touching geometry is not dropped.
    fn cell(&self, p: Vec2) -> (i64, i64) {
        let col = if p.x >= self.width - 1e-3 && p.x <= self.width + 1e-3 {
            self.cols as i64 - 1
        } else {
            (p.x / self.cell_w).floor() as i64
        };
        let row = if p.y >= self.height - 1e-3 && p.y <= self.height + 1e-3 {
            self.rows as i64 - 1
        } else {
            (p.y / self.cell_h).floor() as i64
        };
        (col, row)
    }

    fn put(&mut self, col: i64, row: i64, ch: char) {
        if col < 0 || row < 0 || col >= self.cols as i64 || row >= self.rows as i64 {
            return;
        }
        self.cells[row as usize * self.cols + col as usize] = ch;
    }

    fn line(&mut self, a: Vec2, b: Vec2, ch: Option<char>, style: LineStyle) {
        let (x0, y0) = self.cell(a);
        let (x1, y1) = self.cell(b);
        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();
        let auto = match ch {
            Some(c) => c,
            None => {
                if dx >= 2 * dy {
                    '-'
                } else if dy >= 2 * dx {
                    '|'
                } else if (x1 - x0) * (y1 - y0) > 0 {
                    '\\'
                } else {
                    '/'
                }
            }
        };
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx - dy;
        let (mut x, mut y) = (x0, y0);
        let mut step = 0usize;
        loop {
            let draw = match style {
                LineStyle::Solid => true,
                LineStyle::Dashed => step % 3 != 2,
                LineStyle::Dotted => step.is_multiple_of(2),
            };
            if draw {
                self.put(x, y, auto);
            }
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
            }
            step += 1;
        }
    }

    fn direction_char(dir: Vec2) -> char {
        if dir.x.abs() * CELL_ASPECT >= dir.y.abs() {
            if dir.x >= 0.0 {
                '>'
            } else {
                '<'
            }
        } else if dir.y >= 0.0 {
            'v'
        } else {
            '^'
        }
    }

    fn arrow_head(&mut self, from: Vec2, to: Vec2) {
        let ch = Grid::direction_char(to.sub(from));
        let (c, r) = self.cell(to);
        self.put(c, r, ch);
    }

    fn text(&mut self, at: Vec2, content: &str, anchor: Anchor) {
        let (c, r) = self.cell(at);
        let len = content.chars().count() as i64;
        let start = match anchor {
            Anchor::Start => c,
            Anchor::Middle => c - len / 2,
            Anchor::End => c - len + 1,
        };
        for (i, ch) in content.chars().enumerate() {
            let ch = match ch {
                c if c.is_ascii() => c,
                '·' | '–' | '—' | '−' => '-',
                '→' => '>',
                '←' => '<',
                '°' => 'o',
                _ => '?',
            };
            self.put(start + i as i64, r, ch);
        }
    }

    fn rect(&mut self, r: Rect, style: LineStyle) {
        let (c0, r0) = self.cell(r.min);
        let (c1, r1) = self.cell(r.max);
        if c1 <= c0 || r1 <= r0 {
            self.put(c0, r0, '#');
            return;
        }
        self.line(
            Vec2::new(r.min.x, r.min.y),
            Vec2::new(r.max.x, r.min.y),
            Some('-'),
            style,
        );
        self.line(
            Vec2::new(r.min.x, r.max.y),
            Vec2::new(r.max.x, r.max.y),
            Some('-'),
            style,
        );
        self.line(
            Vec2::new(r.min.x, r.min.y),
            Vec2::new(r.min.x, r.max.y),
            Some('|'),
            style,
        );
        self.line(
            Vec2::new(r.max.x, r.min.y),
            Vec2::new(r.max.x, r.max.y),
            Some('|'),
            style,
        );
        for (c, rr) in [(c0, r0), (c1, r0), (c0, r1), (c1, r1)] {
            self.put(c, rr, '+');
        }
    }

    fn fill(&mut self, r: Rect, ch: char) {
        let (c0, r0) = self.cell(r.min);
        let (c1, r1) = self.cell(r.max);
        for row in r0..=r1 {
            for col in c0..=c1 {
                self.put(col, row, ch);
            }
        }
    }

    fn render(&self) -> String {
        let mut out = String::with_capacity(self.cells.len() + self.rows);
        for row in 0..self.rows {
            let line: String = self.cells[row * self.cols..(row + 1) * self.cols]
                .iter()
                .collect();
            out.push_str(line.trim_end());
            out.push('\n');
        }
        out
    }
}

fn initial(label: &Option<String>, fallback: char) -> char {
    label
        .as_ref()
        .and_then(|l| l.chars().next())
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .unwrap_or(fallback)
}

pub fn canvas_to_ascii(canvas: &Canvas, columns: usize) -> String {
    let mut grid = Grid::new(canvas, columns);
    for pass in [Pass::Lines, Pass::Symbols, Pass::Labels] {
        for item in &canvas.items {
            draw(&mut grid, item, canvas, pass);
        }
    }
    grid.render()
}

fn draw(grid: &mut Grid, item: &CanvasItem, canvas: &Canvas, pass: Pass) {
    match item {
        CanvasItem::Background { .. } => {}
        CanvasItem::Glyph {
            shape,
            at,
            label,
            facing_deg,
            size,
            ..
        } => match pass {
            Pass::Symbols => {
                let ch = match shape {
                    GlyphShape::Character => initial(label, '@'),
                    GlyphShape::Prop => initial(label, '#'),
                    GlyphShape::Vehicle => initial(label, 'V'),
                    GlyphShape::Screen => initial(label, '='),
                    GlyphShape::Environment => initial(label, '~'),
                    GlyphShape::Abstract => initial(label, '*'),
                };
                let (c, r) = grid.cell(*at);
                grid.put(c, r, ch);
                // Facing tick one cell ahead when the glyph is big enough to read it.
                let reach = size.x.max(size.y) / 2.0 + grid.cell_w;
                let tip = at.add(Vec2::from_angle(*facing_deg).scale(reach));
                let (tc, tr) = grid.cell(tip);
                if (tc, tr) != (c, r) {
                    grid.put(tc, tr, Grid::direction_char(Vec2::from_angle(*facing_deg)));
                }
            }
            Pass::Labels => {
                if let Some(label) = label {
                    grid.text(
                        Vec2::new(at.x, at.y + size.y.max(size.x) / 2.0 + grid.cell_h),
                        label,
                        Anchor::Middle,
                    );
                }
            }
            Pass::Lines => {}
        },
        CanvasItem::Silhouette {
            shape,
            rect,
            label,
            label_at,
            ..
        } => match pass {
            Pass::Symbols => {
                let (head, body) = silhouette_parts(*shape, *rect);
                let ch = initial(
                    label,
                    if *shape == GlyphShape::Character {
                        '@'
                    } else {
                        '#'
                    },
                );
                if *shape == GlyphShape::Environment {
                    // An environment box would hide everything; outline it instead.
                    grid.rect(body, LineStyle::Dashed);
                } else {
                    grid.fill(body, ch.to_ascii_lowercase());
                }
                if let Some((c, _)) = head {
                    let (cc, cr) = grid.cell(c);
                    grid.put(cc, cr, 'O');
                }
            }
            Pass::Labels => {
                if let Some(label) = label {
                    grid.text(*label_at, label, Anchor::Middle);
                }
            }
            Pass::Lines => {}
        },
        CanvasItem::CameraWedge {
            at,
            facing_deg,
            hfov_deg,
            length,
            label,
            ..
        } => match pass {
            Pass::Symbols => {
                let half = hfov_deg / 2.0;
                let a = at.add(Vec2::from_angle(facing_deg - half).scale(*length));
                let b = at.add(Vec2::from_angle(facing_deg + half).scale(*length));
                grid.line(*at, a, Some('.'), LineStyle::Solid);
                grid.line(*at, b, Some('.'), LineStyle::Solid);
                let (c, r) = grid.cell(*at);
                grid.put(c, r, 'C');
            }
            Pass::Labels => {
                if let Some(label) = label {
                    grid.text(
                        Vec2::new(at.x + grid.cell_w * 2.0, at.y + grid.cell_h),
                        label,
                        Anchor::Start,
                    );
                }
            }
            Pass::Lines => {}
        },
        CanvasItem::Polyline {
            points,
            style,
            arrow_head,
            kind,
            width,
            ..
        } => {
            // Hairlines (the metre grid) would flood a character grid.
            if pass != Pass::Lines || points.len() < 2 || *width < 1.0 {
                return;
            }
            let ch = match kind {
                ArrowKind::Eyeline => Some(':'),
                ArrowKind::MotionCue if *style != LineStyle::Solid => Some('.'),
                _ => None,
            };
            for w in points.windows(2) {
                grid.line(w[0], w[1], ch, *style);
            }
            if *arrow_head {
                grid.arrow_head(points[points.len() - 2], points[points.len() - 1]);
            }
        }
        CanvasItem::AxisLine {
            from, to, label, ..
        } => match pass {
            Pass::Lines => grid.line(*from, *to, Some('='), LineStyle::Solid),
            Pass::Labels => {
                if let Some(label) = label {
                    let mid = Vec2::new((from.x + to.x) / 2.0, (from.y + to.y) / 2.0 - grid.cell_h);
                    grid.text(mid, label, Anchor::Middle);
                }
            }
            Pass::Symbols => {}
        },
        CanvasItem::Guide { kind, color } => {
            if pass == Pass::Lines {
                for line in guide_to_lines(*kind, canvas.bounds(), *color) {
                    draw(grid, &line, canvas, pass);
                }
            }
        }
        CanvasItem::Rect {
            rect,
            style,
            stroke,
            ..
        } => {
            // A fill-only rect (panel background) is invisible in ASCII.
            if pass == Pass::Lines && stroke.is_some() {
                grid.rect(*rect, *style);
            }
        }
        CanvasItem::Marker { at, label, .. } => match pass {
            Pass::Symbols => {
                let (c, r) = grid.cell(*at);
                grid.put(c, r, 'x');
            }
            Pass::Labels => {
                if let Some(label) = label {
                    grid.text(
                        Vec2::new(at.x + grid.cell_w * 1.5, at.y),
                        label,
                        Anchor::Start,
                    );
                }
            }
            Pass::Lines => {}
        },
        CanvasItem::Text {
            at,
            content,
            anchor,
            ..
        } => {
            if pass == Pass::Labels {
                grid.text(*at, content, *anchor);
            }
        }
        CanvasItem::Diagnostic { at, code, .. } => {
            if pass == Pass::Labels {
                grid.text(*at, &format!("! {code}"), Anchor::Start);
            }
        }
    }
}
