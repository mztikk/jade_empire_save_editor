# Jade Empire savegame format

This document describes the format of `save.sac` as implemented by
`jadeforge-core`, plus the game data files the editor reads for names, styles
and gems. Everything below was derived from retail savegames and the shipped
data files; it is the reference that the parser, the writer and the test suite
are checked against. Facts taken from the community wiki are linked in
[References](#references).

All integers are little endian. The format is a variant of BioWare's *generic
file format* (GFF) that Jade Empire shares with its other Aurora engine games,
with three notable differences: 16 byte fixed-size labels, a struct table that
indirects through a separate field-index array, and the single-field struct
quirk described below.

## Container

```
u32  blockCount                      number of blocks that follow
repeat blockCount times:
    u32  nameLength                  includes the trailing NUL
    u8   name[nameLength]            e.g. "save.sav\0"
    u32  blockSize
    u8   block[blockSize]            starts with "SAV "
```

The block order is significant to the game (the last block is always
`save.sav` in all 89 inspected savegames). JadeForge preserves the order.

Real examples found in savegames:

| block | content |
| --- | --- |
| `J01_BEACH.sav` … | serialized state of one area (creatures, placeables, triggers, ...) |
| `polyplayer.sav` | character sheet; only present in some saves (2 of 89) |
| `save.sav` | global campaign state and the live player creature |

## Block

```
0x00  char[4] "SAV "
0x04  char[4] version, "V3.2" in every known file
0x08  u32 structOffset            always 56
0x0c  u32 structCount
0x10  u32 fieldOffset
0x14  u32 fieldCount
0x18  u32 labelOffset
0x1c  u32 labelCount
0x20  u32 fieldDataOffset
0x24  u32 fieldDataSize
0x28  u32 fieldIndicesOffset
0x2c  u32 fieldIndicesSize
0x30  u32 listIndicesOffset
0x34  u32 listIndicesSize
0x38  struct table   (structCount      * 12 bytes)
      field table    (fieldCount       * 12 bytes)
      label table    (labelCount       * 16 bytes)
      field data     (fieldDataSize    bytes)
      field indices  (fieldIndicesSize bytes)
      list indices   (listIndicesSize  bytes)
```

`structOffset` is always 56 and `listIndicesOffset + listIndicesSize` is always
the size of the block; the parser enforces both. The tables are contiguous and
in the order above in every file that has been inspected; the reader uses the
offsets from the header and does not rely on that.

## Struct table

One entry per struct:

```
u32 structType     0xFFFFFFFF for the root, 0 in every struct inspected so far
u32 fieldReference
u32 fieldCount
```

The root struct is the one with type `0xFFFFFFFF`; in all known files it is
struct 0. The reader does not require a particular value for the other structs.

`fieldReference` has two meanings:

* `fieldCount == 0`: unused, zero.
* `fieldCount == 1`: the entry **is** the index of the single field. This is an
  optimisation that the game's writer performs, and the reason why a naive
  reader produces garbage for small structs.
* `fieldCount >= 2`: a **byte offset** into the field-index section. The
  `fieldCount` 32 bit field indices that follow describe the struct's fields.

The field-index section is exactly filled by all structs with two or more
fields: their ranges tile the section without gaps or overlaps. This invariant
is checked by the test suite for every block of every available savegame and is
the strongest evidence that the interpretation above is correct.

## Field table

One entry per field:

```
u32 type     field type, see below
u32 label    index into the label table
u32 data     32 bit payload, or an offset/struct index, depending on the type
```

## Label table

`labelCount` entries of 16 bytes each. A label is NUL terminated unless it
occupies all 16 bytes (`VisualEffectNode`, `GainExperiencePoints`, ...). Labels
are shared by all structs of a block and are stored in the order the engine
first used them; JadeForge keeps that order and appends new labels.

## Field types

| id | name | `data` holds | payload |
| --- | --- | --- | --- |
| 0 | byte | value | inline |
| 1 | char | value | inline |
| 2 | word | value | inline |
| 3 | short | value | inline |
| 4 | dword | value | inline |
| 5 | int | value | inline |
| 6 | dword64 | offset into field data | 8 bytes |
| 7 | int64 | offset into field data | 8 bytes |
| 8 | float | bit pattern | inline |
| 9 | double | offset into field data | 8 bytes |
| 10 | string | offset into field data | `u32 length` + UTF-8 bytes |
| 11 | resref | offset into field data | `u8 length` + ASCII bytes |
| 12 | locstring (legacy) | offset into field data | see below |
| 13 | void | offset into field data | `u32 length` + raw bytes |
| 14 | struct | struct index | inline |
| 15 | list | offset into list indices | see below |
| 16 | quaternion | offset into field data | 4 × f32 |
| 17 | vector3 | offset into field data | 3 × f32 |
| 18 | locstring | offset into field data | see below |

### Localized strings (type 12 and 18)

```
u32 totalSize     number of bytes that follow
u32 strRef        talk table reference, 0xFFFFFFFF for "none"
u32 textLength    only present when totalSize > 4
u8  text[textLength]
```

So a string that only references the talk table is 8 bytes in total
(`totalSize = 4`), while inline text adds 4 bytes of length and the text itself.
All text in the savegames inspected so far is ASCII; JadeForge reads and writes
UTF-8, which is byte compatible for ASCII.

### Lists (type 15)

`data` is a byte offset into the list-index section. At that offset:

```
u32 count
u32 structIndex[count]
```

Lists are stored inline in that section; consecutive lists are simply
concatenated. Identical lists may be shared, and the reader does not rely on
either behaviour.

## Field data

Values referenced by offset are stored back to back in the field data section.
The game's files store payloads de-duplicated (for example the same string used
by two fields); JadeForge reproduces that: blocks are rewritten with payloads
de-duplicated by `(type, bytes)`.

## The player creature

The live player is `save.sav → PlayerCreature`. Editable fields that JadeForge
exposes:

| path | type | meaning |
| --- | --- | --- |
| `PlayerCreature.FirstName` | string | name |
| `PlayerCreature.Level` | byte | level |
| `PlayerCreature.CurrentXP` | dword | experience |
| `PlayerCreature.Stats.Gold` | dword | gold |
| `PlayerCreature.Stats.UnassignedPts` | dword | unspent attribute points (may be absent) |
| `PlayerCreature.Stats.Health` | float | current health |
| `PlayerCreature.Stats.HealthBase` | dword | maximum health |
| `PlayerCreature.Stats.Chi` | float | current chi |
| `PlayerCreature.Stats.ChiBase` | dword | maximum chi |
| `PlayerCreature.Stats.Focus.STAT_CUR` | float | current focus |
| `PlayerCreature.Stats.Focus.STAT_MAX` | float | maximum focus |
| `PlayerCreature.Stats.Body/Mind/Spirit` | dword | attributes |
| `PlayerCreature.Stats.Charm/Intuition/Intimidate` | int | skills |
| `PlayerCreature.Stats.Alignment` | int | alignment |
| `PlayerCreature.Stats.Appearance` | int | appearance index |
| `PlayerCreature.ItemList[]` | list | inventory items |
| `PlayerCreature.Progress.Kills` | int | kill counter |
| `PlayerCreature.Gems` | list | carried gems, see below |
| `PlayerCreature.Amulet.OpenGemsList` | list | amulet sockets, see below |
| `Campaign.Chapter` | dword | story chapter |
| `Campaign.TimePlayed` | dword64 | play time in microseconds |

The inventory holds structs with `Count` (byte), `ItemId` (word) and `Type`
(byte). Fields are sparse: the game only writes what differs from its defaults,
so for example `Stats.Alignment`, `Stats.UnassignedPts` and the style level
fields are absent until the player first changes them.

`Campaign.TimePlayed / 1_000_000` is the play time in seconds, which matches the
folder timestamps of the test savegames.

`polyplayer.sav`, when present, keeps a second copy of a character sheet at the
root of the block (`FirstName`, `Stats`, ...). JadeForge only falls back to it
when `save.sav` has no player creature, because it can be stale.

## Gems and the amulet

`PlayerCreature.Gems` and `PlayerCreature.Amulet.OpenGemsList` are lists of
structs shaped `{ Gem: { ItemID, GemsEffectList[] } }`. An effect carries
`GemsEffectType` (dword), `GemsEffectValue` (dword), `GemsEffectCurVal` (dword)
and `GemsEffectDurati` (float; the label is truncated to the 16 byte limit of
labels). `Gem.ItemID` is a word and points at a row of `items.2da` of category
`gems`.

`OpenGemsList` always holds seven entries. Which of them are *unlocked sockets*
is decided by `PlayerCreature.Amulet.Type` (a byte):

| `Amulet.Type` | sockets |
| --- | --- |
| absent (and no `Amulet.Active`) | the amulet is not unlocked yet, no sockets |
| 1 | 3 |
| 2 | 5 |
| 3 / 4 | 7 (the list is capped at seven) |

Evidence: across all 89 savegames the number of socketed gems never exceeds the
tier's socket count, and the counts only ever take the values 3, 5 and 7
(tier 1, tier 2, tier 4). Tier 3 was not observed; it is mapped to the cap like
tier 4. Savegames without `Amulet.Type` also have no `Amulet.Active` and no
socketed gems, i.e. the player has not received the amulet yet. The trailing
entries beyond the unlocked sockets are unused; the editor hides them and shows
the tier and socket count above the list.

Moving a gem only moves its struct index from one list to the other, so the gem
data is never rewritten. Some savegames point at the gem struct directly instead
of wrapping it in an entry with a `Gem` field; both shapes are read. An entry
with no `ItemID` and an empty effect list inside the unlocked sockets is an empty
socket and is displayed read-only.

Adding a gem builds the struct from scratch with the field types the game uses
(`Gem.ItemID` word, effect dwords and a float duration) and appends it to
`PlayerCreature.Gems`. The effect data comes from `gems.2da` (`Effect0..2`,
`Parameter0..2`, `Duration0`), joined to `items.2da` by label, so only gems the
game defines can be created. Effect ids are rows of `effects.2da`; the editor
shows their labels (`EFFECT_MODIFYBODYBASE` → `Body`, `EFFECT_MODIFYCHARM` →
`Charm`), with the parameter as the amount.

## Style levels and upgrades

A style struct stores `StyleLevel` (byte) and `StyleLevelInfo` (64 bit mask).
Both fields are absent for a freshly learned style: level 1 means no bits set.
`StyleLevel` is `1 + population count of the mask`. The game derives the unspent
style points from the levels of all owned styles, so editing a level also
changes the style points shown in game.

The mask indexes the **slots of the style's own level-up table**, not rows of
`StyleAdvance.2da`: bit `i` is the `i + 1`-th purchasable entry of the
`<Style>LV.2da` table, flattened row by row. Each slot points at a
`StyleAdvance` row that supplies the effect, the style point cost and the
description. The mapping is `stylesuperlist.StyleData` (row of the style) →
`styledata.2da` row → `levelup2da` column → the `<Style>LV.2da` table. The
tables ship as binary `2DA V2.b` resources inside `data/miniglobal-a.rim`;
`override/` text tables take precedence, which is what makes modded styles work.

Verification: Storm Dragon's table offers advance rows 6-10, 21-25 and 11-15
(slots 1-15). A savegame with Storm Dragon at level 8 carries bits
`{1,2,6,7,11,12,13}` (1-based slots), i.e. chi strike 1-2, duration 1-2 and
speed 1-3. Under the old "bit = advance row" assumption bits 1 and 2 would be
damage tiers, which the style does not even offer. In all 89 test savegames,
every set bit of every style that has a level-up table is inside that style's
slot range, and `level == 1 + popcount` holds for those styles; a test enforces
both. The sparsity of the fields is visible in the data too: 319 of the 551
style structs across the saves have no `StyleLevel`/`StyleLevelInfo` at all,
i.e. they are level 1.

The families visible in the level-up screen come from the level-up table rows
(their `Label` is a talk table reference). The `StyleAdvance.2da` families carry
five tiers with style point costs 1, 2, 4, 6 and 10, for example
`Increase_Damage_1..5` (effect 72, +25 % damage per tier) or
`Speed_Increase_1..5` (effect 81). The game's level-up screen never creates gaps
inside a family.

The editor shows the slots as plain golden orbs per family. Clicking an orb buys
every missing slot up to it, clicking an owned orb removes it and everything
above it, so a family is always a gap-free prefix. Styles whose level-up table
is missing are read-only: guessing slot semantics would write the wrong upgrade.
There is no "free selection" mode, because only the style's real slots exist.

## Game data files

Savegames only store numeric ids and talk table references. JadeForge reads the
installation (found from the save folder, `JADEFORGE_GAME_DIR` wins) for names
and tables. Every file is optional: without the installation the UI falls back
to numeric ids.

| file | format | purpose |
| --- | --- | --- |
| `dialog.tlk` | `TLK V4.0` | localized strings (names and descriptions) |
| `dialogf.tlk` | `TLK V4.0` | female player character variants |
| `override/*.2da` | text `2DA V2.0` | tables a mod or the release leaves loose |
| `data/miniglobal-a.rim` | RIM archive | most 2DA tables, incl. styles and effects |
| `data/globalunload-a.rim` | RIM archive | style animation tables |

`override/` is consulted first and the archives second; names are matched case
insensitively and text tables are decoded lossily, so tables with extended
characters still load.

### Talk table (`TLK V4.0`)

```
0x00  "TLK "
0x04  "V4.0"
0x08  u32 languageCount
0x0c  u32 stringCount
0x10  u32 entryTableOffset
0x14  u32 stringDataOffset
      entries: 10 bytes each, (u32 flags, u32 offset, u16 length)
      string data: UTF-8, not NUL terminated
```

### Text 2DA (`2DA V2.0`)

```
2DA V2.0
<default values>
LABEL   TYPE    ID
0       Sword   weapons
1       ****    gems
```

Cells contain `****` when they have no value. Values may be quoted when they
contain spaces. The row number in front of each line is the row id; rows do not
have to be contiguous.

### Binary 2DA (`2DA V2.b`)

```
"2DA V2.b\n"
column names, each followed by a tab, list terminated by NUL
u32 rowCount
row labels, each followed by a tab
u16 cellOffset[rowCount * columnCount]   row major, relative to cellBase
u16 cellDataSize
cellBase: NUL terminated cell strings
```

This is the layout the style, effect and gem tables use inside the RIM
archives; column names and row labels are tab terminated there.

### RIM archive

```
0x00  "RIM " "V1.0"
0x08  u32 unknown
0x0c  u32 entryCount
0x10  u32 entryTableOffset
entry (32 bytes): name[16]  u32 type  u32 id  u32 offset  u32 size
```

Type `2017` is a 2DA table. Offsets are relative to the start of the archive.

### Names and categories

| data | lookup |
| --- | --- |
| item name | `items.2da` row = `ItemId`, `STRREF_NAME` → `dialog.tlk` |
| item category | `items.2da` `TYPE` column (`gems`, `powerups`, `weapons`, `plot_items`, `mgweapons`, `improve`) |
| item category → savegame `Type` | gems 1, powerups 4, weapons 5, plot_items 6, mgweapons 7, improve 8 |
| style name | `stylesuperlist.2da` row = `StyleId`, `NameRef` → `dialog.tlk` |
| style description | `stylesuperlist.2da` `DescRef` → `dialog.tlk` |
| style kind | template-label prefix (``Martial_Strike``, ``Magic_Fire``, ...); `WeaponCount > 0` is only a fallback for labels the prefix table does not know |
| gem name and effects | `items.2da` joined with `gems.2da` on `LABEL` |
| effect name | row of `effects.2da` (`label`, `effectid`, `durationtype`, `2da`, `row`) |

The category mapping was measured over every item struct of the 89 test
savegames (inventories and store inventories); the six pairs above are the only
ones that occur. `items.2da` writes the improvement category as `improve`; the
wiki calls it *improvements*.

Development placeholders (`[TEMP] ...`, `temp_*`, `zzz*`, `cut_*`) are hidden
from the item picker but stay resolvable by id.

## Alignment

`PlayerCreature.Stats.Alignment` is a signed integer that the engine only writes
once the player's decisions move it: the earliest savegames of a playthrough do
not contain the field at all. In the 77 savegames that have it, the values run
from 131 to 255 and rise with play time, and one savegame sits exactly at 255.

The scale is unsigned 0..255: the community documentation for `gems.2da` states
that completely Closed Fist characters have alignment 0 and completely Open Palm
characters have 255. The neutral midpoint is therefore **128**; the editor uses
that reference (`DEFAULT_ALIGNMENT`) for its label and shows "Balanced" at 128.
The observed savegames only cover the Open Palm half (131..255). The raw number
stays editable, so a value can always be checked against the in-game meter; no
other alignment field exists in the player block.

## Story variables and journal

`Campaign.VarTable` is a list of `{ ID, Type, Value, Number? }` entries (1122 in
the 10 h save, 1940 in the 22 h save) that grows with story progress. The IDs
are hashes and there is no name table in the savegame, so a viewer would show
opaque numbers; no name mapping is implemented yet.

Journal entries were not found in `save.sav`. Area state lives in the 30
`J0x_*.sav` blocks, so journal/quest data is most likely there or derived from
the story variables; both need more mapping work before they are exposed.

## Unknowns

* Some fields that look numeric are stored as opaque blobs (for example
  `OceanWaveX`), because that is how the engine serializes them. JadeForge keeps
  them byte identical.
* `Amulet.Type` tier 3 was never observed; it is treated like tier 4 (7 sockets).
* The exact neutral alignment value is inferred as the midpoint 128; the observed
  saves only prove the 0/255 endpoints from the wiki and values 131..255.
* Item icons are not implemented: they live in texture resources (TXB/TGA) in
  the BIF archives, and JadeForge does not read BIF/KEY archives yet. Item names,
  categories and descriptions are resolved.
* Companion stats beyond level and availability are not exposed yet.
* Journal/quest structures and story variable names are unmapped (see above).

## Verification status

The format description above was validated against **89 retail savegames with
962 blocks**: every block parses, every block can be serialized again, and
re-parsing the result yields the identical tree. Unmodified savegames are
written back byte for byte, because untouched blocks are copied verbatim. Two
further properties are enforced across every savegame:

* every style satisfies `StyleLevel == 1 + popcount(StyleLevelInfo)` and every
  set bit is inside its style's slot range,
* every socketed gem fits the amulet tier's socket count.

## References

Community documentation, used for the semantics of the game data tables:

* [Gems.2da](https://jadeempire-modding.fandom.com/wiki/Gems.2da) — effect
  columns, unique sets, alignment requirement (0 = Closed Fist, 255 = Open Palm).
* [Effects.2da](https://jadeempire-modding.fandom.com/wiki/Effects.2da) — the
  columns and the effect id list.
* [Items.2da](https://jadeempire-modding.fandom.com/wiki/Items.2da) — item
  categories, including styles being filed under powerups.
* [Styleadvance.2da](https://jadeempire-modding.fandom.com/wiki/Styleadvance.2da)
  — style upgrade effects and costs.
* [2DA Files](https://jadeempire-modding.fandom.com/wiki/2DA_Files) — index of
  the tables.
