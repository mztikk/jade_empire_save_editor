//! Header, preview, footer and empty-state widgets of the shell.

use std::path::PathBuf;
use std::rc::Rc;

use freya::elements::image::ImageHandle;
use freya::prelude::*;

use crate::form::Form;
use crate::session::{GameStatus, MessageKind};
use crate::theme;

use super::Ctx;

#[allow(
    clippy::too_many_arguments, // one parameter per independent shell input
    clippy::needless_pass_by_value, // the closures own the sync handle
    clippy::too_many_lines // one declarative header keeps the rows together
)]
pub(super) fn header(
    ctx: &Ctx,
    directory: State<String>,
    message: &crate::session::Message,
    game_status: &GameStatus,
    dirty: bool,
    form_differs: bool,
    sync: Rc<dyn Fn()>,
) -> Element {
    let status_color = match message.kind {
        MessageKind::Info => theme::MUTED,
        MessageKind::Success => theme::SUCCESS,
        MessageKind::Error => theme::DANGER,
    };
    let status_live = match message.kind {
        MessageKind::Error => accesskit::Live::Assertive,
        MessageKind::Info | MessageKind::Success => accesskit::Live::Polite,
    };
    let (game_text, game_color) = match game_status {
        GameStatus::Loaded { items, .. } => (
            crate::widgets::plural(*items, "item", "items") + " loaded",
            theme::JADE,
        ),
        GameStatus::NamesOnly { .. } => ("English names loaded".to_owned(), theme::JADE),
        GameStatus::Missing => ("game data not found".to_owned(), theme::DIM),
        GameStatus::Error(_) => ("game data unreadable".to_owned(), theme::DANGER),
    };

    let scan = {
        let ctx = ctx.clone();
        let sync = sync.clone();
        move |_| {
            if form_differs {
                ctx.run(|session| {
                    session.set_error("Fix the invalid character fields before scanning.");
                });
                return;
            }
            let path = PathBuf::from(directory.read().as_str());
            if ctx.run(|session| session.set_directory(path)) {
                sync();
            }
        }
    };

    rect()
        .vertical()
        .width(Size::fill())
        .padding(Gaps::new(
            theme::space::FIELD,
            theme::space::CARD,
            theme::space::XL,
            theme::space::CARD,
        ))
        .spacing(theme::space::XL)
        .background(theme::PANEL)
        .child(
            rect()
                .horizontal()
                .width(Size::fill())
                .spacing(theme::space::XL)
                .content(Content::Flex)
                .child(
                    crate::widgets::a11y_text(super::WINDOW_TITLE)
                        .font_size(theme::font::TITLE)
                        .font_weight(FontWeight::BOLD)
                        .color(theme::ACCENT),
                )
                .child(
                    crate::widgets::a11y_text("Jade Empire save editor")
                        .font_size(theme::font::CAPTION)
                        .color(theme::DIM),
                )
                .child(rect().width(Size::flex(1.0)))
                .child(if dirty {
                    crate::widgets::badge("unsaved changes", theme::SURFACE_ACTIVE, theme::ACCENT)
                } else if form_differs {
                    crate::widgets::badge(
                        "invalid character input",
                        theme::SURFACE_ACTIVE,
                        theme::WARNING,
                    )
                } else {
                    crate::widgets::badge("all changes saved", theme::SURFACE, theme::DIM)
                })
                .child(crate::widgets::badge(game_text, theme::SURFACE, game_color)),
        )
        .child(
            rect()
                .horizontal()
                .width(Size::fill())
                .spacing(theme::space::LG)
                .content(Content::Flex)
                .child(
                    crate::widgets::a11y_text("SAVE FOLDER")
                        .font_size(theme::font::MICRO)
                        .font_weight(FontWeight::BOLD)
                        .color(theme::DIM),
                )
                .child(
                    Input::new(directory)
                        .placeholder("Path to the Jade Empire Save folder")
                        .width(Size::flex(1.0)),
                )
                .child(Button::new().outline().child("Browse…").on_press({
                    let ctx = ctx.clone();
                    let sync = sync.clone();
                    let mut directory = directory;
                    move |_| {
                        if form_differs {
                            ctx.run(|session| {
                                session.set_error(
                                    "Fix the invalid character fields before choosing a folder.",
                                );
                            });
                            return;
                        }
                        let start = crate::dialog::start_directory(directory.read().as_str());
                        let Some(folder) = crate::dialog::pick_folder(start.as_deref()) else {
                            return;
                        };
                        let folder_label = folder.display().to_string();
                        if ctx.run(|session| session.set_directory(folder)) {
                            *directory.write() = folder_label;
                            sync();
                        }
                    }
                }))
                .child(Button::new().outline().child("Scan").on_press(scan)),
        )
        .child(
            rect()
                .vertical()
                .width(Size::fill())
                .height(Size::px(theme::font::LINE_HEIGHT * 2.0))
                .main_align(Alignment::Center)
                .child(
                    crate::widgets::a11y_text(message.text.clone())
                        .a11y_builder(move |node| {
                            node.set_role(AccessibilityRole::Status);
                            node.set_live(status_live);
                        })
                        .font_size(theme::font::CAPTION)
                        .max_lines(2)
                        .color(status_color),
                ),
        )
        .into_element()
}

/// Renders the savegame screenshot slot.
///
/// The frame has a fixed height so that the placeholder and a decoded
/// screenshot occupy the same space: selecting a savegame must never move the
/// save list below it.
pub(super) fn preview_slot(preview: Option<ImageHandle>, selected: bool) -> Element {
    let message = if selected {
        "no screenshot"
    } else {
        "select a save"
    };
    // Keep the root and its content box stable while the preview loads. Freya's
    // diff can panic if an element changes its root type in place, especially
    // during rapid tab/window changes.
    let frame = rect()
        .width(Size::fill())
        .height(Size::px(theme::layout::PREVIEW))
        .center()
        .key("save-preview-frame");
    let content = rect()
        .width(Size::px(theme::layout::PREVIEW))
        .height(Size::px(theme::layout::PREVIEW))
        .center()
        .key("save-preview-content");
    match preview {
        Some(handle) => frame
            .child(
                content.child(
                    image(handle)
                        .width(Size::px(theme::layout::PREVIEW))
                        .height(Size::px(theme::layout::PREVIEW))
                        .corner_radius(theme::radius::LARGE)
                        .sampling_mode(SamplingMode::Bilinear)
                        .aspect_ratio(AspectRatio::Min)
                        .key("save-preview-image"),
                ),
            )
            .into_element(),
        None => frame
            .child(
                content
                    .background(theme::SURFACE)
                    .corner_radius(theme::radius::LARGE)
                    .child(
                        crate::widgets::a11y_text(message)
                            .font_size(theme::font::MICRO)
                            .color(theme::DIM)
                            .key("save-preview-placeholder"),
                    ),
            )
            .into_element(),
    }
}

#[allow(clippy::large_types_passed_by_value, clippy::needless_pass_by_value)] // `Form` is a bundle of cheap `Copy` handles
pub(super) fn footer(
    ctx: &Ctx,
    form: Form,
    dirty: bool,
    form_differs: bool,
    sync: Rc<dyn Fn()>,
) -> Element {
    let save = {
        let ctx = ctx.clone();
        let sync = sync.clone();
        move |_| {
            let applied = ctx.run(|session| session.apply(&form.read()));
            if applied.is_ok() {
                ctx.run(|session| {
                    let _ = session.save();
                });
                // Only normalise the form after the values were accepted. A
                // rejected form must keep its input so the user can fix it.
                sync();
            }
        }
    };
    let revert = {
        let ctx = ctx.clone();
        let sync = sync.clone();
        move |_| {
            if dirty
                && !crate::dialog::confirm(
                    "Discard changes?",
                    "Reload the savegame from disk and discard every change that is only in memory?",
                )
            {
                return;
            }
            ctx.run(|session| {
                let _ = session.revert();
            });
            sync();
        }
    };

    rect()
        .horizontal()
        .width(Size::fill())
        .spacing(theme::space::LG)
        .content(Content::Flex)
        .padding(Gaps::new(
            theme::space::LG,
            theme::space::CARD,
            theme::space::XL,
            theme::space::CARD,
        ))
        .background(theme::PANEL)
        .child(Button::new().filled().child("Save to disk").on_press(save))
        .child(Button::new().flat().child("Revert").on_press(revert))
        .child(rect().width(Size::flex(1.0)))
        .child(
            crate::widgets::a11y_text(if form_differs {
                "Character fields contain invalid values, they are not applied."
            } else if dirty {
                "Changes are only in memory until you save."
            } else {
                "Nothing to save."
            })
            .font_size(theme::font::MICRO)
            .color(if form_differs {
                theme::WARNING
            } else if dirty {
                theme::ACCENT
            } else {
                theme::DIM
            }),
        )
        .into_element()
}

pub(super) fn empty_state(savegames: usize) -> Element {
    let text = if savegames == 0 {
        "Point JadeForge at the folder that contains your Jade Empire savegames and press Scan."
    } else {
        "Pick a savegame on the left to start editing."
    };
    rect()
        .vertical()
        .width(Size::fill())
        .spacing(theme::space::MD)
        .padding(Gaps::new_all(theme::space::PAGE))
        .child(
            crate::widgets::a11y_text("No savegame selected")
                .a11y_builder(|node| node.set_role(AccessibilityRole::Heading))
                .font_size(theme::font::HEADING)
                .font_weight(FontWeight::BOLD)
                .color(theme::TEXT),
        )
        .child(
            crate::widgets::a11y_text(text)
                .font_size(theme::font::LABEL)
                .color(theme::MUTED),
        )
        .child(
            crate::widgets::a11y_text(
                "JadeForge never writes a savegame without verifying it and keeping a backup.",
            )
            .font_size(theme::font::CAPTION)
            .color(theme::DIM),
        )
        .into_element()
}
