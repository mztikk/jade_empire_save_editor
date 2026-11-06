//! The gems tab: amulet sockets and carried gems.
//!
//! Editing is limited to moving a gem between the two lists and removing one:
//! the gem struct itself is never rewritten, so its effects stay untouched.

use freya::prelude::*;

use crate::app::Ctx;
use crate::i18n::{fill, t};
use crate::session::{GemRow, GemSuggestion, Section};
use crate::theme;
use crate::widgets::{self, Action};
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
    available: &[GemSuggestion],
    amulet: Option<AmuletInfo>,
    hovered: State<Option<crate::app::HoverKey>>,
    filter_id: AccessibilityId,
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
    let can_socket = amulet.is_some_and(|amulet| socketed < usize::from(amulet.sockets));
    let amulet_title = amulet.map_or_else(
        || t("No amulet yet").to_owned(),
        |amulet| amulet_summary(amulet, socketed),
    );
    let amulet_error = ctx.session.borrow().section_error(Section::Amulet).cloned();
    let gems_error = ctx.session.borrow().section_error(Section::Gems).cloned();
    let amulet_rows: Vec<Element> = if let Some(error) = &amulet_error {
        vec![widgets::hint(fill(
            "The amulet section could not be read: {error}",
            &[("error", &error.message)],
        ))]
    } else if amulet.is_none() {
        vec![widgets::hint(t(
            "The amulet is unlocked early in the story; it has no sockets before that.",
        ))]
    } else if amulet_gems.is_empty() {
        vec![widgets::hint(t("No gems socketed into the amulet."))]
    } else {
        amulet_gems
            .into_iter()
            .map(|gem| gem_row(ctx, gem, true, true, amulet.is_some(), hovered, filter_id))
            .collect()
    };
    let inventory_rows: Vec<Element> = if let Some(error) = &gems_error {
        vec![widgets::hint(fill(
            "The gems section could not be read: {error}",
            &[("error", &error.message)],
        ))]
    } else if inventory.is_empty() {
        vec![widgets::hint(t("No gems carried in the inventory."))]
    } else {
        inventory
            .into_iter()
            .map(|gem| {
                gem_row(
                    ctx,
                    gem,
                    false,
                    can_socket,
                    amulet.is_some(),
                    hovered,
                    filter_id,
                )
            })
            .collect()
    };

    let needle = query.read().trim().to_lowercase();
    let matching: Vec<&GemSuggestion> = available
        .iter()
        .filter(|gem| needle.is_empty() || gem.name.to_lowercase().contains(&needle))
        .collect();
    let mut picker_rows = Vec::with_capacity(matching.len());
    for gem in &matching {
        let ctx = ctx.clone();
        let item_id = gem.item_id;
        let effects = if gem.effects.is_empty() {
            t("no effects").to_owned()
        } else {
            gem.effects.join(" · ")
        };
        picker_rows.push(widgets::PickerRow {
            key: item_id,
            accessible: fill(
                "{name} gem, {effects}",
                &[("name", &gem.name), ("effects", &effects)],
            ),
            title: gem.name.clone(),
            subtitle: effects.clone(),
            action_label: fill("Add {name}", &[("name", &gem.name)]),
            action_text: t("Add").to_owned(),
            action: Action::new(move || ctx.run(|session| session.add_gem(item_id))),
            select: None,
            selected: false,
        });
    }
    let mut add_children = vec![widgets::filter_input(
        query,
        crate::app::GEM_FILTER_PLACEHOLDER,
        "Clear the gem filter",
        filter_id,
    )];
    add_children.push(
        rect()
            .horizontal()
            .width(Size::fill())
            .spacing(theme::space::md())
            .content(Content::Flex)
            .child(
                widgets::a11y_text(widgets::plural(matching.len(), "gem", "gems"))
                    .font_size(theme::font::micro())
                    .color(theme::dim()),
            )
            .into_element(),
    );
    if picker_rows.is_empty() {
        add_children.push(widgets::hint(if available.is_empty() {
            t("The game data with the gem definitions was not found.")
        } else {
            t("No gem matches this filter.")
        }));
    } else {
        add_children.push(widgets::picker_list(t("Gems you can add"), picker_rows));
    }
    let add_card = widgets::card(
        t("Add a gem"),
        Some(t("From the game data").to_owned()),
        add_children,
    );

    rect()
        .vertical()
        .width(Size::fill())
        .spacing(theme::space::xxl())
        .child(widgets::card(t("Amulet"),
            Some(amulet_title),
            {
                let mut children = vec![rect()
                    .vertical()
                    .width(Size::fill())
                    .spacing(theme::space::xs())
                    .children(amulet_rows)
                    .into_element()];
                if let Some(amulet) = amulet {
                    children.push(widgets::hint(if amulet.sockets >= 7 {
                        t("All seven sockets are unlocked.")
                    } else {
                        t("More sockets unlock as the story progresses.")
                    }));
                }
                children
            },
        ))
        // The carried gems come before the long picker: the owned list is what
        // the player looks at first, and the 95-row add list would otherwise
        // push the inventory card below the fold.
        .child(widgets::card(t("Inventory"),
            Some(widgets::plural(carried, "carried gem", "carried gems")),
            vec![
                rect()
                    .vertical()
                    .width(Size::fill())
                    .spacing(theme::space::xs())
                    .children(inventory_rows)
                    .into_element(),
                widgets::hint(t("Moving a gem keeps its effects; JadeForge only changes which list references it."),
                ),
            ],
        ))
        .child(add_card)
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
        "{socketed} of {sockets} filled · amulet tier {tier}",
        &[
            ("socketed", &socketed.to_string()),
            (
                "sockets",
                &widgets::plural(usize::from(amulet.sockets), "socket", "sockets"),
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
        t("Move to inventory")
    } else {
        t("Move to amulet")
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
                        "{action} ({name})",
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
                    t("Amulet full")
                } else {
                    t("No amulet yet")
                })
                .font_size(theme::font::micro())
                .color(theme::dim())
                .into_element(),
            );
        }
        let remove_name = gem.name.clone();
        actions.push(
            widgets::row_action(
                fill("Remove {name}", &[("name", &remove_name)]),
                t("Remove"),
                Action::new(move || {
                    if !crate::dialog::confirm(
                        t("Remove gem?"),
                        &fill(
                            "Remove {name}? You can still press Revert before saving.",
                            &[("name", &remove_name)],
                        ),
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
        .a11y_alt(fill("{name} gem", &[("name", &gem.name)]))
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
                        t("no effects").to_owned()
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
