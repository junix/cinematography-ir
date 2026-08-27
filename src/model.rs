use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub type Frame = u64;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CineProject {
    pub schema_version: String,
    pub id: String,
    pub title: String,
    pub frame_rate: FrameRate,
    #[serde(default)]
    pub coordinate_system: CoordinateSystem,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
    pub scenes: Vec<Scene>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FrameRate {
    pub numerator: u32,
    #[serde(default = "one_u32")]
    pub denominator: u32,
}

impl FrameRate {
    pub fn frames_per_second(self) -> Option<f64> {
        (self.numerator > 0 && self.denominator > 0)
            .then(|| self.numerator as f64 / self.denominator as f64)
    }
}

/// World coordinate conventions carried by every document.
///
/// Orientation semantics derived from these axes (used by the solver, views,
/// and adapters — never assume a backend's convention):
///
/// * A camera or subject with identity `rotation_deg` looks along
///   `forward_axis` with `up_axis` as its up vector. Its right vector is
///   `forward × up` for right-handed systems and `up × forward` for
///   left-handed systems.
/// * `yaw` rotates about `up_axis`, `pitch` about the right vector, `roll`
///   about `forward_axis`; rotations compose as yaw, then pitch, then roll.
/// * Positive rotations follow the physical right-hand rule about those
///   vectors in every system, so `+yaw` always turns toward the camera's left
///   and `+pitch` always tilts up regardless of which axes are chosen. This
///   deliberately differs from Unity's Euler angles (which are left-hand-rule
///   in its left-handed frame); adapters importing or exporting Unity data
///   must negate the angles.
/// * `forward_axis` must not be parallel to `up_axis` (validated).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CoordinateSystem {
    #[serde(default)]
    pub units: Unit,
    #[serde(default)]
    pub handedness: Handedness,
    #[serde(default)]
    pub up_axis: AxisName,
    #[serde(default = "default_forward_axis")]
    pub forward_axis: SignedAxis,
}

impl Default for CoordinateSystem {
    fn default() -> Self {
        Self {
            units: Unit::Meters,
            handedness: Handedness::Right,
            up_axis: AxisName::Y,
            forward_axis: SignedAxis::NegativeZ,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    #[default]
    Meters,
    Centimeters,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Handedness {
    Left,
    #[default]
    Right,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AxisName {
    X,
    #[default]
    Y,
    Z,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SignedAxis {
    PositiveX,
    NegativeX,
    PositiveY,
    NegativeY,
    PositiveZ,
    NegativeZ,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Scene {
    pub id: String,
    pub title: String,
    pub duration_frames: Frame,
    #[serde(default)]
    pub narrative: NarrativeIntent,
    #[serde(default)]
    pub subjects: Vec<Subject>,
    #[serde(default)]
    pub markers: Vec<Marker>,
    #[serde(default)]
    pub axes: Vec<ContinuityAxis>,
    #[serde(default)]
    pub blocking: Vec<BlockingTrack>,
    #[serde(default)]
    pub beats: Vec<Beat>,
    #[serde(default)]
    pub lighting_setups: Vec<LightingSetup>,
    /// Legacy final-edit shots. New coverage workflows may additionally
    /// declare overlapping `takes` and map them through `edit_timeline`.
    #[serde(default)]
    pub shots: Vec<Shot>,
    #[serde(default)]
    pub takes: Vec<Take>,
    #[serde(default)]
    pub edit_timeline: Vec<EditClip>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct NarrativeIntent {
    #[serde(default)]
    pub dramatic_question: Option<String>,
    #[serde(default)]
    pub themes: Vec<String>,
    #[serde(default)]
    pub desired_audience_state: Vec<String>,
    #[serde(default)]
    pub emotional_arc: Vec<EmotionalBeat>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EmotionalBeat {
    pub frame: Frame,
    pub label: String,
    #[serde(default)]
    pub valence: Option<f32>,
    #[serde(default)]
    pub arousal: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Subject {
    pub id: String,
    pub name: String,
    pub kind: SubjectKind,
    /// `position` is the subject's base point (ground contact for characters,
    /// vehicles, and props), not its centre. Blocking keyframes use the same
    /// convention.
    #[serde(default)]
    pub initial_transform: Transform,
    /// Bounding size in the subject's local space, in metres, independent of
    /// `coordinate_system.up_axis`: `x` = width (left–right), `y` = height
    /// (base to top), `z` = depth (front–back). When absent, solvers and views
    /// substitute a per-`kind` default (e.g. 0.5 × 1.75 × 0.35 for characters).
    #[serde(default)]
    pub dimensions_m: Option<Vec3>,
    #[serde(default)]
    pub geometry_proxy: Option<GeometryProxy>,
    #[serde(default)]
    pub pose_track: Option<PoseTrack>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SubjectKind {
    Character,
    Prop,
    Vehicle,
    Environment,
    Screen,
    Abstract,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Marker {
    pub id: String,
    pub name: String,
    pub position: Vec3,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ContinuityAxis {
    pub id: String,
    pub name: String,
    pub purpose: AxisPurpose,
    pub from: TargetRef,
    pub to: TargetRef,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AxisPurpose {
    Relationship,
    Movement,
    Eyeline,
    Geography,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BlockingTrack {
    pub subject_id: String,
    pub keyframes: Vec<BlockingKeyframe>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BlockingKeyframe {
    pub frame: Frame,
    pub transform: Transform,
    #[serde(default)]
    pub gaze_target: Option<TargetRef>,
    #[serde(default)]
    pub action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Beat {
    pub id: String,
    pub range: FrameRange,
    pub intent: String,
    #[serde(default)]
    pub emphasis: BeatEmphasis,
    #[serde(default)]
    pub subject_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum BeatEmphasis {
    Low,
    #[default]
    Medium,
    High,
    Climax,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Shot {
    pub id: String,
    pub range: FrameRange,
    #[serde(default)]
    pub purpose: Vec<ShotPurpose>,
    #[serde(default)]
    pub coverage_role: CoverageRole,
    pub framing: Framing,
    pub camera: CameraPlan,
    #[serde(default)]
    pub lighting_setup_id: Option<String>,
    #[serde(default)]
    pub continuity: ContinuityAnnotations,
    #[serde(default)]
    pub transition_in: Transition,
    /// Rich transition contract. When absent, `transition_in` remains the
    /// backward-compatible source of transition kind.
    #[serde(default)]
    pub transition: Option<TransitionSpec>,
    #[serde(default)]
    pub relations: Vec<ShotRelation>,
    /// Optional authored higher-level cinematography phrases. The compiler
    /// also infers phrases from operations without replacing authored ones.
    #[serde(default)]
    pub phrases: Vec<ShotPhrase>,
    #[serde(default)]
    pub prohibit: Vec<Prohibition>,
    #[serde(default)]
    pub notes: Vec<String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

impl Shot {
    pub fn duration_frames(&self) -> Frame {
        self.range.duration()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ShotPurpose {
    Establish,
    Orient,
    Reveal,
    Emphasize,
    Observe,
    Identify,
    Connect,
    Isolate,
    Subjective,
    Reaction,
    Transition,
    Disorient,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CoverageRole {
    Master,
    TwoShot,
    OverTheShoulder,
    Single,
    CloseUp,
    PointOfView,
    Reaction,
    Insert,
    Cutaway,
    Detail,
    Establishing,
    #[default]
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Framing {
    pub shot_size: ShotSize,
    #[serde(default)]
    pub horizontal_angle: HorizontalAngle,
    #[serde(default)]
    pub vertical_angle: VerticalAngle,
    #[serde(default)]
    pub roll_deg: f32,
    #[serde(default)]
    pub subject_ids: Vec<String>,
    #[serde(default)]
    pub composition: Composition,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ShotSize {
    ExtremeLong,
    Long,
    MediumLong,
    Medium,
    MediumCloseUp,
    CloseUp,
    ExtremeCloseUp,
    Insert,
    Custom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum HorizontalAngle {
    Frontal,
    ThreeQuarter,
    Profile,
    RearThreeQuarter,
    Rear,
    PointOfView,
    OverTheShoulder,
    #[default]
    Custom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum VerticalAngle {
    Overhead,
    High,
    #[default]
    EyeLevel,
    Low,
    WormsEye,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct Composition {
    #[serde(default)]
    pub strategy: CompositionStrategy,
    #[serde(default)]
    pub headroom: Option<f32>,
    #[serde(default)]
    pub lead_room: Option<f32>,
    #[serde(default)]
    pub negative_space: Option<ScreenRegion>,
    #[serde(default)]
    pub depth_layers: Vec<DepthLayer>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CompositionStrategy {
    Centered,
    RuleOfThirds,
    Symmetric,
    Asymmetric,
    GoldenRatio,
    Diagonal,
    FrameWithinFrame,
    DeepFocus,
    Flat,
    #[default]
    Natural,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScreenRegion {
    Left,
    Right,
    Top,
    Bottom,
    Center,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DepthLayer {
    pub name: String,
    pub role: DepthRole,
    #[serde(default)]
    pub subject_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DepthRole {
    Foreground,
    Midground,
    Background,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CameraPlan {
    #[serde(default)]
    pub rig: CameraRig,
    pub initial_state: CameraState,
    #[serde(default)]
    pub operations: Vec<TimedCameraOperation>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CameraRig {
    #[default]
    Locked,
    Tripod,
    Dolly,
    Slider,
    Jib,
    Crane,
    Handheld,
    Shoulder,
    Steadicam,
    Gimbal,
    Drone,
    VehicleMount,
    Virtual,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CameraState {
    pub transform: Transform,
    #[serde(default)]
    pub lens: LensState,
    #[serde(default)]
    pub focus: FocusState,
    #[serde(default)]
    pub exposure: ExposureState,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LensState {
    #[serde(default = "default_focal_length")]
    pub focal_length_mm: f32,
    #[serde(default = "default_sensor_width")]
    pub sensor_width_mm: f32,
    #[serde(default = "default_aperture")]
    pub aperture_f: f32,
    #[serde(default = "one_f32")]
    pub anamorphic_squeeze: f32,
}

impl Default for LensState {
    fn default() -> Self {
        Self {
            focal_length_mm: default_focal_length(),
            sensor_width_mm: default_sensor_width(),
            aperture_f: default_aperture(),
            anamorphic_squeeze: one_f32(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct FocusState {
    #[serde(default)]
    pub target: Option<TargetRef>,
    #[serde(default)]
    pub distance_m: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExposureState {
    #[serde(default = "default_iso")]
    pub iso: u32,
    #[serde(default = "default_shutter_angle")]
    pub shutter_angle_deg: f32,
    #[serde(default = "default_white_balance")]
    pub white_balance_k: u32,
    #[serde(default)]
    pub nd_stops: f32,
}

impl Default for ExposureState {
    fn default() -> Self {
        Self {
            iso: default_iso(),
            shutter_angle_deg: default_shutter_angle(),
            white_balance_k: default_white_balance(),
            nd_stops: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TimedCameraOperation {
    /// Shot-local, half-open frame range: [start, end).
    pub range: FrameRange,
    #[serde(default)]
    pub easing: Easing,
    pub operation: CameraOperation,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum CameraOperation {
    Hold,
    Pan {
        degrees: f32,
    },
    Tilt {
        degrees: f32,
    },
    Roll {
        degrees: f32,
    },
    Dolly {
        distance_m: f32,
    },
    Truck {
        distance_m: f32,
    },
    Pedestal {
        distance_m: f32,
    },
    Translate {
        delta: Vec3,
        #[serde(default)]
        space: TransformSpace,
    },
    Rotate {
        delta_deg: EulerDeg,
        #[serde(default)]
        space: TransformSpace,
    },
    LookAt {
        target: TargetRef,
    },
    Orbit {
        target: TargetRef,
        azimuth_deg: f32,
        #[serde(default)]
        elevation_deg: f32,
        #[serde(default)]
        radius_m: Option<f32>,
    },
    Follow {
        target: TargetRef,
        offset: Vec3,
        /// Time-domain lag. When present this is independent of frame rate.
        #[serde(default)]
        lag_half_life_s: Option<f32>,
        /// Backward-compatible 24 fps response coefficient. New documents
        /// should use `lag_half_life_s`.
        #[serde(default = "default_damping")]
        damping: f32,
    },
    Crane {
        delta: Vec3,
        #[serde(default)]
        look_at: Option<TargetRef>,
    },
    Zoom {
        to_focal_length_mm: f32,
    },
    RackFocus {
        target: TargetRef,
        #[serde(default)]
        to_distance_m: Option<f32>,
    },
    DollyZoom {
        target: TargetRef,
        to_distance_m: f32,
        #[serde(default = "default_subject_fraction")]
        keep_subject_frame_fraction: f32,
    },
    HandheldNoise {
        translation_amplitude_m: f32,
        rotation_amplitude_deg: f32,
        frequency_hz: f32,
        seed: u64,
    },
    Reveal {
        target: TargetRef,
        occluder: TargetRef,
        lateral_distance_m: f32,
        #[serde(default)]
        visibility: Option<RevealVisibilitySpec>,
        #[serde(default)]
        preserve: Option<RevealPreserveSpec>,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RevealVisibilitySpec {
    pub from: f32,
    pub to: f32,
    #[serde(default = "default_true")]
    pub monotonic: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct RevealPreserveSpec {
    #[serde(default)]
    pub target_screen_y: Option<f32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TransformSpace {
    World,
    #[default]
    CameraLocal,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Easing {
    Linear,
    EaseIn,
    EaseOut,
    #[default]
    EaseInOut,
    SmoothStep,
    Hold,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct ContinuityAnnotations {
    #[serde(default)]
    pub axis_observations: Vec<AxisObservation>,
    #[serde(default)]
    pub screen_directions: Vec<ScreenDirectionObservation>,
    #[serde(default)]
    pub eyelines: Vec<EyelineObservation>,
    #[serde(default)]
    pub camera_azimuth_deg: Option<f32>,
    #[serde(default)]
    pub bridge: Option<ContinuityBridge>,
    #[serde(default)]
    pub spatial_reset: bool,
    #[serde(default)]
    pub direction_change_visible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AxisObservation {
    pub axis_id: String,
    pub side: AxisSide,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AxisSide {
    Positive,
    Negative,
    OnAxis,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ScreenDirectionObservation {
    pub subject_id: String,
    pub direction: ScreenDirection,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScreenDirection {
    Left,
    Right,
    Stationary,
    TowardCamera,
    AwayFromCamera,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EyelineObservation {
    pub subject_id: String,
    pub target: TargetRef,
    pub screen_direction: HorizontalDirection,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HorizontalDirection {
    Left,
    Right,
    Center,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContinuityBridge {
    VisibleAxisCross,
    OnAxisShot,
    NeutralInsert,
    ReestablishingShot,
    CharacterReblocking,
    DeliberateDisorientation,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Transition {
    #[default]
    Cut,
    Dissolve,
    Fade,
    Wipe,
    MatchCut,
    JumpCut,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TransitionSpec {
    pub kind: Transition,
    #[serde(default)]
    pub duration_frames: Frame,
    #[serde(default)]
    pub curve: Easing,
    #[serde(default)]
    pub match_anchor: Option<MatchAnchor>,
    #[serde(default)]
    pub direction: Option<TransitionDirection>,
}

impl TransitionSpec {
    pub fn cut(kind: Transition) -> Self {
        Self {
            kind,
            duration_frames: 0,
            curve: Easing::Hold,
            match_anchor: None,
            direction: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MatchAnchor {
    #[serde(default)]
    pub subject_id: Option<String>,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub source_frame: Option<Frame>,
    #[serde(default)]
    pub target_frame: Option<Frame>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransitionDirection {
    Left,
    Right,
    Up,
    Down,
    In,
    Out,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ShotRelation {
    pub kind: ShotRelationKind,
    pub shot_id: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ShotRelationKind {
    Establishes,
    NarrowsFrom,
    ReverseOf,
    ReactionTo,
    RevealsFrom,
    MatchesActionFrom,
    CrossesAxisFrom,
    ReestablishesAfter,
}

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord,
)]
#[serde(rename_all = "snake_case")]
pub enum ShotPhrase {
    PressurePushIn,
    SubjectLockDollyZoom,
    ForegroundParallaxReveal,
    VisibleAxisCross,
    RackFocusReveal,
    ReactionHold,
    WalkAndTalkFollow,
    OrbitRelationshipShift,
}

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord,
)]
#[serde(rename_all = "snake_case")]
pub enum Prohibition {
    ActorTranslationAsCameraSubstitute,
    DigitalCropAsDollySubstitute,
    BackgroundDeformationAsParallaxSubstitute,
    GlobalBlurAsRackFocusSubstitute,
    HandheldMotion,
    UnplannedCut,
}

/// Camera coverage over performance/story time. Takes may overlap.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Take {
    pub id: String,
    pub performance_range: FrameRange,
    #[serde(default)]
    pub purpose: Vec<ShotPurpose>,
    #[serde(default)]
    pub coverage_role: CoverageRole,
    pub framing: Framing,
    pub camera_program: CameraPlan,
    #[serde(default)]
    pub lighting_setup_id: Option<String>,
    #[serde(default)]
    pub continuity: ContinuityAnnotations,
    #[serde(default)]
    pub phrases: Vec<ShotPhrase>,
    #[serde(default)]
    pub notes: Vec<String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

/// Editorial selection from a take. `source_range` is in performance time;
/// `edit_range` is in final-edit time and may overlap for transitions.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EditClip {
    pub id: String,
    pub take_id: String,
    pub source_range: FrameRange,
    pub edit_range: FrameRange,
    #[serde(default = "default_cut_transition")]
    pub transition_in: TransitionSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GeometryProxy {
    Box,
    CapsuleRig,
    Plane,
    Wall,
    Doorway { width_m: f32, height_m: f32 },
    ConvexHull { points: Vec<Vec3> },
    MeshRef { uri: String },
    SilhouetteCard,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct PoseTrack {
    #[serde(default)]
    pub frames: Vec<PoseFrame>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PoseFrame {
    pub frame: Frame,
    #[serde(default)]
    pub joints: BTreeMap<String, Vec3>,
    #[serde(default)]
    pub gaze: Option<TargetRef>,
    #[serde(default)]
    pub contacts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LightingSetup {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub contrast_ratio: Option<f32>,
    #[serde(default)]
    pub sources: Vec<LightSource>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LightSource {
    pub id: String,
    pub kind: LightKind,
    pub role: LightRole,
    pub transform: Transform,
    #[serde(default)]
    pub intensity_lux: Option<f32>,
    #[serde(default = "default_white_balance")]
    pub color_temperature_k: u32,
    #[serde(default)]
    pub motivated_by: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LightKind {
    Point,
    Spot,
    Area,
    Directional,
    Practical,
    Environment,
    EmissiveSurface,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LightRole {
    Key,
    Fill,
    Back,
    Rim,
    Background,
    Practical,
    Ambient,
    Motivated,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum TargetRef {
    Subject { id: String },
    Marker { id: String },
    Point { position: Vec3 },
}

impl TargetRef {
    pub fn subject_id(&self) -> Option<&str> {
        match self {
            Self::Subject { id } => Some(id),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Transform {
    #[serde(default)]
    pub position: Vec3,
    #[serde(default)]
    pub rotation_deg: EulerDeg,
    #[serde(default = "default_scale")]
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec3::default(),
            rotation_deg: EulerDeg::default(),
            scale: default_scale(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct Vec3 {
    #[serde(default)]
    pub x: f32,
    #[serde(default)]
    pub y: f32,
    #[serde(default)]
    pub z: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct EulerDeg {
    #[serde(default)]
    pub pitch: f32,
    #[serde(default)]
    pub yaw: f32,
    #[serde(default)]
    pub roll: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FrameRange {
    pub start: Frame,
    pub end: Frame,
}

impl FrameRange {
    pub fn is_valid(self) -> bool {
        self.start < self.end
    }

    pub fn duration(self) -> Frame {
        self.end.saturating_sub(self.start)
    }
}

fn one_u32() -> u32 {
    1
}

fn one_f32() -> f32 {
    1.0
}

fn default_forward_axis() -> SignedAxis {
    SignedAxis::NegativeZ
}

fn default_scale() -> Vec3 {
    Vec3 {
        x: 1.0,
        y: 1.0,
        z: 1.0,
    }
}

fn default_focal_length() -> f32 {
    50.0
}

fn default_sensor_width() -> f32 {
    36.0
}

fn default_aperture() -> f32 {
    2.8
}

fn default_iso() -> u32 {
    800
}

fn default_shutter_angle() -> f32 {
    180.0
}

fn default_white_balance() -> u32 {
    5600
}

fn default_damping() -> f32 {
    0.2
}

fn default_subject_fraction() -> f32 {
    0.4
}

fn default_true() -> bool {
    true
}

fn default_cut_transition() -> TransitionSpec {
    TransitionSpec::cut(Transition::Cut)
}
