# Verification

Thesis: 论点：作者意图以 YAML / JSON 进入，经 validate、analyze、compile 收成一份编译 IR；prompt、view、render 只消费这份产物。

## Kept

- docs/architecture-infographic.tex
- docs/architecture.html
- docs/infographics/conditioning-pipeline/ (sibling figure, untouched)
- no sibling docs/architecture-infographic.svg exists

## Changed (this remake only)

- docs/infographics/architecture.svg
- docs/infographics/index.html
- docs/infographics/proof.png
- docs/infographics/build.py
- docs/infographics/README.md
- docs/infographics/contract.md
- docs/infographics/VERIFICATION.md
- docs/infographics/data/copy.json
- docs/infographics/data/provenance.json

## Render

- rasterizer: rsvg-convert 2.62.3
- command: `rsvg-convert -w 1280 -h 720 docs/infographics/architecture.svg -o docs/infographics/proof.png`
- SVG: 1280×720, SHA-256 `c54481075795a75847c431b43b712bb37c6fa6bb27fcb44569f87dfbfa6ddd4b`
- PNG proof: 1280×720 RGB

## Objective error gate

- tool: svg-linter 0.1.0
- catalog_hash: sha256:fdfd45cabf9908c69853058ca51fba80878a99d21941eebf956258a904e55bb9
- selectors: `svg/duplicate-id`, `svg/dangling-reference`
- `--require-complete --fail-on error`
- result: exit 0, 0 findings, 2 rules fully evaluated

## Hygiene review

- selectors: excessive-path-complexity, unused-definition, external-resource, raster-upscale, open-filled-path
- result: 0 findings

## Collision review

- selectors: line-overlap, line-crossing, line-text-overlap, text-text-overlap, canvas-edge-clearance, ambiguous-junction, connector-through-shape, edge-congestion, filter-clipping-risk, low-contrast-text
- first pass: 5 candidates (2 line-text-overlap, 3 line-crossing)
- disposition: all five were real layout defects (提交意图 on the Fog spine; 审片图 on the render drop; consumption rule crossing the three fan-out edges). Fixed in build.py by moving labels beside/under their strokes and shortening the dashed rule so it no longer crosses connectors.
- second pass on the published SVG: 0 findings
- unannotated proof.png and numbered lint PNG inspected at 1280×720; title, intent→validate, analyze, IR fan-out, reject, and footer crops opened

## Code-detail sweep

- delivered SVG/HTML have no `.rs`, type names, or function names
- CSS `width:1280` style hits are not file:line coordinates

## Quarantine

- native SVG text (not path-font); path-font quarantine not applicable
- no CSS custom properties; `var()` quarantine not applicable

## Not verified

- No browser-harness (not requested). index.html was not opened in a browser.
- Grayscale print was not inspected.
- just build / just test / just install were not run.
- pm doctor was not run.
