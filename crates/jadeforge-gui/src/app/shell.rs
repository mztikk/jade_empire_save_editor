//! Header, preview, footer and empty-state widgets of the shell.

use std::path::PathBuf;
use std::rc::Rc;

use freya::elements::image::ImageHandle;
use freya::prelude::*;

use crate::form::Form;
use crate::session::{FieldError, GameStatus, MessageKind};
use crate::theme;
use crate::widgets::{AccessibleButton, Action};

use super::Ctx;

/// Opens the native folder dialog and scans the chosen folder.
///
/// The unsaved-changes check runs before the dialog opens: asking the user to
/// pick a folder and only then refusing it wastes their choice. A rejected scan
/// restores the folder that is actually open, so the path field never shows a
/// folder `JadeForge` is not using.
pub(super) fn choose_folder(
    ctx: &Ctx,
    mut directory: State<String>,
    start_scan: &Rc<dyn Fn(PathBuf) -> bool>,
) -> bool {
    if ctx.session.borrow().is_dirty() {
        ctx.run(|session| {
            session.set_error(
                "This savegame has unsaved changes: save them or press Revert before choosing another folder.",
            );
        });
        return false;
    }
    let start = crate::dialog::start_directory(directory.read().as_str());
    let folder = match crate::dialog::pick_folder(start.as_deref()) {
        crate::dialog::PickOutcome::Picked(folder) => folder,
        crate::dialog::PickOutcome::Cancelled => {
            ctx.run(|session| {
                session.inform("No folder was chosen; the save folder on screen is unchanged.");
            });
            return false;
        }
    };
    let folder_label = folder.display().to_string();
    if start_scan(folder) {
        *directory.write() = folder_label;
        true
    } else {
        let real = ctx.session.borrow().directory().display().to_string();
        *directory.write() = real;
        false
    }
}

/// Copies a status message to the system clipboard.
pub(super) fn copy_status(ctx: &Ctx, text: String) {
    let message = match Clipboard::set(text) {
        Ok(()) => "Copied the status message to the clipboard.".to_owned(),
        Err(_) => {
            "Could not copy the status message: the system clipboard is not available.".to_owned()
        }
    };
    ctx.run(|session| session.notify(&message));
}

#[allow(
    clippy::too_many_arguments, // one parameter per independent shell input
    clippy::needless_pass_by_value, // the closures own the sync handle
    clippy::too_many_lines // one declarative header keeps the rows together
)]
pub(super) fn header(
    ctx: &Ctx,
    directory: State<String>,
    game_directory: State<String>,
    message: &crate::session::Message,
    game_status: &GameStatus,
    dirty: bool,
    has_save: bool,
    form_errors: &[FieldError],
    scanning: bool,
    show_about: State<bool>,
    start_scan: Rc<dyn Fn(PathBuf) -> bool>,
    cancel_scan: Rc<dyn Fn()>,
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
    let game_text = match game_status {
        GameStatus::Loading => "loading game data…".to_owned(),
        GameStatus::Loaded { items, .. } => {
            crate::widgets::plural(*items, "item", "items") + " loaded"
        }
        GameStatus::NamesOnly { .. } => "English names loaded".to_owned(),
        GameStatus::Missing => "game data not found".to_owned(),
        GameStatus::Error(_) => "game data unreadable".to_owned(),
    };
    let game_color = match game_status {
        GameStatus::Loaded { .. } | GameStatus::NamesOnly { .. } => theme::JADE,
        GameStatus::Loading | GameStatus::Missing => theme::DIM,
        GameStatus::Error(_) => theme::DANGER,
    };
    let game_loaded = matches!(game_status, GameStatus::Loaded { .. });

    let scan = Rc::new({
        let ctx = ctx.clone();
        let start_scan = start_scan.clone();
        move || {
            let mut directory = directory;
            let path = PathBuf::from(directory.read().as_str());
            if !start_scan(path) {
                // Do not leave a rejected path in the field: show the folder
                // JadeForge actually has open.
                let real = ctx.session.borrow().directory().display().to_string();
                *directory.write() = real;
            }
        }
    });
    let browse = {
        let ctx = ctx.clone();
        let start_scan = start_scan.clone();
        move |_| {
            choose_folder(&ctx, directory, &start_scan);
        }
    };
    let use_game = {
        let ctx = ctx.clone();
        let mut game_directory = game_directory;
        move |_| {
            let path = PathBuf::from(game_directory.read().as_str());
            if ctx.run(|session| session.set_game_directory(path)) {
                let root = match ctx.session.borrow().game_status() {
                    GameStatus::Loaded { root, .. } => root.display().to_string(),
                    _ => String::new(),
                };
                *game_directory.write() = root;
            }
        }
    };
    let browse_game = {
        let ctx = ctx.clone();
        let mut game_directory = game_directory;
        move |_| {
            let start = crate::dialog::start_directory(game_directory.read().as_str());
            let folder = match crate::dialog::pick_folder(start.as_deref()) {
                crate::dialog::PickOutcome::Picked(folder) => folder,
                crate::dialog::PickOutcome::Cancelled => {
                    ctx.run(|session| {
                        session.inform("No folder was chosen; the game data folder is unchanged.");
                    });
                    return;
                }
            };
            let label = folder.display().to_string();
            if ctx.run(|session| session.set_game_directory(folder)) {
                *game_directory.write() = label;
            }
        }
    };
    // A glyph, not only the colour, marks a successful action.
    let status_text = if message.kind == MessageKind::Success {
        format!("✓ {}", message.text)
    } else {
        message.text.clone()
    };
    let copy_message = {
        let ctx = ctx.clone();
        let text = message.text.clone();
        Action::new(move || copy_status(&ctx, text.clone()))
    };

    let mut badges: Vec<Element> = Vec::new();
    if dirty {
        badges.push(crate::widgets::badge(
            "unsaved changes",
            theme::WARNING,
            theme::BACKGROUND,
        ));
    }
    if !form_errors.is_empty() {
        badges.push(crate::widgets::badge(
            crate::widgets::plural(form_errors.len(), "invalid field", "invalid fields"),
            theme::SURFACE_ACTIVE,
            theme::WARNING,
        ));
    }
    if !dirty && form_errors.is_empty() {
        badges.push(if has_save {
            crate::widgets::badge("all changes saved", theme::SURFACE, theme::DIM)
        } else {
            crate::widgets::badge("no savegame open", theme::SURFACE, theme::DIM)
        });
    }

    let mut rows: Vec<Element> = vec![
        rect()
            .horizontal()
            .width(Size::fill())
            .spacing(theme::space::MD)
            .content(Content::wrap_spacing(theme::space::MD))
            .cross_align(Alignment::Center)
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
            .children(badges)
            .child(crate::widgets::badge(game_text, theme::SURFACE, game_color))
            .child(
                rect()
                    .width(Size::px(theme::layout::DIVIDER))
                    .height(Size::px(theme::font::LINE_HEIGHT))
                    .background(theme::BORDER),
            )
            .child(about_button(show_about))
            .into_element(),
        rect()
            .horizontal()
            .width(Size::fill())
            .spacing(theme::space::LG)
            .content(Content::Flex)
            .cross_align(Alignment::Center)
            .child(
                crate::widgets::a11y_text("SAVE FOLDER")
                    .font_size(theme::font::MICRO)
                    .font_weight(FontWeight::BOLD)
                    .color(theme::DIM),
            )
            .child(
                Input::new(directory)
                    .placeholder("Path to the Jade Empire Save folder")
                    .width(Size::flex(1.0))
                    .on_submit({
                        let scan = scan.clone();
                        move |_: String| scan()
                    }),
            )
            .child(Button::new().outline().child("Browse…").on_press(browse))
            .child(
                Button::new()
                    .outline()
                    .enabled(!scanning)
                    .child(if scanning { "Scanning…" } else { "Scan" })
                    .on_press({
                        let scan = scan.clone();
                        move |_| scan()
                    }),
            )
            .maybe(scanning, |el| {
                let cancel_scan = cancel_scan.clone();
                el.child(
                    Button::new()
                        .flat()
                        .child("Cancel scan")
                        .on_press(move |_| cancel_scan()),
                )
            })
            .into_element(),
    ];

    if !game_loaded {
        rows.push(
            rect()
                .horizontal()
                .width(Size::fill())
                .spacing(theme::space::LG)
                .content(Content::Flex)
                .cross_align(Alignment::Center)
                .child(
                    crate::widgets::a11y_text("GAME DATA")
                        .font_size(theme::font::MICRO)
                        .font_weight(FontWeight::BOLD)
                        .color(theme::DIM),
                )
                .child(
                    Input::new(game_directory)
                        .placeholder("Path to the Jade Empire installation (with dialog.tlk)")
                        .width(Size::flex(1.0)),
                )
                .child(
                    Button::new()
                        .outline()
                        .child("Browse…")
                        .on_press(browse_game),
                )
                .child(Button::new().outline().child("Use").on_press(use_game))
                .into_element(),
        );
    }

    rows.push(
        rect()
            .horizontal()
            .width(Size::fill())
            .spacing(theme::space::MD)
            .content(Content::Flex)
            .cross_align(Alignment::Center)
            .child(
                rect()
                    .vertical()
                    .width(Size::flex(1.0))
                    // A fixed three-line slot: the status text can be long, but
                    // growing it must never move the tab bar below the header.
                    .height(Size::px(theme::font::LINE_HEIGHT * 3.0))
                    .main_align(Alignment::Center)
                    .child(
                        crate::widgets::a11y_text(status_text.clone())
                            .a11y_builder(move |node| {
                                node.set_role(AccessibilityRole::Status);
                                node.set_live(status_live);
                            })
                            .font_size(theme::font::CAPTION)
                            .max_lines(3)
                            .text_overflow(TextOverflow::Ellipsis)
                            .color(status_color),
                    ),
            )
            .child(AccessibleButton::new(
                "Copy the status message",
                copy_message,
                rect()
                    .padding(Gaps::new_symmetric(theme::space::XS, theme::space::MD))
                    .child(
                        crate::widgets::a11y_text("Copy")
                            .font_size(theme::font::MICRO)
                            .color(theme::MUTED),
                    ),
            ))
            .into_element(),
    );

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
        .children(rows)
        .into_element()
}

/// Top-bar button that opens the About / help surface.
fn about_button(mut show_about: State<bool>) -> Element {
    crate::widgets::outline_action(
        "About JadeForge, version, settings and backups",
        "About",
        Action::new(move || show_about.set(true)),
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
        .main_align(Alignment::Start)
        .cross_align(Alignment::Start)
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
                        .corner_radius(theme::radius::CARD)
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
                    .corner_radius(theme::radius::CARD)
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

#[allow(
    clippy::large_types_passed_by_value, // `Form` is a bundle of cheap `Copy` handles
    clippy::needless_pass_by_value // the closures own the sync handle
)]
pub(super) fn footer(
    ctx: &Ctx,
    form: Form,
    dirty: bool,
    has_save: bool,
    form_errors: &[FieldError],
    sync: Rc<dyn Fn()>,
) -> Element {
    let save = {
        let ctx = ctx.clone();
        let sync = sync.clone();
        move |_| {
            let errors = ctx.run(|session| session.form_errors(&form.read()));
            if let Some(first) = errors.first() {
                form.focus(first.field);
            }
            let saved = ctx.run(|session| session.save_applied(&form.read()));
            // An invalid form keeps its text so the user can fix it; a valid
            // form is normalised to the values that were written.
            if errors.is_empty() && saved.is_ok() {
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

    let can_save = has_save && dirty;
    let can_revert = has_save && dirty;
    let (hint_text, hint_color) = if !has_save {
        ("Open a savegame to edit it.", theme::DIM)
    } else if !form_errors.is_empty() && dirty {
        (
            "Some character fields are invalid and were not applied; your other edits can still be saved.",
            theme::WARNING,
        )
    } else if !form_errors.is_empty() {
        (
            "Some character fields are invalid and were not applied; fix the highlighted fields to apply them.",
            theme::WARNING,
        )
    } else if dirty {
        ("Changes are only in memory until you save.", theme::WARNING)
    } else {
        (
            "All changes are saved. There is nothing to write.",
            theme::DIM,
        )
    };

    rect()
        .horizontal()
        .width(Size::fill())
        .spacing(theme::space::LG)
        .content(Content::Flex)
        .cross_align(Alignment::Center)
        .padding(Gaps::new(
            theme::space::LG,
            theme::space::CARD,
            theme::space::XL,
            theme::space::CARD,
        ))
        .background(theme::PANEL)
        .child(
            Button::new()
                .filled()
                .enabled(can_save)
                .child("Save to disk")
                .on_press(save),
        )
        .child(
            Button::new()
                .flat()
                .enabled(can_revert)
                .child("Revert")
                .on_press(revert),
        )
        .child(rect().width(Size::flex(1.0)))
        .child(
            crate::widgets::a11y_text(hint_text)
                .font_size(theme::font::MICRO)
                .color(hint_color),
        )
        .into_element()
}

/// First-start placeholder while the scan worker reads the save folder.
///
/// The save list and the editor would otherwise both render their empty states
/// for a moment and then jump to the real content. Fixed card blocks keep the
/// layout close to the eventual editor while the status says what is happening.
pub(super) fn loading_state() -> Element {
    rect()
        .vertical()
        .width(Size::fill())
        .spacing(theme::space::XXL)
        .padding(Gaps::new_all(theme::space::PAGE))
        .child(
            crate::widgets::a11y_text("Loading savegames…")
                .a11y_builder(|node| node.set_role(AccessibilityRole::Heading))
                .font_size(theme::font::HEADING)
                .font_weight(FontWeight::BOLD)
                .color(theme::TEXT),
        )
        .child(
            crate::widgets::a11y_text(
                "JadeForge is reading the save folder and the game data. \
                 The save list appears as soon as the scan finishes.",
            )
            .font_size(theme::font::LABEL)
            .color(theme::MUTED),
        )
        .child(skeleton_block(SKELETON_HERO_HEIGHT))
        .child(skeleton_block(SKELETON_CARD_HEIGHT))
        .child(skeleton_block(SKELETON_CARD_HEIGHT))
        .into_element()
}

/// Height of the simulated overview hero in the loading state.
const SKELETON_HERO_HEIGHT: f32 = 180.0;
/// Height of one simulated overview card in the loading state.
const SKELETON_CARD_HEIGHT: f32 = 110.0;

/// A neutral placeholder card for the loading state.
fn skeleton_block(height: f32) -> Element {
    rect()
        .width(Size::fill())
        .height(Size::px(height))
        .background(theme::PANEL)
        .corner_radius(theme::radius::PANEL)
        .into_element()
}

/// The empty editor: a primary folder action instead of a dead panel.
#[allow(clippy::needless_pass_by_value)] // the closure owns the sync handle
pub(super) fn empty_state(
    ctx: &Ctx,
    savegames: usize,
    directory: State<String>,
    start_scan: Rc<dyn Fn(PathBuf) -> bool>,
) -> Element {
    let text = if savegames == 0 {
        "Point JadeForge at the folder that contains your Jade Empire savegames and press Scan."
    } else {
        "Pick a savegame on the left to start editing."
    };
    let choose = {
        let ctx = ctx.clone();
        move |_| {
            choose_folder(&ctx, directory, &start_scan);
        }
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
            Button::new()
                .filled()
                .child("Choose save folder")
                .on_press(choose),
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
