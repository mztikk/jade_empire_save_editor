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
    /// Gold, or empty when the savegame has no gold field.
    ///
    /// The field is optional in the format; keeping it empty preserves the
    /// absence instead of silently materialising `Some(0)` and making the form
    /// differ from an untouched savegame.
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
            gold: character
                .gold
                .map_or_else(String::new, |gold| gold.to_string()),
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

    /// Builds a character by applying every field that validates on its own.
    ///
    /// Applying fields independently is what lets the editor keep working with
    /// one mistyped field: the valid edits still reach the in-memory savegame
    /// and can be saved, while the invalid field is reported separately.
    /// Errors are ordered like [`CharacterField::ALL`].
    #[allow(clippy::too_many_lines)] // one field per line keeps the validation readable
    #[must_use]
    pub fn to_character_partial(&self, base: &Character) -> (Character, Vec<FieldError>) {
        let mut character = base.clone();
        let mut errors = Vec::new();

        match check_name(&self.name) {
            Ok(()) => character.name.clone_from(&self.name),
            Err(message) => errors.push(FieldError::new(CharacterField::Name, message)),
        }
        match parse_checked(CharacterField::Level, &self.level, 1, 255) {
            Ok(value) => character.level = value as u8,
            Err(error) => errors.push(error),
        }
        match parse_checked(
            CharacterField::Experience,
            &self.experience,
            0,
            u64::from(u32::MAX),
        ) {
            Ok(value) => character.experience = value as u32,
            Err(error) => errors.push(error),
        }
        if self.gold.trim().is_empty() {
            if base.gold.is_some() {
                errors.push(FieldError::new(
                    CharacterField::Gold,
                    "gold: a value is required because this savegame has a gold field",
                ));
            }
        } else {
            match parse_checked(CharacterField::Gold, &self.gold, 0, u64::from(u32::MAX)) {
                Ok(value) => character.gold = Some(value as u32),
                Err(error) => errors.push(error),
            }
        }
        match parse_checked(
            CharacterField::AttributePoints,
            &self.attribute_points,
            0,
            u64::from(u32::MAX),
        ) {
            Ok(value) => character.attribute_points = value as u32,
            Err(error) => errors.push(error),
        }
        match parse_float_checked(CharacterField::Health, &self.health) {
            Ok(value) => character.health = value,
            Err(error) => errors.push(error),
        }
        match parse_checked(CharacterField::HealthMax, &self.health_max, 1, 1_000_000) {
            Ok(value) => character.health_max = value as u32,
            Err(error) => errors.push(error),
        }
        match parse_float_checked(CharacterField::Chi, &self.chi) {
            Ok(value) => character.chi = value,
            Err(error) => errors.push(error),
        }
        match parse_checked(CharacterField::ChiMax, &self.chi_max, 1, 1_000_000) {
            Ok(value) => character.chi_max = value as u32,
            Err(error) => errors.push(error),
        }
        match parse_float_checked(CharacterField::Focus, &self.focus) {
            Ok(value) => character.focus = value,
            Err(error) => errors.push(error),
        }
        match parse_float_checked(CharacterField::FocusMax, &self.focus_max) {
            Ok(value) => character.focus_max = value,
            Err(error) => errors.push(error),
        }
        match parse_checked(CharacterField::Body, &self.body, 0, 1000) {
            Ok(value) => character.body = value as u32,
            Err(error) => errors.push(error),
        }
        match parse_checked(CharacterField::Mind, &self.mind, 0, 1000) {
            Ok(value) => character.mind = value as u32,
            Err(error) => errors.push(error),
        }
        match parse_checked(CharacterField::Spirit, &self.spirit, 0, 1000) {
            Ok(value) => character.spirit = value as u32,
            Err(error) => errors.push(error),
        }
        match parse_signed_checked(CharacterField::Charm, &self.charm) {
            Ok(value) => character.charm = value,
            Err(error) => errors.push(error),
        }
        match parse_signed_checked(CharacterField::Intuition, &self.intuition) {
            Ok(value) => character.intuition = value,
            Err(error) => errors.push(error),
        }
        match parse_signed_checked(CharacterField::Intimidate, &self.intimidate) {
            Ok(value) => character.intimidate = value,
            Err(error) => errors.push(error),
        }
        match parse_signed_checked(CharacterField::Alignment, &self.alignment) {
            Ok(value) => character.alignment = value,
            Err(error) => errors.push(error),
        }
        match parse_signed_checked(CharacterField::Appearance, &self.appearance) {
            Ok(value) if (0..=1_000_000).contains(&value) => character.appearance = value,
            Ok(_) => errors.push(FieldError::out_of_range(
                CharacterField::Appearance,
                &self.appearance,
            )),
            Err(error) => errors.push(error),
        }
        match parse_checked(CharacterField::Chapter, &self.chapter, 0, 100) {
            Ok(value) => character.chapter = value as u32,
            Err(error) => errors.push(error),
        }
        match parse_checked(CharacterField::Kills, &self.kills, 0, u64::from(u32::MAX)) {
            Ok(value) => character.kills = value as u32,
            Err(error) => errors.push(error),
        }
        (character, errors)
    }

    /// Every field that cannot be applied, in [`CharacterField::ALL`] order.
    #[must_use]
    pub fn validate_against(&self, base: &Character) -> Vec<FieldError> {
        self.to_character_partial(base).1
    }

    /// Builds a character by applying the form on top of a base character.
    ///
    /// Returns the first field error when any field is invalid; use
    /// [`CharacterForm::to_character_partial`] to apply the valid fields of an
    /// otherwise invalid form.
    pub fn to_character(&self, base: &Character) -> Result<Character, FormError> {
        let (character, errors) = self.to_character_partial(base);
        match errors.first() {
            Some(error) => Err(FormError(error.message.clone())),
            None => Ok(character),
        }
    }
}

/// One editable character field, used for per-field validation and focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CharacterField {
    /// Character name.
    Name,
    /// Level.
    Level,
    /// Experience points.
    Experience,
    /// Gold, which is optional in the format.
    Gold,
    /// Unspent attribute points.
    AttributePoints,
    /// Current health.
    Health,
    /// Maximum health.
    HealthMax,
    /// Current chi.
    Chi,
    /// Maximum chi.
    ChiMax,
    /// Current focus.
    Focus,
    /// Maximum focus.
    FocusMax,
    /// Body attribute.
    Body,
    /// Mind attribute.
    Mind,
    /// Spirit attribute.
    Spirit,
    /// Charm skill.
    Charm,
    /// Intuition skill.
    Intuition,
    /// Intimidate skill.
    Intimidate,
    /// Alignment.
    Alignment,
    /// Appearance index.
    Appearance,
    /// Story chapter.
    Chapter,
    /// Kills.
    Kills,
}

impl CharacterField {
    /// Every field, in the order the character tab shows them.
    pub const ALL: [Self; 21] = [
        Self::Name,
        Self::Level,
        Self::Experience,
        Self::Gold,
        Self::AttributePoints,
        Self::Health,
        Self::HealthMax,
        Self::Chi,
        Self::ChiMax,
        Self::Focus,
        Self::FocusMax,
        Self::Body,
        Self::Mind,
        Self::Spirit,
        Self::Charm,
        Self::Intuition,
        Self::Intimidate,
        Self::Alignment,
        Self::Appearance,
        Self::Chapter,
        Self::Kills,
    ];

    /// Position in [`CharacterField::ALL`].
    #[must_use]
    pub fn index(self) -> usize {
        self as usize
    }

    /// Field name as it appears in error messages.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Level => "level",
            Self::Experience => "experience",
            Self::Gold => "gold",
            Self::AttributePoints => "attribute points",
            Self::Health => "health",
            Self::HealthMax => "maximum health",
            Self::Chi => "chi",
            Self::ChiMax => "maximum chi",
            Self::Focus => "focus",
            Self::FocusMax => "maximum focus",
            Self::Body => "body",
            Self::Mind => "mind",
            Self::Spirit => "spirit",
            Self::Charm => "charm",
            Self::Intuition => "intuition",
            Self::Intimidate => "intimidate",
            Self::Alignment => "alignment",
            Self::Appearance => "appearance",
            Self::Chapter => "chapter",
            Self::Kills => "kills",
        }
    }

    /// Caption of the input.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Name => "NAME",
            Self::Level => "LEVEL",
            Self::Experience => "EXPERIENCE",
            Self::Gold => "GOLD",
            Self::AttributePoints => "UNSPENT ATTRIBUTE POINTS",
            Self::Health => "HEALTH",
            Self::HealthMax => "HEALTH MAX",
            Self::Chi => "CHI",
            Self::ChiMax => "CHI MAX",
            Self::Focus => "FOCUS",
            Self::FocusMax => "FOCUS MAX",
            Self::Body => "BODY",
            Self::Mind => "MIND",
            Self::Spirit => "SPIRIT",
            Self::Charm => "CHARM",
            Self::Intuition => "INTUITION",
            Self::Intimidate => "INTIMIDATE",
            Self::Alignment => "ALIGNMENT",
            Self::Appearance => "APPEARANCE",
            Self::Chapter => "CHAPTER",
            Self::Kills => "KILLS",
        }
    }

    /// Accepted values, mirroring the range checks in `jadeforge-core`.
    #[must_use]
    pub fn range(self) -> &'static str {
        match self {
            Self::Name => "at most 64 characters",
            Self::Level => "1 to 255",
            Self::Experience | Self::AttributePoints | Self::Kills => "0 to 4294967295",
            Self::Gold => "empty or 0 to 4294967295",
            Self::Health | Self::Chi | Self::Focus | Self::FocusMax | Self::Appearance => {
                "0 to 1000000"
            }
            Self::HealthMax | Self::ChiMax => "1 to 1000000",
            Self::Body | Self::Mind | Self::Spirit => "0 to 1000",
            Self::Charm | Self::Intuition | Self::Intimidate | Self::Alignment => {
                "-1000000 to 1000000"
            }
            Self::Chapter => "0 to 100",
        }
    }

    /// Raw text of the field in `form`.
    #[must_use]
    pub fn text(self, form: &CharacterForm) -> &str {
        match self {
            Self::Name => &form.name,
            Self::Level => &form.level,
            Self::Experience => &form.experience,
            Self::Gold => &form.gold,
            Self::AttributePoints => &form.attribute_points,
            Self::Health => &form.health,
            Self::HealthMax => &form.health_max,
            Self::Chi => &form.chi,
            Self::ChiMax => &form.chi_max,
            Self::Focus => &form.focus,
            Self::FocusMax => &form.focus_max,
            Self::Body => &form.body,
            Self::Mind => &form.mind,
            Self::Spirit => &form.spirit,
            Self::Charm => &form.charm,
            Self::Intuition => &form.intuition,
            Self::Intimidate => &form.intimidate,
            Self::Alignment => &form.alignment,
            Self::Appearance => &form.appearance,
            Self::Chapter => &form.chapter,
            Self::Kills => &form.kills,
        }
    }
}

/// A character field whose text cannot be applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldError {
    /// The field that failed.
    pub field: CharacterField,
    /// Human-readable reason, including the field name.
    pub message: String,
}

impl FieldError {
    /// Creates an error for a field.
    #[must_use]
    pub fn new(field: CharacterField, message: impl Into<String>) -> Self {
        Self {
            field,
            message: message.into(),
        }
    }

    fn out_of_range(field: CharacterField, raw: &str) -> Self {
        Self::new(
            field,
            format!(
                "{}: {:?} is outside {}",
                field.key(),
                raw.trim(),
                field.range()
            ),
        )
    }
}

fn check_name(name: &str) -> Result<(), String> {
    if name.chars().count() > 64 || name.chars().any(char::is_control) {
        return Err(format!(
            "name: at most 64 printable characters ({} now)",
            name.chars().count()
        ));
    }
    Ok(())
}

fn parse_checked(field: CharacterField, raw: &str, min: u64, max: u64) -> Result<u64, FieldError> {
    let value: u64 = raw.trim().parse().map_err(|_| {
        FieldError::new(
            field,
            format!("{}: {:?} is not a number", field.key(), raw.trim()),
        )
    })?;
    if value < min || value > max {
        return Err(FieldError::out_of_range(field, raw));
    }
    Ok(value)
}

fn parse_signed_checked(field: CharacterField, raw: &str) -> Result<i32, FieldError> {
    let value: i32 = raw.trim().parse().map_err(|_| {
        FieldError::new(
            field,
            format!("{}: {:?} is not a number", field.key(), raw.trim()),
        )
    })?;
    if !(-1_000_000..=1_000_000).contains(&value) {
        return Err(FieldError::out_of_range(field, raw));
    }
    Ok(value)
}

fn parse_float_checked(field: CharacterField, raw: &str) -> Result<f32, FieldError> {
    let value: f32 = raw.trim().parse().map_err(|_| {
        FieldError::new(
            field,
            format!("{}: {:?} is not a number", field.key(), raw.trim()),
        )
    })?;
    if !value.is_finite() || !(0.0..=1_000_000.0).contains(&value) {
        return Err(FieldError::out_of_range(field, raw));
    }
    Ok(value)
}
