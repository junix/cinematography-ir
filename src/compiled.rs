//! Shared Compiled Cinematography / Guidance IR.
//!
//! This is the narrow waist between authored intent and every downstream
//! consumer.  It retains semantic intent, adds temporal phases and observable
//! screen-space tracks, and owns the constraints used by solve, view, prompt,
//! execute, and compare.

use std::collections::BTreeSet;
use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::diagnostic::ValidationReport;
use crate::math::{oriented_basis, Basis, Vec3};
use crate::model::{
    AxisObservation, Beat, CameraOperation, CameraPlan, CineProject, Composition,
    ContinuityAnnotations, CoordinateSystem, CoverageRole, DepthLayer, EditClip, ExposureState,
    Frame, FrameRange, FrameRate, Framing, HorizontalAngle, LightingSetup, Marker, NarrativeIntent,
    Prohibition, Scene, Shot, ShotPhrase, ShotPurpose, ShotRelation, ShotRelationKind, ShotSize,
    Take, TargetRef, TransitionSpec, VerticalAngle,
};
use crate::solve::{
    normalize_units, solve_project, SolveError, SolveOptions, SolvedCameraFrame, SolvedProject,
    SolvedScene, SolvedShot, SolvedSubjectTrack,
};

pub const COMPILED_SCHEMA_VERSION: &str = "compiled-guidance-0.1";

#[derive(Debug, Clone)]
pub struct CompileOptions {
    pub solve: SolveOptions,
    pub aspect_ratio: f32,
    pub subject_fraction_tolerance: f32,
    pub screen_center_tolerance: f32,
    pub horizon_tolerance_deg: f32,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            solve: SolveOptions::default(),
            aspect_ratio: 16.0 / 9.0,
            subject_fraction_tolerance: 0.015,
            screen_center_tolerance: 0.02,
            horizon_tolerance_deg: 0.5,
        }
    }
}

#[derive(Debug)]
pub enum CompileError {
    Solve(SolveError),
    InvalidOptions(String),
    MissingSolvedScene(String),
    MissingSolvedShot(String),
    MissingTake(String),
    InvalidEdit(String),
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Solve(error) => write!(f, "{error}"),
            Self::InvalidOptions(message) => write!(f, "invalid compile options: {message}"),
            Self::MissingSolvedScene(id) => write!(f, "solver did not produce scene '{id}'"),
            Self::MissingSolvedShot(id) => write!(f, "solver did not produce shot '{id}'"),
            Self::MissingTake(id) => write!(f, "edit timeline references unknown take '{id}'"),
            Self::InvalidEdit(message) => write!(f, "invalid edit timeline: {message}"),
        }
    }
}

impl std::error::Error for CompileError {}

impl From<SolveError> for CompileError {
    fn from(value: SolveError) -> Self {
        Self::Solve(value)
    }
}

#[derive(Debug, Clone)]
pub struct CompileOutput {
    pub compiled: CompiledProject,
    /// Retained for compatibility adapters that still lower solved camera
    /// frames into legacy payloads. New consumers should read `compiled`.
    pub solved: SolvedProject,
    pub report: ValidationReport,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CompiledProject {
    pub schema_version: String,
    pub source_id: String,
    pub source_title: String,
    pub source_schema_version: String,
    pub frame_rate: FrameRate,
    pub coordinate_system: CoordinateSystem,
    pub aspect_ratio: f32,
    pub scenes: Vec<CompiledScene>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CompiledScene {
    pub id: String,
    pub title: String,
    pub duration_frames: Frame,
    pub narrative: NarrativeIntent,
    pub markers: Vec<Marker>,
    pub axes: Vec<crate::model::ContinuityAxis>,
    pub beats: Vec<Beat>,
    pub lighting_setups: Vec<LightingSetup>,
    pub light_tracks: Vec<CompiledLightTrack>,
    pub subjects: Vec<SolvedSubjectTrack>,
    pub shots: Vec<CompiledShot>,
    pub takes: Vec<Take>,
    pub edit_timeline: Vec<EditClip>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CompiledShot {
    pub id: String,
    pub source_range: FrameRange,
    pub edit_range: FrameRange,
    pub take_id: String,
    pub intent: ShotIntent,
    pub phases: Vec<CompiledPhase>,
    pub camera_track: CameraTrack,
    pub subject_tracks: Vec<CompiledSubjectTrack>,
    pub lens_track: LensTrack,
    pub focus_track: FocusTrack,
    pub exposure_track: ExposureTrack,
    pub screen_tracks: Vec<SubjectScreenTrack>,
    pub constraints: Vec<ShotConstraint>,
    pub evaluations: Vec<ConstraintEvaluation>,
    pub edit_relation: EditRelation,
    pub prohibitions: Vec<Prohibition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ShotIntent {
    pub source_path: String,
    pub purpose: Vec<ShotPurpose>,
    pub coverage_role: CoverageRole,
    pub framing: Framing,
    pub camera: CameraPlan,
    pub lighting_setup_id: Option<String>,
    pub lighting: Option<LightingSetup>,
    pub continuity: ContinuityAnnotations,
    pub transition_in: TransitionSpec,
    pub relations: Vec<ShotRelation>,
    pub phrases: Vec<ShotPhrase>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CompiledPhase {
    pub id: String,
    pub local_range: FrameRange,
    pub edit_range: FrameRange,
    pub kind: PhaseKind,
    pub operation_indices: Vec<usize>,
    pub beat_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PhaseKind {
    Hold,
    Motion,
    Focus,
    Reveal,
    Settle,
    Mixed,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CameraTrack {
    pub frames: Vec<SolvedCameraFrame>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CompiledSubjectTrack {
    pub subject_id: String,
    pub name: String,
    pub kind: crate::model::SubjectKind,
    pub dimensions_m: Vec3,
    pub color: crate::palette::Rgb,
    pub color_name: String,
    pub transforms: Vec<crate::model::Transform>,
    pub cues: Vec<crate::solve::SolvedCue>,
    pub poses: Option<crate::model::PoseTrack>,
    pub geometry_proxy: Option<crate::model::GeometryProxy>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LensTrack {
    pub frames: Vec<LensFrame>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LensFrame {
    pub frame: Frame,
    pub focal_length_mm: f32,
    pub sensor_width_mm: f32,
    pub aperture_f: f32,
    pub anamorphic_squeeze: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FocusTrack {
    pub frames: Vec<FocusFrame>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FocusFrame {
    pub frame: Frame,
    pub target: Option<TargetRef>,
    pub distance_m: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExposureTrack {
    pub frames: Vec<ExposureFrame>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExposureFrame {
    pub frame: Frame,
    pub iso: u32,
    pub shutter_angle_deg: f32,
    pub white_balance_k: u32,
    pub nd_stops: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CompiledLightTrack {
    pub setup_id: String,
    pub source_id: String,
    pub kind: crate::model::LightKind,
    pub role: crate::model::LightRole,
    pub frames: Vec<CompiledLightFrame>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CompiledLightFrame {
    pub frame: Frame,
    pub transform: crate::model::Transform,
    pub intensity_lux: Option<f32>,
    pub color_temperature_k: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SubjectScreenTrack {
    pub subject_id: String,
    pub frames: Vec<SubjectScreenFrame>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SubjectScreenFrame {
    pub frame: Frame,
    pub bbox: NormalizedRect,
    pub visible_bbox: Option<NormalizedRect>,
    pub visible_fraction: f32,
    pub center: ScreenPoint,
    pub depth_m: Option<f32>,
    pub focus_score: Option<f32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct ScreenPoint {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct NormalizedRect {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

impl NormalizedRect {
    pub fn width(self) -> f32 {
        (self.max_x - self.min_x).max(0.0)
    }

    pub fn height(self) -> f32 {
        (self.max_y - self.min_y).max(0.0)
    }

    pub fn area(self) -> f32 {
        self.width() * self.height()
    }

    pub fn center(self) -> ScreenPoint {
        ScreenPoint {
            x: (self.min_x + self.max_x) * 0.5,
            y: (self.min_y + self.max_y) * 0.5,
        }
    }

    pub fn intersect(self, other: Self) -> Option<Self> {
        let result = Self {
            min_x: self.min_x.max(other.min_x),
            min_y: self.min_y.max(other.min_y),
            max_x: self.max_x.min(other.max_x),
            max_y: self.max_y.min(other.max_y),
        };
        (result.width() > 0.0 && result.height() > 0.0).then_some(result)
    }

    pub fn clipped(self) -> Option<Self> {
        self.intersect(Self {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 1.0,
            max_y: 1.0,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ShotConstraint {
    pub id: String,
    pub source_path: String,
    pub range: FrameRange,
    pub tolerance: f32,
    pub kind: ShotConstraintKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ShotConstraintKind {
    SubjectBboxHeight {
        subject_id: String,
        target: f32,
    },
    SubjectScreenCenter {
        subject_id: String,
        target: ScreenPoint,
    },
    SubjectScreenY {
        subject_id: String,
        target: f32,
    },
    PairScreenSeparation {
        subject_ids: [String; 2],
        trend: Trend,
    },
    HorizonLock {
        target_roll_deg: f32,
    },
    NoCameraMotion,
    RevealVisibility {
        subject_id: String,
        from: f32,
        to: f32,
        monotonic: bool,
    },
    FocusHandoff {
        target: TargetRef,
    },
    ShotSize {
        value: ShotSize,
    },
    HorizontalAngle {
        value: HorizontalAngle,
    },
    VerticalAngle {
        value: VerticalAngle,
    },
    Composition {
        value: Composition,
    },
    DepthLayers {
        value: Vec<DepthLayer>,
    },
    ContinuityAxis {
        observations: Vec<AxisObservation>,
    },
    CameraRig {
        value: crate::model::CameraRig,
    },
    AnamorphicSqueeze {
        target: f32,
    },
    Exposure {
        value: ExposureState,
    },
    Lighting {
        setup_id: Option<String>,
    },
    Transition {
        value: TransitionSpec,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Trend {
    Increasing,
    Decreasing,
    Stable,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConstraintEvaluation {
    pub constraint_id: String,
    pub status: ConstraintStatus,
    pub requested: String,
    pub actual: String,
    pub max_error: Option<f32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConstraintStatus {
    Pass,
    Fail,
    Indeterminate,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EditRelation {
    pub previous_shot_id: Option<String>,
    pub next_shot_id: Option<String>,
    pub transition_in: TransitionSpec,
    pub relations: Vec<ShotRelation>,
}

pub fn compile_project(
    project: &CineProject,
    options: &CompileOptions,
) -> Result<CompileOutput, CompileError> {
    if !options.aspect_ratio.is_finite() || options.aspect_ratio <= 0.0 {
        return Err(CompileError::InvalidOptions(
            "aspect_ratio must be finite and positive".to_owned(),
        ));
    }
    let normalized = normalize_units(project);
    let source = &*normalized;
    let solved_output = solve_project(source, &options.solve)?;
    let compiled = compile_solved(source, &solved_output.solved, options)?;
    Ok(CompileOutput {
        compiled,
        solved: solved_output.solved,
        report: solved_output.report,
    })
}

pub fn compile_solved(
    project: &CineProject,
    solved: &SolvedProject,
    options: &CompileOptions,
) -> Result<CompiledProject, CompileError> {
    let mut scenes = Vec::with_capacity(project.scenes.len());
    for scene in &project.scenes {
        let solved_scene = solved
            .scene(&scene.id)
            .ok_or_else(|| CompileError::MissingSolvedScene(scene.id.clone()))?;
        scenes.push(compile_scene(
            scene,
            solved_scene,
            &project.coordinate_system,
            project.frame_rate.frames_per_second().unwrap_or(24.0),
            options,
        )?);
    }
    Ok(CompiledProject {
        schema_version: COMPILED_SCHEMA_VERSION.to_owned(),
        source_id: project.id.clone(),
        source_title: project.title.clone(),
        source_schema_version: project.schema_version.clone(),
        frame_rate: project.frame_rate,
        coordinate_system: project.coordinate_system.clone(),
        aspect_ratio: options.aspect_ratio,
        scenes,
    })
}

fn compile_scene(
    scene: &Scene,
    solved: &SolvedScene,
    coordinate_system: &CoordinateSystem,
    fps: f64,
    options: &CompileOptions,
) -> Result<CompiledScene, CompileError> {
    let light_tracks = scene
        .lighting_setups
        .iter()
        .flat_map(|setup| {
            setup.sources.iter().map(|source| CompiledLightTrack {
                setup_id: setup.id.clone(),
                source_id: source.id.clone(),
                kind: source.kind,
                role: source.role,
                frames: (0..scene.duration_frames)
                    .map(|frame| CompiledLightFrame {
                        frame,
                        transform: source.transform,
                        intensity_lux: source.intensity_lux,
                        color_temperature_k: source.color_temperature_k,
                    })
                    .collect(),
            })
        })
        .collect();

    let context = ShotCompileContext {
        scene,
        solved,
        coordinate_system,
        options,
    };
    let mut source_shots: Vec<&Shot> = scene.shots.iter().collect();
    source_shots.sort_by_key(|shot| (shot.range.start, shot.range.end));
    let mut shots = Vec::with_capacity(source_shots.len());
    for (index, shot) in source_shots.iter().enumerate() {
        let solved_shot = solved
            .shot(&shot.id)
            .ok_or_else(|| CompileError::MissingSolvedShot(shot.id.clone()))?;
        shots.push(compile_shot(
            &context,
            shot,
            solved_shot,
            index,
            source_shots.len(),
        ));
    }
    if source_shots.is_empty() && !scene.edit_timeline.is_empty() {
        let context = crate::solve::SceneContext {
            scene,
            cs: coordinate_system,
            fps,
            subjects: solved.subjects.clone(),
        };
        let clip_count = scene.edit_timeline.len();
        let mut clips: Vec<&EditClip> = scene.edit_timeline.iter().collect();
        clips.sort_by_key(|clip| (clip.edit_range.start, clip.edit_range.end));
        for (index, clip) in clips.into_iter().enumerate() {
            let take = scene
                .takes
                .iter()
                .find(|take| take.id == clip.take_id)
                .ok_or_else(|| CompileError::MissingTake(clip.take_id.clone()))?;
            if clip.source_range.start < take.performance_range.start
                || clip.source_range.end > take.performance_range.end
                || clip.source_range.duration() != clip.edit_range.duration()
            {
                return Err(CompileError::InvalidEdit(format!(
                    "clip '{}' must select an in-take source range with the same duration as edit_range",
                    clip.id
                )));
            }
            let full_shot = shot_from_take(take, take.performance_range, None);
            let sampled = crate::solve::sampler::sample_shot(&context, &full_shot);
            let frames: Vec<SolvedCameraFrame> = (clip.source_range.start..clip.source_range.end)
                .enumerate()
                .filter_map(|(offset, frame)| {
                    sampled.frame_at(frame).cloned().map(|mut value| {
                        value.frame = clip.edit_range.start + offset as u64;
                        value
                    })
                })
                .collect();
            let solved_clip = SolvedShot {
                id: clip.id.clone(),
                range: clip.edit_range,
                coverage_role: take.coverage_role,
                shot_size: take.framing.shot_size,
                purpose: take.purpose.clone(),
                subject_ids: take.framing.subject_ids.clone(),
                sensor_width_mm: take.camera_program.initial_state.lens.sensor_width_mm,
                frames,
            };
            let clip_subjects: Vec<SolvedSubjectTrack> = solved
                .subjects
                .iter()
                .map(|track| {
                    let mut remapped = track.clone();
                    remapped.transforms = (0..scene.duration_frames)
                        .map(|edit_frame| {
                            let source_frame = if edit_frame >= clip.edit_range.start
                                && edit_frame < clip.edit_range.end
                            {
                                clip.source_range.start + (edit_frame - clip.edit_range.start)
                            } else {
                                clip.source_range.start
                            };
                            track.transform_at(source_frame)
                        })
                        .collect();
                    remapped
                })
                .collect();
            let clip_scene = SolvedScene {
                id: solved.id.clone(),
                title: solved.title.clone(),
                duration_frames: solved.duration_frames,
                subjects: clip_subjects,
                shots: vec![solved_clip.clone()],
            };
            let mut authored = shot_from_take(take, clip.edit_range, Some(clip));
            authored.camera = slice_camera_plan(
                &take.camera_program,
                take.performance_range,
                clip.source_range,
                solved_clip.frames.first(),
            );
            let clip_context = ShotCompileContext {
                scene,
                solved: &clip_scene,
                coordinate_system,
                options,
            };
            let mut compiled =
                compile_shot(&clip_context, &authored, &solved_clip, index, clip_count);
            compiled.source_range = clip.source_range;
            compiled.take_id = take.id.clone();
            compiled.intent.source_path = format!(
                "scenes[{}].edit_timeline[{}] -> takes[{}]",
                scene.id, clip.id, take.id
            );
            shots.push(compiled);
        }
    }

    let takes = if scene.takes.is_empty() {
        source_shots
            .iter()
            .map(|shot| Take {
                id: shot.id.clone(),
                performance_range: shot.range,
                purpose: shot.purpose.clone(),
                coverage_role: shot.coverage_role,
                framing: shot.framing.clone(),
                camera_program: shot.camera.clone(),
                lighting_setup_id: shot.lighting_setup_id.clone(),
                continuity: shot.continuity.clone(),
                phrases: shot.phrases.clone(),
                notes: shot.notes.clone(),
                metadata: shot.metadata.clone(),
            })
            .collect()
    } else {
        scene.takes.clone()
    };
    let edit_timeline = if scene.edit_timeline.is_empty() {
        source_shots
            .iter()
            .map(|shot| EditClip {
                id: shot.id.clone(),
                take_id: shot.id.clone(),
                source_range: shot.range,
                edit_range: shot.range,
                transition_in: rich_transition(shot),
            })
            .collect()
    } else {
        scene.edit_timeline.clone()
    };

    Ok(CompiledScene {
        id: scene.id.clone(),
        title: scene.title.clone(),
        duration_frames: scene.duration_frames,
        narrative: scene.narrative.clone(),
        markers: scene.markers.clone(),
        axes: scene.axes.clone(),
        beats: scene.beats.clone(),
        lighting_setups: scene.lighting_setups.clone(),
        light_tracks,
        subjects: solved.subjects.clone(),
        shots,
        takes,
        edit_timeline,
    })
}

fn shot_from_take(take: &Take, range: FrameRange, clip: Option<&EditClip>) -> Shot {
    Shot {
        id: clip.map_or_else(|| take.id.clone(), |clip| clip.id.clone()),
        range,
        purpose: take.purpose.clone(),
        coverage_role: take.coverage_role,
        framing: take.framing.clone(),
        camera: take.camera_program.clone(),
        lighting_setup_id: take.lighting_setup_id.clone(),
        continuity: take.continuity.clone(),
        transition_in: clip.map_or(crate::model::Transition::Cut, |value| {
            value.transition_in.kind
        }),
        transition: clip.map(|value| value.transition_in.clone()),
        relations: Vec::new(),
        phrases: take.phrases.clone(),
        prohibit: Vec::new(),
        notes: take.notes.clone(),
        metadata: take.metadata.clone(),
    }
}

fn slice_camera_plan(
    plan: &CameraPlan,
    performance_range: FrameRange,
    source_range: FrameRange,
    first: Option<&SolvedCameraFrame>,
) -> CameraPlan {
    let clip_start = source_range.start.saturating_sub(performance_range.start);
    let clip_end = source_range.end.saturating_sub(performance_range.start);
    let mut result = plan.clone();
    if let Some(first) = first {
        result.initial_state.transform.position = first.position;
        result.initial_state.transform.rotation_deg = first.rotation_deg;
        result.initial_state.lens.focal_length_mm = first.focal_length_mm;
        result.initial_state.lens.aperture_f = first.aperture_f;
        result.initial_state.focus.target = first.focus_target.clone();
        result.initial_state.focus.distance_m = first.focus_distance_m;
    }
    result.operations = plan
        .operations
        .iter()
        .filter_map(|timed| {
            let start = timed.range.start.max(clip_start);
            let end = timed.range.end.min(clip_end);
            (start < end).then(|| {
                let mut value = timed.clone();
                value.range = FrameRange {
                    start: start - clip_start,
                    end: end - clip_start,
                };
                value
            })
        })
        .collect();
    result
}

struct ShotCompileContext<'a> {
    scene: &'a Scene,
    solved: &'a SolvedScene,
    coordinate_system: &'a CoordinateSystem,
    options: &'a CompileOptions,
}

fn compile_shot(
    context: &ShotCompileContext<'_>,
    shot: &Shot,
    solved: &SolvedShot,
    index: usize,
    shot_count: usize,
) -> CompiledShot {
    let scene = context.scene;
    let solved_scene = context.solved;
    let coordinate_system = context.coordinate_system;
    let options = context.options;
    let phases = compile_phases(scene, shot);
    let subject_tracks = solved_scene
        .subjects
        .iter()
        .map(|track| {
            let source = scene
                .subjects
                .iter()
                .find(|subject| subject.id == track.subject_id);
            CompiledSubjectTrack {
                subject_id: track.subject_id.clone(),
                name: track.name.clone(),
                kind: track.kind,
                dimensions_m: track.dimensions_m,
                color: track.color,
                color_name: track.color_name.clone(),
                transforms: (shot.range.start..shot.range.end)
                    .map(|frame| track.transform_at(frame))
                    .collect(),
                cues: (shot.range.start..shot.range.end)
                    .filter_map(|frame| track.cue_at(frame).cloned())
                    .collect(),
                poses: source.and_then(|subject| subject.pose_track.clone()),
                geometry_proxy: source.and_then(|subject| subject.geometry_proxy.clone()),
            }
        })
        .collect();
    let screen_tracks = compile_screen_tracks(
        solved_scene,
        solved,
        &scene.subjects,
        coordinate_system,
        options.aspect_ratio,
    );
    let constraints = compile_constraints(scene, shot, solved, &screen_tracks, &phases, options);
    let evaluations = constraints
        .iter()
        .map(|constraint| {
            evaluate_constraint(
                constraint,
                scene,
                shot,
                solved,
                solved_scene,
                coordinate_system,
                &screen_tracks,
            )
        })
        .collect();
    let lens = &shot.camera.initial_state.lens;
    let exposure = &shot.camera.initial_state.exposure;
    let lighting = shot
        .lighting_setup_id
        .as_ref()
        .and_then(|id| scene.lighting_setups.iter().find(|setup| &setup.id == id))
        .cloned();
    let mut prohibitions: BTreeSet<Prohibition> = shot.prohibit.iter().copied().collect();
    for timed in &shot.camera.operations {
        match timed.operation {
            CameraOperation::DollyZoom { .. } => {
                prohibitions.extend([
                    Prohibition::ActorTranslationAsCameraSubstitute,
                    Prohibition::DigitalCropAsDollySubstitute,
                    Prohibition::HandheldMotion,
                    Prohibition::UnplannedCut,
                ]);
            }
            CameraOperation::Reveal { .. } => {
                prohibitions.insert(Prohibition::BackgroundDeformationAsParallaxSubstitute);
            }
            CameraOperation::RackFocus { .. } => {
                prohibitions.insert(Prohibition::GlobalBlurAsRackFocusSubstitute);
            }
            _ => {}
        }
    }

    CompiledShot {
        id: shot.id.clone(),
        source_range: shot.range,
        edit_range: shot.range,
        take_id: shot.id.clone(),
        intent: ShotIntent {
            source_path: format!("scenes[{}].shots[{}]", scene.id, shot.id),
            purpose: shot.purpose.clone(),
            coverage_role: shot.coverage_role,
            framing: shot.framing.clone(),
            camera: shot.camera.clone(),
            lighting_setup_id: shot.lighting_setup_id.clone(),
            lighting,
            continuity: shot.continuity.clone(),
            transition_in: rich_transition(shot),
            relations: shot.relations.clone(),
            phrases: compiled_phrases(shot),
            notes: shot.notes.clone(),
        },
        phases,
        camera_track: CameraTrack {
            frames: solved.frames.clone(),
        },
        subject_tracks,
        lens_track: LensTrack {
            frames: solved
                .frames
                .iter()
                .map(|frame| LensFrame {
                    frame: frame.frame,
                    focal_length_mm: frame.focal_length_mm,
                    sensor_width_mm: solved.sensor_width_mm,
                    aperture_f: frame.aperture_f,
                    anamorphic_squeeze: lens.anamorphic_squeeze,
                })
                .collect(),
        },
        focus_track: FocusTrack {
            frames: solved
                .frames
                .iter()
                .map(|frame| FocusFrame {
                    frame: frame.frame,
                    target: frame.focus_target.clone(),
                    distance_m: frame.focus_distance_m,
                })
                .collect(),
        },
        exposure_track: ExposureTrack {
            frames: solved
                .frames
                .iter()
                .map(|frame| exposure_frame(frame.frame, exposure))
                .collect(),
        },
        screen_tracks,
        constraints,
        evaluations,
        edit_relation: EditRelation {
            previous_shot_id: (index > 0).then(|| ordered_edit_ids(scene)[index - 1].clone()),
            next_shot_id: (index + 1 < shot_count)
                .then(|| ordered_edit_ids(scene)[index + 1].clone()),
            transition_in: rich_transition(shot),
            relations: shot.relations.clone(),
        },
        prohibitions: prohibitions.into_iter().collect(),
    }
}

fn compiled_phrases(shot: &Shot) -> Vec<ShotPhrase> {
    let mut phrases: BTreeSet<_> = shot.phrases.iter().copied().collect();
    if shot.purpose.contains(&ShotPurpose::Reaction)
        && shot
            .camera
            .operations
            .iter()
            .all(|timed| matches!(timed.operation, CameraOperation::Hold))
    {
        phrases.insert(ShotPhrase::ReactionHold);
    }
    if shot
        .relations
        .iter()
        .any(|relation| relation.kind == ShotRelationKind::CrossesAxisFrom)
        || shot
            .continuity
            .bridge
            .is_some_and(|bridge| bridge == crate::model::ContinuityBridge::VisibleAxisCross)
    {
        phrases.insert(ShotPhrase::VisibleAxisCross);
    }
    for timed in &shot.camera.operations {
        match timed.operation {
            CameraOperation::Dolly { distance_m } if distance_m > 0.0 => {
                phrases.insert(ShotPhrase::PressurePushIn);
            }
            CameraOperation::DollyZoom { .. } => {
                phrases.insert(ShotPhrase::SubjectLockDollyZoom);
            }
            CameraOperation::Reveal { .. } => {
                phrases.insert(ShotPhrase::ForegroundParallaxReveal);
            }
            CameraOperation::RackFocus { .. } => {
                phrases.insert(ShotPhrase::RackFocusReveal);
            }
            CameraOperation::Follow { .. } => {
                phrases.insert(ShotPhrase::WalkAndTalkFollow);
            }
            CameraOperation::Orbit { .. } => {
                phrases.insert(ShotPhrase::OrbitRelationshipShift);
            }
            _ => {}
        }
    }
    phrases.into_iter().collect()
}

fn ordered_edit_ids(scene: &Scene) -> Vec<String> {
    if scene.shots.is_empty() {
        let mut values: Vec<&EditClip> = scene.edit_timeline.iter().collect();
        values.sort_by_key(|value| (value.edit_range.start, value.edit_range.end));
        values.into_iter().map(|value| value.id.clone()).collect()
    } else {
        let mut values: Vec<&Shot> = scene.shots.iter().collect();
        values.sort_by_key(|value| (value.range.start, value.range.end));
        values.into_iter().map(|value| value.id.clone()).collect()
    }
}

fn rich_transition(shot: &Shot) -> TransitionSpec {
    shot.transition
        .clone()
        .unwrap_or_else(|| TransitionSpec::cut(shot.transition_in))
}

fn exposure_frame(frame: Frame, exposure: &ExposureState) -> ExposureFrame {
    ExposureFrame {
        frame,
        iso: exposure.iso,
        shutter_angle_deg: exposure.shutter_angle_deg,
        white_balance_k: exposure.white_balance_k,
        nd_stops: exposure.nd_stops,
    }
}

fn compile_phases(scene: &Scene, shot: &Shot) -> Vec<CompiledPhase> {
    let mut boundaries = BTreeSet::from([0, shot.duration_frames()]);
    for timed in &shot.camera.operations {
        boundaries.insert(timed.range.start.min(shot.duration_frames()));
        boundaries.insert(timed.range.end.min(shot.duration_frames()));
    }
    for beat in &scene.beats {
        if beat.range.start < shot.range.end && shot.range.start < beat.range.end {
            boundaries.insert(beat.range.start.saturating_sub(shot.range.start));
            boundaries.insert(beat.range.end.min(shot.range.end) - shot.range.start);
        }
    }
    let points: Vec<Frame> = boundaries.into_iter().collect();
    let last_operation_end = shot
        .camera
        .operations
        .iter()
        .map(|timed| timed.range.end)
        .max()
        .unwrap_or(0);
    points
        .windows(2)
        .enumerate()
        .filter_map(|(phase_index, pair)| {
            let local_range = FrameRange {
                start: pair[0],
                end: pair[1],
            };
            if !local_range.is_valid() {
                return None;
            }
            let operation_indices: Vec<usize> = shot
                .camera
                .operations
                .iter()
                .enumerate()
                .filter(|(_, timed)| {
                    timed.range.start < local_range.end && local_range.start < timed.range.end
                })
                .map(|(index, _)| index)
                .collect();
            let operations: Vec<&CameraOperation> = operation_indices
                .iter()
                .map(|index| &shot.camera.operations[*index].operation)
                .collect();
            let kind = phase_kind(&operations, local_range.start >= last_operation_end);
            let edit_range = FrameRange {
                start: shot.range.start + local_range.start,
                end: shot.range.start + local_range.end,
            };
            let beat_ids = scene
                .beats
                .iter()
                .filter(|beat| {
                    beat.range.start < edit_range.end && edit_range.start < beat.range.end
                })
                .map(|beat| beat.id.clone())
                .collect();
            Some(CompiledPhase {
                id: format!("phase_{phase_index}_{}", phase_kind_name(kind)),
                local_range,
                edit_range,
                kind,
                operation_indices,
                beat_ids,
            })
        })
        .collect()
}

fn phase_kind(operations: &[&CameraOperation], after_motion: bool) -> PhaseKind {
    if operations.is_empty()
        || operations
            .iter()
            .all(|op| matches!(op, CameraOperation::Hold))
    {
        return if after_motion {
            PhaseKind::Settle
        } else {
            PhaseKind::Hold
        };
    }
    let focus = operations
        .iter()
        .any(|op| matches!(op, CameraOperation::RackFocus { .. }));
    let reveal = operations
        .iter()
        .any(|op| matches!(op, CameraOperation::Reveal { .. }));
    match (operations.len(), focus, reveal) {
        (1, true, _) => PhaseKind::Focus,
        (1, _, true) => PhaseKind::Reveal,
        (1, false, false) => PhaseKind::Motion,
        _ => PhaseKind::Mixed,
    }
}

fn phase_kind_name(kind: PhaseKind) -> &'static str {
    match kind {
        PhaseKind::Hold => "hold",
        PhaseKind::Motion => "motion",
        PhaseKind::Focus => "focus",
        PhaseKind::Reveal => "reveal",
        PhaseKind::Settle => "settle",
        PhaseKind::Mixed => "mixed",
    }
}

#[derive(Clone, Copy)]
struct ProjectedSubject {
    bbox: NormalizedRect,
    visible_bbox: Option<NormalizedRect>,
    depth: f32,
}

fn compile_screen_tracks(
    scene: &SolvedScene,
    shot: &SolvedShot,
    source_subjects: &[crate::model::Subject],
    coordinate_system: &CoordinateSystem,
    aspect: f32,
) -> Vec<SubjectScreenTrack> {
    let mut tracks: Vec<SubjectScreenTrack> = scene
        .subjects
        .iter()
        .map(|subject| SubjectScreenTrack {
            subject_id: subject.subject_id.clone(),
            frames: Vec::with_capacity(shot.frames.len()),
        })
        .collect();
    for camera in &shot.frames {
        let projected: Vec<Option<ProjectedSubject>> = scene
            .subjects
            .iter()
            .map(|subject| {
                project_subject(
                    subject,
                    camera.frame,
                    camera,
                    shot.sensor_width_mm,
                    coordinate_system,
                    aspect,
                )
            })
            .collect();
        for (index, (subject, track)) in scene.subjects.iter().zip(&mut tracks).enumerate() {
            let Some(value) = projected[index] else {
                continue;
            };
            let in_frame_fraction = value
                .visible_bbox
                .map_or(0.0, |visible| visible.area() / value.bbox.area().max(1e-6));
            let occluded_fraction = projected
                .iter()
                .enumerate()
                .filter(|(other_index, other)| {
                    *other_index != index
                        && other.is_some_and(|other| other.depth + 1e-3 < value.depth)
                })
                .filter_map(|(_, other)| other.and_then(|other| value.bbox.intersect(other.bbox)))
                .map(|overlap| overlap.area() / value.bbox.area().max(1e-6))
                .fold(0.0f32, f32::max);
            let focus_score = camera.focus_distance_m.map(|focus| {
                let scale = (value.depth * 0.05).max(0.1);
                (-(value.depth - focus).abs() / scale).exp()
            });
            let _geometry = source_subjects
                .iter()
                .find(|source| source.id == subject.subject_id)
                .and_then(|source| source.geometry_proxy.as_ref());
            track.frames.push(SubjectScreenFrame {
                frame: camera.frame,
                bbox: value.bbox,
                visible_bbox: value.visible_bbox,
                visible_fraction: (in_frame_fraction * (1.0 - occluded_fraction)).clamp(0.0, 1.0),
                center: value.bbox.center(),
                depth_m: Some(value.depth),
                focus_score,
            });
        }
    }
    tracks
}

fn project_subject(
    subject: &SolvedSubjectTrack,
    frame: Frame,
    camera: &SolvedCameraFrame,
    sensor_width_mm: f32,
    coordinate_system: &CoordinateSystem,
    aspect: f32,
) -> Option<ProjectedSubject> {
    let transform = subject.transform_at(frame);
    let subject_basis = oriented_basis(coordinate_system, transform.rotation_deg);
    let camera_basis = Basis {
        right: camera.right,
        up: camera.up,
        forward: camera.forward,
    };
    let half_w = subject.dimensions_m.x * transform.scale.x * 0.5;
    let half_d = subject.dimensions_m.z * transform.scale.z * 0.5;
    let height = subject.dimensions_m.y * transform.scale.y;
    let half_sensor_w = sensor_width_mm / (2.0 * camera.focal_length_mm);
    let half_sensor_h = half_sensor_w / aspect;
    let mut bbox = NormalizedRect {
        min_x: f32::INFINITY,
        min_y: f32::INFINITY,
        max_x: f32::NEG_INFINITY,
        max_y: f32::NEG_INFINITY,
    };
    let mut nearest = f32::INFINITY;
    let mut count = 0usize;
    for sx in [-1.0, 1.0] {
        for sz in [-1.0, 1.0] {
            for sy in [0.0, 1.0] {
                let corner = transform.position
                    + subject_basis.right * (sx * half_w)
                    + subject_basis.forward * (sz * half_d)
                    + subject_basis.up * (sy * height);
                let delta = corner - camera.position;
                let depth = delta.dot(camera_basis.forward);
                if depth <= 0.05 {
                    continue;
                }
                let x = 0.5 + 0.5 * (delta.dot(camera_basis.right) / depth) / half_sensor_w;
                let y = 0.5 - 0.5 * (delta.dot(camera_basis.up) / depth) / half_sensor_h;
                bbox.min_x = bbox.min_x.min(x);
                bbox.min_y = bbox.min_y.min(y);
                bbox.max_x = bbox.max_x.max(x);
                bbox.max_y = bbox.max_y.max(y);
                nearest = nearest.min(depth);
                count += 1;
            }
        }
    }
    (count > 0).then(|| ProjectedSubject {
        bbox,
        visible_bbox: bbox.clipped(),
        depth: nearest,
    })
}

fn compile_constraints(
    scene: &Scene,
    shot: &Shot,
    solved: &SolvedShot,
    screen_tracks: &[SubjectScreenTrack],
    phases: &[CompiledPhase],
    options: &CompileOptions,
) -> Vec<ShotConstraint> {
    let base = format!("scenes[{}].shots[{}]", scene.id, shot.id);
    let mut constraints = vec![
        ShotConstraint {
            id: "framing.shot_size".to_owned(),
            source_path: format!("{base}.framing.shot_size"),
            range: shot.range,
            tolerance: 0.05,
            kind: ShotConstraintKind::ShotSize {
                value: shot.framing.shot_size,
            },
        },
        ShotConstraint {
            id: "framing.horizontal_angle".to_owned(),
            source_path: format!("{base}.framing.horizontal_angle"),
            range: shot.range,
            tolerance: 15.0,
            kind: ShotConstraintKind::HorizontalAngle {
                value: shot.framing.horizontal_angle,
            },
        },
        ShotConstraint {
            id: "framing.vertical_angle".to_owned(),
            source_path: format!("{base}.framing.vertical_angle"),
            range: shot.range,
            tolerance: 10.0,
            kind: ShotConstraintKind::VerticalAngle {
                value: shot.framing.vertical_angle,
            },
        },
        ShotConstraint {
            id: "framing.composition".to_owned(),
            source_path: format!("{base}.framing.composition"),
            range: shot.range,
            tolerance: options.screen_center_tolerance,
            kind: ShotConstraintKind::Composition {
                value: shot.framing.composition.clone(),
            },
        },
        ShotConstraint {
            id: "framing.depth_layers".to_owned(),
            source_path: format!("{base}.framing.composition.depth_layers"),
            range: shot.range,
            tolerance: 0.05,
            kind: ShotConstraintKind::DepthLayers {
                value: shot.framing.composition.depth_layers.clone(),
            },
        },
        ShotConstraint {
            id: "framing.horizon".to_owned(),
            source_path: format!("{base}.framing.roll_deg"),
            range: shot.range,
            tolerance: options.horizon_tolerance_deg,
            kind: ShotConstraintKind::HorizonLock {
                target_roll_deg: shot.framing.roll_deg,
            },
        },
        ShotConstraint {
            id: "camera.rig".to_owned(),
            source_path: format!("{base}.camera.rig"),
            range: shot.range,
            tolerance: 0.0,
            kind: ShotConstraintKind::CameraRig {
                value: shot.camera.rig,
            },
        },
        ShotConstraint {
            id: "camera.anamorphic_squeeze".to_owned(),
            source_path: format!("{base}.camera.initial_state.lens.anamorphic_squeeze"),
            range: shot.range,
            tolerance: 1e-4,
            kind: ShotConstraintKind::AnamorphicSqueeze {
                target: shot.camera.initial_state.lens.anamorphic_squeeze,
            },
        },
        ShotConstraint {
            id: "camera.exposure".to_owned(),
            source_path: format!("{base}.camera.initial_state.exposure"),
            range: shot.range,
            tolerance: 0.0,
            kind: ShotConstraintKind::Exposure {
                value: shot.camera.initial_state.exposure.clone(),
            },
        },
        ShotConstraint {
            id: "lighting.setup".to_owned(),
            source_path: format!("{base}.lighting_setup_id"),
            range: shot.range,
            tolerance: 0.0,
            kind: ShotConstraintKind::Lighting {
                setup_id: shot.lighting_setup_id.clone(),
            },
        },
        ShotConstraint {
            id: "edit.transition_in".to_owned(),
            source_path: format!("{base}.transition"),
            range: FrameRange {
                start: shot.range.start,
                end: (shot.range.start + rich_transition(shot).duration_frames.max(1))
                    .min(shot.range.end),
            },
            tolerance: 0.0,
            kind: ShotConstraintKind::Transition {
                value: rich_transition(shot),
            },
        },
    ];
    if !shot.continuity.axis_observations.is_empty() {
        constraints.push(ShotConstraint {
            id: "continuity.axis".to_owned(),
            source_path: format!("{base}.continuity.axis_observations"),
            range: shot.range,
            tolerance: 0.0,
            kind: ShotConstraintKind::ContinuityAxis {
                observations: shot.continuity.axis_observations.clone(),
            },
        });
    }
    for phase in phases {
        if matches!(phase.kind, PhaseKind::Hold | PhaseKind::Settle) {
            constraints.push(ShotConstraint {
                id: format!("{}.no_camera_motion", phase.id),
                source_path: format!("{base}.camera.operations"),
                range: phase.edit_range,
                tolerance: 1e-4,
                kind: ShotConstraintKind::NoCameraMotion,
            });
        }
    }
    for (operation_index, timed) in shot.camera.operations.iter().enumerate() {
        let range = FrameRange {
            start: shot.range.start + timed.range.start,
            end: shot.range.start + timed.range.end,
        };
        let path = format!("{base}.camera.operations[{operation_index}]");
        match &timed.operation {
            CameraOperation::Hold => constraints.push(ShotConstraint {
                id: format!("operation_{operation_index}.hold"),
                source_path: path,
                range,
                tolerance: 1e-4,
                kind: ShotConstraintKind::NoCameraMotion,
            }),
            CameraOperation::DollyZoom {
                target: TargetRef::Subject { id },
                keep_subject_frame_fraction,
                ..
            } => {
                constraints.push(ShotConstraint {
                    id: format!("operation_{operation_index}.subject_bbox_height"),
                    source_path: format!("{path}.keep_subject_frame_fraction"),
                    range,
                    tolerance: options.subject_fraction_tolerance,
                    kind: ShotConstraintKind::SubjectBboxHeight {
                        subject_id: id.clone(),
                        target: *keep_subject_frame_fraction,
                    },
                });
                if let Some(center) = screen_frame(screen_tracks, id, range.start).map(|f| f.center)
                {
                    constraints.push(ShotConstraint {
                        id: format!("operation_{operation_index}.subject_screen_center"),
                        source_path: path.clone(),
                        range,
                        tolerance: options.screen_center_tolerance,
                        kind: ShotConstraintKind::SubjectScreenCenter {
                            subject_id: id.clone(),
                            target: center,
                        },
                    });
                }
                let background: Vec<String> = shot
                    .framing
                    .composition
                    .depth_layers
                    .iter()
                    .filter(|layer| layer.role == crate::model::DepthRole::Background)
                    .flat_map(|layer| layer.subject_ids.iter().cloned())
                    .collect();
                if background.len() >= 2 {
                    constraints.push(ShotConstraint {
                        id: format!("operation_{operation_index}.background_pair_separation"),
                        source_path: path,
                        range,
                        tolerance: 0.01,
                        kind: ShotConstraintKind::PairScreenSeparation {
                            subject_ids: [background[0].clone(), background[1].clone()],
                            trend: Trend::Increasing,
                        },
                    });
                }
            }
            CameraOperation::Reveal {
                target: TargetRef::Subject { id },
                visibility,
                preserve,
                ..
            } => {
                let start = screen_frame(screen_tracks, id, range.start)
                    .map(|frame| frame.visible_fraction)
                    .unwrap_or(0.0);
                let end = screen_frame(screen_tracks, id, range.end.saturating_sub(1))
                    .map(|frame| frame.visible_fraction)
                    .unwrap_or(start);
                let (from, to, monotonic) = visibility
                    .map(|spec| (spec.from, spec.to, spec.monotonic))
                    .unwrap_or((start, end, true));
                constraints.push(ShotConstraint {
                    id: format!("operation_{operation_index}.reveal_visibility"),
                    source_path: path,
                    range,
                    tolerance: 0.02,
                    kind: ShotConstraintKind::RevealVisibility {
                        subject_id: id.clone(),
                        from,
                        to,
                        monotonic,
                    },
                });
                if let Some(target_y) = preserve.and_then(|value| value.target_screen_y) {
                    constraints.push(ShotConstraint {
                        id: format!("operation_{operation_index}.target_screen_y"),
                        source_path: format!(
                            "{}.camera.operations[{operation_index}].preserve.target_screen_y",
                            shot.id
                        ),
                        range,
                        tolerance: options.screen_center_tolerance,
                        kind: ShotConstraintKind::SubjectScreenY {
                            subject_id: id.clone(),
                            target: target_y,
                        },
                    });
                }
            }
            CameraOperation::RackFocus { target, .. } => constraints.push(ShotConstraint {
                id: format!("operation_{operation_index}.focus_handoff"),
                source_path: path,
                range,
                tolerance: 1.0,
                kind: ShotConstraintKind::FocusHandoff {
                    target: target.clone(),
                },
            }),
            _ => {}
        }
    }
    let _ = solved;
    constraints
}

fn screen_frame<'a>(
    tracks: &'a [SubjectScreenTrack],
    subject_id: &str,
    frame: Frame,
) -> Option<&'a SubjectScreenFrame> {
    tracks
        .iter()
        .find(|track| track.subject_id == subject_id)?
        .frames
        .iter()
        .find(|value| value.frame == frame)
}

fn screen_frames<'a>(
    tracks: &'a [SubjectScreenTrack],
    subject_id: &str,
    range: FrameRange,
) -> impl Iterator<Item = &'a SubjectScreenFrame> {
    tracks
        .iter()
        .find(|track| track.subject_id == subject_id)
        .into_iter()
        .flat_map(|track| track.frames.iter())
        .filter(move |frame| frame.frame >= range.start && frame.frame < range.end)
}

fn pair_separation(
    tracks: &[SubjectScreenTrack],
    subject_ids: &[String; 2],
    frame: Frame,
) -> Option<f32> {
    let a = screen_frame(tracks, &subject_ids[0], frame)?.center;
    let b = screen_frame(tracks, &subject_ids[1], frame)?.center;
    Some(((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt())
}

fn evaluate_constraint(
    constraint: &ShotConstraint,
    source_scene: &Scene,
    source_shot: &Shot,
    shot: &SolvedShot,
    scene: &SolvedScene,
    coordinate_system: &CoordinateSystem,
    screen_tracks: &[SubjectScreenTrack],
) -> ConstraintEvaluation {
    match &constraint.kind {
        ShotConstraintKind::SubjectBboxHeight { subject_id, target } => {
            let errors: Vec<f32> = screen_frames(screen_tracks, subject_id, constraint.range)
                .map(|frame| (frame.bbox.height() - target).abs())
                .collect();
            scalar_evaluation(constraint, *target, &errors)
        }
        ShotConstraintKind::SubjectScreenCenter { subject_id, target } => {
            let errors: Vec<f32> = screen_frames(screen_tracks, subject_id, constraint.range)
                .map(|frame| {
                    ((frame.center.x - target.x).powi(2) + (frame.center.y - target.y).powi(2))
                        .sqrt()
                })
                .collect();
            scalar_evaluation(constraint, 0.0, &errors)
        }
        ShotConstraintKind::SubjectScreenY { subject_id, target } => {
            let errors: Vec<f32> = screen_frames(screen_tracks, subject_id, constraint.range)
                .map(|frame| (frame.center.y - target).abs())
                .collect();
            scalar_evaluation(constraint, *target, &errors)
        }
        ShotConstraintKind::PairScreenSeparation { subject_ids, trend } => {
            let values: Vec<f32> = (constraint.range.start..constraint.range.end)
                .filter_map(|frame| pair_separation(screen_tracks, subject_ids, frame))
                .collect();
            if values.len() < 2 {
                return indeterminate(constraint, "pair not visible for two frames");
            }
            let violations: Vec<f32> = values
                .windows(2)
                .map(|pair| match trend {
                    Trend::Increasing => (pair[0] - pair[1]).max(0.0),
                    Trend::Decreasing => (pair[1] - pair[0]).max(0.0),
                    Trend::Stable => (pair[1] - pair[0]).abs(),
                })
                .collect();
            let max_error = violations.iter().copied().fold(0.0f32, f32::max);
            ConstraintEvaluation {
                constraint_id: constraint.id.clone(),
                status: if max_error <= constraint.tolerance {
                    ConstraintStatus::Pass
                } else {
                    ConstraintStatus::Fail
                },
                requested: format!("{trend:?}"),
                actual: format!("{:.4} -> {:.4}", values[0], values[values.len() - 1]),
                max_error: Some(max_error),
            }
        }
        ShotConstraintKind::HorizonLock { target_roll_deg } => {
            let errors: Vec<f32> = shot
                .frames
                .iter()
                .filter(|frame| {
                    frame.frame >= constraint.range.start && frame.frame < constraint.range.end
                })
                .map(|frame| (frame.rotation_deg.roll - target_roll_deg).abs())
                .collect();
            scalar_evaluation(constraint, *target_roll_deg, &errors)
        }
        ShotConstraintKind::NoCameraMotion => {
            let frames: Vec<&SolvedCameraFrame> = shot
                .frames
                .iter()
                .filter(|frame| {
                    frame.frame >= constraint.range.start && frame.frame < constraint.range.end
                })
                .collect();
            let errors: Vec<f32> = frames
                .windows(2)
                .map(|pair| {
                    (pair[1].position - pair[0].position)
                        .length()
                        .max((pair[1].focal_length_mm - pair[0].focal_length_mm).abs() / 1000.0)
                })
                .collect();
            scalar_evaluation(constraint, 0.0, &errors)
        }
        ShotConstraintKind::RevealVisibility {
            subject_id,
            from,
            to,
            monotonic,
        } => {
            let values: Vec<f32> = screen_frames(screen_tracks, subject_id, constraint.range)
                .map(|frame| frame.visible_fraction)
                .collect();
            if values.is_empty() {
                return indeterminate(constraint, "subject is never projected");
            }
            let mut error = (values[0] - from)
                .abs()
                .max((values[values.len() - 1] - to).abs());
            if *monotonic {
                error = error.max(
                    values
                        .windows(2)
                        .map(|pair| (pair[0] - pair[1]).max(0.0))
                        .fold(0.0f32, f32::max),
                );
            }
            ConstraintEvaluation {
                constraint_id: constraint.id.clone(),
                status: if error <= constraint.tolerance {
                    ConstraintStatus::Pass
                } else {
                    ConstraintStatus::Fail
                },
                requested: format!("{from:.3} -> {to:.3}, monotonic={monotonic}"),
                actual: format!("{:.3} -> {:.3}", values[0], values[values.len() - 1]),
                max_error: Some(error),
            }
        }
        ShotConstraintKind::FocusHandoff { target } => {
            let actual = shot
                .frames
                .iter()
                .find(|frame| {
                    frame.frame >= constraint.range.start
                        && frame.frame < constraint.range.end
                        && frame.focus_target.as_ref() == Some(target)
                })
                .map(|frame| frame.frame);
            ConstraintEvaluation {
                constraint_id: constraint.id.clone(),
                status: if actual.is_some() {
                    ConstraintStatus::Pass
                } else {
                    ConstraintStatus::Fail
                },
                requested: format!("focus {target:?}"),
                actual: actual.map_or_else(|| "not reached".to_owned(), |f| format!("frame {f}")),
                max_error: actual.map(|_| 0.0),
            }
        }
        ShotConstraintKind::ShotSize { value } => {
            let Some(subject_id) = source_shot.framing.subject_ids.first() else {
                return indeterminate(constraint, "shot has no primary subject");
            };
            let values: Vec<_> = screen_frames(screen_tracks, subject_id, constraint.range)
                .map(|frame| frame.bbox.height())
                .collect();
            let Some(actual) =
                (!values.is_empty()).then(|| values.iter().sum::<f32>() / values.len() as f32)
            else {
                return indeterminate(constraint, "primary subject is not projected");
            };
            let bounds = shot_size_bounds(*value);
            let Some((minimum, maximum)) = bounds else {
                return semantic_pass(constraint, format!("custom shot size; actual {actual:.3}"));
            };
            let error = if actual < minimum {
                minimum - actual
            } else if actual > maximum {
                actual - maximum
            } else {
                0.0
            };
            measured_evaluation(
                constraint,
                format!("bbox height {minimum:.3}..{maximum:.3}"),
                format!("mean bbox height {actual:.3}"),
                error,
            )
        }
        ShotConstraintKind::HorizontalAngle { value } => {
            let Some((actual, _)) = framing_angles(
                source_shot,
                shot,
                scene,
                coordinate_system,
                constraint.range,
            ) else {
                return indeterminate(constraint, "primary subject is unavailable");
            };
            let Some(target) = horizontal_target(*value) else {
                return semantic_pass(constraint, format!("authored {value:?}"));
            };
            measured_evaluation(
                constraint,
                format!("{target:.1} degrees"),
                format!("{actual:.1} degrees"),
                (actual - target).abs(),
            )
        }
        ShotConstraintKind::VerticalAngle { value } => {
            let Some((_, actual)) = framing_angles(
                source_shot,
                shot,
                scene,
                coordinate_system,
                constraint.range,
            ) else {
                return indeterminate(constraint, "primary subject is unavailable");
            };
            let Some(target) = vertical_target(*value) else {
                return semantic_pass(constraint, format!("authored {value:?}"));
            };
            measured_evaluation(
                constraint,
                format!("{target:.1} degrees"),
                format!("{actual:.1} degrees"),
                (actual - target).abs(),
            )
        }
        ShotConstraintKind::Composition { value } => {
            let centers: Vec<_> = source_shot
                .framing
                .subject_ids
                .iter()
                .filter_map(|id| screen_frame(screen_tracks, id, constraint.range.start))
                .map(|frame| frame.center)
                .collect();
            if centers.is_empty() {
                return indeterminate(constraint, "framed subjects are not projected");
            }
            use crate::model::CompositionStrategy;
            let error = match value.strategy {
                CompositionStrategy::Centered | CompositionStrategy::Symmetric => {
                    let center =
                        centers.iter().map(|value| value.x).sum::<f32>() / centers.len() as f32;
                    (center - 0.5).abs()
                }
                CompositionStrategy::RuleOfThirds => centers
                    .iter()
                    .map(|center| {
                        [(center.x - 1.0 / 3.0).abs(), (center.x - 2.0 / 3.0).abs()]
                            .into_iter()
                            .fold(f32::INFINITY, f32::min)
                    })
                    .fold(0.0f32, f32::max),
                CompositionStrategy::Asymmetric => {
                    let center =
                        centers.iter().map(|value| value.x).sum::<f32>() / centers.len() as f32;
                    (constraint.tolerance - (center - 0.5).abs()).max(0.0)
                }
                _ => return semantic_pass(constraint, format!("authored {:?}", value.strategy)),
            };
            measured_evaluation(
                constraint,
                format!("{:?}", value.strategy),
                format!("{} subject center(s)", centers.len()),
                error,
            )
        }
        ShotConstraintKind::DepthLayers { value } => {
            if value.len() < 2 {
                return semantic_pass(constraint, "zero or one depth layer".to_owned());
            }
            let frame = constraint.range.start;
            let depths: Vec<_> = value
                .iter()
                .filter_map(|layer| {
                    let values: Vec<_> = layer
                        .subject_ids
                        .iter()
                        .filter_map(|id| screen_frame(screen_tracks, id, frame)?.depth_m)
                        .collect();
                    (!values.is_empty()).then(|| values.iter().sum::<f32>() / values.len() as f32)
                })
                .collect();
            if depths.len() != value.len() {
                return indeterminate(constraint, "a declared depth layer is not projected");
            }
            let error = depths
                .windows(2)
                .map(|pair| (pair[0] + constraint.tolerance - pair[1]).max(0.0))
                .fold(0.0f32, f32::max);
            measured_evaluation(
                constraint,
                "foreground < midground < background".to_owned(),
                format!("depths {depths:?}"),
                error,
            )
        }
        ShotConstraintKind::ContinuityAxis { observations } => {
            let context = crate::solve::SceneContext {
                scene: source_scene,
                cs: coordinate_system,
                fps: 24.0,
                subjects: scene.subjects.clone(),
            };
            let camera = shot.frame_at(constraint.range.start);
            let mut mismatches = 0.0f32;
            let mut compared = 0usize;
            if let Some(camera) = camera {
                for observation in observations {
                    let Some(axis) = source_scene
                        .axes
                        .iter()
                        .find(|axis| axis.id == observation.axis_id)
                    else {
                        continue;
                    };
                    let (Some(from), Some(to)) = (
                        context.resolve(&axis.from, constraint.range.start),
                        context.resolve(&axis.to, constraint.range.start),
                    ) else {
                        continue;
                    };
                    let Some(side) =
                        crate::math::side_of_axis(coordinate_system, from, to, camera.position)
                    else {
                        continue;
                    };
                    compared += 1;
                    let matches = match observation.side {
                        crate::model::AxisSide::Positive => side > 0.0,
                        crate::model::AxisSide::Negative => side < 0.0,
                        crate::model::AxisSide::OnAxis => side.abs() <= 1e-4,
                    };
                    if !matches {
                        mismatches += 1.0;
                    }
                }
            }
            if compared == 0 {
                return indeterminate(constraint, "no axis observation could be measured");
            }
            measured_evaluation(
                constraint,
                format!("{} axis observation(s)", observations.len()),
                format!("{compared} measured"),
                mismatches,
            )
        }
        ShotConstraintKind::CameraRig { value } => {
            semantic_pass(constraint, format!("compiled rig {value:?}"))
        }
        ShotConstraintKind::AnamorphicSqueeze { target } => {
            let actual = source_shot.camera.initial_state.lens.anamorphic_squeeze;
            measured_evaluation(
                constraint,
                format!("{target:.3}x"),
                format!("{actual:.3}x"),
                (actual - target).abs(),
            )
        }
        ShotConstraintKind::Exposure { value } => semantic_pass(
            constraint,
            format!(
                "ISO {}, shutter {:.1}, white balance {}K, ND {:.1}",
                value.iso, value.shutter_angle_deg, value.white_balance_k, value.nd_stops
            ),
        ),
        ShotConstraintKind::Lighting { setup_id } => semantic_pass(
            constraint,
            setup_id.as_ref().map_or_else(
                || "no authored setup".to_owned(),
                |id| format!("setup {id}"),
            ),
        ),
        ShotConstraintKind::Transition { value } => semantic_pass(
            constraint,
            format!("{:?}, {} frame(s)", value.kind, value.duration_frames),
        ),
    }
}

fn scalar_evaluation(
    constraint: &ShotConstraint,
    requested: f32,
    errors: &[f32],
) -> ConstraintEvaluation {
    if errors.is_empty() {
        return indeterminate(constraint, "no samples in constraint range");
    }
    let max_error = errors.iter().copied().fold(0.0f32, f32::max);
    ConstraintEvaluation {
        constraint_id: constraint.id.clone(),
        status: if max_error <= constraint.tolerance {
            ConstraintStatus::Pass
        } else {
            ConstraintStatus::Fail
        },
        requested: format!("{requested:.4} ± {:.4}", constraint.tolerance),
        actual: format!("max error {max_error:.4}"),
        max_error: Some(max_error),
    }
}

fn measured_evaluation(
    constraint: &ShotConstraint,
    requested: String,
    actual: String,
    error: f32,
) -> ConstraintEvaluation {
    ConstraintEvaluation {
        constraint_id: constraint.id.clone(),
        status: if error <= constraint.tolerance {
            ConstraintStatus::Pass
        } else {
            ConstraintStatus::Fail
        },
        requested,
        actual,
        max_error: Some(error),
    }
}

fn semantic_pass(constraint: &ShotConstraint, actual: String) -> ConstraintEvaluation {
    ConstraintEvaluation {
        constraint_id: constraint.id.clone(),
        status: ConstraintStatus::Pass,
        requested: format!("{:?}", constraint.kind),
        actual,
        max_error: Some(0.0),
    }
}

pub(crate) fn shot_size_bounds(value: ShotSize) -> Option<(f32, f32)> {
    match value {
        ShotSize::ExtremeLong => Some((0.02, 0.18)),
        ShotSize::Long => Some((0.10, 0.32)),
        ShotSize::MediumLong => Some((0.22, 0.48)),
        ShotSize::Medium => Some((0.32, 0.62)),
        ShotSize::MediumCloseUp => Some((0.45, 0.76)),
        ShotSize::CloseUp => Some((0.62, 1.00)),
        ShotSize::ExtremeCloseUp => Some((0.82, 1.60)),
        ShotSize::Insert | ShotSize::Custom => None,
    }
}

/// Average horizontal and vertical viewing angles for the primary framed
/// subject. Horizontal angle is measured from the subject's authored forward
/// vector; vertical angle is camera elevation relative to the subject aim
/// point. Both are observable values derived from the solved camera track.
fn framing_angles(
    source_shot: &Shot,
    shot: &SolvedShot,
    scene: &SolvedScene,
    coordinate_system: &CoordinateSystem,
    range: FrameRange,
) -> Option<(f32, f32)> {
    let subject_id = source_shot.framing.subject_ids.first()?;
    let subject = scene
        .subjects
        .iter()
        .find(|track| &track.subject_id == subject_id)?;
    let world_up = crate::math::identity_basis(coordinate_system).up;
    let mut horizontal = 0.0;
    let mut vertical = 0.0;
    let mut samples = 0usize;
    for camera in shot
        .frames
        .iter()
        .filter(|frame| frame.frame >= range.start && frame.frame < range.end)
    {
        let transform = subject.transform_at(camera.frame);
        let basis = oriented_basis(coordinate_system, transform.rotation_deg);
        let aim = transform.position
            + world_up
                * (subject.dimensions_m.y
                    * crate::solve::sampler::aim_height_fraction(subject.kind)
                    * transform.scale.y);
        let to_camera = camera.position - aim;
        let elevation = to_camera.dot(world_up);
        let planar = to_camera - world_up * elevation;
        let Some(planar_direction) = planar.normalized() else {
            continue;
        };
        let subject_forward = basis.forward - world_up * basis.forward.dot(world_up);
        let Some(subject_forward) = subject_forward.normalized() else {
            continue;
        };
        horizontal += subject_forward
            .dot(planar_direction)
            .clamp(-1.0, 1.0)
            .acos()
            .to_degrees();
        vertical += elevation.atan2(planar.length()).to_degrees();
        samples += 1;
    }
    (samples > 0).then(|| (horizontal / samples as f32, vertical / samples as f32))
}

fn horizontal_target(value: HorizontalAngle) -> Option<f32> {
    match value {
        HorizontalAngle::Frontal | HorizontalAngle::PointOfView => Some(0.0),
        HorizontalAngle::ThreeQuarter => Some(45.0),
        HorizontalAngle::Profile => Some(90.0),
        HorizontalAngle::RearThreeQuarter => Some(135.0),
        HorizontalAngle::Rear => Some(180.0),
        HorizontalAngle::OverTheShoulder | HorizontalAngle::Custom => None,
    }
}

fn vertical_target(value: VerticalAngle) -> Option<f32> {
    match value {
        VerticalAngle::Overhead => Some(75.0),
        VerticalAngle::High => Some(25.0),
        VerticalAngle::EyeLevel => Some(0.0),
        VerticalAngle::Low => Some(-20.0),
        VerticalAngle::WormsEye => Some(-70.0),
        VerticalAngle::Custom => None,
    }
}

fn indeterminate(constraint: &ShotConstraint, reason: &str) -> ConstraintEvaluation {
    ConstraintEvaluation {
        constraint_id: constraint.id.clone(),
        status: ConstraintStatus::Indeterminate,
        requested: format!("{:?}", constraint.kind),
        actual: reason.to_owned(),
        max_error: None,
    }
}

impl CompiledProject {
    pub fn scene(&self, id: &str) -> Option<&CompiledScene> {
        self.scenes.iter().find(|scene| scene.id == id)
    }
}

impl CompiledScene {
    pub fn shot(&self, id: &str) -> Option<&CompiledShot> {
        self.shots.iter().find(|shot| shot.id == id)
    }

    pub fn shot_at(&self, frame: Frame) -> Option<&CompiledShot> {
        self.shots
            .iter()
            .find(|shot| frame >= shot.edit_range.start && frame < shot.edit_range.end)
    }

    pub fn subject(&self, id: &str) -> Option<&SolvedSubjectTrack> {
        self.subjects
            .iter()
            .find(|subject| subject.subject_id == id)
    }
}

impl CompiledShot {
    pub fn frame_at(&self, frame: Frame) -> Option<&SolvedCameraFrame> {
        if self.camera_track.frames.is_empty() {
            return None;
        }
        let index = frame.saturating_sub(self.edit_range.start) as usize;
        Some(&self.camera_track.frames[index.min(self.camera_track.frames.len() - 1)])
    }

    pub fn screen_track(&self, subject_id: &str) -> Option<&SubjectScreenTrack> {
        self.screen_tracks
            .iter()
            .find(|track| track.subject_id == subject_id)
    }
}
