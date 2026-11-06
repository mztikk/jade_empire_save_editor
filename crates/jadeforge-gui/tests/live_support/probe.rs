//! Observes the live window and lets the test thread drive it.
//!
//! [`Probe`] is shared with a [`FreyaPlugin`] that runs on the window's event
//! loop. After every redraw the plugin publishes the current labels/paragraphs
//! with their layout rectangles, plus the window facts (title, size, scale and
//! whether it is a Wayland window). The test thread reads that snapshot and can
//! post synthetic input or a close request through the very same plugin handle
//! the shipped application uses.
//!
//! This is deliberately not `freya-testing`: the application runs through
//! `freya-winit`, opens a real window on the compositor and renders with the
//! real Skia/GL pipeline.

use std::sync::{Arc, Mutex, PoisonError};

use freya::prelude::*;
use freya::winit;
use freya::winit::raw_window_handle::{HasDisplayHandle, RawDisplayHandle};
use freya_core::events::platform::PlatformEvent;
use freya_core::tree::Tree;
use freya_core::user_event::{GlobalUserEvent, SingleThreadErasedEvent};
use freya_winit::plugins::{FreyaPlugin, PluginEvent, PluginHandle};
use freya_winit::renderer::{
    NativeEvent, NativeGenericEvent, NativePlatformErasedEventAction, NativeWindowEvent,
    NativeWindowEventAction,
};

/// The kind of text element a snapshot came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    /// A [`Label`] element.
    Label,
    /// A [`Paragraph`] element (input values and placeholders).
    Paragraph,
}

/// One visible piece of text in the live window.
#[derive(Clone, Debug)]
pub struct NodeSnapshot {
    /// The text as rendered.
    pub text: String,
    /// The element kind.
    pub kind: NodeKind,
    /// Left edge in window-local pixels.
    pub x: f32,
    /// Top edge in window-local pixels.
    pub y: f32,
    /// Width in window-local pixels.
    pub width: f32,
    /// Height in window-local pixels.
    pub height: f32,
}

impl NodeSnapshot {
    /// Centre of the node in window-local coordinates, the point tests click.
    #[must_use]
    pub fn center(&self) -> (f64, f64) {
        (
            f64::from(self.x) + f64::from(self.width) / 2.0,
            f64::from(self.y) + f64::from(self.height) / 2.0,
        )
    }
}

/// Facts about the real window, refreshed on every redraw.
#[derive(Clone, Debug, Default)]
pub struct WindowSnapshot {
    /// The window title.
    pub title: String,
    /// Inner (client area) width in physical pixels.
    pub width: u32,
    /// Inner (client area) height in physical pixels.
    pub height: u32,
    /// The compositor scale factor.
    pub scale: f64,
    /// Whether winit connected through Wayland rather than X11.
    pub wayland: bool,
    /// Whether the window has been destroyed.
    pub closed: bool,
    /// How many frames the plugin has seen.
    pub frames: u64,
}

#[derive(Default)]
struct Shared {
    handle: Option<PluginHandle>,
    window_id: Option<winit::window::WindowId>,
    nodes: Vec<NodeSnapshot>,
    window: WindowSnapshot,
    ready: bool,
}

/// A thread-safe view of the live window.
#[derive(Clone, Default)]
pub struct Probe {
    shared: Arc<Mutex<Shared>>,
}

impl Probe {
    /// Creates an empty probe. Attach it with [`Probe::plugin`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The plugin that publishes into this probe.
    #[must_use]
    pub fn plugin(&self) -> ProbePlugin {
        ProbePlugin {
            shared: self.shared.clone(),
        }
    }

    /// The latest text nodes, in tree order.
    #[must_use]
    pub fn nodes(&self) -> Vec<NodeSnapshot> {
        self.with(|shared| shared.nodes.clone())
    }

    /// The latest window facts.
    #[must_use]
    pub fn window(&self) -> WindowSnapshot {
        self.with(|shared| shared.window.clone())
    }

    /// Whether the window has been created and the plugin has run.
    #[must_use]
    pub fn ready(&self) -> bool {
        self.with(|shared| shared.ready)
    }

    /// Finds the first node whose text contains `needle`.
    #[must_use]
    pub fn find(&self, needle: &str) -> Option<NodeSnapshot> {
        self.nodes()
            .into_iter()
            .find(|node| node.text.contains(needle))
    }

    /// Finds the first node whose text is exactly `text`.
    #[must_use]
    pub fn find_exact(&self, text: &str) -> Option<NodeSnapshot> {
        self.nodes().into_iter().find(|node| node.text == text)
    }

    /// Requests a redraw so the plugin publishes a fresh snapshot.
    pub fn wake(&self) {
        self.with(|shared| {
            let (Some(handle), Some(window_id)) = (shared.handle.as_ref(), shared.window_id) else {
                return;
            };
            handle.send_event_loop_event(NativeEvent::Window(NativeWindowEvent {
                window_id,
                action: NativeWindowEventAction::User(UserEvent::RequestRedraw),
            }));
        });
    }

    /// Posts a platform event through the real window event loop.
    pub fn send_event(&self, event: PlatformEvent) {
        self.with(|shared| {
            let (Some(handle), Some(window_id)) = (shared.handle.as_ref(), shared.window_id) else {
                return;
            };
            handle.send_platform_event(event, window_id);
        });
    }

    /// Asks the renderer to close the window, bypassing the unsaved-changes
    /// dialog so a failed test can always tear its window down.
    pub fn close(&self) {
        self.with(|shared| {
            let (Some(handle), Some(window_id)) = (shared.handle.as_ref(), shared.window_id) else {
                return;
            };
            handle.send_event_loop_event(NativeEvent::Generic(NativeGenericEvent::User(
                GlobalUserEvent::Erased(SingleThreadErasedEvent(Box::new(
                    NativePlatformErasedEventAction::CloseWindow(window_id),
                ))),
            )));
        });
    }

    fn with<T>(&self, callback: impl FnOnce(&Shared) -> T) -> T {
        let shared = self.shared.lock().unwrap_or_else(PoisonError::into_inner);
        callback(&shared)
    }
}

/// The Freya plugin that fills a [`Probe`] from the window event loop.
pub struct ProbePlugin {
    shared: Arc<Mutex<Shared>>,
}

impl FreyaPlugin for ProbePlugin {
    fn plugin_id(&self) -> &'static str {
        "jadeforge-live-probe"
    }

    fn on_event(&mut self, event: &mut PluginEvent, handle: PluginHandle) {
        let mut shared = self.shared.lock().unwrap_or_else(PoisonError::into_inner);
        match event {
            PluginEvent::WindowCreated { window, .. } => {
                shared.window = snapshot_window(window);
                shared.window_id = Some(window.id());
                shared.handle = Some(handle);
                shared.ready = true;
            }
            PluginEvent::AfterRedraw { window, tree, .. } => {
                let frames = shared.window.frames.saturating_add(1);
                shared.nodes = collect_nodes(tree);
                shared.window = snapshot_window(window);
                shared.window.frames = frames;
                shared.window_id = Some(window.id());
                shared.handle = Some(handle);
                shared.ready = true;
            }
            PluginEvent::WindowClosed { .. } => {
                shared.window.closed = true;
                shared.ready = false;
                shared.handle = None;
                shared.window_id = None;
            }
            _ => {}
        }
    }
}

fn snapshot_window(window: &winit::window::Window) -> WindowSnapshot {
    let size = window.inner_size();
    let wayland = window
        .display_handle()
        .is_ok_and(|handle| matches!(handle.as_raw(), RawDisplayHandle::Wayland(_)));
    WindowSnapshot {
        title: window.title(),
        width: size.width,
        height: size.height,
        scale: window.scale_factor(),
        wayland,
        closed: false,
        frames: 0,
    }
}

fn collect_nodes(tree: &Tree) -> Vec<NodeSnapshot> {
    let mut nodes = Vec::new();
    tree.traverse_depth(|id| {
        let Some(element) = tree.elements.get(&id) else {
            return;
        };
        let (text, kind) = if let Some(label) = Label::try_downcast(element.as_ref()) {
            (label.text.to_string(), NodeKind::Label)
        } else if let Some(paragraph) = Paragraph::try_downcast(element.as_ref()) {
            (paragraph.to_string(), NodeKind::Paragraph)
        } else {
            return;
        };
        if text.is_empty() {
            return;
        }
        let Some(layout) = tree.layout.get(&id) else {
            return;
        };
        if layout.hidden {
            return;
        }
        nodes.push(NodeSnapshot {
            text,
            kind,
            x: layout.area.origin.x,
            y: layout.area.origin.y,
            width: layout.area.size.width,
            height: layout.area.size.height,
        });
    });
    nodes
}
