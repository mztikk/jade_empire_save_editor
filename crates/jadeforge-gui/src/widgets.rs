//! Small reusable pieces of the `JadeForge` interface.

use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

use freya::prelude::*;

use crate::theme;

/// A text label. Freya 0.5 fills a label's AccessKit value from its text when
/// the accessibility tree is built, and the AT-SPI adapter reads a `Label`
/// node's name from that value, so no extra accessibility builder is needed.
/// The helper keeps every visible string on the same element.
pub fn a11y_text(content: impl Into<String>) -> Label {
    label().text(content.into())
}

/// A card with a title, an optional hint and a vertical body.
pub fn card(title: &'static str, hint: Option<String>, children: Vec<Element>) -> Element {
    rect()
        .vertical()
        .width(Size::fill())
        .spacing(theme::space::XXL)
        .padding(Gaps::new_all(theme::space::CARD))
        .background(theme::PANEL)
        .corner_radius(theme::radius::PANEL)
        .child(
            rect()
                .horizontal()
                .width(Size::fill())
                .spacing(theme::space::LG)
                .content(Content::Flex)
                .child(
                    a11y_text(title)
                        .a11y_builder(|node| node.set_role(AccessibilityRole::Heading))
                        .font_size(theme::font::CARD_TITLE)
                        .font_weight(FontWeight::BOLD)
                        .color(theme::TEXT),
                )
                .child(rect().width(Size::flex(1.0)))
                .child(hint.map_or_else(
                    || a11y_text("").font_size(theme::font::CAPTION),
                    |hint| {
                        a11y_text(hint)
                            .font_size(theme::font::CAPTION)
                            .color(theme::DIM)
                    },
                )),
        )
        .children(children)
        .into_element()
}

/// A row of form fields that wraps on narrow windows.
pub fn field_grid(children: Vec<Element>) -> Element {
    rect()
        .horizontal()
        .width(Size::fill())
        .spacing(theme::space::FIELD)
        .content(Content::wrap_spacing(theme::space::FIELD))
        .children(children)
        .into_element()
}

/// A labelled input.
pub fn field(label_text: &'static str, input: impl IntoElement, width: f32) -> Element {
    rect()
        .vertical()
        .spacing(theme::space::SM)
        .width(Size::px(width))
        // Freya 0.5 still has no Input::label API. Naming the field group at
        // least gives a screen reader the caption before it reaches the input's
        // own value; the true label association is an upstream follow-up.
        .a11y_role(AccessibilityRole::Group)
        .a11y_alt(label_text)
        .child(
            a11y_text(label_text)
                .font_size(theme::font::MICRO)
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
        .spacing(theme::space::XXS)
        .width(Size::px(theme::layout::STAT_WIDTH))
        .child(
            a11y_text(label_text)
                .font_size(theme::font::MICRO)
                .color(theme::DIM),
        )
        .child(
            a11y_text(value)
                .font_size(theme::font::SUBHEADING)
                .font_weight(FontWeight::BOLD)
                .color(theme::TEXT),
        )
        .into_element()
}

/// A coloured pill, used for categories and states.
pub fn badge(text: impl Into<String>, fill: Color, text_color: Color) -> Element {
    rect()
        .padding(Gaps::new_symmetric(theme::space::XXS, theme::space::MD))
        .background(fill)
        .corner_radius(theme::radius::PILL)
        .child(
            a11y_text(text.into())
                .font_size(theme::font::MICRO)
                .color(text_color),
        )
        .into_element()
}

/// A one pixel divider.
pub fn divider() -> Element {
    rect()
        .width(Size::fill())
        .height(Size::px(theme::layout::DIVIDER))
        .background(theme::BORDER)
        .into_element()
}

/// Small muted explanatory text.
pub fn hint(text: impl Into<String>) -> Element {
    a11y_text(text.into())
        .font_size(theme::font::CAPTION)
        .color(theme::DIM)
        .into_element()
}

/// A section label inside a card.
pub fn section_label(text: &'static str) -> Element {
    a11y_text(text)
        .a11y_builder(|node| node.set_role(AccessibilityRole::Heading))
        .font_size(theme::font::MICRO)
        .font_weight(FontWeight::BOLD)
        .color(theme::DIM)
        .into_element()
}

/// A row with a leading label and a trailing element.
pub fn label_row(label_text: impl Into<String>, trailing: impl IntoElement) -> Element {
    rect()
        .horizontal()
        .width(Size::fill())
        .spacing(theme::space::LG)
        .content(Content::Flex)
        .child(
            a11y_text(label_text.into())
                .font_size(theme::font::LABEL)
                .color(theme::MUTED),
        )
        .child(rect().width(Size::flex(1.0)))
        .child(trailing)
        .into_element()
}

/// Details area shown under a row when it is expanded.
pub fn details(description: Option<&str>) -> Option<Element> {
    description
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(|text| {
            let value = text.to_owned();
            rect()
                .width(Size::fill())
                .padding(Gaps::new(
                    theme::space::XXS,
                    theme::space::LG,
                    theme::space::SM,
                    theme::space::LG,
                ))
                .child(
                    paragraph()
                        .width(Size::fill())
                        .max_lines(5)
                        .font_size(theme::font::CAPTION)
                        .color(theme::MUTED)
                        .a11y_builder(move |node| node.set_value(value))
                        .span(text.to_owned()),
                )
                .into_element()
        })
}

/// Formats a count with the correct singular or plural noun.
#[must_use]
pub fn plural(count: usize, singular: &str, plural: &str) -> String {
    format!("{count} {}", if count == 1 { singular } else { plural })
}

/// Name block that expands a description when pressed.
pub fn expanding_name<T>(
    id: T,
    name: String,
    subtitle: String,
    expanded: State<Option<T>>,
    font_size: f32,
) -> Element
where
    T: Copy + Hash + PartialEq + 'static,
{
    ExpandingName {
        id,
        name,
        subtitle,
        expanded,
        font_size,
        key: DiffKey::None,
    }
    .key(id)
    .into_element()
}

#[derive(PartialEq)]
struct ExpandingName<T: 'static> {
    id: T,
    name: String,
    subtitle: String,
    expanded: State<Option<T>>,
    font_size: f32,
    key: DiffKey,
}

impl<T> KeyExt for ExpandingName<T>
where
    T: Copy + Hash + PartialEq + 'static,
{
    fn key_mut(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl<T> Component for ExpandingName<T>
where
    T: Copy + Hash + PartialEq + 'static,
{
    fn render(&self) -> impl IntoElement {
        let a11y_id = use_a11y();
        let focus = use_focus(a11y_id);
        let focused = focus() == Focus::Keyboard;
        let mut expanded = self.expanded;
        let id = self.id;
        let is_expanded = *expanded.read() == Some(id);
        let accessible = format!(
            "{}, {}, {}",
            self.name,
            self.subtitle,
            if is_expanded { "expanded" } else { "collapsed" }
        );
        rect()
            .vertical()
            .spacing(theme::space::XXS)
            .width(Size::flex(1.0))
            .padding(Gaps::new_symmetric(theme::space::SM, theme::space::SM))
            .corner_radius(theme::radius::SMALL)
            .border(if focused {
                Border::new()
                    .fill(theme::ACCENT)
                    .width(theme::stroke::FOCUS)
            } else {
                Border::new()
                    .fill(Color::TRANSPARENT)
                    .width(theme::stroke::FOCUS)
            })
            .a11y_id(a11y_id)
            .a11y_focusable(true)
            .a11y_role(AccessibilityRole::Button)
            .a11y_alt(accessible)
            .a11y_builder(move |node| node.set_expanded(is_expanded))
            .cursor(CursorIcon::Pointer)
            .on_press(move |_| {
                a11y_id.request_focus();
                let current = *expanded.read();
                *expanded.write() = if current == Some(id) { None } else { Some(id) };
            })
            .child(
                a11y_text(self.name.clone())
                    .font_size(self.font_size)
                    .max_lines(1)
                    .color(theme::TEXT),
            )
            .child(
                a11y_text(self.subtitle.clone())
                    .font_size(theme::font::MICRO)
                    .color(theme::DIM),
            )
    }
}

/// Two-column key/value grid that collapses to one column in narrow windows.
#[must_use]
pub fn label_grid(rows: Vec<(String, Element)>) -> Element {
    let cells: Vec<Element> = rows
        .into_iter()
        .map(|(name, value)| {
            rect()
                .horizontal()
                .width(Size::px(theme::layout::LABEL_GRID_WIDTH))
                .spacing(theme::space::XL)
                .content(Content::Flex)
                .padding(Gaps::new_symmetric(theme::space::MD, theme::space::NONE))
                .child(
                    a11y_text(name)
                        .font_size(theme::font::LABEL)
                        .color(theme::MUTED),
                )
                .child(rect().width(Size::flex(1.0)))
                .child(value)
                .into_element()
        })
        .collect();
    rect()
        .horizontal()
        .width(Size::fill())
        .content(Content::wrap_spacing(theme::space::XXL))
        .spacing(theme::space::XXL)
        .children(cells)
        .into_element()
}

/// A shared, cloneable press action.
///
/// Event handlers compare unequal in Freya, which would re-render every list
/// item on every frame. A pointer-identity comparison keeps the state stable.
#[derive(Clone)]
pub struct Action(Rc<RefCell<dyn FnMut()>>);

impl Action {
    /// Wraps a callback.
    pub fn new(action: impl FnMut() + 'static) -> Self {
        Self(Rc::new(RefCell::new(action)))
    }

    fn call(&self) {
        (self.0.borrow_mut())();
    }
}

impl PartialEq for Action {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// A button whose whole surface is a single accessible, focusable node.
///
/// Freya's built-in `Button` does not expose `a11y_alt`, and a label nested in
/// a layout `rect` is not lifted to the button's accessibility node. Tabs,
/// sort controls and savegame rows therefore use this wrapper: it keeps the
/// existing visual content but owns the accessible name and the keyboard/pointer
/// activation.
#[derive(PartialEq)]
pub struct AccessibleButton {
    label: String,
    action: Action,
    content: Element,
    fill_width: bool,
    role: AccessibilityRole,
    selected: bool,
    focusable: bool,
    key: DiffKey,
}

impl AccessibleButton {
    /// Creates a button with an explicit accessible name and visual content.
    pub fn new(label: impl Into<String>, action: Action, content: impl Into<Element>) -> Self {
        Self {
            label: label.into(),
            action,
            content: content.into(),
            fill_width: false,
            role: AccessibilityRole::Button,
            selected: false,
            focusable: true,
            key: DiffKey::None,
        }
    }

    /// Makes the wrapper fill its parent, for full-width list rows.
    #[must_use]
    pub fn fill_width(mut self) -> Self {
        self.fill_width = true;
        self
    }

    /// Uses a more specific accessibility role, for example `Tab` or
    /// `ListBoxOption`.
    #[must_use]
    pub fn role(mut self, role: AccessibilityRole) -> Self {
        self.role = role;
        self
    }

    /// Exposes the selected state of a tab or list option.
    #[must_use]
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Makes the node reachable by keyboard focus.
    #[must_use]
    pub fn focusable(mut self, focusable: bool) -> Self {
        self.focusable = focusable;
        self
    }
}

impl KeyExt for AccessibleButton {
    fn key_mut(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl Component for AccessibleButton {
    fn render(&self) -> impl IntoElement {
        let a11y_id = use_a11y();
        let focus = use_focus(a11y_id);
        let focused = focus() == Focus::Keyboard;
        let action = self.action.clone();
        let selected = self.selected;
        let button = rect()
            .a11y_id(a11y_id)
            .a11y_focusable(self.focusable)
            .a11y_role(self.role)
            .a11y_alt(self.label.clone())
            .a11y_builder(move |node| node.set_selected(selected))
            .cursor(CursorIcon::Pointer)
            .on_press(move |_| {
                a11y_id.request_focus();
                action.call();
            })
            // Reserve the focus ring even when unfocused, so tabbing through
            // the shell never shifts tabs, sort controls or savegame rows.
            .border(
                Border::new()
                    .fill(if focused {
                        theme::TEXT
                    } else {
                        Color::TRANSPARENT
                    })
                    .width(theme::stroke::FOCUS),
            )
            .child(self.content.clone())
            .maybe(self.fill_width, |el| el.width(Size::fill()));
        button.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::plural;

    #[test]
    fn plural_counts_use_the_singular_only_for_one() {
        assert_eq!(plural(0, "effect", "effects"), "0 effects");
        assert_eq!(plural(1, "effect", "effects"), "1 effect");
        assert_eq!(plural(2, "stack", "stacks"), "2 stacks");
    }
}
