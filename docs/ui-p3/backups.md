# Backup browsing and restore screen

## Current state

Backups were created by the core on every save, and the Overview/About pages
showed the latest backup path with copy/open/restore instructions. Finding an
older backup meant opening the file manager and copying files by hand.

## Approach

Core: `jadeforge_core::backup` adds `list_backups` (timestamp parsed from the
`<seconds>.<nanos>.<counter>` name, original path, size, newest first) and
`restore_backup`. The restore path verifies the chosen backup by parsing and
fully verifying it, verifies the current file, writes a **new backup of the
current file first**, then writes the restored bytes to an exclusive temporary
sibling and renames it. A failed verification never touches the savegame. Tests
build savegames from scratch and cover listing, ordering, the safety backup and
the untouched-on-error case.

Session: `Session::backups` maps the core entries for the open savegame;
`Session::restore_backup` refuses while there are unsaved edits, refuses a file
that is not a backup of this savegame, and reloads the open document after the
restore. The safety backup is kept in `last_backup`, so the existing Overview
backup card immediately shows how to undo the restore.

UI: a **Backups** toolbar button opens a dedicated page with the timestamp (UTC),
size, original path and a Restore button per row. The button is disabled (and
the page explains why) while the savegame has unsaved changes. Restore asks for
confirmation through the existing dialog seam and reports success in the status
slot.

## Upstream references

- `std::fs` rename/temporary-file semantics, as used by the existing atomic save:
  <https://doc.rust-lang.org/std/fs/fn.rename.html>
- Freya toolbars/buttons: <https://docs.rs/freya/0.5.0/freya/prelude/>
  (the same `Button`, `card` and `AccessibleButton` primitives as the rest of
  the shell).

## Test plan

- Core: a save creates one listable backup with its original path; two saves
  list newest first and ignore another savegame's files; restoring writes the
  backup bytes and creates a second backup; a broken backup leaves the file
  byte-identical.
- Session: list after save, restore reverts the level, restore refuses while
  dirty and leaves the edit untouched.
- Headless UI: save, open Backups, assert the timestamp row and original path,
  confirm a restore and assert the success status.

## Accessibility, contrast and reduced motion

- Each restore is a focusable button with an accessible name that includes the
  backup timestamp; disabled state is exposed, not only painted.
- The path uses the muted text colour (AA on the card) and wraps.
- No animation.

## Risk

- The timestamp in the name is trusted for ordering; a hand-renamed backup
  falls back to file metadata, and sorting then mixes clocks. The created time
  shown is always the parsed or metadata time, never invented.
- Restoring replaces the current savegame. The mandatory safety backup and the
  confirmation make the action recoverable.
