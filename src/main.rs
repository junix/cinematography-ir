use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use cinematography_ir::execute::blender::{BlenderAdapter, BlenderOptions};
use cinematography_ir::view::{
    ControlProfile, Encoding, LabelMode, Layout, OverlaySet, PanelKind, Sampling, StripAxis,
    ViewIntent,
};
use cinematography_ir::{
    analyze_continuity, compile_project, emit_project_prompts, load_project, render_summary,
    render_view, save_project_json, solve_project, validate_project, CineProject,
    CinematographyAdapter, CompileOptions, ConditioningPass, ExecutionProfile, ExecutionRequest,
    Fidelity, FrameRange, PassKind, PassSpec, PromptDialect, PromptOptions, Severity, SolveError,
    SolveOptions, SolvedProject, ValidationReport, VideoModelProfile, ViewRequest,
};
use clap::{Parser, Subcommand};
use schemars::schema_for;

#[derive(Debug, Parser)]
#[command(name = "cine-ir")]
#[command(about = "Validate, inspect, and exchange Cinematography IR documents")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run structural, numeric, reference, timeline, and continuity checks.
    Validate {
        input: PathBuf,
        /// Emit machine-readable diagnostics.
        #[arg(long)]
        json: bool,
        /// Return a non-zero status when warnings are present.
        #[arg(long)]
        deny_warnings: bool,
    },
    /// Run only cross-shot continuity analysis.
    Analyze {
        input: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Print a compact human-readable shot plan.
    Inspect { input: PathBuf },
    /// Convert YAML or JSON into canonical pretty-printed JSON.
    Normalize {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Generate JSON Schema from the current Rust model.
    Schema {
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Emit the Solved Camera IR schema instead of the source IR schema.
        #[arg(long)]
        solved: bool,
        /// Emit the estimated-trajectory schema consumed by `compare`.
        #[arg(long)]
        estimated: bool,
        /// Emit the shared Compiled Guidance IR schema.
        #[arg(long)]
        compiled: bool,
        /// Emit the typed video-model execution profile schema.
        #[arg(long)]
        execution_profile: bool,
    },
    /// Compare an estimated camera trajectory (e.g. recovered from a generated video) with the solved intent.
    Compare {
        /// Cinematography IR (.yaml/.json) or a solved document.
        input: PathBuf,
        /// Estimated trajectory JSON (`cine-ir schema --estimated`).
        estimate: PathBuf,
        /// Restrict to one scene id (default: the first scene).
        #[arg(long)]
        scene: Option<String>,
        /// `none`, `translation`, or `similarity`.
        #[arg(long, default_value = "translation")]
        align: String,
        /// `exact` or `dtw`; DTW is available for source/compiled guidance input.
        #[arg(long, default_value = "exact")]
        temporal: String,
        #[arg(long)]
        json: bool,
    },
    /// Compile semantic camera operations and blocking into per-frame Solved Camera IR.
    Solve {
        input: PathBuf,
        /// Write solved JSON here instead of stdout.
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Solver fidelity: `draft` (kinematic) or `full` (rig limits + proxy collisions).
        #[arg(long, default_value = "draft")]
        fidelity: String,
        /// Fail when the solver or validator reports warnings.
        #[arg(long)]
        deny_warnings: bool,
    },
    /// Compile source intent into shared phases, tracks, constraints, and edit relations.
    Compile {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Through-the-lens aspect ratio used for screen-space tracks.
        #[arg(long, default_value_t = 16.0 / 9.0)]
        aspect: f32,
        #[arg(long)]
        deny_warnings: bool,
    },
    /// Emit text prompts for shots using a prompt dialect profile.
    Prompt {
        input: PathBuf,
        /// Restrict to one scene id.
        #[arg(long)]
        scene: Option<String>,
        /// Restrict to one shot id.
        #[arg(long)]
        shot: Option<String>,
        /// Dialect: `generic` (embedded), a profile name resolved to
        /// `profiles/prompt/<name>.json`, or a path to a dialect JSON file.
        #[arg(long, default_value = "generic")]
        dialect: String,
        /// Omit distances, angles, and durations.
        #[arg(long)]
        no_measurements: bool,
        /// Omit the scene title / dramatic question prefix.
        #[arg(long)]
        no_scene_context: bool,
        /// Omit "Name (colour)" identity bindings.
        #[arg(long)]
        no_color_binding: bool,
        /// Emit a JSON array of {scene_id, shot_id, text}.
        #[arg(long)]
        json: bool,
    },
    /// Render plan / frame panels as svg, png, ascii, html, or markdown.
    View {
        input: PathBuf,
        /// Restrict to one scene id.
        #[arg(long)]
        scene: Option<String>,
        /// `storyboard`, `motion`, `continuity`, `model-control`, `constraints`, or `comparison`.
        #[arg(long, default_value = "storyboard")]
        intent: String,
        /// `none`, `color`, or `color+name` (default: color+name for human, color for conditioning).
        #[arg(long)]
        labels: Option<String>,
        /// Panels per cell: plan,frame,elevation,timeline,metrics.
        #[arg(long, default_value = "plan,frame")]
        panel: String,
        /// per-shot, per-beat, op-endpoints, semantic-events, phase-keyframes,
        /// constraint-extrema, cut-pairs, every:N, or continuous[:FPS].
        #[arg(long)]
        sampling: Option<String>,
        /// Operation cue scope: current, upcoming, or whole-shot.
        #[arg(long, default_value = "current")]
        cue_scope: String,
        /// Emit a separate cue-map artifact for model-control profiles that support scribbles.
        #[arg(long)]
        cue_map: bool,
        /// Comma list of arrows,paths,axes,eyelines,directions,guides,diagnostics,grid,markers,all,none
        /// (default: all for human, arrows,paths for conditioning; conditioning never draws text).
        #[arg(long)]
        overlay: Option<String>,
        /// `grid[:COLS]`, `strip[:h|v]`, `separate`, or `animate[:FPS]` (html only;
        /// `animate:FPS` is shorthand for `--sampling continuous:FPS`).
        #[arg(long, default_value = "grid:2")]
        layout: String,
        /// `svg`, `png[:SCALE]`, `ascii[:COLS]`, `html`, or `markdown[:COLS]`.
        #[arg(long, default_value = "svg")]
        format: String,
        /// Frame panel aspect ratio (width / height).
        #[arg(long, default_value_t = 16.0 / 9.0)]
        aspect: f32,
        /// Output file (single artifact) or directory (multiple). Text formats default to stdout.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Drive an external runtime to export conditioning passes.
    Render {
        #[command(subcommand)]
        backend: RenderBackend,
    },
}

#[derive(Debug, Subcommand)]
enum RenderBackend {
    /// Build clay proxies in Blender and export depth / normal / id / vector / openpose / beauty.
    Blender {
        /// Cinematography IR (.yaml/.json) or a solved document from `cine-ir solve`.
        input: PathBuf,
        /// Directory receiving transactional scene and per-shot guidance bundles.
        #[arg(long)]
        out_dir: PathBuf,
        /// ExecutionProfile JSON. CLI pass/resolution options override it when present.
        #[arg(long)]
        profile: Option<PathBuf>,
        /// Comma-separated passes: beauty,depth,depth_metric,normal,id,vector,openpose,occlusion,camera,pose_keypoints.
        #[arg(long)]
        passes: Option<String>,
        /// Restrict to one scene id.
        #[arg(long)]
        scene: Option<String>,
        /// Scene-local half-open frame range, e.g. `0..48`.
        #[arg(long)]
        frames: Option<String>,
        /// Output resolution, e.g. `1024x576`.
        #[arg(long)]
        resolution: Option<String>,
        /// Cycles samples for the beauty layer.
        #[arg(long, default_value_t = 32)]
        samples: u32,
        /// Enable depth of field on the beauty layer.
        #[arg(long)]
        dof: bool,
        /// Blender executable (defaults to $BLENDER, then `blender` on PATH).
        #[arg(long)]
        blender: Option<PathBuf>,
        /// Write build.py and manifest.json without launching Blender.
        #[arg(long)]
        script_only: bool,
        /// Per-scene Blender timeout in seconds; zero disables it.
        #[arg(long, default_value_t = 1800)]
        timeout: u64,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Validate {
            input,
            json,
            deny_warnings,
        } => {
            let project = load_project(&input)?;
            let report = validate_project(&project);
            print_report(&report, json)?;
            if report.has_errors() || (deny_warnings && report.has_warnings()) {
                std::process::exit(2);
            }
        }
        Command::Analyze { input, json } => {
            let project = load_project(&input)?;
            let mut report = ValidationReport::default();
            for scene in &project.scenes {
                report.extend(analyze_continuity(scene).diagnostics);
            }
            print_report(&report, json)?;
            if report.has_errors() {
                std::process::exit(2);
            }
        }
        Command::Inspect { input } => {
            let project = load_project(&input)?;
            print!("{}", render_summary(&project));
        }
        Command::Normalize { input, output } => {
            let project = load_project(&input)?;
            match output {
                Some(path) => save_project_json(&project, path)?,
                None => println!("{}", serde_json::to_string_pretty(&project)?),
            }
        }
        Command::Schema {
            output,
            solved,
            estimated,
            compiled,
            execution_profile,
        } => {
            if [solved, estimated, compiled, execution_profile]
                .into_iter()
                .filter(|selected| *selected)
                .count()
                > 1
            {
                anyhow::bail!("choose only one schema variant");
            }
            let schema = if solved {
                schema_for!(cinematography_ir::SolvedProject)
            } else if compiled {
                schema_for!(cinematography_ir::CompiledProject)
            } else if estimated {
                schema_for!(cinematography_ir::compare::EstimatedTrajectory)
            } else if execution_profile {
                schema_for!(cinematography_ir::ExecutionProfile)
            } else {
                schema_for!(CineProject)
            };
            let text = serde_json::to_string_pretty(&schema)?;
            match output {
                Some(path) => fs::write(&path, format!("{text}\n"))
                    .with_context(|| format!("failed to write {}", path.display()))?,
                None => println!("{text}"),
            }
        }
        Command::Solve {
            input,
            output,
            fidelity,
            deny_warnings,
        } => {
            let project = load_project(&input)?;
            let fidelity = match fidelity.as_str() {
                "draft" => Fidelity::Draft,
                "full" => Fidelity::Full,
                other => anyhow::bail!("unknown fidelity '{other}'; expected draft or full"),
            };
            let result = solve_project(&project, &SolveOptions { fidelity });
            let solved = match result {
                Ok(output) => output,
                Err(SolveError::Invalid(report)) => {
                    eprint_report(&report);
                    std::process::exit(2);
                }
                Err(error) => return Err(error.into()),
            };
            eprint_report(&solved.report);
            let text = serde_json::to_string_pretty(&solved.solved)?;
            match output {
                Some(path) => fs::write(&path, format!("{text}\n"))
                    .with_context(|| format!("failed to write {}", path.display()))?,
                None => println!("{text}"),
            }
            if deny_warnings && solved.report.has_warnings() {
                std::process::exit(2);
            }
        }
        Command::Compile {
            input,
            output,
            aspect,
            deny_warnings,
        } => {
            let project = load_project(&input)?;
            let compiled = compile_project(
                &project,
                &CompileOptions {
                    aspect_ratio: aspect,
                    ..CompileOptions::default()
                },
            )
            .map_err(anyhow::Error::new)?;
            eprint_report(&compiled.report);
            if compiled.report.has_errors() || (deny_warnings && compiled.report.has_warnings()) {
                std::process::exit(2);
            }
            let text = format!("{}\n", serde_json::to_string_pretty(&compiled.compiled)?);
            match output {
                Some(path) => fs::write(&path, text)
                    .with_context(|| format!("failed to write {}", path.display()))?,
                None => print!("{text}"),
            }
        }
        Command::Prompt {
            input,
            scene,
            shot,
            dialect,
            no_measurements,
            no_scene_context,
            no_color_binding,
            json,
        } => {
            let project = load_project(&input)?;
            let report = validate_project(&project);
            eprint_report(&report);
            if report.has_errors() {
                std::process::exit(2);
            }
            let dialect = load_dialect(&dialect)?;
            let options = PromptOptions {
                include_measurements: !no_measurements,
                include_scene_context: !no_scene_context,
                include_color_binding: !no_color_binding,
            };
            let prompts = emit_project_prompts(
                &project,
                &dialect,
                &options,
                scene.as_deref(),
                shot.as_deref(),
            );
            if prompts.is_empty() {
                anyhow::bail!("no shots matched the given scene/shot filters");
            }
            if json {
                println!("{}", serde_json::to_string_pretty(&prompts)?);
            } else {
                for (index, prompt) in prompts.iter().enumerate() {
                    if index > 0 {
                        println!();
                    }
                    println!("## {} / {}", prompt.scene_id, prompt.shot_id);
                    println!("{}", prompt.text);
                }
            }
        }
        Command::Compare {
            input,
            estimate,
            scene,
            align,
            temporal,
            json,
        } => {
            use cinematography_ir::compare::{
                compare_compiled_guidance_with_temporal, compare_trajectories,
                render_compiled_report, render_report, Alignment, TemporalAlignment,
            };
            let text = fs::read_to_string(&estimate)
                .with_context(|| format!("failed to read {}", estimate.display()))?;
            let estimated: cinematography_ir::compare::EstimatedTrajectory =
                serde_json::from_str(&text)
                    .with_context(|| format!("invalid estimate {}", estimate.display()))?;
            let alignment = match align.as_str() {
                "none" => Alignment::None,
                "translation" => Alignment::Translation,
                "similarity" | "scale" => Alignment::Similarity,
                other => anyhow::bail!("unknown alignment '{other}'"),
            };
            let temporal_alignment = match temporal.as_str() {
                "exact" | "frame" => TemporalAlignment::Exact,
                "dtw" | "dynamic-time-warping" => TemporalAlignment::DynamicTimeWarping,
                other => anyhow::bail!("unknown temporal alignment '{other}'"),
            };
            if let Ok(project) = load_project(&input) {
                let compiled = compile_project(&project, &CompileOptions::default())?.compiled;
                let target = match &scene {
                    Some(id) => compiled
                        .scene(id)
                        .ok_or_else(|| anyhow::anyhow!("no scene '{id}'"))?,
                    None => compiled
                        .scenes
                        .first()
                        .ok_or_else(|| anyhow::anyhow!("document has no scenes"))?,
                };
                let report = compare_compiled_guidance_with_temporal(
                    target,
                    &estimated,
                    alignment,
                    temporal_alignment,
                );
                if json {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                } else {
                    print!("{}", render_compiled_report(&report));
                }
                if report.shots.is_empty() {
                    std::process::exit(1);
                }
            } else {
                if temporal_alignment != TemporalAlignment::Exact {
                    anyhow::bail!(
                        "DTW requires source Cinematography IR so screen-space guidance is available"
                    );
                }
                let solved = load_solved_or_project(&input)?;
                let target = match &scene {
                    Some(id) => solved
                        .scene(id)
                        .ok_or_else(|| anyhow::anyhow!("no scene '{id}'"))?,
                    None => solved
                        .scenes
                        .first()
                        .ok_or_else(|| anyhow::anyhow!("document has no scenes"))?,
                };
                let report = compare_trajectories(target, &estimated, alignment);
                if json {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                } else {
                    print!("{}", render_report(&report));
                }
                if report.shots.is_empty() {
                    std::process::exit(1);
                }
            }
        }
        Command::View {
            input,
            scene,
            intent,
            labels,
            panel,
            sampling,
            cue_scope,
            cue_map,
            overlay,
            layout,
            format,
            aspect,
            output,
        } => {
            let project = load_project(&input)?;
            let request = build_view_request(
                scene,
                &intent,
                labels.as_deref(),
                &panel,
                sampling.as_deref(),
                &cue_scope,
                cue_map,
                overlay.as_deref(),
                &layout,
                &format,
                aspect,
            )?;
            let artifacts = match render_view(&project, &request) {
                Ok(artifacts) => artifacts,
                Err(cinematography_ir::ViewError::Solve(SolveError::Invalid(report))) => {
                    eprint_report(&report);
                    std::process::exit(2);
                }
                Err(error) => return Err(error.into()),
            };
            write_artifacts(artifacts, output.as_deref())?;
        }
        Command::Render {
            backend:
                RenderBackend::Blender {
                    input,
                    out_dir,
                    profile,
                    passes,
                    scene,
                    frames,
                    resolution,
                    samples,
                    dof,
                    blender,
                    script_only,
                    timeout,
                },
        } => {
            let mut execution_profile = profile
                .as_ref()
                .map(load_execution_profile)
                .transpose()?
                .unwrap_or_else(|| ExecutionProfile {
                    name: "cli_blender".to_owned(),
                    model: VideoModelProfile::default(),
                    passes: ConditioningPass::parse_list("beauty,depth,normal,id")
                        .expect("built-in pass list is valid")
                        .into_iter()
                        .map(|pass| PassSpec::for_conditioning(pass, 0.1, 100.0))
                        .collect(),
                    resolution: [1024, 576],
                    deterministic_seed: 42,
                });
            if let Some(passes) = passes.as_deref() {
                execution_profile.passes = ConditioningPass::parse_list(passes)
                    .map_err(|e| anyhow::anyhow!(e))?
                    .into_iter()
                    .map(|pass| PassSpec::for_conditioning(pass, 0.1, 100.0))
                    .collect();
            }
            if let Some(resolution) = resolution.as_deref() {
                let value = parse_resolution(resolution)?;
                execution_profile.resolution = [value.0, value.1];
            }
            let passes: Vec<_> = execution_profile
                .passes
                .iter()
                .map(|spec| pass_kind_to_conditioning(spec.kind))
                .collect();
            let resolution = (
                execution_profile.resolution[0],
                execution_profile.resolution[1],
            );
            let range = frames.as_deref().map(parse_frame_range).transpose()?;
            let mut adapter = BlenderAdapter::new(BlenderOptions {
                resolution,
                samples,
                use_dof: dof,
                blender_path: blender,
                script_only,
                timeout_s: timeout,
            });
            if let Ok(project) = load_project(&input) {
                let output = compile_project(
                    &project,
                    &CompileOptions {
                        aspect_ratio: resolution.0 as f32 / resolution.1.max(1) as f32,
                        ..CompileOptions::default()
                    },
                )?;
                let profile = execution_profile;
                let mut staged = Vec::new();
                for compiled_scene in &output.compiled.scenes {
                    if scene.as_deref().is_some_and(|id| id != compiled_scene.id) {
                        continue;
                    }
                    adapter.stage_scene(&output.compiled, compiled_scene, &profile)?;
                    staged.push(compiled_scene.id.clone());
                }
                if staged.is_empty() {
                    anyhow::bail!("no scene matched");
                }
                let mut bundles = Vec::new();
                for scene_id in staged {
                    bundles.push(adapter.execute(
                        &ExecutionRequest {
                            scene_id,
                            shot_ids: Vec::new(),
                            frame_range: range,
                            profile: profile.clone(),
                        },
                        &out_dir,
                    )?);
                }
                println!("{}", serde_json::to_string_pretty(&bundles)?);
            } else {
                let solved = load_solved_or_project(&input)?;
                let mut staged = 0;
                for solved_scene in &solved.scenes {
                    if scene.as_deref().is_some_and(|id| id != solved_scene.id) {
                        continue;
                    }
                    adapter.apply_scene(&solved, solved_scene)?;
                    staged += 1;
                }
                if staged == 0 {
                    anyhow::bail!("no scene matched");
                }
                let output = adapter.render_passes(range, &passes, &out_dir)?;
                println!("{}", serde_json::to_string_pretty(&output)?);
            }
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn build_view_request(
    scene: Option<String>,
    intent: &str,
    labels: Option<&str>,
    panel: &str,
    sampling: Option<&str>,
    cue_scope: &str,
    cue_map: bool,
    overlay: Option<&str>,
    layout: &str,
    format: &str,
    aspect: f32,
) -> Result<ViewRequest> {
    let mut request = match intent {
        "human" | "storyboard" => ViewRequest::human(),
        "conditioning" | "cond" | "model-control" => ViewRequest::conditioning(),
        "motion" | "motion-grammar" => {
            let mut request = ViewRequest::human();
            request.view_intent = ViewIntent::MotionGrammar;
            request
        }
        "continuity" => {
            let mut request = ViewRequest::human();
            request.view_intent = ViewIntent::ContinuityReview;
            request.sampling = Sampling::CutPairs;
            request
        }
        "constraints" | "diagnostics" => {
            let mut request = ViewRequest::human();
            request.view_intent = ViewIntent::ConstraintDiagnostics;
            request.sampling = Sampling::ConstraintExtrema;
            request
        }
        "comparison" | "generated-comparison" => {
            let mut request = ViewRequest::human();
            request.view_intent = ViewIntent::GeneratedComparison;
            request
        }
        other => anyhow::bail!("unknown view intent '{other}'"),
    };
    if matches!(request.view_intent, ViewIntent::ModelControl(_)) {
        request.view_intent = ViewIntent::ModelControl(ControlProfile {
            include_cue_map: cue_map,
            ..ControlProfile::default()
        });
    } else if cue_map {
        anyhow::bail!("--cue-map is only valid with --intent model-control");
    }
    request.cue_scope = match cue_scope {
        "current" => cinematography_ir::view::CueScope::Current,
        "upcoming" | "current-and-upcoming" => {
            cinematography_ir::view::CueScope::CurrentAndUpcoming
        }
        "whole-shot" | "summary" => cinematography_ir::view::CueScope::WholeShotSummary,
        other => anyhow::bail!("unknown cue scope '{other}'"),
    };
    request.scene_id = scene;
    if let Some(labels) = labels {
        request.labels = match labels {
            "none" => LabelMode::None,
            "color" | "colour" => LabelMode::Color,
            "color+name" | "colour+name" | "name" => LabelMode::ColorName,
            other => anyhow::bail!("unknown label mode '{other}'"),
        };
    }
    request.panels = panel
        .split(',')
        .map(|p| match p.trim() {
            "plan" => Ok(PanelKind::Plan),
            "frame" => Ok(PanelKind::Frame),
            "elevation" | "side" => Ok(PanelKind::Elevation),
            "timeline" => Ok(PanelKind::Timeline),
            "metrics" | "constraints" => Ok(PanelKind::Metrics),
            other => Err(anyhow::anyhow!("unknown panel '{other}'")),
        })
        .collect::<Result<Vec<_>>>()?;
    if request.panels.is_empty() {
        anyhow::bail!("at least one panel is required");
    }
    let mut animate_fps: Option<u32> = None;
    if let Some(overlay) = overlay {
        request.overlays = OverlaySet::parse_list(overlay).map_err(|e| anyhow::anyhow!(e))?;
    }
    request.layout = match layout.split_once(':') {
        None => match layout {
            "grid" => Layout::Grid { columns: 2 },
            "strip" => Layout::Strip {
                axis: StripAxis::Horizontal,
            },
            "separate" => Layout::Separate,
            "animate" => Layout::Animate,
            other => anyhow::bail!("unknown layout '{other}'"),
        },
        Some(("grid", cols)) => Layout::Grid {
            columns: cols.parse()?,
        },
        Some(("strip", axis)) => Layout::Strip {
            axis: match axis {
                "h" | "horizontal" => StripAxis::Horizontal,
                "v" | "vertical" => StripAxis::Vertical,
                other => anyhow::bail!("unknown strip axis '{other}'"),
            },
        },
        Some(("animate", fps)) => {
            animate_fps = Some(fps.parse()?);
            Layout::Animate
        }
        Some((other, _)) => anyhow::bail!("unknown layout '{other}'"),
    };
    request.sampling = match sampling {
        Some(sampling) => match sampling.split_once(':') {
            None => match sampling {
                "per-shot" | "shot" => Sampling::PerShot,
                "per-beat" | "beat" => Sampling::PerBeat,
                "op-endpoints" | "ops" => Sampling::OperationEndpoints,
                "semantic-events" | "events" => Sampling::SemanticEvents,
                "phase-keyframes" | "phases" => Sampling::PhaseKeyframes,
                "constraint-extrema" | "extrema" => Sampling::ConstraintExtrema,
                "cut-pairs" | "cuts" => Sampling::CutPairs,
                "continuous" => Sampling::Continuous { fps: 0 },
                other => anyhow::bail!("unknown sampling '{other}'"),
            },
            Some(("every", n)) => Sampling::EveryNFrames(n.parse()?),
            Some(("continuous", fps)) => Sampling::Continuous { fps: fps.parse()? },
            Some((other, _)) => anyhow::bail!("unknown sampling '{other}'"),
        },
        None => match animate_fps {
            Some(fps) => Sampling::Continuous { fps },
            None => request.sampling.clone(),
        },
    };
    if matches!(request.view_intent, ViewIntent::ModelControl(_)) {
        request.overlays = OverlaySet::NONE;
        request.labels = LabelMode::Color;
    }
    request.encoding = match format.split_once(':') {
        None => match format {
            "svg" => Encoding::Svg,
            "png" => Encoding::Png { scale: 2.0 },
            "ascii" | "txt" => Encoding::Ascii { columns: 72 },
            "html" => Encoding::Html,
            "markdown" | "md" => Encoding::Markdown { columns: 72 },
            other => anyhow::bail!("unknown format '{other}'"),
        },
        Some(("png", scale)) => Encoding::Png {
            scale: scale.parse()?,
        },
        Some(("ascii", cols)) | Some(("txt", cols)) => Encoding::Ascii {
            columns: cols.parse()?,
        },
        Some(("markdown", cols)) | Some(("md", cols)) => Encoding::Markdown {
            columns: cols.parse()?,
        },
        Some((other, _)) => anyhow::bail!("unknown format '{other}'"),
    };
    if !(aspect.is_finite() && aspect > 0.1) {
        anyhow::bail!("aspect must be a positive ratio");
    }
    request.aspect = aspect;
    if matches!(request.layout, Layout::Animate) && request.encoding != Encoding::Html {
        anyhow::bail!("--layout animate requires --format html");
    }
    Ok(request)
}

/// Writes artifacts to a file, a directory, or stdout (text formats only).
fn write_artifacts(
    artifacts: Vec<cinematography_ir::ViewArtifact>,
    output: Option<&std::path::Path>,
) -> Result<()> {
    match output {
        None => {
            for artifact in &artifacts {
                match artifact.text() {
                    Some(text) => {
                        if artifacts.len() > 1 {
                            println!("==> {}", artifact.name);
                        }
                        print!("{text}");
                    }
                    None => anyhow::bail!(
                        "binary artifact '{}' needs --output <file-or-dir>",
                        artifact.name
                    ),
                }
            }
        }
        Some(path) => {
            let as_dir = artifacts.len() > 1
                || path.is_dir()
                || path.to_string_lossy().ends_with('/')
                || path.extension().is_none();
            if as_dir {
                fs::create_dir_all(path)
                    .with_context(|| format!("failed to create {}", path.display()))?;
                for artifact in &artifacts {
                    let target = path.join(&artifact.name);
                    fs::write(&target, &artifact.bytes)
                        .with_context(|| format!("failed to write {}", target.display()))?;
                    eprintln!("wrote {}", target.display());
                }
            } else {
                let artifact = &artifacts[0];
                fs::write(path, &artifact.bytes)
                    .with_context(|| format!("failed to write {}", path.display()))?;
                eprintln!("wrote {}", path.display());
            }
        }
    }
    Ok(())
}

/// Accepts either a source document (solved on the fly) or a solved document.
fn load_solved_or_project(path: &PathBuf) -> Result<SolvedProject> {
    let text =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    // Structural detection: any JSON whose schema_version starts with
    // "solved-" is a solved document, however it was formatted.
    #[derive(serde::Deserialize)]
    struct Head {
        schema_version: String,
    }
    let is_solved = serde_json::from_str::<Head>(&text)
        .map(|head| head.schema_version.starts_with("solved-"))
        .unwrap_or(false);
    if is_solved {
        return serde_json::from_str(&text)
            .with_context(|| format!("invalid solved document {}", path.display()));
    }
    let project = load_project(path)?;
    match solve_project(&project, &SolveOptions::default()) {
        Ok(output) => {
            eprint_report(&output.report);
            Ok(output.solved)
        }
        Err(SolveError::Invalid(report)) => {
            eprint_report(&report);
            std::process::exit(2);
        }
        Err(error) => Err(error.into()),
    }
}

fn load_execution_profile(path: &PathBuf) -> Result<ExecutionProfile> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("failed to read execution profile {}", path.display()))?;
    let profile: ExecutionProfile = serde_json::from_str(&text)
        .with_context(|| format!("invalid execution profile {}", path.display()))?;
    if profile.resolution[0] == 0 || profile.resolution[1] == 0 {
        anyhow::bail!("execution profile resolution must be non-zero");
    }
    if profile.passes.is_empty() {
        anyhow::bail!("execution profile must request at least one pass");
    }
    Ok(profile)
}

fn pass_kind_to_conditioning(kind: PassKind) -> ConditioningPass {
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

/// `generic`, a bare profile name under `profiles/prompt/`, or a file path.
fn load_dialect(spec: &str) -> Result<PromptDialect> {
    if spec == "generic" {
        return Ok(PromptDialect::generic());
    }
    let looks_like_path = spec.contains('/') || spec.contains('\\') || spec.ends_with(".json");
    let candidates: Vec<PathBuf> = if looks_like_path {
        vec![PathBuf::from(spec)]
    } else {
        vec![
            PathBuf::from("profiles/prompt").join(format!("{spec}.json")),
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("profiles/prompt")
                .join(format!("{spec}.json")),
        ]
    };
    let Some(path) = candidates.iter().find(|p| p.is_file()) else {
        anyhow::bail!(
            "dialect '{spec}' not found; looked for {}",
            candidates
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    };
    let text =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    PromptDialect::from_json(&text).with_context(|| format!("invalid dialect {}", path.display()))
}

fn parse_resolution(text: &str) -> Result<(u32, u32)> {
    let (w, h) = text
        .split_once('x')
        .ok_or_else(|| anyhow::anyhow!("resolution must look like 1024x576"))?;
    Ok((w.trim().parse()?, h.trim().parse()?))
}

fn parse_frame_range(text: &str) -> Result<FrameRange> {
    let (start, end) = text
        .split_once("..")
        .ok_or_else(|| anyhow::anyhow!("frame range must look like 0..48"))?;
    let range = FrameRange {
        start: start.trim().parse()?,
        end: end.trim().parse()?,
    };
    if !range.is_valid() {
        anyhow::bail!("frame range start must be less than end");
    }
    Ok(range)
}

/// Human-readable diagnostics on stderr, keeping stdout for data.
fn eprint_report(report: &ValidationReport) {
    for diagnostic in &report.diagnostics {
        let severity = match diagnostic.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        eprintln!(
            "{severity}[{}] {}: {}",
            diagnostic.code, diagnostic.path, diagnostic.message
        );
        if let Some(hint) = &diagnostic.hint {
            eprintln!("  hint: {hint}");
        }
    }
    if !report.diagnostics.is_empty() {
        eprintln!(
            "{} error(s), {} warning(s)",
            report.error_count(),
            report.warning_count()
        );
    }
}

fn print_report(report: &ValidationReport, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(report)?);
        return Ok(());
    }

    if report.diagnostics.is_empty() {
        println!("OK: no diagnostics");
        return Ok(());
    }

    for diagnostic in &report.diagnostics {
        let severity = match diagnostic.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        println!(
            "{severity}[{}] {}: {}",
            diagnostic.code, diagnostic.path, diagnostic.message
        );
        if let Some(hint) = &diagnostic.hint {
            println!("  hint: {hint}");
        }
    }

    println!(
        "\n{} error(s), {} warning(s)",
        report.error_count(),
        report.warning_count()
    );
    Ok(())
}
