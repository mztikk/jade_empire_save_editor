# Changelog

All notable changes are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Fixed

- Text scaling above 100 percent now keeps the shell usable at 900x700 and the
  default window sizes. The header and tab strip compact, the sidebar keeps its
  save rows instead of its decorative preview, form fields reflow by available
  width, style and item rows stack their actions at 200 percent, and picker
  viewports cap themselves at the window height. A regression matrix covers
  900x700, 1180x760 and 1280x800 at 100/125/150/200 percent in English, plus
  German at 125/150/200 percent.
- Freya input, select and menu text now follows the application type scale,
  input padding and radius scale with it, and page and toolbar actions share one
  ActionButton with filled, outline and flat variants plus a visibly disabled
  state. The save sort control collapses to one row above 100 percent.
- The virtualized pickers and the save browser are now one ListBox tab stop with
  a stable announced active row. Arrow, Page Up/Page Down and Home/End move the
  active row and scroll it into view; Enter activates items, styles, gems and
  saves; Space toggles a picker description. The carried-gem list is reachable
  and scrollable by keyboard.

## [0.4.0] - 2026-11-07

### Added

- Headless accessibility regressions for the Freya 0.5 follow-up: dynamic save
  count, status and empty-state updates, plus AccessKit focus and activation of
  tabs, save rows, footer buttons and row actions. A manual screen-reader
  checklist covers the audible Orca, NVDA and VoiceOver pass.

### Fixed

- Translation filling no longer rescans inserted values, so placeholder-like
  user text survives unchanged. The alignment and style-level limits are now
  documented consistently, the startup and scan state machine in the
  architecture notes matches the code, and a remembered window position selects
  the monitor that contains it instead of always using the primary one.
- Disabled tabs keep their accessible names, keyboard-focused upgrade orbs
  expose the same effect, cost and action detail as pointer hover, and a dirty
  backup browser shows a disabled, non-focusable Restore control instead of an
  action that the session would refuse.
- Rescanning a folder that lost its last readable savegame no longer crashes the
  save list. The filtered indices are derived from the same entry snapshot the
  rows render, so a stale filter can no longer index an entry list that already
  became empty.
- GUI render work now follows the active tab: inactive item and style searches
  are skipped, derived character/form/picker data is shared instead of cloned,
  the save list keeps one stable open action per document, the gems picker
  reuses a memoized row model, and category chips have stable keys. Character
  auto-apply also reuses one canonical serialization for dirty and history.
- The direct item stack and companion level inputs now keep their typed draft
  across hover and revision renders, sync from committed stepper changes,
  normalize after a successful submit, and keep the row context for assistive
  technology.
- Game data loading now reports corrupt talk tables and RIM archives instead of
  treating them as empty installations. Talk table sections and entry ranges,
  RIM payload ranges, and 2DA row widths are validated; text rows name their
  line on failure, and binary cells are interned by pool offset under a decoded
  byte budget.
- Session undo and redo now restore the whole document or nothing, a failed
  reload after a backup restore clears the stale document and reports the
  failure, companion availability diffs show the real before and after values,
  late game data rebuilds rows before they are shown, and an evicted undo
  history is disclosed on the review screen instead of looking complete.
- Chapter edits now create a missing `Campaign.Chapter` field. Learning a style
  from a sparse template now adds the missing identity fields and clears the
  copied `ResistList`, and typed item, companion and style setters reject struct
  indices that are not members of their lists.
- GFF parsing now validates shared list offsets once under a total work bound,
  moves single-use field values instead of cloning them, and saves reuse the
  verified parse instead of parsing the container a second time. Writer payload
  de-duplication compares hashed offsets byte for byte without cloning every
  candidate key.
- GFF parsing now rejects duplicate label names, multiple root structs, and
  localized strings with trailing bytes. A bounded baseline keeps compact,
  de-duplicated writer output parseable without removing the decoded-data guard.
- Save, backup, restore and backup listing now work with bare relative paths
  and symlinked savegames. Backups stay next to the visible savegame path, and
  restoring through a symlink writes its target while keeping the link.
- Backup restore now works over a corrupt current savegame, including from the
  backup browser after selecting an unreadable save entry. It writes the exact
  bytes it verified, preserves temp-file permissions before writing, stamps the
  source from the open handle, and publishes backups without replacing an
  existing name.
- A scan that finishes after the open savegame becomes dirty no longer replaces
  the folder or clears the document. The result is blocked with the unsaved
  changes message until the user saves or reverts; same-folder refreshes still
  apply when the open entry remains.

## [0.3.1] - 2026-11-07

### Fixed

- The Windows GUI executable now links as a GUI-subsystem binary, so opening it
  no longer creates a console window next to the editor. Renderer diagnostics
  are best effort, so the process does not panic when the GUI subsystem gives it
  no stderr handle; redirect stderr to a file to read them.

## [0.3.0] - 2026-11-07

### Added

- P3 strategic desktop UI: light, dark, system-following and high-contrast
  themes from one semantic palette, plus a 100-200 % text scale that grows the
  type, spacing and fixed row/status dimensions without clipping. Both
  preferences persist in the settings file and are selectable on the About
  page; the palette tests cover contrast for every theme.
- A bounded per-action undo/redo history (Ctrl+Z, Ctrl+Y, footer buttons),
  including structural item/style/gem edits. Character typing coalesces into
  one step per 1.5 s burst, undoing every edit clears the dirty state, and a
  save clears the history.
- Save opens a pending-changes review page that lists every field and row with
  its before/after value; only the explicit Write savegame button performs the
  verified write and backup.
- A first-class backup browser with UTC timestamps, the original path, size and
  a verified restore that keeps a safety backup of the file it replaces.
- External-change detection with an assertive explanation banner and an
  explicit Reload from disk path (with discard confirmation when edits are
  unsaved).
- The item, style and gem pickers are virtualized: the full filtered list is
  browsable, only visible rows are built, and rows keep stable ids, accessible
  names and their description expansion.
- A dedicated `jadeforge-i18n` crate with stable semantic ids, a complete
  English source catalog (`crates/jadeforge-i18n/locales/en.toml`) and a complete
  German peer catalog (`de.toml`), plus a generated QA pseudolocale. About shows
  **Deutsch**; the preview mechanism (`complete = false`, `Deutsch (Vorschau)`,
  `⟦id⟧` markers) remains for a future language and never falls back to
  English. Key parity, placeholder parity, English/German completeness and the
  typed-key checks are enforced by tests. The session calls the shared catalog
  directly through `message::tr`/`message::fill`; the GUI re-exports the catalog
  API. The consolidation renamed sentence-slug ids, merged repeated wrappers and
  reused the six shared field ids for the Overview stat lists, leaving 472 ids.
- About/help now shows version, savegame block count and size, theme, scale,
  language, renderer override and backup folder, with a Copy diagnostics
  action and a complete keyboard shortcut sheet (Ctrl+/ opens it). Window size
  and position remembering can be turned off.
- Responsive editor composition: the six identity fields are a fluid 3+3
  grid at the default 1180x760 window and at the 900 minimum, the overview
  glance cells reflow from one column to 2+2+1 and the hero stats stay a 2x2
  block, the editor column stops at 1100 px on wide windows, descriptions
  expand in full, and clipped save, item and companion names get an ellipsis
  plus a tooltip. The overview save path wraps and can be copied.
- DPI-aware window geometry: the default size is clamped to the monitor, the
  minimum follows a short screen, and the size and position are remembered
  between runs (Wayland reports no position, so the stored one is kept).
- Per-field character validation: every field shows its allowed range, an
  invalid input is marked and reported, valid fields of an otherwise invalid
  form are applied, and Save focuses the first invalid field while still
  writing every applied item, style or gem edit.
- Unsaved state is a content comparison, so an edit that is undone clears the
  dirty flag again; Scan keeps the open savegame when the folder did not
  change; a missing, unreadable, empty and file path are reported separately;
  only a successful scan replaces the remembered folder.
- Every scan (startup, the Scan button, Enter on the folder field, F5 and
  Browse) runs on a worker thread with an in-flight status and a Cancel
  action. Cancelled, failed and superseded scans keep the entries and open
  savegame on screen, discard stale results through a scan token, and persist
  nothing; the game data is read on the same worker and the first frame shows
  a loading state instead of waiting for it.
- The missing game installation can be chosen from the header, is remembered
  in the settings file, and the discovered root is shown; sections that could
  not be read are no longer rendered as empty lists.
- Discoverability and editing: disclosure chevrons and Details affordances on
  expandable rows, upgrade-orb effect/cost/owned details, a read-only notice
  for styles without a slot table, gem effect lines in the picker, contextual
  accessible names for row actions, direct numeric stack and companion-level
  editing, desktop shortcuts (Ctrl+S/O/F, F5, Enter, Escape, tab arrows) and
  the character name and modification time in save rows.
- Accessibility and diagnostics: 28 px targets, 11 px micro and 14 px body
  type, 3:1 control borders, a non-colour active-tab underline, an explicit
  success glyph, focus restoration after removing a row, a focusable keyboard
  scroll view, frame-time instrumentation and an 89-save render test.
- A generated English name index for items and styles, plus item categories,
  used when no game installation is available. It keeps downloaded and console
  saves readable on machines without the game and enables the item picker;
  installed data always wins.
- `jadeforge-cli export-names` regenerates the bundled index from an English
  installation. The index is documented in `docs/NAMES.md`.
- Without the game installation, item rows keep their category from the
  savegame's item `Type`, and style rows show the level, weapon count, icon and
  weapon resource names and the time spent from the style struct instead of
  only numeric ids.
- P2 desktop-UI polish: the successful-save backup path is surfaced on the
  Overview with copy/open actions and restore instructions, the close guard
  offers explicit Save / Discard / Cancel, Save and Revert disable themselves
  when there is nothing to do, and folder-dialog cancellation is reported
  separately from an unreadable path.
- An About/help page with the application version, settings file, remembered
  save folder and sort order, backup rules and keyboard shortcuts; a category
  filter for the 291-row item picker; field ranges and units (plus the optional
  Gold field rule) in the character form.
- The About entry point is an outlined toolbar button instead of plain text,
  and opening About clears the tab selection so no editor tab appears active
  while the page is shown.

### Changed

- Windows release binaries can be cross-built and staged on Linux with
  `just publish-windows-cross`. cargo-xwin supplies the Microsoft CRT and SDK,
  so the prebuilt Windows Skia libraries link without a Windows host; Wine can
  smoke-test the result. See `docs/RELEASING.md` for the complete local release
  workflow.
- The no-game-data rule now names the generated English name index as its only
  exception and forbids growing it beyond item/style names and item categories.
- The windowed UI suite now requires `JADEFORGE_LIVE_APPROVED=1` in addition to
  explicit user permission. Plain `just live` and direct `JADEFORGE_LIVE=1`
  runs are refused before a window is created.
- A repository rule now bans putting backticked commands or prose into a
  double-quoted shell argument, after a PR-body quoting mistake executed
  commands.
- P2 copy and vocabulary: counts carry their noun through the shared `plural()`
  helper, the style picker heading is `STYLES TO LEARN`, `Lv`/`Played`/`Unlock`/
  `predefined gems` use the same words as the rest of the UI, raw item
  categories and style kinds map to friendly labels, and the save filter
  placeholder names the character name it also matches.
- The Overview no longer duplicates the sidebar savegame screenshot; the
  sidebar preview aligns with the identity block at the shared card radius; the
  glance values are full-width rows instead of fixed cells; the save folder can
  be opened from the Overview and About pages.
- Initial scans show sidebar and editor skeletons, manual scans show a reserved
  busy indicator in the tab bar, and the snapshot example's `--query` targets
  the active tab's own filter instead of the sidebar save filter.
- `docs/UI-REVIEW.md` records the P2 results and documents which screenshots are
  tracked and how to regenerate them; README and CONTRIBUTING link the
  P1/P2/P3 issues.
- `just install` now installs `jadeforge-cli` alongside the `jadeforge` GUI
  binary, so a source install provides both tools.
- The window-geometry setting is now explicit about Wayland: the size is
  remembered everywhere, while the position is restored only on X11 because
  Wayland compositors place windows themselves. The About page shows the
  compositor hint and points at window rules for a fixed position.

### Fixed

- Expanding an item or style in the add/learn picker now grows that row and
  shows the description inline, like the owned inventory and style rows,
  instead of appending a panel below the whole list. The virtual picker
  measures the laid-out text and uses a dynamic item size, so the full corpus
  stays virtualized.
- A `System` theme choice now follows a dark KDE or GNOME desktop on Linux.
  winit reports no desktop theme there, so the editor reads the freedesktop
  appearance portal (and KDE's `kdeglobals` for the first frame) instead of
  falling back to light, and keeps following portal changes while the window
  is open.

## [0.2.1] - 2026-11-06

### Added

- Repository rules for secrets, unnecessary comments, lint suppressions, the
  named GUI design tokens, and the no-em-dash convention.
- The upstream Freya framework skill at `.dsh/skills/freya/SKILL.md`, adapted
  for the pinned Freya 0.5.0 and linked from `AGENTS.md`.
- Screen-reader activation: Freya 0.5 dispatches AccessKit `Click` actions to
  `on_press` handlers, and a headless test activates a tab through
  `TestingRunner::send_accessibility_action`.

### Changed

- Upgraded the desktop UI from Freya 0.4.3 to Freya 0.5.0, including the
  `freya-testing` headless runner and the Linux-only `freya-core` /
  `freya-winit` windowed-test pins. The direct `accesskit` dependency now
  follows Freya's 0.25 line.
- Adopted the 0.5 accessibility work: labels and paragraphs fill their AccessKit
  value while the tree is built and accessibility nodes refresh from their diff,
  so `widgets::a11y_text` no longer builds value/label nodes or keys by content.
- Adopted the 0.5 scrollbar theme: the two scroll views keep the 9 px,
  always-visible bar with the new `thumb_cross_size`, opacity and gap fields and
  `auto_hide_scrollbars(false)`.
- Adopted per-element cursor styling (`.cursor(CursorIcon::Pointer)`) instead of
  the removed global `Cursor::set` calls, with a headless regression test.
- Ported the 0.5 breaking changes the GUI used: `KeyExt::key_mut` and the
  windowed probe's `NativePlatformErasedEventAction` close event. Audited
  `Platform`/`PlatformWindow`, the scroll controller callback form,
  `CornerRadius`/`Alignment`, the children API and the Card/Tooltip/Input
  themes; none needed a change.
- GUI font sizes, spacing, padding, corner radii, stroke widths and fixed
  component dimensions now use shared named tokens in `theme.rs` instead of
  literal values in the views.
- The savegame and overview placeholders use readable text instead of an em
  dash, and interactive controls use `on_press` so keyboard activation stays
  available.
- Workspace MSRV is 1.99 (was 1.88), matching Freya 0.5.0's pinned toolchain.
  Prebuilt binaries are unaffected; source builds need Rust 1.99 or newer.
  Updated `Cargo.toml`, `README.md`, `AGENTS.md`, `ci/README.md` and the CI MSRV
  job together.

## [0.2.0] - 2026-11-05

### Added

- Application icon: a steel hammer striking a jade gem on the dark tile. The
  icon is embedded as the window icon and committed as SVG/PNG/ICO/ICNS assets.
  Windows executables embed the ICO, macOS builds stage a `JadeForge.app`
  bundle with the ICNS, and Linux releases include the desktop entry and
  hicolor icons for Wayland launchers.
- Automated windowed UI tests (`just live`) that launch the real Freya window
  through `freya-winit`, drive it with KWin's Wayland fake-input protocol and
  capture the presented window. Other compositors use windowed plugin events.
  The suite is opt-in, runs in a real Wayland desktop session (or a desktop VM)
  and is not part of CI; the existing `freya-testing` suite stays the fast,
  exhaustive feedback loop. See [docs/LIVE-UI.md](docs/LIVE-UI.md).
- A durable [UI review status](docs/UI-REVIEW.md) recording the completed P0
  release blockers and linking the tracked P1/P2/P3 follow-up work and the
  upstream accessibility blockers.

### Fixed

- Gem moves now preserve the fixed seven-entry amulet socket list, keep gems
  visible after a move and reject full or missing amulets.
- Empty amulet sockets no longer resolve item id 0 to a generic item in modded
  item tables, which gave empty slots a name and a Remove action.
- Save writes use exclusive temporary files, unique backups, preserved
  permissions, parent-directory fsync and external-change detection; saving
  through a symlink keeps the link.
- GFF, 2DA, RIM, TLK, TGA and JSON parsing now bound counts, references,
  dimensions and decoded data, so hostile files return errors instead of
  aborting.
- Character reads reject wider or negative values instead of truncating, and
  player/campaign edits are committed only after every write succeeds.
- Missing optional character fields (notably gold) are created on edit; numeric
  edits preserve the saved field type. Saves that already lack a gold field no
  longer open with a false "invalid character input" badge, and the form keeps
  the field absent until a value is entered.
- Revert and Remove item/style/gem ask before discarding or deleting, and the
  last fighting style can no longer be removed because JadeForge needs an
  existing style as a template for learning another.
- Style upgrades use the style's real slot table and a gap-free family prefix;
  new styles preserve field types and caller identity.
- Session scan results are tied to their directory, stale scans are discarded,
  same-save clicks keep unsaved edits, and invalid form values are preserved.
- Relative save paths resolve game data, `dialogf.tlk` is a fallback for a
  corrupt `dialog.tlk`, both RIM archives are searched and partial `Choice_*`
  columns are supported.
- The close button asks before discarding unsaved changes.
- Accessibility: raised muted-text contrast, accessible names for tabs, sort
  controls, savegame rows and item/style rows, and keyboard activation for all
  of them. Static page text now has AccessKit values, card/section titles and
  empty-state headings are headings, the status line is a live region, tabs
  expose selected state, and the save list is a single Tab stop with list-box
  roles, selected options and arrow-key navigation. Expanded rows and upgrade
  orbs expose their state, and headless regression tests cover the tree.
- UI layout: compact list rows with controls inside the card, a fixed 120x120
  aspect-preserving save preview, correct singular/plural count labels, a
  900x700 minimum window, and headless geometry tests for row pitch, control
  bounds, preview size and every tab at 900/1600 px.
- Stability: stable keys on dynamic rows, tab content and preview subtrees, plus
  a repeated tab-switch regression test, to avoid the Freya 0.4 diff panic seen
  when switching Gems -> Inventory on a large save.

### Changed

- The UI accent is jade green instead of gold, matching the editor's name, the
  jade gem in the icon and the game's setting. The palette tests now enforce the
  jade hue and WCAG AA contrast for accent text and for text on accent surfaces.
  The style upgrade orbs keep their golden, in-game look.
- The supported minimum window is 900x700 (was 900x600), and the sidebar
  preview is 120x120 (was 160x160), so the save browser keeps several rows
  visible at the smallest size.
- `just install-desktop` now depends on `just install`, so a Linux install is
  one command for the binary, launcher entry and hicolor icons.
- Workspace MSRV is 1.88; the dependency graph was updated to remove the
  vulnerable `quick-xml` 0.38 path.
- CI is active for pushes, pull requests and tags, with gamedata tests, clippy,
  docs, audit, an MSRV job and a synthetic headless UI suite.
- Release artifacts include checksums, third-party notices, a CycloneDX SBOM,
  build provenance on public repositories and the Skia licence.
- Release CI builds Linux and Windows only: macOS is out of the publish matrix
  until its bundle can be signed and notarised, and the published binaries are
  unsigned for now.
- Tag builds now assemble one zip per platform, a single `SHA256SUMS` and a
  GitHub release from the matching changelog section, instead of every build
  job attaching its staged files itself.
- Split the GUI shell into `app/mod.rs`, `app/shell.rs`, `app/sidebar.rs` and
  `app/navigation.rs`; split the session row model and character form into
  `session/model.rs` and `session/form.rs`. Session state is private behind
  read-only accessors, and UI actions return the typed `SessionError` /
  `FormError` instead of bare strings.
- Added deterministic mutation tests, a dependency-free benchmark and
  `cargo-fuzz` targets.

## [0.1.0]

Initial repository snapshot: GFF/container reader and writer, character, inventory,
styles, gems, companions and screenshot support, Freya desktop UI, CLI,
game-data resolution and documentation.
