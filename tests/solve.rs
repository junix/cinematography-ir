use std::path::PathBuf;

use cinematography_ir::math::Vec3;
use cinematography_ir::solve::SolvedCameraFrame;
use cinematography_ir::view::project::FrameProjection;
use cinematography_ir::{
    load_project, normalize_units, solve_project, CameraOperation, CameraRig, CineProject,
    Fidelity, GeometryProxy, Severity, SolveError, SolveOptions, SolvedProject, TargetRef,
};

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(name)
}

fn solve(name: &str) -> SolvedProject {
    let project = load_project(example(name)).expect("example must parse");
    let output = solve_project(&project, &SolveOptions::default()).expect("example must solve");
    let geometry: Vec<_> = output
        .report
        .diagnostics
        .iter()
        .filter(|d| d.code.starts_with("GEOMETRY_"))
        .collect();
    assert!(
        geometry.is_empty(),
        "{name}: unexpected geometry diagnostics {geometry:#?}"
    );
    output.solved
}

fn distance(a: Vec3, b: Vec3) -> f32 {
    (a - b).length()
}

fn subject_height_fraction(
    solved: &SolvedProject,
    scene_index: usize,
    shot_index: usize,
    subject_id: &str,
    frame: u64,
) -> f32 {
    let scene = &solved.scenes[scene_index];
    let shot = &scene.shots[shot_index];
    let camera = shot.frame_at(frame).unwrap();
    let subject = scene.subject(subject_id).unwrap();
    let transform = subject.transform_at(frame);
    let projection = FrameProjection {
        position: camera.position,
        basis: cinematography_ir::math::Basis {
            right: camera.right,
            up: camera.up,
            forward: camera.forward,
        },
        focal_length_mm: camera.focal_length_mm,
        sensor_width_mm: shot.sensor_width_mm,
        aspect: 16.0 / 9.0,
        width: 1600.0,
        height: 900.0,
    };
    let basis =
        cinematography_ir::math::oriented_basis(&solved.coordinate_system, transform.rotation_deg);
    let (bounds, _) = projection
        .project_box(
            transform.position,
            subject.dimensions_m,
            &basis,
            transform.scale,
        )
        .unwrap();
    (bounds.max.y - bounds.min.y) / projection.height
}

#[test]
fn every_example_solves_deterministically() {
    for name in [
        "dialogue.yaml",
        "intentional_axis_cross.yaml",
        "unsafe_axis_cross.yaml",
        "dolly_zoom.yaml",
        "jaws_beach_dolly_zoom.yaml",
        "follow_handheld.yaml",
        "crane_reveal.yaml",
    ] {
        let first = solve(name);
        let second = solve(name);
        assert_eq!(first, second, "{name} must solve identically twice");
        let json_a = serde_json::to_string(&first).unwrap();
        let json_b = serde_json::to_string(&second).unwrap();
        assert_eq!(json_a, json_b);

        for scene in &first.scenes {
            for track in &scene.subjects {
                assert_eq!(track.transforms.len() as u64, scene.duration_frames);
            }
            for shot in &scene.shots {
                assert_eq!(
                    shot.frames.len() as u64,
                    shot.range.duration(),
                    "{name}/{}",
                    shot.id
                );
                for frame in &shot.frames {
                    assert!(frame.position.is_finite());
                    assert!((frame.forward.length() - 1.0).abs() < 1e-4);
                    assert!(frame.forward.dot(frame.up).abs() < 1e-3);
                    assert!(frame.focal_length_mm > 0.0);
                }
            }
        }
    }
}

#[test]
fn solved_json_round_trips() {
    let solved = solve("dialogue.yaml");
    let json = serde_json::to_string_pretty(&solved).unwrap();
    let back: SolvedProject = serde_json::from_str(&json).unwrap();
    assert_eq!(solved, back);
}

#[test]
fn dolly_moves_toward_the_framed_subject() {
    let solved = solve("dialogue.yaml");
    let scene = &solved.scenes[0];
    let alice = scene.subject("alice").unwrap();
    let shot = scene.shot("alice_close_up").unwrap();
    let aim = |frame| {
        let t = alice.transform_at(frame);
        t.position + Vec3::new(0.0, alice.dimensions_m.y * 0.9, 0.0)
    };

    let start = distance(shot.frames[0].position, aim(shot.range.start));
    let end = distance(
        shot.frames.last().unwrap().position,
        aim(shot.range.end - 1),
    );
    assert!(
        end < start,
        "dolly in must reduce distance: {start} -> {end}"
    );

    let mut previous = start;
    for frame in &shot.frames {
        let d = distance(frame.position, aim(frame.frame));
        assert!(d <= previous + 1e-4, "distance must be non-increasing");
        previous = d;
    }
    let travelled = distance(
        shot.frames[0].position,
        shot.frames.last().unwrap().position,
    );
    assert!(
        (travelled - 0.35).abs() < 1e-3,
        "dolly distance {travelled}"
    );
    // Motion is confined to the operation's range [16, 72).
    assert_eq!(shot.frames[16].position, shot.frames[0].position);
    assert_eq!(shot.frames[72].position, shot.frames[79].position);
}

#[test]
fn dolly_zoom_keeps_subject_size_constant() {
    let solved = solve("dolly_zoom.yaml");
    let scene = &solved.scenes[0];
    let hero = scene.subject("hero").unwrap();
    let shot = &scene.shots[0];
    let aim = hero.transforms[0].position + Vec3::new(0.0, hero.dimensions_m.y * 0.9, 0.0);

    let first = &shot.frames[0];
    let last = shot.frames.last().unwrap();
    let d0 = distance(first.position, aim);
    let d1 = distance(last.position, aim);
    assert!((d0 - 4.5).abs() < 0.2, "start distance {d0}");
    assert!((d1 - 2.2).abs() < 0.2, "end distance {d1}");
    assert!(
        last.focal_length_mm < first.focal_length_mm,
        "dolly-in requires zoom-out"
    );

    let mut previous_distance = d0;
    for frame in &shot.frames[24..112] {
        let d = distance(frame.position, aim);
        assert!(d <= previous_distance + 1e-4, "distance must not increase");
        previous_distance = d;
        assert!(
            (subject_height_fraction(&solved, 0, 0, "hero", frame.frame) - 0.42).abs() < 1e-4,
            "requested screen fraction must be solved at frame {}",
            frame.frame
        );
        assert!(
            frame.forward.dot(aim - frame.position) > 0.0,
            "camera keeps facing hero"
        );
        assert_eq!(
            frame.focus_distance_m.map(|v| (v - d).abs() < 1e-3),
            Some(true)
        );
    }
}

#[test]
fn jaws_study_keeps_the_observer_stable_while_the_background_expands() {
    let solved = solve("jaws_beach_dolly_zoom.yaml");
    let scene = &solved.scenes[0];
    let observer = scene.subject("observer").unwrap();
    let background = scene.subject("left_beachgoer").unwrap();
    let shot = &scene.shots[0];
    let before = &shot.frames[48];
    let after = &shot.frames[167];
    let observer_aim =
        observer.transforms[48].position + Vec3::new(0.0, observer.dimensions_m.y * 0.9, 0.0);
    let background_aim =
        background.transforms[48].position + Vec3::new(0.0, background.dimensions_m.y * 0.9, 0.0);

    let background_offset = |frame: &SolvedCameraFrame| {
        let delta = background_aim - frame.position;
        frame.focal_length_mm * delta.dot(frame.right).abs() / delta.dot(frame.forward)
    };

    assert!(
        (subject_height_fraction(&solved, 0, 0, "observer", 48) - 0.42).abs() < 1e-4
            && (subject_height_fraction(&solved, 0, 0, "observer", 167) - 0.42).abs() < 1e-4,
        "observer projected bounds must remain locked"
    );
    assert!(
        after.focal_length_mm > before.focal_length_mm,
        "dolly-out requires zoom-in"
    );
    assert!(
        distance(after.position, observer_aim) > distance(before.position, observer_aim),
        "camera must move away from the observer"
    );
    assert!(
        background_offset(after) > background_offset(before) * 1.25,
        "background must expand outward: {} -> {}",
        background_offset(before),
        background_offset(after)
    );
}

#[test]
fn reveal_visibility_solves_lateral_travel_from_projected_occlusion() {
    let source = r#"
schema_version: "0.1"
id: reveal
title: Reveal
frame_rate: { numerator: 24 }
scenes:
  - id: scene
    title: Scene
    duration_frames: 49
    subjects:
      - id: hero
        name: Hero
        kind: character
        dimensions_m: { x: 0.5, y: 1.7, z: 0.4 }
      - id: wall
        name: Wall
        kind: environment
        dimensions_m: { x: 0.8, y: 1.9, z: 0.2 }
        initial_transform: { position: { x: 0.0, y: 0.0, z: 2.0 } }
        geometry_proxy: { kind: wall }
    shots:
      - id: reveal
        range: { start: 0, end: 49 }
        framing: { shot_size: medium, subject_ids: [hero] }
        camera:
          initial_state:
            transform: { position: { x: 0.0, y: 1.45, z: 5.0 } }
            lens: { focal_length_mm: 50.0 }
          operations:
            - range: { start: 0, end: 49 }
              operation:
                op: reveal
                target: { type: subject, id: hero }
                occluder: { type: subject, id: wall }
                lateral_distance_m: 0.5
                visibility: { from: 0.0, to: 0.8, monotonic: true }
                preserve: { target_screen_y: 0.48 }
"#;
    let project: cinematography_ir::CineProject = serde_yaml::from_str(source).unwrap();
    let compiled =
        cinematography_ir::compile_project(&project, &cinematography_ir::CompileOptions::default())
            .unwrap();
    let shot = &compiled.compiled.scenes[0].shots[0];
    let visible: Vec<_> = shot
        .screen_track("hero")
        .unwrap()
        .frames
        .iter()
        .map(|frame| frame.visible_fraction)
        .collect();
    assert!(visible[0] < 0.15, "start visibility: {}", visible[0]);
    assert!(
        (visible[48] - 0.8).abs() < 0.08,
        "end visibility: {}",
        visible[48]
    );
    assert!(
        visible.windows(2).all(|pair| pair[1] + 0.02 >= pair[0]),
        "visibility should be monotonic: {visible:?}"
    );
    let y = shot.screen_track("hero").unwrap().frames[48].center.y;
    assert!((y - 0.48).abs() < 0.04, "preserved y: {y}");
}

#[test]
fn every_crane_point_target_is_normalized() {
    let source = r#"
schema_version: "0.1"
id: cranes
title: Cranes
frame_rate: { numerator: 24 }
coordinate_system: { units: centimeters }
scenes:
  - id: scene
    title: Scene
    duration_frames: 24
    shots:
      - id: shot
        range: { start: 0, end: 24 }
        framing: { shot_size: medium }
        camera:
          initial_state: { transform: {} }
          operations:
            - range: { start: 0, end: 12 }
              operation: { op: crane, delta: { x: 0, y: 100, z: 0 }, look_at: { type: point, position: { x: 100, y: 200, z: 300 } } }
            - range: { start: 12, end: 24 }
              operation: { op: crane, delta: { x: 0, y: 100, z: 0 }, look_at: { type: point, position: { x: 400, y: 500, z: 600 } } }
"#;
    let project: CineProject = serde_yaml::from_str(source).unwrap();
    let normalized = normalize_units(&project);
    let operations = &normalized.scenes[0].shots[0].camera.operations;
    let points: Vec<Vec3> = operations
        .iter()
        .filter_map(|timed| match &timed.operation {
            CameraOperation::Crane {
                look_at: Some(TargetRef::Point { position }),
                ..
            } => Some(*position),
            _ => None,
        })
        .collect();
    assert_eq!(
        points,
        vec![Vec3::new(1.0, 2.0, 3.0), Vec3::new(4.0, 5.0, 6.0)]
    );
}

#[test]
fn follow_half_life_is_frame_rate_independent() {
    fn sample(fps: u32) -> f32 {
        let duration = fps * 2;
        let source = format!(
            r#"
schema_version: "0.1"
id: follow
title: Follow
frame_rate: {{ numerator: {fps} }}
scenes:
  - id: scene
    title: Scene
    duration_frames: {duration}
    subjects:
      - {{ id: hero, name: Hero, kind: character }}
    blocking:
      - subject_id: hero
        keyframes:
          - {{ frame: 0, transform: {{ position: {{ x: 0, y: 0, z: 0 }} }} }}
          - {{ frame: {duration}, transform: {{ position: {{ x: 2, y: 0, z: 0 }} }} }}
    shots:
      - id: shot
        range: {{ start: 0, end: {duration} }}
        framing: {{ shot_size: medium, subject_ids: [hero] }}
        camera:
          initial_state: {{ transform: {{ position: {{ x: 0, y: 1.5, z: 4 }} }} }}
          operations:
            - range: {{ start: 0, end: {duration} }}
              operation:
                op: follow
                target: {{ type: subject, id: hero }}
                offset: {{ x: 0, y: 1.5, z: 4 }}
                lag_half_life_s: 0.18
"#
        );
        let project: CineProject = serde_yaml::from_str(&source).unwrap();
        let solved = solve_project(&project, &SolveOptions::default())
            .unwrap()
            .solved;
        solved.scenes[0].shots[0]
            .frame_at(fps as u64)
            .unwrap()
            .position
            .x
    }
    assert!((sample(24) - sample(60)).abs() < 0.02);
    // Frame-rate independence alone would also hold for a camera that never
    // moves: the lag itself must match the declared half-life. The subject
    // runs at 1 m/s, so after 1 s the camera must have tracked to within
    // ~v·half_life/ln2 ≈ 0.26 m of the target.
    for (fps, value) in [(24, sample(24)), (60, sample(60))] {
        let lag = 1.0 - value;
        assert!(
            (0.15..0.35).contains(&lag),
            "{fps} fps: camera at {value} m, lag {lag} m does not match a 0.18 s half-life"
        );
    }
}

#[test]
fn follow_handheld_tracks_the_runner_and_finishes_the_zoom() {
    let solved = solve("follow_handheld.yaml");
    let scene = &solved.scenes[0];
    let runner = scene.subject("runner").unwrap();
    let shot = scene.shot("handheld_follow").unwrap();
    let first = &shot.frames[0];
    let last = shot.frames.last().unwrap();

    assert!(
        last.position.z < first.position.z - 7.5,
        "camera must follow the runner down the alley: {:?} -> {:?}",
        first.position,
        last.position
    );
    let runner_aim = runner.transforms.last().unwrap().position
        + Vec3::new(0.0, runner.dimensions_m.y * 0.9, 0.0);
    let follow_distance = distance(last.position, runner_aim);
    assert!(
        (follow_distance - 4.5).abs() < 0.15,
        "follow offset must settle near 4.5 m: {follow_distance}"
    );
    assert!((last.focal_length_mm - 45.0).abs() < 1e-3);
    assert_eq!(shot.frames[120].focal_length_mm, last.focal_length_mm);
}

#[test]
fn crane_reveal_moves_clear_racks_focus_and_rises() {
    let solved = solve("crane_reveal.yaml");
    let shot = solved.scenes[0].shot("reveal_and_rise").unwrap();
    let first = &shot.frames[0];
    let revealed = &shot.frames[71];
    let raised = &shot.frames[167];
    let last = shot.frames.last().unwrap();

    assert!(
        revealed.position.x > first.position.x + 1.6,
        "reveal must clear the foreground column: {:?} -> {:?}",
        first.position,
        revealed.position
    );
    assert!(
        revealed.focus_distance_m.unwrap() > first.focus_distance_m.unwrap() + 3.0,
        "rack focus must move from the column to the visitor"
    );
    assert!(
        raised.position.y > revealed.position.y + 1.9,
        "crane must rise by two metres: {:?} -> {:?}",
        revealed.position,
        raised.position
    );
    assert_eq!(
        raised.position, last.position,
        "camera holds after the crane"
    );
    assert_eq!(raised.rotation_deg, last.rotation_deg);
}

#[test]
fn orbit_keeps_radius_and_faces_target() {
    let solved = solve("intentional_axis_cross.yaml");
    let scene = &solved.scenes[0];
    let shot = scene.shot("visible_cross").unwrap();
    let target = Vec3::new(0.0, 1.4, 0.0);

    // The camera starts 3.0017 m from the centre (0.1 m above it) and the
    // operation declares radius_m: 3.0, so the radius eases from r0 to 3.0.
    let r0 = distance(shot.frames[0].position, target);
    assert!((r0 - 3.0).abs() < 2e-3, "{r0}");
    for frame in &shot.frames {
        let r = distance(frame.position, target);
        assert!(
            r <= r0 + 1e-4 && r >= 3.0 - 1e-4,
            "orbit radius drifted to {r}"
        );
        let to_target = (target - frame.position).normalized().unwrap();
        assert!(
            frame.forward.dot(to_target) > 0.999,
            "camera must face the orbit centre"
        );
    }
    let first = shot.frames[0].position;
    let last = shot.frames.last().unwrap().position;
    assert!(
        (distance(last, target) - 3.0).abs() < 1e-3,
        "radius settles at 3.0"
    );
    assert!((first.z - 3.0).abs() < 1e-3);
    assert!(
        last.z < -2.9,
        "after a 180° orbit the camera is on the far side: {last:?}"
    );
    assert!(last.x.abs() < 0.1);
}

#[test]
fn blocking_interpolates_between_keyframes() {
    let source = r#"
schema_version: "0.1"
id: blocking
title: Blocking
frame_rate: { numerator: 24 }
scenes:
  - id: s
    title: S
    duration_frames: 48
    subjects:
      - { id: walker, name: Walker, kind: character }
    blocking:
      - subject_id: walker
        keyframes:
          - { frame: 8, transform: { position: { x: 0.0, y: 0.0, z: 0.0 }, rotation_deg: { yaw: 0.0 } } }
          - { frame: 40, transform: { position: { x: 4.0, y: 0.0, z: -2.0 }, rotation_deg: { yaw: 90.0 } } }
    shots:
      - id: only
        range: { start: 0, end: 48 }
        framing: { shot_size: medium, subject_ids: [walker] }
        camera:
          initial_state:
            transform: { position: { x: 0.0, y: 1.5, z: 6.0 } }
"#;
    let project: CineProject = serde_yaml::from_str(source).unwrap();
    let solved = solve_project(&project, &SolveOptions::default())
        .unwrap()
        .solved;
    let track = &solved.scenes[0].subjects[0];
    assert_eq!(
        track.transforms[0].position,
        Vec3::ZERO,
        "held before first key"
    );
    let mid = track.transforms[24].position;
    assert!(
        (mid.x - 2.0).abs() < 1e-4 && (mid.z + 1.0).abs() < 1e-4,
        "{mid:?}"
    );
    assert!((track.transforms[24].rotation_deg.yaw - 45.0).abs() < 1e-3);
    assert_eq!(
        track.transforms[47].position,
        Vec3::new(4.0, 0.0, -2.0),
        "held after last key"
    );
    assert_eq!(
        track.dimensions_m,
        Vec3::new(0.5, 1.75, 0.35),
        "character default dimensions"
    );
    assert_eq!(track.color_name, "red");
}

#[test]
fn handheld_noise_is_seeded() {
    let make = |seed: u64| {
        format!(
            r#"
schema_version: "0.1"
id: noise
title: Noise
frame_rate: {{ numerator: 24 }}
scenes:
  - id: s
    title: S
    duration_frames: 48
    subjects:
      - {{ id: a, name: A, kind: character }}
    shots:
      - id: only
        range: {{ start: 0, end: 48 }}
        framing: {{ shot_size: medium, subject_ids: [a] }}
        camera:
          rig: handheld
          initial_state:
            transform: {{ position: {{ x: 0.0, y: 1.5, z: 4.0 }} }}
          operations:
            - range: {{ start: 0, end: 48 }}
              operation:
                op: handheld_noise
                translation_amplitude_m: 0.02
                rotation_amplitude_deg: 0.5
                frequency_hz: 1.5
                seed: {seed}
"#
        )
    };
    let solve_seed = |seed: u64| {
        let project: CineProject = serde_yaml::from_str(&make(seed)).unwrap();
        solve_project(&project, &SolveOptions::default())
            .unwrap()
            .solved
    };
    let a = solve_seed(7);
    let b = solve_seed(7);
    let c = solve_seed(8);
    assert_eq!(a, b, "same seed must reproduce");
    assert_ne!(a, c, "different seeds must differ");

    let base = Vec3::new(0.0, 1.5, 4.0);
    let mut moved = false;
    for frame in &a.scenes[0].shots[0].frames {
        let offset = distance(frame.position, base);
        assert!(offset <= 0.02 * 1.5, "noise exceeds amplitude: {offset}");
        moved |= offset > 1e-4;
    }
    assert!(moved, "noise must actually move the camera");
}

#[test]
fn geometry_checks_catch_a_camera_facing_away() {
    let mut project = load_project(example("dialogue.yaml")).unwrap();
    // Re-introduce the authoring error: camera on the wrong side of the
    // subjects for a -Z-forward system.
    let shot = &mut project.scenes[0].shots[0];
    shot.camera.initial_state.transform.position.z = -5.2;
    let output = solve_project(&project, &SolveOptions::default()).unwrap();
    let codes: Vec<&str> = output
        .report
        .diagnostics
        .iter()
        .map(|d| d.code.as_str())
        .collect();
    assert!(
        codes.contains(&"GEOMETRY_SUBJECT_BEHIND_CAMERA"),
        "{codes:?}"
    );
    assert!(codes.contains(&"GEOMETRY_AXIS_SIDE_MISMATCH"), "{codes:?}");
}

#[test]
fn geometry_checks_catch_subjects_outside_the_fov() {
    let mut project = load_project(example("dialogue.yaml")).unwrap();
    let shot = &mut project.scenes[0].shots[1]; // alice_close_up, 65 mm
    shot.camera.initial_state.transform.rotation_deg.yaw = -60.0; // turn away from Alice
    let output = solve_project(&project, &SolveOptions::default()).unwrap();
    assert!(output
        .report
        .diagnostics
        .iter()
        .any(|d| d.code == "GEOMETRY_SUBJECT_OUTSIDE_FOV"));
}

#[test]
fn axis_side_declarations_match_geometry_in_every_example() {
    // `solve()` already asserts no GEOMETRY_* diagnostics; this spells out the
    // intent for the two examples whose declarations exercise both sides.
    solve("unsafe_axis_cross.yaml");
    solve("intentional_axis_cross.yaml");
}

#[test]
fn full_fidelity_runs_and_invalid_documents_are_rejected() {
    let mut project = load_project(example("dialogue.yaml")).unwrap();
    let full = solve_project(
        &project,
        &SolveOptions {
            fidelity: Fidelity::Full,
        },
    )
    .unwrap();
    assert_eq!(full.solved.fidelity, Fidelity::Full);
    assert_eq!(full.solved.scenes[0].shots.len(), 3);

    let mut locked = single_shot(
        "            - range: { start: 0, end: 48 }\n              operation: { op: dolly, distance_m: 2.0 }",
    );
    locked.scenes[0].shots[0].camera.rig = CameraRig::Locked;
    let locked = solve_project(
        &locked,
        &SolveOptions {
            fidelity: Fidelity::Full,
        },
    )
    .unwrap();
    let frames = &locked.solved.scenes[0].shots[0].frames;
    assert!(
        distance(frames[0].position, frames[47].position) < 1e-5,
        "a locked rig must reject translational camera motion"
    );

    let mut collision = single_shot(
        "            - range: { start: 0, end: 48 }\n              operation: { op: hold }",
    );
    collision.scenes[0].subjects[0].geometry_proxy = Some(GeometryProxy::Box);
    collision.scenes[0].shots[0]
        .camera
        .initial_state
        .transform
        .position = Vec3::new(0.0, 0.875, 0.0);
    let collision = solve_project(
        &collision,
        &SolveOptions {
            fidelity: Fidelity::Full,
        },
    )
    .unwrap();
    assert!(collision
        .report
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "CAMERA_COLLISION_RESOLVED"));
    let camera = collision.solved.scenes[0].shots[0].frames[0].position;
    assert!(distance(camera, Vec3::new(0.0, 0.875, 0.0)) > 0.9);

    project.scenes[0].shots[0]
        .framing
        .subject_ids
        .push("nobody".to_owned());
    match solve_project(&project, &SolveOptions::default()) {
        Err(SolveError::Invalid(report)) => {
            assert!(report.has_errors());
            assert!(
                report.diagnostics.iter().any(|diagnostic| diagnostic.code
                    == "FRAMING_SUBJECT_UNKNOWN"
                    && diagnostic.severity == Severity::Error),
                "the unknown framing subject must be named: {:#?}",
                report.diagnostics
            );
        }
        other => panic!("expected validation failure, got {other:?}"),
    }
}

fn single_shot(operations: &str) -> CineProject {
    let source = format!(
        r#"
schema_version: "0.1"
id: probe
title: Probe
frame_rate: {{ numerator: 24 }}
scenes:
  - id: s
    title: S
    duration_frames: 48
    subjects:
      - {{ id: a, name: A, kind: character }}
    shots:
      - id: only
        range: {{ start: 0, end: 48 }}
        purpose: [observe]
        framing: {{ shot_size: medium, subject_ids: [a] }}
        camera:
          initial_state:
            transform: {{ position: {{ x: 0.0, y: 1.5, z: 4.0 }} }}
            lens: {{ focal_length_mm: 35.0 }}
          operations:
{operations}
"#
    );
    serde_yaml::from_str(&source).unwrap()
}

#[test]
fn target_operations_release_their_channel_after_their_range() {
    // look_at then pan: the pan must accumulate on top of the held look-at yaw.
    let project = single_shot(
        r#"
            - range: { start: 0, end: 24 }
              operation: { op: look_at, target: { type: subject, id: a } }
            - range: { start: 24, end: 48 }
              operation: { op: pan, degrees: 30.0 }
"#,
    );
    let solved = solve_project(&project, &SolveOptions::default())
        .unwrap()
        .solved;
    let frames = &solved.scenes[0].shots[0].frames;
    let held = frames[23].rotation_deg.yaw;
    assert!(
        held.abs() < 1e-3,
        "camera on axis: look_at yaw ≈ 0, got {held}"
    );
    assert!(
        (frames[47].rotation_deg.yaw - (held + 30.0)).abs() < 1e-2,
        "pan must add 30° after look_at: {}",
        frames[47].rotation_deg.yaw
    );

    // orbit then dolly: distance must shrink by the full dolly amount.
    let project = single_shot(
        r#"
            - range: { start: 0, end: 24 }
              operation: { op: orbit, target: { type: subject, id: a }, azimuth_deg: 90.0 }
            - range: { start: 24, end: 48 }
              operation: { op: dolly, distance_m: 1.0 }
"#,
    );
    let solved = solve_project(&project, &SolveOptions::default())
        .unwrap()
        .solved;
    let frames = &solved.scenes[0].shots[0].frames;
    let aim = Vec3::new(0.0, 1.75 * 0.9, 0.0);
    let after_orbit = distance(frames[23].position, aim);
    let after_dolly = distance(frames[47].position, aim);
    assert!(
        (after_orbit - after_dolly - 1.0).abs() < 0.02,
        "dolly must move 1.0 m closer after the orbit: {after_orbit} -> {after_dolly}"
    );
}

#[test]
fn whole_shot_operation_reaches_its_declared_value() {
    let project = single_shot(
        r#"
            - range: { start: 0, end: 48 }
              easing: linear
              operation: { op: dolly, distance_m: 1.0 }
"#,
    );
    let solved = solve_project(&project, &SolveOptions::default())
        .unwrap()
        .solved;
    let frames = &solved.scenes[0].shots[0].frames;
    let travelled = distance(frames[0].position, frames[47].position);
    assert!((travelled - 1.0).abs() < 1e-4, "{travelled}");

    let project = single_shot(
        r#"
            - range: { start: 10, end: 11 }
              operation: { op: pan, degrees: 15.0 }
"#,
    );
    let solved = solve_project(&project, &SolveOptions::default())
        .unwrap()
        .solved;
    let frames = &solved.scenes[0].shots[0].frames;
    assert_eq!(frames[9].rotation_deg.yaw, 0.0);
    assert!(
        (frames[10].rotation_deg.yaw - 15.0).abs() < 1e-4,
        "single-frame op snaps"
    );
    assert!((frames[47].rotation_deg.yaw - 15.0).abs() < 1e-4);
}

#[test]
fn orbit_rotation_eases_in_with_the_position() {
    let project = single_shot(
        r#"
            - range: { start: 0, end: 48 }
              easing: linear
              operation: { op: orbit, target: { type: point, position: { x: 2.0, y: 1.5, z: 0.0 } }, azimuth_deg: 180.0 }
"#,
    );
    let solved = solve_project(&project, &SolveOptions::default())
        .unwrap()
        .solved;
    let frames = &solved.scenes[0].shots[0].frames;
    // Camera starts looking −Z while the centre is off to the right; the yaw
    // must move gradually rather than snapping on the first frame.
    let first_delta = (frames[1].rotation_deg.yaw - frames[0].rotation_deg.yaw).abs();
    let total = (frames[47].rotation_deg.yaw - frames[0].rotation_deg.yaw).abs();
    assert!(
        first_delta < total / 10.0,
        "first-frame yaw jump {first_delta} of {total}"
    );
}

#[test]
fn handheld_noise_ramps_in_and_out() {
    let project = single_shot(
        r#"
            - range: { start: 12, end: 36 }
              operation: { op: handheld_noise, translation_amplitude_m: 0.05, rotation_amplitude_deg: 1.0, frequency_hz: 2.0, seed: 3 }
"#,
    );
    let solved = solve_project(&project, &SolveOptions::default())
        .unwrap()
        .solved;
    let frames = &solved.scenes[0].shots[0].frames;
    let base = Vec3::new(0.0, 1.5, 4.0);
    let first = distance(frames[12].position, base);
    let peak = frames[12..36]
        .iter()
        .map(|f| distance(f.position, base))
        .fold(0.0, f32::max);
    assert!(
        first < peak * 0.5,
        "noise must ramp in: first {first}, peak {peak}"
    );
    assert_eq!(frames[36].position, base, "noise stops after the range");
}

#[test]
fn initial_transform_is_the_implicit_first_keyframe() {
    let source = r#"
schema_version: "0.1"
id: blocking
title: Blocking
frame_rate: { numerator: 24 }
scenes:
  - id: s
    title: S
    duration_frames: 48
    subjects:
      - id: walker
        name: Walker
        kind: character
        initial_transform: { position: { x: 0.0, y: 0.0, z: 0.0 } }
    blocking:
      - subject_id: walker
        keyframes:
          - { frame: 24, transform: { position: { x: 2.0, y: 0.0, z: 0.0 } } }
          - { frame: 48, transform: { position: { x: 2.0, y: 0.0, z: -4.0 } } }
    shots:
      - id: only
        range: { start: 0, end: 48 }
        framing: { shot_size: medium, subject_ids: [walker] }
        camera:
          initial_state:
            transform: { position: { x: 0.0, y: 1.5, z: 6.0 } }
"#;
    let project: CineProject = serde_yaml::from_str(source).unwrap();
    let track = &solve_project(&project, &SolveOptions::default())
        .unwrap()
        .solved
        .scenes[0]
        .subjects[0];
    assert_eq!(
        track.transforms[0].position,
        Vec3::ZERO,
        "starts at initial_transform"
    );
    assert!(
        (track.transforms[12].position.x - 1.0).abs() < 1e-4,
        "halfway to the first key"
    );
    assert!((track.transforms[24].position.x - 2.0).abs() < 1e-4);
    // A boundary keyframe at frame == duration is approached, never reached.
    let last = track.transforms[47].position;
    assert!((last.z - (-4.0 * 23.0 / 24.0)).abs() < 1e-3, "{last:?}");
    assert_eq!(track.cues.len(), 2, "synthetic frame-0 key adds no cue");
}

#[test]
fn centimetre_documents_solve_like_metre_documents() {
    let metres = load_project(example("dialogue.yaml")).unwrap();
    let mut value: serde_yaml::Value = serde_yaml::to_value(&metres).unwrap();
    fn scale_positions(v: &mut serde_yaml::Value) {
        match v {
            serde_yaml::Value::Mapping(map) => {
                let keys: Vec<_> = map.keys().cloned().collect();
                for key in keys {
                    let name = key.as_str().unwrap_or("").to_owned();
                    let child = map.get_mut(&key).unwrap();
                    if name == "position" || name == "delta" || name == "offset" {
                        if let serde_yaml::Value::Mapping(p) = child {
                            for axis in ["x", "y", "z"] {
                                let k = serde_yaml::Value::from(axis);
                                if let Some(n) = p.get(&k).and_then(|n| n.as_f64()) {
                                    p.insert(k, serde_yaml::Value::from(n * 100.0));
                                }
                            }
                        }
                    } else {
                        scale_positions(child);
                    }
                }
            }
            serde_yaml::Value::Sequence(items) => items.iter_mut().for_each(scale_positions),
            _ => {}
        }
    }
    scale_positions(&mut value);
    value["coordinate_system"]["units"] = "centimeters".into();
    let centimetres: CineProject = serde_yaml::from_value(value).unwrap();

    let a = solve_project(&metres, &SolveOptions::default())
        .unwrap()
        .solved;
    let b = solve_project(&centimetres, &SolveOptions::default())
        .unwrap()
        .solved;
    assert_eq!(
        b.coordinate_system.units,
        cinematography_ir::Unit::Meters,
        "solved output is metres"
    );
    for (sa, sb) in a.scenes.iter().zip(&b.scenes) {
        for (ta, tb) in sa.subjects.iter().zip(&sb.subjects) {
            assert_eq!(
                ta.dimensions_m, tb.dimensions_m,
                "dimensions are metres by contract"
            );
            for (x, y) in ta.transforms.iter().zip(&tb.transforms) {
                assert!(distance(x.position, y.position) < 1e-3);
            }
        }
        for (xa, xb) in sa.shots.iter().zip(&sb.shots) {
            for (fa, fb) in xa.frames.iter().zip(&xb.frames) {
                assert!(
                    distance(fa.position, fb.position) < 1e-3,
                    "{} vs {}",
                    fa.frame,
                    fb.frame
                );
                assert!((fa.focal_length_mm - fb.focal_length_mm).abs() < 1e-3);
                assert!((fa.rotation_deg.yaw - fb.rotation_deg.yaw).abs() < 1e-2);
            }
        }
    }
}
