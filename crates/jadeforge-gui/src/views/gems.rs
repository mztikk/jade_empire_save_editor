//! The gems tab: amulet sockets and carried gems.
//!
//! Editing is limited to moving a gem between the two lists and removing one:
//! the gem struct itself is never rewritten, so its effects stay untouched.

use std::rc::Rc;

use freya::prelude::*;

use crate::app::Ctx;
use crate::i18n::{fill, keys, t};
use crate::message::capitalize;
use crate::session::{GemRow, Section};
use crate::theme;
use crate::widgets::{self, Action, PickerRow};
use jadeforge_core::{AmuletInfo, GemLocation};

/// Renders the gems tab.
#[allow(
    clippy::too_many_lines, // one declarative tree keeps the visual order auditable
    clippy::too_many_arguments // one argument per independent tab input
)]
pub fn view(
    ctx: &Ctx,
    gems: &[GemRow],
    query: State<String>,
    inventory_query: State<String>,
    available: &Rc<Vec<PickerRow>>,
    database_empty: bool,
    amulet: Option<AmuletInfo>,
    hovered: State<Option<crate::app::HoverKey>>,
    filter_id: AccessibilityId,
    inventory_filter_id: AccessibilityId,
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
    let carried = inventory.iter().filter(|gem| !gem.empty).count();
    let inventory_needle = inventory_query.read().trim().to_lowercase();
    let matching_inventory: Vec<&GemRow> = inventory
        .iter()
        .copied()
        .filter(|gem| gem_matches_filter(gem, &inventory_needle))
        .collect();
    let matching_carried = matching_inventory.iter().filter(|gem| !gem.empty).count();
    let carried_hint = if inventory_needle.is_empty() {
        carried
    } else {
        matching_carried
    };
    let can_socket = amulet.is_some_and(|amulet| socketed < usize::from(amulet.sockets));
    let amulet_title = amulet.map_or_else(
        || t(keys::GEMS_NO_AMULET).to_owned(),
        |amulet| amulet_summary(amulet, socketed),
    );
    let amulet_error = ctx.session.borrow().section_error(Section::Amulet).cloned();
    let gems_error = ctx.session.borrow().section_error(Section::Gems).cloned();
    let amulet_rows: Vec<Element> = if let Some(error) = &amulet_error {
        vec![widgets::hint(super::section_unreadable(
            Section::Amulet,
            &error.message,
        ))]
    } else if amulet.is_none() {
        vec![widgets::hint(t(keys::GEMS_AMULET_LOCKED))]
    } else if amulet_gems.is_empty() {
        vec![widgets::hint(t(keys::GEMS_EMPTY_AMULET))]
    } else {
        amulet_gems
            .into_iter()
            .map(|gem| gem_row(ctx, gem, true, true, amulet.is_some(), hovered, filter_id))
            .collect()
    };
    let inventory_rows: Vec<Element> = matching_inventory
        .into_iter()
        .map(|gem| {
            gem_row(
                ctx,
                gem,
                false,
                can_socket,
                amulet.is_some(),
                hovered,
                inventory_filter_id,
            )
        })
        .collect();

    let matching = available.len();
    let mut add_children = vec![widgets::filter_input(
        query,
        crate::app::GEM_FILTER_PLACEHOLDER,
        keys::GEMS_CLEAR_FILTER,
        filter_id,
    )];
    add_children.push(
        rect()
            .horizontal()
            .width(Size::fill())
            .spacing(theme::space::md())
            .content(Content::Flex)
            .child(
                widgets::a11y_text(widgets::plural(
                    matching,
                    keys::COUNT_GEM_ONE,
                    keys::COUNT_GEM_OTHER,
                ))
                .font_size(theme::font::micro())
                .color(theme::dim()),
            )
            .into_element(),
    );
    if available.is_empty() {
        add_children.push(widgets::hint(if database_empty {
            t(keys::GEMS_MISSING_HINT)
        } else {
            t(keys::GEMS_NO_MATCH)
        }));
    } else {
        add_children.push(widgets::picker_list_shared(
            t(keys::GEMS_ADD_LIST),
            available.clone(),
        ));
    }
    let add_card = widgets::card(
        t(keys::GEMS_ADD_HEADING),
        Some(t(keys::GEMS_FROM_GAME_DATA).to_owned()),
        add_children,
    );

    rect()
        .vertical()
        .width(Size::fill())
        .spacing(theme::space::xxl())
        .child(widgets::card(
            capitalize(t(keys::LABEL_AMULET)),
            Some(amulet_title),
            {
                let mut children = vec![
                    rect()
                        .vertical()
                        .width(Size::fill())
                        .spacing(theme::space::xs())
                        .children(amulet_rows)
                        .into_element(),
                ];
                if let Some(amulet) = amulet {
                    children.push(widgets::hint(if amulet.sockets >= 7 {
                        t(keys::GEMS_ALL_SOCKETS)
                    } else {
                        t(keys::GEMS_MORE_SOCKETS)
                    }));
                }
                children
            },
        ))
        // The carried gems come before the long picker: the owned list is what
        // the player looks at first, and the 95-row add list would otherwise
        // push the inventory card below the fold.
        .child(widgets::card(
            capitalize(t(keys::LABEL_INVENTORY)),
            Some(widgets::plural(
                carried_hint,
                keys::COUNT_GEM_CARRIED_ONE,
                keys::COUNT_GEM_CARRIED_OTHER,
            )),
            {
                let mut children = Vec::new();
                if gems_error.is_none() {
                    children.push(widgets::filter_input(
                        inventory_query,
                        keys::FILTER_GEMS_INVENTORY,
                        keys::GEMS_CLEAR_INVENTORY_FILTER,
                        inventory_filter_id,
                    ));
                }
                if let Some(error) = &gems_error {
                    children.push(widgets::hint(super::section_unreadable(
                        Section::Gems,
                        &error.message,
                    )));
                } else if inventory.is_empty() && inventory_needle.is_empty() {
                    children.push(widgets::hint(t(keys::GEMS_EMPTY_INVENTORY)));
                } else if inventory_rows.is_empty() {
                    children.push(widgets::hint(t(keys::GEMS_NO_INVENTORY_MATCH)));
                } else {
                    children.push(carried_gem_list(
                        t(keys::GEMS_INVENTORY_LIST),
                        inventory_rows,
                    ));
                }
                children.push(widgets::hint(t(keys::GEMS_MOVE_HINT)));
                children
            },
        ))
        .child(add_card)
        .into_element()
}

/// Number of carried gem rows the viewport shows before it scrolls.
///
/// The add-gem picker below uses the same idea; both lists stay a fixed block
/// instead of letting a long one push the other out of the page.
const CARRIED_VISIBLE_ROWS: usize = 7;

/// Whether a carried gem matches a filter term.
///
/// The effect line is part of the visible row, so searching for an effect such
/// as `+2 Body` narrows the list as usefully as searching for the gem name.
fn gem_matches_filter(gem: &GemRow, needle: &str) -> bool {
    needle.is_empty()
        || gem.name.to_lowercase().contains(needle)
        || gem
            .effects
            .iter()
            .any(|effect| effect.to_lowercase().contains(needle))
}

/// Height of the carried gem viewport for a list of `rows` elements.
///
/// Short lists shrink to their own height; longer ones stop at
/// [`CARRIED_VISIBLE_ROWS`] and scroll inside the card.
fn carried_list_height(rows: usize) -> f32 {
    let visible = rows.clamp(1, CARRIED_VISIBLE_ROWS);
    let gaps = visible.saturating_sub(1);
    theme::layout::gem_row_height() * visible as f32 + theme::space::xs() * gaps as f32
}

/// A scrollable carried-gem list capped at [`CARRIED_VISIBLE_ROWS`] rows.
fn carried_gem_list(label: &str, rows: Vec<Element>) -> Element {
    rect()
        .width(Size::fill())
        .height(Size::px(carried_list_height(rows.len())))
        .a11y_role(AccessibilityRole::ListBox)
        .a11y_alt(label.to_owned())
        .child(
            ScrollView::new()
                .width(Size::fill())
                .height(Size::fill())
                .spacing(theme::space::xs())
                .show_scrollbar(true)
                .auto_hide_scrollbars(false)
                .drag_scrolling(false)
                .key("carried-gem-rows")
                .children(rows),
        )
        .into_element()
}

/// Summary of the amulet state shown in the card hint.
///
/// The socket count is the number of unlocked sockets, not the seven physical
/// entries of `Amulet.OpenGemsList`; the helper keeps the noun singular and
/// names the tier so the hint cannot read like a bare count.
#[must_use]
pub fn amulet_summary(amulet: AmuletInfo, socketed: usize) -> String {
    fill(
        keys::GEMS_SUMMARY,
        &[
            ("socketed", &socketed.to_string()),
            (
                "sockets",
                &widgets::plural(
                    usize::from(amulet.sockets),
                    keys::COUNT_SOCKET_ONE,
                    keys::COUNT_SOCKET_OTHER,
                ),
            ),
            ("tier", &amulet.tier.to_string()),
        ],
    )
}

#[allow(clippy::too_many_lines)] // one declarative row keeps the actions together
fn gem_row(
    ctx: &Ctx,
    gem: &GemRow,
    socketed: bool,
    can_move: bool,
    has_amulet: bool,
    mut hovered: State<Option<crate::app::HoverKey>>,
    filter_id: AccessibilityId,
) -> Element {
    let ctx = ctx.clone();
    let index = gem.struct_index;
    let name = gem.name.clone();
    let key = crate::app::HoverKey::Gem(index);
    let is_hovered = *hovered.read() == Some(key);
    let action = if socketed {
        t(keys::GEMS_MOVE_TO_INVENTORY)
    } else {
        t(keys::GEMS_MOVE_TO_AMULET)
    };
    let mut actions: Vec<Element> = Vec::new();
    if !gem.empty {
        let remove_ctx = ctx.clone();
        if can_move {
            let move_ctx = ctx.clone();
            let move_name = name.clone();
            actions.push(
                widgets::row_action(
                    fill(
                        keys::GEMS_MOVE_ACTION,
                        &[("action", action), ("name", &move_name)],
                    ),
                    action,
                    Action::new(move || move_ctx.run(|session| session.move_gem(index))),
                )
                .into_element(),
            );
        } else {
            actions.push(
                widgets::a11y_text(if has_amulet {
                    t(keys::GEMS_AMULET_FULL)
                } else {
                    t(keys::GEMS_NO_AMULET)
                })
                .font_size(theme::font::micro())
                .color(theme::dim())
                .into_element(),
            );
        }
        let remove_name = gem.name.clone();
        actions.push(
            widgets::row_action(
                fill(keys::GEMS_REMOVE_ACTION, &[("name", &remove_name)]),
                t(keys::GEMS_REMOVE),
                Action::new(move || {
                    if !crate::dialog::confirm(
                        &fill(keys::DIALOG_REMOVE, &[("kind", t(keys::COUNT_GEM_ONE))]),
                        &fill(keys::GEMS_REMOVE_CONFIRM, &[("name", &remove_name)]),
                    ) {
                        return;
                    }
                    remove_ctx.run(|session| session.remove_gem(index));
                    filter_id.request_focus();
                }),
            )
            .into_element(),
        );
    }
    rect()
        .horizontal()
        .width(Size::fill())
        .key(index)
        .a11y_role(AccessibilityRole::Group)
        .a11y_alt(fill(keys::GEMS_ROW_ACCESSIBLE, &[("name", &gem.name)]))
        .spacing(theme::space::lg())
        .content(Content::Flex)
        .padding(Gaps::new_symmetric(theme::space::sm(), theme::space::lg()))
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
                .vertical()
                .spacing(theme::space::xxs())
                .width(Size::flex(1.0))
                .child(widgets::with_tooltip(
                    gem.name.clone(),
                    widgets::a11y_text(gem.name.clone())
                        .font_size(theme::font::body())
                        .max_lines(1)
                        .text_overflow(TextOverflow::Ellipsis)
                        .color(if gem.empty {
                            theme::dim()
                        } else {
                            theme::text()
                        }),
                ))
                .child(
                    widgets::a11y_text(if gem.effects.is_empty() {
                        t(keys::GEMS_NO_EFFECTS).to_owned()
                    } else {
                        gem.effects.join(" · ")
                    })
                    .font_size(theme::font::micro())
                    .max_lines(2)
                    .color(theme::dim()),
                ),
        )
        .children(actions)
        .into_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_carried_list_stops_growing_at_the_viewport_cap() {
        assert!(carried_list_height(2) > carried_list_height(1));
        assert!(carried_list_height(7) > carried_list_height(6));
        assert_eq!(
            carried_list_height(CARRIED_VISIBLE_ROWS),
            carried_list_height(40)
        );
    }

    #[test]
    fn the_carried_filter_matches_names_and_effects() {
        let gem = GemRow {
            struct_index: 3,
            name: "Aurajuwel der Bosheit".to_owned(),
            effects: vec!["+2 Intimidate".to_owned(), "+2 Body".to_owned()],
            empty: false,
            location: GemLocation::Inventory,
        };
        assert!(gem_matches_filter(&gem, "aurajuwel"));
        assert!(gem_matches_filter(&gem, "+2 body"));
        assert!(gem_matches_filter(&gem, ""));
        assert!(!gem_matches_filter(&gem, "friendship"));
    }

    #[test]
    fn the_amulet_summary_names_both_counts_and_the_tier() {
        assert_eq!(
            amulet_summary(
                AmuletInfo {
                    tier: 2,
                    sockets: 5
                },
                1
            ),
            "1 of 5 sockets filled · amulet tier 2"
        );
        assert_eq!(
            amulet_summary(
                AmuletInfo {
                    tier: 1,
                    sockets: 3
                },
                0
            ),
            "0 of 3 sockets filled · amulet tier 1"
        );
    }
}
