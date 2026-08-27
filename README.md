# Cinematography IR

一个面向电影摄影规划、自动分镜、虚拟制片与 Agent 编排的分层中间表示（IR）。项目包含：

- 多章节中文 mdBook：系统介绍景别、构图、镜头、景深、曝光、运镜、场面调度、连续性、灯光、Coverage 与视觉叙事；
- Rust 数据模型：YAML / JSON 序列化与 JSON Schema 生成；
- `cine-ir` CLI：`validate`、`analyze`、`inspect`、`normalize`、`schema`、`solve`、`compile`、`prompt`、`view`、`render blender`、`compare`；
- 共享 Compiled Cinematography / Guidance IR：同时保存作者意图、phase、逐帧世界轨迹、屏幕空间轨迹、约束/容差、禁止条件、拍摄覆盖与剪辑关系；
- 条件信号流水线：`prompt`、`view`、`execute`、`compare` 共同消费这份编译结果，Blender 可输出 beauty / metric + normalized depth / normal / ID / flow / pose / occlusion / camera metadata；
- 连续性分析：180° 轴线、银幕方向、互反视线、30° 小角度切换；
- 正确、故意越轴、错误越轴与 Dolly Zoom 示例；
- 单元测试和可扩展的 backend/compiler 边界。

本项目不把摄影美学简化为固定公式。IR 保存的是**意图、空间、约束、执行参数和显式例外**，以便人工与 Agent 审查、修改、编译和比较镜头方案。

## 分层

```text
Narrative Intent
  戏剧问题 / 情绪节拍 / 观众状态
          |
          v
Staging & Blocking
  人物 / 道具 / 空间 / 视线 / 动作轨迹
          |
          v
Shot Design
  景别 / 机位 / 构图 / POV / Coverage role
          |
          v
Camera Program
  位姿 / 焦距 / 光圈 / 对焦 / 曝光 / 运镜操作
          |
          v
Continuity Constraints
  轴线 / 银幕方向 / 视线 / 30° / 跨轴桥接
          |
          v
Compiled Guidance IR (cine-ir compile)
  intent + phases + world/screen tracks
  constraints + tolerances + edit relations
          |
   +------+-----------+----------------+-------------+
   v                  v                v             v
prompt            view (纯)        execute       compare
时间命令           审片/控制图         shot bundle   意图验收
```

只保存 `position(t)`、`rotation(t)` 和 `focal_length(t)` 会丢失镜头为何存在、为何移动、跨轴是否故意、拉焦在转移什么注意力等信息。因此核心 IR 同时保留高层语义与低层参数。

## 项目结构

```text
cinematography-ir/
├── Cargo.toml
├── README.md
├── book.toml
├── docs/                       # 多章节 mdBook
├── examples/
│   ├── dialogue.yaml
│   ├── intentional_axis_cross.yaml
│   ├── unsafe_axis_cross.yaml
│   ├── dolly_zoom.yaml
│   ├── jaws_beach_dolly_zoom.yaml  # 8 秒经典镜头研究：主体稳定、背景压缩
│   ├── follow_handheld.yaml        # 移动 blocking + 跟拍 + 手持扰动 + 变焦
│   └── crane_reveal.yaml           # 遮挡揭示 + 拉焦 + 升降
├── schema/
├── profiles/prompt/generic.json  # prompt 方言（IR 枚举 → 摄影词汇）
├── assets/fonts/               # PNG 编码内嵌字体（Liberation Sans, OFL）
├── src/
│   ├── model.rs                # IR 类型
│   ├── validate.rs             # 结构与语义验证
│   ├── analyze.rs              # 连续性分析
│   ├── math.rs                 # 坐标系感知的基向量 / 旋转 / 投影数学
│   ├── palette.rs              # 主体确定性配色
│   ├── solve/                  # 语义运镜 → 逐帧 Solved Camera IR + 几何诊断
│   ├── compiled.rs             # 下游共享的 Compiled Guidance IR 窄腰
│   ├── prompt/                 # 文本 prompt 槽（方言 profile）
│   ├── view/                   # 纯视图层：Canvas → svg / png / ascii / html / markdown
│   ├── execute/blender/        # Blender bpy 脚本生成与多通道导出
│   ├── compare.rs              # 世界轨迹 + 屏幕结果 + phase/cut 的闭环测量
│   ├── diagnostic.rs
│   ├── io.rs
│   ├── report.rs
│   ├── lib.rs
│   └── main.rs
└── tests/                      # validation / solve / prompt / view（含 ASCII 快照）/ execute / compare
```

## 使用

```bash
cargo run -- validate examples/dialogue.yaml
cargo run -- validate examples/dialogue.yaml --deny-warnings
cargo run -- analyze examples/unsafe_axis_cross.yaml
cargo run -- inspect examples/dialogue.yaml
cargo run -- normalize examples/dialogue.yaml --output /tmp/dialogue.json
cargo run -- schema --output schema/cinematography-ir.schema.json
cargo run -- schema --compiled --output schema/compiled-guidance-ir.schema.json
cargo run -- schema --execution-profile --output schema/execution-profile.schema.json
```

机器可读诊断：

```bash
cargo run -- validate examples/unsafe_axis_cross.yaml --json
```

条件信号流水线（详见 `docs/15-conditioning-pipeline.md`）：

```bash
cargo run -- solve examples/dolly_zoom.yaml -o /tmp/dz.json          # 逐帧 Solved Camera IR
cargo run -- compile examples/jaws_beach_dolly_zoom.yaml -o /tmp/jaws-guidance.json
cargo run -- prompt examples/dialogue.yaml --shot alice_close_up     # 文本 prompt
cargo run -- view examples/dialogue.yaml --format ascii --layout strip:v
cargo run -- view examples/dialogue.yaml --format png -o /tmp/dialogue.png
cargo run -- view examples/jaws_beach_dolly_zoom.yaml --sampling phase-keyframes \
  --panel plan,frame,elevation,timeline,metrics --format svg -o /tmp/jaws-card.svg
cargo run -- view examples/dolly_zoom.yaml --intent model-control --cue-map \
  --sampling op-endpoints --layout separate --format png -o /tmp/control/
cargo run -- view examples/dolly_zoom.yaml --layout animate:12 --format html -o /tmp/previz.html
cargo run -- render blender examples/dialogue.yaml --out-dir /tmp/passes \
  --profile profiles/execution/generic-dense.json --script-only
cargo run -- compare examples/dialogue.yaml estimate.json --align similarity --temporal dtw
```

### 经典镜头基准：《大白鲨》式反向 Dolly Zoom

`examples/jaws_beach_dolly_zoom.yaml` 把《大白鲨》(1975) 海滩镜头抽象成 8 秒、无对白和演员形象的镜头语法测试。结构依据 [American Cinematographer 对该镜头的说明](https://staging.ascmag.com/articles/shot-craft-camera-movement)：相机后退、焦段变长、前景人物尺度保持稳定，背景向人物压缩。

```bash
cargo run -- validate examples/jaws_beach_dolly_zoom.yaml --deny-warnings
cargo run -- view examples/jaws_beach_dolly_zoom.yaml --layout animate:12 --format html -o /tmp/jaws-beach.html
cargo run -- render blender examples/jaws_beach_dolly_zoom.yaml \
  --out-dir /tmp/jaws-beach --passes beauty --resolution 320x180 --samples 1 \
  --blender /Applications/Blender.app/Contents/MacOS/Blender
```

验收判据：0–2 秒建立稳定空间，2–7 秒 dolly-out + zoom-in，7–8 秒停住；Observer 的投影尺度近似不变，两个 background beachgoer 明显向画面边缘扩张。`tests/solve.rs` 对主体尺度、相机距离、焦段方向和背景扩张分别做量化回归。

### 运镜组合示例：跟拍与揭示

`follow_handheld.yaml` 用移动 blocking 驱动 `follow`，再叠加带 seed 的 `handheld_noise` 和后半程 zoom；`crane_reveal.yaml` 把前景遮挡物、`reveal`、`rack_focus` 与 `crane.look_at` 编排在一个连续镜头中。两者都能通过同一份 IR 生成纯 `view` 表示与 Blender execute 脚本：

```bash
cargo run -- view examples/follow_handheld.yaml --layout animate:12 --format html -o /tmp/follow.html
cargo run -- render blender examples/follow_handheld.yaml --out-dir /tmp/follow-passes --passes depth,normal,id --script-only

cargo run -- view examples/crane_reveal.yaml --format png -o /tmp/reveal.png
cargo run -- render blender examples/crane_reveal.yaml --out-dir /tmp/reveal-passes --passes depth,normal,id,openpose --script-only
```

`reveal.occluder` 当前保留遮挡语义，但 draft solver 不计算真实遮挡边界；这项限制在示例的 `notes` 中显式声明。

构建教材：

```bash
mdbook serve --open
```

单文件版本见 `CINEMATOGRAPHY_GUIDE.md`。

## YAML 运镜结构

运镜时间使用镜头局部半开区间；操作体是显式子对象，便于严格字段校验：

```yaml
operations:
  - range: { start: 16, end: 72 }
    easing: ease_in_out
    operation:
      op: dolly
      distance_m: 0.35
```

若写成 `focal_lenght_mm`、`distnace_m` 等未知字段，反序列化会失败，不会静默使用默认值。

## 当前验证范围

结构性错误：

- 空 ID、重复 ID；
- 不存在的人物、标记、轴线与灯光引用；
- 非法帧区间、镜头重叠、运镜越出镜头时长；
- 非有限数值、负焦距、非法光圈与快门角；
- 未知字段和错误枚举值。

连续性 warning：

- 未经解释的轴线侧翻转；
- 同一主体银幕运动方向突然反转；
- 互反对话视线在画面中指向同一方向；
- 相同主体、相同景别下小于 30° 的切换。

这些是诊断启发式，不是艺术禁令。`bridge`、`spatial_reset`、`direction_change_visible` 和 `jump_cut` 用于显式表达合理或故意的例外。

## 时间与坐标约定

所有区间均为半开区间：

```text
[start, end)
```

旧式单轨文档中 `Shot.range` 是最终剪辑的场景局部帧；`TimedCameraOperation.range` 是镜头局部帧。需要覆盖拍摄、多机位、source handles 或 dissolve 时，使用可重叠的 `Take.performance_range`，再由 `EditClip.source_range → edit_range` 映射到最终剪辑。默认坐标系为右手系、Y 向上、-Z 向前、米，但文件仍显式携带坐标系统，适配器不得假设 Blender、Unreal、Unity 或 Three.js 采用同一约定。

## 明确限制

当前版本已经能编译、执行和按可观察约束验收镜头语言，但仍不是完整摄影物理或视频生成系统。尚未实现：

- 光学畸变、T-stop、镜头呼吸和真实镜头数据库；
- 完整刚体/关节动力学、复杂网格碰撞与全局 UNSAT 约束优化（`--fidelity full` 已实现 rig 速度/加速度限制和显式 proxy 碰撞）；
- 真正的多轨 NLE 与音频 J-cut/L-cut 合成（当前 `Take + EditClip + TransitionSpec` 负责编译 EDL 和 shot bundle）；
- 从几何自动推导银幕方向和视线（轴线侧已可推导并与声明值交叉校验）；
- Unreal / Three.js / USD 适配器（Blender 已有，并已在 Blender 5.2 LTS 实机验证）；
- 从生成视频估计相机轨迹的估计器（`compare` 只实现比对的一半）。

`dolly_zoom.keep_subject_frame_fraction` 与 Reveal 的可见比例/目标屏幕 Y 已按投影几何求解；Full fidelity 只在显式 `GeometryProxy` 上做碰撞，不伪装成生产级物理模拟。明确不做：角色外观身份保真（白模只需可区分，由下游另一模型负责）；视图语义层加载 `.blend` / `.usd` 资产。这些属于 solver/backend，而不是低层 Canvas。
