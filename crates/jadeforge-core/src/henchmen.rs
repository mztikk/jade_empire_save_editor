//! Henchmen (party members) stored in the player creature.
//!
//! Every henchman is a struct in `PlayerCreature.Henchmen.HenchList`. Besides
//! their level, the list controls whether a companion is available and how the
//! AI should behave in combat.

use crate::error::{Error, Result};
use crate::gff::Value;
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
            henchmen.push(Henchman {
                struct_index: *element,
                resref: value("BaseResref")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                level: value("Level")
                    .and_then(Value::as_u64)
                    .map_or(1, |level| level.clamp(1, 255) as u8),
                available: flag(value("Available")),
                available2: flag(value("Available2")),
                state: value("State")
                    .and_then(Value::as_u64)
                    .map_or(0, |state| state as u8),
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
        let flag = u64::from(available);
        self.write_henchman_number(struct_index, "Available", flag)?;
        self.write_henchman_number(struct_index, "Available2", flag)
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

    /// Writes a number into a henchman field, keeping the field's own type.
    fn write_henchman_number(
        &mut self,
        struct_index: usize,
        label: &str,
        value: u64,
    ) -> Result<()> {
        let location = self.player_location()?;
        let block = self
            .block_mut(location.block)
            .ok_or_else(|| Error::Corrupt("character block vanished".to_owned()))?;
        let gff = block.gff_mut();
        let label_index = gff.intern_label(label)?;
        let existing = gff
            .struct_at(struct_index)
            .ok_or_else(|| Error::Corrupt(format!("struct {struct_index} does not exist")))?
            .fields
            .iter()
            .find(|field| field.label == label_index)
            .map(|field| field.type_id);
        let replacement = match existing {
            Some(0) => Value::Byte(u8::try_from(value).map_err(|_| out_of_range(label))?),
            Some(2) => Value::Word(u16::try_from(value).map_err(|_| out_of_range(label))?),
            Some(3) => Value::Short(i16::try_from(value).map_err(|_| out_of_range(label))?),
            Some(5) => Value::Int(i32::try_from(value).map_err(|_| out_of_range(label))?),
            Some(6) => Value::DWord64(value),
            _ => Value::DWord(u32::try_from(value).map_err(|_| out_of_range(label))?),
        };
        if existing.is_some() {
            let structure = gff.struct_at_mut(struct_index).expect("checked above");
            if let Some(field) = structure
                .fields
                .iter_mut()
                .find(|field| field.label == label_index)
            {
                field.value = replacement;
            }
        } else {
            gff.add_field(struct_index, label, replacement)?;
        }
        Ok(())
    }
}

fn flag(value: Option<&Value>) -> bool {
    value.and_then(Value::as_u64).is_some_and(|flag| flag != 0)
}

fn out_of_range(label: &str) -> Error {
    Error::OutOfRange {
        path: format!("henchman {label}"),
        value: "out of range".to_owned(),
        min: "0".to_owned(),
        max: u32::MAX.to_string(),
    }
}
