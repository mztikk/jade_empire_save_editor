//! The styles tab: the fighting styles the player knows.

use freya::prelude::*;

use crate::app::Ctx;
use crate::session::{StyleRow, StyleSuggestion};
use crate::theme;
use crate::widgets;

/// Renders the styles tab.
#[allow(clippy::too_many_arguments)]
pub fn view(
    ctx: &Ctx,
    styles: &[StyleRow],
    query: State<String>,
    matches: &[StyleSuggestion],
    game_ready: bool,
    learnable: Option<usize>,
    matching: usize,
    show_more: impl FnMut(Event<PressEventData>) + 'static,
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
            "A new style is stored with the same field layout as one the player already owns; \
             the game fills in the remaining data from its own tables.",
        ),
    ];

    if game_ready {
        children.push(
            Input::new(query)
                .placeholder("Filter the style list, or leave empty to browse it")
                .width(Size::fill())
                .into_element(),
        );
        if matches.is_empty() {
            children.push(widgets::hint(if query.read().trim().is_empty() {
                "Every style of the game is already known."
            } else {
                "No styles match this search."
            }));
        } else {
            let browsing = query.read().trim().is_empty();
            let shown = matches.len();
            let mut heading = rect()
                .horizontal()
                .width(Size::fill())
                .spacing(10.)
                .content(Content::Flex)
                .child(widgets::section_label(if browsing {
                    "ALL STYLES"
                } else {
                    "SEARCH RESULTS"
                }))
                .child(rect().width(Size::flex(1.0)))
                .child(
                    label()
                        .text(match (browsing, shown == matching) {
                            (true, _) => format!("{shown} of {matching}"),
                            (false, true) => format!("{matching} matches"),
                            (false, false) => format!("{shown} of {matching} matches"),
                        })
                        .font_size(10.)
                        .color(theme::DIM),
                );
            if shown < matching {
                heading = heading.child(
                    Button::new()
                        .compact()
                        .outline()
                        .child("Show more")
                        .on_press(show_more),
                );
            }
            children.push(heading.into_element());
            let mut result_rows: Vec<Element> = Vec::new();
            for suggestion in matches {
                result_rows.push(learn_row(ctx, suggestion));
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
    } else {
        children.push(widgets::hint(
            "The game data was not found, so style names and new styles are unavailable.",
        ));
    }

    widgets::card(
        "Fighting styles",
        Some(match learnable {
            Some(learnable) => format!("{} learned · {learnable} learnable", styles.len()),
            None => format!("{} learned", styles.len()),
        }),
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
                .maybe_child(
                    style_subtitle(style.style_id, &style.label).map(|subtitle| {
                        label()
                            .text(subtitle)
                            .font_size(10.)
                            .color(theme::DIM)
                            .into_element()
                    }),
                )
                .into_element(),
        )
        .child(widgets::badge(
            style.kind,
            theme::SURFACE_ACTIVE,
            style_badge_color(style.kind),
        ))
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

/// Secondary line shown for a style, owned or not.
///
/// `None` when the game data is missing: the name is then just `Style <id>`
/// and repeating the id in a subtitle would be noise.
#[must_use]
pub fn style_subtitle(style_id: u16, label: &str) -> Option<String> {
    (!label.is_empty()).then(|| format!("{label} · #{style_id}"))
}

/// Badge colour for a style kind.
fn style_badge_color(kind: &str) -> Color {
    match kind {
        "magic" | "support" => theme::JADE,
        "transformation" | "unique" => theme::ACCENT,
        _ => theme::MUTED,
    }
}

fn learn_row(ctx: &Ctx, suggestion: &StyleSuggestion) -> Element {
    let learn = {
        let ctx = ctx.clone();
        let style_id = suggestion.style_id;
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
                        .text(suggestion.name.clone())
                        .font_size(12.)
                        .max_lines(1)
                        .color(theme::TEXT),
                )
                .maybe_child(style_subtitle(suggestion.style_id, &suggestion.label).map(
                    |subtitle| {
                        label()
                            .text(subtitle)
                            .font_size(10.)
                            .max_lines(1)
                            .color(theme::DIM)
                            .into_element()
                    },
                ))
                .into_element(),
        )
        .child(widgets::badge(
            suggestion.kind,
            theme::SURFACE_ACTIVE,
            style_badge_color(suggestion.kind),
        ))
        .child(
            Button::new()
                .compact()
                .outline()
                .child("Learn")
                .on_press(learn),
        )
        .into_element()
}
