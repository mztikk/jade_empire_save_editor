//! Gems carried by the player and socketed into the amulet.
//!
//! Inventory gems live in `PlayerCreature.Gems`, amulet gems in
//! `PlayerCreature.Amulet.OpenGemsList`; both lists point at structs shaped
//! `{ Gem: { ItemID, GemsEffectList[] } }`. Moving a gem only changes which list
//! references the struct, so the gem itself is never rewritten.

use crate::error::{Error, Result};
use crate::save::SaveGame;

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
    /// Index of the struct that stores the gem.
    pub struct_index: usize,
    /// Item id, a row in `items.2da` of category `gems`.
    pub item_id: u16,
    /// Where the gem currently is.
    pub location: GemLocation,
    /// Effects stored with the gem.
    pub effects: Vec<GemEffect>,
}

/// The amulet and its unlocked sockets.
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
    pub fn amulet(&self) -> Result<AmuletInfo> {
        let location = self.player_location()?;
        let block = self
            .block(location.block)
            .ok_or_else(|| Error::Corrupt("character block vanished".to_owned()))?;
        let tier = block
            .gff()
            .get(format!("{}Amulet.Type", location.prefix))
            .ok()
            .and_then(|field| field.value.as_u64())
            .and_then(|value| u8::try_from(value).ok())
            .unwrap_or(1)
            .clamp(1, 4);
        Ok(AmuletInfo {
            tier,
            sockets: sockets_for_tier(tier),
        })
    }

    /// Reads the gems of the player.
    pub fn gems(&self) -> Result<Vec<Gem>> {
        let location = self.player_location()?;
        let block = self
            .block(location.block)
            .ok_or_else(|| Error::Corrupt("character block vanished".to_owned()))?;
        let gff = block.gff();
        let mut gems = Vec::new();
        for (path, place) in [
            (format!("{}Gems", location.prefix), GemLocation::Inventory),
            (
                format!("{}Amulet.OpenGemsList", location.prefix),
                GemLocation::Amulet,
            ),
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
                        crate::gff::Value::Struct(inner) => gff.struct_at(inner),
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
        Ok(gems)
    }

    /// Adds a gem with the given predefined effects to the inventory.
    ///
    /// The gem struct is built from scratch with the field types the game uses,
    /// so no template gem has to exist in the savegame.
    ///
    /// # Errors
    ///
    /// Returns an error when the player block or the gem list cannot be
    /// written.
    pub fn add_gem(
        &mut self,
        item_id: u16,
        effects: &[(u32, u32)],
        duration: f32,
    ) -> Result<usize> {
        let location = self.player_location()?;
        let block = self
            .block_mut(location.block)
            .ok_or_else(|| Error::Corrupt("character block vanished".to_owned()))?;
        let gff = block.gff_mut();
        let list_path = format!("{}Gems", location.prefix);
        if gff.get(&list_path).is_err() {
            gff.add(&list_path, crate::gff::Value::List(Vec::new()))?;
        }
        let mut effect_indices = Vec::new();
        for (kind, value) in effects {
            let index = gff.push_struct(0);
            gff.add_field(index, "GemsEffectType", crate::gff::Value::DWord(*kind))?;
            gff.add_field(index, "GemsEffectValue", crate::gff::Value::DWord(*value))?;
            gff.add_field(index, "GemsEffectCurVal", crate::gff::Value::DWord(*value))?;
            gff.add_field(
                index,
                "GemsEffectDurati",
                crate::gff::Value::Float(duration),
            )?;
            effect_indices.push(index);
        }
        let gem = gff.push_struct(0);
        gff.add_field(gem, "ItemID", crate::gff::Value::Word(item_id))?;
        gff.add_field(
            gem,
            "GemsEffectList",
            crate::gff::Value::List(effect_indices),
        )?;
        let entry = gff.push_struct(0);
        gff.add_field(entry, "Gem", crate::gff::Value::Struct(gem))?;
        let list = gff
            .get_mut(&list_path)?
            .value
            .as_list_mut()
            .ok_or_else(|| Error::Corrupt(format!("{list_path} is not a list")))?;
        list.push(entry);
        Ok(entry)
    }

    /// Moves a gem to the other list and returns its new location.
    pub fn move_gem(&mut self, struct_index: usize) -> Result<GemLocation> {
        let current = self.gem_location(struct_index)?;
        let target = match current {
            GemLocation::Inventory => "Amulet.OpenGemsList",
            GemLocation::Amulet => "Gems",
        };
        let source = match current {
            GemLocation::Inventory => "Gems",
            GemLocation::Amulet => "Amulet.OpenGemsList",
        };
        let location = self.player_location()?;
        let block = self
            .block_mut(location.block)
            .ok_or_else(|| Error::Corrupt("character block vanished".to_owned()))?;
        let gff = block.gff_mut();
        let source_path = format!("{}{source}", location.prefix);
        let target_path = format!("{}{target}", location.prefix);
        list_mut(gff, &source_path)?.retain(|candidate| *candidate != struct_index);
        if gff.get(&target_path).is_err() {
            gff.add(&target_path, crate::gff::Value::List(Vec::new()))?;
        }
        list_mut(gff, &target_path)?.push(struct_index);
        Ok(match current {
            GemLocation::Inventory => GemLocation::Amulet,
            GemLocation::Amulet => GemLocation::Inventory,
        })
    }

    /// Removes a gem from whichever list references it.
    pub fn remove_gem(&mut self, struct_index: usize) -> Result<()> {
        let current = self.gem_location(struct_index)?;
        let path = match current {
            GemLocation::Inventory => "Gems",
            GemLocation::Amulet => "Amulet.OpenGemsList",
        };
        let location = self.player_location()?;
        let block = self
            .block_mut(location.block)
            .ok_or_else(|| Error::Corrupt("character block vanished".to_owned()))?;
        let gff = block.gff_mut();
        list_mut(gff, &format!("{}{path}", location.prefix))?
            .retain(|candidate| *candidate != struct_index);
        Ok(())
    }

    fn gem_location(&self, struct_index: usize) -> Result<GemLocation> {
        let location = self.player_location()?;
        let block = self
            .block(location.block)
            .ok_or_else(|| Error::Corrupt("character block vanished".to_owned()))?;
        let gff = block.gff();
        for (path, place) in [
            (format!("{}Gems", location.prefix), GemLocation::Inventory),
            (
                format!("{}Amulet.OpenGemsList", location.prefix),
                GemLocation::Amulet,
            ),
        ] {
            if gff
                .get(&path)
                .ok()
                .and_then(|field| field.value.as_list())
                .is_some_and(|list| list.contains(&struct_index))
            {
                return Ok(place);
            }
        }
        Err(Error::Corrupt(format!(
            "gem struct {struct_index} is in neither list"
        )))
    }
}

fn list_mut<'a>(gff: &'a mut crate::gff::Gff, path: &str) -> Result<&'a mut Vec<usize>> {
    let field = gff.get_mut(path)?;
    if field.value.as_list_mut().is_some() {
        return Ok(field.value.as_list_mut().expect("checked above"));
    }
    Err(Error::Corrupt(format!(
        "{path} is not a list ({})",
        field.value.type_name()
    )))
}

fn read_effects(gff: &crate::gff::Gff, gem: &crate::gff::Struct) -> Vec<GemEffect> {
    let Some(list) = gem
        .fields
        .iter()
        .find(|field| gff.label_name(field.label) == Some("GemsEffectList"))
        .and_then(|field| field.value.as_list())
    else {
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

fn label_field<'a>(
    gff: &crate::gff::Gff,
    structure: &'a crate::gff::Struct,
    label: &str,
) -> Option<&'a crate::gff::Value> {
    structure
        .fields
        .iter()
        .find(|field| gff.label_name(field.label) == Some(label))
        .map(|field| &field.value)
}

fn number(gff: &crate::gff::Gff, structure: &crate::gff::Struct, label: &str) -> Option<i32> {
    label_field(gff, structure, label)
        .and_then(crate::gff::Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
}

fn label_number(gff: &crate::gff::Gff, structure: &crate::gff::Struct, label: &str) -> Option<i64> {
    label_field(gff, structure, label).and_then(crate::gff::Value::as_i64)
}

fn float(gff: &crate::gff::Gff, structure: &crate::gff::Struct, label: &str) -> Option<f32> {
    label_field(gff, structure, label).and_then(crate::gff::Value::as_f32)
}

#[cfg(test)]
mod tests {
    use super::sockets_for_tier;

    #[test]
    fn sockets_follow_the_amulet_tier() {
        assert_eq!(sockets_for_tier(1), 3);
        assert_eq!(sockets_for_tier(2), 5);
        assert_eq!(sockets_for_tier(3), 7);
        assert_eq!(sockets_for_tier(4), 7);
        assert_eq!(sockets_for_tier(0), 3);
    }
}
