//! Command line interface for `JadeForge`.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use jadeforge_core::gff::Value;
use jadeforge_core::{Error, Gff, SaveGame, SaveOptions, locate_player};
use jadeforge_gamedata::GameData;

/// `JadeForge` - a save editor for Jade Empire.
#[derive(Debug, Parser)]
#[command(name = "jadeforge", version, about, long_about = None)]
struct Cli {
    /// Savegame file (`save.sac`) or the folder that contains it.
    #[arg(global = true, value_name = "SAVE")]
    save: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print a summary of a savegame.
    Info,
    /// List the blocks stored in a savegame.
    Blocks,
    /// Print the player character.
    Character,
    /// Print the player inventory.
    Items,
    /// Print the player's fighting styles.
    Styles,
    /// Print the player's companions.
    Companions,
    /// Print a field of a block.
    Get {
        /// Path of the block, for example `save.sav`.
        block: String,
        /// Dotted field path, for example `PlayerCreature.Stats.Gold`.
        path: String,
    },
    /// Change a field of a block.
    Set {
        /// Path of the block, for example `save.sav`.
        block: String,
        /// Dotted field path, for example `PlayerCreature.Stats.Gold`.
        path: String,
        /// New value. Strings and numbers are accepted.
        value: String,
        /// Only print what would change.
        #[arg(long)]
        dry_run: bool,
        /// Do not create a backup file.
        #[arg(long)]
        no_backup: bool,
    },
    /// Dump a block as JSON.
    Dump {
        /// Path of the block. Defaults to `save.sav`.
        #[arg(default_value = "save.sav")]
        block: String,
        /// Optional dotted path that is dumped instead of the whole block.
        #[arg(long)]
        path: Option<String>,
    },
    /// Verify that a savegame can be parsed and written again.
    Verify,
    /// Add an item to the player inventory.
    AddItem {
        /// Item template id.
        item_id: u16,
        /// Item category.
        #[arg(long, default_value_t = 6)]
        kind: u8,
        /// Stack size.
        #[arg(long, default_value_t = 1)]
        count: u8,
        /// Only print what would happen.
        #[arg(long)]
        dry_run: bool,
        /// Do not create a backup file.
        #[arg(long)]
        no_backup: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), Error> {
    let path = resolve_save_path(cli.save.as_deref())?;
    match cli.command {
        Command::Info => info(&path),
        Command::Blocks => blocks(&path),
        Command::Character => character(&path),
        Command::Items => items(&path),
        Command::Styles => styles(&path),
        Command::Companions => companions(&path),
        Command::Get { block, path: field } => get(&path, &block, &field),
        Command::Set {
            block,
            path: field,
            value,
            dry_run,
            no_backup,
        } => set(&path, &block, &field, &value, dry_run, no_backup),
        Command::Dump { block, path: field } => dump(&path, &block, field.as_deref()),
        Command::Verify => verify(&path),
        Command::AddItem {
            item_id,
            kind,
            count,
            dry_run,
            no_backup,
        } => add_item(&path, item_id, kind, count, dry_run, no_backup),
    }
}

fn resolve_save_path(path: Option<&Path>) -> Result<PathBuf, Error> {
    let candidate = path.map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    if candidate.is_dir() {
        let nested = candidate.join("save.sac");
        if nested.is_file() {
            return Ok(nested);
        }
        return Err(Error::Corrupt(format!(
            "{} does not contain a save.sac",
            candidate.display()
        )));
    }
    if candidate.is_file() {
        return Ok(candidate);
    }
    Err(Error::Corrupt(format!(
        "{} does not exist",
        candidate.display()
    )))
}

fn info(path: &Path) -> Result<(), Error> {
    let save = SaveGame::open(path)?;
    println!("Savegame: {}", path.display());
    println!("Blocks:   {}", save.blocks().len());
    let character = save.character()?;
    println!("Name:     {}", character.name);
    println!("Level:    {}", character.level);
    println!("XP:       {}", character.experience);
    if let Some(gold) = character.gold {
        println!("Gold:     {gold}");
    }
    println!("Chapter:  {}", character.chapter);
    println!("Playtime: {}", format_duration(character.time_played));
    println!(
        "Health:   {:.0} / {}",
        character.health, character.health_max
    );
    println!("Chi:      {:.0} / {}", character.chi, character.chi_max);
    println!("Focus:    {:.0} / {}", character.focus, character.focus_max);
    println!(
        "Body {}  Mind {}  Spirit {}",
        character.body, character.mind, character.spirit
    );
    println!(
        "Charm {}  Intuition {}  Intimidate {}",
        character.charm, character.intuition, character.intimidate
    );
    if character.attribute_points > 0 {
        println!("Unspent attribute points: {}", character.attribute_points);
    }
    if let Some(location) = locate_player(&save) {
        println!(
            "Player:   {} (prefix {:?})",
            location.block, location.prefix
        );
    }
    Ok(())
}

fn blocks(path: &Path) -> Result<(), Error> {
    let save = SaveGame::open(path)?;
    for (index, block) in save.blocks().iter().enumerate() {
        println!(
            "{index:3}  {:22}  structs {:5}  fields {:6}  labels {:4}",
            block.name,
            block.gff().structs.len(),
            block.gff().field_count(),
            block.gff().labels.len()
        );
    }
    Ok(())
}

fn character(path: &Path) -> Result<(), Error> {
    let save = SaveGame::open(path)?;
    let character = save.character()?;
    println!("{character:#?}");
    Ok(())
}

fn items(path: &Path) -> Result<(), Error> {
    let save = SaveGame::open(path)?;
    let game = GameData::discover(path);
    let items = save.items()?;
    println!("{:<32} {:>6} {:>6} {:>5}", "name", "id", "type", "count");
    for item in items {
        let name = game
            .as_ref()
            .and_then(|game| game.item(item.item_id))
            .map_or_else(
                || format!("<item {}>", item.item_id),
                |info| info.name.clone(),
            );
        println!(
            "{:<32} {:>6} {:>6} {:>5}",
            name, item.item_id, item.kind, item.count
        );
    }
    Ok(())
}

fn styles(path: &Path) -> Result<(), Error> {
    let save = SaveGame::open(path)?;
    let game = GameData::discover(path);
    let styles = save.styles()?;
    println!("{:<32} {:>6} {:>5}", "name", "level", "id");
    for style in styles {
        let name = game
            .as_ref()
            .and_then(|game| game.style(style.style_id))
            .map_or_else(
                || format!("<style {}>", style.style_id),
                |info| info.name.clone(),
            );
        println!("{:<32} {:>6} {:>5}", name, style.level, style.style_id);
    }
    Ok(())
}

fn companions(path: &Path) -> Result<(), Error> {
    let save = SaveGame::open(path)?;
    let henchmen = save.henchmen()?;
    println!("{:<16} {:>6} {:>10}", "template", "level", "available");
    for henchman in henchmen {
        let available = if henchman.available { "yes" } else { "no" };
        println!(
            "{:<16} {:>6} {:>10}",
            henchman.resref, henchman.level, available
        );
    }
    Ok(())
}

fn get(path: &Path, block_name: &str, field_path: &str) -> Result<(), Error> {
    let save = SaveGame::open(path)?;
    let block = save
        .block(block_name)
        .ok_or_else(|| Error::Corrupt(format!("block {block_name:?} does not exist")))?;
    let field = block.gff().get(field_path)?;
    println!(
        "{field_path} = {} (type {})",
        format_value(&field.value),
        field.value.type_name()
    );
    Ok(())
}

fn set(
    path: &Path,
    block_name: &str,
    field_path: &str,
    raw: &str,
    dry_run: bool,
    no_backup: bool,
) -> Result<(), Error> {
    let mut save = SaveGame::open(path)?;
    let block = save
        .block_mut(block_name)
        .ok_or_else(|| Error::Corrupt(format!("block {block_name:?} does not exist")))?;
    let gff = block.gff_mut();
    let current = gff.get(field_path)?;
    let value = parse_value(&current.value, raw)?;
    let before = format_value(&current.value);
    gff.set(field_path, value.clone())?;
    if dry_run {
        println!(
            "{block_name}:{field_path}: {before} -> {} (dry run)",
            format_value(&value)
        );
        return Ok(());
    }
    save.save(SaveOptions { backup: !no_backup })?;
    println!(
        "{block_name}:{field_path}: {before} -> {}",
        format_value(&value)
    );
    Ok(())
}

fn dump(path: &Path, block_name: &str, field_path: Option<&str>) -> Result<(), Error> {
    let save = SaveGame::open(path)?;
    let block = save
        .block(block_name)
        .ok_or_else(|| Error::Corrupt(format!("block {block_name:?} does not exist")))?;
    let gff: &Gff = block.gff();
    let value = match field_path {
        Some(field_path) => {
            let field = gff.get(field_path)?;
            gff.value_json(&field.value)
        }
        None => gff.to_json(),
    };
    let text = serde_json::to_string_pretty(&value)
        .map_err(|error| Error::Corrupt(format!("cannot render JSON: {error}")))?;
    println!("{text}");
    Ok(())
}

fn verify(path: &Path) -> Result<(), Error> {
    let save = SaveGame::open(path)?;
    save.verify()?;
    println!(
        "ok: {} blocks, {} bytes",
        save.blocks().len(),
        save.to_bytes()?.len()
    );
    Ok(())
}

fn add_item(
    path: &Path,
    item_id: u16,
    kind: u8,
    count: u8,
    dry_run: bool,
    no_backup: bool,
) -> Result<(), Error> {
    let mut save = SaveGame::open(path)?;
    if dry_run {
        println!("would add item {item_id} (type {kind}, count {count})");
        return Ok(());
    }
    let index = save.add_item(item_id, kind, count)?;
    save.save(SaveOptions { backup: !no_backup })?;
    println!("added item {item_id} (type {kind}, count {count}) as struct {index}");
    Ok(())
}

fn parse_value(current: &Value, raw: &str) -> Result<Value, Error> {
    let mismatch = |expected: &'static str| {
        Error::Corrupt(format!(
            "cannot store {raw:?} in a field of type {}; expected {expected}",
            current.type_name()
        ))
    };
    match current {
        Value::Byte(_) => raw
            .parse()
            .map(Value::Byte)
            .map_err(|_| mismatch("a number up to 255")),
        Value::Char(_) => raw
            .parse()
            .map(Value::Char)
            .map_err(|_| mismatch("a number")),
        Value::Word(_) => raw
            .parse()
            .map(Value::Word)
            .map_err(|_| mismatch("a number up to 65535")),
        Value::Short(_) => raw
            .parse()
            .map(Value::Short)
            .map_err(|_| mismatch("a number")),
        Value::DWord(_) => raw
            .parse()
            .map(Value::DWord)
            .map_err(|_| mismatch("a number up to 4294967295")),
        Value::Int(_) => raw
            .parse()
            .map(Value::Int)
            .map_err(|_| mismatch("a number")),
        Value::DWord64(_) => raw
            .parse()
            .map(Value::DWord64)
            .map_err(|_| mismatch("a number")),
        Value::Int64(_) => raw
            .parse()
            .map(Value::Int64)
            .map_err(|_| mismatch("a number")),
        Value::Float(_) => raw
            .parse()
            .map(Value::Float)
            .map_err(|_| mismatch("a number")),
        Value::Double(_) => raw
            .parse()
            .map(Value::Double)
            .map_err(|_| mismatch("a number")),
        Value::String(_) => Ok(Value::String(raw.to_owned())),
        Value::ResRef(_) => Ok(Value::ResRef(raw.to_owned())),
        Value::LocString(locstring) => Ok(Value::LocString(jadeforge_core::LocString {
            text: Some(raw.to_owned()),
            strref: locstring.strref,
        })),
        other => Err(Error::Corrupt(format!(
            "fields of type {} cannot be edited from the command line",
            other.type_name()
        ))),
    }
}

fn format_value(value: &Value) -> String {
    match value {
        Value::LocString(locstring) => locstring
            .text
            .clone()
            .unwrap_or_else(|| format!("<strref {}>", locstring.strref.unwrap_or(0))),
        Value::Void(bytes) => format!("<{} bytes>", bytes.len()),
        Value::Struct(index) => format!("<struct {index}>"),
        Value::List(elements) => format!("<list of {}>", elements.len()),
        Value::Quaternion(values) => format!("{values:?}"),
        Value::Vector3(values) => format!("{values:?}"),
        other => other
            .as_str()
            .map(str::to_owned)
            .or_else(|| other.as_i64().map(|value| value.to_string()))
            .or_else(|| other.as_f32().map(|value| value.to_string()))
            .unwrap_or_else(|| "<value>".to_owned()),
    }
}

fn format_duration(microseconds: u64) -> String {
    let total_seconds = microseconds / 1_000_000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    format!("{hours}h {minutes:02}m {seconds:02}s")
}
