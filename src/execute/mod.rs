//! Execute layer (ADR-1115 D6/D12): adapters that drive external runtimes
//! from Solved Camera IR. Impure by nature — they write files and spawn
//! processes — and therefore isolated from `view` and `prompt`.

pub mod blender;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::model::FrameRange;
use crate::solve::{SolvedProject, SolvedScene};

/// Conditioning channels an adapter can export (ADR-1115 D6).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ConditioningPass {
    /// Shaded clay render; source for edge/canny post-processing.
    Beauty,
    /// Normalised inverse depth, near = white.
    Depth,
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
}

impl ConditioningPass {
    pub const ALL: [ConditioningPass; 6] = [
        ConditioningPass::Beauty,
        ConditioningPass::Depth,
        ConditioningPass::Normal,
        ConditioningPass::ObjectIndex,
        ConditioningPass::Vector,
        ConditioningPass::OpenPose,
    ];

    /// Directory / CLI name.
    pub fn name(self) -> &'static str {
        match self {
            ConditioningPass::Beauty => "beauty",
            ConditioningPass::Depth => "depth",
            ConditioningPass::Normal => "normal",
            ConditioningPass::ObjectIndex => "id",
            ConditioningPass::Vector => "vector",
            ConditioningPass::OpenPose => "openpose",
        }
    }

    pub fn parse(name: &str) -> Option<ConditioningPass> {
        match name.trim().to_ascii_lowercase().as_str() {
            "beauty" | "combined" => Some(ConditioningPass::Beauty),
            "depth" | "z" => Some(ConditioningPass::Depth),
            "normal" => Some(ConditioningPass::Normal),
            "id" | "seg" | "object_index" | "index" => Some(ConditioningPass::ObjectIndex),
            "vector" | "flow" => Some(ConditioningPass::Vector),
            "openpose" | "pose" => Some(ConditioningPass::OpenPose),
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
