//! Markdown editor — block-based rendered view + syntax-highlighted edit view.
//!
//! Two modes:
//! - **Read-only**: renders markdown as styled block widgets
//! - **Editable**: shows a single `gtk::TextView` with markdown syntax highlighting

use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;

use super::model::MdBlock;
use super::parser::parse_markdown;
use super::renderer::render_block;

/// Inner state shared between callbacks.
struct EditorInner {
    source: String,
    read_only: bool,
    on_change: Option<Box<dyn Fn(String)>>,
    rendered_box: gtk::Box,
    edit_buffer: gtk::TextBuffer,
    edit_view: gtk::TextView,
}

/// The main editor widget.
#[derive(Clone)]
pub struct WysiwygEditor {
    container: gtk::Stack,
    inner: Rc<RefCell<EditorInner>>,
}

impl WysiwygEditor {
    pub fn new_read_only() -> Self {
        Self::new(true)
    }

    fn new(read_only: bool) -> Self {
        // Read-only rendered view
        let rendered_scroller = gtk::ScrolledWindow::new();
        rendered_scroller.set_kinetic_scrolling(true);
        rendered_scroller.set_overlay_scrolling(true);
        rendered_scroller.set_propagate_natural_height(true);
        rendered_scroller.set_vexpand(true);
        rendered_scroller.set_hscrollbar_policy(gtk::PolicyType::Never);

        let rendered_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        rendered_box.add_css_class("synap-editor-blocks");
        rendered_scroller.set_child(Some(&rendered_box));

        // Edit view
        let edit_scroller = gtk::ScrolledWindow::new();
        edit_scroller.set_kinetic_scrolling(true);
        edit_scroller.set_overlay_scrolling(true);
        edit_scroller.set_propagate_natural_height(true);
        edit_scroller.set_vexpand(true);
        edit_scroller.set_hscrollbar_policy(gtk::PolicyType::Never);

        let edit_buffer = gtk::TextBuffer::new(None);
        setup_syntax_tags(&edit_buffer);

        let edit_view = gtk::TextView::with_buffer(&edit_buffer);
        edit_view.set_wrap_mode(gtk::WrapMode::WordChar);
        edit_view.set_top_margin(24);
        edit_view.set_bottom_margin(64);
        edit_view.set_left_margin(32);
        edit_view.set_right_margin(32);
        edit_view.add_css_class("synap-editor-source");
        edit_scroller.set_child(Some(&edit_view));

        // Stack
        let container = gtk::Stack::new();
        container.set_transition_type(gtk::StackTransitionType::Crossfade);
        container.set_transition_duration(120);
        container.add_named(&rendered_scroller, Some("rendered"));
        container.add_named(&edit_scroller, Some("edit"));
        container.set_visible_child_name(if read_only { "rendered" } else { "edit" });

        let inner = Rc::new(RefCell::new(EditorInner {
            source: String::new(),
            read_only,
            on_change: None,
            rendered_box,
            edit_buffer,
            edit_view,
        }));

        // Buffer changed → highlight + callback
        // Use try_borrow_mut to avoid panic when called during set_read_only/set_content
        {
            let inner_ref = Rc::downgrade(&inner);
            let buf = inner.borrow().edit_buffer.clone();
            buf.connect_changed(move |buffer| {
                let Some(inner) = inner_ref.upgrade() else { return };
                let Ok(mut inner) = inner.try_borrow_mut() else { return };
                let text = buffer_text(buffer);
                apply_highlighting(buffer, &text);
                inner.source = text.clone();
                if let Some(ref f) = inner.on_change {
                    f(text);
                }
            });
        }

        Self { container, inner }
    }

    pub fn set_on_change(&mut self, f: impl Fn(String) + 'static) {
        self.inner.borrow_mut().on_change = Some(Box::new(f));
    }

    pub fn widget(&self) -> &gtk::Stack {
        &self.container
    }

    pub fn set_read_only(&self, read_only: bool) {
        {
            let mut inner = self.inner.borrow_mut();
            if inner.read_only == read_only {
                return;
            }
            inner.read_only = read_only;

            if read_only {
                rebuild_rendered(&mut inner);
                self.container.set_visible_child_name("rendered");
                return;
            }
        }
        // Drop borrow before set_text — the connect_changed callback needs to borrow_mut
        let (buffer, source) = {
            let inner = self.inner.borrow();
            (inner.edit_buffer.clone(), inner.source.clone())
        };
        buffer.set_text(&source);
        apply_highlighting(&buffer, &source);
        self.container.set_visible_child_name("edit");
        {
            let inner = self.inner.borrow();
            inner.edit_view.grab_focus();
        }
    }

    pub fn set_content(&self, markdown: &str) {
        let mut inner = self.inner.borrow_mut();
        if inner.source == markdown {
            return;
        }
        inner.source = markdown.to_string();
        if inner.read_only {
            rebuild_rendered(&mut inner);
        }
    }

    pub fn content(&self) -> String {
        self.inner.borrow().source.clone()
    }
}

fn rebuild_rendered(inner: &mut EditorInner) {
    while let Some(child) = inner.rendered_box.first_child() {
        inner.rendered_box.remove(&child);
    }
    let blocks = parse_markdown(&inner.source);
    if blocks.is_empty()
        || (blocks.len() == 1 && matches!(blocks[0].kind, super::model::BlockKind::Blank))
    {
        let label = gtk::Label::new(Some("开始记录..."));
        label.add_css_class("synap-editor-empty");
        label.set_xalign(0.0);
        inner.rendered_box.append(&label);
        return;
    }
    for block in &blocks {
        inner.rendered_box.append(&render_block(block));
    }
}

fn buffer_text(buffer: &gtk::TextBuffer) -> String {
    buffer.text(&buffer.start_iter(), &buffer.end_iter(), false).to_string()
}

// ── Syntax highlighting via pulldown-cmark ──

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

fn setup_syntax_tags(buffer: &gtk::TextBuffer) {
    let table = buffer.tag_table();

    let add = |name: &str, configure: fn(&gtk::TextTag)| {
        let tag = gtk::TextTag::new(Some(name));
        configure(&tag);
        table.add(&tag);
    };

    add("h1", |t| { t.set_weight(700); t.set_scale(1.6); t.set_foreground(Some("#3584e4")); });
    add("h2", |t| { t.set_weight(600); t.set_scale(1.35); t.set_foreground(Some("#3584e4")); });
    add("h3", |t| { t.set_weight(600); t.set_scale(1.15); t.set_foreground(Some("#3584e4")); });
    add("h4", |t| { t.set_weight(600); t.set_foreground(Some("#3584e4")); });
    add("bold", |t| { t.set_weight(700); });
    add("italic", |t| { t.set_style(gtk::pango::Style::Italic); });
    add("strike", |t| { t.set_strikethrough(true); });
    add("code_inline", |t| {
        t.set_family(Some("monospace"));
        t.set_font(Some("monospace 13"));
        t.set_background(Some("alpha(currentColor, 0.08)"));
    });
    add("code_block", |t| {
        t.set_family(Some("monospace"));
        t.set_font(Some("monospace 13"));
        t.set_background(Some("alpha(currentColor, 0.05)"));
        t.set_left_margin(16);
    });
    add("link", |t| {
        t.set_foreground(Some("#3584e4"));
        t.set_underline(gtk::pango::Underline::Single);
    });
    add("quote", |t| {
        t.set_foreground(Some("alpha(currentColor, 0.6)"));
        t.set_left_margin(24);
        t.set_style(gtk::pango::Style::Italic);
    });
    add("marker", |t| { t.set_foreground(Some("alpha(currentColor, 0.4)")); });
    add("hr", |t| { t.set_foreground(Some("alpha(currentColor, 0.25)")); });
}

fn apply_highlighting(buffer: &gtk::TextBuffer, text: &str) {
    let mut start = buffer.start_iter();
    let mut end = buffer.end_iter();
    buffer.remove_all_tags(&mut start, &mut end);

    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_HEADING_ATTRIBUTES;

    let parser = Parser::new_ext(text, options);

    for (event, range) in parser.into_offset_iter() {
        let tag_name = match &event {
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => {
                    match level {
                        pulldown_cmark::HeadingLevel::H1 => Some("h1"),
                        pulldown_cmark::HeadingLevel::H2 => Some("h2"),
                        pulldown_cmark::HeadingLevel::H3 => Some("h3"),
                        _ => Some("h4"),
                    }
                }
                Tag::CodeBlock(_) => Some("code_block"),
                Tag::BlockQuote(_) => Some("quote"),
                Tag::Strong => Some("bold"),
                Tag::Emphasis => Some("italic"),
                Tag::Strikethrough => Some("strike"),
                Tag::Link { .. } => Some("link"),
                _ => None,
            },
            Event::Code(_) => Some("code_inline"),
            Event::Rule => Some("hr"),
            _ => None,
        };

        if let Some(name) = tag_name {
            let start_char = text[..range.start].chars().count() as i32;
            let end_char = text[..range.end].chars().count() as i32;
            let mut s = buffer.iter_at_offset(start_char);
            let mut e = buffer.iter_at_offset(end_char);
            if let Some(t) = buffer.tag_table().lookup(name) {
                buffer.apply_tag(&t, &mut s, &mut e);
            }
        }
    }

    // Highlight list markers separately (pulldown-cmark doesn't expose marker ranges)
    for (line_idx, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        let is_marker = trimmed.starts_with("- ")
            || trimmed.starts_with("* ")
            || trimmed.starts_with("+ ")
            || trimmed.starts_with("- [ ] ")
            || trimmed.starts_with("- [x] ")
            || trimmed.starts_with("- [X] ");

        if is_marker {
            let marker_len = if trimmed.starts_with("- [ ] ") || trimmed.starts_with("- [x] ") || trimmed.starts_with("- [X] ") {
                indent + 6
            } else {
                indent + 2
            };
            if let Some(mut s) = buffer.iter_at_line(line_idx as i32) {
                let mut e = s;
                e.forward_chars(marker_len as i32);
                if let Some(t) = buffer.tag_table().lookup("marker") {
                    buffer.apply_tag(&t, &mut s, &mut e);
                }
            }
        }
    }
}
