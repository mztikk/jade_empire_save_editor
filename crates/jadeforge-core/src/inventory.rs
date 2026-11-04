//! Inventory editing.
//!
//! Every item of the player is a struct with `Count`, `ItemId` and `Type`
//! fields that is referenced from the `ItemList` of the player creature.

use crate::error::{Error, Result};
use crate::gff::{Gff, Value};
use crate::save::SaveGame;

/// One item stack in the player inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// Index of the struct that stores the item.
    pub struct_index: usize,
    /// Item template id.
    pub item_id: u16,
    /// Stack size.
    pub count: u8,
    /// Item category, as used by the game's item tables.
    pub kind: u8,
}

impl SaveGame {
    /// Locates the `ItemList` field of the player and returns its path.
    fn item_list_path(&self) -> Result<String> {
        let location = self.player_location()?;
        Ok(format!("{}ItemList", location.prefix))
    }

    /// All items in the player inventory, in list order.
    pub fn items(&self) -> Result<Vec<Item>> {
        let location = self.player_location()?;
        let block = self
            .block(location.block)
            .ok_or_else(|| Error::Corrupt("character block vanished".to_owned()))?;
        let gff = block.gff();
        let path = self.item_list_path()?;
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
        let mut items = Vec::with_capacity(elements.len());
        for element in elements {
            let Some(structure) = gff.struct_at(*element) else {
                return Err(Error::Corrupt(format!(
                    "the item list references the missing struct {element}"
                )));
            };
            let number = |name: &str| -> Option<u64> {
                structure
                    .fields
                    .iter()
                    .find(|field| gff.label_name(field.label) == Some(name))
                    .and_then(|field| field.value.as_u64())
            };
            items.push(Item {
                struct_index: *element,
                item_id: number("ItemId").map_or(0, |value| value as u16),
                count: number("Count").map_or(1, |value| value as u8),
                kind: number("Type").map_or(0, |value| value as u8),
            });
        }
        Ok(items)
    }

    /// Changes the stack size of an item.
    pub fn set_item_count(&mut self, struct_index: usize, count: u8) -> Result<()> {
        if count == 0 {
            return Err(Error::OutOfRange {
                path: format!("item {struct_index} count"),
                value: "0".to_owned(),
                min: "1".to_owned(),
                max: u8::MAX.to_string(),
            });
        }
        let location = self.player_location()?;
        let block = self
            .block_mut(location.block)
            .ok_or_else(|| Error::Corrupt("character block vanished".to_owned()))?;
        let gff = block.gff_mut();
        let position = gff
            .struct_at(struct_index)
            .ok_or_else(|| Error::Corrupt(format!("struct {struct_index} does not exist")))?
            .fields
            .iter()
            .position(|field| gff.label_name(field.label) == Some("Count"))
            .ok_or_else(|| Error::FieldNotFound {
                path: format!("item {struct_index} Count"),
            })?;
        let field = &mut gff
            .struct_at_mut(struct_index)
            .expect("checked above")
            .fields[position];
        match field.value {
            Value::Byte(_) => field.value = Value::Byte(count),
            Value::Word(_) => field.value = Value::Word(u16::from(count)),
            _ => {
                return Err(Error::TypeMismatch {
                    path: format!("item {struct_index} Count"),
                    expected: "a byte or word",
                    actual: field.value.type_name(),
                });
            }
        }
        Ok(())
    }

    /// Removes an item from the inventory.
    ///
    /// The struct itself is left in the file as an orphan; the engine only
    /// follows the references of the item list, so this is safe and avoids
    /// renumbering every other struct.
    pub fn remove_item(&mut self, struct_index: usize) -> Result<bool> {
        let path = self.item_list_path()?;
        let location = self.player_location()?;
        let block = self
            .block_mut(location.block)
            .ok_or_else(|| Error::Corrupt("character block vanished".to_owned()))?;
        let was_modified = block.is_modified();
        let removed = {
            let gff = block.gff_mut();
            let field = gff.get_mut(&path)?;
            let actual = field.value.type_name();
            let elements = field
                .value
                .as_list_mut()
                .ok_or_else(|| Error::TypeMismatch {
                    path: path.clone(),
                    expected: "a list",
                    actual,
                })?;
            let before = elements.len();
            elements.retain(|element| *element != struct_index);
            elements.len() != before
        };
        if !removed && !was_modified {
            block.mark_clean();
        }
        Ok(removed)
    }

    /// Adds a new item stack to the inventory.
    ///
    /// The item struct is appended to the block; the label table is reused so
    /// no new labels are introduced.
    pub fn add_item(&mut self, item_id: u16, kind: u8, count: u8) -> Result<usize> {
        if count == 0 {
            return Err(Error::OutOfRange {
                path: "item count".to_owned(),
                value: "0".to_owned(),
                min: "1".to_owned(),
                max: u8::MAX.to_string(),
            });
        }
        let path = self.item_list_path()?;
        let location = self.player_location()?;
        let block = self
            .block_mut(location.block)
            .ok_or_else(|| Error::Corrupt("character block vanished".to_owned()))?;
        let gff = block.gff_mut();
        let struct_index = gff.push_struct(0);
        gff.add_field(struct_index, "Count", Value::Byte(count))?;
        gff.add_field(struct_index, "ItemId", Value::Word(item_id))?;
        gff.add_field(struct_index, "Type", Value::Byte(kind))?;
        let field = gff.get_mut(&path)?;
        let actual = field.value.type_name();
        let elements = field
            .value
            .as_list_mut()
            .ok_or_else(|| Error::TypeMismatch {
                path: path.clone(),
                expected: "a list",
                actual,
            })?;
        elements.push(struct_index);
        Ok(struct_index)
    }
}

/// Looks up an item inside a GFF block without going through a savegame.
///
/// Exposed for tests and for tooling that works on a single block.
pub fn read_items(gff: &Gff, list_path: &str) -> Result<Vec<Item>> {
    let field = gff.get(list_path)?;
    let elements = field.value.as_list().ok_or_else(|| Error::TypeMismatch {
        path: list_path.to_owned(),
        expected: "a list",
        actual: field.value.type_name(),
    })?;
    let mut items = Vec::with_capacity(elements.len());
    for element in elements {
        let structure = gff
            .struct_at(*element)
            .ok_or_else(|| Error::Corrupt(format!("missing struct {element}")))?;
        let number = |name: &str| -> Option<u64> {
            structure
                .fields
                .iter()
                .find(|field| gff.label_name(field.label) == Some(name))
                .and_then(|field| field.value.as_u64())
        };
        items.push(Item {
            struct_index: *element,
            item_id: number("ItemId").map_or(0, |value| value as u16),
            count: number("Count").map_or(1, |value| value as u8),
            kind: number("Type").map_or(0, |value| value as u8),
        });
    }
    Ok(items)
}
