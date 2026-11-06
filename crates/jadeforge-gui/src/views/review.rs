//! Pending-changes review before a savegame is written.
//!
//! The footer's Save action opens this page instead of writing immediately. It
//! lists every field and row the pending actions changed, grouped by the action
//! that changed them, so the user can check the exact edit before the verified
//! write and its backup happen.

use std::rc::Rc;

use freya::prelude::*;

use crate::form::Form;
use crate::session::PendingChangeGroup;
use crate::theme;
use crate::widgets;

use crate::app::Ctx;
use crate::i18n::{fill, t};

/// Renders the review page.
#[must_use]
pub fn view(ctx: &Ctx, form: Form, close: Rc<dyn Fn()>, sync: &Rc<dyn Fn()>) -> Element {
    let dirty = ctx.session.borrow().is_dirty();
    let groups = ctx.session.borrow().pending_changes();
    let total: usize = groups.iter().map(|group| group.changes.len()).sum();
    let invalid = ctx.session.borrow().form_errors(&form.read());

    let mut children: Vec<Element> = Vec::new();
    children.push(widgets::hint(t("review.explanation")));
    if !invalid.is_empty() {
        children.push(widgets::hint(fill(
            "review.invalid_fields",
            &[("count", &invalid.len().to_string())],
        )));
    }
    children.push(widgets::divider());
    if groups.is_empty() {
        children.push(widgets::hint(if dirty {
            t("review.unsaved_no_fields")
        } else {
            t("review.nothing")
        }));
    } else {
        for (index, group) in groups.iter().enumerate() {
            children.push(group_block(index, group));
        }
    }

    let write = {
        let ctx = ctx.clone();
        let close = close.clone();
        let sync = sync.clone();
        move |_| {
            let invalid = ctx.session.borrow().form_errors(&form.read());
            let saved = ctx.run(|session| session.save_applied(&form.read()));
            if saved.is_ok() {
                // An invalid field keeps its text so the user can fix it; a
                // valid form is normalised to the values that were written.
                if invalid.is_empty() {
                    sync();
                }
                close();
            }
        }
    };
    let cancel = move |_| close();

    children.push(widgets::divider());
    children.push(
        rect()
            .horizontal()
            .width(Size::fill())
            .spacing(theme::space::md())
            .content(Content::wrap_spacing(theme::space::md()))
            .child(
                Button::new()
                    .outline()
                    .child(t("backups.back"))
                    .on_press(cancel),
            )
            .maybe(dirty, |el| {
                el.child(
                    Button::new()
                        .filled()
                        .child(t("review.write"))
                        .on_press(write),
                )
            })
            .into_element(),
    );

    widgets::card(
        t("review.title"),
        Some(fill(
            "review.summary",
            &[
                (
                    "changes",
                    &widgets::plural(total, "count.change.one", "count.change.other"),
                ),
                (
                    "groups",
                    &widgets::plural(groups.len(), "count.group.one", "count.group.other"),
                ),
            ],
        )),
        children,
    )
}

/// One action group: its label and the fields it changed.
fn group_block(index: usize, group: &PendingChangeGroup) -> Element {
    let mut rows: Vec<Element> = Vec::new();
    for (row, change) in group.changes.iter().enumerate() {
        rows.push(
            rect()
                .vertical()
                .width(Size::fill())
                .spacing(theme::space::xxs())
                .key(row)
                .padding(Gaps::new_symmetric(theme::space::xs(), theme::space::sm()))
                .child(
                    widgets::a11y_text(change.field.clone())
                        .font_size(theme::font::label())
                        .color(theme::text())
                        .max_lines(2),
                )
                .child(
                    widgets::a11y_text(fill(
                        "review.change",
                        &[("before", &change.before), ("after", &change.after)],
                    ))
                    .font_size(theme::font::micro())
                    .color(theme::muted()),
                )
                .into_element(),
        );
    }
    rect()
        .vertical()
        .width(Size::fill())
        .spacing(theme::space::xs())
        .key(index)
        .child(widgets::section_label(group.label.to_uppercase()))
        .child(
            rect()
                .vertical()
                .width(Size::fill())
                .spacing(theme::space::xxs())
                .children(rows)
                .into_element(),
        )
        .into_element()
}
