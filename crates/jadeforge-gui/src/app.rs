//! The Freya user interface.
//!
//! The view is deliberately thin: all behaviour lives in
//! [`crate::session::Session`], the component only renders the current state and
//! forwards user input.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use freya::prelude::*;
use jadeforge_core::{Character, Item};

use crate::session::{CharacterForm, MessageKind, Session};

const BACKGROUND: (u8, u8, u8) = (21, 23, 28);
const PANEL: (u8, u8, u8) = (30, 33, 40);
const PANEL_ALT: (u8, u8, u8) = (37, 41, 50);
const TEXT: (u8, u8, u8) = (228, 231, 237);
const MUTED: (u8, u8, u8) = (151, 158, 171);
const ACCENT: (u8, u8, u8) = (223, 181, 87);
const SUCCESS: (u8, u8, u8) = (127, 199, 148);
const ERROR: (u8, u8, u8) = (234, 124, 124);

type SharedSession = Rc<RefCell<Session>>;

/// The text inputs of the character form.
#[derive(Clone, Copy)]
struct Form {
    name: State<String>,
    level: State<String>,
    experience: State<String>,
    gold: State<String>,
    attribute_points: State<String>,
    health: State<String>,
    health_max: State<String>,
    chi: State<String>,
    chi_max: State<String>,
    focus: State<String>,
    focus_max: State<String>,
    body: State<String>,
    mind: State<String>,
    spirit: State<String>,
    charm: State<String>,
    intuition: State<String>,
    intimidate: State<String>,
    alignment: State<String>,
    chapter: State<String>,
    kills: State<String>,
}

impl Form {
    fn new() -> Self {
        Self {
            name: use_state(String::new),
            level: use_state(String::new),
            experience: use_state(String::new),
            gold: use_state(String::new),
            attribute_points: use_state(String::new),
            health: use_state(String::new),
            health_max: use_state(String::new),
            chi: use_state(String::new),
            chi_max: use_state(String::new),
            focus: use_state(String::new),
            focus_max: use_state(String::new),
            body: use_state(String::new),
            mind: use_state(String::new),
            spirit: use_state(String::new),
            charm: use_state(String::new),
            intuition: use_state(String::new),
            intimidate: use_state(String::new),
            alignment: use_state(String::new),
            chapter: use_state(String::new),
            kills: use_state(String::new),
        }
    }

    fn load(&self, form: &CharacterForm) {
        let assign = |mut state: State<String>, value: &String| {
            state.write().clone_from(value);
        };
        assign(self.name, &form.name);
        assign(self.level, &form.level);
        assign(self.experience, &form.experience);
        assign(self.gold, &form.gold);
        assign(self.attribute_points, &form.attribute_points);
        assign(self.health, &form.health);
        assign(self.health_max, &form.health_max);
        assign(self.chi, &form.chi);
        assign(self.chi_max, &form.chi_max);
        assign(self.focus, &form.focus);
        assign(self.focus_max, &form.focus_max);
        assign(self.body, &form.body);
        assign(self.mind, &form.mind);
        assign(self.spirit, &form.spirit);
        assign(self.charm, &form.charm);
        assign(self.intuition, &form.intuition);
        assign(self.intimidate, &form.intimidate);
        assign(self.alignment, &form.alignment);
        assign(self.chapter, &form.chapter);
        assign(self.kills, &form.kills);
    }

    fn read(&self) -> CharacterForm {
        let value = |state: State<String>| state.read().clone();
        CharacterForm {
            name: value(self.name),
            level: value(self.level),
            experience: value(self.experience),
            gold: value(self.gold),
            attribute_points: value(self.attribute_points),
            health: value(self.health),
            health_max: value(self.health_max),
            chi: value(self.chi),
            chi_max: value(self.chi_max),
            focus: value(self.focus),
            focus_max: value(self.focus_max),
            body: value(self.body),
            mind: value(self.mind),
            spirit: value(self.spirit),
            charm: value(self.charm),
            intuition: value(self.intuition),
            intimidate: value(self.intimidate),
            alignment: value(self.alignment),
            chapter: value(self.chapter),
            kills: value(self.kills),
        }
    }
}

/// Root component of the application.
pub fn app() -> impl IntoElement {
    app_with_directory(crate::session::default_save_dir())
}

/// Root component that starts with a specific save folder.
pub fn app_with_directory(directory: PathBuf) -> impl IntoElement {
    let session: SharedSession =
        use_hook(move || Rc::new(RefCell::new(Session::with_directory(directory))));
    let revision = use_state(|| 0_u32);
    let form = Form::new();
    let directory = use_state(|| session.borrow().directory.display().to_string());
    let new_item_id = use_state(|| String::from("1000"));
    let new_item_kind = use_state(|| String::from("6"));
    let new_item_count = use_state(|| String::from("1"));

    // A snapshot of everything the view needs. Cloning keeps the borrow short.
    let entries = session.borrow().entries.clone();
    let selected = session.borrow().selected;
    let message = session.borrow().message.clone();
    let character = session.borrow().character.clone();
    let items = session.borrow().items.clone();
    let has_save = session.borrow().save.is_some();
    let dirty = session.borrow().is_dirty();
    // Reading the revision subscribes this component to session changes.
    let revision_value = *revision.read();
    let _ = revision_value;

    // Loading a save always refreshes the form from the model.
    let sync: Rc<dyn Fn()> = {
        let session = session.clone();
        Rc::new(move || {
            if let Some(character) = session.borrow().character.clone() {
                form.load(&CharacterForm::from_character(&character));
            }
        })
    };

    let header = header(
        &session,
        revision,
        form,
        directory,
        sync.clone(),
        message.clone(),
        dirty,
    );
    let sidebar = sidebar(&session, revision, sync.clone(), &entries, selected);
    let main = main_panel(
        &session,
        revision,
        form,
        character.as_ref(),
        &items,
        new_item_id,
        new_item_kind,
        new_item_count,
        has_save,
    );

    rect()
        .vertical()
        .width(Size::fill())
        .height(Size::fill())
        .content(Content::Flex)
        .background(BACKGROUND)
        .color(TEXT)
        .font_size(14.)
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
                        .child(main),
                ),
        )
}

#[allow(
    clippy::too_many_lines,
    clippy::large_types_passed_by_value,
    clippy::needless_pass_by_value
)] // the form is a bundle of cheap `Copy` state handles
fn header(
    session: &SharedSession,
    revision: State<u32>,
    form: Form,
    directory: State<String>,
    sync: Rc<dyn Fn()>,
    message: crate::session::Message,
    dirty: bool,
) -> Element {
    let status_color = match message.kind {
        MessageKind::Info => MUTED,
        MessageKind::Success => SUCCESS,
        MessageKind::Error => ERROR,
    };

    let scan = {
        let session = session.clone();
        let mut revision = revision;
        let sync = sync.clone();
        move |_| {
            let path = PathBuf::from(directory.read().as_str());
            session.borrow_mut().set_directory(path);
            sync();
            *revision.write() += 1;
        }
    };

    let apply = {
        let session = session.clone();
        let mut revision = revision;
        move |_| {
            let _ = session.borrow_mut().apply(&form.read());
            *revision.write() += 1;
        }
    };

    let save = {
        let session = session.clone();
        let mut revision = revision;
        let sync = sync.clone();
        move |_| {
            let mut borrowed = session.borrow_mut();
            if borrowed.apply(&form.read()).is_ok() {
                let _ = borrowed.save();
            }
            drop(borrowed);
            sync();
            *revision.write() += 1;
        }
    };

    let revert = {
        let session = session.clone();
        let mut revision = revision;
        let sync = sync.clone();
        move |_| {
            let _ = session.borrow_mut().revert();
            sync();
            *revision.write() += 1;
        }
    };

    rect()
        .vertical()
        .width(Size::fill())
        .padding(Gaps::new_symmetric(12., 18.))
        .spacing(8.)
        .background(PANEL)
        .child(
            rect()
                .horizontal()
                .width(Size::fill())
                .spacing(12.)
                .child(
                    label()
                        .text("JadeForge")
                        .font_size(24.)
                        .font_weight(FontWeight::BOLD)
                        .color(ACCENT),
                )
                .child(
                    label()
                        .text("Jade Empire savegame editor")
                        .font_size(12.)
                        .color(MUTED),
                ),
        )
        .child(
            rect()
                .horizontal()
                .width(Size::fill())
                .spacing(10.)
                .content(Content::Flex)
                .child(label().text("Save folder").font_size(12.).color(MUTED))
                .child(
                    Input::new(directory)
                        .placeholder("Path to the Jade Empire Save folder")
                        .width(Size::flex(1.0)),
                )
                .child(Button::new().outline().child("Scan").on_press(scan)),
        )
        .child(
            rect()
                .horizontal()
                .width(Size::fill())
                .spacing(10.)
                .child(Button::new().flat().child("Apply").on_press(apply))
                .child(Button::new().filled().child("Save to disk").on_press(save))
                .child(Button::new().flat().child("Revert").on_press(revert))
                .child(if dirty {
                    label().text("unsaved changes").font_size(12.).color(ACCENT)
                } else {
                    label()
                        .text("everything is saved")
                        .font_size(12.)
                        .color(MUTED)
                }),
        )
        .child(
            label()
                .text(message.text.clone())
                .font_size(12.)
                .max_lines(3)
                .color(status_color),
        )
        .into_element()
}

#[allow(clippy::needless_pass_by_value)]
fn sidebar(
    session: &SharedSession,
    revision: State<u32>,
    sync: Rc<dyn Fn()>,
    entries: &[crate::session::SaveEntry],
    selected: Option<usize>,
) -> Element {
    let rows: Vec<Element> = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let session = session.clone();
            let mut revision = revision;
            let sync = sync.clone();
            let is_selected = selected == Some(index);
            let background = if is_selected { PANEL_ALT } else { PANEL };
            let summary = entry.summary.clone();
            let title = entry.label.clone();
            let broken = entry.error.is_some();
            Button::new()
                .flat()
                .on_press(move |_| {
                    if session.borrow_mut().open(index) {
                        sync();
                    }
                    *revision.write() += 1;
                })
                .child(
                    rect()
                        .vertical()
                        .width(Size::fill())
                        .spacing(2.)
                        .background(background)
                        .padding(Gaps::new_all(8.))
                        .corner_radius(6.)
                        .child(label().text(title).font_size(13.).color(if broken {
                            ERROR
                        } else {
                            TEXT
                        }))
                        .child(label().text(summary).font_size(11.).color(MUTED)),
                )
                .into_element()
        })
        .collect();

    rect()
        .vertical()
        .width(Size::px(320.))
        .height(Size::fill())
        .padding(Gaps::new_all(10.))
        .spacing(6.)
        .background(BACKGROUND)
        .child(
            label()
                .text(format!("{} savegames", entries.len()))
                .font_size(12.)
                .color(MUTED),
        )
        .child(
            ScrollView::new()
                .show_scrollbar(true)
                .drag_scrolling(false)
                .width(Size::fill())
                .height(Size::fill())
                .child(
                    rect()
                        .vertical()
                        .width(Size::fill())
                        .spacing(6.)
                        .children(rows),
                ),
        )
        .into_element()
}

#[allow(
    clippy::too_many_arguments,
    clippy::needless_pass_by_value,
    clippy::large_types_passed_by_value,
    clippy::too_many_lines
)]
fn main_panel(
    session: &SharedSession,
    revision: State<u32>,
    form: Form,
    character: Option<&Character>,
    items: &[Item],
    new_item_id: State<String>,
    new_item_kind: State<String>,
    new_item_count: State<String>,
    has_save: bool,
) -> Element {
    if !has_save {
        return ScrollView::new()
            .width(Size::fill())
            .height(Size::fill())
            .child(
                rect()
                    .vertical()
                    .width(Size::fill())
                    .spacing(8.)
                    .padding(Gaps::new_all(28.))
                    .child(
                        label()
                            .text("No savegame selected")
                            .font_size(20.)
                            .color(TEXT),
                    )
                    .child(
                        label()
                            .text(
                                "Pick a save folder above and select a savegame on the left. \
                                 JadeForge writes a backup next to the savegame before it \
                                 touches the file.",
                            )
                            .font_size(13.)
                            .color(MUTED),
                    ),
            )
            .into_element();
    }

    let item_rows: Vec<Element> = items
        .iter()
        .map(|item| item_row(item, session.clone(), revision))
        .collect();

    let add_item = {
        let session = session.clone();
        let mut revision = revision;
        move |_| {
            let id = new_item_id.read().trim().parse::<u16>();
            let kind = new_item_kind.read().trim().parse::<u8>();
            let count = new_item_count.read().trim().parse::<u8>();
            let mut borrowed = session.borrow_mut();
            match (id, kind, count) {
                (Ok(id), Ok(kind), Ok(count)) => {
                    let _ = borrowed.add_item(id, kind, count);
                }
                _ => borrowed.set_error("Item id, type and count have to be numbers."),
            }
            drop(borrowed);
            *revision.write() += 1;
        }
    };

    let character_section = section(
        "Character",
        vec![
            field("Name", Input::new(form.name).width(Size::px(200.))),
            field("Level", Input::new(form.level).width(Size::px(90.))),
            field(
                "Experience",
                Input::new(form.experience).width(Size::px(140.)),
            ),
            field("Gold", Input::new(form.gold).width(Size::px(140.))),
            field(
                "Unspent points",
                Input::new(form.attribute_points).width(Size::px(90.)),
            ),
            field("Chapter", Input::new(form.chapter).width(Size::px(90.))),
            field("Kills", Input::new(form.kills).width(Size::px(90.))),
            field("Alignment", Input::new(form.alignment).width(Size::px(90.))),
        ],
    );

    let vitals_section = section(
        "Vitals",
        vec![
            field("Health", Input::new(form.health).width(Size::px(110.))),
            field(
                "Health max",
                Input::new(form.health_max).width(Size::px(110.)),
            ),
            field("Chi", Input::new(form.chi).width(Size::px(110.))),
            field("Chi max", Input::new(form.chi_max).width(Size::px(110.))),
            field("Focus", Input::new(form.focus).width(Size::px(110.))),
            field(
                "Focus max",
                Input::new(form.focus_max).width(Size::px(110.)),
            ),
        ],
    );

    let attributes_section = section(
        "Attributes",
        vec![
            field("Body", Input::new(form.body).width(Size::px(90.))),
            field("Mind", Input::new(form.mind).width(Size::px(90.))),
            field("Spirit", Input::new(form.spirit).width(Size::px(90.))),
            field("Charm", Input::new(form.charm).width(Size::px(90.))),
            field("Intuition", Input::new(form.intuition).width(Size::px(90.))),
            field(
                "Intimidate",
                Input::new(form.intimidate).width(Size::px(90.)),
            ),
        ],
    );

    let summary = character.map_or_else(
        || String::from("No character data"),
        |character: &Character| {
            format!(
                "{} - level {} - {}h {:02}m played - {} kills",
                character.name,
                character.level,
                character.time_played / 3_600_000_000,
                (character.time_played / 60_000_000) % 60,
                character.kills
            )
        },
    );

    let inventory_section = stacked_section(
        "Inventory",
        vec![
            label()
                .text(summary)
                .font_size(12.)
                .color(MUTED)
                .into_element(),
            rect()
                .vertical()
                .width(Size::fill())
                .spacing(4.)
                .children(item_rows)
                .into_element(),
            rect()
                .horizontal()
                .spacing(8.)
                .child(label().text("Add item").font_size(12.).color(MUTED))
                .child(
                    Input::new(new_item_id)
                        .placeholder("item id")
                        .width(Size::px(100.)),
                )
                .child(
                    Input::new(new_item_kind)
                        .placeholder("type")
                        .width(Size::px(70.)),
                )
                .child(
                    Input::new(new_item_count)
                        .placeholder("count")
                        .width(Size::px(70.)),
                )
                .child(Button::new().outline().child("Add").on_press(add_item))
                .into_element(),
        ],
    );

    ScrollView::new()
        .show_scrollbar(true)
        .width(Size::fill())
        .height(Size::fill())
        .child(
            rect()
                .vertical()
                .width(Size::fill())
                .spacing(14.)
                .padding(Gaps::new_all(16.))
                .child(character_section)
                .child(vitals_section)
                .child(attributes_section)
                .child(inventory_section),
        )
        .into_element()
}

fn section(title: &'static str, children: Vec<Element>) -> Element {
    rect()
        .vertical()
        .width(Size::fill())
        .spacing(10.)
        .padding(Gaps::new_all(14.))
        .background(PANEL)
        .corner_radius(10.)
        .child(
            label()
                .text(title)
                .font_size(15.)
                .font_weight(FontWeight::BOLD),
        )
        .child(
            rect()
                .horizontal()
                .width(Size::fill())
                .spacing(10.)
                .content(Content::wrap_spacing(10.0))
                .children(children),
        )
        .into_element()
}

fn stacked_section(title: &'static str, children: Vec<Element>) -> Element {
    rect()
        .vertical()
        .width(Size::fill())
        .spacing(10.)
        .padding(Gaps::new_all(14.))
        .background(PANEL)
        .corner_radius(10.)
        .child(
            label()
                .text(title)
                .font_size(15.)
                .font_weight(FontWeight::BOLD),
        )
        .children(children)
        .into_element()
}

fn field(name: &'static str, input: impl IntoElement) -> Element {
    rect()
        .vertical()
        .spacing(4.)
        .child(label().text(name).font_size(11.).color(MUTED))
        .child(input)
        .into_element()
}

fn item_row(item: &Item, session: SharedSession, revision: State<u32>) -> Element {
    let increase = {
        let session = session.clone();
        let mut revision = revision;
        let index = item.struct_index;
        move |_| {
            session.borrow_mut().adjust_item_count(index, 1);
            *revision.write() += 1;
        }
    };
    let decrease = {
        let session = session.clone();
        let mut revision = revision;
        let index = item.struct_index;
        move |_| {
            session.borrow_mut().adjust_item_count(index, -1);
            *revision.write() += 1;
        }
    };
    let remove = {
        let mut revision = revision;
        let index = item.struct_index;
        move |_| {
            session.borrow_mut().remove_item(index);
            *revision.write() += 1;
        }
    };

    rect()
        .horizontal()
        .width(Size::fill())
        .height(Size::px(34.))
        .content(Content::Flex)
        .spacing(8.)
        .padding(Gaps::new_symmetric(4., 8.))
        .background(PANEL_ALT)
        .corner_radius(6.)
        .child(
            label()
                .text(format!("Item {:<6}", item.item_id))
                .font_size(12.)
                .max_lines(1)
                .width(Size::px(110.)),
        )
        .child(
            label()
                .text(format!("type {}", item.kind))
                .font_size(12.)
                .max_lines(1)
                .color(MUTED)
                .width(Size::px(70.)),
        )
        .child(Button::new().flat().child("-").on_press(decrease))
        .child(
            label()
                .text(item.count.to_string())
                .font_size(13.)
                .width(Size::px(36.))
                .text_align(TextAlign::Center),
        )
        .child(Button::new().flat().child("+").on_press(increase))
        .child(rect().width(Size::flex(1.0)))
        .child(Button::new().flat().child("Remove").on_press(remove))
        .into_element()
}
