//! The companions tab.

use freya::prelude::*;

use crate::app::Ctx;
use crate::session::{HenchRow, Section};
use crate::theme;
use crate::widgets::{self, Action};

/// Renders the companions tab.
pub fn view(
    ctx: &Ctx,
    henchmen: &[HenchRow],
    hovered: State<Option<crate::app::HoverKey>>,
) -> Element {
    let mut rows: Vec<Element> = Vec::new();
    for henchman in henchmen {
        rows.push(henchman_row(ctx, henchman, hovered));
    }
    if rows.is_empty() {
        rows.push(match ctx.session.borrow().section_error(Section::Companions) {
            Some(error) => widgets::hint(format!(
                "The companions section could not be read: {}",
                error.message
            )),
            None => widgets::hint(
                "This savegame has no companions yet. They appear here once the story introduces them.",
            ),
        });
    }

    widgets::card(
        "Companions",
        Some(format!("{} known", henchmen.len())),
        vec![
            rect()
                .vertical()
                .width(Size::fill())
                .spacing(theme::space::XS)
                .children(rows)
                .into_element(),
            widgets::divider(),
            widgets::hint(
                "Companion levels follow the player through the game; raising them here only \
                 changes what the companion starts with.",
            ),
        ],
    )
}

#[allow(clippy::too_many_lines)] // one declarative row keeps the actions together
fn henchman_row(
    ctx: &Ctx,
    henchman: &HenchRow,
    mut hovered: State<Option<crate::app::HoverKey>>,
) -> Element {
    let index = henchman.struct_index;
    let level = henchman.level;
    let available = henchman.available;
    let name = henchman.name.clone();
    let key = crate::app::HoverKey::Companion(index);
    let is_hovered = *hovered.read() == Some(key);

    let lower = {
        let ctx = ctx.clone();
        let name = name.clone();
        Action::new(move || {
            ctx.run(|session| {
                session.set_henchman_level(index, level.saturating_sub(1).max(1));
            });
            let _ = &name;
        })
    };
    let raise = {
        let ctx = ctx.clone();
        let name = name.clone();
        Action::new(move || {
            ctx.run(|session| {
                session.set_henchman_level(index, level.saturating_add(1).min(60));
            });
            let _ = &name;
        })
    };
    let toggle = {
        let ctx = ctx.clone();
        Action::new(move || ctx.run(|session| session.toggle_henchman_available(index)))
    };
    let set_level = {
        let ctx = ctx.clone();
        let name = name.clone();
        move |text: String| {
            let Ok(value) = text.trim().parse::<u8>() else {
                ctx.run(|session| {
                    session.set_error(&format!("{name}: enter a level from 1 to 60."));
                });
                return;
            };
            ctx.run(|session| session.set_henchman_level(index, value));
        }
    };

    rect()
        .horizontal()
        .width(Size::fill())
        .key(index)
        .a11y_role(AccessibilityRole::Group)
        .a11y_alt(format!("{} companion", henchman.name))
        .spacing(theme::space::LG)
        .content(Content::Flex)
        .padding(Gaps::new_symmetric(theme::space::SM, theme::space::LG))
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
                .vertical()
                .spacing(theme::space::XXS)
                .width(Size::flex(1.0))
                .child(widgets::with_tooltip(
                    henchman.name.clone(),
                    widgets::a11y_text(henchman.name.clone())
                        .font_size(theme::font::BODY)
                        .max_lines(1)
                        .text_overflow(TextOverflow::Ellipsis)
                        .color(theme::TEXT),
                ))
                .child(
                    widgets::a11y_text(henchman.resref.clone())
                        .font_size(theme::font::MICRO)
                        .color(theme::DIM),
                ),
        )
        .child(if available {
            widgets::badge("available", theme::SURFACE_ACTIVE, theme::JADE)
        } else {
            widgets::badge("hidden", theme::SURFACE_ACTIVE, theme::DANGER)
        })
        .child(widgets::row_action(
            format!("Decrease {name} level"),
            "−",
            lower,
        ))
        .child(
            // Direct numeric editing: level 60 from level 1 is one edit.
            Input::new(WritableSetter::new(level.to_string(), |_: String| {}))
                .width(Size::px(theme::layout::STEPPER_WIDTH))
                .text_align(TextAlign::Center)
                .compact()
                .on_submit(set_level),
        )
        .child(widgets::row_action(
            format!("Increase {name} level"),
            "+",
            raise,
        ))
        .child(widgets::row_action(
            if available {
                format!("Hide {name}")
            } else {
                format!("Make {name} available")
            },
            if available { "Hide" } else { "Unlock" },
            toggle,
        ))
        .into_element()
}
