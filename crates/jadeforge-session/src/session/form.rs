//! The editable character form.

use std::fmt;

use jadeforge_core::Character;

use crate::session::format;

/// A character form value that could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormError(String);

impl FormError {
    /// Human-readable message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.0
    }

    /// True when the message contains `needle`.
    #[must_use]
    pub fn contains(&self, needle: &str) -> bool {
        self.0.contains(needle)
    }
}

impl fmt::Display for FormError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl std::error::Error for FormError {}

/// The editable values of the character form.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CharacterForm {
    /// First name.
    pub name: String,
    /// Level.
    pub level: String,
    /// Experience points.
    pub experience: String,
    /// Gold.
    pub gold: String,
    /// Unspent attribute points.
    pub attribute_points: String,
    /// Current health.
    pub health: String,
    /// Maximum health.
    pub health_max: String,
    /// Current chi.
    pub chi: String,
    /// Maximum chi.
    pub chi_max: String,
    /// Current focus.
    pub focus: String,
    /// Maximum focus.
    pub focus_max: String,
    /// Body.
    pub body: String,
    /// Mind.
    pub mind: String,
    /// Spirit.
    pub spirit: String,
    /// Charm.
    pub charm: String,
    /// Intuition.
    pub intuition: String,
    /// Intimidate.
    pub intimidate: String,
    /// Alignment.
    pub alignment: String,
    /// Appearance index.
    pub appearance: String,
    /// Chapter.
    pub chapter: String,
    /// Kills.
    pub kills: String,
}

impl CharacterForm {
    /// Fills the form from a character.
    #[must_use]
    pub fn from_character(character: &Character) -> Self {
        Self {
            name: character.name.clone(),
            level: character.level.to_string(),
            experience: character.experience.to_string(),
            gold: character.gold.unwrap_or(0).to_string(),
            attribute_points: character.attribute_points.to_string(),
            health: format::exact(character.health),
            health_max: character.health_max.to_string(),
            chi: format::exact(character.chi),
            chi_max: character.chi_max.to_string(),
            focus: format::exact(character.focus),
            focus_max: format::exact(character.focus_max),
            body: character.body.to_string(),
            mind: character.mind.to_string(),
            spirit: character.spirit.to_string(),
            charm: character.charm.to_string(),
            intuition: character.intuition.to_string(),
            intimidate: character.intimidate.to_string(),
            alignment: character.alignment.to_string(),
            appearance: character.appearance.to_string(),
            chapter: character.chapter.to_string(),
            kills: character.kills.to_string(),
        }
    }

    /// Builds a character by applying the form on top of a base character.
    pub fn to_character(&self, base: &Character) -> Result<Character, FormError> {
        let mut character = base.clone();
        character.name.clone_from(&self.name);
        character.level = parse::<u8>("level", &self.level)?;
        character.experience = parse::<u32>("experience", &self.experience)?;
        character.gold = Some(parse::<u32>("gold", &self.gold)?);
        character.attribute_points = parse::<u32>("attribute points", &self.attribute_points)?;
        character.health = parse_float("health", &self.health)?;
        character.health_max = parse::<u32>("maximum health", &self.health_max)?;
        character.chi = parse_float("chi", &self.chi)?;
        character.chi_max = parse::<u32>("maximum chi", &self.chi_max)?;
        character.focus = parse_float("focus", &self.focus)?;
        character.focus_max = parse_float("maximum focus", &self.focus_max)?;
        character.body = parse::<u32>("body", &self.body)?;
        character.mind = parse::<u32>("mind", &self.mind)?;
        character.spirit = parse::<u32>("spirit", &self.spirit)?;
        character.charm = parse::<i32>("charm", &self.charm)?;
        character.intuition = parse::<i32>("intuition", &self.intuition)?;
        character.intimidate = parse::<i32>("intimidate", &self.intimidate)?;
        character.alignment = parse::<i32>("alignment", &self.alignment)?;
        character.appearance = parse::<i32>("appearance", &self.appearance)?;
        character.chapter = parse::<u32>("chapter", &self.chapter)?;
        character.kills = parse::<u32>("kills", &self.kills)?;
        Ok(character)
    }
}

fn parse<T: std::str::FromStr>(what: &str, raw: &str) -> Result<T, FormError> {
    raw.trim()
        .parse()
        .map_err(|_| FormError(format!("{what}: {:?} is not a number", raw.trim())))
}

fn parse_float(what: &str, raw: &str) -> Result<f32, FormError> {
    let value: f32 = raw
        .trim()
        .parse()
        .map_err(|_| FormError(format!("{what}: {:?} is not a number", raw.trim())))?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(FormError(format!("{what}: {raw:?} is not a finite number")))
    }
}
