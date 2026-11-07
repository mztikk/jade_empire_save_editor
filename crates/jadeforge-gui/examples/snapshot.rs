#![allow(clippy::expect_used)] // tests fail loudly; expects are assertions

//! Renders the application into PNG files without opening a window.
//!
//! This is used to produce the screenshots in the documentation and to inspect
//! layout changes during development:
//!
//! ```text
//! cargo run -p jadeforge-gui --example snapshot -- <SAVE_DIR> <OUT.png> [TAB|all] [--height N] [--query TEXT]
//! cargo run -p jadeforge-gui --example snapshot -- <SAVE_DIR> <OUT.png> all --theme light --scale 150
//! ```
//!
//! When the tab argument is `all`, one image per tab is written next to
//! `<OUT.png>` as `<stem>-overview.png`, `<stem>-character.png`, and so on.
//! A single tab can be named by its English slug or its translated label. In
//! the rendered tree, tabs are selected by accessibility role and display
//! order; no translated text or catalog id lookup is used. `--query` types a
//! search term into the active tab's own picker filter; the savegame filter in
//! the sidebar is deliberately never used for a tab that has its own filter. A
//! missing tab, filter or savegame is an error, so a snapshot cannot silently
//! capture the empty state. `--theme` and `--scale` write the same isolated test
//! settings the app reads, so the documented theme and zoom variants are
//! reproducible.

use std::path::PathBuf;

use freya::prelude::*;
use freya_testing::TestingRunner;
use jadeforge_gui::app::{Tab, app_with_directory};
use jadeforge_gui::i18n::t;

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
    let mut theme = None;
    let mut scale = None;
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
            "--theme" => theme = arguments.next(),
            "--scale" => scale = arguments.next().and_then(|value| value.parse::<u16>().ok()),
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

    if theme.is_some() || scale.is_some() {
        let mut config = jadeforge_gui::config::Config::load();
        if let Some(theme) = &theme {
            config.theme = jadeforge_gui::ThemeChoice::from_key(theme);
        }
        if let Some(scale) = scale {
            config.text_scale = jadeforge_gui::normalize_text_scale(scale);
        }
        config.store().expect("store snapshot preferences");
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
    open_first_save(&mut test).expect("the save folder has no savegame to render");
    let all = wanted
        .as_deref()
        .is_some_and(|name| name.eq_ignore_ascii_case("all"));
    let tab = wanted
        .as_deref()
        .filter(|name| !name.eq_ignore_ascii_case("all"))
        .map(|name| tab_from_arg(name).expect("unknown tab name"));

    match (all, tab) {
        (true, _) => {
            for tab in Tab::ALL {
                click_tab(&mut test, tab);
                type_query(&mut test, tab, query.as_deref(), false);
                let stem = output
                    .file_stem()
                    .map_or_else(|| "ui".to_owned(), |stem| stem.to_string_lossy().into());
                let path = output.with_file_name(format!("{stem}-{}.png", tab_slug(tab)));
                test.render_to_file(&path);
                println!("wrote {}", path.display());
            }
        }
        (false, Some(tab)) => {
            click_tab(&mut test, tab);
            if let Some((x, y)) = cursor {
                test.move_cursor(CursorPoint::new(x, y));
                test.sync_and_update();
            }
            if let Some(text) = &click_text {
                let node = find_label(&test, text).expect("--click target was not found");
                click(&mut test, &node);
            }
            type_query(&mut test, tab, query.as_deref(), true);
            test.render_to_file(&output);
            println!("wrote {}", output.display());
        }
        (false, None) => {
            if query.is_some() {
                type_query(&mut test, Tab::Overview, query.as_deref(), true);
            }
            test.render_to_file(&output);
            println!("wrote {}", output.display());
        }
    }
}

/// Waits for the background savegame scan and opens the first entry.
///
/// The scan runs on a worker thread, so the first frame never contains the
/// list; clicking immediately would race it and snapshot the empty state.
fn open_first_save(test: &mut TestingRunner) -> Result<(), &'static str> {
    for _ in 0..500 {
        if let Some(entry) = find_label(test, "Level ") {
            click(test, &entry);
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
        test.sync_and_update();
    }
    Err("no savegame was found in the save folder")
}

/// Types `query` into the filter that belongs to `tab`.
///
/// The lookup is the translated placeholder of that tab's picker, so Styles can
/// no longer capture the sidebar's savegame filter just because it also starts
/// with "Filter ". Tabs without a picker fail a single-tab query and are skipped
/// only by the `all` mode.
fn type_query(test: &mut TestingRunner, tab: Tab, query: Option<&str>, required: bool) {
    let Some(query) = query else {
        return;
    };
    let placeholder = if required {
        tab.filter_placeholder()
            .expect("--query requires a tab with a picker filter")
    } else {
        let Some(placeholder) = tab.filter_placeholder() else {
            return;
        };
        placeholder
    };
    let placeholder = t(placeholder);
    let node = find_paragraph_exact(test, placeholder)
        .expect("the tab's picker filter placeholder was not rendered");
    click(test, &node);
    test.write_text(query.to_owned());
    test.sync_and_update();
    let _ = find_paragraph_exact(test, query)
        .expect("--query was not applied to the tab's picker filter");
}

/// Resolves a tab argument from its English slug or translated label.
fn tab_from_arg(label: &str) -> Option<Tab> {
    Tab::ALL.into_iter().find(|tab| {
        tab_slug(*tab).eq_ignore_ascii_case(label) || t(tab.label()).eq_ignore_ascii_case(label)
    })
}

/// Stable filename slug for a tab.
fn tab_slug(tab: Tab) -> &'static str {
    match tab {
        Tab::Overview => "overview",
        Tab::Character => "character",
        Tab::Styles => "styles",
        Tab::Inventory => "inventory",
        Tab::Gems => "gems",
        Tab::Companions => "companions",
    }
}

/// Clicks a tab by its accessibility role and display order.
///
/// The visible label is translated, so neither the catalog id nor the English
/// source text is safe to search for in the rendered tree.
fn click_tab(test: &mut TestingRunner, tab: Tab) {
    let index = Tab::ALL
        .iter()
        .position(|candidate| *candidate == tab)
        .expect("tab is not in Tab::ALL");
    let node = find_tabs(test)
        .into_iter()
        .nth(index)
        .expect("the requested tab was not rendered");
    click(test, &node);
}

fn find_tabs(test: &TestingRunner) -> Vec<freya_testing::TestingNode> {
    test.find_many(|node, _| (a11y_role(&node) == AccessibilityRole::Tab).then_some(node))
}

fn a11y_role(node: &freya_testing::TestingNode) -> AccessibilityRole {
    let element = node.element();
    let mut builder = element.accessibility().builder.clone();
    element.finish_accessibility(&mut builder);
    builder.role()
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
