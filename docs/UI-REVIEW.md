# Desktop UI review: status and remaining work

**Reviewed:** 2026-11-05 · **Base:** `main` at `9151c2a`
**Scope:** the Freya 0.5 desktop UI in `crates/jadeforge-gui`, the session state it renders, and the real 89-savegame corpus.
**Status:** P0 release blockers are addressed. P1/P2/P3 work is tracked in GitHub issues; the remaining upstream blocker is the missing `Input` label association.

![JadeForge overview](screenshot.png)

## What the review covered

The review read the GUI, session and theme code, ran the headless UI suite and rendered all six tabs at 900×700, 1180×760, 1280×900 and 1600×900, plus expanded, empty, Gold-less and invalid-form states. The real window was inspected over AT-SPI, and the windowed suite was run with real KWin input, including a keyboard-navigation scenario.

The strongest parts of the current UI are the coherent jade-on-charcoal palette, the save-safety pipeline (verification, atomic write, backups and close guard), the fixed preview square, and the layout-shift/geometry tests. The main remaining work is responsive composition, state handling around save/folder changes, discoverability of expansion/upgrades, and accessibility outside the app's control.

## P0: addressed

| Item | Result |
| --- | --- |
| Gold-less saves opened as `invalid character input` | Fixed: a missing `Stats.Gold` stays absent; a UI regression removes the field from the fixture. |
| Save browser showed two rows at the supported minimum | Supported minimum is now **900×700** and the preview is **120×120**; at least four rows fit and this is tested. |
| Revert and destructive removes were one click | Revert, Remove item/style/gem now confirm; the last style cannot be removed because it is the layout template for learning another. |
| Screen reader could not read the page | Static text, headings, status live region, tab/list roles and states, expanded/owned states and a single-tab-stop save list are exposed. Headless a11y tests cover the tree. |
| Baseline CI failed on every PR | Separate fix: macOS non-UTF-8 fixture skip, Windows `sync_parent` lint, fixed sort-arrow slot, and Windows `RUSTDOCFLAGS` environment. |

Merged in [#5](https://github.com/mztikk/jade_empire_save_editor/pull/5) and the baseline fix in [#6](https://github.com/mztikk/jade_empire_save_editor/pull/6).

App-side accessibility is in place, but **full screen-reader operation is still blocked upstream**; see the open issues below.

## Remaining work

The GitHub issues are the source of truth for checklists. This section is a short map.

### P1: [#7](https://github.com/mztikk/jade_empire_save_editor/issues/7)

High-value follow-ups:

- **Responsive composition:** default 1180×760 still wraps `KILLS` alone; the Overview grids use fixed 340/120 px cells; wide windows have no content cap; window geometry/DPI is not remembered.
- **State safety:** Scan clears the open save; Browse opens the dialog before the dirty check; filters/page limits/expanded ids/scroll leak across saves and folders; an invalid character form blocks saving unrelated edits; dirty is a sticky flag, not content equality.
- **Feedback and recovery:** no-save state leaves tabs and Save/Revert enabled; status messages are clipped and ephemeral; folder errors look empty; game data cannot be located from the UI.
- **Discoverability:** expandable rows have no chevron; orbs have no effect/cost preview; read-only styles are silent; descriptions truncate at five lines; gem effects are hidden until added.
- **Editing UX:** no keyboard shortcuts; stack size and companion level are stepper-only; save rows omit the character name and modified time.
- **Accessibility polish:** targets below 24×24 (sort 38×12, ± 23×25), 10–11 px secondary type, low non-text contrast for inputs/scrollbars, colour-only active tab, green overload for unsaved/success, missing hover states.
- **Performance:** manual Scan blocks the UI thread; per-frame derived lists and all-row hover rebuilds; no instrumentation or 89-save render test.

### P2: [#8](https://github.com/mztikk/jade_empire_save_editor/issues/8)

Polish after P1:

- Copy and vocabulary: the empty carried-gems hint can read “1 gem”, `ALL STYLES` lists only unlearned styles, `Level`/`Lv`, `Played`, raw categories and `predefined gems` need a pass.
- Backup discovery/restore, a three-way close dialog, disabled/no-op Save and Revert states.
- Visual polish: duplicate Overview screenshot, sidebar alignment, preview radius, full-width Overview values, open-folder action, loading states.
- Guidance/settings: About/version/help, config location, field ranges/units, item category filtering, save-filter placeholder.
- Tooling/docs: snapshot `--query` targets the first filter (the save filter) instead of the active tab’s filter; document evidence/screenshot maintenance.

### P3: [#9](https://github.com/mztikk/jade_empire_save_editor/issues/9)

Strategic work:

- Light/system theme, high-contrast support, text scaling/zoom, UI localization.
- Virtualized large lists, full undo/redo, pending-changes diff, backup restore screen, external-change reload.
- Item/style icons, About/help, command palette/cheat sheet, configurable window geometry and multi-monitor policy.

## Upstream accessibility blockers

These are not fixable from this repository alone.

Freya 0.5 resolved two of the three blockers: an element with `on_press` now
registers the AccessKit `Click` action and handles the action request, label
and paragraph values are filled while the accessibility tree is built, and
accessibility nodes are refreshed when their diff says so. The headless suite
activates a tab through a synthetic AccessKit `Click` and asserts that dynamic
label text reaches the effective AccessKit node. The remaining blocker is
tracked as an open issue:

| Issue | Blocker |
| --- | --- |
| [#3](https://github.com/mztikk/jade_empire_save_editor/issues/3) | `Input` has no label/labelled-by API; fields are still named by their current value or placeholder, even though captions are exposed as groups. |

## Verification

- `just check`: 306 workspace tests, 42 headless UI tests, clippy and formatting clean.
- `just live`: all 5 windowed scenarios pass, including `keyboard_navigates_the_save_list` (Tab → list box, Down → second save, Enter → opens) through real KWin input.
- Live AT-SPI check: static text, count, status, tab selected state and the focusable save list box were current; the desktop accessibility setting was restored afterwards.
- The P0 work is merged on `main`; the remaining work above is tracked in #7, #8, #9 and the three upstream issues.

## Notes

- The review was performed against `main` at `aecc57a` and re-verified after the P0 merge at `9151c2a`.
- The local evidence images and the longer working report were kept outside the repository (ignored `scratch/`); this file and the linked issues are the durable record.
