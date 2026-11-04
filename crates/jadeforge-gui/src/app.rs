//! The root component and the application shell.
//!
//! The shell owns the navigation (save list, tabs, actions) and renders the
//! active tab. All behaviour lives in [`crate::session::Session`].

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use freya::elements::image::ImageHandle;
use freya::engine::prelude::AlphaType;
use freya::prelude::*;
use jadeforge_core::Character;

use crate::form::Form;
use crate::session::{CharacterForm, GameStatus, MessageKind, Preview, Session, default_save_dir};
use crate::theme;
use crate::views;

/// Shared handles that every view needs.
#[derive(Clone)]
pub struct Ctx {
    /// The editing session.
    pub session: Rc<RefCell<Session>>,
    /// Bumped after every mutation to trigger a re-render.
    pub revision: State<u32>,
    /// The character form state.
    pub form: Form,
}

impl Ctx {
    /// Runs an action on the session and re-renders.
    pub fn run<R>(&self, action: impl FnOnce(&mut Session) -> R) -> R {
        let result = {
            let mut session = self.session.borrow_mut();
            action(&mut session)
        };
        self.bump();
        result
    }

    /// Requests a re-render.
    pub fn bump(&self) {
        let mut revision = self.revision;
        *revision.write() += 1;
    }
}

/// The tabs of the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    /// Summary of the savegame.
    Overview,
    /// Character sheet.
    Character,
    /// Fighting styles.
    Styles,
    /// Inventory.
    Inventory,
    /// Companions.
    Companions,
}

impl Tab {
    const ALL: [Self; 5] = [
        Self::Overview,
        Self::Character,
        Self::Styles,
        Self::Inventory,
        Self::Companions,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Character => "Character",
            Self::Styles => "Styles",
            Self::Inventory => "Inventory",
            Self::Companions => "Companions",
        }
    }
}

/// Root component of the application.
pub fn app() -> impl IntoElement {
    app_with_directory(default_save_dir())
}

/// Root component that starts with a specific save folder.
#[allow(clippy::too_many_lines)] // the shell wires all views in one place
pub fn app_with_directory(directory: PathBuf) -> impl IntoElement {
    use_init_theme(theme::theme);

    let session: Rc<RefCell<Session>> =
        use_hook(move || Rc::new(RefCell::new(Session::with_directory(directory))));
    let revision = use_state(|| 0_u32);
    let form = Form::new();
    let directory_state = use_state(|| session.borrow().directory.display().to_string());
    let tab = use_state(|| Tab::Overview);
    let item_query = use_state(String::new);
    let style_query = use_state(String::new);
    let preview_cache: Rc<RefCell<Option<(u64, ImageHandle)>>> =
        use_hook(|| Rc::new(RefCell::new(None)));

    // Subscribe to session changes.
    let _ = *revision.read();

    let ctx = Ctx {
        session: session.clone(),
        revision,
        form,
    };

    // Snapshot everything the shell needs so the borrow stays short.
    let entries = session.borrow().entries.clone();
    let selected = session.borrow().selected;
    let message = session.borrow().message.clone();
    let character = session.borrow().character.clone();
    let items = session.borrow().items.clone();
    let styles = session.borrow().styles.clone();
    let henchmen = session.borrow().henchmen.clone();
    let dirty = session.borrow().is_dirty();
    let preview = session.borrow().preview.clone();
    let game_status = session.borrow().game_status.clone();
    let has_save = session.borrow().save.is_some();

    let form_differs = character
        .as_ref()
        .is_some_and(|character| form.differs_from(character));
    let preview_handle = cached_preview(&preview_cache, preview.as_ref());
    let game_ready = matches!(game_status, GameStatus::Loaded { .. });
    let item_results = {
        let query = item_query.read().clone();
        session.borrow().search_items(&query, 8)
    };
    let style_matches = {
        let query = style_query.read().trim().to_lowercase();
        let missing = session.borrow().missing_styles();
        if query.is_empty() {
            Vec::new()
        } else {
            missing
                .into_iter()
                .filter(|(_, name, label)| {
                    name.to_lowercase().contains(&query) || label.to_lowercase().contains(&query)
                })
                .collect()
        }
    };

    let sync: Rc<dyn Fn()> = {
        let session = session.clone();
        Rc::new(move || {
            let character = session.borrow().character.clone();
            if let Some(character) = character {
                form.load(&CharacterForm::from_character(&character));
            }
        })
    };

    let active_tab = *tab.read();
    let content = match (active_tab, &character) {
        (Tab::Overview, Some(character)) => views::overview::view(
            &ctx,
            character,
            preview_handle.clone(),
            items.len(),
            styles.len(),
        ),
        (Tab::Character, Some(character)) => views::character::view(&ctx, character),
        (Tab::Styles, _) => {
            views::styles::view(&ctx, &styles, style_query, &style_matches, game_ready)
        }
        (Tab::Inventory, _) => {
            views::inventory::view(&ctx, &items, item_query, &item_results, game_ready)
        }
        (Tab::Companions, _) => views::henchmen::view(&ctx, &henchmen),
        (_, None) => empty_state(entries.len()),
    };

    let header = header(
        &ctx,
        directory_state,
        &message,
        &game_status,
        dirty,
        form_differs,
        sync.clone(),
    );
    let sidebar = sidebar(
        &ctx,
        &entries,
        selected,
        preview_handle,
        character.as_ref(),
        form_differs,
        sync.clone(),
    );
    let tabs = tab_bar(tab, active_tab);
    let footer = footer(&ctx, form, dirty || form_differs, form_differs, sync);

    rect()
        .vertical()
        .width(Size::fill())
        .height(Size::fill())
        .background(theme::BACKGROUND)
        .color(theme::TEXT)
        .content(Content::Flex)
        .child(header)
        .child(
            rect()
                .horizontal()
                .width(Size::fill())
                .height(Size::flex(1.0))
                .child(sidebar)
                .child(
                    rect()
                        .vertical()
                        .width(Size::flex(1.0))
                        .height(Size::fill())
                        .content(Content::Flex)
                        .child(tabs)
                        .child(
                            ScrollView::new()
                                .show_scrollbar(true)
                                .width(Size::fill())
                                .height(Size::flex(1.0))
                                .child(
                                    rect()
                                        .vertical()
                                        .width(Size::fill())
                                        .padding(Gaps::new(16., 18., 16., 18.))
                                        .child(if has_save {
                                            content
                                        } else {
                                            empty_state(entries.len())
                                        }),
                                ),
                        ),
                ),
        )
        .child(footer)
}

/// Builds an [`ImageHandle`] for the preview, cached by generation.
fn cached_preview(
    cache: &Rc<RefCell<Option<(u64, ImageHandle)>>>,
    preview: Option<&Preview>,
) -> Option<ImageHandle> {
    let preview = preview?;
    let mut cache = cache.borrow_mut();
    if let Some((generation, handle)) = cache.as_ref() {
        if *generation == preview.generation {
            return Some(handle.clone());
        }
    }
    let handle = ImageHandle::from_rgba(
        preview.width,
        preview.height,
        Bytes::copy_from_slice(&preview.rgba),
        AlphaType::Opaque,
    )?;
    *cache = Some((preview.generation, handle.clone()));
    Some(handle)
}

#[allow(clippy::too_many_arguments, clippy::needless_pass_by_value)]
fn header(
    ctx: &Ctx,
    directory: State<String>,
    message: &crate::session::Message,
    game_status: &GameStatus,
    dirty: bool,
    form_differs: bool,
    sync: Rc<dyn Fn()>,
) -> Element {
    let status_color = match message.kind {
        MessageKind::Info => theme::MUTED,
        MessageKind::Success => theme::SUCCESS,
        MessageKind::Error => theme::DANGER,
    };
    let (game_text, game_color) = match game_status {
        GameStatus::Loaded { items, .. } => (format!("{items} items loaded"), theme::JADE),
        GameStatus::Missing => ("game data not found".to_owned(), theme::DIM),
        GameStatus::Error(_) => ("game data unreadable".to_owned(), theme::DANGER),
    };

    let scan = {
        let ctx = ctx.clone();
        move |_| {
            if form_differs {
                ctx.run(|session| {
                    session.set_error("Apply or discard the character changes before scanning.");
                });
                return;
            }
            let path = PathBuf::from(directory.read().as_str());
            ctx.run(|session| session.set_directory(path));
            sync();
        }
    };

    rect()
        .vertical()
        .width(Size::fill())
        .padding(Gaps::new(14., 18., 12., 18.))
        .spacing(12.)
        .background(theme::PANEL)
        .child(
            rect()
                .horizontal()
                .width(Size::fill())
                .spacing(12.)
                .content(Content::Flex)
                .child(
                    label()
                        .text("JadeForge")
                        .font_size(22.)
                        .font_weight(FontWeight::BOLD)
                        .color(theme::ACCENT),
                )
                .child(
                    label()
                        .text("Jade Empire save editor")
                        .font_size(11.)
                        .color(theme::DIM),
                )
                .child(rect().width(Size::flex(1.0)))
                .child(if dirty {
                    crate::widgets::badge("unsaved changes", theme::SURFACE_ACTIVE, theme::ACCENT)
                } else if form_differs {
                    crate::widgets::badge("unapplied input", theme::SURFACE_ACTIVE, theme::ACCENT)
                } else {
                    crate::widgets::badge("all changes saved", theme::SURFACE, theme::DIM)
                })
                .child(crate::widgets::badge(game_text, theme::SURFACE, game_color)),
        )
        .child(
            rect()
                .horizontal()
                .width(Size::fill())
                .spacing(10.)
                .content(Content::Flex)
                .child(
                    label()
                        .text("SAVE FOLDER")
                        .font_size(10.)
                        .font_weight(FontWeight::BOLD)
                        .color(theme::DIM),
                )
                .child(
                    Input::new(directory)
                        .placeholder("Path to the Jade Empire Save folder")
                        .width(Size::flex(1.0)),
                )
                .child(Button::new().outline().child("Scan").on_press(scan)),
        )
        .child(
            label()
                .text(message.text.clone())
                .font_size(11.)
                .max_lines(2)
                .color(status_color),
        )
        .into_element()
}

#[allow(clippy::too_many_lines, clippy::needless_pass_by_value)]
#[allow(clippy::too_many_arguments)]
fn sidebar(
    ctx: &Ctx,
    entries: &[crate::session::SaveEntry],
    selected: Option<usize>,
    preview: Option<ImageHandle>,
    character: Option<&Character>,
    form_differs: bool,
    sync: Rc<dyn Fn()>,
) -> Element {
    let mut rows: Vec<Element> = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        let ctx = ctx.clone();
        let sync = sync.clone();
        let is_selected = selected == Some(index);
        let broken = entry.error.is_some();
        let summary = entry.summary();
        let title = entry.label.clone();
        rows.push(
            Button::new()
                .flat()
                .on_press(move |_| {
                    if form_differs && selected != Some(index) {
                        ctx.run(|session| {
                            session.set_error(
                                "Apply or discard the character changes before switching savegames.",
                            );
                        });
                        return;
                    }
                    ctx.run(|session| {
                        session.open(index);
                    });
                    sync();
                })
                .child(
                    rect()
                        .vertical()
                        .width(Size::fill())
                        .spacing(2.)
                        .padding(Gaps::new_symmetric(7., 10.))
                        .corner_radius(8.)
                        .background(if is_selected {
                            theme::SURFACE_ACTIVE
                        } else {
                            theme::SURFACE
                        })
                        .child(
                            label()
                                .text(title)
                                .font_size(12.)
                                .max_lines(1)
                                .color(if broken { theme::DANGER } else { theme::TEXT }),
                        )
                        .child(label().text(summary).font_size(10.).color(theme::DIM)),
                )
                .into_element(),
        );
    }
    if rows.is_empty() {
        rows.push(
            rect()
                .padding(Gaps::new_all(12.))
                .child(
                    label()
                        .text("No savegames in this folder.")
                        .font_size(11.)
                        .color(theme::DIM),
                )
                .into_element(),
        );
    }

    let preview_block = preview.map_or_else(
        || {
            rect()
                .width(Size::fill())
                .height(Size::px(140.))
                .center()
                .background(theme::SURFACE)
                .corner_radius(10.)
                .child(
                    label()
                        .text("select a save")
                        .font_size(10.)
                        .color(theme::DIM),
                )
                .into_element()
        },
        |handle| {
            image(handle)
                .width(Size::fill())
                .height(Size::px(160.))
                .corner_radius(10.)
                .sampling_mode(SamplingMode::Bilinear)
                .aspect_ratio(AspectRatio::Max)
                .into_element()
        },
    );

    let title = character.as_ref().map_or_else(
        || "No savegame".to_owned(),
        |character| character.name.clone(),
    );
    let subtitle = character.as_ref().map_or_else(
        || "—".to_owned(),
        |character| {
            format!(
                "Level {} · {}",
                character.level,
                crate::session::format::duration(character.time_played)
            )
        },
    );

    rect()
        .vertical()
        .width(Size::px(310.))
        .height(Size::fill())
        .content(Content::Flex)
        .padding(Gaps::new(14., 12., 14., 12.))
        .spacing(10.)
        .background(theme::BACKGROUND)
        .child(preview_block)
        .child(
            rect()
                .vertical()
                .spacing(1.)
                .child(
                    label()
                        .text(title)
                        .font_size(15.)
                        .font_weight(FontWeight::BOLD)
                        .max_lines(1)
                        .color(theme::TEXT),
                )
                .child(label().text(subtitle).font_size(10.).color(theme::DIM)),
        )
        .child(
            label()
                .text(format!("SAVEGAMES ({})", entries.len()))
                .font_size(10.)
                .font_weight(FontWeight::BOLD)
                .color(theme::DIM),
        )
        .child(
            ScrollView::new()
                .show_scrollbar(true)
                .width(Size::fill())
                .height(Size::flex(1.0))
                .child(
                    rect()
                        .vertical()
                        .width(Size::fill())
                        .spacing(4.)
                        .children(rows),
                ),
        )
        .into_element()
}

fn tab_bar(tab: State<Tab>, active: Tab) -> Element {
    let mut buttons: Vec<Element> = Vec::new();
    for candidate in Tab::ALL {
        let mut tab = tab;
        let is_active = candidate == active;
        buttons.push(
            Button::new()
                .flat()
                .compact()
                .on_press(move |_| *tab.write() = candidate)
                .child(
                    rect()
                        .padding(Gaps::new_symmetric(6., 12.))
                        .corner_radius(8.)
                        .background(if is_active {
                            theme::SURFACE_ACTIVE
                        } else {
                            Color::TRANSPARENT
                        })
                        .child(
                            label()
                                .text(candidate.label())
                                .font_size(12.)
                                .font_weight(if is_active {
                                    FontWeight::BOLD
                                } else {
                                    FontWeight::NORMAL
                                })
                                .color(if is_active {
                                    theme::ACCENT
                                } else {
                                    theme::MUTED
                                }),
                        ),
                )
                .into_element(),
        );
    }

    rect()
        .vertical()
        .width(Size::fill())
        .spacing(0.)
        .child(
            rect()
                .horizontal()
                .width(Size::fill())
                .spacing(4.)
                .padding(Gaps::new(10., 18., 8., 18.))
                .children(buttons)
                .into_element(),
        )
        .child(
            rect()
                .width(Size::fill())
                .height(Size::px(1.))
                .background(theme::BORDER)
                .into_element(),
        )
        .into_element()
}

#[allow(clippy::large_types_passed_by_value, clippy::needless_pass_by_value)] // `Form` is a bundle of cheap `Copy` handles
fn footer(ctx: &Ctx, form: Form, dirty: bool, form_differs: bool, sync: Rc<dyn Fn()>) -> Element {
    let apply = {
        let ctx = ctx.clone();
        move |_| {
            ctx.run(|session| {
                let _ = session.apply(&form.read());
            });
        }
    };
    let save = {
        let ctx = ctx.clone();
        let sync = sync.clone();
        move |_| {
            ctx.run(|session| {
                if session.apply(&form.read()).is_ok() {
                    let _ = session.save();
                }
            });
            sync();
        }
    };
    let revert = {
        let ctx = ctx.clone();
        let sync = sync.clone();
        move |_| {
            ctx.run(|session| {
                let _ = session.revert();
            });
            sync();
        }
    };

    rect()
        .horizontal()
        .width(Size::fill())
        .spacing(10.)
        .content(Content::Flex)
        .padding(Gaps::new(10., 18., 12., 18.))
        .background(theme::PANEL)
        .child(
            Button::new()
                .outline()
                .child("Apply changes")
                .on_press(apply),
        )
        .child(Button::new().filled().child("Save to disk").on_press(save))
        .child(Button::new().flat().child("Revert").on_press(revert))
        .child(rect().width(Size::flex(1.0)))
        .child(
            label()
                .text(if form_differs {
                    "Character fields are not applied yet."
                } else if dirty {
                    "Changes are only in memory until you save."
                } else {
                    "Nothing to save."
                })
                .font_size(10.)
                .color(if dirty { theme::ACCENT } else { theme::DIM }),
        )
        .into_element()
}

fn empty_state(savegames: usize) -> Element {
    let text = if savegames == 0 {
        "Point JadeForge at the folder that contains your Jade Empire savegames and press Scan."
    } else {
        "Pick a savegame on the left to start editing."
    };
    rect()
        .vertical()
        .width(Size::fill())
        .spacing(8.)
        .padding(Gaps::new_all(28.))
        .child(
            label()
                .text("No savegame selected")
                .font_size(20.)
                .font_weight(FontWeight::BOLD)
                .color(theme::TEXT),
        )
        .child(label().text(text).font_size(12.).color(theme::MUTED))
        .child(
            label()
                .text(
                    "JadeForge never writes a savegame without verifying it and keeping a backup.",
                )
                .font_size(11.)
                .color(theme::DIM),
        )
        .into_element()
}
