//! `JadeForge` - a savegame editor for Jade Empire.

use freya::prelude::*;
use jadeforge_gui::app::{app, window_config};
use jadeforge_gui::geometry::WindowGeometryPlugin;
use jadeforge_gui::renderer;

fn main() {
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
