# Desktop UI review: delivered work and open follow-ups

**Reviewed:** 2026-11-05, P1 follow-up 2026-11-06, P2 polish 2026-11-06, P3 delivery 2026-11-07 · **Base:** `main` at `491163c`
**Scope:** the Freya 0.5 desktop UI in `crates/jadeforge-gui`, the session state it renders, and the real 89-savegame corpus.
**Status:** P0, P1 and P2 are complete, and the P3 work shipped in v0.3.0
([PR #32](https://github.com/mztikk/jade_empire_save_editor/pull/32)) except for
the blocked item-icon item
([#9](https://github.com/mztikk/jade_empire_save_editor/issues/9)). The
remaining open follow-ups are the platform contrast preference
([#33](https://github.com/mztikk/jade_empire_save_editor/issues/33)) and the
upstream `Input` label association
([#3](https://github.com/mztikk/jade_empire_save_editor/issues/3)).

![JadeForge overview](screenshot.png)

## What the review covered

The review read the GUI, session and theme code, ran the headless UI suite and rendered all six tabs at 900×700, 1180×760, 1280×900 and 1600×900, plus expanded, empty, Gold-less and invalid-form states. The real window was inspected over AT-SPI, and the windowed suite was run with real KWin input, including a keyboard-navigation scenario.

The strongest parts of the current UI are the coherent jade-on-charcoal palette, the save-safety pipeline (verification, atomic write, backups and close guard), the fixed preview square, and the layout-shift/geometry tests. The review's main follow-up areas were responsive composition, state handling around save/folder changes, discoverability of expansion/upgrades, and accessibility outside the app's control; the P1-P3 work addressed the app-side items, and Freya 0.5 removed the focus, action and dynamic-label blockers, leaving #3.

## P0: addressed

| Item | Result |
| --- | --- |
| Gold-less saves opened as `invalid character input` | Fixed: a missing `Stats.Gold` stays absent; a UI regression removes the field from the fixture. |
| Save browser showed two rows at the supported minimum | Supported minimum is now **900×700** and the preview is **120×120**; at least four rows fit and this is tested. |
| Revert and destructive removes were one click | Revert, Remove item/style/gem now confirm; the last style cannot be removed because it is the layout template for learning another. |
| Screen reader could not read the page | Static text, headings, status live region, tab/list roles and states, expanded/owned states and a single-tab-stop save list are exposed. Headless a11y tests cover the tree. |
| Baseline CI failed on every PR | Separate fix: macOS non-UTF-8 fixture skip, Windows `sync_parent` lint, fixed sort-arrow slot, and Windows `RUSTDOCFLAGS` environment. |

Merged in [#5](https://github.com/mztikk/jade_empire_save_editor/pull/5) and the baseline fix in [#6](https://github.com/mztikk/jade_empire_save_editor/pull/6).

App-side accessibility is in place, and Freya 0.5 removed the focus, action and dynamic-label blockers. The remaining upstream gap is the `Input` label association (#3); see the status below.

## Delivered work and open follow-ups

The GitHub issues remain the historical checklists. This section is a short map
of what was delivered and what is still open.

### P1: [#7](https://github.com/mztikk/jade_empire_save_editor/issues/7)

Delivered on `main` in [#24](https://github.com/mztikk/jade_empire_save_editor/pull/24):

| Area | Result |
| --- | --- |
| Responsive composition | Fluid 3+3 identity grid at 1180×760 and 900×700, overview glance cells 1-up → 2+2+1, hero stats 2×2, 1100 px editor cap and a 720 px text measure; descriptions expand in full. |
| Window geometry | `geometry::fit_window` clamps the default and remembered size to the monitor, lowers the minimum on short screens and keeps a stored position on screen; a plugin remembers the size everywhere and the position on X11, because Wayland compositors own placement. Tested for 1366×768 at 125 %. |
| Clipped names | Ellipsis plus a tooltip on save, item, companion and sidebar names; the overview path wraps and has a Copy path action. |
| State safety | Scan keeps the open savegame for the same folder and re-locates it by path; Browse checks dirty state before the dialog; filters, paging, expanded rows and scroll positions reset on a document change; every filter has a Clear button. |
| Save vs. form | Each field validates on its own and shows its range; valid fields apply even when another is invalid; Save writes applied edits, reports the invalid form separately and focuses the first invalid field. |
| Dirty state | Content comparison against canonical block payloads: edit-then-restore clears the flag. |
| No-save and feedback | Tabs and Save/Revert are disabled without a savegame and the empty state has a primary folder action; status is a persistent three-line, copyable slot with a success glyph; unreadable sections are not shown as empty lists; folder and game-installation errors are distinct. |
| Discoverability | Chevron/Details rows, orb hover and focus details with the buy/remove explanation, a read-only style notice, gem effect lines in the picker. |
| Editing UX | Contextual accessible names, Ctrl+S/O/F, F5, Enter, Escape and tab arrow keys, direct numeric stack/level editing, character name and UTC modification time in save rows. |
| Accessibility | 28 px row targets, 11 px micro / 14 px body type, 3:1 control borders, tab underline, amber unsaved state, focus restoration after a removal, focusable keyboard scroll view. |
| Performance | Manual Scan (startup, button, Enter, F5 and Browse) runs on a worker with an in-flight status and cancellation; save filter/sort, item suggestions, learnable styles and the form reparse are memoised; save rows keep stable `Action` objects and paint hover without a rebuild; frame-time instrumentation and an 89-save render test. |

Remaining verification caveat for #7:

- `just live` passes six windowed scenarios, including `window_geometry_restarts`,
  which runs twice with one config directory. The first launch is seeded with a
  1000x720 rectangle and an off-screen position; after it closes, the harness
  verifies that the geometry plugin replaced the seed with the real rectangle,
  and the relaunch opens at the stored 1000x720 inner size. Real window
  creation, presentation, pointer input and keyboard navigation are verified.
  The scroll-view paging keys only have their focus target asserted, so the
  real scroll offset remains unverified.

### P2: [#8](https://github.com/mztikk/jade_empire_save_editor/issues/8)

Delivered on `main` in [#30](https://github.com/mztikk/jade_empire_save_editor/pull/30):

| Area | Result |
| --- | --- |
| Copy, counts, vocabulary | Counts use `plural()` with their noun (`30 of 34 styles`, `1 socket`, `0 carried gems`); the amulet hint names the socket count and tier; `ALL STYLES` became `STYLES TO LEARN`; `Lv`, `Played`, `Unlock` and `predefined gems` were replaced by the same wording as the rest of the app; raw `plot_items`/`improve` categories and lower-case style kinds map to friendly labels. |
| Backups, dialogs, no-op states | The successful save message and Overview card show the exact backup path with copy/open actions and restore instructions; the close guard offers explicit Save / Discard / Cancel; Save and Revert are disabled with an explanation when there is nothing to write or discard, while the success status stays visible; folder-dialog cancellation and unreadable picked paths are reported distinctly. |
| Visual polish | The duplicate Overview screenshot was removed and the sidebar preview aligns with the identity block at the shared card radius; the glance values are full-width rows at every supported width; the save path has copy/open-folder actions; the first scan has sidebar and editor skeletons and manual scans have a reserved busy indicator in the tab bar. |
| Guidance and settings | The About entry point is an outlined toolbar button and opening it clears the tab selection, so no editor tab looks active while the page is shown; the page shows the version, config path, remembered save folder and sort order, backup rules and keyboard shortcuts; each character field shows its range and unit and the optional Gold field explains when it is created; the item picker has a category filter; the save filter placeholder names the character name it also matches. |
| Tooling and documentation | Snapshot `--query` resolves the active tab's own filter instead of the first `Filter ` field; this document has a screenshot/evidence maintenance note; README and CONTRIBUTING link the P1/P2/P3 issues. |

### P3: [#9](https://github.com/mztikk/jade_empire_save_editor/issues/9)

Delivered in [v0.3.0](https://github.com/mztikk/jade_empire_save_editor/releases/tag/v0.3.0)
via [#32](https://github.com/mztikk/jade_empire_save_editor/pull/32); each item
has a design note with its test plan in [docs/ui-p3/](ui-p3/README.md). Item
icons are the one blocked item.

| Area | Result |
| --- | --- |
| Themes | Dark, light, system-following and high-contrast palettes share one semantic `Palette`; the choice persists and is selected on About. Palette tests assert WCAG AA on every surface for every theme and AAA for high contrast. High contrast is manual because Freya/winit expose no forced-colours signal (recorded in the note; the platform-contrast follow-up is #33). |
| Text scaling | 100/125/150/200 % scales type, spacing, radii and type-related fixed dimensions; the status slot and save rows grow, and above 100 % the sidebar hides its decorative preview and duplicate selected-save block, the shell compacts, the form reflows and the tab strip drops its busy slot. Freya's Input, Select and MenuItem inherit the body scale, and page/toolbar actions use one shared ActionButton. Tests cover 900x700, 1180x760 and 1280x800 at all four scales, plus German at 125-200 %. |
| Localization | Stable semantic ids in a new `jadeforge-i18n` crate: complete English and German peer catalogs (472 ids), a generated QA pseudolocale, strict key/placeholder/completeness checks and build-generated typed keys. `t`/`fill`/`plural` render `⟦id⟧` markers instead of English fallback; the session calls the shared catalog directly. About shows **Deutsch**, the choice persists as `de`, and the preview path remains for an incomplete future language. The last reuse pass composes the Overview stat lists from the shared field ids and a catalogued separator. |
| Large lists | Save browser unchanged (already virtual); item, style and gem pickers now virtualize the full filtered list, keep stable ids and accessible names, and show descriptions in a panel below the fixed-height rows. |
| Editing safety | Bounded per-action undo/redo with coalesced character typing, a pending-changes review page before Save, a backup browser with verified restore and safety copy, and an external-change banner with an explicit reload/discard path. |
| Content and identity | About now carries diagnostics (version, savegame, blocks, size, theme, scale, language, renderer, backup folder), licenses, a Copy diagnostics action and a complete shortcut sheet opened with Ctrl+/; window remembering is configurable. Item/style icons are blocked: the references resolve but the images are engine BIF/KEY textures, not RIM images. |

P3 verification caveats:

- Item icons are blocked on a new KEY/BIF + engine-texture reader; the evidence
  (icon references, RIM resource types and payload samples) is recorded in
  [docs/ui-p3/icons.md](ui-p3/icons.md). No placeholder icon was added.
- The windowed suite passed all six scenarios on the review desktop (KDE
  Plasma Wayland, KWin fake input) on 2026-11-07. It covered window creation and
  presentation, save opening and tab switching, item filtering, keyboard
  save-list navigation and the two-pass `window_geometry_restarts` (seeded
  1000x720, remembered rectangle, restored 1000x720). The new Backups, Review
  and About pages and the theme/scale variants are covered by the headless
  suite and the snapshot example.
- Linux has no system theme through winit (X11 unsupported, Wayland only
  overrides). A `System` choice therefore reads the freedesktop appearance
  portal, with KDE's `kdeglobals` as the first-frame fallback, and follows
  portal changes while the window is open. Detection was verified on the
  reviewer's KDE Plasma Wayland session, where the portal reports dark; the
  live-desktop test is opt-in because CI has no session bus.

## Upstream accessibility status

Freya 0.5 resolved two of the three blockers. #2 and #4 are ready to close; #3
still needs an upstream `Input` label API.

| Issue | Status |
| --- | --- |
| [#2](https://github.com/mztikk/jade_empire_save_editor/issues/2) | Resolved by Freya 0.5.0: `on_press` registers and handles the AccessKit `Click` action, `Action::Focus`/`Blur` are handled, and every `TreeUpdate` carries the focused id. The live AT-SPI check confirmed focus reporting and activation of tabs, the save row, the footer Undo button and a row action. |
| [#3](https://github.com/mztikk/jade_empire_save_editor/issues/3) | Still blocked: `Input` exposes only `a11y_id`, and its accessibility name is still the current value or placeholder. The captions remain named field groups until Freya can label the input itself. |
| [#4](https://github.com/mztikk/jade_empire_save_editor/issues/4) | Resolved by Freya 0.5.0: a label text change marks the accessibility diff and the node is rebuilt with the current value. The content-based key is gone, and the live AT-SPI check updated the existing count and status nodes through empty, unreadable and populated scans. |

The audible pass for Orca, NVDA and VoiceOver is the manual checklist in
[docs/LIVE-UI.md](LIVE-UI.md); the headless and AT-SPI checks cover the tree,
focus and action path those screen readers consume.

## Screenshots and evidence maintenance

The durable evidence in this repository is deliberately small:

- `docs/screenshot.png` is the README hero (Overview at 1280×800).
- `docs/screenshot-overview.png`, `-character`, `-styles`, `-inventory`,
  `-gems` and `-companions` are the per-tab evidence at the default size.
- The review prose, the P1/P2/P3 issue links and the verification records are
  the rest of the evidence. Working images, recordings and long reports stay in
  the ignored `scratch/` or outside the repository; they are never committed.

Regenerate the tracked images after a visible layout change from a real save
folder (the examples use the synthetic fixture when `JADEFORGE_SAVE_DIR` is
unset):

```sh
cargo run -p jadeforge-gui --example snapshot -- "$JADEFORGE_SAVE_DIR" docs/screenshot.png all
cargo run -p jadeforge-gui --example snapshot -- "$JADEFORGE_SAVE_DIR" docs/screenshot.png Overview
cargo run -p jadeforge-gui --example snapshot -- "$JADEFORGE_SAVE_DIR" scratch/narrow.png Overview --width 900 --height 700
cargo run -p jadeforge-gui --example snapshot -- "$JADEFORGE_SAVE_DIR" scratch/wide.png all --width 1600 --height 900
cargo run -p jadeforge-gui --example snapshot -- "$JADEFORGE_SAVE_DIR" scratch/light.png all --theme light
cargo run -p jadeforge-gui --example snapshot -- "$JADEFORGE_SAVE_DIR" scratch/zoom.png all --scale 200 --width 1600 --height 900
```

`--query <text>` types into the active tab's own picker filter (Inventory,
Styles or Gems) and refuses to fall back to the sidebar save filter. The
headless suite asserts the same placeholders, the row geometry, the responsive
breakpoints and the a11y tree, so a screenshot alone is never the test.

## Verification

- `just check`: the full workspace suite (formatting, clippy, rustdoc, nextest and doctests) passes after the P3 work; the windowed suite passed all six scenarios separately. P3 adds palette/contrast tests for three themes, text-scale token tests, localization catalog/placeholder/completeness tests, core backup tests, session undo/diff/backup/reload tests, virtualized-picker tests and headless UI tests for themes, 200 % text, undo/redo, review, backups, external reload, diagnostics and the shortcut sheet.
- P2 headless coverage: the empty carried-gems count, amulet summary, `STYLES TO LEARN`, friendly item categories and the category filter, `Level`/`Time played`/available wording, the optional-Gold explanation, field units, the save-filter character match, the About page and its settings, the About entry semantics and tab selection while the page is open, backup surfacing/copy/open actions, no-op Save/Revert, folder-picker cancellation and unreadable paths, loading skeletons and the manual-scan busy slot, full-width overview rows at 900/1280/1600, and the sidebar preview alignment. The snapshot example's filter targeting is covered by the tab-filter unit test and a headless Styles-filter test.
- Windowed verification for P2: not run, because the user did not grant permission under AGENTS.md rule 13. The native close dialog's Save / Discard / Cancel labels, the actual folder-dialog cancellation backend and the Windows ComCtl32 v6 manifest therefore remain unverified in a real window; the choice mapping and the save-before-close decision table are unit tested, and the manifest is embedded by the existing Windows resource build.
- Headless coverage of the P1 work: geometry at 1180x760, 900x700 and 1600x900; DPI geometry for 1366x768 at 125 %; scan-keeps-save; dirty content comparison; invalid-form save independence; first-invalid-field focus; focus restoration after removal; target sizes; keyboard scrolling focus; the 89-save render test.
- Headless accessibility coverage for the Freya 0.5 follow-up: dynamic scan count, status and empty-state updates; a synthetic AccessKit `Focus` on a tab; and synthetic `Click` activation of the tab, save row, footer Undo button and an inventory row action. The existing dynamic-hint and focus-restoration tests stay green.
- Live AT-SPI check on KDE Plasma Wayland (2026-11-07): `grab_focus` on a tab reported the `FOCUSED` state; `Click` opened the save row, switched tabs, changed an item stack and pressed the footer Undo button, which cleared the edit. Moving the save folder out and rescanning updated the existing `1 SAVEGAME` and status nodes to `0 SAVEGAMES` and the empty message, and a broken save updated them back; the empty-list hint appeared and disappeared. Orca 51.1 was installed, but its session service was already owned, so audible output stays on the manual checklist below instead of being captured by a second instance.
- `just live`: all six windowed scenarios passed on Wayland with the KWin input backend. The window opened at the configured size, save opening and tab switching, item filtering, keyboard save-list navigation and real frame presentation all worked; the captured frame was written to the ignored `target/live-ui/presented.png`. The two-pass `window_geometry_restarts` scenario opened at the seeded 1000x720, the plugin stored the real rectangle on close, and the relaunch restored 1000x720. Each scenario uses its own settings directory; the two geometry passes deliberately share one. The P3 verification rerun on 2026-11-07 passed the same six scenarios.
- Live screen-reader check: Orca 51.1 (Speech Dispatcher 0.12.1, espeak-ng 1.52) announced the polite scan statuses and the assertive folder error in a manual run. The AT-SPI event log showed the full text with politeness level 1 for the scan messages and level 2 for "The save folder … does not exist …". This closes the P1 status live-region check.
- P0, P1 and P2 are merged on `main`; the P3 work shipped in v0.3.0 (PR #32).
  The only open P3 item is the blocked icon work (#9); #33 tracks platform
  contrast. #2 and #4 have their Freya blockers removed and are ready to close;
  #3 remains blocked by the `Input` label API.

## Notes

- The review was performed against `main` at `aecc57a` and re-verified after the P0 merge at `9151c2a`.
- The local evidence images and the longer working report were kept outside the repository (ignored `scratch/`); this file and the linked issues are the durable record.
