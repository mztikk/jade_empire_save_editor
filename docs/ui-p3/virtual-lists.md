# Virtualized save browser and picker lists

## Current state

The save browser was already a `VirtualScrollView` (P1). The item database,
learnable style list and gem picker were page-limited: the session returned the
first 30 rows and a **Show more** button added another 30. That kept rendering
bounded but made the list awkward to browse, hid most of the item database, and
did not scale to a corpus or a modded installation with thousands of rows.

## Approach

- The save browser keeps its virtual list. It is one `ListBox` tab stop with a
  stable active descendant; its rows are `ListBoxOption` nodes with zero-based
  position and set size information. Up/Down, Page Up/Page Down, Home/End and
  Enter operate on the announced row, and focus never moves to an unmounted
  row.
- The item, style and gem pickers now hand the **complete** filtered list to a
  shared `widgets::picker_list`. It renders a `VirtualScrollView` with a 56 px
  (scaled) collapsed row height and a viewport of at most seven rows; only the
  rows inside the viewport are built. A short list shrinks to its content height,
  and the viewport grows by the expanded row's measured description height so the
  inline text stays visible.
- `PickerRow` carries a stable `u16` game id, the title, the subtitle, the
  description, the add/learn `Action` and an optional description toggle. The
  item and style rows use the existing `ExpandingName` component, so the
  disclosure chevron and its `... , collapsed, Details` accessible name are the
  same as on owned rows.
- The description renders **inside the selected row**, under its title line, so
  a picker row expands exactly like an owned inventory or style row. The virtual
  list uses a dynamic item size: the collapsed row height plus the description
  height measured from the laid-out paragraph (`on_sized`). The first frame
  after expanding uses a conservative estimate at the narrowest useful picker
  width, so the text is visible before the exact measurement replaces it. The
  estimate and the measurement are kept in the picker's own state, so scrolling
  the row out of view and back does not lose the expanded height.
- Rows set an accessible `ListBoxOption` name (`<name> item`, `<name> style`,
  `<name> gem, <effects>`), position and set size. The picker is one `ListBox`
  tab stop with an active descendant that follows the keyboard. Add/Learn
  buttons keep their contextual accessible names but are not extra tab stops:
  Up/Down, Page Up/Page Down, Home/End move the active row and scroll it into
  view, Enter runs the row's add/learn action, Space toggles its description,
  and Escape leaves the list.
- Stable per-list `AccessibilityId`s survive row recycling, so an active row
  keeps the same announced target when the virtual list drops and rebuilds it.
- The nested list disables drag scrolling: the page already scrolls, and a
  press-drag inside the list must not swallow a row click. Its scrollbar stays
  visible.
- `PAGE_SIZE`, the three page-limit states and the Show more buttons were
  removed. The count labels now show the full total (`291 items`,
  `34 styles to learn`, `95 gems`).

Upstream references:

- `VirtualScrollView` and its dynamic `ItemSize` (a fixed size or a per-index
  callback): <https://docs.rs/freya/0.5.0/freya/prelude/struct.VirtualScrollView.html>
- `ExpandingName` and `AccessibleButton` are local components; their AccessKit
  behaviour relies on Freya's `on_press` Click action:
  <https://docs.rs/freya-core/0.5.0/freya_core/elements/struct.Rect.html>

## Test plan

- The real 89-save corpus still renders through the existing save-list tests,
  including "only visible rows are built" and wheel/end scrolling.
- Headless UI: the item picker shows its full count, builds fewer rows than it
  lists, and an expanded row grows in place with its description inside the row
  bounds; the style and gem pickers show full counts and build fewer rows than
  they list.
- Keyboard headless tests focus each picker through AccessKit, move beyond the
  first seven rows, check the announced active `ListBoxOption` and its position,
  keep focus on the list while rows are recycled, and add an item, learn a style
  and add a gem with Enter. Page Up/Page Down, Home/End and Escape are covered on
  the item picker.
- The save-list test checks the announced active save before and after
  Page Down/Home/Arrow Down, then opens it with Enter. The carried-gem test
  focuses the list through AccessKit and scrolls it with Page Down/Home.
- Adding an item from the virtual list still writes to the session (regression
  test `items_can_be_searched_and_added`).
- Saving at 200 % text: the collapsed picker row height and the measured
  description height both scale with the type tokens; snapshots at 900 and
  1600 px confirm the inline text wraps without running off the card.

## Accessibility, contrast and reduced motion

- Each list is one Tab stop. Arrow, Page Up/Page Down and Home/End move the
  announced active row; the list scrolls it into view and focus stays on the
  list while virtual rows mount and unmount. The carried-gem list is focusable
  and scrolls with the same keys, including when focus is on one of its Remove
  buttons.
- Add/Learn targets keep the 28 px minimum at 100 % and scale with the type.
- No animation; scrolling is immediate.

## Risk

- A screen reader reads the active descendant, not every off-screen row. The
  user arrows or pages through the full list; the previous paging exposed only
  the next page button, so the keyboard path is strictly larger now.
- A row action is no longer a per-row tab stop. A screen reader can still
  activate the active row with Enter or the Click action on its Add/Learn
  button; keyboard users do not have to tab through every row to reach the
  page controls.
- An expanded description does not fit in the seven-row viewport alone, so the
  viewport grows to include it and the page scrolls as a whole; the virtual list
  itself does not gain a nested detail scroll. The row height comes from the
  measured paragraph, not a guess, so no text is clipped or overlapped once the
  first frame settles.
