//! About, help and the remembered settings.
//!
//! The window has no separate settings dialog: everything that persists is
//! small enough to show on one page next to the version and the backup rules,
//! which is also the place a user looks when they need to find the config file.

use freya::prelude::*;

use crate::app::Ctx;
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
) -> Element {
    let (sort, descending) = *sort.read();
    let save_folder = ctx.session.borrow().directory().to_path_buf();
    let game_text = match ctx.session.borrow().game_status() {
        GameStatus::Loading => "Reading the game installation…".to_owned(),
        GameStatus::Loaded { root, .. } => root.display().to_string(),
        GameStatus::NamesOnly { .. } => "Bundled English names (no installation)".to_owned(),
        GameStatus::Missing => "No installation configured".to_owned(),
        GameStatus::Error(error) => format!("Unreadable: {error}"),
    };
    let backup = ctx
        .session
        .borrow()
        .last_backup()
        .map(std::path::Path::to_path_buf);
    let config_path = crate::config::Config::path();

    let close = Action::new(move || show_about.set(false));
    let close_button = widgets::outline_action("Close the About page", "Close", close);

    let selected_theme = *theme_choice.read();
    let selected_scale = *text_scale.read();
    let mut appearance: Vec<Element> = Vec::new();
    appearance.push(widgets::hint(
        "The theme and text size apply immediately and are remembered in the settings file. \
         System follows the operating system preference; high contrast can be selected here \
         because the windowing layer does not report the platform's forced-colours setting.",
    ));
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
            .child(widgets::section_label("THEME"))
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
            format!("Use {percent} percent text size"),
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
            .child(widgets::section_label("TEXT SIZE"))
            .children(scale_chips)
            .into_element(),
    );

    let mut settings = Vec::new();
    if save_folder.as_os_str().is_empty() {
        settings.push(widgets::hint(
            "No save folder has been chosen yet. Use Browse… or Ctrl+O to pick one.",
        ));
    } else {
        settings.push(path_section(
            ctx,
            "Save folder",
            &save_folder.display().to_string(),
            save_folder.clone(),
            "savegame",
        ));
    }
    settings.extend([
        widgets::label_row(
            "Sort order",
            widgets::a11y_text(format!(
                "{}, {}",
                sort.label(),
                if descending {
                    "descending"
                } else {
                    "ascending"
                }
            ))
            .font_size(theme::font::label())
            .color(theme::text()),
        ),
        widgets::label_row(
            "Game data",
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
                "Settings file",
                &path.display().to_string(),
                folder,
                "settings",
            ));
        }
        None => settings.push(widgets::hint(
            "No config directory is available in this environment. JADEFORGE_CONFIG_DIR can point \
             at one for portable installs.",
        )),
    }

    let mut backups = vec![widgets::hint(
        "Every save writes a copy of the previous save.sac next to it, named \
             save.sac.jadeforge-backup-<seconds>.<nanos>.<counter>. To undo a save, close \
             JadeForge and copy the backup over save.sac. An earlier backup is never overwritten.",
    )];
    match &backup {
        Some(backup) => {
            let folder = backup
                .parent()
                .map_or_else(|| save_folder.clone(), std::path::Path::to_path_buf);
            backups.push(path_section(
                ctx,
                "Latest backup",
                &backup.display().to_string(),
                folder,
                "backup",
            ));
        }
        None => backups.push(widgets::hint(
            "No backup has been written in this session; saving a change creates one and shows \
             its exact path on the Overview tab.",
        )),
    }

    let mut children = vec![
        rect()
            .horizontal()
            .width(Size::fill())
            .content(Content::Flex)
            .cross_align(Alignment::Center)
            .child(
                widgets::a11y_text("Jade Empire savegame editor")
                    .font_size(theme::font::body())
                    .color(theme::muted()),
            )
            .child(rect().width(Size::flex(1.0)))
            .child(close_button)
            .into_element(),
        widgets::hint(
            "JadeForge reads and writes save.sac files directly. Every write is verified by \
             re-reading the produced file and is written atomically, and the game installation \
             is only ever read.",
        ),
        widgets::divider(),
        widgets::section_label("SETTINGS"),
        widgets::hint(
            "These preferences live in one TOML file. They are remembered between runs and can \
             be edited while JadeForge is closed.",
        ),
    ];
    children.extend(settings);
    children.push(widgets::divider());
    children.push(widgets::section_label("APPEARANCE"));
    children.extend(appearance);
    children.push(widgets::divider());
    children.push(widgets::section_label("BACKUPS"));
    children.extend(backups);
    children.push(widgets::divider());
    children.push(widgets::section_label("KEYBOARD SHORTCUTS"));
    for (action, keys) in [
        ("Save", "Ctrl+S"),
        ("Choose save folder", "Ctrl+O"),
        ("Focus the active filter", "Ctrl+F"),
        ("Scan the save folder", "F5"),
        ("Clear the active filter or close this page", "Escape"),
    ] {
        children.push(widgets::label_row(
            action,
            widgets::a11y_text(keys).font_size(theme::font::label()),
        ));
    }

    widgets::card(
        "About JadeForge",
        Some(format!("Version {}", crate::APP_VERSION)),
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
