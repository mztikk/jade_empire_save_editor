//! The inventory tab: item stacks, the searchable item database and removal.

use freya::prelude::*;

use crate::app::Ctx;
use crate::session::{ItemRow, ItemSuggestion};
use crate::theme;
use crate::widgets;

/// Renders the inventory tab.
#[allow(clippy::too_many_arguments)]
pub fn view(
    ctx: &Ctx,
    items: &[ItemRow],
    query: State<String>,
    results: &[ItemSuggestion],
    game_ready: bool,
    matching: usize,
    total_items: usize,
    expanded: State<Option<usize>>,
    expanded_suggestion: State<Option<u16>>,
    hovered: State<Option<crate::app::HoverKey>>,
    show_more: impl FnMut(Event<PressEventData>) + 'static,
) -> Element {
    let mut rows: Vec<Element> = Vec::new();
    for item in items {
        rows.push(item_row(ctx, item, expanded, hovered));
    }
    if rows.is_empty() {
        rows.push(widgets::hint("The inventory is empty."));
    }

    let mut children = vec![
        rect()
            .vertical()
            .width(Size::fill())
            .spacing(4.)
            .children(rows)
            .into_element(),
        widgets::divider(),
        widgets::section_label("ADD AN ITEM"),
    ];

    if game_ready {
        children.push(
            Input::new(query)
                .placeholder("Filter the item database, or leave empty to browse it")
                .width(Size::fill())
                .into_element(),
        );
        if results.is_empty() {
            children.push(widgets::hint(if query.read().trim().is_empty() {
                "The item database is empty."
            } else {
                "No items match this search."
            }));
        } else {
            let shown = results.len();
            let browsing = query.read().trim().is_empty();
            let total = if browsing { total_items } else { matching };
            let mut heading = rect()
                .horizontal()
                .width(Size::fill())
                .spacing(10.)
                .content(Content::Flex)
                .child(widgets::section_label(if browsing {
                    "ALL ITEMS"
                } else {
                    "SEARCH RESULTS"
                }))
                .child(rect().width(Size::flex(1.0)))
                .child(
                    label()
                        .text(match (browsing, shown == total) {
                            (true, _) => format!("{shown} of {total}"),
                            (false, true) => format!("{total} matches"),
                            (false, false) => format!("{shown} of {total} matches"),
                        })
                        .font_size(10.)
                        .color(theme::DIM),
                );
            if shown < total {
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
            for suggestion in results {
                result_rows.push(suggestion_row(
                    ctx,
                    suggestion,
                    expanded_suggestion,
                    hovered,
                ));
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
            "The game data was not found, so items can only be removed, not added.",
        ));
    }

    widgets::card(
        "Inventory",
        Some(format!("{} stacks", items.len())),
        children,
    )
}

/// Secondary line shown for an item, in the inventory and in the database.
///
/// Without the game data the name is just `Item <id>`, so the subtitle only
/// carries the type to avoid repeating the id.
#[must_use]
pub fn item_subtitle(item_id: u16, category: Option<&str>, kind: u8) -> String {
    match category {
        Some(category) => format!("{category} · #{item_id}"),
        None => format!("type {kind}"),
    }
}

fn item_row(
    ctx: &Ctx,
    item: &ItemRow,
    expanded: State<Option<usize>>,
    mut hovered: State<Option<crate::app::HoverKey>>,
) -> Element {
    let key = crate::app::HoverKey::Item(item.struct_index);
    let is_hovered = *hovered.read() == Some(key);
    let minus = {
        let ctx = ctx.clone();
        let index = item.struct_index;
        move |_| ctx.run(|session| session.adjust_item_count(index, -1))
    };
    let plus = {
        let ctx = ctx.clone();
        let index = item.struct_index;
        move |_| ctx.run(|session| session.adjust_item_count(index, 1))
    };
    let remove = {
        let ctx = ctx.clone();
        let index = item.struct_index;
        move |_| ctx.run(|session| session.remove_item(index))
    };
    let is_expanded = *expanded.read() == Some(item.struct_index);

    rect()
        .vertical()
        .width(Size::fill())
        .background(if is_hovered {
            theme::SURFACE_HOVER
        } else {
            theme::SURFACE
        })
        .corner_radius(8.)
        .on_pointer_enter(move |_| *hovered.write() = Some(key))
        .on_pointer_leave(move |_| {
            if *hovered.read() == Some(key) {
                *hovered.write() = None;
            }
        })
        .child(
            rect()
                .horizontal()
                .width(Size::fill())
                .spacing(10.)
                .content(Content::Flex)
                .padding(Gaps::new_symmetric(4., 6.))
                .child(widgets::expanding_name(
                    item.struct_index,
                    item.name.clone(),
                    item_subtitle(item.item_id, item.category.as_deref(), item.kind),
                    expanded,
                    13.,
                ))
                .child(Button::new().compact().flat().child("−").on_press(minus))
                .child(
                    label()
                        .text(item.count.to_string())
                        .font_size(13.)
                        .width(Size::px(30.))
                        .text_align(TextAlign::Center),
                )
                .child(Button::new().compact().flat().child("+").on_press(plus))
                .child(
                    Button::new()
                        .compact()
                        .flat()
                        .child("Remove")
                        .on_press(remove),
                ),
        )
        .maybe_child(if is_expanded {
            widgets::details(item.description.as_deref())
        } else {
            None
        })
        .into_element()
}

fn suggestion_row(
    ctx: &Ctx,
    suggestion: &ItemSuggestion,
    expanded: State<Option<u16>>,
    mut hovered: State<Option<crate::app::HoverKey>>,
) -> Element {
    let key = crate::app::HoverKey::ItemSuggestion(suggestion.item_id);
    let is_hovered = *hovered.read() == Some(key);
    let add = {
        let ctx = ctx.clone();
        let item_id = suggestion.item_id;
        move |_| {
            ctx.run(|session| {
                let _ = session.add_item(item_id);
            });
        }
    };
    let is_expanded = *expanded.read() == Some(suggestion.item_id);
    rect()
        .vertical()
        .width(Size::fill())
        .background(if is_hovered {
            theme::SURFACE
        } else {
            theme::PANEL
        })
        .corner_radius(6.)
        .on_pointer_enter(move |_| *hovered.write() = Some(key))
        .on_pointer_leave(move |_| {
            if *hovered.read() == Some(key) {
                *hovered.write() = None;
            }
        })
        .child(
            rect()
                .horizontal()
                .width(Size::fill())
                .spacing(10.)
                .content(Content::Flex)
                .padding(Gaps::new_symmetric(3., 4.))
                .child(widgets::expanding_name(
                    suggestion.item_id,
                    suggestion.name.clone(),
                    item_subtitle(
                        suggestion.item_id,
                        Some(suggestion.category.as_str()),
                        suggestion.save_type,
                    ),
                    expanded,
                    12.,
                ))
                .child(Button::new().compact().outline().child("Add").on_press(add)),
        )
        .maybe_child(if is_expanded {
            widgets::details(suggestion.description.as_deref())
        } else {
            None
        })
        .into_element()
}
