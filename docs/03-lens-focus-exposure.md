# 3. 镜头光学、透视、景深与曝光

最重要的技术纠错是：**焦距本身不决定透视；摄影机位置决定透视。**

## 3.1 焦距与视场角

```text
short focal length  -> wide field of view
long focal length   -> narrow field of view
```

若相机位置固定，只更换焦距，物体间的透视比例不变；只是画面截取范围变化。所谓“长焦压缩空间”通常还隐含了为保持主体大小而后退机位：

```text
Wide setup                       Tele setup

[C] -- A -------- B              [C] -------- A -- B
近机位 + 短焦                    远机位 + 长焦
A/B 视角差大                     A/B 视角差小
=> 深度夸张                      => 空间看似压缩
```

工程系统必须分开保存 `camera_position`、`focal_length`、`sensor_width` 和构图约束。

## 3.2 景深

```text
near blur       sharp region        far blur
~~~~~~~~~ |====================| ~~~~~~~~~
                 focus plane
```

景深受光圈、焦距、对焦距离、显示条件、主体与背景距离，以及为保持构图而改变的摄影距离共同影响。浅景深能控制注意力，但也可能让移动表演失焦、多人无法同时清晰、环境关系不可读。

## 3.3 Rack Focus

```text
Frame t0                        Frame t1

A SHARP        B blur           A blur         B SHARP
   O             O                 O              O

attention: A -------------------------------> B
```

IR 保存目标语义，而不只保存距离曲线：

```yaml
operation:
  op: rack_focus
  target: { type: subject, id: witness }
```

执行器依据 blocking 计算每帧焦距。

## 3.4 快门角与运动模糊

```text
small angle                  large angle
O  O  O  O                   O~~~O~~~O
锐利、断续                     模糊、拖曳
```

快门角改变运动质感；ISO/增益影响噪声与动态范围；ND 改变入射光量但不直接改变景深和运动模糊。它们不是等价的亮度旋钮。

## 3.5 Dolly Zoom

Dolly Zoom 的目标是：摄影机距离改变，同时通过焦距求解让主体的画面尺度近似不变。

```text
camera distance changes
        +
focal length solved continuously
        |
        v
subject screen size ~= constant
background perspective changes
```

简单线性叠加 dolly 与 zoom 不保证主体尺度稳定。
