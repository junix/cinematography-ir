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
            if let Some(crate::model::GeometryProxy::ConvexHull { points }) =
                &mut subject.geometry_proxy
            {
                for point in points {
                    pos(point);
                }
            }
            if let Some(pose) = &mut subject.pose_track {
                for frame in &mut pose.frames {
                    for joint in frame.joints.values_mut() {
                        pos(joint);
                    }
                    if let Some(gaze) = &mut frame.gaze {
                        target(gaze);
                    }
                }
            }
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
            normalize_camera_plan(&mut shot.camera, scale);
        }
        for take in &mut scene.takes {
            normalize_camera_plan(&mut take.camera_program, scale);
        }
    }
    Cow::Owned(project)
}

fn normalize_camera_plan(plan: &mut crate::model::CameraPlan, scale: f32) {
    let pos = |value: &mut Vec3| *value = *value * scale;
    let target = |value: &mut TargetRef| {
        if let TargetRef::Point { position } = value {
            *position = *position * scale;
        }
    };
    pos(&mut plan.initial_state.transform.position);
    if let Some(focus) = &mut plan.initial_state.focus.target {
        target(focus);
    }
    for timed in &mut plan.operations {
        match &mut timed.operation {
            CameraOperation::Translate { delta, .. } => pos(delta),
            CameraOperation::Crane { delta, look_at } => {
                pos(delta);
                if let Some(look_at) = look_at {
                    target(look_at);
                }
            }
            CameraOperation::Follow {
                target: value,
                offset,
                ..
            } => {
                target(value);
                pos(offset);
            }
            CameraOperation::LookAt { target: value }
            | CameraOperation::Orbit { target: value, .. }
            | CameraOperation::RackFocus { target: value, .. }
            | CameraOperation::DollyZoom { target: value, .. } => target(value),
            CameraOperation::Reveal {
                target: value,
                occluder,
                ..
            } => {
                target(value);
                target(occluder);
            }
            _ => {}
        }
    }
}

pub fn solve_project(
    project: &CineProject,
    options: &SolveOptions,
) -> Result<SolveOutput, SolveError> {
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
        let mut solved_scene = sampler::sample_scene(scene, cs, fps);
        if options.fidelity == Fidelity::Full {
            apply_full_fidelity(scene, &mut solved_scene, cs, fps, &mut report);
        }
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

fn apply_full_fidelity(
    scene: &crate::model::Scene,
    solved: &mut SolvedScene,
    cs: &crate::model::CoordinateSystem,
    fps: f64,
    report: &mut ValidationReport,
) {
    let dt = (1.0 / fps.max(1.0)) as f32;
    let collidable: std::collections::BTreeSet<_> = scene
        .subjects
        .iter()
        .filter(|subject| subject.geometry_proxy.is_some())
        .map(|subject| subject.id.as_str())
        .collect();
    for shot in &mut solved.shots {
        let Some(source) = scene.shots.iter().find(|source| source.id == shot.id) else {
            continue;
        };
        let (maximum_speed, maximum_acceleration) = rig_limits(source.camera.rig);
        let mut previous_velocity = Vec3::ZERO;
        let mut collision_frames = 0usize;
        for index in 0..shot.frames.len() {
            let draft_aim = shot.frames[index].position
                + shot.frames[index].forward * shot.frames[index].focus_distance_m.unwrap_or(10.0);
            if index > 0 {
                let previous = shot.frames[index - 1].position;
                let desired = (shot.frames[index].position - previous) * (1.0 / dt);
                let speed_limited = desired.normalized().map_or(Vec3::ZERO, |direction| {
                    direction * desired.length().min(maximum_speed)
                });
                let acceleration = (speed_limited - previous_velocity) * (1.0 / dt);
                let velocity = if acceleration.length() > maximum_acceleration {
                    previous_velocity
                        + acceleration.normalized().unwrap_or(Vec3::ZERO)
                            * (maximum_acceleration * dt)
                } else {
                    speed_limited
                };
                shot.frames[index].position = previous + velocity * dt;
                previous_velocity = velocity;
            }
            let frame_number = shot.frames[index].frame;
            let mut correction = Vec3::ZERO;
            for subject in &solved.subjects {
                if !collidable.contains(subject.subject_id.as_str()) {
                    continue;
                }
                let transform = subject.transform_at(frame_number);
                let half_height = subject.dimensions_m.y * transform.scale.y * 0.5;
                let radius = Vec3::new(
                    subject.dimensions_m.x * transform.scale.x * 0.5,
                    half_height,
                    subject.dimensions_m.z * transform.scale.z * 0.5,
                )
                .length()
                    + 0.1;
                let center = transform.position + crate::math::identity_basis(cs).up * half_height;
                let delta = shot.frames[index].position - center;
                if delta.length() < radius {
                    let direction = delta.normalized().unwrap_or(shot.frames[index].right);
                    correction = correction + direction * (radius - delta.length());
                }
            }
            if correction.length() > 0.0 {
                shot.frames[index].position = shot.frames[index].position + correction;
                collision_frames += 1;
            }
            if let Some((yaw, pitch)) = crate::math::look_direction_to_yaw_pitch(
                cs,
                draft_aim - shot.frames[index].position,
            ) {
                shot.frames[index].rotation_deg.yaw = yaw;
                shot.frames[index].rotation_deg.pitch = pitch;
                let basis = crate::math::oriented_basis(cs, shot.frames[index].rotation_deg);
                shot.frames[index].right = basis.right;
                shot.frames[index].up = basis.up;
                shot.frames[index].forward = basis.forward;
            }
        }
        if collision_frames > 0 {
            report.push(crate::diagnostic::Diagnostic::warning(
                "CAMERA_COLLISION_RESOLVED",
                format!("scenes[{}].shots[{}]", scene.id, shot.id),
                format!(
                    "full-fidelity solve displaced the camera outside authored geometry proxies on {collision_frames} frame(s)"
                ),
            ));
        }
    }
}

fn rig_limits(rig: crate::model::CameraRig) -> (f32, f32) {
    use crate::model::CameraRig;
    match rig {
        CameraRig::Tripod | CameraRig::Locked => (0.0, 2.0),
        CameraRig::Dolly | CameraRig::Slider => (8.0, 16.0),
        CameraRig::Gimbal | CameraRig::Steadicam => (5.0, 12.0),
        CameraRig::Handheld | CameraRig::Shoulder => (4.0, 24.0),
        CameraRig::Crane | CameraRig::Jib => (6.0, 10.0),
        CameraRig::Drone => (20.0, 30.0),
        CameraRig::VehicleMount => (30.0, 40.0),
        CameraRig::Virtual => (100.0, 200.0),
    }
}
