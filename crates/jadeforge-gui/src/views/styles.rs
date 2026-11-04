//! The styles tab: the fighting styles the player knows.

use freya::prelude::*;

use crate::app::Ctx;
use crate::session::{StyleRow, StyleSuggestion, StyleUpgrade};
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
    expanded: State<Option<u16>>,
    show_more: impl FnMut(Event<PressEventData>) + 'static,
) -> Element {
    let mut rows: Vec<Element> = Vec::new();
    for style in styles {
        rows.push(style_row(ctx, style, expanded));
    }
    if rows.is_empty() {
        rows.push(widgets::hint(
            "The player has not learned any fighting styles.",
        ));
    }

    let strict = ctx.session.borrow().strict_upgrades_enabled();
    let mut children = vec![
        rules_row(ctx, strict),
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
                result_rows.push(learn_row(ctx, suggestion, expanded));
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

fn style_row(ctx: &Ctx, style: &StyleRow, expanded: State<Option<u16>>) -> Element {
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

    let subtitle = style_subtitle(style.style_id, &style.label)
        .unwrap_or_else(|| format!("#{}", style.style_id));
    let is_expanded = *expanded.read() == Some(style.style_id);

    rect()
        .vertical()
        .width(Size::fill())
        .background(theme::SURFACE)
        .corner_radius(8.)
        .child(
            rect()
                .horizontal()
                .width(Size::fill())
                .spacing(10.)
                .content(Content::Flex)
                .padding(Gaps::new_symmetric(4., 6.))
                .child(widgets::expanding_name(
                    style.style_id,
                    style.name.clone(),
                    subtitle,
                    expanded,
                    13.,
                ))
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
                ),
        )
        .maybe_child(if is_expanded {
            Some(upgrade_panel(ctx, style))
        } else {
            None
        })
        .into_element()
}

/// Game rules toggle: restrict upgrades to what the style offers.
fn rules_row(ctx: &Ctx, strict: bool) -> Element {
    let ctx = ctx.clone();
    rect()
        .horizontal()
        .width(Size::fill())
        .spacing(10.)
        .content(Content::Flex)
        .child(
            Button::new()
                .compact()
                .outline()
                .child(if strict {
                    "Game rules: on"
                } else {
                    "Game rules: off"
                })
                .on_press(move |_| {
                    let current = ctx.session.borrow().strict_upgrades_enabled();
                    ctx.run(|session| session.set_strict_upgrades(!current));
                }),
        )
        .child(widgets::hint(if strict {
            "Only upgrades the style offers, bought in order."
        } else {
            "Any upgrade row, still bought in order."
        }))
        .into_element()
}

/// Expanded panel: description plus the upgrade choices of the style.
fn upgrade_panel(ctx: &Ctx, style: &StyleRow) -> Element {
    let mut rows: Vec<Element> = Vec::new();
    if let Some(details) = widgets::details(style.description.as_deref()) {
        rows.push(details);
    }
    if style.upgrades.is_empty() {
        if style.description.is_none() {
            rows.push(widgets::hint(
                "No upgrade data available: install the game data next to the savegames.",
            ));
        }
        return rect()
            .vertical()
            .width(Size::fill())
            .spacing(4.)
            .padding(Gaps::new(0., 10., 6., 10.))
            .children(rows)
            .into_element();
    }

    rows.push(widgets::section_label("UPGRADES"));
    let strict = ctx.session.borrow().strict_upgrades_enabled();
    let mut family: Option<&str> = None;
    let mut chips: Vec<Element> = Vec::new();
    for upgrade in style
        .upgrades
        .iter()
        .filter(|upgrade| !strict || upgrade.available || upgrade.taken)
    {
        if family.is_some_and(|family| family != upgrade.family) {
            rows.push(upgrade_row(
                family.unwrap_or_default(),
                std::mem::take(&mut chips),
            ));
        }
        if family != Some(upgrade.family.as_str()) {
            family = Some(upgrade.family.as_str());
        }
        chips.push(upgrade_chip(ctx, style.struct_index, upgrade));
    }
    if let Some(family) = family {
        rows.push(upgrade_row(family, chips));
    }
    rows.push(widgets::hint(
        "One chip per upgrade tier; the level follows the number of owned upgrades.",
    ));
    rect()
        .vertical()
        .width(Size::fill())
        .spacing(6.)
        .padding(Gaps::new(0., 10., 8., 10.))
        .children(rows)
        .into_element()
}

/// One family line: its name and a chip per tier.
fn upgrade_row(family: &str, chips: Vec<Element>) -> Element {
    rect()
        .horizontal()
        .width(Size::fill())
        .spacing(6.)
        .content(Content::Flex)
        .child(
            label()
                .text(family_title(family))
                .font_size(11.)
                .width(Size::px(120.))
                .color(theme::MUTED),
        )
        .children(chips)
        .into_element()
}

/// One toggleable upgrade tier.
fn upgrade_chip(ctx: &Ctx, struct_index: usize, upgrade: &StyleUpgrade) -> Element {
    let taken = upgrade.taken;
    let row = upgrade.row;
    let enabled = ctx.session.borrow().can_toggle_upgrade(struct_index, row);
    if !enabled {
        // Dim and inert instead of a button that silently ignores presses.
        return rect()
            .center()
            .width(Size::px(22.))
            .height(Size::px(22.))
            .corner_radius(6.)
            .border(Border::new().fill(theme::BORDER).width(1.))
            .child(
                label()
                    .text(upgrade.tier.to_string())
                    .font_size(11.)
                    .color(theme::DIM),
            )
            .into_element();
    }
    let ctx = ctx.clone();
    let button = Button::new().compact();
    let button = if taken {
        button.filled()
    } else {
        button.outline()
    };
    button
        .child(upgrade.tier.to_string())
        .on_press(move |_| {
            ctx.run(|session| session.toggle_style_upgrade(struct_index, row));
        })
        .into_element()
}

/// Human readable family name for the upgrade rows.
#[must_use]
pub fn family_title(family: &str) -> String {
    let named = match family {
        "Increase_Damage" => "Damage",
        "Chi_Strike_Damage" => "Chi strike",
        "Speed_Increase" => "Speed",
        "Lower_Chi_Cost" => "Chi cost",
        "Duration" => "Duration",
        "Chi_Reduction" => "Chi damage taken",
        "Focus_Cost_Reduction" => "Focus cost",
        "Damage_Reduction" => "Damage taken",
        "Spirit_Thief_Duration" => "Spirit thief",
        "Gun_Focus_Cost_Reduction" => "Gun focus cost",
        _ => family,
    };
    if std::ptr::eq(named, family) {
        named.replace('_', " ")
    } else {
        named.to_owned()
    }
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

fn learn_row(ctx: &Ctx, suggestion: &StyleSuggestion, expanded: State<Option<u16>>) -> Element {
    let learn = {
        let ctx = ctx.clone();
        let style_id = suggestion.style_id;
        move |_| ctx.run(|session| session.add_style(style_id))
    };
    let subtitle = style_subtitle(suggestion.style_id, &suggestion.label)
        .unwrap_or_else(|| format!("#{}", suggestion.style_id));
    let is_expanded = *expanded.read() == Some(suggestion.style_id);

    rect()
        .vertical()
        .width(Size::fill())
        .background(theme::PANEL)
        .corner_radius(6.)
        .child(
            rect()
                .horizontal()
                .width(Size::fill())
                .spacing(10.)
                .content(Content::Flex)
                .padding(Gaps::new_symmetric(3., 4.))
                .child(widgets::expanding_name(
                    suggestion.style_id,
                    suggestion.name.clone(),
                    subtitle,
                    expanded,
                    12.,
                ))
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
                ),
        )
        .maybe_child(if is_expanded {
            widgets::details(suggestion.description.as_deref())
        } else {
            None
        })
        .into_element()
}
