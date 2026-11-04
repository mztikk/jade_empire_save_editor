//! The folder picker overlay.
//!
//! Freya has no native file dialog, and shelling out to a platform dialog
//! would add a dependency that cannot be tested headlessly. The picker is a
//! plain view instead: it walks the file system, can be typed into and is
//! covered by the UI tests.

use std::path::{Path, PathBuf};

use freya::prelude::*;

use crate::app::Ctx;
use crate::session::natural_order;
use crate::theme;
use crate::widgets;

/// Folders that can be entered from `path`, sorted the way a file manager
/// would sort them.
#[must_use]
pub fn subdirectories(path: &Path) -> Vec<PathBuf> {
    let mut folders: Vec<PathBuf> = std::fs::read_dir(path)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|candidate| candidate.is_dir())
        .collect();
    folders.sort_by(|left, right| {
        let left = left
            .file_name()
            .map(|name| name.to_string_lossy().into_owned());
        let right = right
            .file_name()
            .map(|name| name.to_string_lossy().into_owned());
        match (left, right) {
            (Some(left), Some(right)) => natural_order(&left, &right),
            _ => std::cmp::Ordering::Equal,
        }
    });
    folders
}

/// Home directory of the current user, when the platform provides one.
#[must_use]
pub fn home_directory() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
}

/// Path the picker should start at.
#[must_use]
pub fn start_directory(current: &str) -> String {
    if !current.trim().is_empty() && Path::new(current).is_dir() {
        return current.to_owned();
    }
    home_directory()
        .or_else(|| std::env::current_dir().ok())
        .map_or_else(String::new, |path| path.display().to_string())
}

/// Renders the overlay.
#[allow(clippy::too_many_lines)] // one declarative layout is easier to follow
pub fn view(
    ctx: &Ctx,
    path: State<String>,
    open: State<bool>,
    directory: State<String>,
) -> Element {
    let current = PathBuf::from(path.read().clone());
    let folders = subdirectories(&current);
    let readable = current.is_dir();

    let mut rows: Vec<Element> = Vec::new();
    if let Some(parent) = current.parent() {
        let mut path = path;
        let parent = parent.to_path_buf();
        rows.push(
            Button::new()
                .flat()
                .on_press(move |_| *path.write() = parent.display().to_string())
                .child(row_content("..", true))
                .into_element(),
        );
    }
    for folder in folders {
        let mut path = path;
        let label = folder.file_name().map_or_else(
            || folder.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        );
        let target = folder.display().to_string();
        rows.push(
            Button::new()
                .flat()
                .on_press(move |_| (*path.write()).clone_from(&target))
                .child(row_content(&label, true))
                .into_element(),
        );
    }
    if rows.is_empty() {
        rows.push(widgets::hint(if readable {
            "This folder has no subfolders."
        } else {
            "Cannot read this folder."
        }));
    }

    let home = {
        let mut path = path;
        move |_| {
            if let Some(home) = home_directory() {
                *path.write() = home.display().to_string();
            }
        }
    };
    let cancel = {
        let mut open = open;
        move |_| *open.write() = false
    };
    let choose = {
        let ctx = ctx.clone();
        let mut open = open;
        let mut directory = directory;
        move |_| {
            let folder = PathBuf::from(path.read().clone());
            if !folder.is_dir() {
                ctx.run(|session| {
                    session.set_error("That folder does not exist.");
                });
                return;
            }
            *directory.write() = folder.display().to_string();
            ctx.run(|session| session.set_directory(folder));
            *open.write() = false;
        }
    };

    rect()
        .position(Position::new_absolute())
        .layer(Layer::Overlay)
        .width(Size::percent(100.))
        .height(Size::percent(100.))
        .center()
        .background(Color::from_argb(170, 0, 0, 0))
        .child(
            rect()
                .vertical()
                .width(Size::percent(60.))
                .max_width(Size::px(620.))
                .height(Size::percent(72.))
                .padding(Gaps::new_all(18.))
                .spacing(12.)
                .background(theme::PANEL)
                .border(Border::new().fill(theme::BORDER).width(1.))
                .corner_radius(12.)
                .content(Content::Flex)
                .child(
                    rect()
                        .horizontal()
                        .width(Size::fill())
                        .spacing(10.)
                        .content(Content::Flex)
                        .child(
                            label()
                                .text("Choose the save folder")
                                .font_size(16.)
                                .font_weight(FontWeight::BOLD)
                                .color(theme::TEXT),
                        )
                        .child(rect().width(Size::flex(1.0)))
                        .child(Button::new().flat().compact().child("Home").on_press(home)),
                )
                .child(
                    label()
                        .text("The folder that contains one sub-folder per savegame.")
                        .font_size(10.)
                        .color(theme::DIM),
                )
                .child(Input::new(path).width(Size::fill()))
                .child(
                    ScrollView::new()
                        .width(Size::fill())
                        .height(Size::flex(1.0))
                        .show_scrollbar(true)
                        .child(
                            rect()
                                .vertical()
                                .width(Size::fill())
                                .spacing(2.)
                                .children(rows),
                        ),
                )
                .child(
                    rect()
                        .horizontal()
                        .width(Size::fill())
                        .spacing(10.)
                        .content(Content::Flex)
                        .child(Button::new().flat().child("Cancel").on_press(cancel))
                        .child(rect().width(Size::flex(1.0)))
                        .child(
                            Button::new()
                                .filled()
                                .child("Use this folder")
                                .on_press(choose),
                        ),
                ),
        )
        .into_element()
}

fn row_content(text: &str, folder: bool) -> Element {
    rect()
        .horizontal()
        .width(Size::fill())
        .spacing(8.)
        .padding(Gaps::new_symmetric(6., 10.))
        .corner_radius(6.)
        .background(theme::SURFACE)
        .child(
            label()
                .text(if folder { "▸ " } else { "" })
                .font_size(11.)
                .color(theme::DIM),
        )
        .child(
            label()
                .text(text.to_owned())
                .font_size(12.)
                .max_lines(1)
                .color(theme::TEXT),
        )
        .into_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subdirectories_are_sorted_naturally() {
        let root = std::env::temp_dir().join(format!("jadeforge-picker-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for name in ["Auto 10", "Auto 2", "Auto 1"] {
            std::fs::create_dir_all(root.join(name)).expect("create fixture");
        }
        std::fs::write(root.join("file.txt"), b"not a folder").expect("write file");
        let names: Vec<String> = subdirectories(&root)
            .iter()
            .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["Auto 1", "Auto 2", "Auto 10"]);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn start_directory_falls_back_to_a_real_folder() {
        let current = start_directory("/definitely/not/a/folder");
        assert!(Path::new(&current).is_dir(), "{current}");
    }
}
