use super::*;
use crate::model::{
    AxisName, CoverageRole, EulerDeg, FrameRate, Handedness, ShotSize, SignedAxis,
};
use crate::palette::Rgb;
use crate::solve::{Fidelity, SolvedCameraFrame, SolvedShot, SolvedSubjectTrack,
SOLVED_SCHEMA_VERSION};

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

#[test]
fn clamp_range_converts_clamps_and_falls_back_on_invalid_ranges() {
    let payload = BlenderPayload {
        schema_version: PAYLOAD_SCHEMA_VERSION.to_owned(),
        source_id: "probe".to_owned(),
        scene_id: "scene".to_owned(),
        scene_title: "Scene".to_owned(),
        fps_numerator: 24,
        fps_denominator: 1,
        frame_start: 0,
        frame_end: 119,
        render: RenderSettings {
            resolution: [1920, 1080],
            samples: 1,
            passes: Vec::new(),
            frame_range: [0, 119],
            depth_near_m: 0.1,
            depth_far_m: 20.0,
            use_dof: false,
            output_dir: String::new(),
        },
        subjects: Vec::new(),
        shots: Vec::new(),
        light_tracks: Vec::new(),
        edit_timeline: Vec::new(),
    };
    // Authored ranges are half-open; Blender bounds are inclusive.
    assert_eq!(
        payload.clamp_range(Some(FrameRange { start: 0, end: 80 })),
        [0, 79]
    );
    // A one-frame range renders exactly that frame.
    assert_eq!(
        payload.clamp_range(Some(FrameRange { start: 30, end: 31 })),
        [30, 30]
    );
    // Both ends clamp to the scene's inclusive last frame.
    assert_eq!(
        payload.clamp_range(Some(FrameRange { start: 100, end: 400 })),
        [100, 119]
    );
    assert_eq!(
        payload.clamp_range(Some(FrameRange { start: 300, end: 400 })),
        [119, 119]
    );
    // No request: the whole scene, first to last frame.
    assert_eq!(payload.clamp_range(None), [0, 119]);
    // Reversed and empty ranges are invalid and fall back to the whole scene.
    assert_eq!(
        payload.clamp_range(Some(FrameRange { start: 50, end: 20 })),
        [0, 119]
    );
    assert_eq!(
        payload.clamp_range(Some(FrameRange { start: 50, end: 50 })),
        [0, 119]
    );
}

fn camera_at_origin(frame: u64) -> SolvedCameraFrame {
    SolvedCameraFrame {
        frame,
        position: Vec3::ZERO,
        rotation_deg: EulerDeg::default(),
        forward: Vec3::new(0.0, 0.0, -1.0),
        up: Vec3::new(0.0, 1.0, 0.0),
        right: Vec3::new(1.0, 0.0, 0.0),
        focal_length_mm: 50.0,
        aperture_f: 2.8,
        horizontal_fov_deg: 39.6,
        focus_distance_m: None,
        focus_target: None,
    }
}

/// One camera at the origin sampling frame 0; one subject per distance.
fn scene_with_distances(distances: &[f32]) -> SolvedScene {
    let subjects = distances
        .iter()
        .map(|&d| SolvedSubjectTrack {
            subject_id: format!("subject_{d}"),
            name: "Subject".to_owned(),
            kind: SubjectKind::Character,
            dimensions_m: Vec3::new(0.5, 1.7, 0.35),
            color: Rgb { r: 220, g: 50, b: 47 },
            color_name: "red".to_owned(),
            transforms: vec![Transform {
                position: Vec3::new(d, 0.0, 0.0),
                ..Transform::default()
            }],
            cues: Vec::new(),
        })
        .collect();
    SolvedScene {
        id: "scene".to_owned(),
        title: "Scene".to_owned(),
        duration_frames: 1,
        subjects,
        shots: vec![SolvedShot {
            id: "only".to_owned(),
            range: FrameRange { start: 0, end: 1 },
            coverage_role: CoverageRole::Master,
            shot_size: ShotSize::Medium,
            purpose: Vec::new(),
            subject_ids: Vec::new(),
            sensor_width_mm: 36.0,
            frames: vec![camera_at_origin(0)],
        }],
    }
}

#[test]
fn depth_bounds_floor_near_convert_units_and_fall_back_when_unmeasurable() {
    let metres = SolvedProject {
        schema_version: SOLVED_SCHEMA_VERSION.to_owned(),
        source_id: "probe".to_owned(),
        source_title: "Probe".to_owned(),
        source_schema_version: "0.1".to_owned(),
        fidelity: Fidelity::Draft,
        frame_rate: FrameRate {
            numerator: 24,
            denominator: 1,
        },
        coordinate_system: CoordinateSystem::default(),
        scenes: Vec::new(),
    };
    // Closest subject at 0.12 m: min/2 = 0.06 m floors at 0.1 m; the 4 m
    // subject puts far at 1.5·max + 2 m = 8 m.
    let (near, far) = depth_bounds(&metres, &scene_with_distances(&[0.12, 4.0]));
    assert!((near - 0.1).abs() < 1e-6, "{near}");
    assert!((far - 8.0).abs() < 1e-5, "{far}");
    // A centimetre document measures identical metre bounds.
    let mut centimetres = metres.clone();
    centimetres.coordinate_system = CoordinateSystem {
        units: Unit::Centimeters,
        ..CoordinateSystem::default()
    };
    let (near, far) = depth_bounds(&centimetres, &scene_with_distances(&[12.0, 400.0]));
    assert!((near - 0.1).abs() < 1e-6, "{near}");
    assert!((far - 8.0).abs() < 1e-5, "{far}");
    // No measurable distance (an empty scene, or a subject sitting exactly
    // on the camera) falls back to the default bracket, never zero bounds.
    assert_eq!(
        depth_bounds(&metres, &scene_with_distances(&[])),
        (0.5, 20.0)
    );
    assert_eq!(
        depth_bounds(&metres, &scene_with_distances(&[0.0])),
        (0.5, 20.0)
    );
}
