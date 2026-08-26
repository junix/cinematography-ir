//! IR → Blender coordinate conversion and the data payload embedded in the
//! generated `bpy` script. All conversion happens here, in Rust, so the
//! Python side stays convention-free (ADR-1115: adapters own coordinates).
//!
//! Blender world: X right, Y forward (into the scene), Z up, right-handed,
//! metres. Blender cameras look along local −Z with local +Y up. Subject
//! proxies are modelled facing local +Y with local +Z up and local +X to the
//! subject's right.

use serde::{Deserialize, Serialize};

use crate::execute::ConditioningPass;
use crate::math::{identity_basis, oriented_basis, Basis, Vec3};
use crate::model::{CoordinateSystem, FrameRange, SubjectKind, Transform, Unit};
use crate::solve::{SolvedProject, SolvedScene};

pub const PAYLOAD_SCHEMA_VERSION: &str = "blender-payload-0.1";

/// Converts a world vector from the document's system into Blender's.
pub fn to_blender(cs: &CoordinateSystem, v: Vec3) -> Vec3 {
    let id = identity_basis(cs);
    let scale = match cs.units {
        Unit::Meters => 1.0,
        Unit::Centimeters => 0.01,
    };
    Vec3::new(v.dot(id.right), v.dot(id.forward), v.dot(id.up)) * scale
}

/// Direction-only conversion (no unit scaling).
pub fn direction_to_blender(cs: &CoordinateSystem, v: Vec3) -> Vec3 {
    let id = identity_basis(cs);
    Vec3::new(v.dot(id.right), v.dot(id.forward), v.dot(id.up))
}

/// Row-major 3×3 whose columns are the local X, Y, Z axes in Blender world.
fn rows(x: Vec3, y: Vec3, z: Vec3) -> [f32; 9] {
    [x.x, y.x, z.x, x.y, y.y, z.y, x.z, y.z, z.z]
}

/// Camera basis rows for `mathutils.Matrix`: local X = right, Y = up, Z = −forward.
pub fn camera_rows(cs: &CoordinateSystem, right: Vec3, up: Vec3, forward: Vec3) -> [f32; 9] {
    rows(
        direction_to_blender(cs, right),
        direction_to_blender(cs, up),
        direction_to_blender(cs, -forward),
    )
}

/// Subject basis rows: local X = right, Y = forward, Z = up.
pub fn subject_rows(cs: &CoordinateSystem, basis: &Basis) -> [f32; 9] {
    rows(
        direction_to_blender(cs, basis.right),
        direction_to_blender(cs, basis.forward),
        direction_to_blender(cs, basis.up),
    )
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RenderSettings {
    pub resolution: [u32; 2],
    pub samples: u32,
    pub passes: Vec<ConditioningPass>,
    /// Inclusive Blender frame range to render.
    pub frame_range: [u64; 2],
    pub depth_near_m: f32,
    pub depth_far_m: f32,
    pub use_dof: bool,
    pub output_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PayloadSubject {
    pub id: String,
    pub name: String,
    pub kind: SubjectKind,
    /// Cycles object index; 0 is reserved for the floor/background.
    pub pass_index: u32,
    /// sRGB colour in `[0, 1]`.
    pub color: [f32; 3],
    pub color_name: String,
    /// Blender-local size: width (X), depth (Y), height (Z).
    pub dimensions: [f32; 3],
    /// Per Blender frame: location(3) + basis rows(9) + scale(3).
    pub frames: Vec<[f32; 15]>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PayloadShot {
    pub id: String,
    /// Inclusive Blender frame range.
    pub frame_start: u64,
    pub frame_end: u64,
    pub sensor_width_mm: f32,
    pub aperture_f: f32,
    /// Per frame: location(3) + basis rows(9) + focal_length_mm + focus_distance_m.
    pub frames: Vec<[f32; 14]>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BlenderPayload {
    pub schema_version: String,
    pub source_id: String,
    pub scene_id: String,
    pub scene_title: String,
    pub fps_numerator: u32,
    pub fps_denominator: u32,
    /// Inclusive scene frame bounds.
    pub frame_start: u64,
    pub frame_end: u64,
    pub render: RenderSettings,
    pub subjects: Vec<PayloadSubject>,
    pub shots: Vec<PayloadShot>,
}

fn unit_scale(cs: &CoordinateSystem) -> f32 {
    match cs.units {
        Unit::Meters => 1.0,
        Unit::Centimeters => 0.01,
    }
}

/// Builds the Blender payload for one solved scene. Render settings are
/// filled with placeholders and finalised by the adapter.
pub fn build_payload(project: &SolvedProject, scene: &SolvedScene) -> BlenderPayload {
    let cs = &project.coordinate_system;

    let subjects = scene
        .subjects
        .iter()
        .enumerate()
        .map(|(index, track)| {
            let frames = track
                .transforms
                .iter()
                .map(|transform: &Transform| {
                    let location = to_blender(cs, transform.position);
                    let basis = oriented_basis(cs, transform.rotation_deg);
                    let r = subject_rows(cs, &basis);
                    [
                        location.x,
                        location.y,
                        location.z,
                        r[0],
                        r[1],
                        r[2],
                        r[3],
                        r[4],
                        r[5],
                        r[6],
                        r[7],
                        r[8],
                        transform.scale.x,
                        transform.scale.z,
                        transform.scale.y,
                    ]
                })
                .collect();
            let (r, g, b) = track.color.unit();
            PayloadSubject {
                id: track.subject_id.clone(),
                name: track.name.clone(),
                kind: track.kind,
                pass_index: index as u32 + 1,
                color: [r, g, b],
                color_name: track.color_name.clone(),
                // `dimensions_m` is metres by contract, never document units.
                dimensions: [
                    track.dimensions_m.x,
                    track.dimensions_m.z,
                    track.dimensions_m.y,
                ],
                frames,
            }
        })
        .collect();

    let shots = scene
        .shots
        .iter()
        .map(|shot| {
            let frames = shot
                .frames
                .iter()
                .map(|frame| {
                    let location = to_blender(cs, frame.position);
                    let r = camera_rows(cs, frame.right, frame.up, frame.forward);
                    let focus = frame.focus_distance_m.unwrap_or(10.0);
                    [
                        location.x,
                        location.y,
                        location.z,
                        r[0],
                        r[1],
                        r[2],
                        r[3],
                        r[4],
                        r[5],
                        r[6],
                        r[7],
                        r[8],
                        frame.focal_length_mm,
                        focus,
                    ]
                })
                .collect();
            PayloadShot {
                id: shot.id.clone(),
                frame_start: shot.range.start,
                frame_end: shot.range.end.saturating_sub(1),
                sensor_width_mm: shot.sensor_width_mm,
                aperture_f: shot.frames.first().map_or(2.8, |f| f.aperture_f),
                frames,
            }
        })
        .collect();

    let (near, far) = depth_bounds(project, scene);

    BlenderPayload {
        schema_version: PAYLOAD_SCHEMA_VERSION.to_owned(),
        source_id: project.source_id.clone(),
        scene_id: scene.id.clone(),
        scene_title: scene.title.clone(),
        fps_numerator: project.frame_rate.numerator,
        fps_denominator: project.frame_rate.denominator.max(1),
        frame_start: 0,
        frame_end: scene.duration_frames.saturating_sub(1),
        render: RenderSettings {
            resolution: [1024, 576],
            samples: 32,
            passes: vec![ConditioningPass::Beauty, ConditioningPass::Depth],
            frame_range: [0, scene.duration_frames.saturating_sub(1)],
            depth_near_m: near,
            depth_far_m: far,
            use_dof: false,
            output_dir: String::new(),
        },
        subjects,
        shots,
    }
}

/// Depth normalisation bounds from camera↔subject distances across the scene,
/// so depth maps are comparable between frames and shots.
pub fn depth_bounds(project: &SolvedProject, scene: &SolvedScene) -> (f32, f32) {
    let scale = unit_scale(&project.coordinate_system);
    let mut min = f32::INFINITY;
    let mut max: f32 = 0.0;
    for shot in &scene.shots {
        for frame in &shot.frames {
            for track in &scene.subjects {
                let transform = track.transform_at(frame.frame);
                let d = (transform.position - frame.position).length() * scale;
                min = min.min(d);
                max = max.max(d);
            }
        }
    }
    if !min.is_finite() || max <= 0.0 {
        return (0.5, 20.0);
    }
    let near = (min * 0.5).max(0.1);
    let far = (max * 1.5 + 2.0).max(near + 1.0);
    (near, far)
}

impl BlenderPayload {
    pub fn with_render(mut self, settings: RenderSettings) -> Self {
        self.render = settings;
        self
    }

    /// Clamp a requested range to the scene and convert to inclusive bounds.
    pub fn clamp_range(&self, range: Option<FrameRange>) -> [u64; 2] {
        match range {
            Some(r) if r.is_valid() => [
                r.start.min(self.frame_end),
                r.end.saturating_sub(1).min(self.frame_end),
            ],
            _ => [self.frame_start, self.frame_end],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AxisName, Handedness, SignedAxis};

    #[test]
    fn default_system_maps_to_blender_axes() {
        let cs = CoordinateSystem::default();
        let v = to_blender(&cs, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(v, Vec3::new(1.0, -3.0, 2.0));
        assert_eq!(
            direction_to_blender(&cs, Vec3::new(0.0, 0.0, -1.0)),
            Vec3::new(0.0, 1.0, 0.0)
        );
    }

    #[test]
    fn blender_native_system_is_identity() {
        let cs = CoordinateSystem {
            units: Unit::Meters,
            handedness: Handedness::Right,
            up_axis: AxisName::Z,
            forward_axis: SignedAxis::PositiveY,
        };
        assert_eq!(
            to_blender(&cs, Vec3::new(1.0, 2.0, 3.0)),
            Vec3::new(1.0, 2.0, 3.0)
        );
    }

    #[test]
    fn centimetres_are_scaled() {
        let cs = CoordinateSystem {
            units: Unit::Centimeters,
            ..CoordinateSystem::default()
        };
        assert_eq!(
            to_blender(&cs, Vec3::new(100.0, 0.0, 0.0)),
            Vec3::new(1.0, 0.0, 0.0)
        );
    }

    #[test]
    fn camera_rows_put_minus_forward_in_z() {
        let cs = CoordinateSystem::default();
        let id = identity_basis(&cs);
        let r = camera_rows(&cs, id.right, id.up, id.forward);
        // Column Z (indices 2, 5, 8) must be −forward in Blender space = (0, −1, 0).
        assert_eq!([r[2], r[5], r[8]], [0.0, -1.0, 0.0]);
        // Column Y = up = (0, 0, 1).
        assert_eq!([r[1], r[4], r[7]], [0.0, 0.0, 1.0]);
    }
}
