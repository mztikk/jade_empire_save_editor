//! The character tab: identity, vitals, attributes and progression.

use freya::prelude::*;

use crate::app::Ctx;
use crate::form::Form;
use crate::i18n::{fill, t};
use crate::session::{CharacterField, FieldError, format};
use crate::theme;
use crate::widgets;

/// Two lines of hint text at [`theme::font::caption()`].
///
/// The hint changes between one and two lines while the alignment is typed;
/// reserving both lines keeps the cards below it from jumping. A function so
/// the reserved height follows the user's text scale.
fn alignment_hint_height() -> f32 {
    theme::font::line_height() * 2.0
}

/// Explains the numeric alignment the way the game names it.
#[must_use]
pub fn alignment_hint(value: &str) -> String {
    match value.trim().parse::<i32>() {
        Ok(value) => fill(
            "{alignment} (the game starts at {default}, below is Closed Fist, above is Open Palm).",
            &[
                ("alignment", format::alignment(value)),
                ("default", &jadeforge_core::DEFAULT_ALIGNMENT.to_string()),
            ],
        ),
        Err(_) => t("Alignment is a number; below 128 is the Closed Fist, above is the Open Palm.")
            .to_owned(),
    }
}

/// Renders the character tab.
#[allow(clippy::too_many_lines)] // one declarative tree keeps the visual order auditable
pub fn view(ctx: &Ctx, character: &jadeforge_core::Character, errors: &[FieldError]) -> Element {
    let form = ctx.form;

    let identity = widgets::card(t("Identity"),
        Some(t("Every valid change is applied in memory immediately").to_owned()),
        vec![
            // Three fields per row at every supported width: six narrow fields
            // of fixed width used to overflow the card and leave KILLS alone on
            // a second row.
            widgets::field_grid(vec![
                checked(&form, CharacterField::Name, errors),
                checked(&form, CharacterField::Level, errors),
                checked(&form, CharacterField::Experience, errors),
            ]),
            widgets::field_grid(vec![
                widgets::field_checked(
                    CharacterField::Gold,
                    input(&form, CharacterField::Gold).placeholder(t("not set")),
                    widgets::field_error(errors, CharacterField::Gold),
                ),
                checked(&form, CharacterField::Chapter, errors),
                checked(&form, CharacterField::Kills, errors),
            ]),
            widgets::hint(
                t("Gold is optional in the savegame: leave it empty to keep the field absent, or \
                 enter a value to create it when you save."),
            ),
            widgets::divider(),
            widgets::field_grid(vec![
                checked(&form, CharacterField::Alignment, errors),
                checked(&form, CharacterField::Appearance, errors),
                checked(&form, CharacterField::AttributePoints, errors),
            ]),
            alignment_note(alignment_hint(&form.alignment.read())),
            rect()
                .horizontal()
                .spacing(theme::space::md())
                .child(quick_add(ctx, t("+1 point"), 1))
                .child(quick_add(ctx, t("+5 points"), 5))
                .child(widgets::hint(t("Attribute points you can still spend; the field is created when the savegame does not have it."),
                ))
                .into_element(),
        ],
    );

    let vitals = widgets::card(
        t("Vitals"),
        None,
        vec![
            widgets::field_grid(vec![
                checked(&form, CharacterField::Health, errors),
                checked(&form, CharacterField::HealthMax, errors),
                checked(&form, CharacterField::Chi, errors),
                checked(&form, CharacterField::ChiMax, errors),
            ]),
            widgets::divider(),
            widgets::field_grid(vec![
                checked(&form, CharacterField::Focus, errors),
                checked(&form, CharacterField::FocusMax, errors),
            ]),
            widgets::hint(t(
                "Current values may stay below the maximum; the game fills them on rest.",
            )),
        ],
    );

    let attributes = widgets::card(
        t("Attributes"),
        None,
        vec![
            widgets::field_grid(vec![
                checked(&form, CharacterField::Body, errors),
                checked(&form, CharacterField::Mind, errors),
                checked(&form, CharacterField::Spirit, errors),
            ]),
            widgets::divider(),
            widgets::field_grid(vec![
                checked(&form, CharacterField::Charm, errors),
                checked(&form, CharacterField::Intuition, errors),
                checked(&form, CharacterField::Intimidate, errors),
            ]),
        ],
    );

    let summary = widgets::card(
        t("Current sheet"),
        None,
        vec![
            widgets::label_row(
                t("Name"),
                widgets::a11y_text(character.name.clone())
                    .font_size(theme::font::body())
                    .color(theme::text()),
            ),
            widgets::divider(),
            widgets::label_row(
                t("Level"),
                widgets::a11y_text(character.level.to_string())
                    .font_size(theme::font::body())
                    .color(theme::text()),
            ),
            widgets::divider(),
            widgets::label_row(
                t("Time played"),
                widgets::a11y_text(crate::session::format::duration(character.time_played))
                    .font_size(theme::font::body())
                    .color(theme::text()),
            ),
        ],
    );

    rect()
        .vertical()
        .width(Size::fill())
        .spacing(theme::space::xxl())
        .child(identity)
        .child(vitals)
        .child(attributes)
        .child(summary)
        .into_element()
}

/// One character input bound to its form field and accessibility id.
fn input(form: &Form, field: CharacterField) -> Input {
    Input::new(form.state(field))
        .width(Size::fill())
        .a11y_id(form.id(field))
}

/// A field with its caption, range or validation message and error border.
fn checked(form: &Form, field: CharacterField, errors: &[FieldError]) -> Element {
    widgets::field_checked(
        field,
        input(form, field),
        widgets::field_error(errors, field),
    )
}

/// The alignment explanation in a fixed-height slot so typing cannot reflow
/// the cards below it.
fn alignment_note(text: String) -> Element {
    rect()
        .vertical()
        .width(Size::fill())
        .height(Size::px(alignment_hint_height()))
        .main_align(Alignment::Center)
        .child(
            widgets::a11y_text(text)
                .font_size(theme::font::caption())
                .max_lines(2)
                .color(theme::dim()),
        )
        .into_element()
}

/// A small button that adds attribute points to the form.
fn quick_add(ctx: &Ctx, text: &'static str, amount: i64) -> Element {
    let ctx = ctx.clone();
    Button::new()
        .compact()
        .child(text)
        .on_press(move |_| {
            let mut state = ctx.form.attribute_points;
            let current: i64 = state.read().trim().parse().unwrap_or(0);
            *state.write() = (current + amount).clamp(0, 1000).to_string();
        })
        .into_element()
}
