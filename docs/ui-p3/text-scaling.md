# Text scaling and zoom up to 200 %

## Current state

The type scale, spacing scale, radii and fixed row heights were `const f32`
tokens. Views used them directly, so changing the user's text size would have
required a second layout. Fixed-height rows (save rows, the status slot, badges,
steppers) were sized for 11-14 px type and would clip at a larger size.

## Approach

- The tokens became functions that multiply their base value by
  `theme::text_scale()`. This covers `font`, `space`, `radius`, `stroke` and the
  type-related `layout` dimensions.
- The scale is one of `100`, `125`, `150` or `200` percent, persisted as
  `text_scale = 150` in the TOML settings file. Values outside the tested range
  fall back to 100.
- Window dimensions, the sidebar percentage, the scrollbar width and the two
  image slots (preview and portrait) deliberately do not scale: they describe
  the monitor and the sidebar, not the type. Scaling them would push the
  minimum window off a 1366x768 screen at 200 % without improving legibility.
- The status slot is `3 * font::line_height()` and therefore grows with the
  text; `max_lines(3)` keeps it three lines.
- Save rows are `layout::save_row_height() * scale`; the row pitch helper is a
  function so the virtual list's item size follows the scale.
- The sidebar hides its decorative 120 px screenshot once the scale is above
  125 %. Without that, the scaled header, filter and sort controls left the
  save list with no height on a 900 px window at 200 %. The character name and
  level still carry the same information.

Freya also supports a global custom scale factor (`WindowConfig::
with_custom_scale_factor`), which is a DPI-style zoom of the whole logical
surface. JadeForge does not use it for text scaling: it would zoom the fixed
minimum window too, and it cannot be combined with per-token reflow. The config
and the About control implement a true type scale instead.

- `with_custom_scale_factor` (read for the DPI note, not used):
  <https://docs.rs/freya-winit/0.5.0/freya_winit/config/struct.WindowConfig.html#method.with_custom_scale_factor>
- WCAG 2.2 resize text:
  <https://www.w3.org/WAI/WCAG22/Understanding/resize-text.html>

## Test plan

- Unit: at 200 %, `font::body() == 28`, spacing and radii double, save row height
  is 92, the status slot is 84 and the form field minimum is 264. Window,
  preview, portrait, sidebar and scrollbar tokens stay at their base values.
- Unit: the type scale is strictly ordered and the smallest type is still 11 px
  at 100 %.
- Headless UI: launch at `text_scale = 200` in a 900 px-high window, assert the
  installed scale and that the save row is still built with a text height above
  the unscaled line, switch through all six tabs, and assert the footer is still
  inside the window. The About page test switches to 200 % through the real
  control.
- Screenshot: `--scale 200 --width 1600 --height 900` renders every tab for
  visual review.

## Accessibility, contrast and reduced motion

- 200 % text without loss of content is WCAG 1.4.4 (AA); the test covers all
  tabs and the sidebar.
- Controls grow with the type (28 px targets at 100 %, 56 px at 200 %), so the
  target size requirement improves rather than degrades.
- Scaling is instant and adds no motion.

## Risk

- At 200 % the default 1180x760 window is cramped. The layout reflows and
  scrolls, but a taller window is needed to see as much as at 100 %. That is
  normal zoom behaviour and is documented; the minimum supported logical window
  stays 900x700 so the window manager can still place it on a 1366x768 screen.
- The sidebar preview is hidden above 125 %; that is a deliberate responsive
  trade-off and is called out in the text-scaling test.
