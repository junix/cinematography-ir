use std::path::PathBuf;

use cinematography_ir::{
    analyze_continuity, load_project, validate_project, CineProject, Severity,
};

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(name)
}

#[test]
fn dialogue_example_is_clean() {
    let project = load_project(example("dialogue.yaml")).expect("example must parse");
    let report = validate_project(&project);
    assert!(
        report.diagnostics.is_empty(),
        "unexpected diagnostics: {:#?}",
        report.diagnostics
    );
}

#[test]
fn intentional_axis_cross_is_bridged() {
    let project = load_project(example("intentional_axis_cross.yaml")).expect("example must parse");
    let report = validate_project(&project);
    assert!(!report
        .diagnostics
        .iter()
        .any(|item| item.code == "CONTINUITY_AXIS_CROSS"));
    assert!(!report.has_errors());
}

#[test]
fn unsafe_axis_cross_is_reported() {
    let project = load_project(example("unsafe_axis_cross.yaml")).expect("example must parse");
    let report = validate_project(&project);

    assert!(report
        .diagnostics
        .iter()
        .any(|item| item.code == "CONTINUITY_AXIS_CROSS"));
    assert!(report
        .diagnostics
        .iter()
        .any(|item| item.code == "CONTINUITY_EYELINE_MISMATCH"));
    assert!(!report.has_errors());
}

#[test]
fn unknown_fields_are_rejected() {
    let source = r#"
schema_version: "0.1"
id: invalid
title: Invalid field
frame_rate: { numerator: 24 }
focal_lenght_mm: 50
scenes: []
"#;
    let error = serde_yaml::from_str::<CineProject>(source)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("unknown field") && error.contains("focal_lenght_mm"),
        "the error must name the rejected field: {error}"
    );

    // Nested documents reject unknown fields with the same precision.
    let nested = source.replacen("focal_lenght_mm: 50\n", "", 1).replacen(
        "scenes: []",
        "scenes:\n  - id: scene\n    title: Scene\n    duration_frames: 24\n    extra_key: 1",
        1,
    );
    let error = serde_yaml::from_str::<CineProject>(&nested)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("unknown field") && error.contains("extra_key"),
        "{error}"
    );
}

#[test]
fn unknown_subject_reference_is_an_error() {
    let source = r#"
schema_version: "0.1"
id: invalid
title: Invalid reference
frame_rate: { numerator: 24, denominator: 1 }
scenes:
  - id: scene
    title: Invalid reference
    duration_frames: 24
    shots:
      - id: shot
        range: { start: 0, end: 24 }
        purpose: [observe]
        framing:
          shot_size: medium
          subject_ids: [missing]
        camera:
          initial_state:
            transform: {}
            focus:
              target: { type: subject, id: missing }
"#;

    let project: CineProject = serde_yaml::from_str(source).expect("fixture must parse");
    let report = validate_project(&project);
    assert!(report.has_errors());
    assert!(report.diagnostics.iter().any(|item| {
        item.severity == Severity::Error && item.code == "FRAMING_SUBJECT_UNKNOWN"
    }));
    assert!(report
        .diagnostics
        .iter()
        .any(|item| item.code == "TARGET_SUBJECT_UNKNOWN"));
}

#[test]
fn out_of_bounds_camera_operation_is_an_error() {
    let source = r#"
schema_version: "0.1"
id: invalid_operation
title: Invalid operation
frame_rate: { numerator: 24, denominator: 1 }
scenes:
  - id: scene
    title: Invalid operation
    duration_frames: 24
    subjects:
      - { id: actor, name: Actor, kind: character }
    shots:
      - id: shot
        range: { start: 0, end: 24 }
        purpose: [observe]
        framing:
          shot_size: medium
          subject_ids: [actor]
        camera:
          initial_state:
            transform: {}
          operations:
            - range: { start: 0, end: 30 }
              operation:
                op: dolly
                distance_m: 1.0
"#;

    let project: CineProject = serde_yaml::from_str(source).expect("fixture must parse");
    let report = validate_project(&project);
    assert!(report
        .diagnostics
        .iter()
        .any(|item| item.code == "CAMERA_OPERATION_RANGE_INVALID"));
}

#[test]
fn parallel_forward_and_up_axes_are_rejected() {
    let source = r#"
schema_version: "0.1"
id: zup
title: Z-up without a forward axis
frame_rate: { numerator: 24 }
coordinate_system: { up_axis: z }
scenes: []
"#;
    let project: CineProject = serde_yaml::from_str(source).unwrap();
    let report = validate_project(&project);
    assert!(report
        .diagnostics
        .iter()
        .any(|d| d.code == "COORDINATE_SYSTEM_AXES_PARALLEL" && d.severity == Severity::Error));

    let fixed = source.replace("{ up_axis: z }", "{ up_axis: z, forward_axis: positive_y }");
    let project: CineProject = serde_yaml::from_str(&fixed).unwrap();
    assert!(!validate_project(&project)
        .diagnostics
        .iter()
        .any(|d| d.code == "COORDINATE_SYSTEM_AXES_PARALLEL"));
}

#[test]
fn a_previous_jump_cut_does_not_exempt_the_following_cut() {
    let source = r#"
schema_version: "0.1"
id: cuts
title: Cuts
frame_rate: { numerator: 24 }
scenes:
  - id: scene
    title: Scene
    duration_frames: 72
    subjects:
      - { id: hero, name: Hero, kind: character }
    shots:
      - id: first
        range: { start: 0, end: 24 }
        framing: { shot_size: medium, subject_ids: [hero] }
        camera: { initial_state: { transform: {} } }
        continuity: { camera_azimuth_deg: 0 }
      - id: previous
        range: { start: 24, end: 48 }
        framing: { shot_size: medium, subject_ids: [hero] }
        camera: { initial_state: { transform: {} } }
        transition_in: jump_cut
        continuity: { camera_azimuth_deg: 5 }
      - id: next
        range: { start: 48, end: 72 }
        framing: { shot_size: medium, subject_ids: [hero] }
        camera: { initial_state: { transform: {} } }
        transition_in: cut
        continuity: { camera_azimuth_deg: 10 }
"#;
    let project: CineProject = serde_yaml::from_str(source).unwrap();
    let report = analyze_continuity(&project.scenes[0]);
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "CONTINUITY_SMALL_ANGLE_CUT"
            && diagnostic.path.contains("previous->next")
    }));
}
