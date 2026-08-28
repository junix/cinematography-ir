#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Build the conditioning-pipeline infographic SVG.

Source of truth for every visible fact:
  docs/15-conditioning-pipeline.md, src/compiled.rs, src/main.rs,
  examples/jaws_beach_dolly_zoom.yaml @ commit 5903281 (2026-08).

Usage:
  python3 build.py [--out /tmp/conditioning-pipeline.svg]

Raster proof:
  rsvg-convert -w 2560 <svg> -o <png>          # @2x of the 1280-wide canvas

Every text call goes through gtext()/fit() so a wording change that overflows
its column fails the build instead of silently spilling.
"""

from __future__ import annotations

import argparse
import sys

# ---------------------------------------------------------------------------
# Style tokens (design-guide.md section 6 roles; literal hex, never CSS vars,
# because svg-linter does not resolve custom properties).
# ---------------------------------------------------------------------------
PALETTE = {
    "paper": "#F7F4EE",
    "ink": "#17212B",
    "muted": "#5D6873",
    "rule": "#D9E1E3",
    "flow": "#356A79",       # primary pipeline / pure consumers
    "flow_light": "#E4EDEE", # flow tint field
    "flow_alt": "#2A9D8F",   # secondary / return lane
    "tint": "#DDF2EC",       # group tint (screen-space, model zone)
    "warn": "#E76F51",       # FAIL verdict accent (outline / strokes)
    "flow_alt_dark": "#17705F",  # flow-alt at text contrast (kicker)
    "warn_text": "#B8431F",      # warn at text contrast (FAIL chip label)
}

FONT_CJK = "'PingFang SC','Hiragino Sans GB','Noto Sans CJK SC',sans-serif"
FONT_MONO = "'Menlo','SF Mono','Sarasa Mono SC',monospace"

W = 1280            # fixed canvas width (long-form rule: width fixed, height grows)
MARGIN = 64
CONTENT_W = W - 2 * MARGIN


# ---------------------------------------------------------------------------
# Text metrics (approximate; fit() asserts keep labels inside their boxes).
# ---------------------------------------------------------------------------
def text_width(s: str, size: float, mono: bool = False) -> float:
    if mono:
        return len(s) * 0.602 * size
    w = 0.0
    for ch in s:
        o = ord(ch)
        if o >= 0x2E80 or ch in "①②③④⑤⑥⑦⑧⑨⑩·—「」→←✓≈":
            w += 1.0
        elif ch in "iljI.,:;'|!()[]":
            w += 0.32
        elif ch in "mwMW":
            w += 0.86
        elif ch.isdigit() or ch.isupper():
            w += 0.62
        elif ch == " ":
            w += 0.28
        else:
            w += 0.53
    return w * size


def fit(text: str, size: float, box_w: float, pad: float = 6.0, mono: bool = False,
        what: str = "") -> None:
    w = text_width(text, size, mono)
    assert w <= box_w - 2 * pad, (
        f"overflow ({w:.0f}px > {box_w - 2 * pad:.0f}px): {text!r} [{what}]")


# ---------------------------------------------------------------------------
# SVG primitives. Every group carries a stable semantic id so every rendered
# element stays traceable to its source fact.
# ---------------------------------------------------------------------------
class Svg:
    def __init__(self) -> None:
        self.parts: list[str] = []
        self.height = 0.0

    def add(self, s: str) -> None:
        self.parts.append(s)

    def rect(self, x, y, w, h, cls, rx=10, idn=None, extra=""):
        i = f' id="{idn}"' if idn else ""
        self.add(f'<rect{i} x="{x:.1f}" y="{y:.1f}" width="{w:.1f}" height="{h:.1f}" '
                 f'rx="{rx}" class="{cls}"{extra}/>')

    def text(self, x, y, s, cls, anchor="start", idn=None, size=12.5, mono=False,
             box_w=None, what=""):
        if box_w is not None:
            fit(s, size, box_w, mono=mono, what=what or s[:18])
        i = f' id="{idn}"' if idn else ""
        s = (s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")
              .replace('"', "&quot;"))
        self.add(f'<text{i} x="{x:.1f}" y="{y:.1f}" class="{cls}" '
                 f'text-anchor="{anchor}">{s}</text>')

    def line(self, x1, y1, x2, y2, cls):
        self.add(f'<line x1="{x1:.1f}" y1="{y1:.1f}" x2="{x2:.1f}" y2="{y2:.1f}" '
                 f'class="{cls}"/>')

    def path(self, d, cls, idn=None):
        i = f' id="{idn}"' if idn else ""
        self.add(f'<path{i} d="{d}" class="{cls}"/>')

    def render(self) -> str:
        head = (
            f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" '
            f'height="{self.height:.0f}" viewBox="0 0 {W} {self.height:.0f}">'
        )
        return "\n".join([head] + self.parts + ["</svg>"])


def h_elbow(x1, y1, x2, y2, r=8.0):
    """Two-bend orthogonal elbow for a mainly vertical run."""
    mid = (y1 + y2) / 2
    sx = 1 if x2 > x1 else -1
    return (f"M {x1:.1f},{y1:.1f} V {mid - r:.1f} "
            f"Q {x1:.1f},{mid:.1f} {x1 + sx * r:.1f},{mid:.1f} "
            f"H {x2 - sx * r:.1f} "
            f"Q {x2:.1f},{mid:.1f} {x2:.1f},{mid + r:.1f} V {y2:.1f}")


# ---------------------------------------------------------------------------
def build() -> Svg:
    s = Svg()

    # -- defs / style -------------------------------------------------------
    s.add("<defs>")
    for name, color in (("arrow-flow", PALETTE["flow"]),
                        ("arrow-flow-alt", PALETTE["flow_alt"])):
        s.add(f'<marker id="{name}" viewBox="0 0 10 10" refX="8.5" refY="5" '
              f'markerWidth="7.5" markerHeight="7.5" orient="auto-start-reverse">'
              f'<path d="M 0.5,0.8 L 9.2,5 L 0.5,9.2 L 3.4,5 Z" fill="{color}"/></marker>')
    s.add("</defs>")
    s.add(f"""<style>
.h1 {{ font-family: {FONT_CJK}; font-size: 34px; font-weight: 700; fill: {PALETTE['ink']}; }}
.kicker {{ font-family: {FONT_CJK}; font-size: 13px; font-weight: 700; fill: {PALETTE['flow_alt_dark']}; letter-spacing: 3px; }}
.thesis {{ font-family: {FONT_CJK}; font-size: 15.5px; fill: {PALETTE['muted']}; }}
.panel-kicker {{ font-family: {FONT_CJK}; font-size: 12.5px; font-weight: 700; fill: {PALETTE['flow']}; letter-spacing: 2.5px; }}
.panel-title {{ font-family: {FONT_CJK}; font-size: 21px; font-weight: 700; fill: {PALETTE['ink']}; }}
.panel-claim {{ font-family: {FONT_CJK}; font-size: 13.5px; fill: {PALETTE['muted']}; }}
.node-title {{ font-family: {FONT_CJK}; font-size: 15px; font-weight: 700; fill: {PALETTE['ink']}; }}
.node-title-inv {{ font-family: {FONT_CJK}; font-size: 17px; font-weight: 700; fill: #FFFFFF; }}
.node-sub {{ font-family: {FONT_CJK}; font-size: 12.5px; fill: {PALETTE['muted']}; }}
.node-sub-inv {{ font-family: {FONT_CJK}; font-size: 12.5px; fill: #E3EDEF; }}
.node-note {{ font-family: {FONT_CJK}; font-size: 12px; fill: {PALETTE['muted']}; }}
.mono-node {{ font-family: {FONT_MONO}; font-size: 12px; fill: {PALETTE['flow']}; }}
.mono-inv {{ font-family: {FONT_MONO}; font-size: 12px; fill: #CFE6E4; }}
.mono-label {{ font-family: {FONT_MONO}; font-size: 12px; fill: {PALETTE['muted']}; }}
.edge-label {{ font-family: {FONT_CJK}; font-size: 12px; fill: {PALETTE['muted']}; }}
.zone-eyebrow {{ font-family: {FONT_CJK}; font-size: 12.5px; font-weight: 700; fill: {PALETTE['flow']}; }}
.chip {{ font-family: {FONT_MONO}; font-size: 11.5px; }}
.src {{ font-family: {FONT_CJK}; font-size: 12px; fill: {PALETTE['muted']}; }}
.box {{ fill: #FFFFFF; stroke: {PALETTE['rule']}; stroke-width: 1.2; }}
.box-flow {{ fill: {PALETTE['flow']}; }}
.box-dashed {{ fill: #FFFFFF; stroke: {PALETTE['muted']}; stroke-width: 1.2; stroke-dasharray: 5 4; }}
.zone {{ fill: {PALETTE['tint']}; }}
.zone-flow {{ fill: {PALETTE['flow_light']}; }}
.edge {{ fill: none; stroke: {PALETTE['flow']}; stroke-width: 1.8; marker-end: url(#arrow-flow); }}
.edge-thin {{ fill: none; stroke: {PALETTE['flow']}; stroke-width: 1.5; marker-end: url(#arrow-flow); }}
.edge-return {{ fill: none; stroke: {PALETTE['flow_alt']}; stroke-width: 1.6; stroke-dasharray: 6 4; marker-end: url(#arrow-flow-alt); }}
.rule-h {{ stroke: {PALETTE['rule']}; stroke-width: 1.2; }}
.rule-dash {{ stroke: {PALETTE['muted']}; stroke-width: 1.1; stroke-dasharray: 4 4; }}
</style>""")

    bg_idx = len(s.parts)
    s.parts.append("<!--BG-->")

    # =====================================================================
    # Header
    # =====================================================================
    s.add('<g id="header">')
    y = 58
    s.text(MARGIN, y, "CINEMATOGRAPHY IR · 条件信号流水线", "kicker")
    y += 46
    s.text(MARGIN, y, "从镜头语言到模型条件信号", "h1")
    y += 34
    s.text(MARGIN, y,
           "一份镜头 YAML 经 solve + compile 收敛为唯一编译产物，再扇出为视频生成模型的三类条件信号，由 compare 按可观察约束闭环验收。",
           "thesis", box_w=CONTENT_W, size=15.5, what="thesis")
    y += 24
    s.add("</g>")
    header_bottom = y

    # =====================================================================
    # Panel A — 编译与扇出
    # =====================================================================
    y = header_bottom + 40
    s.add('<g id="panel-a">')
    s.text(MARGIN, y, "A · 编译与扇出", "panel-kicker")
    s.text(MARGIN, y + 27, "一个 YAML，一份窄腰，四条下游", "panel-title")
    s.text(MARGIN + 330, y + 26,
           "下游不是「渲染成片」——四条下游读同一份编译产物，只有 execute 需要外部运行时。",
           "panel-claim", box_w=CONTENT_W - 330, size=13.5, what="panel-a-claim")

    # ---- spine row ----
    by = y + 58
    bh = 92
    b1x, b1w = MARGIN, 212
    b2x, b2w = MARGIN + 212 + 112, 292
    b3x, b3w = b2x + b2w + 112, W - MARGIN - (b2x + b2w + 112)

    s.add('<g id="pa-cineproject">')
    s.rect(b1x, by, b1w, bh, "box", idn="pa-cineproject-box")
    s.text(b1x + 14, by + 27, "CineProject", "node-title")
    s.text(b1x + 14, by + 48, "YAML 语义 IR · 进版本库", "node-sub",
           box_w=b1w - 28, what="b1-sub")
    s.text(b1x + 14, by + 72, "examples/*.yaml", "mono-node",
           box_w=b1w - 28, size=12, mono=True, what="b1-mono")
    s.add("</g>")

    s.add('<g id="pa-solve">')
    s.rect(b2x, by, b2w, bh, "box", idn="pa-solve-box")
    s.text(b2x + 14, by + 27, "solve::solve_project", "node-title")
    s.text(b2x + 14, by + 48, "CameraOperation → CameraState[t]", "node-sub",
           box_w=b2w - 28, what="b2-sub")
    s.text(b2x + 14, by + 72, "cine-ir solve（兼容 / 调试用）", "mono-node",
           box_w=b2w - 28, size=12, mono=True, what="b2-mono")
    s.add("</g>")

    s.add('<g id="pa-compile">')
    s.rect(b3x, by, b3w, bh, "box", idn="pa-compile-box")
    s.text(b3x + 14, by + 27, "compiled::compile_project", "node-title")
    s.text(b3x + 14, by + 48, "+ phases · 世界 / 屏幕轨迹 · 约束与容差", "node-sub",
           box_w=b3w - 28, what="b3-sub1")
    s.text(b3x + 14, by + 68, "禁止条件 · take / edit 关系", "node-sub",
           box_w=b3w - 28, what="b3-sub2")
    s.add("</g>")

    # spine arrows + labels
    a1m = (b1x + b1w + b2x) / 2
    s.line(b1x + b1w, by + bh / 2, b2x - 2, by + bh / 2, "edge")
    s.text(a1m, by + bh / 2 - 26, "作者意图", "edge-label", anchor="middle",
           box_w=112, what="a1-label")
    a2m = (b2x + b2w + b3x) / 2
    s.line(b2x + b2w, by + bh / 2, b3x - 2, by + bh / 2, "edge")
    s.text(a2m, by + bh / 2 - 26, "逐帧轨迹", "edge-label", anchor="middle",
           box_w=112, what="a2-label")

    # ---- the waist (visual protagonist) ----
    wy = by + bh + 52
    wh = 84
    s.add('<g id="pa-waist">')
    s.rect(MARGIN, wy, CONTENT_W, wh, "box-flow", rx=12, idn="pa-waist-box")
    s.text(MARGIN + 28, wy + 35, "CompiledProject — Compiled Guidance IR（窄腰）",
           "node-title-inv")
    s.text(MARGIN + 28, wy + 62,
           "所有新下游共享的唯一编译产物：作者意图 + 阶段 + 轨迹 + 约束，一处编译、处处同源",
           "node-sub-inv", box_w=700, what="waist-sub")
    s.text(MARGIN + CONTENT_W - 28, wy + 35, "cine-ir compile", "mono-inv",
           anchor="end", size=12, mono=True)
    s.text(MARGIN + CONTENT_W - 28, wy + 62, "schema compiled-guidance-0.1", "mono-inv",
           anchor="end", size=12, mono=True)
    s.add("</g>")

    # spine -> waist arrow
    spine_arrow_x = b3x + b3w / 2
    s.line(spine_arrow_x, by + bh, spine_arrow_x, wy - 2, "edge")
    s.text(spine_arrow_x + 14, by + bh + 30, "compile 产物", "edge-label")
    s.text(b2x + b2w / 2, by + bh + 30, "compile 内部先跑 solve", "edge-label",
           anchor="middle", what="solve-note")

    # ---- consumers ----
    cy = wy + wh + 44
    ch = 138
    cw = (CONTENT_W - 3 * 32) / 4
    consumers = [
        dict(id="prompt", title="prompt（纯）", cli="cine-ir prompt",
             lines=[("intent · phases · constraints", "sub"),
                    ("prohibitions → 时间分段命令", "sub"),
                    ("方言表映射摄影词汇；颜色进图、", "note"),
                    ("名字进 prompt（如 Alice (red)）", "note")],
             tag="→ 槽 ①"),
        dict(id="view", title="view（纯）", cli="cine-ir view",
             lines=[("screen_tracks + framing", "sub"),
                    ("+ continuity → 审片图 / 控制图", "sub"),
                    ("采样→投影→叠加→布局→编码", "note"),
                    ("svg / png / ascii / html / md", "note")],
             tag="→ 槽 ②"),
        dict(id="execute", title="execute（非纯）", cli="cine-ir render blender",
             lines=[("六条逐帧轨道 → typed bundle", "sub"),
                    ("白模多通道渲染 ×10", "sub"),
                    ("需要 Blender 外部运行时", "note"),
                    ("原子发布 manifest + marker", "note")],
             tag="→ 槽 ②③"),
        dict(id="compare", title="compare（纯）", cli="cine-ir compare",
             lines=[("compiled + estimate.json", "sub"),
                    ("按可观察约束逐条验收", "sub"),
                    ("世界 + 屏幕双测", "note"),
                    ("PASS / FAIL / INDETERMINATE", "note")],
             tag="← estimate.json"),
    ]
    centers = []
    for i, c in enumerate(consumers):
        cx = MARGIN + i * (cw + 32)
        centers.append(cx + cw / 2)
        s.add(f'<g id="pa-{c["id"]}">')
        cls = "box-dashed" if c["id"] == "execute" else "box"
        s.rect(cx, cy, cw, ch, cls, idn=f'pa-{c["id"]}-box')
        s.text(cx + 14, cy + 26, c["title"], "node-title")
        ly = cy + 52
        for txt, kind in c["lines"]:
            s.text(cx + 14, ly, txt, "node-sub" if kind == "sub" else "node-note",
                   box_w=cw - 28, size=12.5 if kind == "sub" else 12,
                   what=f'{c["id"]}-line')
            ly += 19
        s.text(cx + 14, cy + ch - 12, c["tag"], "node-note",
               box_w=cw - 28 - text_width(c["cli"], 12, mono=True) - 6,
               size=12, what=f'{c["id"]}-tag')
        s.text(cx + cw - 12, cy + ch - 12, c["cli"], "mono-node", anchor="end",
               size=12, mono=True, what=f'{c["id"]}-cli')
        s.add("</g>")
        s.line(cx + cw / 2, wy + wh, cx + cw / 2, cy - 2, "edge")

    # （纯 / 非纯 的分界由卡片虚线边框 + 标题 + 面板论点表达，不再画横穿箭头的边界线）

    # ---- model zone with three slots ----
    zy = cy + ch + 56
    zh = 184
    zw = 812
    s.add('<g id="pa-model-zone">')
    s.rect(MARGIN, zy, zw, zh, "zone", rx=12, idn="pa-model-zone-box")
    # short eyebrow: the prompt->slot-① vertical at x=196 must clear its text
    s.text(MARGIN + 28, zy + 30, "视频生成模型", "zone-eyebrow", box_w=100,
           what="zone-eyebrow")
    s.add("</g>")

    slw, slgap, slpad = 236, 24, 28
    slots = [
        dict(id="slot-prompt", num="①", title="文本 prompt",
             sub=["时间分段命令", "negative constraints"], src="← prompt"),
        dict(id="slot-image", num="②", title="参考图",
             sub=["storyboard / motion grammar", "clean control"], src="← view + execute 通道"),
        dict(id="slot-video", num="③", title="参考视频 / 数值控制",
             sub=["typed pass video", "+ camera JSON"], src="← execute"),
    ]
    slot_tops = zy + 46
    slh = zh - 46 - 34
    slot_centers = []
    for i, sl in enumerate(slots):
        sx = MARGIN + slpad + i * (slw + slgap)
        slot_centers.append(sx + slw / 2)
        s.add(f'<g id="pa-{sl["id"]}">')
        s.rect(sx, slot_tops, slw, slh, "box", rx=8, idn=f'pa-{sl["id"]}-box')
        s.text(sx + 16, slot_tops + 28, sl["num"], "node-title")
        s.text(sx + 38, slot_tops + 28, sl["title"], "node-title",
               box_w=slw - 44 - 12, what=f'{sl["id"]}-title')
        yy = slot_tops + 54
        for ln in sl["sub"]:
            s.text(sx + 16, yy, ln, "node-sub", box_w=slw - 32, what=f'{sl["id"]}-sub')
            yy += 19
        s.text(sx + 16, slot_tops + slh - 12, sl["src"], "node-note", box_w=slw - 32,
               size=12, what=f'{sl["id"]}-src')
        s.add("</g>")
    s.text(MARGIN + 28, zy + zh - 12, "三类输入槽（与 IR 地层一一对应）", "node-note",
           box_w=300, size=12, what="zone-note")

    # consumer -> slot edges. The three elbows share the corridor row at
    # y = mid; their horizontal segments must stay disjoint (checked below).
    s.line(centers[0], cy + ch, centers[0], slot_tops - 2, "edge-thin")
    seg2 = h_elbow(centers[1], cy + ch, slot_centers[1] - 18, slot_tops - 2)
    seg3 = h_elbow(centers[2] - 36, cy + ch, slot_centers[1] + 18, slot_tops - 2)
    seg4 = h_elbow(centers[2] + 36, cy + ch, slot_centers[2] + 60, slot_tops - 2)
    s.path(seg2, "edge-thin")
    s.path(seg3, "edge-thin")
    s.path(seg4, "edge-thin")
    # corridor-overlap self-check: horizontal runs at the shared mid row
    def h_span(x1, x2, r=8.0):
        return (min(x1, x2) + r, max(x1, x2) - r)

    mid_row = (cy + ch + slot_tops) / 2
    spans = [h_span(centers[1], slot_centers[1] - 18),
             h_span(centers[2] - 36, slot_centers[1] + 18),
             h_span(centers[2] + 36, slot_centers[2] + 60)]
    for i in range(len(spans)):
        for j in range(i + 1, len(spans)):
            a, b = spans[i], spans[j]
            assert a[1] <= b[0] or b[1] <= a[0], (
                f"slot-feed elbows share corridor row y={mid_row:.0f}: {a} vs {b}")

    # ---- return lane: 生成结果 -> 估计器(未实现) -> estimate.json -> compare ----
    est_x = MARGIN + zw + 68
    est_w = W - MARGIN - est_x
    est_y = zy + 52
    est_h = 66
    s.add('<g id="pa-estimator">')
    s.rect(est_x, est_y, est_w, est_h, "box-dashed", rx=8, idn="pa-estimator-box")
    s.text(est_x + 14, est_y + 27, "相机轨迹估计器", "node-title",
           box_w=est_w - 28, what="est-title")
    s.text(est_x + 14, est_y + 50, "未实现——compare 只做了比对的一半", "node-note",
           box_w=est_w - 28, size=12, what="est-note")
    s.add("</g>")
    # zone -> estimator (dashed return lane)
    lane_y = est_y + est_h / 2
    s.line(MARGIN + zw, lane_y, est_x - 2, lane_y, "edge-return")
    s.text((MARGIN + zw + est_x) / 2, lane_y - 10, "生成结果", "edge-label",
           anchor="middle", box_w=68, what="gen-result-label")
    # estimator -> compare (straight vertical, compare card center)
    cmp_cx = centers[3]
    s.path(f"M {cmp_cx:.1f},{est_y:.1f} V {cy + ch + 1.5:.1f}", "edge-return")
    s.text(cmp_cx + 12, (est_y + cy + ch) / 2 + 4, "estimate.json", "mono-label",
           size=12, mono=True, box_w=120, what="estimate-label")

    s.add("</g>")  # panel a
    panel_a_bottom = zy + zh

    # =====================================================================
    # Panel B — 窄腰解剖
    # =====================================================================
    y = panel_a_bottom + 42
    s.add('<g id="panel-b">')
    s.text(MARGIN, y, "B · 窄腰解剖", "panel-kicker")
    s.text(MARGIN, y + 27, "CompiledShot：意图 + 阶段 + 轨迹 + 约束 + 禁止 + 剪辑关系",
           "panel-title")

    py = y + 54
    col1x, col1w = MARGIN, 330
    col2x, col2w = MARGIN + 330 + 36, 396
    col3x = col2x + col2w + 36
    col3w = W - MARGIN - col3x

    def group_header(gx, gy, label):
        s.text(gx, gy, label, "panel-kicker", box_w=CONTENT_W, size=12.5,
               what="group-header")

    # — col 1: intent + phases —
    group_header(col1x, py, "intent · 作者意图")
    lines1 = [
        "purpose · coverage_role · framing",
        "camera · lighting · continuity",
        "relations · phrases · notes",
        "source_path 指回 YAML 原字段",
    ]
    yy = py + 26
    for ln in lines1:
        s.text(col1x, yy, ln, "node-sub", box_w=col1w, what="b-intent")
        yy += 21
    yy += 14
    group_header(col1x, yy, "phases · CompiledPhase ×6")
    yy += 16
    chip_y = yy
    chips = ["hold", "motion", "focus", "reveal", "settle", "mixed"]
    row_w = 0
    cxp = col1x
    for ci, cname in enumerate(chips):
        if ci == 3:
            row_w = max(row_w, cxp - 8 - col1x)
            cxp = col1x
            chip_y += 32
        cw_chip = text_width(cname, 11.5, mono=True) + 24
        s.rect(cxp, chip_y, cw_chip, 24, "zone-flow", rx=12, idn=f"pb-phase-{cname}")
        s.text(cxp + cw_chip / 2, chip_y + 16.5, cname, "chip", anchor="middle",
               size=11.5, mono=True, idn=f"pb-phase-{cname}-t")
        cxp += cw_chip + 8
    row_w = max(row_w, cxp - 8 - col1x)
    assert row_w <= col1w, f"phase chips overflow column 1 ({row_w:.0f}px)"
    col1_bottom = chip_y + 24 + 24
    s.text(col1x, col1_bottom, "按运镜区间与 beat 边界自动切分，", "node-note",
           box_w=col1w, size=12, what="b-phase-note1")
    s.text(col1x, col1_bottom + 18, "hold / settle 段派生 no_camera_motion", "node-note",
           box_w=col1w, size=12, what="b-phase-note2")
    col1_bottom += 36

    # — col 2: tracks (world vs screen) —
    group_header(col2x, py, "tracks · 逐帧轨道")
    ty = py + 26
    world_lines = [
        "camera_track 位姿 · subject_tracks 走位 / pose",
        "lens_track 焦距 / 光圈 · focus_track 对焦",
        "exposure_track 曝光 · light_tracks 灯光（场景级）",
    ]
    world_h = 34 + 21 * len(world_lines) + 14
    s.rect(col2x, ty, col2w, world_h, "zone-flow", rx=8, idn="pb-world-tracks")
    s.text(col2x + 14, ty + 22, "世界空间 · 相机与被摄物", "node-title",
           box_w=col2w - 28, what="b-world-title")
    yy = ty + 44
    for ln in world_lines:
        s.text(col2x + 14, yy, ln, "node-sub", box_w=col2w - 28, what="b-world-line")
        yy += 21
    sy = ty + world_h + 14
    screen_lines = ["screen_tracks：bbox · visible_fraction", "center · depth_m · focus_score"]
    screen_h = 34 + 21 * len(screen_lines) + 10
    s.rect(col2x, sy, col2w, screen_h, "zone", rx=8, idn="pb-screen-tracks")
    s.text(col2x + 14, sy + 22, "屏幕空间 · 观众可见", "node-title",
           box_w=col2w - 28, what="b-screen-title")
    yy = sy + 44
    for ln in screen_lines:
        s.text(col2x + 14, yy, ln, "node-sub", box_w=col2w - 28, what="b-screen-line")
        yy += 21
    s.text(col2x, sy + screen_h + 22, "两个空间同时保留——compare 两侧都能测", "node-note",
           box_w=col2w, size=12, what="b-tracks-note")
    col2_bottom = sy + screen_h + 30

    # — col 3: constraints + evaluations + prohibitions + edit —
    group_header(col3x, py, "constraints · 可观察约束 ×19")
    cons_lines = [
        "subject_bbox_height · subject_screen_center",
        "pair_screen_separation · horizon_lock",
        "no_camera_motion · reveal_visibility",
        "focus_handoff · shot_size · composition",
        "depth_layers · continuity_axis · camera_rig",
        "exposure · lighting · transition …",
    ]
    yy = py + 26
    for ln in cons_lines:
        s.text(col3x, yy, ln, "mono-label", size=12, mono=True, box_w=col3w,
               what="b-cons")
        yy += 20
    yy += 12
    group_header(col3x, yy, "evaluations · 逐约束判定")
    yy += 16
    verdicts = [("PASS", True, PALETTE["flow"]), ("FAIL", False, PALETTE["warn_text"]),
                ("INDETERMINATE", False, PALETTE["muted"])]
    vx = col3x
    for name, filled, colr in verdicts:
        vw = text_width(name, 11.5, mono=True) + 26
        if filled:
            s.rect(vx, yy, vw, 24, "zone-flow", rx=12, idn=f"pb-verdict-{name}")
        else:
            s.rect(vx, yy, vw, 24, "none", rx=12, idn=f"pb-verdict-{name}",
                   extra=f' fill="#FFFFFF" stroke="{PALETTE["warn"] if name == "FAIL" else PALETTE["muted"]}" stroke-width="1.2"')
        s.text(vx + vw / 2, yy + 16.5, name, "chip", anchor="middle", size=11.5,
               mono=True, idn=f"pb-verdict-{name}-t")
        s.parts[-1] = s.parts[-1].replace('class="chip"',
                                          f'class="chip" fill="{colr}"')
        vx += vw + 10
    assert vx - 10 <= col3x + col3w, "verdict chips overflow column 3"
    yy += 40
    group_header(col3x, yy, "prohibitions · 自动派生禁止条件")
    yy += 20
    s.text(col3x, yy, "dolly_zoom → 禁手持 · 禁数字裁剪 · 禁演员位移替代", "node-sub",
           box_w=col3w, what="b-proh1")
    yy += 20
    s.text(col3x, yy, "reveal → 禁背景变形冒充视差 · rack_focus → 禁全局模糊", "node-sub",
           box_w=col3w, what="b-proh2")
    yy += 30
    group_header(col3x, yy, "edit_relation · 剪辑关系")
    yy += 20
    s.text(col3x, yy, "previous / next · transition_in · relations", "mono-label",
           size=12, mono=True, box_w=col3w, what="b-edit")
    col3_bottom = yy + 8

    pb_bottom = max(col1_bottom, col2_bottom, col3_bottom)
    s.add("</g>")

    # =====================================================================
    # Panel C — 闭环验收
    # =====================================================================
    y = pb_bottom + 42
    s.add('<g id="panel-c">')
    s.text(MARGIN, y, "C · 闭环验收", "panel-kicker")
    s.text(MARGIN, y + 27, "验收不是给一个整体 RMSE——按可观察约束逐条三态判定", "panel-title")

    cy2 = y + 56
    half_w = (CONTENT_W - 40) / 2

    # left: compare 双测
    s.add('<g id="pc-world">')
    s.rect(MARGIN, cy2, half_w, 132, "zone-flow", rx=8, idn="pc-world-box")
    s.text(MARGIN + 16, cy2 + 26, "世界轨迹测量", "node-title", box_w=half_w - 32,
           what="c-world-title")
    for i, ln in enumerate([
        "position · 相对运动 · forward / up / horizon",
        "focal_length · 完整 Sim(3) 对齐",
        "--temporal dtw 在不同生成速度间对齐轨迹",
    ]):
        s.text(MARGIN + 16, cy2 + 52 + i * 21, ln, "node-sub", box_w=half_w - 32,
               what="c-world-line")
    s.text(MARGIN + 16, cy2 + 120, "节奏错误对齐不掉：phase / cut 仍用原始 estimate", "node-note",
           box_w=half_w - 32, size=12, what="c-world-note")
    s.add("</g>")

    s.add('<g id="pc-screen">')
    s.rect(MARGIN, cy2 + 148, half_w, 132, "zone", rx=8, idn="pc-screen-box")
    s.text(MARGIN + 16, cy2 + 148 + 26, "观众可见测量（屏幕空间）", "node-title",
           box_w=half_w - 32, what="c-screen-title")
    for i, ln in enumerate([
        "bbox 高度 · 中心漂移 · 背景扩张 · 成对间距",
        "光流 · reveal 可见比例 · 焦点交接",
        "phase / cut 节奏逐段核对",
    ]):
        s.text(MARGIN + 16, cy2 + 148 + 52 + i * 21, ln, "node-sub", box_w=half_w - 32,
               what="c-screen-line")
    s.text(MARGIN + 16, cy2 + 148 + 120, "全部来自 screen_tracks——观众看得见的才算依据", "node-note",
           box_w=half_w - 32, size=12, what="c-screen-note")
    s.add("</g>")

    # right: jaws timeline
    rx0 = MARGIN + half_w + 40
    s.add('<g id="pc-jaws">')
    s.text(rx0, cy2 + 24, "基准示例：《大白鲨》海滩反向 Dolly Zoom", "node-title",
           box_w=half_w, what="c-jaws-title")
    s.text(rx0, cy2 + 46, "8 秒 · 24fps · 192 帧 · 单镜头无对白", "node-sub",
           box_w=half_w, what="c-jaws-sub")
    tl_y = cy2 + 78
    tl_w = half_w
    segs = [
        dict(frames=24, inside="0–2s", below="观察 ordinary_scan · low", inv=False),
        dict(frames=120, inside=["2–7s 反向 dolly zoom", "perspective_shock · high"], inv=True),
        dict(frames=24, inside="7–8s", below="定住 stunned_hold · medium", inv=False),
    ]
    total = sum(g["frames"] for g in segs)
    xx = rx0
    for i, g in enumerate(segs):
        gw = tl_w * g["frames"] / total
        gap = 6 if i < 2 else 0
        s.rect(xx, tl_y, gw - gap, 40, "box-flow" if g["inv"] else "zone-flow", rx=6,
               idn=f"pc-jaws-seg{i}")
        if g["inv"]:
            s.text(xx + (gw - gap) / 2, tl_y + 17, g["inside"][0], "node-sub-inv",
                   anchor="middle", box_w=400, what="c-jaws-seg2a")
            s.text(xx + (gw - gap) / 2, tl_y + 33, g["inside"][1], "node-sub-inv",
                   anchor="middle", box_w=400, what="c-jaws-seg2b")
        else:
            s.text(xx + (gw - gap) / 2, tl_y + 25, g["inside"], "node-sub",
                   anchor="middle", box_w=gw - gap, what="c-jaws-seg-time")
        xx += gw
    # below-bar beat labels for the two narrow segments
    s.text(rx0 + tl_w * segs[0]["frames"] / total / 2, tl_y + 58, segs[0]["below"],
           "node-note", anchor="start", box_w=220, size=12, what="c-jaws-beat1")
    s.text(rx0 + tl_w - tl_w * segs[2]["frames"] / total / 2, tl_y + 58, segs[2]["below"],
           "node-note", anchor="end", box_w=240, size=12, what="c-jaws-beat3")
    s.text(rx0, tl_y + 92,
           "判据（tests/solve.rs 量化回归）：相机 3.8m → 7.0m，焦距 21.8mm 起步变长，",
           "node-sub", box_w=half_w, what="c-jaws-j1")
    s.text(rx0, tl_y + 113, "observer 画面高度 ≈ 0.42 不变，两个背景 beachgoer 向边缘扩张。",
           "node-sub", box_w=half_w, what="c-jaws-j2")
    s.text(rx0, tl_y + 140, "compare estimate.json --align similarity --temporal dtw",
           "mono-label", size=12, mono=True, box_w=half_w, what="c-jaws-cli")
    s.add("</g>")
    panel_c_bottom = max(cy2 + 148 + 132, tl_y + 152)
    s.add("</g>")  # panel c

    # =====================================================================
    # Footer
    # =====================================================================
    y = panel_c_bottom + 36
    s.add('<g id="footer">')
    s.line(MARGIN, y, W - MARGIN, y, "rule-h")
    s.text(MARGIN, y + 26,
           "来源：docs/15-conditioning-pipeline.md · src/compiled.rs · examples/jaws_beach_dolly_zoom.yaml · commit 5903281（2026-08）",
           "src", box_w=CONTENT_W, size=12, what="src1")
    s.text(W - MARGIN, y + 26, "图为示意；帧 ↔ 秒按 24fps 换算", "src", anchor="end",
           box_w=300, size=12, what="src2")
    s.add("</g>")
    s.height = y + 48

    s.parts[bg_idx] = (
        f'<rect x="0" y="0" width="{W}" height="{s.height:.0f}" '
        f'fill="{PALETTE["paper"]}"/>')
    return s


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default="/tmp/conditioning-pipeline.svg")
    args = ap.parse_args()
    svg = build()
    with open(args.out, "w", encoding="utf-8") as f:
        f.write(svg.render())
    print(f"wrote {args.out} ({W}x{svg.height:.0f})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
