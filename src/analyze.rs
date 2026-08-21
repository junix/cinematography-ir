use std::collections::HashMap;

use crate::diagnostic::{Diagnostic, ValidationReport};
use crate::model::{
    AxisSide, HorizontalDirection, Scene, ScreenDirection, Shot, Transition,
};

pub fn analyze_continuity(scene: &Scene) -> ValidationReport {
    let mut report = ValidationReport::default();
    let mut shots: Vec<&Shot> = scene.shots.iter().collect();
    shots.sort_by_key(|shot| (shot.range.start, shot.range.end));

    for pair in shots.windows(2) {
        let previous = pair[0];
        let next = pair[1];
        analyze_axis_cross(scene, previous, next, &mut report);
        analyze_screen_direction(scene, previous, next, &mut report);
        analyze_thirty_degree_rule(scene, previous, next, &mut report);
        analyze_reciprocal_eyeline(scene, previous, next, &mut report);
    }

    report
}

fn analyze_axis_cross(
    scene: &Scene,
    previous: &Shot,
    next: &Shot,
    report: &mut ValidationReport,
) {
    let previous_sides: HashMap<&str, AxisSide> = previous
        .continuity
        .axis_observations
        .iter()
        .map(|item| (item.axis_id.as_str(), item.side))
        .collect();

    for observation in &next.continuity.axis_observations {
        let Some(previous_side) = previous_sides.get(observation.axis_id.as_str()) else {
            continue;
        };

        let crosses = matches!(previous_side, AxisSide::Positive | AxisSide::Negative)
            && matches!(observation.side, AxisSide::Positive | AxisSide::Negative)
            && *previous_side != observation.side;

        if crosses && !has_continuity_bridge(previous, next) {
            report.push(
                Diagnostic::warning(
                    "CONTINUITY_AXIS_CROSS",
                    format!("scenes[{}].shots[{}->{}]", scene.id, previous.id, next.id),
                    format!(
                        "camera side for axis '{}' changes from {:?} to {:?} without an explicit bridge",
                        observation.axis_id, previous_side, observation.side
                    ),
                )
                .with_hint(
                    "Declare a visible crossing, on-axis shot, neutral insert, re-establishing shot, character reblocking, or deliberate disorientation.",
                ),
            );
        }
    }
}

fn analyze_screen_direction(
    scene: &Scene,
    previous: &Shot,
    next: &Shot,
    report: &mut ValidationReport,
) {
    let previous_directions: HashMap<&str, ScreenDirection> = previous
        .continuity
        .screen_directions
        .iter()
        .map(|item| (item.subject_id.as_str(), item.direction))
        .collect();

    for observation in &next.continuity.screen_directions {
        let Some(previous_direction) = previous_directions.get(observation.subject_id.as_str())
        else {
            continue;
        };

        let reverses = matches!(previous_direction, ScreenDirection::Left | ScreenDirection::Right)
            && matches!(observation.direction, ScreenDirection::Left | ScreenDirection::Right)
            && *previous_direction != observation.direction;

        if reverses
            && !previous.continuity.direction_change_visible
            && !next.continuity.direction_change_visible
            && !has_continuity_bridge(previous, next)
        {
            report.push(
                Diagnostic::warning(
                    "CONTINUITY_SCREEN_DIRECTION_FLIP",
                    format!("scenes[{}].shots[{}->{}]", scene.id, previous.id, next.id),
                    format!(
                        "subject '{}' reverses screen direction from {:?} to {:?} without a visible turn",
                        observation.subject_id, previous_direction, observation.direction
                    ),
                )
                .with_hint(
                    "Show the turn/reblocking, use a neutral or re-establishing shot, or mark the reversal as deliberate.",
                ),
            );
        }
    }
}

fn analyze_thirty_degree_rule(
    scene: &Scene,
    previous: &Shot,
    next: &Shot,
    report: &mut ValidationReport,
) {
    if previous.transition_in == Transition::JumpCut || next.transition_in == Transition::JumpCut {
        return;
    }

    let (Some(previous_azimuth), Some(next_azimuth)) = (
        previous.continuity.camera_azimuth_deg,
        next.continuity.camera_azimuth_deg,
    ) else {
        return;
    };

    if previous.framing.shot_size != next.framing.shot_size
        || !shares_primary_subject(previous, next)
    {
        return;
    }

    let delta = angular_distance_deg(previous_azimuth, next_azimuth);
    if delta < 30.0 && !has_continuity_bridge(previous, next) {
        report.push(
            Diagnostic::warning(
                "CONTINUITY_SMALL_ANGLE_CUT",
                format!("scenes[{}].shots[{}->{}]", scene.id, previous.id, next.id),
                format!(
                    "cut changes camera azimuth by only {:.1}° while keeping shot size and subject",
                    delta
                ),
            )
            .with_hint(
                "Change the angle, shot size, or composition more substantially, or declare a jump cut.",
            ),
        );
    }
}

fn analyze_reciprocal_eyeline(
    scene: &Scene,
    previous: &Shot,
    next: &Shot,
    report: &mut ValidationReport,
) {
    for first in &previous.continuity.eyelines {
        let Some(first_target) = first.target.subject_id() else {
            continue;
        };

        for second in &next.continuity.eyelines {
            let Some(second_target) = second.target.subject_id() else {
                continue;
            };

            let reciprocal = first.subject_id.as_str() == second_target
                && first_target == second.subject_id.as_str();
            let same_direction = first.screen_direction == second.screen_direction
                && matches!(
                    first.screen_direction,
                    HorizontalDirection::Left | HorizontalDirection::Right
                );

            if reciprocal && same_direction && !has_continuity_bridge(previous, next) {
                report.push(
                    Diagnostic::warning(
                        "CONTINUITY_EYELINE_MISMATCH",
                        format!("scenes[{}].shots[{}->{}]", scene.id, previous.id, next.id),
                        format!(
                            "reciprocal eyelines '{}'→'{}' and '{}'→'{}' both point {:?} on screen",
                            first.subject_id,
                            first_target,
                            second.subject_id,
                            second_target,
                            first.screen_direction
                        ),
                    )
                    .with_hint(
                        "Conventional shot/reverse-shot coverage normally gives reciprocal eyelines opposite screen directions.",
                    ),
                );
            }
        }
    }
}

fn has_continuity_bridge(previous: &Shot, next: &Shot) -> bool {
    previous.continuity.spatial_reset
        || next.continuity.spatial_reset
        || previous.continuity.bridge.is_some()
        || next.continuity.bridge.is_some()
}

fn shares_primary_subject(previous: &Shot, next: &Shot) -> bool {
    previous
        .framing
        .subject_ids
        .iter()
        .any(|id| next.framing.subject_ids.iter().any(|other| other == id))
}

fn angular_distance_deg(a: f32, b: f32) -> f32 {
    let delta = (a - b).rem_euclid(360.0);
    delta.min(360.0 - delta)
}
