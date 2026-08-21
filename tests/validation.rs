use std::path::PathBuf;

use cinematography_ir::{load_project, validate_project, CineProject, Severity};

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
    let project =
        load_project(example("intentional_axis_cross.yaml")).expect("example must parse");
    let report = validate_project(&project);
    assert!(
        !report
            .diagnostics
            .iter()
            .any(|item| item.code == "CONTINUITY_AXIS_CROSS")
    );
    assert!(!report.has_errors());
}

#[test]
fn unsafe_axis_cross_is_reported() {
    let project = load_project(example("unsafe_axis_cross.yaml")).expect("example must parse");
    let report = validate_project(&project);

    assert!(
        report
            .diagnostics
            .iter()
            .any(|item| item.code == "CONTINUITY_AXIS_CROSS")
    );
    assert!(
        report
            .diagnostics
            .iter()
            .any(|item| item.code == "CONTINUITY_EYELINE_MISMATCH")
    );
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
    let result = serde_yaml::from_str::<CineProject>(source);
    assert!(result.is_err());
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
