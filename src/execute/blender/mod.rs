//! Blender adapter (ADR-1115 D6): renders clay proxies and exports
//! conditioning passes. Generates a self-contained `bpy` script per scene and
//! runs `blender --background` when a Blender executable is available.

pub mod convert;
pub mod script;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::execute::{CinematographyAdapter, ConditioningPass, PassOutput, ScenePassOutput};
use crate::model::FrameRange;
use crate::solve::{SolvedProject, SolvedScene};

pub use convert::{build_payload, depth_bounds, to_blender, BlenderPayload, RenderSettings};
pub use script::render_script;

#[derive(Debug, Clone)]
pub struct BlenderOptions {
    pub resolution: (u32, u32),
    pub samples: u32,
    pub use_dof: bool,
    /// Explicit Blender executable; otherwise `$BLENDER`, then `blender` on PATH.
    pub blender_path: Option<PathBuf>,
    /// Write scripts and manifests without launching Blender.
    pub script_only: bool,
}

impl Default for BlenderOptions {
    fn default() -> Self {
        Self {
            resolution: (1024, 576),
            samples: 32,
            use_dof: false,
            blender_path: None,
            script_only: false,
        }
    }
}

/// Sidecar written next to each script so downstream tools can split the
/// frame sequence per shot and map colours back to subjects.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Manifest {
    pub schema_version: String,
    pub source_id: String,
    pub scene_id: String,
    pub fps: f64,
    pub frame_range: [u64; 2],
    pub resolution: [u32; 2],
    pub passes: Vec<ConditioningPass>,
    pub depth_near_m: f32,
    pub depth_far_m: f32,
    pub shots: Vec<ManifestShot>,
    pub subjects: Vec<ManifestSubject>,
    pub conventions: ManifestConventions,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ManifestShot {
    pub id: String,
    pub frame_start: u64,
    pub frame_end: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ManifestSubject {
    pub id: String,
    pub name: String,
    pub pass_index: u32,
    pub color_hex: String,
    pub color_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ManifestConventions {
    pub world: String,
    pub depth: String,
    pub normal: String,
    pub id: String,
    pub openpose: String,
    pub frames: String,
}

#[derive(Debug, Default)]
pub struct BlenderAdapter {
    options: BlenderOptions,
    payloads: Vec<BlenderPayload>,
}

impl BlenderAdapter {
    pub fn new(options: BlenderOptions) -> Self {
        Self {
            options,
            payloads: Vec::new(),
        }
    }

    pub fn payloads(&self) -> &[BlenderPayload] {
        &self.payloads
    }

    /// Resolves the Blender executable without running it.
    pub fn locate_blender(&self) -> Option<PathBuf> {
        if let Some(path) = &self.options.blender_path {
            return Some(path.clone());
        }
        if let Ok(env) = std::env::var("BLENDER") {
            if !env.is_empty() {
                return Some(PathBuf::from(env));
            }
        }
        let path_var = std::env::var_os("PATH")?;
        std::env::split_paths(&path_var)
            .map(|dir| dir.join("blender"))
            .find(|candidate| candidate.is_file())
    }

    fn finalize(
        &self,
        payload: &BlenderPayload,
        range: Option<FrameRange>,
        passes: &[ConditioningPass],
        scene_dir: &Path,
    ) -> BlenderPayload {
        let frame_range = payload.clamp_range(range);
        payload.clone().with_render(RenderSettings {
            resolution: [self.options.resolution.0, self.options.resolution.1],
            samples: self.options.samples.max(1),
            passes: passes.to_vec(),
            frame_range,
            depth_near_m: payload.render.depth_near_m,
            depth_far_m: payload.render.depth_far_m,
            use_dof: self.options.use_dof,
            output_dir: scene_dir.to_string_lossy().into_owned(),
        })
    }

    fn manifest(payload: &BlenderPayload) -> Manifest {
        Manifest {
            schema_version: "blender-manifest-0.1".to_owned(),
            source_id: payload.source_id.clone(),
            scene_id: payload.scene_id.clone(),
            fps: payload.fps_numerator as f64 / payload.fps_denominator.max(1) as f64,
            frame_range: payload.render.frame_range,
            resolution: payload.render.resolution,
            passes: payload.render.passes.clone(),
            depth_near_m: payload.render.depth_near_m,
            depth_far_m: payload.render.depth_far_m,
            shots: payload
                .shots
                .iter()
                .map(|shot| ManifestShot {
                    id: shot.id.clone(),
                    frame_start: shot.frame_start,
                    frame_end: shot.frame_end,
                })
                .collect(),
            subjects: payload
                .subjects
                .iter()
                .map(|subject| ManifestSubject {
                    id: subject.id.clone(),
                    name: subject.name.clone(),
                    pass_index: subject.pass_index,
                    color_hex: format!(
                        "#{:02x}{:02x}{:02x}",
                        (subject.color[0] * 255.0).round() as u8,
                        (subject.color[1] * 255.0).round() as u8,
                        (subject.color[2] * 255.0).round() as u8
                    ),
                    color_name: subject.color_name.clone(),
                })
                .collect(),
            conventions: ManifestConventions {
                world: "Blender world: X right, Y forward, Z up, metres".to_owned(),
                depth: "16-bit PNG, linear; near = white (1.0) at depth_near_m, far = black (0.0) at depth_far_m, clamped".to_owned(),
                normal: "8-bit PNG, linear (raw); camera-space normal stored as (x, y, -z) * 0.5 + 0.5 with x right, y up, z toward the camera (Cycles' camera space looks down +z, hence the flip)".to_owned(),
                id: "8-bit sRGB PNG; flat palette colour per subject (see subjects[].color_hex), black background".to_owned(),
                openpose: "8-bit sRGB PNG; COCO-18 skeleton with ControlNet limb colours on black; characters only".to_owned(),
                frames: "files are ####.png numbered by scene frame (0-based); shots[] gives inclusive per-shot ranges".to_owned(),
            },
        }
    }
}

impl CinematographyAdapter for BlenderAdapter {
    fn apply_scene(&mut self, project: &SolvedProject, scene: &SolvedScene) -> Result<()> {
        if scene.duration_frames == 0 {
            bail!("scene '{}' has no frames", scene.id);
        }
        self.payloads.push(build_payload(project, scene));
        Ok(())
    }

    fn render_passes(
        &mut self,
        range: Option<FrameRange>,
        passes: &[ConditioningPass],
        out_dir: &Path,
    ) -> Result<PassOutput> {
        if passes.is_empty() {
            bail!("no passes requested");
        }
        if self.payloads.is_empty() {
            bail!("no scenes staged; call apply_scene first");
        }
        let blender = if self.options.script_only {
            None
        } else {
            self.locate_blender()
        };

        let mut scenes = Vec::with_capacity(self.payloads.len());
        for payload in &self.payloads {
            if let Some(r) = range {
                if r.start > payload.frame_end {
                    bail!(
                        "frame range {}..{} does not intersect scene '{}' (0..{})",
                        r.start,
                        r.end,
                        payload.scene_id,
                        payload.frame_end + 1
                    );
                }
            }
            let scene_dir = out_dir.join(&payload.scene_id);
            fs::create_dir_all(&scene_dir)
                .with_context(|| format!("failed to create {}", scene_dir.display()))?;
            let scene_dir = scene_dir.canonicalize().unwrap_or(scene_dir);
            let finalized = self.finalize(payload, range, passes, &scene_dir);

            let script_path = scene_dir.join("build.py");
            fs::write(&script_path, render_script(&finalized))
                .with_context(|| format!("failed to write {}", script_path.display()))?;
            let manifest_path = scene_dir.join("manifest.json");
            fs::write(
                &manifest_path,
                format!(
                    "{}\n",
                    serde_json::to_string_pretty(&Self::manifest(&finalized))?
                ),
            )
            .with_context(|| format!("failed to write {}", manifest_path.display()))?;

            let mut executed = false;
            if let Some(blender) = &blender {
                let status = Command::new(blender)
                    .arg("--background")
                    .arg("--factory-startup")
                    .arg("--python")
                    .arg(&script_path)
                    .status()
                    .with_context(|| format!("failed to launch {}", blender.display()))?;
                if !status.success() {
                    bail!(
                        "blender exited with {status} while rendering scene '{}'; script: {}",
                        payload.scene_id,
                        script_path.display()
                    );
                }
                executed = true;
            }

            scenes.push(ScenePassOutput {
                scene_id: payload.scene_id.clone(),
                out_dir: scene_dir,
                script_path,
                manifest_path,
                executed,
            });
        }

        if blender.is_none() && !self.options.script_only {
            let scripts: Vec<String> = scenes
                .iter()
                .map(|s| s.script_path.display().to_string())
                .collect();
            return Err(anyhow!(
                "blender executable not found (set --blender PATH or $BLENDER); scripts were written and can be run elsewhere:\n  {}",
                scripts.join("\n  ")
            ));
        }

        Ok(PassOutput {
            passes: passes.to_vec(),
            scenes,
            runtime: blender,
        })
    }
}
