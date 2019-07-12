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

use clipboard::ClipboardProvider;
use clipboard::nop_clipboard::NopClipboardContext;
use clipboard::ClipboardContext;
use std::error::Error;

enum ClipboardEnum {
    Real(ClipboardContext),
    Fake(NopClipboardContext)
}

pub struct ClipboardType {
    data : ClipboardEnum
}

impl ClipboardType {

    #[cfg(not(test))]
    pub fn new() -> ClipboardType {
        let data : ClipboardEnum = match ClipboardContext::new() {
            Ok(context) => ClipboardEnum::Real(context),
            Err(error) => {
                //TODO(njskalski): add logging
                ClipboardEnum::Fake(NopClipboardContext::new().unwrap())
            }
        };

        ClipboardType { data }
    }

    #[cfg(test)]
    pub fn new() -> ClipboardType { ClipboardType { data : ClipboardEnum::Fake(NopClipboardContext::new().unwrap()) } }

    pub fn get_contents(&mut self) -> Result<String, Box<Error>> {
        match &mut self.data {
            ClipboardEnum::Fake(ref mut nop) => nop.get_contents(),
            ClipboardEnum::Real(ref mut clip) => clip.get_contents()
        }
    }
}
