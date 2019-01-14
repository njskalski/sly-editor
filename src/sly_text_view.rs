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
// - syntax highlighting
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

use time;

use buffer_state_observer::BufferStateObserver;
use clipboard;
use clipboard::ClipboardProvider;
use content_provider::{EditEvent, RopeBasedContentProvider};
use cursive::direction::Direction;
use cursive::event::{Event, EventResult, Key, MouseButton, MouseEvent};
use cursive::theme::{Color, ColorType};
use cursive::theme::{ColorStyle, Effect};
use cursive::utils::lines::simple::{prefix, simple_prefix, LinesIterator, Row};
use cursive::vec::Vec2;
use cursive::view::{View, ViewWrapper};
use cursive::views::IdView;
use cursive::{Printer, With, XY};
use events::IChannel;
use events::IEvent;
use rich_content::{RichContent, RichLine};
use ropey::Rope;
use settings::Settings;
use sly_view::SlyView;
use std::borrow::BorrowMut;
use std::cmp;
use std::cmp::min;
use std::collections::HashMap;
use std::iter;
use std::rc::Rc;
use std::usize::MAX;
use unicode_segmentation;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;
use view_handle::ViewHandle;

const INDEX_MARGIN : usize = 1;
const PAGE_WIDTH : usize = 80;

macro_rules! hashmap {
    ($( $key: expr => $val: expr ),*) => {{
         let mut map = ::std::collections::HashMap::new();
         $( map.insert($key, $val); )*
         map
    }}
}

//Cursor: offset in CHARS and "preferred column" (probably cached y coordinate, don't remember).
type Cursor = (usize, Option<usize>);

//const NEWLINE_DRAWING : char = '\u{2424}';

pub struct SlyTextView {
    channel :               IChannel, // interface feedback channel
    buffer :                BufferStateObserver,
    cursors :               Vec<Cursor>, // offset in CHARS, preferred column
    position :              Vec2,        // position of upper left corner of view in file
    last_view_size :        Option<Vec2>, //not sure if using properly
    settings :              Rc<Settings>,
    clipboard_context :     clipboard::ClipboardContext,
    special_char_mappings : HashMap<char, char>,
    handle :                ViewHandle,
}

impl SlyView for SlyTextView {
    fn handle(&self) -> ViewHandle {
        self.handle.clone()
    }
}

impl SlyTextView {
    pub fn new(
        settings : Rc<Settings>,
        buffer : BufferStateObserver,
        channel : IChannel,
    ) -> IdView<Self> {
        let view = SlyTextView {
            channel :               channel,
            buffer :                buffer,
            cursors :               vec![(0, None)],
            position :              Vec2::new(0, 0),
            last_view_size :        None,
            settings :              settings,
            clipboard_context :     clipboard::ClipboardProvider::new().unwrap(),
            special_char_mappings : hashmap!['\n' => '\u{21B5}'],
            handle :                ViewHandle::new(),
        };

        IdView::new(view.handle(), view)
    }

    pub fn buffer_obs(&self) -> &BufferStateObserver {
        &self.buffer
    }

    fn submit_events(&mut self, events : Vec<EditEvent>) {
        self.channel.send(IEvent::BufferEditEvent(self.buffer.buffer_id(), events)).unwrap()
    }

    /// Returns the position of the cursor in the content string.
    pub fn cursors(&self) -> &Vec<Cursor> {
        &self.cursors
    }

    fn toggle_syntax_highlight(&mut self) {
        let mut content = self.buffer.borrow_mut_content();
        let rich_content_enabled = content.is_rich_content_enabled();
        content.set_rich_content_enabled(!rich_content_enabled);
    }
}

//TODO(njskalski) handle too small space.
impl View for SlyTextView {
    fn draw(&self, printer : &Printer) {
        let content = self.buffer.borrow_content();

        let line_count : usize = content.get_lines().len_lines();
        let index_length = line_count.to_string().len();

        let cursors = &self.cursors; //: Vec<Vec2> = textWindow.filter_cursors(&self.cursors);
        let lines = content.get_lines();

        let view_size = self.last_view_size.expect("view size not known.");

        //index + INDEX_MARGIN ----------------------------------------------------------------
        for line_no in
            (self.position.y)..(cmp::min(lines.len_lines(), self.position.y + view_size.y))
        {
            let mut x : usize = 0;

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
                let symbol : char = if line.len_chars() > char_idx {
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

                let color_style : ColorStyle = if self.had_cursor_at(&char_offset) {
                    ColorStyle::highlight()
                } else {
                    if char_idx <= 80 && !special_char {
                        let mut someColor = ColorStyle::primary();

                        match &rich_line_op {
                            None => {}
                            Some(rich_line) => {
                                rich_line.get_color_at(char_idx).map(|color : Color| {
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

    fn required_size(&mut self, constraint : Vec2) -> Vec2 {
        self.last_view_size = Some(constraint);
        //        debug!("got constraint {:?}", constraint);
        constraint //now we just take whole available space
    }

    fn on_event(&mut self, event : Event) -> EventResult {
        let text_keybindings = &self.settings.get_keybindings("text");
        if text_keybindings.contains_key(&event) {
            let action : &String = &text_keybindings[&event];

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

        let text_view_keybindings = &self.settings.get_keybindings("text_view");
        if text_view_keybindings.contains_key(&event) {
            let action : &String = &text_view_keybindings[&event];

            let mut consumed = true;
            match action.as_str() {
                "toggle_syntax_highlighting" => {
                    debug!("toggle syntax highlight");
                    self.toggle_syntax_highlight();
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
                &self.move_all_cursors_left();
            }
            Event::Key(Key::Right) => {
                &self.move_all_cursors_right();
            }
            Event::Key(Key::Up) => {
                &self.move_all_cursors_up(1);
            }
            Event::Key(Key::Down) => {
                &self.move_all_cursors_down(1);
            }
            Event::Key(Key::PageUp) => {
                let height = self.last_view_size.unwrap().y;
                &self.move_all_cursors_up(height);
            }
            Event::Key(Key::PageDown) => {
                let height = self.last_view_size.unwrap().y;
                &self.move_all_cursors_down(height);
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
    //TODO test
    fn has_cursors_outside_view(&self) -> bool {
        let content = self.buffer.borrow_content();
        let rope = content.get_lines();
        let first_offset : usize = rope.line_to_char(self.position.y);
        let last_offset : usize =
            rope.line_to_char(self.position.y + self.last_view_size.unwrap().y) - 1;

        for c in &self.cursors {
            if c.0 < first_offset || c.0 > last_offset {
                return true;
            }
        }
        false
    }

    fn make_sure_first_cursor_visible(&mut self) {
        let y = self.last_view_size.unwrap().y;
        let offset = self.cursors[0].0;
        let line = self.buffer.borrow_content().get_lines().char_to_line(offset);
        if line + 1 > self.position.y + y {
            self.position.y = line - y + 1;
        }

        if line < self.position.y {
            self.position.y = line;
        }
    }

    // These are work-in-progress implementations.
    fn add_text(&mut self, text : &String) {
        let mut edit_events : Vec<EditEvent> = self
            .cursors
            .iter()
            .map(|&cursor| EditEvent::Insert { offset : cursor.0, content : text.clone() })
            .collect();

        let text_len =
            unicode_segmentation::UnicodeSegmentation::graphemes(text.as_str(), true).count();

        edit_events.reverse();
        self.submit_events(edit_events);
        self.mod_all_cursors(text_len as isize);
    }

    fn move_cursor_to_line(rope : &Rope, c : &mut Cursor, other_line : usize) {
        // let rope = &self.buffer.content().get_lines();
        assert!(other_line < rope.len_lines());
        let line = rope.char_to_line(c.0);
        let pos_in_line = c.0 - rope.line_to_char(line);
        if c.1.is_some() {
            assert!(c.1.unwrap() > pos_in_line); //the whole point in tracking that is to be able to shift back right
        }

        let effective_pos_in_line = match c.1 {
            Some(pos) => pos,
            None => pos_in_line,
        };
        c.1 = None;

        let pos_in_line_below = cmp::min(rope.line(other_line).len_chars(), effective_pos_in_line);
        if effective_pos_in_line > pos_in_line_below {
            c.1 = Some(effective_pos_in_line);
        }

        c.0 = rope.line_to_char(other_line) + pos_in_line_below;
    }

    fn move_all_cursors_up(&mut self, len : usize) {
        assert!(len > 0);
        {
            let content = self.buffer.borrow_content();
            let rope : &Rope = content.get_lines();
            for mut c in &mut self.cursors {
                let line = rope.char_to_line(c.0);
                if line == 0 {
                    c.0 = 0;
                    c.1 = None;
                } else {
                    let next_line_no = if line > len { line - len } else { 0 };
                    Self::move_cursor_to_line(rope, &mut c, next_line_no);
                }
            }
        }
        self.make_sure_first_cursor_visible();
    }

    fn move_all_cursors_down(&mut self, len : usize) {
        assert!(len > 0);
        {
            let content = self.buffer.borrow_content();
            let rope : &Rope = content.get_lines();
            for mut c in &mut self.cursors {
                let line = rope.char_to_line(c.0);
                if line == rope.len_lines() - 1 {
                    c.0 = rope.len_chars() - 1;
                    c.1 = None;
                } else {
                    let next_line_no = cmp::min(rope.len_lines() - 1, line + len);
                    Self::move_cursor_to_line(rope, &mut c, next_line_no);
                }
            }
        }
        self.make_sure_first_cursor_visible();
    }

    fn move_all_cursors_left(&mut self) {
        let content = self.buffer.borrow_content();
        let rope = content.get_lines();
        for c in &mut self.cursors {
            if c.0 > 0 {
                c.0 -= 1;
            }
            if c.1.is_some() {
                c.1 = None;
            }
        }
    }

    fn move_all_cursors_right(&mut self) {
        let content = self.buffer.borrow_content();
        let rope = content.get_lines();
        for c in &mut self.cursors {
            if c.0 < rope.len_chars() {
                c.0 += 1;
            }
            if c.1.is_some() {
                c.1 = None;
            }
        }
    }

    fn reduce_cursor_duplicates(&mut self) {
        self.cursors.dedup_by(|a, b| a.0 == b.0);
    }

    fn backspace(&mut self) {
        let mut edit_events : Vec<EditEvent> = self
            .cursors
            .iter()
            .filter(|&cursor| cursor.0 > 0)
            .map(|&cursor| EditEvent::Change {
                offset :  cursor.0 - 1,
                length :  1,
                content : "".to_string(),
            })
            .collect();

        self.mod_all_cursors(-1);

        edit_events.reverse();
        self.submit_events(edit_events);
    }

    fn mod_all_cursors(&mut self, diff : isize) {
        for (i, c) in self.cursors.iter_mut().enumerate() {
            if diff > 0 {
                c.0 += (diff as usize * (i + 1));
            } else {
                let pdiff = (diff * -1) as usize;
                c.0 -= cmp::min(c.0, pdiff * (i + 1));
            }
            c.1 = None;
        }
        self.reduce_cursor_duplicates();
    }

    fn had_cursor_at(&self, offset : &usize) -> bool {
        for ref c in &self.cursors {
            if c.0 == *offset {
                return true;
            }
        }
        false
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//
//
//
// }
