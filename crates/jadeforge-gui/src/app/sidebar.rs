//! Savegame list and sort controls.

use std::rc::Rc;

use freya::elements::image::ImageHandle;
use freya::prelude::*;
use jadeforge_core::Character;

use crate::session::{SaveEntry, SaveSort};
use crate::theme;
use crate::widgets::{AccessibleButton, Action};

use super::shell::preview_slot;
use super::{Ctx, HoverKey, SORT_ARROW_WIDTH};

#[allow(
    clippy::too_many_lines,
    clippy::needless_pass_by_value,
    clippy::too_many_arguments
)]
pub(super) fn sidebar(
    ctx: &Ctx,
    entries: &[SaveEntry],
    visible: &[usize],
    selected: Option<usize>,
    preview: Option<ImageHandle>,
    character: Option<&Character>,
    form_differs: bool,
    query: State<String>,
    sort: State<(SaveSort, bool)>,
    hovered: State<Option<HoverKey>>,
    sync: Rc<dyn Fn()>,
) -> Element {
    let mut rows: Vec<Element> = Vec::new();
    for &index in visible {
        let entry = &entries[index];
        let ctx = ctx.clone();
        let sync = sync.clone();
        let is_selected = selected == Some(index);
        let is_hovered = *hovered.read() == Some(HoverKey::Save(index));
        let broken = entry.error.is_some();
        let summary = entry.summary();
        let title = entry.label.clone();
        let accessible = format!("{title}, {summary}");
        let action = Action::new({
            let ctx = ctx.clone();
            let sync = sync.clone();
            move || {
                if form_differs && selected != Some(index) {
                    ctx.run(|session| {
                        session.set_error(
                            "Fix the invalid character fields before switching savegames.",
                        );
                    });
                    return;
                }
                ctx.run(|session| {
                    session.open(index);
                });
                sync();
            }
        });
        rows.push(
            AccessibleButton::new(
                accessible,
                action,
                rect()
                    .horizontal()
                    .width(Size::fill())
                    .height(Size::px(46.))
                    .corner_radius(8.)
                    .background(if is_selected {
                        theme::SURFACE_ACTIVE
                    } else if is_hovered {
                        theme::SURFACE_HOVER
                    } else {
                        theme::SURFACE
                    })
                    .on_pointer_enter({
                        let mut hovered = hovered;
                        move |_| *hovered.write() = Some(HoverKey::Save(index))
                    })
                    .on_pointer_leave({
                        let mut hovered = hovered;
                        move |_| {
                            if *hovered.read() == Some(HoverKey::Save(index)) {
                                *hovered.write() = None;
                            }
                        }
                    })
                    // The bar is always present, transparent until the row
                    // is selected, so the row text does not shift by its
                    // width when the selection moves.
                    .child(rect().width(Size::px(3.)).height(Size::fill()).background(
                        if is_selected {
                            theme::ACCENT
                        } else {
                            Color::TRANSPARENT
                        },
                    ))
                    .child(
                        rect()
                            .vertical()
                            .width(Size::flex(1.0))
                            .spacing(2.)
                            .padding(Gaps::new_symmetric(7., 10.))
                            .child(
                                label()
                                    .text(title)
                                    .font_size(12.)
                                    .max_lines(1)
                                    .color(if broken { theme::DANGER } else { theme::TEXT }),
                            )
                            .child(label().text(summary).font_size(10.).color(theme::DIM)),
                    )
                    .into_element(),
            )
            .fill_width()
            .key(index)
            .into_element(),
        );
    }
    if rows.is_empty() {
        rows.push(
            rect()
                .padding(Gaps::new_all(12.))
                .child(
                    label()
                        .text(if entries.is_empty() {
                            "No savegames in this folder."
                        } else {
                            "No savegame matches this filter."
                        })
                        .font_size(11.)
                        .color(theme::DIM),
                )
                .into_element(),
        );
    }

    let preview_block = preview_slot(preview, selected.is_some());

    let title = character.as_ref().map_or_else(
        || "No savegame".to_owned(),
        |character| character.name.clone(),
    );
    let subtitle = character.as_ref().map_or_else(
        || "—".to_owned(),
        |character| {
            format!(
                "Level {} · {}",
                character.level,
                crate::session::format::duration(character.time_played)
            )
        },
    );

    rect()
        .vertical()
        .width(Size::percent(24.))
        .min_width(Size::px(230.))
        .max_width(Size::px(320.))
        .height(Size::fill())
        .content(Content::Flex)
        .padding(Gaps::new(14., 12., 14., 12.))
        .spacing(10.)
        .background(theme::BACKGROUND)
        .child(preview_block)
        .child(
            rect()
                .vertical()
                .spacing(1.)
                .child(
                    label()
                        .text(title)
                        .font_size(15.)
                        .font_weight(FontWeight::BOLD)
                        .max_lines(1)
                        .color(theme::TEXT),
                )
                .child(label().text(subtitle).font_size(10.).color(theme::DIM)),
        )
        .child(
            label()
                .text(if visible.len() == entries.len() {
                    format!("SAVEGAMES ({})", entries.len())
                } else {
                    format!("SAVEGAMES ({} of {})", visible.len(), entries.len())
                })
                .font_size(10.)
                .font_weight(FontWeight::BOLD)
                .color(theme::DIM),
        )
        .child(
            Input::new(query)
                .placeholder("Filter by name")
                .width(Size::fill()),
        )
        .child(sort_bar(ctx, sort, *sort.read()))
        .child(
            ScrollView::new()
                .show_scrollbar(true)
                .width(Size::fill())
                .height(Size::flex(1.0))
                .child(
                    rect()
                        .vertical()
                        .width(Size::fill())
                        .spacing(4.)
                        .children(rows),
                ),
        )
        .into_element()
}

/// Sort buttons for the savegame list; pressing the active one flips the order.
fn sort_bar(ctx: &Ctx, mut state: State<(SaveSort, bool)>, active: (SaveSort, bool)) -> Element {
    let mut buttons: Vec<Element> = Vec::new();
    for key in [SaveSort::Name, SaveSort::Level, SaveSort::Played] {
        let (current, descending) = active;
        let is_active = key == current;
        let arrow = if descending { " ↓" } else { " ↑" };
        let text = if is_active {
            format!("{}{arrow}", key.label())
        } else {
            key.label().to_owned()
        };
        let action = Action::new({
            let ctx = ctx.clone();
            move || {
                let (current, descending) = *state.read();
                let next = if current == key {
                    (key, !descending)
                } else {
                    (key, false)
                };
                *state.write() = next;
                let ctx = ctx.clone();
                ctx.run(move |session| session.persist_sort(next.0, next.1));
            }
        });
        let accessible = if is_active {
            format!(
                "Sort by {} ({})",
                key.label(),
                if descending {
                    "descending"
                } else {
                    "ascending"
                }
            )
        } else {
            format!("Sort by {}", key.label())
        };
        buttons.push(
            AccessibleButton::new(
                accessible,
                action,
                rect()
                    .horizontal()
                    .child(label().text(text).font_size(10.).color(if is_active {
                        theme::ACCENT
                    } else {
                        theme::DIM
                    }))
                    // Reserve the arrow column on inactive buttons;
                    // otherwise the buttons after the active one jump when
                    // the sort key changes.
                    .child(
                        rect()
                            .width(Size::px(if is_active { 0. } else { SORT_ARROW_WIDTH }))
                            .height(Size::px(12.)),
                    ),
            )
            .key(key.label())
            .into_element(),
        );
    }
    rect()
        .horizontal()
        .width(Size::fill())
        .spacing(10.)
        .content(Content::Flex)
        .child(
            label()
                .text("SORT")
                .font_size(10.)
                .font_weight(FontWeight::BOLD)
                .color(theme::DIM),
        )
        .children(buttons)
        .into_element()
}
