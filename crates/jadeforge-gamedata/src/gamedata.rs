//! Game data access: talk table strings and 2DA tables.
//!
//! Jade Empire keeps display names outside the savegame: item and style names
//! are talk table references in text 2DA tables. This module loads those tables
//! from a game installation and exposes them as typed records.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::talktable::TalkTable;
use crate::twoda::TwoDa;

/// One row of `items.2da`, resolved to human readable text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemInfo {
    /// Row in `items.2da`, which is the value stored in savegames.
    pub id: u16,
    /// Internal template label, for example `TwoSwords_upgrade_Khana`.
    pub label: String,
    /// Localized name from the talk table.
    pub name: String,
    /// Localized description from the talk table.
    pub description: Option<String>,
    /// Category as written in `items.2da`, for example `weapons`.
    pub category: Option<String>,
    /// Value the savegame uses in the `Type` field of an item struct.
    pub save_type: Option<u8>,
    /// Price in silver.
    pub cost: u32,
    /// Icon resource name.
    pub icon: Option<String>,
}

/// One row of `stylesuperlist.2da`, resolved to human readable text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleInfo {
    /// Row in `stylesuperlist.2da`, which is the value stored in savegames.
    pub id: u16,
    /// Talk table reference of the name.
    pub name_ref: u32,
    /// Internal label, for example `Martial_Legendary_Strike`.
    pub label: String,
    /// Localized name from the talk table.
    pub name: String,
    /// Localized description from the talk table.
    pub description: Option<String>,
    /// Icon resource name.
    pub icon: Option<String>,
    /// Number of weapons the style carries.
    pub weapon_count: u8,
    /// Weapon resource, when the style is a weapon style.
    pub weapon: Option<String>,
}

/// A loaded game installation.
#[derive(Debug)]
pub struct GameData {
    root: PathBuf,
    items: Vec<Option<ItemInfo>>,
    styles: Vec<Option<StyleInfo>>,
    search_index: HashMap<u16, String>,
}

impl GameData {
    /// Tries to find the game installation that a savegame belongs to.
    ///
    /// `save_path` is the `save.sac` file, or the folder that contains it.
    /// `JADEFORGE_GAME_DIR` wins over the discovery.
    #[must_use]
    pub fn discover(save_path: &Path) -> Option<Self> {
        if let Some(directory) = std::env::var_os("JADEFORGE_GAME_DIR") {
            return Self::open(PathBuf::from(directory)).ok();
        }
        let folder = if save_path.is_dir() {
            save_path
        } else {
            save_path.parent()?
        };
        // <game>/Save/<savegame>/save.sac
        for candidate in folder.ancestors().skip(1).take(3) {
            if is_game_root(candidate) {
                return Self::open(candidate.to_path_buf()).ok();
            }
        }
        None
    }

    /// Loads all tables from a game installation directory.
    #[allow(clippy::too_many_lines)] // one pass per table keeps the lookups obvious
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        if !is_game_root(&root) {
            return Err(Error::NotAGameInstallation(root));
        }

        let talktable = ["dialog.tlk", "dialogf.tlk"]
            .iter()
            .map(|name| root.join(name))
            .find(|path| path.is_file())
            .and_then(|path| TalkTable::open(path).ok());

        let items = load_two_da(&root, "items.2da");
        let styles = load_two_da(&root, "stylesuperlist.2da");

        let mut item_records = Vec::new();
        if let (Some(table), Some(talk)) = (&items, &talktable) {
            for (index, row) in table.iter() {
                let Ok(id) = u16::try_from(index) else {
                    continue;
                };
                let label = row
                    .cell(table.column("LABEL").unwrap_or(0))
                    .unwrap_or_default();
                let name_ref = table.uint(index, "STRREF_NAME");
                let description_ref = table.uint(index, "STRREF_DESC");
                let name = name_ref
                    .and_then(|reference| talk.get(reference as u32))
                    .filter(|text| !text.is_empty());
                let Some(name) = name else {
                    continue;
                };
                if name == "****" {
                    continue;
                }
                let category = table
                    .cell(index, "TYPE")
                    .filter(|value| !value.is_empty() && *value != "****")
                    .map(str::to_lowercase);
                item_records.push(Some(ItemInfo {
                    id,
                    label: label.to_owned(),
                    name,
                    description: description_ref.and_then(|reference| talk.get(reference as u32)),
                    save_type: category.as_deref().and_then(save_type_for_category),
                    cost: table.uint(index, "COST").unwrap_or(0) as u32,
                    category,
                    icon: table
                        .cell(index, "ICON")
                        .filter(|value| !value.is_empty() && *value != "****")
                        .map(ToOwned::to_owned),
                }));
            }
        }
        let mut item_slots = vec![None; item_records.len()];
        for record in item_records.into_iter().flatten() {
            let index = record.id as usize;
            if item_slots.len() <= index {
                item_slots.resize(index + 1, None);
            }
            item_slots[index] = Some(record);
        }

        let mut style_slots = Vec::new();
        if let (Some(table), Some(talk)) = (&styles, &talktable) {
            for (index, _) in table.iter() {
                let Ok(id) = u16::try_from(index) else {
                    continue;
                };
                let name_ref = table.uint(index, "NameRef");
                let Some(name) = name_ref.and_then(|reference| talk.get(reference as u32)) else {
                    continue;
                };
                if name.is_empty() || name == "****" {
                    continue;
                }
                if style_slots.len() <= id as usize {
                    style_slots.resize(id as usize + 1, None);
                }
                let Some(name_ref) = name_ref.and_then(|reference| u32::try_from(reference).ok())
                else {
                    continue;
                };
                style_slots[id as usize] = Some(StyleInfo {
                    id,
                    name_ref,
                    label: table.cell(index, "Label").unwrap_or_default().to_owned(),
                    name,
                    description: table
                        .uint(index, "DescRef")
                        .and_then(|reference| talk.get(reference as u32)),
                    icon: table
                        .cell(index, "IconRef")
                        .filter(|value| !value.is_empty() && *value != "****")
                        .map(ToOwned::to_owned),
                    weapon_count: table
                        .uint(index, "WeaponCount")
                        .and_then(|value| u8::try_from(value).ok())
                        .unwrap_or(0),
                    weapon: table
                        .cell(index, "Weapon")
                        .filter(|value| !value.is_empty() && *value != "****")
                        .map(ToOwned::to_owned),
                });
            }
        }

        let mut search_index = HashMap::new();
        for item in item_slots.iter().flatten() {
            search_index.insert(
                item.id,
                format!("{} {}", item.name, item.label).to_lowercase(),
            );
        }

        Ok(Self {
            root,
            items: item_slots,
            styles: style_slots,
            search_index,
        })
    }

    /// Root directory of the installation.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Number of resolved items.
    #[must_use]
    pub fn item_count(&self) -> usize {
        self.items.iter().flatten().count()
    }

    /// Number of resolved styles.
    #[must_use]
    pub fn style_count(&self) -> usize {
        self.styles.iter().flatten().count()
    }

    /// Looks up an item by its row id.
    #[must_use]
    pub fn item(&self, id: u16) -> Option<&ItemInfo> {
        self.items.get(id as usize)?.as_ref()
    }

    /// Looks up a style by its row id.
    #[must_use]
    pub fn style(&self, id: u16) -> Option<&StyleInfo> {
        self.styles.get(id as usize)?.as_ref()
    }

    /// All items with a resolved name, ordered by id.
    pub fn items(&self) -> impl Iterator<Item = &ItemInfo> {
        self.items.iter().flatten()
    }

    /// All styles with a resolved name, ordered by id.
    pub fn styles(&self) -> impl Iterator<Item = &StyleInfo> {
        self.styles.iter().flatten()
    }

    /// Full text search over item names and labels.
    ///
    /// Results that start with the query are ranked first, the rest keep id
    /// order.
    #[must_use]
    pub fn search_items(&self, query: &str, limit: usize) -> Vec<&ItemInfo> {
        let query = query.trim().to_lowercase();
        if query.is_empty() {
            return Vec::new();
        }
        let mut matches: Vec<(bool, &ItemInfo)> = Vec::new();
        for (id, haystack) in &self.search_index {
            if haystack.contains(&query) {
                if let Some(item) = self.item(*id) {
                    matches.push((item.name.to_lowercase().starts_with(&query), item));
                }
            }
        }
        matches.sort_by(|left, right| {
            right
                .0
                .cmp(&left.0)
                .then_with(|| left.1.name.to_lowercase().cmp(&right.1.name.to_lowercase()))
                .then_with(|| left.1.id.cmp(&right.1.id))
        });
        matches
            .into_iter()
            .take(limit)
            .map(|(_, item)| item)
            .collect()
    }
}

/// Value the savegame has to use for an item of the given category.
#[must_use]
pub fn save_type_for_category(category: &str) -> Option<u8> {
    match category.to_lowercase().as_str() {
        "gems" => Some(1),
        "powerups" => Some(4),
        "weapons" => Some(5),
        "plot_items" => Some(6),
        "mgweapons" => Some(7),
        "improve" => Some(8),
        _ => None,
    }
}

/// True when the directory looks like a Jade Empire installation.
#[must_use]
pub fn is_game_root(directory: &Path) -> bool {
    directory.join("dialog.tlk").is_file()
        || directory.join("dialogf.tlk").is_file()
        || directory.join("override").is_dir()
        || directory.join("chitin.key").is_file()
}

fn load_two_da(root: &Path, name: &str) -> Option<TwoDa> {
    let candidates = [root.join("override").join(name), root.join(name)];
    for path in candidates {
        if path.is_file() {
            if let Ok(text) = std::fs::read_to_string(&path) {
                return TwoDa::parse(&text).ok();
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_mapping_covers_the_engine_values() {
        assert_eq!(save_type_for_category("GEMS"), Some(1));
        assert_eq!(save_type_for_category("powerups"), Some(4));
        assert_eq!(save_type_for_category("weapons"), Some(5));
        assert_eq!(save_type_for_category("plot_items"), Some(6));
        assert_eq!(save_type_for_category("mgweapons"), Some(7));
        assert_eq!(save_type_for_category("improve"), Some(8));
        assert_eq!(save_type_for_category("nonsense"), None);
    }

    #[test]
    fn missing_installation_is_reported() {
        let error = GameData::open("/definitely/not/a/game").expect_err("must fail");
        assert!(error.to_string().contains("not a Jade Empire"));
    }
}
