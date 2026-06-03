# GameTerm Scene Panel Codegen Scope

Status: FIRST PASS IMPLEMENTED; PROCEDURAL RENDERING ENABLED.

This document records the first pass for converting VN panel PNGs into compact
procedural panel style data.

## Goal

Move Scene Mode dialogue boxes, Composer boxes, and nameplates toward
resolution-independent code-rendered primitives while preserving the current
visual direction.

The target is not a generic asset vectorizer. Panels are simple UI surfaces, so
they should become a small procedural style:

```text
fill color
border color
alpha
corner radius
border width
slice hint
```

The Rust renderer can then redraw those surfaces at any size instead of relying
on stretched PNG pixels.

## Reference Tools

Open-source raster-to-vector tools are useful references and optional helpers:

- VTracer: Rust-based raster-to-vector tracing for color images.
- Potrace: classic bitmap tracing, strongest for black/white silhouettes.
- AutoTrace: bitmap-to-vector tracing with outline/centerline support.

GameTerm's first pass does not vendor these projects. The helper discovers
local `vtracer`, `potrace`, or `autotrace` binaries and can invoke one for
exploratory SVG output.

## Helper

```sh
cargo run -q -p gameterm-visual --example scene_panel_style -- \
  assets/gameterm-scene/vn-panel.png \
  --pretty
```

Optional SVG trace if a tracing tool is installed:

```sh
cargo run -q -p gameterm-visual --example scene_panel_style -- \
  assets/gameterm-scene/vn-panel.png \
  --trace-svg /tmp/gameterm-vn-panel.svg \
  --output /tmp/gameterm-vn-panel-style.json \
  --pretty
```

The helper currently supports non-interlaced 8-bit RGB and RGBA PNGs. It emits
JSON with:

- source dimensions
- recommended renderer
- fill and border color samples
- alpha samples
- estimated corner radius
- detected local tracing tools

## Why Not Huge Generated Code

A raw PNG-to-code conversion would only move pixels into a source file:

```text
pixel[0] = ...
pixel[1] = ...
```

That would not fix scaling quality. A full traced SVG can also produce a large
path list that is hard to tune.

For panels, the useful conversion is:

```text
PNG panel
-> compact procedural style
-> Rust rounded rectangle renderer
```

## Next Step

Completed first:

- checked-in constants for the current VN panels
- procedural rendering for dialogue panels, Composer panels, and nameplates by
  default
- optional PNG nine-slice compatibility through
  `GAMETERM_SCENE_VN_PANEL_TEXTURE=1`

Remaining follow-up:

- user config file for panel styling
- live debugger controls for generated panel colors and border width
- remove the PNG compatibility path only after the procedural renderer is fully
  dogfooded
