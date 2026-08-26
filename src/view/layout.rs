//! Layout (ADR-1115 D4 stage 4): plan + frame panels → cells → sheets.
//! Grid, strip, and animation frames all consume the same cells.

use crate::view::canvas::{theme, Anchor, Canvas, CanvasItem, LineStyle, Rect, Vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StripAxis {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Layout {
    /// All cells of a scene in one sheet with `columns` per row.
    Grid {
        columns: usize,
    },
    Strip {
        axis: StripAxis,
    },
    /// One artifact per cell.
    Separate,
    /// One HTML artifact per scene playing the cells in order.
    Animate,
}

impl Default for Layout {
    fn default() -> Self {
        Layout::Grid { columns: 2 }
    }
}

pub const CELL_GAP: f32 = 16.0;
pub const TITLE_HEIGHT: f32 = 22.0;

/// A sampled moment rendered as up to two panels plus a caption.
#[derive(Debug, Clone)]
pub struct Cell {
    pub title: String,
    pub plan: Option<Canvas>,
    pub frame: Option<Canvas>,
}

impl Cell {
    /// Composes the panels side by side. `human` adds a title bar and
    /// panel borders; conditioning output gets bare panels.
    pub fn compose(&self, human: bool) -> Canvas {
        let panels: Vec<&Canvas> = [self.plan.as_ref(), self.frame.as_ref()]
            .into_iter()
            .flatten()
            .collect();
        let width: f32 = panels.iter().map(|p| p.width).sum::<f32>()
            + CELL_GAP * panels.len().saturating_sub(1) as f32;
        let height = panels.iter().map(|p| p.height).fold(0.0, f32::max);
        let title_height = if human { TITLE_HEIGHT } else { 0.0 };
        let mut canvas = Canvas::new(width.max(1.0), height + title_height);
        canvas.push(CanvasItem::Background {
            color: theme::WHITE,
        });
        if human {
            canvas.push(CanvasItem::Text {
                at: Vec2::new(4.0, 15.0),
                content: self.title.clone(),
                anchor: Anchor::Start,
                size: 12.0,
                color: theme::INK,
                bold: true,
            });
        }
        let mut x = 0.0;
        for panel in panels {
            let offset = Vec2::new(x, title_height);
            canvas.blit(panel, offset);
            // Guides are panel-relative: re-emit them as absolute rectangles/lines.
            for item in &panel.items {
                if let CanvasItem::Guide { kind, color } = item {
                    let rect = Rect::new(offset, offset.add(Vec2::new(panel.width, panel.height)));
                    canvas.items.extend(guide_to_lines(*kind, rect, *color));
                }
            }
            if human {
                canvas.push(CanvasItem::Rect {
                    rect: Rect::new(offset, offset.add(Vec2::new(panel.width, panel.height))),
                    stroke: Some(theme::GUIDE),
                    fill: None,
                    width: 1.0,
                    style: LineStyle::Solid,
                });
            }
            x += panel.width + CELL_GAP;
        }
        canvas
    }
}

/// Expands a panel-relative guide into absolute canvas lines.
pub fn guide_to_lines(
    kind: crate::view::canvas::GuideKind,
    rect: Rect,
    color: crate::palette::Rgb,
) -> Vec<CanvasItem> {
    use crate::view::canvas::{ArrowKind, GuideKind};
    let line = |a: Vec2, b: Vec2, style: LineStyle| CanvasItem::Polyline {
        points: vec![a, b],
        color,
        width: 1.0,
        style,
        arrow_head: false,
        kind: ArrowKind::MotionCue,
    };
    let w = rect.width();
    let h = rect.height();
    let o = rect.min;
    match kind {
        GuideKind::Thirds => vec![
            line(
                o.add(Vec2::new(w / 3.0, 0.0)),
                o.add(Vec2::new(w / 3.0, h)),
                LineStyle::Dotted,
            ),
            line(
                o.add(Vec2::new(2.0 * w / 3.0, 0.0)),
                o.add(Vec2::new(2.0 * w / 3.0, h)),
                LineStyle::Dotted,
            ),
            line(
                o.add(Vec2::new(0.0, h / 3.0)),
                o.add(Vec2::new(w, h / 3.0)),
                LineStyle::Dotted,
            ),
            line(
                o.add(Vec2::new(0.0, 2.0 * h / 3.0)),
                o.add(Vec2::new(w, 2.0 * h / 3.0)),
                LineStyle::Dotted,
            ),
        ],
        GuideKind::Center => vec![
            line(
                o.add(Vec2::new(w / 2.0, h / 2.0 - 8.0)),
                o.add(Vec2::new(w / 2.0, h / 2.0 + 8.0)),
                LineStyle::Solid,
            ),
            line(
                o.add(Vec2::new(w / 2.0 - 8.0, h / 2.0)),
                o.add(Vec2::new(w / 2.0 + 8.0, h / 2.0)),
                LineStyle::Solid,
            ),
        ],
        GuideKind::SafeArea => vec![CanvasItem::Rect {
            rect: Rect::new(
                o.add(Vec2::new(w * 0.05, h * 0.05)),
                o.add(Vec2::new(w * 0.95, h * 0.95)),
            ),
            stroke: Some(color),
            fill: None,
            width: 1.0,
            style: LineStyle::Dashed,
        }],
        GuideKind::Headroom { y } => vec![line(
            o.add(Vec2::new(0.0, y)),
            o.add(Vec2::new(w, y)),
            LineStyle::Dashed,
        )],
        GuideKind::LeadRoom { x } => vec![line(
            o.add(Vec2::new(x, 0.0)),
            o.add(Vec2::new(x, h)),
            LineStyle::Dashed,
        )],
    }
}

/// Arranges composed cells into a sheet with `columns` per row.
pub fn grid(cells: &[Canvas], columns: usize, human: bool) -> Canvas {
    let columns = columns.max(1);
    if cells.is_empty() {
        return Canvas::new(1.0, 1.0);
    }
    let cell_w = cells.iter().map(|c| c.width).fold(0.0, f32::max);
    let cell_h = cells.iter().map(|c| c.height).fold(0.0, f32::max);
    let rows = cells.len().div_ceil(columns);
    let cols = cells.len().min(columns);
    let width = cols as f32 * cell_w + (cols as f32 - 1.0) * CELL_GAP;
    let height = rows as f32 * cell_h + (rows as f32 - 1.0) * CELL_GAP;
    let mut sheet = Canvas::new(width, height);
    sheet.push(CanvasItem::Background {
        color: theme::WHITE,
    });
    for (index, cell) in cells.iter().enumerate() {
        let col = (index % columns) as f32;
        let row = (index / columns) as f32;
        let offset = Vec2::new(col * (cell_w + CELL_GAP), row * (cell_h + CELL_GAP));
        sheet.blit(cell, offset);
        if !human {
            // Thin separators keep grid cells distinguishable without text.
            sheet.push(CanvasItem::Rect {
                rect: Rect::new(offset, offset.add(Vec2::new(cell.width, cell.height))),
                stroke: Some(theme::GRID),
                fill: None,
                width: 1.0,
                style: LineStyle::Solid,
            });
        }
    }
    sheet
}

pub fn strip(cells: &[Canvas], axis: StripAxis, human: bool) -> Canvas {
    match axis {
        StripAxis::Horizontal => grid(cells, cells.len().max(1), human),
        StripAxis::Vertical => grid(cells, 1, human),
    }
}
