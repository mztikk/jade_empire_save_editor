//! Small reusable pieces of the `JadeForge` interface.

use std::cell::RefCell;
use std::rc::Rc;

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

/// Details area shown under a row when it is expanded.
pub fn details(description: Option<&str>) -> Option<Element> {
    description
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(|text| {
            rect()
                .width(Size::fill())
                .padding(Gaps::new(2., 10., 6., 10.))
                .child(
                    paragraph()
                        .width(Size::fill())
                        .max_lines(5)
                        .font_size(11.)
                        .color(theme::MUTED)
                        .span(text.to_owned()),
                )
                .into_element()
        })
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
    T: Copy + PartialEq + 'static,
{
    ExpandingName {
        id,
        name,
        subtitle,
        expanded,
        font_size,
    }
    .into_element()
}

#[derive(PartialEq)]
struct ExpandingName<T: 'static> {
    id: T,
    name: String,
    subtitle: String,
    expanded: State<Option<T>>,
    font_size: f32,
}

impl<T> Component for ExpandingName<T>
where
    T: Copy + PartialEq + 'static,
{
    fn render(&self) -> impl IntoElement {
        let a11y_id = use_a11y();
        let focus = use_focus(a11y_id);
        let focused = focus() == Focus::Keyboard;
        let mut expanded = self.expanded;
        let id = self.id;
        let accessible = format!("{}, {}", self.name, self.subtitle);
        rect()
            .vertical()
            .spacing(1.)
            .width(Size::fill())
            .padding(Gaps::new_symmetric(6., 6.))
            .corner_radius(4.)
            .border(if focused {
                Border::new().fill(theme::ACCENT).width(1.5)
            } else {
                Border::new().fill(Color::TRANSPARENT).width(1.5)
            })
            .a11y_id(a11y_id)
            .a11y_focusable(true)
            .a11y_role(AccessibilityRole::Button)
            .a11y_alt(accessible)
            .on_pointer_enter(|_| Cursor::set(CursorIcon::Pointer))
            .on_pointer_leave(|_| Cursor::set(CursorIcon::default()))
            .on_all_press(move |_| {
                let current = *expanded.read();
                *expanded.write() = if current == Some(id) { None } else { Some(id) };
            })
            .child(
                label()
                    .text(self.name.clone())
                    .font_size(self.font_size)
                    .max_lines(1)
                    .color(theme::TEXT),
            )
            .child(
                label()
                    .text(self.subtitle.clone())
                    .font_size(10.)
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
                .width(Size::px(340.))
                .spacing(12.)
                .content(Content::Flex)
                .padding(Gaps::new_symmetric(8., 0.))
                .child(label().text(name).font_size(12.).color(theme::MUTED))
                .child(rect().width(Size::flex(1.0)))
                .child(value)
                .into_element()
        })
        .collect();
    rect()
        .horizontal()
        .width(Size::fill())
        .content(Content::wrap_spacing(16.))
        .spacing(16.)
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
}

impl AccessibleButton {
    /// Creates a button with an explicit accessible name and visual content.
    pub fn new(label: impl Into<String>, action: Action, content: impl Into<Element>) -> Self {
        Self {
            label: label.into(),
            action,
            content: content.into(),
            fill_width: false,
        }
    }

    /// Makes the wrapper fill its parent, for full-width list rows.
    #[must_use]
    pub fn fill_width(mut self) -> Self {
        self.fill_width = true;
        self
    }
}

impl Component for AccessibleButton {
    fn render(&self) -> impl IntoElement {
        let a11y_id = use_a11y();
        let focus = use_focus(a11y_id);
        let focused = focus() == Focus::Keyboard;
        let action = self.action.clone();
        let mut button = rect()
            .a11y_id(a11y_id)
            .a11y_focusable(true)
            .a11y_role(AccessibilityRole::Button)
            .a11y_alt(self.label.clone())
            .on_all_press(move |_| action.call())
            .on_pointer_enter(|_| Cursor::set(CursorIcon::Pointer))
            .on_pointer_leave(|_| Cursor::set(CursorIcon::default()))
            .child(self.content.clone());
        if self.fill_width {
            button = button.width(Size::fill());
        }
        if focused {
            button = button.border(Border::new().fill(theme::TEXT).width(1.5));
        }
        button.into_element()
    }
}
