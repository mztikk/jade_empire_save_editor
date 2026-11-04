---
name: freya-ui
description: Use when building or changing the JadeForge desktop UI with Freya 0.4, when a view renders incorrectly or looks off-theme, or when the headless UI tests fail.
---

# Building the JadeForge UI with Freya 0.4

## Structure that must not be broken

- `crates/jadeforge-gui/src/session.rs` owns every behaviour and has **no Freya
  types**. It is unit tested.
- `app.rs` is the shell (header, save list, tabs, action bar); `views/` holds one
  module per tab; `widgets.rs` holds cards/fields/badges; `theme.rs` holds the
  palette.
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

The palette is unit tested: every surface must stay dark (`luminance < 0.25`),
the accent must be warm, and component surfaces must never be white. If a
component looks wrong, the fix belongs in `theme.rs`, not in a one-off
`.background()` on a widget.

## Layout rules learned the hard way

- `Size::flex(1.0)` only works inside a parent with `.content(Content::Flex)`.
  A flex spacer in a normal row pushes siblings off screen and can explode the
  row height (this produced a 180 px header gap once).
- `Size::fill()` is 100 % of the parent, not "the rest". For the main area use
  `Content::Flex` + `Size::flex(1.0)`.
- Rows that must stay one line high need an explicit height; otherwise a fill
  child grows them.
- Form fields wrap with `.content(Content::wrap_spacing(14.0))`.
- Scroll views need a constrained parent (`Size::flex(1.0)` in a flex column).

## Feedback loop

```sh
cargo check -p jadeforge-gui
cargo test -p jadeforge-gui                 # headless UI tests
cargo run -p jadeforge-gui --example snapshot -- "<SAVE_DIR>" /tmp/ui.png all
cargo run -p jadeforge-gui --example snapshot -- "<SAVE_DIR>" /tmp/inv.png Inventory --height 1900 --query "Drachen"
```

Look at the PNG. Squeezed inputs, clipped rows and off-screen buttons are
invisible to the compiler.

## Headless tests

`freya-testing` renders the real tree. Useful patterns:

- find text: `test.find(|_, el| Label::try_downcast(el).filter(|l| l.text.contains(..)).map(|_| ()))`
- inputs are **paragraphs**, not labels: use `Paragraph::try_downcast` and
  `to_string()` to find a search field, click it, then `test.write_text("...")`.
- content below the fold cannot be clicked; launch the runner with a tall window
  for those tests (`(1280.0_f32, 1800.0_f32).into()`).
- clicks: take `node.layout().area`, use its centre with `click_cursor`.

## Before you finish

```sh
cargo test -p jadeforge-gui
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

Regenerate `docs/screenshot*.png` if the layout changed visibly.
