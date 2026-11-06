//! The overview tab: the savegame at a glance.

use freya::prelude::*;

use crate::app::Ctx;
use crate::i18n::{fill, t};
use crate::message::capitalize;
use crate::session::{GameStatus, format};
use crate::theme;
use crate::views::path_actions;
use crate::widgets;

/// Renders the overview tab.
#[allow(clippy::too_many_lines)] // one declarative tree keeps the visual order auditable
pub fn view(
    ctx: &Ctx,
    character: &jadeforge_core::Character,
    item_count: usize,
    style_count: usize,
) -> Element {
    let path = ctx.session.borrow().current_path().map_or_else(
        || t("shell.no_savegame_selected").to_owned(),
        |path| path.display().to_string(),
    );
    let block_count = ctx.session.borrow().block_count();
    let file_size = ctx.session.borrow().file_size();
    let save_folder = Some(ctx.session.borrow().directory().to_path_buf());
    let backup = ctx
        .session
        .borrow()
        .last_backup()
        .map(std::path::Path::to_path_buf);
    let backup_folder = backup
        .as_ref()
        .and_then(|backup| backup.parent().map(std::path::Path::to_path_buf));
    let game_loading = matches!(ctx.session.borrow().game_status(), GameStatus::Loading);
    let (game_text, game_ok) = {
        let session = ctx.session.borrow();
        match session.game_status() {
            GameStatus::Loading => (t("overview.game_data.loading").to_owned(), false),
            GameStatus::Loaded {
                items,
                styles,
                root,
            } => (
                fill(
                    "overview.game_data.resolved",
                    &[
                        (
                            "items",
                            &crate::i18n::plural(*items, "count.item.one", "count.item.other"),
                        ),
                        (
                            "styles",
                            &crate::i18n::plural(*styles, "count.style.one", "count.style.other"),
                        ),
                        ("root", &root.display().to_string()),
                    ],
                ),
                true,
            ),
            GameStatus::NamesOnly { items, styles } => (
                fill(
                    "overview.game_data.bundled",
                    &[
                        (
                            "items",
                            &crate::i18n::plural(*items, "count.item.one", "count.item.other"),
                        ),
                        (
                            "styles",
                            &crate::i18n::plural(*styles, "count.style.one", "count.style.other"),
                        ),
                    ],
                ),
                false,
            ),
            GameStatus::Missing => (t("overview.game_data.missing").to_owned(), false),
            GameStatus::Error(error) => {
                let bundled = if session.has_bundled_names() {
                    t("overview.game_data.bundled_suffix")
                } else {
                    "."
                };
                (
                    fill(
                        "overview.game_data.unreadable",
                        &[("error", error.as_str()), ("bundled", bundled)],
                    ),
                    false,
                )
            }
        }
    };

    let alignment = t(format::alignment(character.alignment));

    let hero = rect()
        .vertical()
        .width(Size::fill())
        .spacing(theme::space::md())
        .padding(Gaps::new_all(theme::space::card()))
        .background(theme::panel())
        .corner_radius(theme::radius::panel())
        .child(
            widgets::a11y_text(character.name.clone())
                .a11y_builder(|node| node.set_role(AccessibilityRole::Heading))
                .font_size(theme::font::display())
                .font_weight(FontWeight::BOLD)
                .max_lines(1)
                .color(theme::text()),
        )
        .child(
            rect()
                .horizontal()
                .spacing(theme::space::md())
                .content(Content::wrap_spacing(theme::space::md()))
                .child(widgets::badge(
                    format!("{} {}", t("character.level"), character.level),
                    theme::accent(),
                    theme::on_accent(),
                ))
                .child(widgets::badge(
                    format!("{} {}", t("character.chapter"), character.chapter),
                    theme::surface_active(),
                    theme::text(),
                ))
                .child(widgets::badge(alignment, theme::surface(), theme::jade()))
                .into_element(),
        )
        .child(
            widgets::a11y_text(fill(
                "overview.time_played",
                &[
                    ("time", &format::duration(character.time_played)),
                    (
                        "kills",
                        &crate::i18n::plural(
                            usize::try_from(character.kills).unwrap_or(usize::MAX),
                            "count.kill.one",
                            "count.kill.other",
                        ),
                    ),
                    (
                        "experience",
                        &crate::i18n::plural(
                            usize::try_from(character.experience).unwrap_or(usize::MAX),
                            "count.experiencepoint.one",
                            "count.experiencepoint.other",
                        ),
                    ),
                ],
            ))
            .font_size(theme::font::label())
            .color(theme::muted()),
        )
        .child(widgets::stat_grid(vec![
            widgets::stat(
                t("field.gold"),
                character
                    .gold
                    .map_or_else(|| t("character.not_set").to_owned(), format::amount),
            ),
            widgets::stat(
                t("field.health"),
                format!(
                    "{} / {}",
                    format::number(character.health),
                    character.health_max
                ),
            ),
            widgets::stat(
                t("field.chi"),
                format!("{} / {}", format::number(character.chi), character.chi_max),
            ),
            widgets::stat(
                t("field.focus"),
                format!(
                    "{} / {}",
                    format::number(character.focus),
                    format::number(character.focus_max)
                ),
            ),
        ]))
        .into_element();

    let quick = widgets::card(
        t("overview.at_a_glance"),
        None,
        vec![widgets::definition_list(vec![
            (
                capitalize(t("label.inventory")),
                widgets::a11y_text(widgets::plural(
                    item_count,
                    "count.stack.one",
                    "count.stack.other",
                ))
                .font_size(theme::font::body())
                .color(theme::text())
                .into_element(),
            ),
            (
                capitalize(t("count.style.other")),
                widgets::a11y_text(widgets::plural(
                    style_count,
                    "count.style.learned.one",
                    "count.style.learned.other",
                ))
                .font_size(theme::font::body())
                .color(theme::text())
                .into_element(),
            ),
            (
                t("field.attribute_points").to_owned(),
                widgets::a11y_text(character.attribute_points.to_string())
                    .font_size(theme::font::body())
                    .color(if character.attribute_points > 0 {
                        theme::accent()
                    } else {
                        theme::muted()
                    })
                    .into_element(),
            ),
            (
                t("overview.body_mind_spirit").to_owned(),
                widgets::a11y_text(format!(
                    "{} / {} / {}",
                    character.body, character.mind, character.spirit
                ))
                .font_size(theme::font::body())
                .color(theme::text())
                .into_element(),
            ),
            (
                t("overview.charm_intuition_intimidate").to_owned(),
                widgets::a11y_text(format!(
                    "{} / {} / {}",
                    character.charm, character.intuition, character.intimidate
                ))
                .font_size(theme::font::body())
                .color(theme::text())
                .into_element(),
            ),
        ])],
    );

    let path_row = rect()
        .vertical()
        .width(Size::fill())
        .spacing(theme::space::xs())
        .child(widgets::label_row(
            t("overview.file"),
            save_folder.clone().map_or_else(
                || widgets::a11y_text("").into_element(),
                |folder| path_actions(ctx, &path, folder, "count.savegame.one"),
            ),
        ))
        .child(
            // Long savegame paths wrap instead of running off the card, and
            // stay copyable through the actions above.
            paragraph()
                .width(Size::fill())
                .max_width(Size::px(theme::layout::text_measure()))
                .font_size(theme::font::label())
                .color(theme::muted())
                .a11y_builder({
                    let path = path.clone();
                    move |node| node.set_value(path)
                })
                .span(path.clone()),
        )
        .into_element();

    let backup_row = match (&backup, &backup_folder) {
        (Some(backup), Some(folder)) => rect()
            .vertical()
            .width(Size::fill())
            .spacing(theme::space::xs())
            .child(widgets::label_row(
                capitalize(t("count.backup.one")),
                path_actions(
                    ctx,
                    &backup.display().to_string(),
                    folder.clone(),
                    "count.backup.one",
                ),
            ))
            .child(
                paragraph()
                    .width(Size::fill())
                    .max_width(Size::px(theme::layout::text_measure()))
                    .font_size(theme::font::label())
                    .color(theme::muted())
                    .a11y_builder({
                        let backup = backup.display().to_string();
                        move |node| node.set_value(backup)
                    })
                    .span(backup.display().to_string()),
            )
            .child(widgets::hint(t("overview.backup.explanation")))
            .into_element(),
        _ => widgets::hint(t("overview.backup.created")),
    };

    let details = widgets::card(
        capitalize(t("count.savegame.one")),
        None,
        vec![
            path_row,
            widgets::divider(),
            backup_row,
            widgets::divider(),
            widgets::label_row(
                t("overview.contents"),
                widgets::a11y_text(format!(
                    "{} · {}",
                    crate::i18n::plural(block_count, "count.block.one", "count.block.other"),
                    file_size.map_or_else(|| t("overview.unknown_size").to_owned(), format::bytes)
                ))
                .font_size(theme::font::label())
                .color(theme::muted()),
            ),
            widgets::divider(),
            rect()
                .horizontal()
                .spacing(theme::space::md())
                .child(
                    widgets::a11y_text(if game_ok {
                        t("overview.game_data")
                    } else {
                        t("overview.notice")
                    })
                    .font_size(theme::font::label())
                    .color(theme::muted()),
                )
                .child(widgets::badge(
                    if game_ok {
                        t("overview.linked")
                    } else if game_loading {
                        t("overview.loading")
                    } else {
                        t("overview.not_found")
                    },
                    if game_ok {
                        theme::surface()
                    } else {
                        theme::surface_active()
                    },
                    if game_ok {
                        theme::jade()
                    } else if game_loading {
                        theme::dim()
                    } else {
                        theme::warning()
                    },
                ))
                .into_element(),
            widgets::hint(game_text),
            widgets::divider(),
            widgets::hint(t("overview.backup.verification")),
        ],
    );

    rect()
        .vertical()
        .width(Size::fill())
        .spacing(theme::space::xxl())
        .child(hero)
        .child(quick)
        .child(details)
        .into_element()
}
