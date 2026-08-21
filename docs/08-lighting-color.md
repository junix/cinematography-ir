# 8. 光线、色彩与影调

光线决定形体、层次、曝光余量、色彩关系和观众注意力。

```text
direction      从哪里来
quality        硬 / 软
intensity      亮度与曝光贡献
color          色温与光谱倾向
size/distance  阴影边缘与衰减
motivation     观众相信光来自哪里
movement       光是否随时间变化
```

“柔光”主要来自相对主体更大的发光面积，不是简单降低亮度。

## 8.1 三点布光只是教学模型

```text
                  Back / Rim
                      *
                     /
                    O actor
                   /|\

       * Key                    * Fill
```

Key 提供主要造型方向；Fill 控制阴影密度；Back/Rim 分离主体；Practical 是画面内可见灯；Motivated 灯模拟可信来源。真实场景可能没有独立 Fill，甚至用负补光加深阴影。

## 8.2 Motivated Lighting

```text
visible window
██████████
     \
      \ perceived source
       O actor

actual off-screen fixture * ---> actor
```

关键是方向、颜色、衰减与环境反应让观众相信光来自窗户，而非灯具真的必须在窗外。

## 8.3 对比与动态范围

```text
low contrast                  high contrast
shadow ---- mid ---- highlight  shadow -------- highlight
信息多、平缓                     分离强、暗部可能丢失
```

单个 `contrast_ratio` 不足以描述画面；局部对比、背景亮度、肤色目标和显示变换同样重要。自动系统应保存曝光优先级和容忍范围，而不是只求平均亮度正确。

## 8.4 色温与白平衡

```text
3200K source + 3200K WB -> relatively neutral
3200K source + 5600K WB -> appears warm
5600K source + 3200K WB -> appears cool
```

色温是光源近似属性；白平衡是摄影系统的解释基准。Kelvin 值不能直接映射为固定情绪标签。

## 8.5 现场色彩与后期调色

```text
production design + lighting + filtration
                 |
                 v
camera negative / log image
                 |
                 v
color management + creative grade
```

核心 IR 暂不绑定特定相机 Log、LUT 或 ACES/OCIO 管线；这些属于 backend 扩展。
