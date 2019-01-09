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

use std::rc::Rc;

use cursive::align::{Align, HAlign, VAlign};
use cursive::direction::Direction;
use cursive::event;
use cursive::event::*;
use cursive::theme::*;
use cursive::traits::*;
use cursive::vec::Vec2;
use cursive::view::{ScrollBase, View};
use cursive::views::*;
use cursive::{Cursive, Printer};

use settings::KeybindingsType;
use unicode_segmentation::UnicodeSegmentation as us;

use std::collections::HashMap;
use std::marker::Sized;

use std::cmp::{Eq, PartialEq};
use std::path::Path;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct ViewItem {
    header : String,
    desc :   Option<String>,
    marker : String,
}

impl ViewItem {
    pub fn new(header : String, desc : Option<String>, marker : String) -> Self {
        ViewItem { header : header, desc : desc, marker : marker }
    }

    pub fn get_header(&self) -> &String {
        &self.header
    }

    pub fn get_description(&self) -> &Option<String> {
        &self.desc
    }

    pub fn get_marker(&self) -> &String {
        &self.marker
    }

    pub fn get_height_in_lines(&self) -> usize {
        1 + (if self.desc.is_none() { 0 } else { 1 })
    }
}

impl PartialEq for ViewItem {
    fn eq(&self, other : &ViewItem) -> bool {
        self.marker == other.marker
    }
}

impl Eq for ViewItem {}

pub fn get_dummy_items() -> Vec<ViewItem> {
    vec![ViewItem { header : "header 1".to_string(),
                    desc :   Some("some boring desc1".to_string()),
                    marker : "1".to_string(), },
         ViewItem { header : "hakuna 2".to_string(),
                    desc :   Some("some boring desc2".to_string()),
                    marker : "2".to_string(), },
         ViewItem { header : "matata 3".to_string(),
                    desc :   Some("some boringmultiline\ndesc3".to_string()),
                    marker : "3".to_string(), },]
}

pub fn file_list_to_items(file_list : &Vec<PathBuf>) -> Vec<ViewItem> {
    // TODO(njskalski) add support to new (non-existent) files.
    file_list.iter()
             .map(|f| ViewItem { header : f.file_name().unwrap().to_string_lossy().to_string(),
                                 desc :   None,
                                 marker : f.to_string_lossy().to_string(), })
             .collect()
}
