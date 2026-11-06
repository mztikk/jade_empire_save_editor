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
    /// Talk table reference of the description.
    pub desc_ref: Option<u32>,
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

/// A predefined gem from `gems.2da`, joined with its item name.
#[derive(Debug, Clone, PartialEq)]
pub struct GemInfo {
    /// Item id, a row in `items.2da` of category `gems`.
    pub item_id: u16,
    /// Internal label, for example `item_gem_body_I`.
    pub label: String,
    /// Localized name from the talk table.
    pub name: String,
    /// Effects as `(type, value)` pairs.
    pub effects: Vec<(u32, u32)>,
    /// Effect duration in seconds.
    pub duration: f32,
}

/// A loaded game installation: item and style names, style slots, gems and
/// effect names.
#[derive(Debug)]
pub struct GameData {
    root: PathBuf,
    items: Vec<Option<ItemInfo>>,
    styles: Vec<Option<StyleInfo>>,
    advances: Vec<Option<Advance>>,
    style_choices: Vec<Option<Vec<StyleChoice>>>,
    gems: Vec<GemInfo>,
    effects: Vec<Option<String>>,
    search_index: HashMap<u16, String>,
    warnings: Vec<String>,
}

/// One line of `StyleAdvance.2da`, the table that style upgrade slots point at.
///
/// Bit *i* of a style's upgrade mask selects slot *i + 1* of the style's own
/// `<Style>LV.2da` table; that slot names one of these rows. The saved level is
/// `1 + number of set bits`. Rows come in families of five tiers with costs
/// 1, 2, 4, 6 and 10.
#[derive(Debug, Clone, PartialEq)]
pub struct Advance {
    /// Row in `StyleAdvance.2da`; a level-up slot points at it.
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

/// One purchasable slot of a style's level-up table.
///
/// The savegame mask is indexed by the **slot**, not by the `StyleAdvance` row:
/// bit `slot - 1` is set when the player bought that entry. The slot points at
/// an advance row, which supplies the effect, cost and description. This was
/// verified against real savegames: Storm Dragon's mask uses slots of its own
/// table whose advance rows (6-10, 21-25, 11-15) would otherwise contradict the
/// saved bits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleChoice {
    /// 1-based position in the style's flattened level-up table.
    pub slot: u16,
    /// Row in `StyleAdvance.2da` this slot grants.
    pub advance: u16,
    /// Localized family name (the `Label` of the level-up table row).
    pub family: String,
    /// Tier inside the family (1 based).
    pub tier: u8,
}

/// Turns an `effects.2da` label into something readable.
///
/// `EFFECT_MODIFYBODYBASE` becomes `Body`, `EFFECT_DAMAGEARMOR` becomes
/// `Damage armor`; unknown labels fall back to a title-cased version of the
/// label with underscores replaced by spaces.
#[must_use]
pub fn pretty_effect_name(label: &str) -> String {
    let name = label
        .strip_prefix("EFFECT_")
        .unwrap_or(label)
        .to_uppercase();
    for (keyword, display) in [
        ("BODY", "Body"),
        ("MIND", "Mind"),
        ("SPIRIT", "Spirit"),
        ("CHARM", "Charm"),
        ("INTUITION", "Intuition"),
        ("INTIMIDATE", "Intimidate"),
        ("FOCUS", "Focus"),
        ("HEALTH", "Health"),
        ("CHI", "Chi"),
        ("DAMAGEARMOR", "Damage armor"),
    ] {
        if name.contains(keyword) {
            return display.to_owned();
        }
    }
    let mut words = String::new();
    for (index, part) in name.split('_').enumerate() {
        if index > 0 {
            words.push(' ');
        }
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            words.push(first);
            words.extend(chars.flat_map(char::to_lowercase));
        }
    }
    words
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

thread_local! {
    /// Game root installed by the test harness for the current thread.
    static TEST_ROOT: std::cell::RefCell<Option<PathBuf>> =
        const { std::cell::RefCell::new(None) };
}

/// Installs a game root for the current thread (tests only).
///
/// This lets the UI tests use the real installation without mutating the
/// process environment (which is unsafe and racy with parallel tests). The
/// installed root is authoritative, so a broken fixture is reported instead of
/// falling back to `JADEFORGE_GAME_DIR` or the save folder.
#[doc(hidden)]
pub fn set_test_game_root(root: Option<PathBuf>) {
    TEST_ROOT.with(|cell| *cell.borrow_mut() = root);
}

impl GameData {
    /// Tries to find the game installation that a savegame belongs to.
    ///
    /// `save_path` is the `save.sac` file, or the folder that contains it.
    /// `JADEFORGE_GAME_DIR` wins over the discovery.
    #[must_use]
    pub fn discover(save_path: &Path) -> Option<Self> {
        Self::discover_result(save_path).ok().flatten()
    }

    /// Like [`GameData::discover`], but keeps the error of an override or game
    /// root that exists and cannot be read. `Ok(None)` means no installation
    /// was found; `Err` means one was found and is broken.
    pub fn discover_result(save_path: &Path) -> Result<Option<Self>> {
        let mut last_error = None;
        if let Some(root) = TEST_ROOT.with(|cell| cell.borrow().clone()) {
            return Self::open(root).map(Some);
        }
        if let Some(directory) = std::env::var_os("JADEFORGE_GAME_DIR") {
            match Self::open(PathBuf::from(directory)) {
                Ok(game) => return Ok(Some(game)),
                Err(error) => last_error = Some(error),
            }
        }
        if let Some(game) = Self::discover_beside(save_path) {
            return Ok(Some(game));
        }
        match last_error {
            Some(error) => Err(error),
            None => Ok(None),
        }
    }

    /// Walks up from the save folder looking for the installation.
    ///
    /// A regular installation is `<game>/Save` or `<game>/Save/<savegame>`, so
    /// the game root is one or two levels above the selected folder. The
    /// environment override is handled by [`GameData::discover`].
    fn discover_beside(save_path: &Path) -> Option<Self> {
        // `Path::ancestors` is lexical, so a relative path such as `save.sac`
        // or `Auto 1` would never walk to the real game folder. Anchor it at
        // the current directory first (without resolving symlinks).
        let save_path = if save_path.is_absolute() {
            save_path.to_path_buf()
        } else {
            std::env::current_dir().ok()?.join(save_path)
        };
        let folder = if save_path.is_dir() {
            save_path.as_path()
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

        // Try both files until one parses. A corrupt `dialog.tlk` must not hide
        // a valid `dialogf.tlk`.
        let talktable = ["dialog.tlk", "dialogf.tlk"]
            .iter()
            .map(|name| root.join(name))
            .filter(|path| path.is_file())
            .find_map(|path| TalkTable::open(path).ok());

        let source = TableSource::new(&root);
        let mut warnings = Vec::new();
        let items = source.load("items.2da", &mut warnings);
        let styles = source.load("stylesuperlist.2da", &mut warnings);
        let advances = source.load("StyleAdvance.2da", &mut warnings);
        let style_data = source.load("styledata.2da", &mut warnings);
        let gem_table = source.load("gems.2da", &mut warnings);
        let effects_table = source.load("effects.2da", &mut warnings);

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
                    .and_then(|reference| u32::try_from(reference).ok())
                    .and_then(|reference| talk.get(reference))
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
                    description: description_ref
                        .and_then(|reference| u32::try_from(reference).ok())
                        .and_then(|reference| talk.get(reference)),
                    save_type: category.as_deref().and_then(save_type_for_category),
                    cost: table
                        .uint(index, "COST")
                        .and_then(|value| u32::try_from(value).ok())
                        .unwrap_or(0),
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
                let Some(name) = name_ref
                    .and_then(|reference| u32::try_from(reference).ok())
                    .and_then(|reference| talk.get(reference))
                else {
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
                    desc_ref: table
                        .uint(index, "DescRef")
                        .and_then(|reference| u32::try_from(reference).ok()),
                    description: table
                        .uint(index, "DescRef")
                        .and_then(|reference| u32::try_from(reference).ok())
                        .and_then(|reference| talk.get(reference)),
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

        let mut effect_names: Vec<Option<String>> = Vec::new();
        if let Some(table) = &effects_table {
            effect_names.resize(table.len(), None);
            for (index, _) in table.iter() {
                let label = table.cell(index, "Label").unwrap_or_default();
                if label.is_empty() || label.eq_ignore_ascii_case("EFFECT_INVALID") {
                    continue;
                }
                effect_names[index] = Some(pretty_effect_name(label));
            }
        }

        let mut gem_infos: Vec<GemInfo> = Vec::new();
        if let Some(table) = &gem_table {
            let mut by_label: HashMap<String, (Vec<(u32, u32)>, f32)> = HashMap::new();
            for (index, row) in table.iter() {
                let label = table
                    .cell(index, "Label")
                    .unwrap_or_default()
                    .to_lowercase();
                let mut effects = Vec::new();
                let mut duration = 0.0_f32;
                for slot in 0..3 {
                    let effect = table
                        .uint(index, &format!("Effect{slot}"))
                        .and_then(|value| u32::try_from(value).ok())
                        .unwrap_or(0);
                    let parameter = table
                        .row(index)
                        .and_then(|row| row.float(table.column(&format!("Parameter{slot}"))?))
                        .unwrap_or(0.0);
                    if effect != 0 {
                        effects.push((effect, parameter.max(0.0) as u32));
                    }
                    if slot == 0 {
                        duration = table
                            .row(index)
                            .and_then(|row| row.float(table.column("Duration0")?))
                            .unwrap_or(0.0);
                    }
                }
                let _ = row;
                by_label.insert(label, (effects, duration));
            }
            for item in item_slots.iter().flatten() {
                if item.category.as_deref() != Some("gems") {
                    continue;
                }
                if let Some((effects, duration)) = by_label.get(&item.label.to_lowercase()) {
                    gem_infos.push(GemInfo {
                        item_id: item.id,
                        label: item.label.clone(),
                        name: item.name.clone(),
                        effects: effects.clone(),
                        duration: *duration,
                    });
                }
            }
            gem_infos.sort_by(|left, right| left.name.cmp(&right.name));
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
                        .and_then(|reference| u32::try_from(reference).ok())
                        .and_then(|reference| talktable.as_ref()?.get(reference))
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
        let mut style_choices: Vec<Option<Vec<StyleChoice>>> = vec![None; style_slots.len()];
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
                if let Some(table) = source.load(&format!("{table_name}.2da"), &mut warnings) {
                    style_choices[index] = Some(level_up_choices(&table, talktable.as_ref()));
                }
            }
        }

        Ok(Self {
            root,
            items: item_slots,
            styles: style_slots,
            advances: advance_slots,
            style_choices,
            gems: gem_infos,
            effects: effect_names,
            search_index,
            warnings,
        })
    }

    /// Root directory of the installation.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Tables that were found but could not be parsed.
    ///
    /// A malformed table degrades to numeric ids just like a missing one, but
    /// the editor can now say that the file exists and is broken instead of
    /// pretending the installation has no data.
    #[must_use]
    pub fn warnings(&self) -> &[String] {
        &self.warnings
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

    /// Purchasable slots of a style, from its `<Style>LV.2da` table.
    ///
    /// `None` when the installation ships no table for the style; the editor
    /// then leaves the style read-only instead of guessing slot semantics.
    #[must_use]
    pub fn style_choices(&self, style_id: u16) -> Option<&[StyleChoice]> {
        self.style_choices
            .get(usize::from(style_id))
            .and_then(Option::as_ref)
            .map(Vec::as_slice)
    }

    /// Predefined gems from `gems.2da`, sorted by name.
    #[must_use]
    pub fn gems(&self) -> &[GemInfo] {
        &self.gems
    }

    /// Looks a predefined gem up by item id.
    #[must_use]
    pub fn gem(&self, item_id: u16) -> Option<&GemInfo> {
        self.gems.iter().find(|gem| gem.item_id == item_id)
    }

    /// Human readable name of an effect id, for example `Body`.
    #[must_use]
    pub fn effect_name(&self, effect: u32) -> Option<&str> {
        self.effects.get(effect as usize).and_then(Option::as_deref)
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
    /// Results whose name starts with the query are ranked first; within each
    /// group the order is name, then id.
    #[must_use]
    pub fn search_items(&self, query: &str, limit: usize) -> Vec<&ItemInfo> {
        let query = query.trim().to_lowercase();
        if query.is_empty() {
            return Vec::new();
        }
        let mut matches: Vec<(bool, &ItemInfo, String)> = Vec::new();
        for (id, haystack) in &self.search_index {
            if haystack.contains(&query)
                && let Some(item) = self.item(*id).filter(|item| is_playable_item(item))
            {
                let sort_name = item.name.to_lowercase();
                matches.push((sort_name.starts_with(&query), item, sort_name));
            }
        }
        matches.sort_by(|left, right| {
            right
                .0
                .cmp(&left.0)
                .then_with(|| left.2.cmp(&right.2))
                .then_with(|| left.1.id.cmp(&right.1.id))
        });
        matches
            .into_iter()
            .take(limit)
            .map(|(_, item, _)| item)
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

/// Flattens a `<Style>LV.2da` table into purchasable slots.
///
/// Each row of the table is one upgrade family (damage, speed, ...) with up to
/// five tiers; the row's `Label` names the family. The slot number is the
/// position in that flattened list and is what the savegame mask indexes.
fn level_up_choices(table: &TwoDa, talk: Option<&TalkTable>) -> Vec<StyleChoice> {
    let choice_columns: Vec<usize> = ["Choice_1", "Choice_2", "Choice_3", "Choice_4", "Choice_5"]
        .iter()
        .filter_map(|name| table.column(name))
        .collect();
    if choice_columns.is_empty() {
        return Vec::new();
    }
    let mut choices = Vec::new();
    for (row_index, row) in table.iter() {
        let family = table
            .uint(row_index, "Label")
            .and_then(|reference| u32::try_from(reference).ok())
            .and_then(|reference| talk.and_then(|talk| talk.get(reference)))
            .unwrap_or_else(|| format!("Upgrades {}", row_index + 1));
        let mut tier = 0_u8;
        for column in &choice_columns {
            let Some(value) = row.uint(*column) else {
                continue;
            };
            let Ok(advance) = u16::try_from(value) else {
                continue;
            };
            if advance == 0 {
                continue;
            }
            tier = tier.saturating_add(1);
            let slot = choices.len() + 1;
            if slot > 64 {
                return choices;
            }
            choices.push(StyleChoice {
                slot: u16::try_from(slot).unwrap_or(64),
                advance,
                family: family.clone(),
                tier,
            });
        }
    }
    choices
}

/// Where the 2DA tables come from: `override/` first, the RIM archives second.
struct TableSource {
    overrides: HashMap<String, PathBuf>,
    rims: Vec<RimArchive>,
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
        let rims = ["miniglobal-a.rim", "globalunload-a.rim"]
            .iter()
            .map(|name| root.join("data").join(name))
            .filter(|path| path.is_file())
            .filter_map(|path| RimArchive::open(path).ok())
            .collect();
        Self { overrides, rims }
    }

    /// Loads a table by file name, honouring `override/` before the archive.
    fn load(&self, name: &str, warnings: &mut Vec<String>) -> Option<TwoDa> {
        let key = name.to_lowercase();
        if let Some(path) = self.overrides.get(&key) {
            // Mod tables are not always UTF-8; decoding lossily keeps them usable.
            match std::fs::read(path) {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    match TwoDa::parse(&text) {
                        Ok(table) => return Some(table),
                        Err(error) => warnings.push(format!("{name}: {error}")),
                    }
                }
                Err(error) => warnings.push(format!("{name}: {error}")),
            }
        }
        let stem = key.strip_suffix(".2da").unwrap_or(&key);
        let bytes = self.rims.iter().find_map(|rim| rim.get_2da(stem))?;
        match TwoDa::parse_binary(bytes) {
            Ok(table) => Some(table),
            Err(error) => {
                warnings.push(format!("{name}: {error}"));
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_root_override_is_used_before_the_environment() {
        let root = std::env::temp_dir().join(format!(
            "jadeforge-test-root-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |duration| duration.as_nanos())
        ));
        std::fs::create_dir_all(root.join("override")).expect("create game layout");
        set_test_game_root(Some(root.clone()));
        let game = GameData::discover(Path::new("/definitely/not/a/save"))
            .expect("the test override supplies the game root");
        assert_eq!(game.root(), root.as_path());
        set_test_game_root(None);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn discovery_finds_the_game_root_above_the_save_folder() {
        let root = std::env::temp_dir().join(format!(
            "jadeforge-discover-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |duration| duration.as_nanos())
        ));
        let game = root.join("Jade Empire");
        let save = game.join("Save");
        std::fs::create_dir_all(game.join("override")).expect("create game layout");
        std::fs::create_dir_all(save.join("Auto 1")).expect("create save folder");
        std::fs::write(save.join("Auto 1").join("save.sac"), b"").expect("write save");

        let discovered = GameData::discover_beside(&save).expect("game above Save");
        assert_eq!(discovered.root(), game.as_path());
        let from_savegame =
            GameData::discover_beside(&save.join("Auto 1")).expect("game above savegame");
        assert_eq!(from_savegame.root(), game.as_path());
        let from_file = GameData::discover_beside(&save.join("Auto 1").join("save.sac"))
            .expect("game above save file");
        assert_eq!(from_file.root(), game.as_path());

        let unrelated = root.join("Documents");
        std::fs::create_dir_all(&unrelated).expect("create unrelated folder");
        assert!(GameData::discover_beside(&unrelated).is_none());

        let _ = std::fs::remove_dir_all(root);
    }

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
    #[test]
    fn a_broken_override_is_reported_instead_of_hidden() {
        let root = std::env::temp_dir().join(format!(
            "jadeforge-broken-root-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |duration| duration.as_nanos())
        ));
        std::fs::create_dir_all(&root).expect("create broken root");
        set_test_game_root(Some(root.clone()));
        let result = GameData::discover_result(Path::new("/definitely/not/a/save"));
        set_test_game_root(None);
        let _ = std::fs::remove_dir_all(root);
        assert!(result.is_err(), "a found but broken root must be an error");
    }

    #[test]
    fn a_broken_test_root_is_not_hidden_by_the_save_folder() {
        let broken = std::env::temp_dir().join(format!(
            "jadeforge-broken-root-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |duration| duration.as_nanos())
        ));
        let game = std::env::temp_dir().join(format!(
            "jadeforge-save-folder-game-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |duration| duration.as_nanos())
        ));
        std::fs::create_dir_all(&broken).expect("create broken root");
        std::fs::create_dir_all(game.join("override")).expect("create game layout");
        set_test_game_root(Some(broken.clone()));
        let result = GameData::discover_result(&game.join("Save").join("Auto 1").join("save.sac"));
        set_test_game_root(None);
        let _ = std::fs::remove_dir_all(broken);
        let _ = std::fs::remove_dir_all(game);
        assert!(
            result.is_err(),
            "a broken test root must not be hidden by the save folder"
        );
    }
}
