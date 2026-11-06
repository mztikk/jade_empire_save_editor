# Windowed UI tests

JadeForge has two complementary UI suites:

- the **headless suite** (`crates/jadeforge-gui/tests/ui.rs`) renders the real
  component tree off-screen with `freya-testing`; it is fast, exhaustive and runs
  on every CI machine, and
- the **windowed suite** (`crates/jadeforge-gui/tests/live.rs`) launches the real
  application through `freya-winit`, opens a real Wayland window and drives it.

The windowed suite exists to catch what an off-screen renderer cannot: window
creation, the winit event loop, real Skia/GL presentation on a compositor,
compositor input (buttons and keys), focus and the shipped window configuration.
It does not replace the headless suite; it deliberately covers only the
window-level path.

> **Hard rule: never run the windowed suite without explicit permission.**
> `JADEFORGE_LIVE_APPROVED=1 just live` and every `JADEFORGE_LIVE=1` run open
> real windows, take focus, move the real pointer and type on the machine's
> desktop. The approval variable is a second gate: plain `just live` refuses,
> and the test binary fails before opening a window unless
> `JADEFORGE_LIVE_APPROVED=1` is also set. Ask the user first, explain why the
> headless suite cannot cover the case, and wait for their answer; a general
> request to run the tests is not permission. Only run it when
> a headless GUI test genuinely cannot reproduce or verify the behaviour (window
> creation, compositor input, focus, presentation, HiDPI or window-manager
> interaction), or when the bug is in the actual windowed GUI or a mismatch
> between the headless and real window. This is [AGENTS.md](../AGENTS.md) rule
> 13.

## When to ask

The suite is not a routine check for every change. Ask for permission when the
change is in the window path:

- a major Freya, winit, renderer, Wayland or accessibility-backend upgrade, or
  any upgrade that changes the live-probe or window dependencies;
- window creation or configuration, the close guard, focus, keyboard or pointer
  input, presentation, HiDPI, plugins or compositor integration;
- a change to `tests/live_support/` or the live probe;
- a bug that reproduces only in the real window or differs from the headless
  result.

Do not ask for view, layout, text, theme, form, session or core changes that the
headless suite and snapshots cover; those stay in the fast loop. If the change
is borderline, ask. Compiling the live target is required when it changed, but
"it compiles" is not a substitute for running it. When the suite is in scope,
ask before `just finish`, wait for the answer, and record the result in the
hand-over. When it is out of scope, say why the headless suite is sufficient.

## Running it

```sh
JADEFORGE_LIVE_APPROVED=1 just live
# or
JADEFORGE_LIVE=1 JADEFORGE_LIVE_APPROVED=1 \
  cargo test -p jadeforge-gui --test live -- --nocapture
```

The commands above are only for use after the explicit permission described in
the hard rule. Plain `just live` is refused, and the test binary also refuses a
run without `JADEFORGE_LIVE_APPROVED=1`; without `JADEFORGE_LIVE=1` it prints a
skip message and exits successfully. That keeps `cargo nextest run --workspace`
from opening windows on a developer's desktop. It requires a Wayland session
(`WAYLAND_DISPLAY`); `JADEFORGE_LIVE_REQUIRE=1` turns a missing display into a
failure instead of a skip.

The test windows appear on the real desktop and take focus. They are activated,
clicked and typed into as part of the scenarios. Run the suite when that is
acceptable, not while a presentation is being recorded.

## What is real

- The application component is `jadeforge_gui::app_with_directory`, the same
  component the binary uses.
- The window is built with `jadeforge_gui::app::window_config`, so the title,
  icon, 1180x760 size, 900x700 minimum, application id and unsaved-changes close
  guard are the production ones. Tests only override the application id to a
  unique value so KWin can find the test window.
- `freya_winit::launch` runs the actual winit event loop and the actual Skia/GL
  renderer.
- On KDE Plasma, pointer motion, clicks and keystrokes are injected through
  KWin's `org_kde_kwin_fake_input` protocol, the Wayland equivalent of XTest.
  Tests find nodes in the live layout and click their real screen coordinates.
- The `the_window_is_really_presented` scenario asks Spectacle for a compositor
  screenshot of the active window, checks that its size is the client area and
  that the image is not blank, and writes it to `target/live-ui/presented.png`.
- Assertions use the live widget tree, not a screenshot diff: the `Probe` plugin
  publishes every label and paragraph with its layout rectangle after each
  render. That makes failures readable (`timed out waiting for "Identity";
  visible text: [...]`) and keeps the suite independent of fonts and themes.

## The two input backends

| backend | used by | input path | screenshot |
| --- | --- | --- | --- |
| `kwin` | KDE Plasma (`auto` default there) | real compositor pointer/key events | yes, via Spectacle |
| `plugin` | anything else | `PlatformEvent`s posted through the windowed plugin handle | no |

`JADEFORGE_LIVE_INPUT=plugin` or `JADEFORGE_LIVE_INPUT=kwin` forces a backend.
The default is auto-detection: try KWin first, fall back to plugin events with a
printed note when KWin, `busctl` or the fake-input protocol is unavailable.

The `plugin` backend is not `freya-testing`. Its events go through the same
windowed event measurer and renderer as winit events, the window is a real
compositor window, and the renderer presents to a real Wayland surface. It does
not exercise compositor input, which is why the KDE path uses fake input.

## Process model

A winit event loop must be created on the main thread, and a process can only
safely run one. The test binary is therefore its own orchestrator
(`harness = false`): with `JADEFORGE_LIVE=1` and `JADEFORGE_LIVE_APPROVED=1` it
builds a synthetic savegame and
game installation, then re-executes itself once per scenario with
`JADEFORGE_LIVE_SCENARIO` set. Each child launches one window, runs one
scenario, closes the window through the plugin (bypassing the unsaved-changes
dialog) and exits with its result. The orchestrator reports per-scenario
results, continues after failures and kills a child that exceeds 120 seconds.

## Fixture and isolation

Each run builds the same synthetic installation the headless suite uses
(`tests/support/mod.rs`): one `Test Save`, `dialog.tlk`, item/style tables and a
screenshot. Every scenario gets its own `JADEFORGE_CONFIG_DIR`, so the developer's
remembered sort order or save folder cannot leak in. The fixture lives in a
temporary directory and is removed after the run.

## Desktop and VM runs

The suite is not part of CI. It needs a real Wayland desktop because it opens
windows, moves the real pointer, types and captures the compositor's output. Run
it from:

- a normal KDE Plasma Wayland session (real compositor input through KWin), or
- a VM with a real Wayland desktop such as KDE Plasma. The KWin backend works
  inside the guest as usual.

A nested or headless compositor can work with `JADEFORGE_LIVE_INPUT=plugin` for
development, but it is not a CI target; the KWin-only screenshot scenario skips
there. GitHub Actions runners have no real desktop, so the suite stays out of
[`.github/workflows/ci.yml`](../.github/workflows/ci.yml).

`JADEFORGE_LIVE_REQUIRE=1` turns a missing `WAYLAND_DISPLAY` into a failure,
which is useful when a script must prove that the suite actually ran.

## Adding a scenario

1. Add a `fn name(driver: &mut Driver) -> Result<(), String>` to
   `tests/live_support/scenarios.rs` and register it in `SCENARIOS`.
2. Use `driver.wait_for_text` / `wait_for_exact` for a node, `driver.click` or
   `driver.click_exact` for real input, and `driver.type_text` for keyboard
   input. Every wait has a timeout and lists the visible text on failure.
3. Keep the scenario window-level. Examples: opening the first savegame,
   switching every tab, filtering the item database by typing, or capturing the
   real window. Layout, geometry and exhaustive tab coverage belong in the
   headless suite.
4. If the scenario needs a screenshot, return `Ok(())` in plugin mode with a
   printed skip; do not make the suite depend on Spectacle for its other
   scenarios.

## Limitations

- Linux/Wayland only. The suite refuses to run on X11 instead of silently
  testing a different path.
- The pointer coordinate mapping requires a compositor scale factor of 1.0.
  Scaling support needs a physical/logical coordinate translation; until then
  the suite fails with a clear error instead of clicking the wrong place.
- The KWin backend depends on `busctl`, KRunner's window runner and Spectacle.
  They are all part of a normal Plasma installation; without them the suite
  falls back to plugin input.
- The suite is intentionally small and parallel to the headless suite. Do not
  duplicate every layout assertion here; headless tests are the fast feedback
  loop.
