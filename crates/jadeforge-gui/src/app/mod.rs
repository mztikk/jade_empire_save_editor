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

use crate::dialog::CloseChoice;
use crate::form::Form;
use crate::session::{CharacterForm, GameStatus, Preview, Session, default_save_dir};
use crate::theme;
use crate::views;
use crate::widgets::Action;
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
    /// An inventory row, by item id.
    Item(usize),
    /// An item-database suggestion, by item id.
    ItemSuggestion(u16),
    /// A style row, by struct index.
    Style(usize),
    /// A learnable style suggestion, by style id.
    StyleSuggestion(u16),
    /// An upgrade orb: style struct index and slot.
    Orb(usize, u16),
    /// A gem row, by struct index.
    Gem(usize),
    /// A companion row, by struct index.
    Companion(usize),
}

impl HoverKey {
    /// Whether this hover belongs to the given style row.
    ///
    /// Orbs are drawn inside their style row, so hovering one keeps the whole
    /// row highlighted. Treating the orb hover as a separate row state made the
    /// row background blink on every step between two orbs.
    pub(crate) fn is_style_row(self, struct_index: usize) -> bool {
        match self {
            Self::Style(index) | Self::Orb(index, _) => index == struct_index,
            _ => false,
        }
    }
}

/// Placeholder of the savegame-list filter.
///
/// The filter matches the folder label and the character name, so the
/// placeholder names both instead of promising a folder-name-only search.
pub const SAVE_FILTER_PLACEHOLDER: &str = "Filter by name or character";
/// Placeholder of the item-database filter.
pub const ITEM_FILTER_PLACEHOLDER: &str = "Filter the item database, or leave empty to browse it";
/// Placeholder of the learnable-style filter.
pub const STYLE_FILTER_PLACEHOLDER: &str = "Filter the style list, or leave empty to browse it";
/// Placeholder of the gem-picker filter.
pub const GEM_FILTER_PLACEHOLDER: &str = "Filter the gem list, or leave empty to browse it";

/// Title of the main window.
///
/// The windowed UI suite finds the application through this title, so keep the
/// binary, the window configuration and the tests in sync.
pub const WINDOW_TITLE: &str = "JadeForge";

/// Whether the running window currently has unsaved changes.
static UI_DIRTY: AtomicBool = AtomicBool::new(false);

thread_local! {
    /// Saves the applied edits when the close dialog asks for it.
    ///
    /// The window close hook runs outside the component tree and has no access
    /// to the session, so the root component registers this handler on every
    /// render. It is thread local because `Session` is a single-threaded
    /// `Rc<RefCell<..>>`; the close hook runs on the same event-loop thread.
    static CLOSE_SAVE_HANDLER: RefCell<Option<Rc<dyn Fn() -> bool>>> =
        const { RefCell::new(None) };
}

/// Registers the handler that writes the applied edits for the close dialog.
fn set_close_save_handler(handler: Rc<dyn Fn() -> bool>) {
    CLOSE_SAVE_HANDLER.with(|slot| *slot.borrow_mut() = Some(handler));
}

/// Runs the registered close-time save, if the app has one.
fn save_before_close() -> bool {
    CLOSE_SAVE_HANDLER.with(|slot| slot.borrow().as_ref().is_some_and(|handler| handler()))
}

/// Turns the close-dialog choice into a window decision.
///
/// Kept pure so the decision table can be tested without a window. The save
/// callback is only invoked for [`CloseChoice::Save`]; a failed save keeps the
/// window open with the error in the status area.
fn close_decision(choice: CloseChoice, save: impl FnOnce() -> bool) -> CloseDecision {
    match choice {
        CloseChoice::Save => {
            if save() {
                CloseDecision::Close
            } else {
                CloseDecision::KeepOpen
            }
        }
        CloseChoice::Discard => CloseDecision::Close,
        CloseChoice::Cancel => CloseDecision::KeepOpen,
    }
}

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
    /// Every tab, in display order.
    pub const ALL: [Self; 6] = [
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

    /// Label shown on the tab and used to identify it in the snapshot example.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Character => "Character",
            Self::Styles => "Styles",
            Self::Inventory => "Inventory",
            Self::Gems => "Gems",
            Self::Companions => "Companions",
        }
    }

    /// Placeholder of the tab's own picker filter, when it has one.
    ///
    /// The savegame-list filter in the sidebar belongs to every tab, so it is
    /// deliberately not returned here: tooling that types a query into "the
    /// active tab's filter" must never fall back to the sidebar for Styles,
    /// Inventory or Gems. Tabs with no picker return `None`.
    #[must_use]
    pub fn filter_placeholder(self) -> Option<&'static str> {
        match self {
            Self::Inventory => Some(ITEM_FILTER_PLACEHOLDER),
            Self::Styles => Some(STYLE_FILTER_PLACEHOLDER),
            Self::Gems => Some(GEM_FILTER_PLACEHOLDER),
            Self::Overview | Self::Character | Self::Companions => None,
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
/// exercise the real title, icon, size, application id, renderer choice and
/// unsaved-changes close guard instead of a test-only window. The hook returns
/// [`CloseDecision::KeepOpen`] when the user declines to discard edits.
#[must_use]
pub fn window_config<E>(app: impl Fn() -> E + 'static) -> WindowConfig
where
    E: IntoElement + 'static,
{
    use freya::winit::dpi::{LogicalPosition, LogicalSize};

    let preferred = (
        f64::from(theme::layout::window_width()),
        f64::from(theme::layout::window_height()),
    );
    let minimum = (
        f64::from(theme::layout::min_window_width()),
        f64::from(theme::layout::min_window_height()),
    );
    let config = crate::config::Config::load();
    let stored = config.remember_window.then_some(config.window).flatten();
    // Without a monitor (a headless start or a failed query) the preferred
    // rectangle is used; the attributes hook replaces it with the fitted one
    // once the real monitor is known.
    let fallback = crate::geometry::fit_window(preferred, stored, None, minimum);

    WindowConfig::new(app)
        .with_title(WINDOW_TITLE)
        .with_icon(LaunchConfig::window_icon(ICON_PNG))
        .with_app_id(APP_ID)
        .with_renderer(crate::renderer::preference())
        .with_size(fallback.width, fallback.height)
        .with_min_size(fallback.min_width, fallback.min_height)
        .with_window_attributes(move |attributes, event_loop| {
            let work = crate::geometry::monitor_work_area(event_loop);
            let fitted = crate::geometry::fit_window(preferred, stored, work, minimum);
            let attributes = attributes
                .with_inner_size(LogicalSize::new(fitted.width, fitted.height))
                .with_min_inner_size(LogicalSize::new(fitted.min_width, fitted.min_height));
            match (fitted.x, fitted.y) {
                (Some(x), Some(y)) => attributes.with_position(LogicalPosition::new(x, y)),
                _ => attributes,
            }
        })
        .with_on_close(|_, _| {
            if !ui_is_dirty() {
                return CloseDecision::Close;
            }
            close_decision(crate::dialog::ask_close_choice(), save_before_close)
        })
}

/// Root component that starts with a specific save folder.
#[allow(clippy::too_many_lines, clippy::redundant_closure_for_method_calls)] // the root wires every view in dependency order
pub fn app_with_directory(directory: PathBuf) -> impl IntoElement {
    // The user's theme and text scale are installed before any child renders.
    // On Windows and macOS Freya's platform preference is reactive; on Linux
    // winit reports nothing, so the binary's startup detection and the portal
    // watcher installed here provide the System choice.
    let saved_config = use_hook(crate::config::Config::load);
    let theme_choice = use_state(|| saved_config.theme);
    let text_scale = use_state(|| saved_config.normalized_text_scale());
    let locale = use_state(|| saved_config.locale);
    let remember_window = use_state(|| saved_config.remember_window);
    crate::i18n::install(*locale.read());
    let platform_preferred = *Platform::get().current_window().preferred_theme.read();
    #[cfg(target_os = "linux")]
    let mut system_preferred = use_state(crate::system_theme::detected);
    #[cfg(target_os = "linux")]
    let _system_theme_updates = use_future(move || {
        let updates = crate::system_theme::enabled().then(crate::system_theme::subscribe);
        async move {
            let Some((_subscription, mut updates)) = updates else {
                return;
            };
            while let Ok(theme) = updates.recv().await {
                system_preferred.set(Some(theme));
            }
        }
    });
    #[cfg(target_os = "linux")]
    let preferred = (*system_preferred.read()).unwrap_or(platform_preferred);
    #[cfg(not(target_os = "linux"))]
    let preferred = platform_preferred;
    let resolved = theme::resolved_choice(*theme_choice.read(), preferred);
    theme::install(theme::UiStyle::new(resolved, *text_scale.read()));
    let mut freya_theme = use_init_theme(theme::theme);
    freya_theme.set_if_modified(theme::theme());
    // Both states are read here so the session-wide re-render below always
    // repaints after a change made on the About page.
    let _ = *theme_choice.read();
    let _ = *text_scale.read();

    let session: Rc<RefCell<Session>> =
        use_hook(move || Rc::new(RefCell::new(Session::pending(directory))));
    let revision = use_state(|| 0_u32);
    // The scan currently in flight. Startup, the Scan button, Enter on the
    // folder field and F5 all only set this state; the worker future below does
    // the reading and applies the result when the request is still current.
    let scan_job = use_state({
        let session = session.clone();
        move || {
            let directory = session.borrow().directory().to_path_buf();
            session.borrow_mut().begin_scan(directory)
        }
    });
    let form = Form::new();
    let directory_state = use_state(|| session.borrow().directory().display().to_string());
    let game_directory = use_state(|| match session.borrow().game_status() {
        GameStatus::Loaded { root, .. } => root.display().to_string(),
        _ => String::new(),
    });
    let tab = use_state(|| Tab::Overview);
    let item_query = use_state(String::new);
    let item_category = use_state(|| None::<String>);
    let show_about = use_state(|| false);
    let show_backups = use_state(|| false);
    let show_review = use_state(|| false);
    let style_query = use_state(String::new);
    let hovered = use_state(|| None::<HoverKey>);
    let gem_query = use_state(String::new);
    let expanded_item = use_state(|| None::<usize>);
    let expanded_suggestion = use_state(|| None::<u16>);
    let expanded_style = use_state(|| None::<u16>);
    let save_query = use_state(String::new);
    let save_focus = use_state(|| 0_usize);
    // One press action per savegame row, kept across renders so hovering a row
    // cannot invalidate it and rebuild every row.
    let save_actions: Rc<RefCell<std::collections::HashMap<usize, Action>>> =
        use_hook(|| Rc::new(RefCell::new(std::collections::HashMap::new())));
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

    // Stable accessibility ids for the four filter inputs, so Ctrl+F can move
    // focus to the filter of the active tab.
    let save_filter_id = use_a11y();
    let item_filter_id = use_a11y();
    let style_filter_id = use_a11y();
    let gem_filter_id = use_a11y();
    // One focus target for the editor's scroll view: static content below the
    // fold is reachable with the arrow and paging keys, not only the wheel.
    let editor_scroll_id = use_a11y();
    let editor_scroll_focus = use_focus(editor_scroll_id);

    // Subscribe to session changes and to the in-flight scan, so starting,
    // cancelling or settling a scan re-renders the header even though most of
    // the scan state lives in the session rather than in reactive state.
    let _ = *revision.read();
    let _ = *scan_job.read();

    let ctx = Ctx {
        session: session.clone(),
        revision,
        form,
    };

    // The native close dialog cannot reach the component tree, so register the
    // save half of its Save / Discard / Cancel choice here. The handler applies
    // the valid form fields and writes the savegame through the normal verified
    // path; a failure leaves the window open with the error in the status slot.
    {
        let ctx = ctx.clone();
        use_hook(move || {
            set_close_save_handler(Rc::new(move || {
                let result = ctx.session.borrow_mut().save_applied(&ctx.form.read());
                let mut revision = ctx.revision;
                *revision.write() += 1;
                result.is_ok()
            }));
        });
    }

    // Record how long a state-changing render took. Reading the revision and
    // the active tab subscribes the effect to the renders that do real work.
    {
        let rendered_at = use_hook(|| std::cell::Cell::new(std::time::Instant::now()));
        use_side_effect(move || {
            let _ = *revision.read();
            let _ = *tab.read();
            let now = std::time::Instant::now();
            crate::perf::record_frame(now.duration_since(rendered_at.replace(now)));
        });
    }

    // Apply valid character edits as they happen, so the form behaves exactly
    // like the item, style and gem edits: everything lives in memory until the
    // user saves. Invalid input stays in the fields for the user to fix.
    // The snapshot is a dependency of the side effect below; it is memoised so
    // the 21 inputs are only re-read when one of them changes.
    let form_snapshot = {
        let snapshot = use_memo(move || form.snapshot());
        snapshot.read().clone()
    };
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

    // Opening another savegame or scanning another folder starts a fresh
    // document: filters, paging, expanded rows and every tab's scroll position
    // belong to the previous one and would otherwise leak across.
    //
    // The reset runs from the actions that change the document instead of an
    // effect: reading a scroll controller inside an effect subscribes it to the
    // layout bounds, which would rerun the reset after every render.
    let reset: Rc<dyn Fn()> = {
        let session = session.clone();
        Rc::new(move || {
            // Each call copies the handles, so the closure stays an `Fn` that
            // the shell can share between the header, the sidebar and the
            // empty state.
            reset_pickers(
                save_query,
                item_query,
                item_category,
                show_about,
                show_backups,
                show_review,
                style_query,
                gem_query,
                expanded_item,
                expanded_suggestion,
                expanded_style,
                hovered,
                directory_state,
                &session,
                &scrolls,
            );
        })
    };

    // Snapshot everything the shell needs so the borrow stays short.
    let disk_changed = session.borrow_mut().check_disk_state();
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

    // The invalid badge comes from parsing the fields, not from "differs":
    // a field that cannot be parsed is invalid even before it is applied. The
    // per-field reparse is memoised against the session revision and the form
    // inputs instead of running on every frame.
    let form_errors = {
        let session = session.clone();
        let errors = use_memo(move || {
            let _ = *revision.read();
            let form = form.read();
            session.borrow().form_errors(&form)
        });
        errors.read().clone()
    };
    let preview_handle = cached_preview(&preview_cache, preview.as_deref());
    let game_ready = matches!(game_status, GameStatus::Loaded { .. });
    let game_loading = matches!(game_status, GameStatus::Loading);
    let names_ready = session.borrow().has_bundled_names();
    let items_ready = session.borrow().items_ready();
    let total_items = if active_tab == Tab::Inventory {
        session.borrow().item_database_count()
    } else {
        0
    };
    // Filtering the whole item database and the learnable style list is
    // derived work: memoise it against the session revision, the filter and the
    // page limit instead of rebuilding 291 suggestions every frame.
    let (item_results, item_matches) = {
        let session = session.clone();
        let results = use_memo(move || {
            let _ = *revision.read();
            let query = item_query.read().clone();
            let category = item_category.read().clone();
            session
                .borrow()
                .search_items_filtered(&query, category.as_deref(), usize::MAX)
        });
        results.read().clone()
    };
    let item_categories = {
        let session = session.clone();
        let categories = use_memo(move || {
            let _ = *revision.read();
            session.borrow().item_categories()
        });
        categories.read().clone()
    };
    let (style_matches, total_missing) = {
        let session = session.clone();
        let results = use_memo(move || {
            let _ = *revision.read();
            let query = style_query.read().trim().to_lowercase();
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
            (matching, total_missing)
        });
        results.read().clone()
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

    // One worker scan for every path. The future triggers whenever `scan_job`
    // changes, starts the filesystem and game-data read on a thread, and
    // applies the result only while the request is still current. A cancel or a
    // superseding scan clears the job, which drops this task and discards the
    // worker result through the session token.
    let scan = use_future({
        let session = session.clone();
        let reset = reset.clone();
        let sync = sync.clone();
        move || {
            // Reading the job subscribes the future to new scan requests.
            let job = scan_job.read().clone();
            let session = session.clone();
            let reset = reset.clone();
            let sync = sync.clone();
            let scan_job = scan_job;
            let mut revision = revision;
            let directory_state = directory_state;
            async move {
                let mut scan_job = scan_job;
                let mut directory_state = directory_state;
                let Some(job) = job else {
                    return;
                };
                let (tx, rx) = futures_channel::oneshot::channel();
                let worker_job = job.clone();
                std::thread::spawn(move || {
                    let _ = tx.send(crate::session::ScanOutcome::run(&worker_job));
                });
                let result = rx.await;
                let applied = {
                    let mut session = session.borrow_mut();
                    match result {
                        Ok(outcome) => session.complete_scan(&job, outcome),
                        Err(_) => {
                            if session.fail_scan(&job) {
                                crate::session::ScanApply::Failed
                            } else {
                                crate::session::ScanApply::Stale
                            }
                        }
                    }
                };
                match applied {
                    crate::session::ScanApply::Applied(completion) => {
                        let real = session.borrow().directory().display().to_string();
                        directory_state.set(real);
                        if completion.directory_changed {
                            reset();
                            sync();
                        }
                        *revision.write() += 1;
                        // Clear the in-flight marker last: changing the state
                        // this future subscribes to may restart it.
                        scan_job.set(None);
                    }
                    crate::session::ScanApply::Failed => {
                        let real = session.borrow().directory().display().to_string();
                        directory_state.set(real);
                        *revision.write() += 1;
                        scan_job.set(None);
                    }
                    crate::session::ScanApply::Stale => {}
                }
            }
        }
    });
    let _ = scan.state();

    // Starts a scan for a path. A rejected folder leaves the previous entries,
    // savegame and remembered path untouched and only reports why.
    let start_scan: Rc<dyn Fn(PathBuf) -> bool> = {
        let session = session.clone();
        Rc::new(move |path| {
            let mut scan_job = scan_job;
            let mut revision = revision;
            let request = session.borrow_mut().begin_scan(path);
            let accepted = request.is_some();
            if let Some(request) = request {
                scan_job.set(Some(request));
            }
            // The rejection message lives in the session, which is not reactive
            // by itself, so both outcomes refresh the header.
            *revision.write() += 1;
            accepted
        })
    };
    // Cancels the in-flight scan; the savegame and entry list stay on screen.
    let cancel_scan: Rc<dyn Fn()> = {
        let session = session.clone();
        Rc::new(move || {
            let mut scan_job = scan_job;
            let mut directory_state = directory_state;
            if session.borrow_mut().cancel_scan() {
                scan_job.set(None);
                let real = session.borrow().directory().display().to_string();
                directory_state.set(real);
            }
        })
    };

    let scanning = session.borrow().is_scanning();
    let tab_content = match (active_tab, &character) {
        (Tab::Overview, Some(character)) => {
            views::overview::view(&ctx, character, items.len(), styles.len())
        }
        (Tab::Character, Some(character)) => views::character::view(&ctx, character, &form_errors),
        (Tab::Styles, _) => views::styles::view(
            &ctx,
            &styles,
            style_query,
            &style_matches,
            game_ready,
            game_loading,
            names_ready,
            game_ready.then_some(total_missing),
            expanded_style,
            hovered,
            style_filter_id,
        ),
        (Tab::Inventory, _) => views::inventory::view(
            &ctx,
            &items,
            item_query,
            item_category,
            &item_categories,
            &item_results,
            items_ready,
            game_loading,
            item_matches,
            total_items,
            expanded_item,
            expanded_suggestion,
            hovered,
            item_filter_id,
        ),
        (Tab::Gems, _) => {
            let available = session.borrow().available_gems();
            let amulet = session.borrow().amulet();
            let gems = session.borrow().gems();
            views::gems::view(
                &ctx,
                &gems,
                gem_query,
                &available,
                amulet,
                hovered,
                gem_filter_id,
            )
        }
        (Tab::Companions, _) => views::henchmen::view(&ctx, &henchmen, hovered),
        (_, None) if scanning => shell::loading_state(),
        (_, None) => shell::empty_state(&ctx, entries.len(), directory_state, start_scan.clone()),
    };
    let show_about_now = *show_about.read();
    let show_backups_now = *show_backups.read();
    let show_review_now = *show_review.read();
    let close_review: Rc<dyn Fn()> = Rc::new(move || {
        let mut state = show_review;
        state.set(false);
    });
    let close_backups: Rc<dyn Fn()> = Rc::new(move || {
        let mut state = show_backups;
        state.set(false);
    });
    let content = if show_review_now {
        views::review::view(&ctx, form, close_review, &sync)
    } else if show_backups_now {
        views::backups::view(&ctx, close_backups, &sync)
    } else if show_about_now {
        views::about::view(
            &ctx,
            save_sort,
            show_about,
            theme_choice,
            text_scale,
            locale,
            remember_window,
        )
    } else {
        tab_content
    };
    let page_index = if show_review_now {
        2
    } else {
        usize::from(show_backups_now)
    };

    let content = rect()
        .width(Size::fill())
        .key(active_tab.index() * 3 + page_index)
        .child(content)
        .into_element();

    let header = shell::header(
        &ctx,
        directory_state,
        game_directory,
        &message,
        &game_status,
        dirty,
        has_save,
        &form_errors,
        scanning,
        show_about,
        show_backups,
        show_review,
        start_scan.clone(),
        cancel_scan.clone(),
    );
    let visible = {
        let session = session.clone();
        let filtered = use_memo(move || {
            let _ = *revision.read();
            let entries = session.borrow().entries();
            let (sort, descending) = *save_sort.read();
            // Shared so the virtual list can compare the filtered rows by
            // identity instead of rebuilding them on every unrelated render.
            Rc::new(crate::session::visible_entries(
                &entries,
                &save_query.read(),
                sort,
                descending,
            ))
        });
        filtered.read().clone()
    };
    let sidebar = sidebar::sidebar(
        &ctx,
        &entries,
        &visible,
        selected,
        save_focus,
        preview_handle,
        character.as_ref(),
        scanning,
        &save_actions,
        save_query,
        save_filter_id,
        save_sort,
        reset.clone(),
        sync.clone(),
    );
    let tabs = navigation::tab_bar(
        tab,
        active_tab,
        has_save,
        scanning,
        show_about,
        show_backups,
        show_review,
    );
    let footer = shell::footer(
        &ctx,
        form,
        dirty,
        has_save,
        &form_errors,
        show_about,
        show_backups,
        show_review,
        sync.clone(),
    );

    // Desktop shortcuts. Save and Scan duplicate the footer/header actions so
    // they work no matter which control has focus.
    let shortcut = {
        let ctx = ctx.clone();
        let sync = sync.clone();
        let start_scan = start_scan.clone();
        let directory = directory_state;
        let active = active_tab;
        let filter_ids = [
            save_filter_id,
            item_filter_id,
            style_filter_id,
            gem_filter_id,
        ];
        let mut save_query = save_query;
        let mut item_query = item_query;
        let mut style_query = style_query;
        let mut gem_query = gem_query;
        let mut show_about = show_about;
        let mut show_backups = show_backups;
        let mut show_review = show_review;
        move |event: Event<KeyboardEventData>| {
            let ctrl = event.modifiers.contains(Modifiers::CONTROL);
            let ctrl_shift = event.modifiers.contains(Modifiers::SHIFT);
            match &event.key {
                Key::Character(character) if ctrl => match character.to_lowercase().as_str() {
                    "s" => {
                        // Match the disabled footer action: a shortcut must not
                        // open an empty review when there is no applied change.
                        if !ctx.session.borrow().is_dirty() {
                            return;
                        }
                        show_about.set(false);
                        show_backups.set(false);
                        show_review.set(true);
                    }
                    "z" => {
                        let changed = if ctrl_shift {
                            ctx.run(Session::redo)
                        } else {
                            ctx.run(Session::undo)
                        };
                        if changed {
                            sync();
                        }
                    }
                    "y" => {
                        if ctx.run(Session::redo) {
                            sync();
                        }
                    }
                    "o" => {
                        shell::choose_folder(&ctx, directory, &start_scan);
                    }
                    "/" | "?" => {
                        show_review.set(false);
                        show_backups.set(false);
                        show_about.set(true);
                    }
                    "f" => {
                        show_about.set(false);
                        let id = match active {
                            Tab::Inventory => filter_ids[1],
                            Tab::Styles => filter_ids[2],
                            Tab::Gems => filter_ids[3],
                            Tab::Overview | Tab::Character | Tab::Companions => filter_ids[0],
                        };
                        id.request_focus();
                    }
                    _ => {}
                },
                Key::Named(NamedKey::F5) => {
                    let mut directory = directory;
                    let path = PathBuf::from(directory.read().as_str());
                    if !start_scan(path) {
                        let real = ctx.session.borrow().directory().display().to_string();
                        *directory.write() = real;
                    }
                }
                Key::Named(NamedKey::Escape) => {
                    if *show_review.read() {
                        show_review.set(false);
                    } else if *show_backups.read() {
                        show_backups.set(false);
                    } else if *show_about.read() {
                        show_about.set(false);
                    } else {
                        match active {
                            Tab::Inventory => item_query.set(String::new()),
                            Tab::Styles => style_query.set(String::new()),
                            Tab::Gems => gem_query.set(String::new()),
                            Tab::Overview | Tab::Character | Tab::Companions => {
                                save_query.set(String::new());
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    };

    rect()
        .vertical()
        .width(Size::fill())
        .height(Size::fill())
        .background(theme::background())
        .color(theme::text())
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
                        .maybe_child(
                            disk_changed.then(|| shell::external_change_banner(&ctx, &sync)),
                        )
                        .child(
                            rect()
                                .a11y_id(editor_scroll_id)
                                .a11y_focusable(true)
                                .a11y_role(AccessibilityRole::ScrollView)
                                .width(Size::fill())
                                .height(Size::flex(1.0))
                                .content(Content::Flex)
                                .border(
                                    Border::new()
                                        .fill(if editor_scroll_focus() == Focus::Keyboard {
                                            theme::text()
                                        } else {
                                            Color::TRANSPARENT
                                        })
                                        .width(theme::stroke::focus()),
                                )
                                .on_key_down({
                                    let mut scroll = scrolls[active_tab.index()];
                                    move |event: Event<KeyboardEventData>| {
                                        let current = scroll.position().y as i32;
                                        let target = match event.key {
                                            Key::Named(NamedKey::ArrowDown) => current + 48,
                                            Key::Named(NamedKey::ArrowUp) => current - 48,
                                            Key::Named(NamedKey::PageDown) => current + 320,
                                            Key::Named(NamedKey::PageUp) => current - 320,
                                            Key::Named(NamedKey::Home) => i32::MIN,
                                            Key::Named(NamedKey::End) => i32::MAX,
                                            _ => return,
                                        };
                                        scroll.scroll_to_y(target);
                                        event.stop_propagation();
                                    }
                                })
                                .child(
                                    ScrollView::new_controlled(scrolls[active_tab.index()])
                                        .show_scrollbar(true)
                                        .auto_hide_scrollbars(false)
                                        .width(Size::fill())
                                        .height(Size::fill())
                                        .child(
                                            rect()
                                                .vertical()
                                                .width(Size::fill())
                                                .cross_align(Alignment::Center)
                                                .padding(Gaps::new(
                                                    theme::space::xxl(),
                                                    theme::space::card(),
                                                    theme::space::xxl(),
                                                    theme::space::card(),
                                                ))
                                                // Wide windows stop stretching the
                                                // cards: past this width a paragraph
                                                // becomes a 150-character line.
                                                .child(
                                                    rect()
                                                        .vertical()
                                                        .width(Size::fill())
                                                        .max_width(Size::px(
                                                            theme::layout::editor_max_width(),
                                                        ))
                                                        // The content already renders the loading
                                                        // skeleton, the About page and the empty
                                                        // state; a second no-save branch here used
                                                        // to swallow all three.
                                                        .child(content),
                                                ),
                                        ),
                                ),
                        ),
                ),
        )
        .child(footer)
        .on_global_key_down(shortcut)
}

/// Clears every per-document picker and scroll state.
///
/// Kept as a free function so the reset stays a `Fn`: each state handle is
/// copied into a mutable local before it is written.
#[allow(clippy::too_many_arguments)] // one handle per independent piece of state
fn reset_pickers(
    save_query: State<String>,
    item_query: State<String>,
    item_category: State<Option<String>>,
    show_about: State<bool>,
    show_backups: State<bool>,
    show_review: State<bool>,
    style_query: State<String>,
    gem_query: State<String>,
    expanded_item: State<Option<usize>>,
    expanded_suggestion: State<Option<u16>>,
    expanded_style: State<Option<u16>>,
    hovered: State<Option<HoverKey>>,
    directory_state: State<String>,
    session: &Rc<RefCell<Session>>,
    scrolls: &[ScrollController; 6],
) {
    let mut save_query = save_query;
    save_query.set(String::new());
    let mut item_query = item_query;
    item_query.set(String::new());
    let mut item_category = item_category;
    item_category.set(None);
    let mut show_about = show_about;
    show_about.set(false);
    let mut show_backups = show_backups;
    show_backups.set(false);
    let mut show_review = show_review;
    show_review.set(false);
    let mut style_query = style_query;
    style_query.set(String::new());
    let mut gem_query = gem_query;
    gem_query.set(String::new());
    let mut expanded_item = expanded_item;
    expanded_item.set(None);
    let mut expanded_suggestion = expanded_suggestion;
    expanded_suggestion.set(None);
    let mut expanded_style = expanded_style;
    expanded_style.set(None);
    let mut hovered = hovered;
    hovered.set(None);
    let mut directory_state = directory_state;
    directory_state.set(session.borrow().directory().display().to_string());
    for scroll in scrolls {
        let mut scroll = *scroll;
        scroll.scroll_to_x(0);
        scroll.scroll_to_y(0);
    }
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
    fn the_close_dialog_choice_table_never_saves_on_discard_or_cancel() {
        assert_eq!(
            close_decision(CloseChoice::Save, || true),
            CloseDecision::Close
        );
        assert_eq!(
            close_decision(CloseChoice::Discard, || { panic!("Discard must not save") }),
            CloseDecision::Close
        );
        assert_eq!(
            close_decision(CloseChoice::Cancel, || { panic!("Cancel must not save") }),
            CloseDecision::KeepOpen
        );
    }

    #[test]
    fn a_failed_save_keeps_the_window_open() {
        assert_eq!(
            close_decision(CloseChoice::Save, || false),
            CloseDecision::KeepOpen
        );
    }

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

    #[test]
    fn tab_filters_never_fall_back_to_the_savegame_filter() {
        assert_eq!(
            Tab::Inventory.filter_placeholder(),
            Some(ITEM_FILTER_PLACEHOLDER)
        );
        assert_eq!(
            Tab::Styles.filter_placeholder(),
            Some(STYLE_FILTER_PLACEHOLDER)
        );
        assert_eq!(Tab::Gems.filter_placeholder(), Some(GEM_FILTER_PLACEHOLDER));
        for tab in [Tab::Overview, Tab::Character, Tab::Companions] {
            assert_eq!(tab.filter_placeholder(), None, "{}", tab.label());
        }
        for placeholder in [
            ITEM_FILTER_PLACEHOLDER,
            STYLE_FILTER_PLACEHOLDER,
            GEM_FILTER_PLACEHOLDER,
        ] {
            assert_ne!(placeholder, SAVE_FILTER_PLACEHOLDER);
        }
    }

    #[test]
    fn an_orb_hover_belongs_to_its_style_row() {
        assert!(HoverKey::Style(3).is_style_row(3));
        assert!(HoverKey::Orb(3, 0).is_style_row(3));
        assert!(HoverKey::Orb(3, 4).is_style_row(3));
        assert!(!HoverKey::Orb(4, 0).is_style_row(3));
        assert!(!HoverKey::Style(4).is_style_row(3));
        assert!(!HoverKey::StyleSuggestion(3).is_style_row(3));
        assert!(!HoverKey::Item(3).is_style_row(3));
    }
}
