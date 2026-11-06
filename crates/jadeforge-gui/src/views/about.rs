//! About, help and the remembered settings.
//!
//! The window has no separate settings dialog: everything that persists is
//! small enough to show on one page next to the version and the backup rules,
//! which is also the place a user looks when they need to find the config file.

use std::fmt::Write as _;

use freya::prelude::*;

use crate::Locale;
use crate::app::Ctx;
use crate::i18n::{fill, t};
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
        GameStatus::Loading => t("Reading the game installation…").to_owned(),
        GameStatus::Loaded { root, .. } => root.display().to_string(),
        GameStatus::NamesOnly { .. } => t("Bundled English names (no installation)").to_owned(),
        GameStatus::Missing => t("No installation configured").to_owned(),
        GameStatus::Error(error) => fill("Unreadable: {error}", &[("error", error.as_str())]),
    };
    let backup = ctx
        .session
        .borrow()
        .last_backup()
        .map(std::path::Path::to_path_buf);
    let config_path = crate::config::Config::path();

    let close = Action::new(move || show_about.set(false));
    let close_button = widgets::outline_action(t("Close the About page"), t("Close"), close);

    let selected_theme = *theme_choice.read();
    let selected_scale = *text_scale.read();
    let mut appearance: Vec<Element> = Vec::new();
    appearance.push(widgets::hint(t(
        "The theme and text size apply immediately and are remembered in the settings file. \
         System follows the operating system preference; high contrast can be selected here \
         because the windowing layer does not report the platform's forced-colours setting.",
    )));
    let mut theme_chips: Vec<Element> = Vec::new();
    for choice in ThemeChoice::ALL {
        let mut theme_choice = theme_choice;
        let scale = selected_scale;
        let selected = selected_theme == choice;
        theme_chips.push(appearance_chip(
            choice.label(),
            choice.description(),
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
            .child(widgets::section_label(t("THEME")))
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
                "Use {percent} percent text size",
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
            .child(widgets::section_label(t("TEXT SIZE")))
            .children(scale_chips)
            .into_element(),
    );
    let selected_locale = *locale.read();
    let mut locale_chips: Vec<Element> = Vec::new();
    for candidate in Locale::ALL {
        let mut locale = locale;
        let selected = selected_locale == candidate;
        locale_chips.push(appearance_chip(
            candidate.label(),
            fill(
                "Use {language} for the interface",
                &[("language", candidate.label())],
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
            .child(widgets::section_label(t("LANGUAGE")))
            .children(locale_chips)
            .into_element(),
    );
    let remember = *remember_window.read();
    // Wayland compositors place windows themselves, so the toggle may only
    // promise the size there. X11 and the desktop platforms restore both.
    let compositor_places_windows = crate::geometry::position_is_compositor_owned();
    let remember_description = if compositor_places_windows {
        t("Remember the window size between runs; the compositor places the window")
    } else {
        t("Remember the window size and position between runs")
    };
    appearance.push(
        rect()
            .horizontal()
            .width(Size::fill())
            .spacing(theme::space::sm())
            .content(Content::wrap_spacing(theme::space::sm()))
            .child(widgets::section_label(t("WINDOW")))
            .child(appearance_chip(
                t("Remember size and position"),
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
        appearance.push(widgets::hint(t(
            "Wayland compositors place windows themselves. Add a window rule in your desktop settings to pin the position.",
        )));
    }

    let mut diagnostics: Vec<(String, String)> = vec![
        (t("Version").to_owned(), crate::APP_VERSION.to_owned()),
        (
            t("Open savegame").to_owned(),
            ctx.session
                .borrow()
                .current_path()
                .map_or_else(|| t("none").to_owned(), |path| path.display().to_string()),
        ),
        (
            t("Blocks").to_owned(),
            ctx.session.borrow().block_count().to_string(),
        ),
        (
            t("File size").to_owned(),
            ctx.session
                .borrow()
                .file_size()
                .map_or_else(|| t("not read").to_owned(), crate::session::format::bytes),
        ),
        (
            t("Theme").to_owned(),
            fill(
                "{theme} at {scale} %",
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
            t("Language").to_owned(),
            crate::i18n::current().label().to_owned(),
        ),
        (
            t("Renderer override").to_owned(),
            std::env::var("JADEFORGE_RENDERER").unwrap_or_else(|_| t("auto").to_owned()),
        ),
        (
            t("Remember window").to_owned(),
            if *remember_window.read() {
                t("yes").to_owned()
            } else {
                t("no").to_owned()
            },
        ),
    ];
    diagnostics.push((
        t("Backup folder").to_owned(),
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
        let mut text = format!("{}\n", t("JadeForge diagnostics"));
        let _ = writeln!(
            text,
            "{}",
            fill("Version: {version}", &[("version", crate::APP_VERSION)],)
        );
        let _ = writeln!(
            text,
            "{}",
            fill(
                "Save folder: {path}",
                &[("path", &save_folder.display().to_string())],
            )
        );
        if let Some(path) = ctx.session.borrow().current_path() {
            let _ = writeln!(
                text,
                "{}",
                fill("Savegame: {path}", &[("path", &path.display().to_string())]),
            );
        }
        let config = config_path.as_ref().map_or_else(
            || t("unavailable").to_owned(),
            |path| path.display().to_string(),
        );
        let _ = writeln!(text, "{}", fill("Config: {path}", &[("path", &config)]));
        let _ = writeln!(
            text,
            "{}",
            fill(
                "Theme: {theme} at {scale} %",
                &[
                    ("theme", theme::current().palette.kind.key()),
                    (
                        "scale",
                        &(theme::current().text_scale * 100.0).round().to_string(),
                    ),
                ],
            )
        );
        let _ = writeln!(
            text,
            "{}",
            fill(
                "Language: {language}",
                &[("language", crate::i18n::current().label())],
            )
        );
        text
    };
    let copy_diagnostics = {
        let ctx = ctx.clone();
        Action::new(move || {
            let copied = crate::clipboard::copy(diagnostics_text.clone());
            ctx.run(|session| {
                session.notify(if copied {
                    t("Copied the diagnostics to the clipboard.")
                } else {
                    t("Could not copy the diagnostics: the system clipboard is not available.")
                });
            });
        })
    };

    let mut settings = Vec::new();
    if save_folder.as_os_str().is_empty() {
        settings.push(widgets::hint(t(
            "No save folder has been chosen yet. Use Browse… or Ctrl+O to pick one.",
        )));
    } else {
        settings.push(path_section(
            ctx,
            t("Save folder"),
            &save_folder.display().to_string(),
            save_folder.clone(),
            "savegame",
        ));
    }
    settings.extend([
        widgets::label_row(
            t("Sort order"),
            widgets::a11y_text(fill(
                "{name}, {direction}",
                &[
                    ("name", sort.label()),
                    (
                        "direction",
                        if descending {
                            t("descending")
                        } else {
                            t("ascending")
                        },
                    ),
                ],
            ))
            .font_size(theme::font::label())
            .color(theme::text()),
        ),
        widgets::label_row(
            t("Game data"),
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
                t("Settings file"),
                &path.display().to_string(),
                folder,
                "settings",
            ));
        }
        None => settings.push(widgets::hint(t(
            "No config directory is available in this environment. JADEFORGE_CONFIG_DIR can point \
             at one for portable installs.",
        ))),
    }

    let mut backups = vec![widgets::hint(t(
        "Every save writes a copy of the previous save.sac next to it, named \
             save.sac.jadeforge-backup-<seconds>.<nanos>.<counter>. To undo a save, close \
             JadeForge and copy the backup over save.sac. An earlier backup is never overwritten.",
    ))];
    match &backup {
        Some(backup) => {
            let folder = backup
                .parent()
                .map_or_else(|| save_folder.clone(), std::path::Path::to_path_buf);
            backups.push(path_section(
                ctx,
                t("Latest backup"),
                &backup.display().to_string(),
                folder,
                "backup",
            ));
        }
        None => backups.push(widgets::hint(t(
            "No backup has been written in this session; saving a change creates one and shows \
             its exact path on the Overview tab.",
        ))),
    }

    let mut children = vec![
        rect()
            .horizontal()
            .width(Size::fill())
            .content(Content::Flex)
            .cross_align(Alignment::Center)
            .child(
                widgets::a11y_text(t("Jade Empire savegame editor"))
                    .font_size(theme::font::body())
                    .color(theme::muted()),
            )
            .child(rect().width(Size::flex(1.0)))
            .child(close_button)
            .into_element(),
        widgets::hint(t(
            "JadeForge reads and writes save.sac files directly. Every write is verified by \
             re-reading the produced file and is written atomically, and the game installation \
             is only ever read.",
        )),
        widgets::divider(),
        widgets::section_label(t("SETTINGS")),
        widgets::hint(t(
            "These preferences live in one TOML file. They are remembered between runs and can \
             be edited while JadeForge is closed.",
        )),
    ];
    children.extend(settings);
    children.push(widgets::divider());
    children.push(widgets::section_label(t("APPEARANCE")));
    children.extend(appearance);
    children.push(widgets::divider());
    children.push(widgets::section_label(t("DIAGNOSTICS")));
    children.extend(diagnostics_rows);
    children.push(
        widgets::outline_action(
            t("Copy the diagnostics to the clipboard"),
            t("Copy diagnostics"),
            copy_diagnostics,
        )
        .into_element(),
    );
    children.push(widgets::divider());
    children.push(widgets::section_label(t("LICENSES")));
    children.push(widgets::hint(t(
        "JadeForge is MIT licensed. The bundled English item and style names are game content \
         and are not covered by that license; see NOTICE and docs/NAMES.md.",
    )));
    children.push(widgets::divider());
    children.push(widgets::section_label(t("BACKUPS")));
    children.extend(backups);
    children.push(widgets::divider());
    children.push(widgets::section_label(t("KEYBOARD SHORTCUTS")));
    for (action, keys) in [
        (t("Save (opens the review)"), "Ctrl+S"),
        (t("Choose save folder"), "Ctrl+O"),
        (t("Focus the active filter"), "Ctrl+F"),
        (t("Undo the last edit"), "Ctrl+Z"),
        (t("Redo an undone edit"), "Ctrl+Y or Ctrl+Shift+Z"),
        (t("Scan the save folder"), "F5"),
        (t("Open this help page"), "Ctrl+/"),
        (t("Clear the active filter or close this page"), "Escape"),
    ] {
        children.push(widgets::label_row(
            action,
            widgets::a11y_text(keys).font_size(theme::font::label()),
        ));
    }

    widgets::card(
        t("About JadeForge"),
        Some(fill(
            "Version {version}",
            &[("version", crate::APP_VERSION)],
        )),
        children,
    )
}

/// A labelled path with wrapping text and copy/open actions.
fn path_section(
    ctx: &Ctx,
    title: &'static str,
    path: &str,
    folder: std::path::PathBuf,
    what: &'static str,
) -> Element {
    let value = path.to_owned();
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
