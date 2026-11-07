//! Window geometry: DPI-aware defaults and the remembered rectangle.
//!
//! The default 1180x760 logical window does not fit every screen: on a
//! 1366x768 display at 125% scaling only about 1093x582 logical pixels are
//! available. The default therefore has to be clamped to the monitor that will
//! show the window, and the minimum size has to follow, or the window manager
//! keeps a window that is larger than the screen.

use freya::winit::monitor::MonitorHandle;
use freya_winit::plugins::{FreyaPlugin, PluginEvent, PluginHandle};

use crate::config::WindowGeometry;
use crate::theme;

/// A rectangle on the monitor, in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorkArea {
    /// Left edge in logical pixels.
    pub x: f64,
    /// Top edge in logical pixels.
    pub y: f64,
    /// Width in logical pixels.
    pub width: f64,
    /// Height in logical pixels.
    pub height: f64,
}

/// The window rectangle the application asks the window manager for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FittedWindow {
    /// Width in logical pixels.
    pub width: f64,
    /// Height in logical pixels.
    pub height: f64,
    /// Smallest width the window manager should allow.
    pub min_width: f64,
    /// Smallest height the window manager should allow.
    pub min_height: f64,
    /// Remembered left edge, when one was stored and a monitor is known.
    pub x: Option<f64>,
    /// Remembered top edge, when one was stored and a monitor is known.
    pub y: Option<f64>,
}

/// Fits the preferred or remembered rectangle into `work`.
///
/// `preferred` and `minimum` come from the layout tokens. A remembered size
/// wins over the preferred one, but neither may exceed the monitor minus a
/// small margin, and the minimum follows the monitor down so a short screen can
/// still display the window. A remembered position is clamped so a window from
/// a disconnected monitor cannot reappear off screen.
#[must_use]
pub fn fit_window(
    preferred: (f64, f64),
    stored: Option<WindowGeometry>,
    work: Option<WorkArea>,
    minimum: (f64, f64),
) -> FittedWindow {
    let margin = f64::from(theme::layout::window_margin());
    let wanted = stored
        .filter(WindowGeometry::is_valid)
        .map_or(preferred, |geometry| {
            (f64::from(geometry.width), f64::from(geometry.height))
        });

    let Some(work) = work.filter(|work| work.width > 1.0 && work.height > 1.0) else {
        return FittedWindow {
            width: wanted.0.max(minimum.0),
            height: wanted.1.max(minimum.1),
            min_width: minimum.0,
            min_height: minimum.1,
            x: stored.and_then(|geometry| geometry.x).map(f64::from),
            y: stored.and_then(|geometry| geometry.y).map(f64::from),
        };
    };

    let max_width = (work.width - 2.0 * margin).max(1.0);
    let max_height = (work.height - 2.0 * margin).max(1.0);
    let min_width = minimum.0.min(max_width);
    let min_height = minimum.1.min(max_height);
    let width = wanted.0.max(min_width).min(max_width);
    let height = wanted.1.max(min_height).min(max_height);

    let clamp = |value: f64, origin: f64, extent: f64, size: f64| {
        value.max(origin).min(origin + (extent - size).max(0.0))
    };
    FittedWindow {
        width,
        height,
        min_width,
        min_height,
        x: stored
            .and_then(|geometry| geometry.x)
            .map(|x| clamp(f64::from(x), work.x, work.width, width)),
        y: stored
            .and_then(|geometry| geometry.y)
            .map(|y| clamp(f64::from(y), work.y, work.height, height)),
    }
}

/// Whether the desktop compositor owns window placement.
///
/// Wayland has no client-side absolute placement. winit returns
/// `NotSupportedError` from `outer_position` and ignores the requested
/// position, so `JadeForge` can restore the size but not the coordinates there.
/// X11 and the two desktop platforms keep the remembered position.
#[must_use]
pub fn position_is_compositor_owned() -> bool {
    cfg!(target_os = "linux")
        && wayland_display_is_set(std::env::var_os("WAYLAND_DISPLAY").as_deref())
}

fn wayland_display_is_set(value: Option<&std::ffi::OsStr>) -> bool {
    value.is_some_and(|value| !value.is_empty())
}

/// Logical bounds of one monitor.
fn logical_work_area(monitor: &MonitorHandle) -> WorkArea {
    let scale = monitor.scale_factor();
    let size = monitor.size().to_logical::<f64>(scale);
    let position = monitor.position().to_logical::<f64>(scale);
    WorkArea {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
    }
}

/// Finds the monitor containing a remembered window position.
fn work_area_containing(areas: &[WorkArea], stored: Option<WindowGeometry>) -> Option<WorkArea> {
    let point =
        stored.and_then(|geometry| Some((f64::from(geometry.x?), f64::from(geometry.y?))))?;
    areas.iter().copied().find(|area| {
        point.0 >= area.x
            && point.0 < area.x + area.width
            && point.1 >= area.y
            && point.1 < area.y + area.height
    })
}

/// Reads the monitor the window will open on as a logical work area.
///
/// winit exposes the monitor bounds, not the desktop's work area, so the whole
/// monitor is used and [`fit_window`] keeps a margin around the window. A
/// remembered position on a secondary monitor selects that monitor instead of
/// always clamping the window to the primary one.
#[must_use]
pub fn monitor_work_area(
    event_loop: &freya::winit::event_loop::ActiveEventLoop,
    stored: Option<WindowGeometry>,
) -> Option<WorkArea> {
    let primary = event_loop
        .primary_monitor()
        .map(|monitor| logical_work_area(&monitor));
    let areas: Vec<WorkArea> = event_loop
        .available_monitors()
        .map(|monitor| logical_work_area(&monitor))
        .collect();
    work_area_containing(&areas, stored)
        .or(primary)
        .or_else(|| areas.first().copied())
}

/// Remembers the window rectangle when the window closes.
///
/// The close hook of a `WindowConfig` only sees the renderer context, whose
/// window is private to `freya-winit`, so the plugin event is the supported way
/// to read the final size and position. Wayland does not report a window
/// position, so a close there stores the size and leaves the coordinates unset;
/// the compositor owns placement.
pub struct WindowGeometryPlugin;

impl FreyaPlugin for WindowGeometryPlugin {
    fn plugin_id(&self) -> &'static str {
        "jadeforge-window-geometry"
    }

    fn on_event(&mut self, event: &mut PluginEvent, _handle: PluginHandle) {
        let PluginEvent::WindowClosed { window, .. } = event else {
            return;
        };
        let scale = window.scale_factor();
        let size = window.inner_size().to_logical::<f64>(scale);
        let position = window
            .outer_position()
            .ok()
            .map(|position| position.to_logical::<f64>(scale));
        if !crate::config::Config::load().remember_window {
            return;
        }
        let geometry = WindowGeometry {
            width: size.width.max(1.0).round() as u32,
            height: size.height.max(1.0).round() as u32,
            x: position.map(|position| position.x.round() as i32),
            y: position.map(|position| position.y.round() as i32),
        };
        crate::config::Config::remember_window(geometry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen(width: f64, height: f64) -> WorkArea {
        WorkArea {
            x: 0.0,
            y: 0.0,
            width,
            height,
        }
    }

    fn preferred() -> (f64, f64) {
        (
            f64::from(theme::layout::window_width()),
            f64::from(theme::layout::window_height()),
        )
    }

    fn minimum() -> (f64, f64) {
        (
            f64::from(theme::layout::min_window_width()),
            f64::from(theme::layout::min_window_height()),
        )
    }

    #[test]
    fn the_default_size_is_requested_on_a_large_screen() {
        let fitted = fit_window(preferred(), None, Some(screen(1920.0, 1080.0)), minimum());
        assert!((fitted.width - 1180.0).abs() < f64::EPSILON);
        assert!((fitted.height - 760.0).abs() < f64::EPSILON);
        assert!((fitted.min_width - 900.0).abs() < f64::EPSILON);
        assert!((fitted.min_height - 700.0).abs() < f64::EPSILON);
        assert_eq!(fitted.x, None);
    }

    #[test]
    fn a_1366x768_screen_at_125_percent_fits_the_default() {
        // 1366x768 physical at 125% is 1092.8x614.4 logical. The window manager
        // keeps a title bar and panel, so the whole monitor is the bound.
        let work = screen(1366.0 / 1.25, 768.0 / 1.25);
        let fitted = fit_window(preferred(), None, Some(work), minimum());
        let margin = f64::from(theme::layout::window_margin());
        assert!(
            fitted.width <= work.width - margin,
            "the default width {} does not fit {work:?}",
            fitted.width
        );
        assert!(
            fitted.height <= work.height - margin,
            "the default height {} does not fit {work:?}",
            fitted.height
        );
        assert!(
            fitted.min_height <= work.height,
            "the minimum height must follow the screen down"
        );
        assert!(
            fitted.min_width <= fitted.width && fitted.min_height <= fitted.height,
            "the minimum must not exceed the initial size"
        );
    }

    #[test]
    fn a_remembered_size_wins_when_it_fits() {
        let stored = WindowGeometry {
            width: 1400,
            height: 900,
            x: None,
            y: None,
        };
        let fitted = fit_window(
            preferred(),
            Some(stored),
            Some(screen(1920.0, 1080.0)),
            minimum(),
        );
        assert!((fitted.width - 1400.0).abs() < f64::EPSILON);
        assert!((fitted.height - 900.0).abs() < f64::EPSILON);
    }

    #[test]
    fn a_remembered_size_larger_than_the_screen_is_clamped() {
        let stored = WindowGeometry {
            width: 2400,
            height: 1400,
            x: None,
            y: None,
        };
        let fitted = fit_window(
            preferred(),
            Some(stored),
            Some(screen(1920.0, 1080.0)),
            minimum(),
        );
        assert!(fitted.width <= 1920.0);
        assert!(fitted.height <= 1080.0);
    }

    #[test]
    fn a_remembered_position_is_kept_inside_the_screen() {
        let stored = WindowGeometry {
            width: 1000,
            height: 700,
            x: Some(-500),
            y: Some(2000),
        };
        let fitted = fit_window(
            preferred(),
            Some(stored),
            Some(screen(1920.0, 1080.0)),
            minimum(),
        );
        assert_eq!(fitted.x, Some(0.0));
        assert_eq!(fitted.y, Some(1080.0 - 700.0));
    }

    #[test]
    fn without_a_monitor_the_remembered_geometry_is_used_as_is() {
        let stored = WindowGeometry {
            width: 1280,
            height: 800,
            x: Some(64),
            y: Some(48),
        };
        let fitted = fit_window(preferred(), Some(stored), None, minimum());
        assert!((fitted.width - 1280.0).abs() < f64::EPSILON);
        assert!((fitted.height - 800.0).abs() < f64::EPSILON);
        assert_eq!(fitted.x, Some(64.0));
        assert_eq!(fitted.y, Some(48.0));
    }

    #[test]
    fn a_remembered_position_selects_the_monitor_containing_it() {
        let areas = [
            screen(1920.0, 1080.0),
            WorkArea {
                x: -1920.0,
                y: 0.0,
                width: 1920.0,
                height: 1080.0,
            },
        ];
        let stored = WindowGeometry {
            width: 1000,
            height: 700,
            x: Some(-500),
            y: Some(200),
        };
        assert_eq!(
            work_area_containing(&areas, Some(stored)),
            Some(areas[1]),
            "the secondary monitor was not selected"
        );
        assert_eq!(work_area_containing(&areas, None), None);
    }

    #[test]
    fn a_window_on_a_second_monitor_stays_inside_that_monitor() {
        let work = WorkArea {
            x: -1920.0,
            y: 0.0,
            width: 1920.0,
            height: 1080.0,
        };
        let stored = WindowGeometry {
            width: 1000,
            height: 700,
            x: Some(-500),
            y: Some(200),
        };
        let fitted = fit_window(preferred(), Some(stored), Some(work), minimum());
        assert_eq!(fitted.x, Some(-1000.0));
        assert_eq!(fitted.y, Some(200.0));
        let displaced = WindowGeometry {
            width: 1000,
            height: 700,
            x: Some(500),
            y: Some(2000),
        };
        let fitted = fit_window(preferred(), Some(displaced), Some(work), minimum());
        assert_eq!(fitted.x, Some(-1000.0));
        assert_eq!(fitted.y, Some(380.0));
    }

    #[test]
    fn a_150_percent_display_still_fits_the_default() {
        // 2560x1440 physical at 150% is 1706.7x960 logical.
        let work = screen(2560.0 / 1.5, 1440.0 / 1.5);
        let fitted = fit_window(preferred(), None, Some(work), minimum());
        assert!(fitted.width <= work.width - 16.0);
        assert!(fitted.height <= work.height - 16.0);
        assert_eq!(fitted.min_width, 900.0);
    }

    #[test]
    fn a_wayland_session_owns_the_window_position() {
        assert!(!wayland_display_is_set(None));
        assert!(!wayland_display_is_set(Some(std::ffi::OsStr::new(""))));
        assert!(wayland_display_is_set(Some(std::ffi::OsStr::new(
            "wayland-1"
        ))));
    }

    #[test]
    fn an_invalid_remembered_size_falls_back_to_the_default() {
        let stored = WindowGeometry {
            width: 0,
            height: 0,
            x: None,
            y: None,
        };
        let fitted = fit_window(preferred(), Some(stored), None, minimum());
        assert!((fitted.width - 1180.0).abs() < f64::EPSILON);
        assert!((fitted.height - 760.0).abs() < f64::EPSILON);
    }
}
