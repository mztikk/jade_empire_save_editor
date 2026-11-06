# Configurable window geometry and multi-monitor policy

## Current state

P1 added a geometry plugin that remembers the window rectangle in the settings
file and clamps it to the monitor. The minimum size was already lowered on
short screens and the default 1180x760 fitted a 1366x768 screen at 125 %. The
policy for multiple monitors and DPI was implicit in the code.

## Approach

- New setting `remember_window = true|false`. When it is off, `window_config`
  ignores a stored rectangle and the plugin does not write one on close, so the
  compositor/window manager places a fresh default window. The About page has a
  **Remember size and position** toggle that persists the setting.
- The window is still opened on the **primary monitor** (falling back to the
  first available monitor) and the remembered rectangle is clamped into that
  monitor's work area. A remembered position from a disconnected monitor cannot
  resurrect an off-screen window; a position from a secondary monitor is
  clamped into the current monitor rather than trusted blindly.
- DPI follows the operating system: the window is created with logical sizes
  and the OS scale factor is applied by winit. JadeForge does not force a
  custom scale factor (Freya's `with_custom_scale_factor`): the text-scale
  setting covers user zoom, and forcing a second global factor would make the
  minimum window dimensions impossible to reason about. The 125 % and 150 %
  monitor cases are unit tested through `fit_window`.
- The documented policy is: primary monitor at open, work area margins of 16 px,
  minimum size follows the available logical space, and the rectangle survives
  restarts only when remembering is on. A future per-monitor DPI change while
  the window is open is handled by winit's scale factor event, not by the
  editor.

Upstream references:

- winit monitor geometry and scale factor:
  <https://docs.rs/winit/0.30/winit/monitor/struct.MonitorHandle.html>
- Freya custom scale factor (evaluated, not used):
  <https://docs.rs/freya-winit/0.5.0/freya_winit/config/struct.WindowConfig.html#method.with_custom_scale_factor>

## Test plan

- `fit_window`: default on a large screen; 1366x768 at 125 %; 2560x1440 at
  150 %; a remembered bigger size clamped; a remembered position clamped;
  a window on a secondary monitor with a negative origin clamped into that
  monitor; no monitor at all.
- Config: `remember_window` defaults to true and round-trips through TOML.
- Headless UI: the About toggle persists both directions.
- Windowed behaviour (where the compositor places the window and whether it
  restores the rectangle) is window-path behaviour. The existing live scenario
  `window_geometry_restarts` covers it, but it was **not run** for this branch
  because permission was not granted; the headless geometry tests cover the
  decision logic only.

## Accessibility, contrast and reduced motion

- The toggle is a radio-style chip with selected state and a spoken
  description.
- No animation; a restart is not required for the setting to affect the next
  window.
- DPI changes do not affect contrast (the palette is logical-colour based).

## Risk

- Wayland does not expose window position; the stored position is kept instead
  of guessed, and the policy above only promises position restoration where the
  compositor reports it.
- The primary monitor is used even if the pointer is on another monitor when
  the app starts; this is deterministic and documented. A per-monitor choice
  would need a compositor API and is out of scope.
