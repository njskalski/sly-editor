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

use buffer_state::BufferState;
use buffer_state::BufferStateRef;
use buffer_state_observer::BufferStateObserver;
use settings::Settings;
use sly_text_view::SlyTextView;
use std::cell::RefCell;
use std::rc::Rc;
use test_utils::basic_setup::tests::BasicSetup;

fn basic_setup() -> (BasicSetup, BufferStateRef) {
    let settings = Rc::new(RefCell::new(Settings::load_default()));
    let buffer = Rc::new(RefCell::new(BufferState::new()));
    let observer = BufferStateObserver::new(buffer.clone());

    let setup =
        BasicSetup::new(move |setupsetup, ichannel| SlyTextView::new(settings, observer, ichannel));

    (setup, buffer)
}

#[test]
fn first_sly_text_test() {
    let (setup, buffer) = basic_setup();
    let observer = BufferStateObserver::new(buffer);
}
