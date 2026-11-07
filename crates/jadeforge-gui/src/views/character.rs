//! The character tab: identity, vitals, attributes and progression.

use freya::prelude::*;

use crate::app::Ctx;
use crate::form::Form;
use crate::i18n::{fill, keys, t};
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
            keys::CHARACTER_ALIGNMENT_TEMPLATE,
            &[
                ("alignment", t(format::alignment(value))),
                ("default", &jadeforge_core::DEFAULT_ALIGNMENT.to_string()),
            ],
        ),
        Err(_) => t(keys::CHARACTER_ALIGNMENT_FALLBACK).to_owned(),
    }
}

/// Renders the character tab.
#[allow(clippy::too_many_lines)] // one declarative tree keeps the visual order auditable
pub fn view(ctx: &Ctx, character: &jadeforge_core::Character, errors: &[FieldError]) -> Element {
    let form = ctx.form;

    let identity = widgets::card(
        t(keys::CHARACTER_IDENTITY),
        Some(t(keys::CHARACTER_CHANGES_HINT).to_owned()),
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
                    input(&form, CharacterField::Gold).placeholder(t(keys::CHARACTER_NOT_SET)),
                    widgets::field_error(errors, CharacterField::Gold),
                ),
                checked(&form, CharacterField::Chapter, errors),
                checked(&form, CharacterField::Kills, errors),
            ]),
            widgets::hint(t(keys::CHARACTER_GOLD_HINT)),
            widgets::divider(),
            widgets::field_grid(vec![
                checked(&form, CharacterField::Alignment, errors),
                appearance_field(ctx, &form, errors),
                checked(&form, CharacterField::AttributePoints, errors),
            ]),
            alignment_note(alignment_hint(&form.alignment.read())),
            rect()
                .horizontal()
                .spacing(theme::space::md())
                .child(quick_add(ctx, t(keys::CHARACTER_QUICK_ADD_ONE), 1))
                .child(quick_add(ctx, t(keys::CHARACTER_QUICK_ADD_FIVE), 5))
                .child(widgets::hint(t(keys::CHARACTER_ATTRIBUTE_POINTS_HINT)))
                .into_element(),
        ],
    );

    let vitals = widgets::card(
        t(keys::CHARACTER_VITALS),
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
            widgets::hint(t(keys::CHARACTER_CURRENT_VALUES_HINT)),
        ],
    );

    let attributes = widgets::card(
        t(keys::CHARACTER_ATTRIBUTES),
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
        t(keys::CHARACTER_CURRENT_SHEET),
        None,
        vec![
            widgets::label_row(
                t(keys::CHARACTER_NAME),
                widgets::a11y_text(character.name.clone())
                    .font_size(theme::font::body())
                    .color(theme::text()),
            ),
            widgets::divider(),
            widgets::label_row(
                t(keys::CHARACTER_LEVEL),
                widgets::a11y_text(character.level.to_string())
                    .font_size(theme::font::body())
                    .color(theme::text()),
            ),
            widgets::divider(),
            widgets::label_row(
                t(keys::CHARACTER_TIME_PLAYED),
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

/// The seven player models as a select; an arbitrary `appearance.2da` row
/// crashes the game.
fn appearance_field(ctx: &Ctx, form: &Form, errors: &[FieldError]) -> Element {
    let options = ctx.session.borrow().appearance_options();
    let current = form
        .appearance
        .read()
        .trim()
        .parse::<i32>()
        .ok()
        .and_then(jadeforge_core::PlayerAppearance::from_row);
    let selected_label = current
        .and_then(|appearance| {
            options
                .iter()
                .find(|option| option.appearance == appearance)
                .map(|option| option.label.clone())
        })
        .unwrap_or_else(|| t(keys::CHARACTER_APPEARANCE_CHOOSE).to_owned());
    let mut state = form.appearance;
    let select = Select::new()
        .list_match_width(true)
        .selected_item(
            widgets::a11y_text(selected_label)
                .font_size(theme::font::body())
                .color(theme::text()),
        )
        .children(options.into_iter().map(|option| {
            let row = option.appearance.row();
            MenuItem::new()
                .selected(current == Some(option.appearance))
                .on_press(move |_| state.set(row.to_string()))
                .child(option.label)
        }));
    widgets::field_checked(
        CharacterField::Appearance,
        select,
        widgets::field_error(errors, CharacterField::Appearance),
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
