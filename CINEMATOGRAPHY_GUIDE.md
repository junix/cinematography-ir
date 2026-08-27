# 电影摄影系统化导论与 Cinematography IR
> 本文件由 `docs/` 中的 mdBook 章节按目录顺序合并，便于单文件阅读。

---

# 导论：把摄影看成约束系统

电影摄影不是“选择一个好看的焦段，再加一种运镜”。它同时处理物理成像、三维空间、二维构图、时间、表演、剪辑连续性和叙事意图。

```text
现实/虚拟世界
  人物、道具、布景、动作、光源
          |
          v
摄影系统
  机位、朝向、镜头、对焦、曝光、运动
          |
          v
二维画面
  景别、构图、遮挡、方向、视觉重量
          |
          v
镜头序列
  切换、节奏、视线、动作匹配、空间连续性
          |
          v
观众模型
  我在哪里？谁在看谁？什么重要？应当感到什么？
```

## 四类陈述必须分开

**事实**：相机位于关系轴哪一侧；焦距是多少；人物朝向何处；镜头持续多少帧。

**推理**：从当前机位投影后，Alice 位于画面左侧；换到轴线另一侧后，左右关系会反转。

**启发式规则**：传统连续剪辑通常避免未经说明的越轴；相近景别的小角度切换可能产生跳切感。

**价值判断**：这个跳切是否有效；低机位是否让人物显得强大；浅景深是否适合故事。

启发式不是物理定律。工程系统应报告风险并允许显式例外，而不是把艺术选择硬编码成禁止项。

## 设计镜头的最小问题集

```text
WHY     镜头为什么存在？
WHAT    观众需要看到什么？
WHO     观众与哪个人物结盟？
WHERE   摄影机和人物在哪里？
WHEN    何时移动、切换或转移焦点？
HOW     用什么机位、焦段、光线和运动实现？
```

只回答 `HOW` 会产生技术堆砌；只回答 `WHY` 又无法执行。Cinematography IR 的目标是连接两者。

---

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

---

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

---

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

---

# 4. 构图、画面重量与空间层次

构图不是“三分法”的同义词。它组织视觉关系：谁重要、谁控制空间、视线往哪里流动、画面是否稳定、环境是否可理解。

## 4.1 画框与空间预算

```text
+------------------------------------------------+
| headroom                                       |
|        O ---------------------> lead room      |
|       /|\                        negative      |
|       / \                        space         |
+------------------------------------------------+
```

- `headroom`：头顶空间；
- `lead room`：视线或运动前方空间；
- `negative space`：承担情绪、期待或威胁的空白区域；
- `frame edge pressure`：主体靠近边缘造成的不稳定或拥挤。

这些不是固定比例。恐惧场景可能故意压缩 lead room，让人物“撞向”画框。

## 4.2 视觉重量

视觉重量受大小、亮度、对比、色彩、清晰度、运动、人脸和位置共同影响：

```text
+-------------------------------+
|      small bright face   *    |
|                               |
| █████ large dark mass         |
+-------------------------------+
```

面积大不一定更抢眼；显著性也不等于叙事重要性。

## 4.3 深度构图

```text
Foreground          Midground             Background

  █ door frame          O actor              /\ hills
  █                    /|\                  /  \
  █                    / \             window light
```

前景可建立深度、形成遮挡、延迟揭示、构成 frame-within-frame，或表达窥视和隔离。深焦让多个层级同时参与叙事；浅景深通过清晰度选择层级。两者都不是天然更“电影化”。

## 4.4 动态构图

动态镜头中的构图是时间函数：

```text
composition(t) =
  subject_screen_bounds(t)
  headroom(t)
  lead_room(t)
  occlusion(t)
  visual_balance(t)
```

只检查起点和终点会漏掉中途裁切、遮挡和构图翻转。真正的执行器需要采样或连续求解。

---

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

---

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

---

# 7. 空间连续性：轴线、方向、视线与切换

连续性规则维护的是观众的空间模型，而不是保护一条抽象几何线。

## 7.1 180° 规则

```text
        conventional camera side

       C1       C2       C3
         \       |       /
          \      |      /
A -------------------------------- B
             relation axis

-------------------------------------
        opposite camera side
                 C4
```

同一侧拍摄通常保持：A 在左并向右看，B 在右并向左看。切到另一侧后，左右与视线关系反转；若无空间重建，观众可能误解人物换位或转身。

## 7.2 合理越轴

```text
1. 连续镜头中看见摄影机穿过轴线
2. 插入 on-axis 中性机位
3. 人物重新调度，建立新轴
4. 全景重新建立地理关系
5. 插入不依赖左右方向的细节/反应镜头
6. 明确追求迷失、冲突或心理断裂
```

```yaml
continuity:
  axis_observations:
    - { axis_id: dialogue_axis, side: negative }
  bridge: visible_axis_cross
```

`bridge` 说明变化为何可理解或为何故意破坏理解，不是粗暴关闭检查。

## 7.3 运动轴与银幕方向

```text
World movement: car ---------------------------------->

Shot 1: --->
Shot 2: --->
Shot 3: <---    # audience may infer a turn-around
```

跨越运动轴会让银幕方向反转。角色真实转身、道路弯曲或镜头内展示方向变化时，反转是合理的。

## 7.4 Eyeline Match

```text
Shot A                            Shot B
Alice O ------ looks right --->     <--- Bob O
```

互相面对的人物在传统反打中通常应有相反银幕视线。若双方都向右看，观众可能认为他们并未面对彼此。完整验证还应检查高低视线、目标位置和距离，当前分析器只覆盖水平关系。

## 7.5 30° Rule

```text
      C2
     /
    /  15°
   O subject
  /
 C1
```

相同主体、相近景别、机位变化很小时，切换可能像无意跳动。30° 是启发式而非精确阈值；焦段、背景、运动和节奏都会影响感知。

当前分析器仅在相同主要主体、相同景别、显式 azimuth 差小于 30°、且未声明 jump cut 或 bridge 时警告。

## 7.6 Match on Action

```text
Shot 1: hand reaches ---- [CUT] ---- Shot 2: hand completes grab
```

动作中切换可以掩盖视角变化。严格验证需要动作事件及相位，不只是镜头时间范围。未来可加入 anticipation、contact、follow-through 等事件标记。

## 7.7 多轴场景

```text
A -------- B       dialogue axis
 \        /
  \      /
     C             triangular relationships

Door --------> A   movement axis
A ----------> X    eyeline axis
```

“遵守轴线”不是单一二元条件，而是选择当前 beat 中要维护哪种关系。

---

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

---

# 9. Coverage、剪辑与镜头序列

Coverage 是为剪辑准备可用视角和表演选择的策略，不是机械地把 master、OTS 和 close-up 全拍一遍。

```text
                      MASTER
                 +-------------+
                 |  A       B  |
                 +-------------+
                       |
          +------------+------------+
          |                         |
       OTS on A                  OTS on B
          |                         |
       CU Alice                  CU Bob
          |                         |
       reaction                  reaction
                 \             /
                   insert/detail
```

## 9.1 Coverage Role

- `master`：建立地理、完整动作和表演；
- `two_shot`：强调关系；
- `over_the_shoulder`：维持对话空间和观察者归属；
- `single / close_up`：隔离人物并读取细微反应；
- `reaction`：改变一句台词的意义；
- `insert`：展示对象、动作或线索；
- `cutaway`：压缩时间或修补连续性；
- `point_of_view`：建立角色所见；
- `establishing`：建立场景地理和语境。

重复同一信息而没有性能、空间或节奏差异的 coverage 会增加成本，却未必增加剪辑选择。

## 9.2 镜头序列是信息调度

```text
frame 0       48          96          144
  |-----------|-----------|------------|
 geography    object      suspicion    confirmation
 master       insert      reaction     reveal
```

摄影设计决定的不仅是看什么，还包括何时知道。提前展示对象产生期待；延迟展示产生疑问；拒绝展示反应会改变观众与人物的结盟。

## 9.3 匹配类型

```text
spatial match     轴线、银幕方向、视线
motion match      动作相位、速度、方向
graphic match     形状、位置、亮度、颜色
sound bridge      声音跨越画面切换
idea match        概念或隐喻连接
```

当前 IR 主要覆盖空间匹配。完整 Editing IR 还需要声音、动作事件、节奏和转场语义。

## 9.4 长镜头不是“无剪辑”

```text
traditional edit:
shot A -> cut -> shot B -> cut -> shot C

one take:
composition A -> movement -> composition B -> reveal -> composition C
```

长镜头把剪辑选择转移到场面调度、构图转换和运镜中。它也有失败模式：重点不清、整条重拍、运动无动机、技术风险高。

---

# 10. 视觉叙事与镜头动机

镜头语言最常见的失败不是技术错误，而是技术选择与叙事问题无关。一个复杂 orbit 可以流畅，却没有传递新的关系、信息或感受。

## 10.1 从意图到参数

```text
Narrative Intent
  "Alice 意识到 Bob 在撒谎"
          |
          v
Perceptual Goal
  先注意 Alice 的微小反应，再重新评价 Bob
          |
          v
Shot Grammar Candidates
  reaction CU / rack focus / push-in / delayed cut
          |
          v
Spatial & Editorial Constraints
  保持对话轴；不要提前暴露桌上的证据
          |
          v
Cinematography Parameters
  65 mm, MCU->CU, slow dolly, focus on Alice
```

Agent 不应从情绪词直接跳到焦距。中间必须经过感知目标、信息顺序和空间约束。

## 10.2 Shot Purpose

```text
orient       建立地理和关系
identify     告诉观众对象是谁/是什么
emphasize    增加信息视觉权重
reveal       延迟后释放信息
isolate      把人物从关系或环境中分离
connect      把人物、对象或空间纳入同一关系
subjective   绑定角色感知
reaction     显示信息造成的后果
transition   从一个状态、空间或时间转入另一个
Disorient    有意破坏稳定模型
```

一个镜头可组合多个目的，但组合越多，越需要检查镜头是否过载。

## 10.3 形式不是固定情绪字典

不可靠的确定性规则：

```text
low angle  -> power
wide lens  -> anxiety
blue light -> sadness
push in    -> emotion
```

这些形式只在特定上下文中可能产生某种联想。低机位可显得滑稽；广角可表现开放；蓝色可表示夜晚、清洁或技术环境；推进也可能只是技术重构图。

## 10.4 三条通道

```text
Information   观众新知道了什么？
Emotion       感知强度或情绪距离如何变化？
Relationship  人物、对象与观众的位置关系如何变化？
```

例如缓慢拉远：逐渐揭示环境；人物在画面中缩小；亲密度下降；人物从主体变成大环境中的小元素。若运镜在三条通道都没有改变，它可能只是装饰。

## 10.5 评估镜头方案

```text
Narrative relevance     是否服务当前 beat？
Spatial legibility      空间模型是否可读或被有意破坏？
Attention control       重要信息是否在正确时机可见？
Continuity/editability  能否与前后镜头连接？
Physical feasibility    器材、焦点、速度和光线是否可执行？
Style consistency       是否符合整段视觉系统？
Cost/robustness         重拍和适配成本如何？
```

---

# 11. Cinematography IR 的分层架构

Cinematography IR 不是渲染场景格式，也不是剪辑工程文件。它连接叙事规划、空间调度、镜头设计和执行器。

## 11.1 设计目标

```text
human-readable       YAML / JSON 可审查
machine-validatable  类型、引用、范围和连续性诊断
semantic             保存镜头理由与编辑职能
deterministic        随机运动携带 seed
extensible           metadata 与版本迁移
backend-neutral      不绑定 Blender / Unreal / Three.js
```

## 11.2 数据层级

```text
CineProject
  |
  +-- frame_rate / coordinate_system / metadata
  |
  +-- Scene
       |
       +-- NarrativeIntent
       +-- Subject / Marker / ContinuityAxis
       +-- BlockingTrack / Beat
       +-- LightingSetup
       +-- Shot[]（兼容的单轨写法）
       +-- Take[] + EditClip[]（覆盖拍摄与最终剪辑）
             |
             +-- purpose / coverage_role
             +-- Framing
             +-- CameraPlan
             |     +-- initial_state
             |     +-- TimedCameraOperation[]
             +-- ContinuityAnnotations
             +-- ShotPhrase / ShotRelation / TransitionSpec / Prohibition
```

## 11.3 语义层与执行层

```yaml
operation:
  op: follow
  target: { type: subject, id: runner }
  offset: { x: 0.0, y: 1.5, z: -3.0 }
  lag_half_life_s: 0.18
```

执行层需要把它编译为逐帧 `CameraState`。编译依赖 blocking、碰撞体、相机动力学、构图约束和 backend 坐标系。

```text
Cinematography IR       authoring source             src/model.rs
       |
       +-- solve ----> Solved Camera IR              src/solve/
       |               compatibility/debug artifact
       v
Compiled Guidance IR   phases + semantic intent      src/compiled.rs
                       world/screen tracks
                       constraints + tolerances
                       edit relations + prohibitions
       |
       +---> prompt    time-ordered commands          src/prompt/
       +---> view      semantic diagrams -> Canvas    src/view/
       +---> execute   typed shot/control bundles     src/execute/
       +---> compare   constraint-driven acceptance   src/compare.rs
```

把逐帧轨迹直接放进作者 IR 会导致文件巨大、意图丢失、人物路径变化后轨迹失效。`SolvedProject` 只保留兼容的逐帧相机状态；真正的下游窄腰是可重建的 `CompiledProject`。同一 `ShotConstraint` 由 solver/compile 求值、view 绘制、prompt 描述、execute 打包、compare 验收，消费者不得再次“镜像 solver”解释 `CameraOperation`。

## 11.4 时间域

```text
Performance/story time: Take.performance_range = [0, 240)
Take-local operation:   operation.range = [16, 72)
Selected source:        EditClip.source_range = [80, 160)
Final edit time:        EditClip.edit_range   = [120, 200)
```

所有范围都是半开区间。Take 可以覆盖同一段表演；EditClip 负责 source frame 到最终剪辑 frame 的映射。转场由 `TransitionSpec` 携带 duration、easing、match anchor 与方向；转场重叠必须由下一 clip 的 transition duration 覆盖。

## 11.5 引用而不是复制

```yaml
focus:
  target: { type: subject, id: alice }
```

优于把 Alice 坐标复制进镜头。执行器应根据 blocking 在每帧解析目标位置。`Marker` 适合命名的空间锚点，`Point` 适合不可移动静态点。

## 11.6 显式连续性注释

理想情况下，轴线侧和银幕方向可从几何推导。但核心 IR 仍保存声明值：

```yaml
continuity:
  axis_observations:
    - { axis_id: dialogue_axis, side: positive }
  camera_azimuth_deg: 55.0
```

原因包括：外部资产可能不在文件中；后期翻转或镜面会改变屏幕结果；主导轴是叙事选择；声明值可与几何推导值交叉验证。

## 11.7 版本与扩展

核心字段语义变化必须升级 `schema_version`。实验性信息可进入 namespaced metadata，但稳定且需要验证的字段应进入正式模型。任意 JSON blob 不是长期互操作方案。

---

# 12. IR 字段与 YAML 示例

完整类型以 `src/model.rs` 和生成的 JSON Schema 为准。本章说明主要语义。

## 12.1 Project

```yaml
schema_version: "0.1"
id: interview_scene
title: "审讯室"
frame_rate:
  numerator: 24000
  denominator: 1001
coordinate_system:
  units: meters
  handedness: right
  up_axis: y
  forward_axis: negative_z
scenes: []
```

帧率使用有理数，避免 23.976 等速率的浮点误差。

## 12.2 Scene

```yaml
- id: interrogation
  title: "谁拿走了钥匙"
  duration_frames: 240
  narrative: ...
  subjects: ...
  markers: ...
  axes: ...
  blocking: ...
  beats: ...
  lighting_setups: ...
  shots: ...          # 兼容的最终剪辑单轨
  takes: ...          # 可重叠的拍摄覆盖
  edit_timeline: ...  # source -> edit 映射
```

`shots` 是兼容的单轨编辑时间线，因此不允许重叠。需要 master、single、reaction 同时覆盖一段表演时，使用可重叠 `takes`；`edit_timeline` 从 take 选择 source range。当前 edit timeline 是画面单轨，转场允许显式 overlap，音频多轨仍在范围外。

## 12.3 Subject 与 TargetRef

```yaml
subjects:
  - id: alice
    name: Alice
    kind: character
    initial_transform:
      position: { x: -1.5, y: 0.0, z: 0.0 }
      rotation_deg: { pitch: 0.0, yaw: 90.0, roll: 0.0 }
    geometry_proxy: { kind: capsule_rig }
    pose_track:
      frames:
        - frame: 0
          joints: { head: { x: -1.5, y: 1.65, z: 0.0 } }
          gaze: { type: subject, id: bob }
```

引用形式：

```yaml
{ type: subject, id: alice }
{ type: marker, id: room_center }
{ type: point, position: { x: 0.0, y: 1.4, z: 0.0 } }
```

## 12.4 ContinuityAxis

```yaml
axes:
  - id: dialogue_axis
    name: Alice–Bob relationship axis
    purpose: relationship
    from: { type: subject, id: alice }
    to: { type: subject, id: bob }
```

端点顺序定义正负侧参考方向。具体叉积符号应由几何推导器统一。

## 12.5 Shot 与 Framing

```yaml
- id: alice_close_up
  range: { start: 80, end: 160 }
  purpose: [isolate, emphasize]
  coverage_role: close_up
  framing:
    shot_size: close_up
    horizontal_angle: three_quarter
    vertical_angle: eye_level
    subject_ids: [alice]
    composition:
      strategy: rule_of_thirds
      headroom: 0.07
      lead_room: 0.28
      negative_space: right
  camera: ...
  phrases: [reaction_hold]
  relations:
    - { kind: reaction_to, shot_id: bob_close_up }
  prohibit: [unplanned_cut]
```

`headroom` 与 `lead_room` 是 `[0,1]` 规范化规划值；当前版本不规定唯一测量算法，backend 应说明其屏幕空间解释。

`ShotPhrase` 位于物理 operation 和剪辑关系之间，包括 `pressure_push_in`、`subject_lock_dolly_zoom`、`foreground_parallax_reveal`、`visible_axis_cross`、`rack_focus_reveal`、`reaction_hold`、`walk_and_talk_follow` 与 `orbit_relationship_shift`。未显式填写时，compiler 会从运镜、purpose 与连续性桥接推导，但不会覆盖作者值。

## 12.6 CameraState

```yaml
initial_state:
  transform:
    position: { x: -0.65, y: 1.48, z: 2.7 }
    rotation_deg: { pitch: 0.0, yaw: 14.0, roll: 0.0 }
  lens:
    focal_length_mm: 65.0
    sensor_width_mm: 36.0
    aperture_f: 2.8
    anamorphic_squeeze: 1.0
  focus:
    target: { type: subject, id: alice }
  exposure:
    iso: 800
    shutter_angle_deg: 180.0
    white_balance_k: 5000
    nd_stops: 0.0
```

## 12.7 TimedCameraOperation

```yaml
operations:
  - range: { start: 16, end: 72 }
    easing: ease_in_out
    operation:
      op: dolly
      distance_m: 0.35
```

Follow 使用与帧率无关的时间常数：

```yaml
operation:
  op: follow
  target: { type: subject, id: runner }
  offset: { x: 0.0, y: 1.5, z: -3.0 }
  lag_half_life_s: 0.18
```

Reveal 可声明可观察的遮挡目标，solver 会根据 target/occluder 的投影包围盒求横移，而不是把它当作普通 truck：

```yaml
operation:
  op: reveal
  target: { type: subject, id: hero }
  occluder: { type: subject, id: doorway }
  lateral_distance_m: 1.0       # 无可见比例时的兼容 fallback
  visibility: { from: 0.05, to: 0.85, monotonic: true }
  preserve: { target_screen_y: 0.48 }
```

支持：

```text
hold
pan / tilt / roll
dolly / truck / pedestal
translate / rotate
look_at
orbit / follow / crane
zoom / rack_focus / dolly_zoom
handheld_noise
reveal
```

操作可以时间重叠，例如 `follow` 与 `handheld_noise` 同时存在。求解器必须定义组合优先级；当前验证器只检查范围和参数。

## 12.8 Take、EditClip 与 TransitionSpec

```yaml
takes:
  - id: master_take
    performance_range: { start: 0, end: 240 }
    coverage_role: master
    framing: { shot_size: medium, subject_ids: [alice, bob] }
    camera_program: { initial_state: { transform: {} } }
edit_timeline:
  - id: master_clip
    take_id: master_take
    source_range: { start: 24, end: 96 }
    edit_range: { start: 0, end: 72 }
    transition_in:
      kind: cut
      duration_frames: 0
      easing: linear
```

`source_range` 必须位于 take 的 performance range 内；当前不做 retime，所以 source/edit duration 必须相等。`TransitionSpec` 还可携带 `match_anchor` 和 `direction`。legacy `Shot.transition_in` 仍可读取，编译时会升级为 rich transition。

## 12.9 ContinuityAnnotations

```yaml
continuity:
  axis_observations:
    - { axis_id: dialogue_axis, side: positive }
  screen_directions:
    - { subject_id: runner, direction: right }
  eyelines:
    - subject_id: alice
      target: { type: subject, id: bob }
      screen_direction: right
  camera_azimuth_deg: 55.0
  spatial_reset: false
  direction_change_visible: false
```

允许的 `bridge`：`visible_axis_cross`、`on_axis_shot`、`neutral_insert`、`reestablishing_shot`、`character_reblocking`、`deliberate_disorientation`。

---

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

---

# 14. Agent、求解器与执行器

Cinematography Agent 不应直接输出逐帧相机坐标。更稳健的系统是多阶段编译器。

## 14.1 流水线

```text
Script / Storyboard / Director Notes
                 |
                 v
Narrative Parser
  scenes, beats, information dependencies
                 |
                 v
Staging Planner
  subjects, marks, gaze, action events, axes
                 |
                 v
Shot Planner
  purpose, coverage, POV, framing candidates
                 |
                 v
Cinematography Constraint Solver
  lens, pose, focus, lighting, movement
                 |
                 v
Continuity Analyzer + Critic
  axis, direction, eyeline, editability
                 |
                 v
Trajectory Compiler
  Compiled Guidance IR:
  phases + camera/focus/lens/exposure/light curves
  subject screen tracks + constraints + edit relations
                 |
                 v
Backend Adapter
  Blender / Unreal / Three.js / USD / robotic camera
                 |
                 v
Render / Preview / Measurement Feedback
```

每层产生可检查 artifact，避免一个模型调用同时负责叙事、几何、物理与格式正确性。

## 14.2 候选与约束

```text
Beat: "Alice notices the hidden key"

A: cut to insert -> reaction CU
B: rack focus from Alice to key
C: lateral reveal behind foreground object
D: slow push-in while key enters frame edge
```

排序依据：

```text
hard constraints:
  target visible
  no collision
  focus range feasible
  edit timeline valid

soft constraints:
  preserve axis
  minimize acceleration/jerk
  maintain composition
  match style
  control production cost
```

硬约束不可满足时，应返回冲突集合，而不是偷偷生成穿墙相机或失焦镜头。

## 14.3 Follow 求解示例

```text
Given:
  target transform T_s(t)
  desired screen position p*
  desired shot size s*
  camera dynamics limits
  collision geometry

Solve:
  camera pose T_c(t)
  focal length f(t)
  focus distance d(t)

Minimize:
  composition error
  acceleration/jerk
  obstacle penetration
  deviation from style
```

`camera.position = target + offset` 只适合最简单情况。

## 14.4 约束冲突

```text
keep 85 mm lens
keep full-body framing
camera cannot leave room
subject walks toward wall
```

这些条件可能不可同时满足。系统应指出 UNSAT core，并提出缩短焦距、改变景别、调整 blocking、切镜或允许遮挡等候选放宽方案。

## 14.5 Critic 必须看测量结果

```text
IR static checks
geometry simulation
preview render
screen-space subject tracking
focus/occlusion metrics
human/director notes
```

视觉 critic 可检测人脸裁切、lead room 翻转、意外遮挡、运动模糊、灯光穿帮和 reveal 提前泄漏。

## 14.6 决策权限

建议把规划字段分为：

```text
locked       不可修改
constrained  可在范围内求解
suggested    可替换但需解释
free         可自由探索
```

Agent 不应未经授权修改故事事实、生产安全约束或导演锁定决策。

## 14.7 Backend 边界

```rust,ignore
// src/execute/mod.rs
trait CinematographyAdapter {
    fn capabilities(&self) -> AdapterCapabilities;
    fn stage_scene(
        &mut self,
        project: &CompiledProject,
        scene: &CompiledScene,
        profile: &ExecutionProfile,
    ) -> Result<()>;
    fn execute(&mut self, request: &ExecutionRequest, out_dir: &Path)
        -> Result<ExecutionBundle>;
}
```

运镜求解不在适配器内。`CompiledProject` 已经带有逐帧世界状态、原始语义、屏幕轨迹、约束和剪辑关系，因此 Blender 与未来 USD / Unreal 不会分别解释 Dolly Zoom、Reveal 或 phase。`AdapterCapabilities` 对 metric depth、flow、pose、lighting、focus、camera matrices、transition 和具体 `PassKind` 显式协商；不支持的格式必须失败，不能伪造。

执行输出是事务化 shot bundle，不是最终成片：先写临时目录并执行/验证 frame inventory，成功后写 manifest + complete marker 并原子发布。bundle 记录 source/compiled/profile hash、seed、runtime 版本、期望/实际帧数和 stdout/stderr；失败目录保留诊断但没有有效 manifest。每个 shot 单独携带 prompt、negative prompt、review diagram、typed controls、camera/screen tracks、constraints 与 validation，场景根目录的 `edit.json` 负责后续 EDL 组装。

`ExecutionProfile` 把文本时间线、start/end image、dense control、depth、pose、segmentation、camera API、negative prompt 与最大帧数能力同 pass encoding 绑定。仓库提供 `profiles/execution/generic-dense.json`、`image-to-video.json` 和 `camera-api.json` 示例；text-only 模型直接消费 `cine-ir prompt` 的分阶段 command，不启动 Blender。

---

# 15. 条件信号流水线：solve / prompt / view / execute

> 设计决策见 ADR-1115（`~/projects/docs/adr/1115-cinematography-ir-view-and-execute-layers.md`）。

Cinematography IR 的下游不是「渲染成片」，而是把镜头语言编译成**视频生成模型能吃的条件信号**。目标模型接受三类输入，IR 的三个地层恰好一一对应：

| 模型输入槽 | 消费 IR 的层 | 字段 | 实现模块 | CLI |
|---|---|---|---|---|
| ① 文本 prompt | `CompiledShot.intent + phases + constraints + prohibitions` | 时间分段命令与 negative constraints | `prompt/` | `cine-ir prompt` |
| ② 参考图 | `screen_tracks + framing + continuity` | storyboard / motion grammar / clean control | `view/`、`execute/blender` | `cine-ir view` |
| ③ 参考视频/数值控制 | `camera/subject/lens/focus/exposure/light tracks` | typed pass video + camera JSON | `execute/blender` | `cine-ir render blender` |

## 15.1 分层

```text
CineProject（语义 IR，源数据，进版本库）
        |
        v
   solve::solve_project
   CameraOperation -> CameraState[t]
        |
   compiled::compile_project
   phases + world/screen tracks + constraints
   prohibitions + take/edit relations
        |
   CompiledProject（所有新下游的共享编译产物）
        |
   +----+------------------+-------------------+----------------+
   v                       v                   v                v
prompt（纯）          view（纯）          execute（非纯）   compare（纯）
时间命令               Diagram IR → Canvas  typed shot bundle  可观察约束验收
```

归属判据固定为三问：会因几何失败吗（solve）？需要外部运行时或真实资产吗（execute）？只是把同一份信息重新编码吗（纯渲染器）。**输出会动不代表它是执行；需要外部运行时和真实资产才代表。**

## 15.2 Solved Camera IR 与 Compiled Guidance IR

`cine-ir solve` 是兼容/调试用的逐帧轨迹；`cine-ir compile` 才是下游窄腰：

```bash
cine-ir solve examples/dolly_zoom.yaml -o /tmp/dz.json
cine-ir schema --solved
cine-ir compile examples/jaws_beach_dolly_zoom.yaml -o /tmp/jaws-guidance.json
cine-ir schema --compiled
```

每帧携带 `position`、`rotation_deg`、以及世界空间的 `forward / up / right` 单位向量，消费者无需重推欧拉约定。`draft` 是确定性运动学；`full` 在 draft 结果上应用 rig 速度/加速度上限、显式 `GeometryProxy` 碰撞，并在位置修正后重新建立相机基向量。组合规则：

- 相对运镜（pan/tilt/roll/dolly/truck/pedestal/translate/rotate/crane 位移/reveal 位移）按每帧进度增量 `Δu` 沿相机**当前**轴累积，重叠运镜自然叠加；
- 目标运镜（look_at/orbit/zoom/rack_focus/dolly_zoom）在开始时快照相机，按 `u` 从快照插值到目标；区间内最后一帧到达目标，**区间结束后释放通道**，后续运镜在保持的终态上继续累积；
- `follow` 使用 `lag_half_life_s` 转换为 `alpha = 1 - 2^(-dt/half_life)`，因此 24/30/60 fps 的时间响应一致；legacy `damping` 按 24 fps 换算；
- `handheld_noise` 是确定性叠加层（seed 决定），带约 0.25 s 的淡入淡出，不回写基态；
- `dolly_zoom` 每帧按投影包围盒求焦距，使主体 bbox height 达到 `keep_subject_frame_fraction`；无可投影 subject 时才退回 `focal ∝ distance`；
- `reveal.visibility` 根据 target/occluder 投影遮挡求横移，`preserve.target_screen_y` 再求 pitch，使可见比例与屏幕 Y 都可验收。

求解同时产出几何诊断：`GEOMETRY_SUBJECT_BEHIND_CAMERA`、`GEOMETRY_SUBJECT_OUTSIDE_FOV`、`GEOMETRY_AXIS_SIDE_MISMATCH`（声明的轴线侧与几何推导不一致；正侧定义为从 `from` 看向 `to` 时相机在右手边）。

### 坐标约定

身份旋转的相机沿 `forward_axis` 看，`up_axis` 为上；右向量为 `forward × up`（右手系）或 `up × forward`（左手系）。旋转顺序 yaw → pitch → roll，并且**在任何坐标系下 `+yaw` 都向左转、`+pitch` 都向上抬**（物理右手定则）。这与 Unity 的欧拉角相反，适配 Unity 数据须取反。`forward_axis` 与 `up_axis` 平行会被校验器拒绝（`COORDINATE_SYSTEM_AXES_PARALLEL`）。

## 15.3 文本 prompt 与方言

```bash
cine-ir prompt examples/dialogue.yaml --shot alice_close_up
cine-ir prompt examples/dialogue.yaml --dialect profiles/prompt/generic.json --json
```

IR 枚举不直接进 prompt，而是经 `profiles/prompt/<model>.json` 的方言表映射为摄影词汇（`medium_close_up` → "medium close-up"）。方言文件必须覆盖全部枚举变体与 emitter 用到的短语键，加载时校验完整性。角色用**颜色进图、名字进 prompt** 的方式绑定身份："Alice (red)"，颜色与条件图中的白模颜色一致（D7a/D10）。走位关键帧的 `action` 自由文本原样进入 prompt——这是当前表达角色动作的通道。

## 15.4 View：审片图、运动语法与模型控制分开

```text
1. 时间采样  --sampling per-shot | per-beat | op-endpoints | semantic-events | phase-keyframes | constraint-extrema | cut-pairs | every:N | continuous[:fps]
2. 场景投影  --panel plan,frame,elevation,timeline,metrics
3. 符号叠加  --overlay ... 受 --intent 门控
4. 布局      --layout grid:N | strip:h|v | separate | animate[:fps]（animate:FPS 等价于 --sampling continuous:FPS）
5. 编码      --format svg | png[:scale] | ascii[:cols] | html | markdown[:cols]
```

```bash
# 人类审查：拼格 PNG，含标注、轴线、参考线与诊断
cine-ir view examples/dialogue.yaml --format png -o /tmp/dialogue.png
# 终端速览
cine-ir view examples/dialogue.yaml --format ascii --layout strip:v
# ASCII + 文字（每镜一段 prompt）
cine-ir view examples/dialogue.yaml --format markdown
# 完整审片卡：phase keyframes + global/local geometry + elevation + curves
cine-ir view examples/jaws_beach_dolly_zoom.yaml --intent storyboard \
  --sampling phase-keyframes --panel plan,frame,elevation,timeline,metrics \
  --format svg -o /tmp/jaws-card.svg
# clean control 与独立 scribble cue map；箭头不会烧进 proxy 图
cine-ir view examples/dolly_zoom.yaml --intent model-control --cue-map \
  --sampling op-endpoints --layout separate --format png -o /tmp/control/
# 动态预览：HTML 动画，可直接交给 insight-video 出 mp4
cine-ir view examples/dolly_zoom.yaml --layout animate:12 --format html -o /tmp/previz.html
```

所有编码器消费同一个 `Canvas`（`view::canvas`），因此 SVG 与 ASCII 画的是同一张图。俯视投影沿 `up_axis` 压平、以 `forward_axis` 为页面上方，不假设 Y-up；同一场景的所有面板共用一次拟合，拼格与动画帧的比例一致。

### `--intent`

| intent | 回答的问题 | 默认内容 |
|---|---|---|
| `storyboard` | 镜头从何状态到何状态 | plan/frame，可加 elevation/timeline/metrics |
| `motion` | 相机机制与画面运动如何不同 | camera path、image-motion、focus、occlusion cue |
| `continuity` | 一刀两侧的轴线/视线/方向 | cut pairs |
| `model-control` | 模型实际消费什么 | clean proxy；`--cue-map` 另产独立 scribble 图 |
| `constraints` | 哪个可观察约束超差 | constraint extrema + PASS/FAIL |
| `comparison` | 生成结果与 guidance 哪里不同 | comparison overlay 入口 |

clean model-control 中出现文字或箭头都是缺陷。只有 `ControlProfile.include_cue_map` 为真时才额外输出 cue-map artifact；cue map 没有 silhouette，不污染 beauty/depth/ID。`CueScope::Current` 是默认；`upcoming` 和 `whole-shot` 仅供显式 review summary。

### 画面运动提示（箭头）

语义图层把 `CameraCommand`、`ImageMotion`、`FocusCue`、`OcclusionCue`、`EditCue` 分开。Dolly 的 camera path 和深度相关 radial flow、Zoom 的统一缩放、Dolly Zoom 的主体 bbox lock 与背景 separation、Rack Focus 的焦平面 handoff、Reveal 的 visible-fraction 曲线不再共用一个“运动箭头”。所有语义 item 先 lower 到同一 Canvas，再编码 SVG/PNG/ASCII/HTML；Canvas 支持 opacity/group/clip/z-index/cubic path/vector field/image embedding 与 label collision。

ASCII 对非 ASCII 标签写成 `uXXXX`，不再输出 `?`。PNG 的 Latin 字体内嵌；包含中文时加载 host CJK fallback，并用不同中文字符的栅格结果回归，避免统一 missing-glyph 方框。纯几何与 Latin 产物仍保持 byte-stable。

### HTML 动画与 insight-video

`--layout animate --format html` 产出的页面自包含，根元素 `<main data-composition-id data-duration>`（时长以秒计，取自场景帧数 / 帧率，与采样密度无关），每个采样格带 `data-t` 场景时间，并实现 `window.__insightVideo.seek(t)`：在浏览器中自动播放，被 insight-video 首次 seek 后停止自播、精确定帧，满足其确定性门。`--sampling per-shot --layout animate` 也成立——每格持续到下一个镜头开始：

```bash
insight-video render /tmp/previz.html -o /tmp/previz.mp4
```

## 15.5 Blender：白模条件序列的多通道渲染器

```bash
cine-ir render blender examples/dialogue.yaml --out-dir /tmp/passes \
  --profile profiles/execution/generic-dense.json
cine-ir render blender examples/dialogue.yaml --out-dir /tmp/passes --script-only
BLENDER=/opt/blender/blender cine-ir render blender solved.json --out-dir /tmp/passes --frames 80..160
```

Blender 的角色**不是**出成片，而是渲染带最简人形骨架的黏土白模并一次导出全部主流条件通道，因此「目标模型未定」不阻塞：

| 通道 | 来源 | 编码 |
|---|---|---|
| `beauty` | Cycles 合成层 | 8-bit sRGB PNG，黑天空 + 灰地面 |
| `depth_metric` | Z pass | 32-bit float EXR，米 |
| `depth` | Z pass → Map Range | 16-bit data PNG，近白远黑；manifest 给 near/far |
| `normal` | 材质覆盖：相机空间法线 `(n+1)/2` | 16-bit data PNG |
| `id` | 材质覆盖：Object Info Color | 16-bit data PNG，每个主体一种确定性色 |
| `vector` | Vector pass | OpenEXR 32-bit |
| `openpose` | 独立 view layer 的发光骨架 | COCO-18 关节 + ControlNet 肢体配色，黑底 |
| `occlusion` | proxy mask | 16-bit mask |
| `camera` | intrinsics/extrinsics | JSON |
| `pose_keypoints` | authored pose/gaze/contact | JSON |

payload 还包括 geometry proxy、pose/gaze/contact、rig、phase、constraints/evaluations、prohibitions、screen tracks、exposure、authored lights 与 edit timeline。Beauty 使用作者灯光/曝光/白平衡/DOF/motion blur；没有灯光才使用 neutral fallback。坐标转换全部在 Rust 侧完成，Python 只接收 Blender 世界空间。

执行先渲染到 `.scene.rendering-PID`，验证文件数/格式后写最终 manifest 与 complete marker，再原子发布；失败保留 `.scene.failed-PID` 的 build.py/stdout/stderr，但不会留下貌似成功的 manifest。最终 scene bundle 含 `edit.json` 和 `shots/<id>/{guidance,prompt,negative_prompt,review,controls,metadata,manifest}`，记录 runtime/source/compiled/profile hash、seed 与 frame inventory。

无 Blender 的 CI 环境以 `tests/python/bpy_stub` 桩执行整段脚本，验证控制流与数据管线；涉及 Blender 的改动还需在 Blender 5.2 LTS 中完成真实 `bpy` 运行与输出制品解码验证。

## 15.6 闭环测量

```bash
cine-ir schema --estimated                       # 估计轨迹的 JSON 结构
cine-ir compare examples/dialogue.yaml estimate.json --align similarity
```

`compare` 同时测世界与观众可见结果：position/relative motion、forward/up/horizon、focal、full Sim(3) alignment，以及 bbox height、center drift、background scale、pair separation、optical flow、reveal visibility、focus handoff、phase/cut timing。`--temporal dtw` 可在不同生成速度之间对齐轨迹与屏幕结果，但 phase/cut timing 仍用原始 estimate 计算，不能把节奏错误对齐掉。报告按 constraint 输出 `PASS / FAIL / INDETERMINATE`，而不是只给一个 overall RMSE。

## 15.7 明确不做

- 角色外观身份保真：白模只需可区分（颜色 + 可选人名），不需可辨认；由下游另一模型的输入负责。
- 视图层加载资产：一旦读 `.blend` / `.usd`，它就不再是纯函数。
- 在几何通道上绘制箭头或文字。

---

# 附录：术语与概念边界

| 术语 | 精确定义 | 常见混淆 |
|---|---|---|
| Shot | 剪辑中连续不中断的画面单元 | 不等于一次 Take |
| Take | 同一拍摄方案的一次实际录制 | 多个 Take 可供一个 Shot 选择 |
| Setup | 机位、器材、灯光等现场配置 | 不一定等于单个 Shot |
| Framing | 主体在画框中的组织 | 不只等于景别 |
| Shot Size | 主体在画面中的尺度类别 | 不直接规定焦距或距离 |
| Pan | 机位固定的水平旋转 | 不等于 Truck |
| Tilt | 机位固定的上下旋转 | 不等于 Pedestal |
| Dolly | 摄影机沿前后方向位移 | 不等于 Zoom |
| Truck | 摄影机横向位移 | Track 在不同语境含义不稳定 |
| Zoom | 改变焦距/视场角 | 不改变摄影机位置 |
| Rack Focus | 镜头内改变焦点目标 | 不等于 Zoom |
| Blocking | 人物与动作的时空调度 | 不只是站位 |
| Axis | 为维持空间方向建立的关系/运动线 | 不是永久固定物理线 |
| Screen Direction | 主体在二维画面中的运动/朝向 | 不等于世界坐标方向 |
| Eyeline Match | 镜头间视线与目标位置关系匹配 | 不只检查左右 |
| 30° Rule | 避免小机位变化产生无意跳切的启发式 | 不是精确物理阈值 |
| Coverage | 为编辑准备的镜头集合 | 不是越多越好 |
| POV | 被镜头序列建立的角色所见 | 眼睛高度不自动等于 POV |
| Perspective | 由观察位置决定的投影比例关系 | 焦距改变视场角；移动机位才改变透视 |
| Depth of Field | 可接受清晰范围 | 由多变量共同决定 |
| Shutter Angle | 每帧曝光占帧周期的角度表示 | 影响运动模糊，不只是亮度 |
| Motivated Light | 让观众相信来自场景来源的灯光 | 实际灯具可在画外 |

```text
Narrative Intent
      |
      v
Blocking -----> Continuity Axis
      |               |
      v               v
Framing <---- Camera Position/Angle
      |
      +---- Lens / Focus / Exposure
      +---- Lighting / Color
      |
      v
Shot Sequence -----> Audience Spatial & Emotional Model
```
