# 12. IR 字段与 YAML 示例

完整类型以 `src/model.rs` 和生成的 JSON Schema 为准。本章说明主要语义。

## 12.1 Project

```yaml
schema_version: "0.1"
id: interview_scene
title: "审讯室"
frame_rate:
  numerator: 24000
  denominator: 1001
coordinate_system:
  units: meters
  handedness: right
  up_axis: y
  forward_axis: negative_z
scenes: []
```

帧率使用有理数，避免 23.976 等速率的浮点误差。

## 12.2 Scene

```yaml
- id: interrogation
  title: "谁拿走了钥匙"
  duration_frames: 240
  narrative: ...
  subjects: ...
  markers: ...
  axes: ...
  blocking: ...
  beats: ...
  lighting_setups: ...
  shots: ...
```

`shots` 是单轨编辑时间线，因此默认不允许重叠。多机位素材与 split-screen 属于后续多轨扩展。

## 12.3 Subject 与 TargetRef

```yaml
subjects:
  - id: alice
    name: Alice
    kind: character
    initial_transform:
      position: { x: -1.5, y: 0.0, z: 0.0 }
      rotation_deg: { pitch: 0.0, yaw: 90.0, roll: 0.0 }
```

引用形式：

```yaml
{ type: subject, id: alice }
{ type: marker, id: room_center }
{ type: point, position: { x: 0.0, y: 1.4, z: 0.0 } }
```

## 12.4 ContinuityAxis

```yaml
axes:
  - id: dialogue_axis
    name: Alice–Bob relationship axis
    purpose: relationship
    from: { type: subject, id: alice }
    to: { type: subject, id: bob }
```

端点顺序定义正负侧参考方向。具体叉积符号应由几何推导器统一。

## 12.5 Shot 与 Framing

```yaml
- id: alice_close_up
  range: { start: 80, end: 160 }
  purpose: [isolate, emphasize]
  coverage_role: close_up
  framing:
    shot_size: close_up
    horizontal_angle: three_quarter
    vertical_angle: eye_level
    subject_ids: [alice]
    composition:
      strategy: rule_of_thirds
      headroom: 0.07
      lead_room: 0.28
      negative_space: right
  camera: ...
```

`headroom` 与 `lead_room` 是 `[0,1]` 规范化规划值；当前版本不规定唯一测量算法，backend 应说明其屏幕空间解释。

## 12.6 CameraState

```yaml
initial_state:
  transform:
    position: { x: -0.65, y: 1.48, z: -2.7 }
    rotation_deg: { pitch: 0.0, yaw: -14.0, roll: 0.0 }
  lens:
    focal_length_mm: 65.0
    sensor_width_mm: 36.0
    aperture_f: 2.8
    anamorphic_squeeze: 1.0
  focus:
    target: { type: subject, id: alice }
  exposure:
    iso: 800
    shutter_angle_deg: 180.0
    white_balance_k: 5000
    nd_stops: 0.0
```

## 12.7 TimedCameraOperation

```yaml
operations:
  - range: { start: 16, end: 72 }
    easing: ease_in_out
    operation:
      op: dolly
      distance_m: 0.35
```

支持：

```text
hold
pan / tilt / roll
dolly / truck / pedestal
translate / rotate
look_at
orbit / follow / crane
zoom / rack_focus / dolly_zoom
handheld_noise
reveal
```

操作可以时间重叠，例如 `follow` 与 `handheld_noise` 同时存在。求解器必须定义组合优先级；当前验证器只检查范围和参数。

## 12.8 ContinuityAnnotations

```yaml
continuity:
  axis_observations:
    - { axis_id: dialogue_axis, side: positive }
  screen_directions:
    - { subject_id: runner, direction: right }
  eyelines:
    - subject_id: alice
      target: { type: subject, id: bob }
      screen_direction: right
  camera_azimuth_deg: 55.0
  spatial_reset: false
  direction_change_visible: false
```

允许的 `bridge`：`visible_axis_cross`、`on_axis_shot`、`neutral_insert`、`reestablishing_shot`、`character_reblocking`、`deliberate_disorientation`。
