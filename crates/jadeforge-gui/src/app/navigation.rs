//! Tab bar.

use freya::prelude::*;

use crate::theme;
use crate::widgets::{AccessibleButton, Action};

use super::Tab;

#[allow(clippy::too_many_lines)] // one declarative tab bar keeps the states together
pub(super) fn tab_bar(
    tab: State<Tab>,
    active: Tab,
    enabled: bool,
    scanning: bool,
    mut show_about: State<bool>,
) -> Element {
    let mut buttons: Vec<Element> = Vec::new();
    let about_open = *show_about.read();
    for candidate in Tab::ALL {
        let mut tab = tab;
        let is_active = candidate == active && !about_open;
        let action = Action::new(move || {
            *tab.write() = candidate;
            show_about.set(false);
        });
        let content = rect()
            .vertical()
            .padding(Gaps::new_symmetric(theme::space::SM, theme::space::XL))
            .corner_radius(theme::radius::MEDIUM)
            .background(if is_active && enabled {
                theme::SURFACE_ACTIVE
            } else {
                Color::TRANSPARENT
            })
            // The underline is the non-colour cue. It is a bottom border whose
            // width is reserved on every tab, so selecting a tab does not move
            // the bar, and it does not give the tab a fill-width child (which
            // used to make every tab claim the whole row and wrap the bar into
            // a column).
            .border(
                Border::new()
                    .width(BorderWidth {
                        top: 0.0,
                        right: 0.0,
                        bottom: theme::stroke::EMPHASIS,
                        left: 0.0,
                    })
                    .alignment(BorderAlignment::Inner)
                    .fill(if is_active && enabled {
                        theme::ACCENT
                    } else {
                        Color::TRANSPARENT
                    }),
            )
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
                        let mut show_about = show_about;
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
                                show_about.set(false);
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
                .spacing(theme::space::MD)
                .content(Content::Flex)
                .cross_align(Alignment::Center)
                .padding(Gaps::new(
                    theme::space::LG,
                    theme::space::CARD,
                    theme::space::MD,
                    theme::space::CARD,
                ))
                .child(
                    rect()
                        .horizontal()
                        .width(Size::flex(1.0))
                        .spacing(theme::space::XS)
                        .content(Content::wrap_spacing(theme::space::XS))
                        .a11y_role(AccessibilityRole::TabList)
                        .children(buttons),
                )
                .child(busy_slot(scanning))
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

/// Reserved scan-status slot at the end of the tab bar.
///
/// It is always rendered, empty when idle, so starting or cancelling a manual
/// scan never moves the tab row. The header already has the verbose status;
/// this is the persistent "work is happening" cue next to the content.
fn busy_slot(scanning: bool) -> Element {
    rect()
        .width(Size::px(theme::layout::BUSY_SLOT))
        .height(Size::px(theme::font::LINE_HEIGHT))
        .main_align(Alignment::Center)
        .cross_align(Alignment::End)
        .child(
            crate::widgets::a11y_text(if scanning { "SCANNING…" } else { "" })
                .a11y_builder(move |node| {
                    if scanning {
                        node.set_role(AccessibilityRole::Status);
                        node.set_live(accesskit::Live::Polite);
                    }
                })
                .font_size(theme::font::MICRO)
                .font_weight(FontWeight::BOLD)
                .max_lines(1)
                .color(theme::WARNING),
        )
        .into_element()
}
