use std::path::PathBuf;

use cinematography_ir::compiled::{ConstraintStatus, PhaseKind, ShotConstraintKind};
use cinematography_ir::{
    compile_project, load_project, CompileOptions, CompiledProject, ShotPhrase,
};

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(name)
}

#[test]
fn jaws_compiles_phases_screen_constraints_and_prohibitions() {
    let project = load_project(example("jaws_beach_dolly_zoom.yaml")).unwrap();
    let output = compile_project(&project, &CompileOptions::default()).unwrap();
    let scene = &output.compiled.scenes[0];
    let shot = &scene.shots[0];

    assert_eq!(
        shot.phases
            .iter()
            .map(|phase| (phase.local_range, phase.kind))
            .collect::<Vec<_>>(),
        vec![
            (
                cinematography_ir::FrameRange { start: 0, end: 48 },
                PhaseKind::Hold
            ),
            (
                cinematography_ir::FrameRange {
                    start: 48,
                    end: 168
                },
                PhaseKind::Motion
            ),
            (
                cinematography_ir::FrameRange {
                    start: 168,
                    end: 192
                },
                PhaseKind::Settle
            ),
        ]
    );

    let bbox = shot
        .constraints
        .iter()
        .find(|constraint| {
            matches!(
                constraint.kind,
                ShotConstraintKind::SubjectBboxHeight { .. }
            )
        })
        .unwrap();
    assert_eq!(
        shot.evaluations
            .iter()
            .find(|value| value.constraint_id == bbox.id)
            .unwrap()
            .status,
        ConstraintStatus::Pass
    );
    assert!(shot.constraints.iter().any(|constraint| matches!(
        constraint.kind,
        ShotConstraintKind::PairScreenSeparation { .. }
    )));
    assert!(shot
        .prohibitions
        .contains(&cinematography_ir::Prohibition::DigitalCropAsDollySubstitute));
    assert!(shot
        .intent
        .phrases
        .contains(&ShotPhrase::SubjectLockDollyZoom));
    assert_eq!(shot.screen_tracks.len(), 3);
    assert_eq!(shot.screen_tracks[0].frames.len(), 192);
}

#[test]
fn compiled_guidance_is_deterministic_self_contained_and_round_trips() {
    let project = load_project(example("dialogue.yaml")).unwrap();
    let first = compile_project(&project, &CompileOptions::default())
        .unwrap()
        .compiled;
    let second = compile_project(&project, &CompileOptions::default())
        .unwrap()
        .compiled;
    assert_eq!(first, second);
    let json = serde_json::to_string_pretty(&first).unwrap();
    let decoded: CompiledProject = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, first);
    assert_eq!(decoded.scenes[0].takes.len(), 3);
    assert_eq!(decoded.scenes[0].edit_timeline.len(), 3);
    assert!(decoded.scenes[0]
        .shots
        .iter()
        .all(|shot| !shot.intent.source_path.is_empty()));
}

#[test]
fn compiled_lighting_and_exposure_retain_authored_intent() {
    let project = load_project(example("dialogue.yaml")).unwrap();
    let compiled = compile_project(&project, &CompileOptions::default())
        .unwrap()
        .compiled;
    let scene = &compiled.scenes[0];
    // One track per authored light source, carrying the authored values.
    let authored: Vec<_> = project.scenes[0]
        .lighting_setups
        .iter()
        .flat_map(|setup| {
            setup.sources.iter().map(move |source| {
                (
                    setup.id.clone(),
                    source.id.clone(),
                    source.kind,
                    source.role,
                    source.intensity_lux,
                    source.color_temperature_k,
                )
            })
        })
        .collect();
    assert_eq!(scene.light_tracks.len(), authored.len());
    for (track, (setup_id, source_id, kind, role, intensity_lux, temperature_k)) in
        scene.light_tracks.iter().zip(authored)
    {
        assert_eq!(
            (track.setup_id.as_str(), track.source_id.as_str()),
            (setup_id.as_str(), source_id.as_str()),
        );
        assert_eq!((track.kind, track.role), (kind, role));
        assert_eq!(
            track.frames.len() as u64,
            scene.duration_frames,
            "{} must span the whole scene",
            source_id
        );
        for (index, frame) in track.frames.iter().enumerate() {
            assert_eq!(frame.frame, index as u64, "sequential frame numbering");
            assert_eq!(frame.intensity_lux, intensity_lux);
            assert_eq!(frame.color_temperature_k, temperature_k);
        }
    }
    let shot = scene
        .shots
        .iter()
        .find(|shot| shot.intent.lighting.is_some())
        .unwrap();
    assert_eq!(
        shot.exposure_track.frames.len() as u64,
        shot.edit_range.duration()
    );
    assert_eq!(
        shot.exposure_track.frames[0].white_balance_k,
        shot.intent.camera.initial_state.exposure.white_balance_k
    );
}

#[test]
fn overlapping_takes_compile_through_an_explicit_edit_timeline() {
    let source = r#"
schema_version: "0.1"
id: coverage
title: Coverage
frame_rate: { numerator: 24 }
coordinate_system: { units: centimeters, handedness: right, up_axis: y, forward_axis: negative_z }
scenes:
  - id: scene
    title: Scene
    duration_frames: 48
    subjects:
      - { id: hero, name: Hero, kind: character }
    takes:
      - id: master_take
        performance_range: { start: 0, end: 48 }
        purpose: [observe]
        coverage_role: master
        framing: { shot_size: medium, subject_ids: [hero] }
        camera_program:
          initial_state:
            transform: { position: { x: -100, y: 150, z: 500 } }
      - id: close_take
        performance_range: { start: 0, end: 48 }
        purpose: [emphasize]
        coverage_role: close_up
        framing: { shot_size: close_up, subject_ids: [hero] }
        camera_program:
          initial_state:
            transform: { position: { x: 100, y: 150, z: 300 } }
    edit_timeline:
      - id: master_clip
        take_id: master_take
        source_range: { start: 0, end: 24 }
        edit_range: { start: 0, end: 24 }
      - id: close_clip
        take_id: close_take
        source_range: { start: 24, end: 48 }
        edit_range: { start: 24, end: 48 }
"#;
    let project: cinematography_ir::CineProject = serde_yaml::from_str(source).unwrap();
    let output = compile_project(&project, &CompileOptions::default()).unwrap();
    assert!(
        !output.report.has_errors(),
        "{:#?}",
        output.report.diagnostics
    );
    let scene = &output.compiled.scenes[0];
    assert_eq!(scene.shots.len(), 2);
    assert_eq!(scene.shots[0].take_id, "master_take");
    assert_eq!(scene.shots[1].take_id, "close_take");
    assert_eq!(scene.shots[1].source_range.start, 24);
    assert!((scene.shots[0].camera_track.frames[0].position.x + 1.0).abs() < 1e-5);
    assert!((scene.shots[1].camera_track.frames[0].position.x - 1.0).abs() < 1e-5);
}
