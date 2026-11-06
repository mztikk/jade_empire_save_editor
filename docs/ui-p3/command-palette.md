# Shortcut cheat sheet

## Decision

A full command palette is unnecessary for a six-tab editor with eleven
actions: the About page already is the searchable help surface, and a palette
would duplicate the save/folder/review flows while adding a second focus trap.
The cheaper, clearer choice from the issue is a complete shortcut cheat sheet.

## Current state and approach

The About/help page has a **KEYBOARD SHORTCUTS** section. It now covers every
global shortcut: Ctrl+S (opens the review), Ctrl+O, Ctrl+F, Ctrl+Z, Ctrl+Y,
Ctrl+Shift+Z, F5, Ctrl+/ and Escape. **Ctrl+/ opens the About page from any
tab or page**, so the cheat sheet is reachable without hunting through the
toolbar.

A command palette was evaluated and rejected for this release:

- every command already has a visible toolbar/footer control and a shortcut,
- the palette would need its own focus management, filtering, stable keys and
  accessibility tree on top of the existing pages,
- the issue only asks for one of the two options.

## Test plan

- Headless UI: Ctrl+/ opens the page and the sheet lists the undo shortcut.
- The About diagnostics test asserts Ctrl+Z in the sheet.
- Escape closes the page (existing test).

## Accessibility, contrast and reduced motion

- The sheet is plain label rows in the existing card, readable by screen
  readers as a heading plus label/value rows.
- No animation or transient overlay; opening is a normal page switch.

## Risk

- Ctrl+/ depends on the keyboard layout reporting `/`; the handler also accepts
  `?` for layouts where Ctrl+/ produces that character. A future command palette
  can reuse the same global key handler if the action set grows.
