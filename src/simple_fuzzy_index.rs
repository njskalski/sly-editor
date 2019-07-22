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

use regex;
use std::iter::FromIterator;
use std::rc::Rc;
use crate::fuzzy_view_item::ViewItem;
use crate::fuzzy_index_trait::FuzzyIndexTrait;
use crate::interface::InterfaceNotifier;

#[derive(Debug)]
pub struct SimpleIndex {
    items: Vec<Rc<ViewItem>>,
}

impl SimpleIndex {
    pub fn new(items: Vec<Rc<ViewItem>>) -> Self {
        SimpleIndex { items: items }
    }
    pub fn add(&mut self, mut new_items: Vec<Rc<ViewItem>>) {
        self.items.append(&mut new_items);
    }
}

impl FuzzyIndexTrait for SimpleIndex {
    fn get_results_for(
        &mut self,
        query: &String,
        limit_op: Option<usize>,
        _: Option<InterfaceNotifier>,
    ) -> Vec<Rc<ViewItem>> {
        let re: regex::Regex = query_to_regex(query);
        let mut res: Vec<Rc<ViewItem>> = Vec::new();

        for ref item in &self.items {
            if re.is_match(&item.get_header()) {
                res.push((*item).clone());

                if let Some(limit) = limit_op {
                    if res.len() == limit {
                        break;
                    }
                }
            }
        }

        res
    }
}

fn query_to_regex(query: &String) -> regex::Regex {
    let mut regex_vec: Vec<char> = Vec::new();

    regex_vec.append(&mut vec!['.', '*']);

    for letter in query.chars() {
        regex_vec.push('[');
        for subletter in letter.to_lowercase() {
            regex_vec.push(subletter);
        }

        for subletter in letter.to_uppercase() {
            regex_vec.push(subletter);
        }
        regex_vec.push(']');
        regex_vec.append(&mut vec!['.', '*']);
    }

    let regex_str: String = String::from_iter(regex_vec);

    debug!("regex str {:?}", regex_str);

    let regex: regex::Regex = regex::Regex::new(&regex_str).unwrap();

    regex
}
