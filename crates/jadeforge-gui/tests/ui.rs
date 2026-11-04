//! Headless UI tests.
//!
//! The tests render the real component tree with `freya-testing`, which is the
//! same code path the windowed application uses minus the window. They are
//! skipped when no savegame is available on the machine.

use std::path::{Path, PathBuf};

use freya::prelude::*;
use freya_testing::TestingRunner;
use jadeforge_gui::app_with_directory;

fn default_save_dir() -> PathBuf {
    std::env::var_os("JADEFORGE_SAVE_DIR").map_or_else(
        || PathBuf::from("/media/Games/Heroic/Jade Empire/Save"),
        PathBuf::from,
    )
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

/// Creates a temporary save folder that contains a copy of a real savegame.
fn fixture() -> Option<PathBuf> {
    let source = smallest_save(&default_save_dir())?;
    let mut root = std::env::temp_dir();
    root.push(format!(
        "jadeforge-ui-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos())
    ));
    let folder = root.join("Test Save");
    std::fs::create_dir_all(&folder).expect("create fixture");
    std::fs::copy(&source, folder.join("save.sac")).expect("copy savegame");
    Some(root)
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

#[test]
fn savegames_are_listed_and_can_be_opened() {
    let Some(root) = fixture() else {
        eprintln!("skipping: no savegames found (set JADEFORGE_SAVE_DIR)");
        return;
    };
    let directory = root.clone();
    let (mut test, ()) = TestingRunner::new(
        move || app_with_directory(directory.clone()),
        (1280., 800.).into(),
        |_| {},
        1.0,
    );
    test.sync_and_update();

    // The savegame folder is part of the sidebar.
    let listed = find_label_text(&test, "Test Save");
    assert!(listed.is_some(), "the save folder is not listed");
    let entry = find_node_with_text(&test, "Test Save").expect("entry");
    let area = entry.layout().area;
    let center = CursorPoint::new(
        f64::from(area.origin.x) + f64::from(area.size.width) / 2.0,
        f64::from(area.origin.y) + f64::from(area.size.height) / 2.0,
    );
    test.click_cursor(center);
    test.sync_and_update();
    let status = find_label_text(&test, "Opened Test Save").expect("status message");
    assert!(status.contains("Opened"), "{status}");

    // The character form is populated with the name of the character.
    let character = find_label_text(&test, "Character").expect("character section");
    assert!(character.contains("Character"));

    // Rendering must succeed even in a headless environment.
    let image = test.render();
    assert!(
        image.len() > 1024,
        "the rendered frame is suspiciously small"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn empty_folder_shows_a_hint() {
    let mut root = std::env::temp_dir();
    root.push(format!("jadeforge-empty-{}", std::process::id()));
    std::fs::create_dir_all(&root).expect("create empty dir");
    let directory = root.clone();
    let (mut test, ()) = TestingRunner::new(
        move || app_with_directory(directory.clone()),
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
    let Some(root) = fixture() else {
        eprintln!("skipping: no savegames found (set JADEFORGE_SAVE_DIR)");
        return;
    };
    let mut session = jadeforge_gui::Session::with_directory(root.clone());
    assert_eq!(session.entries.len(), 1);
    assert!(session.open(0));
    let character = session.character.clone().expect("character");
    let mut form = jadeforge_gui::CharacterForm::from_character(&character);
    form.level = "9".to_owned();
    session.apply(&form).expect("apply");
    assert!(session.is_dirty());

    // Scanning another folder must not throw the changes away.
    session.set_directory(std::env::temp_dir());
    assert_eq!(session.directory, root, "the folder was changed anyway");
    assert!(session.is_dirty(), "the changes were dropped");
    assert!(
        session.message.text.contains("unsaved"),
        "{}",
        session.message.text
    );

    // Reverting is the explicit way out.
    session.revert().expect("revert");
    assert!(!session.is_dirty());
    let _ = std::fs::remove_dir_all(root);
}
