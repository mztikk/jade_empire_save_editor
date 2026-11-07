//! Typed view of the player character.
//!
//! The player creature lives in the `save.sav` block below `PlayerCreature`.
//! A few older releases ship a `polyplayer.sav` block that keeps a character
//! sheet at the root; when `save.sav` has no player the editor falls back to
//! that block.

use crate::error::{Error, Result};
use crate::gff::{Gff, Value};
use crate::save::{GLOBAL_BLOCK, POLYPLAYER_BLOCK, SaveGame};

/// Alignment value the engine starts every character with.
///
/// The field only appears once the player's decisions move it, so a savegame
/// without it is balanced rather than closed fist. Everything below is the Way
/// of the Closed Fist, everything above the Way of the Open Palm.
pub const DEFAULT_ALIGNMENT: i64 = 128;

/// Lowest alignment the game's meter can show.
pub const MIN_ALIGNMENT: i32 = 0;

/// Highest alignment the game's meter can show.
pub const MAX_ALIGNMENT: i32 = 255;

/// Highest value the engine can represent for a signed 32 bit statistic.
///
/// Gold is stored as a `dword` and the kill counter as an `int`, but Jade
/// Empire's script API reads both as signed `int` (`GetGold`, ...), so a stored
/// `4294967295` reaches the game as `-1`. The editor uses this limit for the
/// fields that have no narrower bound in the game's own tables.
pub const MAX_STAT: u32 = 2_147_483_647;

/// Highest level in the shipped `levelxp.2da`.
///
/// The table holds thresholds for levels `0..=32`; level 32 needs 1,000,000 XP.
/// Higher levels have no table row, so the engine cannot level them.
pub const MAX_LEVEL: u8 = 32;

/// Experience needed to reach [`MAX_LEVEL`] in the shipped `levelxp.2da`.
pub const MAX_EXPERIENCE: u32 = 1_000_000;

/// Highest unspent attribute point balance a retail character can reach.
///
/// Character creation grants four points and every one of the 31 level-ups to
/// the level 32 cap grants three, for `4 + 3 * 31 = 97`.
pub const MAX_ATTRIBUTE_POINTS: u32 = 97;

/// Highest row of the shipped `appearance.2da` (rows `0..=236`).
///
/// `Stats.Appearance` is a row index, not a free number. The engine crashes
/// when the row does not exist.
pub const MAX_APPEARANCE: i32 = 236;

/// Highest internal story chapter in the shipped campaign.
///
/// The field is absent until the campaign starts, so `0` stays valid as the
/// "no chapter recorded" default.
pub const MAX_CHAPTER: u32 = 8;

/// A single field that was changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    /// Absolute path of the field.
    pub field: String,
    /// Value before the edit.
    pub before: String,
    /// Value after the edit.
    pub after: String,
}

/// Everything a player can sensibly edit about the character.
#[derive(Debug, Clone, PartialEq)]
pub struct Character {
    /// First name shown in the game.
    pub name: String,
    /// Level, stored as a byte and bounded by [`MAX_LEVEL`].
    pub level: u8,
    /// Current experience points, bounded by [`MAX_EXPERIENCE`].
    pub experience: u32,
    /// Gold, when the savegame has a gold field.
    ///
    /// The engine reads the stored dword as a signed `int`; see [`MAX_STAT`].
    pub gold: Option<u32>,
    /// Unspent attribute points, bounded by [`MAX_ATTRIBUTE_POINTS`].
    pub attribute_points: u32,
    /// Current health.
    pub health: f32,
    /// Maximum health.
    pub health_max: u32,
    /// Current chi.
    pub chi: f32,
    /// Maximum chi.
    pub chi_max: u32,
    /// Current focus.
    pub focus: f32,
    /// Maximum focus.
    pub focus_max: f32,
    /// Body attribute.
    pub body: u32,
    /// Mind attribute.
    pub mind: u32,
    /// Spirit attribute.
    pub spirit: u32,
    /// Charm skill.
    pub charm: i32,
    /// Intuition skill.
    pub intuition: i32,
    /// Intimidate skill.
    pub intimidate: i32,
    /// Alignment, from the Closed Fist (`0`) to the Open Palm (`255`).
    pub alignment: i32,
    /// Appearance index into `appearance.2da`, bounded by [`MAX_APPEARANCE`].
    pub appearance: i32,
    /// Story chapter, bounded by [`MAX_CHAPTER`].
    pub chapter: u32,
    /// Total play time in microseconds.
    pub time_played: u64,
    /// Number of kills.
    ///
    /// The engine reads the stored value as a signed `int`; see [`MAX_STAT`].
    pub kills: u32,
}

/// Where the player creature is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerLocation {
    /// Name of the block that holds the player.
    pub block: &'static str,
    /// Prefix that has to be prepended to every path inside the block.
    pub prefix: &'static str,
}

/// Finds the block and the path prefix that hold the player creature.
#[must_use]
pub fn locate_player(save: &SaveGame) -> Option<PlayerLocation> {
    if let Ok(block) = save.global_block()
        && player_at(block.gff(), "PlayerCreature.").is_some()
    {
        return Some(PlayerLocation {
            block: GLOBAL_BLOCK,
            prefix: "PlayerCreature.",
        });
    }
    if let Some(block) = save.block(POLYPLAYER_BLOCK)
        && player_at(block.gff(), "").is_some()
    {
        return Some(PlayerLocation {
            block: POLYPLAYER_BLOCK,
            prefix: "",
        });
    }
    None
}

fn player_at(gff: &Gff, prefix: &str) -> Option<usize> {
    let field = gff.get(format!("{prefix}FirstName")).ok()?;
    if !matches!(field.value, Value::String(_)) {
        return None;
    }
    Some(gff.root)
}

impl SaveGame {
    /// Location of the player creature inside this savegame.
    pub fn player_location(&self) -> Result<PlayerLocation> {
        locate_player(self).ok_or_else(|| {
            Error::Corrupt("the savegame does not contain a player character".to_owned())
        })
    }

    /// Reads the editable character data.
    pub fn character(&self) -> Result<Character> {
        let location = self.player_location()?;
        let block = self
            .block(location.block)
            .ok_or_else(|| Error::Corrupt("character block vanished".to_owned()))?;
        let mut character = Character::read(block.gff(), location.prefix)?;
        if location.block != GLOBAL_BLOCK
            && let Ok(global) = self.global_block()
        {
            character.read_global(global.gff());
        }
        Ok(character)
    }

    /// Writes the editable character data and returns the fields that changed.
    ///
    /// Both blocks are edited on working copies and committed only after every
    /// write succeeded. A rejected value therefore leaves the whole savegame
    /// untouched, even when the character and campaign live in different blocks.
    pub fn apply_character(&mut self, character: &Character) -> Result<Vec<Change>> {
        let location = self.player_location()?;
        let player_name = location.block;
        let same_block = player_name == GLOBAL_BLOCK;

        let player_original = self
            .block(player_name)
            .ok_or_else(|| Error::Corrupt("character block vanished".to_owned()))?
            .gff()
            .clone();
        let mut player_working = player_original;
        let mut changes = character.write(&mut player_working, location.prefix)?;

        if same_block {
            let global_changes = character.write_global(&mut player_working)?;
            if !changes.is_empty() || !global_changes.is_empty() {
                let block = self
                    .block_mut(player_name)
                    .ok_or_else(|| Error::Corrupt("character block vanished".to_owned()))?;
                *block.gff_mut() = player_working;
            }
            changes.extend(global_changes);
            return Ok(changes);
        }

        let global_original = self.block(GLOBAL_BLOCK).map(|block| {
            let was_modified = block.is_modified();
            (block.gff().clone(), was_modified)
        });
        let mut global_changes = Vec::new();
        let mut global_working = None;
        if let Some((working, _)) = global_original.clone() {
            let mut working = working;
            global_changes = character.write_global(&mut working)?;
            global_working = Some(working);
        }

        if !changes.is_empty() {
            let block = self
                .block_mut(player_name)
                .ok_or_else(|| Error::Corrupt("character block vanished".to_owned()))?;
            *block.gff_mut() = player_working;
        }
        if let (Some(working), Some((_, was_modified))) = (global_working, global_original) {
            if !global_changes.is_empty() {
                let block = self
                    .block_mut(GLOBAL_BLOCK)
                    .ok_or_else(|| Error::Corrupt("campaign block vanished".to_owned()))?;
                *block.gff_mut() = working;
            } else if !was_modified && let Some(block) = self.block_mut(GLOBAL_BLOCK) {
                block.mark_clean();
            }
        }
        changes.extend(global_changes);
        Ok(changes)
    }
}

impl Character {
    /// Reads the character from a GFF block.
    ///
    /// Numeric fields are read through checked conversions. A value that does
    /// not fit the typed model is an error rather than a silent truncation, so
    /// an unchanged apply can never rewrite a wider value into a narrower one.
    pub fn read(gff: &Gff, prefix: &str) -> Result<Self> {
        let path = |suffix: &str| format!("{prefix}{suffix}");
        let number = |suffix: &str| unsigned_field(gff, &path(suffix));
        let float = |suffix: &str| float_field(gff, &path(suffix));

        let health = float("Stats.Health")?.unwrap_or(0.0);
        let chi = float("Stats.Chi")?.unwrap_or(0.0);
        let focus = float("Stats.Focus.STAT_CUR")?.unwrap_or(0.0);

        let name = match optional_field(gff, &path("FirstName"))? {
            Some(value) => value.as_str().unwrap_or_default().to_owned(),
            None => String::new(),
        };
        let level = match number("Level")? {
            Some(value) => u8::try_from(value)
                .map_err(|_| narrow_error(&path("Level"), value, "an 8 bit level"))?,
            None => 1,
        };
        let experience = match number("CurrentXP")? {
            Some(value) => u32::try_from(value)
                .map_err(|_| narrow_error(&path("CurrentXP"), value, "32 bit experience"))?,
            None => 0,
        };
        let gold = match number("Stats.Gold")? {
            Some(value) => Some(
                u32::try_from(value)
                    .map_err(|_| narrow_error(&path("Stats.Gold"), value, "32 bit gold"))?,
            ),
            None => None,
        };
        let attribute_points = match number("Stats.UnassignedPts")? {
            Some(value) => u32::try_from(value)
                .map_err(|_| narrow_error(&path("Stats.UnassignedPts"), value, "32 bit points"))?,
            None => 0,
        };
        let health_max = match number("Stats.HealthBase")? {
            Some(value) => u32::try_from(value)
                .map_err(|_| narrow_error(&path("Stats.HealthBase"), value, "32 bit health"))?,
            None => health.max(0.0) as u32,
        };
        let chi_max = match number("Stats.ChiBase")? {
            Some(value) => u32::try_from(value)
                .map_err(|_| narrow_error(&path("Stats.ChiBase"), value, "32 bit chi"))?,
            None => chi.max(0.0) as u32,
        };
        let focus_max = float("Stats.Focus.STAT_MAX")?.unwrap_or(focus);
        let body = checked_u32(gff, &path("Stats.Body"), 0)?;
        let mind = checked_u32(gff, &path("Stats.Mind"), 0)?;
        let spirit = checked_u32(gff, &path("Stats.Spirit"), 0)?;
        let charm = checked_i32(gff, &path("Stats.Charm"), 0)?;
        let intuition = checked_i32(gff, &path("Stats.Intuition"), 0)?;
        let intimidate = checked_i32(gff, &path("Stats.Intimidate"), 0)?;
        let alignment = checked_i32(
            gff,
            &path("Stats.Alignment"),
            i32::try_from(DEFAULT_ALIGNMENT).unwrap_or(0),
        )?;
        let appearance = checked_i32(gff, &path("Stats.Appearance"), 0)?;
        let chapter = checked_u32(gff, "Campaign.Chapter", 0)?;
        let time_played = optional_field(gff, "Campaign.TimePlayed")?
            .map(|value| {
                value.as_u64().ok_or_else(|| {
                    Error::Corrupt(
                        "field \"Campaign.TimePlayed\" is not a non-negative number".to_owned(),
                    )
                })
            })
            .transpose()?
            .unwrap_or(0);
        let kills = checked_u32(gff, &path("Progress.Kills"), 0)?;

        Ok(Self {
            name,
            level,
            experience,
            gold,
            attribute_points,
            health,
            health_max,
            chi,
            chi_max,
            focus,
            focus_max,
            body,
            mind,
            spirit,
            charm,
            intuition,
            intimidate,
            alignment,
            appearance,
            chapter,
            time_played,
            kills,
        })
    }

    /// Reads the values that are stored in the campaign block.
    pub fn read_global(&mut self, gff: &Gff) {
        if let Ok(field) = gff.get("Campaign.Chapter") {
            self.chapter = field.value.as_u64().unwrap_or(0) as u32;
        }
        if let Ok(field) = gff.get("Campaign.TimePlayed") {
            self.time_played = field.value.as_u64().unwrap_or(0);
        }
    }

    /// Writes the character into a GFF block, returning the changed fields.
    #[allow(clippy::too_many_lines)] // one line per editable field reads best
    pub fn write(&self, gff: &mut Gff, prefix: &str) -> Result<Vec<Change>> {
        let mut changes = Vec::new();
        let path = |suffix: &str| format!("{prefix}{suffix}");

        check_unsigned("level", u64::from(self.level), 1, u64::from(MAX_LEVEL))?;
        check_unsigned(
            "experience",
            u64::from(self.experience),
            0,
            u64::from(MAX_EXPERIENCE),
        )?;
        if let Some(gold) = self.gold {
            check_unsigned("gold", u64::from(gold), 0, u64::from(MAX_STAT))?;
        }
        check_unsigned(
            "attribute points",
            u64::from(self.attribute_points),
            0,
            u64::from(MAX_ATTRIBUTE_POINTS),
        )?;
        check_unsigned("body", u64::from(self.body), 0, 1000)?;
        check_unsigned("mind", u64::from(self.mind), 0, 1000)?;
        check_unsigned("spirit", u64::from(self.spirit), 0, 1000)?;
        check_unsigned("health", u64::from(self.health_max), 1, 1_000_000)?;
        check_unsigned("chi", u64::from(self.chi_max), 1, 1_000_000)?;
        check_float("health", self.health)?;
        check_float("chi", self.chi)?;
        check_float("focus", self.focus)?;
        check_float("focus max", self.focus_max)?;
        check_signed("charm", i64::from(self.charm), -1_000_000, 1_000_000)?;
        check_signed(
            "intuition",
            i64::from(self.intuition),
            -1_000_000,
            1_000_000,
        )?;
        check_signed(
            "intimidate",
            i64::from(self.intimidate),
            -1_000_000,
            1_000_000,
        )?;
        check_signed(
            "alignment",
            i64::from(self.alignment),
            i64::from(MIN_ALIGNMENT),
            i64::from(MAX_ALIGNMENT),
        )?;
        check_signed(
            "appearance",
            i64::from(self.appearance),
            0,
            i64::from(MAX_APPEARANCE),
        )?;
        check_unsigned("kills", u64::from(self.kills), 0, u64::from(MAX_STAT))?;
        check_unsigned(
            "chapter",
            u64::from(self.chapter),
            0,
            u64::from(MAX_CHAPTER),
        )?;
        check_name(&self.name)?;

        set_text(gff, &path("FirstName"), &self.name, &mut changes)?;
        set_number(
            gff,
            &path("Level"),
            i64::from(self.level),
            false,
            &mut changes,
        )?;
        set_number(
            gff,
            &path("CurrentXP"),
            i64::from(self.experience),
            false,
            &mut changes,
        )?;
        if let Some(gold) = self.gold {
            set_number(
                gff,
                &path("Stats.Gold"),
                i64::from(gold),
                true,
                &mut changes,
            )?;
        }
        set_number(
            gff,
            &path("Stats.UnassignedPts"),
            i64::from(self.attribute_points),
            true,
            &mut changes,
        )?;
        set_float(gff, &path("Stats.Health"), self.health, &mut changes)?;
        set_number(
            gff,
            &path("Stats.HealthBase"),
            i64::from(self.health_max),
            false,
            &mut changes,
        )?;
        set_float(gff, &path("Stats.Chi"), self.chi, &mut changes)?;
        set_number(
            gff,
            &path("Stats.ChiBase"),
            i64::from(self.chi_max),
            false,
            &mut changes,
        )?;
        set_float(gff, &path("Stats.Focus.STAT_CUR"), self.focus, &mut changes)?;
        set_float(
            gff,
            &path("Stats.Focus.STAT_MAX"),
            self.focus_max,
            &mut changes,
        )?;
        set_number(
            gff,
            &path("Stats.Body"),
            i64::from(self.body),
            false,
            &mut changes,
        )?;
        set_number(
            gff,
            &path("Stats.Mind"),
            i64::from(self.mind),
            false,
            &mut changes,
        )?;
        set_number(
            gff,
            &path("Stats.Spirit"),
            i64::from(self.spirit),
            false,
            &mut changes,
        )?;
        set_number(
            gff,
            &path("Stats.Charm"),
            i64::from(self.charm),
            false,
            &mut changes,
        )?;
        set_number(
            gff,
            &path("Stats.Intuition"),
            i64::from(self.intuition),
            false,
            &mut changes,
        )?;
        set_number(
            gff,
            &path("Stats.Intimidate"),
            i64::from(self.intimidate),
            false,
            &mut changes,
        )?;
        set_number(
            gff,
            &path("Stats.Alignment"),
            i64::from(self.alignment),
            false,
            &mut changes,
        )?;
        set_number(
            gff,
            &path("Stats.Appearance"),
            i64::from(self.appearance),
            false,
            &mut changes,
        )?;
        set_number(
            gff,
            &path("Progress.Kills"),
            i64::from(self.kills),
            false,
            &mut changes,
        )?;
        Ok(changes)
    }

    /// Writes the values that live in the campaign block.
    pub fn write_global(&self, gff: &mut Gff) -> Result<Vec<Change>> {
        let mut changes = Vec::new();
        check_unsigned(
            "chapter",
            u64::from(self.chapter),
            0,
            u64::from(MAX_CHAPTER),
        )?;
        set_number(
            gff,
            "Campaign.Chapter",
            i64::from(self.chapter),
            false,
            &mut changes,
        )?;
        if let Ok(field) = gff.get("Campaign.TimePlayed")
            && let Value::DWord64(before) = field.value
            && before != self.time_played
        {
            gff.set("Campaign.TimePlayed", Value::DWord64(self.time_played))?;
            changes.push(Change {
                field: "Campaign.TimePlayed".to_owned(),
                before: before.to_string(),
                after: self.time_played.to_string(),
            });
        }
        Ok(changes)
    }
}

fn optional_field<'a>(gff: &'a Gff, path: &str) -> Result<Option<&'a Value>> {
    match gff.get(path) {
        Ok(field) => Ok(Some(&field.value)),
        Err(jadeforge_gff::Error::FieldNotFound { .. }) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn unsigned_field(gff: &Gff, path: &str) -> Result<Option<u64>> {
    optional_field(gff, path)?
        .map(|value| {
            value.as_u64().ok_or_else(|| {
                Error::Corrupt(format!("field {path:?} is not a non-negative number"))
            })
        })
        .transpose()
}

fn signed_field(gff: &Gff, path: &str) -> Result<Option<i64>> {
    optional_field(gff, path)?
        .map(|value| {
            value
                .as_i64()
                .ok_or_else(|| Error::Corrupt(format!("field {path:?} is not an integer")))
        })
        .transpose()
}

fn float_field(gff: &Gff, path: &str) -> Result<Option<f32>> {
    optional_field(gff, path)?
        .map(|value| {
            value
                .as_f32()
                .ok_or_else(|| Error::Corrupt(format!("field {path:?} is not a number")))
        })
        .transpose()
}

fn checked_u32(gff: &Gff, path: &str, default: u32) -> Result<u32> {
    match unsigned_field(gff, path)? {
        Some(value) => {
            u32::try_from(value).map_err(|_| narrow_error(path, value, "a 32 bit value"))
        }
        None => Ok(default),
    }
}

fn checked_i32(gff: &Gff, path: &str, default: i32) -> Result<i32> {
    match signed_field(gff, path)? {
        Some(value) => {
            i32::try_from(value).map_err(|_| narrow_error(path, value, "a 32 bit value"))
        }
        None => Ok(default),
    }
}

fn narrow_error(path: &str, value: impl std::fmt::Display, target: &str) -> Error {
    Error::Corrupt(format!(
        "field {path:?} value {value} does not fit in {target}"
    ))
}

fn check_unsigned(name: &str, value: u64, min: u64, max: u64) -> Result<()> {
    if value < min || value > max {
        return Err(Error::OutOfRange {
            path: name.to_owned(),
            value: value.to_string(),
            min: min.to_string(),
            max: max.to_string(),
        });
    }
    Ok(())
}

fn check_signed(name: &str, value: i64, min: i64, max: i64) -> Result<()> {
    if value < min || value > max {
        return Err(Error::OutOfRange {
            path: name.to_owned(),
            value: value.to_string(),
            min: min.to_string(),
            max: max.to_string(),
        });
    }
    Ok(())
}

fn check_name(name: &str) -> Result<()> {
    if name.chars().count() > 64 || name.chars().any(char::is_control) {
        return Err(Error::OutOfRange {
            path: "name".to_owned(),
            value: name.to_owned(),
            min: "printable text".to_owned(),
            max: "64 characters".to_owned(),
        });
    }
    Ok(())
}

fn check_float(name: &str, value: f32) -> Result<()> {
    if !value.is_finite() || value < 0.0 || value > 1_000_000.0 {
        return Err(Error::OutOfRange {
            path: name.to_owned(),
            value: value.to_string(),
            min: "0".to_owned(),
            max: "1000000".to_owned(),
        });
    }
    Ok(())
}

/// Replaces the value of an existing field, keeping its numeric type.
///
/// When the field does not exist and `create` is set, a new `dword` field is
/// added to the parent struct.
fn set_number(
    gff: &mut Gff,
    path: &str,
    value: i64,
    create: bool,
    changes: &mut Vec<Change>,
) -> Result<()> {
    let Ok(field) = gff.get(path) else {
        if create && value != 0 {
            let gold = u32::try_from(value).map_err(|_| out_of_range(path, value))?;
            gff.add(path, Value::DWord(gold))?;
            changes.push(Change {
                field: path.to_owned(),
                before: "<missing>".to_owned(),
                after: value.to_string(),
            });
        }
        return Ok(());
    };
    let before = field.value.as_i64().unwrap_or(0);
    if before == value {
        return Ok(());
    }
    let replacement = match field.type_id {
        0 => Value::Byte(byte_of(path, value)?),
        2 => Value::Word(word_of(path, value)?),
        3 => Value::Short(i16::try_from(value).map_err(|_| out_of_range(path, value))?),
        4 => Value::DWord(u32::try_from(value).map_err(|_| out_of_range(path, value))?),
        5 => Value::Int(i32::try_from(value).map_err(|_| out_of_range(path, value))?),
        6 => Value::DWord64(u64::try_from(value).map_err(|_| out_of_range(path, value))?),
        7 => Value::Int64(value),
        _ => {
            return Err(Error::TypeMismatch {
                path: path.to_owned(),
                expected: "a numeric field",
                actual: field.value.type_name(),
            });
        }
    };
    gff.set(path, replacement)?;
    changes.push(Change {
        field: path.to_owned(),
        before: before.to_string(),
        after: value.to_string(),
    });
    Ok(())
}

fn set_float(gff: &mut Gff, path: &str, value: f32, changes: &mut Vec<Change>) -> Result<()> {
    let Ok(field) = gff.get(path) else {
        return Ok(());
    };
    let before = field.value.as_f32().unwrap_or(0.0);
    if before.to_bits() == value.to_bits() {
        return Ok(());
    }
    if !matches!(field.value, Value::Float(_)) {
        return Err(Error::TypeMismatch {
            path: path.to_owned(),
            expected: "a float field",
            actual: field.value.type_name(),
        });
    }
    gff.set(path, Value::Float(value))?;
    changes.push(Change {
        field: path.to_owned(),
        before: format!("{before}"),
        after: format!("{value}"),
    });
    Ok(())
}

fn set_text(gff: &mut Gff, path: &str, value: &str, changes: &mut Vec<Change>) -> Result<()> {
    if value.chars().count() > 64 || value.chars().any(char::is_control) {
        return Err(Error::OutOfRange {
            path: path.to_owned(),
            value: value.to_owned(),
            min: "printable text".to_owned(),
            max: "64 characters".to_owned(),
        });
    }
    let Ok(field) = gff.get(path) else {
        return Ok(());
    };
    let before = field.value.as_str().unwrap_or_default().to_owned();
    if before == value {
        return Ok(());
    }
    if !matches!(field.value, Value::String(_)) {
        return Err(Error::TypeMismatch {
            path: path.to_owned(),
            expected: "a string field",
            actual: field.value.type_name(),
        });
    }
    gff.set(path, Value::String(value.to_owned()))?;
    changes.push(Change {
        field: path.to_owned(),
        before,
        after: value.to_owned(),
    });
    Ok(())
}

fn byte_of(path: &str, value: i64) -> Result<u8> {
    u8::try_from(value).map_err(|_| out_of_range(path, value))
}

fn word_of(path: &str, value: i64) -> Result<u16> {
    u16::try_from(value).map_err(|_| out_of_range(path, value))
}

fn out_of_range(path: &str, value: i64) -> Error {
    Error::OutOfRange {
        path: path.to_owned(),
        value: value.to_string(),
        min: i64::MIN.to_string(),
        max: i64::MAX.to_string(),
    }
}
