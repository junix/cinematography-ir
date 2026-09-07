# 视觉契约 · cinematography-ir 编译窄腰

写于绘图之前。Class B：作者意图 YAML/JSON 经 validate / analyze / compile，收成编译 IR，再由 prompt / view / render 消费。页上不出现函数名、类型名、文件名；命令行动词可以出现。

## 读者

要先看清意图如何进门、如何收成窄腰、谁读编译结果，而不想先读源码的人。

## 论题（一句）

作者意图以 YAML / JSON 进入，经 validate、analyze、compile 收成一份编译 IR；prompt、view、render 只消费这份产物。

## 读者问题

意图从哪进来、经过什么门、谁吃编译结果？

## 语言

简体中文。命令行动词保留源拼写：validate、analyze、compile、prompt、view、render。页上禁止函数名、类型名、文件名。

## 证据（只收录已核对的事实）

| 可见主张 | 来源 | 备注 |
|---|---|---|
| 作者以 YAML / JSON 写下镜头意图 | README 分层与使用 | 叙事、调度、镜头、运镜、连续性同在一份意图里 |
| 门禁命令：validate、analyze、compile | README 开篇 CLI | 页上不写内部函数 |
| validate 拦结构与引用错误；未知字段直接拒绝 | 验证说明 §13.1 | Error 使文件无法可靠解释 |
| analyze 报轴线、银幕方向、视线、30°；是启发式不是禁令 | README 当前验证范围 | 故意例外须显式写出 |
| compile 产出下游共享的编译 IR 窄腰 | 分层架构 §11.3；条件信号 §15.2 | 不只是逐帧位姿 |
| prompt / view / render 消费编译结果 | 条件信号 §15.1；README 使用 | 不再重解作者运镜命令 |
| 编译 IR 保留意图、相位、轨道、约束、禁止、剪辑关系 | README 开篇；§11.3 | 页上不写类型名 |

不画：六层蛋糕、源码文件卡片、函数名、四条下游全表（那是条件信号长图）。

## 叙事顺序

1. 左上论题；右上指纹安静。
2. 左列作者意图（YAML / JSON）提交进 validate。
3. 粗脊：validate → analyze → compile → 编译 IR。
4. validate 向下拒绝结构失败；analyze 向下标出连续性警告。
5. 编译 IR 扇出到 prompt、view、render。
6. 一条虚线标出下游消费。

## 语法

单向过程脊 + 失败支路 + 三消费者扇出。主角是加粗 Fog 脊（16 px）止于金色编译 IR。

## 盒子

只有作者意图（持久文稿）和编译 IR（终点产物）有边或色块。阶段用编号圆。边界是一条虚线。

## 媒介与尺寸

slide-16x9 / 1280×720，安全边 40 px，底部 80 px。校样走 rsvg-convert。中文不小于 12 px。零 CDN。

## 调色

Paper `#F7F4EE` · Ink `#17212B` · Slate `#5D6873` · Fog `#D9E1E3` · Ocean `#356A79` · Teal `#2A9D8F` · Mint `#DDF2EC` · Coral `#E76F51` · Gold `#E9C46A`。饱和角色：Ocean 主路、Coral 拒、Gold 编译 IR（终点）。警告支路用 Slate 虚线。render 用 Teal 标为执行消费。

## 与旧图的关系

旧 `docs/architecture-infographic.tex` 与 `docs/architecture.html` **不改**。条件信号长图仍在 `docs/infographics/conditioning-pipeline/`。
