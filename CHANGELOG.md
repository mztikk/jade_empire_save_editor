# Changelog

All notable changes are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

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
  edits preserve the saved field type.
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
  of them.
- UI layout: compact list rows with controls inside the card, a fixed 160x160
  aspect-preserving save preview, correct singular/plural count labels, and
  headless geometry tests for row pitch, control bounds, preview size and every
  tab at 900/1600 px.
- Stability: stable keys on dynamic rows, tab content and preview subtrees, plus
  a repeated tab-switch regression test, to avoid the Freya 0.4 diff panic seen
  when switching Gems -> Inventory on a large save.

### Changed

- Workspace MSRV is 1.88; the dependency graph was updated to remove the
  vulnerable `quick-xml` 0.38 path.
- CI is active for pushes, pull requests and tags, with gamedata tests, clippy,
  docs, audit, an MSRV job and a synthetic headless UI suite.
- Release artifacts include checksums, third-party notices, a CycloneDX SBOM,
  build provenance, conditional macOS/Windows signing and the Skia licence.
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
