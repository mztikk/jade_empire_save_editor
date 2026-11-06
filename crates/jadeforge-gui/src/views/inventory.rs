//! The inventory tab: item stacks, the searchable item database and removal.

use freya::prelude::*;

use crate::app::Ctx;
use crate::i18n::{fill, t};
use crate::message::capitalize;
use crate::session::{ItemRow, ItemSuggestion, Section};
use crate::theme;
use crate::widgets::{self, AccessibleButton, Action};

/// Renders the inventory tab.
#[allow(
    clippy::too_many_arguments, // one argument per reactive tab input; grouping would only rename them
    clippy::too_many_lines // one declarative tab tree keeps the visual order auditable
)]
pub fn view(
    ctx: &Ctx,
    items: &[ItemRow],
    query: State<String>,
    category: State<Option<String>>,
    categories: &[String],
    results: &[ItemSuggestion],
    game_ready: bool,
    game_loading: bool,
    matching: usize,
    total_items: usize,
    expanded: State<Option<usize>>,
    expanded_suggestion: State<Option<u16>>,
    hovered: State<Option<crate::app::HoverKey>>,
    filter_id: AccessibilityId,
) -> Element {
    let mut rows: Vec<Element> = Vec::new();
    for item in items {
        rows.push(item_row(ctx, item, expanded, hovered, filter_id));
    }
    if rows.is_empty() {
        rows.push(match ctx.session.borrow().section_error(Section::Items) {
            Some(error) => widgets::hint(fill(
                "inventory.section_error",
                &[("error", &error.message)],
            )),
            None => widgets::hint(t("inventory.empty_hint")),
        });
    }

    let mut children = vec![
        rect()
            .vertical()
            .width(Size::fill())
            .spacing(theme::space::xs())
            .children(rows)
            .into_element(),
        widgets::divider(),
        widgets::section_label(t("inventory.add_heading")),
    ];

    if game_ready {
        children.push(widgets::filter_input(
            query,
            crate::app::ITEM_FILTER_PLACEHOLDER,
            "inventory.clear_filter",
            filter_id,
        ));
        if !categories.is_empty() {
            children.push(category_filter(category, categories));
        }
        let active_category = category.read().clone();
        if results.is_empty() {
            children.push(widgets::hint(if query.read().trim().is_empty() {
                if active_category.is_some() {
                    t("inventory.no_category")
                } else {
                    t("inventory.empty_database")
                }
            } else {
                t("inventory.no_match")
            }));
        } else {
            let browsing = query.read().trim().is_empty() && active_category.is_none();
            let total = if browsing { total_items } else { matching };
            let heading_text = active_category.as_deref().map_or_else(
                || {
                    if browsing {
                        t("inventory.all_chip").to_uppercase()
                    } else {
                        t("inventory.search_results").to_owned()
                    }
                },
                |category| crate::session::item_category_label(category).to_uppercase(),
            );
            let count_text = if browsing {
                widgets::plural(total, "count.item.one", "count.item.other")
            } else {
                widgets::plural(total, "count.match.one", "count.match.other")
            };
            let heading = rect()
                .horizontal()
                .width(Size::fill())
                .spacing(theme::space::lg())
                .content(Content::Flex)
                .child(widgets::section_label(&heading_text))
                .child(rect().width(Size::flex(1.0)))
                .child(
                    widgets::a11y_text(count_text)
                        .font_size(theme::font::micro())
                        .color(theme::dim()),
                );
            children.push(heading.into_element());
            let selected = *expanded_suggestion.read();
            let mut picker_rows = Vec::with_capacity(results.len());
            for suggestion in results {
                let id = suggestion.item_id;
                let selectable = suggestion
                    .description
                    .as_deref()
                    .is_some_and(|text| !text.trim().is_empty());
                let select = selectable.then_some(expanded_suggestion);
                picker_rows.push(widgets::PickerRow {
                    key: id,
                    title: suggestion.name.clone(),
                    subtitle: item_subtitle(
                        id,
                        Some(suggestion.category.as_str()),
                        suggestion.save_type,
                    ),
                    accessible: fill("inventory.accessible", &[("name", &suggestion.name)]),
                    action_label: fill("gems.add_action", &[("name", &suggestion.name)]),
                    action_text: t("gems.add").to_owned(),
                    action: {
                        let ctx = ctx.clone();
                        let id = suggestion.item_id;
                        Action::new(move || {
                            ctx.run(|session| {
                                let _ = session.add_item(id);
                            });
                        })
                    },
                    select,
                    selected: selected == Some(id),
                });
            }
            children.push(widgets::picker_list(t("inventory.add_list"), picker_rows));
            if let Some(selected) = selected
                && let Some(suggestion) = results.iter().find(|row| row.item_id == selected)
            {
                children.extend(widgets::details(suggestion.description.as_deref()));
            }
        }
    } else if game_loading {
        children.push(widgets::hint(t("inventory.reading_hint")));
    } else {
        children.push(widgets::hint(t("inventory.missing_hint")));
    }

    widgets::card(
        capitalize(t("label.inventory")),
        Some(widgets::plural(
            items.len(),
            "count.stack.one",
            "count.stack.other",
        )),
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
        Some(category) => format!(
            "{} · #{item_id}",
            crate::session::item_category_label(category)
        ),
        None => fill("inventory.type", &[("kind", &kind.to_string())]),
    }
}

/// Category chips for the item picker.
///
/// The database is 291 rows long, so a category filter is the difference
/// between browsing and scrolling. `All` is a chip as well, so clearing the
/// category never depends on knowing that pressing the active chip toggles it.
fn category_filter(mut category: State<Option<String>>, categories: &[String]) -> Element {
    let mut sorted: Vec<&String> = categories.iter().collect();
    sorted.sort_by_key(|raw| crate::session::item_category_label(raw));
    let active = category.read().clone();
    let mut chips: Vec<Element> = Vec::new();
    chips.push(category_chip(
        t("inventory.all_chip").to_owned(),
        t("inventory.all_chip.description").to_owned(),
        active.is_none(),
        Action::new(move || category.set(None)),
    ));
    for raw in sorted {
        let label = crate::session::item_category_label(raw);
        let mut category = category;
        let selected = active.as_deref() == Some(raw.as_str());
        let value = raw.clone();
        chips.push(category_chip(
            label.clone(),
            fill("inventory.category_filter", &[("label", &label)]),
            selected,
            Action::new(move || category.set(Some(value.clone()))),
        ));
    }
    rect()
        .horizontal()
        .width(Size::fill())
        .spacing(theme::space::sm())
        .content(Content::wrap_spacing(theme::space::sm()))
        .child(widgets::section_label(t("inventory.category")))
        .children(chips)
        .into_element()
}

/// One category chip.
fn category_chip(label: String, accessible: String, selected: bool, action: Action) -> Element {
    AccessibleButton::new(
        accessible,
        action,
        rect()
            .padding(Gaps::new_symmetric(theme::space::xxs(), theme::space::md()))
            .background(if selected {
                theme::surface_active()
            } else {
                theme::surface()
            })
            .corner_radius(theme::radius::pill())
            .child(
                widgets::a11y_text(label)
                    .font_size(theme::font::micro())
                    .max_lines(1)
                    .color(if selected {
                        theme::accent()
                    } else {
                        theme::muted()
                    }),
            ),
    )
    .role(AccessibilityRole::RadioButton)
    .selected(selected)
    .hover_background(theme::surface_hover())
    .into_element()
}

#[allow(clippy::too_many_lines)] // one declarative row keeps the actions together
fn item_row(
    ctx: &Ctx,
    item: &ItemRow,
    expanded: State<Option<usize>>,
    mut hovered: State<Option<crate::app::HoverKey>>,
    filter_id: AccessibilityId,
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
                t("inventory.remove.title"),
                &fill("inventory.remove.confirm", &[("name", &name)]),
            ) {
                return;
            }
            ctx.run(|session| session.remove_item(index));
            // The focused Remove button just disappeared; keep the keyboard
            // user inside the list by focusing its filter.
            filter_id.request_focus();
        })
    };
    let set_count = {
        let ctx = ctx.clone();
        let index = item.struct_index;
        let name = item.name.clone();
        move |text: String| {
            let Ok(count) = text.trim().parse::<u8>() else {
                ctx.run(|session| {
                    session.set_error(&fill("inventory.stack_error", &[("name", &name)]));
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
        .a11y_alt(fill("inventory.accessible", &[("name", &item.name)]))
        .background(if is_hovered {
            theme::surface_hover()
        } else {
            theme::surface()
        })
        .corner_radius(theme::radius::medium())
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
                .spacing(theme::space::sm())
                .content(Content::Flex)
                .cross_align(Alignment::Center)
                .padding(Gaps::new_symmetric(theme::space::xs(), theme::space::sm()))
                .child(widgets::expanding_name(
                    item.struct_index,
                    item.name.clone(),
                    item_subtitle(item.item_id, item.category.as_deref(), item.kind),
                    expanded,
                    theme::font::body(),
                ))
                .child(widgets::row_action(
                    fill("inventory.decrease", &[("name", &item.name)]),
                    "−",
                    minus,
                ))
                .child(
                    // Direct numeric editing: a 254-item stack is one edit,
                    // not 254 stepper clicks. Enter applies the value.
                    Input::new(WritableSetter::new(item.count.to_string(), |_: String| {}))
                        .width(Size::px(theme::layout::item_count_width()))
                        .text_align(TextAlign::Center)
                        .compact()
                        .on_submit(set_count),
                )
                .child(widgets::row_action(
                    fill("inventory.increase", &[("name", &item.name)]),
                    "+",
                    plus,
                ))
                .child(widgets::row_action(
                    fill("gems.remove.action", &[("name", &item.name)]),
                    t("gems.remove"),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_subtitle_keeps_the_friendly_category_and_id_together() {
        assert_eq!(item_subtitle(42, Some("plot_items"), 6), "Plot items · #42");
        assert_eq!(item_subtitle(42, Some("improve"), 8), "Improvements · #42");
        assert_eq!(item_subtitle(42, None, 9), "type 9");
    }
}
