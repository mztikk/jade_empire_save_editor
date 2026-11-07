# Reload from disk after an external change

## Current state

The core recorded a source stamp (length, modification time, inode on Unix) and
refused to overwrite a savegame that changed on disk, but the UI only surfaced
that as a save error. There was no explicit reload path and no explanation of
what to do.

## Approach

Session:

- The load path records a `DiskStamp` (size + modification time). A save and a
  reload refresh it.
- `check_disk_state` compares the current file with the stamp. It is called once
  per render, which is cheap (one `stat`) and catches external edits as soon as
  any UI state changes. A file replaced with the same size and timestamp is
  still caught by the core's authoritative check inside `save`.
- `reload_from_disk` refuses while the in-memory savegame has unsaved changes
  and explains that saving or reverting comes first. The banner offers the
  explicit discard path (`revert`) with a confirmation.
- A failed save caused by a core `Conflict` sets `disk_changed` and replaces the
  error with an explanation: the file changed, it was not overwritten, reload
  or restore a backup.

UI: an assertive live-region banner above the editor content appears when
`disk_changed` is set. Its **Reload from disk** button reloads directly when the
savegame is clean, and asks for confirmation before discarding unsaved edits
when it is dirty. Reload also re-syncs the character form, so the fields show
the file that is on disk.

Detection granularity: an external change is visible after the next render
triggered by any interaction. There is no background file watcher, which avoids
polling a network or Wine filesystem; the save conflict check remains the
authoritative guard. This limitation is recorded in the risk section.

## Test plan

- Session: writing a changed valid savegame behind the open session sets
  `disk_changed`; a dirty reload is refused with the "unsaved changes" message;
  undoing the edit then reloads and shows the new level.
- Headless UI: rewrite the savegame with a level of 42, force a render, assert
  the "changed on disk" explanation, click Reload from disk, assert the status
  and that the sidebar shows Level 42.
- Save conflict handling reuses the core's tested `Conflict` path; the session
  test asserts the explanation is set from that error variant.

## Accessibility, contrast and reduced motion

- The banner text is a `Status` node with an assertive live region; the button
  is a normal focusable button.
- Warning colour is used with an explicit text message, never colour alone.
- No animation; the banner disappears instantly after reload.

## Risk

- A same-size, same-mtime replacement is not detected until the save check.
  Content hashing on every render was rejected as too expensive for folders on
  Wine/Steam mounts; the core remains the final guard.
- Reload closes the undo history because the on-disk baseline changed.
