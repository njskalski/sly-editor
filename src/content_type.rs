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

// the "rich" text format is strongly work in progress, it was written primarily to check what would
// be the cost of integrating syntax highlighting from other editors. I decided to proceed with that
// approach, it's on the May 2018 roadmap.

// TODO(njskalski) secure with accessors after fixing the format.

use syntax;

use ropey::Rope;
use std::ops::{Index, IndexMut};
use std::iter::{Iterator, ExactSizeIterator};
use content_provider::RopeBasedContentProvider;
use std::cell::Ref;

#[derive(Debug)]
pub struct Color {
    pub r : u8,
    pub g : u8,
    pub b : u8
}

#[derive(Debug)]
pub struct RichLine {
    pub body : Vec<(Color, String)>
}

//TODO(njskalski): obviously optimise
#[derive(Debug)]
pub struct RichContent {
//    co : Ref<'a, RopeBasedContentProvider>,
    lines : Vec<RichLine>
}



impl RichContent {
    pub fn new(rope : &Rope) -> Self {
        let lines = syntax::rope_to_colors(rope, None);
        return RichContent { lines }
    }
}

struct RichLinesIterator<'a> {
    content : &'a RichContent,
    line_no : usize
}
//
//impl ExactSizeIterator for RichLinesIterator {
//
//}

impl <'a> Iterator for RichLinesIterator<'a> {
    type Item = &'a RichLine;

    fn next(&mut self) -> Option<Self::Item> {
        let len = self.content.lines.len();

        if len < self.line_no {
            let old_line_no = self.line_no;
            self.line_no += 1;
            let line : Self::Item = &self.content.lines[old_line_no];
            Some(line)
        } else { None }
    }
}

//impl std::iter::ExactSizeIterator for RichContent {
//
//}