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
    pub shots: Vec<Shot>,
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
    #[serde(default)]
    pub initial_transform: Transform,
    #[serde(default)]
    pub dimensions_m: Option<Vec3>,
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
    Pan { degrees: f32 },
    Tilt { degrees: f32 },
    Roll { degrees: f32 },
    Dolly { distance_m: f32 },
    Truck { distance_m: f32 },
    Pedestal { distance_m: f32 },
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
    LookAt { target: TargetRef },
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
        #[serde(default = "default_damping")]
        damping: f32,
    },
    Crane {
        delta: Vec3,
        #[serde(default)]
        look_at: Option<TargetRef>,
    },
    Zoom { to_focal_length_mm: f32 },
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
    },
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
