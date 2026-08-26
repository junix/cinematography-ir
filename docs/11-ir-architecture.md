# 11. Cinematography IR 的分层架构

Cinematography IR 不是渲染场景格式，也不是剪辑工程文件。它连接叙事规划、空间调度、镜头设计和执行器。

## 11.1 设计目标

```text
human-readable       YAML / JSON 可审查
machine-validatable  类型、引用、范围和连续性诊断
semantic             保存镜头理由与编辑职能
deterministic        随机运动携带 seed
extensible           metadata 与版本迁移
backend-neutral      不绑定 Blender / Unreal / Three.js
```

## 11.2 数据层级

```text
CineProject
  |
  +-- frame_rate / coordinate_system / metadata
  |
  +-- Scene
       |
       +-- NarrativeIntent
       +-- Subject / Marker / ContinuityAxis
       +-- BlockingTrack / Beat
       +-- LightingSetup
       +-- Shot[]
             |
             +-- purpose / coverage_role
             +-- Framing
             +-- CameraPlan
             |     +-- initial_state
             |     +-- TimedCameraOperation[]
             +-- ContinuityAnnotations
             +-- transition / notes
```

## 11.3 语义层与执行层

```yaml
operation:
  op: follow
  target: { type: subject, id: runner }
  offset: { x: 0.0, y: 1.5, z: -3.0 }
  damping: 0.2
```

执行层需要把它编译为逐帧 `CameraState`。编译依赖 blocking、碰撞体、相机动力学、构图约束和 backend 坐标系。

```text
Cinematography IR     semantic source            src/model.rs
       |
       v
Solved Camera IR      sampled/baked result       src/solve/  (`cine-ir solve`)
       |
       +---> prompt   text conditioning          src/prompt/ (`cine-ir prompt`)
       +---> view     plan/frame canvases        src/view/   (`cine-ir view`)
       +---> execute  Blender passes             src/execute/(`cine-ir render blender`)
```

把逐帧轨迹直接放进核心 IR 会导致文件巨大、意图丢失、人物路径变化后轨迹失效、不同执行器无法重新求解。`SolvedProject` 是可重建的编译产物：适配器与动画预览都只消费它，从不各自解释 `CameraOperation`。分层细节见第 15 章与 ADR-1115。

## 11.4 时间域

```text
Scene-local:
Shot.range = [80, 160)

Shot-local:
operation.range = [16, 72)

Absolute scene interval:
[96, 152)
```

半开区间避免切点重复归属。

## 11.5 引用而不是复制

```yaml
focus:
  target: { type: subject, id: alice }
```

优于把 Alice 坐标复制进镜头。执行器应根据 blocking 在每帧解析目标位置。`Marker` 适合命名的空间锚点，`Point` 适合不可移动静态点。

## 11.6 显式连续性注释

理想情况下，轴线侧和银幕方向可从几何推导。但核心 IR 仍保存声明值：

```yaml
continuity:
  axis_observations:
    - { axis_id: dialogue_axis, side: positive }
  camera_azimuth_deg: 55.0
```

原因包括：外部资产可能不在文件中；后期翻转或镜面会改变屏幕结果；主导轴是叙事选择；声明值可与几何推导值交叉验证。

## 11.7 版本与扩展

核心字段语义变化必须升级 `schema_version`。实验性信息可进入 namespaced metadata，但稳定且需要验证的字段应进入正式模型。任意 JSON blob 不是长期互操作方案。
