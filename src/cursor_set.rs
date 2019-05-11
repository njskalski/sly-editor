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

// Cursor == (Selection, Anchor), thanks Kakoune!
// both positions and anchor are counted in CHARS not offsets.

// The cursor points to a index where a NEW character will be included, or old character will be
// REPLACED.

// Cursor pointing to a newline character is visualized as an option to append preceding it line.

// So Cursor can point 1 character BEYOND length of buffer!

// Newline is always an end of previous line, not a beginning of new.

use ropey::Rope;
use fuzzy_query_view::FuzzyQueryResult::Selected;
use buffer_state::BufferState;
use std::borrow::Borrow;
use std::collections::HashSet;
use serde::de::Unexpected::NewtypeStruct;

const NEWLINE_LENGTH : usize = 1; // TODO(njskalski): add support for multisymbol newlines?

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Selection {
    pub b : usize, //begin inclusive
    pub e : usize, //end EXCLUSIVE (as *everywhere*)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cursor {
    pub s : Option<Selection>, // selection
    pub a: usize, //anchor
    pub preferred_column : Option<usize>,
}

impl Cursor {
    pub fn clear_selection(&mut self) {
        self.s = None;
    }

    pub fn clear_pc(&mut self) {
        self.preferred_column = None;
    }

    // Clears both selection and preferred column.
    pub fn clear_both(&mut self) {
        self.s = None;
        self.preferred_column = None;
    }
}

impl Into<Cursor> for (usize, usize, usize) {
    fn into(self) -> (Cursor) {
        Cursor {
            s : Some(Selection { b : self.0, e : self.1} ),
            a : self.2,
            preferred_column : None,
        }
    }
}

impl Into<Cursor> for usize {
    fn into(self) -> Cursor {
        Cursor {
            s : None,
            a : self,
            preferred_column : None
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CursorSet {
    set : Vec<Cursor>
}

impl CursorSet {
    pub fn new(set : Vec<Cursor>) -> Self {
        CursorSet { set }
    }

    pub fn set(&self) -> &Vec<Cursor> {
        &self.set
    }
}

impl CursorSet {

    pub fn move_left(&mut self) {
        self.move_left_by(1);
    }

    pub fn move_left_by(&mut self, l : usize) {
        for mut c in &mut self.set {
            c.clear_both();
            if c.a > 0 {
                c.a -= std::cmp::min(c.a, l);
            };
        }
    }

    pub fn move_right(&mut self, bs : &BufferState) {
        self.move_right_by(bs, 1);
    }

    pub fn move_right_by(&mut self, bs : &BufferState, l : usize) {
        let len = bs.get_content().get_lines().len_chars();

        for mut c in &mut self.set {
            c.clear_both();
            //we allow anchor after last char (so you can backspace last char)
            if c.a < len {
                c.a = std::cmp::min(c.a + l, len);
            };
        }
    }

    pub fn move_down_by(&mut self, bs : &BufferState, l :usize) {
        if l == 0 {
            return;
        }

        let string = bs.get_content().get_lines().to_string();

        let rope : &Rope = bs.get_content().get_lines();
        let last_line = rope.len_lines();

        for mut c in &mut self.set  {
            //getting data
            let cur_line = rope.char_to_line(c.a);
            let new_line = std::cmp::min(cur_line + l, last_line);
            let cur_line_begin_char_idx = rope.line_to_char(cur_line);
            let current_char_idx = c.a - cur_line_begin_char_idx;


            // This is actually right. Ropey counts '\n' as last character of current line.
            let last_char_idx_in_new_line = if new_line == last_line {
                rope.len_chars()
            } else {
                rope.line_to_char(new_line+1) - NEWLINE_LENGTH
            };

            let new_line_begin = rope.line_to_char(new_line);
            let new_line_num_chars = last_char_idx_in_new_line + 1 - new_line_begin ;

            //setting data
            
            c.clear_selection();

            if let Some(preferred_column) = c.preferred_column {
                assert!(preferred_column >= current_char_idx);
                if preferred_column <= new_line_num_chars {
                    c.clear_pc();
                    c.a = new_line_begin + preferred_column;
                } else {
                    c.a = new_line_begin + new_line_num_chars;
                }
            } else {
                // inequality below is interesting.
                // The line with characters 012 is 3 characters long. So if current char idx is 3
                // it means that line below needs at least 4 character to host it without shift left.
                if new_line_num_chars <= current_char_idx {
                    c.a = last_char_idx_in_new_line;
                    c.preferred_column = Some(current_char_idx);
                } else {
                    c.a = new_line_begin + current_char_idx;
                }
            }
        }
    }

    /// TODO(njskalski): how to reduce selections? Overlapping selections?
    /// TODO(njskalski): it would make a sense not to reduce cursors that have identical .a but different .preferred_column.
    /// Yet we want not to put characters twice for overlapping cursors.
    pub fn reduce(&mut self) {
        let mut curs : HashSet<usize> = HashSet::new();

//        dbg!(&self.set);

        let mut old_curs : Vec<Cursor> = vec![];
        std::mem::swap(&mut old_curs, &mut self.set);

        for mut c in &old_curs {
            let mut found = false;
            for oc in &self.set {
                if c.a == oc.a {
                    found = true;
                    break;
                }
            }

            if !found {
                self.set.push(c.clone());
            }
        }

//        dbg!(&self.set);

//        self.set.sort();
//        self.set.dedup();
    }

}