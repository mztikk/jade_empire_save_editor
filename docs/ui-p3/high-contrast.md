# High contrast and the platform forced-colours setting

## Current state

Freya 0.5 exposes exactly one accessibility-relevant platform preference:
`PlatformWindow.preferred_theme`, a `PreferredTheme::{Light, Dark}` value fed by
winit's `WindowEvent::ThemeChanged`. There is no forced-colours, high-contrast or
`prefers-contrast` signal in Freya's platform layer or in winit 0.30. The
application therefore cannot detect the Windows High Contrast /
`SystemParametersInfo(SPI_GETHIGHCONTRAST)` setting or the GNOME/KDE
high-contrast theme automatically. Reading it on Windows would require `unsafe`
Win32 calls, which the workspace forbids.

Upstream evidence:

- `PreferredTheme` has two variants only:
  <https://docs.rs/freya-core/0.5.0/freya_core/platform/enum.PreferredTheme.html>
- winit's window theme is also `Light`/`Dark` only:
  <https://docs.rs/winit/0.30/winit/window/enum.Theme.html>
- Freya tracks theming in the platform layer:
  <https://github.com/marc2332/freya/blob/main/crates/freya-core/src/platform.rs>
- WCAG 2.2 high contrast is described by AAA 7:1 and non-text 4.5:1:
  <https://www.w3.org/WAI/WCAG22/Understanding/contrast-enhanced.html>
- Freya links `mundy`, which can read the platform contrast preference (the
  Linux portal reports `org.freedesktop.appearance contrast`, Windows high
  contrast is exposed too):
  <https://docs.rs/mundy/0.2.3/mundy/enum.Contrast.html>. That is the likely
  route to automatic high contrast; it is not wired into the `System` choice
  yet, so the theme remains a manual selection. Tracked in #33.

## Approach

A complete `Palette::HIGH_CONTRAST` is selected explicitly (About page) or by a
future platform signal. It uses:

- black background and near-black surfaces,
- pure white text, muted and dim text above 7:1 on every surface,
- a bright green accent and a yellow-gold orb,
- white control borders and dividers (4.5:1 or better),
- the focus ring uses the accent, which is above 7:1 on the background.

The palette test suite runs the same contrast contract for all palettes and an
additional AAA test for this one. The automatic detection gap is recorded in the
[progress table](README.md) as blocked upstream, not silently dropped.

## Test plan

- Unit: high-contrast surfaces are near-black, text/muted/dim/status colours
  meet 7:1 on every surface, primary text reaches at least 20:1, accents and
  borders meet 7:1/4.5:1.
- Headless UI: launch with `theme = "high-contrast"`, assert the installed
  palette is black/white and that the shell and a tab still render.
- The About page test switches to high contrast through the real radio button
  and re-reads the persisted choice.
- Manual check of a real forced-colours session is impossible without the
  upstream API; the note records that limitation.

## Accessibility, contrast and reduced motion

- Targets WCAG 2.2 AAA for text, exceeding the AA requirement.
- No information is carried by colour alone: badges keep their text, the focus
  ring is a border, and the unsaved badge keeps its label.
- No animation is added; switching palettes is instant.

## Risk

- The palette is verified by contrast maths, not by a real Windows High Contrast
  or GNOME high-contrast session. A future Freya/winit release that exposes the
  platform setting should map it to this palette automatically; until then users
  select it manually.
