//! The companions tab.

use freya::prelude::*;

use crate::app::Ctx;
use crate::i18n::{fill, t};
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
        rows.push(
            match ctx.session.borrow().section_error(Section::Companions) {
                Some(error) => widgets::hint(fill(
                    "companions.section_error",
                    &[("error", &error.message)],
                )),
                None => widgets::hint(t("companions.empty_hint")),
            },
        );
    }

    widgets::card(
        t("nav.companions"),
        Some(widgets::plural(
            henchmen.len(),
            "count.companion.known.one",
            "count.companion.known.other",
        )),
        vec![
            rect()
                .vertical()
                .width(Size::fill())
                .spacing(theme::space::xs())
                .children(rows)
                .into_element(),
            widgets::divider(),
            widgets::hint(t("companions.level_hint")),
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
                    session.set_error(&fill("companions.level_error", &[("name", &name)]));
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
        .a11y_alt(fill("companions.accessible", &[("name", &henchman.name)]))
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
                    henchman.name.clone(),
                    widgets::a11y_text(henchman.name.clone())
                        .font_size(theme::font::body())
                        .max_lines(1)
                        .text_overflow(TextOverflow::Ellipsis)
                        .color(theme::text()),
                ))
                .child(
                    widgets::a11y_text(henchman.resref.clone())
                        .font_size(theme::font::micro())
                        .color(theme::dim()),
                ),
        )
        .child(if available {
            widgets::badge(
                t("companions.available"),
                theme::surface_active(),
                theme::jade(),
            )
        } else {
            // Hidden is a resting state, not a failure: a calmer colour than
            // the danger red that made an unavailable companion look broken.
            widgets::badge(t("companions.hidden"), theme::surface(), theme::muted())
        })
        .child(widgets::row_action(
            fill("companions.decrease", &[("name", &name)]),
            "−",
            lower,
        ))
        .child(
            // Direct numeric editing: level 60 from level 1 is one edit.
            Input::new(WritableSetter::new(level.to_string(), |_: String| {}))
                .width(Size::px(theme::layout::stepper_width()))
                .text_align(TextAlign::Center)
                .compact()
                .on_submit(set_level),
        )
        .child(widgets::row_action(
            fill("companions.increase", &[("name", &name)]),
            "+",
            raise,
        ))
        .child(widgets::row_action(
            if available {
                fill("companions.hide_action", &[("name", &name)])
            } else {
                fill("companions.make_available_action", &[("name", &name)])
            },
            if available {
                t("companions.hide")
            } else {
                t("companions.make_available")
            },
            toggle,
        ))
        .into_element()
}
