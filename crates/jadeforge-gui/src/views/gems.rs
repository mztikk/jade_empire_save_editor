//! The gems tab: amulet sockets and carried gems.
//!
//! Editing is limited to moving a gem between the two lists and removing one:
//! the gem struct itself is never rewritten, so its effects stay untouched.

use freya::prelude::*;

use crate::app::Ctx;
use crate::session::{GemRow, Section};
use crate::theme;
use crate::widgets;
use jadeforge_core::{AmuletInfo, GemLocation};

/// Renders the gems tab.
#[allow(clippy::too_many_lines)] // one declarative tree keeps the visual order auditable
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
    let amulet_error = ctx.session.borrow().section_error(Section::Amulet).cloned();
    let gems_error = ctx.session.borrow().section_error(Section::Gems).cloned();
    let amulet_rows: Vec<Element> = if let Some(error) = &amulet_error {
        vec![widgets::hint(format!(
            "The amulet section could not be read: {}",
            error.message
        ))]
    } else if amulet.is_none() {
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
    let inventory_rows: Vec<Element> = if let Some(error) = &gems_error {
        vec![widgets::hint(format!(
            "The gems section could not be read: {}",
            error.message
        ))]
    } else if inventory.is_empty() {
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
                .spacing(theme::space::LG)
                .content(Content::Flex)
                .padding(Gaps::new_symmetric(theme::space::XS, theme::space::MD))
                .child(widgets::with_tooltip(
                    name.clone(),
                    widgets::a11y_text(name.clone())
                        .font_size(theme::font::LABEL)
                        .max_lines(1)
                        .text_overflow(TextOverflow::Ellipsis)
                        .color(theme::TEXT),
                ))
                .child(rect().width(Size::flex(1.0)))
                .child(
                    widgets::a11y_text(widgets::plural(*effects, "effect", "effects"))
                        .font_size(theme::font::MICRO)
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
    let mut add_children = vec![widgets::filter_input(
        query,
        "Filter the gem list, or leave empty to browse it",
        "Clear the gem filter",
    )];
    if matching.len() > shown {
        let mut gem_limit = gem_limit;
        add_children.push(
            rect()
                .horizontal()
                .width(Size::fill())
                .spacing(theme::space::MD)
                .content(Content::Flex)
                .child(
                    widgets::a11y_text(format!(
                        "{shown} of {}",
                        widgets::plural(matching.len(), "gem", "gems")
                    ))
                    .font_size(theme::font::MICRO)
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
            .spacing(theme::space::XXS)
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
        .spacing(theme::space::XXL)
        .child(widgets::card(
            "Amulet",
            Some(amulet_title),
            {
                let mut children = vec![rect()
                    .vertical()
                    .width(Size::fill())
                    .spacing(theme::space::XS)
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
                    .spacing(theme::space::XS)
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
    let mut actions: Vec<Element> = Vec::new();
    if !gem.empty {
        let remove_ctx = ctx.clone();
        if can_move {
            let move_ctx = ctx.clone();
            actions.push(
                Button::new()
                    .compact()
                    .flat()
                    .child(action)
                    .on_press(move |_| move_ctx.run(|session| session.move_gem(index)))
                    .into_element(),
            );
        } else {
            actions.push(
                widgets::a11y_text(if has_amulet {
                    "Amulet full"
                } else {
                    "No amulet yet"
                })
                .font_size(theme::font::MICRO)
                .color(theme::DIM)
                .into_element(),
            );
        }
        let remove_name = gem.name.clone();
        actions.push(
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
                })
                .into_element(),
        );
    }
    rect()
        .horizontal()
        .width(Size::fill())
        .key(index)
        .a11y_role(AccessibilityRole::Group)
        .a11y_alt(format!("{} gem", gem.name))
        .spacing(theme::space::LG)
        .content(Content::Flex)
        .padding(Gaps::new_symmetric(theme::space::SM, theme::space::LG))
        .background(theme::SURFACE)
        .corner_radius(theme::radius::MEDIUM)
        .child(
            rect()
                .vertical()
                .spacing(theme::space::XXS)
                .width(Size::flex(1.0))
                .child(widgets::with_tooltip(
                    gem.name.clone(),
                    widgets::a11y_text(gem.name.clone())
                        .font_size(theme::font::BODY)
                        .max_lines(1)
                        .text_overflow(TextOverflow::Ellipsis)
                        .color(if gem.empty { theme::DIM } else { theme::TEXT }),
                ))
                .child(
                    widgets::a11y_text(if gem.effects.is_empty() {
                        "no effects".to_owned()
                    } else {
                        gem.effects.join(" · ")
                    })
                    .font_size(theme::font::MICRO)
                    .max_lines(2)
                    .color(theme::DIM),
                ),
        )
        .children(actions)
        .into_element()
}
