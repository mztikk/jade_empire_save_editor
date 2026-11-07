//! Player fighting styles and their upgrade levels.
//!
//! Every style the player owns is a struct in `PlayerCreature.Stats.Styles`.
//! The struct carries a `StyleId` (row in `stylesuperlist.2da`), a `StyleLevel`
//! and a `StyleLevelInfo` bit mask of the upgrades that were picked. The level
//! is always derived: `1 + number of set bits` in the mask. The game derives how
//! many style points are left from the levels of all owned styles.

use std::collections::HashSet;

use crate::error::{Error, Result};
use crate::gff::{Gff, Value};
use crate::save::SaveGame;

/// Highest style level `JadeForge` allows to be set.
///
/// The level is `1 + popcount(StyleLevelInfo)`, so a 64 bit mask can describe
/// level 65. `JadeForge` caps styles at level 64, which means at most 63 mask bits
/// may be set; buying the 64th upgrade is rejected by
/// [`SaveGame::set_style_family_slot`].
pub const MAX_STYLE_LEVEL: u8 = 64;

/// Identity of a style that is being learned.
///
/// `add_style` clones the field layout of an existing style (so the new struct
/// has the same fields and type tags) and replaces the identity fields with the
/// values from the game data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleIdentity {
    /// Row in `stylesuperlist.2da`.
    pub style_id: u16,
    /// Talk table reference of the name.
    pub name_ref: u32,
    /// Talk table reference of the description.
    pub desc_ref: Option<u32>,
    /// Icon resource name.
    pub icon: Option<String>,
    /// Weapon resource name.
    pub weapon: Option<String>,
    /// Number of weapons the style carries.
    pub weapon_count: u8,
}

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
    /// Weapon resource name, when the style carries a weapon.
    pub weapon: Option<String>,
    /// Number of weapons the style carries (`WeaponCount`).
    pub weapon_count: u8,
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
                Some(value) => value
                    .as_u64()
                    .map_or(1, |level| u8::try_from(level).unwrap_or(1).clamp(1, 255)),
                None => u8::try_from(mask.count_ones() + 1).unwrap_or(MAX_STYLE_LEVEL),
            };
            styles.push(Style {
                struct_index: *element,
                style_id: field("StyleId")
                    .and_then(Value::as_u64)
                    .and_then(|value| u16::try_from(value).ok())
                    .unwrap_or(0),
                level,
                mask,
                name_ref: field("NameRef")
                    .and_then(Value::as_u64)
                    .and_then(|value| u32::try_from(value).ok()),
                desc_ref: field("DescRef")
                    .and_then(Value::as_u64)
                    .and_then(|value| u32::try_from(value).ok()),
                icon: field("IconRef")
                    .and_then(Value::as_str)
                    .filter(|icon| !icon.is_empty())
                    .map(ToOwned::to_owned),
                weapon: field("WeaponRef")
                    .and_then(Value::as_str)
                    .filter(|weapon| !weapon.is_empty())
                    .map(ToOwned::to_owned),
                weapon_count: field("WeaponCount")
                    .and_then(Value::as_u64)
                    .and_then(|count| u8::try_from(count).ok())
                    .unwrap_or(0),
                time_spent: field("TimeSpentInStyle")
                    .and_then(Value::as_f32)
                    .unwrap_or(0.0),
            });
        }
        Ok(styles)
    }

    /// Sets one upgrade family to the state implied by clicking `clicked`.
    ///
    /// `family_slots` is the ordered list of slots that belong to the family,
    /// as resolved from the style's `<Style>LV.2da` table. Clicking an unowned
    /// slot buys every missing slot up to it; clicking an owned slot clears it
    /// and every slot above it. The mask and level are always written together
    /// and the operation is all-or-nothing. Returns the number of bits that
    /// actually changed.
    pub fn set_style_family_slot(
        &mut self,
        struct_index: usize,
        family_slots: &[u16],
        clicked: u16,
    ) -> Result<u32> {
        if family_slots.is_empty() {
            return Err(Error::Corrupt(
                "a style upgrade family needs at least one slot".to_owned(),
            ));
        }
        if !(1..=64).contains(&clicked) {
            return Err(Error::OutOfRange {
                path: "style upgrade slot".to_owned(),
                value: clicked.to_string(),
                min: "1".to_owned(),
                max: "64".to_owned(),
            });
        }
        let mut slots = family_slots.to_vec();
        slots.sort_unstable();
        slots.dedup();
        if slots.len() != family_slots.len() {
            return Err(Error::Corrupt(
                "a style upgrade family contains duplicate slots".to_owned(),
            ));
        }
        if !slots.contains(&clicked) {
            return Err(Error::Corrupt(format!(
                "slot {clicked} is not part of this upgrade family"
            )));
        }
        if slots.iter().any(|slot| !(1..=64).contains(slot)) {
            return Err(Error::OutOfRange {
                path: "style upgrade slot".to_owned(),
                value: family_slots
                    .iter()
                    .find(|slot| !(1..=64).contains(*slot))
                    .copied()
                    .unwrap_or(0)
                    .to_string(),
                min: "1".to_owned(),
                max: "64".to_owned(),
            });
        }

        let list_path = self.styles_path()?;
        let changed = self.edit_player_gff(|gff, _prefix| {
            crate::save::require_list_member(gff, &list_path, struct_index)?;
            let mask = read_style_mask(gff, struct_index)?;
            ensure_family_prefix(mask, &slots)?;
            let taken = mask & bit(clicked) != 0;
            let mut next = mask;
            for slot in &slots {
                let target = if taken {
                    *slot >= clicked
                } else {
                    *slot <= clicked
                };
                if target {
                    if taken {
                        next &= !bit(*slot);
                    } else {
                        next |= bit(*slot);
                    }
                }
            }
            if next == mask {
                return Ok(None);
            }
            let level = u8::try_from(next.count_ones() + 1).map_err(|_| Error::OutOfRange {
                path: "style level".to_owned(),
                value: next.count_ones().to_string(),
                min: "1".to_owned(),
                max: MAX_STYLE_LEVEL.to_string(),
            })?;
            if level > MAX_STYLE_LEVEL {
                return Err(Error::OutOfRange {
                    path: "style level".to_owned(),
                    value: level.to_string(),
                    min: "1".to_owned(),
                    max: MAX_STYLE_LEVEL.to_string(),
                });
            }
            write_style_mask_and_level(gff, struct_index, next, level)?;
            Ok(Some((next ^ mask).count_ones()))
        })?;
        Ok(changed.unwrap_or(0))
    }

    /// Adds a style by cloning the field layout of an existing one.
    ///
    /// The identity fields (`StyleId`, `NameRef`, `DescRef`, `IconRef`,
    /// `WeaponRef`, `WeaponCount`) are taken from `identity`. Every other field,
    /// including its type tag, is copied from `template`; combat multipliers are
    /// recalculated by the game from the style tables.
    pub fn add_style(&mut self, template: usize, identity: &StyleIdentity) -> Result<usize> {
        let list_path = self.styles_path()?;
        let index = self.edit_player_gff(|gff, _prefix| {
            let list = gff.get(&list_path)?;
            let list_value = list.value.as_list().ok_or_else(|| Error::TypeMismatch {
                path: list_path.clone(),
                expected: "a list",
                actual: list.value.type_name(),
            })?;
            if !list_value.contains(&template) {
                return Err(Error::Corrupt(format!(
                    "style template {template} is not in the player's style list"
                )));
            }
            let source = gff
                .struct_at(template)
                .ok_or_else(|| Error::Corrupt(format!("struct {template} does not exist")))?
                .clone();
            let index = gff.push_struct(source.type_id);
            let mut seen_identity = HashSet::new();
            for field in source.fields {
                let name = gff.label_name(field.label).unwrap_or_default().to_owned();
                let value = match name.as_str() {
                    "StyleId" => {
                        seen_identity.insert(name.clone());
                        number_value(field.type_id, u64::from(identity.style_id))?
                    }
                    "NameRef" => {
                        seen_identity.insert(name.clone());
                        number_value(field.type_id, u64::from(identity.name_ref))?
                    }
                    "DescRef" => {
                        seen_identity.insert(name.clone());
                        number_value(field.type_id, u64::from(identity.desc_ref.unwrap_or(0)))?
                    }
                    "WeaponCount" => {
                        seen_identity.insert(name.clone());
                        number_value(field.type_id, u64::from(identity.weapon_count))?
                    }
                    "FirstLoad" | "StyleLevelInfo" => number_value(field.type_id, 0)?,
                    "StyleLevel" => number_value(field.type_id, 1)?,
                    "IconRef" => {
                        seen_identity.insert(name.clone());
                        Value::String(identity.icon.clone().unwrap_or_default())
                    }
                    "WeaponRef" => {
                        seen_identity.insert(name.clone());
                        Value::String(identity.weapon.clone().unwrap_or_default())
                    }
                    "VisualEffectNode" => Value::String(String::new()),
                    // A learned style starts without inherited resistances and
                    // without references into the template's structs.
                    "ResistList" => Value::List(Vec::new()),
                    "TimeSpentInStyle" => float_value(field.type_id, 0.0)?,
                    _ => field.value,
                };
                gff.add_field_typed(index, &name, field.type_id, value)?;
            }
            add_missing_style_identity(gff, index, identity, &seen_identity)?;

            let list = gff.get_mut(&list_path)?;
            let actual = list.value.type_name();
            let list = list
                .value
                .as_list_mut()
                .ok_or_else(|| Error::TypeMismatch {
                    path: list_path.clone(),
                    expected: "a list",
                    actual,
                })?;
            list.push(index);
            Ok(Some(index))
        })?;
        index.ok_or_else(|| Error::Corrupt("the style was not added".to_owned()))
    }

    /// Removes a style from the player, leaving the struct behind as an orphan.
    pub fn remove_style(&mut self, struct_index: usize) -> Result<bool> {
        let path = self.styles_path()?;
        let removed = self.edit_player_gff(|gff, _prefix| {
            let list = gff.get_mut(&path)?;
            let actual = list.value.type_name();
            let elements = list
                .value
                .as_list_mut()
                .ok_or_else(|| Error::TypeMismatch {
                    path: path.clone(),
                    expected: "a list",
                    actual,
                })?;
            let before = elements.len();
            elements.retain(|element| *element != struct_index);
            if elements.len() == before {
                Ok(None)
            } else {
                Ok(Some(true))
            }
        })?;
        Ok(removed.unwrap_or(false))
    }
}

/// Adds identity fields that a sparse template did not carry.
fn add_missing_style_identity(
    gff: &mut Gff,
    index: usize,
    identity: &StyleIdentity,
    seen: &HashSet<String>,
) -> Result<()> {
    let name_ref = i32::try_from(identity.name_ref)
        .map_err(|_| Error::Corrupt("style name reference does not fit an int field".to_owned()))?;
    let desc_ref = i32::try_from(identity.desc_ref.unwrap_or(0)).map_err(|_| {
        Error::Corrupt("style description reference does not fit an int field".to_owned())
    })?;
    let fields = [
        ("StyleId", 4, Value::DWord(u32::from(identity.style_id))),
        ("NameRef", 5, Value::Int(name_ref)),
        ("DescRef", 5, Value::Int(desc_ref)),
        (
            "IconRef",
            10,
            Value::String(identity.icon.clone().unwrap_or_default()),
        ),
        (
            "WeaponRef",
            10,
            Value::String(identity.weapon.clone().unwrap_or_default()),
        ),
        (
            "WeaponCount",
            5,
            Value::Int(i32::from(identity.weapon_count)),
        ),
    ];
    for (label, type_id, value) in fields {
        if !seen.contains(label) {
            gff.add_field_typed(index, label, type_id, value)?;
        }
    }
    Ok(())
}

fn bit(slot: u16) -> u64 {
    1_u64 << (slot - 1)
}

fn read_style_mask(gff: &Gff, struct_index: usize) -> Result<u64> {
    let structure = gff
        .struct_at(struct_index)
        .ok_or_else(|| Error::Corrupt(format!("style struct {struct_index} does not exist")))?;
    let Some(field) = structure
        .fields
        .iter()
        .find(|field| gff.label_name(field.label) == Some("StyleLevelInfo"))
    else {
        return Ok(0);
    };
    match &field.value {
        Value::DWord64(mask) if field.type_id == 6 => Ok(*mask),
        _ => Err(Error::TypeMismatch {
            path: "StyleLevelInfo".to_owned(),
            expected: "a dword64 upgrade mask",
            actual: field.value.type_name(),
        }),
    }
}

fn ensure_family_prefix(mask: u64, slots: &[u16]) -> Result<()> {
    let mut saw_empty = false;
    for slot in slots {
        if mask & bit(*slot) == 0 {
            saw_empty = true;
        } else if saw_empty {
            return Err(Error::Corrupt(
                "style upgrade family has a gap; repair it before changing it".to_owned(),
            ));
        }
    }
    Ok(())
}

fn write_style_mask_and_level(
    gff: &mut Gff,
    struct_index: usize,
    mask: u64,
    level: u8,
) -> Result<()> {
    let structure = gff
        .struct_at(struct_index)
        .ok_or_else(|| Error::Corrupt(format!("style struct {struct_index} does not exist")))?;
    let mask_position = structure
        .fields
        .iter()
        .position(|field| gff.label_name(field.label) == Some("StyleLevelInfo"));
    let level_field = structure
        .fields
        .iter()
        .find(|field| gff.label_name(field.label) == Some("StyleLevel"))
        .map(|field| (field.type_id, field.value.type_name()));
    if let Some(position) = mask_position {
        if structure.fields[position].type_id != 6 {
            return Err(Error::TypeMismatch {
                path: "StyleLevelInfo".to_owned(),
                expected: "a dword64 upgrade mask",
                actual: structure.fields[position].value.type_name(),
            });
        }
        gff.struct_at_mut(struct_index)
            .ok_or_else(|| Error::Corrupt(format!("style struct {struct_index} does not exist")))?
            .fields[position]
            .value = Value::DWord64(mask);
    } else {
        gff.add_field(struct_index, "StyleLevelInfo", Value::DWord64(mask))?;
    }

    let value = match level_field {
        Some((type_id, actual)) => {
            number_value(type_id, u64::from(level)).map_err(|_| Error::TypeMismatch {
                path: "StyleLevel".to_owned(),
                expected: "a numeric field",
                actual,
            })?
        }
        None => Value::Byte(level),
    };
    let position = gff
        .struct_at(struct_index)
        .ok_or_else(|| Error::Corrupt(format!("style struct {struct_index} does not exist")))?
        .fields
        .iter()
        .position(|field| gff.label_name(field.label) == Some("StyleLevel"));
    if let Some(position) = position {
        gff.struct_at_mut(struct_index)
            .ok_or_else(|| Error::Corrupt(format!("style struct {struct_index} does not exist")))?
            .fields[position]
            .value = value;
    } else {
        gff.add_field(struct_index, "StyleLevel", Value::Byte(level))?;
    }
    Ok(())
}

fn number_value(type_id: u32, value: u64) -> Result<Value> {
    Ok(match type_id {
        0 => Value::Byte(u8::try_from(value).map_err(|_| type_error(type_id))?),
        1 => Value::Char(i8::try_from(value).map_err(|_| type_error(type_id))?),
        2 => Value::Word(u16::try_from(value).map_err(|_| type_error(type_id))?),
        3 => Value::Short(i16::try_from(value).map_err(|_| type_error(type_id))?),
        4 => Value::DWord(u32::try_from(value).map_err(|_| type_error(type_id))?),
        5 => Value::Int(i32::try_from(value).map_err(|_| type_error(type_id))?),
        6 => Value::DWord64(value),
        7 => Value::Int64(i64::try_from(value).map_err(|_| type_error(type_id))?),
        _ => return Err(type_error(type_id)),
    })
}

fn type_error(type_id: u32) -> Error {
    Error::TypeMismatch {
        path: "style field".to_owned(),
        expected: "a numeric field type",
        actual: match type_id {
            8 => "float",
            9 => "double",
            10 => "string",
            _ => "non-numeric",
        },
    }
}

fn float_value(type_id: u32, value: f32) -> Result<Value> {
    match type_id {
        8 => Ok(Value::Float(value)),
        9 => Ok(Value::Double(f64::from(value))),
        _ => Err(Error::TypeMismatch {
            path: "TimeSpentInStyle".to_owned(),
            expected: "a float field",
            actual: "non-float",
        }),
    }
}
