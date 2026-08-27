# 15. 条件信号流水线：solve / prompt / view / execute

> 设计决策见 ADR-1115（`~/projects/docs/adr/1115-cinematography-ir-view-and-execute-layers.md`）。

Cinematography IR 的下游不是「渲染成片」，而是把镜头语言编译成**视频生成模型能吃的条件信号**。目标模型接受三类输入，IR 的三个地层恰好一一对应：

| 模型输入槽 | 消费 IR 的层 | 字段 | 实现模块 | CLI |
|---|---|---|---|---|
| ① 文本 prompt | `CompiledShot.intent + phases + constraints + prohibitions` | 时间分段命令与 negative constraints | `prompt/` | `cine-ir prompt` |
| ② 参考图 | `screen_tracks + framing + continuity` | storyboard / motion grammar / clean control | `view/`、`execute/blender` | `cine-ir view` |
| ③ 参考视频/数值控制 | `camera/subject/lens/focus/exposure/light tracks` | typed pass video + camera JSON | `execute/blender` | `cine-ir render blender` |

## 15.1 分层

```text
CineProject（语义 IR，源数据，进版本库）
        |
        v
   solve::solve_project
   CameraOperation -> CameraState[t]
        |
   compiled::compile_project
   phases + world/screen tracks + constraints
   prohibitions + take/edit relations
        |
   CompiledProject（所有新下游的共享编译产物）
        |
   +----+------------------+-------------------+----------------+
   v                       v                   v                v
prompt（纯）          view（纯）          execute（非纯）   compare（纯）
时间命令               Diagram IR → Canvas  typed shot bundle  可观察约束验收
```

归属判据固定为三问：会因几何失败吗（solve）？需要外部运行时或真实资产吗（execute）？只是把同一份信息重新编码吗（纯渲染器）。**输出会动不代表它是执行；需要外部运行时和真实资产才代表。**

## 15.2 Solved Camera IR 与 Compiled Guidance IR

`cine-ir solve` 是兼容/调试用的逐帧轨迹；`cine-ir compile` 才是下游窄腰：

```bash
cine-ir solve examples/dolly_zoom.yaml -o /tmp/dz.json
cine-ir schema --solved
cine-ir compile examples/jaws_beach_dolly_zoom.yaml -o /tmp/jaws-guidance.json
cine-ir schema --compiled
```

每帧携带 `position`、`rotation_deg`、以及世界空间的 `forward / up / right` 单位向量，消费者无需重推欧拉约定。`draft` 是确定性运动学；`full` 在 draft 结果上应用 rig 速度/加速度上限、显式 `GeometryProxy` 碰撞，并在位置修正后重新建立相机基向量。组合规则：

- 相对运镜（pan/tilt/roll/dolly/truck/pedestal/translate/rotate/crane 位移/reveal 位移）按每帧进度增量 `Δu` 沿相机**当前**轴累积，重叠运镜自然叠加；
- 目标运镜（look_at/orbit/zoom/rack_focus/dolly_zoom）在开始时快照相机，按 `u` 从快照插值到目标；区间内最后一帧到达目标，**区间结束后释放通道**，后续运镜在保持的终态上继续累积；
- `follow` 使用 `lag_half_life_s` 转换为 `alpha = 1 - 2^(-dt/half_life)`，因此 24/30/60 fps 的时间响应一致；legacy `damping` 按 24 fps 换算；
- `handheld_noise` 是确定性叠加层（seed 决定），带约 0.25 s 的淡入淡出，不回写基态；
- `dolly_zoom` 每帧按投影包围盒求焦距，使主体 bbox height 达到 `keep_subject_frame_fraction`；无可投影 subject 时才退回 `focal ∝ distance`；
- `reveal.visibility` 根据 target/occluder 投影遮挡求横移，`preserve.target_screen_y` 再求 pitch，使可见比例与屏幕 Y 都可验收。

求解同时产出几何诊断：`GEOMETRY_SUBJECT_BEHIND_CAMERA`、`GEOMETRY_SUBJECT_OUTSIDE_FOV`、`GEOMETRY_AXIS_SIDE_MISMATCH`（声明的轴线侧与几何推导不一致；正侧定义为从 `from` 看向 `to` 时相机在右手边）。

### 坐标约定

身份旋转的相机沿 `forward_axis` 看，`up_axis` 为上；右向量为 `forward × up`（右手系）或 `up × forward`（左手系）。旋转顺序 yaw → pitch → roll，并且**在任何坐标系下 `+yaw` 都向左转、`+pitch` 都向上抬**（物理右手定则）。这与 Unity 的欧拉角相反，适配 Unity 数据须取反。`forward_axis` 与 `up_axis` 平行会被校验器拒绝（`COORDINATE_SYSTEM_AXES_PARALLEL`）。

## 15.3 文本 prompt 与方言

```bash
cine-ir prompt examples/dialogue.yaml --shot alice_close_up
cine-ir prompt examples/dialogue.yaml --dialect profiles/prompt/generic.json --json
```

IR 枚举不直接进 prompt，而是经 `profiles/prompt/<model>.json` 的方言表映射为摄影词汇（`medium_close_up` → "medium close-up"）。方言文件必须覆盖全部枚举变体与 emitter 用到的短语键，加载时校验完整性。角色用**颜色进图、名字进 prompt** 的方式绑定身份："Alice (red)"，颜色与条件图中的白模颜色一致（D7a/D10）。走位关键帧的 `action` 自由文本原样进入 prompt——这是当前表达角色动作的通道。

## 15.4 View：审片图、运动语法与模型控制分开

```text
1. 时间采样  --sampling per-shot | per-beat | op-endpoints | semantic-events | phase-keyframes | constraint-extrema | cut-pairs | every:N | continuous[:fps]
2. 场景投影  --panel plan,frame,elevation,timeline,metrics
3. 符号叠加  --overlay ... 受 --intent 门控
4. 布局      --layout grid:N | strip:h|v | separate | animate[:fps]（animate:FPS 等价于 --sampling continuous:FPS）
5. 编码      --format svg | png[:scale] | ascii[:cols] | html | markdown[:cols]
```

```bash
# 人类审查：拼格 PNG，含标注、轴线、参考线与诊断
cine-ir view examples/dialogue.yaml --format png -o /tmp/dialogue.png
# 终端速览
cine-ir view examples/dialogue.yaml --format ascii --layout strip:v
# ASCII + 文字（每镜一段 prompt）
cine-ir view examples/dialogue.yaml --format markdown
# 完整审片卡：phase keyframes + global/local geometry + elevation + curves
cine-ir view examples/jaws_beach_dolly_zoom.yaml --intent storyboard \
  --sampling phase-keyframes --panel plan,frame,elevation,timeline,metrics \
  --format svg -o /tmp/jaws-card.svg
# clean control 与独立 scribble cue map；箭头不会烧进 proxy 图
cine-ir view examples/dolly_zoom.yaml --intent model-control --cue-map \
  --sampling op-endpoints --layout separate --format png -o /tmp/control/
# 动态预览：HTML 动画，可直接交给 insight-video 出 mp4
cine-ir view examples/dolly_zoom.yaml --layout animate:12 --format html -o /tmp/previz.html
```

所有编码器消费同一个 `Canvas`（`view::canvas`），因此 SVG 与 ASCII 画的是同一张图。俯视投影沿 `up_axis` 压平、以 `forward_axis` 为页面上方，不假设 Y-up；同一场景的所有面板共用一次拟合，拼格与动画帧的比例一致。

### `--intent`

| intent | 回答的问题 | 默认内容 |
|---|---|---|
| `storyboard` | 镜头从何状态到何状态 | plan/frame，可加 elevation/timeline/metrics |
| `motion` | 相机机制与画面运动如何不同 | camera path、image-motion、focus、occlusion cue |
| `continuity` | 一刀两侧的轴线/视线/方向 | cut pairs |
| `model-control` | 模型实际消费什么 | clean proxy；`--cue-map` 另产独立 scribble 图 |
| `constraints` | 哪个可观察约束超差 | constraint extrema + PASS/FAIL |
| `comparison` | 生成结果与 guidance 哪里不同 | comparison overlay 入口 |

clean model-control 中出现文字或箭头都是缺陷。只有 `ControlProfile.include_cue_map` 为真时才额外输出 cue-map artifact；cue map 没有 silhouette，不污染 beauty/depth/ID。`CueScope::Current` 是默认；`upcoming` 和 `whole-shot` 仅供显式 review summary。

### 画面运动提示（箭头）

语义图层把 `CameraCommand`、`ImageMotion`、`FocusCue`、`OcclusionCue`、`EditCue` 分开。Dolly 的 camera path 和深度相关 radial flow、Zoom 的统一缩放、Dolly Zoom 的主体 bbox lock 与背景 separation、Rack Focus 的焦平面 handoff、Reveal 的 visible-fraction 曲线不再共用一个“运动箭头”。所有语义 item 先 lower 到同一 Canvas，再编码 SVG/PNG/ASCII/HTML；Canvas 支持 opacity/group/clip/z-index/cubic path/vector field/image embedding 与 label collision。

ASCII 对非 ASCII 标签写成 `uXXXX`，不再输出 `?`。PNG 的 Latin 字体内嵌；包含中文时加载 host CJK fallback，并用不同中文字符的栅格结果回归，避免统一 missing-glyph 方框。纯几何与 Latin 产物仍保持 byte-stable。

### HTML 动画与 insight-video

`--layout animate --format html` 产出的页面自包含，根元素 `<main data-composition-id data-duration>`（时长以秒计，取自场景帧数 / 帧率，与采样密度无关），每个采样格带 `data-t` 场景时间，并实现 `window.__insightVideo.seek(t)`：在浏览器中自动播放，被 insight-video 首次 seek 后停止自播、精确定帧，满足其确定性门。`--sampling per-shot --layout animate` 也成立——每格持续到下一个镜头开始：

```bash
insight-video render /tmp/previz.html -o /tmp/previz.mp4
```

## 15.5 Blender：白模条件序列的多通道渲染器

```bash
cine-ir render blender examples/dialogue.yaml --out-dir /tmp/passes \
  --profile profiles/execution/generic-dense.json
cine-ir render blender examples/dialogue.yaml --out-dir /tmp/passes --script-only
BLENDER=/opt/blender/blender cine-ir render blender solved.json --out-dir /tmp/passes --frames 80..160
```

Blender 的角色**不是**出成片，而是渲染带最简人形骨架的黏土白模并一次导出全部主流条件通道，因此「目标模型未定」不阻塞：

| 通道 | 来源 | 编码 |
|---|---|---|
| `beauty` | Cycles 合成层 | 8-bit sRGB PNG，黑天空 + 灰地面 |
| `depth_metric` | Z pass | 32-bit float EXR，米 |
| `depth` | Z pass → Map Range | 16-bit data PNG，近白远黑；manifest 给 near/far |
| `normal` | 材质覆盖：相机空间法线 `(n+1)/2` | 16-bit data PNG |
| `id` | 材质覆盖：Object Info Color | 16-bit data PNG，每个主体一种确定性色 |
| `vector` | Vector pass | OpenEXR 32-bit |
| `openpose` | 独立 view layer 的发光骨架 | COCO-18 关节 + ControlNet 肢体配色，黑底 |
| `occlusion` | proxy mask | 16-bit mask |
| `camera` | intrinsics/extrinsics | JSON |
| `pose_keypoints` | authored pose/gaze/contact | JSON |

payload 还包括 geometry proxy、pose/gaze/contact、rig、phase、constraints/evaluations、prohibitions、screen tracks、exposure、authored lights 与 edit timeline。Beauty 使用作者灯光/曝光/白平衡/DOF/motion blur；没有灯光才使用 neutral fallback。坐标转换全部在 Rust 侧完成，Python 只接收 Blender 世界空间。

执行先渲染到 `.scene.rendering-PID`，验证文件数/格式后写最终 manifest 与 complete marker，再原子发布；失败保留 `.scene.failed-PID` 的 build.py/stdout/stderr，但不会留下貌似成功的 manifest。最终 scene bundle 含 `edit.json` 和 `shots/<id>/{guidance,prompt,negative_prompt,review,controls,metadata,manifest}`，记录 runtime/source/compiled/profile hash、seed 与 frame inventory。

无 Blender 的 CI 环境以 `tests/python/bpy_stub` 桩执行整段脚本，验证控制流与数据管线；涉及 Blender 的改动还需在 Blender 5.2 LTS 中完成真实 `bpy` 运行与输出制品解码验证。

## 15.6 闭环测量

```bash
cine-ir schema --estimated                       # 估计轨迹的 JSON 结构
cine-ir compare examples/dialogue.yaml estimate.json --align similarity
```

`compare` 同时测世界与观众可见结果：position/relative motion、forward/up/horizon、focal、full Sim(3) alignment，以及 bbox height、center drift、background scale、pair separation、optical flow、reveal visibility、focus handoff、phase/cut timing。`--temporal dtw` 可在不同生成速度之间对齐轨迹与屏幕结果，但 phase/cut timing 仍用原始 estimate 计算，不能把节奏错误对齐掉。报告按 constraint 输出 `PASS / FAIL / INDETERMINATE`，而不是只给一个 overall RMSE。

## 15.7 明确不做

- 角色外观身份保真：白模只需可区分（颜色 + 可选人名），不需可辨认；由下游另一模型的输入负责。
- 视图层加载资产：一旦读 `.blend` / `.usd`，它就不再是纯函数。
- 在几何通道上绘制箭头或文字。
