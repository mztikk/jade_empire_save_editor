//! Command line interface for `JadeForge`.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

macro_rules! out {
    ($($arg:tt)*) => {
        write_stdout(format_args!($($arg)*))?
    };
}

fn write_stdout(arguments: std::fmt::Arguments<'_>) -> Result<(), Error> {
    let mut stdout = std::io::stdout().lock();
    writeln!(stdout, "{arguments}").map_err(|error| Error::io("<stdout>", error))
}

use clap::{Parser, Subcommand};
use jadeforge_core::gff::Value;
use jadeforge_core::{Error, Gff, SaveGame, SaveOptions, locate_player};
use jadeforge_gamedata::{GameData, NameIndex, category_for_save_type, export_english};

/// `JadeForge` - a save editor for Jade Empire.
#[derive(Debug, Parser)]
#[command(name = "jadeforge-cli", version, about, long_about = None)]
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
    /// Change a field of a block (raw write, no typed range checks).
    ///
    /// This is the debugging escape hatch. The desktop editor and the typed
    /// commands range check values; `set` writes whatever the field can hold.
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
        /// Emit JSON. This is the only output format; the flag exists so that
        /// documented `dump --json` commands keep working.
        #[arg(long)]
        json: bool,
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
    /// Write the bundled English name index from an installation.
    ///
    /// This is a maintainer command: it extracts item and style names, not a
    /// general data dump, and must point at an English installation.
    ExportNames {
        /// Game installation root (the folder that contains `dialog.tlk`).
        #[arg(long, value_name = "DIR")]
        game: PathBuf,
        /// Output TOML file.
        #[arg(long, value_name = "FILE")]
        output: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(Error::Io { source, .. }) if source.kind() == std::io::ErrorKind::BrokenPipe => {
            // `jadeforge-cli dump | head` is a normal way to inspect a large
            // export; a closed pipe is not an application error.
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), Error> {
    match cli.command {
        Command::ExportNames { game, output } => export_names(&game, &output),
        command => {
            let path = resolve_save_path(cli.save.as_deref())?;
            run_save(command, &path)
        }
    }
}

fn run_save(command: Command, path: &Path) -> Result<(), Error> {
    match command {
        Command::Info => info(path),
        Command::Blocks => blocks(path),
        Command::Character => character(path),
        Command::Items => items(path),
        Command::Styles => styles(path),
        Command::Companions => companions(path),
        Command::Get { block, path: field } => get(path, &block, &field),
        Command::Set {
            block,
            path: field,
            value,
            dry_run,
            no_backup,
        } => set(path, &block, &field, &value, dry_run, no_backup),
        Command::Dump {
            block,
            path: field,
            json: _,
        } => dump(path, &block, field.as_deref()),
        Command::Verify => verify(path),
        Command::AddItem {
            item_id,
            kind,
            count,
            dry_run,
            no_backup,
        } => add_item(path, item_id, kind, count, dry_run, no_backup),
        Command::ExportNames { .. } => Err(Error::Corrupt(
            "export-names does not use a savegame".to_owned(),
        )),
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
    out!("Savegame: {}", path.display());
    out!("Blocks:   {}", save.blocks().len());
    let character = save.character()?;
    out!("Name:     {}", character.name);
    out!("Level:    {}", character.level);
    out!("XP:       {}", character.experience);
    if let Some(gold) = character.gold {
        out!("Gold:     {gold}");
    }
    out!("Chapter:  {}", character.chapter);
    out!("Playtime: {}", format_duration(character.time_played));
    out!(
        "Health:   {:.0} / {}",
        character.health,
        character.health_max
    );
    out!("Chi:      {:.0} / {}", character.chi, character.chi_max);
    out!("Focus:    {:.0} / {}", character.focus, character.focus_max);
    out!(
        "Body {}  Mind {}  Spirit {}",
        character.body,
        character.mind,
        character.spirit
    );
    out!(
        "Charm {}  Intuition {}  Intimidate {}",
        character.charm,
        character.intuition,
        character.intimidate
    );
    if character.attribute_points > 0 {
        out!("Unspent attribute points: {}", character.attribute_points);
    }
    if let Some(location) = locate_player(&save) {
        out!(
            "Player:   {} (prefix {:?})",
            location.block,
            location.prefix
        );
    }
    Ok(())
}

fn blocks(path: &Path) -> Result<(), Error> {
    let save = SaveGame::open(path)?;
    for (index, block) in save.blocks().iter().enumerate() {
        out!(
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
    out!("{character:#?}");
    Ok(())
}

fn items(path: &Path) -> Result<(), Error> {
    let save = SaveGame::open(path)?;
    let game = GameData::discover(path);
    let names = NameIndex::bundled_english();
    let items = save.items()?;
    out!(
        "{:<32} {:>6} {:<12} {:>5}",
        "name",
        "id",
        "category",
        "count"
    );
    for item in items {
        let info = game.as_ref().and_then(|game| game.item(item.item_id));
        let bundled = if game.is_none() {
            names.and_then(|names| names.item(item.item_id))
        } else {
            None
        };
        let name = info
            .map(|info| info.name.clone())
            .or_else(|| bundled.map(|item| item.name.clone()))
            .unwrap_or_else(|| format!("<item {}>", item.item_id));
        let category = info
            .and_then(|info| info.category.clone())
            .or_else(|| bundled.and_then(|item| item.category.clone()))
            .or_else(|| category_for_save_type(item.kind).map(str::to_owned))
            .unwrap_or_else(|| format!("type {}", item.kind));
        out!(
            "{:<32} {:>6} {:<12} {:>5}",
            name,
            item.item_id,
            category,
            item.count
        );
    }
    Ok(())
}

fn styles(path: &Path) -> Result<(), Error> {
    let save = SaveGame::open(path)?;
    let game = GameData::discover(path);
    let names = NameIndex::bundled_english();
    let styles = save.styles()?;
    out!("{:<32} {:>6} {:>5}", "name", "level", "id");
    for style in styles {
        let bundled = if game.is_none() {
            names.and_then(|names| names.style(style.style_id))
        } else {
            None
        };
        let name = game
            .as_ref()
            .and_then(|game| game.style(style.style_id))
            .map(|info| info.name.clone())
            .or_else(|| bundled.map(ToOwned::to_owned))
            .unwrap_or_else(|| format!("<style {}>", style.style_id));
        out!("{:<32} {:>6} {:>5}", name, style.level, style.style_id);
    }
    Ok(())
}

fn companions(path: &Path) -> Result<(), Error> {
    let save = SaveGame::open(path)?;
    let henchmen = save.henchmen()?;
    out!("{:<16} {:>6} {:>10}", "template", "level", "available");
    for henchman in henchmen {
        let available = if henchman.available { "yes" } else { "no" };
        out!(
            "{:<16} {:>6} {:>10}",
            henchman.resref,
            henchman.level,
            available
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
    out!(
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
        out!(
            "{block_name}:{field_path}: {before} -> {} (dry run)",
            format_value(&value)
        );
        return Ok(());
    }
    save.save(SaveOptions { backup: !no_backup })?;
    out!(
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
            jadeforge_core::json::value_json(gff, &field.value)
        }
        None => jadeforge_core::json::to_json(gff),
    };
    let text = serde_json::to_string_pretty(&value)
        .map_err(|error| Error::Corrupt(format!("cannot render JSON: {error}")))?;
    out!("{text}");
    Ok(())
}

fn verify(path: &Path) -> Result<(), Error> {
    let save = SaveGame::open(path)?;
    save.verify()?;
    out!(
        "ok: {} blocks, {} bytes",
        save.blocks().len(),
        save.to_bytes()?.len()
    );
    Ok(())
}

fn export_names(game_root: &Path, output: &Path) -> Result<(), Error> {
    let game = GameData::open(game_root)
        .map_err(|error| Error::Corrupt(format!("cannot read game data: {error}")))?;
    let text = export_english(&game)
        .map_err(|error| Error::Corrupt(format!("cannot build the name index: {error}")))?;
    std::fs::write(output, text).map_err(|source| Error::io(output, source))?;
    out!(
        "wrote {} items and {} styles to {}",
        game.playable_item_count(),
        game.style_count(),
        output.display()
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
    let index = save.add_item(item_id, kind, count)?;
    if dry_run {
        out!("would add item {item_id} (type {kind}, count {count}) as struct {index} (dry run)");
        return Ok(());
    }
    save.save(SaveOptions { backup: !no_backup })?;
    out!("added item {item_id} (type {kind}, count {count}) as struct {index}");
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
