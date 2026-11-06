//! The individual tabs of the editor.

use std::path::PathBuf;

use freya::prelude::*;

use crate::app::Ctx;
use crate::i18n::{fill, t};
use crate::theme;
use crate::widgets::{self, AccessibleButton, Action};

pub mod about;
pub mod backups;
pub mod character;
pub mod gems;
pub mod henchmen;
pub mod inventory;
pub mod overview;
pub mod review;
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
            let what = t(what);
            let message = if crate::clipboard::copy(path.clone()) {
                fill("path.copied", &[("what", what)])
            } else {
                fill("path.copy_failed", &[("what", what)])
            };
            ctx.run(|session| session.notify(&message));
        })
    };
    let open = {
        let ctx = ctx.clone();
        Action::new(move || match crate::paths::open_folder(&folder) {
            Ok(()) => ctx.run(|session| {
                session.notify(&fill("path.opened", &[("what", t(what))]));
            }),
            Err(error) => ctx.run(|session| {
                session.set_error(&fill(
                    "path.open_failed",
                    &[
                        ("what", t(what)),
                        ("folder", &folder.display().to_string()),
                        ("error", &error.to_string()),
                    ],
                ));
            }),
        })
    };
    rect()
        .horizontal()
        .spacing(theme::space::md())
        .content(Content::wrap_spacing(theme::space::md()))
        .child(path_button(
            fill("path.copy_action", &[("what", t(what))]),
            t("path.copy"),
            copy,
        ))
        .child(path_button(
            fill("path.open_action", &[("what", t(what))]),
            t("path.open"),
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
            .padding(Gaps::new_symmetric(theme::space::xs(), theme::space::md()))
            .child(
                widgets::a11y_text(text)
                    .font_size(theme::font::micro())
                    .color(theme::muted()),
            ),
    )
    .hover_background(theme::surface_hover())
    .into_element()
}
