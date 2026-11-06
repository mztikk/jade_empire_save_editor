//! The root component and the application shell.
//!
//! The shell owns the navigation (save list, tabs, actions) and renders the
//! active tab. All behaviour lives in [`crate::session::Session`].

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};

use freya::elements::image::ImageHandle;
use freya::engine::prelude::AlphaType;
use freya::prelude::*;

use crate::form::Form;
use crate::session::{CharacterForm, GameStatus, Preview, Session, default_save_dir};
use crate::theme;
use crate::views;
use crate::{APP_ID, ICON_PNG};

mod navigation;
mod shell;
mod sidebar;

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

/// Which interactive row is currently hovered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoverKey {
    /// A savegame row, by index in `Session::entries`.
    Save(usize),
    /// An inventory row, by item id.
    Item(usize),
    /// An item-database suggestion, by item id.
    ItemSuggestion(u16),
    /// A style row, by struct index.
    Style(usize),
    /// An upgrade orb: style struct index and slot.
    Orb(usize, u16),
}

/// Entries a searchable picker renders per page (items, styles, gems).
pub const PAGE_SIZE: usize = 30;

/// Title of the main window.
///
/// The windowed UI suite finds the application through this title, so keep the
/// binary, the window configuration and the tests in sync.
pub const WINDOW_TITLE: &str = "JadeForge";

/// Whether the running window currently has unsaved changes.
static UI_DIRTY: AtomicBool = AtomicBool::new(false);

/// Returns whether the app currently has unsaved changes.
///
/// The window close hook runs outside the component tree, so the shell mirrors
/// its dirty flag here on every render.
#[must_use]
pub fn ui_is_dirty() -> bool {
    UI_DIRTY.load(Ordering::Relaxed)
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
    /// Gems in the amulet and the inventory.
    Gems,
    /// Companions.
    Companions,
}

impl Tab {
    pub(super) const ALL: [Self; 6] = [
        Self::Overview,
        Self::Character,
        Self::Styles,
        Self::Inventory,
        Self::Gems,
        Self::Companions,
    ];

    /// Position of the tab in [`Tab::ALL`], used to index per-tab state.
    pub(super) fn index(self) -> usize {
        match self {
            Self::Overview => 0,
            Self::Character => 1,
            Self::Styles => 2,
            Self::Inventory => 3,
            Self::Gems => 4,
            Self::Companions => 5,
        }
    }

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Character => "Character",
            Self::Styles => "Styles",
            Self::Inventory => "Inventory",
            Self::Gems => "Gems",
            Self::Companions => "Companions",
        }
    }
}

/// Root component of the application.
pub fn app() -> impl IntoElement {
    app_with_directory(default_save_dir())
}

/// Builds the main application window.
///
/// The binary and the automated windowed tests both use this, so the tests
/// exercise the real title, icon, size, application id and unsaved-changes
/// close guard instead of a test-only window. The hook returns
/// [`CloseDecision::KeepOpen`] when the user declines to discard edits.
#[must_use]
pub fn window_config<E>(app: impl Fn() -> E + 'static) -> WindowConfig
where
    E: IntoElement + 'static,
{
    WindowConfig::new(app)
        .with_title(WINDOW_TITLE)
        .with_icon(LaunchConfig::window_icon(ICON_PNG))
        .with_app_id(APP_ID)
        .with_size(
            theme::layout::WINDOW_WIDTH.into(),
            theme::layout::WINDOW_HEIGHT.into(),
        )
        .with_min_size(
            theme::layout::MIN_WINDOW_WIDTH.into(),
            theme::layout::MIN_WINDOW_HEIGHT.into(),
        )
        .with_on_close(|_, _| {
            if !ui_is_dirty() {
                return CloseDecision::Close;
            }
            let answer = rfd::MessageDialog::new()
                .set_title("Unsaved changes")
                .set_description(
                    "This savegame has changes that are only in memory. Close JadeForge and discard them?",
                )
                .set_buttons(rfd::MessageButtons::YesNo)
                .set_level(rfd::MessageLevel::Warning)
                .show();
            if matches!(answer, rfd::MessageDialogResult::Yes) {
                CloseDecision::Close
            } else {
                CloseDecision::KeepOpen
            }
        })
}

/// Root component that starts with a specific save folder.
#[allow(clippy::too_many_lines, clippy::redundant_closure_for_method_calls)] // the root wires every view in dependency order
pub fn app_with_directory(directory: PathBuf) -> impl IntoElement {
    use_init_theme(theme::theme);

    let session: Rc<RefCell<Session>> =
        use_hook(move || Rc::new(RefCell::new(Session::empty(directory))));
    let revision = use_state(|| 0_u32);
    // Scan savegames on a worker thread; the future applies the result and
    // bumps the revision, so the first frame never waits for the scan.
    let scan = use_future({
        let session = session.clone();
        move || {
            let session = session.clone();
            let mut revision = revision;
            async move {
                let directory = session.borrow().directory().to_path_buf();
                let token = session.borrow().scan_token();
                let (tx, rx) = futures_channel::oneshot::channel();
                let worker_directory = directory.clone();
                std::thread::spawn(move || {
                    let _ = tx.send(Session::scan_entries(&worker_directory));
                });
                let applied = match rx.await {
                    Ok(entries) => session
                        .borrow_mut()
                        .apply_entries_for(&directory, token, entries),
                    Err(_) => session
                        .borrow_mut()
                        .report_scan_failure_for(&directory, token),
                };
                if applied {
                    *revision.write() += 1;
                }
            }
        }
    });
    let _ = scan.state();
    let form = Form::new();
    let directory_state = use_state(|| session.borrow().directory().display().to_string());
    let tab = use_state(|| Tab::Overview);
    let item_query = use_state(String::new);
    let style_query = use_state(String::new);
    let item_limit = use_state(|| PAGE_SIZE);
    let style_limit = use_state(|| PAGE_SIZE);
    let gem_limit = use_state(|| PAGE_SIZE);
    let hovered = use_state(|| None::<HoverKey>);
    let gem_query = use_state(String::new);
    let expanded_item = use_state(|| None::<usize>);
    let expanded_suggestion = use_state(|| None::<u16>);
    let expanded_style = use_state(|| None::<u16>);
    let save_query = use_state(String::new);
    let save_focus = use_state(|| 0_usize);
    let save_sort = use_state(|| {
        let config = crate::config::Config::load();
        let sort = config
            .sort
            .as_deref()
            .and_then(crate::session::SaveSort::from_key)
            .unwrap_or(crate::session::SaveSort::Name);
        (sort, config.sort_descending)
    });
    let preview_cache: Rc<RefCell<Option<(u64, ImageHandle)>>> =
        use_hook(|| Rc::new(RefCell::new(None)));
    // One scroll controller per tab so each tab keeps its own scroll position.
    let scrolls: [ScrollController; 6] = [
        use_scroll_controller(ScrollConfig::default),
        use_scroll_controller(ScrollConfig::default),
        use_scroll_controller(ScrollConfig::default),
        use_scroll_controller(ScrollConfig::default),
        use_scroll_controller(ScrollConfig::default),
        use_scroll_controller(ScrollConfig::default),
    ];

    // Subscribe to session changes.
    let _ = *revision.read();

    let ctx = Ctx {
        session: session.clone(),
        revision,
        form,
    };

    // Apply valid character edits as they happen, so the form behaves exactly
    // like the item, style and gem edits: everything lives in memory until the
    // user saves. Invalid input stays in the fields for the user to fix.
    let form_snapshot = form.snapshot();
    {
        let mut ctx = ctx.clone();
        use_side_effect_with_deps(&form_snapshot, move |_| {
            if ctx
                .session
                .borrow_mut()
                .apply_form_silently(&ctx.form.read())
            {
                *ctx.revision.write() += 1;
            }
        });
    }

    // Snapshot everything the shell needs so the borrow stays short.
    let active_tab = *tab.read();
    let entries = session.borrow().entries();
    let selected = session.borrow().selected();
    let message = session.borrow().message().clone();
    let character = session.borrow().character().cloned();
    let items = session.borrow().items();
    let styles = session.borrow().styles();
    let henchmen = if active_tab == Tab::Companions {
        session.borrow().henchmen()
    } else {
        Rc::new(Vec::new())
    };
    let dirty = session.borrow().is_dirty();
    UI_DIRTY.store(dirty, Ordering::Relaxed);
    let preview = session.borrow().preview();
    let game_status = session.borrow().game_status().clone();
    let has_save = session.borrow().opened_save().is_some();

    let form_differs = character
        .as_ref()
        .is_some_and(|character| form.differs_from(character));
    let preview_handle = cached_preview(&preview_cache, preview.as_deref());
    let game_ready = matches!(game_status, GameStatus::Loaded { .. });
    let total_items = if active_tab == Tab::Inventory {
        session
            .borrow()
            .game()
            .map_or(0, |game| game.playable_item_count())
    } else {
        0
    };
    let (item_results, item_matches) = if active_tab == Tab::Inventory {
        let query = item_query.read().clone();
        session.borrow().search_items(&query, *item_limit.read())
    } else {
        (Vec::new(), 0)
    };
    let (style_matches, style_matches_total, total_missing) = if active_tab == Tab::Styles {
        let query = style_query.read().trim().to_lowercase();
        let limit = *style_limit.read();
        let missing = session.borrow().missing_styles();
        let total_missing = missing.len();
        let matching: Vec<_> = missing
            .into_iter()
            .filter(|style| {
                query.is_empty()
                    || style.name.to_lowercase().contains(&query)
                    || style.label.to_lowercase().contains(&query)
            })
            .collect();
        let total = matching.len();
        (
            matching.into_iter().take(limit).collect::<Vec<_>>(),
            total,
            total_missing,
        )
    } else {
        (Vec::new(), 0, 0)
    };

    let sync: Rc<dyn Fn()> = {
        let session = session.clone();
        Rc::new(move || {
            let character = session.borrow().character().cloned();
            if let Some(character) = character {
                form.load(&CharacterForm::from_character(&character));
            }
        })
    };

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
            let mut style_limit = style_limit;
            views::styles::view(
                &ctx,
                &styles,
                style_query,
                &style_matches,
                game_ready,
                game_ready.then_some(total_missing),
                style_matches_total,
                expanded_style,
                hovered,
                move |_| *style_limit.write() += PAGE_SIZE,
            )
        }
        (Tab::Inventory, _) => {
            let mut item_limit = item_limit;
            views::inventory::view(
                &ctx,
                &items,
                item_query,
                &item_results,
                game_ready,
                item_matches,
                total_items,
                expanded_item,
                expanded_suggestion,
                hovered,
                move |_| *item_limit.write() += PAGE_SIZE,
            )
        }
        (Tab::Gems, _) => {
            let available = session.borrow().available_gems();
            let amulet = session.borrow().amulet();
            let gems = session.borrow().gems();
            views::gems::view(&ctx, &gems, gem_query, &available, amulet, gem_limit)
        }
        (Tab::Companions, _) => views::henchmen::view(&ctx, &henchmen),
        (_, None) => shell::empty_state(entries.len()),
    };

    let content = rect()
        .width(Size::fill())
        .key(active_tab.index())
        .child(content)
        .into_element();

    let header = shell::header(
        &ctx,
        directory_state,
        &message,
        &game_status,
        dirty,
        form_differs,
        sync.clone(),
    );
    let (sort_key, sort_descending) = *save_sort.read();
    let visible =
        crate::session::visible_entries(&entries, &save_query.read(), sort_key, sort_descending);
    let sidebar = sidebar::sidebar(
        &ctx,
        &entries,
        &visible,
        selected,
        save_focus,
        preview_handle,
        character.as_ref(),
        form_differs,
        save_query,
        save_sort,
        hovered,
        sync.clone(),
    );
    let tabs = navigation::tab_bar(tab, active_tab);
    let footer = shell::footer(&ctx, form, dirty, form_differs, sync);

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
                            ScrollView::new_controlled(scrolls[active_tab.index()])
                                .show_scrollbar(true)
                                .auto_hide_scrollbars(false)
                                .width(Size::fill())
                                .height(Size::flex(1.0))
                                .child(
                                    rect()
                                        .vertical()
                                        .width(Size::fill())
                                        .padding(Gaps::new(
                                            theme::space::XXL,
                                            theme::space::CARD,
                                            theme::space::XXL,
                                            theme::space::CARD,
                                        ))
                                        .child(if has_save {
                                            content
                                        } else {
                                            shell::empty_state(entries.len())
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
    if let Some((generation, handle)) = cache.as_ref()
        && *generation == preview.generation
    {
        return Some(handle.clone());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_positions_match_the_all_array() {
        for (position, tab) in Tab::ALL.into_iter().enumerate() {
            assert_eq!(tab.index(), position, "{}", tab.label());
        }
    }

    #[test]
    fn every_tab_has_a_distinct_label() {
        let mut labels: Vec<&str> = Tab::ALL.iter().map(|tab| tab.label()).collect();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), Tab::ALL.len());
    }
}
