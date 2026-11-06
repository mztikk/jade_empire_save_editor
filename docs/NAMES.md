# Bundled English names

Savegames store item and style ids, not names. The editor normally resolves
names from the user's Jade Empire installation. Players who download a save or
move one from a console to a PC without the game installed would otherwise only
see ids, so JadeForge ships a small generated English fallback index.

## What is in the index

`crates/jadeforge-gamedata/data/names-en.toml` contains only:

- item id, English name and `items.2da` category,
- style id and English name.

It deliberately does **not** contain descriptions, 2DA tables, upgrade rows,
effect names, icons or any other extracted text. This is the single exception
to the "no game data files" rule in [AGENTS.md](../AGENTS.md); do not extend it.

The item category is enough to enable the item picker: the savegame's `Type`
value still decides the category of an owned item, while a picked item uses the
bundled category to derive its `Type`. Descriptions and upgrade data remain
unavailable, so style upgrade editing, learning styles and gem creation still
require a real installation.

## Precedence

1. A loaded game installation always wins. This keeps mods and localized
   installations correct.
2. The bundled index is used only when no installation was found (or the found
   installation could not be read).
3. Unknown ids fall back to the savegame-derived display: `Item <id>` or
   `Style <id>`, the category from the savegame `Type`, and the style's level,
   weapons, icon ref and time spent.

The UI labels this state as "English names loaded" or "bundled English names";
it never pretends the installation was found.

## Regenerating the index

The index is generated, not edited by hand. It must be generated from an
**English** installation, because the name strings live in `dialog.tlk`. The
2DA tables themselves are language independent, so an English `dialog.tlk` can
be combined with the tables from another installation.

```sh
cargo run -p jadeforge-cli -- export-names \
  --game "/path/to/Jade Empire" \
  --output crates/jadeforge-gamedata/data/names-en.toml
```

The command reads the installation only. It writes a deterministic, id-sorted
TOML file with a format version, language and source header. Commit the result
and run the tests; never copy `dialog.tlk`, 2DA files or savegames into the
repository.

## Updating the file

- Regenerate it from a known English installation.
- Review the diff: only names and categories may change.
- Keep the format version in sync with `crates/jadeforge-gamedata/src/names.rs`
  when the schema changes.
- Do not add translations or other languages to this file. A future language
  pack would need its own decision and provenance.

## License

The index contains short names from Jade Empire. Those names remain the
property of their respective rights holders and are not covered by JadeForge's
MIT license; they are included only to identify items and styles in the editor.
See [NOTICE](../NOTICE).
