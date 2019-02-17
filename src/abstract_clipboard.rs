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

use clipboard::nop_clipboard::NopClipboardContext;
use clipboard::ClipboardContext;
use clipboard::ClipboardProvider;
use std::error::Error;

#[cfg(test)]
pub type ClipboardType = NopClipboardContext;

#[cfg(test)]
pub fn default_clipboard() -> Result<ClipboardType, Box<std::error::Error>> {
    NopClipboardContext::new()
}

#[cfg(not(test))]
pub type ClipboardType = ClipboardContext;

#[cfg(not(test))]
pub fn default_clipboard() -> Result<ClipboardType, Box<std::error::Error>> {
    ClipboardProvider::new()
}
