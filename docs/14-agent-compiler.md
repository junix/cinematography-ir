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
  Compiled Guidance IR:
  phases + camera/focus/lens/exposure/light curves
  subject screen tracks + constraints + edit relations
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
// src/execute/mod.rs
trait CinematographyAdapter {
    fn capabilities(&self) -> AdapterCapabilities;
    fn stage_scene(
        &mut self,
        project: &CompiledProject,
        scene: &CompiledScene,
        profile: &ExecutionProfile,
    ) -> Result<()>;
    fn execute(&mut self, request: &ExecutionRequest, out_dir: &Path)
        -> Result<ExecutionBundle>;
}
```

运镜求解不在适配器内。`CompiledProject` 已经带有逐帧世界状态、原始语义、屏幕轨迹、约束和剪辑关系，因此 Blender 与未来 USD / Unreal 不会分别解释 Dolly Zoom、Reveal 或 phase。`AdapterCapabilities` 对 metric depth、flow、pose、lighting、focus、camera matrices、transition 和具体 `PassKind` 显式协商；不支持的格式必须失败，不能伪造。

执行输出是事务化 shot bundle，不是最终成片：先写临时目录并执行/验证 frame inventory，成功后写 manifest + complete marker 并原子发布。bundle 记录 source/compiled/profile hash、seed、runtime 版本、期望/实际帧数和 stdout/stderr；失败目录保留诊断但没有有效 manifest。每个 shot 单独携带 prompt、negative prompt、review diagram、typed controls、camera/screen tracks、constraints 与 validation，场景根目录的 `edit.json` 负责后续 EDL 组装。

`ExecutionProfile` 把文本时间线、start/end image、dense control、depth、pose、segmentation、camera API、negative prompt 与最大帧数能力同 pass encoding 绑定。仓库提供 `profiles/execution/generic-dense.json`、`image-to-video.json` 和 `camera-api.json` 示例；text-only 模型直接消费 `cine-ir prompt` 的分阶段 command，不启动 Blender。
