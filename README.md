# JadeForge

A savegame editor for **Jade Empire** for Linux, Windows and macOS.

![JadeForge showing a savegame](docs/screenshot.png)

JadeForge reads and writes `save.sac` files directly and uses the game's own
data files to show real item, style and location names. It never needs a running
game, never installs hooks and never modifies game files: it edits the same data
the game does, verifies the result and keeps a backup of the original.

## Features

- **Character editing** – name, level, experience, gold, unspent attribute
  points, health, chi and focus (current and maximum), body/mind/spirit,
  charm/intuition/intimidate, alignment, chapter and kill counter.
- **Fighting styles** – every learned style with its localized name, type and
  level. Levels can be raised or lowered; raising a level unlocks the next
  upgrade slot so the game's own level-up accounting stays consistent. Styles
  can also be learned and removed.
- **Inventory** – every stack with its real name and category resolved from
  `items.2da` and `dialog.tlk`. Stack sizes can be changed, items removed, and
  new items can be added through a searchable database of the game's items.
- **Companions** – all party members with their level and availability, editable.
- **Savegame browser** – scans the save folder and shows the game's own
  screenshot, character name, level and play time for every savegame.
- **Safe by design** – every write is verified by re-parsing the produced file,
  written atomically through a temporary file, and guarded by a timestamped
  backup.
- **Byte exact when untouched** – opening and saving without changes produces
  the identical file, because only edited blocks are re-serialized.
- **Command line tool** for scripting, bulk inspection and debugging.

![Character tab](docs/screenshot-character.png)

## Requirements

- Rust 1.85 or newer to build from source.
- Linux: an X11 or Wayland session with OpenGL 3.3 (or the equivalent on
  macOS/Windows). The GUI is a native window, there is no browser or Electron
  layer.

## Building

```sh
cargo build --release
```

The binaries are then at:

- `target/release/jadeforge-gui` – desktop application
- `target/release/jadeforge` – command line tool

Run them directly:

```sh
./target/release/jadeforge-gui
```

or start the CLI against a save folder:

```sh
./target/release/jadeforge info "/media/Games/Heroic/Jade Empire/Save/Auto 56 - 16h 22min - level 15"
```

## Using the desktop application

1. Start `jadeforge-gui`.
2. Put the path of your Jade Empire `Save` folder into the *Save folder* field
   and press **Scan**. On Linux the GOG default
   (`/media/Games/Heroic/Jade Empire/Save`) is used if it exists, on Windows the
   GOG Galaxy default. `JADEFORGE_SAVE_DIR` overrides both.
3. Pick a savegame on the left; the game's own screenshot and character summary
   appear above the list.
4. Edit the values on the tabs. Text fields are staged until you press **Apply
   changes** or **Save to disk**; items, styles and companions are changed in
   memory immediately (the header shows "unsaved changes" and switching away is
   blocked until you save or revert).
5. Press **Save to disk**. A backup called
   `save.sac.jadeforge-backup-<unix timestamp>` is created next to the savegame;
   copy it back over `save.sac` to undo a change.

## Game data

Names are not stored in savegames, they are talk table references in 2DA tables.
JadeForge looks for the game installation next to the save folder (the parent of
`Save/`) and reads only:

| file | used for |
| --- | --- |
| `dialog.tlk` | every localized name and description |
| `override/items.2da` | item names, categories, icons |
| `override/stylesuperlist.2da` | fighting style names and weapon data |

Set `JADEFORGE_GAME_DIR` to point at an installation somewhere else. When the
files are missing the editor still works, it just shows numeric ids instead of
names. Nothing in the game folder is ever written.

## Using the command line tool

```sh
jadeforge [SAVE] <COMMAND>

Commands:
  info                      Print a summary of a savegame
  blocks                    List the blocks stored in a savegame
  character                 Print the player character
  items                     Print the player inventory (with names)
  styles                    Print the player's fighting styles
  companions                Print the player's companions
  get    <BLOCK> <PATH>     Print a field
  set    <BLOCK> <PATH> <VALUE> [--dry-run] [--no-backup]
  dump   [BLOCK] [--path]   Dump a block as JSON
  verify                    Verify that a savegame can be parsed and written
  add-item <ITEM_ID> [--kind 6] [--count 1] [--dry-run]
```

`SAVE` may be the savegame file itself or the folder that contains it.

Examples:

```sh
# Show the character of one savegame
jadeforge "/path/to/Save/Auto 56 - 16h 22min - level 15" info

# Show the learned styles and the inventory with names
jadeforge save.sac styles
jadeforge save.sac items

# Give the character gold
jadeforge save.sac set save.sav PlayerCreature.Stats.Gold 999999

# Inspect the raw structure
jadeforge save.sac dump save.sav --path PlayerCreature.Stats

# Verify a savegame without touching it
jadeforge save.sac verify
```

## Where are my savegames?

| platform | default location |
| --- | --- |
| Linux (GOG / Heroic) | `/media/Games/Heroic/Jade Empire/Save` |
| Windows (GOG) | `C:\Program Files (x86)\GOG Galaxy\Games\Jade Empire\Save` |
| macOS | `/Applications/Jade Empire/Save` |

Any folder works as long as it contains one sub-folder per savegame with a
`save.sac` inside. Point `JADEFORGE_SAVE_DIR` at it to override the default.

## How it works

The savegame format is documented in [docs/FORMAT.md](docs/FORMAT.md) and the
internals in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md). In short: a
`save.sac` is a container of named, self describing blocks; the editor decodes
the block holding the player, changes only the fields that differ, verifies the
result and writes it back.

## Development

```sh
cargo test --workspace        # unit, synthetic, real savegame and UI tests
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

Tests that need real savegames or the game data are skipped unless the files are
available:

```sh
JADEFORGE_SAVE_DIR="/path/to/Jade Empire/Save" cargo test --workspace
```

The headless UI tests render the real component tree with `freya-testing`, so
they work on CI machines without a display. The GitHub Actions definition is in
[ci/github-actions.yml](ci/github-actions.yml) (see [ci/README.md](ci/README.md)
for why it is not enabled yet).

See [AGENTS.md](AGENTS.md) for the development guidelines and
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the crate layout.

## Status

The format reader/writer is validated against 88 retail savegames (943 blocks):
every block parses, every block survives a serialize/parse round trip, and
untouched savegames are written back byte for byte.

Not implemented yet:

- editing story variables, journal entries and gem slots,
- companion stats beyond level and availability,
- item icons in the interface (names and categories are resolved).

## License

MIT
