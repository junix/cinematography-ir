#!/usr/bin/env python3
"""Offline structural checks for repository fixtures and documentation.

This is a development aid, not the authoritative IR parser. The Rust model and
validator remain authoritative.
"""
from __future__ import annotations

import re
import sys
import tomllib
from pathlib import Path
from typing import Any

import yaml

ROOT = Path(__file__).resolve().parents[1]
ERRORS: list[str] = []


def fail(path: str, message: str) -> None:
    ERRORS.append(f"{path}: {message}")


def mapping(value: Any, path: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        fail(path, "expected mapping")
        return {}
    return value


def sequence(value: Any, path: str) -> list[Any]:
    if not isinstance(value, list):
        fail(path, "expected sequence")
        return []
    return value


def check_keys(
    obj: dict[str, Any],
    *,
    allowed: set[str],
    required: set[str],
    path: str,
) -> None:
    unknown = set(obj) - allowed
    missing = required - set(obj)
    if unknown:
        fail(path, f"unknown field(s): {sorted(unknown)}")
    if missing:
        fail(path, f"missing field(s): {sorted(missing)}")


def enum(value: Any, choices: set[str], path: str) -> None:
    if value not in choices:
        fail(path, f"expected one of {sorted(choices)}, got {value!r}")


def frame_range(value: Any, path: str, *, upper: int | None = None) -> tuple[int, int]:
    obj = mapping(value, path)
    check_keys(obj, allowed={"start", "end"}, required={"start", "end"}, path=path)
    start, end = obj.get("start"), obj.get("end")
    if not isinstance(start, int) or not isinstance(end, int):
        fail(path, "start/end must be integers")
        return 0, 0
    if start >= end:
        fail(path, "must satisfy start < end")
    if upper is not None and end > upper:
        fail(path, f"end {end} exceeds upper bound {upper}")
    return start, end


def vec3(value: Any, path: str) -> None:
    obj = mapping(value, path)
    check_keys(obj, allowed={"x", "y", "z"}, required=set(), path=path)
    for key, item in obj.items():
        if key in {"x", "y", "z"} and not isinstance(item, (int, float)):
            fail(f"{path}.{key}", "must be numeric")


def euler(value: Any, path: str) -> None:
    obj = mapping(value, path)
    check_keys(obj, allowed={"pitch", "yaw", "roll"}, required=set(), path=path)
    for key, item in obj.items():
        if not isinstance(item, (int, float)):
            fail(f"{path}.{key}", "must be numeric")


def transform(value: Any, path: str) -> None:
    obj = mapping(value, path)
    check_keys(
        obj,
        allowed={"position", "rotation_deg", "scale"},
        required=set(),
        path=path,
    )
    if "position" in obj:
        vec3(obj["position"], f"{path}.position")
    if "rotation_deg" in obj:
        euler(obj["rotation_deg"], f"{path}.rotation_deg")
    if "scale" in obj:
        vec3(obj["scale"], f"{path}.scale")


def target(value: Any, path: str, subjects: set[str], markers: set[str]) -> None:
    obj = mapping(value, path)
    kind = obj.get("type")
    if kind == "subject":
        check_keys(obj, allowed={"type", "id"}, required={"type", "id"}, path=path)
        if obj.get("id") not in subjects:
            fail(path, f"unknown subject {obj.get('id')!r}")
    elif kind == "marker":
        check_keys(obj, allowed={"type", "id"}, required={"type", "id"}, path=path)
        if obj.get("id") not in markers:
            fail(path, f"unknown marker {obj.get('id')!r}")
    elif kind == "point":
        check_keys(
            obj,
            allowed={"type", "position"},
            required={"type", "position"},
            path=path,
        )
        vec3(obj.get("position"), f"{path}.position")
    else:
        fail(path, f"invalid target type {kind!r}")


OP_FIELDS: dict[str, tuple[set[str], set[str]]] = {
    "hold": ({"op"}, {"op"}),
    "pan": ({"op", "degrees"}, {"op", "degrees"}),
    "tilt": ({"op", "degrees"}, {"op", "degrees"}),
    "roll": ({"op", "degrees"}, {"op", "degrees"}),
    "dolly": ({"op", "distance_m"}, {"op", "distance_m"}),
    "truck": ({"op", "distance_m"}, {"op", "distance_m"}),
    "pedestal": ({"op", "distance_m"}, {"op", "distance_m"}),
    "translate": ({"op", "delta", "space"}, {"op", "delta"}),
    "rotate": ({"op", "delta_deg", "space"}, {"op", "delta_deg"}),
    "look_at": ({"op", "target"}, {"op", "target"}),
    "orbit": (
        {"op", "target", "azimuth_deg", "elevation_deg", "radius_m"},
        {"op", "target", "azimuth_deg"},
    ),
    "follow": ({"op", "target", "offset", "damping"}, {"op", "target", "offset"}),
    "crane": ({"op", "delta", "look_at"}, {"op", "delta"}),
    "zoom": ({"op", "to_focal_length_mm"}, {"op", "to_focal_length_mm"}),
    "rack_focus": ({"op", "target", "to_distance_m"}, {"op", "target"}),
    "dolly_zoom": (
        {"op", "target", "to_distance_m", "keep_subject_frame_fraction"},
        {"op", "target", "to_distance_m"},
    ),
    "handheld_noise": (
        {
            "op",
            "translation_amplitude_m",
            "rotation_amplitude_deg",
            "frequency_hz",
            "seed",
        },
        {
            "op",
            "translation_amplitude_m",
            "rotation_amplitude_deg",
            "frequency_hz",
            "seed",
        },
    ),
    "reveal": (
        {"op", "target", "occluder", "lateral_distance_m"},
        {"op", "target", "occluder", "lateral_distance_m"},
    ),
}


def operation(value: Any, path: str, subjects: set[str], markers: set[str]) -> None:
    obj = mapping(value, path)
    op = obj.get("op")
    if op not in OP_FIELDS:
        fail(path, f"unknown operation {op!r}")
        return
    allowed, required = OP_FIELDS[op]
    check_keys(obj, allowed=allowed, required=required, path=path)
    if op in {"translate", "crane"} and "delta" in obj:
        vec3(obj["delta"], f"{path}.delta")
    if op == "rotate" and "delta_deg" in obj:
        euler(obj["delta_deg"], f"{path}.delta_deg")
    if op == "follow" and "offset" in obj:
        vec3(obj["offset"], f"{path}.offset")
    for key in ("target", "occluder", "look_at"):
        if key in obj:
            target(obj[key], f"{path}.{key}", subjects, markers)
    if "space" in obj:
        enum(obj["space"], {"world", "camera_local"}, f"{path}.space")


def camera(value: Any, path: str, subjects: set[str], markers: set[str], shot_duration: int) -> None:
    obj = mapping(value, path)
    check_keys(
        obj,
        allowed={"rig", "initial_state", "operations"},
        required={"initial_state"},
        path=path,
    )
    if "rig" in obj:
        enum(
            obj["rig"],
            {
                "locked", "tripod", "dolly", "slider", "jib", "crane", "handheld",
                "shoulder", "steadicam", "gimbal", "drone", "vehicle_mount", "virtual",
            },
            f"{path}.rig",
        )
    state = mapping(obj.get("initial_state"), f"{path}.initial_state")
    check_keys(
        state,
        allowed={"transform", "lens", "focus", "exposure"},
        required={"transform"},
        path=f"{path}.initial_state",
    )
    transform(state.get("transform"), f"{path}.initial_state.transform")
    if "lens" in state:
        lens = mapping(state["lens"], f"{path}.initial_state.lens")
        check_keys(
            lens,
            allowed={"focal_length_mm", "sensor_width_mm", "aperture_f", "anamorphic_squeeze"},
            required=set(),
            path=f"{path}.initial_state.lens",
        )
    if "focus" in state:
        focus = mapping(state["focus"], f"{path}.initial_state.focus")
        check_keys(
            focus,
            allowed={"target", "distance_m"},
            required=set(),
            path=f"{path}.initial_state.focus",
        )
        if "target" in focus:
            target(focus["target"], f"{path}.initial_state.focus.target", subjects, markers)
    if "exposure" in state:
        exposure = mapping(state["exposure"], f"{path}.initial_state.exposure")
        check_keys(
            exposure,
            allowed={"iso", "shutter_angle_deg", "white_balance_k", "nd_stops"},
            required=set(),
            path=f"{path}.initial_state.exposure",
        )

    for i, timed_value in enumerate(sequence(obj.get("operations", []), f"{path}.operations")):
        timed_path = f"{path}.operations[{i}]"
        timed = mapping(timed_value, timed_path)
        check_keys(
            timed,
            allowed={"range", "easing", "operation"},
            required={"range", "operation"},
            path=timed_path,
        )
        frame_range(timed.get("range"), f"{timed_path}.range", upper=shot_duration)
        if "easing" in timed:
            enum(
                timed["easing"],
                {"linear", "ease_in", "ease_out", "ease_in_out", "smooth_step", "hold"},
                f"{timed_path}.easing",
            )
        operation(timed.get("operation"), f"{timed_path}.operation", subjects, markers)


def framing(value: Any, path: str, subjects: set[str]) -> None:
    obj = mapping(value, path)
    check_keys(
        obj,
        allowed={
            "shot_size", "horizontal_angle", "vertical_angle", "roll_deg", "subject_ids",
            "composition",
        },
        required={"shot_size"},
        path=path,
    )
    enum(
        obj.get("shot_size"),
        {
            "extreme_long", "long", "medium_long", "medium", "medium_close_up",
            "close_up", "extreme_close_up", "insert", "custom",
        },
        f"{path}.shot_size",
    )
    if "horizontal_angle" in obj:
        enum(
            obj["horizontal_angle"],
            {
                "frontal", "three_quarter", "profile", "rear_three_quarter", "rear",
                "point_of_view", "over_the_shoulder", "custom",
            },
            f"{path}.horizontal_angle",
        )
    if "vertical_angle" in obj:
        enum(
            obj["vertical_angle"],
            {"overhead", "high", "eye_level", "low", "worms_eye", "custom"},
            f"{path}.vertical_angle",
        )
    for subject_id in sequence(obj.get("subject_ids", []), f"{path}.subject_ids"):
        if subject_id not in subjects:
            fail(f"{path}.subject_ids", f"unknown subject {subject_id!r}")
    if "composition" in obj:
        composition = mapping(obj["composition"], f"{path}.composition")
        check_keys(
            composition,
            allowed={"strategy", "headroom", "lead_room", "negative_space", "depth_layers", "notes"},
            required=set(),
            path=f"{path}.composition",
        )
        if "strategy" in composition:
            enum(
                composition["strategy"],
                {
                    "centered", "rule_of_thirds", "symmetric", "asymmetric", "golden_ratio",
                    "diagonal", "frame_within_frame", "deep_focus", "flat", "natural",
                },
                f"{path}.composition.strategy",
            )
        if "negative_space" in composition:
            enum(
                composition["negative_space"],
                {"left", "right", "top", "bottom", "center"},
                f"{path}.composition.negative_space",
            )
        for i, layer_value in enumerate(sequence(composition.get("depth_layers", []), f"{path}.composition.depth_layers")):
            layer_path = f"{path}.composition.depth_layers[{i}]"
            layer = mapping(layer_value, layer_path)
            check_keys(
                layer,
                allowed={"name", "role", "subject_ids"},
                required={"name", "role"},
                path=layer_path,
            )
            enum(layer.get("role"), {"foreground", "midground", "background"}, f"{layer_path}.role")
            for subject_id in sequence(layer.get("subject_ids", []), f"{layer_path}.subject_ids"):
                if subject_id not in subjects:
                    fail(f"{layer_path}.subject_ids", f"unknown subject {subject_id!r}")


def continuity(value: Any, path: str, subjects: set[str], markers: set[str], axes: set[str]) -> None:
    obj = mapping(value, path)
    check_keys(
        obj,
        allowed={
            "axis_observations", "screen_directions", "eyelines", "camera_azimuth_deg",
            "bridge", "spatial_reset", "direction_change_visible",
        },
        required=set(),
        path=path,
    )
    for i, obs_value in enumerate(sequence(obj.get("axis_observations", []), f"{path}.axis_observations")):
        obs_path = f"{path}.axis_observations[{i}]"
        obs = mapping(obs_value, obs_path)
        check_keys(obs, allowed={"axis_id", "side"}, required={"axis_id", "side"}, path=obs_path)
        if obs.get("axis_id") not in axes:
            fail(obs_path, f"unknown axis {obs.get('axis_id')!r}")
        enum(obs.get("side"), {"positive", "negative", "on_axis"}, f"{obs_path}.side")
    for i, item_value in enumerate(sequence(obj.get("screen_directions", []), f"{path}.screen_directions")):
        item_path = f"{path}.screen_directions[{i}]"
        item = mapping(item_value, item_path)
        check_keys(item, allowed={"subject_id", "direction"}, required={"subject_id", "direction"}, path=item_path)
        if item.get("subject_id") not in subjects:
            fail(item_path, f"unknown subject {item.get('subject_id')!r}")
        enum(item.get("direction"), {"left", "right", "stationary", "toward_camera", "away_from_camera"}, f"{item_path}.direction")
    for i, line_value in enumerate(sequence(obj.get("eyelines", []), f"{path}.eyelines")):
        line_path = f"{path}.eyelines[{i}]"
        line = mapping(line_value, line_path)
        check_keys(line, allowed={"subject_id", "target", "screen_direction"}, required={"subject_id", "target", "screen_direction"}, path=line_path)
        if line.get("subject_id") not in subjects:
            fail(line_path, f"unknown subject {line.get('subject_id')!r}")
        target(line.get("target"), f"{line_path}.target", subjects, markers)
        enum(line.get("screen_direction"), {"left", "right", "center"}, f"{line_path}.screen_direction")
    if "bridge" in obj:
        enum(
            obj["bridge"],
            {
                "visible_axis_cross", "on_axis_shot", "neutral_insert", "reestablishing_shot",
                "character_reblocking", "deliberate_disorientation",
            },
            f"{path}.bridge",
        )


def project(value: Any, path: str) -> None:
    obj = mapping(value, path)
    check_keys(
        obj,
        allowed={"schema_version", "id", "title", "frame_rate", "coordinate_system", "metadata", "scenes"},
        required={"schema_version", "id", "title", "frame_rate", "scenes"},
        path=path,
    )
    rate = mapping(obj.get("frame_rate"), f"{path}.frame_rate")
    check_keys(rate, allowed={"numerator", "denominator"}, required={"numerator"}, path=f"{path}.frame_rate")
    if "coordinate_system" in obj:
        coords = mapping(obj["coordinate_system"], f"{path}.coordinate_system")
        check_keys(coords, allowed={"units", "handedness", "up_axis", "forward_axis"}, required=set(), path=f"{path}.coordinate_system")
    scene_ids: set[str] = set()
    for scene_index, scene_value in enumerate(sequence(obj.get("scenes"), f"{path}.scenes")):
        scene_path = f"{path}.scenes[{scene_index}]"
        scene = mapping(scene_value, scene_path)
        check_keys(
            scene,
            allowed={
                "id", "title", "duration_frames", "narrative", "subjects", "markers", "axes",
                "blocking", "beats", "lighting_setups", "shots", "metadata",
            },
            required={"id", "title", "duration_frames", "shots"},
            path=scene_path,
        )
        scene_id = scene.get("id")
        if scene_id in scene_ids:
            fail(scene_path, f"duplicate scene id {scene_id!r}")
        scene_ids.add(scene_id)
        duration = scene.get("duration_frames")
        if not isinstance(duration, int) or duration <= 0:
            fail(f"{scene_path}.duration_frames", "must be positive integer")
            duration = 0

        subject_values = sequence(scene.get("subjects", []), f"{scene_path}.subjects")
        subjects: set[str] = set()
        for i, subject_value in enumerate(subject_values):
            subject_path = f"{scene_path}.subjects[{i}]"
            subject = mapping(subject_value, subject_path)
            check_keys(subject, allowed={"id", "name", "kind", "initial_transform", "dimensions_m", "metadata"}, required={"id", "name", "kind"}, path=subject_path)
            enum(subject.get("kind"), {"character", "prop", "vehicle", "environment", "screen", "abstract"}, f"{subject_path}.kind")
            if subject.get("id") in subjects:
                fail(subject_path, f"duplicate subject id {subject.get('id')!r}")
            subjects.add(subject.get("id"))
            if "initial_transform" in subject:
                transform(subject["initial_transform"], f"{subject_path}.initial_transform")
            if "dimensions_m" in subject:
                vec3(subject["dimensions_m"], f"{subject_path}.dimensions_m")

        markers: set[str] = set()
        for i, marker_value in enumerate(sequence(scene.get("markers", []), f"{scene_path}.markers")):
            marker_path = f"{scene_path}.markers[{i}]"
            marker = mapping(marker_value, marker_path)
            check_keys(marker, allowed={"id", "name", "position"}, required={"id", "name", "position"}, path=marker_path)
            markers.add(marker.get("id"))
            vec3(marker.get("position"), f"{marker_path}.position")

        axes: set[str] = set()
        for i, axis_value in enumerate(sequence(scene.get("axes", []), f"{scene_path}.axes")):
            axis_path = f"{scene_path}.axes[{i}]"
            axis = mapping(axis_value, axis_path)
            check_keys(axis, allowed={"id", "name", "purpose", "from", "to"}, required={"id", "name", "purpose", "from", "to"}, path=axis_path)
            axes.add(axis.get("id"))
            enum(axis.get("purpose"), {"relationship", "movement", "eyeline", "geography", "custom"}, f"{axis_path}.purpose")
            target(axis.get("from"), f"{axis_path}.from", subjects, markers)
            target(axis.get("to"), f"{axis_path}.to", subjects, markers)

        lighting_ids: set[str] = set()
        for i, setup_value in enumerate(sequence(scene.get("lighting_setups", []), f"{scene_path}.lighting_setups")):
            setup_path = f"{scene_path}.lighting_setups[{i}]"
            setup = mapping(setup_value, setup_path)
            check_keys(setup, allowed={"id", "name", "contrast_ratio", "sources", "notes"}, required={"id", "name"}, path=setup_path)
            lighting_ids.add(setup.get("id"))
            for j, source_value in enumerate(sequence(setup.get("sources", []), f"{setup_path}.sources")):
                source_path = f"{setup_path}.sources[{j}]"
                source = mapping(source_value, source_path)
                check_keys(source, allowed={"id", "kind", "role", "transform", "intensity_lux", "color_temperature_k", "motivated_by"}, required={"id", "kind", "role", "transform"}, path=source_path)
                enum(source.get("kind"), {"point", "spot", "area", "directional", "practical", "environment", "emissive_surface"}, f"{source_path}.kind")
                enum(source.get("role"), {"key", "fill", "back", "rim", "background", "practical", "ambient", "motivated"}, f"{source_path}.role")
                transform(source.get("transform"), f"{source_path}.transform")

        for i, track_value in enumerate(sequence(scene.get("blocking", []), f"{scene_path}.blocking")):
            track_path = f"{scene_path}.blocking[{i}]"
            track = mapping(track_value, track_path)
            check_keys(track, allowed={"subject_id", "keyframes"}, required={"subject_id", "keyframes"}, path=track_path)
            if track.get("subject_id") not in subjects:
                fail(track_path, f"unknown subject {track.get('subject_id')!r}")
            last = -1
            for j, key_value in enumerate(sequence(track.get("keyframes"), f"{track_path}.keyframes")):
                key_path = f"{track_path}.keyframes[{j}]"
                key = mapping(key_value, key_path)
                check_keys(key, allowed={"frame", "transform", "gaze_target", "action"}, required={"frame", "transform"}, path=key_path)
                frame = key.get("frame")
                if not isinstance(frame, int) or frame <= last or frame > duration:
                    fail(f"{key_path}.frame", "must be strictly increasing and within scene")
                if isinstance(frame, int):
                    last = frame
                transform(key.get("transform"), f"{key_path}.transform")
                if "gaze_target" in key:
                    target(key["gaze_target"], f"{key_path}.gaze_target", subjects, markers)

        for i, beat_value in enumerate(sequence(scene.get("beats", []), f"{scene_path}.beats")):
            beat_path = f"{scene_path}.beats[{i}]"
            beat = mapping(beat_value, beat_path)
            check_keys(beat, allowed={"id", "range", "intent", "emphasis", "subject_ids"}, required={"id", "range", "intent"}, path=beat_path)
            frame_range(beat.get("range"), f"{beat_path}.range", upper=duration)
            if "emphasis" in beat:
                enum(beat["emphasis"], {"low", "medium", "high", "climax"}, f"{beat_path}.emphasis")

        shot_intervals: list[tuple[int, int, str]] = []
        for i, shot_value in enumerate(sequence(scene.get("shots"), f"{scene_path}.shots")):
            shot_path = f"{scene_path}.shots[{i}]"
            shot = mapping(shot_value, shot_path)
            check_keys(
                shot,
                allowed={"id", "range", "purpose", "coverage_role", "framing", "camera", "lighting_setup_id", "continuity", "transition_in", "notes", "metadata"},
                required={"id", "range", "framing", "camera"},
                path=shot_path,
            )
            start, end = frame_range(shot.get("range"), f"{shot_path}.range", upper=duration)
            shot_intervals.append((start, end, str(shot.get("id"))))
            for purpose in sequence(shot.get("purpose", []), f"{shot_path}.purpose"):
                enum(purpose, {"establish", "orient", "reveal", "emphasize", "observe", "identify", "connect", "isolate", "subjective", "reaction", "transition", "disorient"}, f"{shot_path}.purpose")
            if "coverage_role" in shot:
                enum(shot["coverage_role"], {"master", "two_shot", "over_the_shoulder", "single", "close_up", "point_of_view", "reaction", "insert", "cutaway", "detail", "establishing", "custom"}, f"{shot_path}.coverage_role")
            framing(shot.get("framing"), f"{shot_path}.framing", subjects)
            camera(shot.get("camera"), f"{shot_path}.camera", subjects, markers, end - start)
            if "lighting_setup_id" in shot and shot["lighting_setup_id"] not in lighting_ids:
                fail(f"{shot_path}.lighting_setup_id", f"unknown lighting setup {shot['lighting_setup_id']!r}")
            if "continuity" in shot:
                continuity(shot["continuity"], f"{shot_path}.continuity", subjects, markers, axes)

        shot_intervals.sort()
        for previous, current in zip(shot_intervals, shot_intervals[1:]):
            if current[0] != previous[1]:
                fail(scene_path, f"edit timeline discontinuity between {previous[2]!r} and {current[2]!r}")
        if shot_intervals and (shot_intervals[0][0] != 0 or shot_intervals[-1][1] != duration):
            fail(scene_path, "shots do not cover complete scene duration")


def rust_balanced(path: Path) -> None:
    text = path.read_text()
    stack: list[tuple[str, int]] = []
    pairs = {')': '(', ']': '[', '}': '{'}
    i = 0
    n = len(text)
    while i < n:
        ch = text[i]
        if text.startswith('//', i):
            i = text.find('\n', i)
            if i == -1:
                break
            continue
        if text.startswith('/*', i):
            depth = 1
            i += 2
            while i < n and depth:
                if text.startswith('/*', i):
                    depth += 1
                    i += 2
                elif text.startswith('*/', i):
                    depth -= 1
                    i += 2
                else:
                    i += 1
            if depth:
                fail(str(path), "unterminated block comment")
            continue
        if ch == 'r':
            match = re.match(r'r(#+)?"', text[i:])
            if match:
                hashes = match.group(1) or ''
                i += len(match.group(0))
                end = '"' + hashes
                j = text.find(end, i)
                if j == -1:
                    fail(str(path), "unterminated raw string")
                    break
                i = j + len(end)
                continue
        if ch == '"':
            i += 1
            escaped = False
            while i < n:
                if not escaped and text[i] == '"':
                    i += 1
                    break
                if not escaped and text[i] == '\\':
                    escaped = True
                else:
                    escaped = False
                i += 1
            continue
        if ch == "'":
            # Lifetimes such as 'a are not character literals.
            if i + 2 < n and text[i + 1].isalpha() and text[i + 2] not in {"'", "\\"}:
                i += 1
                continue
            i += 1
            escaped = False
            while i < n:
                if not escaped and text[i] == "'":
                    i += 1
                    break
                if not escaped and text[i] == '\\':
                    escaped = True
                else:
                    escaped = False
                i += 1
            continue
        if ch in '([{':
            stack.append((ch, i))
        elif ch in ')]}':
            if not stack or stack[-1][0] != pairs[ch]:
                fail(str(path), f"unmatched {ch!r} at byte {i}")
                return
            stack.pop()
        i += 1
    if stack:
        fail(str(path), f"unclosed delimiter(s): {stack[-5:]}")


def main() -> int:
    with (ROOT / "Cargo.toml").open("rb") as handle:
        cargo = tomllib.load(handle)
    if cargo.get("package", {}).get("edition") != "2021":
        fail("Cargo.toml", "expected Rust 2021 edition")

    summary = (ROOT / "docs/SUMMARY.md").read_text()
    links = re.findall(r"\]\(([^)#]+\.md)\)", summary)
    for link in links:
        if not (ROOT / "docs" / link).exists():
            fail("docs/SUMMARY.md", f"missing linked chapter {link}")
    if len(links) < 10:
        fail("docs/SUMMARY.md", "expected at least 10 chapters")

    for yaml_path in sorted((ROOT / "examples").glob("*.yaml")):
        with yaml_path.open() as handle:
            value = yaml.safe_load(handle)
        project(value, str(yaml_path.relative_to(ROOT)))

    for rust_path in sorted(ROOT.glob("src/*.rs")) + sorted(ROOT.glob("tests/*.rs")):
        rust_balanced(rust_path)

    if ERRORS:
        print("STATIC CHECK FAILED", file=sys.stderr)
        for item in ERRORS:
            print(f"- {item}", file=sys.stderr)
        return 1

    print(f"STATIC CHECK OK: {len(links)} docs chapters, 4 YAML examples, Rust delimiters balanced")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
