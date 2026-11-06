# Desktop UI review: status and remaining work

**Reviewed:** 2026-11-05, P1 follow-up 2026-11-06, P2 polish 2026-11-06 · **Base:** `main` at `491163c`
**Scope:** the Freya 0.5 desktop UI in `crates/jadeforge-gui`, the session state it renders, and the real 89-savegame corpus.
**Status:** P0 release blockers, the P1 follow-ups and the P2 polish are
addressed. The P3 strategic work is implemented on `feat/ui-p3-strategic` and
verified with the headless suite and snapshots; the item-icon reference is
blocked on engine BIF/KEY texture data (see
[docs/ui-p3/icons.md](ui-p3/icons.md)), and the windowed suite was not run
because permission was not granted. The remaining upstream blocker is the
missing `Input` label association.

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

Addressed on the `fix/ui-p1-followups` branch:

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

Addressed on `fix/ui-p2-polish`:

| Area | Result |
| --- | --- |
| Copy, counts, vocabulary | Counts use `plural()` with their noun (`30 of 34 styles`, `1 socket`, `0 carried gems`); the amulet hint names the socket count and tier; `ALL STYLES` became `STYLES TO LEARN`; `Lv`, `Played`, `Unlock` and `predefined gems` were replaced by the same wording as the rest of the app; raw `plot_items`/`improve` categories and lower-case style kinds map to friendly labels. |
| Backups, dialogs, no-op states | The successful save message and Overview card show the exact backup path with copy/open actions and restore instructions; the close guard offers explicit Save / Discard / Cancel; Save and Revert are disabled with an explanation when there is nothing to write or discard, while the success status stays visible; folder-dialog cancellation and unreadable picked paths are reported distinctly. |
| Visual polish | The duplicate Overview screenshot was removed and the sidebar preview aligns with the identity block at the shared card radius; the glance values are full-width rows at every supported width; the save path has copy/open-folder actions; the first scan has sidebar and editor skeletons and manual scans have a reserved busy indicator in the tab bar. |
| Guidance and settings | The About entry point is an outlined toolbar button and opening it clears the tab selection, so no editor tab looks active while the page is shown; the page shows the version, config path, remembered save folder and sort order, backup rules and keyboard shortcuts; each character field shows its range and unit and the optional Gold field explains when it is created; the item picker has a category filter; the save filter placeholder names the character name it also matches. |
| Tooling and documentation | Snapshot `--query` resolves the active tab's own filter instead of the first `Filter ` field; this document has a screenshot/evidence maintenance note; README and CONTRIBUTING link the P1/P2/P3 issues. |

### P3: [#9](https://github.com/mztikk/jade_empire_save_editor/issues/9)

Addressed on `feat/ui-p3-strategic`; each item has a design note with its test
plan in [docs/ui-p3/](ui-p3/README.md). Item icons are the one blocked item.

| Area | Result |
| --- | --- |
| Themes | Dark, light, system-following and high-contrast palettes share one semantic `Palette`; the choice persists and is selected on About. Palette tests assert WCAG AA on every surface for every theme and AAA for high contrast. High contrast is manual because Freya/winit expose no forced-colours signal (recorded in the note). |
| Text scaling | 100/125/150/200 % scales type, spacing, radii and type-related fixed dimensions; the status slot and save rows grow, and the sidebar hides its decorative preview above 125 % so the list stays usable. Tests render all tabs at 200 %. |
| Localization | English source catalog with a QA pseudolocale, `LC_ALL`/`LANG` detection, an About selector and a source-scan completeness test. No unreviewed translation ships; dynamic session messages remain English templates and are documented as the follow-up. |
| Large lists | Save browser unchanged (already virtual); item, style and gem pickers now virtualize the full filtered list, keep stable ids and accessible names, and show descriptions in a panel below the fixed-height rows. |
| Editing safety | Bounded per-action undo/redo with coalesced character typing, a pending-changes review page before Save, a backup browser with verified restore and safety copy, and an external-change banner with an explicit reload/discard path. |
| Content and identity | About now carries diagnostics (version, savegame, blocks, size, theme, scale, language, renderer, backup folder), licenses, a Copy diagnostics action and a complete shortcut sheet opened with Ctrl+/; window remembering is configurable. Item/style icons are blocked: the references resolve but the images are engine BIF/KEY textures, not RIM images. |

P3 verification caveats:

- Item icons are blocked on a new KEY/BIF + engine-texture reader; the evidence
  (icon references, RIM resource types and payload samples) is recorded in
  [docs/ui-p3/icons.md](ui-p3/icons.md). No placeholder icon was added.
- The windowed suite was not run for P3 because permission was not granted.
  Theme, scaling, localization, page routing and the new banners are covered by
  the headless suite and the snapshot example; the only window-path behaviour
  in P3 is the configurable window remembering, whose decision logic is unit
  tested and whose real-window scenario (`window_geometry_restarts`) remains
  un-run on this branch.
- Linux has no system theme through winit (X11 unsupported, Wayland only
  overrides). A `System` choice therefore reads the freedesktop appearance
  portal, with KDE's `kdeglobals` as the first-frame fallback, and follows
  portal changes while the window is open. Detection was verified on the
  reviewer's KDE Plasma Wayland session, where the portal reports dark; the
  live-desktop test is opt-in because CI has no session bus.

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

- `just check`: the full workspace suite (formatting, clippy, rustdoc, nextest and doctests) passes after the P3 work; the opt-in live suite remains skipped. P3 adds palette/contrast tests for three themes, text-scale token tests, source-scan localization completeness, core backup tests, session undo/diff/backup/reload tests, virtualized-picker tests and headless UI tests for themes, 200 % text, undo/redo, review, backups, external reload, diagnostics and the shortcut sheet.
- P2 headless coverage: the empty carried-gems count, amulet summary, `STYLES TO LEARN`, friendly item categories and the category filter, `Level`/`Time played`/available wording, the optional-Gold explanation, field units, the save-filter character match, the About page and its settings, the About entry semantics and tab selection while the page is open, backup surfacing/copy/open actions, no-op Save/Revert, folder-picker cancellation and unreadable paths, loading skeletons and the manual-scan busy slot, full-width overview rows at 900/1280/1600, and the sidebar preview alignment. The snapshot example's filter targeting is covered by the tab-filter unit test and a headless Styles-filter test.
- Windowed verification for P2: not run, because the user did not grant permission under AGENTS.md rule 13. The native close dialog's Save / Discard / Cancel labels, the actual folder-dialog cancellation backend and the Windows ComCtl32 v6 manifest therefore remain unverified in a real window; the choice mapping and the save-before-close decision table are unit tested, and the manifest is embedded by the existing Windows resource build.
- Headless coverage of the P1 work: geometry at 1180x760, 900x700 and 1600x900; DPI geometry for 1366x768 at 125 %; scan-keeps-save; dirty content comparison; invalid-form save independence; first-invalid-field focus; focus restoration after removal; target sizes; keyboard scrolling focus; the 89-save render test.
- `just live`: all six windowed scenarios passed on Wayland with the KWin input backend. The window opened at the configured size, save opening and tab switching, item filtering, keyboard save-list navigation and real frame presentation all worked; the captured frame was written to the ignored `target/live-ui/presented.png`. The two-pass `window_geometry_restarts` scenario opened at the seeded 1000x720, the plugin stored the real rectangle on close, and the relaunch restored 1000x720. Each scenario uses its own settings directory; the two geometry passes deliberately share one.
- Live screen-reader check: Orca 51.1 (Speech Dispatcher 0.12.1, espeak-ng 1.52) announced the polite scan statuses and the assertive folder error in a manual run. The AT-SPI event log showed the full text with politeness level 1 for the scan messages and level 2 for "The save folder … does not exist …". This closes the P1 status live-region check.
- The P0 work is merged on `main`; the remaining P1 items and #8/#9 are tracked in the issues above.

## Notes

- The review was performed against `main` at `aecc57a` and re-verified after the P0 merge at `9151c2a`.
- The local evidence images and the longer working report were kept outside the repository (ignored `scratch/`); this file and the linked issues are the durable record.
