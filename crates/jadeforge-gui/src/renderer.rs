//! Renderer selection and GPU memory tuning.
//!
//! Freya renders through Skia. By default it picks the best available GPU
//! backend (Vulkan on Linux, Metal on macOS, OpenGL elsewhere), which keeps the
//! interface smooth but also maps a large graphics driver and Skia's GPU
//! resource cache into the process. A save editor is mostly static and works
//! from any renderer, so the choice is exposed through
//! [`RENDERER_ENV`] for users who prefer a smaller memory footprint:
//!
//! ```text
//! JADEFORGE_RENDERER=software jadeforge
//! ```
//!
//! Unsetting the variable (or setting it to `auto`) returns to the default GPU
//! backend. `JADEFORGE_RENDERER` wins over Freya's own `FREYA_RENDERER`; when
//! this variable is unset, `FREYA_RENDERER` keeps working as before.
//!
//! The backends are Freya's, not ours. Linux and Windows ship OpenGL and
//! Vulkan; macOS only has Metal, so any GPU value there ends up on Metal.
//! Freya 0.5 has no Direct3D driver, even though Skia itself can render with
//! D3D12, so there is no value for it here. A backend that fails to initialize
//! falls back (Vulkan to OpenGL to software), which is why
//! [`RendererLogPlugin`] reports what the window actually got.

use std::io::Write as _;

use freya::prelude::{GpuPreference, GpuResourceCacheLimit, RendererPreference};
use freya_winit::plugins::{FreyaPlugin, PluginEvent, PluginHandle};

/// Environment variable that overrides the renderer for this run.
///
/// Accepted values, case-insensitive: `auto`, `software` (or `cpu`), `opengl`
/// (or `gl`) and `vulkan` (or `vk`). An unknown value falls back to `auto` and
/// reports a warning when stderr is connected.
pub const RENDERER_ENV: &str = "JADEFORGE_RENDERER";

/// Skia GPU resource cache limit used by the default renderer, in MiB.
///
/// Skia defaults to 256 MiB, which it happily fills with resources it no longer
/// needs; on an integrated GPU that memory is system RAM. The editor's largest
/// GPU resource is a 512 px savegame screenshot (1 MiB), and its glyph atlases
/// only need a few MiB, so 32 MiB leaves plenty of headroom without letting the
/// cache grow into the hundreds of megabytes.
pub const GPU_RESOURCE_CACHE_LIMIT_MB: usize = 32;

/// Renderer requested for the main window.
///
/// Reads [`RENDERER_ENV`] once at startup; an unset variable keeps Freya's
/// automatic choice.
#[must_use]
pub fn preference() -> RendererPreference {
    let Ok(value) = std::env::var(RENDERER_ENV) else {
        return RendererPreference::Auto;
    };
    parse(&value).unwrap_or_else(|| {
        report(format_args!(
            "jadeforge: ignoring unknown {RENDERER_ENV} value {value:?}; \
             expected auto, software, opengl or vulkan"
        ));
        RendererPreference::Auto
    })
}

/// Parses one [`RENDERER_ENV`] value.
fn parse(value: &str) -> Option<RendererPreference> {
    match value.trim().to_ascii_lowercase().as_str() {
        "" | "auto" => Some(RendererPreference::Auto),
        "software" | "cpu" => Some(RendererPreference::Software),
        "opengl" | "gl" => Some(RendererPreference::OpenGl),
        "vulkan" | "vk" => Some(RendererPreference::Vulkan {
            gpu_preference: GpuPreference::Auto,
        }),
        _ => None,
    }
}

/// GPU resource cache limit handed to Freya for every window.
#[must_use]
pub fn gpu_resource_cache_limit() -> GpuResourceCacheLimit {
    GpuResourceCacheLimit::from_mb(GPU_RESOURCE_CACHE_LIMIT_MB)
}

/// Reports the renderer a window actually got, after any fallback.
///
/// Asking for a backend is not a guarantee that it is the one drawing: Freya
/// falls back when initialization fails, most visibly for Vulkan on machines
/// without a working driver. One line on stderr (`jadeforge: renderer ...`)
/// makes the real choice visible when the editor is started from a terminal or
/// its journal is read, and when stderr is redirected to a file on Windows.
pub struct RendererLogPlugin;

impl FreyaPlugin for RendererLogPlugin {
    fn plugin_id(&self) -> &'static str {
        "jadeforge-renderer-log"
    }

    fn on_event(&mut self, event: &mut PluginEvent, _handle: PluginHandle) {
        let (driver, gpu) = match event {
            PluginEvent::WindowCreated {
                graphics_driver,
                gpu_name,
                ..
            }
            | PluginEvent::GraphicsDriverChanged {
                graphics_driver,
                gpu_name,
                ..
            } => (*graphics_driver, *gpu_name),
            _ => return,
        };
        report(format_args!(
            "jadeforge: renderer {}",
            describe(driver, gpu)
        ));
    }
}

/// Writes one diagnostic line to stderr, ignoring a missing or closed stderr.
///
/// A Windows GUI-subsystem process started from Explorer has no stderr handle,
/// and `eprintln!` panics when that write fails. Diagnostics are best effort;
/// redirect stderr to a file when the renderer line is needed.
fn report(arguments: std::fmt::Arguments<'_>) {
    let _ = writeln!(std::io::stderr(), "{arguments}");
}

/// Human readable `driver (gpu)` line.
fn describe(driver: &str, gpu: Option<&str>) -> String {
    match gpu.filter(|name| !name.is_empty()) {
        Some(gpu) => format!("{driver} ({gpu})"),
        None => driver.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn software_can_be_selected_and_reverted() {
        assert_eq!(parse("software"), Some(RendererPreference::Software));
        assert_eq!(parse(" CPU "), Some(RendererPreference::Software));
        assert_eq!(parse("auto"), Some(RendererPreference::Auto));
        assert_eq!(parse(""), Some(RendererPreference::Auto));
    }

    #[test]
    fn gpu_backends_are_spelled_out() {
        assert_eq!(parse("opengl"), Some(RendererPreference::OpenGl));
        assert_eq!(parse("GL"), Some(RendererPreference::OpenGl));
        assert_eq!(
            parse("vulkan"),
            Some(RendererPreference::Vulkan {
                gpu_preference: GpuPreference::Auto
            })
        );
        assert_eq!(
            parse("vk"),
            Some(RendererPreference::Vulkan {
                gpu_preference: GpuPreference::Auto
            })
        );
    }

    #[test]
    fn unknown_values_are_rejected_so_the_caller_can_warn() {
        assert_eq!(parse("voodoo"), None);
        assert_eq!(parse("metal"), None);
    }

    #[test]
    fn the_gpu_cache_limit_leaves_room_for_the_screenshot_and_glyphs() {
        assert_eq!(gpu_resource_cache_limit().bytes(), 32 * 1024 * 1024);
    }

    #[test]
    fn the_renderer_line_names_the_driver_and_gpu_when_known() {
        assert_eq!(
            describe("Vulkan", Some("AMD Radeon RX 9070 XT")),
            "Vulkan (AMD Radeon RX 9070 XT)"
        );
        assert_eq!(describe("Software", None), "Software");
        assert_eq!(describe("OpenGL", Some("")), "OpenGL");
    }
}
