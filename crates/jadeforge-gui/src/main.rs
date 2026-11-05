//! `JadeForge` - a savegame editor for Jade Empire.

use freya::prelude::*;
use jadeforge_gui::app::{app, window_config};

fn main() {
    launch(LaunchConfig::new().with_window(window_config(app)));
}
