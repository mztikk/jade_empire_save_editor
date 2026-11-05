//! `JadeForge` - a savegame editor for Jade Empire.

use freya::prelude::*;
use jadeforge_gui::app::app;

fn main() {
    launch(
        LaunchConfig::new().with_window(
            WindowConfig::new(app)
                .with_title("JadeForge")
                .with_size(1180.0, 760.0)
                .with_min_size(900.0, 600.0)
                .with_on_close(|_, _| {
                    if !jadeforge_gui::app::ui_is_dirty() {
                        return CloseDecision::Close;
                    }
                    let answer = rfd::MessageDialog::new()
                        .set_title("Unsaved changes")
                        .set_description(
                            "This savegame has changes that are only in memory. Close JadeForge and discard them?",
                        )
                        .set_buttons(rfd::MessageButtons::YesNo)
                        .set_level(rfd::MessageLevel::Warning)
                        .show();
                    if matches!(answer, rfd::MessageDialogResult::Yes) {
                        CloseDecision::Close
                    } else {
                        CloseDecision::KeepOpen
                    }
                }),
        ),
    );
}
