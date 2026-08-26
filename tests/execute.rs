use std::path::{Path, PathBuf};
use std::process::Command;

use cinematography_ir::execute::blender::{
    build_payload, render_script, BlenderAdapter, BlenderOptions,
};
use cinematography_ir::{
    load_project, solve_project, CinematographyAdapter, ConditioningPass, FrameRange, SolveOptions,
    SolvedProject,
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
    let payload = build_payload(&solved, &solved.scenes[0]);
    assert!(payload.render.depth_near_m < 2.2 && payload.render.depth_near_m >= 0.1);
    assert!(
        payload.render.depth_far_m > 8.5,
        "{}",
        payload.render.depth_far_m
    );
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
    assert!(!a.contains("node.base_path"));
    assert!(!a.contains("node.file_slots"));
    assert!(!a.contains("CompositorNodeMapRange"));
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
fn missing_blender_reports_the_written_scripts() {
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
        out.join("realization/build.py").is_file(),
        "script must exist even when launch fails"
    );
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
}

#[test]
fn pass_names_round_trip() {
    for pass in ConditioningPass::ALL {
        assert_eq!(ConditioningPass::parse(pass.name()), Some(pass));
    }
    assert!(ConditioningPass::parse_list("depth,bogus").is_err());
    assert!(ConditioningPass::parse_list("").is_err());
    assert_eq!(
        ConditioningPass::parse_list("depth, depth ,seg").unwrap(),
        vec![ConditioningPass::Depth, ConditioningPass::ObjectIndex]
    );
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
