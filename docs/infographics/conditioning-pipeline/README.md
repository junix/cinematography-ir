# 条件信号流水线信息图（conditioning-pipeline infographic）

`conditioning-pipeline.svg` / `.png`：Cinematography IR 的 solve → compile → prompt / view / execute / compare 执行链总览。
`build.py` 是唯一的规范源（canonical source）——改文案/布局请改它，不要手改 SVG。

## 构建

```bash
python3 docs/infographics/conditioning-pipeline/build.py \
  --out docs/infographics/conditioning-pipeline/conditioning-pipeline.svg
rsvg-convert -w 2560 docs/infographics/conditioning-pipeline/conditioning-pipeline.svg \
  -o docs/infographics/conditioning-pipeline/conditioning-pipeline.png
```

依赖：Python 3（仅标准库）、`rsvg-convert`（librsvg ≥ 2.62）、字体 PingFang SC / Menlo（macOS 系统自带）。
`build.py` 内置逐行 `fit()` 宽度守卫与槽位馈线走廊重叠自检：任何文案改动导致的溢出会在构建期断言失败，而不是悄悄画出边界。

## 视觉契约

| 项 | 内容 |
|---|---|
| 受众 | 阅读 `docs/15-conditioning-pipeline.md` 的工程师、Agent 编排与视频生成管线开发者 |
| 论点 | 一份镜头 YAML 经 solve + compile 收敛为唯一编译产物（Compiled Guidance IR 窄腰），再扇出为视频生成模型的三类条件信号，并由 compare 按可观察约束闭环验收 |
| 读者问题 | 四个下游各消费什么？纯 / 非纯边界在哪？验收如何闭环？ |
| 叙事顺序 | A 编译与扇出（主角：窄腰色带）→ B 窄腰解剖（CompiledShot 内容）→ C 闭环验收（compare 双测 + Jaws 基准） |
| 语言 | 简体中文；CLI 命令、枚举、通道名保留原文 |
| 画布 | `long` 预设：宽 1280 固定、高按内容生长（1780）；PNG 证明 @2x = 2560 × 3560 |
| 后端 | 手绘叙事 SVG（原生文本）；无交互、无网络依赖、无外部图像 |
| 语义 ID | `header` / `panel-a` / `pa-cineproject` / `pa-waist` / `pa-prompt…` / `pa-slot-*` / `pa-estimator` / `panel-b` / `pb-*` / `panel-c` / `pc-*` / `footer` |

## 事实来源（evidence table）

| 图中事实 | 来源 |
|---|---|
| 四条下游 prompt / view / execute / compare 及纯 / 非纯归属 | `docs/15-conditioning-pipeline.md` §15.1 |
| 三类模型输入槽 ① 文本 ② 参考图 ③ 参考视频 / 数值控制 及其与 IR 地层的对应 | `docs/15` 开篇表 |
| solve：`CameraOperation → CameraState[t]`，draft / full，几何诊断 ×3 | `docs/15` §15.2 |
| compile 产物字段（phases / 轨道 / 约束 / 禁止 / take / edit） | `src/compiled.rs` `CompiledShot` |
| 约束 kind ×19、PhaseKind ×6、判定 PASS / FAIL / INDETERMINATE | `src/compiled.rs`（枚举点数） |
| prohibitions 自动派生规则（dolly_zoom +4、reveal +1、rack_focus +1） | `src/compiled.rs` `compile_shot` |
| 世界 / 屏幕双空间轨道，compare 双侧测量 | `src/compiled.rs` + `docs/15` §15.6 |
| Jaws 基准：8 s @24fps（192 帧）、0–2/2–7/7–8 三拍、3.8→7.0 m、bbox ≈ 0.42、focal 21.8 mm 起 | `examples/jaws_beach_dolly_zoom.yaml` + `README.md` |
| 轨迹估计器未实现（compare 只做了比对的一半） | `README.md` 明确限制 |
| schema 版本 `compiled-guidance-0.1` | `src/compiled.rs` `COMPILED_SCHEMA_VERSION` |

基线 commit：`5903281`（2026-08）。

## 验证清单（2026-08-28 执行）

- `svg-linter 0.1.0` 客观错误门禁（`svg/duplicate-id,svg/dangling-reference`，`--require-complete --fail-on error`）：exit 0，0 findings，2 规则全量评估
- 卫生审查（`svg/excessive-path-complexity,svg/unused-definition,svg/external-resource,svg/raster-upscale,svg/open-filled-path`）：0 findings
- 碰撞审查（`svg/line-overlap,svg/line-crossing,svg/line-text-overlap,svg/text-text-overlap,svg/canvas-edge-clearance,svg/ambiguous-junction,svg/connector-through-shape,svg/edge-congestion,svg/filter-clipping-risk,svg/low-contrast-text`）：0 findings（首轮 9 项，逐条定位修复后归零）
- 字体度量校准：估算器 vs Hiragino Sans GB / Menlo 实测（PIL），139 条文本中位偏差 0.0%，全部在 ±6% 内且守卫余量充足
- 视觉复核：未标注 PNG（全图 + A/B/C 三面板裁切）与编号 lint PNG 均过审；灰度证明下纯 / 非纯（线型 + 文字）与三态判定（描边 + 标签）语义保留
- 处置记录：首轮 9 项碰撞候选中 6 项为真实缺陷（execute 两支线走廊共线、边界说明被箭头穿过、slot 卡内 7px 基线擦碰、kicker 对比度 3.03、FAIL 芯片对比度、空填充芯片），已全部在 `build.py` 修复并复检；其余 3 项经坐标核算为评估器/降采样噪声，逐条排除
- SVG SHA-256：`29af8dfbac31d97ee2147f5cb27ad299a39ef39b747a9b4d36b381b49cbed7bb`

隔离区声明：本 SVG 为原生文本（非 path-font），路径字体隔离区不适用；未使用 CSS 自定义属性，`var()` 隔离区不适用。
