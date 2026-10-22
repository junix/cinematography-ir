use std::path::PathBuf;

use cinematography_ir::compiled::{ConstraintStatus, PhaseKind, ShotConstraintKind, Trend};
use cinematography_ir::{
    compile_project, load_project, CompileOptions, CompiledProject, Prohibition, ShotPhrase,
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
    // The authored background depth layer contributes exactly one separation
    // constraint: its first two subjects, locked to an increasing trend.
    let separations: Vec<&ShotConstraintKind> = shot
        .constraints
        .iter()
        .map(|constraint| &constraint.kind)
        .filter(|kind| matches!(kind, ShotConstraintKind::PairScreenSeparation { .. }))
        .collect();
    assert_eq!(
        separations,
        vec![&ShotConstraintKind::PairScreenSeparation {
            subject_ids: ["left_beachgoer".to_owned(), "right_beachgoer".to_owned()],
            trend: Trend::Increasing,
        }]
    );
    // The shot authors no extra prohibitions: the dolly zoom alone contributes
    // its four guards, in BTreeSet (declaration) order.
    assert_eq!(
        shot.prohibitions,
        vec![
            Prohibition::ActorTranslationAsCameraSubstitute,
            Prohibition::DigitalCropAsDollySubstitute,
            Prohibition::HandheldMotion,
            Prohibition::UnplannedCut,
        ]
    );
    // Likewise the shot authors no phrases: the dolly zoom derives exactly one.
    assert_eq!(shot.intent.phrases, vec![ShotPhrase::SubjectLockDollyZoom]);
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

/// Compiles a two-prop scene viewed head-on: `plane_hit` sits with its
/// nearest corner exactly on the authored 3.85 m focus plane, `deep` is 4 m
/// behind it. Returns the compiled shot.
fn compile_coc_scene(aperture_f: f32) -> CompiledProject {
    let source = format!(
        r#"
schema_version: "0.1"
id: cocscreen
title: CoC screen
frame_rate: {{ numerator: 24 }}
scenes:
  - id: s
    title: S
    duration_frames: 4
    subjects:
      - id: plane_hit
        name: PlaneHit
        kind: prop
        dimensions_m: {{ x: 0.3, y: 0.3, z: 0.3 }}
        initial_transform: {{ position: {{ x: 0.0, y: 0.0, z: -4.0 }} }}
      - id: deep
        name: Deep
        kind: prop
        dimensions_m: {{ x: 0.3, y: 0.3, z: 0.3 }}
        initial_transform: {{ position: {{ x: 0.0, y: 0.0, z: -8.0 }} }}
    shots:
      - id: only
        range: {{ start: 0, end: 4 }}
        framing: {{ shot_size: medium, subject_ids: [plane_hit] }}
        camera:
          initial_state:
            transform: {{ position: {{ x: 0.0, y: 0.0, z: 0.0 }} }}
            lens: {{ focal_length_mm: 50.0, aperture_f: {aperture_f} }}
            focus: {{ distance_m: 3.85 }}
"#
    );
    let project: cinematography_ir::CineProject = serde_yaml::from_str(&source).unwrap();
    compile_project(&project, &CompileOptions::default())
        .unwrap()
        .compiled
}

#[test]
fn screen_tracks_carry_the_thin_lens_coc() {
    use cinematography_ir::solve::circle_of_confusion_m;
    let compiled = compile_coc_scene(2.8);
    let shot = &compiled.scenes[0].shots[0];

    for track in &shot.screen_tracks {
        for frame in &track.frames {
            // The per-subject CoC is exactly the shared thin-lens derivation
            // (lemo-opuscar core/three/post.js:20) applied to the frame's
            // own measured depth.
            assert_eq!(
                frame.coc_m,
                circle_of_confusion_m(50.0, 2.8, 3.85, frame.depth_m.unwrap()),
                "{} at frame {}",
                track.subject_id,
                frame.frame
            );
        }
    }

    // Zero at the focus plane (nearest corner depth 4 - 0.15 = 3.85 m).
    let on_plane = shot.screen_track("plane_hit").unwrap().frames[0].clone();
    assert!(on_plane.coc_m.unwrap().abs() < 1e-9, "{on_plane:?}");
    assert!(on_plane.focus_coc_score.unwrap() > 0.999_999);

    // Behind the plane: positive CoC, hyperbolic in 1/focus - 1/depth, and
    // a strictly lower CoC-based score.
    let deep = shot.screen_track("deep").unwrap().frames[0].clone();
    let coc = deep.coc_m.unwrap();
    assert!(coc > 0.0, "{coc}");
    let depth = deep.depth_m.unwrap();
    let expected = circle_of_confusion_m(50.0, 2.8, 4.0, depth).unwrap();
    assert!(
        (coc / expected - (1.0 / 3.85 - 1.0 / depth) / (1.0 / 4.0 - 1.0 / depth)).abs() < 1e-4,
        "coc must follow 1/focus - 1/depth, not |depth - focus|"
    );
    assert!(deep.focus_coc_score.unwrap() < on_plane.focus_coc_score.unwrap());

    // The lens track mirrors the solved frame's primary-subject CoC, so
    // adapters read one number per frame without touching screen tracks.
    for (lens, camera) in shot.lens_track.frames.iter().zip(&shot.camera_track.frames) {
        assert_eq!(lens.coc_m, camera.coc_m);
    }
    // The frame-level CoC measures the primary subject's aim point (a prop
    // aims at half height: depth 4.0 m), which sits 15 cm behind the
    // nearest projected corner the screen track measures — two depths of
    // the same subject, by design.
    assert_eq!(
        shot.lens_track.frames[0].coc_m,
        circle_of_confusion_m(50.0, 2.8, 3.85, 4.0)
    );

    // The legacy heuristic stays untouched alongside the CoC fields.
    assert!(on_plane.focus_score.unwrap() > 0.99);
    assert!(deep.focus_score.unwrap() < on_plane.focus_score.unwrap());
}

#[test]
fn coc_based_focus_score_responds_to_the_aperture() {
    let fast = compile_coc_scene(2.8);
    let slow = compile_coc_scene(5.6);
    let frame = |project: &CompiledProject| {
        project.scenes[0].shots[0]
            .screen_track("deep")
            .unwrap()
            .frames[0]
            .clone()
    };
    let (fast, slow) = (frame(&fast), frame(&slow));
    // Halving the f-number doubles the physical aperture, so the blur and
    // its hyperbolic growth double...
    assert!(
        (fast.coc_m.unwrap() - 2.0 * slow.coc_m.unwrap()).abs() < 1e-9,
        "{} vs {}",
        fast.coc_m.unwrap(),
        slow.coc_m.unwrap()
    );
    // ...which the CoC-based score reflects and the legacy depth-only
    // heuristic cannot.
    assert!(fast.focus_coc_score.unwrap() < slow.focus_coc_score.unwrap());
    assert_eq!(fast.focus_score, slow.focus_score);
    assert_eq!(fast.depth_m, slow.depth_m);
}
