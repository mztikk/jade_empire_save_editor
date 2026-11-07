//! About, help and the remembered settings.
//!
//! The window has no separate settings dialog: everything that persists is
//! small enough to show on one page next to the version and the backup rules,
//! which is also the place a user looks when they need to find the config file.

use std::fmt::Write as _;

use freya::prelude::*;

use crate::Locale;
use crate::app::Ctx;
use crate::i18n::{Key, fill, keys, locale_label, t};
use crate::message::capitalize;
use crate::preferences::{TEXT_SCALES, ThemeChoice};
use crate::session::{GameStatus, SaveSort};
use crate::theme;
use crate::views::path_actions;
use crate::widgets::{self, AccessibleButton, Action};

/// Renders the About / help surface.
#[allow(
    clippy::too_many_arguments, // three independent reactive preferences plus the shell handles
    clippy::too_many_lines // one declarative page keeps the help text together
)]
pub fn view(
    ctx: &Ctx,
    sort: State<(SaveSort, bool)>,
    mut show_about: State<bool>,
    theme_choice: State<ThemeChoice>,
    text_scale: State<u16>,
    locale: State<Locale>,
    remember_window: State<bool>,
) -> Element {
    let (sort, descending) = *sort.read();
    let save_folder = ctx.session.borrow().directory().to_path_buf();
    let game_text = match ctx.session.borrow().game_status() {
        GameStatus::Loading => t(keys::ABOUT_GAME_DATA_LOADING).to_owned(),
        GameStatus::Loaded { root, .. } => root.display().to_string(),
        GameStatus::NamesOnly { .. } => t(keys::ABOUT_GAME_DATA_BUNDLED).to_owned(),
        GameStatus::Missing => t(keys::ABOUT_GAME_DATA_MISSING).to_owned(),
        GameStatus::Error(error) => fill(
            keys::ABOUT_GAME_DATA_UNREADABLE,
            &[("error", error.as_str())],
        ),
    };
    let backup = ctx
        .session
        .borrow()
        .last_backup()
        .map(std::path::Path::to_path_buf);
    let config_path = crate::config::Config::path();

    let close = Action::new(move || show_about.set(false));
    let close_button =
        widgets::ActionButton::new(t(keys::ABOUT_CLOSE_PAGE), t(keys::ABOUT_CLOSE), close);

    let selected_theme = *theme_choice.read();
    let selected_scale = *text_scale.read();
    let mut appearance: Vec<Element> = Vec::new();
    appearance.push(widgets::hint(t(keys::ABOUT_APPEARANCE_EXPLANATION)));
    let mut theme_chips: Vec<Element> = Vec::new();
    for choice in ThemeChoice::ALL {
        let mut theme_choice = theme_choice;
        let scale = selected_scale;
        let selected = selected_theme == choice;
        theme_chips.push(appearance_chip(
            t(choice.label()),
            t(choice.description()),
            selected,
            Action::new(move || {
                theme_choice.set(choice);
                crate::config::Config::remember_ui_preferences(choice, scale);
            }),
        ));
    }
    appearance.push(
        rect()
            .horizontal()
            .width(Size::fill())
            .spacing(theme::space::sm())
            .content(Content::wrap_spacing(theme::space::sm()))
            .child(widgets::section_label(t(keys::LABEL_THEME).to_uppercase()))
            .children(theme_chips)
            .into_element(),
    );
    let mut scale_chips: Vec<Element> = Vec::new();
    for percent in TEXT_SCALES {
        let mut text_scale = text_scale;
        let theme = selected_theme;
        let selected = selected_scale == percent;
        scale_chips.push(appearance_chip(
            format!("{percent} %"),
            fill(
                keys::ABOUT_TEXT_SIZE_OPTION,
                &[("percent", &percent.to_string())],
            ),
            selected,
            Action::new(move || {
                text_scale.set(percent);
                crate::config::Config::remember_ui_preferences(theme, percent);
            }),
        ));
    }
    appearance.push(
        rect()
            .horizontal()
            .width(Size::fill())
            .spacing(theme::space::sm())
            .content(Content::wrap_spacing(theme::space::sm()))
            .child(widgets::section_label(t(keys::ABOUT_TEXT_SIZE_HEADING)))
            .children(scale_chips)
            .into_element(),
    );
    let selected_locale = *locale.read();
    let mut locale_chips: Vec<Element> = Vec::new();
    for candidate in Locale::ALL {
        let mut locale = locale;
        let selected = selected_locale == candidate;
        locale_chips.push(appearance_chip(
            locale_label(candidate),
            fill(
                keys::ABOUT_LANGUAGE_OPTION,
                &[("language", locale_label(candidate))],
            ),
            selected,
            Action::new(move || {
                locale.set(candidate);
                crate::config::Config::remember_locale(candidate);
            }),
        ));
    }
    appearance.push(
        rect()
            .horizontal()
            .width(Size::fill())
            .spacing(theme::space::sm())
            .content(Content::wrap_spacing(theme::space::sm()))
            .child(widgets::section_label(
                t(keys::LABEL_LANGUAGE).to_uppercase(),
            ))
            .children(locale_chips)
            .into_element(),
    );
    let remember = *remember_window.read();
    // Wayland compositors place windows themselves, so the toggle may only
    // promise the size there. X11 and the desktop platforms restore both.
    let compositor_places_windows = crate::geometry::position_is_compositor_owned();
    let remember_description = if compositor_places_windows {
        t(keys::ABOUT_REMEMBER_SIZE_WAYLAND)
    } else {
        t(keys::ABOUT_REMEMBER_SIZE_DESCRIPTION)
    };
    appearance.push(
        rect()
            .horizontal()
            .width(Size::fill())
            .spacing(theme::space::sm())
            .content(Content::wrap_spacing(theme::space::sm()))
            .child(widgets::section_label(t(keys::ABOUT_WINDOW_HEADING)))
            .child(appearance_chip(
                t(keys::ABOUT_REMEMBER_SIZE),
                remember_description,
                remember,
                Action::new(move || {
                    let mut state = remember_window;
                    let next = !*state.read();
                    state.set(next);
                    crate::config::Config::remember_window_preference(next);
                }),
            ))
            .into_element(),
    );
    if compositor_places_windows && remember {
        appearance.push(widgets::hint(t(keys::ABOUT_WINDOW_WAYLAND_HINT)));
    }

    let mut diagnostics: Vec<(String, String)> = vec![
        (
            t(keys::ABOUT_DIAGNOSTICS_VERSION).to_owned(),
            crate::APP_VERSION.to_owned(),
        ),
        (
            t(keys::ABOUT_DIAGNOSTICS_OPEN_SAVEGAME).to_owned(),
            ctx.session.borrow().current_path().map_or_else(
                || t(keys::ABOUT_DIAGNOSTICS_NONE).to_owned(),
                |path| path.display().to_string(),
            ),
        ),
        (
            capitalize(t(keys::COUNT_BLOCK_OTHER)),
            ctx.session.borrow().block_count().to_string(),
        ),
        (
            t(keys::ABOUT_DIAGNOSTICS_FILE_SIZE).to_owned(),
            ctx.session.borrow().file_size().map_or_else(
                || t(keys::ABOUT_DIAGNOSTICS_NOT_READ).to_owned(),
                crate::session::format::bytes,
            ),
        ),
        (
            capitalize(t(keys::LABEL_THEME)),
            fill(
                keys::ABOUT_DIAGNOSTICS_THEME_VALUE,
                &[
                    ("theme", theme::current().palette.kind.key()),
                    (
                        "scale",
                        &(theme::current().text_scale * 100.0).round().to_string(),
                    ),
                ],
            ),
        ),
        (
            capitalize(t(keys::LABEL_LANGUAGE)),
            locale_label(crate::i18n::current()).to_owned(),
        ),
        (
            t(keys::ABOUT_DIAGNOSTICS_RENDERER).to_owned(),
            std::env::var("JADEFORGE_RENDERER")
                .unwrap_or_else(|_| t(keys::ABOUT_DIAGNOSTICS_AUTO).to_owned()),
        ),
        (
            t(keys::ABOUT_DIAGNOSTICS_REMEMBER_WINDOW).to_owned(),
            if *remember_window.read() {
                t(keys::ABOUT_DIAGNOSTICS_YES).to_owned()
            } else {
                t(keys::ABOUT_DIAGNOSTICS_NO).to_owned()
            },
        ),
    ];
    diagnostics.push((
        t(keys::ABOUT_DIAGNOSTICS_BACKUP_FOLDER).to_owned(),
        backup
            .as_ref()
            .and_then(|backup| backup.parent())
            .map_or_else(
                || save_folder.display().to_string(),
                |path| path.display().to_string(),
            ),
    ));
    let mut diagnostics_rows: Vec<Element> = Vec::new();
    for (name, value) in diagnostics {
        diagnostics_rows.push(widgets::label_row(
            name,
            widgets::a11y_text(value)
                .font_size(theme::font::label())
                .color(theme::text()),
        ));
    }
    let diagnostics_text = {
        let mut text = format!("{}\n", t(keys::ABOUT_DIAGNOSTICS_TITLE));
        report_line(
            &mut text,
            t(keys::ABOUT_DIAGNOSTICS_VERSION),
            crate::APP_VERSION,
        );
        report_line(
            &mut text,
            &capitalize(t(keys::LABEL_SAVE_FOLDER)),
            &save_folder.display().to_string(),
        );
        if let Some(path) = ctx.session.borrow().current_path() {
            report_line(
                &mut text,
                &capitalize(t(keys::COUNT_SAVEGAME_ONE)),
                &path.display().to_string(),
            );
        }
        let config = config_path.as_ref().map_or_else(
            || t(keys::ABOUT_DIAGNOSTICS_UNAVAILABLE).to_owned(),
            |path| path.display().to_string(),
        );
        report_line(&mut text, t(keys::ABOUT_DIAGNOSTICS_CONFIG), &config);
        report_line(
            &mut text,
            &capitalize(t(keys::LABEL_THEME)),
            &fill(
                keys::ABOUT_DIAGNOSTICS_THEME_VALUE,
                &[
                    ("theme", theme::current().palette.kind.key()),
                    (
                        "scale",
                        &(theme::current().text_scale * 100.0).round().to_string(),
                    ),
                ],
            ),
        );
        report_line(
            &mut text,
            &capitalize(t(keys::LABEL_LANGUAGE)),
            locale_label(crate::i18n::current()),
        );
        text
    };
    let copy_diagnostics = {
        let ctx = ctx.clone();
        Action::new(move || {
            let copied = crate::clipboard::copy(diagnostics_text.clone());
            ctx.run(|session| {
                session.notify(if copied {
                    t(keys::ABOUT_DIAGNOSTICS_COPIED)
                } else {
                    t(keys::ERROR_CLIPBOARD_UNAVAILABLE)
                });
            });
        })
    };

    let mut settings = Vec::new();
    if save_folder.as_os_str().is_empty() {
        settings.push(widgets::hint(t(keys::ABOUT_SAVE_FOLDER_NONE)));
    } else {
        settings.push(path_section(
            ctx,
            capitalize(t(keys::LABEL_SAVE_FOLDER)),
            &save_folder.display().to_string(),
            save_folder.clone(),
            keys::COUNT_SAVEGAME_ONE,
        ));
    }
    settings.extend([
        widgets::label_row(
            t(keys::ABOUT_SORT_ORDER),
            widgets::a11y_text(fill(
                keys::ABOUT_,
                &[
                    ("name", t(sort.label())),
                    (
                        "direction",
                        if descending {
                            t(keys::SORT_DESCENDING)
                        } else {
                            t(keys::SORT_ASCENDING)
                        },
                    ),
                ],
            ))
            .font_size(theme::font::label())
            .color(theme::text()),
        ),
        widgets::label_row(
            t(keys::OVERVIEW_GAME_DATA),
            widgets::a11y_text(game_text)
                .font_size(theme::font::label())
                .color(theme::text()),
        ),
    ]);
    match &config_path {
        Some(path) => {
            let folder = path
                .parent()
                .map_or_else(|| save_folder.clone(), std::path::Path::to_path_buf);
            settings.push(path_section(
                ctx,
                t(keys::ABOUT_SETTINGS_FILE),
                &path.display().to_string(),
                folder,
                keys::PATH_SETTINGS,
            ));
        }
        None => settings.push(widgets::hint(t(keys::ABOUT_CONFIG_NONE))),
    }

    let mut backups = vec![widgets::hint(t(keys::ABOUT_BACKUP_EXPLANATION))];
    match &backup {
        Some(backup) => {
            let folder = backup
                .parent()
                .map_or_else(|| save_folder.clone(), std::path::Path::to_path_buf);
            backups.push(path_section(
                ctx,
                t(keys::ABOUT_LATEST_BACKUP),
                &backup.display().to_string(),
                folder,
                keys::COUNT_BACKUP_ONE,
            ));
        }
        None => backups.push(widgets::hint(t(keys::ABOUT_BACKUP_NONE))),
    }

    let mut children = vec![
        rect()
            .horizontal()
            .width(Size::fill())
            .content(Content::Flex)
            .cross_align(Alignment::Center)
            .child(
                widgets::a11y_text(t(keys::ABOUT_TAGLINE))
                    .font_size(theme::font::body())
                    .color(theme::muted()),
            )
            .child(rect().width(Size::flex(1.0)))
            .child(close_button)
            .into_element(),
        widgets::hint(t(keys::ABOUT_READ_WRITE_SUMMARY)),
        widgets::divider(),
        widgets::section_label(t(keys::ABOUT_SETTINGS_HEADING)),
        widgets::hint(t(keys::ABOUT_SETTINGS_EXPLANATION)),
    ];
    children.extend(settings);
    children.push(widgets::divider());
    children.push(widgets::section_label(t(keys::FIELD_APPEARANCE)));
    children.extend(appearance);
    children.push(widgets::divider());
    children.push(widgets::section_label(t(keys::ABOUT_DIAGNOSTICS_HEADING)));
    children.extend(diagnostics_rows);
    children.push(
        widgets::ActionButton::new(
            t(keys::ABOUT_COPY_DIAGNOSTICS_HINT),
            t(keys::ABOUT_COPY_DIAGNOSTICS),
            copy_diagnostics,
        )
        .into_element(),
    );
    children.push(widgets::divider());
    children.push(widgets::section_label(t(keys::ABOUT_LICENSES_HEADING)));
    children.push(widgets::hint(t(keys::ABOUT_LICENSE_SUMMARY)));
    children.push(widgets::divider());
    children.push(widgets::section_label(
        t(keys::COUNT_BACKUP_OTHER).to_uppercase(),
    ));
    children.extend(backups);
    children.push(widgets::divider());
    children.push(widgets::section_label(t(keys::ABOUT_SHORTCUTS_HEADING)));
    for (action, keys) in [
        (t(keys::ABOUT_SHORTCUT_SAVE), "Ctrl+S"),
        (t(keys::SHELL_CHOOSE_SAVE_FOLDER), "Ctrl+O"),
        (t(keys::ABOUT_SHORTCUT_FOCUS), "Ctrl+F"),
        (t(keys::ABOUT_SHORTCUT_UNDO), "Ctrl+Z"),
        (t(keys::ABOUT_SHORTCUT_REDO), "Ctrl+Y or Ctrl+Shift+Z"),
        (t(keys::ABOUT_SHORTCUT_SCAN), "F5"),
        (t(keys::ABOUT_SHORTCUT_HELP), "Ctrl+/"),
        (t(keys::ABOUT_SHORTCUT_ESCAPE), "Escape"),
    ] {
        children.push(widgets::label_row(
            action,
            widgets::a11y_text(keys).font_size(theme::font::label()),
        ));
    }

    widgets::card(
        t(keys::ABOUT_TITLE),
        Some(fill(
            keys::ABOUT_DIAGNOSTICS_VERSION_FOOTER,
            &[("version", crate::APP_VERSION)],
        )),
        children,
    )
}

/// Appends one `label: value` line to the copied diagnostics report.
fn report_line(text: &mut String, label: &str, value: &str) {
    let _ = writeln!(
        text,
        "{}",
        fill(
            keys::ABOUT_DIAGNOSTICS_LINE,
            &[("label", label), ("value", value)]
        ),
    );
}

/// A labelled path with wrapping text and copy/open actions.
fn path_section(
    ctx: &Ctx,
    title: impl Into<String>,
    path: &str,
    folder: std::path::PathBuf,
    what: Key,
) -> Element {
    let value = path.to_owned();
    let title = title.into();
    rect()
        .vertical()
        .width(Size::fill())
        .spacing(theme::space::xs())
        .child(
            widgets::a11y_text(title)
                .font_size(theme::font::label())
                .color(theme::muted()),
        )
        .child(
            paragraph()
                .width(Size::fill())
                .max_width(Size::px(theme::layout::text_measure()))
                .font_size(theme::font::label())
                .color(theme::text())
                .a11y_builder(move |node| node.set_value(value))
                .span(path.to_owned()),
        )
        .child(path_actions(ctx, path, folder, what))
        .into_element()
}

/// A selectable appearance option, exposed as a radio button.
fn appearance_chip(
    label: impl Into<String>,
    accessible: impl Into<String>,
    selected: bool,
    action: Action,
) -> Element {
    AccessibleButton::new(
        accessible,
        action,
        rect()
            .padding(Gaps::new_symmetric(theme::space::xxs(), theme::space::md()))
            .background(if selected {
                theme::surface_active()
            } else {
                theme::surface()
            })
            .border(
                Border::new()
                    .fill(if selected {
                        theme::accent()
                    } else {
                        Color::TRANSPARENT
                    })
                    .width(theme::stroke::focus()),
            )
            .corner_radius(theme::radius::pill())
            .child(
                widgets::a11y_text(label.into())
                    .font_size(theme::font::micro())
                    .max_lines(1)
                    .color(if selected {
                        theme::accent()
                    } else {
                        theme::muted()
                    }),
            ),
    )
    .role(AccessibilityRole::RadioButton)
    .selected(selected)
    .hover_background(theme::surface_hover())
    .into_element()
}
