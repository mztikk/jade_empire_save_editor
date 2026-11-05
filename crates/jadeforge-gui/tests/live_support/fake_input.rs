//! Synthetic pointer and keyboard input through KWin's fake-input protocol.
//!
//! `org_kde_kwin_fake_input` is the Wayland equivalent of XTest. KWin exposes
//! it to non-sandboxed clients and treats an `authenticate` request as consent
//! (the current implementation is intentionally a TODO upstream). The test
//! connects as a normal Wayland client, so the application receives real
//! compositor input: pointer motion, button events and key events.

use std::time::Duration;

use wayland_client::globals::GlobalListContents;
use wayland_client::protocol::wl_registry;
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle};
use wayland_protocols_plasma::fake_input::client::org_kde_kwin_fake_input::OrgKdeKwinFakeInput;

/// Linux input event codes used by the fake-input protocol.
const BTN_LEFT: u32 = 0x110;
const KEY_LEFTSHIFT: u32 = 42;
const KEY_ENTER: u32 = 28;

/// Letter keycodes for a US layout, indexed from `a`.
const LETTERS: [u32; 26] = [
    30, 48, 46, 32, 18, 33, 34, 35, 23, 36, 37, 38, 50, 49, 24, 25, 16, 19, 31, 20, 22, 47, 17, 45,
    21, 44,
];

/// A connected fake-input device.
pub struct FakeInput {
    connection: Connection,
    /// Kept alive so the registry queue outlives the bound proxy.
    _queue: EventQueue<State>,
    device: OrgKdeKwinFakeInput,
}

struct State;

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<OrgKdeKwinFakeInput, ()> for State {
    fn event(
        _: &mut Self,
        _: &OrgKdeKwinFakeInput,
        _: <OrgKdeKwinFakeInput as wayland_client::Proxy>::Event,
        _state: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl FakeInput {
    /// Connects to the current Wayland compositor and authenticates.
    ///
    /// Fails when the compositor does not expose the KDE fake-input protocol
    /// (for example GNOME, Weston or an X11 session), which makes the caller
    /// fall back to the plugin backend.
    pub fn connect() -> Result<Self, String> {
        let connection = Connection::connect_to_env()
            .map_err(|error| format!("cannot connect to the Wayland compositor: {error}"))?;
        let (globals, queue) =
            wayland_client::globals::registry_queue_init::<State>(&connection)
                .map_err(|error| format!("cannot enumerate Wayland globals: {error}"))?;
        let handle = queue.handle();
        let device: OrgKdeKwinFakeInput = globals
            .bind(&handle, 1..=5, ())
            .map_err(|error| format!("org_kde_kwin_fake_input is unavailable: {error}"))?;
        if device.version() < 4 {
            return Err(format!(
                "KWin fake input is too old for absolute motion and keys (version {})",
                device.version()
            ));
        }
        let input = Self {
            connection,
            _queue: queue,
            device,
        };
        input.device.authenticate(
            "JadeForge windowed UI tests".to_owned(),
            "automated UI tests driving the real application window".to_owned(),
        );
        input.flush()?;
        std::thread::sleep(Duration::from_millis(100));
        Ok(input)
    }

    /// Moves the real compositor pointer to global `(x, y)`.
    pub fn move_pointer(&self, x: f64, y: f64) -> Result<(), String> {
        self.device.pointer_motion_absolute(x, y);
        self.flush()
    }

    /// Clicks the left mouse button at global `(x, y)`.
    pub fn click(&self, x: f64, y: f64) -> Result<(), String> {
        self.move_pointer(x, y)?;
        std::thread::sleep(Duration::from_millis(40));
        self.device.button(BTN_LEFT, 1);
        self.flush()?;
        std::thread::sleep(Duration::from_millis(40));
        self.device.button(BTN_LEFT, 0);
        self.flush()?;
        std::thread::sleep(Duration::from_millis(120));
        Ok(())
    }

    /// Types `text` using the current keyboard layout (US keycodes).
    pub fn type_text(&self, text: &str) -> Result<(), String> {
        for character in text.chars() {
            let (keycode, shift) = character_keycode(character)
                .ok_or_else(|| format!("no keycode for {character:?} in the live test"))?;
            if shift {
                self.key(KEY_LEFTSHIFT, true)?;
            }
            self.key(keycode, true)?;
            self.key(keycode, false)?;
            if shift {
                self.key(KEY_LEFTSHIFT, false)?;
            }
            std::thread::sleep(Duration::from_millis(18));
        }
        Ok(())
    }

    fn key(&self, code: u32, pressed: bool) -> Result<(), String> {
        self.device.keyboard_key(code, u32::from(pressed));
        self.flush()?;
        std::thread::sleep(Duration::from_millis(12));
        Ok(())
    }

    fn flush(&self) -> Result<(), String> {
        self.connection
            .flush()
            .map_err(|error| format!("cannot flush Wayland requests: {error}"))
    }
}

fn character_keycode(character: char) -> Option<(u32, bool)> {
    match character {
        'a'..='z' => Some((LETTERS[(character as u8 - b'a') as usize], false)),
        'A'..='Z' => Some((LETTERS[(character as u8 - b'A') as usize], true)),
        '0' => Some((11, false)),
        '1'..='9' => Some((u32::from(character as u8 - b'0') + 1, false)),
        ' ' => Some((57, false)),
        '\n' => Some((KEY_ENTER, false)),
        '-' => Some((12, false)),
        '_' => Some((12, true)),
        '=' => Some((13, false)),
        '+' => Some((13, true)),
        '[' => Some((26, false)),
        ']' => Some((27, false)),
        ';' => Some((39, false)),
        ':' => Some((39, true)),
        '\'' => Some((40, false)),
        '"' => Some((40, true)),
        ',' => Some((51, false)),
        '.' => Some((52, false)),
        '/' => Some((53, false)),
        '?' => Some((53, true)),
        '\\' => Some((43, false)),
        '`' => Some((41, false)),
        _ => None,
    }
}
