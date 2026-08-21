# 2. 镜头、景别、机位与视点

镜头设计至少有五个相互独立的维度：景别、水平角度、垂直角度、摄影机高度和叙事视点。

## 2.1 景别

```text
环境信息  <--------------------------------->  面部/细节信息

 ELS      LS      MLS      MS      MCU      CU      ECU

+------+ +------+ +------+ +------+ +------+ +------+ +------+
|  .   | |  O   | |  O   | |  O   | | OOO  | |OOOOOO| | eye  |
| land | | /|\  | | /|\  | | /|\  | | /|\  | |OOOOOO| | only |
|      | | / \  | | / \  | |      | |      | |      | |      |
+------+ +------+ +------+ +------+ +------+ +------+ +------+
```

景别描述主体在画面中的尺度和叙事距离，不直接规定焦距或物理距离。相同景别可由不同焦段与摄影距离实现，空间感不同。

```yaml
framing:
  shot_size: close_up
  subject_ids: [alice]
```

## 2.2 水平与垂直角度

```text
Top View

          frontal
             C
             |
             v
profile C -> O <- C profile
          Actor
           / \
 rear  C /   \ C three-quarter
```

`frontal` 直接、可能带审视感；`three_quarter` 保留面部体积；`profile` 强调方向和对峙；`rear` 限制表情信息；`over_the_shoulder` 把观察者绑定到对话关系。

高机位与俯拍相关但不相同：摄影机可以很高却保持水平，也可以略高但明显向下倾斜。`low angle = power` 只是常见联想，不是可靠函数。

## 2.3 POV 与主观程度

```text
Objective              OTS                  POV

[C] -- A -- B       [C] A) ---> B      A eyes ---> B
旁观者               与 A 结盟             近似 A 所见
```

严格 POV 通常需要镜头序列建立：角色看向画外 → 目标画面 → 角色反应。单独一张眼睛高度的画面不自动成为 POV。

## 2.4 Coverage Role 与 Purpose

`coverage_role` 表示镜头在剪辑覆盖中的职能；`purpose` 表示叙事目的。两者不能合并：

```yaml
coverage_role: close_up
purpose: [reaction, emphasize]
```

同一个 close-up 可以用于识别、反应、孤立、揭示或误导；“揭示”也可以由推进、横移、拉焦、遮挡解除或切镜完成。
