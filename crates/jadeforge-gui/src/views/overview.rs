//! The overview tab: the savegame at a glance.

use freya::elements::image::ImageHandle;
use freya::prelude::*;

use crate::app::Ctx;
use crate::session::{GameStatus, format};
use crate::theme;
use crate::widgets;

/// Renders the overview tab.
#[allow(clippy::too_many_lines)] // one declarative layout is easier to follow
pub fn view(
    ctx: &Ctx,
    character: &jadeforge_core::Character,
    preview: Option<ImageHandle>,
    item_count: usize,
    style_count: usize,
) -> Element {
    let path = ctx
        .session
        .borrow()
        .current_path()
        .map_or_else(|| "—".to_owned(), |path| path.display().to_string());
    let block_count = ctx.session.borrow().block_count();
    let file_size = ctx.session.borrow().file_size();
    let (game_text, game_ok) = {
        let session = ctx.session.borrow();
        match session.game_status() {
            GameStatus::Loaded {
                items,
                styles,
                root,
            } => (
                format!(
                    "{items} items and {styles} styles resolved from {}",
                    root.display()
                ),
                true,
            ),
            GameStatus::Missing => (
                "No game installation found next to the savegames: names fall back to ids."
                    .to_owned(),
                false,
            ),
            GameStatus::Error(error) => (format!("Cannot read the game data: {error}"), false),
        }
    };

    let portrait = preview.map_or_else(
        || {
            rect()
                .width(Size::px(164.))
                .height(Size::px(164.))
                .center()
                .background(theme::SURFACE)
                .corner_radius(12.)
                .child(label().text("no\nimage").font_size(11.).color(theme::DIM))
                .into_element()
        },
        |handle| {
            image(handle)
                .width(Size::px(164.))
                .height(Size::px(164.))
                .corner_radius(12.)
                .sampling_mode(SamplingMode::Bilinear)
                .aspect_ratio(AspectRatio::Min)
                .image_cover(ImageCover::Center)
                .into_element()
        },
    );

    let alignment = format::alignment(character.alignment);

    let hero = rect()
        .horizontal()
        .width(Size::fill())
        .spacing(20.)
        .padding(Gaps::new_all(18.))
        .background(theme::PANEL)
        .corner_radius(14.)
        .child(portrait)
        .child(
            rect()
                .vertical()
                .spacing(8.)
                .child(
                    label()
                        .text(character.name.clone())
                        .font_size(26.)
                        .font_weight(FontWeight::BOLD)
                        .max_lines(1)
                        .color(theme::TEXT),
                )
                .child(
                    rect()
                        .horizontal()
                        .spacing(8.)
                        .child(widgets::badge(
                            format!("Level {}", character.level),
                            theme::ACCENT,
                            theme::ON_ACCENT,
                        ))
                        .child(widgets::badge(
                            format!("Chapter {}", character.chapter),
                            theme::SURFACE_ACTIVE,
                            theme::TEXT,
                        ))
                        .child(widgets::badge(alignment, theme::SURFACE, theme::JADE)),
                )
                .child(
                    label()
                        .text(format!(
                            "{} played · {} kills · {} experience",
                            format::duration(character.time_played),
                            character.kills,
                            format::amount(character.experience)
                        ))
                        .font_size(12.)
                        .color(theme::MUTED),
                )
                .child(
                    rect()
                        .horizontal()
                        .content(Content::wrap_spacing(26.))
                        .spacing(26.)
                        .child(widgets::stat(
                            "GOLD",
                            character
                                .gold
                                .map_or_else(|| "—".to_owned(), format::amount),
                        ))
                        .child(widgets::stat(
                            "HEALTH",
                            format!(
                                "{} / {}",
                                format::number(character.health),
                                character.health_max
                            ),
                        ))
                        .child(widgets::stat(
                            "CHI",
                            format!("{} / {}", format::number(character.chi), character.chi_max),
                        ))
                        .child(widgets::stat(
                            "FOCUS",
                            format!(
                                "{} / {}",
                                format::number(character.focus),
                                format::number(character.focus_max)
                            ),
                        )),
                ),
        )
        .into_element();

    let quick = widgets::card(
        "At a glance",
        None,
        vec![widgets::label_grid(vec![
            (
                "Inventory".to_owned(),
                label()
                    .text(widgets::plural(item_count, "stack", "stacks"))
                    .font_size(13.)
                    .color(theme::TEXT)
                    .into_element(),
            ),
            (
                "Styles".to_owned(),
                label()
                    .text(format!("{style_count} learned"))
                    .font_size(13.)
                    .color(theme::TEXT)
                    .into_element(),
            ),
            (
                "Unspent attribute points".to_owned(),
                label()
                    .text(character.attribute_points.to_string())
                    .font_size(13.)
                    .color(if character.attribute_points > 0 {
                        theme::ACCENT
                    } else {
                        theme::MUTED
                    })
                    .into_element(),
            ),
            (
                "Body / Mind / Spirit".to_owned(),
                label()
                    .text(format!(
                        "{} / {} / {}",
                        character.body, character.mind, character.spirit
                    ))
                    .font_size(13.)
                    .color(theme::TEXT)
                    .into_element(),
            ),
            (
                "Charm / Intuition / Intimidate".to_owned(),
                label()
                    .text(format!(
                        "{} / {} / {}",
                        character.charm, character.intuition, character.intimidate
                    ))
                    .font_size(13.)
                    .color(theme::TEXT)
                    .into_element(),
            ),
        ])],
    );

    let details = widgets::card(
        "Savegame",
        None,
        vec![
            widgets::label_row(
                "File",
                label().text(path).font_size(12.).color(theme::MUTED),
            ),
            widgets::divider(),
            widgets::label_row(
                "Contents",
                label()
                    .text(format!(
                        "{block_count} blocks · {}",
                        file_size.map_or_else(|| "unknown size".to_owned(), format::bytes)
                    ))
                    .font_size(12.)
                    .color(theme::MUTED),
            ),
            widgets::divider(),
            rect()
                .horizontal()
                .spacing(8.)
                .child(
                    label()
                        .text(if game_ok { "Game data" } else { "Notice" })
                        .font_size(12.)
                        .color(theme::MUTED),
                )
                .child(widgets::badge(
                    if game_ok { "linked" } else { "not found" },
                    if game_ok {
                        theme::SURFACE
                    } else {
                        theme::SURFACE_ACTIVE
                    },
                    if game_ok { theme::JADE } else { theme::WARNING },
                ))
                .into_element(),
            widgets::hint(game_text),
            widgets::divider(),
            widgets::hint(
                "JadeForge verifies every savegame by re-reading it before writing, \
                 writes it atomically and keeps a timestamped backup next to the file.",
            ),
        ],
    );

    rect()
        .vertical()
        .width(Size::fill())
        .spacing(16.)
        .child(hero)
        .child(quick)
        .child(details)
        .into_element()
}
