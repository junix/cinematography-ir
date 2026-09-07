#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Build the cinematography-ir Class B architecture infographic (1280x720).

Canonical outputs: architecture.svg, index.html, README.md, VERIFICATION.md
Chinese copy lives in data/copy.json and is emitted as UTF-8 by this script.
Rebuild from the project root:

    python3 docs/infographics/build.py
    rsvg-convert -w 1280 -h 720 docs/infographics/architecture.svg -o docs/infographics/proof.png

Do not edit docs/architecture-infographic.tex or its sibling svg/html.
"""

from __future__ import annotations

import html
import json
import re
from pathlib import Path

OUT = Path(__file__).resolve().parent
W, H = 1280, 720

PAPER = "#F7F4EE"
INK = "#17212B"
SLATE = "#5D6873"
FOG = "#D9E1E3"
OCEAN = "#356A79"
TEAL = "#2A9D8F"
MINT = "#DDF2EC"
CORAL = "#E76F51"
GOLD = "#E9C46A"

SERIF = "Source Han Serif SC, Songti SC, STSong, serif"

FORBIDDEN = (
    r"\b\w+\.rs\b",
    r"\bCineProject\b",
    r"\bCompiledProject\b",
    r"\bCompiledShot\b",
    r"\bCameraState\b",
    r"\bCameraOperation\b",
    r"\bcompile_project\b",
    r"\bvalidate_project\b",
    r"\banalyze_continuity\b",
    r"src/",
    r"file:line",
)


def load_copy() -> dict[str, str]:
    path = OUT / "data" / "copy.json"
    data = json.loads(path.read_text(encoding="utf-8"))
    assert isinstance(data, dict)
    return {str(k): str(v) for k, v in data.items()}


T = load_copy()


def esc(s: str) -> str:
    return html.escape(s, quote=True)


def text_width(s: str, size: float) -> float:
    w = 0.0
    for ch in s:
        o = ord(ch)
        if o >= 0x2E80 or ch in "·—「」→←":
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


def fit(s: str, size: float, box_w: float, pad: float = 6.0, what: str = "") -> None:
    w = text_width(s, size)
    assert w <= box_w - 2 * pad, (
        f"overflow ({w:.0f}px > {box_w - 2 * pad:.0f}px): {s!r} [{what}]"
    )


def text(x, y, s, *, fill, size, weight=400, anchor="start", extra="") -> str:
    return (
        f'<text x="{x:.1f}" y="{y:.1f}" fill="{fill}" font-family="{SERIF}" '
        f'font-size="{size}" font-weight="{weight}" text-anchor="{anchor}"{extra}>'
        f"{esc(s)}</text>"
    )


def elbow_hh(x1, y1, x2, y2, mid=None, r=8.0) -> str:
    if abs(y1 - y2) < 0.6:
        return f"M {x1:.1f},{y1:.1f} H {x2:.1f}"
    if abs(x1 - x2) < 0.6:
        return f"M {x1:.1f},{y1:.1f} V {y2:.1f}"
    if mid is None:
        mid = (x1 + x2) / 2.0
    dx = 1.0 if x2 >= x1 else -1.0
    dy = 1.0 if y2 >= y1 else -1.0
    r = max(2.0, min(r, abs(mid - x1) - 2.0, abs(x2 - mid) - 2.0, abs(y2 - y1) / 2.0 - 1.0))
    return (
        f"M {x1:.1f},{y1:.1f} "
        f"H {mid - dx * r:.1f} "
        f"Q {mid:.1f},{y1:.1f} {mid:.1f},{y1 + dy * r:.1f} "
        f"V {y2 - dy * r:.1f} "
        f"Q {mid:.1f},{y2:.1f} {mid + dx * r:.1f},{y2:.1f} "
        f"H {x2:.1f}"
    )


def elbow_vh(x1, y1, x2, y2, r=8.0) -> str:
    if abs(x1 - x2) < 0.6:
        return f"M {x1:.1f},{y1:.1f} V {y2:.1f}"
    if abs(y1 - y2) < 0.6:
        return f"M {x1:.1f},{y1:.1f} H {x2:.1f}"
    dy = 1.0 if y2 >= y1 else -1.0
    dx = 1.0 if x2 >= x1 else -1.0
    r = max(2.0, min(r, abs(y2 - y1) - 2.0, abs(x2 - x1) - 2.0))
    return (
        f"M {x1:.1f},{y1:.1f} "
        f"V {y2 - dy * r:.1f} "
        f"Q {x1:.1f},{y2:.1f} {x1 + dx * r:.1f},{y2:.1f} "
        f"H {x2:.1f}"
    )


def sweep_code_detail(blob: str) -> None:
    for pat in FORBIDDEN:
        hit = re.search(pat, blob)
        assert hit is None, f"code detail on page: {pat} -> {hit.group(0)!r}"


def build_svg() -> str:
    store = (56.0, 308.0, 168.0, 66.0)
    store_out = (store[0] + store[2], store[1] + store[3] / 2.0)
    v = (392.0, 296.0)
    a = (608.0, 432.0)
    c = (824.0, 296.0)
    ir = (1072.0, 352.0)
    p = (928.0, 552.0)
    vw = (1072.0, 572.0)
    rnd = (1196.0, 552.0)
    rs, rc, rr = 18.0, 11.0, 3.5
    irx, iry = 102.0, 56.0

    v_w, v_e, v_s = (v[0] - rs, v[1]), (v[0] + rs, v[1]), (v[0], v[1] + rs)
    a_w, a_e, a_s = (a[0] - rs, a[1]), (a[0] + rs, a[1]), (a[0], a[1] + rs)
    c_w, c_e = (c[0] - rs, c[1]), (c[0] + rs, c[1])
    ir_w = (ir[0] - irx, ir[1])
    ir_sw = (ir[0] - 40.0, ir[1] + iry)
    ir_s = (ir[0], ir[1] + iry)
    ir_se = (ir[0] + 40.0, ir[1] + iry)
    p_n = (p[0], p[1] - rc)
    vw_n = (vw[0], vw[1] - rc)
    rnd_n = (rnd[0], rnd[1] - rc)

    fit(T["title"], 28, 720, what="title")
    fit(T["thesis1"], 14, 860, what="thesis1")
    fit(T["thesis2"], 14, 860, what="thesis2")
    fit(T["intent"], 15, store[2], what="intent")
    fit(T["intent_s"], 12, store[2], what="intent_s")
    fit(T["ir"], 18, irx * 2 - 16, what="ir")
    fit(T["ir_k"], 13, irx * 2 - 16, what="ir_k")

    parts: list[str] = []
    parts.append(
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" '
        f'viewBox="0 0 {W} {H}" role="img" xml:lang="zh-CN" '
        f'aria-labelledby="svg-title svg-desc">'
    )
    parts.append(f'<title id="svg-title">{esc(T["html_title"])}</title>')
    parts.append(f'<desc id="svg-desc">{esc(T["html_alt"])}</desc>')
    parts.append("<defs>")
    parts.append(
        f'<marker id="arrow-flow" markerWidth="10" markerHeight="8" refX="9" refY="4" '
        f'orient="auto" markerUnits="userSpaceOnUse">'
        f'<path d="M0,0 L10,4 L0,8 Z" fill="{OCEAN}"/></marker>'
    )
    parts.append(
        f'<marker id="arrow-warn" markerWidth="10" markerHeight="8" refX="9" refY="4" '
        f'orient="auto" markerUnits="userSpaceOnUse">'
        f'<path d="M0,0 L10,4 L0,8 Z" fill="{CORAL}"/></marker>'
    )
    parts.append(
        f'<marker id="arrow-muted" markerWidth="10" markerHeight="8" refX="9" refY="4" '
        f'orient="auto" markerUnits="userSpaceOnUse">'
        f'<path d="M0,0 L10,4 L0,8 Z" fill="{SLATE}"/></marker>'
    )
    parts.append("</defs>")
    parts.append(f'<rect id="page" x="0" y="0" width="{W}" height="{H}" fill="{PAPER}"/>')

    parts.append(
        f'<ellipse id="zone-waist" cx="{ir[0]:.1f}" cy="{ir[1]:.1f}" '
        f'rx="132" ry="86" fill="{MINT}"/>'
    )
    parts.append(
        f'<line id="boundary-consume" x1="860" y1="468" x2="980" y2="468" '
        f'stroke="{SLATE}" stroke-width="1.2" stroke-dasharray="5 4"/>'
    )

    mid_sv, mid_va, mid_ac, mid_ci = 308.0, 500.0, 716.0, 928.0
    spine = [
        ("spine-intent-validate", elbow_hh(store_out[0], store_out[1], v_w[0], v_w[1], mid_sv, 8)),
        ("spine-validate-analyze", elbow_hh(v_e[0], v_e[1], a_w[0], a_w[1], mid_va, 8)),
        ("spine-analyze-compile", elbow_hh(a_e[0], a_e[1], c_w[0], c_w[1], mid_ac, 8)),
        ("spine-compile-ir", elbow_hh(c_e[0], c_e[1], ir_w[0], ir_w[1], mid_ci, 8)),
    ]
    for sid, d in spine:
        parts.append(
            f'<path id="{sid}" d="{d}" fill="none" stroke="{FOG}" stroke-width="16" '
            f'stroke-linecap="butt" stroke-linejoin="round"/>'
        )

    edges = [
        (
            "edge-fail",
            f"M {v_s[0]:.1f},{v_s[1]:.1f} V 500.0",
            CORAL,
            1.5,
            "url(#arrow-warn)",
            "",
        ),
        (
            "edge-warn",
            f"M {a_s[0]:.1f},{a_s[1]:.1f} V 500.0",
            SLATE,
            1.4,
            "url(#arrow-muted)",
            ' stroke-dasharray="6 4"',
        ),
        (
            "edge-prompt",
            elbow_vh(ir_sw[0], ir_sw[1], p_n[0], p_n[1], 8),
            OCEAN,
            1.7,
            "url(#arrow-flow)",
            "",
        ),
        (
            "edge-view",
            f"M {ir_s[0]:.1f},{ir_s[1]:.1f} V {vw_n[1]:.1f}",
            OCEAN,
            1.7,
            "url(#arrow-flow)",
            "",
        ),
        (
            "edge-render",
            elbow_vh(ir_se[0], ir_se[1], rnd_n[0], rnd_n[1], 8),
            OCEAN,
            1.7,
            "url(#arrow-flow)",
            "",
        ),
    ]
    for eid, d, color, sw, marker, extra in edges:
        parts.append(
            f'<path id="{eid}" d="{d}" fill="none" stroke="{color}" stroke-width="{sw}" '
            f'stroke-linecap="butt" stroke-linejoin="round"{extra} marker-end="{marker}"/>'
        )

    parts.append(text(56, 52, T["kicker"], fill=OCEAN, size=13, weight=700, extra=' id="kicker"'))
    parts.append(text(56, 90, T["title"], fill=INK, size=28, weight=700, extra=' id="hero-title"'))
    parts.append(text(56, 122, T["thesis1"], fill=SLATE, size=14, extra=' id="hero-thesis-1"'))
    parts.append(text(56, 144, T["thesis2"], fill=SLATE, size=14, extra=' id="hero-thesis-2"'))
    parts.append(
        text(1224, 52, T["finger"], fill=OCEAN, size=13, weight=700, anchor="end", extra=' id="fingerprint"')
    )
    parts.append(text(860, 456, T["boundary"], fill=SLATE, size=13, extra=' id="label-boundary"'))

    parts.append(
        text(store[0], 396, T["intent_bits"], fill=SLATE, size=12, extra=' id="note-intent-bits"')
    )

    parts.append(text(v[0], 266, T["s1"], fill=INK, size=15, weight=700, anchor="middle", extra=' id="label-validate"'))
    parts.append(text(v[0], 248, T["s1s"], fill=SLATE, size=12, anchor="middle", extra=' id="sub-validate"'))
    parts.append(text(a[0] - 28, 418, T["s2"], fill=INK, size=15, weight=700, anchor="end", extra=' id="label-analyze"'))
    parts.append(text(a[0] - 28, 400, T["s2s"], fill=SLATE, size=12, anchor="end", extra=' id="sub-analyze"'))
    parts.append(text(c[0], 266, T["s3"], fill=INK, size=15, weight=700, anchor="middle", extra=' id="label-compile"'))
    parts.append(text(c[0], 248, T["s3s"], fill=SLATE, size=12, anchor="middle", extra=' id="sub-compile"'))

    parts.append(text(p[0], 578, T["p"], fill=INK, size=14, weight=700, anchor="middle", extra=' id="label-prompt"'))
    parts.append(text(vw[0], 598, T["v"], fill=INK, size=14, weight=700, anchor="middle", extra=' id="label-view"'))
    parts.append(text(rnd[0], 578, T["r"], fill=INK, size=14, weight=700, anchor="middle", extra=' id="label-render"'))

    parts.append(text(236, 326, T["el_submit"], fill=SLATE, size=13, extra=' id="el-submit"'))
    parts.append(text(mid_va + 24, 356, T["el_ok"], fill=SLATE, size=13, extra=' id="el-ok"'))
    parts.append(text(mid_ac + 16, 356, T["el_diag"], fill=SLATE, size=13, extra=' id="el-diag"'))
    parts.append(text(mid_ci + 10, 318, T["el_waist"], fill=SLATE, size=13, extra=' id="el-waist"'))
    parts.append(text(p[0], 596, T["el_prompt"], fill=SLATE, size=12, anchor="middle", extra=' id="el-prompt"'))
    parts.append(text(vw[0], 616, T["el_view"], fill=SLATE, size=12, anchor="middle", extra=' id="el-view"'))
    parts.append(text(rnd[0], 596, T["el_render"], fill=SLATE, size=12, anchor="middle", extra=' id="el-render"'))
    parts.append(text(v[0] + 12, 470, T["el_fail"], fill=SLATE, size=13, extra=' id="el-fail"'))
    parts.append(text(a[0] + 12, 518, T["note_warn"], fill=SLATE, size=12, extra=' id="note-warn"'))

    parts.append(f'<circle id="mark-reject" cx="{v[0]:.1f}" cy="508.0" r="{rr}" fill="{CORAL}"/>')
    parts.append(text(v[0] + 12, 512, T["rej"], fill=INK, size=13, weight=700, extra=' id="label-reject"'))
    parts.append(text(v[0] + 12, 530, T["rej_s"], fill=SLATE, size=12, extra=' id="sub-reject"'))

    parts.append(text(56, 628, T["source"], fill=SLATE, size=12, extra=' id="source-note"'))

    parts.append(
        f'<rect id="store-intent" x="{store[0]:.1f}" y="{store[1]:.1f}" '
        f'width="{store[2]:.1f}" height="{store[3]:.1f}" rx="6" '
        f'fill="{PAPER}" stroke="{OCEAN}" stroke-width="1.4"/>'
    )
    parts.append(
        f'<ellipse id="term-ir" cx="{ir[0]:.1f}" cy="{ir[1]:.1f}" '
        f'rx="{irx:.1f}" ry="{iry:.1f}" fill="{GOLD}"/>'
    )
    parts.append(f'<circle id="stage-validate" cx="{v[0]:.1f}" cy="{v[1]:.1f}" r="{rs:.1f}" fill="{OCEAN}"/>')
    parts.append(f'<circle id="stage-analyze" cx="{a[0]:.1f}" cy="{a[1]:.1f}" r="{rs:.1f}" fill="{OCEAN}"/>')
    parts.append(f'<circle id="stage-compile" cx="{c[0]:.1f}" cy="{c[1]:.1f}" r="{rs:.1f}" fill="{OCEAN}"/>')
    parts.append(f'<circle id="term-prompt" cx="{p[0]:.1f}" cy="{p[1]:.1f}" r="{rc:.1f}" fill="{OCEAN}"/>')
    parts.append(f'<circle id="term-view" cx="{vw[0]:.1f}" cy="{vw[1]:.1f}" r="{rc:.1f}" fill="{OCEAN}"/>')
    parts.append(f'<circle id="term-render" cx="{rnd[0]:.1f}" cy="{rnd[1]:.1f}" r="{rc:.1f}" fill="{TEAL}"/>')
    parts.append(text(v[0], v[1] + 5, "01", fill=PAPER, size=12, weight=700, anchor="middle"))
    parts.append(text(a[0], a[1] + 5, "02", fill=PAPER, size=12, weight=700, anchor="middle"))
    parts.append(text(c[0], c[1] + 5, "03", fill=PAPER, size=12, weight=700, anchor="middle"))

    # Labels that sit on filled nodes must be painted after the fills.
    parts.append(
        text(ir[0], 346, T["ir"], fill=INK, size=18, weight=700, anchor="middle", extra=' id="label-ir"')
    )
    parts.append(
        text(ir[0], 368, T["ir_k"], fill=INK, size=13, weight=700, anchor="middle", extra=' id="sub-ir"')
    )
    parts.append(
        text(
            store[0] + store[2] / 2.0,
            store[1] + 28.0,
            T["intent"],
            fill=INK,
            size=15,
            weight=700,
            anchor="middle",
            extra=' id="label-intent"',
        )
    )
    parts.append(
        text(
            store[0] + store[2] / 2.0,
            store[1] + 48.0,
            T["intent_s"],
            fill=SLATE,
            size=12,
            anchor="middle",
            extra=' id="sub-intent"',
        )
    )

    parts.append("</svg>\n")
    svg = "\n".join(parts)
    sweep_code_detail(svg)
    return svg


def build_html(svg_href: str = "architecture.svg") -> str:
    return (
        "<!DOCTYPE html>\n"
        '<html lang="zh-CN">\n'
        "<head>\n"
        '<meta charset="utf-8">\n'
        '<meta name="viewport" content="width=device-width, initial-scale=1">\n'
        f"<title>{esc(T['html_title'])}</title>\n"
        "<style>\n"
        f"html,body{{margin:0;background:{PAPER};color:{INK};}}\n"
        "main{width:1280px;height:720px;margin:32px auto;}\n"
        "img{display:block;width:1280px;height:720px;border:0;}\n"
        "</style>\n"
        "</head>\n"
        "<body>\n"
        f'<main><img src="{svg_href}" width="1280" height="720" alt="{esc(T["html_alt"])}"></main>\n'
        "<script>window.__infographicReady=true;</script>\n"
        "</body>\n"
        "</html>\n"
    )


def build_readme() -> str:
    return (
        f"# {T['readme_h1']}\n\n"
        f"{T['readme_intro']}\n\n"
        f"## {T['readme_h_contract']}\n\n"
        f"- {T['readme_thesis']}\n"
        f"- {T['readme_q']}\n"
        f"- {T['readme_audience']}\n"
        f"- {T['readme_lang']}\n"
        f"- {T['readme_medium']}\n"
        f"- {T['readme_grammar']}\n"
        f"- {T['readme_hero']}\n"
        f"- {T['readme_source']}\n"
        f"- {T['readme_offpage']}\n\n"
        f"## {T['readme_h_rebuild']}\n\n"
        "```bash\n"
        "python3 docs/infographics/build.py\n"
        "rsvg-convert -w 1280 -h 720 docs/infographics/architecture.svg -o docs/infographics/proof.png\n"
        "```\n\n"
        f"{T['readme_kept']}\n"
    )


def build_contract() -> str:
    return (
        f"# {T['contract_h1']}\n\n"
        f"{T['contract_lede']}\n\n"
        f"## {T['contract_h_reader']}\n\n"
        f"{T['contract_reader']}\n\n"
        f"## {T['contract_h_thesis']}\n\n"
        f"{T['contract_thesis']}\n\n"
        f"## {T['contract_h_q']}\n\n"
        f"{T['contract_q']}\n\n"
        f"## {T['contract_h_lang']}\n\n"
        f"{T['contract_lang']}\n\n"
        f"## {T['contract_h_evidence']}\n\n"
        f"{T['contract_evidence']}\n\n"
        f"## {T['contract_h_order']}\n\n"
        f"{T['contract_order']}\n\n"
        f"## {T['contract_h_grammar']}\n\n"
        f"{T['contract_grammar']}\n\n"
        f"## {T['contract_h_box']}\n\n"
        f"{T['contract_box']}\n\n"
        f"## {T['contract_h_medium']}\n\n"
        f"{T['contract_medium']}\n\n"
        f"## {T['contract_h_palette']}\n\n"
        f"{T['contract_palette']}\n\n"
        f"## {T['contract_h_old']}\n\n"
        f"{T['contract_old']}\n"
    )


def build_verification() -> str:
    return (
        "# Verification\n\n"
        f"Thesis: {T['readme_thesis']}\n\n"
        "## Kept\n\n"
        "- docs/architecture-infographic.tex\n"
        "- docs/architecture.html\n"
        "- docs/infographics/conditioning-pipeline/ (sibling figure, untouched)\n\n"
        "## Changed (this remake only)\n\n"
        "- docs/infographics/architecture.svg\n"
        "- docs/infographics/index.html\n"
        "- docs/infographics/proof.png\n"
        "- docs/infographics/build.py\n"
        "- docs/infographics/README.md\n"
        "- docs/infographics/contract.md\n"
        "- docs/infographics/VERIFICATION.md\n"
        "- docs/infographics/data/copy.json\n"
        "- docs/infographics/data/provenance.json\n\n"
        "## Objective error gate\n\n"
        "svg-linter selectors: `svg/duplicate-id`, `svg/dangling-reference`.\n"
        "Result: filled after the render pass.\n\n"
        "## Collision review\n\n"
        "Selectors: line-overlap, line-crossing, line-text-overlap, text-text-overlap,\n"
        "canvas-edge-clearance, ambiguous-junction, connector-through-shape,\n"
        "edge-congestion, filter-clipping-risk, low-contrast-text.\n\n"
        "Disposition is filled after inspecting proof.png and the numbered lint PNG.\n\n"
        "## Not verified\n\n"
        "- No browser-harness (not requested). index.html was not opened in a browser.\n"
        "- Grayscale print was not inspected.\n"
        "- just build / just test / just install were not run.\n"
    )


def build_provenance() -> dict:
    return {
        "id": "cinematography-ir-class-b-architecture",
        "thesis": T["readme_thesis"],
        "scope": "Class B: author intent YAML/JSON to validate/analyze/compile; compiled IR consumed by prompt/view/render",
        "page_excludes": ["function names", "type names", "file names"],
        "anchors": [
            {
                "claim": "Authoring source is YAML/JSON cinematography intent",
                "file": "README.md",
                "line": "17-47",
            },
            {
                "claim": "CLI verbs include validate, analyze, compile, prompt, view, render blender",
                "file": "README.md",
                "line": "7",
            },
            {
                "claim": "validate catches structural and semantic errors; unknown fields fail closed",
                "file": "docs/13-validation.md",
                "line": "12-30",
            },
            {
                "claim": "analyze reports continuity warnings (axis, screen direction, eyeline, 30 deg); they are heuristics, not bans",
                "file": "README.md",
                "line": "188-195",
            },
            {
                "claim": "compile produces the shared compiled guidance IR waist",
                "file": "docs/11-ir-architecture.md",
                "line": "52-70",
            },
            {
                "claim": "prompt, view, and render consume the compiled result rather than re-interpreting author operations",
                "file": "docs/15-conditioning-pipeline.md",
                "line": "15-32",
            },
        ],
    }


def main() -> None:
    svg = build_svg()
    (OUT / "architecture.svg").write_text(svg, encoding="utf-8")
    (OUT / "index.html").write_text(build_html(), encoding="utf-8")
    (OUT / "README.md").write_text(build_readme(), encoding="utf-8")
    (OUT / "contract.md").write_text(build_contract(), encoding="utf-8")
    (OUT / "VERIFICATION.md").write_text(build_verification(), encoding="utf-8")
    data = OUT / "data"
    data.mkdir(exist_ok=True)
    (data / "provenance.json").write_text(
        json.dumps(build_provenance(), ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    print(f"wrote {OUT / 'architecture.svg'}")


if __name__ == "__main__":
    main()
