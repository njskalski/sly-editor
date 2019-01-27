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

use buffer_state_observer::BufferStateObserver;
use fuzzy_index_trait::FuzzyIndexTrait;
use fuzzy_view_item::*;
use std::rc::Rc;
use std::cmp;

pub struct BufferIndex {
    buffers : Vec<BufferStateObserver>,
    items :   Vec<Rc<ViewItem>>,
}

impl BufferIndex {
    pub fn new(buffers : Vec<BufferStateObserver>) -> Self {
        let items = buffers.iter().map(|buffer| { Rc::new(buffer_to_item(buffer))}).collect();
        BufferIndex { buffers : buffers, items : items }
    }
}

impl FuzzyIndexTrait for BufferIndex {
    fn get_results_for(&mut self, query : &String, limit_op : Option<usize>) -> Vec<Rc<ViewItem>> {
        if let Some(limit) = limit_op {
            self.items[..cmp::min(limit, self.items.len())].to_vec()
        } else {
            self.items.clone()
        }
    }
}

fn buffer_to_item(buffer : &BufferStateObserver) -> ViewItem {

    let header : String = match buffer.get_filename() {
        Some(filename) => filename.to_string_lossy().to_string(),
        None => "<unnamed>".to_string()
    };

    let marker = buffer.buffer_id().to_string();

    ViewItem::new(
        header,
        buffer.get_path().map(|path| path.to_string_lossy().to_string()),
        marker,
    )
}