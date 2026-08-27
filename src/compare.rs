//! Closed-loop measurement (ADR-1115 D11): compares an externally estimated
//! camera trajectory — e.g. recovered from a generated video — against the
//! solved intent. The estimator is out of scope; this is the comparison half
//! that makes arrows-vs-no-arrows or depth-vs-beauty an A/B with numbers.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::compiled::{
    CompiledScene, ConstraintStatus, NormalizedRect, ScreenPoint, ShotConstraintKind,
};
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
    /// Detected edit boundaries in scene-local frames.
    #[serde(default)]
    pub cuts: Vec<Frame>,
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
    pub up: Option<Vec3>,
    #[serde(default)]
    pub focal_length_mm: Option<f32>,
    #[serde(default)]
    pub subjects: Vec<EstimatedSubjectScreenFrame>,
    #[serde(default)]
    pub optical_flow: Option<ScreenPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EstimatedSubjectScreenFrame {
    pub subject_id: String,
    pub bbox: NormalizedRect,
    #[serde(default)]
    pub visible_fraction: Option<f32>,
    #[serde(default)]
    pub depth_m: Option<f32>,
    #[serde(default)]
    pub focus_score: Option<f32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Alignment {
    /// Compare raw coordinates.
    None,
    /// Remove the centroid offset (estimators rarely share the world origin).
    #[default]
    Translation,
    /// Full Sim(3): remove centroid offset, global rotation, and uniform
    /// scale (monocular estimators rarely share world coordinates).
    Similarity,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TemporalAlignment {
    /// Match frames by their declared scene-frame number.
    #[default]
    Exact,
    /// Match camera progress with dynamic time warping. Phase/cut timing is
    /// still reported against the unwarped estimate so timing mistakes remain
    /// visible instead of being hidden by alignment.
    DynamicTimeWarping,
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
    /// Angle of the global rotation removed by Sim(3) alignment.
    #[serde(default)]
    pub applied_rotation_deg: Option<f32>,
    /// RMSE between consecutive relative camera displacements.
    #[serde(default)]
    pub relative_motion_rmse_m: Option<f32>,
    /// Mean angle between solved and estimated camera up vectors.
    #[serde(default)]
    pub horizon_mean_error_deg: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
pub struct ShotPerceptualMetrics {
    pub bbox_height_mean_error: Option<f32>,
    pub bbox_height_max_error: Option<f32>,
    pub center_drift_mean: Option<f32>,
    pub center_drift_max: Option<f32>,
    pub background_scale_ratio: Option<f32>,
    pub pair_separation_mean_error: Option<f32>,
    pub optical_flow_mean_error: Option<f32>,
    pub reveal_visibility_mean_error: Option<f32>,
    pub focus_handoff_error_frames: Option<i64>,
    pub horizon_mean_error_deg: Option<f32>,
    pub phase_timing_error_frames: Vec<i64>,
    pub cut_timing_error_frames: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ConstraintComparison {
    pub constraint_id: String,
    pub status: ConstraintStatus,
    pub tolerance: f32,
    pub max_error: Option<f32>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct CompiledShotComparison {
    pub shot_id: String,
    pub trajectory: Option<ShotComparison>,
    pub perceptual: ShotPerceptualMetrics,
    pub constraints: Vec<ConstraintComparison>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct CompiledComparisonReport {
    pub scene_id: String,
    pub alignment: Alignment,
    pub temporal_alignment: TemporalAlignment,
    pub shots: Vec<CompiledShotComparison>,
    pub missing_shots: Vec<String>,
    pub unmatched_shots: Vec<String>,
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

#[derive(Clone, Copy)]
struct Quaternion {
    w: f32,
    x: f32,
    y: f32,
    z: f32,
}

impl Quaternion {
    const IDENTITY: Self = Self {
        w: 1.0,
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    fn rotate(self, value: Vec3) -> Vec3 {
        let q = Vec3::new(self.x, self.y, self.z);
        let t = cross(q, value) * 2.0;
        value + t * self.w + cross(q, t)
    }

    fn angle_deg(self) -> f32 {
        (2.0 * self.w.abs().clamp(0.0, 1.0).acos()).to_degrees()
    }
}

fn cross(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}

/// Horn's absolute-orientation quaternion, evaluated through a shifted power
/// iteration on the symmetric 4×4 matrix. The shift makes the eigenvalue for
/// the best rotation dominant even for nearly planar camera paths.
fn best_rotation(from: &[Vec3], to: &[Vec3]) -> Quaternion {
    if from.len() < 2 || from.len() != to.len() {
        return Quaternion::IDENTITY;
    }
    let mut s = [[0.0f32; 3]; 3];
    for (from, to) in from.iter().zip(to) {
        let a = [from.x, from.y, from.z];
        let b = [to.x, to.y, to.z];
        for row in 0..3 {
            for column in 0..3 {
                s[row][column] += a[row] * b[column];
            }
        }
    }
    let trace = s[0][0] + s[1][1] + s[2][2];
    let mut n = [[0.0f32; 4]; 4];
    n[0] = [
        trace,
        s[1][2] - s[2][1],
        s[2][0] - s[0][2],
        s[0][1] - s[1][0],
    ];
    n[1] = [
        n[0][1],
        s[0][0] - s[1][1] - s[2][2],
        s[0][1] + s[1][0],
        s[2][0] + s[0][2],
    ];
    n[2] = [
        n[0][2],
        n[1][2],
        -s[0][0] + s[1][1] - s[2][2],
        s[1][2] + s[2][1],
    ];
    n[3] = [n[0][3], n[1][3], n[2][3], -s[0][0] - s[1][1] + s[2][2]];
    let shift = n
        .iter()
        .flat_map(|row| row.iter())
        .map(|value| value.abs())
        .sum::<f32>()
        .max(1.0);
    for (index, row) in n.iter_mut().enumerate() {
        row[index] += shift;
    }
    let mut q = [1.0f32, 0.0, 0.0, 0.0];
    for _ in 0..64 {
        let mut next = [0.0f32; 4];
        for row in 0..4 {
            next[row] = (0..4).map(|column| n[row][column] * q[column]).sum();
        }
        let norm = next.iter().map(|value| value * value).sum::<f32>().sqrt();
        if norm <= 1e-8 {
            return Quaternion::IDENTITY;
        }
        for value in &mut next {
            *value /= norm;
        }
        q = next;
    }
    Quaternion {
        w: q[0],
        x: q[1],
        y: q[2],
        z: q[3],
    }
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

    let (offset_s, offset_e, scale, rotation) = match alignment {
        Alignment::None => (Vec3::ZERO, Vec3::ZERO, 1.0, Quaternion::IDENTITY),
        Alignment::Translation => (
            centroid(&solved_points),
            centroid(&estimated_points),
            1.0,
            Quaternion::IDENTITY,
        ),
        Alignment::Similarity => {
            let cs = centroid(&solved_points);
            let ce = centroid(&estimated_points);
            let rs = rms_radius(&solved_points, cs);
            let re = rms_radius(&estimated_points, ce);
            let scale = if re > 1e-6 && rs > 1e-6 { rs / re } else { 1.0 };
            let mut centered_s: Vec<_> = solved_points.iter().map(|point| *point - cs).collect();
            let mut centered_e: Vec<_> = estimated_points.iter().map(|point| *point - ce).collect();
            let orientation_weight = rs.max(1.0);
            for (solved, estimated) in &pairs {
                if let Some(forward) = estimated.forward.and_then(Vec3::normalized) {
                    centered_e.push(forward * orientation_weight);
                    centered_s.push(solved.forward * orientation_weight);
                }
                if let Some(up) = estimated.up.and_then(Vec3::normalized) {
                    centered_e.push(up * orientation_weight);
                    centered_s.push(solved.up * orientation_weight);
                }
            }
            (cs, ce, scale, best_rotation(&centered_e, &centered_s))
        }
    };
    let aligned: Vec<Vec3> = estimated_points
        .iter()
        .map(|p| rotation.rotate(*p - offset_e) * scale + offset_s)
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
            let f = rotation.rotate(e.forward?).normalized()?;
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

    let relative_motion_rmse_m = (pairs.len() > 1).then(|| {
        let sum: f32 = solved_points
            .windows(2)
            .zip(aligned.windows(2))
            .map(|(solved, estimated)| {
                let error = (solved[1] - solved[0]) - (estimated[1] - estimated[0]);
                error.dot(error)
            })
            .sum();
        (sum / (pairs.len() - 1) as f32).sqrt()
    });
    let horizon_errors: Vec<f32> = pairs
        .iter()
        .filter_map(|(solved, estimated)| {
            let up = rotation.rotate(estimated.up?).normalized()?;
            Some(solved.up.dot(up).clamp(-1.0, 1.0).acos().to_degrees())
        })
        .collect();
    let horizon_mean_error_deg = (!horizon_errors.is_empty())
        .then(|| horizon_errors.iter().sum::<f32>() / horizon_errors.len() as f32);

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
        applied_rotation_deg: (alignment == Alignment::Similarity).then_some(rotation.angle_deg()),
        relative_motion_rmse_m,
        horizon_mean_error_deg,
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

/// Compares an estimate against the shared compiled guidance, including the
/// observable screen-space contract used by view, prompt, and execution.
pub fn compare_compiled_guidance(
    scene: &CompiledScene,
    estimated: &EstimatedTrajectory,
    alignment: Alignment,
) -> CompiledComparisonReport {
    compare_compiled_guidance_with_temporal(scene, estimated, alignment, TemporalAlignment::Exact)
}

pub fn compare_compiled_guidance_with_temporal(
    scene: &CompiledScene,
    estimated: &EstimatedTrajectory,
    alignment: Alignment,
    temporal_alignment: TemporalAlignment,
) -> CompiledComparisonReport {
    let solved = crate::solve::SolvedScene {
        id: scene.id.clone(),
        title: scene.title.clone(),
        duration_frames: scene.duration_frames,
        subjects: scene.subjects.clone(),
        shots: scene
            .shots
            .iter()
            .map(|shot| crate::solve::SolvedShot {
                id: shot.id.clone(),
                range: shot.edit_range,
                coverage_role: shot.intent.coverage_role,
                shot_size: shot.intent.framing.shot_size,
                purpose: shot.intent.purpose.clone(),
                subject_ids: shot.intent.framing.subject_ids.clone(),
                sensor_width_mm: shot
                    .lens_track
                    .frames
                    .first()
                    .map_or(36.0, |frame| frame.sensor_width_mm),
                frames: shot.camera_track.frames.clone(),
            })
            .collect(),
    };
    let aligned_estimate = match temporal_alignment {
        TemporalAlignment::Exact => estimated.clone(),
        TemporalAlignment::DynamicTimeWarping => temporal_warp(scene, estimated),
    };
    let trajectory = compare_trajectories(&solved, &aligned_estimate, alignment);
    let expected_cuts: Vec<_> = scene
        .shots
        .iter()
        .skip(1)
        .map(|shot| shot.edit_range.start)
        .collect();
    let cut_errors = align_event_frames(&expected_cuts, &estimated.cuts);
    let shots = scene
        .shots
        .iter()
        .filter_map(|compiled| {
            let source_estimate = estimated.shots.iter().find(|shot| shot.id == compiled.id)?;
            let estimate = aligned_estimate
                .shots
                .iter()
                .find(|shot| shot.id == compiled.id)?;
            let mut perceptual = perceptual_metrics(compiled, estimate);
            perceptual.phase_timing_error_frames = phase_timing_errors(compiled, source_estimate);
            perceptual.cut_timing_error_frames = cut_errors.clone();
            perceptual.horizon_mean_error_deg = trajectory
                .shots
                .iter()
                .find(|shot| shot.shot_id == compiled.id)
                .and_then(|shot| shot.horizon_mean_error_deg);
            Some(CompiledShotComparison {
                shot_id: compiled.id.clone(),
                trajectory: trajectory
                    .shots
                    .iter()
                    .find(|shot| shot.shot_id == compiled.id)
                    .cloned(),
                perceptual,
                constraints: compiled
                    .constraints
                    .iter()
                    .map(|constraint| compare_constraint(constraint, compiled, estimate))
                    .collect(),
            })
        })
        .collect();
    CompiledComparisonReport {
        scene_id: scene.id.clone(),
        alignment,
        temporal_alignment,
        shots,
        missing_shots: trajectory.missing_shots,
        unmatched_shots: trajectory.unmatched_shots,
    }
}

fn temporal_warp(scene: &CompiledScene, estimated: &EstimatedTrajectory) -> EstimatedTrajectory {
    EstimatedTrajectory {
        source: estimated.source.clone(),
        cuts: estimated.cuts.clone(),
        shots: estimated
            .shots
            .iter()
            .map(|shot| {
                scene
                    .shot(&shot.id)
                    .map_or_else(|| shot.clone(), |compiled| warp_shot(compiled, shot))
            })
            .collect(),
    }
}

fn warp_shot(compiled: &crate::compiled::CompiledShot, estimated: &EstimatedShot) -> EstimatedShot {
    let expected = &compiled.camera_track.frames;
    if expected.is_empty() || estimated.frames.is_empty() {
        return estimated.clone();
    }
    let expected_progress = path_progress(expected.iter().map(|frame| frame.position));
    let actual_progress = path_progress(estimated.frames.iter().map(|frame| frame.position));
    let n = expected.len();
    let m = estimated.frames.len();
    let mut cost = vec![f32::INFINITY; (n + 1) * (m + 1)];
    let mut predecessor = vec![0u8; (n + 1) * (m + 1)];
    cost[0] = 0.0;
    for i in 1..=n {
        for j in 1..=m {
            let progress_cost = (expected_progress[i - 1] - actual_progress[j - 1]).abs();
            let expected_time = (i - 1) as f32 / (n - 1).max(1) as f32;
            let actual_time = (j - 1) as f32 / (m - 1).max(1) as f32;
            let local = progress_cost + 0.05 * (expected_time - actual_time).abs();
            let diagonal = cost[(i - 1) * (m + 1) + j - 1];
            let vertical = cost[(i - 1) * (m + 1) + j];
            let horizontal = cost[i * (m + 1) + j - 1];
            let (previous, direction) = if diagonal <= vertical && diagonal <= horizontal {
                (diagonal, 1)
            } else if vertical <= horizontal {
                (vertical, 2)
            } else {
                (horizontal, 3)
            };
            cost[i * (m + 1) + j] = local + previous;
            predecessor[i * (m + 1) + j] = direction;
        }
    }
    let mut matches = vec![Vec::new(); n];
    let (mut i, mut j) = (n, m);
    while i > 0 && j > 0 {
        matches[i - 1].push(j - 1);
        match predecessor[i * (m + 1) + j] {
            1 => {
                i -= 1;
                j -= 1;
            }
            2 => i -= 1,
            3 => j -= 1,
            _ => break,
        }
    }
    let mut last = 0usize;
    let frames = matches
        .into_iter()
        .enumerate()
        .map(|(index, mut candidates)| {
            candidates.sort_unstable();
            if let Some(candidate) = candidates.get(candidates.len() / 2) {
                last = *candidate;
            }
            let mut frame = estimated.frames[last.min(m - 1)].clone();
            frame.frame = expected[index].frame;
            frame
        })
        .collect();
    EstimatedShot {
        id: estimated.id.clone(),
        frames,
    }
}

fn path_progress(points: impl Iterator<Item = Vec3>) -> Vec<f32> {
    let points: Vec<_> = points.collect();
    let mut progress = Vec::with_capacity(points.len());
    let mut total = 0.0;
    progress.push(0.0);
    for pair in points.windows(2) {
        total += (pair[1] - pair[0]).length();
        progress.push(total);
    }
    if total > 1e-6 {
        for value in &mut progress {
            *value /= total;
        }
    } else if progress.len() > 1 {
        let denominator = (progress.len() - 1) as f32;
        for (index, value) in progress.iter_mut().enumerate() {
            *value = index as f32 / denominator;
        }
    }
    progress
}

fn estimated_frame(shot: &EstimatedShot, frame: Frame) -> Option<&EstimatedFrame> {
    shot.frames.iter().find(|value| value.frame == frame)
}

fn estimated_subject<'a>(
    frame: &'a EstimatedFrame,
    subject_id: &str,
) -> Option<&'a EstimatedSubjectScreenFrame> {
    frame
        .subjects
        .iter()
        .find(|subject| subject.subject_id == subject_id)
}

fn perceptual_metrics(
    compiled: &crate::compiled::CompiledShot,
    estimated: &EstimatedShot,
) -> ShotPerceptualMetrics {
    let mut bbox_errors = Vec::new();
    let mut center_errors = Vec::new();
    let mut reveal_errors = Vec::new();
    let mut flow_errors = Vec::new();
    for track in &compiled.screen_tracks {
        for expected in &track.frames {
            let Some(frame) = estimated_frame(estimated, expected.frame) else {
                continue;
            };
            let Some(actual) = estimated_subject(frame, &track.subject_id) else {
                continue;
            };
            bbox_errors.push((actual.bbox.height() - expected.bbox.height()).abs());
            let actual_center = actual.bbox.center();
            center_errors.push(
                ((actual_center.x - expected.center.x).powi(2)
                    + (actual_center.y - expected.center.y).powi(2))
                .sqrt(),
            );
            if let Some(visible) = actual.visible_fraction {
                reveal_errors.push((visible - expected.visible_fraction).abs());
            }
        }
    }
    for pair in compiled.camera_track.frames.windows(2) {
        let (Some(a), Some(b)) = (
            estimated_frame(estimated, pair[0].frame),
            estimated_frame(estimated, pair[1].frame),
        ) else {
            continue;
        };
        if let Some(actual) = b.optical_flow {
            let expected = expected_average_flow(compiled, pair[0].frame, pair[1].frame);
            if let Some(expected) = expected {
                flow_errors.push(
                    ((actual.x - expected.x).powi(2) + (actual.y - expected.y).powi(2)).sqrt(),
                );
            }
        }
        let _ = a;
    }
    let pair_errors = pair_constraint_errors(compiled, estimated);
    ShotPerceptualMetrics {
        bbox_height_mean_error: mean(&bbox_errors),
        bbox_height_max_error: maximum(&bbox_errors),
        center_drift_mean: mean(&center_errors),
        center_drift_max: maximum(&center_errors),
        background_scale_ratio: background_scale_ratio(compiled, estimated),
        pair_separation_mean_error: mean(&pair_errors),
        optical_flow_mean_error: mean(&flow_errors),
        reveal_visibility_mean_error: mean(&reveal_errors),
        focus_handoff_error_frames: focus_handoff_error(compiled, estimated),
        horizon_mean_error_deg: None,
        phase_timing_error_frames: Vec::new(),
        cut_timing_error_frames: Vec::new(),
    }
}

fn mean(values: &[f32]) -> Option<f32> {
    (!values.is_empty()).then(|| values.iter().sum::<f32>() / values.len() as f32)
}

fn maximum(values: &[f32]) -> Option<f32> {
    values.iter().copied().reduce(f32::max)
}

fn expected_average_flow(
    shot: &crate::compiled::CompiledShot,
    from: Frame,
    to: Frame,
) -> Option<ScreenPoint> {
    let values: Vec<_> = shot
        .screen_tracks
        .iter()
        .filter_map(|track| {
            let a = track.frames.iter().find(|frame| frame.frame == from)?;
            let b = track.frames.iter().find(|frame| frame.frame == to)?;
            Some(ScreenPoint {
                x: b.center.x - a.center.x,
                y: b.center.y - a.center.y,
            })
        })
        .collect();
    (!values.is_empty()).then(|| ScreenPoint {
        x: values.iter().map(|value| value.x).sum::<f32>() / values.len() as f32,
        y: values.iter().map(|value| value.y).sum::<f32>() / values.len() as f32,
    })
}

fn pair_constraint_errors(
    shot: &crate::compiled::CompiledShot,
    estimated: &EstimatedShot,
) -> Vec<f32> {
    let mut errors = Vec::new();
    for constraint in &shot.constraints {
        let ShotConstraintKind::PairScreenSeparation { subject_ids, .. } = &constraint.kind else {
            continue;
        };
        for frame in constraint.range.start..constraint.range.end {
            let expected = pair_screen_separation(shot, subject_ids, frame);
            let actual = estimated_frame(estimated, frame).and_then(|frame| {
                let a = estimated_subject(frame, &subject_ids[0])?.bbox.center();
                let b = estimated_subject(frame, &subject_ids[1])?.bbox.center();
                Some(((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt())
            });
            if let (Some(expected), Some(actual)) = (expected, actual) {
                errors.push((actual - expected).abs());
            }
        }
    }
    errors
}

fn pair_screen_separation(
    shot: &crate::compiled::CompiledShot,
    ids: &[String; 2],
    frame: Frame,
) -> Option<f32> {
    let center = |id: &str| {
        shot.screen_track(id)?
            .frames
            .iter()
            .find(|value| value.frame == frame)
            .map(|value| value.center)
    };
    let a = center(&ids[0])?;
    let b = center(&ids[1])?;
    Some(((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt())
}

fn background_scale_ratio(
    shot: &crate::compiled::CompiledShot,
    estimated: &EstimatedShot,
) -> Option<f32> {
    let primary = &shot.intent.framing.subject_ids;
    let background = shot
        .screen_tracks
        .iter()
        .find(|track| !primary.contains(&track.subject_id))?;
    let expected_first = background.frames.first()?.bbox.height();
    let expected_last = background.frames.last()?.bbox.height();
    let actual_first = estimated_subject(estimated.frames.first()?, &background.subject_id)?
        .bbox
        .height();
    let actual_last = estimated_subject(estimated.frames.last()?, &background.subject_id)?
        .bbox
        .height();
    let expected = expected_last / expected_first.max(1e-6);
    let actual = actual_last / actual_first.max(1e-6);
    Some(actual / expected.max(1e-6))
}

fn focus_handoff_error(
    shot: &crate::compiled::CompiledShot,
    estimated: &EstimatedShot,
) -> Option<i64> {
    let target = shot.constraints.iter().find_map(|constraint| {
        let ShotConstraintKind::FocusHandoff { target } = &constraint.kind else {
            return None;
        };
        target.subject_id().map(str::to_owned)
    })?;
    let expected = shot.focus_track.frames.iter().find_map(|frame| {
        (frame.target.as_ref().and_then(|value| value.subject_id()) == Some(target.as_str()))
            .then_some(frame.frame)
    })?;
    let actual = estimated.frames.iter().find_map(|frame| {
        estimated_subject(frame, &target)
            .and_then(|subject| subject.focus_score)
            .is_some_and(|score| score >= 0.5)
            .then_some(frame.frame)
    })?;
    Some(actual as i64 - expected as i64)
}

fn phase_timing_errors(
    shot: &crate::compiled::CompiledShot,
    estimated: &EstimatedShot,
) -> Vec<i64> {
    let expected: Vec<_> = shot
        .phases
        .iter()
        .filter(|phase| {
            !matches!(
                phase.kind,
                crate::compiled::PhaseKind::Hold | crate::compiled::PhaseKind::Settle
            )
        })
        .flat_map(|phase| {
            [
                phase.edit_range.start,
                phase.edit_range.end.saturating_sub(1),
            ]
        })
        .collect();
    let mut actual = Vec::new();
    let moving: Vec<_> = estimated
        .frames
        .windows(2)
        .filter(|pair| {
            (pair[1].position - pair[0].position).length() > 1e-4
                || pair[0]
                    .focal_length_mm
                    .zip(pair[1].focal_length_mm)
                    .is_some_and(|(a, b)| (b - a).abs() > 0.01)
        })
        .map(|pair| pair[1].frame)
        .collect();
    if let (Some(first), Some(last)) = (moving.first(), moving.last()) {
        actual.extend([*first, *last]);
    }
    align_event_frames(&expected, &actual)
}

fn align_event_frames(expected: &[Frame], actual: &[Frame]) -> Vec<i64> {
    expected
        .iter()
        .filter_map(|expected| {
            actual
                .iter()
                .min_by_key(|actual| actual.abs_diff(*expected))
                .map(|actual| *actual as i64 - *expected as i64)
        })
        .collect()
}

fn compare_constraint(
    constraint: &crate::compiled::ShotConstraint,
    shot: &crate::compiled::CompiledShot,
    estimated: &EstimatedShot,
) -> ConstraintComparison {
    let errors: Vec<f32> = match &constraint.kind {
        ShotConstraintKind::SubjectBboxHeight { subject_id, target } => estimated
            .frames
            .iter()
            .filter(|frame| {
                frame.frame >= constraint.range.start && frame.frame < constraint.range.end
            })
            .filter_map(|frame| estimated_subject(frame, subject_id))
            .map(|subject| (subject.bbox.height() - target).abs())
            .collect(),
        ShotConstraintKind::SubjectScreenCenter { subject_id, target } => estimated
            .frames
            .iter()
            .filter(|frame| {
                frame.frame >= constraint.range.start && frame.frame < constraint.range.end
            })
            .filter_map(|frame| estimated_subject(frame, subject_id))
            .map(|subject| {
                let center = subject.bbox.center();
                ((center.x - target.x).powi(2) + (center.y - target.y).powi(2)).sqrt()
            })
            .collect(),
        ShotConstraintKind::SubjectScreenY { subject_id, target } => estimated
            .frames
            .iter()
            .filter(|frame| {
                frame.frame >= constraint.range.start && frame.frame < constraint.range.end
            })
            .filter_map(|frame| estimated_subject(frame, subject_id))
            .map(|subject| (subject.bbox.center().y - target).abs())
            .collect(),
        ShotConstraintKind::PairScreenSeparation { .. } => pair_constraint_errors(shot, estimated),
        ShotConstraintKind::NoCameraMotion => estimated
            .frames
            .windows(2)
            .filter(|pair| {
                pair[1].frame >= constraint.range.start && pair[1].frame < constraint.range.end
            })
            .map(|pair| (pair[1].position - pair[0].position).length())
            .collect(),
        ShotConstraintKind::RevealVisibility {
            subject_id,
            from,
            to,
            monotonic,
        } => {
            let values: Vec<_> = estimated
                .frames
                .iter()
                .filter(|frame| {
                    frame.frame >= constraint.range.start && frame.frame < constraint.range.end
                })
                .filter_map(|frame| estimated_subject(frame, subject_id)?.visible_fraction)
                .collect();
            let mut errors = Vec::new();
            if let (Some(first), Some(last)) = (values.first(), values.last()) {
                errors.extend([(first - from).abs(), (last - to).abs()]);
                if *monotonic {
                    errors.extend(values.windows(2).map(|pair| (pair[0] - pair[1]).max(0.0)));
                }
            }
            errors
        }
        ShotConstraintKind::HorizonLock { .. } => estimated
            .frames
            .iter()
            .filter(|frame| {
                frame.frame >= constraint.range.start && frame.frame < constraint.range.end
            })
            .filter_map(|frame| {
                let expected = shot.frame_at(frame.frame)?.up;
                let actual = frame.up?.normalized()?;
                Some(expected.dot(actual).clamp(-1.0, 1.0).acos().to_degrees())
            })
            .collect(),
        ShotConstraintKind::FocusHandoff { .. } => focus_handoff_error(shot, estimated)
            .map(|error| vec![error.unsigned_abs() as f32])
            .unwrap_or_default(),
        ShotConstraintKind::ShotSize { value } => {
            let Some(subject_id) = shot.intent.framing.subject_ids.first() else {
                return indeterminate_comparison(constraint, "shot has no primary subject");
            };
            let Some((minimum, maximum)) = crate::compiled::shot_size_bounds(*value) else {
                return indeterminate_comparison(
                    constraint,
                    "custom/insert shot size has no numeric acceptance band",
                );
            };
            estimated
                .frames
                .iter()
                .filter_map(|frame| estimated_subject(frame, subject_id))
                .map(|subject| {
                    let height = subject.bbox.height();
                    if height < minimum {
                        minimum - height
                    } else if height > maximum {
                        height - maximum
                    } else {
                        0.0
                    }
                })
                .collect()
        }
        ShotConstraintKind::Composition { value } => {
            use crate::model::CompositionStrategy;
            estimated
                .frames
                .iter()
                .filter(|frame| {
                    frame.frame >= constraint.range.start && frame.frame < constraint.range.end
                })
                .filter_map(|frame| {
                    let centers: Vec<_> = shot
                        .intent
                        .framing
                        .subject_ids
                        .iter()
                        .filter_map(|id| {
                            estimated_subject(frame, id).map(|value| value.bbox.center())
                        })
                        .collect();
                    (!centers.is_empty()).then(|| match value.strategy {
                        CompositionStrategy::Centered | CompositionStrategy::Symmetric => {
                            (centers.iter().map(|center| center.x).sum::<f32>()
                                / centers.len() as f32
                                - 0.5)
                                .abs()
                        }
                        CompositionStrategy::RuleOfThirds => centers
                            .iter()
                            .map(|center| {
                                (center.x - 1.0 / 3.0)
                                    .abs()
                                    .min((center.x - 2.0 / 3.0).abs())
                            })
                            .fold(0.0f32, f32::max),
                        CompositionStrategy::Asymmetric => {
                            let center = centers.iter().map(|value| value.x).sum::<f32>()
                                / centers.len() as f32;
                            (constraint.tolerance - (center - 0.5).abs()).max(0.0)
                        }
                        _ => 0.0,
                    })
                })
                .collect()
        }
        ShotConstraintKind::DepthLayers { value } => estimated
            .frames
            .iter()
            .filter(|frame| {
                frame.frame >= constraint.range.start && frame.frame < constraint.range.end
            })
            .filter_map(|frame| {
                let depths: Option<Vec<f32>> = value
                    .iter()
                    .map(|layer| {
                        let values: Vec<_> = layer
                            .subject_ids
                            .iter()
                            .filter_map(|id| estimated_subject(frame, id)?.depth_m)
                            .collect();
                        (!values.is_empty())
                            .then(|| values.iter().sum::<f32>() / values.len() as f32)
                    })
                    .collect();
                depths.map(|depths| {
                    depths
                        .windows(2)
                        .map(|pair| (pair[0] + constraint.tolerance - pair[1]).max(0.0))
                        .fold(0.0f32, f32::max)
                })
            })
            .collect(),
        _ => Vec::new(),
    };
    let max_error = maximum(&errors);
    let status = match max_error {
        Some(error) if error <= constraint.tolerance => ConstraintStatus::Pass,
        Some(_) => ConstraintStatus::Fail,
        None => ConstraintStatus::Indeterminate,
    };
    ConstraintComparison {
        constraint_id: constraint.id.clone(),
        status,
        tolerance: constraint.tolerance,
        max_error,
        detail: max_error.map_or_else(
            || "estimate did not provide the required observable channel".to_owned(),
            |error| format!("maximum observable error {error:.5}"),
        ),
    }
}

fn indeterminate_comparison(
    constraint: &crate::compiled::ShotConstraint,
    detail: &str,
) -> ConstraintComparison {
    ConstraintComparison {
        constraint_id: constraint.id.clone(),
        status: ConstraintStatus::Indeterminate,
        tolerance: constraint.tolerance,
        max_error: None,
        detail: detail.to_owned(),
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
                        up: Some(f.up),
                        focal_length_mm: Some(f.focal_length_mm),
                        subjects: Vec::new(),
                        optical_flow: None,
                    })
                    .collect(),
            })
            .collect(),
        cuts: scene
            .shots
            .iter()
            .skip(1)
            .map(|shot| shot.range.start)
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

pub fn render_compiled_report(report: &CompiledComparisonReport) -> String {
    use std::fmt::Write;
    let mut out = format!(
        "scene {} — compiled guidance comparison ({:?}, {:?})\n",
        report.scene_id, report.alignment, report.temporal_alignment
    );
    for shot in &report.shots {
        let _ = writeln!(out, "shot {}", shot.shot_id);
        for constraint in &shot.constraints {
            let _ = writeln!(
                out,
                "  {}: {:?} — {}",
                constraint.constraint_id.to_ascii_uppercase(),
                constraint.status,
                constraint.detail
            );
        }
        if !shot.perceptual.phase_timing_error_frames.is_empty() {
            let _ = writeln!(
                out,
                "  PHASE_TIMING: {:?} frame(s)",
                shot.perceptual.phase_timing_error_frames
            );
        }
        if let Some(error) = shot.perceptual.horizon_mean_error_deg {
            let _ = writeln!(out, "  HORIZON: {error:.3} deg mean error");
        }
    }
    if !report.missing_shots.is_empty() {
        let _ = writeln!(out, "missing: {}", report.missing_shots.join(", "));
    }
    if !report.unmatched_shots.is_empty() {
        let _ = writeln!(out, "unmatched: {}", report.unmatched_shots.join(", "));
    }
    out
}
