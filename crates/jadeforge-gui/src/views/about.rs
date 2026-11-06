//! About, help and the remembered settings.
//!
//! The window has no separate settings dialog: everything that persists is
//! small enough to show on one page next to the version and the backup rules,
//! which is also the place a user looks when they need to find the config file.

use freya::prelude::*;

use crate::app::Ctx;
use crate::session::{GameStatus, SaveSort};
use crate::theme;
use crate::views::path_actions;
use crate::widgets::{self, AccessibleButton, Action};

/// Renders the About / help surface.
#[allow(clippy::too_many_lines)] // one declarative page keeps the help text together
pub fn view(ctx: &Ctx, sort: State<(SaveSort, bool)>, mut show_about: State<bool>) -> Element {
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
    let close_button = AccessibleButton::new(
        "Close the About page",
        close,
        rect()
            .padding(Gaps::new_symmetric(theme::space::XS, theme::space::MD))
            .child(
                widgets::a11y_text("Close")
                    .font_size(theme::font::MICRO)
                    .color(theme::MUTED),
            ),
    )
    .hover_background(theme::SURFACE_HOVER);

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
            .font_size(theme::font::LABEL)
            .color(theme::TEXT),
        ),
        widgets::label_row(
            "Game data",
            widgets::a11y_text(game_text)
                .font_size(theme::font::LABEL)
                .color(theme::TEXT),
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
                    .font_size(theme::font::BODY)
                    .color(theme::MUTED),
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
            widgets::a11y_text(keys).font_size(theme::font::LABEL),
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
        .spacing(theme::space::XS)
        .child(
            widgets::a11y_text(title)
                .font_size(theme::font::LABEL)
                .color(theme::MUTED),
        )
        .child(
            paragraph()
                .width(Size::fill())
                .max_width(Size::px(theme::layout::TEXT_MEASURE))
                .font_size(theme::font::LABEL)
                .color(theme::TEXT)
                .a11y_builder(move |node| node.set_value(value))
                .span(path.to_owned()),
        )
        .child(path_actions(ctx, path, folder, what))
        .into_element()
}
