//! `JadeForge` - a savegame editor for Jade Empire.

// Windows links binaries as console programs by default, so Explorer would open
// a console window next to the editor. The GUI has no console interface;
// redirect stderr to a file when the renderer diagnostics are needed.
#![cfg_attr(windows, windows_subsystem = "windows")]

use freya::prelude::*;
use jadeforge_gui::app::{app, window_config};
use jadeforge_gui::geometry::WindowGeometryPlugin;
use jadeforge_gui::renderer;

fn main() {
    // winit reports no desktop theme on Linux, so a System choice must learn it
    // before the first frame instead of resolving to Freya's light fallback.
    #[cfg(target_os = "linux")]
    jadeforge_gui::system_theme::start();

    launch(
        LaunchConfig::new()
            // Skia's default cache is 256 MiB; this window never needs more
            // than a few tiles and one screenshot.
            .with_gpu_resource_cache_limit(renderer::gpu_resource_cache_limit())
            .with_plugin(WindowGeometryPlugin)
            .with_plugin(renderer::RendererLogPlugin)
            .with_window(window_config(app)),
    );
}
