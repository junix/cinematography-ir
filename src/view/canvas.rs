//! Encoder-independent 2D canvas (ADR-1115 D5). Every view encoder (svg,
//! png, ascii, html) consumes this one structure, so they cannot drift.
//!
//! Coordinates are abstract units with the origin at the top-left and `y`
//! pointing down (SVG convention). Angles are degrees in that frame:
//! `0°` points to `+x` (right), `90°` points to `+y` (down).

use serde::Serialize;

use crate::diagnostic::Severity;
use crate::model::SubjectKind;
use crate::palette::Rgb;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Default)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const fn new(x: f32, y: f32) -> Vec2 {
        Vec2 { x, y }
    }

    pub fn from_angle(degrees: f32) -> Vec2 {
        let (s, c) = degrees.to_radians().sin_cos();
        Vec2 { x: c, y: s }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn add(self, o: Vec2) -> Vec2 {
        Vec2::new(self.x + o.x, self.y + o.y)
    }

    #[allow(clippy::should_implement_trait)]
    pub fn sub(self, o: Vec2) -> Vec2 {
        Vec2::new(self.x - o.x, self.y - o.y)
    }

    pub fn scale(self, s: f32) -> Vec2 {
        Vec2::new(self.x * s, self.y * s)
    }

    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn angle_deg(self) -> f32 {
        self.y.atan2(self.x).to_degrees()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Rect {
    pub min: Vec2,
    pub max: Vec2,
}

impl Rect {
    pub fn new(min: Vec2, max: Vec2) -> Rect {
        Rect {
            min: Vec2::new(min.x.min(max.x), min.y.min(max.y)),
            max: Vec2::new(min.x.max(max.x), min.y.max(max.y)),
        }
    }

    pub fn width(&self) -> f32 {
        self.max.x - self.min.x
    }

    pub fn height(&self) -> f32 {
        self.max.y - self.min.y
    }

    pub fn center(&self) -> Vec2 {
        Vec2::new(
            (self.min.x + self.max.x) / 2.0,
            (self.min.y + self.max.y) / 2.0,
        )
    }

    pub fn translate(&self, d: Vec2) -> Rect {
        Rect {
            min: self.min.add(d),
            max: self.max.add(d),
        }
    }

    pub fn intersect(&self, other: &Rect) -> Option<Rect> {
        let min = Vec2::new(self.min.x.max(other.min.x), self.min.y.max(other.min.y));
        let max = Vec2::new(self.max.x.min(other.max.x), self.max.y.min(other.max.y));
        (min.x < max.x && min.y < max.y).then_some(Rect { min, max })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GlyphShape {
    Character,
    Prop,
    Vehicle,
    Screen,
    Environment,
    Abstract,
}

impl From<SubjectKind> for GlyphShape {
    fn from(kind: SubjectKind) -> Self {
        match kind {
            SubjectKind::Character => GlyphShape::Character,
            SubjectKind::Prop => GlyphShape::Prop,
            SubjectKind::Vehicle => GlyphShape::Vehicle,
            SubjectKind::Environment => GlyphShape::Environment,
            SubjectKind::Screen => GlyphShape::Screen,
            SubjectKind::Abstract => GlyphShape::Abstract,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArrowKind {
    CameraPath,
    SubjectPath,
    ScreenDirection,
    Eyeline,
    /// Screen-space motion cue drawn on frame panels (dolly, pan, ...).
    MotionCue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LineStyle {
    Solid,
    Dashed,
    Dotted,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GuideKind {
    Thirds,
    Center,
    SafeArea,
    /// Horizontal line at `y` (canvas units) marking planned headroom.
    Headroom {
        y: f32,
    },
    /// Vertical line at `x` marking where lead room begins.
    LeadRoom {
        x: f32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Anchor {
    Start,
    Middle,
    End,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "item", rename_all = "snake_case")]
pub enum CanvasItem {
    Background {
        color: Rgb,
    },
    /// Top-down subject symbol on the plan panel.
    Glyph {
        shape: GlyphShape,
        at: Vec2,
        facing_deg: f32,
        /// Footprint (width, depth) in canvas units.
        size: Vec2,
        color: Rgb,
        label: Option<String>,
    },
    /// Camera-view subject silhouette on the frame panel.
    Silhouette {
        shape: GlyphShape,
        rect: Rect,
        color: Rgb,
        label: Option<String>,
        /// Where the label is drawn; chosen by the panel so it stays inside.
        label_at: Vec2,
    },
    CameraWedge {
        at: Vec2,
        facing_deg: f32,
        hfov_deg: f32,
        length: f32,
        color: Rgb,
        label: Option<String>,
    },
    Polyline {
        points: Vec<Vec2>,
        color: Rgb,
        width: f32,
        style: LineStyle,
        arrow_head: bool,
        kind: ArrowKind,
    },
    AxisLine {
        from: Vec2,
        to: Vec2,
        color: Rgb,
        label: Option<String>,
    },
    Guide {
        kind: GuideKind,
        color: Rgb,
    },
    Rect {
        rect: Rect,
        stroke: Option<Rgb>,
        fill: Option<Rgb>,
        width: f32,
        style: LineStyle,
    },
    Marker {
        at: Vec2,
        color: Rgb,
        label: Option<String>,
    },
    Text {
        at: Vec2,
        content: String,
        anchor: Anchor,
        size: f32,
        color: Rgb,
        bold: bool,
    },
    Diagnostic {
        at: Vec2,
        code: String,
        severity: Severity,
    },
}

impl CanvasItem {
    /// Whether the item renders any glyph text — the property the
    /// `Conditioning` intent must keep at zero except for allowed labels.
    pub fn carries_text(&self) -> bool {
        match self {
            CanvasItem::Text { .. } | CanvasItem::Diagnostic { .. } => true,
            CanvasItem::Glyph { label, .. }
            | CanvasItem::Silhouette { label, .. }
            | CanvasItem::CameraWedge { label, .. }
            | CanvasItem::AxisLine { label, .. }
            | CanvasItem::Marker { label, .. } => label.is_some(),
            _ => false,
        }
    }

    pub fn translated(&self, d: Vec2) -> CanvasItem {
        let mut item = self.clone();
        match &mut item {
            CanvasItem::Background { .. } | CanvasItem::Guide { .. } => {}
            CanvasItem::Glyph { at, .. }
            | CanvasItem::CameraWedge { at, .. }
            | CanvasItem::Marker { at, .. }
            | CanvasItem::Text { at, .. }
            | CanvasItem::Diagnostic { at, .. } => *at = at.add(d),
            CanvasItem::Silhouette { rect, label_at, .. } => {
                *rect = rect.translate(d);
                *label_at = label_at.add(d);
            }
            CanvasItem::Rect { rect, .. } => *rect = rect.translate(d),
            CanvasItem::Polyline { points, .. } => {
                for p in points.iter_mut() {
                    *p = p.add(d);
                }
            }
            CanvasItem::AxisLine { from, to, .. } => {
                *from = from.add(d);
                *to = to.add(d);
            }
        }
        item
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Canvas {
    pub width: f32,
    pub height: f32,
    pub items: Vec<CanvasItem>,
}

impl Canvas {
    pub fn new(width: f32, height: f32) -> Canvas {
        Canvas {
            width,
            height,
            items: Vec::new(),
        }
    }

    pub fn push(&mut self, item: CanvasItem) {
        self.items.push(item);
    }

    pub fn bounds(&self) -> Rect {
        Rect::new(Vec2::new(0.0, 0.0), Vec2::new(self.width, self.height))
    }

    /// Copies every item of `other` into this canvas offset by `d`. Guides
    /// are panel-relative and are not merged (they would lose their frame);
    /// a panel background becomes a bounded fill so it cannot paint over
    /// neighbouring panels.
    pub fn blit(&mut self, other: &Canvas, d: Vec2) {
        for item in &other.items {
            match item {
                CanvasItem::Guide { .. } => continue,
                CanvasItem::Background { color } => self.items.push(CanvasItem::Rect {
                    rect: other.bounds().translate(d),
                    stroke: None,
                    fill: Some(*color),
                    width: 0.0,
                    style: LineStyle::Solid,
                }),
                other_item => self.items.push(other_item.translated(d)),
            }
        }
    }

    pub fn text_items(&self) -> usize {
        self.items.iter().filter(|i| i.carries_text()).count()
    }

    pub fn count(&self, predicate: impl Fn(&CanvasItem) -> bool) -> usize {
        self.items.iter().filter(|i| predicate(i)).count()
    }
}

/// Colours shared by encoders so every format reads the same.
pub mod theme {
    use crate::palette::Rgb;

    pub const WHITE: Rgb = Rgb {
        r: 255,
        g: 255,
        b: 255,
    };
    pub const INK: Rgb = Rgb {
        r: 20,
        g: 20,
        b: 24,
    };
    pub const CAMERA: Rgb = Rgb {
        r: 30,
        g: 30,
        b: 36,
    };
    pub const GRID: Rgb = Rgb {
        r: 228,
        g: 228,
        b: 232,
    };
    pub const GUIDE: Rgb = Rgb {
        r: 170,
        g: 170,
        b: 178,
    };
    pub const AXIS: Rgb = Rgb {
        r: 214,
        g: 40,
        b: 40,
    };
    pub const EYELINE: Rgb = Rgb {
        r: 40,
        g: 90,
        b: 200,
    };
    pub const MARKER: Rgb = Rgb {
        r: 120,
        g: 120,
        b: 128,
    };
    pub const WARNING: Rgb = Rgb {
        r: 220,
        g: 130,
        b: 20,
    };
    pub const ERROR: Rgb = Rgb {
        r: 200,
        g: 30,
        b: 30,
    };
    pub const NEUTRAL_SUBJECT: Rgb = Rgb {
        r: 90,
        g: 90,
        b: 96,
    };
}
