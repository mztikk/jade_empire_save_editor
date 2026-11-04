---
name: freya-ui
description: Use when building or changing the JadeForge desktop UI with Freya 0.4, when a view renders incorrectly, or when the headless UI tests fail.
---

# Building the JadeForge UI with Freya 0.4

## Structure that must not be broken

- `crates/jadeforge-gui/src/session.rs` owns every behaviour and has **no Freya
  types**. It can be unit tested.
- `crates/jadeforge-gui/src/app.rs` only renders state and forwards input.
- `Session` lives in an `Rc<RefCell<..>>` created with `use_hook`. Callbacks
  mutate it and bump a `use_state(u32)` revision.
- The root component **must read** the revision (`let _ = *revision.read();`).
  Without the read Freya does not subscribe and the view never re-renders after
  a click. This was a real bug once.

## Freya 0.4 API notes

```rust
launch(LaunchConfig::new().with_window(
    WindowConfig::new(app).with_title("JadeForge").with_size(1180.0, 760.0),
));

let mut name = use_state(String::new);      // State<T>
Input::new(name)                            // State<String> -> Writable<String>
Button::new().filled().child("Save").on_press(move |_| { ... });
ScrollView::new().show_scrollbar(true).child(...);
rect().vertical().horizontal().center().spacing(8.).padding(Gaps::new_all(12.));
label().text("x").font_size(12.).color((228, 231, 237));
```

- Sizes: `Size::fill()` is 100 % of the parent, `Size::flex(1.0)` grows inside a
  `Content::Flex` parent, `Size::px(n)` is fixed.
- `Content::wrap_spacing(n)` makes a row wrap to the next line, which is what
  form sections use so fields do not get squeezed.
- Styling `.color()`/`.font_size()` come from `TextStyleExt`; they are available
  on `rect()` and `label()`.
- Reading a `State` inside a callback: use `peek()`; inside rendering: `read()`.

## Layout pitfalls seen in this project

- A `rect().width(Size::fill())` used as a spacer squeezes the next label to
  zero width; the label then wraps one character per line and the parent grows
  to thousands of pixels. Put the text on its own row or give it a size.
- `Size::fill()` for a child of a vertical stack takes 100 % of the parent
  height, not the remaining space. Use `Content::Flex` + `Size::flex(1.0)` for
  the main area.
- Rows that should be one line high need an explicit `.height(Size::px(34.))`;
  otherwise a `fill` child can make them grow.
- Item rows use a trailing `rect().width(Size::flex(1.0))` to push the action
  button to the right inside a `Content::Flex` row.

## The feedback loop

```sh
# compile
cargo check -p jadeforge-gui

# headless tests (render the real tree, click things)
cargo test -p jadeforge-gui

# screenshot without a window: inspect the layout visually
cargo run -p jadeforge-gui --example snapshot -- \
  "/media/Games/Heroic/Jade Empire/Save" /tmp/ui.png
```

Read the produced PNG. Text overflow, squeezed inputs and off-screen buttons are
obvious there and impossible to see from a compiler error.

The UI tests use `freya-testing`:

```rust
let (mut test, ()) = TestingRunner::new(
    move || app_with_directory(dir.clone()),
    (1280., 800.).into(),
    |_| {},
    1.0,
);
test.sync_and_update();
let node = test.find(|_, element| {
    Label::try_downcast(element).filter(|label| label.text.contains("Level ")).map(|_| ())
});
```

Click a node by its layout area:

```rust
let area = node.layout().area;
test.click_cursor(CursorPoint::new(
    f64::from(area.origin.x) + f64::from(area.size.width) / 2.0,
    f64::from(area.origin.y) + f64::from(area.size.height) / 2.0,
));
test.sync_and_update();
```

## Accessibility and polish

- Every actionable element is a `Button` with a text child; do not use bare
  rects with click handlers.
- Keep the palette in the constants at the top of `app.rs`.
- Status messages go through `Session::message` so both the UI and tests see
  them.

## Before you finish

```sh
cargo test -p jadeforge-gui
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

Regenerate `docs/screenshot.png` if the layout changed visibly.
