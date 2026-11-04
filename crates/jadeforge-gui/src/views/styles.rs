//! The styles tab: the fighting styles the player knows.

use freya::prelude::*;

use crate::app::Ctx;
use crate::session::StyleRow;
use crate::theme;
use crate::widgets;

/// Renders the styles tab.
pub fn view(
    ctx: &Ctx,
    styles: &[StyleRow],
    query: State<String>,
    matches: &[(u16, String, String)],
    game_ready: bool,
) -> Element {
    let mut rows: Vec<Element> = Vec::new();
    for style in styles {
        rows.push(style_row(ctx, style));
    }
    if rows.is_empty() {
        rows.push(widgets::hint(
            "The player has not learned any fighting styles.",
        ));
    }

    let mut children = vec![
        rect()
            .vertical()
            .width(Size::fill())
            .spacing(4.)
            .children(rows)
            .into_element(),
        widgets::divider(),
        widgets::section_label("LEARN A STYLE"),
        widgets::hint(
            "New styles copy the combat data of a style the player already owns; \
             the game recomputes the rest from its own tables.",
        ),
    ];

    if game_ready {
        children.push(
            Input::new(query)
                .placeholder("Search styles, for example \"sword\" or \"dragon\"")
                .width(Size::fill())
                .into_element(),
        );
        if !query.read().trim().is_empty() {
            if matches.is_empty() {
                children.push(widgets::hint("Every matching style is already known."));
            } else {
                let mut result_rows: Vec<Element> = Vec::new();
                for (style_id, name, label) in matches.iter().take(8) {
                    result_rows.push(learn_row(ctx, *style_id, name, label));
                }
                children.push(
                    rect()
                        .vertical()
                        .width(Size::fill())
                        .spacing(2.)
                        .children(result_rows)
                        .into_element(),
                );
            }
        }
    } else {
        children.push(widgets::hint(
            "The game data was not found, so style names and new styles are unavailable.",
        ));
    }

    widgets::card(
        "Fighting styles",
        Some(format!("{} learned", styles.len())),
        children,
    )
}

fn style_row(ctx: &Ctx, style: &StyleRow) -> Element {
    let index = style.struct_index;
    let level = style.level;

    let lower = {
        let ctx = ctx.clone();
        move |_| ctx.run(|session| session.set_style_level(index, level.saturating_sub(1).max(1)))
    };
    let raise = {
        let ctx = ctx.clone();
        move |_| {
            ctx.run(|session| {
                session.set_style_level(index, (level + 1).min(jadeforge_core::MAX_STYLE_LEVEL));
            });
        }
    };
    let remove = {
        let ctx = ctx.clone();
        move |_| ctx.run(|session| session.remove_style(index))
    };

    rect()
        .horizontal()
        .width(Size::fill())
        .spacing(10.)
        .content(Content::Flex)
        .padding(Gaps::new_symmetric(6., 10.))
        .background(theme::SURFACE)
        .corner_radius(8.)
        .child(
            rect()
                .vertical()
                .spacing(1.)
                .width(Size::flex(1.0))
                .child(
                    label()
                        .text(style.name.clone())
                        .font_size(13.)
                        .max_lines(1)
                        .color(theme::TEXT),
                )
                .child(
                    label()
                        .text(format!("style id {}", style.style_id))
                        .font_size(10.)
                        .color(theme::DIM),
                ),
        )
        .child(if style.weapon {
            widgets::badge("weapon", theme::SURFACE_ACTIVE, theme::JADE)
        } else {
            widgets::badge("unarmed", theme::SURFACE_ACTIVE, theme::MUTED)
        })
        .child(Button::new().compact().flat().child("−").on_press(lower))
        .child(
            label()
                .text(format!("Lv {level}"))
                .font_size(12.)
                .width(Size::px(46.))
                .text_align(TextAlign::Center)
                .color(theme::ACCENT),
        )
        .child(Button::new().compact().flat().child("+").on_press(raise))
        .child(
            Button::new()
                .compact()
                .flat()
                .child("Remove")
                .on_press(remove),
        )
        .into_element()
}

fn learn_row(ctx: &Ctx, style_id: u16, name: &str, style_label: &str) -> Element {
    let learn = {
        let ctx = ctx.clone();
        move |_| ctx.run(|session| session.add_style(style_id))
    };
    rect()
        .horizontal()
        .width(Size::fill())
        .spacing(10.)
        .content(Content::Flex)
        .padding(Gaps::new_symmetric(5., 8.))
        .background(theme::PANEL)
        .corner_radius(6.)
        .child(
            rect()
                .vertical()
                .spacing(1.)
                .width(Size::flex(1.0))
                .child(
                    label()
                        .text(name.to_owned())
                        .font_size(12.)
                        .max_lines(1)
                        .color(theme::TEXT),
                )
                .child(
                    label()
                        .text(style_label.to_owned())
                        .font_size(9.)
                        .max_lines(1)
                        .color(theme::DIM),
                ),
        )
        .child(
            label()
                .text(format!("#{style_id}"))
                .font_size(10.)
                .color(theme::DIM),
        )
        .child(
            Button::new()
                .compact()
                .outline()
                .child("Learn")
                .on_press(learn),
        )
        .into_element()
}
