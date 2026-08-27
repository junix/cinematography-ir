//! Text-prompt slot (ADR-1115 D1 ①/D9): the upper IR layers rendered as a
//! natural-language shot description in a target model's dialect.
//!
//! Pure: no filesystem, no process, no network. Dialects are parsed by the
//! caller and passed in.

pub mod dialect;

use std::fmt::Write;

use crate::compiled::{compile_project, CompiledScene, CompiledShot, ConstraintStatus, PhaseKind};
use crate::model::{
    CameraOperation, CineProject, DepthRole, Scene, Shot, TargetRef, TimedCameraOperation,
    Transition,
};
use crate::palette::{scene_palette, NamedColor};

use dialect::serde_name;
pub use dialect::{DialectError, PromptDialect, GENERIC_DIALECT_JSON};

#[derive(Debug, Clone)]
pub struct PromptOptions {
    /// Append distances, angles, and durations to movement phrases.
    pub include_measurements: bool,
    /// Prefix each prompt with the scene title and dramatic question.
    pub include_scene_context: bool,
    /// Name subjects with their palette colour ("Alice (red)") so the prompt
    /// binds identities to the colour-coded conditioning images (D7a/D10).
    pub include_color_binding: bool,
}

impl Default for PromptOptions {
    fn default() -> Self {
        Self {
            include_measurements: true,
            include_scene_context: true,
            include_color_binding: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ShotPrompt {
    pub scene_id: String,
    pub shot_id: String,
    pub text: String,
}

/// Prompts for every shot (optionally filtered), in timeline order.
pub fn emit_project_prompts(
    project: &CineProject,
    dialect: &PromptDialect,
    options: &PromptOptions,
    scene_filter: Option<&str>,
    shot_filter: Option<&str>,
) -> Vec<ShotPrompt> {
    let project = crate::solve::normalize_units(project);
    let project = &*project;
    let compiled = compile_project(project, &crate::compiled::CompileOptions::default())
        .ok()
        .map(|output| output.compiled);
    let mut prompts = Vec::new();
    for scene in &project.scenes {
        if scene_filter.is_some_and(|id| id != scene.id) {
            continue;
        }
        let compiled_scene = compiled.as_ref().and_then(|value| value.scene(&scene.id));
        let mut compiled_shots: Vec<&CompiledShot> = compiled_scene
            .map(|scene| scene.shots.iter().collect())
            .unwrap_or_default();
        compiled_shots.sort_by_key(|shot| (shot.edit_range.start, shot.edit_range.end));
        if !compiled_shots.is_empty() {
            for compiled_shot in compiled_shots {
                if shot_filter.is_some_and(|id| id != compiled_shot.id) {
                    continue;
                }
                let authored = scene
                    .shots
                    .iter()
                    .find(|shot| shot.id == compiled_shot.id)
                    .cloned()
                    .unwrap_or_else(|| shot_from_compiled(compiled_shot));
                prompts.push(ShotPrompt {
                    scene_id: scene.id.clone(),
                    shot_id: compiled_shot.id.clone(),
                    text: emit_shot_prompt_inner(
                        project,
                        scene,
                        &authored,
                        Some(compiled_shot),
                        dialect,
                        options,
                    ),
                });
            }
            continue;
        }
        let mut shots: Vec<&Shot> = scene.shots.iter().collect();
        shots.sort_by_key(|shot| (shot.range.start, shot.range.end));
        for shot in shots {
            if shot_filter.is_some_and(|id| id != shot.id) {
                continue;
            }
            prompts.push(ShotPrompt {
                scene_id: scene.id.clone(),
                shot_id: shot.id.clone(),
                text: emit_shot_prompt_inner(project, scene, shot, None, dialect, options),
            });
        }
    }
    prompts
}

/// One shot as a single-paragraph prompt.
pub fn emit_shot_prompt(
    project: &CineProject,
    scene: &Scene,
    shot: &Shot,
    dialect: &PromptDialect,
    options: &PromptOptions,
) -> String {
    let compiled = compile_project(project, &crate::compiled::CompileOptions::default())
        .ok()
        .and_then(|output| {
            output
                .compiled
                .scene(&scene.id)
                .and_then(|value| value.shot(&shot.id))
                .cloned()
        });
    emit_shot_prompt_inner(project, scene, shot, compiled.as_ref(), dialect, options)
}

fn emit_shot_prompt_inner(
    project: &CineProject,
    scene: &Scene,
    shot: &Shot,
    compiled: Option<&CompiledShot>,
    dialect: &PromptDialect,
    options: &PromptOptions,
) -> String {
    let palette = scene_palette(scene);
    let fps = project.frame_rate.frames_per_second().unwrap_or(24.0);
    let mut sentences: Vec<String> = Vec::new();

    let name_of = |id: &str| -> String {
        let name = scene
            .subjects
            .iter()
            .find(|s| s.id == id)
            .map(|s| s.name.as_str())
            .unwrap_or(id);
        match (options.include_color_binding, palette.get(id)) {
            (true, Some(color)) => bind_color(dialect, name, *color),
            _ => name.to_owned(),
        }
    };
    let target_name = |target: &TargetRef| -> String {
        match target {
            TargetRef::Subject { id } => name_of(id),
            TargetRef::Marker { id } => scene
                .markers
                .iter()
                .find(|m| &m.id == id)
                .map(|m| m.name.clone())
                .unwrap_or_else(|| id.clone()),
            TargetRef::Point { .. } => dialect.word("fixed_point"),
        }
    };

    // Scene context.
    if options.include_scene_context {
        let mut context = format!("{}: {}.", dialect.word("scene"), scene.title);
        if let Some(question) = &scene.narrative.dramatic_question {
            let _ = write!(context, " {question}");
        }
        sentences.push(context);
    }

    // Transition in.
    if shot.transition_in != Transition::Cut {
        let phrase = dialect.phrase(&dialect.transition, &serde_name(&shot.transition_in));
        if !phrase.is_empty() {
            sentences.push(capitalize(&format!("{phrase}.")));
        }
    }

    // Shot header: coverage role, size, angles.
    let mut header_parts =
        vec![dialect.phrase(&dialect.shot_size, &serde_name(&shot.framing.shot_size))];
    for phrase in [
        dialect.phrase(
            &dialect.horizontal_angle,
            &serde_name(&shot.framing.horizontal_angle),
        ),
        dialect.phrase(
            &dialect.vertical_angle,
            &serde_name(&shot.framing.vertical_angle),
        ),
    ] {
        if !phrase.is_empty() {
            header_parts.push(phrase);
        }
    }
    let role = dialect.phrase(&dialect.coverage_role, &serde_name(&shot.coverage_role));
    sentences.push(capitalize(&format!("{role}: {}.", header_parts.join(", "))));

    // Subjects in frame.
    if !shot.framing.subject_ids.is_empty() {
        let names: Vec<String> = shot
            .framing
            .subject_ids
            .iter()
            .map(|id| name_of(id))
            .collect();
        sentences.push(format!(
            "{} {}.",
            dialect.word("framing"),
            join_natural(&names, &dialect.word("and"))
        ));
    }
    if !shot.framing.composition.depth_layers.is_empty() {
        let mut layers = Vec::new();
        for role in [
            DepthRole::Foreground,
            DepthRole::Midground,
            DepthRole::Background,
        ] {
            let ids: Vec<String> = shot
                .framing
                .composition
                .depth_layers
                .iter()
                .filter(|layer| layer.role == role)
                .flat_map(|layer| layer.subject_ids.iter())
                .map(|id| name_of(id))
                .collect();
            if !ids.is_empty() {
                layers.push(format!(
                    "{}: {}",
                    dialect.phrase(&dialect.depth_role, &serde_name(&role)),
                    ids.join(", ")
                ));
            }
        }
        if !layers.is_empty() {
            sentences.push(capitalize(&format!("{}.", layers.join("; "))));
        }
    }

    // Blocking cues for framed subjects.
    for subject_id in &shot.framing.subject_ids {
        let Some(track) = scene.blocking.iter().find(|t| &t.subject_id == subject_id) else {
            continue;
        };
        let mut keyframes: Vec<_> = track.keyframes.iter().collect();
        keyframes.sort_by_key(|k| k.frame);
        let before: Vec<_> = keyframes
            .iter()
            .filter(|k| k.frame < shot.range.start)
            .copied()
            .collect();
        let inside: Vec<_> = keyframes
            .iter()
            .filter(|k| k.frame >= shot.range.start && k.frame < shot.range.end)
            .copied()
            .collect();
        let mut relevant: Vec<_> = before.last().into_iter().copied().collect();
        relevant.extend(inside);

        let actions: Vec<String> = relevant.iter().filter_map(|k| k.action.clone()).collect();
        let gaze = relevant.iter().rev().find_map(|k| k.gaze_target.as_ref());
        if actions.is_empty() && gaze.is_none() {
            continue;
        }
        let mut line = format!("{}:", name_of(subject_id));
        if !actions.is_empty() {
            let _ = write!(
                line,
                " {}",
                actions.join(&format!(", {} ", dialect.word("then")))
            );
        }
        if let Some(gaze) = gaze {
            if !actions.is_empty() {
                line.push(';');
            }
            let _ = write!(
                line,
                " {} {}",
                dialect.word("looking_at"),
                target_name(gaze)
            );
        }
        line.push('.');
        sentences.push(line);
    }

    // Camera rig and movement.
    let rig = dialect.phrase(&dialect.camera_rig, &serde_name(&shot.camera.rig));
    let movement = if shot.camera.operations.is_empty() {
        dialect.word("static")
    } else {
        let up = crate::math::identity_basis(&project.coordinate_system).up;
        let mut focal = shot.camera.initial_state.lens.focal_length_mm;
        let mut ordered: Vec<&TimedCameraOperation> = shot.camera.operations.iter().collect();
        ordered.sort_by_key(|timed| (timed.range.start, timed.range.end));
        let mut text = String::new();
        let mut previous_end: Option<u64> = None;
        for timed in ordered {
            let (key, measurement, mut next_focal) =
                operation_key(&timed.operation, focal, up, options.include_measurements);
            let measurement = measurement.map(|m| m.replace("{to}", &dialect.word("to")));
            if matches!(timed.operation, CameraOperation::DollyZoom { .. }) {
                // The compiled lens track is authoritative. Prompt rendering
                // never re-solves the coupled camera/lens mechanism.
                if let Some(frame) = compiled.and_then(|shot| {
                    let frame = shot.edit_range.start + timed.range.end.saturating_sub(1);
                    shot.lens_track
                        .frames
                        .iter()
                        .find(|value| value.frame == frame)
                }) {
                    next_focal = frame.focal_length_mm;
                }
            }
            focal = next_focal;
            let mut phrase = dialect.phrase(&dialect.operation, key);
            if let Some(target) = operation_target(&timed.operation) {
                phrase = phrase.replace("{target}", &target_name(target));
            }
            if options.include_measurements {
                let seconds = timed.range.duration() as f64 / fps;
                let mut detail = measurement.unwrap_or_default();
                if !detail.is_empty() {
                    detail.push(' ');
                }
                let _ = write!(
                    detail,
                    "{} {seconds:.1}{}",
                    dialect.word("over"),
                    dialect.word("seconds")
                );
                phrase = format!("{phrase} ({detail})");
            }
            if let Some(end) = previous_end {
                let connective = if timed.range.start < end {
                    dialect.word("while")
                } else {
                    dialect.word("then")
                };
                let _ = write!(text, ", {connective} ");
            }
            text.push_str(&phrase);
            previous_end = Some(previous_end.map_or(timed.range.end, |e| e.max(timed.range.end)));
        }
        text
    };
    sentences.push(capitalize(&format!("{rig}; {movement}.")));

    // Lens.
    let lens = &shot.camera.initial_state.lens;
    let mut lens_line = format!(
        "{:.0} mm {}, f/{:.1}",
        lens.focal_length_mm,
        dialect.word("lens"),
        lens.aperture_f
    );
    let dof = if lens.aperture_f <= 2.0 {
        Some("very_shallow_dof")
    } else if lens.aperture_f <= 2.8 {
        Some("shallow_dof")
    } else if lens.aperture_f >= 8.0 {
        Some("deep_focus")
    } else {
        None
    };
    if let Some(dof) = dof {
        let _ = write!(lens_line, ", {}", dialect.word(dof));
    }
    sentences.push(format!("{lens_line}."));

    // Composition.
    let composition = &shot.framing.composition;
    let mut composition_parts = Vec::new();
    let strategy = dialect.phrase(
        &dialect.composition_strategy,
        &serde_name(&composition.strategy),
    );
    if !strategy.is_empty() {
        composition_parts.push(strategy);
    }
    if let Some(region) = composition.negative_space {
        composition_parts.push(dialect.phrase(&dialect.screen_region, &serde_name(&region)));
    }
    composition_parts.extend(composition.notes.iter().cloned());
    if !composition_parts.is_empty() {
        sentences.push(capitalize(&format!("{}.", composition_parts.join(", "))));
    }

    // Lighting.
    if let Some(setup) = shot
        .lighting_setup_id
        .as_ref()
        .and_then(|id| scene.lighting_setups.iter().find(|s| &s.id == id))
    {
        let mut line = format!("{}: {}", dialect.word("lighting"), setup.name);
        for source in &setup.sources {
            let kind = dialect.phrase(&dialect.light_kind, &serde_name(&source.kind));
            let role = dialect.phrase(&dialect.light_role, &serde_name(&source.role));
            let _ = write!(
                line,
                "; {kind} {} {role} ({}K)",
                dialect.word("as"),
                source.color_temperature_k
            );
            if let Some(motivation) = &source.motivated_by {
                let _ = write!(line, ", {} {motivation}", dialect.word("motivated_by"));
            }
        }
        for note in &setup.notes {
            let _ = write!(line, "; {note}");
        }
        sentences.push(format!("{line}."));
    }

    // Intent and beats.
    if !shot.purpose.is_empty() {
        let purposes: Vec<String> = shot
            .purpose
            .iter()
            .map(|p| dialect.phrase(&dialect.shot_purpose, &serde_name(p)))
            .collect();
        sentences.push(format!(
            "{}: {}.",
            dialect.word("intent"),
            purposes.join(", ")
        ));
    }
    for beat in &scene.beats {
        let overlaps = beat.range.start < shot.range.end && shot.range.start < beat.range.end;
        if overlaps {
            sentences.push(format!("{}: {}.", dialect.word("beat"), beat.intent));
        }
    }

    if let Some(compiled) = compiled {
        let mut phases = Vec::new();
        for phase in &compiled.phases {
            let start = phase.local_range.start as f64 / fps;
            let end = phase.local_range.end as f64 / fps;
            phases.push(format!(
                "{start:.1}-{end:.1}s {}",
                phase_prompt_name(phase.kind)
            ));
        }
        if !phases.is_empty() {
            sentences.push(format!("Timeline: {}.", phases.join("; ")));
        }
        if !compiled.prohibitions.is_empty() {
            let prohibitions: Vec<_> = compiled
                .prohibitions
                .iter()
                .map(prohibition_prompt)
                .collect();
            sentences.push(format!("Do not {}.", prohibitions.join(", do not ")));
        }
        let failed: Vec<_> = compiled
            .evaluations
            .iter()
            .filter(|evaluation| evaluation.status == ConstraintStatus::Fail)
            .map(|evaluation| {
                format!(
                    "{} must pass{}",
                    humanize_constraint_id(&evaluation.constraint_id),
                    evaluation
                        .max_error
                        .map_or(String::new(), |error| format!("; maximum error {error:.4}"))
                )
            })
            .collect();
        if !failed.is_empty() {
            sentences.push(format!("Acceptance checks: {}.", failed.join("; ")));
        }
        let passed = compiled
            .evaluations
            .iter()
            .filter(|evaluation| evaluation.status == ConstraintStatus::Pass)
            .count();
        let adapter_owned = compiled
            .evaluations
            .len()
            .saturating_sub(passed + failed.len());
        sentences.push(format!(
            "Compiled acceptance: {passed} observable checks pass; {adapter_owned} checks require a capable execution adapter."
        ));
    }

    // Notes.
    if !shot.notes.is_empty() {
        sentences.push(format!(
            "{}: {}",
            dialect.word("notes"),
            shot.notes.join(" ")
        ));
    }

    let cinematic = dialect.word("cinematic");
    if !cinematic.is_empty() {
        sentences.push(capitalize(&format!("{cinematic}.")));
    }

    sentences.join(" ")
}

/// Concise, time-ordered command for video models. It contains only compiled
/// observable results and explicit prohibitions.
pub fn emit_guidance_command(scene: &CompiledScene, shot: &CompiledShot, fps: f64) -> String {
    let mut lines = Vec::new();
    for phase in &shot.phases {
        let start = phase.local_range.start as f64 / fps.max(1.0);
        let end = phase.local_range.end as f64 / fps.max(1.0);
        let first = shot.frame_at(phase.edit_range.start);
        let last = shot.frame_at(phase.edit_range.end.saturating_sub(1));
        let camera = first.zip(last).map_or(String::new(), |(first, last)| {
            format!(
                "; camera ({:.2},{:.2},{:.2}) to ({:.2},{:.2},{:.2}), lens {:.1}mm to {:.1}mm",
                first.position.x,
                first.position.y,
                first.position.z,
                last.position.x,
                last.position.y,
                last.position.z,
                first.focal_length_mm,
                last.focal_length_mm
            )
        });
        lines.push(format!(
            "{start:.1}-{end:.1}s: {}{camera}",
            phase_prompt_name(phase.kind)
        ));
    }
    for constraint in &shot.evaluations {
        lines.push(format!(
            "constraint {}: requested {}; compiled {} ({:?})",
            constraint.constraint_id, constraint.requested, constraint.actual, constraint.status
        ));
    }
    if !shot.prohibitions.is_empty() {
        lines.push(format!(
            "prohibit: {}",
            shot.prohibitions
                .iter()
                .map(prohibition_prompt)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !shot.intent.phrases.is_empty() {
        lines.push(format!(
            "shot phrases: {}",
            shot.intent
                .phrases
                .iter()
                .map(shot_phrase_prompt)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    format!(
        "scene {} / shot {}\n{}",
        scene.id,
        shot.id,
        lines.join("\n")
    )
}

fn shot_phrase_prompt(value: &crate::model::ShotPhrase) -> &'static str {
    use crate::model::ShotPhrase;
    match value {
        ShotPhrase::PressurePushIn => "pressure push-in",
        ShotPhrase::SubjectLockDollyZoom => "subject-lock dolly zoom",
        ShotPhrase::ForegroundParallaxReveal => "foreground parallax reveal",
        ShotPhrase::VisibleAxisCross => "visible axis cross",
        ShotPhrase::RackFocusReveal => "rack-focus reveal",
        ShotPhrase::ReactionHold => "reaction hold",
        ShotPhrase::WalkAndTalkFollow => "walk-and-talk follow",
        ShotPhrase::OrbitRelationshipShift => "orbit relationship shift",
    }
}

fn shot_from_compiled(compiled: &CompiledShot) -> Shot {
    Shot {
        id: compiled.id.clone(),
        range: compiled.edit_range,
        purpose: compiled.intent.purpose.clone(),
        coverage_role: compiled.intent.coverage_role,
        framing: compiled.intent.framing.clone(),
        camera: compiled.intent.camera.clone(),
        lighting_setup_id: compiled.intent.lighting_setup_id.clone(),
        continuity: compiled.intent.continuity.clone(),
        transition_in: compiled.intent.transition_in.kind,
        transition: Some(compiled.intent.transition_in.clone()),
        relations: compiled.intent.relations.clone(),
        phrases: compiled.intent.phrases.clone(),
        prohibit: compiled.prohibitions.clone(),
        notes: compiled.intent.notes.clone(),
        metadata: Default::default(),
    }
}

fn phase_prompt_name(kind: PhaseKind) -> &'static str {
    match kind {
        PhaseKind::Hold => "hold the camera completely still",
        PhaseKind::Motion => "execute the authored physical camera move",
        PhaseKind::Focus => "move the focal plane between the authored targets",
        PhaseKind::Reveal => "increase target visibility through foreground parallax",
        PhaseKind::Settle => "stop all camera and lens motion and settle",
        PhaseKind::Mixed => "execute the coupled camera program",
    }
}

pub fn prohibition_prompt(value: &crate::model::Prohibition) -> &'static str {
    use crate::model::Prohibition;
    match value {
        Prohibition::ActorTranslationAsCameraSubstitute => "move actors to imitate camera motion",
        Prohibition::DigitalCropAsDollySubstitute => {
            "use a digital crop instead of a physical dolly"
        }
        Prohibition::BackgroundDeformationAsParallaxSubstitute => {
            "deform the background to imitate parallax"
        }
        Prohibition::GlobalBlurAsRackFocusSubstitute => {
            "use global blur instead of a focal-plane handoff"
        }
        Prohibition::HandheldMotion => "add handheld motion",
        Prohibition::UnplannedCut => "insert an unplanned cut",
    }
}

fn humanize_constraint_id(id: &str) -> String {
    id.replace(['.', '_'], " ")
}

fn bind_color(dialect: &PromptDialect, name: &str, color: NamedColor) -> String {
    dialect
        .word("color_binding")
        .replace("{name}", name)
        .replace("{color}", color.name)
}

fn operation_target(operation: &CameraOperation) -> Option<&TargetRef> {
    match operation {
        CameraOperation::LookAt { target }
        | CameraOperation::Orbit { target, .. }
        | CameraOperation::Follow { target, .. }
        | CameraOperation::RackFocus { target, .. }
        | CameraOperation::DollyZoom { target, .. }
        | CameraOperation::Reveal { target, .. } => Some(target),
        CameraOperation::Crane { look_at, .. } => look_at.as_ref(),
        _ => None,
    }
}

/// Dialect key, optional measurement text, and the focal length after the
/// operation (so successive zooms know their direction).
fn operation_key(
    operation: &CameraOperation,
    focal_length_mm: f32,
    up: crate::model::Vec3,
    measurements: bool,
) -> (&'static str, Option<String>, f32) {
    let m = |text: String| if measurements { Some(text) } else { None };
    let to_word = "{to}";
    match operation {
        CameraOperation::Hold => ("hold", None, focal_length_mm),
        CameraOperation::Pan { degrees } => (
            if *degrees >= 0.0 {
                "pan_left"
            } else {
                "pan_right"
            },
            m(format!("{:.0}°", degrees.abs())),
            focal_length_mm,
        ),
        CameraOperation::Tilt { degrees } => (
            if *degrees >= 0.0 {
                "tilt_up"
            } else {
                "tilt_down"
            },
            m(format!("{:.0}°", degrees.abs())),
            focal_length_mm,
        ),
        CameraOperation::Roll { degrees } => ("roll", m(format!("{degrees:.0}°")), focal_length_mm),
        CameraOperation::Dolly { distance_m } => (
            if *distance_m >= 0.0 {
                "dolly_in"
            } else {
                "dolly_out"
            },
            m(format!("{:.2} m", distance_m.abs())),
            focal_length_mm,
        ),
        CameraOperation::Truck { distance_m } => (
            if *distance_m >= 0.0 {
                "truck_right"
            } else {
                "truck_left"
            },
            m(format!("{:.2} m", distance_m.abs())),
            focal_length_mm,
        ),
        CameraOperation::Pedestal { distance_m } => (
            if *distance_m >= 0.0 {
                "pedestal_up"
            } else {
                "pedestal_down"
            },
            m(format!("{:.2} m", distance_m.abs())),
            focal_length_mm,
        ),
        CameraOperation::Translate { delta, .. } => (
            "translate",
            m(format!("{:.2} m", delta.length())),
            focal_length_mm,
        ),
        CameraOperation::Rotate { .. } => ("rotate", None, focal_length_mm),
        CameraOperation::LookAt { .. } => ("look_at", None, focal_length_mm),
        CameraOperation::Orbit { azimuth_deg, .. } => {
            ("orbit", m(format!("{azimuth_deg:.0}°")), focal_length_mm)
        }
        CameraOperation::Follow { .. } => ("follow", None, focal_length_mm),
        CameraOperation::Crane { delta, .. } => {
            let vertical = delta.dot(up);
            if vertical.abs() < 1e-3 {
                return (
                    "translate",
                    m(format!("{:.2} m", delta.length())),
                    focal_length_mm,
                );
            }
            (
                if vertical >= 0.0 {
                    "crane_up"
                } else {
                    "crane_down"
                },
                m(format!("{:.2} m", vertical.abs())),
                focal_length_mm,
            )
        }
        CameraOperation::Zoom { to_focal_length_mm } => (
            if *to_focal_length_mm >= focal_length_mm {
                "zoom_in"
            } else {
                "zoom_out"
            },
            m(format!("{} {to_focal_length_mm:.0} mm", to_word)),
            *to_focal_length_mm,
        ),
        CameraOperation::RackFocus { .. } => ("rack_focus", None, focal_length_mm),
        CameraOperation::DollyZoom { to_distance_m, .. } => (
            "dolly_zoom",
            m(format!("{} {to_distance_m:.1} m", to_word)),
            focal_length_mm,
        ),
        CameraOperation::HandheldNoise { .. } => ("handheld_noise", None, focal_length_mm),
        CameraOperation::Reveal {
            lateral_distance_m, ..
        } => (
            "reveal",
            m(format!("{:.2} m", lateral_distance_m.abs())),
            focal_length_mm,
        ),
    }
}

fn join_natural(items: &[String], and: &str) -> String {
    match items {
        [] => String::new(),
        [only] => only.clone(),
        [head @ .., last] => format!("{} {and} {last}", head.join(", ")),
    }
}

fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}
