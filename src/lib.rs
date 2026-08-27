//! A layered, serializable intermediate representation for cinematography planning.
//!
//! Layers (see ADR-1115):
//!
//! * `model` / `validate` / `analyze` — the semantic IR and its static checks.
//! * `solve` — semantic camera language → per-frame Solved Camera IR.
//! * `prompt` — the text-prompt conditioning slot.
//! * `view` — pure 2D renderers (plan / frame canvases → svg, png, ascii, html).
//! * `execute` — adapters that drive external runtimes (Blender).

pub mod analyze;
pub mod compare;
pub mod compiled;
pub mod diagnostic;
pub mod execute;
pub mod io;
pub mod math;
pub mod model;
pub mod palette;
pub mod prompt;
pub mod report;
pub mod solve;
pub mod validate;
pub mod view;

pub use analyze::analyze_continuity;
pub use compiled::*;
pub use diagnostic::{Diagnostic, Severity, ValidationReport};
pub use execute::{
    AdapterCapabilities, ChannelLayout, CinematographyAdapter, ColorSpace, ConditioningPass,
    ExecutionBundle, ExecutionProfile, ExecutionRequest, PassKind, PassOutput, PassSpec,
    PixelEncoding, VideoModelProfile,
};
pub use io::{load_project, save_project_json};
pub use model::*;
pub use prompt::{
    emit_guidance_command, emit_project_prompts, emit_shot_prompt, PromptDialect, PromptOptions,
    ShotPrompt,
};
pub use report::render_summary;
pub use solve::{
    normalize_units, solve_project, Fidelity, SolveError, SolveOptions, SolveOutput, SolvedProject,
};
pub use validate::validate_project;
pub use view::{render_view, ViewArtifact, ViewError, ViewRequest};
