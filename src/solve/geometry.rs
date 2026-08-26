//! Geometry-derived diagnostics on solved scenes: checks that only become
//! possible once positions are sampled (ADR-1115 "从几何自动推导").

use crate::diagnostic::{Diagnostic, ValidationReport};
use crate::math::side_of_axis;
use crate::model::{AxisSide, CoordinateSystem, Scene};
use crate::solve::sampler::SceneContext;
use crate::solve::solved::SolvedScene;

/// Tolerance in metres below which a camera counts as on-axis.
const ON_AXIS_EPSILON_M: f32 = 0.05;

pub fn check_scene(
    scene: &Scene,
    solved: &SolvedScene,
    cs: &CoordinateSystem,
    fps: f64,
    report: &mut ValidationReport,
) {
    let ctx = SceneContext {
        scene,
        cs,
        fps,
        subjects: solved.subjects.clone(),
    };

    for shot in &scene.shots {
        let Some(solved_shot) = solved.shot(&shot.id) else {
            continue;
        };
        let Some(first) = solved_shot.first() else {
            continue;
        };
        let path = format!("scenes[{}].shots[{}]", scene.id, shot.id);
        let half_fov = first.horizontal_fov_deg / 2.0;

        for (index, subject_id) in shot.framing.subject_ids.iter().enumerate() {
            let Some(track) = solved.subject(subject_id) else {
                continue;
            };
            let aim = ctx.aim_point(track, shot.range.start);
            let offset = aim - first.position;
            let depth = offset.dot(first.forward);
            let subject_path = format!("{path}.framing.subject_ids[{index}]");
            if depth <= 0.0 {
                report.push(
                    Diagnostic::warning(
                        "GEOMETRY_SUBJECT_BEHIND_CAMERA",
                        subject_path,
                        format!(
                            "framed subject '{subject_id}' is behind the camera at frame {}",
                            shot.range.start
                        ),
                    )
                    .with_hint(
                        "Check the camera's position/yaw against coordinate_system.forward_axis; an identity camera looks along forward_axis.",
                    ),
                );
                continue;
            }
            let lateral = offset.dot(first.right);
            let angle = (lateral / depth).atan().to_degrees().abs();
            if angle > half_fov {
                report.push(
                    Diagnostic::warning(
                        "GEOMETRY_SUBJECT_OUTSIDE_FOV",
                        subject_path,
                        format!(
                            "framed subject '{subject_id}' is {angle:.1}° off-axis but the horizontal half-FOV is {half_fov:.1}°"
                        ),
                    )
                    .with_hint("Turn the camera toward the subject, widen the lens, or drop it from framing.subject_ids."),
                );
            }
        }

        for (index, observation) in shot.continuity.axis_observations.iter().enumerate() {
            let Some(axis) = scene.axes.iter().find(|a| a.id == observation.axis_id) else {
                continue;
            };
            if observation.side == AxisSide::OnAxis {
                continue;
            }
            let (Some(from), Some(to)) = (
                ctx.resolve(&axis.from, shot.range.start),
                ctx.resolve(&axis.to, shot.range.start),
            ) else {
                continue;
            };
            let Some(side) = side_of_axis(cs, from, to, first.position) else {
                continue;
            };
            let derived = if side > ON_AXIS_EPSILON_M {
                AxisSide::Positive
            } else if side < -ON_AXIS_EPSILON_M {
                AxisSide::Negative
            } else {
                AxisSide::OnAxis
            };
            if derived != observation.side {
                report.push(
                    Diagnostic::warning(
                        "GEOMETRY_AXIS_SIDE_MISMATCH",
                        format!("{path}.continuity.axis_observations[{index}].side"),
                        format!(
                            "declared side {:?} of axis '{}' but the camera geometry derives {:?} ({side:+.2} m from the axis plane)",
                            observation.side, axis.id, derived
                        ),
                    )
                    .with_hint(
                        "Positive means the camera is on the right-hand side when travelling from the axis' `from` endpoint to `to`. Fix the declaration or the camera.",
                    ),
                );
            }
        }
    }
}
