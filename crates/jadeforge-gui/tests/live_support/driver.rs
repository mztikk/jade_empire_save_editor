//! Test driver: finds text in the live window and clicks/types into it.
//!
//! Two input backends exist:
//!
//! * `plugin` posts [`PlatformEvent`]s through the window's plugin handle. The
//!   events still go through the windowed renderer and its event measurer, and
//!   the window is a real compositor window, but no OS-level input is involved.
//!   This backend works under any Wayland compositor, including Weston.
//! * `kwin` injects real pointer and key events through KWin's fake-input
//!   protocol. This is the default on KDE Plasma and exercises the full input
//!   path from the compositor to the application.

use std::path::Path;
use std::time::{Duration, Instant};

use freya::prelude::*;
use freya_core::events::platform::{KeyboardEventName, MouseEventName, PlatformEvent};

use super::fake_input::FakeInput;
use super::kwin::KWin;
use super::live_app_id;
use super::probe::{NodeSnapshot, Probe, WindowSnapshot};

/// Which input path the driver uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputMode {
    /// Synthetic events through the Freya plugin handle.
    Plugin,
    /// Real compositor input through KWin's fake-input protocol.
    KWin,
}

type Origin = (f64, f64);

/// A driver bound to one live window.
pub struct Driver {
    probe: Probe,
    input: InputBackend,
    window: WindowSnapshot,
}

enum InputBackend {
    Plugin,
    KWin {
        kwin: KWin,
        fake: FakeInput,
        origin: Origin,
    },
}

impl Driver {
    /// Attaches with an explicit backend, or with `None` to try real KWin
    /// input and fall back to plugin events.
    pub fn attach_with_fallback(
        probe: Probe,
        mode: Option<InputMode>,
        wait: Duration,
    ) -> Result<Self, String> {
        match mode {
            Some(mode) => Self::attach(probe, mode, wait),
            None => match Self::attach(probe.clone(), InputMode::KWin, wait) {
                Ok(driver) => Ok(driver),
                Err(error) => {
                    eprintln!(
                        "note: KWin input is unavailable ({error}); falling back to plugin events"
                    );
                    Self::attach(probe, InputMode::Plugin, wait)
                }
            },
        }
    }

    /// Waits for the window and attaches an input backend.
    ///
    /// The caller chooses the backend; [`Driver::attach_with_fallback`] does
    /// the auto-detection.
    pub fn attach(probe: Probe, mode: InputMode, wait: Duration) -> Result<Self, String> {
        let deadline = Instant::now() + wait;
        loop {
            if probe.ready() && probe.window().width > 0 && probe.window().frames > 0 {
                break;
            }
            if probe.window().closed {
                return Err("the window closed before the test could attach".to_owned());
            }
            if Instant::now() >= deadline {
                return Err("the application window did not appear".to_owned());
            }
            probe.wake();
            std::thread::sleep(Duration::from_millis(25));
        }
        let window = probe.window();
        if !window.wayland {
            return Err(format!(
                "JadeForge opened through X11, but these tests are Wayland-only (WAYLAND_DISPLAY={:?})",
                std::env::var_os("WAYLAND_DISPLAY")
            ));
        }
        if (window.scale - 1.0).abs() > f64::EPSILON {
            return Err(format!(
                "the live window runs at scale {}, but pointer coordinates are only implemented for scale 1.0",
                window.scale
            ));
        }
        let input = match mode {
            InputMode::Plugin => InputBackend::Plugin,
            InputMode::KWin => InputBackend::KWin {
                kwin: KWin::find(&live_app_id(), jadeforge_gui::app::WINDOW_TITLE)?,
                fake: FakeInput::connect()?,
                origin: (0.0, 0.0),
            },
        };
        let mut driver = Self {
            probe,
            input,
            window,
        };
        if let InputBackend::KWin { .. } = &driver.input {
            driver.refresh_origin()?;
            driver.activate()?;
        }
        Ok(driver)
    }

    /// The input backend this driver attached.
    #[must_use]
    pub fn input_mode(&self) -> InputMode {
        match self.input {
            InputBackend::Plugin => InputMode::Plugin,
            InputBackend::KWin { .. } => InputMode::KWin,
        }
    }

    /// The latest window facts.
    #[must_use]
    pub fn window(&self) -> WindowSnapshot {
        self.window.clone()
    }

    /// Waits until a node containing `needle` exists.
    pub fn wait_for_text(
        &mut self,
        needle: &str,
        timeout: Duration,
    ) -> Result<NodeSnapshot, String> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(node) = self.probe.find(needle) {
                return Ok(node);
            }
            if self.probe.window().closed {
                return Err(format!("window closed while waiting for {needle:?}"));
            }
            if Instant::now() >= deadline {
                let mut seen: Vec<String> = self
                    .probe
                    .nodes()
                    .into_iter()
                    .map(|node| node.text)
                    .take(40)
                    .collect();
                seen.sort();
                seen.dedup();
                return Err(format!(
                    "timed out waiting for {needle:?}; visible text: {seen:?}"
                ));
            }
            self.probe.wake();
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    /// Waits until a node whose text is exactly `text` exists.
    pub fn wait_for_exact(
        &mut self,
        text: &str,
        timeout: Duration,
    ) -> Result<NodeSnapshot, String> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(node) = self.probe.find_exact(text) {
                return Ok(node);
            }
            if self.probe.window().closed {
                return Err(format!(
                    "window closed while waiting for the exact text {text:?}"
                ));
            }
            if Instant::now() >= deadline {
                return Err(format!("timed out waiting for the exact text {text:?}"));
            }
            self.probe.wake();
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    /// Clicks the centre of `node`.
    pub fn click(&mut self, node: &NodeSnapshot) -> Result<(), String> {
        let (x, y) = node.center();
        if matches!(&self.input, InputBackend::KWin { .. }) {
            self.activate()?;
            self.refresh_origin()?;
        }
        match &mut self.input {
            InputBackend::Plugin => {
                self.probe
                    .send_event(mouse_event(MouseEventName::MouseMove, x, y));
                std::thread::sleep(Duration::from_millis(15));
                self.probe
                    .send_event(mouse_event(MouseEventName::MouseDown, x, y));
                std::thread::sleep(Duration::from_millis(15));
                self.probe
                    .send_event(mouse_event(MouseEventName::MouseUp, x, y));
                std::thread::sleep(Duration::from_millis(30));
            }
            InputBackend::KWin { fake, origin, .. } => {
                let global = (origin.0 + x, origin.1 + y);
                fake.click(global.0, global.1)?;
            }
        }
        Ok(())
    }

    /// Clicks the first node whose text is exactly `text`.
    pub fn click_exact(&mut self, text: &str, timeout: Duration) -> Result<NodeSnapshot, String> {
        let node = self.wait_for_exact(text, timeout)?;
        self.click(&node)?;
        Ok(node)
    }

    /// Types `text` into the focused field.
    pub fn type_text(&mut self, text: &str) -> Result<(), String> {
        match &mut self.input {
            InputBackend::Plugin => {
                self.probe.send_event(PlatformEvent::Keyboard {
                    name: KeyboardEventName::KeyDown,
                    key: Key::Character(text.to_owned()),
                    code: Code::Unidentified,
                    modifiers: Modifiers::default(),
                });
                std::thread::sleep(Duration::from_millis(30));
            }
            InputBackend::KWin { kwin, fake, .. } => {
                kwin.activate()?;
                fake.type_text(text)?;
            }
        }
        Ok(())
    }

    /// Presses and releases one named key through the active backend.
    pub fn press_key(&mut self, key: &Key) -> Result<(), String> {
        match &mut self.input {
            InputBackend::Plugin => {
                for name in [KeyboardEventName::KeyDown, KeyboardEventName::KeyUp] {
                    self.probe.send_event(PlatformEvent::Keyboard {
                        name,
                        key: key.clone(),
                        code: Code::Unidentified,
                        modifiers: Modifiers::default(),
                    });
                    std::thread::sleep(Duration::from_millis(20));
                }
            }
            InputBackend::KWin { kwin, fake, .. } => {
                kwin.activate()?;
                let code =
                    live_keycode(key).ok_or_else(|| format!("no live keycode for {key:?}"))?;
                fake.tap_keycode(code)?;
                std::thread::sleep(Duration::from_millis(60));
            }
        }
        Ok(())
    }

    /// Captures the active window with Spectacle.
    ///
    /// Only available on the KWin backend; the plugin backend runs on
    /// compositors without a screenshot CLI.
    pub fn screenshot(&mut self, path: &Path) -> Result<(u32, u32), String> {
        if matches!(self.input, InputBackend::Plugin) {
            return Err("screenshots need the KWin input backend".to_owned());
        }
        self.activate()?;
        self.refresh_origin()?;
        std::thread::sleep(Duration::from_millis(150));
        match &mut self.input {
            InputBackend::Plugin => Err("screenshots need the KWin input backend".to_owned()),
            InputBackend::KWin { .. } => KWin::capture_active(path),
        }
    }

    /// Activates and focuses the window.
    pub fn activate(&mut self) -> Result<(), String> {
        if let InputBackend::KWin { kwin, .. } = &mut self.input {
            kwin.activate()?;
            std::thread::sleep(Duration::from_millis(80));
        }
        Ok(())
    }

    fn refresh_origin(&mut self) -> Result<(), String> {
        let updated = match &self.input {
            InputBackend::Plugin => None,
            InputBackend::KWin { kwin, .. } => Some(self.origin_for(kwin)?),
        };
        if let (Some(updated), InputBackend::KWin { origin, .. }) = (updated, &mut self.input) {
            *origin = updated;
        }
        Ok(())
    }

    fn origin_for(&self, kwin: &KWin) -> Result<Origin, String> {
        let info = kwin.info()?;
        info.client_origin(self.window.width, self.window.height)
    }
}

/// Linux evdev keycodes for the named keys used by the windowed scenarios.
fn live_keycode(key: &Key) -> Option<u32> {
    match key {
        Key::Named(NamedKey::Tab) => Some(15),
        Key::Named(NamedKey::Enter) => Some(28),
        Key::Named(NamedKey::Escape) => Some(1),
        Key::Named(NamedKey::Home) => Some(102),
        Key::Named(NamedKey::ArrowUp) => Some(103),
        Key::Named(NamedKey::End) => Some(107),
        Key::Named(NamedKey::ArrowDown) => Some(108),
        _ => None,
    }
}

fn mouse_event(name: MouseEventName, x: f64, y: f64) -> PlatformEvent {
    PlatformEvent::Mouse {
        name,
        cursor: CursorPoint::new(x, y),
        button: Some(MouseButton::Left),
    }
}
