//! Gems carried by the player and socketed into the amulet.
//!
//! Inventory gems live in `PlayerCreature.Gems`, amulet gems in
//! `PlayerCreature.Amulet.OpenGemsList`. The amulet list is a fixed array of
//! seven socket entries; a socket is empty when its entry has no `Gem` field (or
//! points at a gem without an `ItemID` and effects). Moving a gem therefore
//! swaps the entry between the two lists instead of changing the list length.

use crate::error::{Error, Result};
use crate::gff::{Gff, Struct, Value};
use crate::save::SaveGame;

/// Number of socket entries the game keeps in `Amulet.OpenGemsList`.
pub const AMULET_SOCKET_SLOTS: usize = 7;

/// Which list references a gem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GemLocation {
    /// Carried in the inventory.
    Inventory,
    /// Socketed into the amulet.
    Amulet,
}

/// One gem effect.
#[derive(Debug, Clone, PartialEq)]
pub struct GemEffect {
    /// Effect type id.
    pub kind: i32,
    /// Effect value.
    pub value: i32,
    /// Current value while the effect is active.
    pub current: i32,
    /// Duration in seconds, `0` for permanent effects.
    pub duration: f32,
}

/// One gem.
#[derive(Debug, Clone, PartialEq)]
pub struct Gem {
    /// Index of the struct that stores the gem entry.
    pub struct_index: usize,
    /// Item id, a row in `items.2da` of category `gems`.
    pub item_id: u16,
    /// Where the gem currently is.
    pub location: GemLocation,
    /// Effects stored with the gem.
    pub effects: Vec<GemEffect>,
}

impl Gem {
    /// True when the entry is an empty amulet socket placeholder.
    ///
    /// Empty sockets carry no `ItemID` (or id 0) and no effects. Item id 0 can
    /// still resolve to a generic row in a modded `items.2da`, so name lookup
    /// alone must not decide whether a socket is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.item_id == 0 && self.effects.is_empty()
    }
}

/// The amulet and its unlocked sockets.
///
/// A savegame before the amulet is unlocked has no `Amulet.Type` (and no
/// `Amulet.Active`): the player has no amulet yet, so there are no sockets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AmuletInfo {
    /// Amulet upgrade tier stored in `Amulet.Type` (1 when the field is absent).
    pub tier: u8,
    /// Number of sockets the tier unlocks (3, 5 or 7, capped at the seven
    /// entries the game keeps in `OpenGemsList`).
    pub sockets: u8,
}

/// Sockets unlocked by an amulet tier.
///
/// Evidence from real savegames: tier 1 has three sockets, tier 2 has five and
/// tier 4 has seven; the list never holds more than seven entries.
#[must_use]
pub fn sockets_for_tier(tier: u8) -> u8 {
    match tier {
        0 | 1 => 3,
        2 => 5,
        _ => 7,
    }
}

impl SaveGame {
    /// Reads the amulet tier and the number of sockets it unlocks.
    ///
    /// Returns `None` while the player does not have the amulet yet; the field
    /// only appears once the story unlocks it.
    pub fn amulet(&self) -> Result<Option<AmuletInfo>> {
        let location = self.player_location()?;
        let block = self
            .block(location.block)
            .ok_or_else(|| Error::Corrupt("character block vanished".to_owned()))?;
        Ok(amulet_in(block.gff(), location.prefix))
    }

    /// Reads the gems of the player.
    pub fn gems(&self) -> Result<Vec<Gem>> {
        let location = self.player_location()?;
        let block = self
            .block(location.block)
            .ok_or_else(|| Error::Corrupt("character block vanished".to_owned()))?;
        Ok(gems_in(block.gff(), location.prefix))
    }

    /// Adds a gem with the given predefined effects to the inventory.
    ///
    /// The gem struct is built from scratch with the field types the game uses,
    /// so no template gem has to exist in the savegame.
    ///
    /// # Errors
    ///
    /// Returns an error when the duration is invalid, the player block or the
    /// gem list cannot be written.
    pub fn add_gem(
        &mut self,
        item_id: u16,
        effects: &[(u32, u32)],
        duration: f32,
    ) -> Result<usize> {
        if !duration.is_finite() || duration < 0.0 {
            return Err(Error::OutOfRange {
                path: "gem duration".to_owned(),
                value: duration.to_string(),
                min: "0".to_owned(),
                max: "finite seconds".to_owned(),
            });
        }
        let entry = self.edit_player_gff(|gff, prefix| {
            let list_path = format!("{prefix}Gems");
            match gff.get(&list_path) {
                Ok(field) if field.value.as_list().is_none() => {
                    return Err(Error::TypeMismatch {
                        path: list_path,
                        expected: "a list",
                        actual: field.value.type_name(),
                    });
                }
                Ok(_) => {}
                Err(_) => gff.add(&list_path, Value::List(Vec::new()))?,
            }

            let mut effect_indices = Vec::with_capacity(effects.len());
            for (kind, value) in effects {
                let index = gff.push_struct(0);
                gff.add_field(index, "GemsEffectType", Value::DWord(*kind))?;
                gff.add_field(index, "GemsEffectValue", Value::DWord(*value))?;
                gff.add_field(index, "GemsEffectCurVal", Value::DWord(*value))?;
                gff.add_field(index, "GemsEffectDurati", Value::Float(duration))?;
                effect_indices.push(index);
            }
            let gem = gff.push_struct(0);
            gff.add_field(gem, "ItemID", Value::Word(item_id))?;
            gff.add_field(gem, "GemsEffectList", Value::List(effect_indices))?;
            let entry = gff.push_struct(0);
            gff.add_field(entry, "Gem", Value::Struct(gem))?;
            list_mut(gff, &list_path)?.push(entry);
            Ok(Some(entry))
        })?;
        entry.ok_or_else(|| Error::Corrupt("the gem was not added".to_owned()))
    }

    /// Moves a gem to the other list and returns its new location.
    ///
    /// The amulet list always keeps exactly seven entries. Socketing uses the
    /// first unlocked empty placeholder; moving a gem out replaces its entry
    /// with an empty placeholder. A malformed or full amulet is an error and
    /// leaves the savegame untouched.
    pub fn move_gem(&mut self, struct_index: usize) -> Result<GemLocation> {
        let result = self.edit_player_gff(|gff, prefix| {
            let current = gem_location_in(gff, prefix, struct_index).ok_or_else(|| {
                Error::Corrupt(format!("gem struct {struct_index} is in neither list"))
            })?;
            match current {
                GemLocation::Inventory => {
                    let amulet = amulet_in(gff, prefix).ok_or_else(|| Error::OutOfRange {
                        path: "gem socket".to_owned(),
                        value: struct_index.to_string(),
                        min: "an unlocked amulet".to_owned(),
                        max: "an unlocked amulet".to_owned(),
                    })?;
                    let source_path = format!("{prefix}Gems");
                    let target_path = format!("{prefix}Amulet.OpenGemsList");
                    let source = gem_list(gff, &source_path)?;
                    let source_position = source
                        .iter()
                        .position(|candidate| *candidate == struct_index)
                        .ok_or_else(|| {
                            Error::Corrupt(format!(
                                "gem struct {struct_index} is not in the inventory"
                            ))
                        })?;
                    let target = gem_list(gff, &target_path)?;
                    if target.len() != AMULET_SOCKET_SLOTS {
                        return Err(Error::Corrupt(format!(
                            "{target_path} has {} entries, expected {AMULET_SOCKET_SLOTS}",
                            target.len()
                        )));
                    }
                    let slot = usize::from(amulet.sockets);
                    let Some(position) = target
                        .iter()
                        .take(slot)
                        .position(|entry| is_empty_socket(gff, *entry))
                    else {
                        return Err(Error::OutOfRange {
                            path: "gem socket".to_owned(),
                            value: struct_index.to_string(),
                            min: "a free socket".to_owned(),
                            max: format!("{slot} unlocked sockets"),
                        });
                    };
                    list_mut(gff, &source_path)?.remove(source_position);
                    list_mut(gff, &target_path)?[position] = struct_index;
                    Ok(Some(GemLocation::Amulet))
                }
                GemLocation::Amulet => {
                    let source_path = format!("{prefix}Amulet.OpenGemsList");
                    let target_path = format!("{prefix}Gems");
                    let source = gem_list(gff, &source_path)?;
                    let source_position = source
                        .iter()
                        .position(|candidate| *candidate == struct_index)
                        .ok_or_else(|| {
                            Error::Corrupt(format!("gem struct {struct_index} is not socketed"))
                        })?;
                    if source.len() != AMULET_SOCKET_SLOTS {
                        return Err(Error::Corrupt(format!(
                            "{source_path} has {} entries, expected {AMULET_SOCKET_SLOTS}",
                            source.len()
                        )));
                    }
                    if let Ok(field) = gff.get(&target_path) {
                        if field.value.as_list().is_none() {
                            return Err(Error::TypeMismatch {
                                path: target_path,
                                expected: "a list",
                                actual: field.value.type_name(),
                            });
                        }
                    } else {
                        gff.add(&target_path, Value::List(Vec::new()))?;
                    }
                    let empty = gff.push_struct(0);
                    list_mut(gff, &target_path)?.push(struct_index);
                    list_mut(gff, &source_path)?[source_position] = empty;
                    Ok(Some(GemLocation::Inventory))
                }
            }
        })?;
        result.ok_or_else(|| Error::Corrupt("the gem was not moved".to_owned()))
    }

    /// Removes a gem from whichever list references it.
    ///
    /// A socketed gem leaves an empty socket placeholder behind so the amulet
    /// list keeps its seven entries.
    pub fn remove_gem(&mut self, struct_index: usize) -> Result<()> {
        let result = self.edit_player_gff(|gff, prefix| {
            let current = gem_location_in(gff, prefix, struct_index).ok_or_else(|| {
                Error::Corrupt(format!("gem struct {struct_index} is in neither list"))
            })?;
            match current {
                GemLocation::Inventory => {
                    let path = format!("{prefix}Gems");
                    list_mut(gff, &path)?.retain(|candidate| *candidate != struct_index);
                }
                GemLocation::Amulet => {
                    let path = format!("{prefix}Amulet.OpenGemsList");
                    let list = gem_list(gff, &path)?;
                    if list.len() != AMULET_SOCKET_SLOTS {
                        return Err(Error::Corrupt(format!(
                            "{path} has {} entries, expected {AMULET_SOCKET_SLOTS}",
                            list.len()
                        )));
                    }
                    let position = list
                        .iter()
                        .position(|candidate| *candidate == struct_index)
                        .ok_or_else(|| {
                            Error::Corrupt(format!("gem struct {struct_index} is not socketed"))
                        })?;
                    let empty = gff.push_struct(0);
                    list_mut(gff, &path)?[position] = empty;
                }
            }
            Ok(Some(()))
        })?;
        result.ok_or_else(|| Error::Corrupt("the gem was not removed".to_owned()))
    }
}

/// Reads the amulet state from a block.
fn amulet_in(gff: &Gff, prefix: &str) -> Option<AmuletInfo> {
    let tier_field = gff.get(format!("{prefix}Amulet.Type"));
    let has_active = gff.get(format!("{prefix}Amulet.Active")).is_ok();
    if tier_field.is_err() && !has_active {
        return None;
    }
    let tier = tier_field
        .ok()
        .and_then(|field| field.value.as_u64())
        .and_then(|value| u8::try_from(value).ok())
        .unwrap_or(1)
        .clamp(1, 4);
    Some(AmuletInfo {
        tier,
        sockets: sockets_for_tier(tier),
    })
}

/// Reads all gems from a block.
fn gems_in(gff: &Gff, prefix: &str) -> Vec<Gem> {
    let mut gems = Vec::new();
    for (path, place) in [
        (format!("{prefix}Gems"), GemLocation::Inventory),
        (format!("{prefix}Amulet.OpenGemsList"), GemLocation::Amulet),
    ] {
        let Ok(field) = gff.get(&path) else {
            continue;
        };
        let Some(indices) = field.value.as_list() else {
            continue;
        };
        for index in indices {
            let Some(entry) = gff.struct_at(*index) else {
                continue;
            };
            let gem_struct = entry
                .fields
                .iter()
                .find(|field| gff.label_name(field.label) == Some("Gem"))
                .and_then(|field| match field.value {
                    Value::Struct(inner) => gff.struct_at(inner),
                    _ => None,
                })
                .unwrap_or(entry);
            let item_id = label_number(gff, gem_struct, "ItemID")
                .and_then(|value| u16::try_from(value).ok())
                .unwrap_or(0);
            gems.push(Gem {
                struct_index: *index,
                item_id,
                location: place,
                effects: read_effects(gff, gem_struct),
            });
        }
    }
    gems
}

fn gem_location_in(gff: &Gff, prefix: &str, struct_index: usize) -> Option<GemLocation> {
    for (path, place) in [
        (format!("{prefix}Gems"), GemLocation::Inventory),
        (format!("{prefix}Amulet.OpenGemsList"), GemLocation::Amulet),
    ] {
        if gff
            .get(&path)
            .ok()
            .and_then(|field| field.value.as_list())
            .is_some_and(|list| list.contains(&struct_index))
        {
            return Some(place);
        }
    }
    None
}

fn gem_list<'a>(gff: &'a Gff, path: &str) -> Result<&'a [usize]> {
    let field = gff.get(path)?;
    field.value.as_list().ok_or_else(|| Error::TypeMismatch {
        path: path.to_owned(),
        expected: "a list",
        actual: field.value.type_name(),
    })
}

fn list_mut<'a>(gff: &'a mut Gff, path: &str) -> Result<&'a mut Vec<usize>> {
    let field = gff.get_mut(path)?;
    let actual = field.value.type_name();
    field
        .value
        .as_list_mut()
        .ok_or_else(|| Error::Corrupt(format!("{path} is not a list ({actual})")))
}

/// True when an amulet entry is an empty socket placeholder.
fn is_empty_socket(gff: &Gff, entry_index: usize) -> bool {
    let Some(entry) = gff.struct_at(entry_index) else {
        return true;
    };
    if entry.fields.is_empty() {
        return true;
    }
    let gem = nested_gem(gff, entry).unwrap_or(entry);
    let item_id = label_number(gff, gem, "ItemID").unwrap_or(0);
    let has_effects = label_field(gff, gem, "GemsEffectList")
        .and_then(Value::as_list)
        .is_some_and(|effects| !effects.is_empty());
    item_id == 0 && !has_effects
}

fn nested_gem<'a>(gff: &'a Gff, entry: &'a Struct) -> Option<&'a Struct> {
    entry
        .fields
        .iter()
        .find(|field| gff.label_name(field.label) == Some("Gem"))
        .and_then(|field| match field.value {
            Value::Struct(inner) => gff.struct_at(inner),
            _ => None,
        })
}

fn read_effects(gff: &Gff, gem: &Struct) -> Vec<GemEffect> {
    let Some(list) = label_field(gff, gem, "GemsEffectList").and_then(Value::as_list) else {
        return Vec::new();
    };
    let mut effects = Vec::new();
    for index in list {
        let Some(effect) = gff.struct_at(*index) else {
            continue;
        };
        effects.push(GemEffect {
            kind: number(gff, effect, "GemsEffectType").unwrap_or(0),
            value: number(gff, effect, "GemsEffectValue").unwrap_or(0),
            current: number(gff, effect, "GemsEffectCurVal").unwrap_or(0),
            duration: float(gff, effect, "GemsEffectDurati").unwrap_or(0.0),
        });
    }
    effects
}

fn label_field<'a>(gff: &Gff, structure: &'a Struct, label: &str) -> Option<&'a Value> {
    structure
        .fields
        .iter()
        .find(|field| gff.label_name(field.label) == Some(label))
        .map(|field| &field.value)
}

fn number(gff: &Gff, structure: &Struct, label: &str) -> Option<i32> {
    label_field(gff, structure, label)
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
}

fn label_number(gff: &Gff, structure: &Struct, label: &str) -> Option<i64> {
    label_field(gff, structure, label).and_then(Value::as_i64)
}

fn float(gff: &Gff, structure: &Struct, label: &str) -> Option<f32> {
    label_field(gff, structure, label).and_then(Value::as_f32)
}

#[cfg(test)]
mod tests {
    use super::{Gem, GemLocation, sockets_for_tier};

    #[test]
    fn sockets_follow_the_amulet_tier() {
        assert_eq!(sockets_for_tier(1), 3);
        assert_eq!(sockets_for_tier(2), 5);
        assert_eq!(sockets_for_tier(3), 7);
        assert_eq!(sockets_for_tier(4), 7);
        assert_eq!(sockets_for_tier(0), 3);
    }

    #[test]
    fn an_empty_socket_is_not_a_resolved_item_zero() {
        let empty = Gem {
            struct_index: 1,
            item_id: 0,
            location: GemLocation::Amulet,
            effects: Vec::new(),
        };
        assert!(empty.is_empty());
        let gem = Gem {
            struct_index: 2,
            item_id: 0,
            location: GemLocation::Inventory,
            effects: vec![super::GemEffect {
                kind: 62,
                value: 2,
                current: 2,
                duration: 0.0,
            }],
        };
        assert!(!gem.is_empty());
    }
}
