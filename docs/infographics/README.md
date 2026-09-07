# cinematography-ir 架构信息图

新图只讲这一条路：作者意图进门，经三道命令收成编译 IR，再由 prompt、view、render 消费。旧总览仍留在 docs/，本目录不改那些文件。条件信号长图仍在 conditioning-pipeline/。

## 视觉合同

- 论点：作者意图以 YAML / JSON 进入，经 validate、analyze、compile 收成一份编译 IR；prompt、view、render 只消费这份产物。
- 读者问题：意图从哪进来、经过什么门、谁吃编译结果？
- 受众：要先看清镜头意图如何变成可消费编译产物、而不想先读源码的人
- 语言：简体中文；命令行动词保留原文
- 媒介：slide-16x9 / 1280×720，安全边距 40 px，底部 80 px（用户指定单页尺寸）
- 语法：单向过程脊 + 三消费者扇出；验证失败与连续性警告各占一条支路
- 主角：止于金色编译 IR 的粗脊，再扇出到三个命令
- 规范源：architecture.svg；页面：index.html；校样：proof.png
- 页面不含函数名、类型名、文件名

## 重建

```bash
python3 docs/infographics/build.py
rsvg-convert -w 1280 -h 720 docs/infographics/architecture.svg -o docs/infographics/proof.png
```

旧文件保持不动：docs/architecture-infographic.tex、docs/architecture.html。条件信号长图保持不动。
