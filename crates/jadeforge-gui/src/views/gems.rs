//! The gems tab: amulet sockets and carried gems.
//!
//! Editing is limited to moving a gem between the two lists and removing one:
//! the gem struct itself is never rewritten, so its effects stay untouched.

use freya::prelude::*;

use crate::app::Ctx;
use crate::session::GemRow;
use crate::theme;
use crate::widgets;
use jadeforge_core::{AmuletInfo, GemLocation};

/// Renders the gems tab.
#[allow(clippy::too_many_lines)] // one declarative layout is easier to follow
pub fn view(
    ctx: &Ctx,
    gems: &[GemRow],
    query: State<String>,
    available: &[(u16, String, usize)],
    amulet: Option<AmuletInfo>,
    gem_limit: State<usize>,
) -> Element {
    let amulet_gems: Vec<&GemRow> = gems
        .iter()
        .filter(|gem| gem.location == GemLocation::Amulet)
        .collect();
    let inventory: Vec<&GemRow> = gems
        .iter()
        .filter(|gem| gem.location == GemLocation::Inventory)
        .collect();

    let socketed = gems
        .iter()
        .filter(|gem| gem.location == GemLocation::Amulet && !gem.empty)
        .count();
    let can_socket = amulet.is_some_and(|amulet| socketed < usize::from(amulet.sockets));
    let amulet_title = amulet.map_or_else(
        || "No amulet yet".to_owned(),
        |amulet| {
            format!(
                "{socketed} socketed · tier {} ({} sockets)",
                amulet.tier, amulet.sockets
            )
        },
    );
    let amulet_rows: Vec<Element> = if amulet.is_none() {
        vec![widgets::hint(
            "The amulet is unlocked early in the story; it has no sockets before that.",
        )]
    } else if amulet_gems.is_empty() {
        vec![widgets::hint("No gems socketed into the amulet.")]
    } else {
        amulet_gems
            .into_iter()
            .map(|gem| gem_row(ctx, gem, true, true, amulet.is_some()))
            .collect()
    };
    let inventory_rows: Vec<Element> = if inventory.is_empty() {
        vec![widgets::hint("No gems carried in the inventory.")]
    } else {
        inventory
            .into_iter()
            .map(|gem| gem_row(ctx, gem, false, can_socket, amulet.is_some()))
            .collect()
    };

    let needle = query.read().trim().to_lowercase();
    let matching: Vec<&(u16, String, usize)> = available
        .iter()
        .filter(|(_, name, _)| needle.is_empty() || name.to_lowercase().contains(&needle))
        .collect();
    let limit = *gem_limit.read();
    let shown = matching.len().min(limit);
    let mut add_rows: Vec<Element> = Vec::new();
    for (item_id, name, effects) in matching.iter().take(shown) {
        let ctx = ctx.clone();
        let item_id = *item_id;
        add_rows.push(
            rect()
                .horizontal()
                .width(Size::fill())
                .key(item_id)
                .a11y_role(AccessibilityRole::Group)
                .a11y_alt(format!("{name} gem"))
                .spacing(10.)
                .content(Content::Flex)
                .padding(Gaps::new_symmetric(4., 8.))
                .child(
                    widgets::a11y_text(name.clone())
                        .font_size(12.)
                        .max_lines(1)
                        .color(theme::TEXT),
                )
                .child(rect().width(Size::flex(1.0)))
                .child(
                    widgets::a11y_text(widgets::plural(*effects, "effect", "effects"))
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
    let mut add_children = vec![
        Input::new(query)
            .placeholder("Filter the gem list, or leave empty to browse it")
            .width(Size::fill())
            .into_element(),
    ];
    if matching.len() > shown {
        let mut gem_limit = gem_limit;
        add_children.push(
            rect()
                .horizontal()
                .width(Size::fill())
                .spacing(8.)
                .content(Content::Flex)
                .child(
                    widgets::a11y_text(format!(
                        "{shown} of {}",
                        widgets::plural(matching.len(), "gem", "gems")
                    ))
                    .font_size(10.)
                    .color(theme::DIM),
                )
                .child(rect().width(Size::flex(1.0)))
                .child(
                    Button::new()
                        .compact()
                        .outline()
                        .child("Show more")
                        .on_press(move |_| *gem_limit.write() += crate::app::PAGE_SIZE),
                )
                .into_element(),
        );
    }
    add_children.push(
        rect()
            .vertical()
            .width(Size::fill())
            .spacing(2.)
            .children(add_rows)
            .into_element(),
    );
    let add_card = widgets::card(
        "Add a gem",
        Some("predefined gems".to_owned()),
        add_children,
    );

    rect()
        .vertical()
        .width(Size::fill())
        .spacing(16.)
        .child(widgets::card(
            "Amulet",
            Some(amulet_title),
            {
                let mut children = vec![rect()
                    .vertical()
                    .width(Size::fill())
                    .spacing(4.)
                    .children(amulet_rows)
                    .into_element()];
                if let Some(amulet) = amulet {
                    children.push(widgets::hint(if amulet.sockets >= 7 {
                        "All seven sockets are unlocked."
                    } else {
                        "More sockets unlock as the story progresses."
                    }));
                }
                children
            },
        ))
        .child(add_card)
        .child(widgets::card(
            "Inventory",
            Some(widgets::plural(inventory_rows.len(), "gem", "gems")),
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

fn gem_row(ctx: &Ctx, gem: &GemRow, socketed: bool, can_move: bool, has_amulet: bool) -> Element {
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
        .key(index)
        .a11y_role(AccessibilityRole::Group)
        .a11y_alt(format!("{} gem", gem.name))
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
                    widgets::a11y_text(gem.name.clone())
                        .font_size(13.)
                        .max_lines(1)
                        .color(if gem.empty { theme::DIM } else { theme::TEXT }),
                )
                .child(
                    widgets::a11y_text(if gem.effects.is_empty() {
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
        let remove_ctx = ctx.clone();
        if can_move {
            let move_ctx = ctx.clone();
            row = row.child(
                Button::new()
                    .compact()
                    .flat()
                    .child(action)
                    .on_press(move |_| move_ctx.run(|session| session.move_gem(index))),
            );
        } else {
            row = row.child(
                widgets::a11y_text(if has_amulet {
                    "Amulet full"
                } else {
                    "No amulet yet"
                })
                .font_size(10.)
                .color(theme::DIM),
            );
        }
        let remove_name = gem.name.clone();
        row = row.child(
            Button::new()
                .compact()
                .flat()
                .child("Remove")
                .on_press(move |_| {
                    if !crate::dialog::confirm(
                        "Remove gem?",
                        &format!("Remove {remove_name}? You can still press Revert before saving."),
                    ) {
                        return;
                    }
                    remove_ctx.run(|session| session.remove_gem(index));
                }),
        );
    }
    row.into_element()
}
