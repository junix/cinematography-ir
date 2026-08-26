use std::path::PathBuf;

use cinematography_ir::compare::{
    compare_trajectories, trajectory_from_solved, Alignment, EstimatedShot, EstimatedTrajectory,
};
use cinematography_ir::math::Vec3;
use cinematography_ir::{load_project, solve_project, SolveOptions, SolvedProject};

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
