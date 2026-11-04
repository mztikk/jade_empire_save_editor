//! The character tab: identity, vitals, attributes and progression.

use freya::prelude::*;

use crate::app::Ctx;
use crate::theme;
use crate::widgets;

/// Explains the numeric alignment the way the game names it.
#[must_use]
pub fn alignment_hint(value: &str) -> String {
    match value.trim().parse::<i32>() {
        Ok(value) if value >= 500 => "Alignment 500 and above: Way of the Open Palm.".to_owned(),
        Ok(_) => "Alignment below 500: Way of the Closed Fist.".to_owned(),
        Err(_) => {
            "Alignment is a number; the game shows it as Closed Fist or Open Palm.".to_owned()
        }
    }
}

/// Renders the character tab.
#[allow(clippy::too_many_lines)] // one declarative layout is easier to follow
pub fn view(ctx: &Ctx, character: &jadeforge_core::Character) -> Element {
    let form = ctx.form;

    let identity = widgets::card(
        "Identity",
        Some("Staged changes are applied with Apply or Save to disk".to_owned()),
        vec![
            widgets::field_grid(vec![
                widgets::field("NAME", Input::new(form.name).width(Size::fill()), 220.),
                widgets::field("LEVEL", Input::new(form.level).width(Size::fill()), 90.),
                widgets::field(
                    "EXPERIENCE",
                    Input::new(form.experience).width(Size::fill()),
                    150.,
                ),
                widgets::field("GOLD", Input::new(form.gold).width(Size::fill()), 150.),
                widgets::field("CHAPTER", Input::new(form.chapter).width(Size::fill()), 90.),
                widgets::field("KILLS", Input::new(form.kills).width(Size::fill()), 90.),
            ]),
            widgets::divider(),
            widgets::field_grid(vec![
                widgets::field(
                    "ALIGNMENT",
                    Input::new(form.alignment).width(Size::fill()),
                    110.,
                ),
                widgets::field(
                    "APPEARANCE",
                    Input::new(form.appearance).width(Size::fill()),
                    110.,
                ),
                widgets::field(
                    "UNSPENT ATTRIBUTE POINTS",
                    Input::new(form.attribute_points).width(Size::fill()),
                    170.,
                ),
            ]),
            widgets::hint(alignment_hint(&form.alignment.read())),
            rect()
                .horizontal()
                .spacing(8.)
                .child(quick_add(ctx, "+1 point", 1))
                .child(quick_add(ctx, "+5 points", 5))
                .child(widgets::hint(
                    "Points you can still spend. The game stores them as Stats.UnassignedPts; JadeForge creates the field when it is missing.",
                ))
                .into_element(),
        ],
    );

    let vitals = widgets::card(
        "Vitals",
        None,
        vec![
            widgets::field_grid(vec![
                widgets::field("HEALTH", Input::new(form.health).width(Size::fill()), 110.),
                widgets::field(
                    "HEALTH MAX",
                    Input::new(form.health_max).width(Size::fill()),
                    110.,
                ),
                widgets::field("CHI", Input::new(form.chi).width(Size::fill()), 110.),
                widgets::field(
                    "CHI MAX",
                    Input::new(form.chi_max).width(Size::fill()),
                    110.,
                ),
            ]),
            widgets::divider(),
            widgets::field_grid(vec![
                widgets::field("FOCUS", Input::new(form.focus).width(Size::fill()), 110.),
                widgets::field(
                    "FOCUS MAX",
                    Input::new(form.focus_max).width(Size::fill()),
                    110.,
                ),
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
                widgets::field("BODY", Input::new(form.body).width(Size::fill()), 100.),
                widgets::field("MIND", Input::new(form.mind).width(Size::fill()), 100.),
                widgets::field("SPIRIT", Input::new(form.spirit).width(Size::fill()), 100.),
            ]),
            widgets::divider(),
            widgets::field_grid(vec![
                widgets::field("CHARM", Input::new(form.charm).width(Size::fill()), 100.),
                widgets::field(
                    "INTUITION",
                    Input::new(form.intuition).width(Size::fill()),
                    100.,
                ),
                widgets::field(
                    "INTIMIDATE",
                    Input::new(form.intimidate).width(Size::fill()),
                    100.,
                ),
            ]),
        ],
    );

    let summary = widgets::card(
        "Current sheet",
        None,
        vec![
            widgets::label_row(
                "Name",
                label()
                    .text(character.name.clone())
                    .font_size(13.)
                    .color(theme::TEXT),
            ),
            widgets::divider(),
            widgets::label_row(
                "Level",
                label()
                    .text(character.level.to_string())
                    .font_size(13.)
                    .color(theme::TEXT),
            ),
            widgets::divider(),
            widgets::label_row(
                "Play time",
                label()
                    .text(crate::session::format::duration(character.time_played))
                    .font_size(13.)
                    .color(theme::TEXT),
            ),
        ],
    );

    rect()
        .vertical()
        .width(Size::fill())
        .spacing(16.)
        .child(identity)
        .child(vitals)
        .child(attributes)
        .child(summary)
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
