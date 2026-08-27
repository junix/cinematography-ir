//! Semantic diagram IR lowered to the shared low-level canvas. The semantic
//! layer prevents panel code and encoders from independently reinterpreting
//! compiled camera programs.

use serde::Serialize;

use crate::compiled::{CompiledPhase, CompiledShot, ConstraintStatus, PhaseKind};
use crate::palette::Rgb;

use super::canvas::{theme, Anchor, ArrowKind, Canvas, CanvasItem, LineStyle, Rect, Vec2};

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "diagram", rename_all = "snake_case")]
pub enum DiagramItem {
    CameraTrack(CameraTrackDiagram),
    ScreenTrack(ScreenTrackDiagram),
    OpticalFlowField(FlowDiagram),
    FocusTransition(FocusDiagram),
    RevealProgress(RevealDiagram),
    ConstraintBand(ConstraintDiagram),
    PhaseTimeline(PhaseTimeline),
    CutRelation(CutDiagram),
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CameraTrackDiagram {
    pub points: Vec<Vec2>,
    pub focal_values: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ScreenTrackDiagram {
    pub subject_id: String,
    pub centers: Vec<Vec2>,
    pub bbox_heights: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct FlowDiagram {
    pub vectors: Vec<(Vec2, Vec2)>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct FocusDiagram {
    pub distances_m: Vec<Option<f32>>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RevealDiagram {
    pub subject_id: String,
    pub visible_fraction: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ConstraintDiagram {
    pub id: String,
    pub status: ConstraintStatus,
    pub requested: String,
    pub actual: String,
    pub max_error: Option<f32>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PhaseTimeline {
    pub phases: Vec<CompiledPhase>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CutDiagram {
    pub previous_shot_id: Option<String>,
    pub next_shot_id: Option<String>,
}

pub fn shot_diagrams(shot: &CompiledShot) -> Vec<DiagramItem> {
    let mut items = vec![
        DiagramItem::PhaseTimeline(PhaseTimeline {
            phases: shot.phases.clone(),
        }),
        DiagramItem::CameraTrack(CameraTrackDiagram {
            points: shot
                .camera_track
                .frames
                .iter()
                .map(|frame| Vec2::new(frame.position.x, frame.position.z))
                .collect(),
            focal_values: shot
                .lens_track
                .frames
                .iter()
                .map(|frame| frame.focal_length_mm)
                .collect(),
        }),
        DiagramItem::FocusTransition(FocusDiagram {
            distances_m: shot
                .focus_track
                .frames
                .iter()
                .map(|frame| frame.distance_m)
                .collect(),
        }),
        DiagramItem::CutRelation(CutDiagram {
            previous_shot_id: shot.edit_relation.previous_shot_id.clone(),
            next_shot_id: shot.edit_relation.next_shot_id.clone(),
        }),
    ];
    for track in &shot.screen_tracks {
        items.push(DiagramItem::ScreenTrack(ScreenTrackDiagram {
            subject_id: track.subject_id.clone(),
            centers: track
                .frames
                .iter()
                .map(|frame| Vec2::new(frame.center.x, frame.center.y))
                .collect(),
            bbox_heights: track
                .frames
                .iter()
                .map(|frame| frame.bbox.height())
                .collect(),
        }));
        if track
            .frames
            .windows(2)
            .any(|frames| (frames[1].visible_fraction - frames[0].visible_fraction).abs() > 1e-4)
        {
            items.push(DiagramItem::RevealProgress(RevealDiagram {
                subject_id: track.subject_id.clone(),
                visible_fraction: track
                    .frames
                    .iter()
                    .map(|frame| frame.visible_fraction)
                    .collect(),
            }));
        }
    }
    items.extend(shot.evaluations.iter().map(|evaluation| {
        DiagramItem::ConstraintBand(ConstraintDiagram {
            id: evaluation.constraint_id.clone(),
            status: evaluation.status,
            requested: evaluation.requested.clone(),
            actual: evaluation.actual.clone(),
            max_error: evaluation.max_error,
        })
    }));
    items
}

/// Compact timeline and observable-metric panel used in storyboard cards.
pub fn lower_shot_diagnostics(shot: &CompiledShot, width: f32) -> Canvas {
    let height = 180.0;
    let mut canvas = Canvas::new(width, height);
    canvas.push(CanvasItem::Background {
        color: theme::WHITE,
    });
    let range = shot.edit_range;
    let duration = range.duration().max(1) as f32;
    let x =
        |frame: u64| 12.0 + (frame.saturating_sub(range.start) as f32 / duration) * (width - 24.0);

    canvas.push(CanvasItem::Text {
        at: Vec2::new(12.0, 15.0),
        content: format!("{} · phases / lens / screen constraints", shot.id),
        anchor: Anchor::Start,
        size: 11.0,
        color: theme::INK,
        bold: true,
    });
    for phase in &shot.phases {
        let color = phase_color(phase.kind);
        canvas.push(CanvasItem::Rect {
            rect: Rect::new(
                Vec2::new(x(phase.edit_range.start), 26.0),
                Vec2::new(x(phase.edit_range.end), 48.0),
            ),
            stroke: Some(color),
            fill: Some(color),
            width: 1.0,
            style: LineStyle::Solid,
        });
        canvas.push(CanvasItem::Text {
            at: Vec2::new(
                (x(phase.edit_range.start) + x(phase.edit_range.end)) * 0.5,
                41.0,
            ),
            content: phase_label(phase.kind).to_owned(),
            anchor: Anchor::Middle,
            size: 8.0,
            color: theme::WHITE,
            bold: false,
        });
    }

    plot_series(
        &mut canvas,
        &shot
            .lens_track
            .frames
            .iter()
            .map(|frame| (frame.frame, frame.focal_length_mm))
            .collect::<Vec<_>>(),
        range,
        Rect::new(Vec2::new(12.0, 60.0), Vec2::new(width - 12.0, 98.0)),
        theme::CAMERA,
    );
    canvas.push(CanvasItem::Text {
        at: Vec2::new(14.0, 70.0),
        content: "focal".to_owned(),
        anchor: Anchor::Start,
        size: 8.0,
        color: theme::CAMERA,
        bold: false,
    });

    if let Some(track) = shot
        .screen_tracks
        .iter()
        .find(|track| shot.intent.framing.subject_ids.contains(&track.subject_id))
        .or_else(|| shot.screen_tracks.first())
    {
        plot_series(
            &mut canvas,
            &track
                .frames
                .iter()
                .map(|frame| (frame.frame, frame.bbox.height()))
                .collect::<Vec<_>>(),
            range,
            Rect::new(Vec2::new(12.0, 104.0), Vec2::new(width - 12.0, 142.0)),
            theme::EYELINE,
        );
        canvas.push(CanvasItem::Text {
            at: Vec2::new(14.0, 114.0),
            content: format!("{} bbox", track.subject_id),
            anchor: Anchor::Start,
            size: 8.0,
            color: theme::EYELINE,
            bold: false,
        });
    }

    let passed = shot
        .evaluations
        .iter()
        .filter(|evaluation| evaluation.status == ConstraintStatus::Pass)
        .count();
    let failed = shot
        .evaluations
        .iter()
        .filter(|evaluation| evaluation.status == ConstraintStatus::Fail)
        .count();
    let indeterminate = shot.evaluations.len().saturating_sub(passed + failed);
    canvas.push(CanvasItem::Text {
        at: Vec2::new(12.0, 165.0),
        content: format!("constraints PASS {passed} · FAIL {failed} · ? {indeterminate}"),
        anchor: Anchor::Start,
        size: 9.0,
        color: if failed == 0 { theme::INK } else { theme::AXIS },
        bold: failed > 0,
    });
    canvas.resolve_label_collisions();
    canvas
}

/// Side elevation of camera height over edit time.
pub fn lower_elevation(shot: &CompiledShot, width: f32) -> Canvas {
    let mut canvas = Canvas::new(width, 120.0);
    canvas.push(CanvasItem::Background {
        color: theme::WHITE,
    });
    let values: Vec<_> = shot
        .camera_track
        .frames
        .iter()
        .map(|frame| (frame.frame, frame.position.y))
        .collect();
    plot_series(
        &mut canvas,
        &values,
        shot.edit_range,
        Rect::new(Vec2::new(12.0, 20.0), Vec2::new(width - 12.0, 108.0)),
        theme::CAMERA,
    );
    canvas.push(CanvasItem::Text {
        at: Vec2::new(12.0, 14.0),
        content: "side elevation · camera height".to_owned(),
        anchor: Anchor::Start,
        size: 9.0,
        color: theme::INK,
        bold: true,
    });
    canvas.resolve_label_collisions();
    canvas
}

fn plot_series(
    canvas: &mut Canvas,
    values: &[(u64, f32)],
    range: crate::model::FrameRange,
    rect: Rect,
    color: Rgb,
) {
    if values.is_empty() {
        return;
    }
    let min = values
        .iter()
        .map(|(_, value)| *value)
        .fold(f32::INFINITY, f32::min);
    let max = values
        .iter()
        .map(|(_, value)| *value)
        .fold(f32::NEG_INFINITY, f32::max);
    let spread = (max - min).max(1e-6);
    let duration = range.duration().max(1) as f32;
    let points = values
        .iter()
        .map(|(frame, value)| {
            Vec2::new(
                rect.min.x + frame.saturating_sub(range.start) as f32 / duration * rect.width(),
                rect.max.y - (*value - min) / spread * rect.height(),
            )
        })
        .collect();
    canvas.push(CanvasItem::Polyline {
        points,
        color,
        width: 1.5,
        style: LineStyle::Solid,
        arrow_head: false,
        kind: ArrowKind::MotionCue,
    });
}

fn phase_label(kind: PhaseKind) -> &'static str {
    match kind {
        PhaseKind::Hold => "hold",
        PhaseKind::Motion => "motion",
        PhaseKind::Focus => "focus",
        PhaseKind::Reveal => "reveal",
        PhaseKind::Settle => "settle",
        PhaseKind::Mixed => "mixed",
    }
}

fn phase_color(kind: PhaseKind) -> Rgb {
    match kind {
        PhaseKind::Hold | PhaseKind::Settle => theme::MARKER,
        PhaseKind::Motion => theme::CAMERA,
        PhaseKind::Focus => theme::EYELINE,
        PhaseKind::Reveal => Rgb {
            r: 120,
            g: 84,
            b: 190,
        },
        PhaseKind::Mixed => theme::AXIS,
    }
}
