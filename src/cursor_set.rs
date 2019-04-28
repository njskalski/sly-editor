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

use fuzzy_query_view::FuzzyQueryResult::Selected;
use buffer_state::BufferState;
use std::borrow::Borrow;
use std::collections::HashSet;

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

    pub fn move_right<T : Borrow<BufferState>>(&mut self, buf : T) {
        self.move_right_by(buf, 1);
    }

    pub fn move_right_by<T : Borrow<BufferState>>(&mut self, buf : T, l : usize) {
        let bs : &BufferState = buf.borrow();
        let len = bs.get_content().get_lines().len_chars();

        for mut c in &mut self.set {
            c.clear_both();
            //we allow anchor after last char (so you can backspace last char)
            if c.a < len {
                c.a = std::cmp::min(c.a + l, len);
            };
        }
    }

    pub fn move_down_by<T : Borrow<BufferState>>(&mut self, buf :T, l :usize) {
        let bs : &BufferState = buf.borrow();
        let last_line = bs.get_content().get_lines().len_lines();

        for mut c in &mut self.set {
            let cur_line = bs.get_content().get_lines().char_to_line(c.a);
            let new_line = std::cmp::min(cur_line + l, last_line);
            let line_begin = bs.get_content().get_lines().line_to_char(new_line);

//            let line_end = bs.get_content().n

            let current_column_pref = c.a - line_begin;

            c.clear_selection();

            if c.preferred_column.is_none() {
                c.preffered_column = Some(current_column_pref);
            }




        }

    }

    /// TODO(njskalski): how to reduce selections? Overlapping selections?
    /// TODO(njskalski): it would make a sense not to reduce cursors that have identical .a but different .preferred_column.
    /// Yet we want not to put characters twice for overlapping cursors.
    pub fn reduce(&mut self) {
        let mut curs : HashSet<usize> = HashSet::new();

        dbg!(&self.set);

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

        dbg!(&self.set);

//        self.set.sort();
//        self.set.dedup();
    }

}