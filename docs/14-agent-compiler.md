# 14. Agent、求解器与执行器

Cinematography Agent 不应直接输出逐帧相机坐标。更稳健的系统是多阶段编译器。

## 14.1 流水线

```text
Script / Storyboard / Director Notes
                 |
                 v
Narrative Parser
  scenes, beats, information dependencies
                 |
                 v
Staging Planner
  subjects, marks, gaze, action events, axes
                 |
                 v
Shot Planner
  purpose, coverage, POV, framing candidates
                 |
                 v
Cinematography Constraint Solver
  lens, pose, focus, lighting, movement
                 |
                 v
Continuity Analyzer + Critic
  axis, direction, eyeline, editability
                 |
                 v
Trajectory Compiler
  sampled camera/focus/lens curves
                 |
                 v
Backend Adapter
  Blender / Unreal / Three.js / USD / robotic camera
                 |
                 v
Render / Preview / Measurement Feedback
```

每层产生可检查 artifact，避免一个模型调用同时负责叙事、几何、物理与格式正确性。

## 14.2 候选与约束

```text
Beat: "Alice notices the hidden key"

A: cut to insert -> reaction CU
B: rack focus from Alice to key
C: lateral reveal behind foreground object
D: slow push-in while key enters frame edge
```

排序依据：

```text
hard constraints:
  target visible
  no collision
  focus range feasible
  edit timeline valid

soft constraints:
  preserve axis
  minimize acceleration/jerk
  maintain composition
  match style
  control production cost
```

硬约束不可满足时，应返回冲突集合，而不是偷偷生成穿墙相机或失焦镜头。

## 14.3 Follow 求解示例

```text
Given:
  target transform T_s(t)
  desired screen position p*
  desired shot size s*
  camera dynamics limits
  collision geometry

Solve:
  camera pose T_c(t)
  focal length f(t)
  focus distance d(t)

Minimize:
  composition error
  acceleration/jerk
  obstacle penetration
  deviation from style
```

`camera.position = target + offset` 只适合最简单情况。

## 14.4 约束冲突

```text
keep 85 mm lens
keep full-body framing
camera cannot leave room
subject walks toward wall
```

这些条件可能不可同时满足。系统应指出 UNSAT core，并提出缩短焦距、改变景别、调整 blocking、切镜或允许遮挡等候选放宽方案。

## 14.5 Critic 必须看测量结果

```text
IR static checks
geometry simulation
preview render
screen-space subject tracking
focus/occlusion metrics
human/director notes
```

视觉 critic 可检测人脸裁切、lead room 翻转、意外遮挡、运动模糊、灯光穿帮和 reveal 提前泄漏。

## 14.6 决策权限

建议把规划字段分为：

```text
locked       不可修改
constrained  可在范围内求解
suggested    可替换但需解释
free         可自由探索
```

Agent 不应未经授权修改故事事实、生产安全约束或导演锁定决策。

## 14.7 Backend 边界

```rust,ignore
trait CinematographyBackend {
    fn bake_camera_plan(&self, shot: &Shot) -> Result<BakedCamera>;
    fn apply_lighting(&self, setup: &LightingSetup) -> Result<()>;
    fn render_preview(&self, range: FrameRange) -> Result<PreviewArtifact>;
}
```

Backend 负责坐标变换、单位、关键帧 API、镜头模型和资源绑定；核心 IR 不导入 Blender 或 Unreal 类型。
