//! `JadeForge` - a savegame editor for Jade Empire.

use freya::prelude::*;
use jadeforge_gui::app::{app, window_config};
use jadeforge_gui::geometry::WindowGeometryPlugin;

fn main() {
    launch(
        LaunchConfig::new()
            .with_plugin(WindowGeometryPlugin)
            .with_window(window_config(app)),
    );
}
