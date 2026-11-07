//! The overview tab: the savegame at a glance.

use freya::prelude::*;

use crate::app::Ctx;
use crate::i18n::{fill, keys, t};
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
        || t(keys::SHELL_NO_SAVEGAME_SELECTED).to_owned(),
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
            GameStatus::Loading => (t(keys::OVERVIEW_GAME_DATA_LOADING).to_owned(), false),
            GameStatus::Loaded {
                items,
                styles,
                root,
            } => (
                fill(
                    keys::OVERVIEW_GAME_DATA_RESOLVED,
                    &[
                        (
                            "items",
                            &crate::i18n::plural(
                                *items,
                                keys::COUNT_ITEM_ONE,
                                keys::COUNT_ITEM_OTHER,
                            ),
                        ),
                        (
                            "styles",
                            &crate::i18n::plural(
                                *styles,
                                keys::COUNT_STYLE_ONE,
                                keys::COUNT_STYLE_OTHER,
                            ),
                        ),
                        ("root", &root.display().to_string()),
                    ],
                ),
                true,
            ),
            GameStatus::NamesOnly { items, styles } => (
                fill(
                    keys::OVERVIEW_GAME_DATA_BUNDLED,
                    &[
                        (
                            "items",
                            &crate::i18n::plural(
                                *items,
                                keys::COUNT_ITEM_ONE,
                                keys::COUNT_ITEM_OTHER,
                            ),
                        ),
                        (
                            "styles",
                            &crate::i18n::plural(
                                *styles,
                                keys::COUNT_STYLE_ONE,
                                keys::COUNT_STYLE_OTHER,
                            ),
                        ),
                    ],
                ),
                false,
            ),
            GameStatus::Missing => (t(keys::OVERVIEW_GAME_DATA_MISSING).to_owned(), false),
            GameStatus::Error(error) => {
                let bundled = if session.has_bundled_names() {
                    t(keys::OVERVIEW_GAME_DATA_BUNDLED_SUFFIX)
                } else {
                    "."
                };
                (
                    fill(
                        keys::OVERVIEW_GAME_DATA_UNREADABLE,
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
                    format!("{} {}", t(keys::CHARACTER_LEVEL), character.level),
                    theme::accent(),
                    theme::on_accent(),
                ))
                .child(widgets::badge(
                    format!("{} {}", t(keys::CHARACTER_CHAPTER), character.chapter),
                    theme::surface_active(),
                    theme::text(),
                ))
                .child(widgets::badge(alignment, theme::surface(), theme::jade()))
                .into_element(),
        )
        .child(
            widgets::a11y_text(fill(
                keys::OVERVIEW_TIME_PLAYED,
                &[
                    ("time", &format::duration(character.time_played)),
                    (
                        "kills",
                        &crate::i18n::plural(
                            usize::try_from(character.kills).unwrap_or(usize::MAX),
                            keys::COUNT_KILL_ONE,
                            keys::COUNT_KILL_OTHER,
                        ),
                    ),
                    (
                        "experience",
                        &crate::i18n::plural(
                            usize::try_from(character.experience).unwrap_or(usize::MAX),
                            keys::COUNT_EXPERIENCEPOINT_ONE,
                            keys::COUNT_EXPERIENCEPOINT_OTHER,
                        ),
                    ),
                ],
            ))
            .font_size(theme::font::label())
            .color(theme::muted()),
        )
        .child(widgets::stat_grid(vec![
            widgets::stat(
                t(keys::FIELD_GOLD),
                character
                    .gold
                    .map_or_else(|| t(keys::CHARACTER_NOT_SET).to_owned(), format::amount),
            ),
            widgets::stat(
                t(keys::FIELD_HEALTH),
                format!(
                    "{} / {}",
                    format::number(character.health),
                    character.health_max
                ),
            ),
            widgets::stat(
                t(keys::FIELD_CHI),
                format!("{} / {}", format::number(character.chi), character.chi_max),
            ),
            widgets::stat(
                t(keys::FIELD_FOCUS),
                format!(
                    "{} / {}",
                    format::number(character.focus),
                    format::number(character.focus_max)
                ),
            ),
        ]))
        .into_element();

    let quick = widgets::card(
        t(keys::OVERVIEW_AT_A_GLANCE),
        None,
        vec![widgets::definition_list(vec![
            (
                capitalize(t(keys::LABEL_INVENTORY)),
                widgets::a11y_text(widgets::plural(
                    item_count,
                    keys::COUNT_STACK_ONE,
                    keys::COUNT_STACK_OTHER,
                ))
                .font_size(theme::font::body())
                .color(theme::text())
                .into_element(),
            ),
            (
                capitalize(t(keys::COUNT_STYLE_OTHER)),
                widgets::a11y_text(widgets::plural(
                    style_count,
                    keys::COUNT_STYLE_LEARNED_ONE,
                    keys::COUNT_STYLE_LEARNED_OTHER,
                ))
                .font_size(theme::font::body())
                .color(theme::text())
                .into_element(),
            ),
            (
                t(keys::FIELD_ATTRIBUTE_POINTS).to_owned(),
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
                crate::i18n::join_list(
                    &[keys::FIELD_BODY, keys::FIELD_MIND, keys::FIELD_SPIRIT],
                    keys::COMMON_LIST_SEPARATOR,
                ),
                widgets::a11y_text(format!(
                    "{} / {} / {}",
                    character.body, character.mind, character.spirit
                ))
                .font_size(theme::font::body())
                .color(theme::text())
                .into_element(),
            ),
            (
                crate::i18n::join_list(
                    &[
                        keys::FIELD_CHARM,
                        keys::FIELD_INTUITION,
                        keys::FIELD_INTIMIDATE,
                    ],
                    keys::COMMON_LIST_SEPARATOR,
                ),
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
            t(keys::OVERVIEW_FILE),
            save_folder.clone().map_or_else(
                || widgets::a11y_text("").into_element(),
                |folder| path_actions(ctx, &path, folder, keys::COUNT_SAVEGAME_ONE),
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
                capitalize(t(keys::COUNT_BACKUP_ONE)),
                path_actions(
                    ctx,
                    &backup.display().to_string(),
                    folder.clone(),
                    keys::COUNT_BACKUP_ONE,
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
            .child(widgets::hint(t(keys::OVERVIEW_BACKUP_EXPLANATION)))
            .into_element(),
        _ => widgets::hint(t(keys::OVERVIEW_BACKUP_CREATED)),
    };

    let details = widgets::card(
        capitalize(t(keys::COUNT_SAVEGAME_ONE)),
        None,
        vec![
            path_row,
            widgets::divider(),
            backup_row,
            widgets::divider(),
            widgets::label_row(
                t(keys::OVERVIEW_CONTENTS),
                widgets::a11y_text(format!(
                    "{} · {}",
                    crate::i18n::plural(
                        block_count,
                        keys::COUNT_BLOCK_ONE,
                        keys::COUNT_BLOCK_OTHER
                    ),
                    file_size
                        .map_or_else(|| t(keys::OVERVIEW_UNKNOWN_SIZE).to_owned(), format::bytes)
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
                        t(keys::OVERVIEW_GAME_DATA)
                    } else {
                        t(keys::OVERVIEW_NOTICE)
                    })
                    .font_size(theme::font::label())
                    .color(theme::muted()),
                )
                .child(widgets::badge(
                    if game_ok {
                        t(keys::OVERVIEW_LINKED)
                    } else if game_loading {
                        t(keys::OVERVIEW_LOADING)
                    } else {
                        t(keys::OVERVIEW_NOT_FOUND)
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
            widgets::hint(t(keys::OVERVIEW_BACKUP_VERIFICATION)),
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
