//! The styles tab: the fighting styles the player knows.

use freya::prelude::*;

use crate::app::Ctx;
use std::cell::RefCell;
use std::rc::Rc;

use crate::app::HoverKey;
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
    matching: usize,
    expanded: State<Option<u16>>,
    hovered: State<Option<crate::app::HoverKey>>,
    filter_id: AccessibilityId,
    show_more: impl FnMut(Event<PressEventData>) + 'static,
) -> Element {
    let mut rows: Vec<Element> = Vec::new();
    for style in styles {
        rows.push(style_row(
            ctx, style, expanded, hovered, game_ready, filter_id,
        ));
    }
    if rows.is_empty() {
        rows.push(match ctx.session.borrow().section_error(Section::Styles) {
            Some(error) => widgets::hint(format!(
                "The styles section could not be read: {}",
                error.message
            )),
            None => widgets::hint("The player has not learned any fighting styles."),
        });
    }

    let mut children = vec![
        rect()
            .vertical()
            .width(Size::fill())
            .spacing(theme::space::XS)
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
        children.push(widgets::filter_input(
            query,
            "Filter the style list, or leave empty to browse it",
            "Clear the style filter",
            filter_id,
        ));
        if matches.is_empty() {
            children.push(widgets::hint(if query.read().trim().is_empty() {
                "Every style of the game is already known."
            } else {
                "No styles match this search."
            }));
        } else {
            let browsing = query.read().trim().is_empty();
            let shown = matches.len();
            let heading = rect()
                .horizontal()
                .width(Size::fill())
                .spacing(theme::space::LG)
                .content(Content::Flex)
                .child(widgets::section_label(if browsing {
                    "ALL STYLES"
                } else {
                    "SEARCH RESULTS"
                }))
                .child(rect().width(Size::flex(1.0)))
                .child(
                    widgets::a11y_text(match (browsing, shown == matching) {
                        (true, _) => format!("{shown} of {matching}"),
                        (false, true) => crate::widgets::plural(matching, "match", "matches"),
                        (false, false) => format!(
                            "{shown} of {}",
                            crate::widgets::plural(matching, "match", "matches")
                        ),
                    })
                    .font_size(theme::font::MICRO)
                    .color(theme::DIM),
                )
                .maybe(shown < matching, |el| {
                    el.child(
                        Button::new()
                            .compact()
                            .outline()
                            .child("Show more")
                            .on_press(show_more),
                    )
                });
            children.push(heading.into_element());
            let mut result_rows: Vec<Element> = Vec::new();
            for suggestion in matches {
                result_rows.push(learn_row(ctx, suggestion, expanded, hovered));
            }
            children.push(
                rect()
                    .vertical()
                    .width(Size::fill())
                    .spacing(theme::space::XXS)
                    .children(result_rows)
                    .into_element(),
            );
        }
    } else if game_loading {
        children.push(widgets::hint(
            "Reading the game data; learned styles can already be inspected and removed.",
        ));
    } else if names_ready {
        children.push(widgets::hint(
            "Bundled English names are loaded, but new styles and upgrade details still need \
             the game data. Levels, weapons and time spent come from the savegame.",
        ));
    } else {
        children.push(widgets::hint(
            "The game data was not found, so style names and new styles are unavailable. \
             Levels, weapons and time spent come from the savegame.",
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
    game_ready: bool,
    filter_id: AccessibilityId,
) -> Element {
    let key = crate::app::HoverKey::Style(style.struct_index);
    let is_hovered = *hovered.read() == Some(key);
    let index = style.struct_index;
    let level = style.level;
    let remove = {
        let ctx = ctx.clone();
        let name = style.name.clone();
        Action::new(move || {
            if !crate::dialog::confirm(
                "Remove style?",
                &format!(
                    "Remove {name} from the fighting styles? You can still press Revert before saving."
                ),
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
        .a11y_alt(format!("{} style", style.name))
        .background(if is_hovered {
            theme::SURFACE_HOVER
        } else {
            theme::SURFACE
        })
        .corner_radius(theme::radius::MEDIUM)
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
                .spacing(theme::space::LG)
                .content(Content::Flex)
                .padding(Gaps::new_symmetric(theme::space::XS, theme::space::SM))
                .child(widgets::expanding_name(
                    style.style_id,
                    style.name.clone(),
                    subtitle,
                    expanded,
                    theme::font::BODY,
                ))
                .child(widgets::badge(
                    style.kind,
                    theme::SURFACE_ACTIVE,
                    style_badge_color(style.kind),
                ))
                .child(
                    widgets::a11y_text(format!("Lv {level}"))
                        .font_size(theme::font::LABEL)
                        .width(Size::px(theme::layout::STEPPER_WIDTH))
                        .text_align(TextAlign::Center)
                        .color(theme::ACCENT),
                )
                .child(widgets::row_action(
                    format!("Remove {} style", style.name),
                    "Remove",
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
            .spacing(theme::space::XS)
            .padding(Gaps::new(
                theme::space::NONE,
                theme::space::LG,
                theme::space::SM,
                theme::space::LG,
            ))
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
    rows.push(widgets::hint(
        "Filled gold orbs are owned; an empty ring is available. Clicking tier N buys every          missing tier up to N; clicking an owned tier removes it and everything above it.",
    ));
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
        .spacing(theme::space::SM)
        .padding(Gaps::new(
            theme::space::NONE,
            theme::space::LG,
            theme::space::MD,
            theme::space::LG,
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
        .spacing(theme::space::SM)
        .content(Content::Flex)
        .child(
            widgets::a11y_text(family_title(family))
                .font_size(theme::font::CAPTION)
                .width(Size::px(theme::layout::UPGRADE_LABEL_WIDTH))
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
                theme::ORB_HOVER
            } else {
                theme::ORB
            }
        } else if is_hovered || focused {
            theme::SURFACE_ACTIVE
        } else {
            theme::BACKGROUND
        };
        let border = if focused {
            theme::TEXT
        } else if taken || is_hovered {
            theme::ORB_HOVER
        } else {
            theme::ORB
        };
        let border_width = if focused {
            theme::stroke::EMPHASIS
        } else {
            theme::stroke::FOCUS
        };
        let mut hovered = self.hovered;
        let session = self.session.clone();
        let mut revision = self.revision;
        let struct_index = self.struct_index;
        let slot = self.upgrade.slot;
        rect()
            .center()
            .width(Size::px(theme::layout::ORB))
            .height(Size::px(theme::layout::ORB))
            .corner_radius(theme::radius::PILL)
            .background(background)
            .border(Border::new().fill(border).width(border_width))
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
        "This style has no level-up table in the game data, so its upgrades are read-only here."
    } else {
        "Upgrade details need the game data: point JadeForge at the game installation to see them."
    })
}

/// Full accessible description of one upgrade orb.
fn orb_accessible_name(style_name: &str, upgrade: &StyleUpgrade) -> String {
    let effect = if upgrade.effect.is_empty() {
        String::new()
    } else {
        format!(", {}", upgrade.effect)
    };
    if upgrade.taken {
        format!(
            "{style_name}, {}, tier {}, {}, owned: removes this tier and every tier above it",
            family_title(&upgrade.family),
            upgrade.tier,
            upgrade.name
        )
    } else {
        format!(
            "{style_name}, {}, tier {}, {}{effect}, buys every missing tier up to this one",
            family_title(&upgrade.family),
            upgrade.tier,
            upgrade.name
        )
    }
}

/// The detail line shown under a family while an orb is hovered or focused.
fn orb_detail(style: &StyleRow, upgrade: &StyleUpgrade) -> Element {
    let action = if upgrade.taken {
        "owned: clicking removes this tier and every tier above it"
    } else {
        "available: clicking buys every missing tier up to this one"
    };
    let effect = if upgrade.effect.is_empty() {
        "no listed effect".to_owned()
    } else {
        upgrade.effect.clone()
    };
    let cost = if upgrade.cost == 0 {
        "no style point cost".to_owned()
    } else {
        widgets::plural(usize::from(upgrade.cost), "style point", "style points")
    };
    rect()
        .horizontal()
        .width(Size::fill())
        .key((style.struct_index, upgrade.slot))
        .spacing(theme::space::MD)
        .content(Content::wrap_spacing(theme::space::MD))
        .padding(Gaps::new_symmetric(theme::space::XS, theme::space::NONE))
        .child(
            widgets::a11y_text(format!(
                "{} tier {}: {}",
                family_title(&upgrade.family),
                upgrade.tier,
                upgrade.name
            ))
            .font_size(theme::font::CAPTION)
            .color(theme::TEXT),
        )
        .child(
            widgets::a11y_text(format!("{effect} · {cost} · {action}"))
                .font_size(theme::font::MICRO)
                .color(theme::MUTED),
        )
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
        parts.push(format!("icon {icon}"));
    }
    if style.weapon_count > 0 {
        parts.push(widgets::plural(
            usize::from(style.weapon_count),
            "weapon",
            "weapons",
        ));
    }
    if style.time_spent >= 60 {
        parts.push(format!("{} used", style_time(style.time_spent)));
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
        parts.push(format!("icon {icon}"));
    }
    if let Some(weapon) = &style.weapon {
        parts.push(format!("weapon {weapon}"));
    }
    if style.weapon_count > 0 {
        parts.push(widgets::plural(
            usize::from(style.weapon_count),
            "weapon",
            "weapons",
        ));
    }
    if style.time_spent >= 1 {
        parts.push(format!("{} used", style_time(style.time_spent)));
    }
    (!parts.is_empty()).then(|| format!("Saved style data: {}.", parts.join(" · ")))
}

/// Formats a duration from the savegame's `TimeSpentInStyle` seconds.
fn style_time(seconds: u32) -> String {
    crate::session::format::elapsed(seconds)
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
        Action::new(move || ctx.run(|session| session.add_style(style_id)))
    };
    let subtitle = style_subtitle(suggestion.style_id, &suggestion.label)
        .unwrap_or_else(|| format!("#{}", suggestion.style_id));
    let is_expanded = *expanded.read() == Some(suggestion.style_id);

    rect()
        .vertical()
        .width(Size::fill())
        .key(suggestion.style_id)
        .a11y_role(AccessibilityRole::Group)
        .a11y_alt(format!("{} style", suggestion.name))
        .background(if is_hovered {
            theme::SURFACE
        } else {
            theme::PANEL
        })
        .corner_radius(theme::radius::CONTROL)
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
                .spacing(theme::space::LG)
                .content(Content::Flex)
                .padding(Gaps::new_symmetric(theme::space::XS, theme::space::XS))
                .child(widgets::expanding_name(
                    suggestion.style_id,
                    suggestion.name.clone(),
                    subtitle,
                    expanded,
                    theme::font::LABEL,
                ))
                .child(widgets::badge(
                    suggestion.kind,
                    theme::SURFACE_ACTIVE,
                    style_badge_color(suggestion.kind),
                ))
                .child(widgets::row_action(
                    format!("Learn {} style", suggestion.name),
                    "Learn",
                    learn,
                )),
        )
        .maybe_child(if is_expanded {
            widgets::details(suggestion.description.as_deref())
        } else {
            None
        })
        .into_element()
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
