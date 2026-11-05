# Changelog

All notable changes are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

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
  build provenance and the Skia licence.
- Release CI builds Linux and Windows only: macOS is out of the publish matrix
  until its bundle can be signed and notarised, and the published binaries are
  unsigned for now.
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
