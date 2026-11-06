//! The backup browser: every backup of the open savegame with its timestamp,
//! original path and a restore action.
//!
//! Restoring is a first-class action rather than an instruction to copy files by
//! hand. The session refuses while there are unsaved edits, the core verifies
//! both files, and the current file is backed up before the chosen backup
//! replaces it, so the restore itself can be undone.

use std::path::PathBuf;
use std::rc::Rc;

use freya::prelude::*;

use crate::app::Ctx;
use crate::i18n::{fill, t};
use crate::message::capitalize;
use crate::theme;
use crate::widgets::{self, AccessibleButton, Action};

/// Renders the backup browser.
#[must_use]
pub fn view(ctx: &Ctx, close: Rc<dyn Fn()>, sync: &Rc<dyn Fn()>) -> Element {
    let backups = ctx.session.borrow().backups();
    let dirty = ctx.session.borrow().is_dirty();
    let open = ctx.session.borrow().current_path().map_or_else(
        || t("no savegame open").to_owned(),
        |path| path.display().to_string(),
    );

    let mut children: Vec<Element> = vec![widgets::hint(fill(
        "Backups of the open savegame, newest first. A restore verifies the chosen file, keeps a \
         new backup of the file it replaces, and writes atomically. Open savegame: {open}",
        &[("open", &open)],
    ))];
    if dirty {
        children.push(widgets::hint(t(
            "Save or revert the unsaved changes before restoring a backup: a restore would \
             otherwise silently discard them.",
        )));
    }
    children.push(widgets::divider());
    if backups.is_empty() {
        children.push(widgets::hint(t(
            "No backups were found next to this savegame. The first successful save creates one.",
        )));
    } else {
        let heading = widgets::plural(backups.len(), "backup", "backups");
        children.push(widgets::section_label(heading.to_uppercase()));
        for backup in backups {
            children.push(backup_row(ctx, &backup, dirty, sync));
        }
    }
    children.push(widgets::divider());
    children.push(
        widgets::outline_action(
            t("Close the backup browser"),
            t("Back to editing"),
            Action::new(move || close()),
        )
        .into_element(),
    );

    widgets::card(
        capitalize(t("backups")),
        Some(t("timestamps are UTC").to_owned()),
        children,
    )
}

/// One backup row with its timestamp, original path and restore action.
fn backup_row(
    ctx: &Ctx,
    backup: &crate::session::BackupRow,
    dirty: bool,
    sync: &Rc<dyn Fn()>,
) -> Element {
    let path: PathBuf = backup.path.clone();
    let restore = {
        let ctx = ctx.clone();
        let path = path.clone();
        let sync = sync.clone();
        Action::new(move || {
            if !crate::dialog::confirm(
                t("Restore this backup?"),
                t(
                    "The current savegame is replaced by the chosen backup. JadeForge first writes a \
                 new backup of the current file, so the restore can be undone.",
                ),
            ) {
                return;
            }
            let result = ctx.run(|session| session.restore_backup(&path));
            if result.is_ok() {
                sync();
            }
        })
    };
    let restore = AccessibleButton::new(
        fill(
            "Restore the backup from {timestamp}",
            &[("timestamp", &backup.timestamp())],
        ),
        restore,
        rect()
            .padding(Gaps::new_symmetric(theme::space::xs(), theme::space::md()))
            .border(
                Border::new()
                    .fill(if dirty {
                        theme::dim()
                    } else {
                        theme::control_border()
                    })
                    .width(theme::stroke::hairline()),
            )
            .corner_radius(theme::radius::control())
            .child(
                widgets::a11y_text(t("Restore"))
                    .font_size(theme::font::micro())
                    .color(if dirty { theme::dim() } else { theme::text() }),
            ),
    )
    .focusable(!dirty);

    rect()
        .vertical()
        .width(Size::fill())
        .spacing(theme::space::xxs())
        .key(backup.path.display().to_string())
        .padding(Gaps::new_symmetric(theme::space::xs(), theme::space::sm()))
        .child(
            rect()
                .horizontal()
                .width(Size::fill())
                .spacing(theme::space::md())
                .content(Content::Flex)
                .cross_align(Alignment::Center)
                .child(
                    widgets::a11y_text(format!(
                        "{} · {}",
                        backup.timestamp(),
                        crate::session::format::bytes(backup.size)
                    ))
                    .font_size(theme::font::label())
                    .color(theme::text()),
                )
                .child(rect().width(Size::flex(1.0)))
                .child(restore)
                .into_element(),
        )
        .child(
            widgets::a11y_text(fill(
                "Original: {path}",
                &[("path", &backup.original.display().to_string())],
            ))
            .font_size(theme::font::micro())
            .color(theme::muted())
            .max_lines(2),
        )
        .into_element()
}
