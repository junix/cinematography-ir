# 6. 场面调度：人物、摄影机和光的共同编舞

Blocking 不只是演员站位。在摄影系统中，它包括人物运动、视线、相机路径、焦点变化、灯光可用区和剪辑节点。

```text
ShotState(t) = {
  subjects.transforms(t),
  subjects.gaze(t),
  camera.state(t),
  focus.target(t),
  lights.state(t),
  narrative.beat(t)
}
```

只规划相机而不规划人物，会产生追赶式运镜；只规划人物而忽略光，会让角色走出可用曝光或造型区。

## 6.1 Marks 与 Beats

```text
A starts          A reaches table          A turns to B
  X---------------------X------------------------X
 frame 0              frame 72                  frame 120

       beat 1: approach      beat 2: reveal alliance
```

Mark 是空间位置；Beat 是动作或叙事变化。二者可能重合，也可能不重合。

```yaml
blocking:
  - subject_id: alice
    keyframes:
      - frame: 0
        transform: ...
        gaze_target: { type: subject, id: bob }
        action: "保持距离"
      - frame: 96
        transform: ...
        action: "绕过桌角，进入 Bob 一侧"
```

## 6.2 调度会改变轴线

```text
Initial
A ---------------- B
      relation axis

After A walks around B
             A
             |
             B
        new relation axis
```

轴线不是永久固定线。人物移动、共同注视第三对象、车辆转弯或新说话者加入后，主导轴可能变化。当前 IR 支持多个 `ContinuityAxis`，由每个 Shot 的 `axis_observations` 指明当前关系。

## 6.3 Reveal 有多种机制

```text
A. camera trucks       wall ███ [C] ---> target O
B. actor moves         actor O ---> exposes target X
C. rack focus          foreground sharp -> background sharp
D. cut                 reaction -> insert/object
```

好的设计先选择符合叙事与生产约束的机制，而不是默认使用复杂运镜。

## 6.4 可拍性约束

真实拍摄还需要考虑：摄影机与器材碰撞、镜头最短对焦距离、跟焦速度、灯具和收音穿帮、演员与设备加速度、安全区、多次 take 的可重复性。

虚拟相机也不应默认无质量、可瞬移。过度自由会产生缺乏人体尺度和物理可读性的运动。是否模拟真实器材应是风格参数。
