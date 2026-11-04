//! Headless UI tests.
//!
//! The tests render the real component tree with `freya-testing`, which is the
//! same code path the windowed application uses minus the window. A miniature
//! game installation is assembled in a temporary folder (talk table and the two
//! 2DA tables) so item and style names are resolvable. When neither the game
//! nor savegames are available the tests skip instead of failing.

use std::path::{Path, PathBuf};

use freya::prelude::*;
use freya_testing::TestingRunner;
use jadeforge_gui::app_with_directory;

fn game_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("JADEFORGE_SAVE_DIR")
        .map(PathBuf::from)
        .and_then(|dir| dir.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("/media/Games/Heroic/Jade Empire"));
    dir.join("dialog.tlk").is_file().then_some(dir)
}

fn save_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("JADEFORGE_SAVE_DIR").map_or_else(
        || PathBuf::from("/media/Games/Heroic/Jade Empire/Save"),
        PathBuf::from,
    );
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
}

impl Fixture {
    fn create() -> Option<Self> {
        let save = smallest_save(&save_dir()?)?;
        let root = std::env::temp_dir().join(format!(
            "jadeforge-ui-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |duration| duration.as_nanos())
        ));
        let game = root.join("Game");
        let save_dir = game.join("Save");
        let folder = save_dir.join("Test Save");
        std::fs::create_dir_all(&folder).expect("create fixture");

        let mut game_data = false;
        if let Some(source) = game_dir() {
            let override_dir = game.join("override");
            std::fs::create_dir_all(&override_dir).expect("create override");
            let mut copied = 0;
            for (from, to) in [
                (source.join("dialog.tlk"), game.join("dialog.tlk")),
                (
                    source.join("override").join("items.2da"),
                    override_dir.join("items.2da"),
                ),
                (
                    source.join("override").join("stylesuperlist.2da"),
                    override_dir.join("stylesuperlist.2da"),
                ),
            ] {
                if from.is_file() && std::fs::copy(&from, &to).is_ok() {
                    copied += 1;
                }
            }
            game_data = copied == 3;
        }

        std::fs::copy(&save, folder.join("save.sac")).expect("copy savegame");
        Some(Self {
            root,
            save_dir,
            game_data,
        })
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn find_label_text(test: &TestingRunner, needle: &str) -> Option<String> {
    test.find(|_, element| {
        let label = Label::try_downcast(element)?;
        label.text.contains(needle).then(|| label.text.to_string())
    })
}

fn find_node_with_text(test: &TestingRunner, needle: &str) -> Option<freya_testing::TestingNode> {
    test.find(|node, element| {
        let label = Label::try_downcast(element)?;
        label.text.contains(needle).then_some(node)
    })
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

fn click_text(test: &mut TestingRunner, needle: &str) -> bool {
    match find_node_with_text(test, needle) {
        Some(node) => {
            click_node(test, &node);
            true
        }
        None => false,
    }
}

fn launch_sized(save_dir: PathBuf, height: f64) -> TestingRunner {
    let (mut test, ()) = TestingRunner::new(
        move || app_with_directory(save_dir.clone()),
        (1280.0_f32, height as f32).into(),
        |_| {},
        1.0,
    );
    test.sync_and_update();
    // Open the single savegame of the fixture.
    click_text(&mut test, "Level ");
    test
}

fn launch(save_dir: PathBuf) -> TestingRunner {
    launch_sized(save_dir, 800.0)
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
    assert!(find_label_text(&test, "Apply changes").is_some());
    assert!(find_label_text(&test, "Save to disk").is_some());
    let image = test.render();
    assert!(
        image.len() > 1024,
        "the rendered frame is suspiciously small"
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
    let mut test = launch_sized(fixture.save_dir.clone(), 1800.0);

    assert!(click_text(&mut test, "Styles"));
    // Without a query the whole learnable list is shown, not nothing.
    assert!(
        find_label_text(&test, "ALL STYLES").is_some(),
        "the style list is not browsable without a search"
    );
    // Owned and learnable rows share the `label · #id` subtitle.
    assert!(
        find_label_text(&test, " · #").is_some(),
        "styles do not show the shared label and id subtitle"
    );
    assert!(find_label_text(&test, "Lv ").is_some());

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
    let before = find_label_text(&test, " of ").expect("browse counter");
    assert!(click_text(&mut test, "Show more"), "no show more button");
    let after = find_label_text(&test, " of ").expect("browse counter after paging");
    assert_ne!(before, after, "paging did not change the counter");
    assert!(after.contains("of"), "{after}");
}

#[test]
fn style_levels_can_be_raised() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut test = launch(fixture.save_dir.clone());
    assert!(click_text(&mut test, "Styles"));
    if find_label_text(&test, "Lv ").is_none() {
        eprintln!("skipping: the test savegame has no styles");
        return;
    }
    assert!(click_text(&mut test, "+"), "no plus button");
    let status = find_label_text(&test, "Style level").expect("status message");
    assert!(status.contains("Style level"), "{status}");
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
fn staged_edits_block_scanning() {
    let Some(fixture) = Fixture::create() else {
        eprintln!("skipping: no savegames found");
        return;
    };
    let mut test = launch(fixture.save_dir.clone());
    assert!(click_text(&mut test, "Character"));
    // The first paragraph is the save folder input, the second one is the name.
    let paragraphs = test.find_many(|node, element| {
        let paragraph = Paragraph::try_downcast(element)?;
        (!paragraph.to_string().trim().is_empty()).then_some(node)
    });
    let field = paragraphs.get(1).expect("no character input found");
    click_node(&mut test, field);
    test.write_text("X");
    test.sync_and_update();
    assert!(
        find_label_text(&test, "unapplied input").is_some(),
        "the unapplied changes badge is missing"
    );

    assert!(click_text(&mut test, "Scan"));
    assert!(
        find_label_text(&test, "Apply or discard").is_some(),
        "scanning was not blocked"
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
    assert_eq!(session.entries.len(), 1);
    assert!(session.open(0));
    let character = session.character.clone().expect("character");
    let mut form = jadeforge_gui::CharacterForm::from_character(&character);
    form.level = "9".to_owned();
    session.apply(&form).expect("apply");
    assert!(session.is_dirty());

    // Scanning another folder must not throw the changes away.
    session.set_directory(std::env::temp_dir());
    assert_eq!(session.directory, fixture.save_dir, "folder changed anyway");
    assert!(session.is_dirty(), "the changes were dropped");
    assert!(
        session.message.text.contains("unsaved"),
        "{}",
        session.message.text
    );

    session.revert().expect("revert");
    assert!(!session.is_dirty());
}
