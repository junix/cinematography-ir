use std::path::{Path, PathBuf};
use std::process::Command;

use cinematography_ir::execute::blender::{
    build_payload, render_script, BlenderAdapter, BlenderOptions,
};
use cinematography_ir::{
    compile_project, load_project, solve_project, CinematographyAdapter, CompileOptions,
    ConditioningPass, ExecutionProfile, ExecutionRequest, FrameRange, SolveOptions, SolvedProject,
};

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(name)
}

fn solved(name: &str) -> SolvedProject {
    let project = load_project(example(name)).unwrap();
    solve_project(&project, &SolveOptions::default())
        .unwrap()
        .solved
}

fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Columns of a row-major 3×3 stored at `values[offset..offset+9]`.
fn columns(values: &[f32], offset: usize) -> [[f32; 3]; 3] {
    let r = &values[offset..offset + 9];
    [[r[0], r[3], r[6]], [r[1], r[4], r[7]], [r[2], r[5], r[8]]]
}

#[test]
fn payload_converts_cameras_and_subjects_into_blender_space() {
    let solved = solved("dialogue.yaml");
    let scene = &solved.scenes[0];
    let payload = build_payload(&solved, scene);

    assert_eq!(payload.subjects.len(), 3);
    assert_eq!(payload.shots.len(), 3);
    assert_eq!(payload.frame_end, 239);
    for (index, subject) in payload.subjects.iter().enumerate() {
        assert_eq!(subject.pass_index as usize, index + 1);
        assert_eq!(subject.frames.len(), 240);
        for frame in &subject.frames {
            let [x, y, z] = columns(frame, 3);
            for axis in [x, y, z] {
                assert!((dot(axis, axis) - 1.0).abs() < 1e-4, "unit axis");
            }
            assert!(dot(x, y).abs() < 1e-4 && dot(y, z).abs() < 1e-4 && dot(x, z).abs() < 1e-4);
            // IR width/height/depth → Blender scale (x, z, y): all 1.0 here.
            assert_eq!(&frame[12..15], &[1.0, 1.0, 1.0]);
        }
    }
    // Alice at IR (-1.5, 0, 0) facing +X (yaw −90 in a −Z-forward system):
    let alice = payload.subjects.iter().find(|s| s.id == "alice").unwrap();
    let first = &alice.frames[0];
    assert_eq!(&first[0..3], &[-1.5, 0.0, 0.0]);
    let [_, forward, up] = columns(first, 3);
    assert!(
        (forward[0] - 1.0).abs() < 1e-4,
        "Alice faces Blender +X: {forward:?}"
    );
    assert!((up[2] - 1.0).abs() < 1e-4, "up is Blender +Z");
    // Height 1.7 becomes Blender Z size; depth 0.4 becomes Y.
    assert_eq!(alice.dimensions, [0.5, 0.4, 1.7]);
    assert_eq!(alice.color_name, "red");

    // Master camera: IR (0, 1.45, 5.2) looking −Z ⇒ Blender (0, −5.2, 1.45) looking +Y.
    let master = payload
        .shots
        .iter()
        .find(|s| s.id == "master_two_shot")
        .unwrap();
    assert_eq!(master.frame_start, 0);
    assert_eq!(master.frame_end, 79);
    let cam = &master.frames[0];
    assert!(
        (cam[0]).abs() < 1e-4 && (cam[1] + 5.2).abs() < 1e-4 && (cam[2] - 1.45).abs() < 1e-4,
        "{cam:?}"
    );
    let [right, up, minus_forward] = columns(cam, 3);
    assert!((right[0] - 1.0).abs() < 1e-4, "camera right = +X");
    assert!((up[2] - 1.0).abs() < 1e-4, "camera up = +Z");
    assert!(
        (minus_forward[1] + 1.0).abs() < 1e-4,
        "camera −Z column = −forward = −Y"
    );
    assert_eq!(cam[12], 40.0, "focal length");
    assert!((cam[13] - 5.2).abs() < 1e-3, "focus distance");
}

#[test]
fn depth_bounds_bracket_all_subject_distances() {
    let solved = solved("dolly_zoom.yaml");
    let scene = &solved.scenes[0];
    let payload = build_payload(&solved, scene);
    assert!(payload.render.depth_near_m < 2.2 && payload.render.depth_near_m >= 0.1);
    assert!(
        payload.render.depth_far_m > 8.5,
        "{}",
        payload.render.depth_far_m
    );

    // The bounds come from this scene's own subject distances: every sampled
    // distance must fall inside the bracket, at exactly min/2 (floored at
    // 0.1 m) and 1.5·max + 2 m respectively.
    let mut min = f32::INFINITY;
    let mut max: f32 = 0.0;
    for shot in &scene.shots {
        for frame in &shot.frames {
            for track in &scene.subjects {
                let d = (track.transform_at(frame.frame).position - frame.position).length();
                assert!(
                    payload.render.depth_near_m <= d && d <= payload.render.depth_far_m,
                    "distance {d} at frame {} outside [{}, {}]",
                    frame.frame,
                    payload.render.depth_near_m,
                    payload.render.depth_far_m
                );
                min = min.min(d);
                max = max.max(d);
            }
        }
    }
    let near = (min * 0.5).max(0.1);
    let far = (max * 1.5 + 2.0).max(near + 1.0);
    assert!((payload.render.depth_near_m - near).abs() < 1e-4, "{near}");
    assert!((payload.render.depth_far_m - far).abs() < 1e-4, "{far}");
}

#[test]
fn script_is_deterministic_and_embeds_the_payload() {
    let solved = solved("dialogue.yaml");
    let payload = build_payload(&solved, &solved.scenes[0]);
    let a = render_script(&payload);
    let b = render_script(&payload);
    assert_eq!(a, b);
    assert!(a.contains("json.loads('"));
    assert!(a.contains("\"scene_id\":\"interrogation\""));
    assert!(a.contains("cam_"), "camera objects are created per shot");
    assert!(
        a.contains("timeline_markers.new"),
        "shots switch cameras via markers"
    );
    assert!(
        a.contains("for channelbag in strip.channelbags"),
        "layered Blender actions expose f-curves through channel bags"
    );
    assert!(a.contains("scene.compositing_node_group = tree"));
    assert!(a.contains("NodeGroupOutput"));
    assert!(a.contains("node.file_output_items.new"));
    assert!(a.contains("ShaderNodeMapRange"));
    assert!(a.contains("ShaderNodeMath"));
    assert!(!a.contains("node.base_path"));
    assert!(!a.contains("node.file_slots"));
    assert!(!a.contains("CompositorNodeMapRange"));
    assert!(!a.contains("CompositorNodeMath"));
    assert!(!a.contains("tree = scene.node_tree"));
    assert!(!a.contains(".use_nodes = True"));
    assert!(!a.contains("__PAYLOAD_JSON__"));
}

#[test]
fn script_only_writes_build_script_and_manifest() {
    let solved = solved("dialogue.yaml");
    let out = scratch("blender-script-only");
    let mut adapter = BlenderAdapter::new(BlenderOptions {
        script_only: true,
        ..BlenderOptions::default()
    });
    adapter.apply_scene(&solved, &solved.scenes[0]).unwrap();
    let passes = ConditioningPass::parse_list("depth,id,openpose,normal,beauty,vector").unwrap();
    let output = adapter
        .render_passes(
            Some(FrameRange {
                start: 80,
                end: 160,
            }),
            &passes,
            &out,
        )
        .unwrap();

    assert_eq!(output.scenes.len(), 1);
    let scene = &output.scenes[0];
    assert!(!scene.executed);
    assert!(scene.script_path.ends_with("interrogation/build.py"));
    assert!(scene.manifest_path.is_file());
    assert!(output.runtime.is_none());

    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&scene.manifest_path).unwrap()).unwrap();
    assert_eq!(manifest["frame_range"], serde_json::json!([80, 159]));
    assert_eq!(manifest["subjects"][0]["color_hex"], "#dc322f");
    assert_eq!(manifest["shots"].as_array().unwrap().len(), 3);
    assert_eq!(manifest["passes"].as_array().unwrap().len(), 6);

    let script = std::fs::read_to_string(&scene.script_path).unwrap();
    assert!(script.contains("\"frame_range\":[80,159]"));
    assert!(script.contains("\"output_dir\":\""));
    py_compile(&scene.script_path);

    // Exercise the whole script against a recording bpy stub: control flow,
    // slicing, and data plumbing run for real; only Blender calls are mocked.
    if let Some(summary) = run_with_stub(&scene.script_path) {
        assert_eq!(summary["cameras_new"], 3, "one camera per shot: {summary}");
        assert_eq!(summary["markers_new"], 3, "one marker per shot: {summary}");
        assert_eq!(
            summary["view_layers_new"], 3,
            "normal + id + openpose layers: {summary}"
        );
        assert_eq!(summary["render_calls"], 1, "{summary}");
        assert_eq!(summary["compositor_groups"], 1, "{summary}");
        assert_eq!(summary["file_output_items"], 6, "{summary}");
        assert_eq!(
            summary["objects_new"],
            3 + 3 + 2,
            "empties + cameras + suns: {summary}"
        );
        assert!(
            summary["primitives"].as_u64().unwrap() > 40,
            "two clay characters + a prop + floor: {summary}"
        );
        assert!(
            summary["compositor_nodes"].as_u64().unwrap() >= 11,
            "{summary}"
        );
        assert!(
            scene.out_dir.join("done.json").is_file(),
            "script wrote its completion marker"
        );
    }
}

#[test]
fn compiled_execution_publishes_typed_scene_and_shot_bundles() {
    let project = load_project(example("jaws_beach_dolly_zoom.yaml")).unwrap();
    let compiled = compile_project(&project, &CompileOptions::default()).unwrap();
    let scene = &compiled.compiled.scenes[0];
    let out = scratch("compiled-shot-bundle");
    let profile = ExecutionProfile::generic_control();
    let mut adapter = BlenderAdapter::new(BlenderOptions {
        script_only: true,
        ..BlenderOptions::default()
    });
    adapter
        .stage_scene(&compiled.compiled, scene, &profile)
        .unwrap();
    let bundle = adapter
        .execute(
            &ExecutionRequest {
                scene_id: scene.id.clone(),
                shot_ids: Vec::new(),
                frame_range: None,
                profile,
            },
            &out,
        )
        .unwrap();
    assert!(!bundle.executed);
    assert!(bundle.complete_path.is_file());
    assert!(bundle.root.join("edit.json").is_file());
    let shot = bundle.root.join("shots/beach_dolly_zoom");
    for relative in [
        "guidance.json",
        "prompt.txt",
        "negative_prompt.txt",
        "review/motion_grammar.svg",
        "review/elevation.svg",
        "metadata/camera.json",
        "metadata/screen_tracks.json",
        "metadata/constraints.json",
        "metadata/validation.json",
        "manifest.json",
    ] {
        assert!(shot.join(relative).is_file(), "missing {relative}");
    }
    let guidance: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(shot.join("guidance.json")).unwrap())
            .unwrap();
    assert_eq!(guidance["phases"].as_array().unwrap().len(), 3);
    assert!(guidance["screen_tracks"].as_array().unwrap().len() >= 3);
    assert!(guidance["constraints"].as_array().unwrap().len() >= 10);
    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&bundle.manifest_path).unwrap()).unwrap();
    assert_eq!(manifest["status"], "complete");
    assert!(manifest["pass_specs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|spec| {
            spec["kind"] == "metric_depth" && spec["file_pattern"] == "depth_metric/####.exr"
        }));
    let script = bundle.root.join("build.py");
    py_compile(&script);
    if let Some(summary) = run_with_stub(&script) {
        assert_eq!(summary["cameras_new"], 1, "{summary}");
        assert_eq!(summary["render_calls"], 1, "{summary}");
    }
}

/// Runs the script under `tests/python/bpy_stub`; `None` when python3 is absent.
fn run_with_stub(script: &Path) -> Option<serde_json::Value> {
    let runner = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("python")
        .join("run_build_with_stub.py");
    let output = Command::new("python3")
        .arg(&runner)
        .arg(script)
        .output()
        .ok()?;
    assert!(
        output.status.success(),
        "stubbed run failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    let last = text.lines().last().unwrap_or("");
    Some(serde_json::from_str(last).expect("summary JSON"))
}

#[test]
fn missing_blender_preserves_diagnostics_without_publishing_a_manifest() {
    let solved = solved("dolly_zoom.yaml");
    let out = scratch("blender-missing");
    let mut adapter = BlenderAdapter::new(BlenderOptions {
        blender_path: Some(PathBuf::from("/nonexistent/blender-binary")),
        ..BlenderOptions::default()
    });
    adapter.apply_scene(&solved, &solved.scenes[0]).unwrap();
    let error = adapter
        .render_passes(None, &[ConditioningPass::Depth], &out)
        .unwrap_err();
    let message = error.to_string();
    assert!(message.contains("failed to launch"), "{message}");
    assert!(
        !out.join("realization").exists(),
        "failed transactions must not look like valid scene bundles"
    );
    let failed = std::fs::read_dir(&out)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(".realization.failed-"))
        })
        .expect("failed transaction is retained for diagnostics");
    assert!(failed.join("build.py").is_file());
    assert!(!failed.join("manifest.json").exists());
}

#[test]
fn frame_ranges_outside_the_scene_are_rejected() {
    let solved = solved("dolly_zoom.yaml");
    let out = scratch("blender-bad-range");
    let mut adapter = BlenderAdapter::new(BlenderOptions {
        script_only: true,
        ..BlenderOptions::default()
    });
    adapter.apply_scene(&solved, &solved.scenes[0]).unwrap();
    let error = adapter
        .render_passes(
            Some(FrameRange {
                start: 300,
                end: 400,
            }),
            &[ConditioningPass::Depth],
            &out,
        )
        .unwrap_err();
    assert!(error.to_string().contains("does not intersect"), "{error}");
    // A partially overlapping range is clamped, not rejected.
    let output = adapter
        .render_passes(
            Some(FrameRange {
                start: 100,
                end: 400,
            }),
            &[ConditioningPass::Depth],
            &out,
        )
        .unwrap();
    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&output.scenes[0].manifest_path).unwrap())
            .unwrap();
    assert_eq!(manifest["frame_range"], serde_json::json!([100, 119]));

    // No requested range: the whole scene renders, first to last frame.
    let output = adapter
        .render_passes(None, &[ConditioningPass::Depth], &out)
        .unwrap();
    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&output.scenes[0].manifest_path).unwrap())
            .unwrap();
    assert_eq!(manifest["frame_range"], serde_json::json!([0, 119]));
}

#[test]
fn pass_names_round_trip() {
    for pass in ConditioningPass::ALL {
        assert_eq!(ConditioningPass::parse(pass.name()), Some(pass));
    }
    // Names are matched case-insensitively, with whitespace and aliases.
    assert_eq!(
        ConditioningPass::parse(" Depth "),
        Some(ConditioningPass::Depth)
    );
    assert_eq!(
        ConditioningPass::parse("METRIC_DEPTH"),
        Some(ConditioningPass::DepthMetric)
    );
    assert_eq!(
        ConditioningPass::parse_list("depth,bogus").unwrap_err(),
        "unknown pass 'bogus'",
        "the error must name the rejected pass"
    );
    assert_eq!(
        ConditioningPass::parse_list("").unwrap_err(),
        "at least one pass is required"
    );
    assert_eq!(
        ConditioningPass::parse_list("depth, depth ,seg").unwrap(),
        vec![ConditioningPass::Depth, ConditioningPass::ObjectIndex]
    );
    // Aliases of the same pass collapse to one entry, keeping first-seen order.
    assert_eq!(
        ConditioningPass::parse_list("z,flow, depth ,openpose,pose").unwrap(),
        vec![
            ConditioningPass::Depth,
            ConditioningPass::Vector,
            ConditioningPass::OpenPose
        ]
    );
}

#[test]
fn checked_in_execution_profiles_are_strict_and_adapter_compatible() {
    for name in [
        "generic-dense.json",
        "image-to-video.json",
        "camera-api.json",
    ] {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("profiles/execution")
            .join(name);
        let profile: ExecutionProfile =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(!profile.passes.is_empty(), "{name}");
        // Each pass must own a distinct, non-empty file pattern: two passes
        // writing the same pattern would silently overwrite each other.
        let mut patterns: Vec<&str> = profile
            .passes
            .iter()
            .map(|pass| pass.file_pattern.as_str())
            .collect();
        assert!(
            patterns.iter().all(|pattern| !pattern.is_empty()),
            "{name}: empty file pattern"
        );
        let unique = |values: &mut Vec<&str>| {
            values.sort_unstable();
            let before = values.len();
            values.dedup();
            before == values.len()
        };
        assert!(unique(&mut patterns), "{name}: duplicate file patterns");
        let mut kinds: Vec<&str> = profile.passes.iter().map(|pass| pass.kind.name()).collect();
        assert!(unique(&mut kinds), "{name}: duplicate pass kinds");
        let adapter = BlenderAdapter::default();
        let capabilities = adapter.capabilities();
        assert!(
            profile
                .passes
                .iter()
                .all(|pass| capabilities.supported_passes.contains(&pass.kind)),
            "{name} requests an unsupported pass"
        );
    }
}

#[test]
fn dof_requires_an_explicit_or_derived_focus_distance() {
    let source = r#"
schema_version: "0.1"
id: no_focus
title: No focus
frame_rate: { numerator: 24 }
scenes:
  - id: scene
    title: Scene
    duration_frames: 1
    shots:
      - id: shot
        range: { start: 0, end: 1 }
        framing: { shot_size: medium }
        camera: { initial_state: { transform: {} } }
"#;
    let project: cinematography_ir::CineProject = serde_yaml::from_str(source).unwrap();
    let solved = solve_project(&project, &SolveOptions::default())
        .unwrap()
        .solved;
    let mut adapter = BlenderAdapter::new(BlenderOptions {
        use_dof: true,
        ..BlenderOptions::default()
    });
    let error = adapter.apply_scene(&solved, &solved.scenes[0]).unwrap_err();
    assert!(error
        .to_string()
        .contains("has no focus target or distance"));
}

/// Syntax-checks the generated script with the system Python (bpy is only
/// imported, never executed, by `py_compile`). Skips when python3 is absent.
fn py_compile(script: &Path) {
    let status = Command::new("python3")
        .arg("-m")
        .arg("py_compile")
        .arg(script)
        .status();
    match status {
        Ok(status) => assert!(status.success(), "generated script failed py_compile"),
        Err(_) => eprintln!("python3 not available; skipping py_compile"),
    }
}
