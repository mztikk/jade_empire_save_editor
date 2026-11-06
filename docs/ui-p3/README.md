# P3 strategic UI work: design notes and progress

Standalone design notes for the 13 checkboxes in
[issue #9](https://github.com/mztikk/jade_empire_save_editor/issues/9). Each
note records the current state, the approach, the upstream API it relies on,
the test plan, the accessibility/contrast/reduced-motion criteria and the risk.
Implementation and screenshots land together with the note.

Every item is checked off against the headless suite
(`cargo nextest run -p jadeforge-gui`) and the snapshot example at 900, 1280
and 1600 px. The windowed suite is never run without explicit permission
(AGENTS.md rule 13); gaps are listed here rather than hidden.

| # | item | note | status |
| --- | --- | --- | --- |
| 1 | light/system-following theme | [theme-light.md](theme-light.md) | implemented |
| 2 | high contrast and forced colours | [high-contrast.md](high-contrast.md) | implemented manually; automatic detection blocked upstream |
| 3 | text scaling to 200 % | [text-scaling.md](text-scaling.md) | implemented |
| 4 | UI localization | [localization.md](localization.md) | pending |
| 5 | virtualized browser and pickers | [virtual-lists.md](virtual-lists.md) | pending |
| 6 | per-action undo/redo | [undo-redo.md](undo-redo.md) | implemented |
| 7 | pending-changes diff before save | [pending-diff.md](pending-diff.md) | implemented |
| 8 | backup browsing and restore screen | [backups.md](backups.md) | implemented |
| 9 | reload from disk after an external change | [reload-from-disk.md](reload-from-disk.md) | implemented |
| 10 | item and style icons | [icons.md](icons.md) | pending |
| 11 | About/help with diagnostics | [about-diagnostics.md](about-diagnostics.md) | pending |
| 12 | command palette or shortcut sheet | [command-palette.md](command-palette.md) | pending |
| 13 | configurable geometry, DPI and multi-monitor policy | [geometry-dpi.md](geometry-dpi.md) | pending |

## Order and dependencies

The work follows the issue order with one dependency:

1. The theme/scaling foundation first, because every view and every screenshot
   depends on it.
2. Editing safety next (undo, diff, backups, reload), session layer before UI.
3. Large lists, because virtualization must keep the focus semantics that the
   editing-safety dialogs also use.
4. Localization, once the visible strings have stopped moving.
5. Content and identity last, so the About page can show the final settings.

## Cross-cutting acceptance criteria

These apply to every item and are repeated in its note:

- **Accessibility**: keyboard reachable, focus visible, AccessKit role/name on
  every new control, list rows keyed and single-tab-stop semantics preserved.
- **Contrast**: WCAG 2.2 AA for text and 3:1 for non-text controls in all three
  palettes; high contrast targets AAA (7:1) where practical. The palette unit
  tests enforce the numbers.
- **Reduced motion**: the application has no non-essential animation. Theme,
  scale, dialog and list changes are instant; no new animation is introduced,
  so `prefers-reduced-motion` requires no switch. Any future animation must be
  gated by the same acceptance test.
- **No literal layout values**: new sizes are theme tokens, and the 200 % tests
  must stay green.

## Evidence

Tracked screenshots are regenerated from the real corpus with
`cargo run -p jadeforge-gui --example snapshot -- "$JADEFORGE_SAVE_DIR" ...`.
The snapshot example accepts `--theme dark|light|high-contrast` and
`--scale 100|125|150|200` so the documented variants are reproducible.
