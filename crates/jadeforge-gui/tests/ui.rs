#![allow(clippy::expect_used)] // tests fail loudly; expects are assertions

//! Headless UI tests.
//!
//! The tests render the real component tree with `freya-testing`, which is the
//! same code path the windowed application uses minus the window. A miniature
//! game installation is assembled in a temporary folder (talk table and the two
//! 2DA tables) so item and style names are resolvable. When neither the game
//! nor savegames are available the tests skip instead of failing.

use std::path::{Path, PathBuf};

use freya::elements::image::Image;
use freya::prelude::*;
use freya_core::events::platform::{KeyboardEventName, PlatformEvent};
use freya_testing::TestingRunner;
use jadeforge_gui::ThemeChoice;
use jadeforge_gui::app_with_directory;
use jadeforge_gui::theme::{self, PaletteKind};

mod support;

fn game_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("JADEFORGE_GAME_DIR").map(PathBuf::from)?;
    dir.join("dialog.tlk").is_file().then_some(dir)
}

fn save_dir_from_env() -> Option<PathBuf> {
    let dir = std::env::var_os("JADEFORGE_SAVE_DIR").map(PathBuf::from)?;
    dir.is_dir().then_some(dir)
}

fn smallest_save(dir: &Path) -> Option<PathBuf> {
    let mut candidates: Vec<(u64, PathBuf)> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path().join("save.sac"))
        .filter(|path| path.is_file())
        .filter_map(|path| {
            let size = std::fs::metadata(&path).ok()?.len();
            Some((size, path))
        })
        .collect();
    candidates.sort();
    candidates.into_iter().next().map(|(_, path)| path)
}

/// Assembles a temporary installation with one savegame.
struct Fixture {
    root: PathBuf,
    save_dir: PathBuf,
    game_data: bool,
    /// Whether the copied savegame brought its `SaveImage.tga` along.
    screenshot: bool,
}

impl Fixture {
    fn create() -> Option<Self> {
        Self::create_at(Self::unique_root())
    }

    /// Creates the fixture below a path long enough to make the scan message
    /// wrap onto its second line.
    fn create_with_long_path() -> Option<Self> {
        let mut root = Self::unique_root();
        for index in 0..12 {
            root.push(format!("jadeforge-long-path-{index}-0123456789"));
        }
        Self::create_at(root)
    }

    fn unique_root() -> PathBuf {
        std::env::temp_dir().join(format!(
            "jadeforge-ui-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |duration| duration.as_nanos())
        ))
    }

    fn create_at(root: PathBuf) -> Option<Self> {
        let game = root.join("Game");
        let save_dir = game.join("Save");
        let folder = save_dir.join("Test Save");
        std::fs::create_dir_all(&folder).ok()?;

        // Prefer the real corpus when a developer has it; otherwise build a
        // complete synthetic installation so CI exercises the UI too.
        let real = save_dir_from_env()
            .and_then(|source_saves| smallest_save(&source_saves).map(|save| (save, source_saves)));
        let (save, source_game, screenshot_available) = if let Some((save, source_saves)) = real {
            let game_root = game_dir().or_else(|| {
                jadeforge_gamedata::GameData::discover(&source_saves)
                    .map(|game| game.root().to_path_buf())
            });
            std::fs::copy(&save, folder.join("save.sac")).ok()?;
            let screenshot = std::fs::copy(
                save.with_file_name(jadeforge_core::screenshot::SAVE_IMAGE_NAME),
                folder.join(jadeforge_core::screenshot::SAVE_IMAGE_NAME),
            )
            .is_ok();
            (save, game_root, screenshot)
        } else {
            let (save, game_root) = support::synthetic(&root)?;
            (save, Some(game_root), true)
        };

        let game_data = source_game.is_some();
        if let Some(source) = source_game {
            jadeforge_gamedata::set_test_game_root(Some(source));
        }

        let screenshot = screenshot_available && jadeforge_core::SaveImage::load(&save).is_ok();
        Some(Self {
            root,
            save_dir,
            game_data,
            screenshot,
        })
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        jadeforge_gamedata::set_test_game_root(None);
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn find_label_text(test: &TestingRunner, needle: &str) -> Option<String> {
    test.find(|_, element| {
        let label = Label::try_downcast(element)?;
        label.text.contains(needle).then(|| label.text.to_string())
    })
}

/// Finds the `n of m <noun>` page counter, ignoring item names with " of ".
fn find_counter(test: &TestingRunner) -> Option<String> {
    test.find(|_, element| {
        let label = Label::try_downcast(element)?;
        let label_text = label.text.as_ref();
        let (count, rest) = label_text.split_once(" of ")?;
        let total = rest.split_whitespace().next()?;
        let digits = |value: &str| {
            !value.is_empty() && value.chars().all(|character| character.is_ascii_digit())
        };
        (digits(count) && digits(total)).then(|| label_text.to_owned())
    })
}

fn find_node_with_text(test: &TestingRunner, needle: &str) -> Option<freya_testing::TestingNode> {
    test.find(|node, element| {
        let label = Label::try_downcast(element)?;
        label.text.contains(needle).then_some(node)
    })
}

/// Finds visible text in either a label or a wrapping paragraph.
fn find_text_node(test: &TestingRunner, needle: &str) -> Option<freya_testing::TestingNode> {
    test.find(|node, element| {
        if let Some(label) = Label::try_downcast(element)
            && label.text.contains(needle)
        {
            return Some(node);
        }
        let paragraph = Paragraph::try_downcast(element)?;
        paragraph.to_string().contains(needle).then_some(node)
    })
}

fn find_label_exact(test: &TestingRunner, wanted: &str) -> Option<freya_testing::TestingNode> {
    test.find(|node, element| {
        let label = Label::try_downcast(element)?;
        (label.text.as_ref() == wanted).then_some(node)
    })
}

fn find_image(test: &TestingRunner) -> Option<freya_testing::TestingNode> {
    test.find(|node, element| Image::try_downcast(element).map(|_| node))
}

fn click_node(test: &mut TestingRunner, node: &freya_testing::TestingNode) {
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

fn wait_for_text(test: &mut TestingRunner, needle: &str) {
    for _ in 0..500 {
        if find_node_with_text(test, needle).is_some() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
        test.sync_and_update();
    }
}

/// Runs the extra layout passes a virtual list needs once its viewport has been
/// measured, so the rows that fit the viewport are actually built.
fn settle(test: &mut TestingRunner) {
    for _ in 0..4 {
        test.sync_and_update();
    }
}

fn click_text(test: &mut TestingRunner, needle: &str) -> bool {
    match find_node_with_text(test, needle) {
        Some(node) => {
            click_node(test, &node);
            true
        }
        None => false,
    }
}

fn click_label_exact(test: &mut TestingRunner, wanted: &str) -> bool {
    match find_label_exact(test, wanted) {
        Some(node) => {
            click_node(test, &node);
            true
        }
        None => false,
    }
}

/// Renders the application at a given size and waits for the background scan,
/// without opening the savegame.
fn launch_unopened_at(save_dir: PathBuf, width: f64, height: f64) -> TestingRunner {
    // Keep the developer's settings (and therefore their saved sort order) out
    // of the UI tests.
    let _ = jadeforge_gui::config::ensure_test_path();
    let (mut test, ()) = TestingRunner::new(
        move || app_with_directory(save_dir.clone()),
        (width as f32, height as f32).into(),
        |_| {},
        1.0,
    );
    test.sync_and_update();
    // The scan runs after the first frame.
    wait_for_text(&mut test, "Level ");
    test
}

fn launch_unopened(save_dir: PathBuf, height: f64) -> TestingRunner {
    launch_unopened_at(save_dir, 1280.0, height)
}

fn launch_sized(save_dir: PathBuf, height: f64) -> TestingRunner {
    let mut test = launch_unopened(save_dir, height);
    // Open the single savegame of the fixture.
    click_text(&mut test, "Level ");
    test
}

fn launch(save_dir: PathBuf) -> TestingRunner {
    // Tall enough that the character card's quick-add row is above the fold;
    // the content scrolls at shorter window sizes.
    launch_sized(save_dir, 1000.0)
}

/// Finds a paragraph whose text contains `needle` (input placeholders and
/// values are paragraphs, not labels).
fn find_paragraph(test: &TestingRunner, needle: &str) -> Option<freya_testing::TestingNode> {
    test.find(|node, element| {
        let paragraph = Paragraph::try_downcast(element)?;
        paragraph.to_string().contains(needle).then_some(node)
    })
}

#[test]
fn repeated_tab_switching_is_stable() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut test = launch_sized(fixture.save_dir.clone(), 900.0);
    for _ in 0..4 {
        assert!(click_text(&mut test, "Gems"), "Gems tab missing");
        assert!(click_text(&mut test, "Inventory"), "Inventory tab missing");
        assert!(click_text(&mut test, "Styles"), "Styles tab missing");
    }
    assert!(find_label_text(&test, "Fighting styles").is_some());
}

#[test]
fn every_tab_renders_at_narrow_and_wide_sizes() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    for width in [900.0, 1600.0] {
        let mut test = launch_unopened_at(fixture.save_dir.clone(), width, 900.0);
        assert!(click_text(&mut test, "Level "), "savegame did not open");
        for (tab, heading) in [
            ("Overview", "At a glance"),
            ("Character", "Identity"),
            ("Styles", "Fighting styles"),
            ("Inventory", "Inventory"),
            ("Gems", "Amulet"),
            ("Companions", "Companions"),
        ] {
            assert!(click_text(&mut test, tab), "tab {tab} is missing");
            assert!(
                find_label_text(&test, heading).is_some(),
                "tab {tab} did not render at {width}px"
            );
        }
    }
}

#[test]
fn savegames_are_listed_and_can_be_opened() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found (set JADEFORGE_SAVE_DIR)");
        return;
    };
    let mut test = launch(fixture.save_dir.clone());

    assert!(
        find_label_text(&test, "Opened Test Save").is_some(),
        "the savegame was not opened"
    );
    assert!(find_label_text(&test, "Level ").is_some());
    // The overview shows the character and the action bar.
    assert!(find_label_text(&test, "Save to disk").is_some());
    let image = test.render();
    assert!(
        image.len() > 1024,
        "the rendered frame is suspiciously small"
    );
}

#[test]
fn opening_a_save_does_not_move_the_sidebar() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found (set JADEFORGE_SAVE_DIR)");
        return;
    };
    if !fixture.screenshot {
        eprintln!("skipping: the savegame has no screenshot");
        return;
    }
    let mut test = launch_unopened(fixture.save_dir.clone(), 800.0);

    let before = find_node_with_text(&test, "SAVEGAME")
        .expect("the savegame heading is missing")
        .layout()
        .area;
    let row_before = find_label_exact(&test, "Test Save")
        .expect("the savegame row is missing")
        .layout()
        .area
        .origin
        .x;

    assert!(
        click_text(&mut test, "Level "),
        "the savegame row is missing"
    );
    // The preview is decoded before the session re-renders, so the image has
    // to be part of the very next frame.
    let image = find_image(&test).expect("the screenshot was not rendered");
    let after = find_node_with_text(&test, "SAVEGAME")
        .expect("the savegame heading is missing")
        .layout()
        .area;
    let row_after = find_label_exact(&test, "Test Save")
        .expect("the savegame row is missing")
        .layout()
        .area
        .origin
        .x;

    assert!(
        (after.origin.y - before.origin.y).abs() < 0.5,
        "the sidebar moved by {} px when the screenshot appeared",
        after.origin.y - before.origin.y
    );
    assert!(
        (row_after - row_before).abs() < 0.5,
        "the savegame row text moved by {} px when it was selected",
        row_after - row_before
    );
    let image_area = image.layout().area;
    assert!(
        image_area.size.height <= 120.5 && (image_area.size.width - 120.0).abs() < 0.5,
        "the screenshot is {}x{} px; it must stay a fixed 120x120 square",
        image_area.size.width,
        image_area.size.height
    );
}

#[test]
fn the_overview_shows_one_preview_aligned_with_the_sidebar_identity() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    if !fixture.screenshot {
        eprintln!("skipping: the savegame has no screenshot");
        return;
    }
    let test = launch_sized(fixture.save_dir.clone(), 900.0);
    // The sidebar already shows the savegame screenshot; a second copy in the
    // overview hero would duplicate it.
    let images = test.find_many(|node, element| Image::try_downcast(element).map(|_| node));
    assert_eq!(
        images.len(),
        1,
        "the overview shows {} screenshots; the sidebar preview must be the only one",
        images.len()
    );
    let image_area = images[0].layout().area;
    let heading = find_node_with_text(&test, "SAVEGAME")
        .expect("the sidebar heading is missing")
        .layout()
        .area;
    assert!(
        (image_area.origin.x - heading.origin.x).abs() < 1.0,
        "the sidebar preview starts at {} px but the identity block starts at {} px",
        image_area.origin.x,
        heading.origin.x
    );
}

#[test]
fn the_save_browser_stays_usable_at_the_minimum_window_size() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    // The fixture has one save; a few copies make the list browser test the
    // real situation (the retail corpus has 89).
    for suffix in 2..=6 {
        let folder = fixture.save_dir.join(format!("Test Save {suffix}"));
        std::fs::create_dir_all(&folder).expect("create extra save folder");
        std::fs::copy(
            fixture.save_dir.join("Test Save").join("save.sac"),
            folder.join("save.sac"),
        )
        .expect("copy savegame");
    }

    let mut test = launch_unopened_at(fixture.save_dir.clone(), 900.0, 700.0);
    settle(&mut test);
    let fourth = find_label_exact(&test, "Test Save 4").expect("the fourth save row is missing");
    let footer = find_label_exact(&test, "Save to disk").expect("the footer is missing");
    let fourth_area = fourth.layout().area;
    assert!(
        fourth_area.origin.y + fourth_area.size.height < footer.layout().area.origin.y,
        "only three save rows fit at the supported 900x700 minimum"
    );
}

#[test]
fn long_status_messages_do_not_move_the_tab_bar() {
    let Some(fixture) = Fixture::create_with_long_path() else {
        eprintln!("skipping: no savegames found (set JADEFORGE_SAVE_DIR)");
        return;
    };
    let mut test = launch_unopened(fixture.save_dir.clone(), 800.0);

    let status =
        find_node_with_text(&test, "Found 1 savegame").expect("the scan message is missing");
    assert!(
        status.layout().area.size.height > 20.0,
        "the fixture path did not make the scan message wrap"
    );
    let before = find_node_with_text(&test, "Overview")
        .expect("the tab bar is missing")
        .layout()
        .area
        .origin
        .y;

    assert!(
        click_text(&mut test, "Level "),
        "the savegame row is missing"
    );
    let after = find_node_with_text(&test, "Overview")
        .expect("the tab bar is missing")
        .layout()
        .area
        .origin
        .y;

    assert!(
        (after - before).abs() < 0.5,
        "the tab bar moved by {} px when the status message shrank",
        after - before
    );
}

#[test]
fn switching_tabs_does_not_move_the_tab_bar() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found (set JADEFORGE_SAVE_DIR)");
        return;
    };
    let mut test = launch_unopened(fixture.save_dir.clone(), 800.0);

    let character_before = find_label_exact(&test, "Character")
        .expect("the Character tab is missing")
        .layout()
        .area
        .origin
        .x;
    let styles_before = find_label_exact(&test, "Styles")
        .expect("the Styles tab is missing")
        .layout()
        .area
        .origin
        .x;

    assert!(click_label_exact(&mut test, "Character"));

    let character_after = find_label_exact(&test, "Character")
        .expect("the Character tab is missing")
        .layout()
        .area
        .origin
        .x;
    let styles_after = find_label_exact(&test, "Styles")
        .expect("the Styles tab is missing")
        .layout()
        .area
        .origin
        .x;

    assert!(
        (character_after - character_before).abs() < 0.5,
        "the active tab moved by {} px",
        character_after - character_before
    );
    assert!(
        (styles_after - styles_before).abs() < 0.5,
        "the following tabs moved by {} px",
        styles_after - styles_before
    );
}

#[test]
fn sorting_does_not_move_the_sort_buttons() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found (set JADEFORGE_SAVE_DIR)");
        return;
    };
    let mut test = launch_unopened(fixture.save_dir.clone(), 800.0);

    let level_before = find_label_exact(&test, "Level")
        .expect("the Level sort button is missing")
        .layout()
        .area
        .origin
        .x;

    // Flipping the direction only changes the arrow glyph.
    assert!(click_label_exact(&mut test, "Name"));
    let level_after_flip = find_label_exact(&test, "Level")
        .expect("the Level sort button is missing")
        .layout()
        .area
        .origin
        .x;
    assert!(
        (level_after_flip - level_before).abs() < 0.5,
        "the sort buttons moved by {} px when the arrow flipped",
        level_after_flip - level_before
    );

    // Moving the arrow to another column changes both neighbouring widths.
    assert!(click_label_exact(&mut test, "Time played"));
    let level_after_move = find_label_exact(&test, "Level")
        .expect("the Level sort button is missing")
        .layout()
        .area
        .origin
        .x;
    assert!(
        (level_after_move - level_before).abs() < 0.5,
        "the sort buttons moved by {} px when the arrow changed column",
        level_after_move - level_before
    );
}

/// Focuses the alignment input by clicking below its label and types an
/// invalid value into it.
fn focus_alignment_input(test: &mut TestingRunner) {
    let label = find_label_exact(test, "ALIGNMENT").expect("the alignment field is missing");
    let area = label.layout().area;
    let point = CursorPoint::new(
        f64::from(area.origin.x) + 10.0,
        f64::from(area.origin.y) + f64::from(area.size.height) + 20.0,
    );
    test.move_cursor(point);
    test.sync_and_update();
    test.click_cursor(point);
    test.sync_and_update();
    test.write_text("x");
    test.sync_and_update();
}

#[test]
fn editing_the_alignment_does_not_move_the_character_cards() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found (set JADEFORGE_SAVE_DIR)");
        return;
    };
    // At this width the valid hint and the invalid hint used to need different
    // numbers of lines.
    let mut test = launch_unopened_at(fixture.save_dir.clone(), 700.0, 900.0);
    assert!(
        click_text(&mut test, "Level "),
        "the savegame row is missing"
    );
    assert!(click_label_exact(&mut test, "Character"));

    assert!(
        find_node_with_text(&test, "the game starts at").is_some(),
        "the alignment hint is missing"
    );
    let vitals_before = find_label_exact(&test, "Vitals")
        .expect("the Vitals card is missing")
        .layout()
        .area
        .origin
        .y;

    focus_alignment_input(&mut test);
    assert!(
        find_node_with_text(&test, "Alignment is a number").is_some(),
        "the alignment input was not focused"
    );
    let vitals_after = find_label_exact(&test, "Vitals")
        .expect("the Vitals card is missing")
        .layout()
        .area
        .origin
        .y;

    assert!(
        (vitals_after - vitals_before).abs() < 0.5,
        "the cards below the hint moved by {} px while typing",
        vitals_after - vitals_before
    );
}

#[test]
fn dynamic_label_text_reaches_the_accessibility_tree() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut test = launch_unopened_at(fixture.save_dir.clone(), 900.0, 900.0);
    assert!(
        click_text(&mut test, "Level "),
        "the savegame row is missing"
    );
    assert!(click_label_exact(&mut test, "Character"));

    let before =
        find_node_with_text(&test, "the game starts at").expect("the alignment hint is missing");
    assert!(
        a11y_value(&before).is_some_and(|value| value.contains("the game starts at")),
        "the initial alignment hint has no accessible value"
    );

    focus_alignment_input(&mut test);
    let after =
        find_node_with_text(&test, "Alignment is a number").expect("the invalid hint is missing");
    assert_eq!(
        a11y_value(&after).as_deref(),
        Some("Alignment is a number; below 128 is the Closed Fist, above is the Open Palm."),
        "the changed hint did not reach the effective AccessKit node"
    );
}

#[test]
fn tabs_switch_the_content() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut test = launch(fixture.save_dir.clone());

    assert!(click_text(&mut test, "Styles"));
    assert!(find_label_text(&test, "Fighting styles").is_some());

    assert!(click_text(&mut test, "Inventory"));
    assert!(find_label_text(&test, "ADD AN ITEM").is_some());

    assert!(click_text(&mut test, "Companions"));
    assert!(find_label_text(&test, "Companions").is_some());

    assert!(click_text(&mut test, "Character"));
    assert!(find_label_text(&test, "UNSPENT ATTRIBUTE POINTS").is_some());
}

#[test]
fn buttons_use_the_pointer_cursor() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut test = launch_sized(fixture.save_dir.clone(), 1000.0);
    let tab = test
        .find(|node, _| {
            (a11y_role(&node) == "Tab" && a11y_label(&node).as_deref() == Some("Character"))
                .then_some(node)
        })
        .expect("Character tab");
    let area = tab.layout().area;
    test.move_cursor(CursorPoint::new(
        f64::from(area.origin.x) + f64::from(area.size.width) / 2.0,
        f64::from(area.origin.y) + f64::from(area.size.height) / 2.0,
    ));
    test.sync_and_update();
    assert_eq!(
        test.cursor_icon(),
        CursorIcon::Pointer,
        "the tab does not show a pointer cursor"
    );
}

#[test]
fn items_can_be_searched_and_added() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    if !fixture.game_data {
        eprintln!("skipping: no game data to resolve item names");
        return;
    }
    // A tall window keeps the add-item section visible without scrolling.
    let mut test = launch_sized(fixture.save_dir.clone(), 1800.0);
    assert!(click_text(&mut test, "Inventory"));

    // Focus the search field and type.
    let input =
        find_paragraph(&test, "Filter the item database").expect("the search input is missing");
    click_node(&mut test, &input);
    test.write_text("Viper");
    test.sync_and_update();
    assert!(
        find_label_text(&test, "Viper").is_some(),
        "no search result for Viper"
    );

    assert!(click_text(&mut test, "Add"), "no add button in the results");
    let status = find_label_text(&test, "Added").expect("status message");
    assert!(status.contains("Added"), "{status}");
}

#[test]
fn every_list_is_browsable_and_labelled_the_same_way() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    if !fixture.game_data {
        eprintln!("skipping: no game data to browse");
        return;
    }
    let mut test = launch_sized(fixture.save_dir.clone(), 1800.0);

    assert!(click_text(&mut test, "Styles"));
    // Without a query the whole learnable list is shown, not nothing.
    assert!(
        find_label_text(&test, "STYLES TO LEARN").is_some(),
        "the style list is not browsable without a search"
    );
    // Owned and learnable rows share the `label · #id` subtitle.
    assert!(
        find_label_text(&test, " · #").is_some(),
        "styles do not show the shared label and id subtitle"
    );
    assert!(find_label_text(&test, "Level ").is_some());

    assert!(click_text(&mut test, "Inventory"));
    assert!(
        find_label_text(&test, "ALL ITEMS").is_some(),
        "the item list is not browsable without a search"
    );
    assert!(
        find_label_text(&test, " of ").is_some(),
        "the item list does not show how many entries exist"
    );
    assert!(
        find_label_text(&test, " · #").is_some(),
        "items do not show the category and id subtitle"
    );
}

fn exact_label_nodes(test: &TestingRunner, wanted: &str) -> Vec<freya_testing::TestingNode> {
    test.find_many(|node, element| {
        let label = Label::try_downcast(element)?;
        (label.text.as_ref() == wanted).then_some(node)
    })
}

fn assert_compact_action_rows(test: &TestingRunner, action: &str, width: f32) {
    let actions = exact_label_nodes(test, action);
    if actions.len() < 2 {
        return;
    }
    let mut ys: Vec<f32> = actions
        .iter()
        .map(|node| node.layout().area.origin.y)
        .collect();
    ys.sort_by(f32::total_cmp);
    for pair in ys.windows(2) {
        assert!(
            pair[1] - pair[0] < 80.0,
            "rows are {} px apart; the name block consumed the row",
            pair[1] - pair[0]
        );
    }
    for node in &actions {
        let area = node.layout().area;
        assert!(
            area.origin.x + area.size.width < width - 10.0,
            "the row controls run off the card at x={}",
            area.origin.x
        );
    }
}

#[test]
fn gem_and_companion_rows_stay_compact_and_inside_the_card() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut test = launch_sized(fixture.save_dir.clone(), 1400.0);
    assert!(click_text(&mut test, "Gems"));
    assert_compact_action_rows(&test, "Add", 1400.0);
    assert!(click_text(&mut test, "Companions"));
    assert_compact_action_rows(&test, "Hide", 1400.0);
}

#[test]
fn companion_availability_uses_one_vocabulary() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let Some(session) = probe_session(&fixture) else {
        eprintln!("skipping: the fixture save could not be opened");
        return;
    };
    let Some(companion) = session.henchmen().first().cloned() else {
        eprintln!("skipping: the fixture has no companions");
        return;
    };
    let mut test = launch_sized(fixture.save_dir.clone(), 1400.0);
    assert!(click_text(&mut test, "Companions"));
    if companion.available {
        assert!(find_label_text(&test, "available").is_some());
        assert!(
            click_a11y_label(&mut test, &format!("Hide {}", companion.name)),
            "the available companion has no Hide action"
        );
        assert!(
            find_label_text(&test, "hidden").is_some(),
            "hiding a companion does not show the hidden state"
        );
        assert!(
            find_label_text(&test, "Unlock").is_none(),
            "the hidden state still offers Unlock instead of Make available"
        );
        assert!(
            click_a11y_label(&mut test, &format!("Make {} available", companion.name)),
            "the hidden companion has no Make available action"
        );
        assert!(find_label_text(&test, "available").is_some());
    } else {
        assert!(
            click_a11y_label(&mut test, &format!("Make {} available", companion.name)),
            "the hidden companion has no Make available action"
        );
        assert!(
            click_a11y_label(&mut test, &format!("Hide {}", companion.name)),
            "the available companion has no Hide action"
        );
    }
}

#[test]
fn style_rows_stay_compact_and_inside_the_card() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut test = launch_sized(fixture.save_dir.clone(), 1400.0);
    assert!(click_text(&mut test, "Styles"));
    assert_compact_action_rows(&test, "Remove", 1400.0);
    assert_compact_action_rows(&test, "Learn", 1400.0);
}

#[test]
fn inventory_rows_stay_compact_and_inside_the_card() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut test = launch_sized(fixture.save_dir.clone(), 1400.0);
    assert!(click_text(&mut test, "Inventory"));
    assert_compact_action_rows(&test, "Remove", 1400.0);
    assert_compact_action_rows(&test, "Add", 1400.0);
}

fn count_labels(test: &TestingRunner) -> usize {
    test.find_many(|_, element| {
        (Label::try_downcast(element).is_some() || Paragraph::try_downcast(element).is_some())
            .then_some(())
    })
    .len()
}

#[test]
fn the_style_picker_lists_only_unlearned_styles() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let Some(session) = probe_session(&fixture) else {
        eprintln!("skipping: the fixture save could not be opened");
        return;
    };
    let mut test = launch_sized(fixture.save_dir.clone(), 1400.0);
    assert!(click_text(&mut test, "Styles"));
    assert!(
        find_label_text(&test, "STYLES TO LEARN").is_some(),
        "the unlearned style list is still called ALL STYLES"
    );
    assert!(
        find_label_text(&test, "ALL STYLES").is_none(),
        "the style list still claims to show all styles"
    );
    if let Some(style) = session.styles().first() {
        // `Lv` read; the row now uses the same `Level` wording as the rest of
        // the application, and the kind badge is title-cased.
        assert!(
            find_label_text(&test, &format!("Level {}", style.level)).is_some(),
            "the style row does not show its level as `Level N`"
        );
        assert!(
            find_label_text(&test, "Lv ").is_none(),
            "the style row still abbreviates Level as Lv"
        );
        assert!(
            find_label_text(&test, &jadeforge_gui::session::style_kind_label(style.kind)).is_some(),
            "the style kind badge does not use the friendly label for {}",
            style.kind
        );
    }
}

#[test]
fn the_item_picker_can_filter_by_category() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    if !fixture.game_data {
        eprintln!("skipping: no game data to browse");
        return;
    }
    let mut test = launch_unopened_at(fixture.save_dir.clone(), 900.0, 1800.0);
    assert!(click_text(&mut test, "Level "), "the savegame did not open");
    assert!(click_text(&mut test, "Inventory"));
    assert!(
        find_label_text(&test, "CATEGORY").is_some(),
        "the item picker has no category filter"
    );
    // The chips wrap as whole labels, never as "Weapon" plus a lone "s".
    let chip = find_label_exact(&test, "Weapons").expect("the Weapons chip is missing");
    assert!(
        chip.layout().area.size.height <= 14.5,
        "the Weapons chip label wrapped inside its pill ({} px)",
        chip.layout().area.size.height
    );
    assert!(
        click_a11y_label(&mut test, "Show only Gems"),
        "the Gems category chip is missing"
    );
    settle(&mut test);
    let heading = find_label_exact(&test, "GEMS").expect("the Gems result heading is missing");
    let heading_y = heading.layout().area.origin.y;
    let weapons = test.find_many(|node, element| {
        let label = Label::try_downcast(element)?;
        label.text.contains("Weapons · #").then_some(node)
    });
    assert!(
        weapons
            .iter()
            .all(|node| node.layout().area.origin.y < heading_y),
        "a weapon survived the Gems category filter"
    );
    let gems = test.find_many(|node, element| {
        let label = Label::try_downcast(element)?;
        label.text.contains("Gems · #").then_some(node)
    });
    assert!(
        gems.iter()
            .any(|node| node.layout().area.origin.y > heading_y),
        "the Gems category filter produced no gem rows"
    );
    assert!(
        click_a11y_label(&mut test, "Show items from every category"),
        "the All items chip is missing"
    );
    test.sync_and_update();
    assert!(
        find_label_text(&test, "ALL ITEMS").is_some(),
        "the category filter could not be cleared"
    );
}

#[test]
fn the_styles_tab_filter_does_not_filter_the_save_list() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut test = launch_sized(fixture.save_dir.clone(), 1400.0);
    assert!(click_text(&mut test, "Styles"));
    // Both filters exist, but they are targeted by their own placeholders.
    assert!(
        find_paragraph(&test, "Filter by name or character").is_some(),
        "the savegame filter is missing"
    );
    let style_filter =
        find_paragraph(&test, "Filter the style list").expect("the style filter is missing");
    click_node(&mut test, &style_filter);
    test.write_text("no-such-style");
    test.sync_and_update();
    assert!(
        find_label_text(&test, "No styles match this search.").is_some(),
        "typing into the style filter did not filter the style list"
    );
    assert!(
        find_label_text(&test, "1 SAVEGAME").is_some(),
        "the save list was filtered instead of the style list"
    );
}

#[test]
fn the_save_filter_matches_the_character_name() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let Some(session) = probe_session(&fixture) else {
        eprintln!("skipping: the fixture save could not be opened");
        return;
    };
    let Some(name) = session.character().map(|character| character.name.clone()) else {
        eprintln!("skipping: the fixture has no character");
        return;
    };
    if name.trim().is_empty() {
        eprintln!("skipping: the fixture character has no name");
        return;
    }
    let mut test = launch_sized(fixture.save_dir.clone(), 900.0);
    let filter = find_paragraph(&test, "Filter by name or character").expect("save filter");
    click_node(&mut test, &filter);
    test.write_text(&name);
    test.sync_and_update();
    assert!(
        find_label_text(&test, "1 SAVEGAME").is_some(),
        "the save filter did not match the character name {name:?}"
    );
    assert!(
        find_label_text(&test, "0 OF").is_none(),
        "filtering by the character name hid the only savegame"
    );
}

#[test]
fn rows_expand_to_show_descriptions() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    if !fixture.game_data {
        eprintln!("skipping: no game data to resolve descriptions");
        return;
    }
    let mut test = launch_sized(fixture.save_dir.clone(), 1600.0);

    assert!(click_text(&mut test, "Inventory"));
    let before = count_labels(&test);
    let subtitle = find_node_with_text(&test, " · #").expect("item subtitle");
    click_node(&mut test, &subtitle);
    assert!(
        count_labels(&test) > before,
        "clicking an item did not reveal its description"
    );
    let subtitle = find_node_with_text(&test, " · #").expect("item subtitle");
    click_node(&mut test, &subtitle);
    assert_eq!(count_labels(&test), before, "the row did not collapse");

    assert!(click_text(&mut test, "Styles"));
    let before = count_labels(&test);
    let subtitle = find_node_with_text(&test, " · #").expect("style subtitle");
    click_node(&mut test, &subtitle);
    assert!(
        count_labels(&test) > before,
        "clicking a style did not reveal its description"
    );
}

#[test]
fn style_upgrades_are_bought_and_removed_in_steps() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    if !fixture.game_data {
        eprintln!("skipping: no game data to resolve upgrades");
        return;
    }
    let mut session = jadeforge_gui::Session::with_directory(fixture.save_dir.clone());
    assert!(session.open(0));
    let Some(style) = session.styles().first().cloned() else {
        eprintln!("skipping: the test savegame has no styles");
        return;
    };
    // A family with at least three free tiers, starting at tier 1.
    let mut family = None;
    for candidate in &style.upgrades {
        let mut tiers: Vec<_> = style
            .upgrades
            .iter()
            .filter(|upgrade| upgrade.family == candidate.family && !upgrade.taken)
            .collect();
        tiers.sort_by_key(|upgrade| upgrade.tier);
        if tiers.len() >= 3 && tiers[0].tier == 1 {
            family = Some((tiers[0].slot, tiers[2].slot, tiers[1].slot));
            break;
        }
    }
    let Some((first, third, second)) = family else {
        eprintln!("skipping: no family with three free tiers");
        return;
    };
    let before = style.level;

    // Jumping from nothing to tier 3 buys 1, 2 and 3 at once.
    assert!(
        session.change_upgrade(style.struct_index, third),
        "{}",
        session.message().text
    );
    let styles = session.styles();
    let after = styles
        .iter()
        .find(|candidate| candidate.struct_index == style.struct_index)
        .expect("style");
    assert_eq!(after.level, before + 3);
    for slot in [first, second, third] {
        let taken = after
            .upgrades
            .iter()
            .find(|upgrade| upgrade.slot == slot)
            .is_some_and(|upgrade| upgrade.taken);
        assert!(taken, "slot {slot} should be owned");
    }

    // Clicking the first tier removes the whole family again.
    assert!(
        session.change_upgrade(style.struct_index, first),
        "{}",
        session.message().text
    );
    let styles = session.styles();
    let restored = styles
        .iter()
        .find(|candidate| candidate.struct_index == style.struct_index)
        .expect("style");
    assert_eq!(restored.level, before);
    assert_eq!(restored.mask, style.mask);
}

#[test]
fn style_rows_have_no_level_stepper() {
    // The level is derived from the upgrades; + / - buttons must not come back.
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut test = launch_sized(fixture.save_dir.clone(), 900.0);
    assert!(click_text(&mut test, "Styles"));
    let exact = |text: &str| {
        test.find(|_, element| {
            let label = Label::try_downcast(element)?;
            (label.text == text).then_some(())
        })
        .is_some()
    };
    assert!(!exact("+"), "a level stepper is back");
    assert!(!exact("−"), "a level stepper is back");
}

#[test]
fn clicking_an_upgrade_orb_changes_the_style() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    if !fixture.game_data {
        eprintln!("skipping: no game data to resolve upgrades");
        return;
    }
    let mut test = launch_sized(fixture.save_dir.clone(), 1400.0);
    assert!(click_text(&mut test, "Styles"));
    let subtitle = find_node_with_text(&test, " · #").expect("style subtitle");
    click_node(&mut test, &subtitle);
    assert!(find_label_text(&test, "UPGRADES").is_some());
    // Orbs are plain circles without text: find the first 26x26 node.
    let orb = test
        .find(|node, _| {
            let size = node.layout().area.size;
            ((size.width - 26.0).abs() < 0.5 && (size.height - 26.0).abs() < 0.5).then_some(node)
        })
        .expect("no upgrade orb");
    click_node(&mut test, &orb);
    let changed =
        find_label_text(&test, "Bought").is_some() || find_label_text(&test, "Removed").is_some();
    assert!(changed, "clicking an orb did not change an upgrade");
}

#[test]
fn every_saved_style_mask_fits_its_slot_table() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    if !fixture.game_data {
        // Style slot tables live in the game data; without it every style is
        // legitimately unresolvable, so there is nothing to check.
        eprintln!("skipping: no game data to resolve the style slot tables");
        return;
    }
    // With a real corpus, walk all of it; the copied fixture contains only the
    // smallest savegame, which may have no upgrades.
    let dir = save_dir_from_env().unwrap_or_else(|| fixture.save_dir.clone());
    let mut session = jadeforge_gui::Session::with_directory(&dir);
    if session.entries().is_empty() {
        eprintln!("skipping: no savegames");
        return;
    }
    let mut checked = 0_usize;
    for index in 0..session.entries().len() {
        if !session.open(index) {
            continue;
        }
        for style in session.styles().iter() {
            let Some(choices) = session
                .game()
                .as_ref()
                .and_then(|game| game.style_choices(style.style_id))
            else {
                continue;
            };
            let slots = choices.len();
            for slot in 0..64_u16 {
                if style.mask & (1_u64 << slot) != 0 {
                    assert!(
                        usize::from(slot) < slots,
                        "{}: bit {} outside {slots} slots",
                        session.entries()[index].label,
                        slot + 1
                    );
                    checked += 1;
                }
            }
            assert_eq!(u32::from(style.level), 1 + style.mask.count_ones());
        }
    }
    assert!(checked > 0, "no upgrade bits were checked");
}

#[test]
fn gems_can_be_moved_and_removed() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut session = jadeforge_gui::Session::with_directory(fixture.save_dir.clone());
    assert!(session.open(0));
    let socketed = session
        .gems()
        .iter()
        .filter(|gem| gem.location == jadeforge_core::GemLocation::Amulet)
        .count();
    let sockets = session
        .amulet()
        .map_or(0, |amulet| usize::from(amulet.sockets));
    let Some(gem) = session
        .gems()
        .iter()
        .find(|gem| gem.location == jadeforge_core::GemLocation::Amulet || socketed < sockets)
        .cloned()
    else {
        eprintln!("skipping: no gem can be moved to a visible slot");
        return;
    };
    let before = gem.location;
    session.move_gem(gem.struct_index);
    let gems = session.gems();
    let moved = gems
        .iter()
        .find(|candidate| candidate.struct_index == gem.struct_index)
        .expect("gem");
    assert_ne!(moved.location, before);
    session.remove_gem(gem.struct_index);
    assert!(
        session
            .gems()
            .iter()
            .all(|candidate| candidate.struct_index != gem.struct_index)
    );
}

#[test]
fn gems_from_the_game_data_can_be_added() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    if !fixture.game_data {
        eprintln!("skipping: no game data");
        return;
    }
    let mut session = jadeforge_gui::Session::with_directory(fixture.save_dir.clone());
    assert!(session.open(0));
    let available = session.available_gems();
    if available.is_empty() {
        eprintln!("skipping: no gems");
        return;
    }
    let before = session.gems().len();
    let gem = available[0].clone();
    session.add_gem(gem.item_id);
    assert_eq!(session.gems().len(), before + 1);
    assert!(
        session.gems().iter().any(|added| added.name == gem.name),
        "{}",
        session.message().text
    );
}

#[test]
fn the_gems_tab_lists_amulet_and_inventory() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut test = launch_sized(fixture.save_dir.clone(), 900.0);
    assert!(click_text(&mut test, "Gems"));
    assert!(find_label_text(&test, "Amulet").is_some());
    assert!(
        find_label_text(&test, "sockets filled").is_some()
            || find_label_text(&test, "No amulet yet").is_some(),
        "the amulet card did not render its state"
    );
}

#[test]
fn an_empty_carried_gems_list_does_not_report_a_gem() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let has_amulet = probe_session(&fixture).is_some_and(|session| session.amulet().is_some());
    let mut test = launch_sized(fixture.save_dir.clone(), 1800.0);
    assert!(click_text(&mut test, "Gems"));
    // Remove every real gem through the visible row action; empty socket rows
    // have no Remove button, so this also exercises the amulet placeholders.
    for _ in 0..40 {
        jadeforge_gui::dialog::set_next_confirm_for_tests(true);
        if !click_text(&mut test, "Remove") {
            break;
        }
        test.sync_and_update();
    }
    assert!(
        find_label_text(&test, "0 carried gems").is_some(),
        "the empty carried-gems hint still counts its own placeholder row"
    );
    assert!(
        find_label_text(&test, "No gems carried in the inventory.").is_some(),
        "the empty carried-gems list has no empty hint"
    );
    assert!(
        find_label_exact(&test, "1 gem").is_none(),
        "the empty carried-gems hint claims there is a gem"
    );
    assert!(
        find_label_exact(&test, "1 carried gem").is_none(),
        "the empty carried-gems hint claims there is a carried gem"
    );
    if has_amulet {
        // The amulet summary names both numbers and the tier.
        assert!(
            find_label_text(&test, "sockets filled").is_some(),
            "the amulet summary does not name the socket count"
        );
        assert!(
            find_label_text(&test, "amulet tier").is_some(),
            "the amulet summary does not name the tier"
        );
    } else {
        assert!(find_label_text(&test, "No amulet yet").is_some());
    }
}

#[test]
fn expanded_styles_show_upgrade_choices() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    if !fixture.game_data {
        eprintln!("skipping: no game data to resolve upgrades");
        return;
    }
    let mut test = launch_sized(fixture.save_dir.clone(), 1400.0);
    assert!(click_text(&mut test, "Styles"));
    let subtitle = find_node_with_text(&test, " · #").expect("style subtitle");
    click_node(&mut test, &subtitle);
    assert!(find_label_text(&test, "UPGRADES").is_some());
    assert!(
        find_label_text(&test, "Filled gold orbs are owned").is_some(),
        "the upgrade panel did not render"
    );
}

#[test]
fn pickers_share_the_same_page_size() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut session = jadeforge_gui::Session::with_directory(fixture.save_dir.clone());
    assert!(session.open(0));
    let styles = session.missing_styles().len();
    let gems = session.available_gems().len();
    let mut test = launch_sized(fixture.save_dir.clone(), 900.0);

    if styles > jadeforge_gui::app::PAGE_SIZE {
        assert!(click_text(&mut test, "Styles"));
        assert!(
            find_label_text(&test, "30 of ").is_some(),
            "styles do not render the shared page size of 30"
        );
    } else {
        eprintln!("skipping style page size: only {styles} missing styles");
    }

    if gems > jadeforge_gui::app::PAGE_SIZE {
        assert!(click_text(&mut test, "Gems"));
        assert!(
            find_label_text(&test, "30 of ").is_some(),
            "gems do not render the shared page size of 30"
        );
        assert!(click_text(&mut test, "Show more"));
        test.sync_and_update();
        assert!(
            find_label_text(&test, "60 of ").is_some(),
            "the page step is not the shared page size"
        );
    } else {
        eprintln!("skipping gem page size: only {gems} gems");
    }
}

#[test]
fn show_more_pages_through_the_item_database() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    if !fixture.game_data {
        eprintln!("skipping: no game data to browse");
        return;
    }
    let mut test = launch_sized(fixture.save_dir.clone(), 1800.0);
    assert!(click_text(&mut test, "Inventory"));
    let before = find_counter(&test).expect("browse counter");
    assert!(click_text(&mut test, "Show more"), "no show more button");
    let after = find_counter(&test).expect("browse counter after paging");
    assert_ne!(before, after, "paging did not change the counter");
    assert!(after.contains("of"), "{after}");
}

#[test]
fn clicking_around_does_not_panic() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut test = launch(fixture.save_dir.clone());
    for tab in ["Character", "Styles", "Inventory", "Companions", "Overview"] {
        assert!(click_text(&mut test, tab), "tab {tab} is missing");
        let _ = test.render();
    }
    // Hovering the savegame entry must not break the layout either.
    if let Some(node) = find_node_with_text(&test, "Test Save") {
        let area = node.layout().area;
        test.move_cursor(CursorPoint::new(
            f64::from(area.origin.x) + f64::from(area.size.width) / 2.0,
            f64::from(area.origin.y) + f64::from(area.size.height) / 2.0,
        ));
        test.sync_and_update();
        let _ = test.render();
    }
}

#[test]
fn character_edits_apply_immediately() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut test = launch(fixture.save_dir.clone());
    assert!(click_text(&mut test, "Character"));
    // "+1 point" edits the form; the value has to reach the in-memory savegame
    // immediately, exactly like an item or style edit.
    assert!(click_text(&mut test, "+1 point"), "no quick add button");
    for _ in 0..3 {
        test.sync_and_update();
    }
    assert!(
        find_label_text(&test, "unsaved changes").is_some()
            || find_label_text(&test, "in memory").is_some(),
        "the point was not applied in memory"
    );

    assert!(click_text(&mut test, "Scan"));
    assert!(
        find_label_text(&test, "unsaved changes").is_some(),
        "scanning was not blocked"
    );
}

#[test]
fn form_edits_apply_in_memory_without_a_message() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut session = jadeforge_gui::Session::with_directory(fixture.save_dir.clone());
    assert!(session.open(0));
    let character = session.character().expect("character").clone();
    let mut form = jadeforge_gui::CharacterForm::from_character(&character);
    form.level = "9".to_owned();
    let before = session.message().text.clone();
    assert!(session.apply_form_silently(&form));
    assert!(session.is_dirty());
    assert_eq!(
        session.message().text,
        before,
        "auto-apply must stay silent"
    );
}

fn scroll_offset(test: &TestingRunner) -> f32 {
    // Freya 0.5 renders the scrollbar in an overlaid strip and moves its thumb
    // with `offset_y` even when the content is at the top, so the thumb reports
    // a small inset offset that is not a content translation. Content offsets
    // are much larger than the 20 px strip; ignore the strip's own movement.
    const SCROLLBAR_STRIP: f32 = 20.0;
    test.find_many(|node, _| {
        let value = node.layout().offset_y.get();
        (value.abs() > SCROLLBAR_STRIP).then_some(value.abs())
    })
    .into_iter()
    .fold(0.0_f32, f32::max)
}

#[test]
fn tabs_keep_their_own_scroll_position() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut test = launch_sized(fixture.save_dir.clone(), 800.0);
    assert!(click_text(&mut test, "Styles"));
    assert!(scroll_offset(&test) < 1.0, "Styles starts scrolled");

    assert!(click_text(&mut test, "Inventory"));
    assert!(scroll_offset(&test) < 1.0, "Inventory starts scrolled");
    // The wheel event only reaches the scroll view when the pointer is over it.
    test.move_cursor(CursorPoint::new(600.0, 350.0));
    test.sync_and_update();
    test.scroll(
        CursorPoint::new(600.0, 350.0),
        CursorPoint::new(0.0, -600.0),
    );
    test.sync_and_update();
    let inventory_offset = scroll_offset(&test);
    if inventory_offset < 1.0 {
        // Without game data the inventory is short; there is nothing to scroll.
        eprintln!("skipping: the inventory is not scrollable at this window size");
        return;
    }

    assert!(click_text(&mut test, "Styles"));
    assert!(
        scroll_offset(&test) < 1.0,
        "Styles shares Inventory's scroll"
    );

    assert!(click_text(&mut test, "Inventory"));
    let restored = scroll_offset(&test);
    assert!(
        (restored - inventory_offset).abs() < 1.0,
        "Inventory lost its scroll position ({restored} instead of {inventory_offset})"
    );
}

#[test]
fn savegames_can_be_filtered_and_sorted() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    // A second savegame so filtering has something to hide.
    let second = fixture.save_dir.join("Other Save");
    std::fs::create_dir_all(&second).expect("create second save");
    std::fs::copy(
        fixture.save_dir.join("Test Save").join("save.sac"),
        second.join("save.sac"),
    )
    .expect("copy savegame");

    let mut test = launch_sized(fixture.save_dir.clone(), 900.0);
    assert!(find_label_text(&test, "2 SAVEGAMES").is_some());

    // Filter by name.
    let filter = find_paragraph(&test, "Filter by name or character").expect("filter input");
    click_node(&mut test, &filter);
    test.write_text("Other");
    test.sync_and_update();
    assert!(
        find_label_text(&test, "1 OF 2 SAVEGAMES").is_some(),
        "the list was not filtered"
    );

    // Clear the filter again by selecting everything and deleting it.
    test.write_text("\u{8}".repeat(5));
    test.sync_and_update();

    // Sorting: the active key shows a direction arrow, clicking it flips.
    assert!(find_label_exact(&test, "Time played").is_some());
    assert!(click_label_exact(&mut test, "Time played"));
    assert!(find_label_text(&test, "↑").is_some());
    assert!(click_label_exact(&mut test, "Time played"));
    assert!(find_label_text(&test, "↓").is_some());
}

#[test]
fn reverting_asks_before_discarding_changes() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut test = launch_sized(fixture.save_dir.clone(), 1000.0);
    assert!(click_text(&mut test, "Character"));
    assert!(click_text(&mut test, "+1 point"));
    for _ in 0..3 {
        test.sync_and_update();
    }
    assert!(find_label_text(&test, "unsaved changes").is_some());

    jadeforge_gui::dialog::set_next_confirm_for_tests(false);
    assert!(click_text(&mut test, "Revert"));
    assert!(
        find_label_text(&test, "unsaved changes").is_some(),
        "canceling Revert must keep the unsaved changes"
    );

    jadeforge_gui::dialog::set_next_confirm_for_tests(true);
    assert!(click_text(&mut test, "Revert"));
    assert!(
        find_label_text(&test, "all changes saved").is_some(),
        "confirming Revert must reload the savegame"
    );
}

#[test]
fn removing_an_item_asks_for_confirmation() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut test = launch_sized(fixture.save_dir.clone(), 1800.0);
    assert!(click_text(&mut test, "Inventory"));

    jadeforge_gui::dialog::set_next_confirm_for_tests(false);
    assert!(click_text(&mut test, "Remove"));
    assert!(
        find_label_text(&test, "Removed").is_none(),
        "canceling the confirmation must keep the item"
    );

    jadeforge_gui::dialog::set_next_confirm_for_tests(true);
    assert!(click_text(&mut test, "Remove"));
    assert!(
        find_label_text(&test, "Removed").is_some(),
        "confirming the removal must remove the item"
    );
}

#[test]
fn the_browse_button_uses_the_folder_dialog() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut test = launch_sized(fixture.save_dir.clone(), 900.0);

    // The headless runner cannot open a portal dialog, so the test seam answers
    // it with the fixture folder.
    jadeforge_gui::dialog::set_next_pick_for_tests(Some(fixture.save_dir.clone()));
    assert!(click_text(&mut test, "Browse"), "no browse button");
    wait_for_text(&mut test, "Found 1 savegame");

    // Cancelling changes nothing.
    jadeforge_gui::dialog::set_next_pick_for_tests(None);
    assert!(click_text(&mut test, "Browse"));
    wait_for_text(&mut test, "Found 1 savegame");
}

#[test]
fn the_folder_picker_reports_cancellation_and_unreadable_paths() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut test = launch_sized(fixture.save_dir.clone(), 900.0);

    // Cancelling the native dialog is its own state, not an empty folder.
    jadeforge_gui::dialog::set_next_pick_for_tests(None);
    assert!(click_text(&mut test, "Browse"), "no browse button");
    assert!(
        find_label_text(&test, "No folder was chosen").is_some(),
        "cancelling the folder dialog is not reported"
    );
    assert!(
        find_label_text(&test, "1 SAVEGAME").is_some(),
        "cancelling the folder dialog changed the save list"
    );

    // A picked path that is not a folder is reported distinctly.
    let file = fixture.root.join("not-a-folder");
    std::fs::write(&file, b"x").expect("write file");
    jadeforge_gui::dialog::set_next_pick_for_tests(Some(file));
    assert!(click_text(&mut test, "Browse"));
    assert!(
        find_label_text(&test, "is a file, not a save folder").is_some(),
        "picking a file is not reported as a distinct failure"
    );
    assert!(find_label_text(&test, "1 SAVEGAME").is_some());

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let unreadable = fixture.root.join("unreadable");
        std::fs::create_dir_all(&unreadable).expect("create folder");
        std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o000))
            .expect("chmod");
        let denied = std::fs::read_dir(&unreadable).is_err();
        if denied {
            jadeforge_gui::dialog::set_next_pick_for_tests(Some(unreadable.clone()));
            assert!(click_text(&mut test, "Browse"));
            assert!(
                find_label_text(&test, "cannot be listed").is_some(),
                "a permission failure is not reported as a read error"
            );
            assert!(find_label_text(&test, "1 SAVEGAME").is_some());
        } else {
            eprintln!("skipping the permission check: this user can read 0o000 folders");
        }
        let _ = std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o755));
    }
}

#[test]
fn the_character_form_shows_units_and_the_optional_gold_field() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut test = launch_sized(fixture.save_dir.clone(), 1400.0);
    assert!(click_text(&mut test, "Character"));
    assert!(
        find_label_text(&test, "health points").is_some(),
        "the health fields do not show their unit"
    );
    assert!(
        find_label_text(&test, "1 to 255 (level)").is_some(),
        "the level field does not combine its range and unit"
    );
    assert!(
        find_label_text(&test, "1 to 1000000 health points").is_some(),
        "the maximum health field does not show its unit"
    );
    assert!(
        find_label_text(&test, "0 to 100 (chapter)").is_some(),
        "the chapter field does not say what it counts"
    );
    assert!(
        find_label_text(&test, "0 to 1000000 (index)").is_some(),
        "the appearance field does not say it is an index"
    );
    assert!(
        find_label_text(&test, "128 balanced").is_some(),
        "the alignment field does not explain its balanced value"
    );
    assert!(
        find_label_text(&test, "Gold is optional in the savegame").is_some(),
        "the optional Gold field is not explained"
    );
}

#[test]
fn broken_savegames_are_reported() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let broken = fixture.save_dir.join("ZZ Broken Save");
    std::fs::create_dir_all(&broken).expect("create broken save");
    std::fs::write(broken.join("save.sac"), b"SAV V3.2 not really").expect("write broken save");

    let save_dir = fixture.save_dir.clone();
    let (mut test, ()) = TestingRunner::new(
        move || app_with_directory(save_dir.clone()),
        (1280.0_f32, 800.0_f32).into(),
        |_| {},
        1.0,
    );
    test.sync_and_update();
    wait_for_text(&mut test, "ZZ Broken Save");

    let scan = find_label_text(&test, "cannot be read").expect("scan warning");
    assert!(scan.contains("1 of them"), "{scan}");
    assert!(
        find_label_text(&test, "Cannot be read").is_some(),
        "the broken entry has no subtitle"
    );
    assert!(click_text(&mut test, "ZZ Broken Save"));
    assert!(find_label_text(&test, "Cannot open").is_some());
}

#[test]
fn empty_folder_shows_a_hint() {
    let root = std::env::temp_dir().join(format!("jadeforge-empty-{}", std::process::id()));
    std::fs::create_dir_all(&root).expect("create empty dir");
    let (mut test, ()) = TestingRunner::new(
        {
            let root = root.clone();
            move || app_with_directory(root.clone())
        },
        (1000., 700.).into(),
        |_| {},
        1.0,
    );
    test.sync_and_update();
    wait_for_text(&mut test, "No savegames found");
    let message = find_label_text(&test, "No savegames found").expect("hint");
    assert!(message.contains("No savegames found"));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn unsaved_changes_are_protected() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut session = jadeforge_gui::Session::with_directory(fixture.save_dir.clone());
    assert_eq!(session.entries().len(), 1);
    assert!(session.open(0));
    let character = session.character().expect("character").clone();
    let mut form = jadeforge_gui::CharacterForm::from_character(&character);
    form.level = "9".to_owned();
    session.apply(&form).expect("apply");
    assert!(session.is_dirty());

    // Scanning another folder must not throw the changes away.
    session.set_directory(std::env::temp_dir());
    assert_eq!(
        session.directory(),
        fixture.save_dir,
        "folder changed anyway"
    );
    assert!(session.is_dirty(), "the changes were dropped");
    assert!(
        session.message().text.contains("unsaved"),
        "{}",
        session.message().text
    );

    session.revert().expect("revert");
    assert!(!session.is_dirty());
}

/// The AccessKit node the element contributes to the tree.
///
/// `finish_accessibility` is what Freya applies while building the real tree
/// (it fills label and paragraph values from their text), so tests inspect that
/// node instead of the raw builder to assert what assistive technology sees.
fn a11y_node(node: &freya_testing::TestingNode) -> accesskit::Node {
    let element = node.element();
    let mut builder = element.accessibility().builder.clone();
    element.finish_accessibility(&mut builder);
    builder
}

fn a11y_role(node: &freya_testing::TestingNode) -> String {
    format!("{:?}", a11y_node(node).role())
}

fn a11y_value(node: &freya_testing::TestingNode) -> Option<String> {
    a11y_node(node)
        .value()
        .map(std::string::ToString::to_string)
}

fn a11y_label(node: &freya_testing::TestingNode) -> Option<String> {
    a11y_node(node)
        .label()
        .map(std::string::ToString::to_string)
}

fn a11y_selected(node: &freya_testing::TestingNode) -> bool {
    a11y_node(node).is_selected().unwrap_or(false)
}

fn selected_tab_labels(test: &TestingRunner) -> Vec<String> {
    test.find_many(|node, _| (a11y_role(&node) == "Tab").then_some(node))
        .iter()
        .filter(|node| a11y_selected(node))
        .filter_map(a11y_label)
        .collect()
}

fn a11y_focusable(node: &freya_testing::TestingNode) -> bool {
    let element = node.element();
    element.accessibility().a11y_focusable.is_enabled()
}

fn a11y_live(node: &freya_testing::TestingNode) -> String {
    format!("{:?}", a11y_node(node).live())
}

#[test]
fn static_text_is_exposed_to_assistive_technology() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut test = launch_sized(fixture.save_dir.clone(), 1000.0);
    assert!(click_text(&mut test, "Character"));

    for text in [
        "JadeForge",
        "SAVE FOLDER",
        "1 SAVEGAME",
        "all changes saved",
        "Identity",
        "NAME",
        "LEVEL",
        "GOLD",
    ] {
        let node = find_label_exact(&test, text).unwrap_or_else(|| panic!("missing {text:?}"));
        assert_eq!(
            a11y_value(&node).as_deref(),
            Some(text),
            "the static label {text:?} has no accessible value"
        );
    }

    let identity = find_label_exact(&test, "Identity").expect("Identity heading");
    assert_eq!(a11y_role(&identity), "Heading");

    let status = find_node_with_text(&test, "Opened Test Save").expect("status message");
    assert_eq!(a11y_role(&status), "Status");
    assert_ne!(a11y_live(&status), "Off", "the status is not a live region");

    let name_group = test.find_many(|node, _| {
        (a11y_role(&node) == "Group" && a11y_label(&node).is_some_and(|label| label == "NAME"))
            .then_some(node)
    });
    assert_eq!(
        name_group.len(),
        1,
        "the NAME field has no accessible group"
    );
}

#[test]
fn assistive_technology_can_activate_controls() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut test = launch_sized(fixture.save_dir.clone(), 1000.0);
    assert!(
        find_label_exact(&test, "Identity").is_none(),
        "the Character tab should not be rendered yet"
    );

    let tab = test
        .find(|node, _| {
            (a11y_role(&node) == "Tab" && a11y_label(&node).as_deref() == Some("Character"))
                .then_some(node)
        })
        .expect("Character tab");
    let target = tab
        .element()
        .accessibility()
        .a11y_id
        .expect("the tab has no accessibility id");
    let request = accesskit::ActionRequest {
        action: accesskit::Action::Click,
        target_tree: accesskit::TreeId::ROOT,
        target_node: target,
        data: None,
    };
    assert!(
        test.send_accessibility_action(request),
        "the accessible tree does not know the Character tab"
    );
    test.sync_and_update();
    assert!(
        find_label_exact(&test, "Identity").is_some(),
        "a screen reader Click did not activate the Character tab"
    );
}

#[test]
fn tabs_and_the_save_list_expose_roles_states_and_focus() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let test = launch_sized(fixture.save_dir.clone(), 1000.0);

    let tabs = test.find_many(|node, _| (a11y_role(&node) == "Tab").then_some(node));
    assert_eq!(tabs.len(), 6, "expected six tab buttons");
    assert_eq!(
        tabs.iter().filter(|node| a11y_selected(node)).count(),
        1,
        "exactly one tab must be selected"
    );
    let tab_list = test.find_many(|node, _| (a11y_role(&node) == "TabList").then_some(node));
    assert_eq!(tab_list.len(), 1, "missing the tab list role");

    let options = test.find_many(|node, _| (a11y_role(&node) == "ListBoxOption").then_some(node));
    assert_eq!(options.len(), 1, "expected one save option");
    assert_eq!(
        options.iter().filter(|node| a11y_selected(node)).count(),
        1,
        "the opened save must be selected"
    );
    let list_boxes = test.find_many(|node, _| (a11y_role(&node) == "ListBox").then_some(node));
    assert_eq!(list_boxes.len(), 1, "missing the list box role");
    assert_eq!(
        list_boxes
            .iter()
            .filter(|node| a11y_focusable(node))
            .count(),
        1,
        "the save list must be exactly one tab stop"
    );
    assert_eq!(
        options.iter().filter(|node| a11y_focusable(node)).count(),
        0,
        "individual options must not add to the Tab order"
    );
    assert!(
        a11y_label(&options[0])
            .as_deref()
            .is_some_and(|label| label.contains("Test Save")),
        "the save option lost its accessible name"
    );
}

#[test]
fn the_save_list_keeps_one_tab_stop_with_many_saves() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    for suffix in 2..=4 {
        let folder = fixture.save_dir.join(format!("Test Save {suffix}"));
        std::fs::create_dir_all(&folder).expect("create extra save folder");
        std::fs::copy(
            fixture.save_dir.join("Test Save").join("save.sac"),
            folder.join("save.sac"),
        )
        .expect("copy savegame");
    }
    let mut test = launch_unopened_at(fixture.save_dir.clone(), 900.0, 700.0);
    settle(&mut test);
    let options = test.find_many(|node, _| (a11y_role(&node) == "ListBoxOption").then_some(node));
    // Only the rows that fit the viewport are built; with four savegames that
    // still has to include the ones on screen.
    assert!(!options.is_empty(), "the save list built no rows at all");
    assert!(options.len() <= 4, "more rows than savegames were built");
    let list_boxes = test.find_many(|node, _| (a11y_role(&node) == "ListBox").then_some(node));
    assert_eq!(list_boxes.len(), 1);
    assert_eq!(
        list_boxes
            .iter()
            .filter(|node| a11y_focusable(node))
            .count(),
        1,
        "the list box must be one tab stop"
    );
    assert_eq!(
        options.iter().filter(|node| a11y_focusable(node)).count(),
        0,
        "a list box must not put every row in the Tab order"
    );
}

#[test]
fn the_save_list_only_builds_visible_rows_and_still_reaches_the_end() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    // Hard links keep this cheap while still giving the scan 41 distinct
    // savegames. The zero-padded suffix keeps the name sort predictable.
    let source = fixture.save_dir.join("Test Save").join("save.sac");
    for suffix in 2..=41 {
        let folder = fixture.save_dir.join(format!("Test Save {suffix:02}"));
        std::fs::create_dir_all(&folder).expect("create extra save folder");
        std::fs::hard_link(&source, folder.join("save.sac")).expect("link savegame");
    }
    let mut test = launch_unopened_at(fixture.save_dir.clone(), 900.0, 700.0);
    settle(&mut test);
    let options = test.find_many(|node, _| (a11y_role(&node) == "ListBoxOption").then_some(node));
    assert!(!options.is_empty(), "the save list built no rows");
    assert!(
        options.len() < 20,
        "a virtualized list must not build all 41 savegames, built {}",
        options.len()
    );

    // End moves the roving focus to the last savegame, scrolls it into view and
    // Enter opens it, so virtualization did not break keyboard browsing.
    assert!(click_text(&mut test, "Level "), "clicking a save failed");
    test.press_key(Key::Named(NamedKey::End));
    test.sync_and_update();
    test.press_key(Key::Named(NamedKey::Enter));
    test.sync_and_update();
    assert!(
        find_label_text(&test, "Opened Test Save 41").is_some(),
        "End + Enter did not open the last savegame"
    );
}

#[test]
fn the_save_list_scrolls_to_the_end_with_the_wheel() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let source = fixture.save_dir.join("Test Save").join("save.sac");
    for suffix in 2..=41 {
        let folder = fixture.save_dir.join(format!("Test Save {suffix:02}"));
        std::fs::create_dir_all(&folder).expect("create extra save folder");
        std::fs::hard_link(&source, folder.join("save.sac")).expect("link savegame");
    }
    let mut test = launch_unopened_at(fixture.save_dir.clone(), 900.0, 700.0);
    settle(&mut test);
    assert!(
        find_label_exact(&test, "Test Save").is_some(),
        "the first row is missing"
    );
    // Open a savegame like a real session: the preview shrinks the list and the
    // selected row is revealed. Scrolling must not keep snapping back to it.
    assert!(click_text(&mut test, "Level "), "could not open a save");
    settle(&mut test);

    let list = test
        .find(|node, _| (a11y_role(&node) == "ListBox").then_some(node))
        .expect("save list");
    let area = list.layout().area;
    let wheel_at = CursorPoint::new(
        f64::from(area.origin.x) + 20.0,
        f64::from(area.origin.y) + f64::from(area.size.height) / 2.0,
    );
    test.move_cursor(wheel_at);
    test.sync_and_update();

    for _ in 0..40 {
        test.scroll(wheel_at, CursorPoint::new(0.0, -200.0));
    }
    assert!(
        scroll_offset(&test) > 0.0,
        "the save list did not scroll at all"
    );
    assert!(
        find_label_exact(&test, "Test Save 41").is_some(),
        "the wheel did not reach the last savegame"
    );

    for _ in 0..40 {
        test.scroll(wheel_at, CursorPoint::new(0.0, 200.0));
    }
    assert!(
        find_label_exact(&test, "Test Save").is_some(),
        "the wheel did not return to the first savegame"
    );
}

#[test]
fn the_save_list_scrollbar_reaches_the_end() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let source = fixture.save_dir.join("Test Save").join("save.sac");
    for suffix in 2..=41 {
        let folder = fixture.save_dir.join(format!("Test Save {suffix:02}"));
        std::fs::create_dir_all(&folder).expect("create extra save folder");
        std::fs::hard_link(&source, folder.join("save.sac")).expect("link savegame");
    }
    let mut test = launch_unopened_at(fixture.save_dir.clone(), 900.0, 700.0);
    settle(&mut test);
    let list = test
        .find(|node, _| (a11y_role(&node) == "ListBox").then_some(node))
        .expect("save list");
    let area = list.layout().area;
    let x = f64::from(area.origin.x + area.size.width) - 10.0;
    let top = f64::from(area.origin.y) + 30.0;
    let bottom = f64::from(area.origin.y + area.size.height) - 10.0;
    test.move_cursor(CursorPoint::new(x, top));
    test.press_cursor(CursorPoint::new(x, top));
    for step in 1..=10 {
        let y = top + (bottom - top) * f64::from(step) / 10.0;
        test.move_cursor(CursorPoint::new(x, y));
    }
    test.release_cursor(CursorPoint::new(x, bottom));
    test.sync_and_update();
    assert!(
        find_label_exact(&test, "Test Save 41").is_some(),
        "dragging the scrollbar did not reach the last savegame"
    );
}

#[test]
fn arrow_keys_move_the_save_list_focus() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    for suffix in 2..=3 {
        let folder = fixture.save_dir.join(format!("Test Save {suffix}"));
        std::fs::create_dir_all(&folder).expect("create extra save folder");
        std::fs::copy(
            fixture.save_dir.join("Test Save").join("save.sac"),
            folder.join("save.sac"),
        )
        .expect("copy savegame");
    }
    let mut test = launch_unopened_at(fixture.save_dir.clone(), 900.0, 700.0);
    assert!(
        click_text(&mut test, "Test Save"),
        "could not focus the save list"
    );
    test.press_key(Key::Named(NamedKey::ArrowDown));
    test.sync_and_update();

    test.press_key(Key::Named(NamedKey::Enter));
    test.sync_and_update();
    assert!(
        find_label_text(&test, "Opened Test Save 2").is_some(),
        "Enter did not open the row that received keyboard focus"
    );
}

#[test]
fn a_save_without_gold_opens_without_an_invalid_form() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    // Remove the optional gold field from the copied fixture, then save it
    // back. This is the shape of the two retail saves that used to open as
    // invalid and could not be switched or rescanned.
    {
        let path = fixture.save_dir.join("Test Save").join("save.sac");
        let mut save = jadeforge_core::SaveGame::open(&path).expect("open fixture save");
        save.block_mut("save.sav")
            .expect("save block")
            .gff_mut()
            .remove("PlayerCreature.Stats.Gold")
            .expect("remove gold");
        save.save(jadeforge_core::SaveOptions { backup: false })
            .expect("save fixture without gold");
    }

    let mut test = launch_sized(fixture.save_dir.clone(), 1000.0);
    assert!(
        find_label_text(&test, "invalid field").is_none(),
        "a save without gold was marked invalid"
    );
    assert!(
        find_label_text(&test, "invalid fields").is_none(),
        "a save without gold was marked invalid"
    );
    assert!(
        find_label_text(&test, "they are not applied").is_none(),
        "the footer reported an invalid form"
    );
    assert!(click_text(&mut test, "Character"));
    assert!(
        find_paragraph(&test, "not set").is_some(),
        "the absent gold field should show its placeholder"
    );
}

/// Layout area of an exact label, which is the visible anchor of its field.
fn label_area(test: &TestingRunner, wanted: &str) -> freya::prelude::Area {
    find_label_exact(test, wanted)
        .expect("the label should be rendered")
        .layout()
        .area
}

fn assert_same_row(test: &TestingRunner, left: &str, right: &str) {
    let left = label_area(test, left);
    let right = label_area(test, right);
    assert!(
        (left.origin.y - right.origin.y).abs() < 0.5,
        "{left:?} and {right:?} are not on the same row ({} vs {})",
        left.origin.y,
        right.origin.y
    );
}

fn assert_different_rows(test: &TestingRunner, left: &str, right: &str) {
    let left = label_area(test, left);
    let right = label_area(test, right);
    assert!(
        (left.origin.y - right.origin.y).abs() > 10.0,
        "{left:?} and {right:?} wrapped onto the same row unexpectedly"
    );
}

#[test]
fn identity_fields_use_three_columns_at_the_default_and_minimum_size() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    for (width, height) in [(1180.0_f32, 760.0_f32), (900.0, 700.0)] {
        let mut test = launch_unopened_at(
            fixture.save_dir.clone(),
            f64::from(width),
            f64::from(height),
        );
        assert!(click_text(&mut test, "Level "), "the savegame did not open");
        assert!(
            click_label_exact(&mut test, "Character"),
            "the Character tab is missing"
        );

        // The six identity fields are two deliberate rows of three.
        assert_same_row(&test, "NAME", "LEVEL");
        assert_same_row(&test, "LEVEL", "EXPERIENCE");
        assert_same_row(&test, "GOLD", "CHAPTER");
        assert_same_row(&test, "CHAPTER", "KILLS");
        assert_different_rows(&test, "NAME", "GOLD");
        // The regression: KILLS used to sit alone on a third row.
        assert_different_rows(&test, "EXPERIENCE", "GOLD");

        for field in ["NAME", "EXPERIENCE", "KILLS"] {
            let area = label_area(&test, field);
            assert!(
                area.origin.x + area.size.width <= width,
                "{field} runs off the {width}px window"
            );
        }
    }
}

#[test]
fn overview_glance_rows_use_the_full_width_at_every_size() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };

    for width in [900.0, 1280.0, 1600.0] {
        let mut test = launch_unopened_at(fixture.save_dir.clone(), width, 900.0);
        assert!(click_text(&mut test, "Level "), "the savegame did not open");
        // Every glance value is one full-width row: no fixed 340 px cell can
        // leave half of a 900 px card empty while the next row starts below.
        // The labels are chosen from the glance section only; "Inventory" and
        // "Styles" are also tab labels on one row.
        assert_different_rows(&test, "Unspent attribute points", "Body / Mind / Spirit");
        assert_different_rows(
            &test,
            "Body / Mind / Spirit",
            "Charm / Intuition / Intimidate",
        );
        // The four hero stats stay a 2x2 grid instead of wrapping 3+1.
        assert_same_row(&test, "GOLD", "HEALTH");
        assert_same_row(&test, "CHI", "FOCUS");
        assert_different_rows(&test, "GOLD", "CHI");
    }
}

#[test]
fn the_editor_column_stops_growing_on_wide_windows() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    // "Unspent attribute points" is the first cell of the second glance row, so
    // its x is the left edge of the editor column.
    let mut wide = launch_unopened_at(fixture.save_dir.clone(), 1600.0, 900.0);
    assert!(click_text(&mut wide, "Level "), "the savegame did not open");
    let capped = label_area(&wide, "Unspent attribute points").origin.x;
    assert!(
        (390.0..480.0).contains(&capped),
        "the editor column starts at {capped}px on a 1600px window; it is not capped and centred"
    );

    let mut narrow = launch_unopened_at(fixture.save_dir.clone(), 900.0, 900.0);
    assert!(
        click_text(&mut narrow, "Level "),
        "the savegame did not open"
    );
    let flush = label_area(&narrow, "Unspent attribute points").origin.x;
    assert!(
        flush < 300.0,
        "the capped column should only apply on wide windows; it starts at {flush}px at 900"
    );
}

fn count_labels_containing(test: &TestingRunner, needle: &str) -> usize {
    test.find_many(|_, element| {
        let label = Label::try_downcast(element)?;
        label.text.contains(needle).then_some(())
    })
    .len()
}

#[test]
fn clipped_names_show_the_full_text_in_a_tooltip() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut test = launch_unopened(fixture.save_dir.clone(), 900.0);
    let row = find_label_exact(&test, "Test Save").expect("the save row is missing");
    let before = count_labels_containing(&test, "Test Save");
    let area = row.layout().area;
    test.move_cursor(CursorPoint::new(
        f64::from(area.origin.x) + f64::from(area.size.width) / 2.0,
        f64::from(area.origin.y) + f64::from(area.size.height) / 2.0,
    ));
    test.sync_and_update();
    // The tooltip waits 500 ms before it appears, so poll the async task queue.
    test.poll(
        std::time::Duration::from_millis(20),
        std::time::Duration::from_millis(800),
    );
    assert!(
        count_labels_containing(&test, "Test Save") > before,
        "hovering a clipped save name did not show its tooltip"
    );
}

#[test]
fn the_overview_save_path_can_be_copied() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    // Tall window: the Savegame card sits below the fold at the default size.
    let mut test = launch_sized(fixture.save_dir.clone(), 1800.0);
    assert!(click_text(&mut test, "Copy path"), "no copy path button");
    assert!(
        find_label_text(&test, "clipboard").is_some(),
        "copying the path did not report a status message"
    );
}

#[test]
fn the_about_page_shows_the_version_settings_and_backups() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let _ = jadeforge_gui::config::ensure_test_path();
    let mut config = jadeforge_gui::config::Config::load();
    config.sort = Some("level".to_owned());
    config.sort_descending = true;
    config.store().expect("store the test settings");
    let settings_path = jadeforge_gui::config::Config::path().expect("a test settings path");

    let mut test = launch_unopened_at(fixture.save_dir.clone(), 900.0, 1200.0);
    assert!(click_label_exact(&mut test, "About"), "no About button");
    assert!(
        find_label_text(&test, "About JadeForge").is_some(),
        "the About page did not open"
    );
    assert!(
        find_label_text(&test, &format!("Version {}", jadeforge_gui::APP_VERSION)).is_some(),
        "the About page does not show the version"
    );
    assert!(
        find_label_text(&test, "SETTINGS").is_some(),
        "the About page has no settings section"
    );
    assert!(
        find_label_text(&test, "BACKUPS").is_some(),
        "the About page has no backup explanation"
    );
    assert!(
        find_label_text(&test, "Level, descending").is_some(),
        "the remembered sort order is not exposed"
    );
    assert!(
        find_text_node(&test, "jadeforge-backup").is_some(),
        "the backup explanation does not name the backup files"
    );
    assert!(
        find_text_node(&test, &settings_path.display().to_string()).is_some(),
        "the settings file path is not shown"
    );
    assert!(
        find_text_node(&test, &fixture.save_dir.display().to_string()).is_some(),
        "the remembered save folder is not shown"
    );

    jadeforge_gui::clipboard::capture_next_copy_for_tests();
    assert!(
        click_a11y_label(&mut test, "Copy the settings path"),
        "the settings file has no copy action"
    );
    assert_eq!(
        jadeforge_gui::clipboard::last_copied_for_tests(),
        Some(settings_path.display().to_string()),
        "the settings action copied the wrong path"
    );
    assert!(
        find_label_text(&test, "Copied the settings path").is_some(),
        "copying the settings path did not report success"
    );
    jadeforge_gui::paths::capture_next_open_for_tests();
    assert!(
        click_a11y_label(&mut test, "Open the settings folder"),
        "the settings file has no open action"
    );
    assert_eq!(
        jadeforge_gui::paths::last_opened_for_tests(),
        settings_path.parent().map(std::path::Path::to_path_buf),
        "the settings action opened the wrong folder"
    );

    assert!(click_label_exact(&mut test, "Close"), "no Close button");
    assert!(
        find_label_text(&test, "No savegame selected").is_some(),
        "closing About did not return to the editor"
    );
}

#[test]
fn about_clears_the_selected_tab_and_returns_to_it() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut test = launch_sized(fixture.save_dir.clone(), 1000.0);
    assert!(click_label_exact(&mut test, "Character"));
    assert!(find_label_text(&test, "Identity").is_some());
    assert_eq!(
        selected_tab_labels(&test),
        vec!["Character".to_owned()],
        "the Character tab is not selected before opening About"
    );

    let about = test
        .find(|node, _| {
            (a11y_label(&node).as_deref() == Some("About JadeForge, version, settings and backups"))
                .then_some(node)
        })
        .expect("the About button is missing");
    assert_eq!(
        a11y_role(&about),
        "Button",
        "the About entry point is not exposed as a button"
    );
    assert!(
        about.layout().area.size.height >= 27.5,
        "the About button is only {} px tall",
        about.layout().area.size.height
    );
    click_node(&mut test, &about);
    settle(&mut test);

    assert!(
        find_label_text(&test, "About JadeForge").is_some(),
        "the About page did not open"
    );
    assert!(
        find_label_text(&test, "Identity").is_none(),
        "the Character content is still shown behind About"
    );
    assert!(
        selected_tab_labels(&test).is_empty(),
        "a tab is still selected while About is shown"
    );
    assert_eq!(
        test.find_many(|node, _| (a11y_role(&node) == "Tab").then_some(node))
            .len(),
        6,
        "the tab bar disappeared while About was shown"
    );

    assert!(click_label_exact(&mut test, "Close"), "no Close button");
    settle(&mut test);
    assert!(
        find_label_text(&test, "Identity").is_some(),
        "closing About did not return to the Character tab"
    );
    assert_eq!(
        selected_tab_labels(&test),
        vec!["Character".to_owned()],
        "the Character tab was not selected again after closing About"
    );
}

fn click_a11y_label(test: &mut TestingRunner, wanted: &str) -> bool {
    let node = test.find(|node, _| (a11y_label(&node).as_deref() == Some(wanted)).then_some(node));
    match node {
        Some(node) => {
            click_node(test, &node);
            true
        }
        None => false,
    }
}

/// Builds a copy of the fixture savegame in a folder that has no installation
/// beside it, so the game data is genuinely missing.
fn isolated_save_root(fixture: &Fixture) -> PathBuf {
    let root = fixture.root.join("isolated");
    let folder = root.join("Save").join("Test Save");
    std::fs::create_dir_all(&folder).expect("create isolated save");
    std::fs::copy(
        fixture.save_dir.join("Test Save").join("save.sac"),
        folder.join("save.sac"),
    )
    .expect("copy savegame");
    root.join("Save")
}

#[test]
fn scanning_the_same_folder_keeps_the_savegame_open() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut test = launch_sized(fixture.save_dir.clone(), 1000.0);
    assert!(
        find_label_text(&test, "Opened Test Save").is_some(),
        "the savegame did not open"
    );
    assert!(click_text(&mut test, "Scan"), "no scan button");
    wait_for_text(&mut test, "Found 1 savegame");
    // The same folder was refreshed: the open savegame must survive.
    assert!(click_label_exact(&mut test, "Character"));
    assert!(
        find_label_text(&test, "Identity").is_some(),
        "Scan closed the savegame it had just re-scanned"
    );
}

#[test]
fn browsing_checks_for_unsaved_changes_before_the_dialog() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut test = launch_sized(fixture.save_dir.clone(), 1000.0);
    assert!(click_text(&mut test, "Inventory"), "no inventory tab");
    assert!(click_label_exact(&mut test, "+"), "no count stepper");
    as_many_updates(&mut test);
    assert!(
        find_label_text(&test, "unsaved changes").is_some(),
        "the item edit did not mark the save dirty"
    );

    let other = fixture.root.join("other-save");
    let other_folder = other.join("Test Save");
    std::fs::create_dir_all(&other_folder).expect("create other save");
    std::fs::copy(
        fixture.save_dir.join("Test Save").join("save.sac"),
        other_folder.join("save.sac"),
    )
    .expect("copy savegame");
    for suffix in 2..=3 {
        let folder = other.join(format!("Test Save {suffix}"));
        std::fs::create_dir_all(&folder).expect("create other save");
        std::fs::copy(other_folder.join("save.sac"), folder.join("save.sac"))
            .expect("copy savegame");
    }
    jadeforge_gui::dialog::set_next_pick_for_tests(Some(other.clone()));
    assert!(click_text(&mut test, "Browse"), "no browse button");
    assert!(
        find_label_text(&test, "unsaved changes").is_some(),
        "the unsaved guard is missing"
    );
    // The dialog answer must not have been consumed: the folder was refused
    // before opening it, so the save list still belongs to the original folder.
    assert!(
        find_label_text(&test, "1 SAVEGAME").is_some(),
        "Browse changed the folder despite unsaved changes"
    );
}

fn as_many_updates(test: &mut TestingRunner) {
    for _ in 0..3 {
        test.sync_and_update();
    }
}

#[test]
fn edit_then_restore_clears_the_dirty_state() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut test = launch_sized(fixture.save_dir.clone(), 1400.0);
    assert!(click_text(&mut test, "Inventory"), "no inventory tab");
    assert!(click_label_exact(&mut test, "+"), "no count stepper");
    as_many_updates(&mut test);
    assert!(
        find_label_text(&test, "unsaved changes").is_some(),
        "the edit was not applied"
    );
    assert!(click_label_exact(&mut test, "−"), "no count stepper");
    as_many_updates(&mut test);
    if find_label_text(&test, "all changes saved").is_none() {
        eprintln!(
            "labels: {:?}",
            test.find_many(
                |_, element| Label::try_downcast(element).map(|label| label.text.to_string())
            )
            .into_iter()
            .filter(|text| text.contains("stack")
                || text.contains("saved")
                || text.contains("memory")
                || text.contains("changes"))
            .collect::<Vec<_>>()
        );
    }
    assert!(
        find_label_text(&test, "all changes saved").is_some(),
        "restoring the count did not clear the dirty state"
    );
}

#[test]
fn every_filter_has_a_clear_button() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let second = fixture.save_dir.join("Other Save");
    std::fs::create_dir_all(&second).expect("create second save");
    std::fs::copy(
        fixture.save_dir.join("Test Save").join("save.sac"),
        second.join("save.sac"),
    )
    .expect("copy savegame");

    let mut test = launch_sized(fixture.save_dir.clone(), 1000.0);
    assert!(find_label_text(&test, "2 SAVEGAMES").is_some());
    let filter = find_paragraph(&test, "Filter by name or character").expect("filter input");
    click_node(&mut test, &filter);
    test.write_text("Other");
    test.sync_and_update();
    assert!(find_label_text(&test, "1 OF 2 SAVEGAMES").is_some());
    assert!(
        click_a11y_label(&mut test, "Clear the savegame filter"),
        "the save filter has no clear button"
    );
    assert!(find_label_text(&test, "2 SAVEGAMES").is_some());

    assert!(click_text(&mut test, "Inventory"));
    let filter = find_paragraph(&test, "Filter the item database").expect("item filter");
    click_node(&mut test, &filter);
    test.write_text("Viper");
    test.sync_and_update();
    assert!(find_label_text(&test, "SEARCH RESULTS").is_some());
    assert!(
        click_a11y_label(&mut test, "Clear the item database filter"),
        "the item filter has no clear button"
    );
    assert!(find_label_text(&test, "ALL ITEMS").is_some());
}

#[test]
fn the_no_save_state_disables_editing() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut test = launch_unopened_at(fixture.save_dir.clone(), 1180.0, 760.0);
    assert!(find_label_text(&test, "No savegame selected").is_some());
    assert!(find_label_text(&test, "no savegame open").is_some());
    assert!(
        find_label_text(&test, "Choose save folder").is_some(),
        "the empty state has no primary folder action"
    );

    // A tab click must not activate the editor without a savegame.
    assert!(click_text(&mut test, "Character"));
    assert!(
        find_label_text(&test, "Identity").is_none(),
        "a tab opened the editor without a savegame"
    );
    // Save must not report a successful no-op either.
    assert!(click_text(&mut test, "Save to disk"));
    assert!(
        find_label_text(&test, "Nothing was changed").is_none(),
        "Save ran without a savegame"
    );

    // The primary action scans the picked folder.
    jadeforge_gui::dialog::set_next_pick_for_tests(Some(fixture.save_dir.clone()));
    assert!(click_text(&mut test, "Choose save folder"));
    wait_for_text(&mut test, "1 SAVEGAME");
    assert!(
        click_text(&mut test, "Level "),
        "the scanned save did not open"
    );
    assert!(click_label_exact(&mut test, "Character"));
    assert!(find_label_text(&test, "Identity").is_some());
}

#[test]
fn the_game_installation_can_be_chosen_from_the_ui() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    // The fixture normally installs the game root for the whole thread; find
    // the root it resolved before clearing the override.
    let probe = jadeforge_gui::Session::with_directory(fixture.save_dir.clone());
    let Some(root) = (match probe.game_status() {
        jadeforge_gui::GameStatus::Loaded { root, .. } => Some(root.clone()),
        _ => None,
    }) else {
        eprintln!("skipping: the fixture has no readable game data");
        return;
    };
    let save_dir = isolated_save_root(&fixture);
    jadeforge_gamedata::set_test_game_root(None);
    let mut test = launch_unopened_at(save_dir, 1180.0, 900.0);
    assert!(
        find_label_text(&test, "GAME DATA").is_some(),
        "the missing game data has no folder action"
    );
    let input =
        find_paragraph(&test, "Path to the Jade Empire installation").expect("game folder input");
    click_node(&mut test, &input);
    test.write_text(root.display().to_string());
    test.sync_and_update();
    assert!(click_text(&mut test, "Use"), "no use button");
    let status = find_label_text(&test, "Using the game data in").expect("game data status");
    assert!(status.contains(&root.display().to_string()), "{status}");
    assert!(
        find_label_text(&test, "items loaded").is_some(),
        "the chosen installation did not load"
    );
    jadeforge_gamedata::set_test_game_root(None);
}

#[test]
fn an_unreadable_section_is_not_reported_as_empty() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    {
        let path = fixture.save_dir.join("Test Save").join("save.sac");
        let mut save = jadeforge_core::SaveGame::open(&path).expect("open fixture save");
        let field = save
            .block_mut("save.sav")
            .expect("save block")
            .gff_mut()
            .get_mut("PlayerCreature.ItemList")
            .expect("item list");
        field.type_id = 0;
        field.value = jadeforge_core::Value::Byte(1);
        save.save(jadeforge_core::SaveOptions { backup: false })
            .expect("save fixture with an unreadable inventory");
    }
    let mut test = launch_sized(fixture.save_dir.clone(), 1400.0);
    // The status message mentions "Inventory" too, so use the exact tab label.
    assert!(click_label_exact(&mut test, "Inventory"));
    assert!(
        find_label_text(&test, "inventory section could not be read").is_some(),
        "an unreadable inventory looks like an empty one"
    );
    assert!(
        find_label_text(&test, "The inventory is empty").is_none(),
        "an unreadable inventory was reported as empty"
    );
}

#[test]
fn a_rejected_scan_restores_the_real_folder_path() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut test = launch_unopened_at(fixture.save_dir.clone(), 1180.0, 900.0);
    let real = fixture.save_dir.display().to_string();
    let input = test
        .find(|node, element| {
            let paragraph = Paragraph::try_downcast(element)?;
            paragraph.to_string().contains(&real).then_some(node)
        })
        .expect("the save folder input is missing");
    click_node(&mut test, &input);
    test.write_text("\u{8}".repeat(real.chars().count() + 4));
    test.write_text("/definitely/not/a/save/folder");
    test.sync_and_update();
    assert!(click_text(&mut test, "Scan"), "no scan button");
    assert!(
        find_label_text(&test, "does not exist").is_some(),
        "a missing folder was not reported"
    );
    assert!(
        test.find(|node, element| {
            let paragraph = Paragraph::try_downcast(element)?;
            paragraph.to_string().contains(&real).then_some(node)
        })
        .is_some(),
        "the rejected path stayed in the folder input"
    );
    assert!(find_label_text(&test, "1 SAVEGAME").is_some());
}

#[test]
fn save_focuses_the_first_invalid_field_and_still_saves() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut test = launch_sized(fixture.save_dir.clone(), 1400.0);
    assert!(click_text(&mut test, "Inventory"));
    assert!(click_label_exact(&mut test, "+"), "no count stepper");
    as_many_updates(&mut test);
    assert!(click_label_exact(&mut test, "Character"));

    // Put an invalid value into LEVEL, which is the first invalid field.
    let level = find_label_exact(&test, "LEVEL").expect("the level field is missing");
    let area = level.layout().area;
    let point = CursorPoint::new(
        f64::from(area.origin.x) + 10.0,
        f64::from(area.origin.y) + f64::from(area.size.height) + 20.0,
    );
    test.move_cursor(point);
    test.sync_and_update();
    test.click_cursor(point);
    test.sync_and_update();
    test.write_text("x");
    test.sync_and_update();
    assert!(
        find_label_text(&test, "invalid field").is_some(),
        "the invalid value did not reach the badge"
    );
    let level_id = test.sync_and_update().focus;
    assert!(click_label_exact(&mut test, "Character"));
    let moved = test.sync_and_update().focus;
    assert_ne!(
        moved, level_id,
        "focus did not move away from the level field"
    );

    assert!(click_text(&mut test, "Save to disk"), "no save button");
    let focused = test.sync_and_update().focus;
    assert_eq!(
        focused, level_id,
        "Save did not focus the first invalid field"
    );
    assert!(
        find_label_text(&test, "Saved").is_some(),
        "the valid item edit was not saved"
    );
    assert!(
        find_label_text(&test, "invalid field").is_some(),
        "the invalid field disappeared instead of staying for a fix"
    );
}

#[test]
fn save_and_revert_only_act_when_there_are_changes() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut test = launch_sized(fixture.save_dir.clone(), 1400.0);

    // Nothing has changed since the savegame was opened: both actions are
    // disabled and must not run.
    assert!(click_text(&mut test, "Save to disk"));
    test.sync_and_update();
    assert!(
        find_label_text(&test, "Nothing changed").is_none(),
        "Save ran with nothing to save"
    );
    assert!(click_text(&mut test, "Revert"));
    test.sync_and_update();
    assert!(
        find_label_text(&test, "Reloaded").is_none(),
        "Revert ran with nothing to discard"
    );

    // Make an edit, save, and keep the success confirmation on screen while
    // the actions become unavailable again.
    assert!(click_text(&mut test, "Character"));
    assert!(click_text(&mut test, "+1 point"), "no quick-add button");
    as_many_updates(&mut test);
    assert!(find_label_text(&test, "unsaved changes").is_some());
    assert!(click_text(&mut test, "Save to disk"));
    wait_for_text(&mut test, "Saved ");
    assert!(
        find_label_text(&test, "all changes saved").is_some(),
        "the saved state is not reported"
    );
    assert!(
        find_label_text(&test, "Saved ").is_some(),
        "the success confirmation did not stay visible"
    );
    assert!(click_text(&mut test, "Save to disk"));
    test.sync_and_update();
    assert!(
        find_label_text(&test, "Nothing changed").is_none(),
        "Save ran again after it had already saved"
    );
    assert!(
        find_label_text(&test, "Saved ").is_some(),
        "the success confirmation disappeared after the disabled Save click"
    );
}

#[test]
fn a_save_surfaces_the_backup_and_opens_the_save_folder() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut test = launch_sized(fixture.save_dir.clone(), 2000.0);
    assert!(click_text(&mut test, "Character"));
    assert!(click_text(&mut test, "+1 point"), "no quick-add button");
    as_many_updates(&mut test);
    assert!(click_text(&mut test, "Save to disk"));
    wait_for_text(&mut test, "Saved ");
    assert!(click_label_exact(&mut test, "Overview"));

    // The backup is not just mentioned; its exact path and actions appear on
    // the Savegame card after a successful save.
    assert!(
        find_label_text(&test, "Backup").is_some(),
        "the overview does not show the backup row"
    );
    assert!(
        find_text_node(&test, "jadeforge-backup").is_some(),
        "the backup path is not shown"
    );
    jadeforge_gui::clipboard::capture_next_copy_for_tests();
    assert!(
        click_a11y_label(&mut test, "Copy the backup path"),
        "the backup path has no copy action"
    );
    assert!(
        jadeforge_gui::clipboard::last_copied_for_tests()
            .as_deref()
            .is_some_and(|path| path.contains("jadeforge-backup")),
        "the backup action copied the wrong path"
    );
    assert!(
        find_label_text(&test, "Copied the backup path").is_some(),
        "copying the backup path did not report success"
    );

    jadeforge_gui::paths::capture_next_open_for_tests();
    assert!(
        click_a11y_label(&mut test, "Open the backup folder"),
        "the backup folder has no open action"
    );
    assert_eq!(
        jadeforge_gui::paths::last_opened_for_tests(),
        Some(fixture.save_dir.join("Test Save")),
        "the backup action opened the wrong folder"
    );
    assert!(
        find_label_text(&test, "Opened the backup folder").is_some(),
        "opening the backup folder did not report success"
    );

    jadeforge_gui::paths::capture_next_open_for_tests();
    assert!(
        click_a11y_label(&mut test, "Open the savegame folder"),
        "the savegame folder has no open action"
    );
    assert_eq!(
        jadeforge_gui::paths::last_opened_for_tests(),
        Some(fixture.save_dir.clone()),
        "the save folder action opened the wrong folder"
    );
}

fn probe_session(fixture: &Fixture) -> Option<jadeforge_gui::Session> {
    let mut session = jadeforge_gui::Session::with_directory(fixture.save_dir.clone());
    session.open(0).then_some(session)
}

fn click_ctrl(test: &mut TestingRunner, key: Key) {
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key,
        code: Code::Unidentified,
        modifiers: Modifiers::CONTROL,
    });
    test.sync_and_update();
}

#[test]
fn rows_show_a_details_chevron() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut test = launch_sized(fixture.save_dir.clone(), 1400.0);
    assert!(click_label_exact(&mut test, "Inventory"));
    assert!(
        find_label_exact(&test, "▸").is_some(),
        "a collapsed row has no disclosure chevron"
    );
    let subtitle = find_node_with_text(&test, " · #").expect("item subtitle");
    click_node(&mut test, &subtitle);
    assert!(
        find_label_exact(&test, "▾").is_some(),
        "the chevron did not change when the row expanded"
    );
    let row = test
        .find(|node, _| {
            a11y_label(&node)
                .as_deref()
                .is_some_and(|label| label.contains("Details"))
                .then_some(node)
        })
        .expect("the row action has no Details affordance");
    assert!(a11y_label(&row).is_some());
}

#[test]
fn the_gem_picker_shows_effect_lines_before_adding() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let Some(session) = probe_session(&fixture) else {
        eprintln!("skipping: the fixture save could not be opened");
        return;
    };
    let Some(gem) = session
        .available_gems()
        .into_iter()
        .find(|gem| !gem.effects.is_empty())
    else {
        eprintln!("skipping: no gem has effects");
        return;
    };
    let mut test = launch_sized(fixture.save_dir.clone(), 1800.0);
    assert!(click_text(&mut test, "Gems"), "no gems tab");
    let expected = format!("{} gem, {}", gem.name, gem.effects.join(" · "));
    assert!(
        test.find(
            |node, _| (a11y_label(&node).as_deref() == Some(expected.as_str())).then_some(node)
        )
        .is_some(),
        "the gem picker does not expose the effect lines before adding {expected}"
    );
    assert!(
        find_label_text(&test, &gem.effects[0]).is_some(),
        "the gem effect line is not rendered"
    );
}

#[test]
fn repeated_row_actions_have_contextual_names() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let Some(session) = probe_session(&fixture) else {
        eprintln!("skipping: the fixture save could not be opened");
        return;
    };
    let mut test = launch_sized(fixture.save_dir.clone(), 2000.0);

    let has_name = |test: &TestingRunner, wanted: &str| {
        test.find(|node, _| (a11y_label(&node).as_deref() == Some(wanted)).then_some(node))
            .is_some()
    };

    if let Some(item) = session.items().first().cloned() {
        assert!(click_label_exact(&mut test, "Inventory"));
        assert!(
            has_name(&test, &format!("Remove {}", item.name)),
            "the item remove action has no contextual name"
        );
        assert!(
            has_name(&test, &format!("Increase {} stack", item.name)),
            "the count stepper has no contextual name"
        );
        assert!(
            has_name(&test, &format!("Decrease {} stack", item.name)),
            "the count stepper has no contextual name"
        );
    }

    assert!(click_label_exact(&mut test, "Styles"));
    if let Some(style) = session.styles().first() {
        assert!(
            has_name(&test, &format!("Remove {} style", style.name)),
            "the style remove action has no contextual name"
        );
    }
    if let Some(learnable) = session.missing_styles().first() {
        assert!(
            has_name(&test, &format!("Learn {} style", learnable.name)),
            "the learn action has no contextual name"
        );
    }

    assert!(click_label_exact(&mut test, "Companions"));
    if let Some(companion) = session.henchmen().first() {
        assert!(
            has_name(&test, &format!("Increase {} level", companion.name)),
            "the companion level action has no contextual name"
        );
    }
}

#[test]
fn a_stack_size_can_be_typed_directly() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let Some(session) = probe_session(&fixture) else {
        eprintln!("skipping: the fixture save could not be opened");
        return;
    };
    let Some(item) = session.items().first().cloned() else {
        eprintln!("skipping: the fixture has no items");
        return;
    };
    let wanted = if item.count < 255 {
        item.count + 1
    } else {
        item.count - 1
    };
    let mut test = launch_sized(fixture.save_dir.clone(), 1800.0);
    assert!(click_label_exact(&mut test, "Inventory"));
    let paragraph = test.find(|node, _| {
        (a11y_role(&node) == "TextInput"
            && a11y_value(&node).as_deref() == Some(&item.count.to_string()))
        .then_some(node)
    });
    let Some(paragraph) = paragraph else {
        eprintln!("skipping: could not locate the count input");
        return;
    };
    click_node(&mut test, &paragraph);
    test.write_text("\u{8}".repeat(3));
    test.write_text(wanted.to_string());
    test.press_key(Key::Named(NamedKey::Enter));
    test.sync_and_update();
    assert!(
        find_label_text(&test, "in the stack").is_some(),
        "typing a stack size did not report a change"
    );
}

#[test]
fn a_companion_level_can_be_typed_directly() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let Some(session) = probe_session(&fixture) else {
        eprintln!("skipping: the fixture save could not be opened");
        return;
    };
    let Some(companion) = session.henchmen().first().cloned() else {
        eprintln!("skipping: the fixture has no companions");
        return;
    };
    let mut test = launch_sized(fixture.save_dir.clone(), 1600.0);
    assert!(click_label_exact(&mut test, "Companions"));
    let paragraph = test.find(|node, _| {
        (a11y_role(&node) == "TextInput"
            && a11y_value(&node).as_deref() == Some(&companion.level.to_string()))
        .then_some(node)
    });
    let Some(paragraph) = paragraph else {
        eprintln!("skipping: the level input is not rendered");
        return;
    };
    click_node(&mut test, &paragraph);
    test.write_text("\u{8}".repeat(2));
    test.write_text("42");
    test.press_key(Key::Named(NamedKey::Enter));
    test.sync_and_update();
    assert!(
        find_label_text(&test, "Companion level set to 42").is_some(),
        "typing a companion level did not apply"
    );
}

#[test]
fn desktop_shortcuts_drive_the_shell() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let session = jadeforge_gui::Session::with_directory(fixture.save_dir.clone());
    let total = session.entries().len();
    if total == 0 {
        eprintln!("skipping: no savegames");
        return;
    }
    let mut test = launch_sized(fixture.save_dir.clone(), 1200.0);
    let savegames = if total == 1 { "SAVEGAME" } else { "SAVEGAMES" };

    // Ctrl+F focuses the save filter of the active tab (Overview on start) and
    // Escape clears it again. The save filter exists with and without game
    // data, unlike a picker filter.
    click_ctrl(&mut test, Key::Character("f".to_owned()));
    // Focus requests are applied on the next accessibility pass; give the
    // runner that frame before typing.
    test.sync_and_update();
    test.write_text("zzzz-no-such-save");
    test.sync_and_update();
    assert!(
        find_label_text(&test, &format!("0 OF {total} {savegames}")).is_some(),
        "Ctrl+F did not focus the save filter"
    );
    test.press_key(Key::Named(NamedKey::Escape));
    assert!(
        find_label_text(&test, &format!("{total} {savegames}")).is_some(),
        "Escape did not clear the save filter"
    );

    // F5 rescans the current folder and keeps the savegame open.
    assert!(click_text(&mut test, "Scan"));
    test.press_key(Key::Named(NamedKey::F5));
    wait_for_text(&mut test, "Found ");
    assert!(
        find_label_text(&test, "Found ").is_some(),
        "F5 did not scan"
    );

    // Arrow keys move between tabs while a tab has focus.
    assert!(click_label_exact(&mut test, "Character"));
    test.press_key(Key::Named(NamedKey::ArrowRight));
    assert!(
        find_label_text(&test, "Fighting styles").is_some(),
        "ArrowRight did not move to the next tab"
    );

    // Ctrl+S saves the applied edits.
    assert!(click_label_exact(&mut test, "Inventory"));
    assert!(click_label_exact(&mut test, "+"), "no count stepper");
    as_many_updates(&mut test);
    click_ctrl(&mut test, Key::Character("s".to_owned()));
    assert!(
        find_label_text(&test, "Saved").is_some(),
        "Ctrl+S did not save"
    );
}

#[test]
fn save_rows_show_the_character_and_modification_time() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let session = jadeforge_gui::Session::with_directory(fixture.save_dir.clone());
    let Some(entry) = session
        .entries()
        .first()
        .filter(|entry| entry.character.is_some() && entry.modified.is_some())
        .cloned()
    else {
        eprintln!("skipping: the fixture save has no readable metadata");
        return;
    };
    let character = entry.character.expect("character name");
    let test = launch_unopened_at(fixture.save_dir.clone(), 1180.0, 900.0);
    let row = find_node_with_text(&test, &character).expect("the character name is not in the row");
    let value = a11y_label(&row)
        .or_else(|| a11y_value(&row))
        .unwrap_or_default();
    assert!(
        value.contains("Time played"),
        "the save row does not use the same Time played wording: {value}"
    );
    assert!(
        value.contains("UTC"),
        "the save row does not show the modification time: {value}"
    );
    assert!(value.contains(&character), "{value}");
}

/// Finds the style row that carries the given style name.
fn find_style_row(test: &TestingRunner, name: &str) -> Option<freya_testing::TestingNode> {
    let wanted = format!("{name} style");
    test.find(|node, _| {
        (a11y_role(&node) == "Group" && a11y_label(&node).as_deref() == Some(&wanted))
            .then_some(node)
    })
}

/// Solid background colour of the node, if it paints one.
fn background_of(node: &freya_testing::TestingNode) -> Option<Color> {
    let element = node.element();
    element.style().background.as_color()
}

fn orb_detail_is_visible(test: &TestingRunner) -> bool {
    find_text_node(test, "available: clicking buys").is_some()
        || find_text_node(test, "owned: clicking removes").is_some()
}

/// Expands the first learned style and returns its name.
fn expand_first_style(test: &mut TestingRunner) -> String {
    let (_row, label) = test
        .find(|node, _| {
            if a11y_role(&node) != "Group" {
                return None;
            }
            let label = a11y_label(&node)?;
            label.ends_with(" style").then_some((node, label))
        })
        .expect("style row");
    let name = label
        .strip_suffix(" style")
        .expect("the row label ends in ` style`")
        .to_owned();
    let toggle = test
        .find(|node, _| {
            (a11y_role(&node) == "Button"
                && a11y_label(&node).is_some_and(|label| {
                    label.starts_with(&format!("{name}, ")) && label.ends_with("Details")
                }))
            .then_some(node)
        })
        .expect("style toggle");
    click_node(test, &toggle);
    assert!(
        find_label_text(test, "UPGRADES").is_some(),
        "the style did not expand"
    );
    name
}

/// Layout bounds `(x, y, width, height)` of two adjacent orbs in one family.
fn adjacent_orb_areas(test: &TestingRunner, name: &str) -> Option<[(f32, f32, f32, f32); 2]> {
    let prefix = format!("{name}, ");
    let mut orbs: Vec<freya_testing::TestingNode> = test.find_many(|node, _| {
        (a11y_role(&node) == "Button"
            && a11y_label(&node)
                .is_some_and(|label| label.starts_with(&prefix) && label.contains(", tier ")))
        .then_some(node)
    });
    orbs.sort_by(|left, right| {
        let left = left.layout().area;
        let right = right.layout().area;
        left.origin
            .y
            .total_cmp(&right.origin.y)
            .then(left.origin.x.total_cmp(&right.origin.x))
    });
    orbs.windows(2).find_map(|pair| {
        let (first, second) = (&pair[0], &pair[1]);
        let (a, b) = (first.layout().area, second.layout().area);
        (a.origin.y.total_cmp(&b.origin.y).is_eq() && a.origin.x < b.origin.x).then_some([
            (a.origin.x, a.origin.y, a.size.width, a.size.height),
            (b.origin.x, b.origin.y, b.size.width, b.size.height),
        ])
    })
}

fn orb_center(area: (f32, f32, f32, f32)) -> CursorPoint {
    CursorPoint::new(
        f64::from(area.0) + f64::from(area.2) / 2.0,
        f64::from(area.1) + f64::from(area.3) / 2.0,
    )
}

#[test]
fn sweeping_across_upgrade_orbs_keeps_the_style_hovered() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    if !fixture.game_data {
        eprintln!("skipping: no game data to resolve upgrades");
        return;
    }

    let mut test = launch_sized(fixture.save_dir.clone(), 1600.0);
    assert!(click_label_exact(&mut test, "Styles"));
    let name = expand_first_style(&mut test);
    let Some([first, second]) = adjacent_orb_areas(&test, &name) else {
        eprintln!("skipping: the expanded style has no two orbs on one row");
        return;
    };
    let row_background =
        |test: &TestingRunner| background_of(&find_style_row(test, &name).expect("style row"));

    // The row stays highlighted while one of its own orbs is hovered. Losing
    // this turned every step from one orb to the next into a blink.
    test.move_cursor(orb_center(first));
    test.sync_and_update();
    assert_eq!(
        row_background(&test),
        Some(jadeforge_gui::theme::surface_hover()),
        "hovering an orb dropped the style row highlight"
    );
    assert!(
        orb_detail_is_visible(&test),
        "the hovered orb is not explained"
    );

    // The gaps between the orbs belong to the orb hit target, so the hovered
    // orb (and its detail line) survives the crossing instead of falling back
    // to the row background for a moment.
    let gap_y = f64::from(first.1) + f64::from(first.3) / 2.0;
    for x in [
        f64::from(first.0 + first.2) + 1.0,
        f64::from(second.0) - 1.0,
    ] {
        test.move_cursor(CursorPoint::new(x, gap_y));
        test.sync_and_update();
        assert_eq!(
            row_background(&test),
            Some(jadeforge_gui::theme::surface_hover()),
            "the style row lost its highlight in the gap between orbs"
        );
        assert!(
            orb_detail_is_visible(&test),
            "crossing the gap dropped the hovered orb"
        );
    }

    test.move_cursor(orb_center(second));
    test.sync_and_update();
    assert_eq!(
        row_background(&test),
        Some(jadeforge_gui::theme::surface_hover()),
        "the second orb did not keep the style row highlighted"
    );
    assert!(
        orb_detail_is_visible(&test),
        "the second orb is not explained"
    );
}

#[test]
fn hovering_an_upgrade_orb_explains_it() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    if !fixture.game_data {
        eprintln!("skipping: no game data to resolve upgrades");
        return;
    }
    let mut test = launch_sized(fixture.save_dir.clone(), 1400.0);
    assert!(click_label_exact(&mut test, "Styles"));
    let subtitle = find_node_with_text(&test, " · #").expect("style subtitle");
    click_node(&mut test, &subtitle);
    assert!(find_label_text(&test, "UPGRADES").is_some());
    let orb = test
        .find(|node, _| {
            (a11y_role(&node) == "Button"
                && a11y_label(&node)
                    .as_deref()
                    .is_some_and(|label| label.contains("tier 1")))
            .then_some(node)
        })
        .expect("no upgrade orb");
    let area = orb.layout().area;
    test.move_cursor(CursorPoint::new(
        f64::from(area.origin.x) + f64::from(area.size.width) / 2.0,
        f64::from(area.origin.y) + f64::from(area.size.height) / 2.0,
    ));
    test.sync_and_update();
    assert!(
        find_text_node(&test, "available: clicking buys").is_some()
            || find_text_node(&test, "owned: clicking removes").is_some(),
        "hovering an orb did not explain it"
    );
    let label = a11y_label(&orb).unwrap_or_default();
    assert!(
        label.contains("tier 1") && (label.contains("buys") || label.contains("removes")),
        "the orb's accessible name lacks its action: {label}"
    );
}

#[test]
fn controls_meet_the_minimum_target_size() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut test = launch_sized(fixture.save_dir.clone(), 1400.0);

    for label in ["Name", "Level", "Time played"] {
        let node = test
            .find(|node, _| {
                a11y_label(&node)
                    .as_deref()
                    .is_some_and(|wanted| wanted.starts_with(&format!("Sort by {label}")))
                    .then_some(node)
            })
            .expect("the sort control is missing");
        assert!(
            node.layout().area.size.height >= 23.5,
            "the {label} sort control is only {} px tall",
            node.layout().area.size.height
        );
    }

    let tab = test
        .find(|node, _| {
            (a11y_role(&node) == "Tab" && a11y_label(&node).as_deref() == Some("Character"))
                .then_some(node)
        })
        .expect("the Character tab is missing");
    assert!(
        tab.layout().area.size.height >= 23.5,
        "the tab target is only {} px tall",
        tab.layout().area.size.height
    );

    assert!(click_label_exact(&mut test, "Inventory"));
    let plus = test
        .find(|node, _| {
            a11y_label(&node)
                .as_deref()
                .is_some_and(|label| label.starts_with("Increase ") && label.ends_with(" stack"))
                .then_some(node)
        })
        .expect("the count stepper is missing");
    let area = plus.layout().area;
    assert!(
        area.size.width >= 23.5 && area.size.height >= 23.5,
        "the stepper target is {}x{} px",
        area.size.width,
        area.size.height
    );
}

#[test]
fn removing_a_row_restores_focus_to_the_filter() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut test = launch_sized(fixture.save_dir.clone(), 1800.0);
    assert!(click_label_exact(&mut test, "Inventory"));
    // The save filter is the first empty text input in the tree; the active
    // tab's filter comes later.
    let filters = test.find_many(|node, _| {
        (a11y_role(&node) == "TextInput" && a11y_value(&node).is_none_or(|value| value.is_empty()))
            .then_some(node)
    });
    let filter = filters.last().expect("the inventory filter is missing");
    let filter_id = filter
        .element()
        .accessibility()
        .a11y_id
        .expect("the filter has no accessibility id");

    jadeforge_gui::dialog::set_next_confirm_for_tests(true);
    assert!(click_label_exact(&mut test, "Remove"), "no remove action");
    let focused = test.sync_and_update().focus;
    assert_eq!(
        focused, filter_id,
        "removing the focused row left focus nowhere useful"
    );
}

#[test]
fn the_editor_scroll_view_is_keyboard_focusable() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut test = launch_sized(fixture.save_dir.clone(), 900.0);
    assert!(click_label_exact(&mut test, "Styles"));
    let scroll_view = test
        .find(|node, _| (a11y_role(&node) == "ScrollView" && a11y_focusable(&node)).then_some(node))
        .expect("the editor scroll view is missing");
    let target = scroll_view
        .element()
        .accessibility()
        .a11y_id
        .expect("the scroll view has no accessibility id");
    assert!(
        a11y_focusable(&scroll_view),
        "the scroll view is not focusable"
    );
    let request = accesskit::ActionRequest {
        action: accesskit::Action::Focus,
        target_tree: accesskit::TreeId::ROOT,
        target_node: target,
        data: None,
    };
    assert!(
        test.send_accessibility_action(request),
        "the scroll view cannot be focused through assistive technology"
    );
    let focused = test.sync_and_update().focus;
    assert_eq!(focused, target, "the Focus action did not move focus");
    // The paging keys are wired to the active tab's scroll controller. The
    // headless runner does not report the ScrollView bounds, so the resulting
    // offset cannot be asserted here; the pointer-wheel test has the same
    // limitation and skips.
    test.press_key(Key::Named(NamedKey::PageDown));
    test.sync_and_update();
}

#[test]
fn eighty_nine_saves_render_within_the_frame_budget() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    // The retail corpus has 89 savegames; the fixture copied once per save so
    // the save list, its scroll view and the sidebar are exercised at that size.
    for suffix in 2..=89 {
        let folder = fixture.save_dir.join(format!("Test Save {suffix}"));
        std::fs::create_dir_all(&folder).expect("create extra save folder");
        std::fs::copy(
            fixture.save_dir.join("Test Save").join("save.sac"),
            folder.join("save.sac"),
        )
        .expect("copy savegame");
    }
    let mut test = launch_unopened_at(fixture.save_dir.clone(), 1180.0, 760.0);
    for _ in 0..2000 {
        if find_label_text(&test, "89 SAVEGAMES").is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
        test.sync_and_update();
    }
    assert!(find_label_text(&test, "89 SAVEGAMES").is_some());

    let started = std::time::Instant::now();
    let _ = test.render();
    let first = started.elapsed();
    assert!(
        first < std::time::Duration::from_secs(2),
        "the first 89-save frame took {first:?}"
    );

    // Opening a savegame must not become quadratic with the list size.
    assert!(click_text(&mut test, "Level "), "the savegame did not open");
    let started = std::time::Instant::now();
    let _ = test.render();
    let opened = started.elapsed();
    assert!(
        opened < std::time::Duration::from_secs(2),
        "the opened 89-save frame took {opened:?}"
    );
    assert!(
        crate::perf_frames() > 0,
        "frame instrumentation did not record a render"
    );
}

fn perf_frames() -> u64 {
    jadeforge_gui::perf::frames()
}

#[test]
fn the_first_frame_shows_the_scan_in_progress() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let _ = jadeforge_gui::config::ensure_test_path();
    // Slow the startup worker down so the first frame is observable.
    jadeforge_gui::session::set_scan_delay_for_tests(std::time::Duration::from_millis(200));
    let save_dir = fixture.save_dir.clone();
    let (mut test, ()) = TestingRunner::new(
        move || app_with_directory(save_dir.clone()),
        (1180.0_f32, 760.0_f32).into(),
        |_| {},
        1.0,
    );
    test.sync_and_update();
    assert!(
        find_label_text(&test, "Scanning ").is_some(),
        "the first frame does not report the scan in progress"
    );
    assert!(
        find_label_text(&test, "Loading savegames…").is_some(),
        "the editor has no loading skeleton during the first scan"
    );
    assert!(
        find_label_text(&test, "LOADING SAVEGAMES…").is_some(),
        "the sidebar has no loading state during the first scan"
    );
    assert!(
        find_label_text(&test, "SCANNING…").is_some(),
        "the tab bar has no visible busy state during the first scan"
    );
    assert!(
        find_label_text(&test, "1 SAVEGAME").is_none(),
        "the first frame already waited for the scan"
    );

    // Poll until the worker applies the result.
    for _ in 0..500 {
        if find_label_text(&test, "1 SAVEGAME").is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
        test.sync_and_update();
    }
    assert!(
        find_label_text(&test, "1 SAVEGAME").is_some(),
        "the startup scan never settled"
    );
    assert!(
        find_label_text(&test, "Scanning ").is_none(),
        "the in-flight state stayed after the scan settled"
    );
    assert!(
        find_label_text(&test, "SCANNING…").is_none(),
        "the tab-bar busy state stayed after the scan settled"
    );
    jadeforge_gui::session::set_scan_delay_for_tests(std::time::Duration::ZERO);
}

#[test]
fn a_scan_can_be_cancelled_without_losing_the_save() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut test = launch_sized(fixture.save_dir.clone(), 1000.0);
    jadeforge_gui::session::set_scan_delay_for_tests(std::time::Duration::from_millis(250));
    assert!(click_text(&mut test, "Scan"), "no scan button");
    assert!(
        find_label_text(&test, "Scanning ").is_some(),
        "the Scan action did not enter the in-flight state"
    );
    assert!(
        find_label_text(&test, "SCANNING…").is_some(),
        "a manual scan has no visible busy state in the tab bar"
    );
    assert!(
        find_label_text(&test, "Cancel scan").is_some(),
        "there is no way to cancel the scan"
    );

    assert!(
        click_text(&mut test, "Cancel scan"),
        "the cancel control is not clickable"
    );
    assert!(
        find_label_text(&test, "Scan cancelled").is_some(),
        "the cancel is not reported"
    );
    assert!(
        find_label_text(&test, "Cancel scan").is_none(),
        "the cancel control stayed after cancelling"
    );
    assert!(
        find_label_text(&test, "1 SAVEGAME").is_some(),
        "cancelling cleared the save list"
    );
    // The open savegame is still open.
    assert!(click_label_exact(&mut test, "Character"));
    assert!(find_label_text(&test, "Identity").is_some());

    // The worker result that arrives after the delay must not be applied.
    std::thread::sleep(std::time::Duration::from_millis(300));
    test.sync_and_update();
    test.sync_and_update();
    assert!(
        find_label_text(&test, "1 SAVEGAME").is_some(),
        "a cancelled scan applied its result anyway"
    );
    assert!(
        find_label_text(&test, "Scanning ").is_none(),
        "a cancelled scan left the in-flight state behind"
    );
    assert!(
        find_label_text(&test, "SCANNING…").is_none(),
        "a cancelled scan left the tab-bar busy state behind"
    );
    jadeforge_gui::session::set_scan_delay_for_tests(std::time::Duration::ZERO);
}

#[test]
fn a_scan_settles_after_the_in_flight_state() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut test = launch_sized(fixture.save_dir.clone(), 1000.0);
    jadeforge_gui::session::set_scan_delay_for_tests(std::time::Duration::from_millis(200));
    assert!(click_label_exact(&mut test, "Scan"), "no scan button");
    assert!(
        find_label_text(&test, "Scanning ").is_some(),
        "the Scan action did not report progress"
    );
    wait_for_text(&mut test, "Found 1 savegame");
    assert!(
        find_label_text(&test, "Cancel scan").is_none(),
        "the in-flight state did not settle"
    );
    assert!(
        find_label_text(&test, "Scanning ").is_none(),
        "the progress message did not settle"
    );
    jadeforge_gui::session::set_scan_delay_for_tests(std::time::Duration::ZERO);
}

#[test]
fn scanning_another_folder_resets_the_filter_after_the_scan() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let second_root = fixture.root.join("second-save-root");
    let second_folder = second_root.join("Test Save");
    std::fs::create_dir_all(&second_folder).expect("create second save folder");
    std::fs::copy(
        fixture.save_dir.join("Test Save").join("save.sac"),
        second_folder.join("save.sac"),
    )
    .expect("copy savegame");

    let mut test = launch_sized(fixture.save_dir.clone(), 1200.0);
    let filter = find_paragraph(&test, "Filter by name or character").expect("save filter");
    click_node(&mut test, &filter);
    test.write_text("does-not-match");
    test.sync_and_update();
    assert!(
        find_label_text(&test, "0 OF 1 SAVEGAME").is_some(),
        "the filter did not apply"
    );

    jadeforge_gui::dialog::set_next_pick_for_tests(Some(second_root));
    assert!(click_text(&mut test, "Browse"), "no browse button");
    // The new folder scan is applied and the document reset clears the filter.
    wait_for_text(&mut test, "Found 1 savegame");
    settle(&mut test);
    assert!(
        find_label_text(&test, "0 OF").is_none(),
        "the filter survived a folder scan"
    );
    assert!(
        find_paragraph(&test, "does-not-match").is_none(),
        "the filter input was not cleared"
    );
}

#[test]
fn upgrade_detail_keeps_the_full_width() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    if !fixture.game_data {
        eprintln!("skipping: no game data to resolve upgrades");
        return;
    }
    let mut test = launch_unopened_at(fixture.save_dir.clone(), 1460.0, 900.0);
    assert!(click_text(&mut test, "Level "));
    assert!(click_label_exact(&mut test, "Styles"));
    let name = expand_first_style(&mut test);
    let prefix = format!("{name}, ");
    let orbs: Vec<_> = test.find_many(|node, _| {
        (a11y_role(&node) == "Button"
            && a11y_label(&node)
                .is_some_and(|label| label.starts_with(&prefix) && label.contains(", tier ")))
        .then_some(node)
    });
    assert!(!orbs.is_empty(), "the expanded style has no upgrade orbs");

    for orb in orbs {
        let label = a11y_label(&orb).expect("orb label");
        let tier = label
            .split(", tier ")
            .nth(1)
            .and_then(|rest| rest.split(',').next())
            .expect("orb tier");
        let area = orb.layout().area;
        test.move_cursor(CursorPoint::new(
            f64::from(area.origin.x) + f64::from(area.size.width) / 2.0,
            f64::from(area.origin.y) + f64::from(area.size.height) / 2.0,
        ));
        test.sync_and_update();
        let title = find_text_node(&test, &format!("tier {tier}:"))
            .unwrap_or_else(|| panic!("no detail title for tier {tier}"));
        let title_area = title.layout().area;
        assert!(
            title_area.size.width > 300.0,
            "the detail title for tier {tier} is squeezed into {} px at x {}",
            title_area.size.width,
            title_area.origin.x
        );
        let action = find_text_node(&test, "clicking buys")
            .or_else(|| find_text_node(&test, "clicking removes"))
            .unwrap_or_else(|| panic!("no detail action for tier {tier}"));
        let action_area = action.layout().area;
        assert!(
            action_area.size.width > 300.0,
            "the detail action for tier {tier} is squeezed into {} px at x {}",
            action_area.size.width,
            action_area.origin.x
        );
    }
}

/// Writes UI preferences into the isolated test settings before a launch.
fn write_ui_preferences(theme: ThemeChoice, text_scale: u16) {
    let _ = jadeforge_gui::config::ensure_test_path();
    let mut config = jadeforge_gui::config::Config::load();
    config.theme = theme;
    config.text_scale = text_scale;
    config.store().expect("store UI preferences");
}

#[test]
fn the_light_theme_renders_with_the_light_palette() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    write_ui_preferences(ThemeChoice::Light, 100);
    let mut test = launch_sized(fixture.save_dir.clone(), 900.0);
    assert_eq!(theme::current().palette.kind, theme::PaletteKind::Light);
    assert!(
        theme::background().r() > 200,
        "the light background is not light"
    );
    // The shell and the editor still render with the same structure.
    assert!(find_label_text(&test, "SAVEGAME").is_some());
    assert!(click_text(&mut test, "Styles"), "Styles did not render");
    assert!(find_label_text(&test, "Fighting styles").is_some());
}

#[test]
fn the_high_contrast_theme_renders_black_on_white() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    write_ui_preferences(ThemeChoice::HighContrast, 100);
    let test = launch_sized(fixture.save_dir.clone(), 900.0);
    assert_eq!(
        theme::current().palette.kind,
        theme::PaletteKind::HighContrast
    );
    assert_eq!(theme::background(), Color::from_rgb(0, 0, 0));
    assert_eq!(theme::text(), Color::WHITE);
    assert!(find_label_text(&test, "SAVEGAME").is_some());
}

#[test]
fn the_system_theme_follows_the_platform_preference() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    // The headless runner reports a light platform preference.
    write_ui_preferences(ThemeChoice::System, 100);
    let test = launch_sized(fixture.save_dir.clone(), 900.0);
    assert_eq!(theme::current().palette.kind, theme::PaletteKind::Light);
    drop(test);
}

#[test]
fn the_about_page_changes_theme_and_text_scale_and_persists_them() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut test = launch_sized(fixture.save_dir.clone(), 1800.0);
    assert_eq!(theme::current().palette.kind, theme::PaletteKind::Dark);
    assert!(click_text(&mut test, "About"), "About is missing");

    assert!(click_text(&mut test, "Light"), "Light chip is missing");
    assert_eq!(theme::current().palette.kind, theme::PaletteKind::Light);
    let config = jadeforge_gui::config::Config::load();
    assert_eq!(config.theme, ThemeChoice::Light);

    assert!(
        click_text(&mut test, "High contrast"),
        "the HC chip is missing"
    );
    assert_eq!(
        theme::current().palette.kind,
        theme::PaletteKind::HighContrast
    );

    assert!(click_text(&mut test, "200 %"), "the 200 % chip is missing");
    assert!((theme::current().text_scale - 2.0).abs() < f32::EPSILON);
    let config = jadeforge_gui::config::Config::load();
    assert_eq!(config.theme, ThemeChoice::HighContrast);
    assert_eq!(config.normalized_text_scale(), 200);
}

#[test]
fn every_tab_renders_at_200_percent_text_without_a_layout_panic() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    write_ui_preferences(ThemeChoice::Dark, 200);
    let mut test = launch_sized(fixture.save_dir.clone(), 900.0);
    assert!((theme::current().text_scale - 2.0).abs() < f32::EPSILON);
    // The scaled save row still shows its level line as one row.
    let level = find_node_with_text(&test, "Level ").expect("the save row is missing");
    assert!(
        level.layout().area.size.height >= 20.0,
        "the scaled save row text is only {} px tall",
        level.layout().area.size.height
    );
    for tab in [
        "Overview",
        "Character",
        "Styles",
        "Inventory",
        "Gems",
        "Companions",
    ] {
        assert!(click_text(&mut test, tab), "{tab} did not render at 200 %");
        settle(&mut test);
    }
    // The status slot and the footer are still inside the window.
    let footer = find_label_exact(&test, "Save to disk").expect("the footer is missing");
    assert!(footer.layout().area.origin.y > 0.0);
}
