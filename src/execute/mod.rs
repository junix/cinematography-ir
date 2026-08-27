//! Execute layer (ADR-1115 D6/D12): adapters that drive external runtimes
//! from Compiled Guidance IR. Impure by nature — they write files and spawn
//! processes — and therefore isolated from `view` and `prompt`.

pub mod blender;

use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::compiled::{CompiledProject, CompiledScene};
use crate::model::FrameRange;
use crate::solve::{SolvedProject, SolvedScene};

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord,
)]
#[serde(rename_all = "snake_case")]
pub enum PassKind {
    Beauty,
    MetricDepth,
    NormalizedDepth,
    Normal,
    SubjectId,
    OpticalFlow,
    PoseImage,
    PoseKeypoints,
    Occlusion,
    Camera,
}

impl PassKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Beauty => "beauty",
            Self::MetricDepth => "depth_metric",
            Self::NormalizedDepth => "depth_normalized",
            Self::Normal => "normal",
            Self::SubjectId => "id",
            Self::OpticalFlow => "flow",
            Self::PoseImage => "pose",
            Self::PoseKeypoints => "pose_keypoints",
            Self::Occlusion => "occlusion",
            Self::Camera => "camera",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PixelEncoding {
    Png8,
    Png16,
    Exr32Float,
    Json,
    Npz,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ColorSpace {
    Srgb,
    Linear,
    Data,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChannelLayout {
    Gray,
    Rgb,
    Rgba,
    Xy,
    JsonRecords,
    Matrix,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Normalization {
    pub minimum: f32,
    pub maximum: f32,
    pub near_is_white: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PassSpec {
    pub kind: PassKind,
    pub encoding: PixelEncoding,
    pub color_space: ColorSpace,
    pub channels: ChannelLayout,
    #[serde(default)]
    pub normalization: Option<Normalization>,
    #[serde(default)]
    pub background_value: Vec<f32>,
    pub file_pattern: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct AdapterCapabilities {
    pub supports_metric_depth: bool,
    pub supports_optical_flow: bool,
    pub supports_pose: bool,
    pub supports_lighting: bool,
    pub supports_focus: bool,
    pub supports_camera_matrices: bool,
    pub supports_transitions: bool,
    pub supported_passes: Vec<PassKind>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VideoModelProfile {
    pub name: String,
    pub supports_text_timeline: bool,
    pub supports_start_end_frames: bool,
    pub supports_dense_control_video: bool,
    pub supports_depth: bool,
    pub supports_pose: bool,
    pub supports_segmentation: bool,
    pub supports_camera_trajectory: bool,
    pub supports_negative_prompt: bool,
    pub maximum_frames: Option<u32>,
}

impl Default for VideoModelProfile {
    fn default() -> Self {
        Self {
            name: "generic_dense_control".to_owned(),
            supports_text_timeline: true,
            supports_start_end_frames: true,
            supports_dense_control_video: true,
            supports_depth: true,
            supports_pose: true,
            supports_segmentation: true,
            supports_camera_trajectory: true,
            supports_negative_prompt: true,
            maximum_frames: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExecutionProfile {
    pub name: String,
    pub model: VideoModelProfile,
    pub passes: Vec<PassSpec>,
    pub resolution: [u32; 2],
    pub deterministic_seed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExecutionRequest {
    pub scene_id: String,
    #[serde(default)]
    pub shot_ids: Vec<String>,
    #[serde(default)]
    pub frame_range: Option<FrameRange>,
    pub profile: ExecutionProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExecutionBundle {
    pub scene_id: String,
    pub root: PathBuf,
    pub manifest_path: PathBuf,
    pub complete_path: PathBuf,
    pub executed: bool,
}

impl PassSpec {
    pub fn for_conditioning(pass: ConditioningPass, near_m: f32, far_m: f32) -> Self {
        match pass {
            ConditioningPass::Beauty => Self {
                kind: PassKind::Beauty,
                encoding: PixelEncoding::Png8,
                color_space: ColorSpace::Srgb,
                channels: ChannelLayout::Rgb,
                normalization: None,
                background_value: vec![0.0, 0.0, 0.0],
                file_pattern: "beauty/####.png".to_owned(),
            },
            ConditioningPass::Depth => Self::normalized_depth(near_m, far_m),
            ConditioningPass::DepthMetric => Self::metric_depth(),
            ConditioningPass::Normal => Self {
                kind: PassKind::Normal,
                encoding: PixelEncoding::Png16,
                color_space: ColorSpace::Data,
                channels: ChannelLayout::Rgb,
                normalization: None,
                background_value: vec![0.5, 0.5, 0.5],
                file_pattern: "normal/####.png".to_owned(),
            },
            ConditioningPass::ObjectIndex => Self {
                kind: PassKind::SubjectId,
                encoding: PixelEncoding::Png16,
                color_space: ColorSpace::Data,
                channels: ChannelLayout::Rgb,
                normalization: None,
                background_value: vec![0.0, 0.0, 0.0],
                file_pattern: "id/####.png".to_owned(),
            },
            ConditioningPass::Vector => Self::optical_flow(),
            ConditioningPass::OpenPose => Self {
                kind: PassKind::PoseImage,
                encoding: PixelEncoding::Png8,
                color_space: ColorSpace::Srgb,
                channels: ChannelLayout::Rgb,
                normalization: None,
                background_value: vec![0.0, 0.0, 0.0],
                file_pattern: "openpose/####.png".to_owned(),
            },
            ConditioningPass::Occlusion => Self {
                kind: PassKind::Occlusion,
                encoding: PixelEncoding::Png16,
                color_space: ColorSpace::Data,
                channels: ChannelLayout::Gray,
                normalization: None,
                background_value: vec![0.0],
                file_pattern: "occlusion/####.png".to_owned(),
            },
            ConditioningPass::Camera => Self::camera_json(),
            ConditioningPass::PoseKeypoints => Self {
                kind: PassKind::PoseKeypoints,
                encoding: PixelEncoding::Json,
                color_space: ColorSpace::Data,
                channels: ChannelLayout::JsonRecords,
                normalization: None,
                background_value: Vec::new(),
                file_pattern: "metadata/pose.json".to_owned(),
            },
        }
    }

    pub fn metric_depth() -> Self {
        Self {
            kind: PassKind::MetricDepth,
            encoding: PixelEncoding::Exr32Float,
            color_space: ColorSpace::Data,
            channels: ChannelLayout::Gray,
            normalization: None,
            background_value: vec![0.0],
            file_pattern: "depth_metric/####.exr".to_owned(),
        }
    }

    pub fn normalized_depth(near_m: f32, far_m: f32) -> Self {
        Self {
            kind: PassKind::NormalizedDepth,
            encoding: PixelEncoding::Png16,
            color_space: ColorSpace::Data,
            channels: ChannelLayout::Gray,
            normalization: Some(Normalization {
                minimum: near_m,
                maximum: far_m,
                near_is_white: true,
            }),
            background_value: vec![0.0],
            file_pattern: "depth/####.png".to_owned(),
        }
    }

    pub fn optical_flow() -> Self {
        Self {
            kind: PassKind::OpticalFlow,
            encoding: PixelEncoding::Exr32Float,
            color_space: ColorSpace::Data,
            channels: ChannelLayout::Xy,
            normalization: None,
            background_value: vec![0.0, 0.0],
            file_pattern: "vector/####.exr".to_owned(),
        }
    }

    pub fn camera_json() -> Self {
        Self {
            kind: PassKind::Camera,
            encoding: PixelEncoding::Json,
            color_space: ColorSpace::Data,
            channels: ChannelLayout::Matrix,
            normalization: None,
            background_value: Vec::new(),
            file_pattern: "metadata/camera.json".to_owned(),
        }
    }
}

impl ExecutionProfile {
    pub fn generic_control() -> Self {
        Self {
            name: "generic_control".to_owned(),
            model: VideoModelProfile::default(),
            passes: vec![
                PassSpec::metric_depth(),
                PassSpec::normalized_depth(0.1, 100.0),
                PassSpec::optical_flow(),
                PassSpec::camera_json(),
            ],
            resolution: [1024, 576],
            deterministic_seed: 42,
        }
    }
}

/// Conditioning channels an adapter can export (ADR-1115 D6).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ConditioningPass {
    /// Shaded clay render; source for edge/canny post-processing.
    Beauty,
    /// Normalised inverse depth, near = white.
    Depth,
    /// Metric camera-space depth in metres (float EXR).
    DepthMetric,
    /// Camera-space normals encoded as RGB.
    Normal,
    /// Flat per-subject colours (palette) — segmentation / identity.
    #[serde(rename = "id")]
    ObjectIndex,
    /// Motion vectors (OpenEXR).
    Vector,
    /// OpenPose-style coloured skeleton on black.
    #[serde(rename = "openpose")]
    OpenPose,
    /// Visible proxy mask used with depth to reason about occlusion.
    Occlusion,
    /// Numeric camera intrinsics/extrinsics JSON.
    Camera,
    /// Authored pose/gaze/contact records JSON.
    PoseKeypoints,
}

impl ConditioningPass {
    pub const ALL: [ConditioningPass; 10] = [
        ConditioningPass::Beauty,
        ConditioningPass::Depth,
        ConditioningPass::DepthMetric,
        ConditioningPass::Normal,
        ConditioningPass::ObjectIndex,
        ConditioningPass::Vector,
        ConditioningPass::OpenPose,
        ConditioningPass::Occlusion,
        ConditioningPass::Camera,
        ConditioningPass::PoseKeypoints,
    ];

    /// Directory / CLI name.
    pub fn name(self) -> &'static str {
        match self {
            ConditioningPass::Beauty => "beauty",
            ConditioningPass::Depth => "depth",
            ConditioningPass::DepthMetric => "depth_metric",
            ConditioningPass::Normal => "normal",
            ConditioningPass::ObjectIndex => "id",
            ConditioningPass::Vector => "vector",
            ConditioningPass::OpenPose => "openpose",
            ConditioningPass::Occlusion => "occlusion",
            ConditioningPass::Camera => "camera",
            ConditioningPass::PoseKeypoints => "pose_keypoints",
        }
    }

    pub fn parse(name: &str) -> Option<ConditioningPass> {
        match name.trim().to_ascii_lowercase().as_str() {
            "beauty" | "combined" => Some(ConditioningPass::Beauty),
            "depth" | "z" => Some(ConditioningPass::Depth),
            "depth_metric" | "metric_depth" => Some(ConditioningPass::DepthMetric),
            "normal" => Some(ConditioningPass::Normal),
            "id" | "seg" | "object_index" | "index" => Some(ConditioningPass::ObjectIndex),
            "vector" | "flow" => Some(ConditioningPass::Vector),
            "openpose" | "pose" => Some(ConditioningPass::OpenPose),
            "occlusion" | "occ" => Some(ConditioningPass::Occlusion),
            "camera" | "camera_json" => Some(ConditioningPass::Camera),
            "pose_keypoints" | "pose_json" => Some(ConditioningPass::PoseKeypoints),
            _ => None,
        }
    }

    /// Parses a comma-separated list, rejecting unknown names.
    pub fn parse_list(text: &str) -> Result<Vec<ConditioningPass>, String> {
        let mut passes = Vec::new();
        for item in text.split(',').filter(|s| !s.trim().is_empty()) {
            let pass = ConditioningPass::parse(item)
                .ok_or_else(|| format!("unknown pass '{}'", item.trim()))?;
            if !passes.contains(&pass) {
                passes.push(pass);
            }
        }
        if passes.is_empty() {
            return Err("at least one pass is required".to_owned());
        }
        Ok(passes)
    }
}

/// What an adapter produced for one scene.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScenePassOutput {
    pub scene_id: String,
    pub out_dir: PathBuf,
    pub script_path: PathBuf,
    pub manifest_path: PathBuf,
    /// `true` when the external runtime actually ran and exited successfully.
    pub executed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PassOutput {
    pub passes: Vec<ConditioningPass>,
    pub scenes: Vec<ScenePassOutput>,
    /// Runtime executable used, when one ran.
    pub runtime: Option<PathBuf>,
}

/// Contract for every execute-layer adapter (ADR-1115 D13 naming).
pub trait CinematographyAdapter {
    fn capabilities(&self) -> AdapterCapabilities;

    /// Canonical staging path. The adapter must not reinterpret source IR.
    fn stage_scene(
        &mut self,
        _project: &CompiledProject,
        _scene: &CompiledScene,
        _profile: &ExecutionProfile,
    ) -> anyhow::Result<()> {
        anyhow::bail!("this adapter has not implemented compiled scene staging")
    }

    fn execute(
        &mut self,
        _request: &ExecutionRequest,
        _out_dir: &Path,
    ) -> anyhow::Result<ExecutionBundle> {
        anyhow::bail!("this adapter has not implemented execution bundles")
    }

    /// Stage the solved cameras, subjects, and lighting of one scene.
    fn apply_scene(&mut self, project: &SolvedProject, scene: &SolvedScene) -> anyhow::Result<()>;

    /// Export conditioning channels for the staged scenes. This is not a
    /// "final render": the output is clay-proxy conditioning imagery.
    fn render_passes(
        &mut self,
        range: Option<FrameRange>,
        passes: &[ConditioningPass],
        out_dir: &Path,
    ) -> anyhow::Result<PassOutput>;
}
