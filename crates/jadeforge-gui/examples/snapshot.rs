//! Renders the application into PNG files without opening a window.
//!
//! This is used to produce the screenshots in the documentation and to inspect
//! layout changes during development:
//!
//! ```text
//! cargo run -p jadeforge-gui --example snapshot -- <SAVE_DIR> <OUT.png> [TAB|all] [--height N] [--query TEXT]
//! ```
//!
//! When the tab argument is `all`, one image per tab is written next to
//! `<OUT.png>`. `--query` types a search term into the active tab's search box.

use std::path::PathBuf;

use freya::prelude::*;
use freya_testing::TestingRunner;
use jadeforge_gui::app_with_directory;

const TABS: [&str; 5] = ["Overview", "Character", "Styles", "Inventory", "Companions"];

fn main() {
    let mut arguments = std::env::args().skip(1);
    let directory = PathBuf::from(
        arguments
            .next()
            .expect("usage: snapshot <SAVE_DIR> <OUT.png> [TAB|all]"),
    );
    let output = PathBuf::from(
        arguments
            .next()
            .expect("usage: snapshot <SAVE_DIR> <OUT.png> [TAB|all]"),
    );
    let mut wanted = None;
    let mut click_text = None;
    let mut width = 1280.0_f32;
    let mut height = 800.0_f32;
    let mut query = None;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--width" => {
                width = arguments
                    .next()
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(1280.0);
            }
            "--height" => {
                height = arguments
                    .next()
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(800.0);
            }
            "--query" => query = arguments.next(),
            "--click" => click_text = arguments.next(),
            other => wanted = Some(other.to_owned()),
        }
    }

    let start = directory.clone();
    let (mut test, ()) = TestingRunner::new(
        move || app_with_directory(start.clone()),
        (width, height).into(),
        |_| {},
        1.0,
    );
    test.sync_and_update();

    // Open the first savegame so the screenshots show the editor.
    if let Some(entry) = find_label(&test, "Level ") {
        click(&mut test, &entry);
    }

    match wanted.as_deref() {
        Some("all") => {
            let _ = &query;
            for tab in TABS {
                if let Some(node) = find_label(&test, tab) {
                    click(&mut test, &node);
                }
                let stem = output
                    .file_stem()
                    .map_or_else(|| "ui".to_owned(), |stem| stem.to_string_lossy().into());
                let path = output.with_file_name(format!("{stem}-{}.png", tab.to_lowercase()));
                test.render_to_file(&path);
                println!("wrote {}", path.display());
            }
        }
        Some(tab) => {
            if let Some(node) = find_label(&test, tab) {
                click(&mut test, &node);
            }
            if let Some(text) = &click_text
                && let Some(node) = find_label(&test, text)
            {
                click(&mut test, &node);
            }
            if let Some(query) = &query {
                if let Some(node) =
                    find_paragraph(&test, "Filter ").or_else(|| find_paragraph(&test, "Search "))
                {
                    click(&mut test, &node);
                    test.write_text(query.clone());
                    test.sync_and_update();
                }
            }
            test.render_to_file(&output);
            println!("wrote {}", output.display());
        }
        None => {
            test.render_to_file(&output);
            println!("wrote {}", output.display());
        }
    }
}

fn find_paragraph(test: &TestingRunner, wanted: &str) -> Option<freya_testing::TestingNode> {
    test.find(|node, element| {
        let paragraph = Paragraph::try_downcast(element)?;
        paragraph.to_string().contains(wanted).then_some(node)
    })
}

fn find_label(test: &TestingRunner, wanted: &str) -> Option<freya_testing::TestingNode> {
    test.find(|node, element| {
        let label = Label::try_downcast(element)?;
        label.text.contains(wanted).then_some(node)
    })
}

fn click(test: &mut TestingRunner, node: &freya_testing::TestingNode) {
    let area = node.layout().area;
    let center = CursorPoint::new(
        f64::from(area.origin.x) + f64::from(area.size.width) / 2.0,
        f64::from(area.origin.y) + f64::from(area.size.height) / 2.0,
    );
    test.move_cursor(center);
    test.sync_and_update();
    test.click_cursor(center);
    test.sync_and_update();
}
