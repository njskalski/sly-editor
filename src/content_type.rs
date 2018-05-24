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


use ropey::Rope;

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

#[derive(Debug)]
pub struct RichContentType {
    pub lines : Vec<RichLine>
}

impl RichContentType {
    pub fn get_lines_no(&self) -> usize {
        self.lines.len()
    }

    pub fn new() -> Self {
        RichContentType {
            lines : vec![RichLine{ body : Vec::<(Color, String)>::new()}]
        }
    }
}
