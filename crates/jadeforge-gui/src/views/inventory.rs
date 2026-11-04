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
    show_more: impl FnMut(Event<PressEventData>) + 'static,
) -> Element {
    let mut rows: Vec<Element> = Vec::new();
    for item in items {
        rows.push(item_row(ctx, item));
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
                result_rows.push(suggestion_row(ctx, suggestion));
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
#[must_use]
pub fn item_subtitle(item_id: u16, category: Option<&str>, kind: u8) -> String {
    match category {
        Some(category) => format!("{category} · #{item_id}"),
        None => format!("type {kind} · #{item_id}"),
    }
}

fn item_row(ctx: &Ctx, item: &ItemRow) -> Element {
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
                        .text(item.name.clone())
                        .font_size(13.)
                        .max_lines(1)
                        .color(theme::TEXT),
                )
                .child(
                    label()
                        .text(item_subtitle(
                            item.item_id,
                            item.category.as_deref(),
                            item.kind,
                        ))
                        .font_size(10.)
                        .color(theme::DIM),
                )
                .into_element(),
        )
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
        )
        .into_element()
}

fn suggestion_row(ctx: &Ctx, suggestion: &ItemSuggestion) -> Element {
    let add = {
        let ctx = ctx.clone();
        let item_id = suggestion.item_id;
        move |_| {
            ctx.run(|session| {
                let _ = session.add_item(item_id);
            });
        }
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
                .child(
                    label()
                        .text(item_subtitle(
                            suggestion.item_id,
                            Some(suggestion.category.as_str()),
                            suggestion.save_type,
                        ))
                        .font_size(10.)
                        .color(theme::DIM),
                )
                .into_element(),
        )
        .child(Button::new().compact().outline().child("Add").on_press(add))
        .into_element()
}
