# Item and style icons from the game data

## Status: blocked on unavailable image data

The names, categories and descriptions already resolve from `items.2da` /
`stylesuperlist.2da` and `dialog.tlk`. The icon **references** resolve too:
`jadeforge-gamedata` exposes `ItemInfo.icon` from the `ICON` column and the
savegame carries a style `IconRef`.

Concrete evidence gathered from the retail installation
(`/media/Games/Heroic/Jade Empire`, read only):

- `override/items.2da` has an `ICON` column; for example item `item_gem_body_I`
  points at `ig_body`, and weapons point at names such as `w_icon_Damulet`.
- `data/global-a.rim` and `data/miniglobal-a.rim` contain entries named
  `ig_body`, `ig_body5`, `ig_body5h`, `w_icon_bottle`, `w_icon_damulet`, but
  they are **not images**:

  | archive | entry | resource type | size | payload |
  | --- | --- | --- | --- | --- |
  | global-a.rim | `ig_body5` | 3017 | 1170 | 32x32-looking custom block, no TGA header |
  | global-a.rim | `w_icon_bottle` | 2002 | 11160 | model data containing the string `w_icon_bottle` |
  | global-a.rim | `w_icon_bottle` | 3016 | 4680 | float-like data, not a TGA/DDS image |
  | global-a.rim | `w_icon_damulet` | 2002 | 9383 | model data |
  | global-a.rim | `w_icon_damulet` | 3016 | 0 | empty |

- A scan of the RIM payloads for TGA-like headers (id length 0, image type 2 or
  10) found no item or icon resource with a readable image.
- The image assets themselves live in the engine's BIF/KEY archives
  (`chitin.key`, `data/designer.bif`, ...). The repository has no BIF/KEY
  reader, and those archives are compressed engine resources, not loose TGAs.
  Committing game assets is forbidden by AGENTS.md rule 7 regardless.

## Why this is blocked

Rendering icons at production quality needs, in order:

1. a KEY/BIF reader that resolves the icon resource name to a payload,
2. the specific texture format the engine uses (not TGA or DDS in the files
   sampled above),
3. the item-icon to texture mapping (mods and the `ICON` column can change it),
4. a runtime-only cache with a graceful fallback to the current text rows when
   the installation (or the icon) is missing.

That is a new binary-format subsystem comparable to the GFF reader, and it
needs a real installation to validate every step. It cannot be faked with the
already-bundled name index, which contains names only. The row subtitle
(`category · #id`) and the expanded description remain the fallback, and no
placeholder image is shipped.

No code was changed for this item; the design note records the evidence and the
blocker rather than dropping the scope silently.

## Acceptance criteria if the blocker is removed

- Icons are loaded read-only from the user's installation at runtime and never
  committed; a missing file falls back to the existing text row.
- The image is decorative: the item name and category remain the accessible
  name, and the icon gets an empty alt so a screen reader does not repeat it.
- The icon is checked against every surface for non-text contrast (3:1) in all
  three themes; a pale icon gets a themed backing plate if needed.
- Reduced motion: icons are static, no hover animation is added. Any future
  hover effect must respect the no-motion rule used by the rest of the shell.
- Tests would cover: resource lookup by icon name from a synthetic archive, a
  missing resource fallback, the palette contrast check, and a headless render
  at 900/1280/1600 px plus 200 % text.
