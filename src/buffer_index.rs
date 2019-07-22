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

use crate::buffer_state_observer::BufferStateObserver;
use crate::fuzzy_index_trait::FuzzyIndexTrait;
use crate::interface::InterfaceNotifier;
use std::cmp;
use std::fmt;
use std::rc::Rc;
use crate::fuzzy_view_item::ViewItem;

pub struct BufferIndex {
    buffers: Vec<BufferStateObserver>,
    items: Vec<Rc<ViewItem>>,
}

impl BufferIndex {
    pub fn new(buffers: Vec<BufferStateObserver>) -> Self {
        let items = buffers.iter().map(|buffer| Rc::new(buffer_to_item(buffer))).collect();
        BufferIndex { buffers: buffers, items: items }
    }
}

impl fmt::Debug for BufferIndex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "BufferIndex : items {:?}", &self.items)
    }
}

impl FuzzyIndexTrait for BufferIndex {
    fn get_results_for(
        &mut self,
        query: &String,
        limit_op: Option<usize>,
        _: Option<InterfaceNotifier>,
    ) -> Vec<Rc<ViewItem>> {
        if let Some(limit) = limit_op {
            self.items[..cmp::min(limit, self.items.len())].to_vec()
        } else {
            self.items.clone()
        }
    }
}

fn buffer_to_item(buffer: &BufferStateObserver) -> ViewItem {
    let header: String = match buffer.get_filename() {
        Some(filename) => {
            format!("{}{}", filename.to_string_lossy(), if buffer.modified() { " *" } else { "" })
        }
        None => {
            format!("<unnamed> {}{}", buffer.buffer_id(), if buffer.modified() { " *" } else { "" })
        }
    };

    let marker = buffer.buffer_id().to_string();

    ViewItem::new(
        header,
        buffer.get_path().map(|path| path.to_string_lossy().to_string()),
        marker,
        None,
    )
}
