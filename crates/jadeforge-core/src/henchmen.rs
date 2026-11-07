//! Henchmen (party members) stored in the player creature.
//!
//! Every henchman is a struct in `PlayerCreature.Henchmen.HenchList`. Besides
//! their level, the list controls whether a companion is available and how the
//! AI should behave in combat.

use crate::error::{Error, Result};
use crate::gff::{Gff, Value};
use crate::save::SaveGame;

/// One companion of the player.
#[derive(Debug, Clone, PartialEq)]
pub struct Henchman {
    /// Index of the struct that stores the henchman.
    pub struct_index: usize,
    /// Creature template, for example `j00h_ho`.
    pub resref: String,
    /// Companion level.
    pub level: u8,
    /// Whether the companion can be selected.
    pub available: bool,
    /// Second availability flag used by the game.
    pub available2: bool,
    /// Internal state of the companion.
    pub state: u8,
    /// Seconds the companion fought for the player.
    pub combat_time: f32,
}

impl SaveGame {
    fn henchmen_path(&self) -> Result<String> {
        let location = self.player_location()?;
        Ok(format!("{}Henchmen.HenchList", location.prefix))
    }

    /// All companions of the player.
    pub fn henchmen(&self) -> Result<Vec<Henchman>> {
        let location = self.player_location()?;
        let block = self
            .block(location.block)
            .ok_or_else(|| Error::Corrupt("character block vanished".to_owned()))?;
        let gff = block.gff();
        let path = self.henchmen_path()?;
        let Ok(field) = gff.get(&path) else {
            return Ok(Vec::new());
        };
        let Some(elements) = field.value.as_list() else {
            return Err(Error::TypeMismatch {
                path,
                expected: "a list",
                actual: field.value.type_name(),
            });
        };
        let mut henchmen = Vec::with_capacity(elements.len());
        for element in elements {
            let structure = gff.struct_at(*element).ok_or_else(|| {
                Error::Corrupt(format!(
                    "the henchman list references the missing struct {element}"
                ))
            })?;
            let value = |name: &str| -> Option<&Value> {
                structure
                    .fields
                    .iter()
                    .find(|field| gff.label_name(field.label) == Some(name))
                    .map(|field| &field.value)
            };
            let level = checked_u8(value("Level"), &format!("henchman {element} Level"), 1, 1)?;
            let state = checked_u8(value("State"), &format!("henchman {element} State"), 0, 0)?;
            henchmen.push(Henchman {
                struct_index: *element,
                resref: value("BaseResref")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                level,
                available: flag(value("Available")),
                available2: flag(value("Available2")),
                state,
                combat_time: value("CombatTime").and_then(Value::as_f32).unwrap_or(0.0),
            });
        }
        Ok(henchmen)
    }

    /// Sets the level of a companion.
    pub fn set_henchman_level(&mut self, struct_index: usize, level: u8) -> Result<()> {
        if !(1..=60).contains(&level) {
            return Err(Error::OutOfRange {
                path: "henchman level".to_owned(),
                value: level.to_string(),
                min: "1".to_owned(),
                max: "60".to_owned(),
            });
        }
        self.write_henchman_number(struct_index, "Level", u64::from(level))
    }

    /// Makes a companion available or unavailable.
    pub fn set_henchman_available(&mut self, struct_index: usize, available: bool) -> Result<()> {
        let list_path = self.henchmen_path()?;
        let flag = u64::from(available);
        let changed = self.edit_player_gff(|gff, _prefix| {
            crate::save::require_list_member(gff, &list_path, struct_index)?;
            let first = write_henchman_number(gff, struct_index, "Available", flag)?;
            let second = write_henchman_number(gff, struct_index, "Available2", flag)?;
            if first || second {
                Ok(Some(()))
            } else {
                Ok(None)
            }
        })?;
        let _ = changed;
        Ok(())
    }

    /// Sets the combat behaviour of a companion.
    pub fn set_henchman_combat_mode(&mut self, struct_index: usize, mode: u8) -> Result<()> {
        if mode > 2 {
            return Err(Error::OutOfRange {
                path: "combat mode".to_owned(),
                value: mode.to_string(),
                min: "0".to_owned(),
                max: "2".to_owned(),
            });
        }
        self.write_henchman_number(struct_index, "CombatMode", u64::from(mode))
    }

    fn write_henchman_number(
        &mut self,
        struct_index: usize,
        label: &str,
        value: u64,
    ) -> Result<()> {
        let list_path = self.henchmen_path()?;
        let changed = self.edit_player_gff(|gff, _prefix| {
            crate::save::require_list_member(gff, &list_path, struct_index)?;
            if write_henchman_number(gff, struct_index, label, value)? {
                Ok(Some(()))
            } else {
                Ok(None)
            }
        })?;
        let _ = changed;
        Ok(())
    }
}

/// Writes a number into one henchman field, keeping its type tag.
///
/// Returns `true` when the value actually changed. The caller runs on a working
/// copy, so a failure cannot dirty the savegame or leave a label behind.
fn write_henchman_number(
    gff: &mut Gff,
    struct_index: usize,
    label: &str,
    value: u64,
) -> Result<bool> {
    let structure = gff
        .struct_at(struct_index)
        .ok_or_else(|| Error::Corrupt(format!("struct {struct_index} does not exist")))?;
    let position = structure
        .fields
        .iter()
        .position(|field| gff.label_name(field.label) == Some(label));
    if let Some(position) = position {
        let field = &structure.fields[position];
        let current = field.value.as_u64().ok_or_else(|| Error::TypeMismatch {
            path: format!("henchman {struct_index} {label}"),
            expected: "a non-negative number",
            actual: field.value.type_name(),
        })?;
        if current == value {
            return Ok(false);
        }
        let replacement = number_value(field.type_id, value).map_err(|_| Error::TypeMismatch {
            path: format!("henchman {struct_index} {label}"),
            expected: "a numeric field type",
            actual: field.value.type_name(),
        })?;
        gff.struct_at_mut(struct_index)
            .and_then(|structure| structure.fields.get_mut(position))
            .ok_or_else(|| Error::Corrupt(format!("struct {struct_index} does not exist")))?
            .value = replacement;
        Ok(true)
    } else {
        let value = u32::try_from(value).map_err(|_| Error::OutOfRange {
            path: format!("henchman {struct_index} {label}"),
            value: value.to_string(),
            min: "0".to_owned(),
            max: u32::MAX.to_string(),
        })?;
        gff.add_field(struct_index, label, Value::DWord(value))?;
        Ok(true)
    }
}

fn number_value(type_id: u32, value: u64) -> Result<Value> {
    Ok(match type_id {
        0 => Value::Byte(u8::try_from(value).map_err(|_| unsupported(type_id))?),
        1 => Value::Char(i8::try_from(value).map_err(|_| unsupported(type_id))?),
        2 => Value::Word(u16::try_from(value).map_err(|_| unsupported(type_id))?),
        3 => Value::Short(i16::try_from(value).map_err(|_| unsupported(type_id))?),
        4 => Value::DWord(u32::try_from(value).map_err(|_| unsupported(type_id))?),
        5 => Value::Int(i32::try_from(value).map_err(|_| unsupported(type_id))?),
        6 => Value::DWord64(value),
        7 => Value::Int64(i64::try_from(value).map_err(|_| unsupported(type_id))?),
        _ => return Err(unsupported(type_id)),
    })
}

fn unsupported(type_id: u32) -> Error {
    Error::TypeMismatch {
        path: "henchman field".to_owned(),
        expected: "a numeric field type",
        actual: match type_id {
            8 => "float",
            9 => "double",
            10 => "string",
            _ => "non-numeric",
        },
    }
}

fn checked_u8(value: Option<&Value>, path: &str, default: u8, min: u8) -> Result<u8> {
    match value {
        Some(value) => {
            let number = value
                .as_u64()
                .ok_or_else(|| Error::Corrupt(format!("field {path:?} is not a number")))?;
            let number = u8::try_from(number)
                .map_err(|_| Error::Corrupt(format!("field {path:?} is out of range")))?;
            if number < min {
                return Err(Error::Corrupt(format!("field {path:?} is out of range")));
            }
            Ok(number)
        }
        None => Ok(default),
    }
}

fn flag(value: Option<&Value>) -> bool {
    value.and_then(Value::as_u64).is_some_and(|flag| flag != 0)
}
