# Pending-changes review and diff before Save

## Current state

Save wrote immediately after confirming nothing. The status said "unsaved
changes", but the user could not see which fields or rows would be written,
especially after edits on several tabs.

## Approach

Every recorded history action carries `PendingChange` rows: the field path or
row description plus the before and after values. `Session::pending_changes`
groups the applied entries by action label. Character edits reuse the
`jadeforge_core::Change` list from `apply_character` (absolute field paths and
formatted values); inventory, style, gem and companion actions add their own
row descriptions.

The footer Save button and Ctrl+S now open a **Review changes** page instead of
writing. It lists:

- a warning that nothing is written yet,
- how many character fields are invalid and will not be applied,
- one block per action group, with `field` and `was X, now Y` per row,
- **Back to editing** and **Write savegame**.

**Write savegame** calls the unchanged `Session::save_applied` path (verified
round trip, atomic write, backup); only a successful write closes the page and
re-syncs the form. An invalid field keeps its text after the write, matching the
pre-P3 behaviour. Cancel and Escape close the review without touching the file.

The native close-guard Save still writes directly: it runs outside the
component tree, where a review page cannot be shown. That path is documented in
the close-dialog design and is unchanged.

## Test plan

- Session: applying a level edit produces a group whose change has the
  `Level` path, `before = "7"`, `after = "9"`; saving clears the groups.
- Headless UI: Save opens Review changes, names
  `PlayerCreature.Stats.UnassignedPts`, shows `was ...`, and Back to editing
  writes nothing; the invalid-field save shows the "still invalid" note and
  keeps the field text.
- Existing save tests were updated to press Write savegame, so the verified
  write path is still exercised end to end.
- No backup default changed: the review says the previous file is kept and the
  write still uses `SaveOptions { backup: true }`.

## Accessibility, contrast and reduced motion

- The review page uses the normal card/section/label hierarchy and ordinary
  buttons with focus rings.
- The field path is a label and the before/after line is a muted label, so
  screen readers read both without a colour cue.
- No animation; page switches are instant.

## Risk

- The review is a page rather than a modal, so the tab bar stays visible. Tab
  clicks close it deliberately; the page is keyed to avoid state reuse.
- A structural change that keeps the savegame dirty with no described field
  shows a generic "Savegame structure" row, never an empty confirmation.
