use std::fmt::Write;

use crate::model::{CameraOperation, CineProject};

pub fn render_summary(project: &CineProject) -> String {
    let mut output = String::new();
    let fps = project
        .frame_rate
        .frames_per_second()
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "invalid".to_owned());

    let _ = writeln!(output, "Project: {} ({})", project.title, project.id);
    let _ = writeln!(output, "Schema: {} | FPS: {}", project.schema_version, fps);
    let _ = writeln!(output, "Scenes: {}", project.scenes.len());

    for scene in &project.scenes {
        let seconds = project
            .frame_rate
            .frames_per_second()
            .filter(|value| *value > 0.0)
            .map(|value| scene.duration_frames as f64 / value);
        let duration = seconds
            .map(|value| format!("{value:.2}s"))
            .unwrap_or_else(|| "unknown".to_owned());

        let _ = writeln!(
            output,
            "\nScene {}: {} — {} frames ({})",
            scene.id, scene.title, scene.duration_frames, duration
        );
        if let Some(question) = &scene.narrative.dramatic_question {
            let _ = writeln!(output, "  Dramatic question: {question}");
        }
        let _ = writeln!(
            output,
            "  Subjects: {} | Axes: {} | Beats: {} | Shots: {}",
            scene.subjects.len(),
            scene.axes.len(),
            scene.beats.len(),
            scene.shots.len()
        );

        let mut shots: Vec<_> = scene.shots.iter().collect();
        shots.sort_by_key(|shot| (shot.range.start, shot.range.end));
        for shot in shots {
            let movement_summary = if shot.camera.operations.is_empty() {
                "locked".to_owned()
            } else {
                shot.camera
                    .operations
                    .iter()
                    .map(|timed| operation_name(&timed.operation))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let _ = writeln!(
                output,
                "  [{:>5}..{:<5}) {:<20} {:?}/{:?} | {:?} | {}",
                shot.range.start,
                shot.range.end,
                shot.id,
                shot.coverage_role,
                shot.framing.shot_size,
                shot.camera.rig,
                movement_summary
            );
        }
    }

    output
}

fn operation_name(operation: &CameraOperation) -> &'static str {
    match operation {
        CameraOperation::Hold => "hold",
        CameraOperation::Pan { .. } => "pan",
        CameraOperation::Tilt { .. } => "tilt",
        CameraOperation::Roll { .. } => "roll",
        CameraOperation::Dolly { .. } => "dolly",
        CameraOperation::Truck { .. } => "truck",
        CameraOperation::Pedestal { .. } => "pedestal",
        CameraOperation::Translate { .. } => "translate",
        CameraOperation::Rotate { .. } => "rotate",
        CameraOperation::LookAt { .. } => "look_at",
        CameraOperation::Orbit { .. } => "orbit",
        CameraOperation::Follow { .. } => "follow",
        CameraOperation::Crane { .. } => "crane",
        CameraOperation::Zoom { .. } => "zoom",
        CameraOperation::RackFocus { .. } => "rack_focus",
        CameraOperation::DollyZoom { .. } => "dolly_zoom",
        CameraOperation::HandheldNoise { .. } => "handheld_noise",
        CameraOperation::Reveal { .. } => "reveal",
    }
}
