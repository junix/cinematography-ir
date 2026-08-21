# 5. 运镜：位移、旋转、光学与程序化运动

“运镜”是上位概念。内部应区分刚体位移、姿态旋转、光学变化和运动风格，否则同一个命令可能在不同执行器中产生不一致结果。

## 5.1 六自由度

```text
Translation                    Rotation

          +Y                     pitch
           ^                       ^
           |                       |
-X <---- [C] ----> +X        roll [C] yaw
          /
         / +Z
```

基础映射：

```text
pan        = yaw rotation
tilt       = pitch rotation
roll       = optical-axis rotation
truck      = local X translation
pedestal   = local Y translation
dolly      = local forward translation
```

`track` 在不同语境可表示沿轨道运动或横移，含义不稳定。IR 采用较明确的操作名，并让 backend 统一坐标正负号。

## 5.2 原地旋转

```text
Pan / Top View

 target A       target B
      \           /
       \         /
          [C]
     position fixed
```

Pan 与 Tilt 改变观察方向但不产生摄影机平移视差；Roll 改变画面地平线，通常需要清晰动机。

## 5.3 摄影机位移

```text
Dolly:      [C] -----------> O
Truck:      [C1] ---> [C2] ---> [C3]
Pedestal:        [C3]
                  ^
                 [C2]
                  ^
                 [C1]
```

推进会改变摄影距离和透视；横移会产生前后景视差，可解除遮挡或重组关系；升降与 Tilt 不同，因为机位和遮挡关系也改变。

## 5.4 复合路径

```text
Orbit
          C2
       .-'  '-.
     C1   O    C3
       '-.__.-'
          C4
```

Orbit 通常是平移加持续 `look_at`。它容易跨越人物关系轴，因此必须决定是让观众在连续镜头中看见跨轴、经过 on-axis、通过人物重排建立新轴，还是故意制造失序。

```text
Crane / Jib
          [C high]
             \
              \
             pivot O------ arm
                \
                 [C low]
```

Crane 轨迹通常为弧线，不等同于纯 pedestal。

## 5.5 Follow 是约束，不是轨迹

```text
[C] ----->  O ----->
             actor
```

Follow 需要持续追踪目标并保持 offset、景别或屏幕位置。执行器还需处理目标转弯、速度变化、遮挡、碰撞和相机加速度限制。

## 5.6 光学与焦点变化

```text
zoom        focal_length(t)
rack_focus  focus_target(t) / focus_distance(t)
```

Zoom 不改变相机位置；Rack Focus 不改变视场角。它们属于广义镜头操作，但不是刚体运动。

## 5.7 Handheld 不是白噪声

```text
ideal path:     ------------------------>
handheld path:  --~--^---v--~---^------>
```

可信手持通常包含步态低频位移、操作者修正、中高频微动、构图反馈和偶发冲击。当前 `handheld_noise` 只是可复现的基础占位符，不是完整人体动力学模型。

## 5.8 运镜动机

```text
发现信息      -> push in / rack focus / insert
关系变化      -> arc / reblocking / two-shot transformation
环境压倒人物  -> pull out / crane up
遮挡解除      -> truck / reveal
跟随行动      -> follow / lead / chase
主观失稳      -> roll / handheld / dolly zoom
```

这些是候选映射，不是固定字典。
