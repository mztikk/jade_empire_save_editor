//! About, help and the remembered settings.
//!
//! The window has no separate settings dialog: everything that persists is
//! small enough to show on one page next to the version and the backup rules,
//! which is also the place a user looks when they need to find the config file.

use std::fmt::Write as _;

use freya::prelude::*;

use crate::Locale;
use crate::app::Ctx;
use crate::i18n::{fill, locale_label, t};
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
        GameStatus::Loading => t("about.game_data.loading").to_owned(),
        GameStatus::Loaded { root, .. } => root.display().to_string(),
        GameStatus::NamesOnly { .. } => t("about.game_data.bundled").to_owned(),
        GameStatus::Missing => t("about.game_data.missing").to_owned(),
        GameStatus::Error(error) => {
            fill("about.game_data.unreadable", &[("error", error.as_str())])
        }
    };
    let backup = ctx
        .session
        .borrow()
        .last_backup()
        .map(std::path::Path::to_path_buf);
    let config_path = crate::config::Config::path();

    let close = Action::new(move || show_about.set(false));
    let close_button = widgets::outline_action(t("about.close_page"), t("about.close"), close);

    let selected_theme = *theme_choice.read();
    let selected_scale = *text_scale.read();
    let mut appearance: Vec<Element> = Vec::new();
    appearance.push(widgets::hint(t("about.appearance.explanation")));
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
            .child(widgets::section_label(t("label.theme").to_uppercase()))
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
                "about.text_size.option",
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
            .child(widgets::section_label(t("about.text_size_heading")))
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
                "about.language.option",
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
            .child(widgets::section_label(t("label.language").to_uppercase()))
            .children(locale_chips)
            .into_element(),
    );
    let remember = *remember_window.read();
    // Wayland compositors place windows themselves, so the toggle may only
    // promise the size there. X11 and the desktop platforms restore both.
    let compositor_places_windows = crate::geometry::position_is_compositor_owned();
    let remember_description = if compositor_places_windows {
        t("about.remember_size_wayland")
    } else {
        t("about.remember_size_description")
    };
    appearance.push(
        rect()
            .horizontal()
            .width(Size::fill())
            .spacing(theme::space::sm())
            .content(Content::wrap_spacing(theme::space::sm()))
            .child(widgets::section_label(t("about.window_heading")))
            .child(appearance_chip(
                t("about.remember_size"),
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
        appearance.push(widgets::hint(t("about.window.wayland_hint")));
    }

    let mut diagnostics: Vec<(String, String)> = vec![
        (
            t("about.diagnostics.version").to_owned(),
            crate::APP_VERSION.to_owned(),
        ),
        (
            t("about.diagnostics.open_savegame").to_owned(),
            ctx.session.borrow().current_path().map_or_else(
                || t("about.diagnostics.none").to_owned(),
                |path| path.display().to_string(),
            ),
        ),
        (
            capitalize(t("count.block.other")),
            ctx.session.borrow().block_count().to_string(),
        ),
        (
            t("about.diagnostics.file_size").to_owned(),
            ctx.session.borrow().file_size().map_or_else(
                || t("about.diagnostics.not_read").to_owned(),
                crate::session::format::bytes,
            ),
        ),
        (
            capitalize(t("label.theme")),
            fill(
                "about.diagnostics.theme_value",
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
            capitalize(t("label.language")),
            locale_label(crate::i18n::current()).to_owned(),
        ),
        (
            t("about.diagnostics.renderer").to_owned(),
            std::env::var("JADEFORGE_RENDERER")
                .unwrap_or_else(|_| t("about.diagnostics.auto").to_owned()),
        ),
        (
            t("about.diagnostics.remember_window").to_owned(),
            if *remember_window.read() {
                t("about.diagnostics.yes").to_owned()
            } else {
                t("about.diagnostics.no").to_owned()
            },
        ),
    ];
    diagnostics.push((
        t("about.diagnostics.backup_folder").to_owned(),
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
        let mut text = format!("{}\n", t("about.diagnostics.title"));
        report_line(
            &mut text,
            t("about.diagnostics.version"),
            crate::APP_VERSION,
        );
        report_line(
            &mut text,
            &capitalize(t("label.save.folder")),
            &save_folder.display().to_string(),
        );
        if let Some(path) = ctx.session.borrow().current_path() {
            report_line(
                &mut text,
                &capitalize(t("count.savegame.one")),
                &path.display().to_string(),
            );
        }
        let config = config_path.as_ref().map_or_else(
            || t("about.diagnostics.unavailable").to_owned(),
            |path| path.display().to_string(),
        );
        report_line(&mut text, t("about.diagnostics.config"), &config);
        report_line(
            &mut text,
            &capitalize(t("label.theme")),
            &fill(
                "about.diagnostics.theme_value",
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
            &capitalize(t("label.language")),
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
                    t("about.diagnostics.copied")
                } else {
                    t("about.diagnostics.copy_failed")
                });
            });
        })
    };

    let mut settings = Vec::new();
    if save_folder.as_os_str().is_empty() {
        settings.push(widgets::hint(t("about.save_folder.none")));
    } else {
        settings.push(path_section(
            ctx,
            capitalize(t("label.save.folder")),
            &save_folder.display().to_string(),
            save_folder.clone(),
            "count.savegame.one",
        ));
    }
    settings.extend([
        widgets::label_row(
            t("about.sort_order"),
            widgets::a11y_text(fill(
                "about.",
                &[
                    ("name", t(sort.label())),
                    (
                        "direction",
                        if descending {
                            t("sort.descending")
                        } else {
                            t("sort.ascending")
                        },
                    ),
                ],
            ))
            .font_size(theme::font::label())
            .color(theme::text()),
        ),
        widgets::label_row(
            t("overview.game_data"),
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
                t("about.settings_file"),
                &path.display().to_string(),
                folder,
                "path.settings",
            ));
        }
        None => settings.push(widgets::hint(t("about.config.none"))),
    }

    let mut backups = vec![widgets::hint(t("about.backup.explanation"))];
    match &backup {
        Some(backup) => {
            let folder = backup
                .parent()
                .map_or_else(|| save_folder.clone(), std::path::Path::to_path_buf);
            backups.push(path_section(
                ctx,
                t("about.latest_backup"),
                &backup.display().to_string(),
                folder,
                "count.backup.one",
            ));
        }
        None => backups.push(widgets::hint(t("about.backup.none"))),
    }

    let mut children = vec![
        rect()
            .horizontal()
            .width(Size::fill())
            .content(Content::Flex)
            .cross_align(Alignment::Center)
            .child(
                widgets::a11y_text(t("about.tagline"))
                    .font_size(theme::font::body())
                    .color(theme::muted()),
            )
            .child(rect().width(Size::flex(1.0)))
            .child(close_button)
            .into_element(),
        widgets::hint(t("about.read_write.summary")),
        widgets::divider(),
        widgets::section_label(t("about.settings_heading")),
        widgets::hint(t("about.settings.explanation")),
    ];
    children.extend(settings);
    children.push(widgets::divider());
    children.push(widgets::section_label(t("field.appearance")));
    children.extend(appearance);
    children.push(widgets::divider());
    children.push(widgets::section_label(t("about.diagnostics_heading")));
    children.extend(diagnostics_rows);
    children.push(
        widgets::outline_action(
            t("about.copy_diagnostics_hint"),
            t("about.copy_diagnostics"),
            copy_diagnostics,
        )
        .into_element(),
    );
    children.push(widgets::divider());
    children.push(widgets::section_label(t("about.licenses_heading")));
    children.push(widgets::hint(t("about.license.summary")));
    children.push(widgets::divider());
    children.push(widgets::section_label(
        t("count.backup.other").to_uppercase(),
    ));
    children.extend(backups);
    children.push(widgets::divider());
    children.push(widgets::section_label(t("about.shortcuts_heading")));
    for (action, keys) in [
        (t("about.shortcut.save"), "Ctrl+S"),
        (t("shell.choose_save_folder"), "Ctrl+O"),
        (t("about.shortcut.focus"), "Ctrl+F"),
        (t("about.shortcut.undo"), "Ctrl+Z"),
        (t("about.shortcut.redo"), "Ctrl+Y or Ctrl+Shift+Z"),
        (t("about.shortcut.scan"), "F5"),
        (t("about.shortcut.help"), "Ctrl+/"),
        (t("about.shortcut.escape"), "Escape"),
    ] {
        children.push(widgets::label_row(
            action,
            widgets::a11y_text(keys).font_size(theme::font::label()),
        ));
    }

    widgets::card(
        t("about.title"),
        Some(fill(
            "about.diagnostics.version_footer",
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
            "about.diagnostics.line",
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
    what: &'static str,
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
