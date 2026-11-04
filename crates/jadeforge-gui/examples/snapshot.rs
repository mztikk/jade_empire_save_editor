//! Renders the application into a PNG without opening a window.
//!
//! This is used to produce the screenshots in the documentation and to inspect
//! layout changes during development:
//!
//! ```text
//! cargo run -p jadeforge-gui --example snapshot -- <SAVE_DIR> <OUT.png> [ENTRY]
//! ```

use std::path::PathBuf;

use freya::prelude::*;
use freya_testing::TestingRunner;
use jadeforge_gui::app_with_directory;

fn main() {
    let mut arguments = std::env::args().skip(1);
    let directory = PathBuf::from(
        arguments
            .next()
            .expect("usage: snapshot <SAVE_DIR> <OUT.png> [ENTRY]"),
    );
    let output = PathBuf::from(
        arguments
            .next()
            .expect("usage: snapshot <SAVE_DIR> <OUT.png> [ENTRY]"),
    );
    let wanted = arguments.next();

    let start = directory.clone();
    let (mut test, ()) = TestingRunner::new(
        move || app_with_directory(start.clone()),
        (1280., 800.).into(),
        |_| {},
        1.0,
    );
    test.sync_and_update();

    // Click the first savegame entry (or the one whose summary contains the
    // requested text) so the screenshot shows the editor instead of the
    // welcome screen.
    let entry = test.find(|node, element| {
        let label = Label::try_downcast(element)?;
        let matches = match &wanted {
            Some(wanted) => label.text.contains(wanted.as_str()),
            None => label.text.starts_with("Level "),
        };
        matches.then_some(node)
    });
    if let Some(entry) = entry {
        let area = entry.layout().area;
        let center = CursorPoint::new(
            f64::from(area.origin.x) + f64::from(area.size.width) / 2.0,
            f64::from(area.origin.y) + f64::from(area.size.height) / 2.0,
        );
        test.move_cursor(center);
        test.sync_and_update();
        test.click_cursor(center);
        test.sync_and_update();
    }

    test.render_to_file(&output);
    println!("wrote {}", output.display());
}
