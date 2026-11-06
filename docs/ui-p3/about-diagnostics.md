# About/help surface: version, licenses, config and data-folders

## Current state

The P2 About page showed the version, settings file, remembered save folder,
sort order, backup rules and keyboard shortcuts. A support request still needed
several separate places to collect the game-data state, the savegame identity
and the active theme/language.

## Approach

- A new **DIAGNOSTICS** section lists the version, the open savegame path, the
  block count and file size, the active theme and text scale, the language, the
  renderer override and the backup folder. Missing values are stated explicitly
  ("none", "not read") instead of being left blank.
- **Copy diagnostics** writes the same facts to the system clipboard with the
  existing clipboard seam and reports success in the status slot, so a user can
  paste them into a bug report.
- A **LICENSES** section states the MIT license and that the bundled English
  item/style names are game content covered by NOTICE and docs/NAMES.md. The
  full license files ship next to the binary in the distribution folder.
- The keyboard shortcut list now includes the P3 additions (Ctrl+Z, Ctrl+Y,
  Ctrl+/, review-before-save). Ctrl+/ opens this page from anywhere.

## Test plan

- Headless UI: About contains DIAGNOSTICS, Version, LICENSES, the MIT note and
  Ctrl+Z in the shortcut sheet; clicking Copy diagnostics reaches the clipboard
  seam; the window-remembering toggle persists both directions.
- Ctrl+/ opens the About page and its shortcut sheet.
- The existing About test for version, config and backup rules stays green.
- The page is rendered at 900, 1280 and 1600 px and at 200 % text by the
  snapshot and scaling suites.

## Accessibility, contrast and reduced motion

- Sections keep the card heading/section-label hierarchy and normal AA text
  colours; the config path wraps and has copy/open actions.
- Copy diagnostics is an ordinary focusable outlined button.
- No animation.

## Risk

- The diagnostics intentionally show machine paths. They are user data, never
  telemetry, and go only to the clipboard when the user presses the button.
- Renderer override is read from the environment; the driver actually selected
  is printed to the terminal, not shown here.
