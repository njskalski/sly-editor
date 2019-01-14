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

use core::borrow::BorrowMut;
use cursive::view::View;
use cursive::view::ViewWrapper;
use cursive::views::IdView;
use view_handle::ViewHandle;

/// This is a common trait of all windows used in this program.
pub trait SlyView {
    fn handle(&self) -> ViewHandle;
}

impl<V : SlyView + View> SlyView for cursive::views::IdView<V> {
    fn handle(&self) -> ViewHandle {
        (self as &IdView<V>).with_view(|v| v.handle()).unwrap()
    }
}
