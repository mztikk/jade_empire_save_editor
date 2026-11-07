//! Savegame list and sort controls.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use freya::elements::image::ImageHandle;
use freya::prelude::*;
use jadeforge_core::Character;

use crate::i18n::{fill, keys, t};
use crate::message::capitalize;
use crate::session::{SaveEntry, SaveSort};
use crate::theme;
use crate::widgets::{AccessibleButton, Action};

use super::Ctx;
use super::shell::preview_slot;

#[allow(
    clippy::too_many_lines, // one declarative sidebar layout keeps the preview, filters and list in order
    clippy::needless_pass_by_value, // the closures own the sync handle
    clippy::too_many_arguments // each argument is a distinct reactive handle
)]
pub(super) fn sidebar(
    ctx: &Ctx,
    entries: &Rc<Vec<SaveEntry>>,
    visible: &Rc<Vec<usize>>,
    selected: Option<usize>,
    focus: State<usize>,
    preview: Option<ImageHandle>,
    character: Option<&Character>,
    scanning: bool,
    actions: &Rc<RefCell<HashMap<usize, Action>>>,
    query: State<String>,
    filter_id: AccessibilityId,
    sort: State<(SaveSort, bool)>,
    reset: Rc<dyn Fn()>,
    sync: Rc<dyn Fn()>,
) -> Element {
    let open = {
        let ctx = ctx.clone();
        let reset = reset.clone();
        let sync = sync.clone();
        use_hook(move || {
            OpenSave::new(move |index| {
                let changed = ctx.run(|session| {
                    let before = session.selected();
                    session.open(index);
                    session.selected() != before
                });
                if changed {
                    reset();
                }
                sync();
            })
        })
    };

    let focus_index = *focus.read();
    let focused_index = if visible.contains(&focus_index) {
        Some(focus_index)
    } else {
        visible.first().copied()
    };

    // Above 100 percent the fixed preview and the duplicate selected-save
    // block consume the list's height: at 125 percent German the list measured
    // 21 px. Both are decorative here, so the save browser keeps its rows
    // instead.
    let large_text = theme::text_scale() > 1.0;
    let show_preview = !large_text;
    let preview_block = preview_slot(preview, selected.is_some());
    let heading = if entries.is_empty() && scanning {
        t(keys::SHELL_LOADING_SAVEGAMES).to_uppercase()
    } else if visible.len() == entries.len() {
        crate::widgets::plural(
            entries.len(),
            keys::COUNT_SAVEGAME_ONE,
            keys::COUNT_SAVEGAME_OTHER,
        )
        .to_uppercase()
    } else {
        let shown = visible.len().to_string();
        let total = crate::widgets::plural(
            entries.len(),
            keys::COUNT_SAVEGAME_ONE,
            keys::COUNT_SAVEGAME_OTHER,
        )
        .to_uppercase();
        fill(
            keys::SAVE_BROWSER_OF,
            &[("shown", &shown), ("total", &total)],
        )
    };
    let list = if entries.is_empty() && scanning {
        loading_rows()
    } else {
        SaveList {
            data: SaveRows {
                entries: Rc::clone(entries),
                visible: Rc::clone(visible),
                selected,
                focused: focused_index,
                focus,
                actions: Rc::clone(actions),
                open,
            },
            key: DiffKey::None,
        }
        .key("save-list")
        .into_element()
    };

    let title = character.as_ref().map_or_else(
        || t(keys::SHELL_NO_SAVEGAME_SELECTED).to_owned(),
        |character| character.name.clone(),
    );
    let subtitle = character.as_ref().map_or_else(
        || t(keys::SAVE_BROWSER_SELECT_A_SAVEGAME).to_owned(),
        |character| {
            fill(
                keys::SESSION_ROW_LEVEL_TIME,
                &[
                    ("level", &character.level.to_string()),
                    (
                        "time",
                        &crate::session::format::duration(character.time_played),
                    ),
                ],
            )
        },
    );
    let identity_block = (!large_text).then(|| {
        rect()
            .vertical()
            .spacing(theme::space::xs())
            .child(crate::widgets::with_tooltip(
                title.clone(),
                crate::widgets::a11y_text(title)
                    .a11y_builder(|node| node.set_role(AccessibilityRole::Heading))
                    .font_size(theme::font::card_title())
                    .font_weight(FontWeight::BOLD)
                    .max_lines(1)
                    .text_overflow(TextOverflow::Ellipsis)
                    .color(theme::text()),
            ))
            .child(
                crate::widgets::a11y_text(subtitle)
                    .font_size(theme::font::micro())
                    .color(theme::dim()),
            )
    });

    rect()
        .vertical()
        .width(Size::percent(theme::layout::sidebar_percent()))
        .min_width(Size::px(theme::layout::sidebar_min_width()))
        .max_width(Size::px(theme::layout::sidebar_max_width()))
        .height(Size::fill())
        .content(Content::Flex)
        .padding(Gaps::new(
            theme::space::field(),
            theme::space::xl(),
            theme::space::field(),
            theme::space::xl(),
        ))
        .spacing(theme::space::lg())
        .background(theme::background())
        .maybe_child(show_preview.then_some(preview_block))
        .maybe_child(identity_block)
        .child(
            crate::widgets::a11y_text(heading)
                .font_size(theme::font::micro())
                .font_weight(FontWeight::BOLD)
                .color(if entries.is_empty() && scanning {
                    theme::warning()
                } else {
                    theme::dim()
                }),
        )
        .child(crate::widgets::filter_input(
            query,
            crate::app::SAVE_FILTER_PLACEHOLDER,
            keys::SAVE_BROWSER_CLEAR_FILTER,
            filter_id,
        ))
        .child(sort_bar(ctx, sort, *sort.read()))
        .child(list)
        .into_element()
}

/// Placeholder rows shown while the first scan is still running.
///
/// They occupy the same row pitch as real savegames so selecting the first
/// save does not move the heading or the filter below it. They have no text,
/// so the sidebar's status heading and the header carry the accessible state.
fn loading_rows() -> Element {
    let mut rows: Vec<Element> = Vec::new();
    for index in 0..6 {
        rows.push(
            rect()
                .horizontal()
                .width(Size::fill())
                .height(Size::px(theme::layout::save_row_height()))
                .padding(Gaps::new_symmetric(theme::space::md(), theme::space::lg()))
                .cross_align(Alignment::Center)
                .background(theme::surface())
                .corner_radius(theme::radius::medium())
                .key(index)
                .child(
                    rect()
                        .width(Size::percent(theme::layout::skeleton_bar_percent()))
                        .height(Size::px(theme::font::line_height()))
                        .background(theme::surface_active())
                        .corner_radius(theme::radius::small()),
                )
                .into_element(),
        );
    }
    rect()
        .vertical()
        .width(Size::fill())
        .height(Size::flex(1.0))
        .spacing(theme::space::xs())
        .children(rows)
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

/// Height of one savegame row plus the gap before the next row.
///
/// A function rather than a constant so the pitch follows the user's text
/// scale; the row must grow with its text or the name and level clip.
fn save_row_pitch() -> f32 {
    theme::layout::save_row_height() + theme::space::xs()
}

/// Everything the virtual save list needs to build one row on demand.
#[derive(Clone)]
struct SaveRows {
    entries: Rc<Vec<SaveEntry>>,
    visible: Rc<Vec<usize>>,
    selected: Option<usize>,
    focused: Option<usize>,
    focus: State<usize>,
    actions: Rc<RefCell<HashMap<usize, Action>>>,
    open: OpenSave,
}

impl PartialEq for SaveRows {
    /// Shared collections compare by identity: cloning the data is cheap, and
    /// comparing every savegame label on each render would make the list pay
    /// for the rows it no longer builds.
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.entries, &other.entries)
            && Rc::ptr_eq(&self.visible, &other.visible)
            && Rc::ptr_eq(&self.actions, &other.actions)
            && self.selected == other.selected
            && self.focused == other.focused
            && self.focus == other.focus
            && self.open == other.open
    }
}

/// The accessible save list: one focusable list box with roving visual focus.
///
/// Rows are built by [`VirtualScrollView`], so a folder with hundreds of
/// savegames costs the same to render as a folder with ten. Before this, every
/// row (two tooltips and two text paragraphs) was built even when it was far
/// outside the sidebar, which cost well over a hundred kilobytes per savegame.
#[derive(PartialEq)]
struct SaveList {
    data: SaveRows,
    key: DiffKey,
}

impl KeyExt for SaveList {
    fn key_mut(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl Component for SaveList {
    fn render(&self) -> impl IntoElement {
        crate::perf::record_save_list_render();
        let a11y_id = use_a11y();
        let focus_state = use_focus(a11y_id);
        let is_focused = focus_state() == Focus::Keyboard;
        let mut focus = self.data.focus;
        let mut scroll = use_scroll_controller(ScrollConfig::default);
        let visible = self.data.visible.clone();
        let open = self.data.open.clone();
        let _ = *focus.read();

        // A savegame can be selected from outside the list (a rescan, opening
        // another folder); keep the selected row in view then too, but only
        // once per selection change (see `reveal_selection`).
        let revealed = use_hook(|| Rc::new(RevealedPosition::default()));
        let selected_position = self
            .data
            .selected
            .and_then(|selected| visible.iter().position(|index| *index == selected));
        use_side_effect_with_deps(&selected_position, move |position| {
            reveal_selection(&mut scroll, &revealed, *position);
        });

        let content = if visible.is_empty() {
            rect()
                .padding(Gaps::new_all(theme::space::xl()))
                .child(
                    crate::widgets::a11y_text(if self.data.entries.is_empty() {
                        t(keys::SAVE_BROWSER_EMPTY_FOLDER)
                    } else {
                        t(keys::SAVE_BROWSER_NO_MATCH)
                    })
                    .font_size(theme::font::caption())
                    .color(theme::dim()),
                )
                .into_element()
        } else {
            let data = self.data.clone();
            VirtualScrollView::new_with_data(data, |item, data: &SaveRows| {
                data.visible.get(item.index).map_or_else(
                    || rect().height(Size::px(item.size)).into_element(),
                    |index| save_row(data, *index, item.size),
                )
            })
            .length(visible.len())
            .item_size(save_row_pitch())
            .show_scrollbar(true)
            .auto_hide_scrollbars(false)
            .scroll_controller(scroll)
            .key("save-rows")
            .into_element()
        };

        rect()
            .vertical()
            .width(Size::fill())
            .height(Size::flex(1.0))
            .a11y_id(a11y_id)
            .a11y_focusable(true)
            .a11y_role(AccessibilityRole::ListBox)
            .a11y_alt(capitalize(t(keys::COUNT_SAVEGAME_OTHER)))
            .border(
                Border::new()
                    .fill(if is_focused {
                        theme::text()
                    } else {
                        Color::TRANSPARENT
                    })
                    .width(theme::stroke::focus()),
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
                    if let Some(position) =
                        visible.iter().position(|candidate| *candidate == target)
                    {
                        reveal_row(&mut scroll, position);
                    }
                }
            })
            .child(content)
            .into_element()
    }
}

/// Builds one savegame row for the virtual list.
///
/// `size` is the pitch the list reserved for the row; the card takes the
/// remaining height after the gap. The filter is derived from the same entry
/// list, so a missing entry means the two snapshots disagreed; render the
/// reserved gap instead of panicking on user data.
fn save_row(data: &SaveRows, index: usize, size: f32) -> Element {
    let Some(entry) = data.entries.get(index) else {
        return rect()
            .width(Size::fill())
            .height(Size::px(size))
            .into_element();
    };
    let is_selected = data.selected == Some(index);
    // The roving list focus is not a DOM focus on the row, so the row paints
    // it; hover is handled by the button itself and therefore does not rebuild
    // any row.
    let is_focused = data.focused == Some(index);
    let broken = entry.error.is_some();
    let summary = entry.summary();
    let title = entry.label.clone();
    let accessible = format!("{title}, {summary}");
    let action = {
        let mut cache = data.actions.borrow_mut();
        cache
            .entry(index)
            .or_insert_with(|| {
                let open = data.open.clone();
                let mut focus = data.focus;
                Action::new(move || {
                    *focus.write() = index;
                    open.call(index);
                })
            })
            .clone()
    };

    rect()
        .width(Size::fill())
        .height(Size::px(size))
        .padding(Gaps::new(0.0, 0.0, theme::space::xs(), 0.0))
        .key(index)
        .child(
            AccessibleButton::new(
                accessible,
                action,
                rect()
                    .horizontal()
                    .width(Size::fill())
                    .height(Size::px(theme::layout::save_row_height()))
                    .corner_radius(theme::radius::medium())
                    .background(if is_selected {
                        theme::surface_active()
                    } else if is_focused {
                        theme::surface_hover()
                    } else {
                        theme::surface()
                    })
                    // The bar is always present, transparent until the row
                    // is selected, so the row text does not shift by its
                    // width when the selection moves.
                    .child(
                        rect()
                            .width(Size::px(theme::layout::selection_bar_width()))
                            .height(Size::fill())
                            .background(if is_selected {
                                theme::accent()
                            } else {
                                Color::TRANSPARENT
                            }),
                    )
                    .child(
                        rect()
                            .vertical()
                            .width(Size::flex(1.0))
                            .spacing(theme::space::xxs())
                            .padding(Gaps::new_symmetric(theme::space::md(), theme::space::lg()))
                            .child(crate::widgets::with_tooltip(
                                format!("{}, {summary}", entry.label),
                                crate::widgets::a11y_text(title)
                                    .font_size(theme::font::label())
                                    .max_lines(1)
                                    .text_overflow(TextOverflow::Ellipsis)
                                    .color(if broken {
                                        theme::danger()
                                    } else {
                                        theme::text()
                                    }),
                            ))
                            .child(crate::widgets::with_tooltip(
                                format!("{}, {summary}", entry.label),
                                crate::widgets::a11y_text(summary)
                                    .font_size(theme::font::micro())
                                    .max_lines(1)
                                    .text_overflow(TextOverflow::Ellipsis)
                                    .color(theme::dim()),
                            )),
                    )
                    .into_element(),
            )
            .role(AccessibilityRole::ListBoxOption)
            .selected(is_selected)
            .focusable(false)
            .hover_background(theme::surface_hover())
            .fill_width()
            .into_element(),
        )
        .into_element()
}

/// Last selection position that was scrolled into view.
///
/// The outer `Option` distinguishes "nothing revealed yet" from "no row is
/// selected or the selected savegame is filtered out".
type RevealedPosition = std::cell::Cell<Option<Option<usize>>>;

/// Reveals the selected row once per selection change.
///
/// `use_side_effect_with_deps` runs after every render, not only when its
/// dependency changed. Re-revealing on each scroll tick pulled the list back to
/// the selected savegame and made everything below it unreachable.
fn reveal_selection(
    controller: &mut ScrollController,
    revealed: &RevealedPosition,
    position: Option<usize>,
) {
    if revealed.get() == Some(position) {
        return;
    }
    revealed.set(Some(position));
    if let Some(position) = position {
        reveal_row(controller, position);
    }
}

/// Scrolls the list just enough to show the row at `position` completely.
fn reveal_row(controller: &mut ScrollController, position: usize) {
    let Some((_, viewport)) = *controller.bounds.read() else {
        return;
    };
    if viewport.height <= 0.0 {
        return;
    }
    let top = position as f32 * save_row_pitch();
    let bottom = top + save_row_pitch();
    let viewport_top = -controller.position().y;
    let viewport_bottom = viewport_top + viewport.height;
    if top < viewport_top {
        controller.scroll_to_y(-(top.round() as i32));
    } else if bottom > viewport_bottom {
        controller.scroll_to_y(-((bottom - viewport.height).round() as i32));
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
            fill(
                keys::SAVE_BROWSER_SORT_BY_ACTIVE,
                &[
                    ("name", t(key.label())),
                    (
                        "direction",
                        if descending {
                            t(keys::SORT_DESCENDING)
                        } else {
                            t(keys::SORT_ASCENDING)
                        },
                    ),
                ],
            )
        } else {
            fill(keys::SAVE_BROWSER_SORT_BY, &[("name", t(key.label()))])
        };
        buttons.push(
            AccessibleButton::new(
                accessible,
                action,
                rect()
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .padding(Gaps::new_symmetric(theme::space::sm(), theme::space::xs()))
                    .child(
                        crate::widgets::a11y_text(t(key.label()))
                            .font_size(theme::font::micro())
                            .color(if is_active {
                                theme::accent()
                            } else {
                                theme::muted()
                            }),
                    )
                    // The arrow lives in a fixed slot on every button. Its
                    // glyph width differs between platform fonts, so sizing
                    // the slot from the active label moved the following
                    // buttons.
                    .child(
                        rect()
                            .width(Size::px(theme::layout::sort_arrow_width()))
                            .height(Size::px(theme::layout::sort_arrow_height()))
                            .center()
                            .child(if is_active {
                                crate::widgets::a11y_text(arrow)
                                    .font_size(theme::font::micro())
                                    .color(theme::accent())
                            } else {
                                crate::widgets::a11y_text("").font_size(theme::font::micro())
                            }),
                    ),
            )
            .hover_background(theme::surface_hover())
            .key(t(key.label()))
            .into_element(),
        );
    }
    rect()
        .horizontal()
        .width(Size::fill())
        .spacing(theme::space::xs())
        .content(Content::wrap_spacing(theme::space::xs()))
        .children(buttons)
        .into_element()
}
