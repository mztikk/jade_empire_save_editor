# Application icon assets

`icon.svg` is the source of truth: a steel cross-peen hammer striking a faceted
jade gem, with gold forge sparks, on the dark rounded tile. The artwork group is
scaled 1.18x and recentred inside the 512 px tile; change that transform to give
the mark more or less breathing room.

Everything else is generated from that file and committed:

| file | use |
| --- | --- |
| `icon-{16,24,32,48,64,128,256,512,1024}.png` | window icon, Linux hicolor sizes, macOS/Windows stores |
| `jadeforge.ico` | Windows executable and shortcut icon |
| `jadeforge.icns` | macOS `.app` bundle icon |
| `linux/jadeforge.desktop` | Linux launcher entry; its `Icon=` matches `StartupWMClass` and the `jadeforge` window app id |

## Regenerating

Edit `icon.svg`, then run:

```sh
just icons
```

This requires `rsvg-convert` (librsvg), ImageMagick 7 (`magick`) and Python 3.
`ci/icons.py` renders every PNG, lets ImageMagick assemble the multi-size ICO,
and packs the ICNS container itself because ImageMagick does not write a usable
one. The generated files are committed so normal builds need none of those
tools.

## Platform notes

- **X11 and Windows:** the GUI embeds `icon-256.png` and passes it to
  `WindowConfig::with_icon`; the window manager or shell scales it. On Windows
  the crate's `build.rs` also embeds `jadeforge.ico` in the executable, which is
  what Explorer, shortcuts and the taskbar read.
- **Wayland:** the compositor ignores client window icons. It matches the
  window's app id (`jadeforge`) against `jadeforge.desktop` and loads the icon
  from the hicolor theme, which is why the desktop entry and the size-named PNGs
  live here. `just install-desktop` installs the GUI and CLI binaries, the entry
  and the icons for a source install; release folders carry the same files under
  `share/`.
- **macOS:** the window icon API is unsupported on macOS. `ci/macos-bundle.sh`
  wraps the staged binary in `JadeForge.app` and puts `jadeforge.icns` in
  `Contents/Resources`; the Dock and Finder read it from the bundle. The bare
  binary stays next to the bundle for terminal launches.

`just publish` stages all three: the ICO inside the Windows `.exe`, the
`JadeForge.app` bundle in each macOS folder, and `share/applications` plus
`share/icons/hicolor` in each Linux folder.
