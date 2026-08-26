# 15. 条件信号流水线：solve / prompt / view / execute

> 设计决策见 ADR-1115（`~/projects/docs/adr/1115-cinematography-ir-view-and-execute-layers.md`）。

Cinematography IR 的下游不是「渲染成片」，而是把镜头语言编译成**视频生成模型能吃的条件信号**。目标模型接受三类输入，IR 的三个地层恰好一一对应：

| 模型输入槽 | 消费 IR 的层 | 字段 | 实现模块 | CLI |
|---|---|---|---|---|
| ① 文本 prompt | 上层：叙事 / 意图 | `NarrativeIntent`、`ShotPurpose`、`Beat`、`CoverageRole` | `prompt/` | `cine-ir prompt` |
| ② 参考图 | 中层：空间 / 构图 | `Framing`、`BlockingTrack`、`Subject`、`ContinuityAxis` | `view/`（纯）、`execute/blender`（单帧） | `cine-ir view` |
| ③ 参考视频 | 中层 + 下层：执行参数 | `Transform`、`LensState`、`CameraOperation` | `execute/blender`（多通道）、`view/`（HTML 动画） | `cine-ir render blender`、`cine-ir view --layout animate` |

## 15.1 分层

```text
CineProject（语义 IR，源数据，进版本库）
        |
        v
   solve::solve_project           <- 唯一可能因几何原因失败的层
   CameraOperation -> CameraState[t]
   BlockingTrack   -> Transform[t]
        |
   SolvedProject（编译产物，可落盘，不进版本库）
        |
   +----+------------------+-------------------+
   |                       |                   |
   v                       v                   v
prompt（纯）          view（纯）          execute（非纯）
文本方言 profile      Canvas → svg/png/    Blender bpy 脚本
                      ascii/html/markdown  depth/normal/id/vector/openpose
```

归属判据固定为三问：会因几何失败吗（solve）？需要外部运行时或真实资产吗（execute）？只是把同一份信息重新编码吗（纯渲染器）。**输出会动不代表它是执行；需要外部运行时和真实资产才代表。**

## 15.2 Solved Camera IR

`cine-ir solve` 把语义运镜采样为逐帧状态：

```bash
cine-ir solve examples/dolly_zoom.yaml -o /tmp/dz.json
cine-ir schema --solved
```

每帧携带 `position`、`rotation_deg`、以及世界空间的 `forward / up / right` 单位向量，消费者无需重推欧拉约定。当前只有 `draft` 精度：套用 `Easing`，求解 look-at / orbit / follow / dolly-zoom 的运动学，不做碰撞与动力学。组合规则（也是 `Fidelity::Draft` 的契约）：

- 相对运镜（pan/tilt/roll/dolly/truck/pedestal/translate/rotate/crane 位移/reveal 位移）按每帧进度增量 `Δu` 沿相机**当前**轴累积，重叠运镜自然叠加；
- 目标运镜（look_at/orbit/zoom/rack_focus/dolly_zoom）在开始时快照相机，按 `u` 从快照插值到目标；区间内最后一帧到达目标，**区间结束后释放通道**，后续运镜在保持的终态上继续累积；
- `follow` 每帧重定位：`p += (target + offset − p)·(1 − damping)`，`offset` 为世界空间；
- `handheld_noise` 是确定性叠加层（seed 决定），带约 0.25 s 的淡入淡出，不回写基态；
- `dolly_zoom` 保持 `focal ∝ distance`，暂不求解 `keep_subject_frame_fraction`。

求解同时产出几何诊断：`GEOMETRY_SUBJECT_BEHIND_CAMERA`、`GEOMETRY_SUBJECT_OUTSIDE_FOV`、`GEOMETRY_AXIS_SIDE_MISMATCH`（声明的轴线侧与几何推导不一致；正侧定义为从 `from` 看向 `to` 时相机在右手边）。

### 坐标约定

身份旋转的相机沿 `forward_axis` 看，`up_axis` 为上；右向量为 `forward × up`（右手系）或 `up × forward`（左手系）。旋转顺序 yaw → pitch → roll，并且**在任何坐标系下 `+yaw` 都向左转、`+pitch` 都向上抬**（物理右手定则）。这与 Unity 的欧拉角相反，适配 Unity 数据须取反。`forward_axis` 与 `up_axis` 平行会被校验器拒绝（`COORDINATE_SYSTEM_AXES_PARALLEL`）。

## 15.3 文本 prompt 与方言

```bash
cine-ir prompt examples/dialogue.yaml --shot alice_close_up
cine-ir prompt examples/dialogue.yaml --dialect profiles/prompt/generic.json --json
```

IR 枚举不直接进 prompt，而是经 `profiles/prompt/<model>.json` 的方言表映射为摄影词汇（`medium_close_up` → "medium close-up"）。方言文件必须覆盖全部枚举变体与 emitter 用到的短语键，加载时校验完整性。角色用**颜色进图、名字进 prompt** 的方式绑定身份："Alice (red)"，颜色与条件图中的白模颜色一致（D7a/D10）。走位关键帧的 `action` 自由文本原样进入 prompt——这是当前表达角色动作的通道。

## 15.4 视图：一条五段流水线

```text
1. 时间采样  --sampling per-shot | per-beat | op-endpoints | every:N | continuous[:fps]
2. 场景投影  --panel plan,frame        俯视机位图 | 画面构图图
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
# 条件图：每个镜头一张，仅白模几何与运动箭头，零文字
cine-ir view examples/dialogue.yaml --intent conditioning --layout separate --format png -o /tmp/cond/
# 动态预览：HTML 动画，可直接交给 insight-video 出 mp4
cine-ir view examples/dolly_zoom.yaml --layout animate:12 --format html -o /tmp/previz.html
```

所有编码器消费同一个 `Canvas`（`view::canvas`），因此 SVG 与 ASCII 画的是同一张图。俯视投影沿 `up_axis` 压平、以 `forward_axis` 为页面上方，不假设 Y-up；同一场景的所有面板共用一次拟合，拼格与动画帧的比例一致。

### `--intent`

| | `human` | `conditioning` |
|---|---|---|
| 白模几何、相机楔形 | ✅ | ✅ |
| 相机路径 / 走位箭头 / 画面运动提示 | ✅ | ✅ |
| 轴线、视线、银幕方向、参考线、标记、诊断 | ✅ | ❌（请求了也会被门控剥离） |
| 文字 | ✅ | 仅 `--labels color+name` 时的角色名 |

条件产物中出现任何其他文字都是缺陷，测试以 `Canvas::text_items() == 0` 把关。

### 画面运动提示（箭头）

单图条件路径需要箭头表达运动（图像无法原生表达时间）：dolly / zoom 为四角向内（外）箭头，pan 为顶部横向箭头，tilt 为右侧纵向箭头，truck / reveal 为底部横向箭头，pedestal / crane 为左侧纵向箭头，orbit 为顶部弧形箭头，dolly zoom 为向内角箭头加底部反向箭头。箭头只进入 2D canvas 路径，**不进入 Blender 的 depth / normal / id 通道**（几何通道会把箭头当实体）。走视频路径时运动由序列本身表达，箭头冗余。

### HTML 动画与 insight-video

`--layout animate --format html` 产出的页面自包含，根元素 `<main data-composition-id data-duration>`（时长以秒计，取自场景帧数 / 帧率，与采样密度无关），每个采样格带 `data-t` 场景时间，并实现 `window.__insightVideo.seek(t)`：在浏览器中自动播放，被 insight-video 首次 seek 后停止自播、精确定帧，满足其确定性门。`--sampling per-shot --layout animate` 也成立——每格持续到下一个镜头开始：

```bash
insight-video render /tmp/previz.html -o /tmp/previz.mp4
```

## 15.5 Blender：白模条件序列的多通道渲染器

```bash
cine-ir render blender examples/dialogue.yaml --out-dir /tmp/passes --passes beauty,depth,normal,id,openpose,vector
cine-ir render blender examples/dialogue.yaml --out-dir /tmp/passes --script-only   # 只写 build.py + manifest.json
BLENDER=/opt/blender/blender cine-ir render blender solved.json --out-dir /tmp/passes --frames 80..160
```

Blender 的角色**不是**出成片，而是渲染带最简人形骨架的黏土白模并一次导出全部主流条件通道，因此「目标模型未定」不阻塞：

| 通道 | 来源 | 编码 |
|---|---|---|
| `beauty` | Cycles 合成层 | 8-bit sRGB PNG，黑天空 + 灰地面 |
| `depth` | Z pass → Map Range | 16-bit 线性 PNG，近白远黑；`manifest.json` 给出 near/far |
| `normal` | 材质覆盖：相机空间法线 `(n+1)/2` | 8-bit 线性 PNG |
| `id` | 材质覆盖：Object Info Color | 8-bit sRGB PNG，每个主体一种调色板色，与 2D canvas 一致 |
| `vector` | Vector pass | OpenEXR 32-bit |
| `openpose` | 独立 view layer 的发光骨架 | COCO-18 关节 + ControlNet 肢体配色，黑底 |

坐标转换全部在 Rust 侧完成（`execute/blender/convert.rs`），Python 脚本只接收 Blender 世界空间的 位置 + 基向量；相机沿本地 −Z、主体面朝本地 +Y。每个镜头一台相机，用时间线标记切换。输出目录：`<out-dir>/<scene_id>/{build.py, manifest.json, <pass>/####.png}`。

本项目在没有 Blender 的机器上以 `tests/python/bpy_stub` 桩执行整段脚本来验证控制流与数据管线；真实 `bpy` API 调用需在装有 Blender 4.x 的机器上跑通。

## 15.6 闭环测量

```bash
cine-ir schema --estimated                       # 估计轨迹的 JSON 结构
cine-ir compare examples/dialogue.yaml estimate.json --align similarity
```

`compare` 把外部估计的相机轨迹（例如从生成视频恢复）与求解意图逐帧比对：位置 RMSE / 最大偏差、朝向角误差、焦距误差、运动方向一致性（逐帧位移向量余弦均值）、路径长度比。对齐方式 `none | translation | similarity`（去质心 + 均匀尺度，不对齐旋转）。估计器本身不在本项目内；这一半使「带箭头 vs 不带」「depth vs beauty」「方言 A vs B」成为有数字的 A/B。

## 15.7 明确不做

- 角色外观身份保真：白模只需可区分（颜色 + 可选人名），不需可辨认；由下游另一模型的输入负责。
- 视图层加载资产：一旦读 `.blend` / `.usd`，它就不再是纯函数。
- 在几何通道上绘制箭头或文字。
