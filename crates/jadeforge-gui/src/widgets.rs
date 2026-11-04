//! Small reusable pieces of the `JadeForge` interface.

use freya::prelude::*;

use crate::theme;

/// A card with a title, an optional hint and a vertical body.
pub fn card(title: &'static str, hint: Option<String>, children: Vec<Element>) -> Element {
    let mut body = rect()
        .vertical()
        .width(Size::fill())
        .spacing(16.)
        .padding(Gaps::new_all(18.))
        .background(theme::PANEL)
        .corner_radius(14.)
        .child(
            rect()
                .horizontal()
                .width(Size::fill())
                .spacing(10.)
                .content(Content::Flex)
                .child(
                    label()
                        .text(title)
                        .font_size(15.)
                        .font_weight(FontWeight::BOLD)
                        .color(theme::TEXT),
                )
                .child(rect().width(Size::flex(1.0)))
                .child(hint.map_or_else(
                    || label().text("").font_size(11.),
                    |hint| label().text(hint).font_size(11.).color(theme::DIM),
                )),
        );
    for child in children {
        body = body.child(child);
    }
    body.into_element()
}

/// A row of form fields that wraps on narrow windows.
pub fn field_grid(children: Vec<Element>) -> Element {
    rect()
        .horizontal()
        .width(Size::fill())
        .spacing(14.)
        .content(Content::wrap_spacing(14.0))
        .children(children)
        .into_element()
}

/// A labelled input.
pub fn field(label_text: &'static str, input: impl IntoElement, width: f32) -> Element {
    rect()
        .vertical()
        .spacing(6.)
        .width(Size::px(width))
        .child(
            label()
                .text(label_text)
                .font_size(10.)
                .font_weight(FontWeight::BOLD)
                .color(theme::DIM),
        )
        .child(input)
        .into_element()
}

/// A read only statistic.
pub fn stat(label_text: &'static str, value: String) -> Element {
    rect()
        .vertical()
        .spacing(2.)
        .width(Size::px(120.))
        .child(label().text(label_text).font_size(10.).color(theme::DIM))
        .child(
            label()
                .text(value)
                .font_size(18.)
                .font_weight(FontWeight::BOLD)
                .color(theme::TEXT),
        )
        .into_element()
}

/// A coloured pill, used for categories and states.
pub fn badge(text: impl Into<String>, fill: Color, text_color: Color) -> Element {
    rect()
        .padding(Gaps::new_symmetric(2., 8.))
        .background(fill)
        .corner_radius(99.)
        .child(label().text(text.into()).font_size(10.).color(text_color))
        .into_element()
}

/// A one pixel divider.
pub fn divider() -> Element {
    rect()
        .width(Size::fill())
        .height(Size::px(1.))
        .background(theme::BORDER)
        .into_element()
}

/// Small muted explanatory text.
pub fn hint(text: impl Into<String>) -> Element {
    label()
        .text(text.into())
        .font_size(11.)
        .color(theme::DIM)
        .into_element()
}

/// A section label inside a card.
pub fn section_label(text: &'static str) -> Element {
    label()
        .text(text)
        .font_size(10.)
        .font_weight(FontWeight::BOLD)
        .color(theme::DIM)
        .into_element()
}

/// A row with a leading label and a trailing element.
pub fn label_row(label_text: impl Into<String>, trailing: impl IntoElement) -> Element {
    rect()
        .horizontal()
        .width(Size::fill())
        .spacing(10.)
        .content(Content::Flex)
        .child(
            label()
                .text(label_text.into())
                .font_size(12.)
                .color(theme::MUTED),
        )
        .child(rect().width(Size::flex(1.0)))
        .child(trailing)
        .into_element()
}
