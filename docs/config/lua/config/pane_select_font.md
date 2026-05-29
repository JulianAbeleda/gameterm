---
tags:
  - appearance
  - pane_select
  - font
---
# `pane_select_font`

{{since('nightly')}}

Configures the font to use for pane selection mode. The `pane_select_font`
setting can specify a set of fallbacks and other options, and is described
in more detail in the [Fonts](../../fonts.md) section.

If not specified, the font is same as the font in `window_frame.font`

You will typically use [gameterm.font](../gameterm/font.md) or
[gameterm.font_with_fallback](../gameterm/font_with_fallback.md) to specify the
font.

To specify `pane_select_font`:

```lua
config.pane_select_font = gameterm.font 'Roboto'
```
