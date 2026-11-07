# JadeForge

A savegame editor for **Jade Empire** on Linux and Windows. macOS is supported
as a self-build only.

![JadeForge showing a savegame](docs/screenshot.png)

JadeForge reads and writes `save.sac` files directly and uses the game's own
data files to show real item, style and location names. It never needs a running
game, never installs hooks and never modifies game files: it edits the same data
the game does, verifies the result and keeps a backup of the original.

## Features

- **Character editing** – name, level, experience, gold, unspent attribute
  points, health, chi and focus (current and maximum), body/mind/spirit,
  charm/intuition/intimidate, alignment (with the game's Closed Fist / Open
  Palm label), appearance, chapter and kill counter.
- **Fighting styles** – every learned style with its localized name, kind
  (martial, weapon, magic, support, transformation, improvised, unique) and
  level. Expanding a style shows its description and the actual upgrades behind
  the level: one plain golden orb per tier and family (no numbers: the left-to-right order is the tier) (damage, chi strike, speed, chi cost,
  duration, ...), each one a row of `StyleAdvance.2da` with its own cost. The
  level is read-only and follows `1 + number of owned upgrades`, exactly like the
  game stores it. Clicking an orb buys every missing tier up to it, clicking an
  owned orb removes it together with everything above it, so `1 -> 4` and
  `5 -> 0` are one click and a family is always a gap-free prefix. The orbs come
  from the style's own `<Style>LV.2da` table, read from the shipped
  `data/miniglobal-a.rim` archive through `styledata.2da`; styles without a
  table are read-only instead of guessing. All learnable styles are listed for
  browsing, with a filter for searching, and styles can be learned and removed.
- **Inventory** – every stack with its real name and category resolved from
  `items.2da` and `dialog.tlk`. Stack sizes can be changed, items removed, and
  new items can be added from the game's item database, which is browsable in
  full, filterable by category and searchable by name or template. Rows always show the same
  `category · #id` (items) and `template · #id` (styles) subtitle. Click a row
  to expand the game's own description.
- **Companions** – all party members with their level and availability, editable.
- **Gems** – amulet sockets and carried gems with their actual effect lines
  (for example `+4 Body · +4 Spirit`). A gem can be moved between the two lists
  or removed without touching its effects. The amulet's tier and socket count
  are shown. `Amulet.Type` is the tier, not the socket count: tiers 1, 2 and 4
  unlock 3, 5 and 7 sockets. Before the amulet is unlocked the tab says so,
  locked sockets are hidden, empty unlocked sockets are shown as such, and gems
  from the game data in `gems.2da` can be added to the inventory through a
  searchable list.
- **Savegame browser** – scans the save folder and shows the game's own
  screenshot, character name, level and time played for every savegame. The
  list can be filtered by folder or character name and sorted by name, level or
  time played, with natural ordering (`Auto 2` before `Auto 10`).
- **Native folder dialog** – **Browse…** opens the operating system's own folder
  dialog (XDG desktop portal on Linux, Win32 on Windows, `NSOpenPanel` on macOS),
  so the save folder never has to be typed. Cancelling the dialog and picking an
  unreadable path are reported distinctly. The choice is remembered between runs
  and no installation path is baked into the program.
- **Themes and text size** – the original dark jade theme, a light parchment
  theme, system-following mode and a high-contrast theme, shared through one
  semantic palette. Text can be scaled to 125, 150 or 200 %: rows, badges and
  the three-line status slot grow with it. Both choices are remembered and
  available on the About page.
- **Undo, review and backups** – every edit has an undo step (Ctrl+Z, Ctrl+Y),
  Save opens a pending-changes page that lists the exact fields and rows before
  the verified write, and a backup browser restores an older file with its
  timestamp and original path. A savegame that changed on disk is reported
  with a Reload action instead of being overwritten.
- **Large collections** – the save browser and the item/style/gem pickers are
  virtualized, so the full item database and a save folder with hundreds of
  entries stay responsive while only the visible rows are built.
- **Localization-ready UI** – interface strings use stable semantic ids in an
  English source catalog plus a peer German catalog in
  `crates/jadeforge-i18n/locales`, and a generated QA pseudolocale. English and
  German are complete; a future language can start as a `Deutsch (Vorschau)`-style
  preview that shows `⟦id⟧` markers until filled, with no English fallback.
  Adding a language means one TOML file and one locale variant, not touching the
  views; ids are generated as typed compile-time constants.
- **Help and diagnostics** – About shows the version, configuration and data
  folders, block count, theme, language and renderer preference, copies a
  diagnostics report, and lists every keyboard shortcut (Ctrl+/ opens it).
- **Responsive layout** – the sidebar, tab bar and overview values adapt down to
  small windows instead of clipping, and long scans show a loading state while
  the save list and editor fill in.
- **Native icon** – the hammer-and-jade mark is embedded in the window and in
  the Windows executable, staged as a macOS `.app` bundle, and installed with
  the launcher entry and hicolor icons on Linux.
- **Safe by design** – every write is verified by re-parsing the produced file,
  written atomically through an exclusive temporary file, and guarded by a
  unique backup. After a save the Overview shows the exact backup path with
  copy/open actions, and the About page explains how to restore it. The original
  permissions are kept, a savegame that changed on disk is reported instead of
  overwritten, and saving through a symlink keeps the link.
- **Byte exact when untouched** – opening and saving without changes produces
  the identical file, because only edited blocks are re-serialized.
- **Command line tool** for scripting, bulk inspection and debugging.

![Character tab](docs/screenshot-character.png)

![Fighting styles](docs/screenshot-styles.png)

## Requirements

- Rust 1.99 or newer to build from source.
- Linux: an X11 or Wayland session with OpenGL 3.3 (or the equivalent on
  Windows) and, for the native folder dialog, `xdg-desktop-portal`
  (installed by default on GNOME, KDE and most desktops). The GUI is a native
  window, there is no browser or Electron layer.
- macOS: supported as a self-build only, with no official binaries. The GUI is
  compile-checked on macOS in CI but is not exercised in a real window there.

## Building

```sh
cargo build --release
```

The binaries are then at:

- `target/release/jadeforge` – desktop application
- `target/release/jadeforge-cli` – command line tool

Run them directly:

```sh
./target/release/jadeforge
```

or start the CLI against a save folder:

```sh
./target/release/jadeforge-cli info "/path/to/Jade Empire/Save/Auto 56 - 16h 22min - level 15"
```

### Distribution builds

`just publish` builds both binaries with the `dist` profile (release
optimisation with fat LTO) and stages them in
`dist/jadeforge-<version>-<target>/` with the licence, README and the
platform's icon files.

The Linux binaries inside that folder have to be built against an older glibc
than current distributions ship, otherwise they refuse to start on SteamOS /
Steam Deck (`GLIBC_2.44 not found`). `just publish-linux` does that in a rootless
Podman container based on Ubuntu 22.04 (glibc 2.35), stages the v3 and
generic-CPU GUI and CLI binaries into `dist/` and verifies both with
`ci/check-glibc.sh`:

```sh
just publish-linux
```

It needs `podman` (`sudo apt install podman`) and the first run builds the image
in `ci/Containerfile`; later runs reuse the cached layers and the
`target/podman` build cache.

Linux and Windows x86-64 releases contain two GUI binaries and two CLI binaries:

| binary | ISA | CPUs |
| --- | --- | --- |
| `jadeforge`, `jadeforge-cli` | **x86-64-v3** (AVX2, FMA, BMI2) | Haswell (2013+), Steam Deck Zen 2 |
| `jadeforge-generic-cpu`, `jadeforge-cli-generic-cpu` | baseline | every x86-64 CPU |

The fallback is built and staged automatically, so users on pre-2013 CPUs run
the `jadeforge-generic-cpu` binary and everyone else gets the faster default.
Run it on each platform you want to ship:

| host | targets |
| --- | --- |
| Linux | the native target, and Windows with `cargo-xwin` |
| Windows | the native MSVC target |

Windows binaries normally have to be built on Windows: the Freya GUI links
prebuilt Skia libraries. Windows can be cross-built from Linux with
`cargo-xwin`; install it with
`cargo install --locked cargo-xwin@0.23.1`, then run:

```sh
just publish-windows-cross
```

The cross recipe supplies the Microsoft CRT and SDK, builds the x86-64-v3 and
baseline binaries, and stages them in `dist/`. See
[ci/README.md](ci/README.md#windows-cross-build) for the prerequisites and the
Wine smoke test. `just publish-target <triple>` builds and stages a single
target, for example `aarch64-unknown-linux-gnu` when an ARM64 cross compiler is
installed.

A self-built macOS binary (`just publish` on a Mac) stages `JadeForge.app` next
to the bare binaries; the bundle carries the `.icns` so Finder and the Dock show
the icon. That is the self-build path: there is no official macOS release.
Windows executables embed the `.ico`. Linux folders contain
`share/applications/jadeforge.desktop` and `share/icons/hicolor/...`; install
the CPU-appropriate binary as `jadeforge` in a directory on `PATH`, then copy
`share/` into `~/.local/share` so Wayland launchers resolve the icon.

The [CI workflow](.github/workflows/ci.yml) runs `just publish` on Linux and
Windows, stages third-party notices, an SBOM and (for public repositories)
build provenance, and uploads the staged folders as per-OS artifacts; pull
requests skip that job. On a `v*`
tag the `release` job zips each platform folder, writes one `SHA256SUMS`, and
creates the GitHub release from the matching changelog section. When GitHub
Actions is unavailable, `just publish-linux` and `just publish-windows-cross`
stage the same folders locally; `ci/assemble-release.sh` and
`ci/publish-release.sh` then assemble and publish the release. The complete
manual runbook is in [docs/RELEASING.md](docs/RELEASING.md). macOS is not in
the release matrix: the project has no Mac and no Apple ID, so the bundle
cannot be built, signed or notarised, and macOS stays a self-build only. The
published binaries are unsigned for now, so Windows SmartScreen warns about an
unknown publisher.

The `just` shortcuts are optional; the cargo commands above remain the
canonical ones. `just check` runs the formatting, clippy and test gate,
`just setup`/`just start`/`just finish` manage task branches, and `just
coverage` writes a line/function summary plus HTML and lcov reports to
`coverage/`.

## Using the desktop application

1. Start `jadeforge`.
2. Pick your Jade Empire `Save` folder with **Browse…** (the native dialog, or
   type the path) and
   press **Scan**. JadeForge has no built-in guess where the game is installed:
   GOG, Steam, Heroic, Wine and manual installs all differ, so the folder is
   chosen once and remembered in the user configuration, a small TOML file
   (`$XDG_CONFIG_HOME/jadeforge/config`, `%APPDATA%\jadeforge\config` or
   `~/Library/Application Support/jadeforge/config`). `JADEFORGE_SAVE_DIR`
   overrides the folder for one run and `JADEFORGE_CONFIG_DIR` moves the
   settings file (useful for portable installs).
3. Pick a savegame on the left; the game's own screenshot and character summary
   appear above the list. The list can be filtered by folder or character name
   and sorted by name, level or time played, and that choice (like the save
   folder) is remembered for the next start. The **About** button shows the
   version, the settings file, the remembered folder/sort order and the backup
   rules.
4. Edit the values on the tabs. Every change (text fields, items, styles,
   gems and companions) is applied to the in-memory savegame immediately; the
   header shows "unsaved changes" and switching away is blocked until you save
   or revert. A field with invalid text is reported and not applied until it is
   fixed.
5. Press **Save to disk**. The **Review changes** page opens with the exact
   fields and rows that will be written; press **Write savegame** to perform the
   verified write. A backup called
   `save.sac.jadeforge-backup-<seconds>.<nanos>.<counter>` is created next to
   the savegame; the Overview then shows its exact path and a button that opens
   the folder. Copy it back over `save.sac` to undo a change. Repeated saves
   never overwrite an earlier backup. The Save and Revert buttons are disabled
   while there is nothing to write or discard, and the last success stays in the
   status area.

## Memory use

JadeForge renders with Freya and Skia. By default Freya picks the GPU backend
(Vulkan on Linux, Metal on macOS, OpenGL elsewhere). Two things keep that
reasonable for a save editor: the savegame list is virtualized, so a folder with
hundreds of saves costs the same to render as one with ten, and the Skia GPU
resource cache is capped at 32 MiB instead of Skia's 256 MiB default. The
savegame's unsaved-changes baseline is stored as a 128 bit fingerprint per
block instead of a second copy of the block bytes.

When memory matters more than GPU acceleration (small VMs, remote desktops,
integrated graphics), the whole interface can be rendered on the CPU:

```sh
JADEFORGE_RENDERER=software jadeforge
```

The software renderer is still Skia, just without a GPU context and driver.
The editor is mostly static, so the trade is cheap: a full 1280x800 frame takes
a few milliseconds here and an idle window uses no measurable CPU.
`JADEFORGE_RENDERER` accepts `auto`, `software`, `opengl` and `vulkan`
(case-insensitive), and wins over Freya's own `FREYA_RENDERER`. Unset it (or use
`auto`) to return to the default renderer for later runs.

The backends are Freya's, not reimplemented here. A backend that cannot
initialize falls back instead of preventing the editor from starting, and the
one the window actually got is printed at startup, for example
`jadeforge: renderer OpenGL (AMD Radeon RX 9070 XT)`. Run the editor from a
terminal or check the journal to see it.

| platform | `auto` picks | explicit choices |
| --- | --- | --- |
| Linux | Vulkan, then OpenGL, then software | `vulkan`, `opengl`, `software` |
| Windows | OpenGL | `vulkan` (falls back to OpenGL, then software), `opengl`, `software` |
| macOS | Metal | `software`; `opengl` and `vulkan` still resolve to Metal, because Freya only implements Metal there |

Direct3D is not available. Skia itself can render through D3D12, but Freya 0.5
has neither a Direct3D driver nor a renderer preference for it, so JadeForge has
nothing to select. On Windows, `JADEFORGE_RENDERER=vulkan` uses the newer
graphics API; Direct3D support would have to come from an upstream Freya driver.

For scale: a live 1000x720 window with an opened savegame peaks at about 52 MiB
with `JADEFORGE_RENDERER=software`. The GPU path adds the graphics driver and
Skia's GPU resources on top of that, which is what puts the same window into the
100-120 MiB range.

## Game data

Names are not stored in savegames, they are talk table references in 2DA tables.
JadeForge looks for the game installation next to the save folder (the parent of
`Save/`, or `Save/<savegame>` when a savegame folder is selected). The
`JADEFORGE_GAME_DIR` override wins; if it is unset or does not point at an
installation, the normal layout is used. It reads these files:

| file | used for |
| --- | --- |
| `dialog.tlk` / `dialogf.tlk` | every localized name and description |
| `items.2da` | item names and categories; icon references are parsed but not rendered |
| `stylesuperlist.2da` | fighting style names and weapon data |
| `StyleAdvance.2da` | upgrade rows behind the style tiers |
| `styledata.2da` | the style to `<Style>LV.2da` mapping |
| `gems.2da` | predefined gems |
| `effects.2da` | effect names |
| `<Style>LV.2da` | the purchasable upgrade slots of one style |

The 2DA tables come from `override/` first and from the shipped RIM archives
second. Most tables ship as binary `2DA V2.b` resources inside
`data/miniglobal-a.rim`; the style animation tables are in
`data/globalunload-a.rim`, and the editor reads both archives (for example
`styledata.2da`, which maps every style to its own level-up table).
Files in `override/` take precedence over the archive exactly like the engine
does, so **mods that add styles or change tables are picked up**: extend
`stylesuperlist.2da`, add a `styledata.2da` row pointing at the new
`<Style>LV.2da`, and the editor resolves the modded upgrades like the shipped
ones. Override files are read case-insensitively and tolerate non-UTF-8 text.

Set `JADEFORGE_GAME_DIR` to point at an installation somewhere else. When the
files are missing the editor still works: a bundled English index supplies item
and style names and item categories, while a style's level, weapons, icon
reference and time spent still come from the savegame. Descriptions, upgrade
tables and effect names still need the installation. Nothing in the game folder
is ever written. The index, its scope and its regeneration command are documented
in [docs/NAMES.md](docs/NAMES.md).

## Using the command line tool

```sh
jadeforge-cli [SAVE] <COMMAND>

Commands:
  info                      Print a summary of a savegame
  blocks                    List the blocks stored in a savegame
  character                 Print the player character
  items                     Print the player inventory (with names)
  styles                    Print the player's fighting styles
  companions                Print the player's companions
  get    <BLOCK> <PATH>     Print a field
  set    <BLOCK> <PATH> <VALUE> [--dry-run] [--no-backup]
  dump   [BLOCK] [--path] [--json]   Dump a block as JSON
  verify                    Verify that a savegame can be parsed and written
  add-item <ITEM_ID> [--kind 6] [--count 1] [--dry-run] [--no-backup]
  export-names --game <DIR> --output <FILE>
                            Regenerate the bundled English name index
```

`SAVE` may be the savegame file itself or the folder that contains it.
`export-names` is a maintainer command and does not read a savegame. `dump`
always emits JSON; `--json` is accepted for scripts that spell it out.
`set` is the raw debugging escape hatch: unlike the desktop editor it does not
range check known character fields.

Examples:

```sh
# Show the character of one savegame
jadeforge-cli "/path/to/Save/Auto 56 - 16h 22min - level 15" info

# Show the learned styles and the inventory with names
jadeforge-cli save.sac styles
jadeforge-cli save.sac items

# Give the character gold
jadeforge-cli save.sac set save.sav PlayerCreature.Stats.Gold 999999

# Inspect the raw structure
jadeforge-cli save.sac dump save.sav --path PlayerCreature.Stats

# Verify a savegame without touching it
jadeforge-cli save.sac verify
```

## Where are my savegames?

Jade Empire keeps its savegames in a folder called `Save` next to the game
executable (`dialog.tlk`, `override/`). Where that folder is depends on how the
game was installed, so JadeForge asks for it once instead of guessing:

- GOG: the folder the GOG installer created, usually below `Galaxy/Games` or
  your library drive.
- Steam or Heroic: the prefix/library folder you installed it into.
- Wine/Proton: the prefix's `drive_c` path.

Any folder works as long as it contains one sub-folder per savegame with a
`save.sac` inside. Use **Browse…** in the application (the system's folder
dialog), or set `JADEFORGE_SAVE_DIR` for scripts and CI.

## How it works

The savegame format is documented in [docs/FORMAT.md](docs/FORMAT.md) and the
internals in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md). In short: a
`save.sac` is a container of named, self describing blocks; the editor decodes
the block holding the player, changes only the fields that differ, verifies the
result and writes it back.

## Development

Work happens on task branches; `main` only receives merged branches. Install the
tracked hooks once per clone and start every task from the latest `origin/main`:

```sh
just setup
just start fix/save-load    # <type>/<slug>
# ... work ...
just finish                 # fails on main, a dirty tree or no commits
```

```sh
cargo install cargo-nextest --locked   # once; the test runner
cargo nextest run --workspace          # unit, synthetic, real savegame and headless UI tests
cargo test --workspace --doc           # doctests, which nextest does not run
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
JADEFORGE_LIVE_APPROVED=1 just live    # windowed UI tests; ask first (rule 13)
```

A branch is ready for review only when the worktree is clean and `just finish`
passes; see [CONTRIBUTING.md](CONTRIBUTING.md) for the full checklist.

To install the GUI and CLI binaries into `~/.cargo/bin` (which must be on
`PATH`), tuned for the current machine's CPU:

```sh
just install
```

This uses `-C target-cpu=native`, so the resulting binaries can use every CPU
feature the build host has; they are not portable to other machines. Use
`just publish` / `just publish-linux` for distribution binaries. Skia is still
the prebuilt library, so only the Rust code gets native code generation.

On Linux, `just install-desktop` installs everything in one step: the GUI and
CLI binaries, the desktop entry and the hicolor icons. The launcher and Wayland
compositors can then resolve the icon by app id:

```sh
just install-desktop
```

The launcher entry expects `jadeforge` on `PATH` (the location `just install`
uses). The icon master lives in `crates/jadeforge-gui/assets/icon.svg`;
`just icons` regenerates the PNG, ICO and ICNS files from it (needs
`rsvg-convert`, ImageMagick and Python 3).

Tests that need real savegames or the game data are skipped unless the files are
available:

```sh
JADEFORGE_SAVE_DIR="/path/to/Jade Empire/Save" cargo nextest run --workspace
```

The headless UI tests render the real component tree with `freya-testing` and
build a synthetic savegame plus installation when no game data is present, so
they work on CI machines without a display or the retail game. Render a tab
without a window with the snapshot example:

```sh
cargo run -p jadeforge-gui --example snapshot -- "<SAVE_DIR>" docs/screenshot.png all
# --width 900 / --height 1800 for narrow or tall layouts
# --theme dark|light|high-contrast and --scale 100|125|150|200 for variants
# --query viper types into the active tab's own filter (Items, Styles or Gems)
```

The windowed suite is the second feedback loop: it launches the real application
through `freya-winit`, opens a real Wayland window and drives it with real
compositor input on KDE (or with events through the windowed plugin handle on
other compositors). It is opt-in with `JADEFORGE_LIVE_APPROVED=1 just live`, so
normal test runs never open windows. Run it from a real Wayland desktop session, or from a VM with a real
desktop; it is not part of CI. **Never run it without explicit permission**
([AGENTS.md](AGENTS.md) rule 13), and only when a headless test cannot cover the
case or the bug is in the real window. See
[docs/LIVE-UI.md](docs/LIVE-UI.md) for the backends, the process model and how to
add a scenario. The active workflow is
[`.github/workflows/ci.yml`](.github/workflows/ci.yml); see
[ci/README.md](ci/README.md) for its jobs and the release signing checklist.

See [AGENTS.md](AGENTS.md) for the development guidelines,
[CONTRIBUTING.md](CONTRIBUTING.md) for the review checklist,
[SECURITY.md](SECURITY.md) for vulnerability reporting and
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the crate layout. User-visible
changes are listed in [CHANGELOG.md](CHANGELOG.md).

## UI review and follow-ups

The desktop UI review and its evidence live in
[docs/UI-REVIEW.md](docs/UI-REVIEW.md); the P3 design notes and their progress
table are in [docs/ui-p3/](docs/ui-p3/README.md).

P0 ([#5](https://github.com/mztikk/jade_empire_save_editor/pull/5)),
P1 ([#7](https://github.com/mztikk/jade_empire_save_editor/issues/7)) and
P2 ([#8](https://github.com/mztikk/jade_empire_save_editor/issues/8)) are
complete. The P3 work shipped in
[v0.3.0](https://github.com/mztikk/jade_empire_save_editor/releases/tag/v0.3.0)
([PR #32](https://github.com/mztikk/jade_empire_save_editor/pull/32)) except for
the blocked item-icon item
([#9](https://github.com/mztikk/jade_empire_save_editor/issues/9)). The remaining
open follow-ups are the platform contrast preference
([#33](https://github.com/mztikk/jade_empire_save_editor/issues/33)) and the
upstream `Input` label association
([#3](https://github.com/mztikk/jade_empire_save_editor/issues/3)).

## Status

The format reader/writer is validated against 89 retail savegames (962 blocks):
every block parses, every block survives a serialize/parse round trip, and
untouched savegames are written back byte for byte.

Not implemented yet:

- editing story variables and journal entries,
- companion stats beyond level and availability,
- item icons in the interface. The icon references resolve, but the image data
  lives in compressed engine BIF/KEY archives rather than the RIM tables the
  editor reads; the evidence and the required format work are recorded in
  [docs/ui-p3/icons.md](docs/ui-p3/icons.md). No placeholder image is shipped.

## License

The JadeForge source code is MIT licensed. The bundled English item and style
names are game content and are not covered by that license; see
[NOTICE](NOTICE) and [docs/NAMES.md](docs/NAMES.md).
