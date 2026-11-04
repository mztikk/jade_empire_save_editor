//! `JadeForge` - a savegame editor for Jade Empire.

use freya::prelude::*;
use jadeforge_gui::app::app;

fn main() {
    launch(
        LaunchConfig::new().with_window(
            WindowConfig::new(app)
                .with_title("JadeForge")
                .with_size(1180.0, 760.0)
                .with_min_size(900.0, 600.0),
        ),
    );
}
