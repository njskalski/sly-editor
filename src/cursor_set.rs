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

use fuzzy_query_view::FuzzyQueryResult::Selected;

pub struct Selection {
    pub b : usize, //begin
    pub e : usize, //end
}

pub struct Cursor {
    pub s : Option<Selection>, // selection
    pub a: usize, //anchor
    pub preferred_column : Option<usize>,
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