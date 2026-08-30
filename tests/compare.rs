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
    let rendered = cinematography_ir::compare::render_report(&report);
    assert!(
        rendered.contains("missing in estimate: master_two_shot"),
        "{rendered}"
    );
    assert!(
        rendered.contains("not in solved scene: ghost"),
        "{rendered}"
    );

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

#[test]
fn out_of_range_and_duplicate_estimate_frames_are_dropped_not_fatal() {
    let solved = solved("dialogue.yaml");
    let scene = &solved.scenes[0];

    // First half of the estimate declares frame numbers far outside the
    // shot's range: those frames must be dropped, the rest still compared.
    let mut estimate = trajectory_from_solved(scene);
    let close = estimate
        .shots
        .iter_mut()
        .find(|s| s.id == "alice_close_up")
        .unwrap();
    for frame in &mut close.frames[..40] {
        frame.frame += 1000;
    }
    let report = compare_trajectories(scene, &estimate, Alignment::None);
    let close = report
        .shots
        .iter()
        .find(|s| s.shot_id == "alice_close_up")
        .unwrap();
    assert_eq!(close.frames_compared, 40);
    assert!(close.position_rmse_m < 1e-6, "{close:?}");
    assert!(!report.missing_shots.contains(&"alice_close_up".to_owned()));

    // Duplicated declarations of a solved frame keep the first estimate for
    // that frame; the +5 m clones appended afterwards must be ignored.
    let mut estimate = trajectory_from_solved(scene);
    let close = estimate
        .shots
        .iter_mut()
        .find(|s| s.id == "alice_close_up")
        .unwrap();
    let clones: Vec<_> = close.frames[..40]
        .iter()
        .map(|frame| {
            let mut clone = frame.clone();
            clone.position = clone.position + Vec3::new(5.0, 0.0, 0.0);
            clone
        })
        .collect();
    close.frames.extend(clones);
    let report = compare_trajectories(scene, &estimate, Alignment::None);
    let close = report
        .shots
        .iter()
        .find(|s| s.shot_id == "alice_close_up")
        .unwrap();
    assert_eq!(close.frames_compared, 80);
    assert!(close.position_rmse_m < 1e-6, "{close:?}");
}

#[test]
fn a_shot_whose_frames_never_match_is_reported_missing() {
    let solved = solved("dialogue.yaml");
    let scene = &solved.scenes[0];
    let mut estimate = trajectory_from_solved(scene);
    let close = estimate
        .shots
        .iter_mut()
        .find(|s| s.id == "alice_close_up")
        .unwrap();
    for frame in &mut close.frames {
        frame.frame += 100_000;
    }
    let report = compare_trajectories(scene, &estimate, Alignment::None);
    assert_eq!(report.shots.len(), 2);
    assert_eq!(report.missing_shots, vec!["alice_close_up".to_owned()]);
    assert!(
        report.unmatched_shots.is_empty(),
        "the shot id exists; only its frame numbers never matched"
    );
    let rendered = cinematography_ir::compare::render_report(&report);
    assert!(
        rendered.contains("missing in estimate: alice_close_up"),
        "{rendered}"
    );
}

#[test]
fn per_channel_errors_carry_exact_known_values() {
    let solved = solved("dialogue.yaml");
    let scene = &solved.scenes[0];
    let mut estimate = trajectory_from_solved(scene);
    let close = estimate
        .shots
        .iter_mut()
        .find(|s| s.id == "alice_close_up")
        .unwrap();
    for frame in &mut close.frames {
        let forward = frame.forward.unwrap();
        // Up replaced by the true forward: 90° of horizon error.
        frame.up = Some(forward);
        // Forward flipped: 180° of viewing-direction error.
        frame.forward = Some(-forward);
        // Focal overstated by exactly 2 mm on every frame.
        frame.focal_length_mm = frame.focal_length_mm.map(|f| f + 2.0);
    }
    let report = compare_trajectories(scene, &estimate, Alignment::None);
    let close = report
        .shots
        .iter()
        .find(|s| s.shot_id == "alice_close_up")
        .unwrap();
    assert!(
        (close.forward_mean_error_deg.unwrap() - 180.0).abs() < 1e-2,
        "{close:?}"
    );
    assert!(
        (close.focal_mean_abs_error_mm.unwrap() - 2.0).abs() < 1e-4,
        "{close:?}"
    );
    assert!(
        (close.horizon_mean_error_deg.unwrap() - 90.0).abs() < 0.5,
        "{close:?}"
    );
    // Positions are untouched: the trajectory channel stays exact, and the
    // flipped forward must not leak into the motion-direction agreement.
    assert!(close.position_rmse_m < 1e-6);
    assert!(close.motion_direction_agreement.unwrap() > 0.99);

    // A constant per-frame drift of 0.1 m in x shows up as exactly 0.1 m of
    // relative-motion RMSE while the absolute position error grows.
    let mut estimate = trajectory_from_solved(scene);
    let close = estimate
        .shots
        .iter_mut()
        .find(|s| s.id == "alice_close_up")
        .unwrap();
    for (index, frame) in close.frames.iter_mut().enumerate() {
        frame.position.x += 0.1 * index as f32;
    }
    let report = compare_trajectories(scene, &estimate, Alignment::None);
    let close = report
        .shots
        .iter()
        .find(|s| s.shot_id == "alice_close_up")
        .unwrap();
    assert!(
        (close.relative_motion_rmse_m.unwrap() - 0.1).abs() < 2e-3,
        "{close:?}"
    );
    assert!(close.position_rmse_m > 0.5);
}

#[test]
fn a_single_matched_frame_degenerates_gracefully_under_similarity() {
    let solved = solved("dialogue.yaml");
    let scene = &solved.scenes[0];
    let mut estimate = trajectory_from_solved(scene);
    let close = estimate
        .shots
        .iter_mut()
        .find(|s| s.id == "alice_close_up")
        .unwrap();
    close.frames.truncate(1);
    let report = compare_trajectories(scene, &estimate, Alignment::Similarity);
    let close = report
        .shots
        .iter()
        .find(|s| s.shot_id == "alice_close_up")
        .unwrap();
    assert_eq!(close.frames_compared, 1);
    assert!(close.position_rmse_m < 1e-6, "{close:?}");
    // One point has no radius: the scale guard falls back to 1.0 and the
    // orientation-only rotation stays at identity.
    assert_eq!(close.applied_scale, Some(1.0));
    assert!(close.applied_rotation_deg.unwrap() < 1e-2);
    // Derived-from-windows channels have nothing to compute.
    assert_eq!(close.relative_motion_rmse_m, None);
    assert_eq!(close.motion_direction_agreement, None);
    assert_eq!(close.path_length_ratio, None);
    // Channels that only need one frame are still measured.
    assert!(close.forward_mean_error_deg.unwrap() < 1e-2);
    assert_eq!(close.focal_mean_abs_error_mm, Some(0.0));
}

#[test]
fn compiled_compare_marks_unobservable_constraints_indeterminate() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("jaws_beach_dolly_zoom.yaml");
    let project = load_project(path).unwrap();
    let output = compile_project(&project, &CompileOptions::default()).unwrap();
    let scene = &output.compiled.scenes[0];
    // trajectory_from_solved provides camera channels but no screen-space
    // subjects at all.
    let estimate = trajectory_from_solved(&output.solved.scenes[0]);
    let report = compare_compiled_guidance(scene, &estimate, Alignment::None);
    let shot = &report.shots[0];
    let bbox = shot
        .constraints
        .iter()
        .find(|constraint| constraint.constraint_id.contains("subject_bbox_height"))
        .unwrap();
    assert_eq!(bbox.status, ConstraintStatus::Indeterminate);
    assert_eq!(bbox.max_error, None);
    assert_eq!(
        bbox.detail,
        "estimate did not provide the required observable channel"
    );
    let separation = shot
        .constraints
        .iter()
        .find(|constraint| {
            constraint
                .constraint_id
                .contains("background_pair_separation")
        })
        .unwrap();
    assert_eq!(separation.status, ConstraintStatus::Indeterminate);
    assert_eq!(separation.max_error, None);
    // Perceptual metrics that need subjects are absent, not fake zeros.
    assert_eq!(shot.perceptual.bbox_height_mean_error, None);
    assert_eq!(shot.perceptual.center_drift_max, None);
    assert_eq!(shot.perceptual.pair_separation_mean_error, None);
    // The trajectory channel is still fully measured.
    assert!(shot.trajectory.as_ref().unwrap().position_rmse_m < 1e-6);
}

#[test]
fn cut_timing_errors_are_exact_and_shared_across_shots() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("dialogue.yaml");
    let project = load_project(path).unwrap();
    let output = compile_project(&project, &CompileOptions::default()).unwrap();
    let scene = &output.compiled.scenes[0];
    let truth = trajectory_from_solved(&output.solved.scenes[0]);

    let exact = compare_compiled_guidance(scene, &truth, Alignment::None);
    assert_eq!(exact.shots.len(), 3);
    for shot in &exact.shots {
        assert_eq!(
            shot.perceptual.cut_timing_error_frames,
            vec![0, 0],
            "{}",
            shot.shot_id
        );
    }

    let mut estimate = trajectory_from_solved(&output.solved.scenes[0]);
    for cut in &mut estimate.cuts {
        *cut += 2;
    }
    let shifted = compare_compiled_guidance(scene, &estimate, Alignment::None);
    for shot in &shifted.shots {
        assert_eq!(
            shot.perceptual.cut_timing_error_frames,
            vec![2, 2],
            "{}",
            shot.shot_id
        );
    }
    // The moving shot reports its phase boundaries too. The dolly starts on
    // the Motion phase boundary (frame 96); motion is observed on the frame
    // it happens on, so the entry boundary reads +1 and the exit reads 0.
    let moving = exact
        .shots
        .iter()
        .find(|shot| shot.shot_id == "alice_close_up")
        .unwrap();
    assert_eq!(
        moving.perceptual.phase_timing_error_frames,
        vec![1, 0],
        "{:?}",
        moving.perceptual.phase_timing_error_frames
    );
}

#[test]
fn estimate_schema_rejects_unknown_fields() {
    let error = serde_json::from_str::<EstimatedTrajectory>(
        r#"{"shots": [], "cuts": [], "estimator_version": "1.0"}"#,
    )
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("unknown field") && error.contains("estimator_version"),
        "the error must name the rejected field: {error}"
    );

    // `source` and `cuts` default when absent.
    let minimal: EstimatedTrajectory = serde_json::from_str(r#"{"shots": []}"#).unwrap();
    assert_eq!(minimal.source, None);
    assert!(minimal.cuts.is_empty());
}
