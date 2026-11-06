//! Savegame list and sort controls.

use std::rc::Rc;

use freya::elements::image::ImageHandle;
use freya::prelude::*;
use jadeforge_core::Character;

use crate::session::{SaveEntry, SaveSort};
use crate::theme;
use crate::widgets::{AccessibleButton, Action};

use super::shell::preview_slot;
use super::{Ctx, HoverKey};

#[allow(
    clippy::too_many_lines, // one declarative sidebar layout keeps the rows together
    clippy::needless_pass_by_value, // the closures own the sync handle
    clippy::too_many_arguments // each argument is a distinct reactive handle
)]
pub(super) fn sidebar(
    ctx: &Ctx,
    entries: &[SaveEntry],
    visible: &[usize],
    selected: Option<usize>,
    focus: State<usize>,
    preview: Option<ImageHandle>,
    character: Option<&Character>,
    form_differs: bool,
    query: State<String>,
    sort: State<(SaveSort, bool)>,
    hovered: State<Option<HoverKey>>,
    sync: Rc<dyn Fn()>,
) -> Element {
    let open = OpenSave::new({
        let ctx = ctx.clone();
        let sync = sync.clone();
        move |index| {
            if form_differs && selected != Some(index) {
                ctx.run(|session| {
                    session
                        .set_error("Fix the invalid character fields before switching savegames.");
                });
                return;
            }
            ctx.run(|session| {
                session.open(index);
            });
            sync();
        }
    });

    let focus_index = *focus.read();
    let focused_index = if visible.contains(&focus_index) {
        Some(focus_index)
    } else {
        visible.first().copied()
    };
    let mut rows: Vec<Element> = Vec::new();
    for &index in visible {
        let entry = &entries[index];
        let is_selected = selected == Some(index);
        let is_focused = focused_index == Some(index);
        let is_hovered = *hovered.read() == Some(HoverKey::Save(index));
        let broken = entry.error.is_some();
        let summary = entry.summary();
        let title = entry.label.clone();
        let accessible = format!("{title}, {summary}");
        let open_for_row = open.clone();
        let mut focus_for_row = focus;
        let action = Action::new(move || {
            *focus_for_row.write() = index;
            open_for_row.call(index);
        });
        rows.push(
            AccessibleButton::new(
                accessible,
                action,
                rect()
                    .horizontal()
                    .width(Size::fill())
                    .height(Size::px(theme::layout::SAVE_ROW_HEIGHT))
                    .corner_radius(theme::radius::MEDIUM)
                    .background(if is_selected {
                        theme::SURFACE_ACTIVE
                    } else if is_hovered || is_focused {
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
                    .child(
                        rect()
                            .width(Size::px(theme::layout::SELECTION_BAR_WIDTH))
                            .height(Size::fill())
                            .background(if is_selected {
                                theme::ACCENT
                            } else {
                                Color::TRANSPARENT
                            }),
                    )
                    .child(
                        rect()
                            .vertical()
                            .width(Size::flex(1.0))
                            .spacing(theme::space::XXS)
                            .padding(Gaps::new_symmetric(theme::space::MD, theme::space::LG))
                            .child(crate::widgets::with_tooltip(
                                format!("{title}, {summary}"),
                                crate::widgets::a11y_text(title)
                                    .font_size(theme::font::LABEL)
                                    .max_lines(1)
                                    .text_overflow(TextOverflow::Ellipsis)
                                    .color(if broken { theme::DANGER } else { theme::TEXT }),
                            ))
                            .child(
                                crate::widgets::a11y_text(summary)
                                    .font_size(theme::font::MICRO)
                                    .color(theme::DIM),
                            ),
                    )
                    .into_element(),
            )
            .role(AccessibilityRole::ListBoxOption)
            .selected(is_selected)
            .focusable(false)
            .fill_width()
            .key(index)
            .into_element(),
        );
    }
    if rows.is_empty() {
        rows.push(
            rect()
                .padding(Gaps::new_all(theme::space::XL))
                .child(
                    crate::widgets::a11y_text(if entries.is_empty() {
                        "No savegames in this folder."
                    } else {
                        "No savegame matches this filter."
                    })
                    .font_size(theme::font::CAPTION)
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
        || "Select a savegame".to_owned(),
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
        .width(Size::percent(theme::layout::SIDEBAR_PERCENT))
        .min_width(Size::px(theme::layout::SIDEBAR_MIN_WIDTH))
        .max_width(Size::px(theme::layout::SIDEBAR_MAX_WIDTH))
        .height(Size::fill())
        .content(Content::Flex)
        .padding(Gaps::new(
            theme::space::FIELD,
            theme::space::XL,
            theme::space::FIELD,
            theme::space::XL,
        ))
        .spacing(theme::space::LG)
        .background(theme::BACKGROUND)
        .child(preview_block)
        .child(
            rect()
                .vertical()
                .spacing(theme::space::XXS)
                .child(crate::widgets::with_tooltip(
                    title.clone(),
                    crate::widgets::a11y_text(title)
                        .a11y_builder(|node| node.set_role(AccessibilityRole::Heading))
                        .font_size(theme::font::CARD_TITLE)
                        .font_weight(FontWeight::BOLD)
                        .max_lines(1)
                        .text_overflow(TextOverflow::Ellipsis)
                        .color(theme::TEXT),
                ))
                .child(
                    crate::widgets::a11y_text(subtitle)
                        .font_size(theme::font::MICRO)
                        .color(theme::DIM),
                ),
        )
        .child(
            crate::widgets::a11y_text(if visible.len() == entries.len() {
                format!("SAVEGAMES ({})", entries.len())
            } else {
                format!("SAVEGAMES ({} of {})", visible.len(), entries.len())
            })
            .font_size(theme::font::MICRO)
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
                .auto_hide_scrollbars(false)
                .width(Size::fill())
                .height(Size::flex(1.0))
                .child(
                    SaveList {
                        rows,
                        visible: Rc::new(visible.to_vec()),
                        focus,
                        open,
                        key: DiffKey::None,
                    }
                    .key("save-list")
                    .into_element(),
                ),
        )
        .into_element()
}

/// Opens a savegame by index; shared by the rows, the list box and Enter.
#[derive(Clone)]
struct OpenSave(Rc<dyn Fn(usize)>);

impl PartialEq for OpenSave {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl OpenSave {
    fn new(action: impl Fn(usize) + 'static) -> Self {
        Self(Rc::new(action))
    }

    fn call(&self, index: usize) {
        (self.0)(index);
    }
}

/// The accessible save list: one focusable list box with roving visual focus.
#[derive(PartialEq)]
struct SaveList {
    rows: Vec<Element>,
    visible: Rc<Vec<usize>>,
    focus: State<usize>,
    open: OpenSave,
    key: DiffKey,
}

impl KeyExt for SaveList {
    fn key_mut(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl Component for SaveList {
    fn render(&self) -> impl IntoElement {
        let a11y_id = use_a11y();
        let focus_state = use_focus(a11y_id);
        let is_focused = focus_state() == Focus::Keyboard;
        let mut focus = self.focus;
        let visible = self.visible.clone();
        let open = self.open.clone();
        let _ = *focus.read();

        rect()
            .vertical()
            .width(Size::fill())
            .spacing(theme::space::XS)
            .a11y_id(a11y_id)
            .a11y_focusable(true)
            .a11y_role(AccessibilityRole::ListBox)
            .a11y_alt("Savegames")
            .border(
                Border::new()
                    .fill(if is_focused {
                        theme::TEXT
                    } else {
                        Color::TRANSPARENT
                    })
                    .width(theme::stroke::FOCUS),
            )
            .on_press(move |_| a11y_id.request_focus())
            .on_key_down(move |event: Event<KeyboardEventData>| {
                let current = *focus.read();
                let current = if visible.contains(&current) {
                    current
                } else {
                    visible.first().copied().unwrap_or(0)
                };
                let position = visible.iter().position(|candidate| *candidate == current);
                let target = match event.key {
                    Key::Named(NamedKey::ArrowDown) => position.map_or_else(
                        || visible.first().copied(),
                        |position| visible.get(position + 1).copied(),
                    ),
                    Key::Named(NamedKey::ArrowUp) => position
                        .and_then(|position| position.checked_sub(1))
                        .and_then(|position| visible.get(position).copied()),
                    Key::Named(NamedKey::Home) => visible.first().copied(),
                    Key::Named(NamedKey::End) => visible.last().copied(),
                    Key::Named(NamedKey::Enter) => {
                        open.call(current);
                        None
                    }
                    _ => None,
                };
                if let Some(target) = target {
                    *focus.write() = target;
                }
            })
            .children(self.rows.clone())
            .into_element()
    }
}

/// Sort buttons for the savegame list; pressing the active one flips the order.
fn sort_bar(ctx: &Ctx, mut state: State<(SaveSort, bool)>, active: (SaveSort, bool)) -> Element {
    let mut buttons: Vec<Element> = Vec::new();
    for key in [SaveSort::Name, SaveSort::Level, SaveSort::Played] {
        let (current, descending) = active;
        let is_active = key == current;
        let arrow = if descending { "↓" } else { "↑" };
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
                    .child(
                        crate::widgets::a11y_text(key.label())
                            .font_size(theme::font::MICRO)
                            .color(if is_active { theme::ACCENT } else { theme::DIM }),
                    )
                    // The arrow lives in a fixed slot on every button. Its
                    // glyph width differs between platform fonts, so sizing
                    // the slot from the active label moved the following
                    // buttons.
                    .child(
                        rect()
                            .width(Size::px(theme::layout::SORT_ARROW_WIDTH))
                            .height(Size::px(theme::layout::SORT_ARROW_HEIGHT))
                            .center()
                            .child(if is_active {
                                crate::widgets::a11y_text(arrow)
                                    .font_size(theme::font::MICRO)
                                    .color(theme::ACCENT)
                            } else {
                                crate::widgets::a11y_text("").font_size(theme::font::MICRO)
                            }),
                    ),
            )
            .key(key.label())
            .into_element(),
        );
    }
    rect()
        .horizontal()
        .width(Size::fill())
        .spacing(theme::space::LG)
        .content(Content::Flex)
        .child(
            crate::widgets::a11y_text("SORT")
                .font_size(theme::font::MICRO)
                .font_weight(FontWeight::BOLD)
                .color(theme::DIM),
        )
        .children(buttons)
        .into_element()
}
