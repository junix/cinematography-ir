//! Time sampling (ADR-1115 D4 stage 1): which scene frames become panels.

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
                    frames.push((shot.range.start + timed.range.end).min(shot.range.end - 1));
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
