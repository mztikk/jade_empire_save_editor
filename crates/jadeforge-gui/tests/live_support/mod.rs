//! Support code for the windowed (not `freya-testing`) UI suite.
//!
//! The suite is split into:
//!
//! * [`probe`]: a Freya plugin that publishes the live window's text and facts
//!   to the test thread and exposes the event-loop handle for input and close.
//! * [`kwin`]: window lookup, activation, geometry and screenshots through
//!   KWin's D-Bus interface (KDE Plasma only).
//! * [`fake_input`]: real pointer/keyboard events through KWin's Wayland
//!   fake-input protocol (KDE Plasma only).
//! * [`driver`]: the backend-independent click/type/wait API used by tests.
//! * [`scenarios`]: the scenarios themselves.

pub mod driver;
pub mod fake_input;
pub mod kwin;
pub mod orchestrator;
pub mod probe;
pub mod scenarios;

/// The application id used by the windowed test processes.
///
/// It is unique per scenario child, so KWin can find the test window even when
/// another JadeForge instance is open. The window title stays the production
/// title.
#[must_use]
pub fn live_app_id() -> String {
    format!("jadeforge-live-{}", std::process::id())
}
