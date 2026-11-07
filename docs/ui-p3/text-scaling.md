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
- Freya's built-in `Input`, `Select` and `MenuItem` inherit `font::body()`
  from the root, and the input layout theme uses scaled padding and radius.
  Page and toolbar actions share one `ActionButton` with filled, outline and
  flat variants plus a real disabled state, so the app no longer mixes the
  16 px framework `Button` with caption-sized custom buttons.
- Above 100 % the sidebar sort buttons collapse to one row. Pressing the
  control cycles field and direction, keeping the save list height instead of
  wrapping three buttons into several rows.
- The status slot is `3 * font::line_height()` at full width and
  `2 * font::line_height()` in the compact shell, so it grows with the text
  while the compact header keeps enough room for the editor. `max_lines` keeps
  the status text inside its slot.
- Save rows are `layout::save_row_height() * scale`; the row pitch helper is a
  function so the virtual list's item size follows the scale.
- The sidebar hides its decorative 120 px screenshot and the duplicate
  selected-save block at any scale above 100 %. The shell also switches to a
  compact header at the minimum width or above 100 % text: the tagline and
  badges give way, the status slot uses two lines, the tab strip drops its
  reserved busy slot, and the save browser keeps a usable list. At 200 % on
  900x700 the list still shows rows and the editor shows its first card.
- The character form reduces its column count by available width: three or four
  fields at 100 %, two or three at larger scales, and one column when even two
  minimum-width fields cannot fit. Style, item and gem rows stack their actions
  below the name at 200 % instead of squeezing the name into a letter column.
- Picker and carried-gem viewports cap themselves at a fraction of the window
  height, so seven scaled rows can no longer grow taller than a short window.

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
  is 92, the base status slot is 84 (the compact shell uses two lines, 56) and
  the form field minimum is 264. Window, preview, portrait, sidebar and
  scrollbar tokens stay at their base values.
- Unit: the type scale is strictly ordered and the smallest type is still 11 px
  at 100 %.
- Headless UI matrix: 900x700, 1180x760 and 1280x800 at 100, 125, 150 and
  200 percent. Every configuration must open a save, keep at least one save
  row, leave a real editor viewport, render all six tabs, keep every text input
  inside the window and keep the footer inside the window. A German variant
  covers 125, 150 and 200 percent at 900x700. The About page test switches to
  200 % through the real control.
- Headless control tests assert a built-in folder input and the shared
  header/footer actions grow together with the scale, and that the one-row
  compact sort control updates the field and direction.
- Screenshot: `--scale 200 --width 900 --height 700` and the larger sizes render
  the reflowed shell for visual review.

## Accessibility, contrast and reduced motion

- 200 % text without loss of content is WCAG 1.4.4 (AA); the test covers all
  tabs and the sidebar.
- Controls grow with the type (28 px targets at 100 %, 56 px at 200 %), so the
  target size requirement improves rather than degrades.
- Scaling is instant and adds no motion.

## Risk

- At 200 % the default 1180x760 window switches to the compact shell and a
  single-column form at the narrower end of the range. The layout reflows and
  scrolls; a taller or wider window shows more at once. That is normal zoom
  behaviour, and the minimum supported logical window stays 900x700 so the
  window manager can still place it on a 1366x768 screen.
- The sidebar preview and duplicate selected-save block are hidden above 100 %.
  That is a deliberate responsive trade-off and is called out in the
  text-scaling tests.
- At 200 % on 900x700 the form uses one column and the style and item rows stack
  their controls. More vertical scrolling is expected; no content is clipped.
