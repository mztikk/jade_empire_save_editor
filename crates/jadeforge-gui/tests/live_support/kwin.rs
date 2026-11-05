//! Window lookup, activation and screenshots through KWin's D-Bus interface.
//!
//! KDE Plasma exposes every window through KRunner's `/WindowsRunner` object
//! and `org.kde.KWin.getWindowInfo`. `busctl --json` gives us typed JSON, so
//! the tests do not have to parse the human-readable `qdbus` output. This is
//! only used on the KDE fallback path; the plugin input backend needs no KWin.

use std::path::Path;
use std::process::Command;

use serde_json::Value;

/// Geometry and identity of one KWin window.
#[derive(Clone, Debug)]
pub struct WindowInfo {
    /// Left edge of the frame, in compositor coordinates.
    pub x: f64,
    /// Top edge of the frame, in compositor coordinates.
    pub y: f64,
    /// Frame width, including decorations.
    pub width: f64,
    /// Frame height, including decorations.
    pub height: f64,
    /// The window's `xdg_toplevel` app id (`resourceClass` in KWin).
    pub resource_class: String,
}

impl WindowInfo {
    /// Top-left corner of the client area.
    ///
    /// KWin reports the frame rectangle. JadeForge's window manager decoration
    /// is a title bar at the top, so the client area starts below the
    /// difference between the frame and the known inner size.
    pub fn client_origin(&self, inner_width: u32, inner_height: u32) -> Result<(f64, f64), String> {
        let horizontal = (self.width - f64::from(inner_width)) / 2.0;
        let vertical = self.height - f64::from(inner_height);
        if horizontal < -1.0 || vertical < -1.0 {
            return Err(format!(
                "window frame {}x{} is smaller than the client area {inner_width}x{inner_height}",
                self.width, self.height
            ));
        }
        Ok((self.x + horizontal.max(0.0), self.y + vertical.max(0.0)))
    }
}

/// One matching window from KRunner, with the id needed to activate it.
#[derive(Clone, Debug)]
struct WindowMatch {
    id: String,
    uuid: String,
}

/// A window found by application id, ready to be activated and measured.
pub struct KWin {
    id: String,
}

impl KWin {
    /// Finds the window with caption `title` whose `xdg_toplevel` app id is
    /// `app_id`.
    ///
    /// The app id is unique per scenario child, which separates the test window
    /// from any other JadeForge instance. Returns an error when KWin, KRunner
    /// or `busctl` are unavailable, so the caller can fall back to the plugin
    /// input backend.
    pub fn find(app_id: &str, title: &str) -> Result<Self, String> {
        let matches = match_windows(title)?;
        for candidate in matches {
            let Ok(info) = window_info(&candidate.uuid) else {
                continue;
            };
            if info.resource_class == app_id {
                return Ok(Self { id: candidate.id });
            }
        }
        Err(format!("no KWin window has the app id {app_id:?}"))
    }

    /// Asks KWin to activate (raise and focus) the window.
    pub fn activate(&self) -> Result<(), String> {
        run(&[
            "call",
            "org.kde.KWin",
            "/WindowsRunner",
            "org.kde.krunner1",
            "Run",
            "ss",
            &self.id,
            "",
        ])?;
        Ok(())
    }

    /// Reads the current frame geometry.
    pub fn info(&self) -> Result<WindowInfo, String> {
        let value = call_json(&[
            "call",
            "org.kde.KWin",
            "/KWin",
            "org.kde.KWin",
            "getWindowInfo",
            "s",
            &self.uuid()?,
            "--json=short",
        ])?;
        let map = value
            .get("data")
            .and_then(Value::as_array)
            .and_then(|data| data.first())
            .and_then(Value::as_object)
            .ok_or_else(|| "KWin returned no window info".to_owned())?;
        if map.is_empty() {
            return Err("KWin does not know this window anymore".to_owned());
        }
        parse_window_info(map)
    }

    /// Captures the active window's client area and returns the PNG size.
    ///
    /// The caller activates the window first. Spectacle runs in background
    /// mode (`-b`) without a notification or GUI (`-n`), and excludes the
    /// window shadow (`-S`) and decorations (`-e`) so the image matches the
    /// inner size winit reports.
    pub fn capture_active(path: &Path) -> Result<(u32, u32), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
        }
        if path.exists() {
            let _ = std::fs::remove_file(path);
        }
        let output = Command::new("spectacle")
            .args(["-b", "-n", "-a", "-S", "-e", "-o"])
            .arg(path)
            .output()
            .map_err(|error| format!("cannot run spectacle: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "spectacle failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        image::image_dimensions(path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))
    }

    fn uuid(&self) -> Result<String, String> {
        self.id
            .split_once('{')
            .and_then(|(_, rest)| rest.split_once('}'))
            .map(|(uuid, _)| uuid.to_owned())
            .ok_or_else(|| format!("KRunner match id has no window UUID: {}", self.id))
    }
}

fn match_windows(title: &str) -> Result<Vec<WindowMatch>, String> {
    let value = call_json(&[
        "call",
        "org.kde.KWin",
        "/WindowsRunner",
        "org.kde.krunner1",
        "Match",
        "s",
        title,
        "--json=short",
    ])?;
    let data = value
        .get("data")
        .ok_or_else(|| "KRunner returned no data".to_owned())?;
    let mut ids = Vec::new();
    collect_match_ids(data, &mut ids);
    ids.into_iter()
        .map(|id| {
            let uuid = id
                .split_once('{')
                .and_then(|(_, rest)| rest.split_once('}'))
                .map(|(uuid, _)| uuid.to_owned())
                .ok_or_else(|| format!("KRunner match id has no window UUID: {id}"))?;
            Ok(WindowMatch { id, uuid })
        })
        .collect()
}

fn window_info(uuid: &str) -> Result<WindowInfo, String> {
    let value = call_json(&[
        "call",
        "org.kde.KWin",
        "/KWin",
        "org.kde.KWin",
        "getWindowInfo",
        "s",
        uuid,
        "--json=short",
    ])?;
    let map = value
        .get("data")
        .and_then(Value::as_array)
        .and_then(|data| data.first())
        .and_then(Value::as_object)
        .ok_or_else(|| "KWin returned no window info".to_owned())?;
    parse_window_info(map)
}

fn parse_window_info(map: &serde_json::Map<String, Value>) -> Result<WindowInfo, String> {
    let field = |name: &str| map.get(name).and_then(|value| value.get("data"));
    let number = |name: &str| {
        field(name)
            .and_then(Value::as_f64)
            .ok_or_else(|| format!("KWin window info has no numeric {name}"))
    };
    Ok(WindowInfo {
        x: number("x")?,
        y: number("y")?,
        width: number("width")?,
        height: number("height")?,
        resource_class: field("resourceClass")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
    })
}

fn collect_match_ids(value: &Value, ids: &mut Vec<String>) {
    let Some(array) = value.as_array() else {
        return;
    };
    if let Some(first) = array.first().and_then(Value::as_str)
        && first.contains('{')
        && first.starts_with("0_")
        && array.len() >= 2
    {
        ids.push(first.to_owned());
        return;
    }
    for item in array {
        collect_match_ids(item, ids);
    }
}

fn call_json(arguments: &[&str]) -> Result<Value, String> {
    let stdout = run(arguments)?;
    serde_json::from_str(&stdout)
        .map_err(|error| format!("cannot parse busctl JSON: {error}\n{stdout}"))
}

fn run(arguments: &[&str]) -> Result<String, String> {
    let output = Command::new("busctl")
        .arg("--user")
        .args(arguments)
        .output()
        .map_err(|error| format!("cannot run busctl: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "busctl {} failed: {}",
            arguments.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}
