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

pub struct Selection {
    pub b : usize, //begin inclusive
    pub e : usize, //end EXCLUSIVE (as *everywhere*)
}

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
        for mut c in &mut self.set {
            c.clear_both();
            if c.a > 0 {
                c.a -= 1;
            };
        }
    }

    pub fn move_right<T : Borrow<BufferState>>(&mut self, buf : T) {
        let bs : &BufferState = buf.borrow();
        let len = bs.get_content().get_lines().len_chars();


        for mut c in &mut self.set {
            c.clear_both();
            //we allow anchor after last char (so you can backspace last char)
            if c.a <= len {
                c.a += 1;
            };
        }
    }

    /// TODO(njskalski): how to reduce selections? Overlapping selections?
//    pub fn reduce(&mut self) {
//        self.set.sort();
//        self.set.dedup();
//    }

}