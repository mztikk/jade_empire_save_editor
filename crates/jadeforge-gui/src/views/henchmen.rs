//! The companions tab.

use freya::prelude::*;

use crate::app::{Ctx, RowSession};
use crate::i18n::{fill, keys, t};
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
        rows.push(
            HenchRowView {
                session: RowSession(ctx.session.clone()),
                revision: ctx.revision,
                henchman: henchman.clone(),
                hovered,
                key: DiffKey::None,
            }
            .key(henchman.struct_index)
            .into_element(),
        );
    }
    if rows.is_empty() {
        rows.push(
            match ctx.session.borrow().section_error(Section::Companions) {
                Some(error) => widgets::hint(super::section_unreadable(
                    Section::Companions,
                    &error.message,
                )),
                None => widgets::hint(t(keys::COMPANIONS_EMPTY_HINT)),
            },
        );
    }

    widgets::card(
        t(keys::NAV_COMPANIONS),
        Some(widgets::plural(
            henchmen.len(),
            keys::COUNT_COMPANION_KNOWN_ONE,
            keys::COUNT_COMPANION_KNOWN_OTHER,
        )),
        vec![
            rect()
                .vertical()
                .width(Size::fill())
                .spacing(theme::space::xs())
                .children(rows)
                .into_element(),
            widgets::divider(),
            widgets::hint(t(keys::COMPANIONS_LEVEL_HINT)),
        ],
    )
}

/// One companion row, with a local draft for direct level editing.
#[derive(PartialEq)]
struct HenchRowView {
    session: RowSession,
    revision: State<u32>,
    henchman: HenchRow,
    hovered: State<Option<crate::app::HoverKey>>,
    key: DiffKey,
}

impl KeyExt for HenchRowView {
    fn key_mut(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl Component for HenchRowView {
    fn render_key(&self) -> DiffKey {
        self.key.clone()
    }

    #[allow(clippy::too_many_lines)] // one declarative row keeps the actions together
    fn render(&self) -> impl IntoElement {
        let henchman = &self.henchman;
        let mut hovered = self.hovered;
        let mut draft = use_state(|| henchman.level.to_string());
        let mut synced_level = use_state(|| henchman.level);
        if *synced_level.peek() != henchman.level {
            synced_level.set(henchman.level);
            draft.set(henchman.level.to_string());
        }
        let index = henchman.struct_index;
        let level = henchman.level;
        let available = henchman.available;
        let name = henchman.name.clone();
        let key = crate::app::HoverKey::Companion(index);
        let is_hovered = *hovered.read() == Some(key);

        let lower = {
            let session = self.session.clone();
            let revision = self.revision;
            Action::new(move || {
                session.run(revision, |session| {
                    session.set_henchman_level(index, level.saturating_sub(1).max(1));
                });
            })
        };
        let raise = {
            let session = self.session.clone();
            let revision = self.revision;
            Action::new(move || {
                session.run(revision, |session| {
                    session.set_henchman_level(index, level.saturating_add(1).min(60));
                });
            })
        };
        let toggle = {
            let session = self.session.clone();
            let revision = self.revision;
            Action::new(move || {
                session.run(revision, |session| session.toggle_henchman_available(index));
            })
        };
        let set_level = {
            let session = self.session.clone();
            let revision = self.revision;
            let name = name.clone();
            let mut draft = draft;
            move |text: String| {
                let Ok(value) = text.trim().parse::<u8>() else {
                    session.run(revision, |session| {
                        session.set_error(&fill(keys::COMPANIONS_LEVEL_ERROR, &[("name", &name)]));
                    });
                    return;
                };
                if !(1..=60).contains(&value) {
                    session.run(revision, |session| {
                        session.set_error(&fill(keys::COMPANIONS_LEVEL_ERROR, &[("name", &name)]));
                    });
                    return;
                }
                session.run(revision, |session| session.set_henchman_level(index, value));
                draft.set(value.to_string());
            }
        };

        rect()
            .horizontal()
            .width(Size::fill())
            .key(index)
            .a11y_role(AccessibilityRole::Group)
            .a11y_alt(fill(
                keys::COMPANIONS_ACCESSIBLE,
                &[("name", &henchman.name)],
            ))
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
                    t(keys::COMPANIONS_AVAILABLE),
                    theme::surface_active(),
                    theme::jade(),
                )
            } else {
                // Hidden is a resting state, not a failure: a calmer colour than
                // the danger red that made an unavailable companion look broken.
                widgets::badge(t(keys::COMPANIONS_HIDDEN), theme::surface(), theme::muted())
            })
            .child(widgets::row_action(
                fill(keys::COMPANIONS_DECREASE, &[("name", &name)]),
                "−",
                lower,
            ))
            .child(
                // Direct numeric editing: level 60 from level 1 is one edit.
                Input::new(draft)
                    .width(Size::px(theme::layout::stepper_width()))
                    .text_align(TextAlign::Center)
                    .compact()
                    .on_submit(set_level),
            )
            .child(widgets::row_action(
                fill(keys::COMPANIONS_INCREASE, &[("name", &name)]),
                "+",
                raise,
            ))
            .child(widgets::row_action(
                if available {
                    fill(keys::COMPANIONS_HIDE_ACTION, &[("name", &name)])
                } else {
                    fill(keys::COMPANIONS_MAKE_AVAILABLE_ACTION, &[("name", &name)])
                },
                if available {
                    t(keys::COMPANIONS_HIDE)
                } else {
                    t(keys::COMPANIONS_MAKE_AVAILABLE)
                },
                toggle,
            ))
            .into_element()
    }
}
