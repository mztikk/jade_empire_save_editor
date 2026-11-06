#![allow(clippy::expect_used)] // tests fail loudly; expects are assertions

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
//! `<OUT.png>`. `--query` types a search term into the active tab's own picker
//! filter; the savegame filter in the sidebar is deliberately never used for a
//! tab that has its own filter.

use std::path::PathBuf;

use freya::prelude::*;
use freya_testing::TestingRunner;
use jadeforge_gui::app::{Tab, app_with_directory};

#[allow(clippy::too_many_lines)] // CLI argument handling and one render path
fn main() {
    // Keep the developer's remembered save folder and sort order out of the
    // rendered documentation.
    let _ = jadeforge_gui::config::ensure_test_path();
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
    let mut cursor: Option<(f64, f64)> = None;
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
            "--cursor" => {
                cursor = arguments.next().and_then(|value| {
                    let mut parts = value.split(',');
                    let x = parts.next()?.trim().parse().ok()?;
                    let y = parts.next()?.trim().parse().ok()?;
                    Some((x, y))
                });
            }
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
    open_first_save(&mut test);

    match wanted.as_deref() {
        Some("all") => {
            for tab in Tab::ALL {
                if let Some(node) = find_label(&test, tab.label()) {
                    click(&mut test, &node);
                }
                type_query(&mut test, tab, query.as_deref());
                let stem = output
                    .file_stem()
                    .map_or_else(|| "ui".to_owned(), |stem| stem.to_string_lossy().into());
                let path =
                    output.with_file_name(format!("{stem}-{}.png", tab.label().to_lowercase()));
                test.render_to_file(&path);
                println!("wrote {}", path.display());
            }
        }
        Some(tab) => {
            if let Some(node) = find_label(&test, tab) {
                click(&mut test, &node);
            }
            if let Some((x, y)) = cursor {
                test.move_cursor(CursorPoint::new(x, y));
                test.sync_and_update();
            }
            if let Some(text) = &click_text
                && let Some(node) = find_label(&test, text)
            {
                click(&mut test, &node);
            }
            match tab_from_label(tab) {
                Some(tab) => type_query(&mut test, tab, query.as_deref()),
                None if query.is_some() => {
                    eprintln!("warning: {tab:?} is not a tab; --query was not applied");
                }
                None => {}
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

/// Waits for the background savegame scan and opens the first entry.
///
/// The scan runs on a worker thread, so the first frame never contains the
/// list; clicking immediately would race it and snapshot the empty state.
fn open_first_save(test: &mut TestingRunner) {
    for _ in 0..500 {
        if let Some(entry) = find_label(test, "Level ") {
            click(test, &entry);
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
        test.sync_and_update();
    }
    eprintln!("warning: no savegame was found; the snapshot shows the empty state");
}

/// Types `query` into the filter that belongs to `tab`.
///
/// The lookup is the exact placeholder of that tab's picker, so Styles can no
/// longer capture the sidebar's savegame filter just because it also starts
/// with "Filter ". Tabs without a picker are left alone.
fn type_query(test: &mut TestingRunner, tab: Tab, query: Option<&str>) {
    let Some(query) = query else {
        return;
    };
    let Some(placeholder) = tab.filter_placeholder() else {
        eprintln!(
            "warning: {} has no tab filter; --query was not applied",
            tab.label()
        );
        return;
    };
    if let Some(node) = find_paragraph_exact(test, placeholder) {
        click(test, &node);
        test.write_text(query.to_owned());
        test.sync_and_update();
    }
}

/// Resolves a tab label to its [`Tab`].
fn tab_from_label(label: &str) -> Option<Tab> {
    Tab::ALL
        .into_iter()
        .find(|tab| tab.label().eq_ignore_ascii_case(label))
}

/// Finds an input paragraph by its exact text (a placeholder or value).
fn find_paragraph_exact(test: &TestingRunner, wanted: &str) -> Option<freya_testing::TestingNode> {
    test.find(|node, element| {
        let paragraph = Paragraph::try_downcast(element)?;
        (paragraph.to_string() == wanted).then_some(node)
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
