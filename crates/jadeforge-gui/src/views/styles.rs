//! The styles tab: the fighting styles the player knows.

use freya::prelude::*;

use crate::app::Ctx;
use std::cell::RefCell;
use std::rc::Rc;

use crate::app::HoverKey;
use crate::i18n::{fill, t};
use crate::session::{Section, Session, StyleRow, StyleSuggestion, StyleUpgrade};
use crate::theme;
use crate::widgets::{self, Action};

/// Renders the styles tab.
#[allow(
    clippy::too_many_arguments, // one argument per reactive tab input; grouping would only rename them
    clippy::too_many_lines // one declarative tab tree keeps the visual order auditable
)]
pub fn view(
    ctx: &Ctx,
    styles: &[StyleRow],
    query: State<String>,
    matches: &[StyleSuggestion],
    game_ready: bool,
    game_loading: bool,
    names_ready: bool,
    learnable: Option<usize>,
    expanded: State<Option<u16>>,
    hovered: State<Option<crate::app::HoverKey>>,
    filter_id: AccessibilityId,
) -> Element {
    let mut rows: Vec<Element> = Vec::new();
    for style in styles {
        rows.push(style_row(
            ctx, style, expanded, hovered, game_ready, filter_id,
        ));
    }
    if rows.is_empty() {
        rows.push(match ctx.session.borrow().section_error(Section::Styles) {
            Some(error) => {
                widgets::hint(fill("styles.section_error", &[("error", &error.message)]))
            }
            None => widgets::hint(t("styles.empty_hint")),
        });
    }

    let mut children = vec![
        rect()
            .vertical()
            .width(Size::fill())
            .spacing(theme::space::xs())
            .children(rows)
            .into_element(),
        widgets::divider(),
        widgets::section_label(t("styles.learn_heading")),
        widgets::hint(t("styles.intro")),
    ];

    if game_ready {
        children.push(widgets::filter_input(
            query,
            crate::app::STYLE_FILTER_PLACEHOLDER,
            "styles.clear_filter",
            filter_id,
        ));
        if matches.is_empty() {
            children.push(widgets::hint(if query.read().trim().is_empty() {
                t("styles.all_learned")
            } else {
                t("styles.no_match")
            }));
        } else {
            let browsing = query.read().trim().is_empty();
            let total = matches.len();
            let heading = rect()
                .horizontal()
                .width(Size::fill())
                .spacing(theme::space::lg())
                .content(Content::Flex)
                .child(widgets::section_label(if browsing {
                    t("count.style.tolearn.other").to_uppercase()
                } else {
                    t("inventory.search_results").to_owned()
                }))
                .child(rect().width(Size::flex(1.0)))
                .child(
                    widgets::a11y_text(if browsing {
                        crate::widgets::plural(
                            total,
                            "count.style.tolearn.one",
                            "count.style.tolearn.other",
                        )
                    } else {
                        crate::widgets::plural(total, "count.match.one", "count.match.other")
                    })
                    .font_size(theme::font::micro())
                    .color(theme::dim()),
                );
            children.push(heading.into_element());
            let selected = *expanded.read();
            let mut picker_rows = Vec::with_capacity(matches.len());
            for suggestion in matches {
                let id = suggestion.style_id;
                let subtitle =
                    style_subtitle(id, &suggestion.label).unwrap_or_else(|| format!("#{id}"));
                let selectable = suggestion
                    .description
                    .as_deref()
                    .is_some_and(|text| !text.trim().is_empty());
                let select = selectable.then_some(expanded);
                picker_rows.push(widgets::PickerRow {
                    key: id,
                    title: suggestion.name.clone(),
                    subtitle: format!(
                        "{} · {}",
                        subtitle,
                        crate::session::style_kind_label(suggestion.kind)
                    ),
                    accessible: fill("styles.accessible", &[("name", &suggestion.name)]),
                    action_label: fill("styles.learn_action", &[("name", &suggestion.name)]),
                    action_text: t("styles.learn").to_owned(),
                    action: {
                        let ctx = ctx.clone();
                        Action::new(move || ctx.run(|session| session.add_style(id)))
                    },
                    select,
                    selected: selected == Some(id),
                });
            }
            children.push(widgets::picker_list(t("styles.learn_list"), picker_rows));
            if let Some(selected) = selected
                && let Some(suggestion) = matches.iter().find(|row| row.style_id == selected)
            {
                children.extend(widgets::details(suggestion.description.as_deref()));
            }
        }
    } else if game_loading {
        children.push(widgets::hint(t("styles.reading_hint")));
    } else if names_ready {
        children.push(widgets::hint(t("styles.bundled_hint")));
    } else {
        children.push(widgets::hint(t("styles.missing_hint")));
    }

    widgets::card(
        t("styles.fighting_styles"),
        Some(match learnable {
            Some(learnable) => format!(
                "{} · {}",
                crate::widgets::plural(
                    styles.len(),
                    "count.style.learned.one",
                    "count.style.learned.other"
                ),
                crate::widgets::plural(
                    learnable,
                    "count.style.tolearn.one",
                    "count.style.tolearn.other"
                )
            ),
            None => crate::widgets::plural(
                styles.len(),
                "count.style.learned.one",
                "count.style.learned.other",
            ),
        }),
        children,
    )
}

fn style_row(
    ctx: &Ctx,
    style: &StyleRow,
    expanded: State<Option<u16>>,
    mut hovered: State<Option<crate::app::HoverKey>>,
    game_ready: bool,
    filter_id: AccessibilityId,
) -> Element {
    let key = crate::app::HoverKey::Style(style.struct_index);
    let is_hovered = (*hovered.read()).is_some_and(|key| key.is_style_row(style.struct_index));
    let index = style.struct_index;
    let level = style.level;
    let remove = {
        let ctx = ctx.clone();
        let name = style.name.clone();
        Action::new(move || {
            if !crate::dialog::confirm(
                t("styles.remove.title"),
                &fill("styles.remove.confirm", &[("name", &name)]),
            ) {
                return;
            }
            ctx.run(|session| session.remove_style(index));
            filter_id.request_focus();
        })
    };

    let subtitle =
        style_subtitle(style.style_id, &style.label).unwrap_or_else(|| saved_style_subtitle(style));
    let is_expanded = *expanded.read() == Some(style.style_id);

    rect()
        .vertical()
        .width(Size::fill())
        .key(style.struct_index)
        .a11y_role(AccessibilityRole::Group)
        .a11y_alt(fill("styles.accessible", &[("name", &style.name)]))
        .background(if is_hovered {
            theme::surface_hover()
        } else {
            theme::surface()
        })
        .corner_radius(theme::radius::medium())
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
                .spacing(theme::space::lg())
                .content(Content::Flex)
                .padding(Gaps::new_symmetric(theme::space::xs(), theme::space::sm()))
                .child(widgets::expanding_name(
                    style.style_id,
                    style.name.clone(),
                    subtitle,
                    expanded,
                    theme::font::body(),
                ))
                .child(widgets::badge(
                    crate::session::style_kind_label(style.kind),
                    theme::surface_active(),
                    style_badge_color(style.kind),
                ))
                .child(
                    widgets::a11y_text(format!("{} {level}", t("character.level")))
                        .font_size(theme::font::label())
                        .width(Size::px(theme::layout::level_width()))
                        .text_align(TextAlign::Center)
                        .color(theme::accent()),
                )
                .child(widgets::row_action(
                    fill("styles.remove.action", &[("name", &style.name)]),
                    t("gems.remove"),
                    remove,
                )),
        )
        .maybe_child(if is_expanded {
            Some(upgrade_panel(ctx, style, hovered, game_ready))
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
    game_ready: bool,
) -> Element {
    let mut rows: Vec<Element> = Vec::new();
    if let Some(details) = widgets::details(style.description.as_deref()) {
        rows.push(details);
    }
    if let Some(fields) = saved_style_fields(style) {
        rows.push(widgets::hint(fields));
    }
    if style.upgrades.is_empty() {
        if let Some(notice) = upgrade_notice(game_ready) {
            rows.push(widgets::hint(notice));
        }
        return rect()
            .vertical()
            .width(Size::fill())
            .spacing(theme::space::xs())
            .padding(Gaps::new(
                theme::space::none(),
                theme::space::lg(),
                theme::space::sm(),
                theme::space::lg(),
            ))
            .children(rows)
            .into_element();
    }

    rows.push(widgets::section_label(t("styles.upgrades")));
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
        chips.push(upgrade_chip(
            ctx,
            style.struct_index,
            &style.name,
            upgrade,
            hovered,
        ));
    }
    if let Some(family) = family {
        rows.push(upgrade_row(family, chips));
    }
    rows.push(widgets::hint(t("styles.orb_hint")));
    // A hovered (or keyboard-focused) orb explains itself under the family:
    // name, effect, cost and what a click would do.
    if let Some(crate::app::HoverKey::Orb(struct_index, slot)) = *hovered.read()
        && struct_index == style.struct_index
        && let Some(upgrade) = style.upgrades.iter().find(|upgrade| upgrade.slot == slot)
    {
        rows.push(orb_detail(style, upgrade));
    }
    rect()
        .vertical()
        .width(Size::fill())
        .spacing(theme::space::sm())
        .padding(Gaps::new(
            theme::space::none(),
            theme::space::lg(),
            theme::space::md(),
            theme::space::lg(),
        ))
        .children(rows)
        .into_element()
}

/// One family line: its name and a chip per tier.
fn upgrade_row(family: &str, chips: Vec<Element>) -> Element {
    rect()
        .horizontal()
        .width(Size::fill())
        .key(family.to_owned())
        .spacing(theme::space::none())
        .content(Content::Flex)
        .child(
            widgets::a11y_text(family_title(family))
                .font_size(theme::font::caption())
                .width(Size::px(theme::layout::upgrade_label_width()))
                .color(theme::muted()),
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
    style_name: String,
    upgrade: StyleUpgrade,
    hovered: State<Option<HoverKey>>,
    key: DiffKey,
}

impl KeyExt for UpgradeOrb {
    fn key_mut(&mut self) -> &mut DiffKey {
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
                theme::orb_hover()
            } else {
                theme::orb()
            }
        } else if is_hovered || focused {
            theme::surface_active()
        } else {
            theme::background()
        };
        let border = if focused {
            theme::text()
        } else if taken || is_hovered {
            theme::orb_hover()
        } else {
            theme::orb()
        };
        let border_width = if focused {
            theme::stroke::emphasis()
        } else {
            theme::stroke::focus()
        };
        let mut hovered = self.hovered;
        let session = self.session.clone();
        let mut revision = self.revision;
        let struct_index = self.struct_index;
        let slot = self.upgrade.slot;
        rect()
            .center()
            // The hit target reaches halfway into the gap to each neighbour
            // (the row itself has no spacing), so sweeping from one orb to the
            // next never lands on the row background in between.
            .padding(Gaps::new_symmetric(
                theme::space::none(),
                theme::space::xs(),
            ))
            .a11y_id(a11y_id)
            .a11y_focusable(true)
            .a11y_role(AccessibilityRole::Button)
            .a11y_alt(orb_accessible_name(&self.style_name, &self.upgrade))
            .a11y_builder(move |node| {
                node.set_toggled(if taken {
                    accesskit::Toggled::True
                } else {
                    accesskit::Toggled::False
                });
            })
            .on_pointer_enter(move |_| {
                *hovered.write() = Some(key);
            })
            .on_pointer_leave(move |_| {
                if *hovered.read() == Some(key) {
                    *hovered.write() = None;
                }
            })
            .cursor(CursorIcon::Pointer)
            .on_press(move |_| {
                a11y_id.request_focus();
                session.0.borrow_mut().change_upgrade(struct_index, slot);
                *revision.write() += 1;
            })
            .child(
                rect()
                    .center()
                    .width(Size::px(theme::layout::orb()))
                    .height(Size::px(theme::layout::orb()))
                    .corner_radius(theme::radius::pill())
                    .background(background)
                    .border(Border::new().fill(border).width(border_width)),
            )
            .into_element()
    }
}

/// Renders one orb.
fn upgrade_chip(
    ctx: &Ctx,
    struct_index: usize,
    style_name: &str,
    upgrade: &StyleUpgrade,
    hovered: State<Option<HoverKey>>,
) -> Element {
    UpgradeOrb {
        session: SharedSession(ctx.session.clone()),
        revision: ctx.revision,
        struct_index,
        style_name: style_name.to_owned(),
        upgrade: upgrade.clone(),
        hovered,
        key: DiffKey::None,
    }
    .key((struct_index, upgrade.slot))
    .into_element()
}

/// Notice shown instead of the orb rows when a style has no upgrade slots.
///
/// A style without a `<Style>LV.2da` table is read-only by design; the notice
/// says so explicitly instead of blaming missing game data.
#[must_use]
pub fn upgrade_notice(game_ready: bool) -> Option<&'static str> {
    Some(if game_ready {
        t("styles.read_only_hint")
    } else {
        t("styles.upgrade_needs_data")
    })
}

/// Full accessible description of one upgrade orb.
fn orb_accessible_name(style_name: &str, upgrade: &StyleUpgrade) -> String {
    let effect = if upgrade.effect.is_empty() {
        String::new()
    } else {
        format!(", {}", upgrade.effect)
    };
    let family = family_title(&upgrade.family);
    let tier = upgrade.tier.to_string();
    if upgrade.taken {
        fill(
            "styles.orb.owned",
            &[
                ("style", style_name),
                ("family", &family),
                ("tier", &tier),
                ("name", &upgrade.name),
            ],
        )
    } else {
        fill(
            "styles.orb.available",
            &[
                ("style", style_name),
                ("family", &family),
                ("tier", &tier),
                ("name", &upgrade.name),
                ("effect", &effect),
            ],
        )
    }
}

/// The detail line shown under a family while an orb is hovered or focused.
fn orb_detail(style: &StyleRow, upgrade: &StyleUpgrade) -> Element {
    let action = if upgrade.taken {
        t("styles.owned_action")
    } else {
        t("styles.available_action")
    };
    let effect = if upgrade.effect.is_empty() {
        t("styles.no_effect").to_owned()
    } else {
        upgrade.effect.clone()
    };
    let cost = if upgrade.cost == 0 {
        t("styles.no_cost").to_owned()
    } else {
        widgets::plural(
            usize::from(upgrade.cost),
            "count.style.point.one",
            "count.style.point.other",
        )
    };
    rect()
        .vertical()
        .width(Size::fill())
        .key((style.struct_index, upgrade.slot))
        .spacing(theme::space::xxs())
        .padding(Gaps::new_symmetric(
            theme::space::xs(),
            theme::space::none(),
        ))
        .child(orb_detail_line(
            fill(
                "styles.orb.title",
                &[
                    ("family", &family_title(&upgrade.family)),
                    ("tier", &upgrade.tier.to_string()),
                    ("name", &upgrade.name),
                ],
            ),
            theme::font::caption(),
            theme::text(),
        ))
        .child(orb_detail_line(
            fill(
                "styles.orb.detail",
                &[("effect", &effect), ("cost", &cost), ("action", action)],
            ),
            theme::font::micro(),
            theme::muted(),
        ))
        .into_element()
}

/// One wrapping line of the hover detail.
///
/// The title and the effect/cost/action line each get the full width. As a
/// single wrapping row they competed for space, and the longer title squeezed
/// the second line into a one-word column on the right.
fn orb_detail_line(text: String, font_size: f32, color: Color) -> Element {
    let value = text.clone();
    paragraph()
        .width(Size::fill())
        .max_width(Size::px(theme::layout::text_measure()))
        .font_size(font_size)
        .color(color)
        .a11y_builder(move |node| node.set_value(value))
        .span(text)
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
/// `None` when the game data is missing; the private fallback then uses the
/// fields the savegame itself stores.
#[must_use]
pub fn style_subtitle(style_id: u16, label: &str) -> Option<String> {
    (!label.is_empty()).then(|| format!("{label} · #{style_id}"))
}

/// Secondary line built only from fields stored in the savegame.
///
/// The id replaces the missing name; the weapon count and time spent tell the
/// player which style it is without guessing a label.
fn saved_style_subtitle(style: &StyleRow) -> String {
    let mut parts = vec![format!("#{}", style.style_id)];
    if let Some(icon) = &style.icon {
        parts.push(fill("styles.icon", &[("icon", icon)]));
    }
    if style.weapon_count > 0 {
        parts.push(widgets::plural(
            usize::from(style.weapon_count),
            "count.weapon.one",
            "count.weapon.other",
        ));
    }
    if style.time_spent >= 60 {
        parts.push(fill(
            "styles.time_used",
            &[("time", &style_time(style.time_spent))],
        ));
    }
    parts.join(" · ")
}

/// Resource names and timings from the savegame, shown when no style label is
/// available. `None` once the game data can name the style.
fn saved_style_fields(style: &StyleRow) -> Option<String> {
    if !style.label.is_empty() {
        return None;
    }
    let mut parts = Vec::new();
    if let Some(icon) = &style.icon {
        parts.push(fill("styles.icon", &[("icon", icon)]));
    }
    if let Some(weapon) = &style.weapon {
        parts.push(fill("styles.weapon", &[("weapon", weapon)]));
    }
    if style.weapon_count > 0 {
        parts.push(widgets::plural(
            usize::from(style.weapon_count),
            "count.weapon.one",
            "count.weapon.other",
        ));
    }
    if style.time_spent >= 1 {
        parts.push(fill(
            "styles.time_used",
            &[("time", &style_time(style.time_spent))],
        ));
    }
    (!parts.is_empty()).then(|| fill("styles.saved_data", &[("fields", &parts.join(" · "))]))
}

/// Formats a duration from the savegame's `TimeSpentInStyle` seconds.
fn style_time(seconds: u32) -> String {
    crate::session::format::elapsed(seconds)
}

/// Badge colour for a style kind.
fn style_badge_color(kind: &str) -> Color {
    match kind {
        "magic" | "support" => theme::jade(),
        "transformation" | "unique" => theme::accent(),
        _ => theme::muted(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(label: &str, weapon_count: u8, time_spent: u32) -> StyleRow {
        StyleRow {
            struct_index: 0,
            style_id: 21,
            level: 13,
            name: "Style 21".to_owned(),
            label: label.to_owned(),
            kind: "style",
            description: None,
            icon: Some("I_TCUTS".to_owned()),
            weapon: Some("J00_COLLISION_01".to_owned()),
            weapon_count,
            time_spent,
            mask: 0,
            upgrades: Vec::new(),
        }
    }

    #[test]
    fn a_style_without_a_slot_table_says_it_is_read_only() {
        let notice = upgrade_notice(true).expect("a notice");
        assert!(notice.contains("read-only"), "{notice}");
        let missing = upgrade_notice(false).expect("a notice");
        assert!(missing.contains("game installation"), "{missing}");
    }

    #[test]
    fn missing_game_data_subtitle_uses_savegame_fields() {
        assert_eq!(
            saved_style_subtitle(&row("", 2, 3439)),
            "#21 · icon I_TCUTS · 2 weapons · 57m used"
        );
        assert_eq!(saved_style_subtitle(&row("", 0, 12)), "#21 · icon I_TCUTS");
        let mut without_icon = row("", 0, 12);
        without_icon.icon = None;
        assert_eq!(saved_style_subtitle(&without_icon), "#21");
    }

    #[test]
    fn saved_style_fields_are_only_shown_without_a_label() {
        let fields = saved_style_fields(&row("", 2, 3439)).expect("saved fields");
        assert_eq!(
            fields,
            "Saved style data: icon I_TCUTS · weapon J00_COLLISION_01 · 2 weapons · 57m used."
        );
        let short = saved_style_fields(&row("", 0, 12)).expect("short saved fields");
        assert!(short.contains("12s used"), "{short}");
        assert!(saved_style_fields(&row("Martial_Test", 2, 3439)).is_none());
    }
}
