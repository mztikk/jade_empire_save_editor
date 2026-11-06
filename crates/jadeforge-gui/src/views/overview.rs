//! The overview tab: the savegame at a glance.

use freya::elements::image::ImageHandle;
use freya::prelude::*;

use crate::app::Ctx;
use crate::session::{GameStatus, format};
use crate::theme;
use crate::widgets;

/// Renders the overview tab.
#[allow(clippy::too_many_lines)] // one declarative tree keeps the visual order auditable
pub fn view(
    ctx: &Ctx,
    character: &jadeforge_core::Character,
    preview: Option<ImageHandle>,
    item_count: usize,
    style_count: usize,
) -> Element {
    let path = ctx.session.borrow().current_path().map_or_else(
        || "No savegame".to_owned(),
        |path| path.display().to_string(),
    );
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
                "No game installation found next to the savegames: names fall back to ids, \
                 while categories, levels and timings still come from the savegame."
                    .to_owned(),
                false,
            ),
            GameStatus::Error(error) => (format!("Cannot read the game data: {error}"), false),
        }
    };

    let portrait = preview.map_or_else(
        || {
            rect()
                .width(Size::px(theme::layout::PORTRAIT))
                .height(Size::px(theme::layout::PORTRAIT))
                .center()
                .background(theme::SURFACE)
                .corner_radius(theme::radius::CARD)
                .child(
                    widgets::a11y_text("no\nimage")
                        .font_size(theme::font::CAPTION)
                        .color(theme::DIM),
                )
                .into_element()
        },
        |handle| {
            image(handle)
                .width(Size::px(theme::layout::PORTRAIT))
                .height(Size::px(theme::layout::PORTRAIT))
                .corner_radius(theme::radius::CARD)
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
        .spacing(theme::space::HERO)
        .padding(Gaps::new_all(theme::space::CARD))
        .background(theme::PANEL)
        .corner_radius(theme::radius::PANEL)
        .child(portrait)
        .child(
            rect()
                .vertical()
                .spacing(theme::space::MD)
                .child(
                    widgets::a11y_text(character.name.clone())
                        .a11y_builder(|node| node.set_role(AccessibilityRole::Heading))
                        .font_size(theme::font::DISPLAY)
                        .font_weight(FontWeight::BOLD)
                        .max_lines(1)
                        .color(theme::TEXT),
                )
                .child(
                    rect()
                        .horizontal()
                        .spacing(theme::space::MD)
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
                    widgets::a11y_text(format!(
                        "{} played · {} kills · {} experience",
                        format::duration(character.time_played),
                        character.kills,
                        format::amount(character.experience)
                    ))
                    .font_size(theme::font::LABEL)
                    .color(theme::MUTED),
                )
                .child(
                    rect()
                        .horizontal()
                        .content(Content::wrap_spacing(theme::space::GRID))
                        .spacing(theme::space::GRID)
                        .child(widgets::stat(
                            "GOLD",
                            character
                                .gold
                                .map_or_else(|| "Not set".to_owned(), format::amount),
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
                widgets::a11y_text(widgets::plural(item_count, "stack", "stacks"))
                    .font_size(theme::font::BODY)
                    .color(theme::TEXT)
                    .into_element(),
            ),
            (
                "Styles".to_owned(),
                widgets::a11y_text(format!("{style_count} learned"))
                    .font_size(theme::font::BODY)
                    .color(theme::TEXT)
                    .into_element(),
            ),
            (
                "Unspent attribute points".to_owned(),
                widgets::a11y_text(character.attribute_points.to_string())
                    .font_size(theme::font::BODY)
                    .color(if character.attribute_points > 0 {
                        theme::ACCENT
                    } else {
                        theme::MUTED
                    })
                    .into_element(),
            ),
            (
                "Body / Mind / Spirit".to_owned(),
                widgets::a11y_text(format!(
                    "{} / {} / {}",
                    character.body, character.mind, character.spirit
                ))
                .font_size(theme::font::BODY)
                .color(theme::TEXT)
                .into_element(),
            ),
            (
                "Charm / Intuition / Intimidate".to_owned(),
                widgets::a11y_text(format!(
                    "{} / {} / {}",
                    character.charm, character.intuition, character.intimidate
                ))
                .font_size(theme::font::BODY)
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
                widgets::a11y_text(path)
                    .font_size(theme::font::LABEL)
                    .color(theme::MUTED),
            ),
            widgets::divider(),
            widgets::label_row(
                "Contents",
                widgets::a11y_text(format!(
                    "{block_count} blocks · {}",
                    file_size.map_or_else(|| "unknown size".to_owned(), format::bytes)
                ))
                .font_size(theme::font::LABEL)
                .color(theme::MUTED),
            ),
            widgets::divider(),
            rect()
                .horizontal()
                .spacing(theme::space::MD)
                .child(
                    widgets::a11y_text(if game_ok { "Game data" } else { "Notice" })
                        .font_size(theme::font::LABEL)
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
        .spacing(theme::space::XXL)
        .child(hero)
        .child(quick)
        .child(details)
        .into_element()
}
