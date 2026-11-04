//! Game data access: talk table strings and 2DA tables.
//!
//! Jade Empire keeps display names outside the savegame: item and style names
//! are talk table references in text 2DA tables. This module loads those tables
//! from a game installation and exposes them as typed records.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::rim::RimArchive;
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

/// How a fighting style is used.
///
/// The `Type` column of `stylesuperlist.2da` points into `StyleTypes.2da`
/// (`HAND`, `WEAPON`, `MAGIC`, `TRANSFORM`, `Support`, ...), but it is not a
/// reliable description: dual weapon styles are stored as `MONSTER` and
/// `Unique_Spirit_Thief` as `SpiritThief`. The label prefix is what the
/// designers used to group styles, so it is what the interface shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleKind {
    /// Unarmed martial style.
    Martial,
    /// One or two handed weapon style.
    Weapon,
    /// Magic style.
    Magic,
    /// Support style (buffs, healing).
    Support,
    /// Transformation style.
    Transformation,
    /// Improvised weapon style.
    Improvised,
    /// Unique, story given style.
    Unique,
    /// Not recognised.
    Other,
}

impl StyleKind {
    /// Short lower case name for the interface.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Martial => "martial",
            Self::Weapon => "weapon",
            Self::Magic => "magic",
            Self::Support => "support",
            Self::Transformation => "transformation",
            Self::Improvised => "improvised",
            Self::Unique => "unique",
            Self::Other => "style",
        }
    }
}

/// Classifies a style by its template label prefix.
#[must_use]
pub fn style_kind(label: &str) -> StyleKind {
    let lower = label.to_lowercase();
    if lower.starts_with("martial") {
        StyleKind::Martial
    } else if lower.starts_with("weapon") || lower.starts_with("hench_weapon") {
        StyleKind::Weapon
    } else if lower.starts_with("magic") {
        StyleKind::Magic
    } else if lower.starts_with("support") {
        StyleKind::Support
    } else if lower.starts_with("transformation") || lower.starts_with("c_") {
        StyleKind::Transformation
    } else if lower.starts_with("improvised") {
        StyleKind::Improvised
    } else if lower.starts_with("unique") || lower.starts_with("spirit") {
        StyleKind::Unique
    } else {
        StyleKind::Other
    }
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
    /// Kind of style, derived from the label prefix.
    pub kind: StyleKind,
}

/// A loaded game installation.
#[derive(Debug)]
pub struct GameData {
    root: PathBuf,
    items: Vec<Option<ItemInfo>>,
    styles: Vec<Option<StyleInfo>>,
    advances: Vec<Option<Advance>>,
    style_advances: Vec<Option<Vec<u16>>>,
    search_index: HashMap<u16, String>,
}

/// One line of `StyleAdvance.2da`, the table that style upgrade bits point at.
///
/// Bit *i* of a style's upgrade mask selects row *i + 1*; the saved level is
/// `1 + number of set bits`. Rows come in families of five tiers with costs
/// 1, 2, 4, 6 and 10.
#[derive(Debug, Clone, PartialEq)]
pub struct Advance {
    /// Row in `StyleAdvance.2da`; bit `row - 1` of the style mask selects it.
    pub row: u16,
    /// Internal label, for example `Increase_Damage_3`.
    pub label: String,
    /// Localized name from the talk table.
    pub name: String,
    /// Style points the upgrade costs.
    pub cost: u8,
    /// Effect id applied by the upgrade.
    pub effect: u8,
    /// Parameter passed to the effect.
    pub param: f32,
    /// Family label without the tier suffix, for example `Increase_Damage`.
    pub family: String,
    /// Tier inside the family (1 based), zero when the label has no suffix.
    pub tier: u8,
}

impl Advance {
    /// Short line describing the effect, for example `Damage +25%`.
    #[must_use]
    pub fn effect_text(&self) -> String {
        let percent = self.param * 100.0;
        match self.effect {
            69 => format!("Chi cost {percent:+.0}%"),
            70 => format!("Focus cost {percent:+.0}%"),
            72 => format!("Damage {percent:+.0}%"),
            81 => format!("Speed {percent:+.0}%"),
            84 => format!("Chi strike {percent:+.0}%"),
            85 => format!("Range {percent:+.0}%"),
            86 => format!("Duration {percent:+.0}%"),
            87 => format!("Resistance +{}", self.param),
            _ => format!("Effect {} ({})", self.effect, self.param),
        }
    }
}

/// Splits `Increase_Damage_3` into `("Increase_Damage", 3)`.
#[must_use]
pub fn family_and_tier(label: &str) -> (String, u8) {
    match label.rsplit_once('_') {
        Some((family, tier)) if !family.is_empty() => match tier.parse::<u8>() {
            Ok(tier) => (family.to_owned(), tier),
            Err(_) => (label.to_owned(), 0),
        },
        _ => (label.to_owned(), 0),
    }
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

        let source = TableSource::new(&root);
        let items = source.load("items.2da");
        let styles = source.load("stylesuperlist.2da");
        let advances = source.load("StyleAdvance.2da");
        let style_data = source.load("styledata.2da");

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
                let label = table.cell(index, "Label").unwrap_or_default().to_owned();
                style_slots[id as usize] = Some(StyleInfo {
                    id,
                    name_ref,
                    kind: style_kind(&label),
                    label,
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

        let mut advance_slots: Vec<Option<Advance>> = Vec::new();
        if let Some(table) = &advances {
            advance_slots.resize(table.len(), None);
            for (index, _) in table.iter() {
                let Ok(row) = u16::try_from(index) else {
                    continue;
                };
                if row == 0 {
                    continue;
                }
                let label = table.cell(index, "Label").unwrap_or_default();
                if label.is_empty() || label == "****" || label.eq_ignore_ascii_case("INVALID") {
                    continue;
                }
                let (family, tier) = family_and_tier(label);
                advance_slots[row as usize] = Some(Advance {
                    row,
                    label: label.to_owned(),
                    name: table
                        .uint(index, "DescRef")
                        .and_then(|reference| talktable.as_ref()?.get(reference as u32))
                        .unwrap_or_default(),
                    cost: table
                        .uint(index, "CostToLevelUp")
                        .and_then(|value| u8::try_from(value).ok())
                        .unwrap_or(0),
                    effect: table
                        .uint(index, "Effect1")
                        .and_then(|value| u8::try_from(value).ok())
                        .unwrap_or(0),
                    param: table
                        .row(index)
                        .and_then(|row| row.float(table.column("Param1")?))
                        .unwrap_or(0.0),
                    family,
                    tier,
                });
            }
        }

        // Each style row points at a `styledata.2da` row, which names the
        // `<Style>LV.2da` table with the upgrades the level-up screen offers.
        let mut style_advances: Vec<Option<Vec<u16>>> = vec![None; style_slots.len()];
        if let (Some(styles), Some(style_data)) = (&styles, &style_data) {
            for (index, style) in style_slots.iter().enumerate() {
                if style.is_none() {
                    continue;
                }
                let Some(data_index) = styles
                    .uint(index, "StyleData")
                    .and_then(|value| usize::try_from(value).ok())
                else {
                    continue;
                };
                let Some(table_name) = style_data.cell(data_index, "levelup2da") else {
                    continue;
                };
                let table_name = table_name.trim();
                if table_name.is_empty() || table_name == "****" {
                    continue;
                }
                if let Some(table) = source.load(&format!("{table_name}.2da")) {
                    style_advances[index] = Some(choice_rows(&table));
                }
            }
        }

        Ok(Self {
            root,
            items: item_slots,
            styles: style_slots,
            advances: advance_slots,
            style_advances,
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

    /// All style upgrades that could be resolved, ordered by row.
    pub fn advances(&self) -> impl Iterator<Item = &Advance> {
        self.advances.iter().flatten()
    }

    /// Looks an upgrade up by its `StyleAdvance.2da` row.
    #[must_use]
    pub fn advance(&self, row: u16) -> Option<&Advance> {
        self.advances
            .get(usize::from(row))
            .and_then(Option::as_ref)
            .filter(|advance| advance.row == row)
    }

    /// Upgrade rows the game offers for a style, from its `<Style>LV.2da` table.
    ///
    /// `None` when the installation does not ship a table for the style, which
    /// is the case for most styles: the editor then offers the whole
    /// `StyleAdvance.2da` table instead of guessing.
    #[must_use]
    pub fn allowed_advances(&self, style_id: u16) -> Option<&[u16]> {
        self.style_advances
            .get(usize::from(style_id))
            .and_then(Option::as_ref)
            .map(Vec::as_slice)
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

    /// Items that a player can actually own.
    ///
    /// The table also contains developer placeholders (`[TEMP] ...`, `temp_*`,
    /// `zzz*`) and cut content; those are hidden from the item picker but can
    /// still be looked up by id when a savegame references one.
    pub fn playable_items(&self) -> impl Iterator<Item = &ItemInfo> {
        self.items().filter(|item| is_playable_item(item))
    }

    /// Number of items a player can own.
    #[must_use]
    pub fn playable_item_count(&self) -> usize {
        self.playable_items().count()
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
                if let Some(item) = self.item(*id).filter(|item| is_playable_item(item)) {
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

/// True when an item is not a development placeholder or cut content.
#[must_use]
pub fn is_playable_item(item: &ItemInfo) -> bool {
    let label = item.label.to_lowercase();
    let name = item.name.to_lowercase();
    !(label.starts_with("temp")
        || label.starts_with("zzz")
        || label.starts_with("cut_")
        || label.starts_with("test")
        || name.starts_with("[temp]")
        || name.starts_with("[test]")
        || name.starts_with("zzz"))
}

/// Rows referenced by `Choice_1..5` of a `<Style>LV.2da` table.
fn choice_rows(table: &TwoDa) -> Vec<u16> {
    let mut rows: Vec<u16> = Vec::new();
    for (_, row) in table.iter() {
        for column in ["Choice_1", "Choice_2", "Choice_3", "Choice_4", "Choice_5"] {
            let Some(column) = table.column(column) else {
                continue;
            };
            if let Some(value) = row.uint(column) {
                if let Ok(value) = u16::try_from(value) {
                    if value > 0 && !rows.contains(&value) {
                        rows.push(value);
                    }
                }
            }
        }
    }
    rows.sort_unstable();
    rows
}

/// Where the 2DA tables come from: `override/` first, the RIM archive second.
struct TableSource {
    overrides: HashMap<String, PathBuf>,
    rim: Option<RimArchive>,
}

impl TableSource {
    fn new(root: &Path) -> Self {
        let mut overrides = HashMap::new();
        if let Ok(entries) = std::fs::read_dir(root.join("override")) {
            for entry in entries.flatten() {
                let path = entry.path();
                let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                    continue;
                };
                if name.to_lowercase().ends_with(".2da") {
                    overrides.insert(name.to_lowercase(), path);
                }
            }
        }
        let rim = ["miniglobal-a.rim", "globalunload-a.rim"]
            .iter()
            .map(|name| root.join("data").join(name))
            .find(|path| path.is_file())
            .and_then(|path| RimArchive::open(path).ok());
        Self { overrides, rim }
    }

    /// Loads a table by file name, honouring `override/` before the archive.
    fn load(&self, name: &str) -> Option<TwoDa> {
        let key = name.to_lowercase();
        if let Some(path) = self.overrides.get(&key) {
            if let Ok(text) = std::fs::read_to_string(path) {
                if let Ok(table) = TwoDa::parse(&text) {
                    return Some(table);
                }
            }
        }
        let stem = key.strip_suffix(".2da").unwrap_or(&key);
        self.rim
            .as_ref()
            .and_then(|rim| rim.get_2da(stem))
            .and_then(|bytes| TwoDa::parse_binary(bytes).ok())
    }
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

    #[test]
    fn style_kind_follows_the_label_prefix() {
        assert_eq!(style_kind("Martial_Legendary_Strike"), StyleKind::Martial);
        assert_eq!(style_kind("Weapon_Staff"), StyleKind::Weapon);
        assert_eq!(style_kind("Magic_Fire"), StyleKind::Magic);
        assert_eq!(style_kind("Support_Storm_Dragon"), StyleKind::Support);
        assert_eq!(style_kind("Transformation_Bull"), StyleKind::Transformation);
        assert_eq!(style_kind("Improvised_Legs"), StyleKind::Improvised);
        assert_eq!(style_kind("Unique_Spirit_Thief"), StyleKind::Unique);
        assert_eq!(style_kind("EMPEROR_STAFF"), StyleKind::Other);
        assert_eq!(StyleKind::Support.label(), "support");
    }

    #[test]
    fn development_items_are_not_playable() {
        let item = |label: &str, name: &str| ItemInfo {
            id: 1,
            label: label.to_owned(),
            name: name.to_owned(),
            description: None,
            category: None,
            save_type: None,
            cost: 0,
            icon: None,
        };
        assert!(is_playable_item(&item("sword_01", "Sword")));
        assert!(!is_playable_item(&item("temp_ring", "Ring")));
        assert!(!is_playable_item(&item("TEMP_Ring", "[TEMP] ring")));
        assert!(!is_playable_item(&item("zzz_unused", "Unused")));
        assert!(!is_playable_item(&item("cut_item", "Cut")));
    }
}
