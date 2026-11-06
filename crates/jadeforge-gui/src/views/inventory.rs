//! The inventory tab: item stacks, the searchable item database and removal.

use freya::prelude::*;

use crate::app::Ctx;
use crate::session::{ItemRow, ItemSuggestion, Section};
use crate::theme;
use crate::widgets::{self, Action};

/// Renders the inventory tab.
#[allow(
    clippy::too_many_arguments, // one argument per reactive tab input; grouping would only rename them
    clippy::too_many_lines // one declarative tab tree keeps the visual order auditable
)]
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
    filter_id: AccessibilityId,
    show_more: impl FnMut(Event<PressEventData>) + 'static,
) -> Element {
    let mut rows: Vec<Element> = Vec::new();
    for item in items {
        rows.push(item_row(ctx, item, expanded, hovered));
    }
    if rows.is_empty() {
        rows.push(match ctx.session.borrow().section_error(Section::Items) {
            Some(error) => widgets::hint(format!(
                "The inventory section could not be read: {}",
                error.message
            )),
            None => widgets::hint("The inventory is empty."),
        });
    }

    let mut children = vec![
        rect()
            .vertical()
            .width(Size::fill())
            .spacing(theme::space::XS)
            .children(rows)
            .into_element(),
        widgets::divider(),
        widgets::section_label("ADD AN ITEM"),
    ];

    if game_ready {
        children.push(widgets::filter_input(
            query,
            "Filter the item database, or leave empty to browse it",
            "Clear the item database filter",
            filter_id,
        ));
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
            let heading = rect()
                .horizontal()
                .width(Size::fill())
                .spacing(theme::space::LG)
                .content(Content::Flex)
                .child(widgets::section_label(if browsing {
                    "ALL ITEMS"
                } else {
                    "SEARCH RESULTS"
                }))
                .child(rect().width(Size::flex(1.0)))
                .child(
                    widgets::a11y_text(match (browsing, shown == total) {
                        (true, _) => format!("{shown} of {total}"),
                        (false, true) => widgets::plural(total, "match", "matches"),
                        (false, false) => {
                            format!("{shown} of {}", widgets::plural(total, "match", "matches"))
                        }
                    })
                    .font_size(theme::font::MICRO)
                    .color(theme::DIM),
                )
                .maybe(shown < total, |el| {
                    el.child(
                        Button::new()
                            .compact()
                            .outline()
                            .child("Show more")
                            .on_press(show_more),
                    )
                });
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
                    .spacing(theme::space::XXS)
                    .children(result_rows)
                    .into_element(),
            );
        }
    } else {
        children.push(widgets::hint(
            "The game data was not found, so items can only be removed, not added. \
             The category comes from the item type stored in the savegame.",
        ));
    }

    widgets::card(
        "Inventory",
        Some(widgets::plural(items.len(), "stack", "stacks")),
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

#[allow(clippy::too_many_lines)] // one declarative row keeps the actions together
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
        Action::new(move || ctx.run(|session| session.adjust_item_count(index, -1)))
    };
    let plus = {
        let ctx = ctx.clone();
        let index = item.struct_index;
        Action::new(move || ctx.run(|session| session.adjust_item_count(index, 1)))
    };
    let remove = {
        let ctx = ctx.clone();
        let index = item.struct_index;
        let name = item.name.clone();
        Action::new(move || {
            if !crate::dialog::confirm(
                "Remove item?",
                &format!(
                    "Remove {name} from the inventory? You can still press Revert before saving."
                ),
            ) {
                return;
            }
            ctx.run(|session| session.remove_item(index));
        })
    };
    let set_count = {
        let ctx = ctx.clone();
        let index = item.struct_index;
        let name = item.name.clone();
        move |text: String| {
            let Ok(count) = text.trim().parse::<u8>() else {
                ctx.run(|session| {
                    session.set_error(&format!("{name}: enter a stack size from 1 to 255."));
                });
                return;
            };
            ctx.run(|session| session.set_item_count(index, count));
        }
    };
    let is_expanded = *expanded.read() == Some(item.struct_index);

    rect()
        .vertical()
        .width(Size::fill())
        .key(item.struct_index)
        .a11y_role(AccessibilityRole::Group)
        .a11y_alt(format!("{} item", item.name))
        .background(if is_hovered {
            theme::SURFACE_HOVER
        } else {
            theme::SURFACE
        })
        .corner_radius(theme::radius::MEDIUM)
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
                .spacing(theme::space::SM)
                .content(Content::Flex)
                .cross_align(Alignment::Center)
                .padding(Gaps::new_symmetric(theme::space::XS, theme::space::SM))
                .child(widgets::expanding_name(
                    item.struct_index,
                    item.name.clone(),
                    item_subtitle(item.item_id, item.category.as_deref(), item.kind),
                    expanded,
                    theme::font::BODY,
                ))
                .child(widgets::row_action(
                    format!("Decrease {} stack", item.name),
                    "−",
                    minus,
                ))
                .child(
                    // Direct numeric editing: a 254-item stack is one edit,
                    // not 254 stepper clicks. Enter applies the value.
                    Input::new(WritableSetter::new(item.count.to_string(), |_: String| {}))
                        .width(Size::px(theme::layout::ITEM_COUNT_WIDTH))
                        .text_align(TextAlign::Center)
                        .compact()
                        .on_submit(set_count),
                )
                .child(widgets::row_action(
                    format!("Increase {} stack", item.name),
                    "+",
                    plus,
                ))
                .child(widgets::row_action(
                    format!("Remove {}", item.name),
                    "Remove",
                    remove,
                )),
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
        Action::new(move || {
            ctx.run(|session| {
                let _ = session.add_item(item_id);
            });
        })
    };
    let is_expanded = *expanded.read() == Some(suggestion.item_id);
    rect()
        .vertical()
        .width(Size::fill())
        .key(suggestion.item_id)
        .a11y_role(AccessibilityRole::Group)
        .a11y_alt(format!("{} item", suggestion.name))
        .background(if is_hovered {
            theme::SURFACE
        } else {
            theme::PANEL
        })
        .corner_radius(theme::radius::CONTROL)
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
                .spacing(theme::space::LG)
                .content(Content::Flex)
                .padding(Gaps::new_symmetric(theme::space::XS, theme::space::XS))
                .child(widgets::expanding_name(
                    suggestion.item_id,
                    suggestion.name.clone(),
                    item_subtitle(
                        suggestion.item_id,
                        Some(suggestion.category.as_str()),
                        suggestion.save_type,
                    ),
                    expanded,
                    theme::font::LABEL,
                ))
                .child(widgets::row_action(
                    format!("Add {}", suggestion.name),
                    "Add",
                    add,
                )),
        )
        .maybe_child(if is_expanded {
            widgets::details(suggestion.description.as_deref())
        } else {
            None
        })
        .into_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_subtitle_keeps_the_category_and_id_together() {
        assert_eq!(item_subtitle(42, Some("plot_items"), 6), "plot_items · #42");
        assert_eq!(item_subtitle(42, None, 9), "type 9");
    }
}
