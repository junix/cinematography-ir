//! Time sampling (ADR-1115 D4 stage 1): which scene frames become panels.

use crate::compiled::{CompiledScene, PhaseKind, ShotConstraintKind};
use crate::model::{Frame, Scene, Shot};
use crate::solve::SolvedScene;

#[derive(Debug, Clone, PartialEq, Default)]
pub enum Sampling {
    /// One panel per shot, at the shot's first frame (default).
    #[default]
    PerShot,
    /// One panel per beat, at the beat's first frame.
    PerBeat,
    /// A panel at every camera-operation start and end, plus shot starts.
    OperationEndpoints,
    /// A panel every `n` scene frames.
    EveryNFrames(u64),
    /// Every frame, subsampled to roughly `fps` frames per second.
    Continuous { fps: u32 },
    /// Operation boundaries, velocity midpoints, focus/reveal milestones,
    /// beat apices, and both sides of cuts.
    SemanticEvents,
    /// Start, midpoint, and last active frame of every compiled phase.
    PhaseKeyframes,
    /// Frames where observable constraint signals reach their extrema.
    ConstraintExtrema,
    /// Last frame before and first frame after every edit.
    CutPairs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SamplePoint {
    pub scene_frame: Frame,
    pub shot_id: String,
    /// Human-facing caption (shot id, beat id, ...).
    pub label: String,
}

pub fn sample_points(
    scene: &Scene,
    solved: &SolvedScene,
    sampling: &Sampling,
    project_fps: f64,
) -> Vec<SamplePoint> {
    let mut shots: Vec<&Shot> = scene.shots.iter().collect();
    shots.sort_by_key(|shot| (shot.range.start, shot.range.end));
    let shot_at = |frame: Frame| solved.shot_at(frame).map(|s| s.id.clone());

    match sampling {
        Sampling::PerShot => shots
            .iter()
            .map(|shot| SamplePoint {
                scene_frame: shot.range.start,
                shot_id: shot.id.clone(),
                label: shot.id.clone(),
            })
            .collect(),
        Sampling::PerBeat => {
            let mut beats: Vec<_> = scene.beats.iter().collect();
            beats.sort_by_key(|beat| (beat.range.start, beat.range.end));
            beats
                .into_iter()
                .filter_map(|beat| {
                    shot_at(beat.range.start).map(|shot_id| SamplePoint {
                        scene_frame: beat.range.start,
                        shot_id,
                        label: beat.id.clone(),
                    })
                })
                .collect()
        }
        Sampling::OperationEndpoints => {
            let mut points = Vec::new();
            for shot in &shots {
                let mut frames = vec![shot.range.start];
                for timed in &shot.camera.operations {
                    frames.push(shot.range.start + timed.range.start);
                    let last_active = shot.range.start + timed.range.end.saturating_sub(1);
                    frames.push(last_active.min(shot.range.end.saturating_sub(1)));
                }
                frames.sort_unstable();
                frames.dedup();
                for frame in frames {
                    points.push(SamplePoint {
                        scene_frame: frame,
                        shot_id: shot.id.clone(),
                        label: shot.id.clone(),
                    });
                }
            }
            points
        }
        Sampling::EveryNFrames(n) => every(scene, *n.max(&1), shot_at),
        Sampling::Continuous { fps } => {
            let step = if *fps == 0 || project_fps <= 0.0 {
                1
            } else {
                (project_fps / *fps as f64).round().max(1.0) as u64
            };
            every(scene, step, shot_at)
        }
        Sampling::SemanticEvents
        | Sampling::PhaseKeyframes
        | Sampling::ConstraintExtrema
        | Sampling::CutPairs => Vec::new(),
    }
}

/// Samples directly from the shared compiled timeline. This is the canonical
/// sampler for view consumers; `sample_points` remains as a legacy bridge.
pub fn sample_compiled_points(
    scene: &CompiledScene,
    sampling: &Sampling,
    project_fps: f64,
) -> Vec<SamplePoint> {
    let mut points = Vec::new();
    let mut push = |frame: Frame, shot_id: &str, label: String| {
        points.push(SamplePoint {
            scene_frame: frame,
            shot_id: shot_id.to_owned(),
            label,
        });
    };
    match sampling {
        Sampling::PerShot => {
            for shot in &scene.shots {
                push(shot.edit_range.start, &shot.id, shot.id.clone());
            }
        }
        Sampling::PerBeat => {
            for beat in &scene.beats {
                if let Some(shot) = scene.shot_at(beat.range.start) {
                    push(beat.range.start, &shot.id, beat.id.clone());
                }
            }
        }
        Sampling::OperationEndpoints => {
            for shot in &scene.shots {
                push(shot.edit_range.start, &shot.id, shot.id.clone());
                for phase in &shot.phases {
                    push(
                        phase.edit_range.start,
                        &shot.id,
                        format!("{} start", phase.id),
                    );
                    push(
                        phase.edit_range.end.saturating_sub(1),
                        &shot.id,
                        format!("{} end", phase.id),
                    );
                }
            }
        }
        Sampling::PhaseKeyframes => {
            for shot in &scene.shots {
                for phase in &shot.phases {
                    let end = phase.edit_range.end.saturating_sub(1);
                    let mid = phase.edit_range.start + phase.edit_range.duration() / 2;
                    for (frame, suffix) in [
                        (phase.edit_range.start, "start"),
                        (mid.min(end), "mid"),
                        (
                            end,
                            if phase.kind == PhaseKind::Settle {
                                "settle"
                            } else {
                                "end"
                            },
                        ),
                    ] {
                        push(frame, &shot.id, format!("{} {suffix}", phase.id));
                    }
                }
            }
        }
        Sampling::SemanticEvents => {
            for shot in &scene.shots {
                for phase in &shot.phases {
                    let start = phase.edit_range.start;
                    let end = phase.edit_range.end.saturating_sub(1);
                    let apex = (start + phase.edit_range.duration() / 2).min(end);
                    push(start, &shot.id, format!("{} start", phase.id));
                    if phase.kind != PhaseKind::Hold && phase.kind != PhaseKind::Settle {
                        push(apex, &shot.id, format!("{} apex", phase.id));
                    }
                    if phase.kind == PhaseKind::Reveal {
                        for quarter in [1, 2, 3] {
                            let frame = start + phase.edit_range.duration() * quarter / 4;
                            push(
                                frame.min(end),
                                &shot.id,
                                format!("{} {}%", phase.id, quarter * 25),
                            );
                        }
                    }
                    push(end, &shot.id, format!("{} end", phase.id));
                }
            }
            append_cut_pairs(scene, &mut points);
        }
        Sampling::ConstraintExtrema => {
            for shot in &scene.shots {
                let mut frames = vec![shot.edit_range.start, shot.edit_range.end.saturating_sub(1)];
                for constraint in &shot.constraints {
                    match &constraint.kind {
                        ShotConstraintKind::SubjectBboxHeight { subject_id, .. }
                        | ShotConstraintKind::SubjectScreenCenter { subject_id, .. }
                        | ShotConstraintKind::SubjectScreenY { subject_id, .. }
                        | ShotConstraintKind::RevealVisibility { subject_id, .. } => {
                            if let Some(track) = shot.screen_track(subject_id) {
                                for selector in [
                                    |f: &crate::compiled::SubjectScreenFrame| f.bbox.height(),
                                    |f: &crate::compiled::SubjectScreenFrame| f.visible_fraction,
                                ] {
                                    if let Some(min) = track
                                        .frames
                                        .iter()
                                        .min_by(|a, b| selector(a).total_cmp(&selector(b)))
                                    {
                                        frames.push(min.frame);
                                    }
                                    if let Some(max) = track
                                        .frames
                                        .iter()
                                        .max_by(|a, b| selector(a).total_cmp(&selector(b)))
                                    {
                                        frames.push(max.frame);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                for frame in frames {
                    push(frame, &shot.id, format!("{} constraint", shot.id));
                }
            }
        }
        Sampling::CutPairs => append_cut_pairs(scene, &mut points),
        Sampling::EveryNFrames(n) => {
            let step = (*n).max(1) as usize;
            for frame in (0..scene.duration_frames).step_by(step) {
                if let Some(shot) = scene.shot_at(frame) {
                    push(frame, &shot.id, shot.id.clone());
                }
            }
        }
        Sampling::Continuous { fps } => {
            let step = if *fps == 0 || project_fps <= 0.0 {
                1
            } else {
                (project_fps / *fps as f64).round().max(1.0) as usize
            };
            for frame in (0..scene.duration_frames).step_by(step) {
                if let Some(shot) = scene.shot_at(frame) {
                    push(frame, &shot.id, shot.id.clone());
                }
            }
        }
    }
    points.sort_by_key(|point| {
        (
            point.scene_frame,
            point.shot_id.clone(),
            point.label.clone(),
        )
    });
    points.dedup_by(|a, b| a.scene_frame == b.scene_frame && a.shot_id == b.shot_id);
    points
}

fn append_cut_pairs(scene: &CompiledScene, points: &mut Vec<SamplePoint>) {
    let mut shots: Vec<_> = scene.shots.iter().collect();
    shots.sort_by_key(|shot| (shot.edit_range.start, shot.edit_range.end));
    for pair in shots.windows(2) {
        let previous = pair[0];
        let next = pair[1];
        points.push(SamplePoint {
            scene_frame: previous.edit_range.end.saturating_sub(1),
            shot_id: previous.id.clone(),
            label: format!("cut out {}", previous.id),
        });
        points.push(SamplePoint {
            scene_frame: next.edit_range.start,
            shot_id: next.id.clone(),
            label: format!("cut in {}", next.id),
        });
    }
}

fn every(scene: &Scene, step: u64, shot_at: impl Fn(Frame) -> Option<String>) -> Vec<SamplePoint> {
    (0..scene.duration_frames)
        .step_by(step as usize)
        .filter_map(|frame| {
            shot_at(frame).map(|shot_id| SamplePoint {
                scene_frame: frame,
                label: shot_id.clone(),
                shot_id,
            })
        })
        .collect()
}
