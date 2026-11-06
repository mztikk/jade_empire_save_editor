//! Tab bar.

use freya::prelude::*;

use crate::theme;
use crate::widgets::{AccessibleButton, Action};

use super::Tab;

pub(super) fn tab_bar(tab: State<Tab>, active: Tab) -> Element {
    let mut buttons: Vec<Element> = Vec::new();
    for candidate in Tab::ALL {
        let mut tab = tab;
        let is_active = candidate == active;
        let action = Action::new(move || *tab.write() = candidate);
        buttons.push(
            AccessibleButton::new(
                candidate.label(),
                action,
                rect()
                    .padding(Gaps::new_symmetric(theme::space::SM, theme::space::XL))
                    .corner_radius(theme::radius::MEDIUM)
                    .background(if is_active {
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
                            .color(if is_active {
                                theme::ACCENT
                            } else {
                                theme::MUTED
                            }),
                    ),
            )
            .role(AccessibilityRole::Tab)
            .selected(is_active)
            .key(candidate.index())
            .into_element(),
        );
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
