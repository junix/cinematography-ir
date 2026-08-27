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
  shots: ...          # 兼容的最终剪辑单轨
  takes: ...          # 可重叠的拍摄覆盖
  edit_timeline: ...  # source -> edit 映射
```

`shots` 是兼容的单轨编辑时间线，因此不允许重叠。需要 master、single、reaction 同时覆盖一段表演时，使用可重叠 `takes`；`edit_timeline` 从 take 选择 source range。当前 edit timeline 是画面单轨，转场允许显式 overlap，音频多轨仍在范围外。

## 12.3 Subject 与 TargetRef

```yaml
subjects:
  - id: alice
    name: Alice
    kind: character
    initial_transform:
      position: { x: -1.5, y: 0.0, z: 0.0 }
      rotation_deg: { pitch: 0.0, yaw: 90.0, roll: 0.0 }
    geometry_proxy: { kind: capsule_rig }
    pose_track:
      frames:
        - frame: 0
          joints: { head: { x: -1.5, y: 1.65, z: 0.0 } }
          gaze: { type: subject, id: bob }
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
  phrases: [reaction_hold]
  relations:
    - { kind: reaction_to, shot_id: bob_close_up }
  prohibit: [unplanned_cut]
```

`headroom` 与 `lead_room` 是 `[0,1]` 规范化规划值；当前版本不规定唯一测量算法，backend 应说明其屏幕空间解释。

`ShotPhrase` 位于物理 operation 和剪辑关系之间，包括 `pressure_push_in`、`subject_lock_dolly_zoom`、`foreground_parallax_reveal`、`visible_axis_cross`、`rack_focus_reveal`、`reaction_hold`、`walk_and_talk_follow` 与 `orbit_relationship_shift`。未显式填写时，compiler 会从运镜、purpose 与连续性桥接推导，但不会覆盖作者值。

## 12.6 CameraState

```yaml
initial_state:
  transform:
    position: { x: -0.65, y: 1.48, z: 2.7 }
    rotation_deg: { pitch: 0.0, yaw: 14.0, roll: 0.0 }
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

Follow 使用与帧率无关的时间常数：

```yaml
operation:
  op: follow
  target: { type: subject, id: runner }
  offset: { x: 0.0, y: 1.5, z: -3.0 }
  lag_half_life_s: 0.18
```

Reveal 可声明可观察的遮挡目标，solver 会根据 target/occluder 的投影包围盒求横移，而不是把它当作普通 truck：

```yaml
operation:
  op: reveal
  target: { type: subject, id: hero }
  occluder: { type: subject, id: doorway }
  lateral_distance_m: 1.0       # 无可见比例时的兼容 fallback
  visibility: { from: 0.05, to: 0.85, monotonic: true }
  preserve: { target_screen_y: 0.48 }
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

## 12.8 Take、EditClip 与 TransitionSpec

```yaml
takes:
  - id: master_take
    performance_range: { start: 0, end: 240 }
    coverage_role: master
    framing: { shot_size: medium, subject_ids: [alice, bob] }
    camera_program: { initial_state: { transform: {} } }
edit_timeline:
  - id: master_clip
    take_id: master_take
    source_range: { start: 24, end: 96 }
    edit_range: { start: 0, end: 72 }
    transition_in:
      kind: cut
      duration_frames: 0
      easing: linear
```

`source_range` 必须位于 take 的 performance range 内；当前不做 retime，所以 source/edit duration 必须相等。`TransitionSpec` 还可携带 `match_anchor` 和 `direction`。legacy `Shot.transition_in` 仍可读取，编译时会升级为 rich transition。

## 12.9 ContinuityAnnotations

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
