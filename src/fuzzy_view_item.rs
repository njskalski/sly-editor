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
use cursive::traits::*;
use cursive::vec::Vec2;
use cursive::view::{ScrollBase, View};
use cursive::views::*;
use cursive::{Cursive, Printer};
use cursive::theme::*;

use settings::KeybindingsType;
use unicode_segmentation::UnicodeSegmentation as us;

use std::collections::HashMap;
use std::marker::Sized;

use std::path::Path;

pub trait ViewItem {
    fn get_header(&self) -> &String;
    fn get_description(&self) -> &Option<String>;
    fn get_marker(&self) -> &String; //TODO this should vary
    fn get_height_in_lines(&self) -> usize;
}

#[derive(Clone, Debug)]
pub struct ComplexViewItem {
    header: String,
    desc: Option<String>,
    marker: String,
}

impl ComplexViewItem {
    pub fn new(header : String, desc : Option<String>, marker : String) -> Self {
        ComplexViewItem {
            header : header,
            desc : desc,
            marker : marker
        }
    }
}

impl ViewItem for ComplexViewItem {
    fn get_header(&self) -> &String {
        &self.header
    }

    fn get_description(&self) -> &Option<String> {
        &self.desc
    }

    fn get_marker(&self) -> &String {
        &self.marker
    }

    fn get_height_in_lines(&self) -> usize {
        1 + (if self.desc.is_none() { 0 } else { 1 })
    }
}

pub fn get_dummy_items() -> Vec<ComplexViewItem> {
    vec![
        ComplexViewItem {
            header: "header 1".to_string(),
            desc: Some("some boring desc1".to_string()),
            marker: "1".to_string(),
        },
        ComplexViewItem {
            header: "hakuna 2".to_string(),
            desc: Some("some boring desc2".to_string()),
            marker: "2".to_string(),
        },
        ComplexViewItem {
            header: "matata 3".to_string(),
            desc: Some("some boringmultiline\ndesc3".to_string()),
            marker: "3".to_string(),
        },
    ]
}

pub fn file_list_to_items(file_list : &Vec<String>) -> Vec<ComplexViewItem> {
    file_list.iter().map(|f| {
        // TODO support windows?
        match f.rfind("/") {
            None => {
                ComplexViewItem {
                    header : f.clone(),
                    desc : None,
                    marker: f.clone()
                }
            },
            Some(sep_pos) => {
                ComplexViewItem {
                    header : f[sep_pos+1..].to_string(),
                    desc : Some(f[0..sep_pos].to_string()),
                    marker: f.clone()
                }
            }
        }
    }).collect()
}
