//! Solve layer: semantic `CineProject` → per-frame `SolvedProject`
//! (ADR-1115 D1/D12). The only layer that may fail for geometric reasons.

pub mod easing;
pub mod geometry;
pub mod sampler;
pub mod solved;

use std::borrow::Cow;
use std::fmt;

use crate::diagnostic::ValidationReport;
use crate::model::{CameraOperation, CineProject, TargetRef, Unit, Vec3};
use crate::validate::validate_project;

pub use sampler::{aim_height_fraction, default_dimensions, handheld_noise, SceneContext};
pub use solved::{
    Fidelity, SolvedCameraFrame, SolvedCue, SolvedProject, SolvedScene, SolvedShot,
    SolvedSubjectTrack, SOLVED_SCHEMA_VERSION,
};

#[derive(Debug, Clone, Default)]
pub struct SolveOptions {
    pub fidelity: Fidelity,
}

/// Solved document plus the diagnostics produced along the way (validation
/// warnings and geometry checks).
#[derive(Debug, Clone)]
pub struct SolveOutput {
    pub solved: SolvedProject,
    pub report: ValidationReport,
}

#[derive(Debug, Clone)]
pub enum SolveError {
    /// Structural validation reported errors; solving an invalid document is
    /// meaningless. The report is returned so callers can print it.
    Invalid(ValidationReport),
    InvalidFrameRate,
    FidelityUnsupported(Fidelity),
}

impl fmt::Display for SolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SolveError::Invalid(report) => write!(
                f,
                "document has {} validation error(s); fix them before solving",
                report.error_count()
            ),
            SolveError::InvalidFrameRate => write!(f, "frame rate must be positive"),
            SolveError::FidelityUnsupported(fidelity) => {
                write!(f, "fidelity {fidelity:?} is not implemented; use draft")
            }
        }
    }
}

impl std::error::Error for SolveError {}

/// Returns the document with every position, offset, and delta expressed in
/// metres (`coordinate_system.units = meters`). Fields suffixed `_m` and
/// `dimensions_m` are metres by contract and are left alone, so downstream
/// layers never mix centimetre positions with metre distances.
pub fn normalize_units(project: &CineProject) -> Cow<'_, CineProject> {
    let scale = match project.coordinate_system.units {
        Unit::Meters => return Cow::Borrowed(project),
        Unit::Centimeters => 0.01,
    };
    let mut project = project.clone();
    project.coordinate_system.units = Unit::Meters;
    let pos = |v: &mut Vec3| *v = *v * scale;
    let target = |t: &mut TargetRef| {
        if let TargetRef::Point { position } = t {
            *position = *position * scale;
        }
    };
    for scene in &mut project.scenes {
        for subject in &mut scene.subjects {
            pos(&mut subject.initial_transform.position);
        }
        for marker in &mut scene.markers {
            pos(&mut marker.position);
        }
        for axis in &mut scene.axes {
            target(&mut axis.from);
            target(&mut axis.to);
        }
        for track in &mut scene.blocking {
            for keyframe in &mut track.keyframes {
                pos(&mut keyframe.transform.position);
                if let Some(gaze) = &mut keyframe.gaze_target {
                    target(gaze);
                }
            }
        }
        for setup in &mut scene.lighting_setups {
            for source in &mut setup.sources {
                pos(&mut source.transform.position);
            }
        }
        for shot in &mut scene.shots {
            pos(&mut shot.camera.initial_state.transform.position);
            if let Some(focus) = &mut shot.camera.initial_state.focus.target {
                target(focus);
            }
            for timed in &mut shot.camera.operations {
                match &mut timed.operation {
                    CameraOperation::Translate { delta, .. }
                    | CameraOperation::Crane { delta, .. } => pos(delta),
                    CameraOperation::Follow {
                        target: t, offset, ..
                    } => {
                        target(t);
                        pos(offset);
                    }
                    CameraOperation::LookAt { target: t }
                    | CameraOperation::Orbit { target: t, .. }
                    | CameraOperation::RackFocus { target: t, .. }
                    | CameraOperation::DollyZoom { target: t, .. } => target(t),
                    CameraOperation::Reveal {
                        target: t,
                        occluder,
                        ..
                    } => {
                        target(t);
                        target(occluder);
                    }
                    _ => {}
                }
            }
            if let Some(look_at) = shot.camera.operations.iter_mut().find_map(|timed| {
                if let CameraOperation::Crane {
                    look_at: Some(look_at),
                    ..
                } = &mut timed.operation
                {
                    Some(look_at)
                } else {
                    None
                }
            }) {
                target(look_at);
            }
        }
    }
    Cow::Owned(project)
}

pub fn solve_project(
    project: &CineProject,
    options: &SolveOptions,
) -> Result<SolveOutput, SolveError> {
    if options.fidelity != Fidelity::Draft {
        return Err(SolveError::FidelityUnsupported(options.fidelity));
    }
    let mut report = validate_project(project);
    if report.has_errors() {
        return Err(SolveError::Invalid(report));
    }
    let project = normalize_units(project);
    let project = &*project;
    let fps = project
        .frame_rate
        .frames_per_second()
        .filter(|fps| *fps > 0.0)
        .ok_or(SolveError::InvalidFrameRate)?;

    let cs = &project.coordinate_system;
    let mut scenes = Vec::with_capacity(project.scenes.len());
    for scene in &project.scenes {
        let solved_scene = sampler::sample_scene(scene, cs, fps);
        geometry::check_scene(scene, &solved_scene, cs, fps, &mut report);
        scenes.push(solved_scene);
    }

    Ok(SolveOutput {
        solved: SolvedProject {
            schema_version: SOLVED_SCHEMA_VERSION.to_owned(),
            source_id: project.id.clone(),
            source_title: project.title.clone(),
            source_schema_version: project.schema_version.clone(),
            fidelity: options.fidelity,
            frame_rate: project.frame_rate,
            coordinate_system: cs.clone(),
            scenes,
        },
        report,
    })
}
