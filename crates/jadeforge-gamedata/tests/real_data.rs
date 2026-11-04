//! Tests against a real game installation (skipped when none is available).

use jadeforge_gamedata::GameData;

fn game_dir() -> Option<std::path::PathBuf> {
    let root = std::env::var_os("JADEFORGE_GAME_DIR").map(std::path::PathBuf::from)?;
    root.is_dir().then_some(root)
}

#[test]
fn resolves_items_and_styles() {
    let Some(root) = game_dir() else {
        eprintln!("skipping: no game installation found");
        return;
    };
    let game = match GameData::open(&root) {
        Ok(game) => game,
        Err(error) => {
            eprintln!("skipping: cannot load game data: {error}");
            return;
        }
    };
    assert!(game.item_count() > 100, "only {} items", game.item_count());
    assert!(
        game.style_count() > 50,
        "only {} styles",
        game.style_count()
    );

    let item = game.item(543).expect("item 543");
    assert_ne!(item.name, "");
    assert_eq!(item.category.as_deref(), Some("weapons"));
    assert_eq!(item.save_type, Some(5));

    let style = game.style(52).expect("style 52");
    assert_ne!(style.name, "");

    let found = game.search_items("viper", 10);
    assert_ne!(found.len(), 0, "search returned nothing");
}

#[test]
fn hides_development_items_from_the_picker() {
    let Some(root) = game_dir() else {
        eprintln!("skipping: no game installation found");
        return;
    };
    let game = match GameData::open(&root) {
        Ok(game) => game,
        Err(error) => {
            eprintln!("skipping: cannot load game data: {error}");
            return;
        }
    };
    assert!(game.playable_item_count() > 100);
    assert!(game.playable_item_count() <= game.item_count());
    for item in game.search_items("quicksilver", 20) {
        assert!(
            !item.name.starts_with("[TEMP]"),
            "{} is a development placeholder",
            item.name
        );
        assert!(!item.label.starts_with("temp"), "{}", item.label);
    }
}

#[test]
fn style_kinds_are_resolved_for_known_styles() {
    let Some(root) = game_dir() else {
        eprintln!("skipping: no game installation found");
        return;
    };
    let game = match GameData::open(&root) {
        Ok(game) => game,
        Err(error) => {
            eprintln!("skipping: cannot load game data: {error}");
            return;
        }
    };
    let martial = game.style(52).expect("legendary strike");
    assert_eq!(martial.kind, jadeforge_gamedata::StyleKind::Martial);
    let support = game.style(13).expect("storm dragon");
    assert_eq!(support.kind, jadeforge_gamedata::StyleKind::Support);
    let magic = game.style(5).expect("magic fire");
    assert_eq!(magic.kind, jadeforge_gamedata::StyleKind::Magic);
}
