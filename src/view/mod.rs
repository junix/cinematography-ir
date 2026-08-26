//! View layer (ADR-1115 D1 ②/③, D4, D5, D7): pure renderers from
//! `CineProject` to self-contained artifacts — plan and frame panels encoded
//! as SVG, PNG, ASCII, HTML (still or insight-video animation), or Markdown.
//!
//! Purity contract: nothing in this module touches the filesystem, spawns a
//! process, or reads the clock. The same document and request always yield
//! byte-identical artifacts.

pub mod canvas;
pub mod encode;
pub mod layout;
pub mod panel;
pub mod project;
pub mod sample;

use std::fmt;

use crate::analyze::analyze_continuity;
use crate::diagnostic::Diagnostic;
use crate::model::{CineProject, Scene};
use crate::prompt::{emit_shot_prompt, PromptDialect, PromptOptions};
use crate::solve::{solve_project, SolveError, SolveOptions, SolvedScene};

pub use canvas::{Canvas, CanvasItem};
pub use layout::{Cell, Layout, StripAxis};
pub use panel::{LabelMode, OverlaySet, PanelContext, PanelStyle, RenderIntent};
pub use sample::{SamplePoint, Sampling};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelKind {
    Plan,
    Frame,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Encoding {
    Svg,
    Png {
        scale: f32,
    },
    Ascii {
        columns: usize,
    },
    Html,
    /// ASCII panels plus the shot's prompt text per cell.
    Markdown {
        columns: usize,
    },
}

impl Encoding {
    pub fn extension(&self) -> &'static str {
        match self {
            Encoding::Svg => "svg",
            Encoding::Png { .. } => "png",
            Encoding::Ascii { .. } => "txt",
            Encoding::Html => "html",
            Encoding::Markdown { .. } => "md",
        }
    }

    pub fn media_type(&self) -> MediaType {
        match self {
            Encoding::Svg => MediaType::Svg,
            Encoding::Png { .. } => MediaType::Png,
            Encoding::Ascii { .. } => MediaType::Text,
            Encoding::Html => MediaType::Html,
            Encoding::Markdown { .. } => MediaType::Markdown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaType {
    Svg,
    Png,
    Text,
    Html,
    Markdown,
}

impl MediaType {
    pub fn is_text(self) -> bool {
        !matches!(self, MediaType::Png)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ViewRequest {
    pub scene_id: Option<String>,
    pub intent: RenderIntent,
    pub labels: LabelMode,
    pub sampling: Sampling,
    pub panels: Vec<PanelKind>,
    pub overlays: OverlaySet,
    pub layout: Layout,
    pub encoding: Encoding,
    /// Frame panel aspect ratio (width / height).
    pub aspect: f32,
    pub plan_size: f32,
    pub frame_width: f32,
}

impl ViewRequest {
    pub fn human() -> ViewRequest {
        ViewRequest {
            scene_id: None,
            intent: RenderIntent::Human,
            labels: LabelMode::ColorName,
            sampling: Sampling::PerShot,
            panels: vec![PanelKind::Plan, PanelKind::Frame],
            overlays: OverlaySet::ALL,
            layout: Layout::Grid { columns: 2 },
            encoding: Encoding::Svg,
            aspect: 16.0 / 9.0,
            plan_size: panel::PLAN_SIZE,
            frame_width: panel::FRAME_WIDTH,
        }
    }

    /// Conditioning defaults (D7/D7a): colour-coded geometry and motion
    /// arrows, no text, one artifact per sampled moment.
    pub fn conditioning() -> ViewRequest {
        ViewRequest {
            intent: RenderIntent::Conditioning,
            labels: LabelMode::Color,
            overlays: OverlaySet::CONDITIONING,
            layout: Layout::Separate,
            encoding: Encoding::Png { scale: 2.0 },
            ..ViewRequest::human()
        }
    }

    pub fn style(&self) -> PanelStyle {
        PanelStyle {
            intent: self.intent,
            labels: self.labels,
            overlays: self.overlays,
            aspect: self.aspect,
        }
    }
}

impl Default for ViewRequest {
    fn default() -> Self {
        ViewRequest::human()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ViewArtifact {
    /// Suggested file name, stable across runs.
    pub name: String,
    pub media_type: MediaType,
    pub bytes: Vec<u8>,
}

impl ViewArtifact {
    pub fn text(&self) -> Option<&str> {
        self.media_type
            .is_text()
            .then(|| std::str::from_utf8(&self.bytes).ok())
            .flatten()
    }
}

#[derive(Debug)]
pub enum ViewError {
    Solve(SolveError),
    NoScene(String),
    NoSamples(String),
    Unsupported(String),
    Encode(String),
}

impl fmt::Display for ViewError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ViewError::Solve(e) => write!(f, "cannot solve document: {e}"),
            ViewError::NoScene(id) => write!(f, "no scene matched '{id}'"),
            ViewError::NoSamples(id) => write!(f, "scene '{id}' produced no sample points"),
            ViewError::Unsupported(msg) => write!(f, "{msg}"),
            ViewError::Encode(msg) => write!(f, "encoding failed: {msg}"),
        }
    }
}

impl std::error::Error for ViewError {}

impl From<SolveError> for ViewError {
    fn from(e: SolveError) -> Self {
        ViewError::Solve(e)
    }
}

/// Panels for one scene before layout: cells in sample order.
pub struct SceneCells<'a> {
    pub scene: &'a Scene,
    pub samples: Vec<SamplePoint>,
    pub cells: Vec<Cell>,
}

/// Builds the cells of one scene (exposed for tests and custom layouts).
pub fn build_cells<'a>(
    project: &'a CineProject,
    scene: &'a Scene,
    solved: &SolvedScene,
    diagnostics: &[Diagnostic],
    request: &ViewRequest,
) -> SceneCells<'a> {
    let ctx = PanelContext {
        project,
        scene,
        solved,
        diagnostics,
    };
    let style = request.style();
    let fps = project.frame_rate.frames_per_second().unwrap_or(24.0);
    let plan = panel::scene_plan(&ctx, request.plan_size);
    let samples = sample::sample_points(scene, solved, &request.sampling, fps);
    let cells = samples
        .iter()
        .map(|sample| Cell {
            title: format!("{} · f{}", sample.label, sample.scene_frame),
            plan: request
                .panels
                .contains(&PanelKind::Plan)
                .then(|| panel::plan_panel(&ctx, &style, &plan, sample))
                .flatten(),
            frame: request
                .panels
                .contains(&PanelKind::Frame)
                .then(|| panel::frame_panel(&ctx, &style, request.frame_width, sample))
                .flatten(),
        })
        .collect();
    SceneCells {
        scene,
        samples,
        cells,
    }
}

fn encode_canvas(canvas: &Canvas, encoding: &Encoding, title: &str) -> Result<Vec<u8>, ViewError> {
    Ok(match encoding {
        Encoding::Svg => encode::canvas_to_svg(canvas).into_bytes(),
        Encoding::Png { scale } => encode::canvas_to_png(canvas, *scale)?,
        Encoding::Ascii { columns } => encode::canvas_to_ascii(canvas, *columns).into_bytes(),
        Encoding::Html => encode::still_page(title, canvas).into_bytes(),
        Encoding::Markdown { .. } => {
            return Err(ViewError::Unsupported(
                "markdown is assembled per cell, not per canvas".to_owned(),
            ))
        }
    })
}

fn ascii_columns_for(canvas: &Canvas, cell_width: f32, columns: usize) -> usize {
    let cells_across = (canvas.width / cell_width.max(1.0)).round().max(1.0) as usize;
    columns * cells_across
}

/// Renders every requested scene into artifacts.
pub fn render_view(
    project: &CineProject,
    request: &ViewRequest,
) -> Result<Vec<ViewArtifact>, ViewError> {
    let output = solve_project(project, &SolveOptions::default())?;
    let mut diagnostics = output.report.diagnostics.clone();
    for scene in &project.scenes {
        // validate_project already ran continuity analysis inside solve; keep
        // the explicit call so the dependency is visible and future-proof.
        let continuity = analyze_continuity(scene);
        for d in continuity.diagnostics {
            if !diagnostics.contains(&d) {
                diagnostics.push(d);
            }
        }
    }
    let human = request.intent == RenderIntent::Human;
    let ext = request.encoding.extension();
    let media = request.encoding.media_type();
    let mut artifacts = Vec::new();
    let mut matched = false;

    for scene in &project.scenes {
        if request.scene_id.as_deref().is_some_and(|id| id != scene.id) {
            continue;
        }
        matched = true;
        let Some(solved) = output.solved.scene(&scene.id) else {
            continue;
        };
        let built = build_cells(project, scene, solved, &diagnostics, request);
        if built.cells.is_empty() {
            return Err(ViewError::NoSamples(scene.id.clone()));
        }
        let composed: Vec<Canvas> = built.cells.iter().map(|c| c.compose(human)).collect();
        let cell_width = composed.iter().map(|c| c.width).fold(1.0, f32::max);

        if let Encoding::Markdown { columns } = &request.encoding {
            let text = markdown_document(project, scene, &built, &composed, *columns, human);
            artifacts.push(ViewArtifact {
                name: format!("{}.{ext}", scene.id),
                media_type: media,
                bytes: text.into_bytes(),
            });
            continue;
        }

        match &request.layout {
            Layout::Separate => {
                for (sample, canvas) in built.samples.iter().zip(&composed) {
                    let name = format!(
                        "{}_{}_f{:04}.{ext}",
                        scene.id, sample.shot_id, sample.scene_frame
                    );
                    let encoding = with_columns(&request.encoding, canvas, cell_width);
                    artifacts.push(ViewArtifact {
                        name: name.clone(),
                        media_type: media,
                        bytes: encode_canvas(canvas, &encoding, &name)?,
                    });
                }
            }
            Layout::Grid { columns } => {
                let sheet = layout::grid(&composed, *columns, human);
                let encoding = with_columns(&request.encoding, &sheet, cell_width);
                artifacts.push(ViewArtifact {
                    name: format!("{}.{ext}", scene.id),
                    media_type: media,
                    bytes: encode_canvas(&sheet, &encoding, &scene.id)?,
                });
            }
            Layout::Strip { axis } => {
                let sheet = layout::strip(&composed, *axis, human);
                let encoding = with_columns(&request.encoding, &sheet, cell_width);
                artifacts.push(ViewArtifact {
                    name: format!("{}.{ext}", scene.id),
                    media_type: media,
                    bytes: encode_canvas(&sheet, &encoding, &scene.id)?,
                });
            }
            Layout::Animate => {
                if request.encoding != Encoding::Html {
                    return Err(ViewError::Unsupported(
                        "animate layout requires --format html".to_owned(),
                    ));
                }
                let fps = project.frame_rate.frames_per_second().unwrap_or(24.0);
                let frames: Vec<(f64, &Canvas)> = built
                    .samples
                    .iter()
                    .zip(&composed)
                    .map(|(sample, canvas)| (sample.scene_frame as f64 / fps, canvas))
                    .collect();
                let duration = scene.duration_frames as f64 / fps;
                artifacts.push(ViewArtifact {
                    name: format!("{}.html", scene.id),
                    media_type: MediaType::Html,
                    bytes: encode::animation_page(&scene.id, &frames, duration, fps).into_bytes(),
                });
            }
        }
    }

    if !matched {
        return Err(ViewError::NoScene(
            request.scene_id.clone().unwrap_or_default(),
        ));
    }
    Ok(artifacts)
}

/// ASCII column budgets scale with how many cells sit side by side.
fn with_columns(encoding: &Encoding, canvas: &Canvas, cell_width: f32) -> Encoding {
    match encoding {
        Encoding::Ascii { columns } => Encoding::Ascii {
            columns: ascii_columns_for(canvas, cell_width, *columns),
        },
        other => other.clone(),
    }
}

fn markdown_document(
    project: &CineProject,
    scene: &Scene,
    built: &SceneCells<'_>,
    composed: &[Canvas],
    columns: usize,
    human: bool,
) -> String {
    let dialect = PromptDialect::generic();
    let options = PromptOptions {
        include_scene_context: false,
        ..PromptOptions::default()
    };
    let mut out = format!("# {} — {}\n\n", scene.id, scene.title);
    if human {
        if let Some(question) = &scene.narrative.dramatic_question {
            out.push_str(question);
            out.push_str("\n\n");
        }
    }
    for (sample, canvas) in built.samples.iter().zip(composed) {
        out.push_str(&format!(
            "## {} · f{}\n\n",
            sample.label, sample.scene_frame
        ));
        out.push_str("```text\n");
        out.push_str(&encode::canvas_to_ascii(canvas, columns));
        out.push_str("```\n\n");
        if human {
            if let Some(shot) = scene.shots.iter().find(|s| s.id == sample.shot_id) {
                out.push_str(&emit_shot_prompt(project, scene, shot, &dialect, &options));
                out.push_str("\n\n");
            }
        }
    }
    out
}
