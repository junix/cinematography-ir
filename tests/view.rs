use std::path::PathBuf;

use cinematography_ir::view::canvas::{ArrowKind, CanvasItem, GuideKind};
use cinematography_ir::view::{
    build_cells, Encoding, LabelMode, Layout, OverlaySet, PanelKind, Sampling, StripAxis, ViewError,
};
use cinematography_ir::{load_project, render_view, solve_project, SolveOptions, ViewRequest};

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(name)
}

fn snapshot_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("snapshots")
        .join(name)
}

/// Compares against a committed snapshot; set `UPDATE_SNAPSHOTS=1` to rewrite.
fn assert_snapshot(name: &str, actual: &str) {
    let path = snapshot_path(name);
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "missing snapshot {}; run with UPDATE_SNAPSHOTS=1",
            path.display()
        )
    });
    assert!(
        expected == actual,
        "snapshot {} differs\n--- expected ---\n{expected}\n--- actual ---\n{actual}",
        path.display()
    );
}

fn render(name: &str, request: &ViewRequest) -> Vec<cinematography_ir::ViewArtifact> {
    let project = load_project(example(name)).unwrap();
    render_view(&project, request).unwrap()
}

const EXAMPLES: [&str; 5] = [
    "dialogue.yaml",
    "intentional_axis_cross.yaml",
    "unsafe_axis_cross.yaml",
    "dolly_zoom.yaml",
    "jaws_beach_dolly_zoom.yaml",
];

#[test]
fn ascii_plan_snapshots_are_stable() {
    for name in EXAMPLES {
        let request = ViewRequest {
            encoding: Encoding::Ascii { columns: 72 },
            layout: Layout::Strip {
                axis: StripAxis::Vertical,
            },
            ..ViewRequest::human()
        };
        let artifacts = render(name, &request);
        assert_eq!(artifacts.len(), 1);
        let text = artifacts[0].text().unwrap();
        assert!(
            text.lines().all(|l| l.is_ascii()),
            "{name}: ascii output must be pure ASCII"
        );
        assert!(text.contains('C'), "{name}: camera glyph present");
        assert_snapshot(
            &format!("{}.plan.txt", name.trim_end_matches(".yaml")),
            text,
        );
    }
}

#[test]
fn every_encoding_is_deterministic() {
    for encoding in [
        Encoding::Svg,
        Encoding::Png { scale: 1.0 },
        Encoding::Ascii { columns: 72 },
        Encoding::Html,
        Encoding::Markdown { columns: 72 },
    ] {
        let request = ViewRequest {
            encoding: encoding.clone(),
            ..ViewRequest::human()
        };
        let a = render("dialogue.yaml", &request);
        let b = render("dialogue.yaml", &request);
        assert_eq!(a, b, "{encoding:?} must be byte-stable");
        assert_eq!(a.len(), 1);
        assert!(a[0].name.ends_with(encoding.extension()), "{}", a[0].name);
    }
}

#[test]
fn png_is_a_real_image_of_the_expected_size() {
    let request = ViewRequest {
        encoding: Encoding::Png { scale: 1.0 },
        layout: Layout::Separate,
        panels: vec![PanelKind::Frame],
        ..ViewRequest::human()
    };
    let artifacts = render("dolly_zoom.yaml", &request);
    let bytes = &artifacts[0].bytes;
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    let width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    assert_eq!(width, 400, "frame panel width");
    // 400 / (16/9) = 225 plus a 22 px title bar for human intent.
    assert_eq!(height, 225 + 22);
}

#[test]
fn conditioning_intent_carries_no_text_and_only_motion_overlays() {
    let project = load_project(example("dialogue.yaml")).unwrap();
    let solved = solve_project(&project, &SolveOptions::default()).unwrap();
    let scene = &project.scenes[0];
    let request = ViewRequest {
        // Ask for everything: the intent gate must still strip it.
        overlays: OverlaySet::ALL,
        ..ViewRequest::conditioning()
    };
    let built = build_cells(
        &project,
        scene,
        &solved.solved.scenes[0],
        &solved.report.diagnostics,
        &request,
    );
    assert_eq!(built.cells.len(), 3);
    for cell in &built.cells {
        for canvas in [cell.plan.as_ref().unwrap(), cell.frame.as_ref().unwrap()] {
            assert_eq!(
                canvas.text_items(),
                0,
                "conditioning canvases must carry no text"
            );
            assert_eq!(
                canvas.count(|i| matches!(i, CanvasItem::AxisLine { .. })),
                0
            );
            assert_eq!(canvas.count(|i| matches!(i, CanvasItem::Guide { .. })), 0);
            assert_eq!(
                canvas.count(|i| matches!(i, CanvasItem::Diagnostic { .. })),
                0
            );
            assert_eq!(canvas.count(|i| matches!(i, CanvasItem::Marker { .. })), 0);
        }
        let composed = cell.compose(false);
        assert_eq!(
            composed.text_items(),
            0,
            "composed conditioning cell has no title"
        );
    }
    // Per-shot conditioning samples the shot start and therefore must not
    // leak the later dolly operation into the first frame.
    let alice = &built.cells[1];
    assert_eq!(
        alice.frame.as_ref().unwrap().count(|i| matches!(
            i,
            CanvasItem::Polyline {
                kind: ArrowKind::MotionCue,
                ..
            }
        )),
        0,
        "future dolly cue must not appear at shot start"
    );
    assert_eq!(
        alice.plan.as_ref().unwrap().count(|i| matches!(
            i,
            CanvasItem::Polyline {
                kind: ArrowKind::CameraPath,
                ..
            }
        )),
        1,
        "camera path arrow on the plan"
    );
    // color+name under conditioning may label subjects, but nothing else.
    let named = ViewRequest {
        labels: LabelMode::ColorName,
        ..ViewRequest::conditioning()
    };
    let built = build_cells(
        &project,
        scene,
        &solved.solved.scenes[0],
        &solved.report.diagnostics,
        &named,
    );
    let plan = built.cells[0].plan.as_ref().unwrap();
    let labels = plan.count(|i| matches!(i, CanvasItem::Glyph { label: Some(_), .. }));
    assert_eq!(labels, 3, "three named subjects");
    assert_eq!(plan.text_items(), labels, "no text beyond subject names");
    assert_eq!(
        plan.count(|i| matches!(i, CanvasItem::CameraWedge { label: Some(_), .. })),
        0
    );

    // The SVG for a conditioning artifact contains no <text> element at all.
    let svg = render(
        "dialogue.yaml",
        &ViewRequest {
            encoding: Encoding::Svg,
            ..ViewRequest::conditioning()
        },
    );
    for artifact in &svg {
        assert!(
            !artifact.text().unwrap().contains("<text"),
            "{}",
            artifact.name
        );
    }
}

#[test]
fn human_intent_draws_diagnostics_guides_and_labels() {
    let project = load_project(example("unsafe_axis_cross.yaml")).unwrap();
    let solved = solve_project(&project, &SolveOptions::default()).unwrap();
    let scene = &project.scenes[0];
    let request = ViewRequest::human();
    let built = build_cells(
        &project,
        scene,
        &solved.solved.scenes[0],
        &solved.report.diagnostics,
        &request,
    );
    let second = built.cells[1].plan.as_ref().unwrap();
    let codes: Vec<String> = second
        .items
        .iter()
        .filter_map(|i| match i {
            CanvasItem::Diagnostic { code, .. } => Some(code.clone()),
            _ => None,
        })
        .collect();
    assert!(
        codes.contains(&"CONTINUITY_AXIS_CROSS".to_owned()),
        "{codes:?}"
    );
    assert!(
        codes.contains(&"CONTINUITY_EYELINE_MISMATCH".to_owned()),
        "{codes:?}"
    );
    let first = built.cells[0].plan.as_ref().unwrap();
    assert_eq!(
        first.count(|i| matches!(i, CanvasItem::Diagnostic { .. })),
        0,
        "diagnostics attach to the cut's second shot"
    );
    assert!(first.count(|i| matches!(i, CanvasItem::AxisLine { .. })) == 1);

    let project = load_project(example("dialogue.yaml")).unwrap();
    let solved = solve_project(&project, &SolveOptions::default()).unwrap();
    let built = build_cells(
        &project,
        &project.scenes[0],
        &solved.solved.scenes[0],
        &solved.report.diagnostics,
        &request,
    );
    let close_up = built.cells[1].frame.as_ref().unwrap();
    assert!(close_up.items.iter().any(|i| matches!(
        i,
        CanvasItem::Guide {
            kind: GuideKind::Headroom { .. },
            ..
        }
    )));
    assert!(close_up.items.iter().any(|i| matches!(
        i,
        CanvasItem::Guide {
            kind: GuideKind::LeadRoom { .. },
            ..
        }
    )));
    assert!(close_up
        .items
        .iter()
        .any(|i| matches!(i, CanvasItem::Silhouette { label: Some(l), .. } if l == "Alice")));
    // Bob is behind the camera's field of view in Alice's close-up.
    assert!(!close_up
        .items
        .iter()
        .any(|i| matches!(i, CanvasItem::Silhouette { label: Some(l), .. } if l == "Bob")));
}

#[test]
fn grid_and_separate_share_the_same_panels() {
    let project = load_project(example("dialogue.yaml")).unwrap();
    let solved = solve_project(&project, &SolveOptions::default()).unwrap();
    let scene = &project.scenes[0];
    let grid = ViewRequest {
        layout: Layout::Grid { columns: 3 },
        ..ViewRequest::human()
    };
    let separate = ViewRequest {
        layout: Layout::Separate,
        ..ViewRequest::human()
    };
    let a = build_cells(
        &project,
        scene,
        &solved.solved.scenes[0],
        &solved.report.diagnostics,
        &grid,
    );
    let b = build_cells(
        &project,
        scene,
        &solved.solved.scenes[0],
        &solved.report.diagnostics,
        &separate,
    );
    assert_eq!(a.samples, b.samples);
    let panels = |cells: &[cinematography_ir::view::Cell]| -> Vec<_> {
        cells
            .iter()
            .map(|c| (c.plan.clone(), c.frame.clone()))
            .collect()
    };
    assert_eq!(panels(&a.cells), panels(&b.cells));

    // A sheet owns exactly one full-canvas background; panel backgrounds are
    // bounded fills (a regression here blanked every cell but the last).
    let composed: Vec<_> = a.cells.iter().map(|c| c.compose(true)).collect();
    let sheet = cinematography_ir::view::layout::grid(&composed, 3, true);
    assert_eq!(
        sheet.count(|i| matches!(i, CanvasItem::Background { .. })),
        1
    );
    assert_eq!(
        sheet.count(|i| matches!(i, CanvasItem::Glyph { .. })),
        9,
        "3 subjects × 3 cells"
    );
    assert_eq!(
        sheet.count(|i| matches!(i, CanvasItem::CameraWedge { .. })),
        3
    );

    let separate_artifacts = render("dialogue.yaml", &separate);
    assert_eq!(separate_artifacts.len(), 3);
    assert_eq!(
        separate_artifacts[0].name,
        "interrogation_master_two_shot_f0000.svg"
    );
    assert_eq!(
        separate_artifacts[2].name,
        "interrogation_bob_close_up_f0160.svg"
    );
}

#[test]
fn plan_projection_is_coordinate_system_aware() {
    // Rebuild dialogue.yaml in a Z-up / +Y-forward world; the plan panels
    // must contain the same items at the same canvas positions.
    let project = load_project(example("dialogue.yaml")).unwrap();
    let text = serde_yaml::to_string(&project).unwrap();
    let rotated: cinematography_ir::CineProject = {
        let mut value: serde_yaml::Value = serde_yaml::from_str(&text).unwrap();
        // (x, y, z)_yup → (x, -z, y)_zup for every position/delta triple.
        fn convert(v: &mut serde_yaml::Value) {
            match v {
                serde_yaml::Value::Mapping(map) => {
                    let keys: Vec<_> = map.keys().cloned().collect();
                    let is_vec3 = keys.len() == 3
                        && ["x", "y", "z"]
                            .iter()
                            .all(|k| map.contains_key(serde_yaml::Value::from(*k)));
                    if is_vec3 {
                        let y = map[&serde_yaml::Value::from("y")].as_f64().unwrap();
                        let z = map[&serde_yaml::Value::from("z")].as_f64().unwrap();
                        map.insert(serde_yaml::Value::from("y"), serde_yaml::Value::from(-z));
                        map.insert(serde_yaml::Value::from("z"), serde_yaml::Value::from(y));
                    } else {
                        for key in keys {
                            if matches!(key.as_str(), Some("dimensions_m") | Some("scale")) {
                                continue; // subject-local, never rotated
                            }
                            convert(map.get_mut(&key).unwrap());
                        }
                    }
                }
                serde_yaml::Value::Sequence(items) => items.iter_mut().for_each(convert),
                _ => {}
            }
        }
        convert(&mut value);
        value["coordinate_system"]["up_axis"] = "z".into();
        value["coordinate_system"]["forward_axis"] = "positive_y".into();
        serde_yaml::from_value(value).unwrap()
    };
    let request = ViewRequest {
        panels: vec![PanelKind::Plan],
        ..ViewRequest::human()
    };
    let a = render_view(&project, &request).unwrap();
    let b = render_view(&rotated, &request).unwrap();
    assert_eq!(
        a[0].text().unwrap(),
        b[0].text().unwrap(),
        "plan must not depend on which axis is up"
    );
}

#[test]
fn animation_page_follows_the_insight_video_contract() {
    // dolly_zoom: 120 frames @ 24 fps = 5 s, whatever the sampling rate.
    let base = ViewRequest {
        layout: Layout::Animate,
        encoding: Encoding::Html,
        panels: vec![PanelKind::Plan],
        ..ViewRequest::human()
    };
    let six = render(
        "dolly_zoom.yaml",
        &ViewRequest {
            sampling: Sampling::Continuous { fps: 6 },
            ..base.clone()
        },
    );
    let html = six[0].text().unwrap();
    assert!(
        html.contains(r#"data-composition-id="realization""#),
        "{}",
        &html[..300]
    );
    assert!(
        html.contains(r#"data-duration="5""#),
        "duration is scene time, not sample count"
    );
    assert!(
        html.contains(r#"data-frames="30""#),
        "24/6 = every 4th frame"
    );
    assert_eq!(html.matches("class=\"frame").count(), 30);
    assert!(html.contains(r#"data-t="0""#) && html.contains(r#"data-t="0.16666666666666666""#));
    assert!(html.contains("window.__insightVideo={duration:duration,ready:true,seek:function(t)"));
    assert!(!html.contains("infinite"), "no infinite CSS animations");

    // 7 fps does not divide 24: step = round(24/7) = 3 → 40 frames, still 5 s.
    let seven = render(
        "dolly_zoom.yaml",
        &ViewRequest {
            sampling: Sampling::Continuous { fps: 7 },
            ..base.clone()
        },
    );
    let html = seven[0].text().unwrap();
    assert!(html.contains(r#"data-duration="5""#));
    assert!(html.contains(r#"data-frames="40""#));
    assert!(
        html.contains(r#"data-t="0.125""#),
        "second sample at frame 3"
    );

    // Per-shot sampling animates each cell for its shot's duration.
    let shots = render("dialogue.yaml", &ViewRequest { ..base.clone() });
    let html = shots[0].text().unwrap();
    assert!(
        html.contains(r#"data-duration="10""#),
        "240 frames @ 24 fps"
    );
    assert!(
        html.contains("var times=[0,3.3333333333333335,6.666666666666667];"),
        "{html}"
    );

    let bad = ViewRequest {
        encoding: Encoding::Svg,
        ..base
    };
    let project = load_project(example("dolly_zoom.yaml")).unwrap();
    assert!(matches!(
        render_view(&project, &bad),
        Err(ViewError::Unsupported(_))
    ));
}

#[test]
fn markdown_pairs_ascii_with_prompt_text() {
    let request = ViewRequest {
        encoding: Encoding::Markdown { columns: 60 },
        ..ViewRequest::human()
    };
    let artifacts = render("dialogue.yaml", &request);
    let md = artifacts[0].text().unwrap();
    assert!(md.starts_with("# interrogation — 谁拿走了钥匙\n"));
    assert_eq!(md.matches("```text\n").count(), 3);
    assert!(md.contains("## alice_close_up · f80"));
    assert!(
        md.contains("dolly in, pushing toward the subject"),
        "prompt prose follows the diagram"
    );
}

#[test]
fn sampling_modes_select_the_expected_frames() {
    let project = load_project(example("dialogue.yaml")).unwrap();
    let solved = solve_project(&project, &SolveOptions::default()).unwrap();
    let scene = &project.scenes[0];
    let frames = |sampling: Sampling| -> Vec<u64> {
        let request = ViewRequest {
            sampling,
            ..ViewRequest::human()
        };
        build_cells(
            &project,
            scene,
            &solved.solved.scenes[0],
            &solved.report.diagnostics,
            &request,
        )
        .samples
        .iter()
        .map(|s| s.scene_frame)
        .collect()
    };
    assert_eq!(frames(Sampling::PerShot), vec![0, 80, 160]);
    assert_eq!(frames(Sampling::PerBeat), vec![0, 80, 160]);
    assert_eq!(
        frames(Sampling::OperationEndpoints),
        vec![0, 80, 96, 151, 160]
    );
    assert_eq!(frames(Sampling::EveryNFrames(100)), vec![0, 100, 200]);
    assert_eq!(frames(Sampling::Continuous { fps: 12 }).len(), 120);
}

#[test]
fn unknown_scene_is_an_error() {
    let project = load_project(example("dialogue.yaml")).unwrap();
    let request = ViewRequest {
        scene_id: Some("missing".to_owned()),
        ..ViewRequest::human()
    };
    assert!(matches!(
        render_view(&project, &request),
        Err(ViewError::NoScene(_))
    ));
}

/// Frame-panel motion cues for a single-shot probe: (start_x, start_y, dx, dy) per arrow, per cell.
fn motion_cues(
    operations: &str,
    camera_yaw: f32,
    sampling: Sampling,
) -> Vec<Vec<(f32, f32, f32, f32)>> {
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
      - {{ id: hero, name: Hero, kind: character }}
    shots:
      - id: only
        range: {{ start: 0, end: 48 }}
        purpose: [observe]
        framing: {{ shot_size: medium, subject_ids: [hero] }}
        camera:
          initial_state:
            transform: {{ position: {{ x: 0.0, y: 1.575, z: 5.0 }}, rotation_deg: {{ yaw: {camera_yaw} }} }}
            lens: {{ focal_length_mm: 50.0 }}
          operations:
{operations}
"#
    );
    let project: cinematography_ir::CineProject = serde_yaml::from_str(&source).unwrap();
    let solved = solve_project(&project, &SolveOptions::default()).unwrap();
    let request = ViewRequest {
        sampling,
        panels: vec![PanelKind::Frame],
        ..ViewRequest::conditioning()
    };
    let built = build_cells(
        &project,
        &project.scenes[0],
        &solved.solved.scenes[0],
        &[],
        &request,
    );
    built
        .cells
        .iter()
        .map(|cell| {
            cell.frame
                .as_ref()
                .unwrap()
                .items
                .iter()
                .filter_map(|i| match i {
                    CanvasItem::Polyline {
                        kind: ArrowKind::MotionCue,
                        points,
                        ..
                    } => {
                        let a = points[0];
                        let b = points[points.len() - 1];
                        Some((a.x, a.y, b.x - a.x, b.y - a.y))
                    }
                    _ => None,
                })
                .collect()
        })
        .collect()
}

/// Sign of every arrow's motion relative to the frame centre (400×225 panel):
/// +1 when all point inward, −1 when all point outward, 0 otherwise.
fn radial_sense(arrows: &[(f32, f32, f32, f32)]) -> i32 {
    let (cx, cy) = (200.0, 112.5);
    let dots: Vec<f32> = arrows
        .iter()
        .map(|(ax, ay, dx, dy)| (cx - ax) * dx + (cy - ay) * dy)
        .collect();
    if dots.iter().all(|d| *d > 0.0) {
        1
    } else if dots.iter().all(|d| *d < 0.0) {
        -1
    } else {
        0
    }
}

fn is_inward(arrows: &[(f32, f32, f32, f32)]) -> bool {
    arrows.len() == 4 && radial_sense(arrows) == 1
}

fn is_outward(arrows: &[(f32, f32, f32, f32)]) -> bool {
    arrows.len() == 4 && radial_sense(arrows) == -1
}

#[test]
fn motion_cues_follow_the_solver_not_the_sampled_frame() {
    // Per-shot conditioning samples the shot start. An operation beginning
    // later must not leak a future arrow into that frame.
    let cues = motion_cues(
        "            - range: { start: 12, end: 48 }\n              operation: { op: dolly, distance_m: 1.0 }",
        0.0,
        Sampling::PerShot,
    );
    assert!(cues[0].is_empty(), "future operation leaked: {:?}", cues[0]);

    // Camera-local translate along +x with a yaw of 90° is a truck right, not a dolly.
    let cues = motion_cues(
        "            - range: { start: 0, end: 48 }\n              operation: { op: translate, delta: { x: 1.0, y: 0.0, z: 0.0 } }",
        90.0,
        Sampling::PerShot,
    );
    assert_eq!(cues[0].len(), 1, "one horizontal arrow: {:?}", cues[0]);
    assert!(
        cues[0][0].2 > 0.0 && cues[0][0].3.abs() < 1e-3,
        "truck right → arrow right"
    );

    // Camera-local translate along −z (forward in the default system) is a dolly in.
    let cues = motion_cues(
        "            - range: { start: 0, end: 48 }\n              operation: { op: translate, delta: { x: 0.0, y: 0.0, z: -1.0 } }",
        90.0,
        Sampling::PerShot,
    );
    assert!(is_inward(&cues[0]), "{:?}", cues[0]);

    // Zoom out: the cue must read the same on the first and last frame of the op.
    let cues = motion_cues(
        "            - range: { start: 0, end: 48 }\n              operation: { op: zoom, to_focal_length_mm: 24.0 }",
        0.0,
        Sampling::OperationEndpoints,
    );
    assert_eq!(cues.len(), 2);
    for (index, arrows) in cues.iter().enumerate() {
        assert!(
            is_outward(arrows),
            "zoom out is outward on sample {index}: {arrows:?}"
        );
    }

    // Dolly zoom closing in (5 m → 2.5 m) without any focus block: inward corners.
    let cues = motion_cues(
        "            - range: { start: 0, end: 48 }\n              operation: { op: dolly_zoom, target: { type: subject, id: hero }, to_distance_m: 2.5 }",
        0.0,
        Sampling::PerShot,
    );
    let corners: Vec<(f32, f32, f32, f32)> = cues[0]
        .iter()
        .copied()
        .filter(|(_, _, _, dy)| dy.abs() > 1e-3)
        .collect();
    assert!(is_inward(&corners), "{:?}", cues[0]);

    // Dolly-out/zoom-in makes background proxies spread away from the target.
    let project = load_project(example("jaws_beach_dolly_zoom.yaml")).unwrap();
    let solved = solve_project(&project, &SolveOptions::default()).unwrap();
    let request = ViewRequest {
        sampling: Sampling::OperationEndpoints,
        panels: vec![PanelKind::Frame],
        ..ViewRequest::conditioning()
    };
    let built = build_cells(
        &project,
        &project.scenes[0],
        &solved.solved.scenes[0],
        &[],
        &request,
    );
    let start = built
        .cells
        .iter()
        .find(|cell| cell.title.contains("f48"))
        .unwrap();
    let edge_arrows: Vec<_> = start
        .frame
        .as_ref()
        .unwrap()
        .items
        .iter()
        .filter_map(|item| match item {
            CanvasItem::Polyline {
                kind: ArrowKind::MotionCue,
                points,
                ..
            } if points[0].y > 180.0 && points[0].x > 80.0 && points[0].x < 320.0 => {
                Some((points[0], points[points.len() - 1]))
            }
            _ => None,
        })
        .collect();
    assert_eq!(edge_arrows.len(), 2, "{edge_arrows:?}");
    assert!(edge_arrows[0].1.x < edge_arrows[0].0.x);
    assert!(edge_arrows[1].1.x > edge_arrows[1].0.x);

    // Look-at a centred hero (eye height == camera height) draws no arrow.
    let cues = motion_cues(
        "            - range: { start: 0, end: 48 }\n              operation: { op: look_at, target: { type: subject, id: hero } }",
        0.0,
        Sampling::PerShot,
    );
    assert!(cues[0].is_empty(), "target already on axis: {:?}", cues[0]);
}
