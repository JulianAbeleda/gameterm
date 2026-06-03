# GameTerm Scene Rounded Panel Renderer Scope

Status: SCOPED.

This document scopes the pass for making code-rendered VN panels look like
intentional rounded UI primitives rather than approximate stacked rectangles.

## Problem

Scene Mode now renders dialogue panels, Composer panels, and nameplates in Rust
by default. The current implementation is resolution-independent, but the corner
quality is only a first pass.

The current primitive approximates a rounded rectangle by splitting the corner
area into horizontal strips:

```text
rounded_panel_rects(rect, radius)
-> middle rectangle
-> N top corner strips
-> N bottom corner strips
```

This is why the corners can look less clean than expected. It is not caused by
PNG bit depth. The active default path is not a PNG path; it is procedural
geometry and alpha blending.

## Goal

Make the VN panels read closer to a modern design-tool rounded rectangle:

- smooth rounded corners at fullscreen and windowed sizes
- stable radius independent of terminal cell quirks
- crisp but subtle border
- separate style control for panels and nameplates
- no regressions to text layout, nameplate placement, or image aspect handling

The desired result is a reusable Scene UI primitive:

```text
RoundedPanelSurface
├── fill color
├── border color
├── border width
├── corner radius
├── anti-alias strategy
└── per-surface overrides
```

## Non-Goals

- Do not increase PNG bit depth as the fix.
- Do not return to stretched PNG UI panels as the default.
- Do not implement a full SVG/vector engine for this pass.
- Do not redesign Scene layout, Composer behavior, dialogue history, or asset
  scaling.

## Current Renderer

Current files:

- `gameterm-gui/src/termwindow/render/visual_quad.rs`

Current controls:

- `VN_PANEL_CORNER_SEGMENTS`
- `VN_PANEL_BORDER_WIDTH_PX`
- extracted fill and border colors
- radius derived from `cell_width * 2.2`
- optional PNG fallback through `GAMETERM_SCENE_VN_PANEL_TEXTURE=1`

Current weakness:

- the corner curve is made from coarse rectangular strips
- radius is tied to terminal cell width
- nameplates and panels share the same geometric renderer
- anti-aliasing depends on quad edges and alpha blending rather than an explicit
  edge falloff

## Proposed Implementation

### 1. Add A Style Radius

Extend `VnPanelStyle` with explicit radius controls:

```rust
struct VnPanelStyle {
    fill: LinearRgba,
    border: LinearRgba,
    border_width: f32,
    radius_px: f32,
}
```

Use separate constructors:

```rust
VnPanelStyle::dialogue_panel()
VnPanelStyle::composer_panel()
VnPanelStyle::dialogue_nameplate()
VnPanelStyle::composer_nameplate()
```

This lets the Composer dock, dialogue panel, and nameplates each tune their own
radius without changing layout.

### 2. Replace Strip Rects With A Better Primitive

Preferred first implementation:

```text
rounded_panel_mesh(rect, radius, segments)
```

Generate a triangle fan or triangle list for the rounded rectangle instead of
stacked strips. Keep it CPU-generated and feed it through the existing quad/mesh
render path if available.

If the current renderer only accepts rectangles, use a higher quality
intermediate step:

- increase corner subdivision dynamically based on radius
- allow sub-pixel strip heights
- avoid forced `max(1.0)` strip heights when the segment size should be smaller
- test segment count by pixel radius, not a fixed global constant

Preferred long-term implementation:

```text
signed-distance rounded rectangle shader
```

An SDF primitive would give the cleanest Figma-like edges:

- one rectangle draw
- analytic rounded corner
- explicit anti-alias falloff
- radius and border width controlled directly

This is likely a larger renderer pass, so the first pass can be a better mesh or
dynamic subdivision primitive.

### 3. Add Anti-Alias Intent

Add an explicit edge-softness value to the style:

```rust
edge_softness_px: f32
```

For the mesh/subdivision pass, this can be documented and reserved if the render
path cannot consume it yet.

For an SDF pass, this controls the transition around the rounded edge.

### 4. Keep PNG Compatibility As A Debug Fallback

Keep:

```sh
GAMETERM_SCENE_VN_PANEL_TEXTURE=1
```

Only use this to compare the old texture behavior against the new procedural
primitive. The default remains code-rendered.

## Tests

Unit tests:

- `vn_panel_styles_expose_independent_radius_values`
- `rounded_panel_geometry_clamps_radius_to_rect_size`
- `rounded_panel_geometry_increases_detail_for_larger_radius`
- `rounded_panel_geometry_preserves_rect_bounds`
- `vn_panel_rects_still_include_dialogue_composer_and_nameplates`

Visual smoke:

- fullscreen Scene Mode capture
- windowed Scene Mode capture
- VN layout debugger capture
- Composer prompt capture with dialogue text and Composer text visible

Manual inspection criteria:

- corners are visibly round, not stair-stepped
- border follows the rounded corner cleanly
- fill does not bleed outside the border
- nameplates remain separated from panels
- text remains inside the dialogue and Composer boxes
- resizing from fullscreen to windowed does not distort radius or aspect

## Definition Of Done

- Dialogue panel, Composer dock, dialogue nameplate, and Composer nameplate have
  independent radius/style constructors.
- Rounded panel rendering no longer depends on a coarse fixed eight-strip corner
  approximation.
- Fullscreen and windowed smoke captures show cleaner corners.
- Existing Scene text/layout tests pass.
- Existing aspect-safe image behavior remains unchanged.
- The docs explain that bit depth is not the cause of rounded-corner quality.

## Suggested Commit Plan

1. `[visual] add independent Scene panel radius styles`
2. `[visual] improve Scene rounded panel geometry`
3. `[test] cover Scene rounded panel geometry`
4. `[docs] record Scene rounded panel renderer pass`

