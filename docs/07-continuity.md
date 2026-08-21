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
