//! Typed view of the player character.
//!
//! The player creature lives in the `save.sav` block below `PlayerCreature`.
//! A few older releases ship a `polyplayer.sav` block that keeps a character
//! sheet at the root; when `save.sav` has no player the editor falls back to
//! that block.

use crate::error::{Error, Result};
use crate::gff::{Field, Gff, Value};
use crate::save::{GLOBAL_BLOCK, POLYPLAYER_BLOCK, SaveGame};

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
    /// Level, stored as a byte.
    pub level: u8,
    /// Current experience points.
    pub experience: u32,
    /// Gold, when the savegame has a gold field.
    pub gold: Option<u32>,
    /// Unspent attribute points.
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
    /// Alignment.
    pub alignment: i32,
    /// Appearance index.
    pub appearance: i32,
    /// Story chapter.
    pub chapter: u32,
    /// Total play time in microseconds.
    pub time_played: u64,
    /// Number of kills.
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
    if let Ok(block) = save.global_block() {
        if player_at(block.gff(), "PlayerCreature.").is_some() {
            return Some(PlayerLocation {
                block: GLOBAL_BLOCK,
                prefix: "PlayerCreature.",
            });
        }
    }
    if let Some(block) = save.block(POLYPLAYER_BLOCK) {
        if player_at(block.gff(), "").is_some() {
            return Some(PlayerLocation {
                block: POLYPLAYER_BLOCK,
                prefix: "",
            });
        }
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
        if location.block != GLOBAL_BLOCK {
            if let Ok(global) = self.global_block() {
                character.read_global(global.gff());
            }
        }
        Ok(character)
    }

    /// Writes the editable character data and returns the fields that changed.
    ///
    /// Only values that differ from the savegame are touched, so an untouched
    /// character produces no changes at all.
    pub fn apply_character(&mut self, character: &Character) -> Result<Vec<Change>> {
        let location = self.player_location()?;
        let mut changes = {
            let block = self
                .block_mut(location.block)
                .ok_or_else(|| Error::Corrupt("character block vanished".to_owned()))?;
            // Edit a copy first: if any value is rejected the savegame must stay
            // untouched instead of keeping half of the new values.
            let mut working = block.gff().clone();
            let changes = character.write(&mut working, location.prefix)?;
            if !changes.is_empty() {
                *block.gff_mut() = working;
            }
            changes
        };
        if let Some(block) = self.block_mut(GLOBAL_BLOCK) {
            let was_modified = block.is_modified();
            let global_changes = character.write_global(block.gff_mut())?;
            if global_changes.is_empty() && !was_modified {
                block.mark_clean();
            }
            changes.extend(global_changes);
        }
        Ok(changes)
    }
}

impl Character {
    /// Reads the character from a GFF block.
    pub fn read(gff: &Gff, prefix: &str) -> Result<Self> {
        let path = |suffix: &str| format!("{prefix}{suffix}");
        let read = |suffix: &str| -> Option<&Field> { gff.get(path(suffix)).ok() };

        let number =
            |suffix: &str| -> Option<u64> { read(suffix).and_then(|field| field.value.as_u64()) };
        let float =
            |suffix: &str| -> Option<f32> { read(suffix).and_then(|field| field.value.as_f32()) };
        let signed =
            |suffix: &str| -> Option<i64> { read(suffix).and_then(|field| field.value.as_i64()) };

        let health = float("Stats.Health").unwrap_or(0.0);
        let chi = float("Stats.Chi").unwrap_or(0.0);
        let focus = float("Stats.Focus.STAT_CUR").unwrap_or(0.0);

        Ok(Self {
            name: read("FirstName")
                .and_then(|field| field.value.as_str())
                .unwrap_or_default()
                .to_owned(),
            level: number("Level").map_or(1, |value| u8::try_from(value).unwrap_or(1)),
            experience: number("CurrentXP").map_or(0, |value| u32::try_from(value).unwrap_or(0)),
            gold: number("Stats.Gold").map(|value| u32::try_from(value).unwrap_or(u32::MAX)),
            attribute_points: number("Stats.UnassignedPts")
                .map_or(0, |value| u32::try_from(value).unwrap_or(0)),
            health,
            health_max: number("Stats.HealthBase")
                .map_or(health.max(0.0) as u32, |value| value as u32),
            chi,
            chi_max: number("Stats.ChiBase").map_or(chi.max(0.0) as u32, |value| value as u32),
            focus,
            focus_max: float("Stats.Focus.STAT_MAX").unwrap_or(focus),
            body: number("Stats.Body").unwrap_or(0) as u32,
            mind: number("Stats.Mind").unwrap_or(0) as u32,
            spirit: number("Stats.Spirit").unwrap_or(0) as u32,
            charm: signed("Stats.Charm").unwrap_or(0) as i32,
            intuition: signed("Stats.Intuition").unwrap_or(0) as i32,
            intimidate: signed("Stats.Intimidate").unwrap_or(0) as i32,
            alignment: signed("Stats.Alignment").unwrap_or(0) as i32,
            appearance: signed("Stats.Appearance").unwrap_or(0) as i32,
            chapter: gff
                .get("Campaign.Chapter")
                .ok()
                .and_then(|field| field.value.as_u64())
                .unwrap_or(0) as u32,
            time_played: gff
                .get("Campaign.TimePlayed")
                .ok()
                .and_then(|field| field.value.as_u64())
                .unwrap_or(0),
            kills: number("Progress.Kills").unwrap_or(0) as u32,
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

        check_unsigned("level", u64::from(self.level), 1, 255)?;
        check_unsigned(
            "experience",
            u64::from(self.experience),
            0,
            u64::from(u32::MAX),
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
            -1_000_000,
            1_000_000,
        )?;
        check_signed("appearance", i64::from(self.appearance), 0, 1_000_000)?;
        check_unsigned("kills", u64::from(self.kills), 0, u64::from(u32::MAX))?;
        check_unsigned("chapter", u64::from(self.chapter), 0, 100)?;
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
                false,
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
        check_unsigned("chapter", u64::from(self.chapter), 0, 100)?;
        set_number(
            gff,
            "Campaign.Chapter",
            i64::from(self.chapter),
            false,
            &mut changes,
        )?;
        if let Ok(field) = gff.get("Campaign.TimePlayed") {
            if let Value::DWord64(before) = field.value {
                if before != self.time_played {
                    gff.set("Campaign.TimePlayed", Value::DWord64(self.time_played))?;
                    changes.push(Change {
                        field: "Campaign.TimePlayed".to_owned(),
                        before: before.to_string(),
                        after: self.time_played.to_string(),
                    });
                }
            }
        }
        Ok(changes)
    }
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
    if name.len() > 64 || name.chars().any(char::is_control) {
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
            gff.add(path, Value::DWord(value as u32))?;
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
        6 => Value::DWord64(value as u64),
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
    if value.len() > 64 || value.chars().any(char::is_control) {
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
