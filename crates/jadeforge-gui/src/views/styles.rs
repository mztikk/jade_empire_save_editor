//! The styles tab: the fighting styles the player knows.

use freya::prelude::*;

use crate::app::Ctx;
use std::cell::RefCell;
use std::rc::Rc;

use crate::app::HoverKey;
use crate::session::{Session, StyleRow, StyleSuggestion, StyleUpgrade};
use crate::theme;
use crate::widgets;

/// Renders the styles tab.
#[allow(clippy::too_many_arguments)]
pub fn view(
    ctx: &Ctx,
    styles: &[StyleRow],
    query: State<String>,
    matches: &[StyleSuggestion],
    game_ready: bool,
    learnable: Option<usize>,
    matching: usize,
    expanded: State<Option<u16>>,
    hovered: State<Option<crate::app::HoverKey>>,
    show_more: impl FnMut(Event<PressEventData>) + 'static,
) -> Element {
    let mut rows: Vec<Element> = Vec::new();
    for style in styles {
        rows.push(style_row(ctx, style, expanded, hovered));
    }
    if rows.is_empty() {
        rows.push(widgets::hint(
            "The player has not learned any fighting styles.",
        ));
    }

    let mut children = vec![
        rect()
            .vertical()
            .width(Size::fill())
            .spacing(4.)
            .children(rows)
            .into_element(),
        widgets::divider(),
        widgets::section_label("LEARN A STYLE"),
        widgets::hint(
            "A new style is stored with the same field layout as one the player already owns; \
             the game fills in the remaining data from its own tables.",
        ),
    ];

    if game_ready {
        children.push(
            Input::new(query)
                .placeholder("Filter the style list, or leave empty to browse it")
                .width(Size::fill())
                .into_element(),
        );
        if matches.is_empty() {
            children.push(widgets::hint(if query.read().trim().is_empty() {
                "Every style of the game is already known."
            } else {
                "No styles match this search."
            }));
        } else {
            let browsing = query.read().trim().is_empty();
            let shown = matches.len();
            let mut heading = rect()
                .horizontal()
                .width(Size::fill())
                .spacing(10.)
                .content(Content::Flex)
                .child(widgets::section_label(if browsing {
                    "ALL STYLES"
                } else {
                    "SEARCH RESULTS"
                }))
                .child(rect().width(Size::flex(1.0)))
                .child(
                    label()
                        .text(match (browsing, shown == matching) {
                            (true, _) => format!("{shown} of {matching}"),
                            (false, true) => crate::widgets::plural(matching, "match", "matches"),
                            (false, false) => format!(
                                "{shown} of {}",
                                crate::widgets::plural(matching, "match", "matches")
                            ),
                        })
                        .font_size(10.)
                        .color(theme::DIM),
                );
            if shown < matching {
                heading = heading.child(
                    Button::new()
                        .compact()
                        .outline()
                        .child("Show more")
                        .on_press(show_more),
                );
            }
            children.push(heading.into_element());
            let mut result_rows: Vec<Element> = Vec::new();
            for suggestion in matches {
                result_rows.push(learn_row(ctx, suggestion, expanded, hovered));
            }
            children.push(
                rect()
                    .vertical()
                    .width(Size::fill())
                    .spacing(2.)
                    .children(result_rows)
                    .into_element(),
            );
        }
    } else {
        children.push(widgets::hint(
            "The game data was not found, so style names and new styles are unavailable.",
        ));
    }

    widgets::card(
        "Fighting styles",
        Some(match learnable {
            Some(learnable) => format!("{} learned · {learnable} learnable", styles.len()),
            None => format!("{} learned", styles.len()),
        }),
        children,
    )
}

fn style_row(
    ctx: &Ctx,
    style: &StyleRow,
    expanded: State<Option<u16>>,
    mut hovered: State<Option<crate::app::HoverKey>>,
) -> Element {
    let key = crate::app::HoverKey::Style(style.struct_index);
    let is_hovered = *hovered.read() == Some(key);
    let index = style.struct_index;
    let level = style.level;
    let remove = {
        let ctx = ctx.clone();
        move |_| ctx.run(|session| session.remove_style(index))
    };

    let subtitle = style_subtitle(style.style_id, &style.label)
        .unwrap_or_else(|| format!("#{}", style.style_id));
    let is_expanded = *expanded.read() == Some(style.style_id);

    rect()
        .vertical()
        .width(Size::fill())
        .key(style.struct_index)
        .background(if is_hovered {
            theme::SURFACE_HOVER
        } else {
            theme::SURFACE
        })
        .corner_radius(8.)
        .on_pointer_enter(move |_| *hovered.write() = Some(key))
        .on_pointer_leave(move |_| {
            if *hovered.read() == Some(key) {
                *hovered.write() = None;
            }
        })
        .child(
            rect()
                .horizontal()
                .width(Size::fill())
                .spacing(10.)
                .content(Content::Flex)
                .padding(Gaps::new_symmetric(4., 6.))
                .child(widgets::expanding_name(
                    style.style_id,
                    style.name.clone(),
                    subtitle,
                    expanded,
                    13.,
                ))
                .child(widgets::badge(
                    style.kind,
                    theme::SURFACE_ACTIVE,
                    style_badge_color(style.kind),
                ))
                .child(
                    label()
                        .text(format!("Lv {level}"))
                        .font_size(12.)
                        .width(Size::px(46.))
                        .text_align(TextAlign::Center)
                        .color(theme::ACCENT),
                )
                .child(
                    Button::new()
                        .compact()
                        .flat()
                        .child("Remove")
                        .on_press(remove),
                ),
        )
        .maybe_child(if is_expanded {
            Some(upgrade_panel(ctx, style, hovered))
        } else {
            None
        })
        .into_element()
}

/// Expanded panel: description plus the upgrade choices of the style.
fn upgrade_panel(
    ctx: &Ctx,
    style: &StyleRow,
    hovered: State<Option<crate::app::HoverKey>>,
) -> Element {
    let mut rows: Vec<Element> = Vec::new();
    if let Some(details) = widgets::details(style.description.as_deref()) {
        rows.push(details);
    }
    if style.upgrades.is_empty() {
        if style.description.is_none() {
            rows.push(widgets::hint(
                "No upgrade data available: install the game data next to the savegames.",
            ));
        }
        return rect()
            .vertical()
            .width(Size::fill())
            .spacing(4.)
            .padding(Gaps::new(0., 10., 6., 10.))
            .children(rows)
            .into_element();
    }

    rows.push(widgets::section_label("UPGRADES"));
    let mut family: Option<&str> = None;
    let mut chips: Vec<Element> = Vec::new();
    for upgrade in &style.upgrades {
        if family.is_some_and(|family| family != upgrade.family) {
            rows.push(upgrade_row(
                family.unwrap_or_default(),
                std::mem::take(&mut chips),
            ));
        }
        if family != Some(upgrade.family.as_str()) {
            family = Some(upgrade.family.as_str());
        }
        chips.push(upgrade_chip(ctx, style.struct_index, upgrade, hovered));
    }
    if let Some(family) = family {
        rows.push(upgrade_row(family, chips));
    }
    rows.push(widgets::hint(
        "Orbs are the level-up options, in order; gold means owned.",
    ));
    rect()
        .vertical()
        .width(Size::fill())
        .spacing(6.)
        .padding(Gaps::new(0., 10., 8., 10.))
        .children(rows)
        .into_element()
}

/// One family line: its name and a chip per tier.
fn upgrade_row(family: &str, chips: Vec<Element>) -> Element {
    rect()
        .horizontal()
        .width(Size::fill())
        .key(family.to_owned())
        .spacing(6.)
        .content(Content::Flex)
        .child(
            label()
                .text(family_title(family))
                .font_size(11.)
                .width(Size::px(120.))
                .color(theme::MUTED),
        )
        .children(chips)
        .into_element()
}

/// Session handle that compares by pointer, so it can be a component prop.
#[derive(Clone)]
struct SharedSession(Rc<RefCell<Session>>);

impl PartialEq for SharedSession {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// One upgrade orb: a focusable circle that buys or removes a tier.
///
/// A component is needed (instead of a plain rect) to get an accessibility id
/// and the keyboard focus state, which is what makes Tab + Enter work and lets
/// the focused orb show a ring.
#[derive(PartialEq)]
struct UpgradeOrb {
    session: SharedSession,
    revision: State<u32>,
    struct_index: usize,
    upgrade: StyleUpgrade,
    hovered: State<Option<HoverKey>>,
    key: DiffKey,
}

impl KeyExt for UpgradeOrb {
    fn write_key(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl Component for UpgradeOrb {
    fn render(&self) -> impl IntoElement {
        let a11y_id = use_a11y();
        let focus = use_focus(a11y_id);
        let key = HoverKey::Orb(self.struct_index, self.upgrade.slot);
        let is_hovered = *self.hovered.read() == Some(key);
        let focused = focus() == Focus::Keyboard;
        let taken = self.upgrade.taken;
        let background = if taken {
            if is_hovered || focused {
                theme::ACCENT_HOVER
            } else {
                theme::ACCENT
            }
        } else if is_hovered || focused {
            theme::SURFACE_ACTIVE
        } else {
            theme::BACKGROUND
        };
        let border = if focused {
            theme::TEXT
        } else if taken || is_hovered {
            theme::ACCENT_HOVER
        } else {
            theme::ACCENT
        };
        let border_width = if focused { 2.0 } else { 1.5 };
        let mut hovered = self.hovered;
        let session = self.session.clone();
        let mut revision = self.revision;
        let struct_index = self.struct_index;
        let slot = self.upgrade.slot;
        rect()
            .center()
            .width(Size::px(26.))
            .height(Size::px(26.))
            .corner_radius(13.)
            .background(background)
            .border(Border::new().fill(border).width(border_width))
            .a11y_id(a11y_id)
            .a11y_focusable(true)
            .a11y_role(AccessibilityRole::Button)
            .a11y_alt(format!("tier {}", self.upgrade.tier))
            .on_pointer_enter(move |_| {
                Cursor::set(CursorIcon::Pointer);
                *hovered.write() = Some(key);
            })
            .on_pointer_leave(move |_| {
                Cursor::set(CursorIcon::default());
                if *hovered.read() == Some(key) {
                    *hovered.write() = None;
                }
            })
            .on_all_press(move |_| {
                session.0.borrow_mut().change_upgrade(struct_index, slot);
                *revision.write() += 1;
            })
            .into_element()
    }
}

/// Renders one orb.
fn upgrade_chip(
    ctx: &Ctx,
    struct_index: usize,
    upgrade: &StyleUpgrade,
    hovered: State<Option<HoverKey>>,
) -> Element {
    UpgradeOrb {
        session: SharedSession(ctx.session.clone()),
        revision: ctx.revision,
        struct_index,
        upgrade: upgrade.clone(),
        hovered,
        key: DiffKey::None,
    }
    .key((struct_index, upgrade.slot))
    .into_element()
}

/// Human readable family name for the upgrade rows.
#[must_use]
pub fn family_title(family: &str) -> String {
    if family.contains(' ') {
        family.to_owned()
    } else {
        family.replace('_', " ")
    }
}

/// Secondary line shown for a style, owned or not.
///
/// `None` when the game data is missing: the name is then just `Style <id>`
/// and repeating the id in a subtitle would be noise.
#[must_use]
pub fn style_subtitle(style_id: u16, label: &str) -> Option<String> {
    (!label.is_empty()).then(|| format!("{label} · #{style_id}"))
}

/// Badge colour for a style kind.
fn style_badge_color(kind: &str) -> Color {
    match kind {
        "magic" | "support" => theme::JADE,
        "transformation" | "unique" => theme::ACCENT,
        _ => theme::MUTED,
    }
}

fn learn_row(
    ctx: &Ctx,
    suggestion: &StyleSuggestion,
    expanded: State<Option<u16>>,
    mut hovered: State<Option<crate::app::HoverKey>>,
) -> Element {
    let key = crate::app::HoverKey::Orb(suggestion.style_id.into(), 0);
    let is_hovered = *hovered.read() == Some(key);
    let learn = {
        let ctx = ctx.clone();
        let style_id = suggestion.style_id;
        move |_| ctx.run(|session| session.add_style(style_id))
    };
    let subtitle = style_subtitle(suggestion.style_id, &suggestion.label)
        .unwrap_or_else(|| format!("#{}", suggestion.style_id));
    let is_expanded = *expanded.read() == Some(suggestion.style_id);

    rect()
        .vertical()
        .width(Size::fill())
        .key(suggestion.style_id)
        .background(if is_hovered {
            theme::SURFACE
        } else {
            theme::PANEL
        })
        .corner_radius(6.)
        .on_pointer_enter(move |_| *hovered.write() = Some(key))
        .on_pointer_leave(move |_| {
            if *hovered.read() == Some(key) {
                *hovered.write() = None;
            }
        })
        .child(
            rect()
                .horizontal()
                .width(Size::fill())
                .spacing(10.)
                .content(Content::Flex)
                .padding(Gaps::new_symmetric(3., 4.))
                .child(widgets::expanding_name(
                    suggestion.style_id,
                    suggestion.name.clone(),
                    subtitle,
                    expanded,
                    12.,
                ))
                .child(widgets::badge(
                    suggestion.kind,
                    theme::SURFACE_ACTIVE,
                    style_badge_color(suggestion.kind),
                ))
                .child(
                    Button::new()
                        .compact()
                        .outline()
                        .child("Learn")
                        .on_press(learn),
                ),
        )
        .maybe_child(if is_expanded {
            widgets::details(suggestion.description.as_deref())
        } else {
            None
        })
        .into_element()
}
