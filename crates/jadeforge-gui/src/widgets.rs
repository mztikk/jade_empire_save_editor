//! Small reusable pieces of the `JadeForge` interface.

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;

use freya::prelude::*;

use crate::i18n::{Key, fill, keys, t};
use crate::session::{CharacterField, FieldError};
use crate::theme;

/// A text label. Freya 0.5 fills a label's AccessKit value from its text when
/// the accessibility tree is built, and the AT-SPI adapter reads a `Label`
/// node's name from that value, so no extra accessibility builder is needed.
/// The helper keeps every visible string on the same element.
pub fn a11y_text(content: impl Into<String>) -> Label {
    label().text(content.into())
}

/// A card with a title, an optional hint and a vertical body.
pub fn card(title: impl Into<String>, hint: Option<String>, children: Vec<Element>) -> Element {
    let title = title.into();
    rect()
        .vertical()
        .width(Size::fill())
        .spacing(theme::space::xxl())
        .padding(Gaps::new_all(theme::space::card()))
        .background(theme::panel())
        .corner_radius(theme::radius::panel())
        .child(
            rect()
                .horizontal()
                .width(Size::fill())
                .spacing(theme::space::lg())
                .content(Content::Flex)
                .child(
                    a11y_text(title)
                        .a11y_builder(|node| node.set_role(AccessibilityRole::Heading))
                        .font_size(theme::font::card_title())
                        .font_weight(FontWeight::BOLD)
                        .color(theme::text()),
                )
                .child(rect().width(Size::flex(1.0)))
                .child(hint.map_or_else(
                    || a11y_text("").font_size(theme::font::caption()),
                    |hint| {
                        a11y_text(hint)
                            .font_size(theme::font::caption())
                            .color(theme::dim())
                    },
                )),
        )
        .children(children)
        .into_element()
}

/// One row of form fields that share the width evenly.
///
/// Fields grow to fill the row and never shrink below
/// [`theme::layout::field_min_width()`]. Callers group the fields they want on a
/// row, so a wide window shows a deliberate 3+3 identity grid instead of
/// whichever fields happen to fit; a row that no longer fits keeps its fields
/// inside the horizontal scroll area rather than moving one field to its own
/// row.
pub fn field_grid(children: Vec<Element>) -> Element {
    rect()
        .horizontal()
        .width(Size::fill())
        .spacing(theme::space::field())
        .content(Content::Flex)
        .children(children)
        .into_element()
}

/// A labelled input that shares its row with the other fields.
///
/// The line under the input always shows the field's allowed range and turns
/// into the validation message when the value cannot be applied. Reserving the
/// line keeps the cards still while a field is being fixed, and the wrapper
/// border marks the offending input.
pub fn field_checked(
    field: CharacterField,
    input: impl IntoElement,
    error: Option<&FieldError>,
) -> Element {
    let message = error.map_or_else(|| field.range_text(), |error| error.message.clone());
    let caption = field.caption().to_uppercase();
    let invalid = error.is_some();
    // Freya 0.5 still has no Input::label API. Naming the field group at
    // least gives a screen reader the caption before it reaches the input's
    // own value; the true label association is an upstream follow-up.
    rect()
        .vertical()
        .spacing(theme::space::xs())
        .width(Size::flex(1.0))
        .min_width(Size::px(theme::layout::field_min_width()))
        .a11y_role(AccessibilityRole::Group)
        .a11y_alt(caption.clone())
        .child(
            a11y_text(caption)
                .font_size(theme::font::micro())
                .font_weight(FontWeight::BOLD)
                .color(if invalid {
                    theme::danger()
                } else {
                    theme::dim()
                }),
        )
        .child(
            rect()
                .width(Size::fill())
                .border(
                    Border::new()
                        .fill(if invalid {
                            theme::danger()
                        } else {
                            Color::TRANSPARENT
                        })
                        .width(theme::stroke::focus()),
                )
                .child(input),
        )
        .child(
            rect()
                .width(Size::fill())
                .height(Size::px(theme::font::line_height()))
                .child(with_tooltip(
                    message.clone(),
                    a11y_text(message)
                        .font_size(theme::font::micro())
                        .max_lines(1)
                        .text_overflow(TextOverflow::Ellipsis)
                        .color(if invalid {
                            theme::danger()
                        } else {
                            theme::dim()
                        }),
                )),
        )
        .into_element()
}

/// The validation error for one field, when it has one.
#[must_use]
pub fn field_error(errors: &[FieldError], field: CharacterField) -> Option<&FieldError> {
    errors.iter().find(|error| error.field == field)
}

/// A read only statistic.
pub fn stat(label_text: &'static str, value: String) -> Element {
    rect()
        .vertical()
        .spacing(theme::space::xxs())
        .width(Size::flex(1.0))
        .min_width(Size::px(theme::layout::stat_min_width()))
        .child(
            a11y_text(label_text)
                .font_size(theme::font::micro())
                .color(theme::dim()),
        )
        .child(
            a11y_text(value)
                .font_size(theme::font::subheading())
                .font_weight(FontWeight::BOLD)
                .color(theme::text()),
        )
        .into_element()
}

/// A deliberately two-column statistic grid.
///
/// The four hero values stay a 2x2 block at every supported width instead of
/// wrapping 3+1, which used to leave the last value alone under a wide row.
pub fn stat_grid(children: Vec<Element>) -> Element {
    let mut remaining = children.into_iter();
    let mut rows: Vec<Element> = Vec::new();
    while remaining.len() > 0 {
        let pair: Vec<Element> = remaining.by_ref().take(2).collect();
        rows.push(
            rect()
                .horizontal()
                .width(Size::fill())
                .spacing(theme::space::grid())
                .content(Content::Flex)
                .children(pair)
                .into_element(),
        );
    }
    rect()
        .vertical()
        .width(Size::fill())
        .spacing(theme::space::md())
        .children(rows)
        .into_element()
}

/// A coloured pill, used for categories and states.
pub fn badge(text: impl Into<String>, fill: Color, text_color: Color) -> Element {
    rect()
        .padding(Gaps::new_symmetric(theme::space::xxs(), theme::space::md()))
        .background(fill)
        .corner_radius(theme::radius::pill())
        .child(
            a11y_text(text.into())
                .font_size(theme::font::micro())
                .color(text_color),
        )
        .into_element()
}

/// A one pixel divider.
pub fn divider() -> Element {
    rect()
        .width(Size::fill())
        .height(Size::px(theme::layout::divider()))
        .background(theme::border())
        .into_element()
}

/// Small muted explanatory text.
pub fn hint(text: impl Into<String>) -> Element {
    a11y_text(text.into())
        .font_size(theme::font::caption())
        .color(theme::dim())
        .into_element()
}

/// A section label inside a card.
pub fn section_label(text: impl Into<String>) -> Element {
    a11y_text(text.into())
        .a11y_builder(|node| node.set_role(AccessibilityRole::Heading))
        .font_size(theme::font::micro())
        .font_weight(FontWeight::BOLD)
        .color(theme::dim())
        .into_element()
}

/// A row with a leading label and a trailing element.
pub fn label_row(label_text: impl Into<String>, trailing: impl IntoElement) -> Element {
    rect()
        .horizontal()
        .width(Size::fill())
        .spacing(theme::space::lg())
        .content(Content::Flex)
        .child(
            a11y_text(label_text.into())
                .font_size(theme::font::label())
                .color(theme::muted()),
        )
        .child(rect().width(Size::flex(1.0)))
        .child(trailing)
        .into_element()
}

/// Details area shown under a row when it is expanded.
///
/// The description is shown in full: an expanded row is the user asking for
/// the text, so it must not stop mid-sentence at five lines. The paragraph is
/// capped at a readable measure on very wide windows.
pub fn details(description: Option<&str>) -> Option<Rect> {
    description
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(|text| {
            let value = text.to_owned();
            rect()
                .width(Size::fill())
                .padding(Gaps::new(
                    theme::space::xxs(),
                    theme::space::lg(),
                    theme::space::sm(),
                    theme::space::lg(),
                ))
                .child(
                    paragraph()
                        .width(Size::fill())
                        .max_width(Size::px(theme::layout::text_measure()))
                        .font_size(theme::font::caption())
                        .color(theme::muted())
                        .a11y_builder(move |node| node.set_value(value))
                        .span(text.to_owned()),
                )
        })
}

/// Wraps a name in a tooltip so a clipped label can still be read in full.
pub fn with_tooltip(text: String, content: impl IntoElement) -> Element {
    TooltipContainer::new(Tooltip::new_text(text))
        .child(content)
        .into_element()
}

/// Formats a count with the correct singular or plural noun.
///
/// Delegates to the shared session helper so the shell and the session
/// messages can never disagree about "1 gem" versus "1 gems".
#[must_use]
pub fn plural(count: usize, singular: Key, plural: Key) -> String {
    crate::i18n::plural(count, singular, plural)
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
        let accessible = fill(
            keys::WIDGET_DETAILS,
            &[
                ("name", &self.name),
                ("subtitle", &self.subtitle),
                (
                    "state",
                    if is_expanded {
                        t(keys::WIDGET_EXPANDED)
                    } else {
                        t(keys::WIDGET_COLLAPSED)
                    },
                ),
            ],
        );
        rect()
            .vertical()
            .spacing(theme::space::xxs())
            .width(Size::flex(1.0))
            .padding(Gaps::new_symmetric(theme::space::sm(), theme::space::sm()))
            .corner_radius(theme::radius::small())
            .border(if focused {
                Border::new()
                    .fill(theme::accent())
                    .width(theme::stroke::focus())
            } else {
                Border::new()
                    .fill(Color::TRANSPARENT)
                    .width(theme::stroke::focus())
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
                rect()
                    .horizontal()
                    .width(Size::fill())
                    .spacing(theme::space::sm())
                    .content(Content::Flex)
                    .cross_align(Alignment::Center)
                    .child(
                        rect()
                            .width(Size::flex(1.0))
                            .child(with_tooltip(self.name.clone(), {
                                a11y_text(self.name.clone())
                                    .font_size(self.font_size)
                                    .max_lines(1)
                                    .text_overflow(TextOverflow::Ellipsis)
                                    .color(theme::text())
                            })),
                    )
                    // The chevron is the visible disclosure affordance; its
                    // slot is reserved on every row so the name never jumps.
                    .child(
                        rect()
                            .width(Size::px(theme::layout::chevron_width()))
                            .center()
                            .child(
                                a11y_text(if is_expanded { "▾" } else { "▸" })
                                    .font_size(theme::font::caption())
                                    .color(theme::accent()),
                            ),
                    ),
            )
            .child(
                a11y_text(self.subtitle.clone())
                    .font_size(theme::font::micro())
                    .color(theme::dim()),
            )
    }
}

/// Full-width label/value rows for the overview glance section.
///
/// A fixed-cell grid left an entire half-row empty once the supported 900 px
/// window forced one column, so the values now use the whole card at every
/// width instead of a percentage of it.
#[must_use]
pub fn definition_list(rows: Vec<(String, Element)>) -> Element {
    let mut children: Vec<Element> = Vec::new();
    for (index, (name, value)) in rows.into_iter().enumerate() {
        if index > 0 {
            children.push(divider());
        }
        children.push(label_row(name, value));
    }
    rect()
        .vertical()
        .width(Size::fill())
        .spacing(theme::space::md())
        .children(children)
        .into_element()
}

/// A filter input with a clear button.
///
/// Every picker filter gets one, so a filter never has to be cleared by
/// selecting the text and deleting it.
pub fn filter_input(
    query: State<String>,
    placeholder: Key,
    clear_name: Key,
    a11y_id: AccessibilityId,
) -> Element {
    let mut clear_query = query;
    let clear = Action::new(move || {
        clear_query.write().clear();
    });
    let empty = query.read().is_empty();
    rect()
        .horizontal()
        .width(Size::fill())
        .spacing(theme::space::md())
        .content(Content::Flex)
        .cross_align(Alignment::Center)
        .child(
            Input::new(query)
                .placeholder(t(placeholder))
                .a11y_id(a11y_id)
                .width(Size::flex(1.0)),
        )
        .child(
            AccessibleButton::new(
                t(clear_name),
                clear,
                rect()
                    .padding(Gaps::new_symmetric(theme::space::xs(), theme::space::md()))
                    .child(
                        a11y_text(t(keys::WIDGET_CLEAR))
                            .font_size(theme::font::micro())
                            .color(if empty { theme::dim() } else { theme::muted() }),
                    ),
            )
            .focusable(!empty),
        )
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

    pub(crate) fn call(&self) {
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
    hover_background: Option<Color>,
    key_action: Option<EventHandler<Event<KeyboardEventData>>>,
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
            hover_background: None,
            key_action: None,
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

    /// Paints a hover background, for controls that would otherwise be flat.
    #[must_use]
    pub fn hover_background(mut self, color: Color) -> Self {
        self.hover_background = Some(color);
        self
    }

    /// Handles keys while the button has keyboard focus.
    #[must_use]
    pub fn on_key(mut self, handler: impl Into<EventHandler<Event<KeyboardEventData>>>) -> Self {
        self.key_action = Some(handler.into());
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
        let mut hovered = use_state(|| false);
        let hover_background = self.hover_background;
        let key_action = self.key_action.clone();
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
                        theme::text()
                    } else {
                        Color::TRANSPARENT
                    })
                    .width(theme::stroke::focus()),
            )
            .child(self.content.clone())
            .maybe(self.fill_width, |el| el.width(Size::fill()))
            .maybe(hover_background.is_some(), |el| {
                el.corner_radius(theme::radius::small()).background(
                    hover_background
                        .filter(|_| *hovered.read())
                        .unwrap_or(Color::TRANSPARENT),
                )
            })
            .maybe(hover_background.is_some(), |el| {
                el.on_pointer_enter(move |_| hovered.set(true))
                    .on_pointer_leave(move |_| hovered.set(false))
            })
            .maybe(key_action.is_some(), |el| {
                el.on_key_down(move |event| {
                    if let Some(ref handler) = key_action {
                        handler.call(event);
                    }
                })
            });
        button.into_element()
    }
}

/// A compact row action with a contextual accessible name.
///
/// The visible text stays short (`Remove`, `+`), while assistive technology
/// hears what is being removed or increased. The hover background matches the
/// flat buttons it replaces.
pub fn row_action(
    accessible: impl Into<String>,
    text: impl Into<String>,
    action: Action,
) -> AccessibleButton {
    AccessibleButton::new(
        accessible,
        action,
        rect()
            // 11 px micro type plus 6 px padding on both sides keeps the
            // target at least 24 px tall; the explicit minimum keeps a single
            // glyph wide enough to hit.
            .center()
            .min_width(Size::px(theme::layout::target_min()))
            .min_height(Size::px(theme::layout::target_min()))
            .padding(Gaps::new_symmetric(theme::space::sm(), theme::space::sm()))
            .child(
                a11y_text(text.into())
                    .font_size(theme::font::micro())
                    .color(theme::muted()),
            ),
    )
    .hover_background(theme::surface_hover())
}

/// A filled primary action for a page or dialog.
pub fn filled_action(
    accessible: impl Into<String>,
    text: impl Into<String>,
    action: Action,
) -> AccessibleButton {
    AccessibleButton::new(
        accessible,
        action,
        rect()
            .padding(Gaps::new_symmetric(theme::space::md(), theme::space::xl()))
            .background(theme::accent())
            .corner_radius(theme::radius::control())
            .child(
                a11y_text(text)
                    .font_size(theme::font::caption())
                    .font_weight(FontWeight::BOLD)
                    .color(theme::on_accent()),
            ),
    )
    .hover_background(theme::accent_hover())
}

/// A compact outlined action for a toolbar or page header.
///
/// `AccessibleButton` has no visual chrome of its own, so a transparent action
/// with only a hover state reads like plain text next to status badges. The
/// border and control padding make the action recognizable as a button while
/// the explicit accessible name stays on the button node.
pub fn outline_action(
    accessible: impl Into<String>,
    text: impl Into<String>,
    action: Action,
) -> AccessibleButton {
    AccessibleButton::new(
        accessible,
        action,
        rect()
            .padding(Gaps::new_symmetric(theme::space::md(), theme::space::xl()))
            .border(
                Border::new()
                    .fill(theme::control_border())
                    .width(theme::stroke::hairline()),
            )
            .corner_radius(theme::radius::control())
            .child(
                a11y_text(text)
                    .font_size(theme::font::caption())
                    .font_weight(FontWeight::BOLD)
                    .color(theme::text()),
            ),
    )
    .hover_background(theme::surface_hover())
}

#[cfg(test)]
mod tests {
    use super::{Action, HashMap, PickerRow, picker_details_height, plural};
    use crate::i18n::keys;

    fn picker_row(selected: bool, description: Option<&str>) -> PickerRow {
        PickerRow {
            key: 7,
            title: "Item".to_owned(),
            subtitle: "Weapons · #7".to_owned(),
            accessible: "Item item".to_owned(),
            action_label: "Add Item".to_owned(),
            action_text: "Add".to_owned(),
            action: Action::new(|| {}),
            select: None,
            selected,
            description: description.map(ToOwned::to_owned),
        }
    }

    #[test]
    fn only_selected_picker_rows_add_description_height() {
        let collapsed = picker_row(false, Some("A description."));
        assert_eq!(picker_details_height(&collapsed, &HashMap::new()), 0.0);
        assert_eq!(
            picker_details_height(&picker_row(true, None), &HashMap::new()),
            0.0
        );
        assert_eq!(
            picker_details_height(&picker_row(true, Some("   ")), &HashMap::new()),
            0.0
        );

        let selected = picker_row(true, Some("A description."));
        assert!(
            picker_details_height(&selected, &HashMap::new()) > 0.0,
            "a selected description has no estimated height"
        );
        let mut measured = HashMap::new();
        measured.insert(7, 42.0);
        assert!(
            (picker_details_height(&selected, &measured) - 42.0).abs() < f32::EPSILON,
            "the measured description height was not used"
        );
    }

    #[test]
    fn a_longer_description_estimates_more_lines() {
        let short = picker_row(true, Some("A description."));
        let long = picker_row(true, Some(&"A longer description. ".repeat(8)));
        assert!(
            picker_details_height(&long, &HashMap::new())
                > picker_details_height(&short, &HashMap::new()),
            "the estimate does not grow with the text"
        );
    }

    #[test]
    fn plural_counts_use_the_singular_only_for_one() {
        assert_eq!(
            plural(0, keys::COUNT_EFFECT_ONE, keys::COUNT_EFFECT_OTHER),
            "0 effects"
        );
        assert_eq!(
            plural(1, keys::COUNT_EFFECT_ONE, keys::COUNT_EFFECT_OTHER),
            "1 effect"
        );
        assert_eq!(
            plural(2, keys::COUNT_STACK_ONE, keys::COUNT_STACK_OTHER),
            "2 stacks"
        );
    }
}

/// One row of a virtualized picker.
///
/// The row model is deliberately UI independent of the picker that produced it:
/// the item, style and gem lists all render the same compact two-line row, so a
/// very large database only builds the rows inside the viewport. A row with a
/// description grows in place when selected; its measured height is what keeps
/// the dynamic virtual list accurate.
#[derive(Clone, PartialEq)]
pub struct PickerRow {
    /// Stable key, the game id of the item or style.
    pub key: u16,
    /// Primary line.
    pub title: String,
    /// Secondary line.
    pub subtitle: String,
    /// Accessible name of the whole row.
    pub accessible: String,
    /// Accessible name of the add/learn action.
    pub action_label: String,
    /// Visible action text.
    pub action_text: String,
    /// Adds or learns the row.
    pub action: Action,
    /// Toggles the inline description, when the row has one.
    pub select: Option<State<Option<u16>>>,
    /// Whether the inline description is currently open for this row.
    pub selected: bool,
    /// Game description revealed under the title row when selected.
    pub description: Option<String>,
}

/// The data a virtualized picker builds its visible rows from.
#[derive(Clone, PartialEq)]
struct PickerData {
    /// Every filtered row, in display order.
    rows: Rc<Vec<PickerRow>>,
    /// Measured heights of the expanded descriptions, keyed by row id.
    measured: State<HashMap<u16, f32>>,
}

/// Number of picker rows a virtualized list shows before it scrolls.
///
/// The list is inside the page's own scroll view; seven rows keep the picker
/// reachable while the wheel still scrolls the page when the pointer is
/// outside the list.
const PICKER_VISIBLE_ROWS: usize = 7;

/// Conservative wrap width used before a description has been measured.
///
/// The first render of an expanded row has no measured height yet. Estimating
/// the line count at the narrowest useful picker width overestimates the height,
/// so the text is never clipped in the frame before the exact measure arrives.
const ESTIMATED_DESCRIPTION_WIDTH: f32 = 320.0;

/// Builds a virtualized picker list with a viewport of at most seven rows.
///
/// Only the rows inside the viewport are built. The rows keep their stable
/// `u16` keys across filtering, and the list is a single accessibility list
/// box rather than a tab stop per off-screen row. The viewport grows by the
/// expanded description's height, so the selected row and its text stay
/// visible without a nested scroll.
pub fn picker_list(label: &'static str, rows: Vec<PickerRow>) -> Element {
    picker_list_shared(label, Rc::new(rows))
}

/// A picker list that reuses an already shared row model.
///
/// The gems tab keeps its filtered rows in a memo, so passing the `Rc` here
/// avoids rebuilding the model and lets the virtual list compare it by pointer.
pub fn picker_list_shared(label: &'static str, rows: Rc<Vec<PickerRow>>) -> Element {
    let row_count = rows.len();
    let visible = row_count.clamp(1, PICKER_VISIBLE_ROWS);
    PickerList {
        rows,
        height: theme::layout::picker_row_height() * visible as f32,
        label,
        key: DiffKey::None,
    }
    .key("picker-list")
    .into_element()
}

#[derive(PartialEq)]
struct PickerList {
    rows: Rc<Vec<PickerRow>>,
    height: f32,
    label: &'static str,
    key: DiffKey,
}

impl KeyExt for PickerList {
    fn key_mut(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl Component for PickerList {
    fn render(&self) -> impl IntoElement {
        crate::perf::record_picker_render();
        let rows = self.rows.clone();
        let length = rows.len();
        let measured = use_state(HashMap::<u16, f32>::new);
        // The item-size callback is where the measured heights are read; this
        // read subscribes the component itself, so a new measurement rebuilds
        // the rows even if only the callback ever touched the map.
        let _measured_count = measured.read().len();
        let base = theme::layout::picker_row_height();
        let expanded = rows
            .iter()
            .find(|row| row.selected)
            .map_or(0.0, |row| picker_details_height(row, &measured.read()));
        let item_size = {
            let rows = rows.clone();
            move |index: usize| {
                rows.get(index).map_or(base, |row| {
                    base + picker_details_height(row, &measured.read())
                })
            }
        };
        rect()
            .width(Size::fill())
            .height(Size::px(self.height + expanded))
            .a11y_role(AccessibilityRole::ListBox)
            .a11y_alt(self.label)
            .child(
                VirtualScrollView::new_with_data(
                    PickerData { rows, measured },
                    |item, data: &PickerData| {
                        data.rows.get(item.index).map_or_else(
                            || rect().height(Size::px(item.size)).into_element(),
                            |row| picker_row(row, item.size, data.measured),
                        )
                    },
                )
                .length(length)
                .item_size(item_size)
                .show_scrollbar(true)
                .auto_hide_scrollbars(false)
                // The picker sits inside the page's scroll view; drag scrolling
                // here would capture the pointer press and swallow row clicks.
                .drag_scrolling(false)
                .key("picker-rows"),
            )
            .into_element()
    }
}

/// Height an expanded description adds to its row.
///
/// The exact height is stored by the description's `on_sized` handler once the
/// text has been laid out. Until then a conservative estimate keeps the first
/// frame from clipping the text.
fn picker_details_height(row: &PickerRow, measured: &HashMap<u16, f32>) -> f32 {
    if !row.selected {
        return 0.0;
    }
    let Some(description) = row
        .description
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
    else {
        return 0.0;
    };
    measured
        .get(&row.key)
        .copied()
        .unwrap_or_else(|| estimated_details_height(description))
}

/// Estimates the wrapped height of a description line block.
fn estimated_details_height(description: &str) -> f32 {
    let average_glyph = theme::font::caption() * 0.5;
    let chars_per_line = (ESTIMATED_DESCRIPTION_WIDTH / average_glyph).max(1.0);
    let lines = (description.chars().count() as f32 / chars_per_line)
        .ceil()
        .max(1.0);
    theme::font::line_height() * lines + theme::space::xxs() + theme::space::sm()
}

/// One compact row inside a virtualized picker.
fn picker_row(row: &PickerRow, height: f32, measured: State<HashMap<u16, f32>>) -> Element {
    let content = rect()
        .vertical()
        .width(Size::flex(1.0))
        .spacing(theme::space::xxs())
        .child(
            a11y_text(row.title.clone())
                .font_size(theme::font::label())
                .max_lines(1)
                .text_overflow(TextOverflow::Ellipsis)
                .color(theme::text()),
        )
        .child(
            a11y_text(row.subtitle.clone())
                .font_size(theme::font::micro())
                .max_lines(1)
                .text_overflow(TextOverflow::Ellipsis)
                .color(theme::dim()),
        );
    let title: Element = match row.select {
        Some(expanded) => expanding_name(
            row.key,
            row.title.clone(),
            row.subtitle.clone(),
            expanded,
            theme::font::label(),
        ),
        None => content.into_element(),
    };
    let details = picker_details(row, measured);
    rect()
        .vertical()
        .width(Size::fill())
        .height(Size::px(height))
        .key(row.key)
        .a11y_role(AccessibilityRole::Group)
        .a11y_alt(row.accessible.clone())
        .background(if row.selected {
            theme::surface_hover()
        } else {
            theme::surface()
        })
        .corner_radius(theme::radius::control())
        .child(
            rect()
                .horizontal()
                .width(Size::fill())
                .height(Size::px(theme::layout::picker_row_height()))
                .spacing(theme::space::md())
                .content(Content::Flex)
                .cross_align(Alignment::Center)
                .padding(Gaps::new_symmetric(theme::space::xxs(), theme::space::md()))
                .child(rect().width(Size::flex(1.0)).child(title))
                .child(row_action(
                    row.action_label.clone(),
                    row.action_text.clone(),
                    row.action.clone(),
                )),
        )
        .maybe_child(details)
        .into_element()
}

/// The inline description element of a selected picker row.
///
/// The element reports its laid-out height back into the picker's measurement
/// state. The virtual list uses that height for the next layout, so the row
/// grows around the real text instead of reserving a guessed slot.
fn picker_details(row: &PickerRow, measured: State<HashMap<u16, f32>>) -> Option<Element> {
    if !row.selected {
        return None;
    }
    let mut measured = measured;
    let key = row.key;
    details(row.description.as_deref()).map(|details| {
        details
            .key("picker-details")
            .on_sized(move |event: Event<SizedEventData>| {
                let height = event.area.height();
                if height <= 0.0 {
                    return;
                }
                let changed = measured
                    .peek()
                    .get(&key)
                    .is_none_or(|known| (known - height).abs() > 0.5);
                if changed {
                    measured.write().insert(key, height);
                }
            })
            .into_element()
    })
}
