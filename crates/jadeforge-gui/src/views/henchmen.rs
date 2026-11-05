//! The companions tab.

use freya::prelude::*;

use crate::app::Ctx;
use crate::session::HenchRow;
use crate::theme;
use crate::widgets;

/// Renders the companions tab.
pub fn view(ctx: &Ctx, henchmen: &[HenchRow]) -> Element {
    let mut rows: Vec<Element> = Vec::new();
    for henchman in henchmen {
        rows.push(henchman_row(ctx, henchman));
    }
    if rows.is_empty() {
        rows.push(widgets::hint(
            "This savegame has no companions yet. They appear here once the story introduces them.",
        ));
    }

    widgets::card(
        "Companions",
        Some(format!("{} known", henchmen.len())),
        vec![
            rect()
                .vertical()
                .width(Size::fill())
                .spacing(4.)
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

fn henchman_row(ctx: &Ctx, henchman: &HenchRow) -> Element {
    let index = henchman.struct_index;
    let level = henchman.level;
    let available = henchman.available;

    let lower = {
        let ctx = ctx.clone();
        move |_| {
            ctx.run(|session| session.set_henchman_level(index, level.saturating_sub(1).max(1)));
        }
    };
    let raise = {
        let ctx = ctx.clone();
        move |_| {
            ctx.run(|session| session.set_henchman_level(index, level.saturating_add(1).min(60)));
        }
    };
    let toggle = {
        let ctx = ctx.clone();
        move |_| ctx.run(|session| session.toggle_henchman_available(index))
    };

    rect()
        .horizontal()
        .width(Size::fill())
        .key(index)
        .a11y_role(AccessibilityRole::Group)
        .a11y_alt(format!("{} companion", henchman.name))
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
                    widgets::a11y_text(henchman.name.clone())
                        .font_size(13.)
                        .max_lines(1)
                        .color(theme::TEXT),
                )
                .child(
                    widgets::a11y_text(henchman.resref.clone())
                        .font_size(10.)
                        .color(theme::DIM),
                ),
        )
        .child(if available {
            widgets::badge("available", theme::SURFACE_ACTIVE, theme::JADE)
        } else {
            widgets::badge("hidden", theme::SURFACE_ACTIVE, theme::DANGER)
        })
        .child(Button::new().compact().flat().child("−").on_press(lower))
        .child(
            widgets::a11y_text(format!("Lv {level}"))
                .font_size(12.)
                .width(Size::px(46.))
                .text_align(TextAlign::Center)
                .color(theme::ACCENT),
        )
        .child(Button::new().compact().flat().child("+").on_press(raise))
        .child(
            Button::new()
                .compact()
                .flat()
                .child(if available { "Hide" } else { "Unlock" })
                .on_press(toggle),
        )
        .into_element()
}
