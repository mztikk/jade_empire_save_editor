//! Linux system colour-scheme detection.
//!
//! winit 0.30 does not report the desktop theme on Linux: X11 is unsupported
//! and Wayland only exposes per-window overrides. Freya therefore resolves
//! every Linux window to `PreferredTheme::Light`, which made a `System` choice
//! ignore a dark KDE or GNOME desktop. The freedesktop appearance portal is the
//! supported source; KDE also records the same choice in `kdeglobals`, which is
//! read first so the first frame is already correct without waiting for D-Bus.
//!
//! [`start`] runs in the binary before the window exists. The headless suite
//! never calls it, so tests keep the light platform preference and never open a
//! session bus.

use std::cell::Cell;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::time::Duration;

use freya::prelude::PreferredTheme;
use futures_channel::mpsc::{self, UnboundedReceiver};
use mundy::{ColorScheme, Interest, Preferences, Subscription};

/// How long the first frame waits for the portal before using the fallback.
const PORTAL_TIMEOUT: Duration = Duration::from_millis(200);

const UNKNOWN: u8 = 0;
const LIGHT: u8 = 1;
const DARK: u8 = 2;

/// Set once the binary runs [`start`], which also enables the live watcher.
static ENABLED: AtomicBool = AtomicBool::new(false);
/// Last detected preference; `UNKNOWN` until something reports one.
static DETECTED: AtomicU8 = AtomicU8::new(UNKNOWN);

thread_local! {
    /// Test-only override, per thread so parallel tests cannot leak a theme.
    static TEST_DETECTED: Cell<Option<PreferredTheme>> = const { Cell::new(None) };
}

/// Detects the desktop preference and allows live updates.
///
/// Called once by the GUI binary before the window is created. Tests and the
/// snapshot example do not call this, so they keep the headless platform
/// preference instead of querying the real desktop.
pub fn start() {
    ENABLED.store(true, Ordering::Relaxed);
    if let Some(theme) = kde_preference().or_else(portal_preference) {
        store(theme);
    }
}

/// Whether [`start`] ran.
#[must_use]
pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// The desktop preference detected at startup, if one was available.
#[must_use]
pub fn detected() -> Option<PreferredTheme> {
    TEST_DETECTED.with(Cell::get).or_else(global_detected)
}

fn global_detected() -> Option<PreferredTheme> {
    match DETECTED.load(Ordering::Relaxed) {
        DARK => Some(PreferredTheme::Dark),
        LIGHT => Some(PreferredTheme::Light),
        _ => None,
    }
}

/// Subscribes to desktop preference changes.
///
/// The caller must keep the returned subscription alive; the receiver yields
/// the current preference and then every change. `mundy` runs the callback on
/// its own thread, so the receiver is meant to be polled from the UI thread.
pub fn subscribe() -> (Subscription, UnboundedReceiver<PreferredTheme>) {
    let (sender, receiver) = mpsc::unbounded();
    let subscription = Preferences::subscribe(Interest::ColorScheme, move |preferences| {
        if let Some(theme) = theme_of(preferences.color_scheme) {
            let _ = sender.unbounded_send(theme);
        }
    });
    (subscription, receiver)
}

/// Installs a preference for the current thread's test.
///
/// The headless runner reports a fixed light platform preference, so a test
/// that exercises the `System` choice uses this hook instead of a session bus.
#[doc(hidden)]
pub fn set_for_testing(theme: Option<PreferredTheme>) {
    TEST_DETECTED.with(|slot| slot.set(theme));
}

fn store(theme: PreferredTheme) {
    let value = match theme {
        PreferredTheme::Dark => DARK,
        PreferredTheme::Light => LIGHT,
    };
    DETECTED.store(value, Ordering::Relaxed);
}

fn theme_of(scheme: ColorScheme) -> Option<PreferredTheme> {
    match scheme {
        ColorScheme::Dark => Some(PreferredTheme::Dark),
        ColorScheme::Light => Some(PreferredTheme::Light),
        ColorScheme::NoPreference => None,
    }
}

fn portal_preference() -> Option<PreferredTheme> {
    Preferences::once_blocking(Interest::ColorScheme, PORTAL_TIMEOUT)
        .and_then(|preferences| theme_of(preferences.color_scheme))
}

fn kde_preference() -> Option<PreferredTheme> {
    if !is_kde() {
        return None;
    }
    let path = config_home()?.join("kdeglobals");
    let text = fs::read_to_string(path).ok()?;
    parse_kde_background(&text)
}

fn is_kde() -> bool {
    ["XDG_CURRENT_DESKTOP", "KDE_FULL_SESSION"]
        .into_iter()
        .filter_map(|name| std::env::var(name).ok())
        .any(|value| {
            value.eq_ignore_ascii_case("true")
                || value
                    .split(':')
                    .any(|part| part.eq_ignore_ascii_case("kde"))
        })
}

fn config_home() -> Option<PathBuf> {
    if let Some(directory) = std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
        return Some(PathBuf::from(directory));
    }
    std::env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .map(|home| PathBuf::from(home).join(".config"))
}

/// Reads `[Colors:Window] BackgroundNormal` from a `kdeglobals` file.
///
/// KDE derives the window background from the active colour scheme, so its
/// luminance is the same light/dark decision the appearance portal reports.
fn parse_kde_background(text: &str) -> Option<PreferredTheme> {
    let mut in_window = false;
    for line in text.lines() {
        let line = line.trim();
        if let Some(section) = line
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            in_window = section.eq_ignore_ascii_case("Colors:Window");
            continue;
        }
        if !in_window {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if !key.trim().eq_ignore_ascii_case("BackgroundNormal") {
            continue;
        }
        return parse_rgb(value).map(|rgb| {
            if is_dark(rgb) {
                PreferredTheme::Dark
            } else {
                PreferredTheme::Light
            }
        });
    }
    None
}

fn parse_rgb(value: &str) -> Option<[u8; 3]> {
    let mut channels = [0_u8; 3];
    let mut count = 0;
    for part in value.split(',') {
        let channel = part.trim().parse::<u8>().ok()?;
        *channels.get_mut(count)? = channel;
        count += 1;
    }
    (count == 3).then_some(channels)
}

fn is_dark(rgb: [u8; 3]) -> bool {
    let luminance = 0.2126 * linear(rgb[0]) + 0.7152 * linear(rgb[1]) + 0.0722 * linear(rgb[2]);
    luminance < 0.5
}

fn linear(channel: u8) -> f32 {
    let value = f32::from(channel) / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_kde_window_background_decides_the_scheme() {
        assert_eq!(
            parse_kde_background("[Colors:Window]\nBackgroundNormal=32,35,38\n"),
            Some(PreferredTheme::Dark)
        );
        assert_eq!(
            parse_kde_background("[Colors:Window]\nBackgroundNormal=239,240,241\n"),
            Some(PreferredTheme::Light)
        );
        assert_eq!(
            parse_kde_background("[Colors:Window]\nBackgroundNormal=41,44,48"),
            Some(PreferredTheme::Dark)
        );
    }

    #[test]
    fn other_sections_and_malformed_values_are_ignored() {
        assert_eq!(
            parse_kde_background("[Colors:Button]\nBackgroundNormal=32,35,38\n"),
            None
        );
        assert_eq!(
            parse_kde_background("[Colors:Window]\nBackgroundNormal=32,35\n"),
            None
        );
        assert_eq!(
            parse_kde_background("[Colors:Window]\nBackgroundNormal=not-a-colour\n"),
            None
        );
        assert_eq!(parse_kde_background(""), None);
    }

    #[test]
    fn black_is_dark_and_white_is_light() {
        assert!(is_dark([0, 0, 0]));
        assert!(!is_dark([255, 255, 255]));
    }

    #[test]
    fn portal_schemes_map_to_the_theme() {
        assert_eq!(theme_of(ColorScheme::Dark), Some(PreferredTheme::Dark));
        assert_eq!(theme_of(ColorScheme::Light), Some(PreferredTheme::Light));
        assert_eq!(theme_of(ColorScheme::NoPreference), None);
    }

    #[test]
    fn the_test_hook_is_thread_local() {
        set_for_testing(Some(PreferredTheme::Dark));
        assert_eq!(detected(), Some(PreferredTheme::Dark));
        set_for_testing(None);
    }

    /// Reads the real desktop instead of the fixture.
    ///
    /// Ignored by default because CI has no session bus and no colour-scheme
    /// preference; run it on a desktop after changing the theme:
    /// `cargo test -p jadeforge-gui --lib the_live_desktop -- --ignored --nocapture`.
    #[test]
    #[ignore = "requires a live desktop session"]
    fn the_live_desktop_reports_a_colour_scheme() {
        start();
        eprintln!("detected system theme: {:?}", detected());
        assert!(
            detected().is_some(),
            "the desktop reported no colour scheme"
        );
    }
}
