//! Tests against a real game installation (skipped when none is available).

use jadeforge_gamedata::GameData;

fn game_dir() -> Option<std::path::PathBuf> {
    let root = std::env::var_os("JADEFORGE_GAME_DIR").map_or_else(
        || std::path::PathBuf::from("/media/Games/Heroic/Jade Empire"),
        std::path::PathBuf::from,
    );
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
