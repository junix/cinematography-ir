# 13. 验证、诊断与失败模式

验证器区分 Error 与 Warning：

```text
Error   文件无法可靠解释或执行
Warning 文件可解释，但存在连续性、可剪辑性或设计风险
```

艺术选择不应伪装成类型错误。

## 13.1 结构验证

```text
ID uniqueness
reference resolution
frame range validity
edit timeline overlap/gap
camera operation local range
finite numeric values
positive lens/exposure parameters
strict unknown-field rejection
```

```yaml
lens:
  focal_lenght_mm: 50   # typo
```

应直接拒绝，而不是静默使用默认值。对生成式 Agent，未知字段被吞掉是高风险失败模式，因为输出看似成功，实际语义未生效。

## 13.2 连续性诊断

```text
positive -> negative without bridge
=> CONTINUITY_AXIS_CROSS

same subject right -> left without visible turn
=> CONTINUITY_SCREEN_DIRECTION_FLIP

A looks right to B; B looks right to A
=> CONTINUITY_EYELINE_MISMATCH

same subject + same shot size + azimuth delta < 30°
=> CONTINUITY_SMALL_ANGLE_CUT
```

`examples/unsafe_axis_cross.yaml` 故意构造轴线和视线错误：

```bash
cargo run -- analyze examples/unsafe_axis_cross.yaml
```

失败样例证明规则能区分输入，而不是无条件返回成功。

## 13.3 False Positive 与 False Negative

False positive：几何上已通过演员移动建立新轴，但作者忘记添加 `character_reblocking`。

False negative：作者错误标注两镜头都在 `positive` 侧，分析器不会发现真实几何已越轴。

下一步应使用双通道：

```text
declared annotations
        +
derived geometry observations
        |
        v
consistency check
```

若二者冲突，报告 `DECLARED_DERIVED_MISMATCH`。

## 13.4 诊断 API

机器消费者应依赖稳定 `code`，不要解析自然语言 message：

```json
{
  "code": "CONTINUITY_AXIS_CROSS",
  "severity": "warning",
  "path": "scenes[scene].shots[a->b]",
  "message": "...",
  "hint": "..."
}
```

## 13.5 CI 策略

```bash
# Error 失败，warning 允许
cine-ir validate scene.yaml

# 任何 warning 都失败
cine-ir validate scene.yaml --deny-warnings
```

实际制作可为特定镜头声明 deliberate disorientation，而不是全局关闭规则。
