use std::path::PathBuf;

use cinematography_ir::compare::{
    compare_compiled_guidance, compare_compiled_guidance_with_temporal, compare_trajectories,
    trajectory_from_solved, Alignment, EstimatedShot, EstimatedSubjectScreenFrame,
    EstimatedTrajectory, TemporalAlignment,
};
use cinematography_ir::math::Vec3;
use cinematography_ir::{
    compile_project, load_project, solve_project, CompileOptions, ConstraintStatus, SolveOptions,
    SolvedProject,
};

fn solved(name: &str) -> SolvedProject {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(name);
    let project = load_project(path).unwrap();
    solve_project(&project, &SolveOptions::default())
        .unwrap()
        .solved
}

#[test]
fn identical_trajectories_have_zero_error() {
    let solved = solved("dialogue.yaml");
    let scene = &solved.scenes[0];
    let estimate = trajectory_from_solved(scene);
    let report = compare_trajectories(scene, &estimate, Alignment::None);
    assert_eq!(report.shots.len(), 3);
    assert!(report.missing_shots.is_empty() && report.unmatched_shots.is_empty());
    assert!(report.overall_position_rmse_m.unwrap() < 1e-6);
    for shot in &report.shots {
        assert!(shot.position_rmse_m < 1e-6, "{shot:?}");
        assert!(shot.forward_mean_error_deg.unwrap() < 1e-2);
        assert_eq!(shot.focal_mean_abs_error_mm, Some(0.0));
    }
    let moving = report
        .shots
        .iter()
        .find(|s| s.shot_id == "alice_close_up")
        .unwrap();
    assert!((moving.motion_direction_agreement.unwrap() - 1.0).abs() < 1e-4);
    assert!((moving.path_length_ratio.unwrap() - 1.0).abs() < 1e-4);
    let locked = report
        .shots
        .iter()
        .find(|s| s.shot_id == "master_two_shot")
        .unwrap();
    assert_eq!(
        locked.motion_direction_agreement, None,
        "a locked camera has no motion direction"
    );
    assert_eq!(locked.path_length_ratio, None);
}

#[test]
fn translation_and_scale_are_removed_by_alignment() {
    let solved = solved("dolly_zoom.yaml");
    let scene = &solved.scenes[0];
    let mut estimate = trajectory_from_solved(scene);
    for frame in &mut estimate.shots[0].frames {
        frame.position = (frame.position + Vec3::new(10.0, -3.0, 7.0)) * 2.5;
        frame.forward = None;
        frame.focal_length_mm = None;
    }
    let raw = compare_trajectories(scene, &estimate, Alignment::None);
    assert!(raw.shots[0].position_rmse_m > 10.0);
    let translated = compare_trajectories(scene, &estimate, Alignment::Translation);
    assert!(
        translated.shots[0].position_rmse_m > 1.0,
        "scale still differs"
    );
    let similar = compare_trajectories(scene, &estimate, Alignment::Similarity);
    assert!(
        similar.shots[0].position_rmse_m < 1e-3,
        "{:?}",
        similar.shots[0]
    );
    assert!((similar.shots[0].applied_scale.unwrap() - 0.4).abs() < 1e-3);
    assert!((similar.shots[0].path_length_ratio.unwrap() - 2.5).abs() < 1e-3);
    assert_eq!(similar.shots[0].forward_mean_error_deg, None);
    assert_eq!(similar.shots[0].focal_mean_abs_error_mm, None);
}

#[test]
fn reversed_motion_scores_negative_agreement() {
    let solved = solved("dolly_zoom.yaml");
    let scene = &solved.scenes[0];
    let mut estimate = trajectory_from_solved(scene);
    let positions: Vec<Vec3> = estimate.shots[0]
        .frames
        .iter()
        .map(|f| f.position)
        .collect();
    for (frame, position) in estimate.shots[0]
        .frames
        .iter_mut()
        .zip(positions.iter().rev())
    {
        frame.position = *position;
    }
    let report = compare_trajectories(scene, &estimate, Alignment::Translation);
    let agreement = report.shots[0].motion_direction_agreement.unwrap();
    assert!(agreement < -0.99, "{agreement}");
}

#[test]
fn missing_and_unmatched_shots_are_reported() {
    let solved = solved("dialogue.yaml");
    let scene = &solved.scenes[0];
    let mut estimate = trajectory_from_solved(scene);
    estimate.shots.remove(0);
    estimate.shots.push(EstimatedShot {
        id: "ghost".to_owned(),
        frames: Vec::new(),
    });
    let report = compare_trajectories(scene, &estimate, Alignment::Translation);
    assert_eq!(report.missing_shots, vec!["master_two_shot".to_owned()]);
    assert_eq!(report.unmatched_shots, vec!["ghost".to_owned()]);
    assert_eq!(report.shots.len(), 2);

    // Nothing matched at all: no fake zero.
    estimate.shots.clear();
    let empty = compare_trajectories(scene, &estimate, Alignment::Translation);
    assert_eq!(empty.overall_position_rmse_m, None);
    assert!(cinematography_ir::compare::render_report(&empty).contains("no frames compared"));

    let json = serde_json::to_string(&estimate).unwrap();
    let back: EstimatedTrajectory = serde_json::from_str(&json).unwrap();
    assert_eq!(back, estimate);
}

#[test]
fn similarity_alignment_removes_global_rotation() {
    let solved = solved("dolly_zoom.yaml");
    let scene = &solved.scenes[0];
    let mut estimate = trajectory_from_solved(scene);
    let rotate = |value: Vec3| Vec3::new(-value.z, value.y, value.x);
    for frame in &mut estimate.shots[0].frames {
        frame.position = rotate(frame.position) * 2.0 + Vec3::new(8.0, -3.0, 4.0);
        frame.forward = frame.forward.map(rotate);
        frame.up = frame.up.map(rotate);
    }
    let report = compare_trajectories(scene, &estimate, Alignment::Similarity);
    assert!(
        report.shots[0].position_rmse_m < 1e-3,
        "{:?}",
        report.shots[0]
    );
    assert!(report.shots[0].applied_rotation_deg.unwrap() > 80.0);
    assert!(report.shots[0].horizon_mean_error_deg.unwrap() < 1e-2);
}

#[test]
fn compiled_compare_accepts_exact_jaws_screen_tracks_and_rejects_bbox_drift() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("jaws_beach_dolly_zoom.yaml");
    let project = load_project(path).unwrap();
    let output = compile_project(&project, &CompileOptions::default()).unwrap();
    let scene = &output.compiled.scenes[0];
    let solved_scene = &output.solved.scenes[0];
    let mut estimate = trajectory_from_solved(solved_scene);
    for (estimated_shot, compiled_shot) in estimate.shots.iter_mut().zip(&scene.shots) {
        for frame in &mut estimated_shot.frames {
            frame.subjects = compiled_shot
                .screen_tracks
                .iter()
                .filter_map(|track| {
                    let value = track
                        .frames
                        .iter()
                        .find(|value| value.frame == frame.frame)?;
                    Some(EstimatedSubjectScreenFrame {
                        subject_id: track.subject_id.clone(),
                        bbox: value.bbox,
                        visible_fraction: Some(value.visible_fraction),
                        depth_m: value.depth_m,
                        focus_score: value.focus_score,
                    })
                })
                .collect();
        }
    }
    let exact = compare_compiled_guidance(scene, &estimate, Alignment::None);
    let observer_lock = exact.shots[0]
        .constraints
        .iter()
        .find(|constraint| constraint.constraint_id.contains("bbox_height"))
        .unwrap();
    assert_eq!(observer_lock.status, ConstraintStatus::Pass);
    assert!(exact.shots[0].perceptual.bbox_height_max_error.unwrap() < 1e-6);

    for frame in &mut estimate.shots[0].frames[48..168] {
        let observer = frame
            .subjects
            .iter_mut()
            .find(|subject| subject.subject_id == "observer")
            .unwrap();
        observer.bbox.max_y += 0.08;
    }
    let drifted = compare_compiled_guidance(scene, &estimate, Alignment::None);
    let observer_lock = drifted.shots[0]
        .constraints
        .iter()
        .find(|constraint| constraint.constraint_id.contains("bbox_height"))
        .unwrap();
    assert_eq!(observer_lock.status, ConstraintStatus::Fail);
}

#[test]
fn compiled_compare_can_dtw_align_a_time_warp_without_hiding_timing_metrics() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("dolly_zoom.yaml");
    let project = load_project(path).unwrap();
    let output = compile_project(&project, &CompileOptions::default()).unwrap();
    let scene = &output.compiled.scenes[0];
    let mut estimate = trajectory_from_solved(&output.solved.scenes[0]);
    let original = estimate.shots[0].frames.clone();
    let count = original.len();
    for (index, frame) in estimate.shots[0].frames.iter_mut().enumerate() {
        let t = index as f32 / (count - 1).max(1) as f32;
        let source = ((t * t) * (count - 1) as f32).round() as usize;
        let declared_frame = frame.frame;
        *frame = original[source.min(count - 1)].clone();
        frame.frame = declared_frame;
    }
    let exact = compare_compiled_guidance_with_temporal(
        scene,
        &estimate,
        Alignment::Translation,
        TemporalAlignment::Exact,
    );
    let warped = compare_compiled_guidance_with_temporal(
        scene,
        &estimate,
        Alignment::Translation,
        TemporalAlignment::DynamicTimeWarping,
    );
    let exact_error = exact.shots[0].trajectory.as_ref().unwrap().position_rmse_m;
    let warped_error = warped.shots[0].trajectory.as_ref().unwrap().position_rmse_m;
    assert!(
        warped_error < exact_error * 0.2,
        "{exact_error} -> {warped_error}"
    );
    assert_eq!(
        warped.temporal_alignment,
        TemporalAlignment::DynamicTimeWarping
    );
    assert!(
        !warped.shots[0]
            .perceptual
            .phase_timing_error_frames
            .is_empty(),
        "phase timing must be measured on the original, unwarped estimate"
    );
}
