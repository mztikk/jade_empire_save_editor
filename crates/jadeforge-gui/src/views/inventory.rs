//! The inventory tab: item stacks, search and removal.

use freya::prelude::*;

use crate::app::Ctx;
use crate::session::{ItemRow, ItemSuggestion};
use crate::theme;
use crate::widgets;

/// Renders the inventory tab.
pub fn view(
    ctx: &Ctx,
    items: &[ItemRow],
    query: State<String>,
    results: &[ItemSuggestion],
    game_ready: bool,
) -> Element {
    let mut rows: Vec<Element> = Vec::new();
    for item in items {
        rows.push(item_row(ctx, item));
    }
    if rows.is_empty() {
        rows.push(
            rect()
                .padding(Gaps::new_all(16.))
                .child(
                    label()
                        .text("The inventory is empty.")
                        .font_size(12.)
                        .color(theme::DIM),
                )
                .into_element(),
        );
    }

    let add_section = if game_ready {
        let mut block: Vec<Element> = vec![
            Input::new(query)
                .placeholder("Search items by name or template, for example \"sword\"")
                .width(Size::fill())
                .into_element(),
        ];
        if !results.is_empty() {
            let mut result_rows: Vec<Element> = Vec::new();
            for suggestion in results {
                result_rows.push(suggestion_row(ctx, suggestion));
            }
            block.push(
                rect()
                    .vertical()
                    .width(Size::fill())
                    .spacing(2.)
                    .children(result_rows)
                    .into_element(),
            );
        } else if !query.read().trim().is_empty() {
            block.push(widgets::hint("No items match this search."));
        }
        block
    } else {
        vec![widgets::hint(
            "The game data was not found, so items can only be removed, not added.",
        )]
    };

    widgets::card(
        "Inventory",
        Some(format!("{} stacks", items.len())),
        vec![
            rect()
                .vertical()
                .width(Size::fill())
                .spacing(4.)
                .children(rows)
                .into_element(),
            widgets::divider(),
            widgets::section_label("ADD AN ITEM"),
        ]
        .into_iter()
        .chain(add_section)
        .collect::<Vec<_>>(),
    )
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
                        .text(format!("id {} · type {}", item.item_id, item.kind))
                        .font_size(10.)
                        .color(theme::DIM),
                ),
        )
        .child(match &item.category {
            Some(category) => widgets::badge(category.clone(), theme::SURFACE_ACTIVE, theme::MUTED),
            None => label().text("").font_size(10.).into_element(),
        })
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
            label()
                .text(suggestion.name.clone())
                .font_size(12.)
                .max_lines(1)
                .width(Size::flex(1.0))
                .color(theme::TEXT),
        )
        .child(
            label()
                .text(format!("{} · #{}", suggestion.category, suggestion.item_id))
                .font_size(10.)
                .color(theme::DIM),
        )
        .child(Button::new().compact().outline().child("Add").on_press(add))
        .into_element()
}
