//! The overview tab: the savegame at a glance.

use freya::prelude::*;

use crate::app::Ctx;
use crate::i18n::t;
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
        || "No savegame".to_owned(),
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
            GameStatus::Loading => (
                "Reading the game installation and its tables…".to_owned(),
                false,
            ),
            GameStatus::Loaded {
                items,
                styles,
                root,
            } => (
                format!(
                    "{} and {} resolved from {}",
                    format::plural(*items, "item", "items"),
                    format::plural(*styles, "style", "styles"),
                    root.display()
                ),
                true,
            ),
            GameStatus::NamesOnly { items, styles } => (
                format!(
                    "No game installation found: using bundled English names for {} and {}. \
                     Descriptions and upgrades still need the game data.",
                    format::plural(*items, "item", "items"),
                    format::plural(*styles, "style", "styles")
                ),
                false,
            ),
            GameStatus::Missing => (
                "No game installation found next to the savegames: names fall back to ids, \
                 while categories, levels and timings still come from the savegame."
                    .to_owned(),
                false,
            ),
            GameStatus::Error(error) => {
                let bundled = if session.has_bundled_names() {
                    ". Bundled English names are still available."
                } else {
                    "."
                };
                (
                    format!("Cannot read the game data: {error}{bundled}"),
                    false,
                )
            }
        }
    };

    let alignment = format::alignment(character.alignment);

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
                    format!("Level {}", character.level),
                    theme::accent(),
                    theme::on_accent(),
                ))
                .child(widgets::badge(
                    format!("Chapter {}", character.chapter),
                    theme::surface_active(),
                    theme::text(),
                ))
                .child(widgets::badge(alignment, theme::surface(), theme::jade()))
                .into_element(),
        )
        .child(
            widgets::a11y_text(format!(
                "Time played {} · {} · {}",
                format::duration(character.time_played),
                format::plural(
                    usize::try_from(character.kills).unwrap_or(usize::MAX),
                    "kill",
                    "kills"
                ),
                format::plural(
                    usize::try_from(character.experience).unwrap_or(usize::MAX),
                    "experience point",
                    "experience points"
                )
            ))
            .font_size(theme::font::label())
            .color(theme::muted()),
        )
        .child(widgets::stat_grid(vec![
            widgets::stat(
                t("GOLD"),
                character
                    .gold
                    .map_or_else(|| "Not set".to_owned(), format::amount),
            ),
            widgets::stat(
                t("HEALTH"),
                format!(
                    "{} / {}",
                    format::number(character.health),
                    character.health_max
                ),
            ),
            widgets::stat(
                t("CHI"),
                format!("{} / {}", format::number(character.chi), character.chi_max),
            ),
            widgets::stat(
                t("FOCUS"),
                format!(
                    "{} / {}",
                    format::number(character.focus),
                    format::number(character.focus_max)
                ),
            ),
        ]))
        .into_element();

    let quick = widgets::card(
        t("At a glance"),
        None,
        vec![widgets::definition_list(vec![
            (
                "Inventory".to_owned(),
                widgets::a11y_text(widgets::plural(item_count, "stack", "stacks"))
                    .font_size(theme::font::body())
                    .color(theme::text())
                    .into_element(),
            ),
            (
                "Styles".to_owned(),
                widgets::a11y_text(widgets::plural(
                    style_count,
                    "style learned",
                    "styles learned",
                ))
                .font_size(theme::font::body())
                .color(theme::text())
                .into_element(),
            ),
            (
                "Unspent attribute points".to_owned(),
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
                "Body / Mind / Spirit".to_owned(),
                widgets::a11y_text(format!(
                    "{} / {} / {}",
                    character.body, character.mind, character.spirit
                ))
                .font_size(theme::font::body())
                .color(theme::text())
                .into_element(),
            ),
            (
                "Charm / Intuition / Intimidate".to_owned(),
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
            t("File"),
            save_folder.clone().map_or_else(
                || widgets::a11y_text(t("")).into_element(),
                |folder| path_actions(ctx, &path, folder, "savegame"),
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
                t("Backup"),
                path_actions(ctx, &backup.display().to_string(), folder.clone(), "backup"),
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
            .child(widgets::hint(t(
                "This is the file as it was before the last save. To restore it, close JadeForge \
                 and copy the backup over save.sac.",
            )))
            .into_element(),
        _ => widgets::hint(t(
            "A timestamped backup is created next to the savegame on every save and shown here.",
        )),
    };

    let details = widgets::card(
        t("Savegame"),
        None,
        vec![
            path_row,
            widgets::divider(),
            backup_row,
            widgets::divider(),
            widgets::label_row(
                t("Contents"),
                widgets::a11y_text(format!(
                    "{} · {}",
                    format::plural(block_count, "block", "blocks"),
                    file_size.map_or_else(|| "unknown size".to_owned(), format::bytes)
                ))
                .font_size(theme::font::label())
                .color(theme::muted()),
            ),
            widgets::divider(),
            rect()
                .horizontal()
                .spacing(theme::space::md())
                .child(
                    widgets::a11y_text(if game_ok { "Game data" } else { "Notice" })
                        .font_size(theme::font::label())
                        .color(theme::muted()),
                )
                .child(widgets::badge(
                    if game_ok {
                        "linked"
                    } else if game_loading {
                        "loading"
                    } else {
                        "not found"
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
            widgets::hint(t(
                "JadeForge verifies every savegame by re-reading it before writing, \
                 writes it atomically and keeps a timestamped backup next to the file.",
            )),
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
