/*
Copyright 2018 Google LLC

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    https://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
*/

// missing to MVP:
// - selection and copy to clipboard (paste already works)
// - underlining the symbols that offer navigation options (Language Protocol)
// - status bar (row, column, readonly/rw mode, whether modified, whether out-of-sync)
// - search and replace
// - some way to set additional cursors
// missing nice-to-haves
// - regex in "search"
// - normal/insert mode
// other ideas:
// - python script in replace

// TODO(njskalski) never allow overlapping cursors
// TODO(njskalski) update cursors on autoreload from hard drive (autoreload enabled if non-modified,
// and not disabled in options) TODO(njskalski) use View::layout instead of View::required_size to
// determine window size.

use crate::abstract_clipboard::ClipboardType;
use time;

use crate::buffer_state_observer::BufferStateObserver;
use clipboard;
use clipboard::ClipboardProvider;
use crate::content_provider::{EditEvent, RopeBasedContentProvider};
use cursive::direction::Direction;
use cursive::event::{Event, EventResult, Key, MouseButton, MouseEvent};
use cursive::theme::{Color, ColorType};
use cursive::theme::{ColorStyle, Effect};
use cursive::utils::lines::simple::{prefix, simple_prefix, LinesIterator, Row};
use cursive::vec::Vec2;
use cursive::view::View;
use cursive::view::ViewWrapper;
use cursive::views::IdView;
use cursive::views::ViewRef;
use cursive::{Printer, With, XY};
use crate::events::IChannel;
use crate::events::IEvent;
use crate::rich_content::{RichContent, RichLine};
use ropey::Rope;
use crate::settings::Settings;
use crate::sly_view::SlyView;
use std::borrow::BorrowMut;
use std::cell::Ref;
use std::cell::RefCell;
use std::cmp;
use std::cmp::min;
use std::collections::HashMap;
use std::iter;
use std::rc::Rc;
use std::usize::MAX;
use unicode_segmentation;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;
use crate::view_handle::ViewHandle;
use crate::cursor_set::CursorSet;
use core::borrow::Borrow;

const INDEX_MARGIN: usize = 1;
const PAGE_WIDTH: usize = 80;

//const NEWLINE_DRAWING : char = '\u{2424}';

pub struct SlyTextView {
    channel: IChannel, // interface feedback channel
    buffer: BufferStateObserver,
    cursor_set: CursorSet,
    position: Vec2,               // position of upper left corner of view in file
    last_view_size: Option<Vec2>, //not sure if using properly
    settings: Rc<RefCell<Settings>>,
    clipboard_context: ClipboardType,
    special_char_mappings: HashMap<char, char>,
    handle: ViewHandle,
    syntax_highlighting: bool, //local override of global setting.
}

impl SlyView for SlyTextView {
    fn handle(&self) -> ViewHandle {
        self.handle.clone()
    }
}

impl SlyTextView {
    pub fn new(
        settings: Rc<RefCell<Settings>>,
        buffer: BufferStateObserver,
        channel: IChannel,
    ) -> IdView<Self> {
        
        let syntax_highlighting: bool = {
            let setting_borrowed: &RefCell<Settings> = settings.borrow();
            setting_borrowed.borrow().auto_highlighting_enabled()
        };

        let mut view = SlyTextView {
            channel: channel,
            buffer: buffer,
            cursor_set: CursorSet::single(),
            position: Vec2::new(0, 0),
            last_view_size: None,
            settings: settings,
            clipboard_context: ClipboardType::new(),
            special_char_mappings: hashmap!['\n' => '\u{21B5}'],
            handle: ViewHandle::new(),
            syntax_highlighting: syntax_highlighting,
        };

        if syntax_highlighting && !view.syntax_highlighting_on() {
            view.set_syntax_highlighting(true);
        }

        IdView::new(view.handle(), view)
    }

    fn settings_ref(&self) -> Ref<Settings> {
        (*self.settings).borrow()
    }

    pub fn buffer_obs(&self) -> &BufferStateObserver {
        &self.buffer
    }

    fn submit_events(&mut self, events: Vec<EditEvent>) {
        self.channel.send(IEvent::BufferEditEvent(self.buffer.buffer_id(), events)).unwrap()
    }

    /// Returns the position of the cursor in the content string.
    pub fn cursors(&self) -> &CursorSet {
        &self.cursor_set
    }

    pub fn syntax_highlighting_on(&self) -> bool {
        self.syntax_highlighting && self.buffer.borrow_content().is_rich_content_enabled()
    }

    /// Returns value syntax highlighting is set to. May be different than requested.
    pub fn set_syntax_highlighting(&mut self, enabled: bool) -> bool {
        if enabled {
            self.syntax_highlighting = true;
            let mut content = self.buffer.borrow_mut_content();
            content.set_rich_content_enabled(true)
        } else {
            self.syntax_highlighting = false;
            false
        }
    }
}

//TODO(njskalski) handle too small space.
impl View for SlyTextView {
    fn draw(&self, printer: &Printer) {
        let content = self.buffer.borrow_content();

        let line_count: usize = content.get_lines().len_lines();
        let index_length = line_count.to_string().len();

        let cursors = &self.cursor_set;
        let lines = content.get_lines();

        let view_size = self.last_view_size.expect("view size not known.");

        //index + INDEX_MARGIN ----------------------------------------------------------------
        for line_no in
            (self.position.y)..(cmp::min(lines.len_lines(), self.position.y + view_size.y))
        {
            let mut x: usize = 0;

            let y = line_no - self.position.y;
            let line_desc = (line_no + 1).to_string();
            let local_index_length = line_desc.len(); //logarithm? never heard of it.

            printer.with_color(ColorStyle::secondary(), |printer| {
                for _ in 0..(index_length - local_index_length) {
                    printer.print((x, y), " ");
                    x += 1;
                }
                printer.print((x, y), &line_desc);
                x += local_index_length;
                for _ in 0..INDEX_MARGIN {
                    printer.print((x, y), " ");
                    x += 1;
                }
            });

            assert!(x == index_length + INDEX_MARGIN);
        }
        // end of index + INDEX_MARGIN --------------------------------------------------------

        //line --------------------------------------------------------------------------------

        for line_no in
            (self.position.y)..(cmp::min(lines.len_lines(), self.position.y + view_size.y))
        {
            let y = line_no - self.position.y;
            let line_offset = &content.get_lines().line_to_char(line_no);
            let line = &content.get_lines().line(line_no);
            let rich_line_op = self.buffer.borrow_content().get_rich_line(line_no);

            if rich_line_op.is_some()
                && rich_line_op.as_ref().map(|r| r.get_line_no()).unwrap() != line_no
            {
                error!("rich line {:?}: {:?}", line_no, rich_line_op);
            }

            //this allow a cursor *after* the last character. It's actually needed.
            let add = if line_no == lines.len_lines() - 1 { 1 } else { 0 };

            for char_idx in 0..(line.len_chars() + add) {
                let char_offset = line_offset + char_idx;

                let mut special_char = false;
                let symbol: char = if line.len_chars() > char_idx {
                    let c = line.char(char_idx);
                    if !self.special_char_mappings.contains_key(&c) {
                        c
                    } else {
                        special_char = true;
                        self.special_char_mappings[&c]
                    }
                } else {
                    ' '
                };

                let color_style: ColorStyle = if self.had_cursor_at(&char_offset) {
                    ColorStyle::highlight()
                } else {
                    if char_idx <= 80 && !special_char {
                        let mut someColor = ColorStyle::primary();

                        match &rich_line_op {
                            None => {}
                            Some(rich_line) => {
                                rich_line.get_color_at(char_idx).map(|color: Color| {
                                    someColor.front = ColorType::Color(color);
                                });
                            }
                        };

                        someColor
                    } else {
                        ColorStyle::secondary()
                    }
                };

                // let effect = if self.cursors.contains(&char_offset) {
                //     Effect::Underline
                // } else {
                //     Effect::Simple
                // };
                let effect = Effect::Simple;

                printer.with_color(color_style, |printer| {
                    printer.with_effect(effect, |printer| {
                        printer.print(
                            (char_idx + index_length + INDEX_MARGIN, y),
                            &symbol.to_string(),
                        );
                    });
                });
            }
        }
        //end of line ------------------------------------------------------------------------
    }

    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        self.last_view_size = Some(constraint);
        //        debug!("got constraint {:?}", constraint);
        constraint //now we just take whole available space
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        let text_keybindings = self.settings_ref().get_keybindings("text");
        if text_keybindings.event_to_marker().contains_key(&event) {
            let action: &String = &text_keybindings.event_to_marker()[&event];

            let mut consumed = true;
            match action.as_str() {
                "paste" => {
                    let cc = self.clipboard_context.get_contents();
                    match cc {
                        Ok(ref string) => {
                            self.add_text(string);
                        }
                        Err(err_box) => {
                            info!("Error while attempting to access clipboard: {:?}", err_box);
                        }
                    };
                    debug!("pasted");
                }
                "copy" => {
                    debug!("copy! (NOT IMPLEMENTED)");
                }
                _ => consumed = false,
            };
            if consumed {
                return EventResult::Consumed(None);
            }
        }

        let text_view_keybindings = self.settings_ref().get_keybindings("text_view");
        if text_view_keybindings.event_to_marker().contains_key(&event) {
            let action: &String = &text_view_keybindings.event_to_marker()[&event];

            let mut consumed = true;
            match action.as_str() {
                "toggle_syntax_highlighting" => {
                    debug!("toggle syntax highlight");
                    let old_value = self.syntax_highlighting_on();
                    let new_value = self.set_syntax_highlighting(!old_value);
                    if old_value == false && new_value == false {
                        debug!("syntax highlighting unavailable"); // TODO(njskalski): add some msg.
                    }
                }
                _ => consumed = false,
            };
            if consumed {
                return EventResult::Consumed(None);
            }
        }

        let mut consumed = true;
        match event {
            Event::Char(c) => {
                &self.add_text(&c.to_string());
            }
            Event::Key(Key::Enter) => {
                &self.add_text(&'\n'.to_string());
            }
            Event::Key(Key::Backspace) => {
                &self.backspace();
                debug!("hit backspace");
            }
            Event::Key(Key::Left) => {
                &self.cursor_set.move_left();
            }
            Event::Key(Key::Right) => {
                let buffer_state = self.buffer.borrow_state();
                &self.cursor_set.move_right(&buffer_state);
            }
            Event::Key(Key::Up) => {
                let buffer_state = self.buffer.borrow_state();
                &self.cursor_set.move_vertically_by(&buffer_state, -1);
            }
            Event::Key(Key::Down) => {
                let buffer_state = self.buffer.borrow_state();
                &self.cursor_set.move_vertically_by(&buffer_state, 1);
            }
            Event::Key(Key::PageUp) => {
                let height = self.last_view_size.unwrap().y as isize;
                let buffer_state = self.buffer.borrow_state();
                &self.cursor_set.move_vertically_by(&buffer_state, -height);

            }
            Event::Key(Key::PageDown) => {
                let height = self.last_view_size.unwrap().y as isize;
                let buffer_state = self.buffer.borrow_state();
                &self.cursor_set.move_vertically_by(&buffer_state, height);
            }
            _ => {
                debug!("unhandled event (in sly_text_view) {:?}", event);
                consumed = false;
            }
        };
        if consumed {
            EventResult::Consumed(None)
        } else {
            EventResult::Ignored
        }
    }
}

impl SlyTextView {

    // These are work-in-progress implementations.
    fn add_text(&mut self, text: &String) {
        let mut edit_events: Vec<EditEvent> = self
            .cursor_set
            .set()
            .iter()
            .map(|ref cursor| EditEvent::Insert { offset: cursor.a, content: text.clone() })
            .collect();

        let text_len =
            unicode_segmentation::UnicodeSegmentation::graphemes(text.as_str(), true).count();

        edit_events.reverse();
        self.submit_events(edit_events);
        let buffer_state = self.buffer.borrow_state();
        self.cursor_set.move_right_by(&buffer_state, text_len);
    }

    // TODO(njskalski): fix, test
    fn backspace(&mut self) {
        let mut edit_events: Vec<EditEvent> = self
            .cursor_set
            .set()
            .iter()
            .filter(|&cursor| cursor.a > 0)
            .map(|ref cursor| EditEvent::Change {
                offset: cursor.a - 1,
                length: 1,
                content: "".to_string(),
            })
            .collect();

        self.cursor_set.move_left();

        edit_events.reverse();
        self.submit_events(edit_events);
    }

    // TODO(njskalski): color not only anchor, but also scope.
    fn had_cursor_at(&self, offset: &usize) -> bool {
        for c in self.cursor_set.set() {
            if c.a == *offset {
                return true;
            }
        }
        false
    }
}
