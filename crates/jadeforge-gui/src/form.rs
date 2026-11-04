//! The editable character form.
//!
//! The text inputs are bound to individual states so typing is cheap; the form
//! is converted to a [`CharacterForm`] when it is applied.

use freya::prelude::*;

use crate::session::CharacterForm;

/// State handles of every text input of the character editor.
#[derive(Clone, Copy)]
pub struct Form {
    /// Character name.
    pub name: State<String>,
    /// Level.
    pub level: State<String>,
    /// Experience points.
    pub experience: State<String>,
    /// Gold.
    pub gold: State<String>,
    /// Unspent attribute points.
    pub attribute_points: State<String>,
    /// Current health.
    pub health: State<String>,
    /// Maximum health.
    pub health_max: State<String>,
    /// Current chi.
    pub chi: State<String>,
    /// Maximum chi.
    pub chi_max: State<String>,
    /// Current focus.
    pub focus: State<String>,
    /// Maximum focus.
    pub focus_max: State<String>,
    /// Body attribute.
    pub body: State<String>,
    /// Mind attribute.
    pub mind: State<String>,
    /// Spirit attribute.
    pub spirit: State<String>,
    /// Charm skill.
    pub charm: State<String>,
    /// Intuition skill.
    pub intuition: State<String>,
    /// Intimidate skill.
    pub intimidate: State<String>,
    /// Alignment.
    pub alignment: State<String>,
    /// Story chapter.
    pub chapter: State<String>,
    /// Kills.
    pub kills: State<String>,
}

impl Default for Form {
    fn default() -> Self {
        Self::new()
    }
}

impl Form {
    /// Creates all state handles. Must be called in a stable order.
    pub fn new() -> Self {
        Self {
            name: use_state(String::new),
            level: use_state(String::new),
            experience: use_state(String::new),
            gold: use_state(String::new),
            attribute_points: use_state(String::new),
            health: use_state(String::new),
            health_max: use_state(String::new),
            chi: use_state(String::new),
            chi_max: use_state(String::new),
            focus: use_state(String::new),
            focus_max: use_state(String::new),
            body: use_state(String::new),
            mind: use_state(String::new),
            spirit: use_state(String::new),
            charm: use_state(String::new),
            intuition: use_state(String::new),
            intimidate: use_state(String::new),
            alignment: use_state(String::new),
            chapter: use_state(String::new),
            kills: use_state(String::new),
        }
    }

    /// Fills every input from a character form.
    pub fn load(&self, form: &CharacterForm) {
        let assign = |mut state: State<String>, value: &String| {
            state.write().clone_from(value);
        };
        assign(self.name, &form.name);
        assign(self.level, &form.level);
        assign(self.experience, &form.experience);
        assign(self.gold, &form.gold);
        assign(self.attribute_points, &form.attribute_points);
        assign(self.health, &form.health);
        assign(self.health_max, &form.health_max);
        assign(self.chi, &form.chi);
        assign(self.chi_max, &form.chi_max);
        assign(self.focus, &form.focus);
        assign(self.focus_max, &form.focus_max);
        assign(self.body, &form.body);
        assign(self.mind, &form.mind);
        assign(self.spirit, &form.spirit);
        assign(self.charm, &form.charm);
        assign(self.intuition, &form.intuition);
        assign(self.intimidate, &form.intimidate);
        assign(self.alignment, &form.alignment);
        assign(self.chapter, &form.chapter);
        assign(self.kills, &form.kills);
    }

    /// Reads the current input values.
    pub fn read(&self) -> CharacterForm {
        let value = |state: State<String>| state.read().clone();
        CharacterForm {
            name: value(self.name),
            level: value(self.level),
            experience: value(self.experience),
            gold: value(self.gold),
            attribute_points: value(self.attribute_points),
            health: value(self.health),
            health_max: value(self.health_max),
            chi: value(self.chi),
            chi_max: value(self.chi_max),
            focus: value(self.focus),
            focus_max: value(self.focus_max),
            body: value(self.body),
            mind: value(self.mind),
            spirit: value(self.spirit),
            charm: value(self.charm),
            intuition: value(self.intuition),
            intimidate: value(self.intimidate),
            alignment: value(self.alignment),
            chapter: value(self.chapter),
            kills: value(self.kills),
        }
    }

    /// True when the inputs differ from the character in the savegame.
    pub fn differs_from(&self, character: &jadeforge_core::Character) -> bool {
        self.read() != CharacterForm::from_character(character)
    }
}
