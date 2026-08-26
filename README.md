# Cinematography IR

一个面向电影摄影规划、自动分镜、虚拟制片与 Agent 编排的分层中间表示（IR）。项目包含：

- 多章节中文 mdBook：系统介绍景别、构图、镜头、景深、曝光、运镜、场面调度、连续性、灯光、Coverage 与视觉叙事；
- Rust 数据模型：YAML / JSON 序列化与 JSON Schema 生成；
- `cine-ir` CLI：`validate`、`analyze`、`inspect`、`normalize`、`schema`、`solve`、`prompt`、`view`、`render blender`、`compare`；
- 条件信号流水线（ADR-1115）：语义运镜求解为逐帧 Solved Camera IR，再分别产出文本 prompt、俯视/画面白模图（svg / png / ascii / html 动画 / markdown）与 Blender 多通道条件序列（depth / normal / id / vector / openpose）；
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
Solved Camera IR (cine-ir solve)
          |
   +------+-----------+----------------+
   v                  v                v
prompt            view (纯)        execute (Blender)
文本条件           plan/frame 图     depth/normal/id/... 通道
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
│   └── dolly_zoom.yaml
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
│   ├── prompt/                 # 文本 prompt 槽（方言 profile）
│   ├── view/                   # 纯视图层：Canvas → svg / png / ascii / html / markdown
│   ├── execute/blender/        # Blender bpy 脚本生成与多通道导出
│   ├── compare.rs              # 估计轨迹 vs 求解意图（闭环测量）
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
```

机器可读诊断：

```bash
cargo run -- validate examples/unsafe_axis_cross.yaml --json
```

条件信号流水线（详见 `docs/15-conditioning-pipeline.md`）：

```bash
cargo run -- solve examples/dolly_zoom.yaml -o /tmp/dz.json          # 逐帧 Solved Camera IR
cargo run -- prompt examples/dialogue.yaml --shot alice_close_up     # 文本 prompt
cargo run -- view examples/dialogue.yaml --format ascii --layout strip:v
cargo run -- view examples/dialogue.yaml --format png -o /tmp/dialogue.png
cargo run -- view examples/dialogue.yaml --intent conditioning --layout separate --format png -o /tmp/cond/
cargo run -- view examples/dolly_zoom.yaml --layout animate:12 --format html -o /tmp/previz.html
cargo run -- render blender examples/dialogue.yaml --out-dir /tmp/passes --passes depth,normal,id --script-only
cargo run -- compare examples/dialogue.yaml estimate.json --align similarity
```

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

`Shot.range` 是场景局部帧；`TimedCameraOperation.range` 是镜头局部帧。默认坐标系为右手系、Y 向上、-Z 向前、米，但文件仍显式携带坐标系统，适配器不得假设 Blender、Unreal、Unity 或 Three.js 采用同一约定。

## 明确限制

当前版本是规划 IR、静态分析器与 draft 精度的运动学求解器，不是完整摄影物理模拟器。尚未实现：

- 光学畸变、T-stop、镜头呼吸和真实镜头数据库；
- 碰撞、遮挡、相机动力学限制与约束求解（`--fidelity full`）；
- 多机位同步拍摄与多轨编辑；
- 从几何自动推导银幕方向和视线（轴线侧已可推导并与声明值交叉校验）；
- `dolly_zoom.keep_subject_frame_fraction` 的精确求解；
- Unreal / Three.js / USD 适配器（Blender 已有，并已在 Blender 5.2 LTS 实机验证）；
- 从生成视频估计相机轨迹的估计器（`compare` 只实现比对的一半）。

明确不做：角色外观身份保真（白模只需可区分，由下游另一模型负责）；视图层加载任何资产。这些应位于 compiler/solver/backend 层，而不是继续膨胀核心交换格式。
