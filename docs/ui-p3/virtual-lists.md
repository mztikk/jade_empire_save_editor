# Virtualized save browser and picker lists

## Current state

The save browser was already a `VirtualScrollView` (P1). The item database,
learnable style list and gem picker were page-limited: the session returned the
first 30 rows and a **Show more** button added another 30. That kept rendering
bounded but made the list awkward to browse, hid most of the item database, and
did not scale to a corpus or a modded installation with thousands of rows.

## Approach

- The save browser keeps its virtual list and arrow-key navigation. Its data,
  keys and focus semantics did not change.
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
- Rows set an accessible group name (`<name> item`, `<name> style`,
  `<name> gem, <effects>`). The list is one `ListBox` node. Add/Learn buttons
  keep their contextual accessible names.
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
- Adding an item from the virtual list still writes to the session (regression
  test `items_can_be_searched_and_added`).
- Saving at 200 % text: the collapsed picker row height and the measured
  description height both scale with the type tokens; snapshots at 900 and
  1600 px confirm the inline text wraps without running off the card.

## Accessibility, contrast and reduced motion

- Each row is reachable by Tab while it is built; the inner list scrolls with
  the wheel or the scrollbar. Off-screen rows are not focus stops until they
  scroll into view, which is the normal virtual-list contract.
- Add/Learn targets keep the 28 px minimum at 100 % and scale with the type.
- No animation; scrolling is immediate.

## Risk

- A virtualized picker cannot tab through rows that are outside the viewport;
  the user scrolls the list to reach them. The previous paging had the same
  property (the next page was a button), but the full list is now browsable.
- An expanded description does not fit in the seven-row viewport alone, so the
  viewport grows to include it and the page scrolls as a whole; the virtual list
  itself does not gain a nested detail scroll. The row height comes from the
  measured paragraph, not a guess, so no text is clipped or overlapped once the
  first frame settles.
