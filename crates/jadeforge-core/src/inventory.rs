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
        read_items_from(gff, elements)
    }

    /// Changes the stack size of an item.
    ///
    /// A no-op does not dirty the savegame.
    pub fn set_item_count(&mut self, struct_index: usize, count: u8) -> Result<()> {
        if count == 0 {
            return Err(Error::OutOfRange {
                path: format!("item {struct_index} count"),
                value: "0".to_owned(),
                min: "1".to_owned(),
                max: u8::MAX.to_string(),
            });
        }
        let path = self.item_list_path()?;
        let result = self.edit_player_gff(|gff, _prefix| {
            let list = gff.get(&path)?;
            if list.value.as_list().is_none() {
                return Err(Error::TypeMismatch {
                    path: path.clone(),
                    expected: "a list",
                    actual: list.value.type_name(),
                });
            }
            let structure = gff
                .struct_at(struct_index)
                .ok_or_else(|| Error::Corrupt(format!("struct {struct_index} does not exist")))?;
            let position = structure
                .fields
                .iter()
                .position(|field| gff.label_name(field.label) == Some("Count"))
                .ok_or_else(|| Error::FieldNotFound {
                    path: format!("item {struct_index} Count"),
                })?;
            let field = &structure.fields[position];
            let replacement = match field.type_id {
                0 => Value::Byte(count),
                2 => Value::Word(u16::from(count)),
                _ => {
                    return Err(Error::TypeMismatch {
                        path: format!("item {struct_index} Count"),
                        expected: "a byte or word",
                        actual: field.value.type_name(),
                    });
                }
            };
            if field.value == replacement {
                return Ok(None);
            }
            gff.struct_at_mut(struct_index)
                .and_then(|structure| structure.fields.get_mut(position))
                .ok_or_else(|| Error::Corrupt(format!("struct {struct_index} does not exist")))?
                .value = replacement;
            Ok(Some(()))
        })?;
        let _ = result;
        Ok(())
    }

    /// Removes an item from the inventory.
    ///
    /// The struct itself is left in the file as an orphan; the engine only
    /// follows the references of the item list, so this is safe and avoids
    /// renumbering every other struct. A failed or no-op removal does not dirty
    /// the savegame.
    pub fn remove_item(&mut self, struct_index: usize) -> Result<bool> {
        let path = self.item_list_path()?;
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

    /// Adds a new item stack to the inventory.
    ///
    /// The item struct is appended to the block; the label table is reused so
    /// no new labels are introduced. A missing or malformed item list is an
    /// error and leaves the savegame untouched.
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
        let index = self.edit_player_gff(|gff, _prefix| {
            let list = gff.get(&path)?;
            if list.value.as_list().is_none() {
                return Err(Error::TypeMismatch {
                    path: path.clone(),
                    expected: "a list",
                    actual: list.value.type_name(),
                });
            }
            let struct_index = gff.push_struct(0);
            gff.add_field(struct_index, "Count", Value::Byte(count))?;
            gff.add_field(struct_index, "ItemId", Value::Word(item_id))?;
            gff.add_field(struct_index, "Type", Value::Byte(kind))?;
            let list = gff.get_mut(&path)?;
            let elements = list
                .value
                .as_list_mut()
                .ok_or_else(|| Error::Corrupt(format!("{path} is not a list")))?;
            elements.push(struct_index);
            Ok(Some(struct_index))
        })?;
        index.ok_or_else(|| Error::Corrupt("the item was not added".to_owned()))
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
    read_items_from(gff, elements)
}

fn read_items_from(gff: &Gff, elements: &[usize]) -> Result<Vec<Item>> {
    let mut items = Vec::with_capacity(elements.len());
    for element in elements {
        let structure = gff
            .struct_at(*element)
            .ok_or_else(|| Error::Corrupt(format!("missing struct {element}")))?;
        let number = |name: &str| -> Option<&Value> {
            structure
                .fields
                .iter()
                .find(|field| gff.label_name(field.label) == Some(name))
                .map(|field| &field.value)
        };
        let item_id = match number("ItemId").and_then(Value::as_u64) {
            Some(value) => u16::try_from(value).map_err(|_| {
                Error::Corrupt(format!("item struct {element} has an out-of-range ItemId"))
            })?,
            None => 0,
        };
        let count = match number("Count").and_then(Value::as_u64) {
            Some(value) => u8::try_from(value).map_err(|_| {
                Error::Corrupt(format!("item struct {element} has an out-of-range Count"))
            })?,
            None => 1,
        };
        let kind = match number("Type").and_then(Value::as_u64) {
            Some(value) => u8::try_from(value).map_err(|_| {
                Error::Corrupt(format!("item struct {element} has an out-of-range Type"))
            })?,
            None => 0,
        };
        items.push(Item {
            struct_index: *element,
            item_id,
            count,
            kind,
        });
    }
    Ok(items)
}
