# Light and system-following theme

## Current state (before P3)

The palette in `crates/jadeforge-gui/src/theme.rs` was a set of `const Color`
values and the Freya theme always started from `dark_theme()`. Views referred to
`theme::TEXT`, `theme::PANEL` and friends directly, so there was no way to
select a second palette at runtime. The palette tests asserted that every
surface was dark, which would have rejected a light theme by construction.

## Approach

- One `Palette` value per theme (`DARK`, `LIGHT`, `HIGH_CONTRAST`), with the same
  semantic fields. The old constants moved into `Palette::DARK` unchanged.
- A thread-local `UiStyle` carries the active palette and text scale. The root
  component installs it before building the tree; every view reads colours
  through functions such as `theme::text()` and `theme::surface()`.
- `ThemeChoice` lives in `jadeforge-session::preferences` so the settings file
  can persist it without the session crate depending on Freya. The default stays
  `Dark` to preserve the recognised jade-on-charcoal identity; `System` resolves
  the OS preference reported by Freya, and `Light` is explicit.
- The root subscribes to `Platform::get().current_window().preferred_theme`, so
  changing the OS theme repaints a `System` choice while the window is open.
- The About page gets theme radio buttons. Changing one updates the reactive
  state immediately and writes `theme = "..."` to the TOML settings file.
- The Freya component themes (`button`, `filled_button`, `outline_button`,
  `flat_button`, `input`, `flat_input`, `scrollbar`, `tooltip`) are rebuilt from
  the active palette; the theme name (`jadeforge-dark`, `jadeforge-light`) makes
  `set_if_modified` detect the change.

Upstream references:

- Freya 0.5 theming hook and `Theme` state:
  <https://docs.rs/freya/0.5.0/freya/prelude/fn.use_init_theme.html>
- `PlatformWindow.preferred_theme`:
  <https://docs.rs/freya-core/0.5.0/freya_core/platform/struct.PlatformWindow.html>
- winit theme source (light/dark only):
  <https://docs.rs/winit/0.30/winit/window/enum.Theme.html>
- WCAG 2.2 contrast minimum:
  <https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html>

## Light palette

Warm parchment (`#f6f4ee`) with near-black text (`#1b201d`), white cards and a
dark jade accent (`#0a5c3d`). The accent is dark enough to be used as text on
every light surface (4.5:1) and still passes 4.5:1 for white text when it fills a
button. The gold upgrade orbs use a darker gold (`#8c5c0f`) so the ring stays
above the 3:1 non-text minimum on the active surface.

## Test plan

- Palette unit tests run for all three palettes: text/muted/dim/status colours
  at 4.5:1 on every surface, accent text at 4.5:1, control borders at 3:1,
  focus ring at 3:1, orbs not following the accent.
- Dark tests keep their original assertions (dark surfaces, bright text, jade
  accent) and the light test asserts the inverse (light surfaces, dark text).
- `ThemeChoice` round trip, system resolution and settings persistence are unit
  tested in `jadeforge-session`.
- Headless UI tests write `theme = "light"` and `theme = "system"` into the
  isolated settings file, launch the real root, and assert the installed
  palette. The About test clicks the theme chips and re-reads the settings file.
- Screenshots: `docs/screenshot*.png` stay on the dark default; the light and
  high-contrast variants are rendered with `--theme` into the ignored
  `scratch/` folder (the tracked hero must show the project's identity).

## Accessibility, contrast and reduced motion

- Every palette is checked for WCAG 2.2 AA text contrast and 3:1 non-text
  contrast; the high-contrast note raises the target to AAA.
- The theme radio buttons are real AccessKit `RadioButton` nodes with a selected
  state and a spoken description; no colour-only signal.
- Switching themes is instant. No animation or transition is added, so there is
  nothing to suppress for `prefers-reduced-motion`.

## Risk

- Thread-local style assumes Freya renders a component tree on one thread. The
  pinned engine does; if a future Freya measures text on a worker, the style
  must move into a render context. The headless suite would catch a wrong
  palette, not a racy one.
- The light palette is new visual design; the tracked screenshots stay dark, so
  a reviewer must render the light variant to judge it.
