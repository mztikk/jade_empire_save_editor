//! Header, preview, footer and empty-state widgets of the shell.

use std::path::PathBuf;
use std::rc::Rc;

use freya::elements::image::ImageHandle;
use freya::prelude::*;

use crate::form::Form;
use crate::i18n::{fill, t};
use crate::message::capitalize;
use crate::session::{FieldError, GameStatus, MessageKind, Session};
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
            session.set_error(t("shell.this.savegame.has.unsaved.changes.save"));
        });
        return false;
    }
    let start = crate::dialog::start_directory(directory.read().as_str());
    let folder = match crate::dialog::pick_folder(start.as_deref()) {
        crate::dialog::PickOutcome::Picked(folder) => folder,
        crate::dialog::PickOutcome::Cancelled => {
            ctx.run(|session| {
                session.inform(t("shell.no.folder.was.chosen.the.save"));
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
    let message = if crate::clipboard::copy(text) {
        t("shell.copied.the.status.message.to.the").to_owned()
    } else {
        t("shell.could.not.copy.the.status.message").to_owned()
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
    show_backups: State<bool>,
    show_review: State<bool>,
    start_scan: Rc<dyn Fn(PathBuf) -> bool>,
    cancel_scan: Rc<dyn Fn()>,
) -> Element {
    let status_color = match message.kind {
        MessageKind::Info => theme::muted(),
        MessageKind::Success => theme::success(),
        MessageKind::Error => theme::danger(),
    };
    let status_live = match message.kind {
        MessageKind::Error => accesskit::Live::Assertive,
        MessageKind::Info | MessageKind::Success => accesskit::Live::Polite,
    };
    let game_text = match game_status {
        GameStatus::Loading => t("shell.loading.game.data").to_owned(),
        GameStatus::Loaded { items, .. } => fill(
            "shell.loaded",
            &[(
                "items",
                &crate::widgets::plural(*items, "count.item.one", "count.item.other"),
            )],
        ),
        GameStatus::NamesOnly { .. } => t("shell.english.names.loaded").to_owned(),
        GameStatus::Missing => t("shell.game.data.not.found").to_owned(),
        GameStatus::Error(_) => t("shell.game.data.unreadable").to_owned(),
    };
    let game_color = match game_status {
        GameStatus::Loaded { .. } | GameStatus::NamesOnly { .. } => theme::jade(),
        GameStatus::Loading | GameStatus::Missing => theme::dim(),
        GameStatus::Error(_) => theme::danger(),
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
                        session.inform(t("shell.no.folder.was.chosen.the.game"));
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
            t("shell.unsaved_changes"),
            theme::warning(),
            theme::background(),
        ));
    }
    if !form_errors.is_empty() {
        badges.push(crate::widgets::badge(
            crate::widgets::plural(
                form_errors.len(),
                "count.invalidfield.one",
                "count.invalidfield.other",
            ),
            theme::surface_active(),
            theme::warning(),
        ));
    }
    if !dirty && form_errors.is_empty() {
        badges.push(if has_save {
            crate::widgets::badge(t("shell.all_changes_saved"), theme::surface(), theme::dim())
        } else {
            crate::widgets::badge(t("shell.no_savegame_open"), theme::surface(), theme::dim())
        });
    }

    let mut rows: Vec<Element> = vec![
        rect()
            .horizontal()
            .width(Size::fill())
            .spacing(theme::space::md())
            .content(Content::wrap_spacing(theme::space::md()))
            .cross_align(Alignment::Center)
            .child(
                crate::widgets::a11y_text(super::WINDOW_TITLE)
                    .font_size(theme::font::title())
                    .font_weight(FontWeight::BOLD)
                    .color(theme::accent()),
            )
            .child(
                crate::widgets::a11y_text(t("about.tagline"))
                    .font_size(theme::font::caption())
                    .color(theme::dim()),
            )
            .child(rect().width(Size::flex(1.0)))
            .children(badges)
            .child(crate::widgets::badge(
                game_text,
                theme::surface(),
                game_color,
            ))
            .child(
                rect()
                    .width(Size::px(theme::layout::divider()))
                    .height(Size::px(theme::font::line_height()))
                    .background(theme::border()),
            )
            .child(backups_button(show_backups, show_about, show_review))
            .child(about_button(show_about, show_backups, show_review))
            .into_element(),
        rect()
            .horizontal()
            .width(Size::fill())
            .spacing(theme::space::lg())
            .content(Content::Flex)
            .cross_align(Alignment::Center)
            .child(
                crate::widgets::a11y_text(t("label.save.folder").to_uppercase())
                    .font_size(theme::font::micro())
                    .font_weight(FontWeight::BOLD)
                    .color(theme::dim()),
            )
            .child(
                Input::new(directory)
                    .placeholder(t("shell.save_folder_path"))
                    .width(Size::flex(1.0))
                    .on_submit({
                        let scan = scan.clone();
                        move |_: String| scan()
                    }),
            )
            .child(
                Button::new()
                    .outline()
                    .child(t("shell.browse"))
                    .on_press(browse),
            )
            .child(
                Button::new()
                    .outline()
                    .enabled(!scanning)
                    .child(if scanning {
                        t("shell.scanning")
                    } else {
                        t("shell.scan")
                    })
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
                        .child(t("shell.cancel_scan"))
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
                .spacing(theme::space::lg())
                .content(Content::Flex)
                .cross_align(Alignment::Center)
                .child(
                    crate::widgets::a11y_text(t("overview.game_data").to_uppercase())
                        .font_size(theme::font::micro())
                        .font_weight(FontWeight::BOLD)
                        .color(theme::dim()),
                )
                .child(
                    Input::new(game_directory)
                        .placeholder(t("shell.game_data_path"))
                        .width(Size::flex(1.0)),
                )
                .child(
                    Button::new()
                        .outline()
                        .child(t("shell.browse"))
                        .on_press(browse_game),
                )
                .child(
                    Button::new()
                        .outline()
                        .child(t("shell.use"))
                        .on_press(use_game),
                )
                .into_element(),
        );
    }

    rows.push(
        rect()
            .horizontal()
            .width(Size::fill())
            .spacing(theme::space::md())
            .content(Content::Flex)
            .cross_align(Alignment::Center)
            .child(
                rect()
                    .vertical()
                    .width(Size::flex(1.0))
                    // A fixed three-line slot: the status text can be long, but
                    // growing it must never move the tab bar below the header.
                    .height(Size::px(theme::font::line_height() * 3.0))
                    .main_align(Alignment::Center)
                    .child(
                        crate::widgets::a11y_text(status_text.clone())
                            .a11y_builder(move |node| {
                                node.set_role(AccessibilityRole::Status);
                                node.set_live(status_live);
                            })
                            .font_size(theme::font::caption())
                            .max_lines(3)
                            .text_overflow(TextOverflow::Ellipsis)
                            .color(status_color),
                    ),
            )
            .child(AccessibleButton::new(
                t("shell.copy_status"),
                copy_message,
                rect()
                    .padding(Gaps::new_symmetric(theme::space::xs(), theme::space::md()))
                    .child(
                        crate::widgets::a11y_text(t("shell.copy"))
                            .font_size(theme::font::micro())
                            .color(theme::muted()),
                    ),
            ))
            .into_element(),
    );

    rect()
        .vertical()
        .width(Size::fill())
        .padding(Gaps::new(
            theme::space::field(),
            theme::space::card(),
            theme::space::xl(),
            theme::space::card(),
        ))
        .spacing(theme::space::xl())
        .background(theme::panel())
        .children(rows)
        .into_element()
}

/// Top-bar button that opens the About / help surface.
fn about_button(
    mut show_about: State<bool>,
    mut show_backups: State<bool>,
    mut show_review: State<bool>,
) -> Element {
    crate::widgets::outline_action(
        t("shell.about.details"),
        t("shell.about"),
        Action::new(move || {
            show_about.set(true);
            show_backups.set(false);
            show_review.set(false);
        }),
    )
    .into_element()
}

/// Top-bar button that opens the backup browser.
fn backups_button(
    mut show_backups: State<bool>,
    mut show_about: State<bool>,
    mut show_review: State<bool>,
) -> Element {
    crate::widgets::outline_action(
        t("shell.browse.backups"),
        capitalize(t("count.backup.other")),
        Action::new(move || {
            show_backups.set(true);
            show_about.set(false);
            show_review.set(false);
        }),
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
        t("shell.no.screenshot")
    } else {
        t("shell.select.a.save")
    };
    // Keep the root and its content box stable while the preview loads. Freya's
    // diff can panic if an element changes its root type in place, especially
    // during rapid tab/window changes.
    let frame = rect()
        .width(Size::fill())
        .height(Size::px(theme::layout::preview()))
        .main_align(Alignment::Start)
        .cross_align(Alignment::Start)
        .key("save-preview-frame");
    let content = rect()
        .width(Size::px(theme::layout::preview()))
        .height(Size::px(theme::layout::preview()))
        .center()
        .key("save-preview-content");
    match preview {
        Some(handle) => frame
            .child(
                content.child(
                    image(handle)
                        .width(Size::px(theme::layout::preview()))
                        .height(Size::px(theme::layout::preview()))
                        .corner_radius(theme::radius::card())
                        .sampling_mode(SamplingMode::Bilinear)
                        .aspect_ratio(AspectRatio::Min)
                        .key("save-preview-image"),
                ),
            )
            .into_element(),
        None => frame
            .child(
                content
                    .background(theme::surface())
                    .corner_radius(theme::radius::card())
                    .child(
                        crate::widgets::a11y_text(message)
                            .font_size(theme::font::micro())
                            .color(theme::dim())
                            .key("save-preview-placeholder"),
                    ),
            )
            .into_element(),
    }
}

#[allow(
    clippy::large_types_passed_by_value, // `Form` is a bundle of cheap `Copy` handles
    clippy::needless_pass_by_value, // the closures own the sync handle
    clippy::too_many_arguments, // one parameter per independent footer control
    clippy::too_many_lines // one declarative footer keeps the actions in order
)]
pub(super) fn footer(
    ctx: &Ctx,
    form: Form,
    dirty: bool,
    has_save: bool,
    form_errors: &[FieldError],
    mut show_about: State<bool>,
    mut show_backups: State<bool>,
    mut show_review: State<bool>,
    sync: Rc<dyn Fn()>,
) -> Element {
    let save = {
        let ctx = ctx.clone();
        move |_| {
            let errors = ctx.run(|session| session.form_errors(&form.read()));
            if let Some(first) = errors.first() {
                form.focus(first.field);
            }
            if !ctx.session.borrow().is_dirty() {
                return;
            }
            show_about.set(false);
            show_backups.set(false);
            show_review.set(true);
        }
    };
    let undo = {
        let ctx = ctx.clone();
        let sync = sync.clone();
        move |_| {
            if ctx.run(Session::undo) {
                sync();
            }
        }
    };
    let redo = {
        let ctx = ctx.clone();
        let sync = sync.clone();
        move |_| {
            if ctx.run(Session::redo) {
                sync();
            }
        }
    };
    let can_undo = ctx.session.borrow().can_undo();
    let can_redo = ctx.session.borrow().can_redo();
    let revert = {
        let ctx = ctx.clone();
        let sync = sync.clone();
        move |_| {
            if dirty
                && !crate::dialog::confirm(
                    t("shell.discard.changes"),
                    t("shell.reload.the.savegame.from.disk.and"),
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
        (t("shell.open.a.savegame.to.edit.it"), theme::dim())
    } else if !form_errors.is_empty() && dirty {
        (t("shell.invalid_fields_savable"), theme::warning())
    } else if !form_errors.is_empty() {
        (t("shell.invalid_fields_fix"), theme::warning())
    } else if dirty {
        (
            t("shell.changes.are.only.in.memory.until"),
            theme::warning(),
        )
    } else {
        (t("shell.all.changes.are.saved.there.is"), theme::dim())
    };

    rect()
        .horizontal()
        .width(Size::fill())
        .spacing(theme::space::lg())
        .content(Content::Flex)
        .cross_align(Alignment::Center)
        .padding(Gaps::new(
            theme::space::lg(),
            theme::space::card(),
            theme::space::xl(),
            theme::space::card(),
        ))
        .background(theme::panel())
        .child(
            Button::new()
                .flat()
                .enabled(can_undo)
                .child(t("shell.undo"))
                .on_press(undo),
        )
        .child(
            Button::new()
                .flat()
                .enabled(can_redo)
                .child(t("shell.redo"))
                .on_press(redo),
        )
        .child(
            Button::new()
                .filled()
                .enabled(can_save)
                .child(t("shell.save"))
                .on_press(save),
        )
        .child(
            Button::new()
                .flat()
                .enabled(can_revert)
                .child(t("shell.revert"))
                .on_press(revert),
        )
        .child(rect().width(Size::flex(1.0)))
        .child(
            crate::widgets::a11y_text(hint_text)
                .font_size(theme::font::micro())
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
        .spacing(theme::space::xxl())
        .padding(Gaps::new_all(theme::space::page()))
        .child(
            crate::widgets::a11y_text(t("shell.loading_savegames"))
                .a11y_builder(|node| node.set_role(AccessibilityRole::Heading))
                .font_size(theme::font::heading())
                .font_weight(FontWeight::BOLD)
                .color(theme::text()),
        )
        .child(
            crate::widgets::a11y_text(t("shell.loading.explanation"))
                .font_size(theme::font::label())
                .color(theme::muted()),
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
        .background(theme::panel())
        .corner_radius(theme::radius::panel())
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
        t("shell.point.jadeforge.at.the.folder.that")
    } else {
        t("shell.pick.a.savegame.on.the.left")
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
        .spacing(theme::space::md())
        .padding(Gaps::new_all(theme::space::page()))
        .child(
            crate::widgets::a11y_text(t("shell.no_savegame_selected"))
                .a11y_builder(|node| node.set_role(AccessibilityRole::Heading))
                .font_size(theme::font::heading())
                .font_weight(FontWeight::BOLD)
                .color(theme::text()),
        )
        .child(
            crate::widgets::a11y_text(text)
                .font_size(theme::font::label())
                .color(theme::muted()),
        )
        .child(
            Button::new()
                .filled()
                .child(t("shell.choose_save_folder"))
                .on_press(choose),
        )
        .child(
            crate::widgets::a11y_text(t("shell.savegame.safety"))
                .font_size(theme::font::caption())
                .color(theme::dim()),
        )
        .into_element()
}

/// Banner shown when the open savegame changed on disk outside the editor.
///
/// A clean savegame reloads directly. A dirty one explains that reloading
/// discards the unsaved edits, because the write path refuses to overwrite a
/// file that changed underneath it.
pub(super) fn external_change_banner(ctx: &Ctx, sync: &Rc<dyn Fn()>) -> Element {
    let dirty = ctx.session.borrow().is_dirty();
    let text = if dirty {
        t("shell.disk_changed_dirty")
    } else {
        t("shell.disk_changed")
    };
    let reload = {
        let ctx = ctx.clone();
        let sync = sync.clone();
        move |_| {
            if dirty
                && !crate::dialog::confirm(
                    &format!("{}?", t("shell.reload")),
                    t("shell.the.savegame.changed.on.disk.outside"),
                )
            {
                return;
            }
            let result = if dirty {
                ctx.run(Session::revert)
            } else {
                ctx.run(Session::reload_from_disk)
            };
            if result.is_ok() {
                sync();
            }
        }
    };
    rect()
        .horizontal()
        .width(Size::fill())
        .spacing(theme::space::md())
        .content(Content::Flex)
        .cross_align(Alignment::Center)
        .padding(Gaps::new_symmetric(
            theme::space::md(),
            theme::space::card(),
        ))
        .background(theme::surface_active())
        .child(
            crate::widgets::a11y_text(text)
                .a11y_builder(|node| {
                    node.set_role(AccessibilityRole::Status);
                    node.set_live(accesskit::Live::Assertive);
                })
                .font_size(theme::font::caption())
                .color(theme::warning())
                .width(Size::flex(1.0)),
        )
        .child(
            Button::new()
                .outline()
                .child(t("shell.reload"))
                .on_press(reload),
        )
        .into_element()
}
