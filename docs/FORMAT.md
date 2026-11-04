# Jade Empire savegame format

This document describes the format of `save.sac` as implemented by
`jadeforge-core`. Everything below was derived from observation of retail
savegames; it is the reference that the parser, the writer and the test suite
are checked against.

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
`save.sav`). JadeForge preserves the order.

Real examples found in savegames:

| block | content |
| --- | --- |
| `J01_BEACH.sav` … | serialized state of one area (creatures, placeables, triggers, ...) |
| `polyplayer.sav` | character sheet, only present in a few releases |
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
u32 structType     0xFFFFFFFF for the root, 0 for every other known struct
u32 fieldReference
u32 fieldCount
```

The root struct is the one with type `0xFFFFFFFF`; in all known files it is
struct 0.

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
concatenated (the writer of the game appears to de-duplicate identical lists,
the reader does not care).

## Field data

Values referenced by offset are stored back to back in the field data section in
the order the fields appear in the field table, and the engine de-duplicates
identical payloads (for example the same string used by two fields). JadeForge
reproduces that: blocks are rewritten with payloads de-duplicated by
`(type, bytes)`.

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
| `PlayerCreature.Progress.Kills` | dword | kill counter |
| `Campaign.Chapter` | dword | story chapter |
| `Campaign.TimePlayed` | dword64 | play time in microseconds |

The inventory holds structs with `Count` (byte), `ItemId` (word) and `Type`
(byte).

`polyplayer.sav`, when present, keeps a second copy of a character sheet at the
root of the block (`FirstName`, `Stats`, ...). JadeForge only falls back to it
when `save.sav` has no player creature, because it can be stale.

## Where names come from

Savegames only store numeric ids and talk table references. The names shown by
JadeForge come from the game installation:

| data | lookup |
| --- | --- |
| item name | `items.2da` row = `ItemId`, `STRREF_NAME` → `dialog.tlk` |
| item category | `items.2da` `TYPE` column (`weapons`, `gems`, `improve`, ...) |
| item category → savegame type | gems 1, powerups 4, weapons 5, plot_items 6, mgweapons 7, improve 8 |
| style name | `stylesuperlist.2da` row = `StyleId`, `NameRef` → `dialog.tlk` |
| style is a weapon style | `stylesuperlist.2da` `WeaponCount` > 0 |

The category mapping was measured over every item struct of the 88 test
savegames (inventories and store inventories); the six pairs above are the only
ones that occur.

## Alignment

`PlayerCreature.Stats.Alignment` is a signed integer that the engine only writes
once the player's decisions move it: the earliest savegames of a playthrough do
not contain the field at all. In the 76 savegames that have it, the values run
from 131 to 255 and rise with play time, and one savegame sits exactly at 255.

That matches an unsigned 0..255 scale whose neutral value is **128**: below is
the Way of the Closed Fist, above is the Way of the Open Palm. The editor uses
that reference (`DEFAULT_ALIGNMENT`) for its label and shows "Balanced" at 128.
The raw number stays editable, so a value can always be checked against the
in-game meter; no other alignment field exists in the player block.

## Style levels and upgrades

A style struct stores `StyleLevel` (byte) and `StyleLevelInfo` (64 bit mask).
`StyleLevel` is `1 + population count of the mask`; a freshly learned style has
level 1 and no bits set. The game derives the unspent style points from the
levels of all owned styles, so editing a level also changes the style points
shown in game.

The mask is **not** a plain "how many upgrades" counter: bit *i* selects row
*i + 1* of `StyleAdvance.2da`, which is one concrete upgrade with its own effect
and style point cost. The table groups five tiers per family, with costs
1, 2, 4, 6 and 10:

| rows (1 based) | family | effect |
| --- | --- | --- |
| 1-5 | `Increase_Damage_1..5` | effect 72, +25 % damage per tier |
| 6-10 | `Chi_Strike_Damage_1..5` | effect 84, chi strike multiplier |
| 11-15 | `Speed_Increase_1..5` | effect 81, +5 % speed per tier |
| 16-20 | `Lower_Chi_Cost_1..5` | effect 69, -5 % chi cost per tier |
| 21-25 | `Duration_1..5` | style duration |
| 26-30 | `Chi_Reduction_1..5` | chi damage reduction |

Per style level the game offers the five tiers of one family (the `<Style>LV.2da`
tables hold that mapping); buying one sets its bit. The 76 real savegames
confirm this: a level 11 style carries ten bits, for example `[0-4, 10-14]`
(five damage tiers plus five speed tiers), and every style satisfies
`level == 1 + popcount`.

The editor shows those bits as one golden orb per tier and family inside the
expanded style row.
Clicking an orb sets the family to that tier: missing tiers up to it
are bought, clicking an owned orb removes it and everything above it. That is a
gap-free prefix per family, because the game's own level-up screen never creates
gaps. The level itself is read-only in the editor; it is derived from the mask.

A **Game rules** toggle (on by default)
restricts the orbs to the rows listed by the style's own `<Style>LV.2da` table
(`IronPalmLV`, `ViperLV`, `Ghost_KingLV` in this installation; most styles do
not ship one), and falls back to the full `StyleAdvance.2da` table when the
table is missing so nothing is hidden by guesswork.

## Unknowns

* Some fields that look numeric are stored as opaque blobs (for example
  `OceanWaveX`), because that is how the engine serializes them. JadeForge keeps
  them byte identical.
* Item names live in the game's data files (`.uti` templates inside the BIF
  archives), not in the savegame. JadeForge therefore shows numeric item ids.
* `Alignment` semantics (0 = closed fist, 1000 = open palm) are plausible but
  not verified in game.

## Verification status

The format description above was validated against 88 retail savegames with 943
blocks: every block parses, every block can be serialized again, and re-parsing
the result yields the identical tree. Unmodified savegames are written back
byte for byte, because untouched blocks are copied verbatim.
