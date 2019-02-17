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

use cursive::event::Event;
use cursive::event::Event::*;

#[derive(Clone, Debug)]
pub struct KeyboardShortcut {
    event : Event
}

impl KeyboardShortcut {
    pub fn new(event : Event) -> Self {
        assert!(is_keyboard_event(&event));
        KeyboardShortcut { event }
    }
}

fn is_keyboard_event(event : &Event) -> bool {
    match event {
        &Char(_) => true,
        &CtrlChar(_) => true,
        &AltChar(_) => true,
        &Key(_) => true,
        &Shift(_) => true,
        &Alt(_) => true,
        &AltShift(_) => true,
        &Ctrl(_) => true,
        &CtrlShift(_) => true,
        &CtrlAlt(_) => true,
        &Exit => true, // this is Ctrl(c)
        _ => {
            println!("not keyboard event {:?}", event);
            false
        }
    }
}