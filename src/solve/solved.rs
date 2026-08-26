//! Solved Camera IR: the compiled, per-frame product of [`crate::solve`].
//!
//! `SolvedProject` is a rebuildable artifact (never the source of truth). It
//! is consumed by adapters, animated views, and trajectory comparison, and is
//! self-contained: consumers do not need the originating `CineProject`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::model::{
    CoordinateSystem, CoverageRole, EulerDeg, Frame, FrameRange, FrameRate, ShotPurpose, ShotSize,
    SubjectKind, TargetRef, Transform, Vec3,
};
use crate::palette::Rgb;

/// Schema tag written into every solved document.
pub const SOLVED_SCHEMA_VERSION: &str = "solved-0.1";

/// How much physical fidelity the solver applies.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Fidelity {
    /// Kinematic evaluation only: easing, look-at, orbit, follow, optics.
    /// No collision, no dynamics limits, no occlusion.
    #[default]
    Draft,
    /// Constraint solving with collision and dynamics. Not yet implemented.
    Full,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SolvedProject {
    pub schema_version: String,
    pub source_id: String,
    pub source_title: String,
    pub source_schema_version: String,
    pub fidelity: Fidelity,
    pub frame_rate: FrameRate,
    pub coordinate_system: CoordinateSystem,
    pub scenes: Vec<SolvedScene>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SolvedScene {
    pub id: String,
    pub title: String,
    pub duration_frames: Frame,
    pub subjects: Vec<SolvedSubjectTrack>,
    pub shots: Vec<SolvedShot>,
}

/// One subject sampled at every scene frame.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SolvedSubjectTrack {
    pub subject_id: String,
    pub name: String,
    pub kind: SubjectKind,
    /// Resolved bounding size (width, height, depth) with kind defaults applied.
    pub dimensions_m: Vec3,
    /// Deterministic palette colour (ADR-1115 D10).
    pub color: Rgb,
    pub color_name: String,
    /// `transforms[f]` is the subject's base transform at scene frame `f`.
    pub transforms: Vec<Transform>,
    /// Non-geometric blocking cues (gaze, action text) in frame order.
    pub cues: Vec<SolvedCue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SolvedCue {
    pub frame: Frame,
    #[serde(default)]
    pub gaze_target: Option<TargetRef>,
    #[serde(default)]
    pub action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SolvedShot {
    pub id: String,
    /// Scene-local, half-open.
    pub range: FrameRange,
    pub coverage_role: CoverageRole,
    pub shot_size: ShotSize,
    pub purpose: Vec<ShotPurpose>,
    pub subject_ids: Vec<String>,
    pub sensor_width_mm: f32,
    /// `frames[i]` is the camera at scene frame `range.start + i`.
    pub frames: Vec<SolvedCameraFrame>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SolvedCameraFrame {
    /// Scene-local frame index.
    pub frame: Frame,
    pub position: Vec3,
    pub rotation_deg: EulerDeg,
    /// World-space unit vectors of the camera frame, so consumers never need
    /// to re-derive Euler conventions.
    pub forward: Vec3,
    pub up: Vec3,
    pub right: Vec3,
    pub focal_length_mm: f32,
    pub aperture_f: f32,
    pub horizontal_fov_deg: f32,
    #[serde(default)]
    pub focus_distance_m: Option<f32>,
    #[serde(default)]
    pub focus_target: Option<TargetRef>,
}

impl SolvedProject {
    pub fn scene(&self, id: &str) -> Option<&SolvedScene> {
        self.scenes.iter().find(|scene| scene.id == id)
    }
}

impl SolvedScene {
    pub fn subject(&self, id: &str) -> Option<&SolvedSubjectTrack> {
        self.subjects.iter().find(|track| track.subject_id == id)
    }

    pub fn shot(&self, id: &str) -> Option<&SolvedShot> {
        self.shots.iter().find(|shot| shot.id == id)
    }

    /// The shot covering scene frame `frame`, if any.
    pub fn shot_at(&self, frame: Frame) -> Option<&SolvedShot> {
        self.shots
            .iter()
            .find(|shot| frame >= shot.range.start && frame < shot.range.end)
    }
}

impl SolvedSubjectTrack {
    /// Transform at `frame`, clamped to the scene duration.
    pub fn transform_at(&self, frame: Frame) -> Transform {
        let index = (frame as usize).min(self.transforms.len().saturating_sub(1));
        self.transforms.get(index).copied().unwrap_or_default()
    }

    /// The most recent cue at or before `frame`.
    pub fn cue_at(&self, frame: Frame) -> Option<&SolvedCue> {
        self.cues.iter().rev().find(|cue| cue.frame <= frame)
    }
}

impl SolvedShot {
    /// Camera at scene frame `frame`, clamped to the shot's range.
    pub fn frame_at(&self, frame: Frame) -> Option<&SolvedCameraFrame> {
        if self.frames.is_empty() {
            return None;
        }
        let index = frame.saturating_sub(self.range.start) as usize;
        Some(&self.frames[index.min(self.frames.len() - 1)])
    }

    pub fn first(&self) -> Option<&SolvedCameraFrame> {
        self.frames.first()
    }

    pub fn last(&self) -> Option<&SolvedCameraFrame> {
        self.frames.last()
    }
}
