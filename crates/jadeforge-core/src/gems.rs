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

impl SaveGame {
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
                let Some(base) = gem_base(gff, *index) else {
                    continue;
                };
                let item_id = gff
                    .get(format!("{base}.ItemID"))
                    .ok()
                    .and_then(|field| field.value.as_u64())
                    .and_then(|value| u16::try_from(value).ok())
                    .unwrap_or(0);
                gems.push(Gem {
                    struct_index: *index,
                    item_id,
                    location: place,
                    effects: read_effects(gff, &base),
                });
            }
        }
        Ok(gems)
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

/// Path of the `Gem` struct of an entry. Some savegames point at the gem
/// struct directly, others wrap it in an entry struct with a `Gem` field.
fn gem_base(gff: &crate::gff::Gff, index: usize) -> Option<String> {
    let structure = gff.struct_at(index)?;
    let has_gem = structure
        .fields
        .iter()
        .any(|field| gff.label_name(field.label) == Some("Gem"));
    Some(if has_gem {
        format!("{index}.Gem")
    } else {
        format!("{index}")
    })
}

fn read_effects(gff: &crate::gff::Gff, base: &str) -> Vec<GemEffect> {
    let mut effects = Vec::new();
    let Ok(field) = gff.get(format!("{base}.GemsEffectList")) else {
        return effects;
    };
    let Some(indices) = field.value.as_list() else {
        return effects;
    };
    for index in indices {
        let number = |name: &str| {
            gff.get(format!("{index}.{name}"))
                .ok()
                .and_then(|field| field.value.as_i64())
                .and_then(|value| i32::try_from(value).ok())
        };
        effects.push(GemEffect {
            kind: number("GemsEffectType").unwrap_or(0),
            value: number("GemsEffectValue").unwrap_or(0),
            current: number("GemsEffectCurVal").unwrap_or(0),
            duration: gff
                .get(format!("{index}.GemsEffectDuration"))
                .ok()
                .and_then(|field| field.value.as_f32())
                .unwrap_or(0.0),
        });
    }
    effects
}
