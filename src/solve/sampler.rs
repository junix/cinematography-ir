//! Draft-fidelity kinematic sampler: semantic camera operations and blocking
//! tracks → per-frame states.
//!
//! Composition rules (documented contract for `Fidelity::Draft`):
//!
//! * Operations are evaluated in document order at every shot-local frame
//!   `i`. An operation's progress is `u = ease((i - start) / (end - 1 - start))`,
//!   clamped to `[0, 1]`, so motion occupies `[start, end)` and the final
//!   value is reached on the last frame inside the range.
//! * Relative operations (`pan`, `tilt`, `roll`, `dolly`, `truck`,
//!   `pedestal`, `translate`, `rotate`, `crane` motion, `reveal` motion) apply
//!   the increment `Δu` of their progress each frame along the camera's
//!   *current* axes, so overlapping operations compose.
//! * Target operations (`look_at`, `orbit`, `zoom`, `rack_focus`,
//!   `dolly_zoom`, `crane.look_at`, `reveal` aiming) snapshot the camera when
//!   they start and interpolate from that snapshot toward the target by `u`
//!   while active. The last frame inside the range reaches the target; after
//!   the range ends the channel is released, so later operations accumulate
//!   on top of the held end state. When two of them drive the same channel
//!   simultaneously, the later one in document order wins.
//! * `follow` re-targets the position with a time-domain half-life, so the
//!   response is independent of frame rate. Legacy `damping` values are
//!   interpreted as the equivalent response at 24 fps.
//! * `handheld_noise` is a deterministic overlay added to the output frame
//!   without feeding back into the base state, so it never drifts.
//! * `rotate` treats both spaces as Euler-additive; `dolly_zoom` solves the
//!   requested projected subject height when the target is a subject;
//!   `reveal` currently uses its compiled visibility constraints for
//!   validation while draft motion remains lateral.
//!   These are draft simplifications, not IR semantics.

use crate::math::{
    from_spherical, horizontal_fov_deg, lerp, lerp_angle_deg, look_direction_to_yaw_pitch,
    oriented_basis, rotation_matrix, splitmix64, to_spherical, unit_float, wrap_deg, Basis, Vec3,
};
use crate::model::{
    BlockingKeyframe, CameraOperation, CoordinateSystem, EulerDeg, Frame, FrameRange, Scene, Shot,
    SubjectKind, TargetRef, TimedCameraOperation, Transform, TransformSpace,
};
use crate::palette::scene_palette;
use crate::solve::easing::ease;
use crate::solve::solved::{
    SolvedCameraFrame, SolvedCue, SolvedScene, SolvedShot, SolvedSubjectTrack,
};

const DEFAULT_FRAME_ASPECT: f32 = 16.0 / 9.0;
const LEGACY_FOLLOW_FPS: f32 = 24.0;

/// Default bounding size per subject kind when `dimensions_m` is absent
/// (width, height, depth in metres).
pub fn default_dimensions(kind: SubjectKind) -> Vec3 {
    match kind {
        SubjectKind::Character => Vec3::new(0.50, 1.75, 0.35),
        SubjectKind::Prop => Vec3::new(0.30, 0.30, 0.30),
        SubjectKind::Vehicle => Vec3::new(1.80, 1.50, 4.50),
        SubjectKind::Screen => Vec3::new(1.20, 0.70, 0.05),
        SubjectKind::Environment => Vec3::new(8.00, 3.00, 8.00),
        SubjectKind::Abstract => Vec3::new(0.50, 0.50, 0.50),
    }
}

/// Height fraction of a subject at which cameras aim and focus.
pub fn aim_height_fraction(kind: SubjectKind) -> f32 {
    match kind {
        SubjectKind::Character => 0.9,
        _ => 0.5,
    }
}

/// Everything target resolution needs for one scene.
pub struct SceneContext<'a> {
    pub scene: &'a Scene,
    pub cs: &'a CoordinateSystem,
    pub fps: f64,
    pub subjects: Vec<SolvedSubjectTrack>,
}

impl SceneContext<'_> {
    /// World-space aim point of `target` at scene frame `frame`.
    pub fn resolve(&self, target: &TargetRef, frame: Frame) -> Option<Vec3> {
        match target {
            TargetRef::Subject { id } => {
                let track = self.subjects.iter().find(|t| &t.subject_id == id)?;
                Some(self.aim_point(track, frame))
            }
            TargetRef::Marker { id } => self
                .scene
                .markers
                .iter()
                .find(|m| &m.id == id)
                .map(|m| m.position),
            TargetRef::Point { position } => Some(*position),
        }
    }

    pub fn aim_point(&self, track: &SolvedSubjectTrack, frame: Frame) -> Vec3 {
        let transform = track.transform_at(frame);
        let up = crate::math::identity_basis(self.cs).up;
        transform.position
            + up * (track.dimensions_m.y * aim_height_fraction(track.kind) * transform.scale.y)
    }
}

pub fn sample_subjects(scene: &Scene, _cs: &CoordinateSystem) -> Vec<SolvedSubjectTrack> {
    let palette = scene_palette(scene);
    scene
        .subjects
        .iter()
        .map(|subject| {
            let mut keyframes: Vec<BlockingKeyframe> = scene
                .blocking
                .iter()
                .filter(|track| track.subject_id == subject.id)
                .flat_map(|track| track.keyframes.iter().cloned())
                .collect();
            keyframes.sort_by_key(|k| k.frame);
            let cues = keyframes
                .iter()
                .map(|k| SolvedCue {
                    frame: k.frame,
                    gaze_target: k.gaze_target.clone(),
                    action: k.action.clone(),
                })
                .collect();
            // `initial_transform` is where the subject is at frame 0; a first
            // keyframe later than that is reached by interpolation, not a jump.
            if keyframes.first().is_some_and(|k| k.frame > 0) {
                keyframes.insert(
                    0,
                    BlockingKeyframe {
                        frame: 0,
                        transform: subject.initial_transform,
                        gaze_target: None,
                        action: None,
                    },
                );
            }

            let transforms = (0..scene.duration_frames)
                .map(|frame| interpolate_transform(&keyframes, subject.initial_transform, frame))
                .collect();
            let color = palette
                .get(&subject.id)
                .copied()
                .unwrap_or_else(|| crate::palette::color_for_index(0));

            SolvedSubjectTrack {
                subject_id: subject.id.clone(),
                name: subject.name.clone(),
                kind: subject.kind,
                dimensions_m: subject
                    .dimensions_m
                    .unwrap_or_else(|| default_dimensions(subject.kind)),
                color: color.rgb,
                color_name: color.name.to_owned(),
                transforms,
                cues,
            }
        })
        .collect()
}

/// Piecewise-linear blocking. A keyframe may sit on the scene's end boundary
/// (`frame == duration_frames`): sampled frames approach it but, like every
/// half-open range, never land on it.
fn interpolate_transform(
    keyframes: &[BlockingKeyframe],
    initial: Transform,
    frame: Frame,
) -> Transform {
    let Some(first) = keyframes.first() else {
        return initial;
    };
    if frame <= first.frame {
        return first.transform;
    }
    let last = &keyframes[keyframes.len() - 1];
    if frame >= last.frame {
        return last.transform;
    }
    let next_index = keyframes
        .iter()
        .position(|k| k.frame > frame)
        .unwrap_or(keyframes.len() - 1);
    let a = &keyframes[next_index - 1];
    let b = &keyframes[next_index];
    let span = (b.frame - a.frame) as f32;
    let t = if span > 0.0 {
        (frame - a.frame) as f32 / span
    } else {
        1.0
    };
    Transform {
        position: a.transform.position.lerp(b.transform.position, t),
        rotation_deg: EulerDeg {
            pitch: lerp_angle_deg(
                a.transform.rotation_deg.pitch,
                b.transform.rotation_deg.pitch,
                t,
            ),
            yaw: lerp_angle_deg(
                a.transform.rotation_deg.yaw,
                b.transform.rotation_deg.yaw,
                t,
            ),
            roll: lerp_angle_deg(
                a.transform.rotation_deg.roll,
                b.transform.rotation_deg.roll,
                t,
            ),
        },
        scale: a.transform.scale.lerp(b.transform.scale, t),
    }
}

#[derive(Debug, Clone)]
struct CamState {
    position: Vec3,
    rotation: EulerDeg,
    focal_length_mm: f32,
    focus_distance_m: Option<f32>,
    focus_target: Option<TargetRef>,
    sensor_width_mm: f32,
}

/// Values captured when a target operation starts.
#[derive(Debug, Clone, Default)]
struct Snapshot {
    rotation: EulerDeg,
    focal_length_mm: f32,
    focus_distance_m: Option<f32>,
    /// Orbit: (radius, azimuth, elevation) of camera relative to target.
    spherical: Option<(f32, f32, f32)>,
    /// Dolly zoom: unit direction target → camera and starting distance.
    dolly_zoom: Option<(Vec3, f32)>,
    /// Orbit / dolly zoom: (yaw, pitch) by which the camera initially missed
    /// the centre; faded out over the operation so tracking never snaps.
    aim_offset: (f32, f32),
}

struct OpRuntime {
    previous_u: f32,
    snapshot: Option<Snapshot>,
}

/// Progress of `timed` at shot-local frame `i`.
fn progress(timed: &TimedCameraOperation, i: Frame) -> f32 {
    if i < timed.range.start {
        0.0
    } else if i >= timed.range.end {
        1.0
    } else {
        // The last frame inside the half-open range completes the motion, so
        // an operation spanning the whole shot reaches its declared value.
        let span = timed.range.end - 1 - timed.range.start;
        if span == 0 {
            1.0
        } else {
            ease(timed.easing, (i - timed.range.start) as f32 / span as f32)
        }
    }
}

fn is_active(timed: &TimedCameraOperation, i: Frame) -> bool {
    i >= timed.range.start && i < timed.range.end
}

pub fn sample_shot(ctx: &SceneContext<'_>, shot: &Shot) -> SolvedShot {
    let cs = ctx.cs;
    let initial = &shot.camera.initial_state;
    let mut state = CamState {
        position: initial.transform.position,
        rotation: initial.transform.rotation_deg,
        focal_length_mm: initial.lens.focal_length_mm,
        focus_distance_m: initial.focus.distance_m,
        focus_target: initial.focus.target.clone(),
        sensor_width_mm: initial.lens.sensor_width_mm,
    };
    let sensor_width_mm = initial.lens.sensor_width_mm;
    let aperture_f = initial.lens.aperture_f;
    let mut runtimes: Vec<OpRuntime> = shot
        .camera
        .operations
        .iter()
        .map(|_| OpRuntime {
            previous_u: 0.0,
            snapshot: None,
        })
        .collect();

    let mut frames = Vec::with_capacity(shot.duration_frames() as usize);
    for i in 0..shot.duration_frames() {
        let scene_frame = shot.range.start + i;
        let mut noise_translation = Vec3::ZERO;
        let mut noise_rotation = EulerDeg::default();

        for (timed, runtime) in shot.camera.operations.iter().zip(runtimes.iter_mut()) {
            let u = progress(timed, i);
            if runtime.snapshot.is_none() && i >= timed.range.start {
                runtime.snapshot = Some(snapshot_for(&timed.operation, &state, ctx, scene_frame));
            }
            let du = u - runtime.previous_u;
            runtime.previous_u = u;
            let snapshot = runtime.snapshot.clone().unwrap_or_default();
            apply_operation(
                &timed.operation,
                timed.range,
                &mut state,
                ctx,
                scene_frame,
                i,
                u,
                du,
                is_active(timed, i),
                &snapshot,
                &mut noise_translation,
                &mut noise_rotation,
            );
        }

        let rotation = EulerDeg {
            pitch: state.rotation.pitch + noise_rotation.pitch,
            yaw: state.rotation.yaw + noise_rotation.yaw,
            roll: state.rotation.roll + noise_rotation.roll,
        };
        let basis = oriented_basis(cs, rotation);
        let position = state.position + local_to_world(&basis, noise_translation);
        let focus_distance_m = state.focus_distance_m.or_else(|| {
            state
                .focus_target
                .as_ref()
                .and_then(|t| ctx.resolve(t, scene_frame))
                .map(|p| (p - position).length())
        });

        frames.push(SolvedCameraFrame {
            frame: scene_frame,
            position,
            rotation_deg: rotation,
            forward: basis.forward,
            up: basis.up,
            right: basis.right,
            focal_length_mm: state.focal_length_mm,
            aperture_f,
            horizontal_fov_deg: horizontal_fov_deg(state.focal_length_mm, sensor_width_mm),
            focus_distance_m,
            focus_target: state.focus_target.clone(),
        });
    }

    SolvedShot {
        id: shot.id.clone(),
        range: shot.range,
        coverage_role: shot.coverage_role,
        shot_size: shot.framing.shot_size,
        purpose: shot.purpose.clone(),
        subject_ids: shot.framing.subject_ids.clone(),
        sensor_width_mm,
        frames,
    }
}

fn local_to_world(basis: &Basis, local: Vec3) -> Vec3 {
    basis.right * local.x + basis.up * local.y + basis.forward * local.z
}

fn snapshot_for(
    operation: &CameraOperation,
    state: &CamState,
    ctx: &SceneContext<'_>,
    scene_frame: Frame,
) -> Snapshot {
    let mut snapshot = Snapshot {
        rotation: state.rotation,
        focal_length_mm: state.focal_length_mm,
        focus_distance_m: state.focus_distance_m,
        spherical: None,
        dolly_zoom: None,
        aim_offset: (0.0, 0.0),
    };
    let aim_offset = |center: Vec3| -> (f32, f32) {
        match look_direction_to_yaw_pitch(ctx.cs, center - state.position) {
            Some((yaw, pitch)) => (
                wrap_deg(state.rotation.yaw - yaw),
                state.rotation.pitch - pitch,
            ),
            None => (0.0, 0.0),
        }
    };
    match operation {
        CameraOperation::Orbit { target, .. } => {
            if let Some(center) = ctx.resolve(target, scene_frame) {
                snapshot.spherical = to_spherical(ctx.cs, state.position - center);
                snapshot.aim_offset = aim_offset(center);
            }
        }
        CameraOperation::DollyZoom { target, .. } => {
            if let Some(center) = ctx.resolve(target, scene_frame) {
                let offset = state.position - center;
                if let Some(direction) = offset.normalized() {
                    snapshot.dolly_zoom = Some((direction, offset.length()));
                }
                snapshot.aim_offset = aim_offset(center);
            }
        }
        CameraOperation::RackFocus { target, .. } if snapshot.focus_distance_m.is_none() => {
            let reference = state.focus_target.clone().unwrap_or_else(|| target.clone());
            snapshot.focus_distance_m = ctx
                .resolve(&reference, scene_frame)
                .map(|p| (p - state.position).length());
        }
        _ => {}
    }
    snapshot
}

#[allow(clippy::too_many_arguments)]
fn apply_operation(
    operation: &CameraOperation,
    range: FrameRange,
    state: &mut CamState,
    ctx: &SceneContext<'_>,
    scene_frame: Frame,
    shot_frame: Frame,
    u: f32,
    du: f32,
    active: bool,
    snapshot: &Snapshot,
    noise_translation: &mut Vec3,
    noise_rotation: &mut EulerDeg,
) {
    let cs = ctx.cs;
    match operation {
        CameraOperation::Hold => {}
        CameraOperation::Pan { degrees } => state.rotation.yaw += degrees * du,
        CameraOperation::Tilt { degrees } => state.rotation.pitch += degrees * du,
        CameraOperation::Roll { degrees } => state.rotation.roll += degrees * du,
        CameraOperation::Dolly { distance_m } => {
            let basis = oriented_basis(cs, state.rotation);
            state.position = state.position + basis.forward * (distance_m * du);
        }
        CameraOperation::Truck { distance_m } => {
            let basis = oriented_basis(cs, state.rotation);
            state.position = state.position + basis.right * (distance_m * du);
        }
        CameraOperation::Pedestal { distance_m } => {
            let basis = oriented_basis(cs, state.rotation);
            state.position = state.position + basis.up * (distance_m * du);
        }
        CameraOperation::Translate { delta, space } => {
            let world = match space {
                TransformSpace::World => *delta,
                TransformSpace::CameraLocal => rotation_matrix(cs, state.rotation).apply(*delta),
            };
            state.position = state.position + world * du;
        }
        CameraOperation::Rotate { delta_deg, .. } => {
            state.rotation.pitch += delta_deg.pitch * du;
            state.rotation.yaw += delta_deg.yaw * du;
            state.rotation.roll += delta_deg.roll * du;
        }
        CameraOperation::LookAt { target } => {
            if active {
                aim_blend(state, cs, ctx.resolve(target, scene_frame), snapshot, u);
            }
        }
        CameraOperation::Orbit {
            target,
            azimuth_deg,
            elevation_deg,
            radius_m,
        } => {
            if !active {
                return;
            }
            let (Some(center), Some((r0, az0, el0))) =
                (ctx.resolve(target, scene_frame), snapshot.spherical)
            else {
                return;
            };
            let radius = radius_m.map_or(r0, |r| lerp(r0, r, u));
            let azimuth = az0 + azimuth_deg * u;
            let elevation = el0 + elevation_deg * u;
            state.position = center + from_spherical(cs, radius, azimuth, elevation);
            aim_track(state, cs, center, snapshot, u);
        }
        CameraOperation::Follow {
            target,
            offset,
            lag_half_life_s,
            damping,
        } => {
            if !active {
                return;
            }
            let Some(center) = ctx.resolve(target, scene_frame) else {
                return;
            };
            let desired = center + *offset;
            let half_life = lag_half_life_s.unwrap_or_else(|| {
                let retained = damping.clamp(f32::EPSILON, 1.0 - f32::EPSILON);
                -std::f32::consts::LN_2 / (LEGACY_FOLLOW_FPS * retained.ln())
            });
            let dt = 1.0 / ctx.fps.max(1.0) as f32;
            let gain = if *damping <= 0.0 && lag_half_life_s.is_none() {
                1.0
            } else if *damping >= 1.0 && lag_half_life_s.is_none() {
                0.0
            } else {
                1.0 - 2.0_f32.powf(-dt / half_life.max(f32::EPSILON))
            };
            state.position = state.position.lerp(desired, gain);
            aim_blend(state, cs, Some(center), snapshot, 1.0);
        }
        CameraOperation::Crane { delta, look_at } => {
            state.position = state.position + *delta * du;
            if let (Some(target), true) = (look_at, active) {
                aim_blend(state, cs, ctx.resolve(target, scene_frame), snapshot, u);
            }
        }
        CameraOperation::Zoom { to_focal_length_mm } => {
            if active {
                state.focal_length_mm = lerp(snapshot.focal_length_mm, *to_focal_length_mm, u);
            }
        }
        CameraOperation::RackFocus {
            target,
            to_distance_m,
        } => {
            if !active {
                return;
            }
            let target_distance = to_distance_m.or_else(|| {
                ctx.resolve(target, scene_frame)
                    .map(|p| (p - state.position).length())
            });
            if let (Some(start), Some(end)) = (snapshot.focus_distance_m, target_distance) {
                state.focus_distance_m = Some(lerp(start, end, u));
            } else if let Some(end) = target_distance {
                state.focus_distance_m = Some(end);
            }
            state.focus_target = Some(target.clone());
        }
        CameraOperation::DollyZoom {
            target,
            to_distance_m,
            keep_subject_frame_fraction,
        } => {
            if !active {
                return;
            }
            let (Some(center), Some((direction, d0))) =
                (ctx.resolve(target, scene_frame), snapshot.dolly_zoom)
            else {
                return;
            };
            let distance = lerp(d0, *to_distance_m, u);
            state.position = center + direction * distance;
            aim_track(state, cs, center, snapshot, u);
            state.focal_length_mm = focal_for_subject_fraction(
                ctx,
                state,
                target,
                scene_frame,
                *keep_subject_frame_fraction,
            )
            .unwrap_or(snapshot.focal_length_mm * distance / d0);
            state.focus_distance_m = Some(distance);
            state.focus_target = Some(target.clone());
        }
        CameraOperation::HandheldNoise {
            translation_amplitude_m,
            rotation_amplitude_deg,
            frequency_hz,
            seed,
        } => {
            if !active {
                return;
            }
            let t = shot_frame as f32 / ctx.fps as f32;
            let (translation, rotation) = handheld_noise(
                *seed,
                t,
                *frequency_hz,
                *translation_amplitude_m,
                *rotation_amplitude_deg,
            );
            let ramp = (0.25 * ctx.fps as f32).round().max(1.0);
            let envelope = ((shot_frame + 1 - range.start) as f32 / ramp)
                .min((range.end - shot_frame) as f32 / ramp)
                .clamp(0.0, 1.0);
            *noise_translation = *noise_translation + translation * envelope;
            noise_rotation.pitch += rotation.pitch * envelope;
            noise_rotation.yaw += rotation.yaw * envelope;
            noise_rotation.roll += rotation.roll * envelope;
        }
        CameraOperation::Reveal {
            target,
            lateral_distance_m,
            ..
        } => {
            let basis = oriented_basis(cs, state.rotation);
            state.position = state.position + basis.right * (lateral_distance_m * du);
            if active {
                aim_blend(state, cs, ctx.resolve(target, scene_frame), snapshot, u);
            }
        }
    }
}

fn focal_for_subject_fraction(
    ctx: &SceneContext<'_>,
    state: &CamState,
    target: &TargetRef,
    frame: Frame,
    requested_fraction: f32,
) -> Option<f32> {
    let TargetRef::Subject { id } = target else {
        return None;
    };
    let track = ctx.subjects.iter().find(|track| &track.subject_id == id)?;
    let transform = track.transform_at(frame);
    let subject = oriented_basis(ctx.cs, transform.rotation_deg);
    let camera = oriented_basis(ctx.cs, state.rotation);
    let half_w = track.dimensions_m.x * transform.scale.x / 2.0;
    let half_d = track.dimensions_m.z * transform.scale.z / 2.0;
    let height = track.dimensions_m.y * transform.scale.y;
    let mut min_slope = f32::INFINITY;
    let mut max_slope = f32::NEG_INFINITY;
    for sx in [-1.0, 1.0] {
        for sz in [-1.0, 1.0] {
            for sy in [0.0, 1.0] {
                let corner = transform.position
                    + subject.right * (sx * half_w)
                    + subject.forward * (sz * half_d)
                    + subject.up * (sy * height);
                let delta = corner - state.position;
                let depth = delta.dot(camera.forward);
                if depth <= 0.05 {
                    continue;
                }
                let slope = delta.dot(camera.up) / depth;
                min_slope = min_slope.min(slope);
                max_slope = max_slope.max(slope);
            }
        }
    }
    let span = max_slope - min_slope;
    if !span.is_finite() || span <= 1e-6 {
        return None;
    }
    let sensor_height_mm = state.sensor_width_mm / DEFAULT_FRAME_ASPECT;
    Some((requested_fraction * sensor_height_mm / span).max(1.0))
}

/// Look at `center` every frame, keeping the snapshot's initial misalignment
/// scaled by `1 − u` so a camera that starts off-centre eases onto it.
fn aim_track(
    state: &mut CamState,
    cs: &CoordinateSystem,
    center: Vec3,
    snapshot: &Snapshot,
    u: f32,
) {
    let Some((yaw, pitch)) = look_direction_to_yaw_pitch(cs, center - state.position) else {
        return;
    };
    let fade = 1.0 - u.clamp(0.0, 1.0);
    state.rotation.yaw = yaw + snapshot.aim_offset.0 * fade;
    state.rotation.pitch = pitch + snapshot.aim_offset.1 * fade;
}

/// Blend the camera's yaw/pitch from the snapshot toward looking at `target`.
fn aim_blend(
    state: &mut CamState,
    cs: &CoordinateSystem,
    target: Option<Vec3>,
    snapshot: &Snapshot,
    u: f32,
) {
    let Some(target) = target else {
        return;
    };
    let Some((yaw, pitch)) = look_direction_to_yaw_pitch(cs, target - state.position) else {
        return;
    };
    state.rotation.yaw = lerp_angle_deg(snapshot.rotation.yaw, yaw, u);
    state.rotation.pitch = lerp_angle_deg(snapshot.rotation.pitch, pitch, u);
}

/// Deterministic multi-harmonic handheld noise. Returns camera-local
/// translation (metres) and rotation (degrees) at time `t` seconds.
pub fn handheld_noise(
    seed: u64,
    t: f32,
    frequency_hz: f32,
    translation_amplitude_m: f32,
    rotation_amplitude_deg: f32,
) -> (Vec3, EulerDeg) {
    const HARMONICS: [(f32, f32); 3] = [(1.0, 0.6), (1.7, 0.3), (2.3, 0.1)];
    let mut stream = seed ^ 0x5eed_c1de_0000_0000;
    let mut channel = |amplitude: f32| -> f32 {
        let mut value = 0.0;
        for (multiplier, weight) in HARMONICS {
            let phase = unit_float(splitmix64(&mut stream)) * std::f32::consts::TAU;
            value += weight * (std::f32::consts::TAU * frequency_hz * multiplier * t + phase).sin();
        }
        value * amplitude
    };
    let translation = Vec3::new(
        channel(translation_amplitude_m),
        channel(translation_amplitude_m),
        channel(translation_amplitude_m * 0.5),
    );
    let rotation = EulerDeg {
        pitch: channel(rotation_amplitude_deg),
        yaw: channel(rotation_amplitude_deg),
        roll: channel(rotation_amplitude_deg * 0.5),
    };
    (translation, rotation)
}

pub fn sample_scene(scene: &Scene, cs: &CoordinateSystem, fps: f64) -> SolvedScene {
    let subjects = sample_subjects(scene, cs);
    let ctx = SceneContext {
        scene,
        cs,
        fps,
        subjects,
    };
    let mut shots: Vec<&Shot> = scene.shots.iter().collect();
    shots.sort_by_key(|shot| (shot.range.start, shot.range.end));
    let shots = shots
        .into_iter()
        .map(|shot| sample_shot(&ctx, shot))
        .collect();
    SolvedScene {
        id: scene.id.clone(),
        title: scene.title.clone(),
        duration_frames: scene.duration_frames,
        subjects: ctx.subjects,
        shots,
    }
}
