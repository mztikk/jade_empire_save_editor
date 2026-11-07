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
use crate::i18n::{fill, keys, t};
use crate::message::capitalize;
use crate::theme;
use crate::widgets::{self, AccessibleButton, Action};

/// Renders the backup browser.
#[must_use]
pub fn view(ctx: &Ctx, close: Rc<dyn Fn()>, sync: &Rc<dyn Fn()>) -> Element {
    let backups = ctx.session.borrow().backups();
    let dirty = ctx.session.borrow().is_dirty();
    let open = ctx.session.borrow().current_path().map_or_else(
        || t(keys::SHELL_NO_SAVEGAME_OPEN).to_owned(),
        |path| path.display().to_string(),
    );

    let mut children: Vec<Element> = vec![widgets::hint(fill(
        keys::BACKUPS_EXPLANATION,
        &[("open", &open)],
    ))];
    if dirty {
        children.push(widgets::hint(t(keys::BACKUPS_DIRTY_HINT)));
    }
    children.push(widgets::divider());
    if backups.is_empty() {
        children.push(widgets::hint(t(keys::BACKUPS_EMPTY_HINT)));
    } else {
        let heading = widgets::plural(
            backups.len(),
            keys::COUNT_BACKUP_ONE,
            keys::COUNT_BACKUP_OTHER,
        );
        children.push(widgets::section_label(heading.to_uppercase()));
        for backup in backups {
            children.push(backup_row(ctx, &backup, dirty, sync));
        }
    }
    children.push(widgets::divider());
    children.push(
        widgets::ActionButton::new(
            t(keys::BACKUPS_CLOSE),
            t(keys::BACKUPS_BACK),
            Action::new(move || close()),
        )
        .into_element(),
    );

    widgets::card(
        capitalize(t(keys::COUNT_BACKUP_OTHER)),
        Some(t(keys::BACKUPS_UTC_HINT).to_owned()),
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
    let restore = if dirty {
        rect()
            .padding(Gaps::new_symmetric(theme::space::xs(), theme::space::md()))
            .border(
                Border::new()
                    .fill(theme::dim())
                    .width(theme::stroke::hairline()),
            )
            .corner_radius(theme::radius::control())
            .a11y_role(AccessibilityRole::Button)
            .a11y_alt(fill(
                keys::BACKUPS_RESTORE_ACTION,
                &[("timestamp", &backup.timestamp())],
            ))
            .a11y_builder(accesskit::Node::set_disabled)
            .child(
                widgets::a11y_text(t(keys::BACKUPS_RESTORE))
                    .font_size(theme::font::micro())
                    .color(theme::dim()),
            )
            .into_element()
    } else {
        let restore = {
            let ctx = ctx.clone();
            let path = path.clone();
            let sync = sync.clone();
            Action::new(move || {
                if !crate::dialog::confirm(
                    t(keys::BACKUPS_RESTORE_TITLE),
                    t(keys::BACKUPS_RESTORE_CONFIRM),
                ) {
                    return;
                }
                let result = ctx.run(|session| session.restore_backup(&path));
                if result.is_ok() {
                    sync();
                }
            })
        };
        AccessibleButton::new(
            fill(
                keys::BACKUPS_RESTORE_ACTION,
                &[("timestamp", &backup.timestamp())],
            ),
            restore,
            rect()
                .padding(Gaps::new_symmetric(theme::space::xs(), theme::space::md()))
                .border(
                    Border::new()
                        .fill(theme::control_border())
                        .width(theme::stroke::hairline()),
                )
                .corner_radius(theme::radius::control())
                .child(
                    widgets::a11y_text(t(keys::BACKUPS_RESTORE))
                        .font_size(theme::font::micro())
                        .color(theme::text()),
                ),
        )
        .into_element()
    };

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
                keys::BACKUPS_ORIGINAL,
                &[("path", &backup.original.display().to_string())],
            ))
            .font_size(theme::font::micro())
            .color(theme::muted())
            .max_lines(2),
        )
        .into_element()
}
