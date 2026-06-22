//! WysiwygEditor — the main block-based WYSIWYG editor widget.
//!
//! Renders markdown as a vertical stack of styled block widgets.
//! In edit mode, clicking a block switches it to a TextView for raw editing.
//! On blur/escape, re-parses and re-renders the block.

use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;

use super::model::{BlockKind, MdBlock};
use super::parser::parse_markdown;
use super::renderer::render_block;

/// A block widget that can switch between display and edit mode.
struct BlockEntry {
    block: MdBlock,
    display: gtk::Widget,
    edit: Option<gtk::TextView>,
    mode: BlockMode,
    slot: gtk::Box,
}

#[derive(Clone, Copy, PartialEq)]
enum BlockMode {
    Display,
    Editing,
}

/// Shared inner state for the editor (behind Rc<RefCell<>>).
struct EditorInner {
    blocks_box: gtk::Box,
    entries: Vec<BlockEntry>,
    source: String,
    read_only: bool,
    active_edit: Option<usize>,
    on_change: Option<Box<dyn Fn(String)>>,
}

/// The main WYSIWYG editor widget.
///
/// Uses `Rc<RefCell<>>` internally so click handlers can trigger edit mode.
#[derive(Clone)]
pub struct WysiwygEditor {
    container: gtk::Box,
    inner: Rc<RefCell<EditorInner>>,
}

impl WysiwygEditor {
    /// Create a new editor in read-only display mode.
    pub fn new_read_only() -> Self {
        Self::new(true)
    }

    fn new(read_only: bool) -> Self {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
        container.add_css_class("synap-editor");

        let scroller = gtk::ScrolledWindow::new();
        scroller.set_kinetic_scrolling(true);
        scroller.set_overlay_scrolling(true);
        scroller.set_propagate_natural_height(true);
        scroller.set_vexpand(true);

        let blocks_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        blocks_box.add_css_class("synap-editor-blocks");

        scroller.set_child(Some(&blocks_box));
        container.append(&scroller);

        let inner = EditorInner {
            blocks_box,
            entries: Vec::new(),
            source: String::new(),
            read_only,
            active_edit: None,
            on_change: None,
        };

        Self {
            container,
            inner: Rc::new(RefCell::new(inner)),
        }
    }

    /// Set the callback for content changes.
    pub fn set_on_change(&mut self, f: impl Fn(String) + 'static) {
        self.inner.borrow_mut().on_change = Some(Box::new(f));
    }

    /// Get the root widget for embedding in a parent.
    pub fn widget(&self) -> &gtk::Box {
        &self.container
    }

    /// Toggle read-only mode. When true, blocks cannot be clicked to edit.
    /// Rebuilds blocks to add/remove click handlers.
    /// When switching to editable with empty content, auto-enters edit mode.
    pub fn set_read_only(&self, read_only: bool) {
        let mut inner = self.inner.borrow_mut();
        if inner.read_only == read_only {
            return;
        }
        inner.read_only = read_only;
        let blocks = parse_markdown(&inner.source.clone());
        rebuild_blocks(&mut inner, &blocks, &self.inner);

        // If switching to editable and content is empty, auto-enter edit mode
        if !read_only && inner.source.trim().is_empty() {
            drop(inner);
            let self_ref = self.inner.clone();
            gtk::glib::idle_add_local(move || {
                enter_edit_mode(&self_ref, 0);
                gtk::glib::ControlFlow::Break
            });
        }
    }

    /// Set the markdown content, parsing and rendering all blocks.
    pub fn set_content(&self, markdown: &str) {
        let mut inner = self.inner.borrow_mut();
        if inner.active_edit.is_some() {
            return;
        }
        inner.source = markdown.to_string();
        let blocks = parse_markdown(markdown);
        rebuild_blocks(&mut inner, &blocks, &self.inner);
    }

    /// Get the current raw markdown content.
    pub fn content(&self) -> String {
        self.inner.borrow().source.clone()
    }

    /// Commit all active edits and return the current source.
    #[allow(dead_code)]
    pub fn commit_all(&self) -> String {
        let mut inner = self.inner.borrow_mut();
        if let Some(idx) = inner.active_edit {
            commit_edit_inner(&mut inner, idx);
        }
        let blocks = parse_markdown(&inner.source);
        rebuild_blocks(&mut inner, &blocks, &self.inner);
        inner.source.clone()
    }
}

/// Rebuild all block widgets from a fresh parse.
fn rebuild_blocks(
    inner: &mut EditorInner,
    blocks: &[MdBlock],
    self_ref: &Rc<RefCell<EditorInner>>,
) {
    if let Some(idx) = inner.active_edit {
        commit_edit_inner(inner, idx);
    }

    while let Some(child) = inner.blocks_box.first_child() {
        inner.blocks_box.remove(&child);
    }
    inner.entries.clear();

    if blocks.is_empty()
        || (blocks.len() == 1 && matches!(blocks[0].kind, BlockKind::Blank))
    {
        let label = gtk::Label::new(Some("开始记录..."));
        label.add_css_class("synap-editor-empty");
        label.set_xalign(0.0);
        inner.blocks_box.append(&label);
        return;
    }

    for (i, block) in blocks.iter().enumerate() {
        let entry = create_block_entry(block.clone(), i, inner.read_only, self_ref);
        inner.blocks_box.append(&entry.slot);
        inner.entries.push(entry);
    }
}

/// Create a BlockEntry with click handler wired up.
fn create_block_entry(
    block: MdBlock,
    index: usize,
    read_only: bool,
    self_ref: &Rc<RefCell<EditorInner>>,
) -> BlockEntry {
    let display = render_block(&block);
    let slot = gtk::Box::new(gtk::Orientation::Vertical, 0);
    slot.add_css_class("synap-editor-block-editable");
    slot.append(&display);

    if !read_only {
        let click = gtk::GestureClick::new();
        let inner_ref = self_ref.clone();
        click.connect_pressed(move |_, _, _, _| {
            // Defer to idle so the click event finishes before grab_focus
            let inner_ref = inner_ref.clone();
            gtk::glib::idle_add_local(move || {
                enter_edit_mode(&inner_ref, index);
                gtk::glib::ControlFlow::Break
            });
        });
        slot.add_controller(click);
    }

    BlockEntry {
        block,
        display,
        edit: None,
        mode: BlockMode::Display,
        slot,
    }
}

/// Enter edit mode for a specific block.
fn enter_edit_mode(self_ref: &Rc<RefCell<EditorInner>>, index: usize) {
    let mut inner = self_ref.borrow_mut();
    if inner.read_only || index >= inner.entries.len() {
        return;
    }

    // Commit any other active edit first
    if let Some(active) = inner.active_edit {
        if active != index {
            commit_edit_inner(&mut inner, active);
        }
    }

    let entry = &mut inner.entries[index];
    if entry.mode == BlockMode::Editing {
        return;
    }

    let text_view = gtk::TextView::new();
    text_view.set_wrap_mode(gtk::WrapMode::WordChar);
    text_view.set_top_margin(8);
    text_view.set_bottom_margin(8);
    text_view.set_left_margin(12);
    text_view.set_right_margin(12);
    text_view.add_css_class("synap-editor-edit-view");

    let raw = entry.block.kind.to_markdown();
    text_view.buffer().set_text(&raw);

    entry.slot.remove(&entry.display);
    entry.slot.append(&text_view);
    entry.edit = Some(text_view.clone());
    entry.mode = BlockMode::Editing;
    inner.active_edit = Some(index);

    text_view.grab_focus();
    let buffer = text_view.buffer();
    let start = buffer.start_iter();
    let end = buffer.end_iter();
    buffer.select_range(&start, &end);

    // Connect focus-out to commit
    let inner_ref = self_ref.clone();
    let focus_ctrl = gtk::EventControllerFocus::new();
    focus_ctrl.connect_leave(move |_| {
        // Use idle_add to avoid borrow conflicts
        let inner_ref = inner_ref.clone();
        gtk::glib::idle_add_local(move || {
            let mut inner = inner_ref.borrow_mut();
            if let Some(idx) = inner.active_edit {
                commit_edit_inner(&mut inner, idx);
            }
            gtk::glib::ControlFlow::Break
        });
    });
    text_view.add_controller(focus_ctrl);

    // Connect Escape to commit
    let key_ctrl = gtk::EventControllerKey::new();
    let inner_ref2 = self_ref.clone();
    key_ctrl.connect_key_pressed(move |_, key, _, _| {
        if key == gtk::gdk::Key::Escape {
            let mut inner = inner_ref2.borrow_mut();
            if let Some(idx) = inner.active_edit {
                commit_edit_inner(&mut inner, idx);
            }
            gtk::glib::Propagation::Stop
        } else {
            gtk::glib::Propagation::Proceed
        }
    });
    text_view.add_controller(key_ctrl);
}

/// Commit the edit for a specific block — read text, re-parse, re-render.
fn commit_edit_inner(inner: &mut EditorInner, index: usize) {
    if index >= inner.entries.len() {
        return;
    }

    let entry = &mut inner.entries[index];
    if entry.mode != BlockMode::Editing {
        return;
    }

    let text_view = entry.edit.as_ref().unwrap();
    let buffer = text_view.buffer();
    let start = buffer.start_iter();
    let end = buffer.end_iter();
    let new_text = buffer.text(&start, &end, false).to_string();

    let block_start = entry.block.source_start;
    let block_end = entry.block.source_end;

    if block_start <= inner.source.len() && block_end <= inner.source.len() {
        inner.source = format!(
            "{}{}{}",
            &inner.source[..block_start],
            new_text,
            &inner.source[block_end..]
        );
    }

    let new_blocks = parse_markdown(&new_text);
    let new_block = new_blocks.into_iter().next().unwrap_or(MdBlock {
        kind: BlockKind::Blank,
        source_start: block_start,
        source_end: block_start + new_text.len(),
    });

    entry.slot.remove(text_view);
    let new_display = render_block(&new_block);
    entry.slot.append(&new_display);
    entry.display = new_display;
    entry.block = new_block;
    entry.edit = None;
    entry.mode = BlockMode::Display;
    inner.active_edit = None;

    // Fire change callback
    if let Some(ref f) = inner.on_change {
        f(inner.source.clone());
    }
}
