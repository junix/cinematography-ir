//! Closed-loop measurement (ADR-1115 D11): compares an externally estimated
//! camera trajectory — e.g. recovered from a generated video — against the
//! solved intent. The estimator is out of scope; this is the comparison half
//! that makes arrows-vs-no-arrows or depth-vs-beauty an A/B with numbers.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::math::Vec3;
use crate::model::Frame;
use crate::solve::{SolvedScene, SolvedShot};

/// Minimal trajectory an estimator must emit. Frames are scene-local and
/// matched to the solved shot by `id` and `frame`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EstimatedTrajectory {
    #[serde(default)]
    pub source: Option<String>,
    pub shots: Vec<EstimatedShot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EstimatedShot {
    pub id: String,
    pub frames: Vec<EstimatedFrame>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EstimatedFrame {
    pub frame: Frame,
    pub position: Vec3,
    #[serde(default)]
    pub forward: Option<Vec3>,
    #[serde(default)]
    pub focal_length_mm: Option<f32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Alignment {
    /// Compare raw coordinates.
    None,
    /// Remove the centroid offset (estimators rarely share the world origin).
    #[default]
    Translation,
    /// Remove centroid offset and uniform scale (monocular estimators have
    /// no metric scale). Rotation is not aligned.
    Similarity,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ShotComparison {
    pub shot_id: String,
    pub frames_compared: usize,
    pub position_rmse_m: f32,
    pub position_max_m: f32,
    /// Mean angle between solved and estimated forward vectors, degrees.
    #[serde(default)]
    pub forward_mean_error_deg: Option<f32>,
    #[serde(default)]
    pub focal_mean_abs_error_mm: Option<f32>,
    /// Mean cosine between per-frame displacement vectors: 1 = same motion
    /// direction, −1 = opposite, ~0 = unrelated. `None` when nothing moved.
    #[serde(default)]
    pub motion_direction_agreement: Option<f32>,
    /// Estimated path length divided by solved path length, before scale
    /// alignment. `None` when the solved camera does not move.
    #[serde(default)]
    pub path_length_ratio: Option<f32>,
    /// Scale factor applied to the estimate under `Similarity` alignment.
    #[serde(default)]
    pub applied_scale: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ComparisonReport {
    pub scene_id: String,
    pub alignment: Alignment,
    pub shots: Vec<ShotComparison>,
    /// Frame-weighted RMSE over all compared shots; `None` when no frame
    /// matched (never a misleading 0.0).
    #[serde(default)]
    pub overall_position_rmse_m: Option<f32>,
    /// Solved shots the estimate did not cover.
    pub missing_shots: Vec<String>,
    /// Estimated shots that do not exist in the solved scene.
    pub unmatched_shots: Vec<String>,
}

fn centroid(points: &[Vec3]) -> Vec3 {
    if points.is_empty() {
        return Vec3::ZERO;
    }
    let sum = points.iter().fold(Vec3::ZERO, |acc, p| acc + *p);
    sum * (1.0 / points.len() as f32)
}

fn rms_radius(points: &[Vec3], center: Vec3) -> f32 {
    if points.is_empty() {
        return 0.0;
    }
    let sum: f32 = points.iter().map(|p| (*p - center).dot(*p - center)).sum();
    (sum / points.len() as f32).sqrt()
}

fn path_length(points: &[Vec3]) -> f32 {
    points.windows(2).map(|w| (w[1] - w[0]).length()).sum()
}

fn compare_shot(
    solved: &SolvedShot,
    estimated: &EstimatedShot,
    alignment: Alignment,
) -> Option<ShotComparison> {
    let mut pairs: Vec<(&crate::solve::SolvedCameraFrame, &EstimatedFrame)> = estimated
        .frames
        .iter()
        .filter_map(|e| {
            let index = e.frame.checked_sub(solved.range.start)? as usize;
            let s = solved.frames.get(index)?;
            (s.frame == e.frame).then_some((s, e))
        })
        .collect();
    pairs.sort_by_key(|(s, _)| s.frame);
    pairs.dedup_by_key(|(s, _)| s.frame);
    if pairs.is_empty() {
        return None;
    }

    let solved_points: Vec<Vec3> = pairs.iter().map(|(s, _)| s.position).collect();
    let estimated_points: Vec<Vec3> = pairs.iter().map(|(_, e)| e.position).collect();
    let solved_length = path_length(&solved_points);
    let estimated_length = path_length(&estimated_points);
    let path_length_ratio = (solved_length > 1e-6).then(|| estimated_length / solved_length);

    let (offset_s, offset_e, scale) = match alignment {
        Alignment::None => (Vec3::ZERO, Vec3::ZERO, 1.0),
        Alignment::Translation => (centroid(&solved_points), centroid(&estimated_points), 1.0),
        Alignment::Similarity => {
            let cs = centroid(&solved_points);
            let ce = centroid(&estimated_points);
            let rs = rms_radius(&solved_points, cs);
            let re = rms_radius(&estimated_points, ce);
            let scale = if re > 1e-6 && rs > 1e-6 { rs / re } else { 1.0 };
            (cs, ce, scale)
        }
    };
    let aligned: Vec<Vec3> = estimated_points
        .iter()
        .map(|p| (*p - offset_e) * scale + offset_s)
        .collect();

    let mut sum_sq = 0.0f32;
    let mut max = 0.0f32;
    for (s, e) in solved_points.iter().zip(&aligned) {
        let d = (*s - *e).length();
        sum_sq += d * d;
        max = max.max(d);
    }
    let position_rmse_m = (sum_sq / pairs.len() as f32).sqrt();

    let forward_errors: Vec<f32> = pairs
        .iter()
        .filter_map(|(s, e)| {
            let f = e.forward?.normalized()?;
            Some(s.forward.dot(f).clamp(-1.0, 1.0).acos().to_degrees())
        })
        .collect();
    let forward_mean_error_deg = (!forward_errors.is_empty())
        .then(|| forward_errors.iter().sum::<f32>() / forward_errors.len() as f32);

    let focal_errors: Vec<f32> = pairs
        .iter()
        .filter_map(|(s, e)| e.focal_length_mm.map(|f| (f - s.focal_length_mm).abs()))
        .collect();
    let focal_mean_abs_error_mm = (!focal_errors.is_empty())
        .then(|| focal_errors.iter().sum::<f32>() / focal_errors.len() as f32);

    let mut cosines = Vec::new();
    for i in 1..pairs.len() {
        let ds = solved_points[i] - solved_points[i - 1];
        let de = aligned[i] - aligned[i - 1];
        if let (Some(a), Some(b)) = (ds.normalized(), de.normalized()) {
            cosines.push(a.dot(b));
        }
    }
    let motion_direction_agreement =
        (!cosines.is_empty()).then(|| cosines.iter().sum::<f32>() / cosines.len() as f32);

    Some(ShotComparison {
        shot_id: solved.id.clone(),
        frames_compared: pairs.len(),
        position_rmse_m,
        position_max_m: max,
        forward_mean_error_deg,
        focal_mean_abs_error_mm,
        motion_direction_agreement,
        path_length_ratio,
        applied_scale: (alignment == Alignment::Similarity).then_some(scale),
    })
}

pub fn compare_trajectories(
    scene: &SolvedScene,
    estimated: &EstimatedTrajectory,
    alignment: Alignment,
) -> ComparisonReport {
    let mut shots = Vec::new();
    let mut missing = Vec::new();
    for solved in &scene.shots {
        match estimated.shots.iter().find(|e| e.id == solved.id) {
            Some(e) => match compare_shot(solved, e, alignment) {
                Some(comparison) => shots.push(comparison),
                None => missing.push(solved.id.clone()),
            },
            None => missing.push(solved.id.clone()),
        }
    }
    let unmatched: Vec<String> = estimated
        .shots
        .iter()
        .filter(|e| scene.shot(&e.id).is_none())
        .map(|e| e.id.clone())
        .collect();
    let total_frames: usize = shots.iter().map(|s| s.frames_compared).sum();
    let overall = (total_frames > 0).then(|| {
        (shots
            .iter()
            .map(|s| s.position_rmse_m * s.position_rmse_m * s.frames_compared as f32)
            .sum::<f32>()
            / total_frames as f32)
            .sqrt()
    });
    ComparisonReport {
        scene_id: scene.id.clone(),
        alignment,
        shots,
        overall_position_rmse_m: overall,
        missing_shots: missing,
        unmatched_shots: unmatched,
    }
}

/// Converts a solved scene into the estimate format — handy for producing
/// ground-truth fixtures and for estimator authors.
pub fn trajectory_from_solved(scene: &SolvedScene) -> EstimatedTrajectory {
    EstimatedTrajectory {
        source: Some(format!("solved:{}", scene.id)),
        shots: scene
            .shots
            .iter()
            .map(|shot| EstimatedShot {
                id: shot.id.clone(),
                frames: shot
                    .frames
                    .iter()
                    .map(|f| EstimatedFrame {
                        frame: f.frame,
                        position: f.position,
                        forward: Some(f.forward),
                        focal_length_mm: Some(f.focal_length_mm),
                    })
                    .collect(),
            })
            .collect(),
    }
}

/// Human-readable table.
pub fn render_report(report: &ComparisonReport) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "scene {} — alignment {:?} — overall position RMSE {}",
        report.scene_id,
        report.alignment,
        report
            .overall_position_rmse_m
            .map_or("- (no frames compared)".to_owned(), |v| format!("{v:.3} m"))
    );
    let _ = writeln!(
        out,
        "{:<22} {:>6} {:>9} {:>8} {:>9} {:>9} {:>9} {:>8}",
        "shot", "frames", "rmse[m]", "max[m]", "fwd[deg]", "focal[mm]", "motion", "len%"
    );
    for s in &report.shots {
        let opt = |v: Option<f32>, p: usize| v.map_or("-".to_owned(), |v| format!("{v:.*}", p));
        let _ = writeln!(
            out,
            "{:<22} {:>6} {:>9.3} {:>8.3} {:>9} {:>9} {:>9} {:>8}",
            s.shot_id,
            s.frames_compared,
            s.position_rmse_m,
            s.position_max_m,
            opt(s.forward_mean_error_deg, 2),
            opt(s.focal_mean_abs_error_mm, 2),
            opt(s.motion_direction_agreement, 3),
            opt(s.path_length_ratio.map(|r| r * 100.0), 1),
        );
    }
    if !report.missing_shots.is_empty() {
        let _ = writeln!(
            out,
            "missing in estimate: {}",
            report.missing_shots.join(", ")
        );
    }
    if !report.unmatched_shots.is_empty() {
        let _ = writeln!(
            out,
            "not in solved scene: {}",
            report.unmatched_shots.join(", ")
        );
    }
    out
}
