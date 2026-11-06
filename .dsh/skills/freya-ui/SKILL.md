---
name: freya-ui
description: Use when building or changing the JadeForge desktop UI with Freya 0.5, when a view renders incorrectly or looks off-theme, or when the headless UI tests fail.
---

# Building the JadeForge UI with Freya 0.5

General framework guidance lives in the `freya` skill. This skill covers the
JadeForge-specific structure, layout, theming and test loop.

## Structure that must not be broken

- `crates/jadeforge-session/src/session.rs` owns every behaviour and has **no
  Freya types**. It is unit tested and re-exported by the GUI crate.
- `app.rs` is the shell (header, save list, tabs, action bar); `views/` holds one
  module per tab; `widgets.rs` holds cards/fields/badges; `theme.rs` holds the
  palette, type scale, spacing scale, radii, strokes and fixed layout tokens.
- `Session` lives in an `Rc<RefCell<..>>` created with `use_hook`. Callbacks
  mutate it through `Ctx::run` and bump a `use_state(u32)` revision.
- The root component **must read** the revision (`let _ = *revision.read();`).
  Without the read Freya does not subscribe and the view never re-renders after
  a click.

## Theming

Never rely on Freya's default (light) component colours. `theme::theme()` starts
from `dark_theme()` and then:

1. replaces `theme.colors` with `theme::palette()`,
2. overrides the `button`, `filled_button`, `outline_button`, `flat_button`,
   `input`, `flat_input`, `scrollbar` and `tooltip` component themes.

Freya 0.5's scrollbar theme has separate resting and expanded widths, opacities
and cross gaps instead of the old single `size`. JadeForge keeps the 0.4 look
on purpose: `thumb_cross_size` and `expanded_thumb_cross_size` are both the
`theme::layout::SCROLLBAR` token, both opacities are 255, and both scroll views
set `.auto_hide_scrollbars(false)`, so the bar stays the same 9 px and always
visible. If a view starts hiding its scrollbar or expanding it on hover, check
those two settings before adding per-view overrides.

The palette is unit tested: every surface must stay dark (`luminance < 0.25`),
the accent must be jade green (green dominant, with more blue than red) and meet
WCAG AA on every surface, and component surfaces must never be white. If a
component looks wrong, the fix belongs in `theme.rs`, not in a one-off
`.background()` on a widget.

The style upgrade orbs are the one deliberate gold exception: they mimic the
game's level-up screen and use `theme::ORB`, so do not repaint them with the
accent.

Font sizes and numeric layout values come from named tokens, never literals:
`theme::font::BODY`, `theme::space::MD`, `theme::radius::CARD`,
`theme::stroke::FOCUS`, and the fixed dimensions in `theme::layout`. Add a token
when the scale lacks a value instead of putting a magic number in a view.

## Layout rules learned the hard way

- `Size::flex(1.0)` only works inside a parent with `.content(Content::Flex)`.
  A flex spacer in a normal row pushes siblings off screen and can explode the
  row height (this produced a 180 px header gap once).
- `Size::fill()` is 100 % of the parent, not "the rest". For the main area use
  `Content::Flex` + `Size::flex(1.0)`.
- Rows that must stay one line high need an explicit height; otherwise a fill
  child grows them.
- In a horizontal action row, the expanding name/content block must use
  `Size::flex(1.0)`, never `Size::fill()`. A fill child consumes the controls'
  space and can make the row grow. `assert_compact_action_rows` in
  `tests/ui.rs` checks both the vertical pitch and the right edge of action
  labels; extend it when adding a new list action.
- The sidebar save preview is deliberately a fixed
  `theme::layout::PREVIEW` square, aspect-preserving
  image. Do not make it fill the responsive sidebar: that is what caused the
  zoom/crop regression. The UI test asserts both dimensions, and the
  `theme::layout::MIN_WINDOW_WIDTH` by `theme::layout::MIN_WINDOW_HEIGHT`
  minimum-window test asserts that at least four save rows remain visible.
- Focus rings must reserve their space even when unfocused; otherwise keyboard
  navigation moves tabs, sort controls and rows. `AccessibleButton` always
  carries a transparent `theme::stroke::FOCUS` border and only recolours it on
  focus.
- Every dynamic list item, conditional tab root and preview subtree needs a
  stable key. Freya 0.5 fixed several diff panics, including the unkeyed
  replacement crash 0.4 hit, but keys are still what reconcile reordered rows
  without duplicating or dropping state. `AccessibleButton` and
  `ExpandingName` implement `KeyExt`; tab content and preview roots are keyed.
  Do not remove these keys.
- Pointer cursors are per element in 0.5: use `.cursor(CursorIcon::Pointer)` on
  the hovered element instead of the removed `Cursor::set` calls. The headless
  suite asserts the tab cursor, so a dropped `.cursor(...)` fails a test.
- Accessibility comes from Freya 0.5: labels and paragraphs fill their AccessKit
  value while the tree is built, and `on_press` registers and handles the
  AccessKit `Click` action. Do not add manual `set_value` builders to plain
  labels; the headless suite reads the effective node and activates a tab
  through `send_accessibility_action`.
- No layout shifts: anything whose size changes with its content gets a fixed
  slot instead of natural sizing. The save preview and its placeholder share one
  height, status messages reserve their maximum lines, selection bars and arrow
  columns keep their width when invisible, and the active tab keeps a constant
  font weight. Cover it with a headless test that compares the position of an
  element below the changing content.
- Form fields wrap with `.content(Content::wrap_spacing(theme::space::FIELD))`.
- Scroll views need a constrained parent (`Size::flex(1.0)` in a flex column).
- Responsive sizes come from the layout, not from measuring the window: the
  sidebar is `Size::percent(theme::layout::SIDEBAR_PERCENT)` with
  `theme::layout::SIDEBAR_MIN_WIDTH`/`SIDEBAR_MAX_WIDTH`, and rows that
  must wrap use `Content::wrap_spacing`. Labels that should never break are
  `max_lines(1)`: without it a squeezed tab renders `Comp\nanions`.
- The folder dialog is the native one (`rfd`) in `dialog.rs`; the headless
  tests cannot open it, so they queue an answer with
  `dialog::set_next_pick_for_tests`. Do not put the dialog on a background
  thread: a modal native dialog blocking the window is expected.
- Buttons are hover-coloured by the theme; a row whose child paints its own
  background will not show `flat_button` hover. Do not paint over it if hover
  feedback matters.

## Feedback loop

```sh
cargo check -p jadeforge-gui
cargo check --workspace --all-targets       # includes the Linux live-suite support code
cargo test -p jadeforge-gui                 # headless UI tests
cargo run -p jadeforge-gui --example snapshot -- "<SAVE_DIR>" /tmp/ui.png all
cargo run -p jadeforge-gui --example snapshot -- "<SAVE_DIR>" /tmp/inv.png Inventory --height 1900 --query "Drachen"
cargo run -p jadeforge-gui --example snapshot -- "<SAVE_DIR>" /tmp/small.png Overview --width 900 --height 600
cargo run -p jadeforge-gui --example snapshot -- "<SAVE_DIR>" /tmp/wide.png all --width 1600 --height 900
JADEFORGE_LIVE_APPROVED=1 just live    # ask first for a window-path change; see the Windowed tests rule
```

Check narrow windows too: rows with `spacing` overflow silently at 900 px unless
their container wraps (`Content::wrap_spacing`).

Look at the PNG. Squeezed inputs, clipped rows and off-screen buttons are
invisible to the compiler.

## Windowed tests

**Never run `JADEFORGE_LIVE_APPROVED=1 just live` without asking the user for explicit permission first.** The approval variable is a second gate against accidental invocation, not permission; plain `just live` refuses.
It opens real windows, takes focus, moves the pointer and types on their
desktop. A general request to run the tests is not permission. Only run it when
a headless GUI test genuinely cannot reproduce or verify the problem (window
creation, compositor input, focus, presentation, HiDPI or window-manager
interaction), or when the bug is in the actual windowed GUI or a mismatch
between the headless and real window. Otherwise use the headless suite and
snapshots.

This is a risk-based escalation, not a routine step. Ask for a major Freya or
winit upgrade, a window configuration/close/focus/input/presentation change, a
live-probe change, or a bug that only reproduces in a real window. Do not ask
for view, layout, text, theme, form or session changes that the headless suite
and snapshots cover. If the change is borderline, ask. Before `just finish`,
state whether the suite was run; compiling it is not running it.

Once permission is given: `JADEFORGE_LIVE_APPROVED=1 just live` launches the real application through
`freya-winit` and drives a real Wayland window. On KDE it injects pointer and
key events through KWin's fake-input protocol and captures the window with
Spectacle; elsewhere it posts the same events through the windowed plugin
handle. It is deliberately small; use it for window creation, input, focus and
presentation changes, not for layout assertions. See `docs/LIVE-UI.md` and
`AGENTS.md` rule 13.

## Headless tests

`freya-testing` renders the real tree. Useful patterns:

- find text: `test.find(|_, el| Label::try_downcast(el).filter(|l| l.text.contains(..)).map(|_| ()))`
- inputs are **paragraphs**, not labels: use `Paragraph::try_downcast` and
  `to_string()` to find a search field, click it, then `test.write_text("...")`.
- content below the fold cannot be clicked; launch the runner with a tall window
  for those tests (`(1280.0_f32, 1800.0_f32).into()`).
- clicks: take `node.layout().area`, use its centre with `click_cursor`.
- accessibility: read the effective node with
  `element.finish_accessibility(&mut builder)` before asserting `value()` or
  `label()`; labels fill their value there in 0.5. Screen-reader activation is
  `NodeId` plus `send_accessibility_action(ActionRequest { action: Click, .. })`.
- cursor feedback: `test.move_cursor(...)` then `test.cursor_icon()`; the tab
  test pins `CursorIcon::Pointer` for `AccessibleButton`.

## Before you finish

```sh
cargo test -p jadeforge-gui
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

Regenerate `docs/screenshot*.png` if the layout changed visibly.
