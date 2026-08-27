use std::collections::HashSet;

use crate::analyze::analyze_continuity;
use crate::diagnostic::{Diagnostic, ValidationReport};
use crate::model::*;

pub const SUPPORTED_SCHEMA_VERSION: &str = "0.1";

pub fn validate_project(project: &CineProject) -> ValidationReport {
    let mut report = ValidationReport::default();

    if project.schema_version != SUPPORTED_SCHEMA_VERSION {
        report.push(
            Diagnostic::warning(
                "SCHEMA_VERSION_UNSUPPORTED",
                "schema_version",
                format!(
                    "schema version '{}' is not implemented by this crate ('{}')",
                    project.schema_version, SUPPORTED_SCHEMA_VERSION
                ),
            )
            .with_hint("Migrate the document or use a compatible cine-ir binary."),
        );
    }

    if project.id.trim().is_empty() {
        report.push(Diagnostic::error(
            "PROJECT_ID_EMPTY",
            "id",
            "project id must not be empty",
        ));
    }

    if project.frame_rate.numerator == 0 || project.frame_rate.denominator == 0 {
        report.push(Diagnostic::error(
            "FRAME_RATE_INVALID",
            "frame_rate",
            "frame-rate numerator and denominator must be greater than zero",
        ));
    }

    {
        use crate::model::{AxisName, SignedAxis};
        let cs = &project.coordinate_system;
        let forward_axis = match cs.forward_axis {
            SignedAxis::PositiveX | SignedAxis::NegativeX => AxisName::X,
            SignedAxis::PositiveY | SignedAxis::NegativeY => AxisName::Y,
            SignedAxis::PositiveZ | SignedAxis::NegativeZ => AxisName::Z,
        };
        if forward_axis == cs.up_axis {
            report.push(
                Diagnostic::error(
                    "COORDINATE_SYSTEM_AXES_PARALLEL",
                    "coordinate_system.forward_axis",
                    format!(
                        "forward_axis {:?} is parallel to up_axis {:?}; every camera basis would be degenerate",
                        cs.forward_axis, cs.up_axis
                    ),
                )
                .with_hint(
                    "Set forward_axis explicitly, e.g. positive_y for a Blender-style Z-up world.",
                ),
            );
        }
    }

    if project.scenes.is_empty() {
        report.push(Diagnostic::error(
            "PROJECT_HAS_NO_SCENES",
            "scenes",
            "project must contain at least one scene",
        ));
    }

    check_unique_ids(
        project.scenes.iter().map(|scene| scene.id.as_str()),
        "scenes",
        "scene",
        &mut report,
    );

    for (index, scene) in project.scenes.iter().enumerate() {
        validate_scene(scene, index, &mut report);
        report.extend(analyze_continuity(scene).diagnostics);
    }

    report
}

fn validate_scene(scene: &Scene, scene_index: usize, report: &mut ValidationReport) {
    let base = format!("scenes[{scene_index}]");

    if scene.id.trim().is_empty() {
        report.push(Diagnostic::error(
            "SCENE_ID_EMPTY",
            format!("{base}.id"),
            "scene id must not be empty",
        ));
    }
    if scene.duration_frames == 0 {
        report.push(Diagnostic::error(
            "SCENE_DURATION_INVALID",
            format!("{base}.duration_frames"),
            "scene duration must be greater than zero",
        ));
    }
    if scene.shots.is_empty() {
        report.push(Diagnostic::error(
            "SCENE_HAS_NO_SHOTS",
            format!("{base}.shots"),
            "scene must contain at least one shot on the edit timeline",
        ));
    }

    check_unique_ids(
        scene.subjects.iter().map(|item| item.id.as_str()),
        &format!("{base}.subjects"),
        "subject",
        report,
    );
    check_unique_ids(
        scene.markers.iter().map(|item| item.id.as_str()),
        &format!("{base}.markers"),
        "marker",
        report,
    );
    check_unique_ids(
        scene.axes.iter().map(|item| item.id.as_str()),
        &format!("{base}.axes"),
        "axis",
        report,
    );
    check_unique_ids(
        scene.beats.iter().map(|item| item.id.as_str()),
        &format!("{base}.beats"),
        "beat",
        report,
    );
    check_unique_ids(
        scene.shots.iter().map(|item| item.id.as_str()),
        &format!("{base}.shots"),
        "shot",
        report,
    );
    check_unique_ids(
        scene.lighting_setups.iter().map(|item| item.id.as_str()),
        &format!("{base}.lighting_setups"),
        "lighting setup",
        report,
    );

    let subjects: HashSet<&str> = scene.subjects.iter().map(|item| item.id.as_str()).collect();
    let markers: HashSet<&str> = scene.markers.iter().map(|item| item.id.as_str()).collect();
    let axes: HashSet<&str> = scene.axes.iter().map(|item| item.id.as_str()).collect();
    let lighting: HashSet<&str> = scene
        .lighting_setups
        .iter()
        .map(|item| item.id.as_str())
        .collect();

    validate_subjects(scene, &base, report);
    validate_markers(scene, &base, report);
    validate_axes(scene, &subjects, &markers, &base, report);
    validate_blocking(scene, &subjects, &markers, &base, report);
    validate_beats(scene, &subjects, &base, report);
    validate_narrative(scene, &base, report);
    validate_lighting(scene, &base, report);
    validate_shot_timeline(scene, &base, report);

    for (index, shot) in scene.shots.iter().enumerate() {
        validate_shot(
            shot, index, scene, &subjects, &markers, &axes, &lighting, &base, report,
        );
    }
}

fn validate_subjects(scene: &Scene, base: &str, report: &mut ValidationReport) {
    for (index, subject) in scene.subjects.iter().enumerate() {
        let path = format!("{base}.subjects[{index}]");
        validate_transform(
            subject.initial_transform,
            &format!("{path}.initial_transform"),
            report,
        );
        if let Some(dimensions) = subject.dimensions_m {
            validate_vec3(dimensions, &format!("{path}.dimensions_m"), report);
            if dimensions.x <= 0.0 || dimensions.y <= 0.0 || dimensions.z <= 0.0 {
                report.push(Diagnostic::error(
                    "SUBJECT_DIMENSIONS_INVALID",
                    format!("{path}.dimensions_m"),
                    "subject dimensions must be positive on all axes",
                ));
            }
        }
    }
}

fn validate_markers(scene: &Scene, base: &str, report: &mut ValidationReport) {
    for (index, marker) in scene.markers.iter().enumerate() {
        validate_vec3(
            marker.position,
            &format!("{base}.markers[{index}].position"),
            report,
        );
    }
}

fn validate_axes(
    scene: &Scene,
    subjects: &HashSet<&str>,
    markers: &HashSet<&str>,
    base: &str,
    report: &mut ValidationReport,
) {
    for (index, axis) in scene.axes.iter().enumerate() {
        let path = format!("{base}.axes[{index}]");
        validate_target_ref(
            &axis.from,
            subjects,
            markers,
            &format!("{path}.from"),
            report,
        );
        validate_target_ref(&axis.to, subjects, markers, &format!("{path}.to"), report);
        if axis.from == axis.to {
            report.push(Diagnostic::error(
                "AXIS_DEGENERATE",
                path,
                "continuity axis endpoints must not be identical",
            ));
        }
    }
}

fn validate_blocking(
    scene: &Scene,
    subjects: &HashSet<&str>,
    markers: &HashSet<&str>,
    base: &str,
    report: &mut ValidationReport,
) {
    for (index, track) in scene.blocking.iter().enumerate() {
        let path = format!("{base}.blocking[{index}]");
        if !subjects.contains(track.subject_id.as_str()) {
            report.push(Diagnostic::error(
                "BLOCKING_SUBJECT_UNKNOWN",
                format!("{path}.subject_id"),
                format!("unknown subject '{}'", track.subject_id),
            ));
        }
        if track.keyframes.is_empty() {
            report.push(Diagnostic::warning(
                "BLOCKING_TRACK_EMPTY",
                format!("{path}.keyframes"),
                "blocking track contains no keyframes",
            ));
        }

        let mut last_frame = None;
        for (keyframe_index, keyframe) in track.keyframes.iter().enumerate() {
            let keyframe_path = format!("{path}.keyframes[{keyframe_index}]");
            if keyframe.frame > scene.duration_frames {
                report.push(Diagnostic::error(
                    "BLOCKING_FRAME_OUT_OF_RANGE",
                    format!("{keyframe_path}.frame"),
                    "blocking keyframe lies beyond scene duration",
                ));
            }
            if last_frame.is_some_and(|last| keyframe.frame <= last) {
                report.push(Diagnostic::error(
                    "BLOCKING_FRAMES_NOT_STRICTLY_INCREASING",
                    format!("{keyframe_path}.frame"),
                    "blocking keyframes must be strictly increasing",
                ));
            }
            last_frame = Some(keyframe.frame);
            validate_transform(
                keyframe.transform,
                &format!("{keyframe_path}.transform"),
                report,
            );
            if let Some(target) = &keyframe.gaze_target {
                validate_target_ref(
                    target,
                    subjects,
                    markers,
                    &format!("{keyframe_path}.gaze_target"),
                    report,
                );
            }
        }
    }
}

fn validate_beats(
    scene: &Scene,
    subjects: &HashSet<&str>,
    base: &str,
    report: &mut ValidationReport,
) {
    for (index, beat) in scene.beats.iter().enumerate() {
        let path = format!("{base}.beats[{index}]");
        validate_scene_range(
            beat.range,
            scene.duration_frames,
            &format!("{path}.range"),
            report,
        );
        for subject_id in &beat.subject_ids {
            if !subjects.contains(subject_id.as_str()) {
                report.push(Diagnostic::error(
                    "BEAT_SUBJECT_UNKNOWN",
                    format!("{path}.subject_ids"),
                    format!("unknown subject '{subject_id}'"),
                ));
            }
        }
    }
}

fn validate_narrative(scene: &Scene, base: &str, report: &mut ValidationReport) {
    for (index, beat) in scene.narrative.emotional_arc.iter().enumerate() {
        let path = format!("{base}.narrative.emotional_arc[{index}]");
        if beat.frame > scene.duration_frames {
            report.push(Diagnostic::error(
                "EMOTIONAL_BEAT_OUT_OF_RANGE",
                format!("{path}.frame"),
                "emotional beat lies beyond scene duration",
            ));
        }
        if beat
            .valence
            .is_some_and(|value| !(-1.0..=1.0).contains(&value))
        {
            report.push(Diagnostic::error(
                "EMOTIONAL_VALENCE_INVALID",
                format!("{path}.valence"),
                "valence must be in [-1, 1]",
            ));
        }
        if beat
            .arousal
            .is_some_and(|value| !(0.0..=1.0).contains(&value))
        {
            report.push(Diagnostic::error(
                "EMOTIONAL_AROUSAL_INVALID",
                format!("{path}.arousal"),
                "arousal must be in [0, 1]",
            ));
        }
    }
}

fn validate_lighting(scene: &Scene, base: &str, report: &mut ValidationReport) {
    for (index, setup) in scene.lighting_setups.iter().enumerate() {
        let path = format!("{base}.lighting_setups[{index}]");
        check_unique_ids(
            setup.sources.iter().map(|item| item.id.as_str()),
            &format!("{path}.sources"),
            "light",
            report,
        );
        if setup
            .contrast_ratio
            .is_some_and(|ratio| !ratio.is_finite() || ratio <= 0.0)
        {
            report.push(Diagnostic::error(
                "LIGHTING_CONTRAST_INVALID",
                format!("{path}.contrast_ratio"),
                "contrast ratio must be finite and positive",
            ));
        }
        for (source_index, source) in setup.sources.iter().enumerate() {
            let source_path = format!("{path}.sources[{source_index}]");
            validate_transform(
                source.transform,
                &format!("{source_path}.transform"),
                report,
            );
            if source
                .intensity_lux
                .is_some_and(|value| !value.is_finite() || value < 0.0)
            {
                report.push(Diagnostic::error(
                    "LIGHT_INTENSITY_INVALID",
                    format!("{source_path}.intensity_lux"),
                    "light intensity must be finite and non-negative",
                ));
            }
            if source.color_temperature_k == 0 {
                report.push(Diagnostic::error(
                    "LIGHT_COLOR_TEMPERATURE_INVALID",
                    format!("{source_path}.color_temperature_k"),
                    "color temperature must be greater than zero",
                ));
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn validate_shot(
    shot: &Shot,
    index: usize,
    scene: &Scene,
    subjects: &HashSet<&str>,
    markers: &HashSet<&str>,
    axes: &HashSet<&str>,
    lighting: &HashSet<&str>,
    scene_path: &str,
    report: &mut ValidationReport,
) {
    let path = format!("{scene_path}.shots[{index}]");
    validate_scene_range(
        shot.range,
        scene.duration_frames,
        &format!("{path}.range"),
        report,
    );

    if shot.purpose.is_empty() {
        report.push(Diagnostic::warning(
            "SHOT_PURPOSE_MISSING",
            format!("{path}.purpose"),
            "shot has no declared narrative or editorial purpose",
        ));
    }

    for subject_id in &shot.framing.subject_ids {
        if !subjects.contains(subject_id.as_str()) {
            report.push(Diagnostic::error(
                "FRAMING_SUBJECT_UNKNOWN",
                format!("{path}.framing.subject_ids"),
                format!("unknown subject '{subject_id}'"),
            ));
        }
    }

    if shot
        .framing
        .composition
        .headroom
        .is_some_and(|value| !normalized(value))
    {
        report.push(Diagnostic::error(
            "COMPOSITION_HEADROOM_INVALID",
            format!("{path}.framing.composition.headroom"),
            "headroom must be in [0, 1]",
        ));
    }
    if shot
        .framing
        .composition
        .lead_room
        .is_some_and(|value| !normalized(value))
    {
        report.push(Diagnostic::error(
            "COMPOSITION_LEAD_ROOM_INVALID",
            format!("{path}.framing.composition.lead_room"),
            "lead room must be in [0, 1]",
        ));
    }

    for (layer_index, layer) in shot.framing.composition.depth_layers.iter().enumerate() {
        for subject_id in &layer.subject_ids {
            if !subjects.contains(subject_id.as_str()) {
                report.push(Diagnostic::error(
                    "DEPTH_LAYER_SUBJECT_UNKNOWN",
                    format!("{path}.framing.composition.depth_layers[{layer_index}].subject_ids"),
                    format!("unknown subject '{subject_id}'"),
                ));
            }
        }
    }

    if let Some(lighting_setup_id) = &shot.lighting_setup_id {
        if !lighting.contains(lighting_setup_id.as_str()) {
            report.push(Diagnostic::error(
                "LIGHTING_SETUP_UNKNOWN",
                format!("{path}.lighting_setup_id"),
                format!("unknown lighting setup '{lighting_setup_id}'"),
            ));
        }
    }

    validate_camera_state(
        &shot.camera.initial_state,
        subjects,
        markers,
        &format!("{path}.camera.initial_state"),
        report,
    );

    let shot_duration = shot.duration_frames();
    let mut previous_operation_start = None;
    for (operation_index, timed) in shot.camera.operations.iter().enumerate() {
        let operation_path = format!("{path}.camera.operations[{operation_index}]");
        if !timed.range.is_valid() || timed.range.end > shot_duration {
            report.push(Diagnostic::error(
                "CAMERA_OPERATION_RANGE_INVALID",
                format!("{operation_path}.range"),
                format!("operation range must lie inside shot-local range [0, {shot_duration})"),
            ));
        }
        if previous_operation_start.is_some_and(|start| timed.range.start < start) {
            report.push(Diagnostic::warning(
                "CAMERA_OPERATIONS_UNSORTED",
                format!("{operation_path}.range.start"),
                "camera operations are not ordered by start frame",
            ));
        }
        previous_operation_start = Some(timed.range.start);
        validate_camera_operation(
            &timed.operation,
            subjects,
            markers,
            &format!("{operation_path}.operation"),
            report,
        );
    }

    let mut observed_axes = HashSet::new();
    for (axis_index, observation) in shot.continuity.axis_observations.iter().enumerate() {
        if !axes.contains(observation.axis_id.as_str()) {
            report.push(Diagnostic::error(
                "CONTINUITY_AXIS_UNKNOWN",
                format!("{path}.continuity.axis_observations[{axis_index}].axis_id"),
                format!("unknown continuity axis '{}'", observation.axis_id),
            ));
        }
        if !observed_axes.insert(observation.axis_id.as_str()) {
            report.push(Diagnostic::error(
                "CONTINUITY_AXIS_DUPLICATE",
                format!("{path}.continuity.axis_observations[{axis_index}]"),
                format!("axis '{}' is annotated more than once", observation.axis_id),
            ));
        }
    }

    let mut observed_directions = HashSet::new();
    for (direction_index, observation) in shot.continuity.screen_directions.iter().enumerate() {
        if !subjects.contains(observation.subject_id.as_str()) {
            report.push(Diagnostic::error(
                "SCREEN_DIRECTION_SUBJECT_UNKNOWN",
                format!("{path}.continuity.screen_directions[{direction_index}].subject_id"),
                format!("unknown subject '{}'", observation.subject_id),
            ));
        }
        if !observed_directions.insert(observation.subject_id.as_str()) {
            report.push(Diagnostic::error(
                "SCREEN_DIRECTION_DUPLICATE",
                format!("{path}.continuity.screen_directions[{direction_index}]"),
                format!(
                    "subject '{}' has more than one screen-direction annotation",
                    observation.subject_id
                ),
            ));
        }
    }

    for (eyeline_index, eyeline) in shot.continuity.eyelines.iter().enumerate() {
        let eyeline_path = format!("{path}.continuity.eyelines[{eyeline_index}]");
        if !subjects.contains(eyeline.subject_id.as_str()) {
            report.push(Diagnostic::error(
                "EYELINE_SUBJECT_UNKNOWN",
                format!("{eyeline_path}.subject_id"),
                format!("unknown subject '{}'", eyeline.subject_id),
            ));
        }
        validate_target_ref(
            &eyeline.target,
            subjects,
            markers,
            &format!("{eyeline_path}.target"),
            report,
        );
    }

    if shot
        .continuity
        .camera_azimuth_deg
        .is_some_and(|value| !value.is_finite())
    {
        report.push(Diagnostic::error(
            "CAMERA_AZIMUTH_INVALID",
            format!("{path}.continuity.camera_azimuth_deg"),
            "camera azimuth must be finite",
        ));
    }
}

fn validate_shot_timeline(scene: &Scene, scene_path: &str, report: &mut ValidationReport) {
    let mut shots: Vec<&Shot> = scene.shots.iter().collect();
    shots.sort_by_key(|shot| (shot.range.start, shot.range.end));

    if let Some(first) = shots.first() {
        if first.range.start > 0 {
            report.push(Diagnostic::warning(
                "EDIT_TIMELINE_START_GAP",
                format!("{scene_path}.shots"),
                format!(
                    "edit timeline starts at frame {}, not frame 0",
                    first.range.start
                ),
            ));
        }
    }

    for pair in shots.windows(2) {
        let previous = pair[0];
        let next = pair[1];
        if next.range.start < previous.range.end {
            report.push(Diagnostic::error(
                "EDIT_TIMELINE_OVERLAP",
                format!("{scene_path}.shots[{}->{}]", previous.id, next.id),
                format!(
                    "shots overlap by {} frame(s)",
                    previous.range.end - next.range.start
                ),
            ));
        } else if next.range.start > previous.range.end {
            report.push(Diagnostic::warning(
                "EDIT_TIMELINE_GAP",
                format!("{scene_path}.shots[{}->{}]", previous.id, next.id),
                format!(
                    "{} frame(s) are uncovered on the edit timeline",
                    next.range.start - previous.range.end
                ),
            ));
        }
    }

    if let Some(last) = shots.last() {
        if last.range.end < scene.duration_frames {
            report.push(Diagnostic::warning(
                "EDIT_TIMELINE_END_GAP",
                format!("{scene_path}.shots"),
                format!(
                    "edit timeline ends at frame {}, before scene duration {}",
                    last.range.end, scene.duration_frames
                ),
            ));
        }
    }
}

fn validate_camera_state(
    state: &CameraState,
    subjects: &HashSet<&str>,
    markers: &HashSet<&str>,
    path: &str,
    report: &mut ValidationReport,
) {
    validate_transform(state.transform, &format!("{path}.transform"), report);

    if !positive_finite(state.lens.focal_length_mm) {
        report.push(Diagnostic::error(
            "LENS_FOCAL_LENGTH_INVALID",
            format!("{path}.lens.focal_length_mm"),
            "focal length must be finite and positive",
        ));
    }
    if !positive_finite(state.lens.sensor_width_mm) {
        report.push(Diagnostic::error(
            "LENS_SENSOR_WIDTH_INVALID",
            format!("{path}.lens.sensor_width_mm"),
            "sensor width must be finite and positive",
        ));
    }
    if !positive_finite(state.lens.aperture_f) {
        report.push(Diagnostic::error(
            "LENS_APERTURE_INVALID",
            format!("{path}.lens.aperture_f"),
            "aperture f-number must be finite and positive",
        ));
    }
    if !positive_finite(state.lens.anamorphic_squeeze) {
        report.push(Diagnostic::error(
            "LENS_ANAMORPHIC_SQUEEZE_INVALID",
            format!("{path}.lens.anamorphic_squeeze"),
            "anamorphic squeeze must be finite and positive",
        ));
    }

    if let Some(target) = &state.focus.target {
        validate_target_ref(
            target,
            subjects,
            markers,
            &format!("{path}.focus.target"),
            report,
        );
    }
    if state
        .focus
        .distance_m
        .is_some_and(|value| !positive_finite(value))
    {
        report.push(Diagnostic::error(
            "FOCUS_DISTANCE_INVALID",
            format!("{path}.focus.distance_m"),
            "focus distance must be finite and positive",
        ));
    }

    if state.exposure.iso == 0 {
        report.push(Diagnostic::error(
            "EXPOSURE_ISO_INVALID",
            format!("{path}.exposure.iso"),
            "ISO must be greater than zero",
        ));
    }
    if !state.exposure.shutter_angle_deg.is_finite()
        || state.exposure.shutter_angle_deg <= 0.0
        || state.exposure.shutter_angle_deg > 360.0
    {
        report.push(Diagnostic::error(
            "EXPOSURE_SHUTTER_ANGLE_INVALID",
            format!("{path}.exposure.shutter_angle_deg"),
            "shutter angle must be in (0, 360]",
        ));
    }
    if state.exposure.white_balance_k == 0 {
        report.push(Diagnostic::error(
            "EXPOSURE_WHITE_BALANCE_INVALID",
            format!("{path}.exposure.white_balance_k"),
            "white balance must be greater than zero kelvin",
        ));
    }
    if !state.exposure.nd_stops.is_finite() || state.exposure.nd_stops < 0.0 {
        report.push(Diagnostic::error(
            "EXPOSURE_ND_INVALID",
            format!("{path}.exposure.nd_stops"),
            "ND stops must be finite and non-negative",
        ));
    }
}

fn validate_camera_operation(
    operation: &CameraOperation,
    subjects: &HashSet<&str>,
    markers: &HashSet<&str>,
    path: &str,
    report: &mut ValidationReport,
) {
    match operation {
        CameraOperation::Hold => {}
        CameraOperation::Pan { degrees }
        | CameraOperation::Tilt { degrees }
        | CameraOperation::Roll { degrees } => {
            validate_finite(*degrees, &format!("{path}.degrees"), report);
        }
        CameraOperation::Dolly { distance_m }
        | CameraOperation::Truck { distance_m }
        | CameraOperation::Pedestal { distance_m } => {
            validate_finite(*distance_m, &format!("{path}.distance_m"), report);
        }
        CameraOperation::Translate { delta, .. } => {
            validate_vec3(*delta, &format!("{path}.delta"), report);
        }
        CameraOperation::Rotate { delta_deg, .. } => {
            validate_euler(*delta_deg, &format!("{path}.delta_deg"), report);
        }
        CameraOperation::LookAt { target } => {
            validate_target_ref(target, subjects, markers, &format!("{path}.target"), report);
        }
        CameraOperation::Orbit {
            target,
            azimuth_deg,
            elevation_deg,
            radius_m,
        } => {
            validate_target_ref(target, subjects, markers, &format!("{path}.target"), report);
            validate_finite(*azimuth_deg, &format!("{path}.azimuth_deg"), report);
            validate_finite(*elevation_deg, &format!("{path}.elevation_deg"), report);
            if radius_m.is_some_and(|value| !positive_finite(value)) {
                report.push(Diagnostic::error(
                    "ORBIT_RADIUS_INVALID",
                    format!("{path}.radius_m"),
                    "orbit radius must be finite and positive",
                ));
            }
        }
        CameraOperation::Follow {
            target,
            offset,
            lag_half_life_s,
            damping,
        } => {
            validate_target_ref(target, subjects, markers, &format!("{path}.target"), report);
            validate_vec3(*offset, &format!("{path}.offset"), report);
            if !normalized(*damping) {
                report.push(Diagnostic::error(
                    "FOLLOW_DAMPING_INVALID",
                    format!("{path}.damping"),
                    "follow damping must be in [0, 1]",
                ));
            }
            if lag_half_life_s.is_some_and(|value| !positive_finite(value)) {
                report.push(
                    Diagnostic::error(
                        "CAMERA_FOLLOW_HALF_LIFE_INVALID",
                        format!("{path}.lag_half_life_s"),
                        "follow lag_half_life_s must be finite and greater than zero",
                    )
                    .with_hint("Use a time-domain half-life such as 0.18 seconds."),
                );
            }
        }
        CameraOperation::Crane { delta, look_at } => {
            validate_vec3(*delta, &format!("{path}.delta"), report);
            if let Some(target) = look_at {
                validate_target_ref(
                    target,
                    subjects,
                    markers,
                    &format!("{path}.look_at"),
                    report,
                );
            }
        }
        CameraOperation::Zoom { to_focal_length_mm } => {
            if !positive_finite(*to_focal_length_mm) {
                report.push(Diagnostic::error(
                    "ZOOM_FOCAL_LENGTH_INVALID",
                    format!("{path}.to_focal_length_mm"),
                    "target focal length must be finite and positive",
                ));
            }
        }
        CameraOperation::RackFocus {
            target,
            to_distance_m,
        } => {
            validate_target_ref(target, subjects, markers, &format!("{path}.target"), report);
            if to_distance_m.is_some_and(|value| !positive_finite(value)) {
                report.push(Diagnostic::error(
                    "RACK_FOCUS_DISTANCE_INVALID",
                    format!("{path}.to_distance_m"),
                    "target focus distance must be finite and positive",
                ));
            }
        }
        CameraOperation::DollyZoom {
            target,
            to_distance_m,
            keep_subject_frame_fraction,
        } => {
            validate_target_ref(target, subjects, markers, &format!("{path}.target"), report);
            if !positive_finite(*to_distance_m) {
                report.push(Diagnostic::error(
                    "DOLLY_ZOOM_DISTANCE_INVALID",
                    format!("{path}.to_distance_m"),
                    "target distance must be finite and positive",
                ));
            }
            if !positive_finite(*keep_subject_frame_fraction) || *keep_subject_frame_fraction > 1.0
            {
                report.push(Diagnostic::error(
                    "DOLLY_ZOOM_SUBJECT_FRACTION_INVALID",
                    format!("{path}.keep_subject_frame_fraction"),
                    "subject frame fraction must be in (0, 1]",
                ));
            }
        }
        CameraOperation::HandheldNoise {
            translation_amplitude_m,
            rotation_amplitude_deg,
            frequency_hz,
            ..
        } => {
            if !non_negative_finite(*translation_amplitude_m) {
                report.push(Diagnostic::error(
                    "HANDHELD_TRANSLATION_AMPLITUDE_INVALID",
                    format!("{path}.translation_amplitude_m"),
                    "translation amplitude must be finite and non-negative",
                ));
            }
            if !non_negative_finite(*rotation_amplitude_deg) {
                report.push(Diagnostic::error(
                    "HANDHELD_ROTATION_AMPLITUDE_INVALID",
                    format!("{path}.rotation_amplitude_deg"),
                    "rotation amplitude must be finite and non-negative",
                ));
            }
            if !positive_finite(*frequency_hz) {
                report.push(Diagnostic::error(
                    "HANDHELD_FREQUENCY_INVALID",
                    format!("{path}.frequency_hz"),
                    "frequency must be finite and positive",
                ));
            }
        }
        CameraOperation::Reveal {
            target,
            occluder,
            lateral_distance_m,
        } => {
            validate_target_ref(target, subjects, markers, &format!("{path}.target"), report);
            validate_target_ref(
                occluder,
                subjects,
                markers,
                &format!("{path}.occluder"),
                report,
            );
            validate_finite(
                *lateral_distance_m,
                &format!("{path}.lateral_distance_m"),
                report,
            );
        }
    }
}

fn validate_target_ref(
    target: &TargetRef,
    subjects: &HashSet<&str>,
    markers: &HashSet<&str>,
    path: &str,
    report: &mut ValidationReport,
) {
    match target {
        TargetRef::Subject { id } if !subjects.contains(id.as_str()) => {
            report.push(Diagnostic::error(
                "TARGET_SUBJECT_UNKNOWN",
                path,
                format!("unknown subject '{id}'"),
            ));
        }
        TargetRef::Marker { id } if !markers.contains(id.as_str()) => {
            report.push(Diagnostic::error(
                "TARGET_MARKER_UNKNOWN",
                path,
                format!("unknown marker '{id}'"),
            ));
        }
        TargetRef::Point { position } => validate_vec3(*position, path, report),
        _ => {}
    }
}

fn validate_scene_range(
    range: FrameRange,
    duration: Frame,
    path: &str,
    report: &mut ValidationReport,
) {
    if !range.is_valid() {
        report.push(Diagnostic::error(
            "FRAME_RANGE_EMPTY_OR_REVERSED",
            path,
            "frame range uses half-open semantics and must satisfy start < end",
        ));
    }
    if range.end > duration {
        report.push(Diagnostic::error(
            "FRAME_RANGE_OUT_OF_SCENE",
            path,
            format!(
                "frame range ends at {}, beyond scene duration {duration}",
                range.end
            ),
        ));
    }
}

fn validate_transform(transform: Transform, path: &str, report: &mut ValidationReport) {
    validate_vec3(transform.position, &format!("{path}.position"), report);
    validate_euler(
        transform.rotation_deg,
        &format!("{path}.rotation_deg"),
        report,
    );
    validate_vec3(transform.scale, &format!("{path}.scale"), report);
    if transform.scale.x <= 0.0 || transform.scale.y <= 0.0 || transform.scale.z <= 0.0 {
        report.push(Diagnostic::error(
            "TRANSFORM_SCALE_INVALID",
            format!("{path}.scale"),
            "transform scale must be positive on all axes",
        ));
    }
}

fn validate_vec3(value: Vec3, path: &str, report: &mut ValidationReport) {
    if !value.x.is_finite() || !value.y.is_finite() || !value.z.is_finite() {
        report.push(Diagnostic::error(
            "VECTOR_NOT_FINITE",
            path,
            "vector components must be finite numbers",
        ));
    }
}

fn validate_euler(value: EulerDeg, path: &str, report: &mut ValidationReport) {
    if !value.pitch.is_finite() || !value.yaw.is_finite() || !value.roll.is_finite() {
        report.push(Diagnostic::error(
            "ROTATION_NOT_FINITE",
            path,
            "rotation components must be finite numbers",
        ));
    }
}

fn validate_finite(value: f32, path: &str, report: &mut ValidationReport) {
    if !value.is_finite() {
        report.push(Diagnostic::error(
            "NUMBER_NOT_FINITE",
            path,
            "value must be finite",
        ));
    }
}

fn check_unique_ids<'a>(
    ids: impl IntoIterator<Item = &'a str>,
    path: &str,
    kind: &str,
    report: &mut ValidationReport,
) {
    let mut seen = HashSet::new();
    for id in ids {
        if id.trim().is_empty() {
            report.push(Diagnostic::error(
                "ID_EMPTY",
                path,
                format!("{kind} id must not be empty"),
            ));
        } else if !seen.insert(id) {
            report.push(Diagnostic::error(
                "ID_DUPLICATE",
                path,
                format!("duplicate {kind} id '{id}'"),
            ));
        }
    }
}

fn positive_finite(value: f32) -> bool {
    value.is_finite() && value > 0.0
}

fn non_negative_finite(value: f32) -> bool {
    value.is_finite() && value >= 0.0
}

fn normalized(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}
