//! Blender adapter (ADR-1115 D6): renders clay proxies and exports
//! conditioning passes. Generates a self-contained `bpy` script per scene and
//! runs `blender --background` when a Blender executable is available.

pub mod convert;
pub mod script;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::compiled::{CompiledProject, CompiledScene};
use crate::execute::{
    AdapterCapabilities, CinematographyAdapter, ConditioningPass, ExecutionProfile, PassKind,
    PassOutput, PassSpec, ScenePassOutput,
};
use crate::model::FrameRange;
use crate::solve::{SolvedProject, SolvedScene};

pub use convert::{
    build_compiled_payload, build_payload, depth_bounds, to_blender, BlenderPayload, RenderSettings,
};
pub use script::render_script;

fn blender_command(blender: &Path, script_path: &Path) -> Command {
    let mut command = Command::new(blender);
    command
        .arg("--background")
        .arg("--factory-startup")
        .arg("--disable-autoexec")
        .arg("--python-exit-code")
        .arg("70")
        .arg("--python")
        .arg(script_path);
    command
}

#[derive(Debug, Clone)]
pub struct BlenderOptions {
    pub resolution: (u32, u32),
    pub samples: u32,
    pub use_dof: bool,
    /// Explicit Blender executable; otherwise `$BLENDER`, then `blender` on PATH.
    pub blender_path: Option<PathBuf>,
    /// Write scripts and manifests without launching Blender.
    pub script_only: bool,
    /// Hard runtime limit per scene. Zero disables the timeout.
    pub timeout_s: u64,
}

impl Default for BlenderOptions {
    fn default() -> Self {
        Self {
            resolution: (1024, 576),
            samples: 32,
            use_dof: false,
            blender_path: None,
            script_only: false,
            timeout_s: 1800,
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
    pub pass_specs: Vec<PassSpec>,
    pub depth_near_m: f32,
    pub depth_far_m: f32,
    pub shots: Vec<ManifestShot>,
    pub subjects: Vec<ManifestSubject>,
    pub conventions: ManifestConventions,
    pub source_hash: String,
    pub compiled_ir_hash: String,
    pub profile_hash: String,
    pub deterministic_seed: u64,
    pub expected_frame_count: u64,
    pub actual_frame_counts: std::collections::BTreeMap<String, u64>,
    #[serde(default)]
    pub runtime_version: Option<String>,
    pub status: String,
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
    compiled_scenes: Vec<(CompiledProject, CompiledScene)>,
}

impl BlenderAdapter {
    pub fn new(options: BlenderOptions) -> Self {
        Self {
            options,
            payloads: Vec::new(),
            compiled_scenes: Vec::new(),
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

    fn manifest(
        payload: &BlenderPayload,
        actual_frame_counts: std::collections::BTreeMap<String, u64>,
        runtime_version: Option<String>,
    ) -> Manifest {
        let payload_json = serde_json::to_string(payload).unwrap_or_default();
        let render_json = serde_json::to_string(&payload.render).unwrap_or_default();
        Manifest {
            schema_version: "blender-manifest-0.2".to_owned(),
            source_id: payload.source_id.clone(),
            scene_id: payload.scene_id.clone(),
            fps: payload.fps_numerator as f64 / payload.fps_denominator.max(1) as f64,
            frame_range: payload.render.frame_range,
            resolution: payload.render.resolution,
            passes: payload.render.passes.clone(),
            pass_specs: payload
                .render
                .passes
                .iter()
                .copied()
                .map(|pass| pass_spec(pass, payload.render.depth_near_m, payload.render.depth_far_m))
                .collect(),
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
                normal: "16-bit PNG, linear (raw); camera-space normal stored as (x, y, -z) * 0.5 + 0.5 with x right, y up, z toward the camera (Cycles' camera space looks down +z, hence the flip)".to_owned(),
                id: "16-bit data PNG; flat palette colour per subject (see subjects[].color_hex), black background".to_owned(),
                openpose: "8-bit sRGB PNG; COCO-18 skeleton with ControlNet limb colours on black; characters only".to_owned(),
                frames: "files use the pass_specs[].file_pattern and scene-frame numbering; shots[] gives inclusive per-shot ranges".to_owned(),
            },
            source_hash: format!("fnv1a64:{:016x}", crate::math::stable_hash(&payload.source_id)),
            compiled_ir_hash: format!("fnv1a64:{:016x}", crate::math::stable_hash(&payload_json)),
            profile_hash: format!("fnv1a64:{:016x}", crate::math::stable_hash(&render_json)),
            deterministic_seed: 42,
            expected_frame_count: payload.render.frame_range[1]
                .saturating_sub(payload.render.frame_range[0])
                + 1,
            actual_frame_counts,
            runtime_version,
            status: "complete".to_owned(),
        }
    }
}

fn pass_spec(pass: ConditioningPass, near: f32, far: f32) -> PassSpec {
    PassSpec::for_conditioning(pass, near, far)
}

impl CinematographyAdapter for BlenderAdapter {
    fn capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            supports_metric_depth: true,
            supports_optical_flow: true,
            supports_pose: true,
            supports_lighting: true,
            supports_focus: true,
            supports_camera_matrices: true,
            supports_transitions: false,
            supported_passes: vec![
                PassKind::Beauty,
                PassKind::MetricDepth,
                PassKind::NormalizedDepth,
                PassKind::Normal,
                PassKind::SubjectId,
                PassKind::OpticalFlow,
                PassKind::PoseImage,
                PassKind::PoseKeypoints,
                PassKind::Occlusion,
                PassKind::Camera,
            ],
        }
    }

    fn stage_scene(
        &mut self,
        project: &CompiledProject,
        scene: &CompiledScene,
        profile: &ExecutionProfile,
    ) -> Result<()> {
        if scene.duration_frames == 0 {
            bail!("scene '{}' has no frames", scene.id);
        }
        let capabilities = self.capabilities();
        for pass in &profile.passes {
            if !capabilities.supported_passes.contains(&pass.kind) {
                bail!(
                    "Blender adapter does not support requested pass {:?}",
                    pass.kind
                );
            }
            let canonical =
                PassSpec::for_conditioning(conditioning_for_kind(pass.kind), 0.1, 100.0);
            if pass.encoding != canonical.encoding
                || pass.color_space != canonical.color_space
                || pass.channels != canonical.channels
                || pass.file_pattern != canonical.file_pattern
            {
                bail!(
                    "Blender adapter cannot satisfy {:?} as {:?}/{:?}/{:?} at '{}'; expected {:?}/{:?}/{:?} at '{}'",
                    pass.kind,
                    pass.encoding,
                    pass.color_space,
                    pass.channels,
                    pass.file_pattern,
                    canonical.encoding,
                    canonical.color_space,
                    canonical.channels,
                    canonical.file_pattern
                );
            }
        }
        if self.options.use_dof {
            for shot in &scene.shots {
                if let Some(frame) = shot
                    .focus_track
                    .frames
                    .iter()
                    .find(|frame| frame.distance_m.is_none())
                {
                    bail!(
                        "cannot enable DOF: shot '{}' frame {} has no focus target or distance",
                        shot.id,
                        frame.frame
                    );
                }
            }
        }
        self.options.resolution = (profile.resolution[0], profile.resolution[1]);
        self.payloads.push(build_compiled_payload(project, scene));
        self.compiled_scenes.push((project.clone(), scene.clone()));
        Ok(())
    }

    fn execute(
        &mut self,
        request: &crate::execute::ExecutionRequest,
        out_dir: &Path,
    ) -> Result<crate::execute::ExecutionBundle> {
        let (project, scene) = self
            .compiled_scenes
            .iter()
            .find(|(_, scene)| scene.id == request.scene_id)
            .cloned()
            .ok_or_else(|| anyhow!("compiled scene '{}' is not staged", request.scene_id))?;
        let passes = conditioning_passes(&request.profile.passes)?;
        let selected_payloads: Vec<_> = self
            .payloads
            .iter()
            .filter(|payload| payload.scene_id == request.scene_id)
            .cloned()
            .collect();
        if selected_payloads.is_empty() {
            bail!(
                "compiled scene '{}' has no adapter payload",
                request.scene_id
            );
        }
        let transaction_root = out_dir.join(format!(
            ".{}.bundle-{}",
            request.scene_id,
            std::process::id()
        ));
        if transaction_root.exists() {
            fs::remove_dir_all(&transaction_root)?;
        }
        fs::create_dir_all(&transaction_root)?;
        let original_payloads = std::mem::replace(&mut self.payloads, selected_payloads);
        let render_result = self.render_passes(request.frame_range, &passes, &transaction_root);
        self.payloads = original_payloads;
        let rendered = render_result?;
        let scene_root = transaction_root.join(&request.scene_id);
        let complete = scene_root.join("complete.json");
        if complete.exists() {
            fs::remove_file(&complete)?;
        }
        let manifest_path = scene_root.join("manifest.json");
        let mut manifest: Manifest = serde_json::from_str(&fs::read_to_string(&manifest_path)?)?;
        manifest.profile_hash = format!(
            "fnv1a64:{:016x}",
            crate::math::stable_hash(&serde_json::to_string(&request.profile)?)
        );
        manifest.deterministic_seed = request.profile.deterministic_seed;
        fs::write(
            &manifest_path,
            format!("{}\n", serde_json::to_string_pretty(&manifest)?),
        )?;
        write_shot_bundle(&project, &scene, request, &scene_root)?;
        fs::write(
            &complete,
            format!(
                "{}\n",
                serde_json::to_string_pretty(&serde_json::json!({
                    "scene_id": scene.id,
                    "profile": request.profile.name,
                    "status": "complete",
                    "executed": rendered.scenes.first().is_some_and(|value| value.executed),
                }))?
            ),
        )?;
        fs::create_dir_all(out_dir)?;
        let destination = out_dir.join(&request.scene_id);
        promote_directory(&scene_root, &destination)?;
        let _ = fs::remove_dir_all(&transaction_root);
        Ok(crate::execute::ExecutionBundle {
            scene_id: request.scene_id.clone(),
            root: destination.clone(),
            manifest_path: destination.join("manifest.json"),
            complete_path: destination.join("complete.json"),
            executed: rendered.scenes.first().is_some_and(|value| value.executed),
        })
    }

    fn apply_scene(&mut self, project: &SolvedProject, scene: &SolvedScene) -> Result<()> {
        if scene.duration_frames == 0 {
            bail!("scene '{}' has no frames", scene.id);
        }
        if self.options.use_dof {
            for shot in &scene.shots {
                if let Some(frame) = shot
                    .frames
                    .iter()
                    .find(|frame| frame.focus_distance_m.is_none())
                {
                    bail!(
                        "cannot enable DOF: shot '{}' frame {} has no focus target or distance",
                        shot.id,
                        frame.frame
                    );
                }
            }
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
        if blender.is_none() && !self.options.script_only {
            bail!(
                "blender executable not found (set --blender PATH or $BLENDER); no scene bundle was published"
            );
        }

        fs::create_dir_all(out_dir)
            .with_context(|| format!("failed to create {}", out_dir.display()))?;
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
            let transaction_dir = out_dir.join(format!(
                ".{}.rendering-{}",
                payload.scene_id,
                std::process::id()
            ));
            if transaction_dir.exists() {
                fs::remove_dir_all(&transaction_dir).with_context(|| {
                    format!(
                        "failed to clean stale transaction {}",
                        transaction_dir.display()
                    )
                })?;
            }
            fs::create_dir_all(&transaction_dir)
                .with_context(|| format!("failed to create {}", transaction_dir.display()))?;
            let transaction_dir = transaction_dir.canonicalize().unwrap_or(transaction_dir);
            let finalized = self.finalize(payload, range, passes, &transaction_dir);
            let final_payload = self.finalize(payload, range, passes, &scene_dir);

            let staged_script = transaction_dir.join("build.py");
            fs::write(&staged_script, render_script(&finalized))
                .with_context(|| format!("failed to write {}", staged_script.display()))?;
            write_numeric_metadata(&finalized, &transaction_dir)?;

            let mut executed = false;
            let mut runtime_version = None;
            let mut actual_frame_counts = std::collections::BTreeMap::new();
            if let Some(blender) = &blender {
                let mut command = blender_command(blender, &staged_script);
                let stdout_path = transaction_dir.join("stdout.log");
                let stderr_path = transaction_dir.join("stderr.log");
                command
                    .stdout(Stdio::from(fs::File::create(&stdout_path)?))
                    .stderr(Stdio::from(fs::File::create(&stderr_path)?));
                let child = command
                    .spawn()
                    .with_context(|| format!("failed to launch {}", blender.display()));
                let mut child = match child {
                    Ok(child) => child,
                    Err(error) => {
                        preserve_failed_transaction(&transaction_dir, out_dir, &payload.scene_id)?;
                        return Err(error);
                    }
                };
                let start = Instant::now();
                let status = loop {
                    if let Some(status) = child.try_wait()? {
                        break status;
                    }
                    if self.options.timeout_s > 0
                        && start.elapsed() >= Duration::from_secs(self.options.timeout_s)
                    {
                        let _ = child.kill();
                        let _ = child.wait();
                        preserve_failed_transaction(&transaction_dir, out_dir, &payload.scene_id)?;
                        bail!(
                            "blender timed out after {}s while rendering scene '{}'",
                            self.options.timeout_s,
                            payload.scene_id
                        );
                    }
                    thread::sleep(Duration::from_millis(50));
                };
                if !status.success() {
                    let failed =
                        preserve_failed_transaction(&transaction_dir, out_dir, &payload.scene_id)?;
                    bail!(
                        "blender exited with {status} while rendering scene '{}'; diagnostics: {}",
                        payload.scene_id,
                        failed.display()
                    );
                }
                let done_path = transaction_dir.join("done.json");
                let done: serde_json::Value =
                    serde_json::from_str(&fs::read_to_string(&done_path).with_context(|| {
                        format!(
                            "runtime did not write completion marker {}",
                            done_path.display()
                        )
                    })?)?;
                runtime_version = done["blender_version"].as_str().map(str::to_owned);
                actual_frame_counts = validate_pass_inventory(&finalized, &transaction_dir)?;
                executed = true;
            }

            // The retained script targets the published directory, not the
            // ephemeral transaction path used for the render itself.
            fs::write(&staged_script, render_script(&final_payload))?;
            let manifest_path = transaction_dir.join("manifest.json");
            fs::write(
                &manifest_path,
                format!(
                    "{}\n",
                    serde_json::to_string_pretty(&Self::manifest(
                        &final_payload,
                        actual_frame_counts,
                        runtime_version
                    ))?
                ),
            )?;
            fs::write(
                transaction_dir.join("complete.json"),
                format!(
                    "{}\n",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "scene_id": payload.scene_id,
                        "executed": executed,
                        "status": "complete"
                    }))?
                ),
            )?;
            promote_directory(&transaction_dir, &scene_dir)?;

            let script_path = scene_dir.join("build.py");
            let manifest_path = scene_dir.join("manifest.json");

            scenes.push(ScenePassOutput {
                scene_id: payload.scene_id.clone(),
                out_dir: scene_dir.clone(),
                script_path,
                manifest_path,
                executed,
            });
        }

        Ok(PassOutput {
            passes: passes.to_vec(),
            scenes,
            runtime: blender,
        })
    }
}

fn conditioning_passes(specs: &[PassSpec]) -> Result<Vec<ConditioningPass>> {
    let mut passes = Vec::new();
    for spec in specs {
        let pass = conditioning_for_kind(spec.kind);
        if !passes.contains(&pass) {
            passes.push(pass);
        }
    }
    if passes.is_empty() {
        bail!("execution profile requests no supported passes");
    }
    Ok(passes)
}

fn conditioning_for_kind(kind: PassKind) -> ConditioningPass {
    match kind {
        PassKind::Beauty => ConditioningPass::Beauty,
        PassKind::MetricDepth => ConditioningPass::DepthMetric,
        PassKind::NormalizedDepth => ConditioningPass::Depth,
        PassKind::Normal => ConditioningPass::Normal,
        PassKind::SubjectId => ConditioningPass::ObjectIndex,
        PassKind::OpticalFlow => ConditioningPass::Vector,
        PassKind::PoseImage => ConditioningPass::OpenPose,
        PassKind::PoseKeypoints => ConditioningPass::PoseKeypoints,
        PassKind::Occlusion => ConditioningPass::Occlusion,
        PassKind::Camera => ConditioningPass::Camera,
    }
}

fn write_shot_bundle(
    project: &CompiledProject,
    scene: &CompiledScene,
    request: &crate::execute::ExecutionRequest,
    root: &Path,
) -> Result<()> {
    fs::write(
        root.join("edit.json"),
        format!("{}\n", serde_json::to_string_pretty(&scene.edit_timeline)?),
    )?;
    let shot_root = root.join("shots");
    fs::create_dir_all(&shot_root)?;
    for shot in &scene.shots {
        if !request.shot_ids.is_empty() && !request.shot_ids.contains(&shot.id) {
            continue;
        }
        let root = shot_root.join(&shot.id);
        fs::create_dir_all(root.join("review"))?;
        fs::create_dir_all(root.join("controls"))?;
        fs::create_dir_all(root.join("metadata"))?;
        fs::write(
            root.join("guidance.json"),
            format!("{}\n", serde_json::to_string_pretty(shot)?),
        )?;
        fs::write(
            root.join("prompt.txt"),
            format!(
                "{}\n",
                crate::prompt::emit_guidance_command(
                    scene,
                    shot,
                    project.frame_rate.frames_per_second().unwrap_or(24.0)
                )
            ),
        )?;
        fs::write(
            root.join("negative_prompt.txt"),
            format!(
                "{}\n",
                shot.prohibitions
                    .iter()
                    .map(crate::prompt::prohibition_prompt)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        )?;
        let diagnostics = crate::view::diagram::lower_shot_diagnostics(shot, 640.0);
        fs::write(
            root.join("review/motion_grammar.svg"),
            crate::view::encode::canvas_to_svg(&diagnostics),
        )?;
        let elevation = crate::view::diagram::lower_elevation(shot, 640.0);
        fs::write(
            root.join("review/elevation.svg"),
            crate::view::encode::canvas_to_svg(&elevation),
        )?;
        for spec in &request.profile.passes {
            let pass_root = root.join("controls").join(spec.kind.name());
            fs::create_dir_all(&pass_root)?;
            fs::write(
                pass_root.join("source.json"),
                format!(
                    "{}\n",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "scene_pass": spec.file_pattern,
                        "frame_range": shot.edit_range,
                        "encoding": spec.encoding,
                        "channels": spec.channels,
                    }))?
                ),
            )?;
        }
        fs::write(
            root.join("metadata/camera.json"),
            format!("{}\n", serde_json::to_string_pretty(&shot.camera_track)?),
        )?;
        fs::write(
            root.join("metadata/screen_tracks.json"),
            format!("{}\n", serde_json::to_string_pretty(&shot.screen_tracks)?),
        )?;
        fs::write(
            root.join("metadata/constraints.json"),
            format!("{}\n", serde_json::to_string_pretty(&shot.constraints)?),
        )?;
        fs::write(
            root.join("metadata/validation.json"),
            format!("{}\n", serde_json::to_string_pretty(&shot.evaluations)?),
        )?;
        fs::write(
            root.join("manifest.json"),
            format!(
                "{}\n",
                serde_json::to_string_pretty(&serde_json::json!({
                    "schema_version": "shot-bundle-0.1",
                    "source_id": project.source_id,
                    "scene_id": scene.id,
                    "shot_id": shot.id,
                    "take_id": shot.take_id,
                    "source_range": shot.source_range,
                    "edit_range": shot.edit_range,
                    "profile": request.profile,
                    "seed": request.profile.deterministic_seed,
                }))?
            ),
        )?;
    }
    Ok(())
}

fn validate_pass_inventory(
    payload: &BlenderPayload,
    root: &Path,
) -> Result<std::collections::BTreeMap<String, u64>> {
    let expected = payload.render.frame_range[1].saturating_sub(payload.render.frame_range[0]) + 1;
    let mut inventory = std::collections::BTreeMap::new();
    for pass in &payload.render.passes {
        if *pass == ConditioningPass::Camera || *pass == ConditioningPass::PoseKeypoints {
            let path = match pass {
                ConditioningPass::Camera => root.join("metadata/camera.json"),
                ConditioningPass::PoseKeypoints => root.join("metadata/pose.json"),
                _ => unreachable!(),
            };
            if !path.is_file() {
                bail!("missing metadata pass {}", path.display());
            }
            inventory.insert(pass.name().to_owned(), 1);
            continue;
        }
        let directory = root.join(pass.name());
        let extension = if matches!(
            pass,
            ConditioningPass::Vector | ConditioningPass::DepthMetric
        ) {
            "exr"
        } else {
            "png"
        };
        let actual = fs::read_dir(&directory)
            .with_context(|| format!("missing pass directory {}", directory.display()))?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .is_some_and(|value| value.eq_ignore_ascii_case(extension))
            })
            .count() as u64;
        if actual != expected {
            bail!(
                "pass '{}' produced {actual} frame(s), expected {expected}",
                pass.name()
            );
        }
        inventory.insert(pass.name().to_owned(), actual);
    }
    Ok(inventory)
}

fn write_numeric_metadata(payload: &BlenderPayload, root: &Path) -> Result<()> {
    let needs_metadata = payload.render.passes.iter().any(|pass| {
        matches!(
            pass,
            ConditioningPass::Camera | ConditioningPass::PoseKeypoints
        )
    });
    if !needs_metadata {
        return Ok(());
    }
    let metadata = root.join("metadata");
    fs::create_dir_all(&metadata)?;
    if payload.render.passes.contains(&ConditioningPass::Camera) {
        fs::write(
            metadata.join("camera.json"),
            format!(
                "{}\n",
                serde_json::to_string_pretty(&serde_json::json!({
                    "coordinate_system": "blender_x_right_y_forward_z_up",
                    "frames": payload.shots,
                }))?
            ),
        )?;
    }
    if payload
        .render
        .passes
        .contains(&ConditioningPass::PoseKeypoints)
    {
        let poses: Vec<_> = payload
            .subjects
            .iter()
            .map(|subject| {
                serde_json::json!({
                    "subject_id": subject.id,
                    "pose_track": subject.pose_track,
                })
            })
            .collect();
        fs::write(
            metadata.join("pose.json"),
            format!("{}\n", serde_json::to_string_pretty(&poses)?),
        )?;
    }
    Ok(())
}

fn preserve_failed_transaction(transaction: &Path, root: &Path, scene_id: &str) -> Result<PathBuf> {
    let failed = root.join(format!(".{scene_id}.failed-{}", std::process::id()));
    if failed.exists() {
        fs::remove_dir_all(&failed)?;
    }
    fs::rename(transaction, &failed).with_context(|| {
        format!(
            "failed to preserve transaction {} as {}",
            transaction.display(),
            failed.display()
        )
    })?;
    Ok(failed)
}

fn promote_directory(transaction: &Path, destination: &Path) -> Result<()> {
    let parent = destination
        .parent()
        .ok_or_else(|| anyhow!("output directory has no parent"))?;
    let name = destination
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("scene");
    let backup = parent.join(format!(".{name}.previous-{}", std::process::id()));
    if backup.exists() {
        fs::remove_dir_all(&backup)?;
    }
    let had_previous = destination.exists();
    if had_previous {
        fs::rename(destination, &backup)?;
    }
    if let Err(error) = fs::rename(transaction, destination) {
        if had_previous {
            let _ = fs::rename(&backup, destination);
        }
        return Err(error).with_context(|| {
            format!(
                "failed to publish {} as {}",
                transaction.display(),
                destination.display()
            )
        });
    }
    if had_previous {
        fs::remove_dir_all(backup)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blender_command_propagates_python_failures() {
        let command = blender_command(Path::new("blender"), Path::new("build.py"));
        let args: Vec<_> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            args,
            [
                "--background",
                "--factory-startup",
                "--disable-autoexec",
                "--python-exit-code",
                "70",
                "--python",
                "build.py",
            ]
        );
    }
}
