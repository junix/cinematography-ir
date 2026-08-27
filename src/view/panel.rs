//! Panel builders (ADR-1115 D4 stages 2–3): the plan (top-down) and frame
//! (through-the-lens) canvases for one sampled moment, with overlays gated by
//! `RenderIntent` and `LabelMode` (D7/D7a).

use crate::diagnostic::{Diagnostic, Severity};
use crate::math::{identity_basis, oriented_basis, Vec3};
use crate::model::{
    CameraOperation, CineProject, Composition, CoordinateSystem, Frame, HorizontalDirection, Scene,
    ScreenDirection, ScreenRegion, Shot, ShotSize, TargetRef, TimedCameraOperation, TransformSpace,
};
use crate::palette::Rgb;
use crate::solve::aim_height_fraction;
use crate::solve::{SolvedCameraFrame, SolvedScene, SolvedShot, SolvedSubjectTrack};
use crate::view::canvas::{
    theme, Anchor, ArrowKind, Canvas, CanvasItem, GlyphShape, GuideKind, LineStyle, Rect, Vec2,
};
use crate::view::project::{FrameProjection, PlanProjection};
use crate::view::sample::SamplePoint;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderIntent {
    /// Review output: labels, guides, axes, diagnostics.
    Human,
    /// Model conditioning: geometry and motion arrows only (D7).
    Conditioning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelMode {
    /// Neutral colours, no names.
    None,
    /// Palette colours, no names (default for conditioning).
    Color,
    /// Palette colours and subject names (default for human review).
    ColorName,
}

/// Which overlays a panel may draw. `Conditioning` intent narrows this
/// further regardless of what is requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverlaySet {
    pub arrows: bool,
    pub subject_paths: bool,
    pub axes: bool,
    pub eyelines: bool,
    pub screen_directions: bool,
    pub guides: bool,
    pub diagnostics: bool,
    pub grid: bool,
    pub markers: bool,
}

impl OverlaySet {
    pub const ALL: OverlaySet = OverlaySet {
        arrows: true,
        subject_paths: true,
        axes: true,
        eyelines: true,
        screen_directions: true,
        guides: true,
        diagnostics: true,
        grid: true,
        markers: true,
    };

    pub const NONE: OverlaySet = OverlaySet {
        arrows: false,
        subject_paths: false,
        axes: false,
        eyelines: false,
        screen_directions: false,
        guides: false,
        diagnostics: false,
        grid: false,
        markers: false,
    };

    /// Overlays a conditioning image may carry: motion arrows only.
    pub const CONDITIONING: OverlaySet = OverlaySet {
        arrows: true,
        subject_paths: true,
        ..OverlaySet::NONE
    };

    /// Effective overlays after intent gating.
    pub fn gated(self, intent: RenderIntent) -> OverlaySet {
        match intent {
            RenderIntent::Human => self,
            RenderIntent::Conditioning => OverlaySet {
                arrows: self.arrows,
                subject_paths: self.subject_paths,
                ..OverlaySet::NONE
            },
        }
    }

    /// Parses `arrows,axes,...`; `all` and `none` are accepted.
    pub fn parse_list(text: &str) -> Result<OverlaySet, String> {
        let mut set = OverlaySet::NONE;
        for item in text.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            match item {
                "all" => set = OverlaySet::ALL,
                "none" => set = OverlaySet::NONE,
                "arrows" => set.arrows = true,
                "subject_paths" | "paths" => set.subject_paths = true,
                "axes" => set.axes = true,
                "eyelines" => set.eyelines = true,
                "screen_directions" | "directions" => set.screen_directions = true,
                "guides" => set.guides = true,
                "diagnostics" => set.diagnostics = true,
                "grid" => set.grid = true,
                "markers" => set.markers = true,
                other => return Err(format!("unknown overlay '{other}'")),
            }
        }
        Ok(set)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PanelStyle {
    pub intent: RenderIntent,
    pub labels: LabelMode,
    pub overlays: OverlaySet,
    /// Sensor aspect for the frame panel (width / height).
    pub aspect: f32,
}

impl PanelStyle {
    pub fn human(&self) -> bool {
        self.intent == RenderIntent::Human
    }

    pub fn subject_color(&self, track: &SolvedSubjectTrack) -> Rgb {
        match self.labels {
            LabelMode::None => theme::NEUTRAL_SUBJECT,
            _ => track.color,
        }
    }

    pub fn subject_label(&self, track: &SolvedSubjectTrack) -> Option<String> {
        match self.labels {
            LabelMode::ColorName => Some(track.name.clone()),
            _ => None,
        }
    }
}

/// Everything a panel needs about its scene.
pub struct PanelContext<'a> {
    pub project: &'a CineProject,
    pub scene: &'a Scene,
    pub solved: &'a SolvedScene,
    pub diagnostics: &'a [Diagnostic],
}

impl PanelContext<'_> {
    pub fn cs(&self) -> &CoordinateSystem {
        &self.project.coordinate_system
    }

    pub fn shot(&self, id: &str) -> Option<(&Shot, &SolvedShot)> {
        let shot = self.scene.shots.iter().find(|s| s.id == id)?;
        let solved = self.solved.shot(id)?;
        Some((shot, solved))
    }

    /// Base (ground) position of a target at `frame`.
    fn base_position(&self, target: &TargetRef, frame: Frame) -> Option<Vec3> {
        match target {
            TargetRef::Subject { id } => self
                .solved
                .subject(id)
                .map(|t| t.transform_at(frame).position),
            TargetRef::Marker { id } => self
                .scene
                .markers
                .iter()
                .find(|m| &m.id == id)
                .map(|m| m.position),
            TargetRef::Point { position } => Some(*position),
        }
    }

    /// The point the solver aims at (eye height for characters), so motion
    /// cues agree with the solved camera.
    fn aim_position(&self, target: &TargetRef, frame: Frame) -> Option<Vec3> {
        match target {
            TargetRef::Subject { id } => {
                let track = self.solved.subject(id)?;
                let transform = track.transform_at(frame);
                let up = identity_basis(self.cs()).up;
                Some(
                    transform.position
                        + up * (track.dimensions_m.y
                            * aim_height_fraction(track.kind)
                            * transform.scale.y),
                )
            }
            other => self.base_position(other, frame),
        }
    }

    /// Diagnostics attached to `shot_id` — a shot's own path, or the second
    /// shot of a `a->b` continuity pair.
    pub fn diagnostics_for(&self, shot_id: &str) -> Vec<&Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| diagnostic_targets_shot(&d.path, &self.scene.id, shot_id))
            .collect()
    }

    /// World points that define the plan extent: every subject and camera
    /// position over the whole scene, markers, and axis endpoints.
    pub fn plan_points(&self) -> Vec<Vec3> {
        let mut points = Vec::new();
        for track in &self.solved.subjects {
            points.extend(track.transforms.iter().step_by(4).map(|t| t.position));
            if let Some(last) = track.transforms.last() {
                points.push(last.position);
            }
        }
        for shot in &self.solved.shots {
            points.extend(shot.frames.iter().step_by(4).map(|f| f.position));
            if let Some(last) = shot.frames.last() {
                points.push(last.position);
            }
        }
        points.extend(self.scene.markers.iter().map(|m| m.position));
        points
    }
}

pub fn diagnostic_targets_shot(path: &str, scene_id: &str, shot_id: &str) -> bool {
    if !path.contains(&format!("scenes[{scene_id}]")) {
        return false;
    }
    let Some(start) = path.find("shots[") else {
        return false;
    };
    let rest = &path[start + "shots[".len()..];
    let Some(end) = rest.find(']') else {
        return false;
    };
    let inside = &rest[..end];
    match inside.split_once("->") {
        Some((_, next)) => next == shot_id,
        None => inside == shot_id,
    }
}

pub const PLAN_SIZE: f32 = 400.0;
pub const FRAME_WIDTH: f32 = 400.0;

/// Fits one plan projection for the whole scene so every panel (grid cell or
/// animation frame) shares the same scale and origin.
pub fn scene_plan(ctx: &PanelContext<'_>, size: f32) -> PlanProjection {
    PlanProjection::fit(ctx.cs(), &ctx.plan_points(), size, size, 0.12, 4.0)
}

fn polyline(
    points: Vec<Vec2>,
    color: Rgb,
    width: f32,
    style: LineStyle,
    kind: ArrowKind,
) -> CanvasItem {
    CanvasItem::Polyline {
        points,
        color,
        width,
        style,
        arrow_head: true,
        kind,
    }
}

fn path_length(points: &[Vec2]) -> f32 {
    points.windows(2).map(|w| w[1].sub(w[0]).length()).sum()
}

fn sampled_path<T>(items: &[T], step: usize, at: impl Fn(&T) -> Vec2) -> Vec<Vec2> {
    let mut points: Vec<Vec2> = items.iter().step_by(step.max(1)).map(&at).collect();
    if let Some(last) = items.last() {
        let p = at(last);
        if points.last() != Some(&p) {
            points.push(p);
        }
    }
    points
}

fn short_shot_size(size: ShotSize) -> &'static str {
    match size {
        ShotSize::ExtremeLong => "ELS",
        ShotSize::Long => "LS",
        ShotSize::MediumLong => "MLS",
        ShotSize::Medium => "MS",
        ShotSize::MediumCloseUp => "MCU",
        ShotSize::CloseUp => "CU",
        ShotSize::ExtremeCloseUp => "ECU",
        ShotSize::Insert => "INS",
        ShotSize::Custom => "",
    }
}

/// Top-down plan for `sample`.
pub fn plan_panel(
    ctx: &PanelContext<'_>,
    style: &PanelStyle,
    plan: &PlanProjection,
    sample: &SamplePoint,
) -> Option<Canvas> {
    let (shot, solved_shot) = ctx.shot(&sample.shot_id)?;
    let camera = solved_shot.frame_at(sample.scene_frame)?;
    let cs = ctx.cs();
    let overlays = style.overlays.gated(style.intent);
    let human = style.human();
    let frame = sample.scene_frame;

    let mut canvas = Canvas::new(plan.width, plan.height);
    canvas.push(CanvasItem::Background {
        color: theme::WHITE,
    });

    if human && overlays.grid {
        for (a, b) in plan.grid_lines(1.0) {
            canvas.push(CanvasItem::Polyline {
                points: vec![a, b],
                color: theme::GRID,
                width: 0.5,
                style: LineStyle::Solid,
                arrow_head: false,
                kind: ArrowKind::MotionCue,
            });
        }
    }

    if human && overlays.axes {
        for axis in &ctx.scene.axes {
            let (Some(from), Some(to)) = (
                ctx.base_position(&axis.from, frame),
                ctx.base_position(&axis.to, frame),
            ) else {
                continue;
            };
            canvas.push(CanvasItem::AxisLine {
                from: plan.to_canvas(from),
                to: plan.to_canvas(to),
                color: theme::AXIS,
                label: Some(axis.name.clone()),
            });
        }
    }

    // Subject motion over the shot, then the subjects themselves.
    let shot_frames = |track: &SolvedSubjectTrack| -> Vec<Vec2> {
        let start = shot.range.start as usize;
        let end = (shot.range.end as usize).min(track.transforms.len());
        if start >= end {
            return Vec::new();
        }
        sampled_path(&track.transforms[start..end], 2, |t| {
            plan.to_canvas(t.position)
        })
    };
    for track in &ctx.solved.subjects {
        if overlays.subject_paths {
            let points = shot_frames(track);
            if path_length(&points) > plan.length(0.05) {
                canvas.push(polyline(
                    points,
                    style.subject_color(track),
                    1.5,
                    LineStyle::Dashed,
                    ArrowKind::SubjectPath,
                ));
            }
        }
    }
    for track in &ctx.solved.subjects {
        let transform = track.transform_at(frame);
        let basis = oriented_basis(cs, transform.rotation_deg);
        let facing = plan.direction_deg(basis.forward).unwrap_or(-90.0);
        let size = Vec2::new(
            plan.length(track.dimensions_m.x * transform.scale.x),
            plan.length(track.dimensions_m.z * transform.scale.z),
        );
        canvas.push(CanvasItem::Glyph {
            shape: GlyphShape::from(track.kind),
            at: plan.to_canvas(transform.position),
            facing_deg: facing,
            size,
            color: style.subject_color(track),
            label: style.subject_label(track),
        });
    }

    if human && overlays.eyelines {
        for track in &ctx.solved.subjects {
            let Some(target) = track.cue_at(frame).and_then(|c| c.gaze_target.as_ref()) else {
                continue;
            };
            let Some(to) = ctx.base_position(target, frame) else {
                continue;
            };
            let from = plan.to_canvas(track.transform_at(frame).position);
            canvas.push(polyline(
                vec![from, plan.to_canvas(to)],
                theme::EYELINE,
                1.0,
                LineStyle::Dotted,
                ArrowKind::Eyeline,
            ));
        }
    }

    if human && overlays.screen_directions {
        for observation in &shot.continuity.screen_directions {
            let Some(track) = ctx.solved.subject(&observation.subject_id) else {
                continue;
            };
            let world_dir = match observation.direction {
                ScreenDirection::Left => -camera.right,
                ScreenDirection::Right => camera.right,
                ScreenDirection::TowardCamera => -camera.forward,
                ScreenDirection::AwayFromCamera => camera.forward,
                ScreenDirection::Stationary => continue,
            };
            let Some(angle) = plan.direction_deg(world_dir) else {
                continue;
            };
            let from = plan.to_canvas(track.transform_at(frame).position);
            let len = plan.length(0.8).max(18.0);
            canvas.push(polyline(
                vec![from, from.add(Vec2::from_angle(angle).scale(len))],
                style.subject_color(track),
                2.0,
                LineStyle::Solid,
                ArrowKind::ScreenDirection,
            ));
        }
    }

    // Camera motion over the shot, then the camera itself.
    if overlays.arrows {
        let points = sampled_path(&solved_shot.frames, 2, |f| plan.to_canvas(f.position));
        if path_length(&points) > plan.length(0.05) {
            canvas.push(polyline(
                points,
                theme::CAMERA,
                2.0,
                LineStyle::Solid,
                ArrowKind::CameraPath,
            ));
        }
    }
    let facing = plan.direction_deg(camera.forward).unwrap_or(-90.0);
    let label = human.then(|| {
        let size = short_shot_size(shot.framing.shot_size);
        let mut text = shot.id.clone();
        if !size.is_empty() {
            text.push_str(" · ");
            text.push_str(size);
        }
        text.push_str(&format!(" {:.0}mm", camera.focal_length_mm));
        text
    });
    canvas.push(CanvasItem::CameraWedge {
        at: plan.to_canvas(camera.position),
        facing_deg: facing,
        hfov_deg: camera.horizontal_fov_deg,
        length: plan.length(1.5).clamp(24.0, 90.0),
        color: theme::CAMERA,
        label,
    });

    if human && overlays.markers {
        for marker in &ctx.scene.markers {
            canvas.push(CanvasItem::Marker {
                at: plan.to_canvas(marker.position),
                color: theme::MARKER,
                label: Some(marker.name.clone()),
            });
        }
    }

    if human && overlays.diagnostics {
        let at = plan.to_canvas(camera.position);
        for (index, diagnostic) in ctx.diagnostics_for(&shot.id).iter().enumerate() {
            canvas.push(CanvasItem::Diagnostic {
                at: at.add(Vec2::new(14.0, -16.0 - 13.0 * index as f32)),
                code: diagnostic.code.clone(),
                severity: diagnostic.severity,
            });
        }
    }

    Some(canvas)
}

/// Through-the-lens frame for `sample`.
pub fn frame_panel(
    ctx: &PanelContext<'_>,
    style: &PanelStyle,
    width: f32,
    sample: &SamplePoint,
) -> Option<Canvas> {
    let (shot, solved_shot) = ctx.shot(&sample.shot_id)?;
    let camera = solved_shot.frame_at(sample.scene_frame)?;
    let cs = ctx.cs();
    let overlays = style.overlays.gated(style.intent);
    let human = style.human();
    let frame = sample.scene_frame;
    let height = (width / style.aspect).round();

    let projection = FrameProjection {
        position: camera.position,
        basis: crate::math::Basis {
            right: camera.right,
            up: camera.up,
            forward: camera.forward,
        },
        focal_length_mm: camera.focal_length_mm,
        sensor_width_mm: solved_shot.sensor_width_mm,
        aspect: style.aspect,
        width,
        height,
    };
    let bounds = projection.bounds();

    let mut canvas = Canvas::new(width, height);
    canvas.push(CanvasItem::Background {
        color: theme::WHITE,
    });

    // Painter's order: far subjects first.
    let mut silhouettes: Vec<(f32, CanvasItem)> = Vec::new();
    for track in &ctx.solved.subjects {
        let transform = track.transform_at(frame);
        let basis = oriented_basis(cs, transform.rotation_deg);
        let Some((rect, depth)) = projection.project_box(
            transform.position,
            track.dimensions_m,
            &basis,
            transform.scale,
        ) else {
            continue;
        };
        let Some(visible) = rect.intersect(&bounds) else {
            continue;
        };
        silhouettes.push((
            depth,
            CanvasItem::Silhouette {
                shape: GlyphShape::from(track.kind),
                rect: visible,
                color: style.subject_color(track),
                label: style.subject_label(track),
                // Above the figure when there is room, otherwise just inside it.
                label_at: if visible.min.y >= 14.0 {
                    Vec2::new(visible.center().x, visible.min.y - 4.0)
                } else {
                    Vec2::new(visible.center().x, visible.min.y + 12.0)
                },
            },
        ));
    }
    silhouettes.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    for (_, item) in silhouettes {
        canvas.push(item);
    }

    if overlays.arrows {
        let local = frame.saturating_sub(shot.range.start);
        let ops: Vec<&TimedCameraOperation> = shot
            .camera
            .operations
            .iter()
            .filter(|t| local >= t.range.start && local < t.range.end)
            .collect();
        for cue in motion_cues(&ops, &projection, ctx, solved_shot, camera, frame) {
            canvas.push(cue);
        }
    }

    if human && overlays.guides {
        canvas.push(CanvasItem::Guide {
            kind: GuideKind::Thirds,
            color: theme::GUIDE,
        });
        canvas.push(CanvasItem::Guide {
            kind: GuideKind::Center,
            color: theme::GUIDE,
        });
        canvas.push(CanvasItem::Guide {
            kind: GuideKind::SafeArea,
            color: theme::GRID,
        });
        let composition: &Composition = &shot.framing.composition;
        if let Some(headroom) = composition.headroom {
            canvas.push(CanvasItem::Guide {
                kind: GuideKind::Headroom {
                    y: headroom.clamp(0.0, 1.0) * height,
                },
                color: theme::EYELINE,
            });
        }
        if let (Some(lead), Some(region)) = (composition.lead_room, composition.negative_space) {
            let x = match region {
                ScreenRegion::Right => Some(width * (1.0 - lead.clamp(0.0, 1.0))),
                ScreenRegion::Left => Some(width * lead.clamp(0.0, 1.0)),
                _ => None,
            };
            if let Some(x) = x {
                canvas.push(CanvasItem::Guide {
                    kind: GuideKind::LeadRoom { x },
                    color: theme::EYELINE,
                });
            }
        }
    }

    if human {
        let mut text = format!(
            "{} {:.0}mm f/{:.1}",
            short_shot_size(shot.framing.shot_size),
            camera.focal_length_mm,
            camera.aperture_f
        );
        for eyeline in &shot.continuity.eyelines {
            let dir = match eyeline.screen_direction {
                HorizontalDirection::Left => "<-",
                HorizontalDirection::Right => "->",
                HorizontalDirection::Center => "o",
            };
            text.push_str(&format!("  {} {dir}", eyeline.subject_id));
        }
        canvas.push(CanvasItem::Text {
            at: Vec2::new(6.0, height - 6.0),
            content: text.trim().to_owned(),
            anchor: Anchor::Start,
            size: 10.0,
            color: theme::INK,
            bold: false,
        });
    }

    Some(canvas)
}

/// Screen-space arrows describing how the camera moves (ADR-1115 D8). They
/// carry no text and are the only overlay allowed on conditioning frames.
fn motion_cues(
    ops: &[&TimedCameraOperation],
    projection: &FrameProjection,
    ctx: &PanelContext<'_>,
    solved_shot: &SolvedShot,
    camera: &SolvedCameraFrame,
    frame: Frame,
) -> Vec<CanvasItem> {
    let w = projection.width;
    let h = projection.height;
    let m = w * 0.08;
    let len = w * 0.14;
    let color = theme::CAMERA;
    let arrow = |from: Vec2, to: Vec2| CanvasItem::Polyline {
        points: vec![from, to],
        color,
        width: 3.0,
        style: LineStyle::Solid,
        arrow_head: true,
        kind: ArrowKind::MotionCue,
    };
    let center = Vec2::new(w / 2.0, h / 2.0);
    let inward = |out: bool| -> Vec<CanvasItem> {
        [
            Vec2::new(m, m),
            Vec2::new(w - m, m),
            Vec2::new(m, h - m),
            Vec2::new(w - m, h - m),
        ]
        .into_iter()
        .map(|corner| {
            let dir = center.sub(corner);
            let unit = dir.scale(1.0 / dir.length().max(1e-3));
            let a = corner;
            let b = corner.add(unit.scale(len));
            if out {
                arrow(b, a)
            } else {
                arrow(a, b)
            }
        })
        .collect()
    };
    let horizontal = |y: f32, to_right: bool| {
        let a = Vec2::new(center.x - len / 2.0, y);
        let b = Vec2::new(center.x + len / 2.0, y);
        if to_right {
            arrow(a, b)
        } else {
            arrow(b, a)
        }
    };
    let vertical = |x: f32, up: bool| {
        let a = Vec2::new(x, center.y + len / 2.0);
        let b = Vec2::new(x, center.y - len / 2.0);
        if up {
            arrow(a, b)
        } else {
            arrow(b, a)
        }
    };

    let mut cues = Vec::new();
    for timed in ops {
        match &timed.operation {
            CameraOperation::Hold
            | CameraOperation::RackFocus { .. }
            | CameraOperation::Rotate { .. } => {}
            CameraOperation::Pan { degrees } => cues.push(horizontal(m, *degrees < 0.0)),
            CameraOperation::Tilt { degrees } => cues.push(vertical(w - m, *degrees >= 0.0)),
            CameraOperation::Roll { degrees } => {
                // +roll leans the camera's up toward its right: clockwise on screen.
                let r = len * 0.6;
                let sign = if *degrees >= 0.0 { 1.0 } else { -1.0 };
                let points: Vec<Vec2> = (0..=6)
                    .map(|i| {
                        let a = (-120.0 + sign * 40.0 * i as f32).to_radians();
                        Vec2::new(w - m - r + r * a.cos(), m + r + r * a.sin())
                    })
                    .collect();
                cues.push(CanvasItem::Polyline {
                    points,
                    color,
                    width: 3.0,
                    style: LineStyle::Solid,
                    arrow_head: true,
                    kind: ArrowKind::MotionCue,
                });
            }
            CameraOperation::Dolly { distance_m } => cues.extend(inward(*distance_m < 0.0)),
            CameraOperation::Zoom { to_focal_length_mm } => {
                // Compare with the focal length when the zoom starts, not the
                // sampled frame's (which already equals the target at the end).
                let start_focal = solved_shot
                    .frame_at(solved_shot.range.start + timed.range.start)
                    .map_or(camera.focal_length_mm, |f| f.focal_length_mm);
                cues.extend(inward(*to_focal_length_mm < start_focal))
            }
            CameraOperation::DollyZoom {
                target,
                to_distance_m,
                ..
            } => {
                let start_frame = solved_shot.range.start + timed.range.start;
                let end_frame = (solved_shot.range.start + timed.range.end.saturating_sub(1))
                    .min(solved_shot.range.end.saturating_sub(1));
                let start_distance = solved_shot
                    .frame_at(start_frame)
                    .zip(ctx.aim_position(target, start_frame))
                    .map(|(start, aim)| (start.position - aim).length());
                let camera_moves_away = start_distance.is_some_and(|d| *to_distance_m > d);
                cues.extend(inward(camera_moves_away));

                // Image motion is derived from projected solved states.  When
                // no background proxy is available, distance is the physical
                // fallback for the coupled dolly/zoom mechanism.
                let background_expands =
                    projected_background_spread(ctx, solved_shot, target, start_frame, w, h)
                        .zip(projected_background_spread(
                            ctx,
                            solved_shot,
                            target,
                            end_frame,
                            w,
                            h,
                        ))
                        .map_or(camera_moves_away, |(start, end)| end > start);
                let y = h - m;
                let (left_a, left_b) = (Vec2::new(w * 0.3, y), Vec2::new(w * 0.3 - len / 2.0, y));
                let (right_a, right_b) = (Vec2::new(w * 0.7, y), Vec2::new(w * 0.7 + len / 2.0, y));
                if background_expands {
                    cues.push(arrow(left_a, left_b));
                    cues.push(arrow(right_a, right_b));
                } else {
                    cues.push(arrow(left_b, left_a));
                    cues.push(arrow(right_b, right_a));
                }
            }
            CameraOperation::Truck { distance_m }
            | CameraOperation::Reveal {
                lateral_distance_m: distance_m,
                ..
            } => cues.push(horizontal(h - m, *distance_m >= 0.0)),
            CameraOperation::Pedestal { distance_m } => cues.push(vertical(m, *distance_m >= 0.0)),
            CameraOperation::Crane { delta, .. } => {
                let vertical_component = delta.dot(camera.up);
                if vertical_component.abs() > 1e-3 {
                    cues.push(vertical(m, vertical_component > 0.0));
                }
                let lateral = delta.dot(camera.right);
                if lateral.abs() > 1e-3 {
                    cues.push(horizontal(h - m, lateral > 0.0));
                }
            }
            CameraOperation::Translate { delta, space } => {
                // Camera-local deltas are expressed in the identity frame
                // (the solver rotates them); world deltas project onto the
                // camera's current axes.
                let (lateral, vertical_component, forward) = match space {
                    TransformSpace::World => (
                        delta.dot(camera.right),
                        delta.dot(camera.up),
                        delta.dot(camera.forward),
                    ),
                    TransformSpace::CameraLocal => {
                        let id = identity_basis(ctx.cs());
                        (delta.dot(id.right), delta.dot(id.up), delta.dot(id.forward))
                    }
                };
                if forward.abs() > 1e-3 {
                    cues.extend(inward(forward < 0.0));
                }
                if lateral.abs() > 1e-3 {
                    cues.push(horizontal(h - m, lateral > 0.0));
                }
                if vertical_component.abs() > 1e-3 {
                    cues.push(vertical(m, vertical_component > 0.0));
                }
            }
            CameraOperation::Orbit { azimuth_deg, .. } => {
                let r = len;
                let sign = if *azimuth_deg >= 0.0 { -1.0 } else { 1.0 };
                let points: Vec<Vec2> = (0..=8)
                    .map(|i| {
                        let a = (200.0 + sign * 17.5 * i as f32).to_radians();
                        Vec2::new(center.x + r * a.cos(), m + r * 0.6 + r * 0.6 * a.sin())
                    })
                    .collect();
                cues.push(CanvasItem::Polyline {
                    points,
                    color,
                    width: 3.0,
                    style: LineStyle::Solid,
                    arrow_head: true,
                    kind: ArrowKind::MotionCue,
                });
            }
            CameraOperation::LookAt { target } => {
                if let Some(to) = ctx
                    .aim_position(target, frame)
                    .and_then(|p| projection.project(p).map(|(v, _)| v))
                {
                    let dir = to.sub(center);
                    if dir.length() > 4.0 {
                        let unit = dir.scale(1.0 / dir.length());
                        cues.push(arrow(center, center.add(unit.scale(len))));
                    }
                }
            }
            CameraOperation::Follow { target, .. } => {
                let start = shot_start_position(ctx, target, solved_shot.range.start);
                let end = shot_start_position(ctx, target, solved_shot.range.end.saturating_sub(1));
                if let (Some(a), Some(b)) = (
                    start.and_then(|p| projection.project(p).map(|(v, _)| v)),
                    end.and_then(|p| projection.project(p).map(|(v, _)| v)),
                ) {
                    let dir = b.sub(a);
                    if dir.length() > 4.0 {
                        let unit = dir.scale(1.0 / dir.length());
                        let from = Vec2::new(center.x - unit.x * len / 2.0, h - m);
                        cues.push(arrow(from, from.add(Vec2::new(unit.x * len, 0.0))));
                    }
                }
            }
            CameraOperation::HandheldNoise { .. } => {
                let points: Vec<Vec2> = (0..=6)
                    .map(|i| {
                        let x = m + i as f32 * len / 6.0;
                        let y = if i % 2 == 0 { m } else { m + 6.0 };
                        Vec2::new(x, y)
                    })
                    .collect();
                cues.push(CanvasItem::Polyline {
                    points,
                    color,
                    width: 2.0,
                    style: LineStyle::Solid,
                    arrow_head: false,
                    kind: ArrowKind::MotionCue,
                });
            }
        }
    }
    cues
}

fn projected_background_spread(
    ctx: &PanelContext<'_>,
    shot: &SolvedShot,
    target: &TargetRef,
    frame: Frame,
    width: f32,
    height: f32,
) -> Option<f32> {
    let camera = shot.frame_at(frame)?;
    let projection = FrameProjection {
        position: camera.position,
        basis: crate::math::Basis {
            right: camera.right,
            up: camera.up,
            forward: camera.forward,
        },
        focal_length_mm: camera.focal_length_mm,
        sensor_width_mm: shot.sensor_width_mm,
        aspect: width / height,
        width,
        height,
    };
    let target_screen = projection.project(ctx.aim_position(target, frame)?)?.0;
    let target_id = match target {
        TargetRef::Subject { id } => Some(id.as_str()),
        _ => None,
    };
    let offsets: Vec<f32> = ctx
        .solved
        .subjects
        .iter()
        .filter(|track| Some(track.subject_id.as_str()) != target_id)
        .filter_map(|track| {
            projection
                .project(ctx.aim_position(
                    &TargetRef::Subject {
                        id: track.subject_id.clone(),
                    },
                    frame,
                )?)
                .map(|(point, _)| point.sub(target_screen).length())
        })
        .collect();
    (!offsets.is_empty()).then(|| offsets.iter().sum::<f32>() / offsets.len() as f32)
}

fn shot_start_position(ctx: &PanelContext<'_>, target: &TargetRef, frame: Frame) -> Option<Vec3> {
    ctx.base_position(target, frame)
}

/// Rectangle helper used by encoders for silhouettes.
pub fn silhouette_parts(shape: GlyphShape, rect: Rect) -> (Option<(Vec2, f32)>, Rect) {
    match shape {
        GlyphShape::Character => {
            let head_r = (rect.height() * 0.11).min(rect.width() * 0.45).max(1.0);
            let head = (Vec2::new(rect.center().x, rect.min.y + head_r), head_r);
            let body = Rect::new(Vec2::new(rect.min.x, rect.min.y + head_r * 2.0), rect.max);
            (Some(head), body)
        }
        _ => (None, rect),
    }
}

pub fn severity_color(severity: Severity) -> Rgb {
    match severity {
        Severity::Error => theme::ERROR,
        Severity::Warning => theme::WARNING,
    }
}
