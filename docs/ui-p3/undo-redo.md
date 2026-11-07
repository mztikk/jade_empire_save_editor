# Per-action undo and redo history

## Current state

Before P3 the editor had confirmations for destructive rows and Revert-all
(reload from disk). There was no way to undo one edit; a wrong stack size or a
style upgrade meant reverting every change since the last save.

## Approach

`Session` keeps a bounded history of `HistoryEntry` values. Each entry stores:

- a short action label (`Character`, `Item stack`, `Style upgrade`, ...),
- the field/row before and after values used by the diff screen,
- the canonical payload of every savegame block the action changed, before and
  after the action. A block that was untouched is stored as `None` and restored
  from the container's original bytes.

Restoring a payload parses it as the same block, validates it and replaces the
block tree, so undo is exact at block level and also works for structural edits
(adding an item appends a struct; restoring the old payload removes it).
`SaveGame::restore_block_from_source` marks a block clean again, which lets
`refresh_dirty` clear the unsaved badge when every edit is undone.

Coalescing: consecutive edits that change the same set of character form fields
within 1.5 s merge into one entry, so typing `123` is one undo. Item and style
actions are individual entries. A new action after undo truncates the redo
branch.

Bounds: at most 100 entries and 32 MiB of captured payloads. Older entries are
dropped first; a savegame's `save.sav` is a few hundred kilobytes, so this is
dozens of steps in practice. A save clears the history because the on-disk
baseline is now the saved file.

UI: Undo/Redo buttons in the footer, Ctrl+Z and Ctrl+Y (plus Ctrl+Shift+Z). The
buttons disable when there is nothing to do; both re-sync the character form
from the restored savegame.

Core APIs used: `SaveGame::block_canonical_payload`,
`SaveGame::restore_block_payload`, `SaveGame::restore_block_from_source` in
[crates/jadeforge-core/src/save.rs](../../crates/jadeforge-core/src/save.rs).
The undo history itself lives in
[crates/jadeforge-session/src/session.rs](../../crates/jadeforge-session/src/session.rs).

## Test plan

- Session: two character edits undo/redo in order; undoing everything clears
  `is_dirty`; typing two digits coalesces to one step; changing an item stack
  restores the previous count and clears the dirty state.
- After a save, `can_undo` is false and the history is empty.
- Headless UI: footer Undo and Redo update the status badge; Ctrl+Z and Ctrl+Y
  do the same.
- Memory cap and serialization failure paths are covered by the state
  comparison and the existing savegame round-trip tests; the cap itself has a
  documented constant and is not time-based.

## Accessibility, contrast and reduced motion

- Buttons are ordinary theme buttons with visible disabled states and focus
  rings.
- Undo/redo has no animation.
- Status text announces the undone action; screen readers hear it in the
  existing live region.

## Risk

- History stores canonical block payloads, not full savegame copies, but a very
  large edited block still costs memory. The byte cap bounds it.
- Coalescing is time based. A slow typist gets one entry per 1.5 s pause, which
  is predictable; the alternative (one entry per keystroke) was rejected.
