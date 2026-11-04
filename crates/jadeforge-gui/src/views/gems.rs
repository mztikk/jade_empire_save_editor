//! The gems tab: amulet sockets and carried gems.
//!
//! Editing is limited to moving a gem between the two lists and removing one:
//! the gem struct itself is never rewritten, so its effects stay untouched.

use freya::prelude::*;

use crate::app::Ctx;
use crate::session::GemRow;
use crate::theme;
use crate::widgets;
use jadeforge_core::GemLocation;

/// Renders the gems tab.
#[allow(clippy::too_many_lines)] // one declarative layout is easier to follow
pub fn view(
    ctx: &Ctx,
    gems: &[GemRow],
    query: State<String>,
    available: &[(u16, String, usize)],
) -> Element {
    let amulet: Vec<&GemRow> = gems
        .iter()
        .filter(|gem| gem.location == GemLocation::Amulet)
        .collect();
    let inventory: Vec<&GemRow> = gems
        .iter()
        .filter(|gem| gem.location == GemLocation::Inventory)
        .collect();

    let amulet_rows: Vec<Element> = if amulet.is_empty() {
        vec![widgets::hint("No gems socketed into the amulet.")]
    } else {
        amulet
            .into_iter()
            .map(|gem| gem_row(ctx, gem, true))
            .collect()
    };
    let inventory_rows: Vec<Element> = if inventory.is_empty() {
        vec![widgets::hint("No gems carried in the inventory.")]
    } else {
        inventory
            .into_iter()
            .map(|gem| gem_row(ctx, gem, false))
            .collect()
    };

    let needle = query.read().trim().to_lowercase();
    let mut add_rows: Vec<Element> = Vec::new();
    for (item_id, name, effects) in available {
        if !needle.is_empty() && !name.to_lowercase().contains(&needle) {
            continue;
        }
        if add_rows.len() >= 12 {
            break;
        }
        let ctx = ctx.clone();
        let item_id = *item_id;
        add_rows.push(
            rect()
                .horizontal()
                .width(Size::fill())
                .spacing(10.)
                .content(Content::Flex)
                .padding(Gaps::new_symmetric(4., 8.))
                .child(
                    label()
                        .text(name.clone())
                        .font_size(12.)
                        .max_lines(1)
                        .color(theme::TEXT),
                )
                .child(rect().width(Size::flex(1.0)))
                .child(
                    label()
                        .text(format!("{effects} effects"))
                        .font_size(10.)
                        .color(theme::DIM),
                )
                .child(
                    Button::new()
                        .compact()
                        .outline()
                        .child("Add")
                        .on_press(move |_| ctx.run(|session| session.add_gem(item_id))),
                )
                .into_element(),
        );
    }
    if add_rows.is_empty() {
        add_rows.push(widgets::hint(if available.is_empty() {
            "The game data with the gem definitions was not found."
        } else {
            "No gem matches this filter."
        }));
    }
    let add_card = widgets::card(
        "Add a gem",
        Some("predefined gems".to_owned()),
        vec![
            Input::new(query)
                .placeholder("Filter the gem list, or leave empty to browse it")
                .width(Size::fill())
                .into_element(),
            rect()
                .vertical()
                .width(Size::fill())
                .spacing(2.)
                .children(add_rows)
                .into_element(),
        ],
    );

    rect()
        .vertical()
        .width(Size::fill())
        .spacing(16.)
        .child(widgets::card(
            "Amulet",
            Some(format!("{} socketed", amulet_rows.len().saturating_sub(0))),
            vec![rect()
                .vertical()
                .width(Size::fill())
                .spacing(4.)
                .children(amulet_rows)
                .into_element()],
        ))
        .child(add_card)
        .child(widgets::card(
            "Inventory",
            Some(format!("{} gems", inventory_rows.len())),
            vec![
                rect()
                    .vertical()
                    .width(Size::fill())
                    .spacing(4.)
                    .children(inventory_rows)
                    .into_element(),
                widgets::hint(
                    "Moving a gem keeps its effects; JadeForge only changes which list references it.",
                ),
            ],
        ))
        .into_element()
}

fn gem_row(ctx: &Ctx, gem: &GemRow, socketed: bool) -> Element {
    let ctx = ctx.clone();
    let index = gem.struct_index;
    let action = if socketed {
        "Move to inventory"
    } else {
        "Move to amulet"
    };
    let mut row = rect()
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
                        .text(gem.name.clone())
                        .font_size(13.)
                        .max_lines(1)
                        .color(if gem.empty { theme::DIM } else { theme::TEXT }),
                )
                .child(
                    label()
                        .text(if gem.effects.is_empty() {
                            "no effects".to_owned()
                        } else {
                            gem.effects.join(" · ")
                        })
                        .font_size(10.)
                        .max_lines(2)
                        .color(theme::DIM),
                ),
        );
    if !gem.empty {
        let move_ctx = ctx.clone();
        let remove_ctx = ctx.clone();
        row = row
            .child(
                Button::new()
                    .compact()
                    .flat()
                    .child(action)
                    .on_press(move |_| move_ctx.run(|session| session.move_gem(index))),
            )
            .child(
                Button::new()
                    .compact()
                    .flat()
                    .child("Remove")
                    .on_press(move |_| remove_ctx.run(|session| session.remove_gem(index))),
            );
    }
    row.into_element()
}
