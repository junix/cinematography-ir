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
       +-- Shot[]（兼容的单轨写法）
       +-- Take[] + EditClip[]（覆盖拍摄与最终剪辑）
             |
             +-- purpose / coverage_role
             +-- Framing
             +-- CameraPlan
             |     +-- initial_state
             |     +-- TimedCameraOperation[]
             +-- ContinuityAnnotations
             +-- ShotPhrase / ShotRelation / TransitionSpec / Prohibition
```

## 11.3 语义层与执行层

```yaml
operation:
  op: follow
  target: { type: subject, id: runner }
  offset: { x: 0.0, y: 1.5, z: -3.0 }
  lag_half_life_s: 0.18
```

执行层需要把它编译为逐帧 `CameraState`。编译依赖 blocking、碰撞体、相机动力学、构图约束和 backend 坐标系。

```text
Cinematography IR       authoring source             src/model.rs
       |
       +-- solve ----> Solved Camera IR              src/solve/
       |               compatibility/debug artifact
       v
Compiled Guidance IR   phases + semantic intent      src/compiled.rs
                       world/screen tracks
                       constraints + tolerances
                       edit relations + prohibitions
       |
       +---> prompt    time-ordered commands          src/prompt/
       +---> view      semantic diagrams -> Canvas    src/view/
       +---> execute   typed shot/control bundles     src/execute/
       +---> compare   constraint-driven acceptance   src/compare.rs
```

把逐帧轨迹直接放进作者 IR 会导致文件巨大、意图丢失、人物路径变化后轨迹失效。`SolvedProject` 只保留兼容的逐帧相机状态；真正的下游窄腰是可重建的 `CompiledProject`。同一 `ShotConstraint` 由 solver/compile 求值、view 绘制、prompt 描述、execute 打包、compare 验收，消费者不得再次“镜像 solver”解释 `CameraOperation`。

## 11.4 时间域

```text
Performance/story time: Take.performance_range = [0, 240)
Take-local operation:   operation.range = [16, 72)
Selected source:        EditClip.source_range = [80, 160)
Final edit time:        EditClip.edit_range   = [120, 200)
```

所有范围都是半开区间。Take 可以覆盖同一段表演；EditClip 负责 source frame 到最终剪辑 frame 的映射。转场由 `TransitionSpec` 携带 duration、easing、match anchor 与方向；转场重叠必须由下一 clip 的 transition duration 覆盖。

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
