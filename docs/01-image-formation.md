# 1. 影像形成与分析层级

摄影术语常被混用，因为人们在不同层级使用同一个词。“拉近”可能指摄影机推进、变焦或后期裁切；三者都能让主体变大，但空间结果不同。

## 1.1 成像链

```text
Scene Geometry + Materials + Lights
                 |
                 v
             Lens System
  focal length / aperture / distortion
                 |
                 v
          Shutter + Sensor
  exposure / motion blur / noise / dynamic range
                 |
                 v
            Image Pipeline
  debayer / color transform / grade / display
```

摄影机只记录光。故事意义来自选择：放什么进画面、从哪里看、何时看、与哪些镜头连接。

## 1.2 三维世界与二维银幕

```text
World Space                     Screen Space

B  (x,y,z)                        +----------------+
        \                         | A        B     |
         \                        |                |
          [Camera]  ----------->  |      table     |
         /
        /
A  (x,y,z)
```

观众通常不知道真实坐标，只能从投影、运动、遮挡和剪辑推断空间。因此世界坐标未变，换机位仍可能让银幕左右关系反转。

## 1.3 四层分析

```text
L4  Narrative / Perception
    权力、亲密、孤独、悬念、主观性

L3  Editorial Grammar
    建立镜头、反打、反应、插入、匹配、跳切

L2  Cinematography
    景别、构图、机位、焦段、光线、运镜

L1  Physical Parameters
    pose、focal length、aperture、focus、ISO、shutter
```

同一个叙事意图可以有多种摄影实现。“孤独”可用极远景缩小人物，也可用近景中的巨大负空间、遮挡或拒绝切反应镜头实现。IR 因此不应规定 `isolation == extreme_long_shot`，而应保存意图、候选策略和约束。

## 1.4 Shot、Take、Setup、Scene、Sequence

```text
Sequence 叙事段落
  └── Scene 场景/时空单元
       └── Shot 剪辑后的连续画面单元
            └── Take 同一镜头方案的一次实际拍摄

Setup = 机位、灯光、器材和现场布置
```

当前 IR 表示编辑时间线上的 Shot 设计，不是现场 Take 日志或生产排期。
