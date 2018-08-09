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

use std::cell::Ref;
use std::cell::RefCell;
use std::rc::Rc;

use content_provider::RopeBasedContentProvider;
use buffer_state::BufferState;

use cursive;

#[derive(Clone)]
pub struct BufferStateObserver {
    buffer_state : Rc<RefCell<BufferState>>,
}

impl BufferStateObserver {
    pub fn new(buffer_state : Rc<RefCell<BufferState>>) -> Self {
        BufferStateObserver{ buffer_state : buffer_state }
    }

    /// borrows unmutably content
    pub fn content(&self) -> Ref<RopeBasedContentProvider> {
        Ref::map(self.buffer_state.borrow(), |x| x.get_content())
    }

    pub fn is_loaded(&self) -> bool {
        self.buffer_state.borrow().get_screen_id().is_some()
    }

    pub fn get_screen_id(&self) -> cursive::ScreenId {
        self.buffer_state.borrow().get_screen_id().unwrap()
    }

    pub fn get_path(&self) -> &Option<String> {
        self.buffer_state.borrow().get_path_ref()
    }
}
