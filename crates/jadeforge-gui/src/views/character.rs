//! The character tab: identity, vitals, attributes and progression.

use freya::prelude::*;

use crate::app::Ctx;
use crate::form::Form;
use crate::session::{CharacterField, FieldError, format};
use crate::theme;
use crate::widgets;

/// Two lines of hint text at [`theme::font::CAPTION`].
///
/// The hint changes between one and two lines while the alignment is typed;
/// reserving both lines keeps the cards below it from jumping.
const ALIGNMENT_HINT_HEIGHT: f32 = theme::font::LINE_HEIGHT * 2.0;

/// Explains the numeric alignment the way the game names it.
#[must_use]
pub fn alignment_hint(value: &str) -> String {
    match value.trim().parse::<i32>() {
        Ok(value) => format!(
            "{} (the game starts at {}, below is Closed Fist, above is Open Palm).",
            format::alignment(value),
            jadeforge_core::DEFAULT_ALIGNMENT
        ),
        Err(_) => "Alignment is a number; below 128 is the Closed Fist, above is the Open Palm."
            .to_owned(),
    }
}

/// Renders the character tab.
#[allow(clippy::too_many_lines)] // one declarative tree keeps the visual order auditable
pub fn view(ctx: &Ctx, character: &jadeforge_core::Character, errors: &[FieldError]) -> Element {
    let form = ctx.form;

    let identity = widgets::card(
        "Identity",
        Some("Every valid change is applied in memory immediately".to_owned()),
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
                    input(&form, CharacterField::Gold).placeholder("not set"),
                    widgets::field_error(errors, CharacterField::Gold),
                ),
                checked(&form, CharacterField::Chapter, errors),
                checked(&form, CharacterField::Kills, errors),
            ]),
            widgets::divider(),
            widgets::field_grid(vec![
                checked(&form, CharacterField::Alignment, errors),
                checked(&form, CharacterField::Appearance, errors),
                checked(&form, CharacterField::AttributePoints, errors),
            ]),
            alignment_note(alignment_hint(&form.alignment.read())),
            rect()
                .horizontal()
                .spacing(theme::space::MD)
                .child(quick_add(ctx, "+1 point", 1))
                .child(quick_add(ctx, "+5 points", 5))
                .child(widgets::hint(
                    "Attribute points you can still spend; the field is created when the savegame does not have it.",
                ))
                .into_element(),
        ],
    );

    let vitals = widgets::card(
        "Vitals",
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
            widgets::hint(
                "Current values may stay below the maximum; the game fills them on rest.",
            ),
        ],
    );

    let attributes = widgets::card(
        "Attributes",
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
        "Current sheet",
        None,
        vec![
            widgets::label_row(
                "Name",
                widgets::a11y_text(character.name.clone())
                    .font_size(theme::font::BODY)
                    .color(theme::TEXT),
            ),
            widgets::divider(),
            widgets::label_row(
                "Level",
                widgets::a11y_text(character.level.to_string())
                    .font_size(theme::font::BODY)
                    .color(theme::TEXT),
            ),
            widgets::divider(),
            widgets::label_row(
                "Play time",
                widgets::a11y_text(crate::session::format::duration(character.time_played))
                    .font_size(theme::font::BODY)
                    .color(theme::TEXT),
            ),
        ],
    );

    rect()
        .vertical()
        .width(Size::fill())
        .spacing(theme::space::XXL)
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
        .height(Size::px(ALIGNMENT_HINT_HEIGHT))
        .main_align(Alignment::Center)
        .child(
            widgets::a11y_text(text)
                .font_size(theme::font::CAPTION)
                .max_lines(2)
                .color(theme::DIM),
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
