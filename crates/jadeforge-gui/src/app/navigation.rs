//! Tab bar.

use freya::prelude::*;

use crate::theme;
use crate::widgets::{AccessibleButton, Action};

use super::Tab;

#[allow(clippy::too_many_lines)] // one declarative tab bar keeps the states together
pub(super) fn tab_bar(tab: State<Tab>, active: Tab, enabled: bool) -> Element {
    let mut buttons: Vec<Element> = Vec::new();
    for candidate in Tab::ALL {
        let mut tab = tab;
        let is_active = candidate == active;
        let action = Action::new(move || *tab.write() = candidate);
        let content = rect()
            .vertical()
            .spacing(theme::space::XS)
            .padding(Gaps::new_symmetric(theme::space::SM, theme::space::XL))
            .corner_radius(theme::radius::MEDIUM)
            .background(if is_active && enabled {
                theme::SURFACE_ACTIVE
            } else {
                Color::TRANSPARENT
            })
            .child(
                crate::widgets::a11y_text(candidate.label())
                    .font_size(theme::font::LABEL)
                    .max_lines(1)
                    // Keep the weight constant: bold text is wider,
                    // so toggling it moved the active tab.
                    .font_weight(FontWeight::BOLD)
                    .color(if !enabled {
                        theme::DIM
                    } else if is_active {
                        theme::ACCENT
                    } else {
                        theme::MUTED
                    }),
            )
            // The underline is the non-colour cue: it is part of every tab so
            // selecting one does not move the bar, but only the active tab
            // shows it.
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::px(theme::stroke::EMPHASIS))
                    .corner_radius(theme::radius::PILL)
                    .background(if is_active && enabled {
                        theme::ACCENT
                    } else {
                        Color::TRANSPARENT
                    }),
            )
            .into_element();
        if enabled {
            buttons.push(
                AccessibleButton::new(candidate.label(), action, content)
                    .role(AccessibilityRole::Tab)
                    .selected(is_active)
                    .hover_background(theme::SURFACE_HOVER)
                    // Arrow keys move between tabs while one of them has
                    // keyboard focus; Home and End jump to the ends.
                    .on_key({
                        let mut tab = tab;
                        move |event: Event<KeyboardEventData>| {
                            let target = match event.key {
                                Key::Named(NamedKey::ArrowLeft) => {
                                    Some((candidate.index() + Tab::ALL.len() - 1) % Tab::ALL.len())
                                }
                                Key::Named(NamedKey::ArrowRight) => {
                                    Some((candidate.index() + 1) % Tab::ALL.len())
                                }
                                Key::Named(NamedKey::Home) => Some(0),
                                Key::Named(NamedKey::End) => Some(Tab::ALL.len() - 1),
                                _ => None,
                            };
                            if let Some(target) = target
                                && let Some(next) = Tab::ALL.get(target)
                            {
                                *tab.write() = *next;
                                event.stop_propagation();
                            }
                        }
                    })
                    .key(candidate.index())
                    .into_element(),
            );
        } else {
            buttons.push(
                rect()
                    .key(candidate.index())
                    .child(content)
                    .a11y_role(AccessibilityRole::Tab)
                    .a11y_builder(accesskit::Node::set_disabled)
                    .into_element(),
            );
        }
    }

    rect()
        .vertical()
        .width(Size::fill())
        .spacing(theme::space::NONE)
        .child(
            rect()
                .horizontal()
                .width(Size::fill())
                .spacing(theme::space::XS)
                .content(Content::wrap_spacing(theme::space::XS))
                .padding(Gaps::new(
                    theme::space::LG,
                    theme::space::CARD,
                    theme::space::MD,
                    theme::space::CARD,
                ))
                .a11y_role(AccessibilityRole::TabList)
                .children(buttons)
                .into_element(),
        )
        .child(
            rect()
                .width(Size::fill())
                .height(Size::px(theme::layout::DIVIDER))
                .background(theme::BORDER)
                .into_element(),
        )
        .into_element()
}
