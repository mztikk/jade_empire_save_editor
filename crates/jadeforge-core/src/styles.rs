//! Player fighting styles and their upgrade levels.
//!
//! Every style the player owns is a struct in `PlayerCreature.Stats.Styles`.
//! The struct carries a `StyleId` (row in `stylesuperlist.2da`), a `StyleLevel`
//! and a `StyleLevelInfo` bit mask of the upgrades that were picked. The level
//! is `1 + number of set bits`; the game derives how many style points are left
//! from the levels of all owned styles.

use crate::error::{Error, Result};
use crate::gff::{Gff, Value};
use crate::save::SaveGame;

/// Highest style level `JadeForge` allows to be set.
pub const MAX_STYLE_LEVEL: u8 = 64;

/// One fighting style of the player.
#[derive(Debug, Clone, PartialEq)]
pub struct Style {
    /// Index of the struct that stores the style.
    pub struct_index: usize,
    /// Row in `stylesuperlist.2da`.
    pub style_id: u16,
    /// Level, `1` for a freshly learned style.
    pub level: u8,
    /// Raw `StyleLevelInfo` upgrade mask.
    pub mask: u64,
    /// Talk table reference of the name, as stored in the savegame.
    pub name_ref: Option<u32>,
    /// Talk table reference of the description.
    pub desc_ref: Option<u32>,
    /// Icon resource name.
    pub icon: Option<String>,
    /// Seconds the player spent in this style.
    pub time_spent: f32,
}

impl Style {
    /// Number of upgrades that were picked.
    #[must_use]
    pub fn upgrades(&self) -> u32 {
        self.mask.count_ones()
    }
}

impl SaveGame {
    /// The `Styles` list of the player.
    fn styles_path(&self) -> Result<String> {
        let location = self.player_location()?;
        Ok(format!("{}Stats.Styles", location.prefix))
    }

    /// All fighting styles the player has learned.
    pub fn styles(&self) -> Result<Vec<Style>> {
        let location = self.player_location()?;
        let block = self
            .block(location.block)
            .ok_or_else(|| Error::Corrupt("character block vanished".to_owned()))?;
        let gff = block.gff();
        let path = self.styles_path()?;
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
        let mut styles = Vec::with_capacity(elements.len());
        for element in elements {
            let structure = gff.struct_at(*element).ok_or_else(|| {
                Error::Corrupt(format!(
                    "the style list references the missing struct {element}"
                ))
            })?;
            let field = |name: &str| -> Option<&Value> {
                structure
                    .fields
                    .iter()
                    .find(|field| gff.label_name(field.label) == Some(name))
                    .map(|field| &field.value)
            };
            let mask = match field("StyleLevelInfo") {
                Some(Value::DWord64(mask)) => *mask,
                _ => 0,
            };
            let level = match field("StyleLevel") {
                Some(value) => value.as_u64().unwrap_or(1).clamp(1, 255) as u8,
                None => u8::try_from(mask.count_ones() + 1).unwrap_or(MAX_STYLE_LEVEL),
            };
            styles.push(Style {
                struct_index: *element,
                style_id: field("StyleId")
                    .and_then(Value::as_u64)
                    .map_or(0, |value| value as u16),
                level,
                mask,
                name_ref: field("NameRef")
                    .and_then(Value::as_u64)
                    .map(|value| value as u32),
                desc_ref: field("DescRef")
                    .and_then(Value::as_u64)
                    .map(|value| value as u32),
                icon: field("IconRef")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                time_spent: field("TimeSpentInStyle")
                    .and_then(Value::as_f32)
                    .unwrap_or(0.0),
            });
        }
        Ok(styles)
    }

    /// Changes the level of a style.
    ///
    /// Raising the level sets the lowest unused upgrade bits, lowering it
    /// clears the highest set bits, so the upgrade mask stays consistent with
    /// the level and existing choices are preserved.
    pub fn set_style_level(&mut self, struct_index: usize, level: u8) -> Result<()> {
        if !(1..=MAX_STYLE_LEVEL).contains(&level) {
            return Err(Error::OutOfRange {
                path: "style level".to_owned(),
                value: level.to_string(),
                min: "1".to_owned(),
                max: MAX_STYLE_LEVEL.to_string(),
            });
        }
        let location = self.player_location()?;
        let block = self
            .block_mut(location.block)
            .ok_or_else(|| Error::Corrupt("character block vanished".to_owned()))?;
        let gff = block.gff_mut();
        let structure = gff
            .struct_at(struct_index)
            .ok_or_else(|| Error::Corrupt(format!("struct {struct_index} does not exist")))?;
        let position = |gff: &crate::gff::Gff| -> Option<usize> {
            gff.struct_at(struct_index)?
                .fields
                .iter()
                .position(|field| gff.label_name(field.label) == Some("StyleLevelInfo"))
        };

        let mut mask = match structure
            .fields
            .iter()
            .find(|field| gff.label_name(field.label) == Some("StyleLevelInfo"))
            .map(|field| &field.value)
        {
            Some(Value::DWord64(mask)) => *mask,
            _ => 0,
        };

        let wanted = u32::from(level - 1);
        while mask.count_ones() < wanted {
            let bit = (0..64)
                .find(|bit| mask & (1 << bit) == 0)
                .ok_or_else(|| Error::Corrupt("style has no free upgrade bits".to_owned()))?;
            mask |= 1 << bit;
        }
        while mask.count_ones() > wanted {
            let bit = (0..64)
                .rev()
                .find(|bit| mask & (1 << bit) != 0)
                .ok_or_else(|| Error::Corrupt("style has no upgrade bits to clear".to_owned()))?;
            mask &= !(1 << bit);
        }

        if let Some(position) = position(gff) {
            let field = &mut gff
                .struct_at_mut(struct_index)
                .expect("checked above")
                .fields[position];
            field.value = Value::DWord64(mask);
        } else {
            gff.add_field(struct_index, "StyleLevelInfo", Value::DWord64(mask))?;
        }

        // `StyleLevel` is a byte and has to follow the mask.
        let level_index = gff
            .struct_at(struct_index)
            .expect("checked above")
            .fields
            .iter()
            .position(|field| gff.label_name(field.label) == Some("StyleLevel"));
        if let Some(position) = level_index {
            gff.struct_at_mut(struct_index)
                .expect("checked above")
                .fields[position]
                .value = Value::Byte(level);
        } else {
            gff.add_field(struct_index, "StyleLevel", Value::Byte(level))?;
        }
        Ok(())
    }

    /// Sets or clears one upgrade bit of a style and keeps the level in sync.
    ///
    /// Bit `row - 1` selects `StyleAdvance.2da` row `row`; the level is
    /// `1 + number of set bits`, exactly like the game stores it. Returns the
    /// new mask.
    pub fn set_style_upgrade(&mut self, struct_index: usize, row: u16, taken: bool) -> Result<u64> {
        if !(1..=64).contains(&row) {
            return Err(Error::OutOfRange {
                path: "style upgrade".to_owned(),
                value: row.to_string(),
                min: "1".to_owned(),
                max: "64".to_owned(),
            });
        }
        let location = self.player_location()?;
        let block = self
            .block_mut(location.block)
            .ok_or_else(|| Error::Corrupt("character block vanished".to_owned()))?;
        let gff = block.gff_mut();
        if gff.struct_at(struct_index).is_none() {
            return Err(Error::Corrupt(format!(
                "struct {struct_index} does not exist"
            )));
        }
        let mask = read_style_mask(gff, struct_index);
        let bit = 1_u64 << (row - 1);
        let new_mask = if taken { mask | bit } else { mask & !bit };
        if new_mask == mask {
            return Ok(mask);
        }
        let level = u8::try_from(new_mask.count_ones() + 1).unwrap_or(u8::MAX);
        if level > MAX_STYLE_LEVEL {
            return Err(Error::OutOfRange {
                path: "style level".to_owned(),
                value: level.to_string(),
                min: "1".to_owned(),
                max: MAX_STYLE_LEVEL.to_string(),
            });
        }
        write_style_level(gff, struct_index, new_mask, level)?;
        Ok(new_mask)
    }

    /// Adds a style by cloning the field layout of an existing one.
    ///
    /// The identity of the style (`StyleId`, `NameRef`, `DescRef`, `IconRef`,
    /// weapons) is taken from the caller, everything else from `template`,
    /// which should be a style the player already owns. Combat multipliers are
    /// therefore inherited from the template; the game recomputes most of them
    /// from the style tables anyway.
    pub fn add_style(&mut self, template: usize, style_id: u16, name_ref: u32) -> Result<usize> {
        let location = self.player_location()?;
        let list_path = self.styles_path()?;
        let block = self
            .block_mut(location.block)
            .ok_or_else(|| Error::Corrupt("character block vanished".to_owned()))?;
        let gff = block.gff_mut();
        let source = gff
            .struct_at(template)
            .ok_or_else(|| Error::Corrupt(format!("struct {template} does not exist")))?
            .clone();
        let index = gff.push_struct(source.type_id);
        for field in source.fields {
            let name = gff.label_name(field.label).unwrap_or_default().to_owned();
            let value = match name.as_str() {
                "StyleId" => Value::DWord(u32::from(style_id)),
                "NameRef" => Value::DWord(name_ref),
                "StyleLevel" => Value::Byte(1),
                "StyleLevelInfo" => Value::DWord64(0),
                "TimeSpentInStyle" => Value::Float(0.0),
                "FirstLoad" => Value::Byte(0),
                _ => field.value,
            };
            gff.add_field(index, &name, value)?;
        }
        let field = gff.get_mut(&list_path)?;
        let actual = field.value.type_name();
        let elements = field.value.as_list_mut().ok_or(Error::TypeMismatch {
            path: list_path,
            expected: "a list",
            actual,
        })?;
        elements.push(index);
        Ok(index)
    }

    /// Removes a style from the player, leaving the struct behind as an orphan.
    pub fn remove_style(&mut self, struct_index: usize) -> Result<bool> {
        let path = self.styles_path()?;
        let location = self.player_location()?;
        let block = self
            .block_mut(location.block)
            .ok_or_else(|| Error::Corrupt("character block vanished".to_owned()))?;
        let gff = block.gff_mut();
        let field = gff.get_mut(&path)?;
        let actual = field.value.type_name();
        let elements = field.value.as_list_mut().ok_or(Error::TypeMismatch {
            path,
            expected: "a list",
            actual,
        })?;
        let before = elements.len();
        elements.retain(|element| *element != struct_index);
        Ok(elements.len() != before)
    }
}

/// Reads the upgrade mask of a style struct, treating a missing field as zero.
fn read_style_mask(gff: &Gff, struct_index: usize) -> u64 {
    gff.struct_at(struct_index)
        .and_then(|structure| {
            structure
                .fields
                .iter()
                .find(|field| gff.label_name(field.label) == Some("StyleLevelInfo"))
        })
        .and_then(|field| match field.value {
            Value::DWord64(mask) => Some(mask),
            _ => None,
        })
        .unwrap_or(0)
}

/// Writes the mask and the level field of a style struct.
fn write_style_level(gff: &mut Gff, struct_index: usize, mask: u64, level: u8) -> Result<()> {
    let mask_position = gff.struct_at(struct_index).and_then(|structure| {
        structure
            .fields
            .iter()
            .position(|field| gff.label_name(field.label) == Some("StyleLevelInfo"))
    });
    if let Some(position) = mask_position {
        gff.struct_at_mut(struct_index)
            .expect("struct checked by the caller")
            .fields[position]
            .value = Value::DWord64(mask);
    } else {
        gff.add_field(struct_index, "StyleLevelInfo", Value::DWord64(mask))?;
    }
    let level_position = gff.struct_at(struct_index).and_then(|structure| {
        structure
            .fields
            .iter()
            .position(|field| gff.label_name(field.label) == Some("StyleLevel"))
    });
    if let Some(position) = level_position {
        gff.struct_at_mut(struct_index)
            .expect("struct checked by the caller")
            .fields[position]
            .value = Value::Byte(level);
    } else {
        gff.add_field(struct_index, "StyleLevel", Value::Byte(level))?;
    }
    Ok(())
}
