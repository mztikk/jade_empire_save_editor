//! The individual tabs of the editor.

use std::path::PathBuf;

use freya::prelude::*;

use crate::app::Ctx;
use crate::theme;
use crate::widgets::{self, AccessibleButton, Action};

pub mod about;
pub mod character;
pub mod gems;
pub mod henchmen;
pub mod inventory;
pub mod overview;
pub mod styles;

/// Copy and open actions for a path shown in a card.
///
/// The path is also rendered as a wrapping paragraph by the caller; these
/// buttons make it usable without selecting text by hand. `what` names the
/// path in the status and accessibility labels ("savegame", "backup", ...).
pub(crate) fn path_actions(ctx: &Ctx, path: &str, folder: PathBuf, what: &'static str) -> Element {
    let copy = {
        let ctx = ctx.clone();
        let path = path.to_owned();
        Action::new(move || {
            let message = match Clipboard::set(path.clone()) {
                Ok(()) => format!("Copied the {what} path to the clipboard."),
                Err(_) => format!(
                    "Could not copy the {what} path: the system clipboard is not available."
                ),
            };
            ctx.run(|session| session.notify(&message));
        })
    };
    let open = {
        let ctx = ctx.clone();
        Action::new(move || match crate::paths::open_folder(&folder) {
            Ok(()) => ctx.run(|session| {
                session.notify(&format!("Opened the {what} folder in the file manager."));
            }),
            Err(error) => ctx.run(|session| {
                session.set_error(&format!(
                    "Could not open the {what} folder {}: {error}",
                    folder.display()
                ));
            }),
        })
    };
    rect()
        .horizontal()
        .spacing(theme::space::MD)
        .content(Content::wrap_spacing(theme::space::MD))
        .child(path_button(
            format!("Copy the {what} path"),
            "Copy path",
            copy,
        ))
        .child(path_button(
            format!("Open the {what} folder"),
            "Open folder",
            open,
        ))
        .into_element()
}

/// One compact outlined button with an accessible name.
fn path_button(accessible: String, text: &'static str, action: Action) -> Element {
    AccessibleButton::new(
        accessible,
        action,
        rect()
            .padding(Gaps::new_symmetric(theme::space::XS, theme::space::MD))
            .child(
                widgets::a11y_text(text)
                    .font_size(theme::font::MICRO)
                    .color(theme::MUTED),
            ),
    )
    .hover_background(theme::SURFACE_HOVER)
    .into_element()
}
