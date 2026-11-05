//! Windowed UI tests: the real application window, not `freya-testing`.
//!
//! These tests run the shipped Freya application through `freya-winit`, open a
//! real Wayland window and drive it with real input. On KDE Plasma the input
//! travels through KWin's `org_kde_kwin_fake_input` protocol; on other
//! compositors a test plugin posts the same platform events into the real
//! window's event loop. Both paths render with Skia/GL on the compositor.
//!
//! **Never run this suite without explicit permission.** It opens real windows,
//! takes focus, moves the pointer and types on the machine's desktop; only use
//! it when a headless UI test cannot cover the case or the bug is in the real
//! window (`AGENTS.md` rule 13).
//!
//! The suite is opt-in so the normal headless test run never opens windows:
//!
//! ```sh
//! just live
//! # or
//! JADEFORGE_LIVE=1 cargo test -p jadeforge-gui --test live -- --nocapture
//! ```
//!
//! Run it from a real Wayland desktop session, or from a VM with a real
//! desktop. It is deliberately not part of CI: it opens windows, takes focus
//! and needs a compositor. Set `JADEFORGE_LIVE_REQUIRE=1` to turn a missing
//! display into a failure instead of a skip.

// Tests assert by panicking, and the scenario runner turns those panics into
// a failing exit status.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
// The docs name desktop products (KWin, KRunner, XTest) and the application;
// backticking every product name would hurt the prose.
#![allow(clippy::doc_markdown)]

#[cfg(target_os = "linux")]
mod live_support;
#[cfg(target_os = "linux")]
mod support;

#[cfg(target_os = "linux")]
fn main() -> std::process::ExitCode {
    live_support::orchestrator::run()
}

#[cfg(not(target_os = "linux"))]
fn main() -> std::process::ExitCode {
    println!("skipping the windowed UI tests: Wayland testing is implemented for Linux only");
    std::process::ExitCode::SUCCESS
}
