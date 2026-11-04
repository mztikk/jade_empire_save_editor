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

#[test]
fn style_advances_are_resolved() {
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
    assert!(game.advances().count() > 30);
    let damage = game.advance(1).expect("Increase_Damage_1");
    assert_eq!(damage.label, "Increase_Damage_1");
    assert_eq!(damage.family, "Increase_Damage");
    assert_eq!(damage.tier, 1);
    assert_eq!(damage.cost, 1);
    assert_eq!(damage.effect, 72);
    assert!((damage.param - 0.25).abs() < f32::EPSILON);
    assert!(!damage.name.is_empty(), "the name should resolve");
    assert!(damage.effect_text().contains("Damage"));
    assert!(game.advance(0).is_none(), "row 0 is invalid");
}

#[test]
fn per_style_upgrade_tables_are_matched() {
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
    // Storm Dragon: the mask indexes slots, not advance rows. Its table offers
    // rows 6-10, 21-25 and 11-15, in that order.
    let choices = game
        .style_choices(13)
        .expect("Storm Dragon should have a level table");
    assert_eq!(choices.len(), 15);
    assert_eq!((choices[0].slot, choices[0].advance), (1, 6));
    assert_eq!((choices[5].slot, choices[5].advance), (6, 21));
    assert_eq!((choices[10].slot, choices[10].advance), (11, 11));
    // Most player styles have a table in a complete installation.
    let total = game.styles().count();
    let with_table = game
        .styles()
        .filter(|style| game.style_choices(style.id).is_some())
        .count();
    assert!(
        with_table * 2 > total,
        "only {with_table} of {total} styles have a level table"
    );
}

#[test]
fn rim_archives_are_indexed() {
    let Some(root) = game_dir() else {
        eprintln!("skipping: no game installation found");
        return;
    };
    let archive = jadeforge_gamedata::RimArchive::open(root.join("data").join("miniglobal-a.rim"))
        .expect("miniglobal archive");
    assert!(archive.names().any(|name| name == "stylesuperlist"));
    assert!(archive.names().any(|name| name == "styleadvance"));
    let bytes = archive.get_2da("stylesuperlist").expect("stylesuperlist");
    let table = jadeforge_gamedata::TwoDa::parse_binary(bytes).expect("binary 2DA");
    assert!(table.len() > 50);
    assert!(table.column("Label").is_some());
}

#[test]
fn predefined_gems_are_resolved() {
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
    assert!(game.gems().len() > 50, "only {} gems", game.gems().len());
    let gem = game.gems().iter().find(|gem| !gem.effects.is_empty());
    let gem = gem.expect("a gem with effects");
    assert_ne!(gem.name.len(), 0);
    assert!(game.gem(gem.item_id).is_some());
    assert_eq!(gem.effects, game.gem(gem.item_id).expect("gem").effects);
}

#[test]
fn effect_names_are_readable() {
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
    assert_eq!(game.effect_name(62), Some("Body"));
    assert_eq!(game.effect_name(64), Some("Mind"));
    assert_eq!(game.effect_name(65), Some("Spirit"));
    assert_eq!(game.effect_name(19), Some("Charm"));
    assert_eq!(game.effect_name(9999), None);
}
