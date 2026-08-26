use std::path::PathBuf;

use cinematography_ir::{emit_project_prompts, load_project, PromptDialect, PromptOptions};

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(name)
}

fn prompts(name: &str, options: &PromptOptions) -> Vec<(String, String)> {
    let project = load_project(example(name)).unwrap();
    emit_project_prompts(&project, &PromptDialect::generic(), options, None, None)
        .into_iter()
        .map(|p| (p.shot_id, p.text))
        .collect()
}

/// Raw IR identifiers must never leak: CamelCase enum variants or
/// snake_case tokens (`medium_close_up`, `ease_in_out`).
fn assert_no_raw_identifiers(text: &str) {
    let camel = text
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|word| word.len() > 3)
        .find(|word| {
            let humps = word
                .chars()
                .skip(1)
                .filter(|c| c.is_ascii_uppercase())
                .count();
            word.chars().next().is_some_and(|c| c.is_ascii_uppercase()) && humps >= 1
        });
    assert!(
        camel.is_none(),
        "CamelCase identifier leaked: {camel:?} in {text}"
    );
    let snake = text.split_whitespace().find(|word| {
        word.contains('_') && word.chars().all(|c| c.is_ascii_lowercase() || c == '_')
    });
    assert!(
        snake.is_none(),
        "snake_case identifier leaked: {snake:?} in {text}"
    );
}

#[test]
fn every_example_shot_gets_a_clean_prompt() {
    for name in [
        "dialogue.yaml",
        "intentional_axis_cross.yaml",
        "unsafe_axis_cross.yaml",
        "dolly_zoom.yaml",
    ] {
        let all = prompts(name, &PromptOptions::default());
        assert!(!all.is_empty(), "{name}");
        for (shot, text) in &all {
            assert!(!text.trim().is_empty(), "{name}/{shot}");
            assert_no_raw_identifiers(text);
        }
    }
}

#[test]
fn prompts_are_deterministic() {
    assert_eq!(
        prompts("dialogue.yaml", &PromptOptions::default()),
        prompts("dialogue.yaml", &PromptOptions::default())
    );
}

#[test]
fn dialogue_master_prompt_binds_colours_and_carries_free_text() {
    let all = prompts("dialogue.yaml", &PromptOptions::default());
    let (_, master) = all.iter().find(|(id, _)| id == "master_two_shot").unwrap();
    assert!(master.contains("Alice (red)"), "{master}");
    assert!(master.contains("Bob (blue)"), "{master}");
    assert!(master.contains("Table (green)"), "{master}");
    assert!(
        master.contains("Master shot: medium long shot, frontal angle, at eye level."),
        "{master}"
    );
    assert!(
        master.contains("坐着，保持克制"),
        "blocking action text must reach the prompt: {master}"
    );
    assert!(master.contains("looking at Bob (blue)"), "{master}");
    assert!(master.contains("40 mm lens, f/4.0"), "{master}");
    assert!(master.contains("Symmetrical composition"), "{master}");
    assert!(master.contains("Lighting: 冷色顶灯与弱补光"), "{master}");
    assert!(
        master.contains("Intent: establish the space, orient the audience."),
        "{master}"
    );
    assert!(
        master.contains("Beat: 建立人物位置与桌面的权力分隔."),
        "{master}"
    );
    assert!(
        master.contains("Camera on a tripod; no camera movement."),
        "{master}"
    );

    let (_, close) = all.iter().find(|(id, _)| id == "alice_close_up").unwrap();
    assert!(
        close.contains("dolly in, pushing toward the subject (0.35 m over 2.3s)"),
        "{close}"
    );
    assert!(close.contains("shallow depth of field"), "{close}");
    assert!(close.contains("negative space on the right"), "{close}");
    assert!(
        !close.contains("Bob (blue):"),
        "unframed subjects get no blocking line: {close}"
    );
}

#[test]
fn dolly_zoom_prompt_names_the_move_and_target() {
    let all = prompts("dolly_zoom.yaml", &PromptOptions::default());
    let (_, text) = &all[0];
    assert!(text.contains("dolly zoom on Hero (blue)"), "{text}");
    assert!(text.contains("vertigo effect"), "{text}");
    assert!(text.contains("background: Corridor (red)"), "{text}");
    assert!(text.contains("Notes: 执行器需要联立求解"), "{text}");
}

#[test]
fn options_switch_off_bindings_measurements_and_context() {
    let options = PromptOptions {
        include_measurements: false,
        include_scene_context: false,
        include_color_binding: false,
    };
    let all = prompts("dialogue.yaml", &options);
    let (_, close) = all.iter().find(|(id, _)| id == "alice_close_up").unwrap();
    assert!(!close.contains("(red)"), "{close}");
    assert!(!close.contains("0.35 m"), "{close}");
    assert!(!close.starts_with("Scene:"), "{close}");
    assert!(close.contains("Alice:"), "{close}");
}

#[test]
fn generic_dialect_is_complete_and_gaps_are_reported() {
    PromptDialect::generic().check_complete().unwrap();

    let mut value: serde_json::Value =
        serde_json::from_str(cinematography_ir::prompt::GENERIC_DIALECT_JSON).unwrap();
    value["shot_size"]
        .as_object_mut()
        .unwrap()
        .remove("close_up");
    value["operation"]
        .as_object_mut()
        .unwrap()
        .remove("dolly_zoom");
    let error = PromptDialect::from_json(&value.to_string()).unwrap_err();
    let message = error.to_string();
    assert!(message.contains("shot_size[close_up]"), "{message}");
    assert!(message.contains("operation[dolly_zoom]"), "{message}");

    let bad = PromptDialect::from_json("{ not json").unwrap_err();
    assert!(bad.to_string().starts_with("invalid dialect JSON"));
}

#[test]
fn filters_select_scene_and_shot() {
    let project = load_project(example("dialogue.yaml")).unwrap();
    let dialect = PromptDialect::generic();
    let one = emit_project_prompts(
        &project,
        &dialect,
        &PromptOptions::default(),
        None,
        Some("bob_close_up"),
    );
    assert_eq!(one.len(), 1);
    assert_eq!(one[0].shot_id, "bob_close_up");
    let none = emit_project_prompts(
        &project,
        &dialect,
        &PromptOptions::default(),
        Some("missing"),
        None,
    );
    assert!(none.is_empty());
}

fn prompt_for(operations: &str, extra_camera: &str) -> String {
    let source = format!(
        r#"
schema_version: "0.1"
id: probe
title: Probe
frame_rate: {{ numerator: 24 }}
scenes:
  - id: s
    title: S
    duration_frames: 120
    subjects:
      - {{ id: hero, name: Hero, kind: character }}
    shots:
      - id: only
        range: {{ start: 0, end: 120 }}
        purpose: [observe]
        framing: {{ shot_size: medium, subject_ids: [hero] }}
        camera:
          rig: dolly
          initial_state:
            transform: {{ position: {{ x: 0.0, y: 1.55, z: 4.5 }} }}
            lens: {{ focal_length_mm: 50.0 }}
{extra_camera}
          operations:
{operations}
"#
    );
    let project: cinematography_ir::CineProject = serde_yaml::from_str(&source).unwrap();
    let options = PromptOptions {
        include_scene_context: false,
        ..PromptOptions::default()
    };
    emit_project_prompts(&project, &PromptDialect::generic(), &options, None, None)
        .remove(0)
        .text
}

#[test]
fn overlapping_operations_are_narrated_with_while() {
    let text = prompt_for(
        r#"
            - range: { start: 0, end: 120 }
              operation: { op: follow, target: { type: subject, id: hero }, offset: { x: 0.0, y: 1.5, z: 3.0 } }
            - range: { start: 0, end: 120 }
              operation: { op: handheld_noise, translation_amplitude_m: 0.01, rotation_amplitude_deg: 0.3, frequency_hz: 1.0, seed: 1 }
            - range: { start: 60, end: 120 }
              operation: { op: zoom, to_focal_length_mm: 70.0 }
"#,
        "",
    );
    assert!(text.contains("camera follows Hero (red)"), "{text}");
    assert!(text.contains(", while handheld shake"), "{text}");
    assert!(
        text.contains(", while zoom in"),
        "zoom overlaps the follow: {text}"
    );
    assert!(!text.contains(", then"), "{text}");
}

#[test]
fn zoom_after_dolly_zoom_uses_the_post_move_focal_length() {
    // 50 mm at 4.5 m → ~24 mm at 2.2 m, so a zoom to 40 mm is a zoom IN.
    let text = prompt_for(
        r#"
            - range: { start: 0, end: 60 }
              operation: { op: dolly_zoom, target: { type: subject, id: hero }, to_distance_m: 2.2 }
            - range: { start: 60, end: 120 }
              operation: { op: zoom, to_focal_length_mm: 40.0 }
"#,
        "",
    );
    assert!(text.contains("dolly zoom on Hero (red)"), "{text}");
    assert!(text.contains(", then zoom in (to 40 mm"), "{text}");
}

#[test]
fn lateral_crane_is_a_plain_move_and_vertical_crane_reports_height() {
    let lateral = prompt_for(
        r#"
            - range: { start: 0, end: 48 }
              operation: { op: crane, delta: { x: 2.0, y: 0.0, z: 0.0 } }
"#,
        "",
    );
    assert!(
        lateral.contains("camera moves (2.00 m over 2.0s)"),
        "{lateral}"
    );
    let vertical = prompt_for(
        r#"
            - range: { start: 0, end: 48 }
              operation: { op: crane, delta: { x: 0.0, y: -1.5, z: 1.0 } }
"#,
        "",
    );
    assert!(
        vertical.contains("crane down (1.50 m over 2.0s)"),
        "{vertical}"
    );
}
